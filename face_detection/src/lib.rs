use anyhow::Result;
use image::{DynamicImage, GenericImageView, imageops::FilterType};
use ndarray::Array4;
use onnx::new_session_from_path;
use std::path::PathBuf;
use log::info;
use onnx::Session;
use onnx::Value;
use ndarray::ArrayViewD;
/// Target dimensions for the face detection model (please see: https://github.com/onnx/models/tree/main/validated/vision/body_analysis/ultraface)
pub const TARGET_WIDTH: u32 = 640;
pub const TARGET_HEIGHT: u32 = 480;

#[derive(Debug, Clone)]
pub struct FaceRegion {
    pub x1: u32,
    pub y1: u32,
    pub x2: u32,
    pub y2: u32,
    pub confidence: f32,
}

impl FaceRegion {
    /// Returns whether this region should be considered for scrambling
    pub fn is_valid_for_scrambling(&self, min_confidence: f32) -> bool {
        self.confidence >= min_confidence
    }

    /// Gets the dimensions of the face region
    pub fn dimensions(&self) -> (u32, u32) {
        (self.x2 - self.x1, self.y2 - self.y1)
    }
    
    /// Expands the region by a factor to ensure we capture the full face
    pub fn expand(&self, factor: f32, max_width: u32, max_height: u32) -> Self {
        let width = (self.x2 - self.x1) as f32;
        let height = (self.y2 - self.y1) as f32;
        let expand_x = (width * (factor - 1.0) / 2.0) as u32;
        let expand_y = (height * (factor - 1.0) / 2.0) as u32;

        Self {
            x1: self.x1.saturating_sub(expand_x),
            y1: self.y1.saturating_sub(expand_y),
            x2: (self.x2 + expand_x).min(max_width),
            y2: (self.y2 + expand_y).min(max_height),
            confidence: self.confidence,
        }
    }
}

/// Loads the face detection model: This currently TODO.
pub fn load_face_detector(model_path: Option<PathBuf>) -> Result<Session> {
    let model_path = if let Some(path) = model_path {
        path
    } else {
        #[cfg(debug_assertions)]
        let path = PathBuf::from("resources/models/version-RFB-640.onnx");
        
        #[cfg(not(debug_assertions))]
        let path = {
            let exe_path = std::env::current_exe().expect("Failed to get executable path");
            info!("Executable path: {:?}", exe_path);

            let resource_path = if cfg!(target_os = "macos") {
                // macOS: navigate from .app/Contents/MacOS/exe to .app/Contents/Resources
                exe_path
                    .parent() 
                    .and_then(|p| p.parent())
                    .map(|p| p.join("Resources/models/version-RFB-640.onnx"))
            } else {
                // Windows: resources are in the same directory as the executable
                exe_path
                    .parent()
                    .map(|p| p.join("resources/models/version-RFB-640.onnx"))
            }.expect("Failed to construct resource path");
                
            info!("Constructed resource path: {:?}", resource_path);
            info!("Resource path exists: {}", resource_path.exists());
            
            resource_path
        };
        
        path
    };
    
    info!("Loading face detection model from {:?}", model_path);
    new_session_from_path(model_path)
}
fn preprocess_image(image: &DynamicImage) -> Array4<f32> {
    let resized = image.resize_exact(TARGET_WIDTH, TARGET_HEIGHT, FilterType::CatmullRom);
    let (width, height) = resized.dimensions();
    let mut tensor = Array4::<f32>::zeros((1, 3, height as usize, width as usize));
    
    for (x, y, pixel) in resized.pixels() {
        let [r, g, b, _] = pixel.0;
        tensor[[0, 0, y as usize, x as usize]] = (r as f32 - 127.0) / 128.0;
        tensor[[0, 1, y as usize, x as usize]] = (g as f32 - 127.0) / 128.0;
        tensor[[0, 2, y as usize, x as usize]] = (b as f32 - 127.0) / 128.0;
    }
    tensor
}

/// Detects face regions in an image that can be used for selective scrambling
/// expansion_factor: How much to expand detected regions?
pub fn detect_face_regions(
    image: &DynamicImage, 
    session: &Session, 
    confidence_threshold: f32,
    expansion_factor: Option<f32>,
) -> Result<Vec<FaceRegion>> {
    let input_tensor = preprocess_image(image);
    let input_name = &session.inputs[0].name;
    let input_values = std::collections::HashMap::from([(
        input_name.as_str(),
        Value::from_array(input_tensor.view())?
    )]);
    let outputs = session.run(input_values)?;
    let output_names: Vec<_> = session.outputs.iter().map(|o| o.name.as_str()).collect();
    let confidences: ArrayViewD<f32> = outputs[output_names[0]].try_extract_tensor()?;
    let boxes: ArrayViewD<f32> = outputs[output_names[1]].try_extract_tensor()?;
    info!("Processing detections... Confidence shape: {:?}, Boxes shape: {:?}", 
          confidences.shape(), boxes.shape());
    let num_detections = confidences.shape()[1];
    let (orig_width, orig_height) = image.dimensions();
    let mut detected_boxes = Vec::new();
    for i in 0..num_detections {
        let face_confidence = confidences[[0, i, 1]];
        if face_confidence < confidence_threshold {
            continue;
        }
        
        // corner coordinates
        let x1 = boxes[[0, i, 0]];
        let y1 = boxes[[0, i, 1]];
        let x2 = boxes[[0, i, 2]];
        let y2 = boxes[[0, i, 3]];
        
        info!("Raw detection: x1={}, y1={}, x2={}, y2={}, conf={}", 
              x1, y1, x2, y2, face_confidence);
        // Scale to image dimensions
        let mut region = FaceRegion {
            x1: (x1 * orig_width as f32) as u32,
            y1: (y1 * orig_height as f32) as u32,
            x2: (x2 * orig_width as f32) as u32,
            y2: (y2 * orig_height as f32) as u32,
            confidence: face_confidence,
        };
        // Skip invalid boxes
        if region.x2 <= region.x1 || region.y2 <= region.y1 {
            continue;
        }
        // Only expand if requested and box is valid
        if let Some(factor) = expansion_factor {
            let orig_region = region.clone();
            region = region.expand(factor, orig_width, orig_height);
            info!("Expanded region: {:?} -> {:?}", orig_region, region);
        }
        detected_boxes.push(region);
    }
    // Sort by confidence and apply NMS (please see:https://github.com/onnx/models/blob/main/validated/vision/body_analysis/ultraface/dependencies/box_utils.py)
    detected_boxes.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
    let regions = apply_nms(detected_boxes, 0.5)?;
    info!("Detected {} face regions above threshold {}", regions.len(), confidence_threshold);
    Ok(regions)
}

fn calculate_iou(a: &FaceRegion, b: &FaceRegion) -> f32 {
    let x_left = a.x1.max(b.x1) as f32;
    let y_top = a.y1.max(b.y1) as f32;
    let x_right = a.x2.min(b.x2) as f32;
    let y_bottom = a.y2.min(b.y2) as f32;
    
    if x_right < x_left || y_bottom < y_top {
        return 0.0;
    }
    
    let intersection = (x_right - x_left) * (y_bottom - y_top);
    let area_a = (a.x2 - a.x1) as f32 * (a.y2 - a.y1) as f32;
    let area_b = (b.x2 - b.x1) as f32 * (b.y2 - b.y1) as f32;
    
    intersection / (area_a + area_b - intersection)
}

fn apply_nms(boxes: Vec<FaceRegion>, iou_threshold: f32) -> Result<Vec<FaceRegion>> {
    let mut keep = vec![true; boxes.len()];
    let mut result = Vec::new();
    
    for i in 0..boxes.len() {
        if !keep[i] {
            continue;
        }
        
        result.push(boxes[i].clone());
        
        for j in (i + 1)..boxes.len() {
            if !keep[j] {
                continue;
            }
            
            let iou = calculate_iou(&boxes[i], &boxes[j]);
            if iou > iou_threshold {
                keep[j] = false;
            }
        }
    }
    
    Ok(result)
}
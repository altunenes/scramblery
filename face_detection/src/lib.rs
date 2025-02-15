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
    let model_path = model_path.unwrap_or_else(|| PathBuf::from("models/version-RFB-640.onnx"));
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
    let input_tensor: ndarray::ArrayBase<ndarray::OwnedRepr<f32>, ndarray::Dim<[usize; 4]>> = preprocess_image(image);
    let input_name = &session.inputs[0].name;
    let input_values = std::collections::HashMap::from([(
        input_name.as_str(),
        Value::from_array(input_tensor.view())?
    )]);
    let outputs = session.run(input_values)?;
    let output_names: Vec<_> = session.outputs.iter().map(|o| o.name.as_str()).collect();
    let confidences: ArrayViewD<f32> = outputs[output_names[0]].try_extract_tensor()?;
    let boxes: ArrayViewD<f32> = outputs[output_names[1]].try_extract_tensor()?;
    let num_detections = confidences.shape()[1];
    let (orig_width, orig_height) = image.dimensions();
    let mut regions = Vec::new();
    for i in 0..num_detections {
        let conf = confidences[[0, i, 0]];
        if conf < confidence_threshold {
            continue;
        }
        let mut region = FaceRegion {
            x1: (boxes[[0, i, 0]] * orig_width as f32) as u32,
            y1: (boxes[[0, i, 1]] * orig_height as f32) as u32,
            x2: (boxes[[0, i, 2]] * orig_width as f32) as u32,
            y2: (boxes[[0, i, 3]] * orig_height as f32) as u32,
            confidence: conf,
        };
        if let Some(factor) = expansion_factor {
            region = region.expand(factor, orig_width, orig_height);
        }
        regions.push(region);
    }
    info!("Detected {} face regions above threshold {}", regions.len(), confidence_threshold);
    Ok(regions)
}
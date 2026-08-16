use anyhow::Result;
use image::RgbaImage;
use ndarray::Array4;
use onnx::new_session_from_path;
use onnx::Session;
use onnx::Value;
use log::info;
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::OnceLock;

static OPTICAL_FLOW_SESSION: OnceLock<Mutex<Session>> = OnceLock::new();

pub const FLOW_MODEL_WIDTH: u32 = 512;
pub const FLOW_MODEL_HEIGHT: u32 = 288;

/// Optical flow field in channel-first layout: [2 * H * W] with all flow_x then all flow_y
pub struct FlowField {
    pub width: usize,
    pub height: usize,
    pub data: Vec<f32>, // [2 * H * W] channel-first: all flow_x then all flow_y
}

/// Loads the SEA-RAFT optical flow model, using a cached session if available
pub fn load_optical_flow_model(model_path: Option<PathBuf>) -> Result<&'static Mutex<Session>> {
    if let Some(session) = OPTICAL_FLOW_SESSION.get() {
        return Ok(session);
    }

    let model_path = if let Some(path) = model_path {
        path
    } else {
        #[cfg(debug_assertions)]
        let path = PathBuf::from("resources/models/searaft_flow_512x288.onnx");

        #[cfg(not(debug_assertions))]
        let path = {
            let exe_path = std::env::current_exe().expect("Failed to get executable path");
            info!("Executable path: {:?}", exe_path);

            let resource_path = if cfg!(target_os = "macos") {
                exe_path
                    .parent()
                    .and_then(|p| p.parent())
                    .map(|p| p.join("Resources/models/searaft_flow_512x288.onnx"))
            } else {
                exe_path
                    .parent()
                    .map(|p| p.join("models/searaft_flow_512x288.onnx"))
            }
            .expect("Failed to construct resource path");

            info!("Constructed resource path: {:?}", resource_path);
            info!("Resource path exists: {}", resource_path.exists());

            resource_path
        };

        path
    };

    info!("Loading optical flow model from {:?}", model_path);
    let session = new_session_from_path(model_path)?;
    if OPTICAL_FLOW_SESSION.set(Mutex::new(session)).is_err() {
        info!("Warning: Optical flow session was already initialized by another thread");
    }
    Ok(OPTICAL_FLOW_SESSION.get().unwrap())
}

/// Preprocess an RGBA image to NCHW float32 tensor [1, 3, H, W] with range 0-255.
fn preprocess_for_flow(image: &RgbaImage) -> Array4<f32> {
    let (width, height) = image.dimensions();
    let (w, h) = (width as usize, height as usize);
    let hw = h * w;
    let raw = image.as_raw();

    let mut data = vec![0f32; 3 * hw];
    for i in 0..hw {
        data[i]          = raw[i * 4]     as f32;
        data[hw + i]     = raw[i * 4 + 1] as f32;
        data[2 * hw + i] = raw[i * 4 + 2] as f32;
    }

    Array4::from_shape_vec((1, 3, h, w), data).expect("shape mismatch in preprocess_for_flow")
}

fn resample_flow(flow: &FlowField, dst_w: usize, dst_h: usize) -> FlowField {
    let (src_w, src_h) = (flow.width, flow.height);
    if src_w == dst_w && src_h == dst_h {
        return FlowField { width: src_w, height: src_h, data: flow.data.clone() };
    }

    let scale_x = dst_w as f32 / src_w as f32;
    let scale_y = dst_h as f32 / src_h as f32;
    let src_hw = src_w * src_h;

    let mut data = vec![0f32; 2 * dst_w * dst_h];
    let (out_x, out_y) = data.split_at_mut(dst_w * dst_h);

    // Sample at pixel centres so the mapping stays symmetric at the borders
    let sample = |plane: &[f32], x: usize, y: usize| -> f32 {
        let sx = ((x as f32 + 0.5) / scale_x - 0.5).clamp(0.0, (src_w - 1) as f32);
        let sy = ((y as f32 + 0.5) / scale_y - 0.5).clamp(0.0, (src_h - 1) as f32);
        let (x0, y0) = (sx.floor() as usize, sy.floor() as usize);
        let (x1, y1) = ((x0 + 1).min(src_w - 1), (y0 + 1).min(src_h - 1));
        let (fx, fy) = (sx - x0 as f32, sy - y0 as f32);
        let p00 = plane[y0 * src_w + x0];
        let p10 = plane[y0 * src_w + x1];
        let p01 = plane[y1 * src_w + x0];
        let p11 = plane[y1 * src_w + x1];
        (1.0 - fx) * (1.0 - fy) * p00
            + fx * (1.0 - fy) * p10
            + (1.0 - fx) * fy * p01
            + fx * fy * p11
    };

    out_x
        .par_chunks_mut(dst_w)
        .zip(out_y.par_chunks_mut(dst_w))
        .enumerate()
        .for_each(|(y, (row_x, row_y))| {
            for x in 0..dst_w {
                row_x[x] = sample(&flow.data[..src_hw], x, y) * scale_x;
                row_y[x] = sample(&flow.data[src_hw..], x, y) * scale_y;
            }
        });

    FlowField { width: dst_w, height: dst_h, data }
}

/// Compute optical flow from img1 to img2 using SEA-RAFT.
///
/// Inference runs at the model's fixed size ([`FLOW_MODEL_WIDTH`] x
/// [`FLOW_MODEL_HEIGHT`]); the returned field is scaled back up to the input
/// resolution, so callers always get a full res flow field.
pub fn compute_optical_flow(
    img1: &RgbaImage,
    img2: &RgbaImage,
    session: &Mutex<Session>,
) -> Result<FlowField> {
    let (w1, h1) = img1.dimensions();
    let (w2, h2) = img2.dimensions();
    anyhow::ensure!(
        w1 == w2 && h1 == h2,
        "Image dimensions must match: ({}, {}) vs ({}, {})",
        w1, h1, w2, h2
    );

    // The bundled model has its input shape frozen, so downscale to it. Triangle
    // filtering low passes the frame first, which avoids aliasing motion detail
    // into the estimate.
    let needs_resize = w1 != FLOW_MODEL_WIDTH || h1 != FLOW_MODEL_HEIGHT;
    let (small1, small2);
    let (in1, in2) = if needs_resize {
        small1 = image::imageops::resize(img1, FLOW_MODEL_WIDTH, FLOW_MODEL_HEIGHT, image::imageops::FilterType::Triangle);
        small2 = image::imageops::resize(img2, FLOW_MODEL_WIDTH, FLOW_MODEL_HEIGHT, image::imageops::FilterType::Triangle);
        (&small1, &small2)
    } else {
        (img1, img2)
    };

    let tensor1 = preprocess_for_flow(in1);
    let tensor2 = preprocess_for_flow(in2);

    let mut session = session
        .lock()
        .map_err(|e| anyhow::anyhow!("Session mutex poisoned: {}", e))?;

    let input_names: Vec<String> = session.inputs().iter().map(|i| i.name().to_string()).collect();
    let output_names: Vec<String> = session.outputs().iter().map(|o| o.name().to_string()).collect();

    let input_values = std::collections::HashMap::from([
        (input_names[0].as_str(), Value::from_array(tensor1)?),
        (input_names[1].as_str(), Value::from_array(tensor2)?),
    ]);

    let outputs = session.run(input_values)?;

    // Extract the flow output [1, 2, H, W]
    let flow_key = output_names
        .iter()
        .find(|n| n.contains("flow"))
        .unwrap_or(&output_names[0]);
    let flow_tensor: ndarray::ArrayViewD<f32> = outputs[flow_key.as_str()].try_extract_array()?;
    let flow_shape = flow_tensor.shape();
    info!(
        "Flow output shape: {:?} (model grid [1, 2, {}, {}], target {}x{})",
        flow_shape, FLOW_MODEL_HEIGHT, FLOW_MODEL_WIDTH, w1, h1
    );

    let height = flow_shape[2];
    let width = flow_shape[3];

    // The ONNX output is [1, 2, H, W]
    let data = if let Some(slice) = flow_tensor.as_slice() {
        slice.to_vec()
    } else {
        flow_tensor.iter().copied().collect()
    };

    let flow = FlowField { width, height, data };

    // Bring the field back to the caller's resolution so warping stays full size.
    Ok(resample_flow(&flow, w1 as usize, h1 as usize))
}

/// Warp an image using an optical flow field with bilinear interpolation.
pub fn warp_image(image: &RgbaImage, flow: &FlowField) -> Result<RgbaImage> {
    let (w, h) = image.dimensions();
    anyhow::ensure!(
        flow.width == w as usize && flow.height == h as usize,
        "Flow field dimensions ({}, {}) must match image ({}, {})",
        flow.width, flow.height, w, h
    );

    let width = flow.width;
    let height = flow.height;

    let mut raw = vec![0u8; (w * h * 4) as usize];
    raw.par_chunks_mut(4).enumerate().for_each(|(idx, chunk)| {
        let y = idx / width;
        let x = idx % width;
        let flow_x = flow.data[y * width + x];              // channel 0
        let flow_y = flow.data[height * width + y * width + x]; // channel 1
        let src_x = x as f32 + flow_x;
        let src_y = y as f32 + flow_y;
        let pixel = bilinear_sample(image, src_x, src_y);
        chunk[0] = pixel[0];
        chunk[1] = pixel[1];
        chunk[2] = pixel[2];
        chunk[3] = pixel[3];
    });

    RgbaImage::from_raw(w, h, raw)
        .ok_or_else(|| anyhow::anyhow!("Failed to create warped image from raw buffer"))
}

/// Bilinear interpolation with edge clamping
fn bilinear_sample(image: &RgbaImage, x: f32, y: f32) -> image::Rgba<u8> {
    let (w, h) = image.dimensions();
    let max_x = (w as f32) - 1.0;
    let max_y = (h as f32) - 1.0;

    let x = x.clamp(0.0, max_x);
    let y = y.clamp(0.0, max_y);

    let x0 = x.floor() as u32;
    let y0 = y.floor() as u32;
    let x1 = (x0 + 1).min(w - 1);
    let y1 = (y0 + 1).min(h - 1);

    let fx = x - x0 as f32;
    let fy = y - y0 as f32;

    let p00 = image.get_pixel(x0, y0).0;
    let p10 = image.get_pixel(x1, y0).0;
    let p01 = image.get_pixel(x0, y1).0;
    let p11 = image.get_pixel(x1, y1).0;

    let mut result = [0u8; 4];
    for c in 0..4 {
        let v = (1.0 - fx) * (1.0 - fy) * p00[c] as f32
            + fx * (1.0 - fy) * p10[c] as f32
            + (1.0 - fx) * fy * p01[c] as f32
            + fx * fy * p11[c] as f32;
        result[c] = v.round().clamp(0.0, 255.0) as u8;
    }

    image::Rgba(result)
}


pub fn export_flo_file(flow: &FlowField, path: &Path) -> Result<()> {
    use std::io::Write;

    let mut file = std::fs::File::create(path)?;
    let magic: f32 = 202021.25;
    file.write_all(&magic.to_le_bytes())?;
    file.write_all(&(flow.width as i32).to_le_bytes())?;
    file.write_all(&(flow.height as i32).to_le_bytes())?;

    // Write interleaved flow data (x, y per pixel)
    let hw = flow.height * flow.width;
    for y in 0..flow.height {
        for x in 0..flow.width {
            let flow_x = flow.data[y * flow.width + x];
            let flow_y = flow.data[hw + y * flow.width + x];
            file.write_all(&flow_x.to_le_bytes())?;
            file.write_all(&flow_y.to_le_bytes())?;
        }
    }

    info!("Exported flow file to {:?}", path);
    Ok(())
}

//! Face outlines from MediaPipe Face Mesh V2 landmarks (Apache-2.0, Google).
//!
//! A square crop around a detected face box (1.5 times its larger side, borders extended) is
//! resized to 256 x 256 and passed to the landmark model. Of its 478 landmarks, the 36 that trace
//! the face oval are returned in image coordinates.

use std::sync::{Mutex, OnceLock};

use anyhow::Result;
use image::{imageops::FilterType, DynamicImage, GenericImageView, RgbaImage};
use ndarray::{Array4, ArrayViewD};
use onnx::{new_deterministic_session_from_memory, Session, Value};

use crate::FaceRegion;

static FACE_MESH_MODEL: &[u8] = include_bytes!("../../resources/models/face_mesh_v2.onnx");
static FACE_MESH_SESSION: OnceLock<Mutex<Session>> = OnceLock::new();

const INPUT_SIZE: u32 = 256;
const CROP_SCALE: f32 = 1.5;
const LANDMARKS: usize = 478;

/// Face Mesh landmark indices along the face oval, in order around the face.
pub const FACE_OVAL: [usize; 36] = [
    10, 338, 297, 332, 284, 251, 389, 356, 454, 323, 361, 288, 397, 365, 379, 378, 400, 377, 152, 148, 176, 149, 150,
    136, 172, 58, 132, 93, 234, 127, 162, 21, 54, 103, 67, 109,
];

fn session() -> Result<&'static Mutex<Session>> {
    if let Some(session) = FACE_MESH_SESSION.get() {
        return Ok(session);
    }
    // CPU-only so that outlines, and hence outputs, are reproducible everywhere.
    let session = new_deterministic_session_from_memory(FACE_MESH_MODEL)?;
    let _ = FACE_MESH_SESSION.set(Mutex::new(session));
    Ok(FACE_MESH_SESSION.get().expect("session was just set"))
}

/// The face oval of the face in `region`, as (x, y) points in image coordinates.
pub fn face_outline(image: &DynamicImage, region: &FaceRegion) -> Result<Vec<(f32, f32)>> {
    let (cx, cy) = ((region.x1 + region.x2) as f32 / 2.0, (region.y1 + region.y2) as f32 / 2.0);
    let side = ((region.x2 - region.x1).max(region.y2 - region.y1) as f32 * CROP_SCALE).round().max(1.0);
    let (x0, y0) = (cx - side / 2.0, cy - side / 2.0);
    let (w, h) = (image.width() as f32, image.height() as f32);
    let crop = RgbaImage::from_fn(side as u32, side as u32, |x, y| {
        let sx = (x0 + x as f32).clamp(0.0, w - 1.0) as u32;
        let sy = (y0 + y as f32).clamp(0.0, h - 1.0) as u32;
        image.get_pixel(sx, sy)
    });
    let small = image::imageops::resize(&crop, INPUT_SIZE, INPUT_SIZE, FilterType::Triangle);
    let mut tensor = Array4::<f32>::zeros((1, INPUT_SIZE as usize, INPUT_SIZE as usize, 3));
    for (x, y, p) in small.enumerate_pixels() {
        for c in 0..3 {
            tensor[[0, y as usize, x as usize, c]] = p.0[c] as f32 / 255.0;
        }
    }

    let mut session = session()?.lock().map_err(|e| anyhow::anyhow!("Session mutex poisoned: {e}"))?;
    let input = session.inputs()[0].name().to_string();
    let outputs = session.run(std::collections::HashMap::from([(input.as_str(), Value::from_array(tensor)?)]))?;
    let flat: Vec<f32> = outputs
        .iter()
        .find_map(|(_, v)| {
            let a: ArrayViewD<f32> = v.try_extract_array().ok()?;
            (a.len() == LANDMARKS * 3).then(|| a.iter().copied().collect())
        })
        .ok_or_else(|| anyhow::anyhow!("Face Mesh returned no landmarks"))?;

    let scale = side / INPUT_SIZE as f32;
    Ok(FACE_OVAL.iter().map(|&k| (x0 + flat[3 * k] * scale, y0 + flat[3 * k + 1] * scale)).collect())
}

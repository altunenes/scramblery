use base64::{Engine as _, engine::general_purpose};
use image::ImageFormat;
use std::io::Cursor;
use crate::Result;

/// Processes a base64-encoded image with the given scramble options
/// 
/// Base64 is a way to encode binary data (like images) into text that can be safely
/// transmitted between the frontend and backend. This is necessary because:
/// 1. Web browsers can only handle text data in their APIs
/// 2. JSON (which Tauri uses) can only contain text
/// 
/// The process is:
/// 1. Frontend: Image -> Base64 text
/// 2. Send to backend as text
/// 3. Backend: Base64 text -> Image -> Process -> Base64 text
/// 4. Send back to frontend as text
/// 5. Frontend: Display Base64 text as image
pub fn process_base64_image(
    image_data: &str,
    options: &crate::scramble::ScrambleOptions,
) -> Result<String> {
    let image_bytes = general_purpose::STANDARD
        .decode(image_data)
        .map_err(|e| anyhow::anyhow!("Failed to decode image: {}", e))?;

    let img = image::load_from_memory(&image_bytes)
        .map_err(|e| anyhow::anyhow!("Failed to load image: {}", e))?;

    let scrambled = crate::scramble::scramble_pixels(&img, options)?;

    // Convert the processed image back to binary data (PNG format)
    let mut buffer = Cursor::new(Vec::new());
    scrambled
        .write_to(&mut buffer, ImageFormat::Png)
        .map_err(|e| anyhow::anyhow!("Failed to encode scrambled image: {}", e))?;

    // Convert binary data back to base64 text
    Ok(general_purpose::STANDARD.encode(buffer.into_inner()))
}
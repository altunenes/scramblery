use std::path::{Path, PathBuf};

use anyhow::Context;
use image::DynamicImage;

pub const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "tif", "tiff", "bmp", "gif", "webp"];

pub fn is_image(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| IMAGE_EXTENSIONS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

/// Image files directly inside `dir`, sorted by file name so runs are reproducible.
pub fn list_images(dir: &Path) -> crate::Result<Vec<PathBuf>> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .with_context(|| format!("Cannot read folder {}", dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_file() && is_image(p))
        .collect();
    files.sort();
    Ok(files)
}

pub fn open_image(path: &Path) -> crate::Result<DynamicImage> {
    image::open(path).with_context(|| format!("Cannot open {}", path.display()))
}

/// Saves an image, converting to a layout the target format supports
/// (JPEG and BMP hold 8-bit images without alpha; GIF and WebP hold 8-bit images).
pub fn save_image(image: &DynamicImage, path: &Path) -> crate::Result<()> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    let converted;
    let image = match ext.as_str() {
        "jpg" | "jpeg" | "bmp" => {
            converted = if image.color().channel_count() <= 2 {
                DynamicImage::ImageLuma8(image.to_luma8())
            } else {
                DynamicImage::ImageRgb8(image.to_rgb8())
            };
            &converted
        }
        "gif" | "webp" => {
            converted = DynamicImage::ImageRgba8(image.to_rgba8());
            &converted
        }
        _ => image,
    };
    image.save(path).with_context(|| format!("Cannot save {}", path.display()))
}

/// Writes `value` as pretty-printed JSON.
pub fn write_json<T: serde::Serialize>(value: &T, path: &Path) -> crate::Result<()> {
    let text = serde_json::to_string_pretty(value)?;
    std::fs::write(path, text).with_context(|| format!("Cannot write {}", path.display()))
}

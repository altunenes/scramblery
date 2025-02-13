use image::{DynamicImage, GenericImageView};
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

use super::types::ScrambleOptions;
use crate::Result;

pub fn scramble_pixels(
    image: &DynamicImage,
    options: &ScrambleOptions,
) -> Result<DynamicImage> {
    let (width, height) = image.dimensions();
    let mut scrambled = image.to_rgba8();
    
    let mut rng = if let Some(seed) = options.seed {
        StdRng::seed_from_u64(seed)
    } else {
        StdRng::from_os_rng()
    };
    
    let total_pixels = width * height;
    let pixels_to_scramble = (total_pixels as f32 * options.intensity) as u32;
    
    for _ in 0..pixels_to_scramble {
        let x1 = rng.random_range(0..width);
        let y1 = rng.random_range(0..height);
        let x2 = rng.random_range(0..width);
        let y2 = rng.random_range(0..height);
        
        // Pixel swap using safe array access
        let px1 = scrambled[(x1, y1)];
        let px2 = scrambled[(x2, y2)];
        scrambled.put_pixel(x1, y1, px2);
        scrambled.put_pixel(x2, y2, px1);
    }
    
    Ok(DynamicImage::ImageRgba8(scrambled))
}
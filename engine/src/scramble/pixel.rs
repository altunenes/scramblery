use image::{DynamicImage, GenericImageView, RgbaImage, Rgba};
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use face_detection::{detect_face_regions, load_face_detector, FaceRegion};

use super::types::{ScrambleOptions, BackgroundMode};
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

    if let Some(face_opts) = &options.face_detection {
        // Load face detector and detect faces
        let session = load_face_detector(None)?;
        let face_regions = detect_face_regions(
            image,
            session,
            face_opts.confidence_threshold,
            Some(face_opts.expansion_factor),
        )?;

        match face_opts.background_mode {
            BackgroundMode::Include => {
                // Scramble only face regions, keep background
                for region in face_regions {
                    scramble_region(&mut scrambled, &region, options.intensity, &mut rng);
                }
            },
            BackgroundMode::Exclude => {
                // Create a new image with only scrambled face regions
                let mut new_image = RgbaImage::new(width, height);
                
                for region in face_regions {
                    let region_width = region.x2 - region.x1;
                    let region_height = region.y2 - region.y1;
                    
                    // Create a vector to store the region's pixels
                    let mut region_pixels: Vec<Rgba<u8>> = Vec::with_capacity(
                        (region_width * region_height) as usize
                    );
                    
                    // Copy pixels from the original image
                    for y in region.y1..region.y2 {
                        for x in region.x1..region.x2 {
                            region_pixels.push(*scrambled.get_pixel(x, y));
                        }
                    }
                    
                    // Scramble the pixels in the vector
                    let pixels_to_scramble = (region_pixels.len() as f32 * options.intensity) as usize;
                    for _ in 0..pixels_to_scramble {
                        let idx1 = rng.random_range(0..region_pixels.len());
                        let idx2 = rng.random_range(0..region_pixels.len());
                        region_pixels.swap(idx1, idx2);
                    }
                    
                    // Copy scrambled pixels back
                    let mut pixel_idx = 0;
                    for y in region.y1..region.y2 {
                        for x in region.x1..region.x2 {
                            new_image.put_pixel(x, y, region_pixels[pixel_idx]);
                            pixel_idx += 1;
                        }
                    }
                }
                
                scrambled = new_image;
            }
        }
    } else {
        // Original full image scrambling
        let total_pixels = width * height;
        let pixels_to_scramble = (total_pixels as f32 * options.intensity) as u32;
        
        for _ in 0..pixels_to_scramble {
            let x1 = rng.random_range(0..width);
            let y1 = rng.random_range(0..height);
            let x2 = rng.random_range(0..width);
            let y2 = rng.random_range(0..height);
            
            let px1 = scrambled.get_pixel(x1, y1).clone();
            let px2 = scrambled.get_pixel(x2, y2).clone();
            scrambled.put_pixel(x1, y1, px2);
            scrambled.put_pixel(x2, y2, px1);
        }
    }
    
    Ok(DynamicImage::ImageRgba8(scrambled))
}

fn scramble_region(
    image: &mut RgbaImage,
    region: &FaceRegion,
    intensity: f32,
    rng: &mut StdRng,
) {
    let region_width = region.x2 - region.x1;
    let region_height = region.y2 - region.y1;
    let total_pixels = region_width * region_height;
    let pixels_to_scramble = (total_pixels as f32 * intensity) as u32;

    for _ in 0..pixels_to_scramble {
        let x1 = rng.random_range(region.x1..region.x2);
        let y1 = rng.random_range(region.y1..region.y2);
        let x2 = rng.random_range(region.x1..region.x2);
        let y2 = rng.random_range(region.y1..region.y2);
        
        let px1 = image.get_pixel(x1, y1).clone();
        let px2 = image.get_pixel(x2, y2).clone();
        image.put_pixel(x1, y1, px2);
        image.put_pixel(x2, y2, px1);
    }
}
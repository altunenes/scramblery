use image::{DynamicImage, GenericImageView, RgbaImage};
use image::imageops;
use face_detection::{detect_face_regions, load_face_detector};
use super::types::{BlurOptions, BackgroundMode};
use crate::Result;
use crate::FaceDetectionOptions;

pub struct BlurScrambler {
    options: BlurOptions,
}

impl BlurScrambler {
    pub fn new(options: BlurOptions) -> Self {
        Self {
            options,
        }
    }

    pub fn scramble(&self, image: &DynamicImage) -> Result<DynamicImage> {
        // Convert to RGBA for consistent handling
        let rgba_image = image.to_rgba8();
        
        // Apply Gaussian blur using the image crate's function
        let blurred_rgba = imageops::blur(&rgba_image, self.options.sigma);
        
        Ok(DynamicImage::ImageRgba8(blurred_rgba))
    }

    pub fn scramble_with_face_detection(
        &self,
        image: &DynamicImage,
        face_opts: &FaceDetectionOptions,
    ) -> Result<DynamicImage> {
        let session = load_face_detector(None)?;
        let face_regions = detect_face_regions(
            image,
            &session,
            face_opts.confidence_threshold,
            Some(face_opts.expansion_factor),
        )?;

        let (width, height) = image.dimensions();
        
        match face_opts.background_mode {
            BackgroundMode::Include => {
                // Blur only face regions, keep background intact
                let mut result = image.to_rgba8();
                
                for region in face_regions {
                    let region_width = region.x2 - region.x1;
                    let region_height = region.y2 - region.y1;
                    
                    // Extract the region
                    let sub_image = image.crop_imm(
                        region.x1, region.y1, region_width, region_height
                    );
                    
                    // Blur the region
                    let blurred_sub = imageops::blur(&sub_image, self.options.sigma);
                    
                    // Copy blurred region back to the result
                    for y in 0..region_height {
                        for x in 0..region_width {
                            if x + region.x1 < width && y + region.y1 < height {
                                let pixel = blurred_sub.get_pixel(x, y);
                                result.put_pixel(x + region.x1, y + region.y1, *pixel);
                            }
                        }
                    }
                }
                
                Ok(DynamicImage::ImageRgba8(result))
            },
            BackgroundMode::Exclude => {
                // Output only the blurred face regions
                let mut result = RgbaImage::new(width, height);
                
                for region in face_regions {
                    let region_width = region.x2 - region.x1;
                    let region_height = region.y2 - region.y1;
                    
                    // Extract the region
                    let sub_image = image.crop_imm(
                        region.x1, region.y1, region_width, region_height
                    );
                    
                    // Blur the region
                    let blurred_sub = imageops::blur(&sub_image, self.options.sigma);
                    
                    // Copy blurred region to the result
                    for y in 0..region_height {
                        for x in 0..region_width {
                            if x + region.x1 < width && y + region.y1 < height {
                                let pixel = blurred_sub.get_pixel(x, y);
                                result.put_pixel(x + region.x1, y + region.y1, *pixel);
                            }
                        }
                    }
                }
                
                Ok(DynamicImage::ImageRgba8(result))
            }
        }
    }
}
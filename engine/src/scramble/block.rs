use image::{DynamicImage, GenericImageView, Rgba, RgbaImage};
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use crate::Result;
use super::types::BlockOptions;
use face_detection::{detect_face_regions, load_face_detector};
use crate::FaceDetectionOptions;
use crate::BackgroundMode;

pub struct BlockScrambler {
    options: BlockOptions,
    rng: StdRng,
}

impl BlockScrambler {
    pub fn new(options: BlockOptions, seed: Option<u64>) -> Self {
        let rng = if let Some(seed) = seed {
            StdRng::seed_from_u64(seed)
        } else {
            StdRng::from_os_rng()
        };

        Self {
            options,
            rng,
        }
    }


    pub fn scramble(&mut self, image: &DynamicImage) -> Result<DynamicImage> {
        let img_buffer = image.to_rgba8();
        let (width, height) = img_buffer.dimensions();
        let (block_w, block_h) = self.options.block_size;
        
        // Calculate the number of blocks in each dimension
        let blocks_x = (width + block_w - 1) / block_w;
        let blocks_y = (height + block_h - 1) / block_h;
        let total_blocks = (blocks_x * blocks_y) as usize;
        
        // Create a vector of block indices and shuffle them
        let mut block_indices: Vec<usize> = (0..total_blocks).collect();
        for i in 0..total_blocks {
            let j = self.rng.random_range(0..total_blocks);
            block_indices.swap(i, j);
        }
        
        // Create a new buffer for the scrambled image
        let mut scrambled = RgbaImage::new(width, height);
        
        // Copy blocks to their new positions
        for by in 0..blocks_y {
            for bx in 0..blocks_x {
                let orig_idx = (by * blocks_x + bx) as usize;
                let new_idx = block_indices[orig_idx];
                let new_by = new_idx as u32 / blocks_x;
                let new_bx = new_idx as u32 % blocks_x;
                
                //block boundaries
                let src_x = bx * block_w;
                let src_y = by * block_h;
                let dst_x = new_bx * block_w;
                let dst_y = new_by * block_h;
                
                // Copy the block
                self.copy_block(
                    &img_buffer,
                    &mut scrambled,
                    src_x,
                    src_y,
                    dst_x,
                    dst_y,
                    block_w,
                    block_h,
                    width,
                    height
                );
            }
        }
        
        // Apply edge interpolation if enabled? This currently experimental :(
        if self.options.interpolate_edges {
            self.interpolate_block_edges(&mut scrambled);
        }
        
        Ok(DynamicImage::ImageRgba8(scrambled))
    }

    /// Copies a block from one position to another, handling edge cases
    fn copy_block(
        &self,
        source: &RgbaImage,
        target: &mut RgbaImage,
        src_x: u32,
        src_y: u32,
        dst_x: u32,
        dst_y: u32,
        block_w: u32,
        block_h: u32,
        width: u32,
        height: u32,
    ) {
        for y in 0..block_h {
            if src_y + y >= height || dst_y + y >= height {
                break;
            }
            for x in 0..block_w {
                if src_x + x >= width || dst_x + x >= width {
                    break;
                }
                let pixel = *source.get_pixel(src_x + x, src_y + y);
                target.put_pixel(dst_x + x, dst_y + y, pixel);
            }
        }
    }

    /// Blends two pixels together for smooth transitions
    fn blend_pixels(&self, p1: Rgba<u8>, p2: Rgba<u8>) -> Rgba<u8> {
        let blend_factor = 0.5;
        Rgba([
            ((p1[0] as f32 * (1.0 - blend_factor) + p2[0] as f32 * blend_factor) as u8),
            ((p1[1] as f32 * (1.0 - blend_factor) + p2[1] as f32 * blend_factor) as u8),
            ((p1[2] as f32 * (1.0 - blend_factor) + p2[2] as f32 * blend_factor) as u8),
            ((p1[3] as f32 * (1.0 - blend_factor) + p2[3] as f32 * blend_factor) as u8),
        ])
    }


    pub fn scramble_with_face_detection(
        &mut self,
        image: &DynamicImage,
        face_opts: &FaceDetectionOptions,
    ) -> Result<DynamicImage> {
        let session = load_face_detector(None)?;
        let face_regions = detect_face_regions(
            image,
            session,
            face_opts.confidence_threshold,
            Some(face_opts.expansion_factor),
        )?;

        match face_opts.background_mode {
            BackgroundMode::Include => {
                let mut result = image.to_rgba8();
                for region in face_regions {
                    let region_width = region.x2 - region.x1;
                    let region_height = region.y2 - region.y1;
                    
                    let mut region_img = RgbaImage::new(region_width, region_height);
                    for y in 0..region_height {
                        for x in 0..region_width {
                            let pixel = image.get_pixel(x + region.x1, y + region.y1);
                            region_img.put_pixel(x, y, pixel);
                        }
                    }
                    
                    let mut region_scrambler = BlockScrambler::new(
                        self.options.clone(),
                        None
                    );
                    
                    let processed = region_scrambler.scramble(&DynamicImage::ImageRgba8(region_img))?;
                    let processed_img = processed.to_rgba8();
                    
                    for y in 0..region_height {
                        for x in 0..region_width {
                            let px = processed_img.get_pixel(x, y);
                            result.put_pixel(x + region.x1, y + region.y1, *px);
                        }
                    }
                }
                Ok(DynamicImage::ImageRgba8(result))
            },
            BackgroundMode::Exclude => {
                let (width, height) = image.dimensions();
                let mut result = RgbaImage::new(width, height);
                
                for region in face_regions {
                    let region_width = region.x2 - region.x1;
                    let region_height = region.y2 - region.y1;
                    
                    let mut region_img = RgbaImage::new(region_width, region_height);
                    for y in 0..region_height {
                        for x in 0..region_width {
                            let pixel = image.get_pixel(x + region.x1, y + region.y1);
                            region_img.put_pixel(x, y, pixel);
                        }
                    }
                    
                    let mut region_scrambler = BlockScrambler::new(
                        self.options.clone(),
                        None
                    );
                    
                    let processed = region_scrambler.scramble(&DynamicImage::ImageRgba8(region_img))?;
                    let processed_img = processed.to_rgba8();
                    
                    for y in 0..region_height {
                        for x in 0..region_width {
                            let px = processed_img.get_pixel(x, y);
                            result.put_pixel(x + region.x1, y + region.y1, *px);
                        }
                    }
                }
                Ok(DynamicImage::ImageRgba8(result))
            }
        }
    }

    fn interpolate_block_edges(&self, image: &mut RgbaImage) {
        let (width, height) = image.dimensions();
        let (block_w, block_h) = self.options.block_size;
        
        // Horizontal edges
        for by in 1..((height + block_h - 1) / block_h) {
            let y = by * block_h;
            if y >= height {
                continue;
            }
            
            for x in 0..width {
                let top = *image.get_pixel(x, y - 1);
                let bottom = *image.get_pixel(x, y);
                let blended = self.blend_pixels(top, bottom);
                image.put_pixel(x, y - 1, blended);
            }
        }
        
        // Vertical edges
        for bx in 1..((width + block_w - 1) / block_w) {
            let x = bx * block_w;
            if x >= width {
                continue;
            }
            
            for y in 0..height {
                let left = *image.get_pixel(x - 1, y);
                let right = *image.get_pixel(x, y);
                let blended = self.blend_pixels(left, right);
                image.put_pixel(x - 1, y, blended);
            }
        }
    }
}
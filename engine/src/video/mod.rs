use std::path::PathBuf;
use anyhow::Result;
use video_processor::{VideoProcessor, VideoFrameExt, Writable};
use image::{DynamicImage, RgbaImage};
use serde::{Serialize, Deserialize};

use crate::scramble::ScrambleOptions;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VideoProcessingOptions {
    pub input_path: PathBuf,
    pub output_path: PathBuf,
    pub scramble_options: ScrambleOptions,
}

pub fn process_video(options: &VideoProcessingOptions) -> Result<()> {
    let processor = VideoProcessor::new()?;
    
    let scramble_options = options.scramble_options.clone();
    
    processor.process_video(
        &options.input_path,
        &options.output_path,
        move |frame| {
            let width = frame.width() as u32;
            let height = frame.height() as u32;
            let stride = frame.plane_stride()[0] as usize;
            let data = frame.plane_data(0).unwrap();
            
            let mut image = RgbaImage::new(width, height);
            for y in 0..height {
                for x in 0..width {
                    let offset = (y as usize * stride + x as usize * 4);
                    let pixel = image::Rgba([
                        data[offset],
                        data[offset + 1],
                        data[offset + 2],
                        data[offset + 3],
                    ]);
                    image.put_pixel(x, y, pixel);
                }
            }
            
            let processed = crate::scramble::scramble_pixels(
                &DynamicImage::ImageRgba8(image),
                &scramble_options,
            )?;
            
            let processed = processed.to_rgba8();
            let data = frame.plane_data_mut(0).unwrap();
            
            for y in 0..height {
                for x in 0..width {
                    let pixel = processed.get_pixel(x, y);
                    let offset = (y as usize * stride + x as usize * 4);
                    data[offset] = pixel[0];
                    data[offset + 1] = pixel[1];
                    data[offset + 2] = pixel[2];
                    data[offset + 3] = pixel[3];
                }
            }
            
            Ok(())
        },
    )?;
    
    Ok(())
}
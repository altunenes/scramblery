mod types;
pub use types::*;

use std::path::Path;
use rayon::prelude::*;
use anyhow::Context;
use crate::Result;
use crate::ScrambleType;
use crate::ScrambleOptions;
pub fn process_directory(options: &BatchProcessingOptions) -> Result<Vec<ProcessingResult>> {

    std::fs::create_dir_all(&options.output_dir)?;
    

    let entries: Vec<_> = std::fs::read_dir(&options.input_dir)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            if let Some(ext) = entry.path().extension() {
                matches!(ext.to_str().unwrap_or("").to_lowercase().as_str(),
                    "jpg" | "jpeg" | "png" | "gif" | "webp")
            } else {
                false
            }
        })
        .collect();

    
    let results: Vec<ProcessingResult> = entries.par_iter()
        .map(|entry| {
            let input_path = entry.path();
            let file_name = input_path.file_name().unwrap();
            let output_path = options.output_dir.join(file_name);
            
            match process_single_image(&input_path, &output_path, &options.scramble_options) {
                Ok(_) => ProcessingResult {
                    input_path: input_path.to_owned(),
                    output_path: output_path.to_owned(),
                    success: true,
                    error: None,
                },
                Err(e) => ProcessingResult {
                    input_path: input_path.to_owned(),
                    output_path: output_path.to_owned(),
                    success: false,
                    error: Some(e.to_string()),
                },
            }
        })
        .collect();

    Ok(results)
}

fn process_single_image(
    input_path: &Path,
    output_path: &Path,
    options: &ScrambleOptions,
) -> Result<()> {
    let img = image::open(input_path)
        .with_context(|| format!("Failed to open image: {}", input_path.display()))?;

    let scrambled = match &options.scramble_type {
        ScrambleType::Pixel => crate::scramble::scramble_pixels(&img, options)?,
        ScrambleType::Fourier(fourier_opts) => {
            let mut scrambler = crate::scramble::FourierScrambler::new(
                img.width() as usize,
                img.height() as usize,
                fourier_opts.clone(),
                options.seed,
            );
            
            if let Some(face_opts) = &options.face_detection {
                scrambler.scramble_with_face_detection(&img, face_opts)?
            } else {
                scrambler.scramble(&img)?
            }
        },
        ScrambleType::Block(block_opts) => {
            let mut scrambler = crate::scramble::BlockScrambler::new(
                block_opts.clone(),
                options.seed,
            );
            
            if let Some(face_opts) = &options.face_detection {
                scrambler.scramble_with_face_detection(&img, face_opts)?
            } else {
                scrambler.scramble(&img)?
            }
        },
        ScrambleType::Blur(blur_opts) => {
            let scrambler = crate::scramble::BlurScrambler::new(
                blur_opts.clone(),
            );
            
            if let Some(face_opts) = &options.face_detection {
                scrambler.scramble_with_face_detection(&img, face_opts)?
            } else {
                scrambler.scramble(&img)?
            }
        }
    };

    scrambled.save(output_path)
        .with_context(|| format!("Failed to save image: {}", output_path.display()))?;

    Ok(())
}

pub fn process_directory_with_progress<F>(
    options: &BatchProcessingOptions,
    progress_callback: F,
) -> Result<Vec<ProcessingResult>>
where
    F: ProgressCallback,
{
    std::fs::create_dir_all(&options.output_dir)?;
    let entries: Vec<_> = std::fs::read_dir(&options.input_dir)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            if let Some(ext) = entry.path().extension() {
                matches!(ext.to_str().unwrap_or("").to_lowercase().as_str(),
                    "jpg" | "jpeg" | "png" | "gif" | "webp")
            } else {
                false
            }
        })
        .collect();

    let total_files = entries.len();
    let processed = std::sync::atomic::AtomicUsize::new(0);

    let results: Vec<ProcessingResult> = entries.par_iter()
        .map(|entry| {
            let input_path = entry.path();
            let result = process_single_image(
                &input_path,
                &options.output_dir.join(input_path.file_name().unwrap()),
                &options.scramble_options,
            );

            let current_processed = processed.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
            progress_callback(BatchProgress {
                total_files,
                processed_files: current_processed,
                current_file: Some(input_path.clone()),
            });

            match result {
                Ok(_) => ProcessingResult {
                    input_path: input_path.to_owned(),
                    output_path: options.output_dir.join(input_path.file_name().unwrap()),
                    success: true,
                    error: None,
                },
                Err(e) => ProcessingResult {
                    input_path: input_path.to_owned(),
                    output_path: options.output_dir.join(input_path.file_name().unwrap()),
                    success: false,
                    error: Some(e.to_string()),
                },
            }
        })
        .collect();

    progress_callback(BatchProgress {
        total_files,
        processed_files: total_files,
        current_file: None,
    });

    Ok(results)
}
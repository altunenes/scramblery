use image::{DynamicImage, GenericImageView, ImageBuffer,Rgb};
use ndarray::Array2;
use num_complex::Complex64;
use rustfft::{Fft, FftPlanner};
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use crate::Result;
use super::types::{FourierOptions, FrequencyRange, PaddingMode};
use face_detection::{detect_face_regions, load_face_detector};
use crate::FaceDetectionOptions;
use crate::BackgroundMode;
use image::GenericImage;
pub struct FourierScrambler {
    width: usize,
    height: usize,
    fft: std::sync::Arc<dyn Fft<f64>>,
    ifft: std::sync::Arc<dyn Fft<f64>>,
    options: FourierOptions,
    rng: StdRng,
}


impl FourierScrambler {
    pub fn new(width: usize, height: usize, options: FourierOptions, seed: Option<u64>) -> Self {
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(get_optimal_fft_size(width));
        let ifft = planner.plan_fft_inverse(get_optimal_fft_size(width));
        
        let rng = if let Some(seed) = seed {
            StdRng::seed_from_u64(seed)
        } else {
            StdRng::from_os_rng()
        };

        Self {
            width,
            height,
            fft,
            ifft,
            options,
            rng,
        }
    }

    pub fn scramble(&mut self, image: &DynamicImage) -> Result<DynamicImage> {
        let (width, height) = image.dimensions();
        self.width = width as usize;
        self.height = height as usize;
        
        // Convert image to RGB and split channels
        let channels = self.split_channels(image)?;
        
        // Process each channel
        let processed_channels: Vec<Array2<f64>> = channels
            .into_iter()
            .map(|channel| self.process_channel(channel))
            .collect::<Result<Vec<_>>>()?;
        
        // Combine channels back into image
        self.combine_channels(processed_channels)
    }
    pub fn scramble_with_face_detection(
        &mut self,
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
        let mut result = image.clone();
    
        match face_opts.background_mode {
            BackgroundMode::Include => {
                // Process only face regions
                for region in face_regions {
                    // Create a subimage view
                    let region_width = region.x2 - region.x1;
                    let region_height = region.y2 - region.y1;
                    let mut region_img = DynamicImage::new_rgb8(region_width, region_height);
                    // Copy pixels from original image to region
                    for y in 0..region_height {
                        for x in 0..region_width {
                            let pixel = image.get_pixel(x + region.x1, y + region.y1);
                            region_img.put_pixel(x, y, pixel);
                        }
                    }
                    // Process the region
                    let processed = self.scramble(&region_img)?;
                    // Copy processed region back
                    for y in 0..region_height {
                        for x in 0..region_width {
                            let px = processed.get_pixel(x, y);
                            result.put_pixel(x + region.x1, y + region.y1, px);
                        }
                    }
                }
                Ok(result)
            },
            BackgroundMode::Exclude => {
                // Create a new blank image
                let mut new_image = DynamicImage::new_rgb8(width, height);
                for region in face_regions {
                    let region_width = region.x2 - region.x1;
                    let region_height = region.y2 - region.y1;
                    let mut region_img = DynamicImage::new_rgb8(region_width, region_height);
                    // Copy pixels from original image to region
                    for y in 0..region_height {
                        for x in 0..region_width {
                            let pixel = image.get_pixel(x + region.x1, y + region.y1);
                            region_img.put_pixel(x, y, pixel);
                        }
                    }
                    // Process the region
                    let processed = self.scramble(&region_img)?;
                    // Copy processed region to new image
                    for y in 0..region_height {
                        for x in 0..region_width {
                            let px = processed.get_pixel(x, y);
                            new_image.put_pixel(x + region.x1, y + region.y1, px);
                        }
                    }
                }
                Ok(new_image)
            }
        }
    }
    fn process_channel(&mut self, channel: Array2<f64>) -> Result<Array2<f64>> {
        let padded = self.apply_padding(&channel)?;
        let mut complex_data = self.to_complex(&padded);
        // Perform forward FFT
        self.fft.process(&mut complex_data);
        if self.options.phase_scramble {
            // For phase scrambling, we only scramble phases
            self.scramble_phases(&mut complex_data);
        } else {
            // For other operations, use the original frequency domain processing
            self.scramble_frequency_domain(&mut complex_data);
        }
        // Perform inverse FFT
        self.ifft.process(&mut complex_data);
        // Remove padding and normalize
        let mut result = self.remove_padding(&complex_data, channel.dim())?;
        //output values are normalized
        for val in result.iter_mut() {
            *val = val.max(0.0).min(1.0);
        }

        Ok(result)
    }

    fn scramble_frequency_domain(&mut self, data: &mut [Complex64]) {
        // Apply frequency range filter
        self.apply_frequency_filter(data);
        
        if self.options.phase_scramble {
            self.scramble_phases(data);
        }
        
        if self.options.magnitude_scramble {
            self.scramble_magnitudes(data);
        }
    }
    fn to_complex(&self, real: &Array2<f64>) -> Vec<Complex64> {
        let size = real.len();
        let mut complex = Vec::with_capacity(size);
        
        for &val in real.iter() {
            complex.push(Complex64::new(val, 0.0));
        }
        
        complex
    }
    fn split_channels(&self, image: &DynamicImage) -> Result<Vec<Array2<f64>>> {
        let rgb = image.to_rgb8();
        let (width, height) = (self.width, self.height);
        
        let mut channels = Vec::with_capacity(3);
        for c in 0..3 {
            let mut channel = Array2::zeros((height, width));
            for y in 0..height {
                for x in 0..width {
                    let pixel = rgb.get_pixel(x as u32, y as u32);
                    channel[[y, x]] = pixel[c] as f64 / 255.0;
                }
            }
            channels.push(channel);
        }
        
        Ok(channels)
    }
    fn combine_channels(&self, channels: Vec<Array2<f64>>) -> Result<DynamicImage> {
        let (width, height) = (self.width as u32, self.height as u32);
        let mut image = ImageBuffer::new(width, height);
        
        for y in 0..height {
            for x in 0..width {
                let r = (channels[0][[y as usize, x as usize]] * 255.0) as u8;
                let g = (channels[1][[y as usize, x as usize]] * 255.0) as u8;
                let b = (channels[2][[y as usize, x as usize]] * 255.0) as u8;
                image.put_pixel(x, y, Rgb([r, g, b]));
            }
        }
        
        Ok(DynamicImage::ImageRgb8(image))
    }
    fn apply_padding(&self, channel: &Array2<f64>) -> Result<Array2<f64>> {
        let (height, width) = channel.dim();
        let padded_size = get_optimal_fft_size(width.max(height));
        let mut padded = Array2::zeros((padded_size, padded_size));
        
        match self.options.padding_mode {
            PaddingMode::Zero => {
                // Zero padding is already done by creating zeros array
                for y in 0..height {
                    for x in 0..width {
                        padded[[y, x]] = channel[[y, x]];
                    }
                }
            },
            PaddingMode::Reflect => {
                // Copy original data
                for y in 0..height {
                    for x in 0..width {
                        padded[[y, x]] = channel[[y, x]];
                    }
                }
                
                // Reflect horizontally
                for y in 0..height {
                    for x in width..padded_size {
                        padded[[y, x]] = channel[[y, 2 * width - x - 1]];
                    }
                }
                
                // Reflect vertically
                for y in height..padded_size {
                    for x in 0..padded_size {
                        padded[[y, x]] = padded[[2 * height - y - 1, x]];
                    }
                }
            },
            PaddingMode::Wrap => {
                for y in 0..padded_size {
                    for x in 0..padded_size {
                        padded[[y, x]] = channel[[y % height, x % width]];
                    }
                }
            }
        }
        
        Ok(padded)
    }
    fn remove_padding(&self, complex_data: &[Complex64], original_dim: (usize, usize)) -> Result<Array2<f64>> {
        let (height, width) = original_dim;
        let mut result = Array2::zeros((height, width));
        
        for y in 0..height {
            for x in 0..width {
                let val = complex_data[y * get_optimal_fft_size(width) + x];
                result[[y, x]] = val.re;
            }
        }
        
        Ok(result)
    }
    fn scramble_phases(&mut self, data: &mut [Complex64]) {
        let size = (data.len() as f32).sqrt() as usize;
        
        // Generate random phase matrix
        let random_phases: Vec<f64> = (0..data.len())
            .map(|_| self.rng.gen_range(-std::f64::consts::PI..std::f64::consts::PI))
            .collect();

        // Maintain conjugate symmetry for real-valued output
        for y in 0..size {
            for x in 0..size {
                let idx = y * size + x;
                let magnitude = data[idx].norm();
                
                // Apply random phase while preserving magnitude
                data[idx] = Complex64::from_polar(magnitude, random_phases[idx]);

                // Maintain conjugate symmetry
                if x > 0 && y > 0 {
                    let conj_x = size - x;
                    let conj_y = size - y;
                    if conj_x < size && conj_y < size {
                        let conj_idx = conj_y * size + conj_x;
                        data[conj_idx] = data[idx].conj();
                    }
                }
            }
        }

        // DC component (0,0) remains real
        data[0] = Complex64::from(data[0].norm());
    }

    fn apply_frequency_filter(&self, data: &mut [Complex64]) {
        let size = (data.len() as f32).sqrt() as usize;
        let center = size / 2;
        
        match &self.options.frequency_range {
            FrequencyRange::HighPass(cutoff) => {
                for y in 0..size {
                    for x in 0..size {
                        let dist = frequency_distance(x, y, center);
                        if dist < cutoff * (center as f32) {
                            data[y * size + x] = Complex64::new(0.0, 0.0);
                        }
                    }
                }
            },
            FrequencyRange::LowPass(cutoff) => {
                for y in 0..size {
                    for x in 0..size {
                        let dist = frequency_distance(x, y, center);
                        if dist > cutoff * (center as f32) {
                            data[y * size + x] = Complex64::new(0.0, 0.0);
                        }
                    }
                }
            },
            _ => {}
        }
    }
    fn scramble_magnitudes(&mut self, data: &mut [Complex64]) {
        let mut magnitudes: Vec<f64> = data.iter()
            .map(|c| c.norm())
            .collect();
        
        // Scramble magnitudes
        for i in 0..magnitudes.len() {
            let j = self.rng.gen_range(0..magnitudes.len());
            magnitudes.swap(i, j);
        }
        
        // Apply scrambled magnitudes
        for (value, &magnitude) in data.iter_mut().zip(magnitudes.iter()) {
            let phase = value.arg();
            *value = Complex64::from_polar(magnitude, phase);
        }
    }
}

// Helper functions
fn get_optimal_fft_size(size: usize) -> usize {
    let mut optimal_size = size;
    while !is_power_of_two(optimal_size) {
        optimal_size += 1;
    }
    optimal_size
}

fn is_power_of_two(n: usize) -> bool {
    n != 0 && (n & (n - 1)) == 0
}

fn frequency_distance(x: usize, y: usize, center: usize) -> f32 {
    let dx = (x as isize - center as isize).abs() as f32;
    let dy = (y as isize - center as isize).abs() as f32;
    (dx * dx + dy * dy).sqrt()
}
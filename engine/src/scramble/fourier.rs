use image::{DynamicImage, GenericImageView, ImageBuffer, Rgb};
use ndarray::Array2;
use num_complex::Complex64;
use rustfft::{Fft, FftPlanner};
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use crate::Result;
use super::types::{FourierOptions, PaddingMode};
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
        // Determine the padded size (square) based on the maximum dimension.
        let padded_size = get_optimal_fft_size(width.max(height));
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(padded_size);
        let ifft = planner.plan_fft_inverse(padded_size);
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

    /// Scrambles a single image.
    /// If the `grayscale` option is enabled, the image is first converted to grayscale,
    /// processed as a single luminance channel, and returned as a grayscale image.
    /// Otherwise, each color channel is processed separately.
    pub fn scramble(&mut self, image: &DynamicImage) -> Result<DynamicImage> {
        if self.options.grayscale {
            let gray_image = image.to_luma8();
            let (width, height) = gray_image.dimensions();
            let mut channel = Array2::zeros((height as usize, width as usize));
            for y in 0..(height as usize) {
                for x in 0..(width as usize) {
                    channel[[y, x]] = gray_image.get_pixel(x as u32, y as u32)[0] as f64 / 255.0;
                }
            }
            let processed_channel = self.process_channel(channel)?;
            let mut output = image::GrayImage::new(width, height);
            for y in 0..(height as usize) {
                for x in 0..(width as usize) {
                    let val = (processed_channel[[y, x]] * 255.0).clamp(0.0, 255.0) as u8;
                    output.put_pixel(x as u32, y as u32, image::Luma([val]));
                }
            }
            return Ok(DynamicImage::ImageLuma8(output));
        }

        let (width, height) = image.dimensions();
        self.width = width as usize;
        self.height = height as usize;
        let channels = self.split_channels(image)?;
        let processed_channels: Vec<Array2<f64>> = channels
            .into_iter()
            .map(|channel| self.process_channel(channel))
            .collect::<Result<Vec<_>>>()?;
        self.combine_channels(processed_channels)
    }
    /// Processes face regions separately.
    /// A new scrambler is created for each region so that its FFT plan matches the region size.
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
                for region in face_regions {
                    let region_width = region.x2 - region.x1;
                    let region_height = region.y2 - region.y1;
                    let mut region_img = DynamicImage::new_rgb8(region_width, region_height);
                    for y in 0..region_height {
                        for x in 0..region_width {
                            let pixel = image.get_pixel(x + region.x1, y + region.y1);
                            region_img.put_pixel(x, y, pixel);
                        }
                    }
                    // Create a new scrambler for the region so that FFT planning is correct.
                    let mut region_scrambler =
                    FourierScrambler::new(region_width as usize, region_height as usize, self.options.clone(), None);                    
                    let processed = region_scrambler.scramble(&region_img)?;
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
                let mut new_image = DynamicImage::new_rgb8(width, height);
                for region in face_regions {
                    let region_width = region.x2 - region.x1;
                    let region_height = region.y2 - region.y1;
                    let mut region_img = DynamicImage::new_rgb8(region_width, region_height);
                    for y in 0..region_height {
                        for x in 0..region_width {
                            let pixel = image.get_pixel(x + region.x1, y + region.y1);
                            region_img.put_pixel(x, y, pixel);
                        }
                    }
                    let mut region_scrambler =
                    FourierScrambler::new(region_width as usize, region_height as usize, self.options.clone(), None);
                    let processed = region_scrambler.scramble(&region_img)?;
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
    /// Processes a single channel: pads the image, computes its 2D FFT,
    /// scrambles its phase, then computes the inverse FFT.
    fn process_channel(&mut self, channel: Array2<f64>) -> Result<Array2<f64>> {
        let padded = self.apply_padding(&channel)?;
        let n = padded.dim().0;
        let mut complex_data = self.to_complex(&padded);
        self.fft2d(&mut complex_data, n);
        if self.options.phase_scramble {
            self.phase_scramble(&mut complex_data);
        }
        self.ifft2d(&mut complex_data, n);
        let mut result = self.remove_padding(&complex_data, channel.dim())?;
        for val in result.iter_mut() {
            *val = val.max(0.0).min(1.0);
        }
        Ok(result)
    }

    fn fft2d(&self, data: &mut [Complex64], n: usize) {
        for row in 0..n {
            let start = row * n;
            let end = start + n;
            self.fft.process(&mut data[start..end]);
        }
        let mut column = vec![Complex64::new(0.0, 0.0); n];
        for col in 0..n {
            for row in 0..n {
                column[row] = data[row * n + col];
            }
            self.fft.process(&mut column);
            for row in 0..n {
                data[row * n + col] = column[row];
            }
        }
    }

    fn ifft2d(&self, data: &mut [Complex64], n: usize) {
        for row in 0..n {
            let start = row * n;
            let end = start + n;
            self.ifft.process(&mut data[start..end]);
        }
        let mut column = vec![Complex64::new(0.0, 0.0); n];
        for col in 0..n {
            for row in 0..n {
                column[row] = data[row * n + col];
            }
            self.ifft.process(&mut column);
            for row in 0..n {
                data[row * n + col] = column[row];
            }
        }
        let scale = 1.0 / (n * n) as f64;
        for val in data.iter_mut() {
            *val = *val * scale;
        }
    }

    fn phase_scramble(&mut self, data: &mut [Complex64]) {
        let n = (data.len() as f64).sqrt() as usize;
        for y in 0..n {
            for x in 0..n {
                let sym_y = if y == 0 { 0 } else { n - y };
                let sym_x = if x == 0 { 0 } else { n - x };
                if y > sym_y || (y == sym_y && x > sym_x) {
                    continue;
                }
                let idx = y * n + x;
                let orig = data[idx];
                let mag = orig.norm();
                let orig_phase = orig.arg();
                let random_phase = self.rng.gen_range(0.0..(2.0 * std::f64::consts::PI));
                let dphase = angle_difference(random_phase, orig_phase);
                let new_phase = orig_phase + self.options.intensity as f64 * dphase;
                let new_val = Complex64::from_polar(mag, new_phase);
                data[idx] = new_val;
                if !(y == sym_y && x == sym_x) {
                    let sym_idx = sym_y * n + sym_x;
                    data[sym_idx] = new_val.conj();
                }
            }
        }
    }

    fn to_complex(&self, real: &Array2<f64>) -> Vec<Complex64> {
        real.iter().map(|&val| Complex64::new(val, 0.0)).collect()
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
                for y in 0..height {
                    for x in 0..width {
                        padded[[y, x]] = channel[[y, x]];
                    }
                }
            },
            PaddingMode::Reflect => {
                // Use reflect_index to fill entire padded image.
                for y in 0..padded_size {
                    for x in 0..padded_size {
                        let src_y = reflect_index(y, height);
                        let src_x = reflect_index(x, width);
                        padded[[y, x]] = channel[[src_y, src_x]];
                    }
                }
            },
            PaddingMode::Wrap => {
                for y in 0..padded_size {
                    for x in 0..padded_size {
                        padded[[y, x]] = channel[[y % height, x % width]];
                    }
                }
            },
        }
        Ok(padded)
    }

    fn remove_padding(&self, complex_data: &[Complex64], original_dim: (usize, usize)) -> Result<Array2<f64>> {
        let (height, width) = original_dim;
        let padded_size = get_optimal_fft_size(width.max(height));
        let mut result = Array2::zeros((height, width));
        for y in 0..height {
            for x in 0..width {
                let idx = y * padded_size + x;
                result[[y, x]] = complex_data[idx].re;
            }
        }
        Ok(result)
    }
}

/// Returns the next power of two greater than or equal to `size`.
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

/// Reflects index `x` for an original size `size` using symmetric reflection.
fn reflect_index(x: usize, size: usize) -> usize {
    if size == 0 {
        return 0;
    }
    if size == 1 {
        return 0;
    }
    let period = 2 * size - 2;
    let x_mod = x % period;
    if x_mod < size {
        x_mod
    } else {
        period - x_mod
    }
}

/// Computes the minimal angular difference between two angles (in radians),
/// accounting for wrapping at ±π.
fn angle_difference(a: f64, b: f64) -> f64 {
    let mut diff = a - b;
    while diff > std::f64::consts::PI {
        diff -= 2.0 * std::f64::consts::PI;
    }
    while diff < -std::f64::consts::PI {
        diff += 2.0 * std::f64::consts::PI;
    }
    diff
}

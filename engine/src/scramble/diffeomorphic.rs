// Diffeomorphic scrambling based on Stojanoski & Cusack (2014).
// Code adapted from the original MATLAB implementation https://github.com/rhodricusack/diffeomorph/, MIT License.

use image::{DynamicImage, GenericImageView, RgbaImage}; 
use face_detection::{detect_face_regions, load_face_detector};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use super::types::{DiffeomorphicOptions, BackgroundMode};
use crate::Result;
use crate::FaceDetectionOptions;

pub struct DiffeomorphicScrambler {
    options: DiffeomorphicOptions,
    rng: StdRng,
}

impl DiffeomorphicScrambler {
    pub fn new(options: DiffeomorphicOptions, seed: Option<u64>) -> Self {
        let rng = match seed {
            Some(s) => StdRng::seed_from_u64(s),
            None => StdRng::from_os_rng(),
        };
        Self { options, rng }
    }

    pub fn scramble(&mut self, image: &DynamicImage) -> Result<DynamicImage> {
        let rgba = image.to_rgba8();
        let (width, height) = rgba.dimensions();

        let (warp_x, warp_y) = self.make_warp_field(width as usize, height as usize);
        let result = apply_warp(&rgba, &warp_x, &warp_y);

        Ok(DynamicImage::ImageRgba8(result))
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

        let (width, height) = image.dimensions();

        match face_opts.background_mode {
            BackgroundMode::Include => {
                let mut result = image.to_rgba8();
                for region in face_regions {
                    let rw = region.x2 - region.x1;
                    let rh = region.y2 - region.y1;
                    let sub = image.crop_imm(region.x1, region.y1, rw, rh);
                    let sub_rgba = sub.to_rgba8();

                    let (wx, wy) = self.make_warp_field(rw as usize, rh as usize);
                    let warped = apply_warp(&sub_rgba, &wx, &wy);

                    for y in 0..rh {
                        for x in 0..rw {
                            if x + region.x1 < width && y + region.y1 < height {
                                result.put_pixel(x + region.x1, y + region.y1, *warped.get_pixel(x, y));
                            }
                        }
                    }
                }
                Ok(DynamicImage::ImageRgba8(result))
            }
            BackgroundMode::Exclude => {
                let mut result = RgbaImage::new(width, height);
                for region in face_regions {
                    let rw = region.x2 - region.x1;
                    let rh = region.y2 - region.y1;
                    let sub = image.crop_imm(region.x1, region.y1, rw, rh);
                    let sub_rgba = sub.to_rgba8();

                    let (wx, wy) = self.make_warp_field(rw as usize, rh as usize);
                    let warped = apply_warp(&sub_rgba, &wx, &wy);

                    for y in 0..rh {
                        for x in 0..rw {
                            if x + region.x1 < width && y + region.y1 < height {
                                result.put_pixel(x + region.x1, y + region.y1, *warped.get_pixel(x, y));
                            }
                        }
                    }
                }
                Ok(DynamicImage::ImageRgba8(result))
            }
        }
    }

    /// Generate a diffeomorphic warp field using random DCT components.
    /// Returns (warp_x, warp_y) absolute coordinate fields, each of size width*height.
    /// Matches the getdiffeo function from Stojanoski & Cusack (2014).
    fn make_warp_field(&mut self, width: usize, height: usize) -> (Vec<f32>, Vec<f32>) {
        let n = self.options.n_comp as usize;
        let nsteps = self.options.n_steps.max(1) as usize;
        let max_dist = self.options.max_distortion;
        let two_pi = 2.0 * std::f32::consts::PI;

        // Match MATLAB: 4 independent phase arrays + separate amplitudes for X and Y
        // ph(xc, yc, 1..4) — random phases
        // a(xc, yc) — amplitude for X displacement
        // b(xc, yc) — amplitude for Y displacement
        let mut ph = vec![vec![[0.0f32; 4]; n]; n];
        let mut amp_x = vec![vec![0.0f32; n]; n];
        let mut amp_y = vec![vec![0.0f32; n]; n];

        for i in 0..n {
            for j in 0..n {
                for k in 0..4 {
                    ph[i][j][k] = self.rng.random::<f32>() * two_pi;
                }
                amp_x[i][j] = self.rng.random::<f32>() * two_pi;
                amp_y[i][j] = self.rng.random::<f32>() * two_pi;
            }
        }

        // Evaluate smooth displacement field
        // MATLAB: Xn += a(xc,yc) * cos(xc*XI/imsz*2*pi + ph(xc,yc,1)) * cos(yc*YI/imsz*2*pi + ph(xc,yc,2))
        let mut field_x = vec![0.0f32; width * height];
        let mut field_y = vec![0.0f32; width * height];

        for py in 0..height {
            for px in 0..width {
                let mut dx = 0.0f32;
                let mut dy = 0.0f32;
                // MATLAB uses 1-indexed coords: XI goes 1..imsz
                let coord_x = (px + 1) as f32 / width as f32;
                let coord_y = (py + 1) as f32 / height as f32;

                for i in 0..n {
                    let xc = (i + 1) as f32;
                    for j in 0..n {
                        let yc = (j + 1) as f32;
                        dx += amp_x[i][j]
                            * (xc * coord_x * two_pi + ph[i][j][0]).cos()
                            * (yc * coord_y * two_pi + ph[i][j][1]).cos();
                        dy += amp_y[i][j]
                            * (xc * coord_x * two_pi + ph[i][j][2]).cos()
                            * (yc * coord_y * two_pi + ph[i][j][3]).cos();
                    }
                }
                let idx = py * width + px;
                field_x[idx] = dx;
                field_y[idx] = dy;
            }
        }

        // Normalize X and Y separately to unit RMS, then scale to maxdistortion/nsteps
        // MATLAB: Xn = Xn / sqrt(mean(Xn(:).^2)); XIn = maxdistortion * Xn / nsteps;
        let npix = (width * height) as f32;
        let rms_x = (field_x.iter().map(|v| v * v).sum::<f32>() / npix).sqrt();
        let rms_y = (field_y.iter().map(|v| v * v).sum::<f32>() / npix).sqrt();

        if rms_x > 1e-8 {
            let scale = max_dist / (nsteps as f32 * rms_x);
            for v in field_x.iter_mut() { *v *= scale; }
        }
        if rms_y > 1e-8 {
            let scale = max_dist / (nsteps as f32 * rms_y);
            for v in field_y.iter_mut() { *v *= scale; }
        }

        // field_x/field_y now contain per-step displacement
        // Iteratively compose nsteps small warps to build diffeomorphic mapping

        // Start with identity: warp(px, py) = (px, py)
        let mut warp_x = vec![0.0f32; width * height];
        let mut warp_y = vec![0.0f32; width * height];

        for py in 0..height {
            for px in 0..width {
                let idx = py * width + px;
                warp_x[idx] = px as f32;
                warp_y[idx] = py as f32;
            }
        }

        // Each step: new_warp(p) = old_warp(p + field(p))
        // Equivalent to MATLAB's iterative interp2(image, cy, cx) applied nsteps times
        for _ in 0..nsteps {
            let mut new_warp_x = vec![0.0f32; width * height];
            let mut new_warp_y = vec![0.0f32; width * height];

            for py in 0..height {
                for px in 0..width {
                    let idx = py * width + px;
                    let sx = px as f32 + field_x[idx];
                    let sy = py as f32 + field_y[idx];

                    let (wx, wy) = sample_warp_bilinear(&warp_x, &warp_y, width, height, sx, sy);
                    new_warp_x[idx] = wx;
                    new_warp_y[idx] = wy;
                }
            }

            warp_x = new_warp_x;
            warp_y = new_warp_y;
        }

        (warp_x, warp_y)
    }
}

/// Sample the warp field at a fractional position using bilinear interpolation.
fn sample_warp_bilinear(
    warp_x: &[f32], warp_y: &[f32],
    width: usize, height: usize,
    x: f32, y: f32,
) -> (f32, f32) {
    // Clamp to valid range
    let x = x.clamp(0.0, (width - 1) as f32);
    let y = y.clamp(0.0, (height - 1) as f32);

    let x0 = x.floor() as usize;
    let y0 = y.floor() as usize;
    let x1 = (x0 + 1).min(width - 1);
    let y1 = (y0 + 1).min(height - 1);

    let fx = x - x0 as f32;
    let fy = y - y0 as f32;

    let i00 = y0 * width + x0;
    let i10 = y0 * width + x1;
    let i01 = y1 * width + x0;
    let i11 = y1 * width + x1;

    let wx = warp_x[i00] * (1.0 - fx) * (1.0 - fy)
        + warp_x[i10] * fx * (1.0 - fy)
        + warp_x[i01] * (1.0 - fx) * fy
        + warp_x[i11] * fx * fy;

    let wy = warp_y[i00] * (1.0 - fx) * (1.0 - fy)
        + warp_y[i10] * fx * (1.0 - fy)
        + warp_y[i01] * (1.0 - fx) * fy
        + warp_y[i11] * fx * fy;

    (wx, wy)
}

/// Apply a warp field to an image using bilinear sampling.
fn apply_warp(image: &RgbaImage, warp_x: &[f32], warp_y: &[f32]) -> RgbaImage {
    let (width, height) = image.dimensions();
    let w = width as usize;
    let h = height as usize;
    let mut result = RgbaImage::new(width, height);

    for py in 0..h {
        for px in 0..w {
            let idx = py * w + px;
            let sx = warp_x[idx];
            let sy = warp_y[idx];

            let pixel = bilinear_sample_image(image, sx, sy);
            result.put_pixel(px as u32, py as u32, pixel);
        }
    }

    result
}

/// Bilinear sample a pixel from an RGBA image at fractional coordinates.
fn bilinear_sample_image(image: &RgbaImage, x: f32, y: f32) -> image::Rgba<u8> {
    let (width, height) = image.dimensions();
    let x = x.clamp(0.0, (width - 1) as f32);
    let y = y.clamp(0.0, (height - 1) as f32);

    let x0 = x.floor() as u32;
    let y0 = y.floor() as u32;
    let x1 = (x0 + 1).min(width - 1);
    let y1 = (y0 + 1).min(height - 1);

    let fx = x - x0 as f32;
    let fy = y - y0 as f32;

    let p00 = image.get_pixel(x0, y0).0;
    let p10 = image.get_pixel(x1, y0).0;
    let p01 = image.get_pixel(x0, y1).0;
    let p11 = image.get_pixel(x1, y1).0;

    let mut out = [0u8; 4];
    for c in 0..4 {
        let v = p00[c] as f32 * (1.0 - fx) * (1.0 - fy)
            + p10[c] as f32 * fx * (1.0 - fy)
            + p01[c] as f32 * (1.0 - fx) * fy
            + p11[c] as f32 * fx * fy;
        out[c] = v.round().clamp(0.0, 255.0) as u8;
    }

    image::Rgba(out)
}

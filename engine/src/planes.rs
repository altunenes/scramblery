use image::{DynamicImage, ImageBuffer, Luma, LumaA, Rgb, Rgba};

/// Rec. 709 luma weights, applied to encoded (gamma-corrected) channel values.
pub const LUMA_WEIGHTS: [f32; 3] = [0.2126, 0.7152, 0.0722];

/// sRGB transfer function: encoded value to linear light. Values outside 0..1 are extended
/// symmetrically and with the same power law, so they survive a round trip.
pub fn srgb_to_linear(v: f32) -> f32 {
    let a = v.abs();
    let l = if a <= 0.04045 { a / 12.92 } else { ((a + 0.055) / 1.055).powf(2.4) };
    l.copysign(v)
}

/// Inverse of `srgb_to_linear`.
pub fn linear_to_srgb(v: f32) -> f32 {
    let a = v.abs();
    let e = if a <= 0.0031308 { a * 12.92 } else { 1.055 * a.powf(1.0 / 2.4) - 0.055 };
    e.copysign(v)
}

/// An image as floating-point planes with nominal range 0..1.
///
/// Colour images have three planes (R, G, B), grayscale images one. The bit depth of
/// the source is kept so that results are written back without losing precision.
#[derive(Debug, Clone, PartialEq)]
pub struct Planes {
    pub width: usize,
    pub height: usize,
    pub color: Vec<Vec<f32>>,
    pub alpha: Option<Vec<f32>>,
    pub sixteen_bit: bool,
}

impl Planes {
    pub fn from_image(image: &DynamicImage) -> Self {
        let (width, height) = (image.width() as usize, image.height() as usize);
        let sixteen_bit = matches!(
            image,
            DynamicImage::ImageLuma16(_)
                | DynamicImage::ImageLumaA16(_)
                | DynamicImage::ImageRgb16(_)
                | DynamicImage::ImageRgba16(_)
                | DynamicImage::ImageRgb32F(_)
                | DynamicImage::ImageRgba32F(_)
        );
        let grayscale = matches!(
            image,
            DynamicImage::ImageLuma8(_)
                | DynamicImage::ImageLumaA8(_)
                | DynamicImage::ImageLuma16(_)
                | DynamicImage::ImageLumaA16(_)
        );
        let has_alpha = image.color().has_alpha();
        let rgba = image.to_rgba32f();
        let n = width * height;

        let channels = if grayscale { 1 } else { 3 };
        let mut color = vec![vec![0.0f32; n]; channels];
        let mut alpha = has_alpha.then(|| vec![0.0f32; n]);
        for (i, px) in rgba.pixels().enumerate() {
            for (c, plane) in color.iter_mut().enumerate() {
                plane[i] = px.0[c];
            }
            if let Some(a) = alpha.as_mut() {
                a[i] = px.0[3];
            }
        }
        Self { width, height, color, alpha, sixteen_bit }
    }

    pub fn len(&self) -> usize {
        self.width * self.height
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn is_grayscale(&self) -> bool {
        self.color.len() == 1
    }

    /// Rec. 709 luma of the encoded values.
    pub fn luminance(&self) -> Vec<f32> {
        if self.is_grayscale() {
            return self.color[0].clone();
        }
        (0..self.len())
            .map(|i| {
                LUMA_WEIGHTS[0] * self.color[0][i]
                    + LUMA_WEIGHTS[1] * self.color[1][i]
                    + LUMA_WEIGHTS[2] * self.color[2][i]
            })
            .collect()
    }

    /// A copy with colour values converted from sRGB-encoded to linear light.
    pub fn to_linear(&self) -> Self {
        Self { color: self.color.iter().map(|p| p.iter().map(|&v| srgb_to_linear(v)).collect()).collect(), ..self.clone() }
    }

    /// A copy with colour values converted from linear light to sRGB-encoded.
    pub fn to_encoded(&self) -> Self {
        Self { color: self.color.iter().map(|p| p.iter().map(|&v| linear_to_srgb(v)).collect()).collect(), ..self.clone() }
    }

    /// A grayscale copy holding the luminance, keeping alpha and bit depth.
    pub fn to_grayscale(&self) -> Self {
        Self {
            width: self.width,
            height: self.height,
            color: vec![self.luminance()],
            alpha: self.alpha.clone(),
            sixteen_bit: self.sixteen_bit,
        }
    }

    /// Clamps colour values to 0..1 and returns the fraction of samples that were outside.
    pub fn clip(&mut self) -> f64 {
        let mut clipped = 0usize;
        let mut total = 0usize;
        for plane in &mut self.color {
            for v in plane.iter_mut() {
                if *v < 0.0 || *v > 1.0 {
                    clipped += 1;
                    *v = v.clamp(0.0, 1.0);
                }
            }
            total += plane.len();
        }
        if total == 0 {
            0.0
        } else {
            clipped as f64 / total as f64
        }
    }

    /// Scales each colour channel around its own mean by the largest common factor (at most 1)
    /// that brings every value into 0..1, and returns that factor.
    pub fn compress_to_range(&mut self) -> f64 {
        let means: Vec<f64> = self
            .color
            .iter()
            .map(|p| p.iter().map(|&v| v as f64).sum::<f64>() / p.len().max(1) as f64)
            .collect();
        let mut scale = 1.0f64;
        for (plane, &mean) in self.color.iter().zip(&means) {
            for &v in plane {
                let d = v as f64 - mean;
                if v > 1.0 && d > 0.0 {
                    scale = scale.min((1.0 - mean) / d);
                } else if v < 0.0 && d < 0.0 {
                    scale = scale.min(-mean / d);
                }
            }
        }
        let scale = scale.max(0.0);
        if scale < 1.0 {
            for (plane, &mean) in self.color.iter_mut().zip(&means) {
                for v in plane.iter_mut() {
                    *v = (mean + (*v as f64 - mean) * scale) as f32;
                }
            }
        }
        scale
    }

    pub fn crop(&self, x: usize, y: usize, w: usize, h: usize) -> Self {
        let crop_plane = |plane: &Vec<f32>| {
            let mut out = Vec::with_capacity(w * h);
            for row in y..y + h {
                out.extend_from_slice(&plane[row * self.width + x..row * self.width + x + w]);
            }
            out
        };
        Self {
            width: w,
            height: h,
            color: self.color.iter().map(crop_plane).collect(),
            alpha: self.alpha.as_ref().map(crop_plane),
            sixteen_bit: self.sixteen_bit,
        }
    }

    /// Copies `patch` into this image with its top-left corner at (x, y).
    pub fn paste(&mut self, patch: &Planes, x: usize, y: usize) {
        let width = self.width;
        let paste_plane = |dst: &mut Vec<f32>, src: &Vec<f32>| {
            for row in 0..patch.height {
                let d = (y + row) * width + x;
                dst[d..d + patch.width].copy_from_slice(&src[row * patch.width..(row + 1) * patch.width]);
            }
        };
        for (dst, src) in self.color.iter_mut().zip(&patch.color) {
            paste_plane(dst, src);
        }
        if let (Some(dst), Some(src)) = (self.alpha.as_mut(), patch.alpha.as_ref()) {
            paste_plane(dst, src);
        }
    }

    /// Converts back to an image with the source's channel layout and bit depth.
    /// Values are clamped to 0..1 and rounded to the nearest level.
    pub fn to_image(&self) -> DynamicImage {
        let (w, h) = (self.width as u32, self.height as u32);
        let n = self.len();
        let gray = self.is_grayscale();
        let sample = |plane: &[f32], i: usize| plane[i].clamp(0.0, 1.0);

        if self.sixteen_bit {
            let q = |v: f32| (v * 65535.0).round() as u16;
            match (gray, &self.alpha) {
                (true, None) => DynamicImage::ImageLuma16(ImageBuffer::from_fn(w, h, |x, y| {
                    let i = (y * w + x) as usize;
                    Luma([q(sample(&self.color[0], i))])
                })),
                (true, Some(a)) => DynamicImage::ImageLumaA16(ImageBuffer::from_fn(w, h, |x, y| {
                    let i = (y * w + x) as usize;
                    LumaA([q(sample(&self.color[0], i)), q(sample(a, i))])
                })),
                (false, None) => DynamicImage::ImageRgb16(ImageBuffer::from_fn(w, h, |x, y| {
                    let i = (y * w + x) as usize;
                    Rgb([0, 1, 2].map(|c| q(sample(&self.color[c], i))))
                })),
                (false, Some(a)) => DynamicImage::ImageRgba16(ImageBuffer::from_fn(w, h, |x, y| {
                    let i = (y * w + x) as usize;
                    let [r, g, b] = [0, 1, 2].map(|c| q(sample(&self.color[c], i)));
                    Rgba([r, g, b, q(sample(a, i))])
                })),
            }
        } else {
            let q = |v: f32| (v * 255.0).round() as u8;
            debug_assert!(self.color.iter().all(|p| p.len() == n));
            match (gray, &self.alpha) {
                (true, None) => DynamicImage::ImageLuma8(ImageBuffer::from_fn(w, h, |x, y| {
                    let i = (y * w + x) as usize;
                    Luma([q(sample(&self.color[0], i))])
                })),
                (true, Some(a)) => DynamicImage::ImageLumaA8(ImageBuffer::from_fn(w, h, |x, y| {
                    let i = (y * w + x) as usize;
                    LumaA([q(sample(&self.color[0], i)), q(sample(a, i))])
                })),
                (false, None) => DynamicImage::ImageRgb8(ImageBuffer::from_fn(w, h, |x, y| {
                    let i = (y * w + x) as usize;
                    Rgb([0, 1, 2].map(|c| q(sample(&self.color[c], i))))
                })),
                (false, Some(a)) => DynamicImage::ImageRgba8(ImageBuffer::from_fn(w, h, |x, y| {
                    let i = (y * w + x) as usize;
                    let [r, g, b] = [0, 1, 2].map(|c| q(sample(&self.color[c], i)));
                    Rgba([r, g, b, q(sample(a, i))])
                })),
            }
        }
    }
}

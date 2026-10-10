//! An aperture: the image is shown through a centred circular or elliptical window and the
//! surround is filled with a uniform colour. With the window's normalised radius `d`, the weight
//! `w` follows the same smoothstep rule as face masks (see `mask`), and the output is
//! `w * image + (1 - w) * surround`.

use serde::{Deserialize, Serialize};

use super::mask::weight;
use super::types::MaskShape;
use crate::planes::Planes;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq)]
pub struct ApertureOptions {
    pub shape: ApertureShape,
    /// Window size: the diameter relative to the shorter image side (circle) or to each image
    /// side (ellipse). 1 touches the image edges.
    pub size: f32,
    /// Soft-edge width as a fraction of the window radius.
    pub feather: f32,
    pub surround: Surround,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq)]
pub enum ApertureShape {
    Circle,
    Ellipse,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq)]
pub enum Surround {
    /// Neutral grey at the image's mean luminance.
    MeanLuminance,
    /// The mean of each colour channel.
    MeanColor,
    /// A given grey level (0..1).
    Gray(f32),
}

impl Default for ApertureOptions {
    fn default() -> Self {
        Self { shape: ApertureShape::Circle, size: 0.9, feather: 0.15, surround: Surround::MeanLuminance }
    }
}

/// Window weights for a `width` x `height` image.
pub fn weights(width: usize, height: usize, options: &ApertureOptions) -> Vec<f32> {
    let (cx, cy) = (width as f32 / 2.0, height as f32 / 2.0);
    let (rx, ry) = match options.shape {
        ApertureShape::Circle => {
            let r = options.size * width.min(height) as f32 / 2.0;
            (r, r)
        }
        ApertureShape::Ellipse => (options.size * cx, options.size * cy),
    };
    let (rx, ry) = (rx.max(0.5), ry.max(0.5));
    (0..width * height)
        .map(|i| {
            let u = ((i % width) as f32 + 0.5 - cx) / rx;
            let v = ((i / width) as f32 + 0.5 - cy) / ry;
            weight(MaskShape::Ellipse, options.feather, u, v)
        })
        .collect()
}

/// Applies the aperture in place. The surround statistics are taken from the image as given.
pub fn apply(planes: &mut Planes, options: &ApertureOptions) {
    let n = planes.len().max(1) as f64;
    let surround: Vec<f32> = match options.surround {
        Surround::MeanLuminance => {
            let mean = (planes.luminance().iter().map(|&v| v as f64).sum::<f64>() / n) as f32;
            vec![mean; planes.color.len()]
        }
        Surround::MeanColor => planes.color.iter().map(|p| (p.iter().map(|&v| v as f64).sum::<f64>() / n) as f32).collect(),
        Surround::Gray(level) => vec![level; planes.color.len()],
    };
    let w = weights(planes.width, planes.height, options);
    for (plane, s) in planes.color.iter_mut().zip(surround) {
        for (v, w) in plane.iter_mut().zip(&w) {
            *v = w * *v + (1.0 - w) * s;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aperture_keeps_the_centre_and_fills_the_surround_with_mean_grey() {
        let (w, h) = (40, 30);
        let plane: Vec<f32> = (0..w * h).map(|i| (i % 7) as f32 / 7.0).collect();
        let mut planes = Planes { width: w, height: h, color: vec![plane.clone(), plane.clone(), plane.clone()], alpha: None, sixteen_bit: false };
        let mean = planes.luminance().iter().sum::<f32>() / (w * h) as f32;
        let options = ApertureOptions { size: 0.5, feather: 0.0, ..Default::default() };
        apply(&mut planes, &options);
        let centre = (h / 2) * w + w / 2;
        assert_eq!(planes.color[0][centre], plane[centre]);
        assert!((planes.color[1][0] - mean).abs() < 1e-6);
        assert!((planes.color[2][w * h - 1] - mean).abs() < 1e-6);
    }
}

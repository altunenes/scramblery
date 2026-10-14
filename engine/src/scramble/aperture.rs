//! An aperture: the image is shown through a circular or elliptical window, centred on the image
//! or on each detected face, and the surround is filled with a uniform colour. With the window's
//! normalised radius `d`, the weight `w` follows the same smoothstep rule as face masks (see
//! `mask`), and the output is `w * image + (1 - w) * surround`. With several windows, `w` is the
//! largest of their weights.

use face_detection::FaceRegion;
use serde::{Deserialize, Serialize};

use super::mask::weight;
use super::types::MaskShape;
use crate::planes::Planes;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq)]
pub struct ApertureOptions {
    pub shape: ApertureShape,
    /// Window size. Centred on the image: the diameter relative to the shorter image side
    /// (circle) or to each image side (ellipse), where 1 touches the image edges. Centred on
    /// faces: relative to the face box, where 1 passes through its corners.
    pub size: f32,
    /// Soft-edge width as a fraction of the window radius.
    pub feather: f32,
    pub surround: Surround,
    #[serde(default)]
    pub centre: ApertureCentre,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Default)]
pub enum ApertureCentre {
    #[default]
    Image,
    /// One window on each detected face; where no face is found, only the surround is shown.
    /// Without face detection, the image centre is used.
    Faces,
}

/// A face box in image coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FaceBox {
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
}

impl From<&FaceRegion> for FaceBox {
    fn from(r: &FaceRegion) -> Self {
        Self { x0: r.x1 as f32, y0: r.y1 as f32, x1: r.x2 as f32, y1: r.y2 as f32 }
    }
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
        Self { shape: ApertureShape::Circle, size: 0.9, feather: 0.15, surround: Surround::MeanLuminance, centre: ApertureCentre::Image }
    }
}

/// Window weights for a `width` x `height` image. `faces` are the detected face boxes, or `None`
/// without face detection.
pub fn weights(width: usize, height: usize, options: &ApertureOptions, faces: Option<&[FaceBox]>) -> Vec<f32> {
    // Each window as its centre and radii.
    let windows: Vec<(f32, f32, f32, f32)> = match (options.centre, faces) {
        (ApertureCentre::Faces, Some(faces)) => faces
            .iter()
            .map(|f| {
                let (hw, hh) = ((f.x1 - f.x0) / 2.0, (f.y1 - f.y0) / 2.0);
                let (rx, ry) = match options.shape {
                    ApertureShape::Circle => {
                        let r = options.size * hw.hypot(hh);
                        (r, r)
                    }
                    ApertureShape::Ellipse => (options.size * std::f32::consts::SQRT_2 * hw, options.size * std::f32::consts::SQRT_2 * hh),
                };
                (f.x0 + hw, f.y0 + hh, rx, ry)
            })
            .collect(),
        _ => {
            let (cx, cy) = (width as f32 / 2.0, height as f32 / 2.0);
            let (rx, ry) = match options.shape {
                ApertureShape::Circle => {
                    let r = options.size * width.min(height) as f32 / 2.0;
                    (r, r)
                }
                ApertureShape::Ellipse => (options.size * cx, options.size * cy),
            };
            vec![(cx, cy, rx, ry)]
        }
    };
    (0..width * height)
        .map(|i| {
            let (x, y) = ((i % width) as f32 + 0.5, (i / width) as f32 + 0.5);
            windows
                .iter()
                .map(|&(cx, cy, rx, ry)| weight(MaskShape::Ellipse, options.feather, (x - cx) / rx.max(0.5), (y - cy) / ry.max(0.5)))
                .fold(0.0, f32::max)
        })
        .collect()
}

/// Applies the aperture in place. The surround statistics are taken from the image as given.
pub fn apply(planes: &mut Planes, options: &ApertureOptions, faces: Option<&[FaceBox]>) {
    let n = planes.len().max(1) as f64;
    let surround: Vec<f32> = match options.surround {
        Surround::MeanLuminance => {
            let mean = (planes.luminance().iter().map(|&v| v as f64).sum::<f64>() / n) as f32;
            vec![mean; planes.color.len()]
        }
        Surround::MeanColor => planes.color.iter().map(|p| (p.iter().map(|&v| v as f64).sum::<f64>() / n) as f32).collect(),
        Surround::Gray(level) => vec![level; planes.color.len()],
    };
    let w = weights(planes.width, planes.height, options, faces);
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
        apply(&mut planes, &options, None);
        let centre = (h / 2) * w + w / 2;
        assert_eq!(planes.color[0][centre], plane[centre]);
        assert!((planes.color[1][0] - mean).abs() < 1e-6);
        assert!((planes.color[2][w * h - 1] - mean).abs() < 1e-6);
    }

    #[test]
    fn face_windows_are_centred_on_each_face_and_hide_everything_without_faces() {
        let (w, h) = (100, 60);
        let options = ApertureOptions { size: 1.0, feather: 0.0, centre: ApertureCentre::Faces, ..Default::default() };
        let faces = [FaceBox { x0: 10.0, y0: 10.0, x1: 20.0, y1: 30.0 }, FaceBox { x0: 70.0, y0: 20.0, x1: 90.0, y1: 40.0 }];
        let window = weights(w, h, &options, Some(&faces));
        assert_eq!(window[20 * w + 15], 1.0);
        assert_eq!(window[30 * w + 80], 1.0);
        assert_eq!(window[30 * w + 50], 0.0);
        // At size 1 the circle passes through the box corners.
        assert_eq!(window[11 * w + 11], 1.0);
        assert!(weights(w, h, &options, Some(&[])).iter().all(|&v| v == 0.0));
        assert_eq!(weights(w, h, &options, None)[30 * w + 50], 1.0);
    }
}

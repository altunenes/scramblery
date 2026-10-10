//! Weights of the processed region of a face.
//!
//! With normalised coordinates `u`, `v` in -1..1 across the box, the distance from the centre is
//! `d = max(|u|, |v|)` for a rectangle and `d = sqrt(u² + v²)` for the inscribed ellipse. The
//! weight is 1 for `d <= 1 - f`, 0 for `d >= 1`, and follows the smoothstep curve `t²(3 - 2t)`
//! with `t = (1 - d) / f` in between, where `f` is the feather fraction. For a face outline, the
//! normalised depth inside the polygon is its distance to the nearest edge divided by half the
//! smaller side of the polygon's bounding box, and the same curve applies. The output is
//! `weight * scrambled + (1 - weight) * original`.

use face_detection::FaceRegion;
use image::DynamicImage;

use super::types::{FaceDetectionOptions, MaskShape};

fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Weight at normalised position (u, v) of a box.
pub fn weight(shape: MaskShape, feather: f32, u: f32, v: f32) -> f32 {
    let d = match shape {
        MaskShape::Rectangle => u.abs().max(v.abs()),
        MaskShape::Ellipse | MaskShape::Outline => u.hypot(v),
    };
    if d >= 1.0 {
        return 0.0;
    }
    let f = feather.clamp(0.0, 1.0);
    if f <= 0.0 || d <= 1.0 - f {
        return 1.0;
    }
    let t = (1.0 - d) / f;
    t * t * (3.0 - 2.0 * t)
}

/// Weights for a `width` x `height` box, sampled at pixel centres.
pub fn box_weights(width: usize, height: usize, shape: MaskShape, feather: f32) -> Vec<f32> {
    let (hw, hh) = (width as f32 / 2.0, height as f32 / 2.0);
    (0..width * height)
        .map(|i| {
            let u = ((i % width) as f32 + 0.5 - hw) / hw;
            let v = ((i / width) as f32 + 0.5 - hh) / hh;
            weight(shape, feather, u, v)
        })
        .collect()
}

/// A face region: its bounding rectangle in the image and the weights inside it.
pub struct Region {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
    pub weights: Vec<f32>,
}

/// Weights inside a polygon (image coordinates) over the rectangle at (x0, y0) of size w x h.
pub fn polygon_weights(points: &[(f32, f32)], feather: f32, x0: usize, y0: usize, w: usize, h: usize) -> Vec<f32> {
    let (min_x, max_x) = points.iter().fold((f32::MAX, f32::MIN), |(a, b), p| (a.min(p.0), b.max(p.0)));
    let (min_y, max_y) = points.iter().fold((f32::MAX, f32::MIN), |(a, b), p| (a.min(p.1), b.max(p.1)));
    let half = ((max_x - min_x).min(max_y - min_y) / 2.0).max(1.0);
    let f = feather.clamp(0.0, 1.0);
    let edges: Vec<((f32, f32), (f32, f32))> = (0..points.len()).map(|i| (points[i], points[(i + 1) % points.len()])).collect();
    (0..w * h)
        .map(|i| {
            let (px, py) = ((x0 + i % w) as f32 + 0.5, (y0 + i / w) as f32 + 0.5);
            let mut inside = false;
            let mut dist = f32::MAX;
            for &((ax, ay), (bx, by)) in &edges {
                if (ay > py) != (by > py) && px < ax + (py - ay) / (by - ay) * (bx - ax) {
                    inside = !inside;
                }
                let (ex, ey) = (bx - ax, by - ay);
                let t = (((px - ax) * ex + (py - ay) * ey) / (ex * ex + ey * ey).max(1e-6)).clamp(0.0, 1.0);
                dist = dist.min((px - ax - t * ex).hypot(py - ay - t * ey));
            }
            if !inside {
                0.0
            } else if f <= 0.0 {
                1.0
            } else {
                smoothstep(dist / half / f)
            }
        })
        .collect()
}

/// The processed region of one detected face, clipped to the image. Returns `None` when it
/// falls outside the image.
pub fn face_region(image: &DynamicImage, region: &FaceRegion, options: &FaceDetectionOptions) -> crate::Result<Option<Region>> {
    let (iw, ih) = (image.width() as usize, image.height() as usize);
    let clip = |x0: f32, y0: f32, x1: f32, y1: f32| {
        let (x0, y0) = (x0.floor().max(0.0) as usize, y0.floor().max(0.0) as usize);
        let (x1, y1) = ((x1.ceil().max(0.0) as usize).min(iw), (y1.ceil().max(0.0) as usize).min(ih));
        (x1 > x0 && y1 > y0).then_some((x0, y0, x1 - x0, y1 - y0))
    };
    match options.shape {
        MaskShape::Rectangle | MaskShape::Ellipse => {
            let Some((x, y, width, height)) = clip(region.x1 as f32, region.y1 as f32, region.x2 as f32, region.y2 as f32) else {
                return Ok(None);
            };
            let weights = box_weights(width, height, options.shape, options.feather);
            Ok(Some(Region { x, y, width, height, weights }))
        }
        MaskShape::Outline => {
            let outline = face_detection::outline::face_outline(image, region)?;
            let n = outline.len() as f32;
            let (cx, cy) = outline.iter().fold((0.0, 0.0), |(a, b), p| (a + p.0 / n, b + p.1 / n));
            let scale = options.expansion_factor.max(0.1);
            let points: Vec<(f32, f32)> = outline.iter().map(|&(x, y)| (cx + (x - cx) * scale, cy + (y - cy) * scale)).collect();
            let (x0, x1) = points.iter().fold((f32::MAX, f32::MIN), |(a, b), p| (a.min(p.0), b.max(p.0)));
            let (y0, y1) = points.iter().fold((f32::MAX, f32::MIN), |(a, b), p| (a.min(p.1), b.max(p.1)));
            let Some((x, y, width, height)) = clip(x0, y0, x1, y1) else { return Ok(None) };
            let weights = polygon_weights(&points, options.feather, x, y, width, height);
            Ok(Some(Region { x, y, width, height, weights }))
        }
    }
}

/// Weights for a whole image: the maximum over all detected faces.
pub fn image_weights(image: &DynamicImage, regions: &[FaceRegion], options: &FaceDetectionOptions) -> crate::Result<Vec<f32>> {
    let width = image.width() as usize;
    let mut mask = vec![0.0f32; width * image.height() as usize];
    for r in regions {
        if let Some(region) = face_region(image, r, options)? {
            for y in 0..region.height {
                for x in 0..region.width {
                    let m = &mut mask[(region.y + y) * width + region.x + x];
                    *m = m.max(region.weights[y * region.width + x]);
                }
            }
        }
    }
    Ok(mask)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hard_edges_are_binary_and_soft_edges_fall_off_smoothly() {
        assert_eq!(weight(MaskShape::Rectangle, 0.0, 0.99, 0.99), 1.0);
        assert_eq!(weight(MaskShape::Ellipse, 0.0, 0.8, 0.8), 0.0);
        let profile: Vec<f32> = (0..=20).map(|i| weight(MaskShape::Ellipse, 0.4, i as f32 / 20.0, 0.0)).collect();
        assert_eq!(profile[0], 1.0);
        assert_eq!(profile[12], 1.0);
        assert_eq!(profile[20], 0.0);
        assert!(profile.windows(2).all(|p| p[1] <= p[0]));
        assert!((weight(MaskShape::Ellipse, 0.4, 0.8, 0.0) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn polygon_weights_are_zero_outside_and_rise_towards_the_centre() {
        // A 20 x 20 square polygon at (10, 10) inside a 40 x 40 area.
        let square = [(10.0, 10.0), (30.0, 10.0), (30.0, 30.0), (10.0, 30.0)];
        let hard = polygon_weights(&square, 0.0, 0, 0, 40, 40);
        assert_eq!(hard[5 * 40 + 5], 0.0);
        assert_eq!(hard[20 * 40 + 20], 1.0);
        assert_eq!(hard.iter().filter(|&&w| w == 1.0).count(), 400);
        let soft = polygon_weights(&square, 0.5, 0, 0, 40, 40);
        let row: Vec<f32> = (10..21).map(|x| soft[20 * 40 + x]).collect();
        assert!(row.windows(2).all(|p| p[1] >= p[0]));
        assert!(row[0] < 0.1 && row[10] == 1.0);
    }
}

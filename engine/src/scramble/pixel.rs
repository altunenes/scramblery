use rand::seq::SliceRandom;

use crate::planes::Planes;
use crate::seed;

/// Randomly permutes a fraction `intensity` of the pixel positions among themselves.
/// Every pixel value is kept, so all colour and luminance histograms are preserved exactly.
/// With the same seed, the positions moved at a lower intensity are a subset of those moved
/// at a higher one.
pub fn scramble(input: &Planes, intensity: f32, rng: &mut seed::Rng) -> Planes {
    let n = input.len();
    let k = ((intensity.clamp(0.0, 1.0) as f64) * n as f64).round() as usize;
    let mut order: Vec<usize> = (0..n).collect();
    order.shuffle(rng);
    let positions = order[..k].to_vec();
    let mut targets = positions.clone();
    targets.shuffle(rng);

    let permute = |plane: &Vec<f32>| {
        let mut out = plane.clone();
        for (&from, &to) in positions.iter().zip(&targets) {
            out[to] = plane[from];
        }
        out
    };
    Planes {
        color: input.color.iter().map(permute).collect(),
        alpha: input.alpha.as_ref().map(permute),
        ..input.clone()
    }
}

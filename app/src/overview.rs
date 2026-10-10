//! Points for the set overview: mean luminance against RMS contrast, before and after.

use crate::ScatterPoint;

pub struct Overview {
    pub points: Vec<ScatterPoint>,
    pub x_range: (String, String),
    pub y_range: (String, String),
    /// Half-range of mean luminance and RMS contrast over the "after" points.
    pub spread_after: (f64, f64),
    pub spread_before: (f64, f64),
}

fn half_range(values: impl Iterator<Item = f64>) -> f64 {
    let (lo, hi) = values.fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| (lo.min(v), hi.max(v)));
    if lo.is_finite() { (hi - lo) / 2.0 } else { 0.0 }
}

/// `before` and `after` hold (mean luminance, RMS contrast) pairs.
pub fn build(before: &[(f64, f64)], after: &[(f64, f64)]) -> Overview {
    let all = before.iter().chain(after);
    let (mut x0, mut x1, mut y0, mut y1) = (f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::NEG_INFINITY);
    for &(x, y) in all {
        x0 = x0.min(x);
        x1 = x1.max(x);
        y0 = y0.min(y);
        y1 = y1.max(y);
    }
    // Pad so that points do not sit on the frame, and keep a minimum span.
    let pad = |lo: f64, hi: f64| {
        let span = (hi - lo).max(0.02);
        let mid = (lo + hi) / 2.0;
        ((mid - span * 0.55).max(0.0), (mid + span * 0.55).min(1.0))
    };
    let (x0, x1) = pad(x0, x1);
    let (y0, y1) = pad(y0, y1);
    let point = |&(x, y): &(f64, f64), after: bool| ScatterPoint {
        x: ((x - x0) / (x1 - x0)) as f32,
        y: ((y - y0) / (y1 - y0)) as f32,
        after,
    };
    let points = before.iter().map(|p| point(p, false)).chain(after.iter().map(|p| point(p, true))).collect();
    Overview {
        points,
        x_range: (format!("{x0:.2}"), format!("{x1:.2}")),
        y_range: (format!("{y0:.2}"), format!("{y1:.2}")),
        spread_before: (half_range(before.iter().map(|p| p.0)), half_range(before.iter().map(|p| p.1))),
        spread_after: (half_range(after.iter().map(|p| p.0)), half_range(after.iter().map(|p| p.1))),
    }
}

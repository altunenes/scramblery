//! Chart geometry for the analysis view, as SVG path data in a 1000 x 300 view box.

use engine::analysis::ImageStats;
use engine::FrequencyUnit;

pub const VIEW_W: f64 = 1000.0;
pub const VIEW_H: f64 = 300.0;
const MARGIN: f64 = 8.0;

pub struct Tick {
    pub position: f32,
    pub label: String,
}

fn y_of(t: f64) -> f64 {
    VIEW_H - MARGIN - t.clamp(0.0, 1.0) * (VIEW_H - 2.0 * MARGIN)
}

fn polyline(points: impl Iterator<Item = (f64, f64)>) -> String {
    let mut d = String::new();
    for (i, (x, y)) in points.enumerate() {
        d.push_str(if i == 0 { "M " } else { " L " });
        d.push_str(&format!("{x:.1} {y:.1}"));
    }
    if d.is_empty() {
        d.push_str("M 0 0");
    }
    d
}

/// Histograms of two images on a shared vertical scale: the first as a filled area,
/// the second as an outline.
pub fn histograms(a: &ImageStats, b: &ImageStats) -> (String, String) {
    let max = a.histogram.iter().chain(&b.histogram).copied().max().unwrap_or(1).max(1) as f64;
    let line = |s: &ImageStats| {
        let n = s.histogram.len().max(2) as f64 - 1.0;
        polyline(s.histogram.iter().enumerate().map(|(i, &c)| (i as f64 / n * VIEW_W, y_of(c as f64 / max))))
    };
    let base = y_of(0.0);
    (format!("{} L {VIEW_W} {base:.1} L 0 {base:.1} Z", line(a)), line(b))
}

/// Log-log amplitude spectra of two images on shared axes, with frequency ticks in `unit`.
pub fn spectra(a: &ImageStats, b: &ImageStats, unit: FrequencyUnit) -> (String, String, Vec<Tick>) {
    let f_min = a.spectrum_frequencies.first().copied().unwrap_or(0.01).max(1e-4);
    let (lx0, lx1) = (f_min.log10(), 0.5f64.log10());
    let amplitudes = a.spectrum_amplitudes.iter().chain(&b.spectrum_amplitudes).filter(|v| **v > 0.0);
    let (ly0, ly1) = amplitudes.fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| (lo.min(v.log10()), hi.max(v.log10())));
    let span = (ly1 - ly0).max(1e-9);

    let path = |s: &ImageStats| {
        polyline(
            s.spectrum_frequencies
                .iter()
                .zip(&s.spectrum_amplitudes)
                .filter(|(_, a)| **a > 0.0)
                .map(|(f, v)| ((f.log10() - lx0) / (lx1 - lx0) * VIEW_W, y_of((v.log10() - ly0) / span))),
        )
    };

    let mut ticks = Vec::new();
    for exponent in -4..=4 {
        for mantissa in [1.0, 2.0, 5.0] {
            let value = mantissa * 10f64.powi(exponent);
            let cpp = unit.to_cycles_per_pixel(value as f32, a.width);
            if cpp >= f_min && cpp <= 0.5 {
                ticks.push(Tick {
                    position: ((cpp.log10() - lx0) / (lx1 - lx0)) as f32,
                    label: format_number(value),
                });
            }
        }
    }
    (path(a), path(b), ticks)
}

fn format_number(v: f64) -> String {
    let s = format!("{v:.4}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

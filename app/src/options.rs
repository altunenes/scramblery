use engine::aperture::{ApertureCentre, ApertureOptions, ApertureShape, Surround};
use engine::matching::{MatchOptions, MatchProperty, MatchTarget};
use engine::{
    BackgroundMode, BlockOptions, BlurOptions, DiffeomorphicOptions, FaceDetectionOptions,
    FourierOptions, LocalPhaseOptions, FrequencyRange, FrequencyUnit, MaskShape, PaddingMode, RangeHandling, ScrambleOptions, ScrambleType,
};
use slint::ComponentHandle;

use crate::{AppWindow, Params};

pub fn frequency_unit(ui: &AppWindow) -> FrequencyUnit {
    let p = ui.global::<Params>();
    match p.get_frequency_unit() {
        1 => FrequencyUnit::CyclesPerImage,
        2 => FrequencyUnit::CyclesPerDegree { pixels_per_degree: p.get_pixels_per_degree() },
        _ => FrequencyUnit::CyclesPerPixel,
    }
}

fn padding_mode(index: i32) -> PaddingMode {
    match index {
        1 => PaddingMode::Reflect,
        2 => PaddingMode::Mean,
        _ => PaddingMode::Wrap,
    }
}

fn padding_index(mode: PaddingMode) -> i32 {
    match mode {
        PaddingMode::Wrap => 0,
        PaddingMode::Reflect => 1,
        PaddingMode::Mean => 2,
    }
}

/// The fixed seed, if one is set and valid.
pub fn fixed_seed(ui: &AppWindow) -> Option<u64> {
    let p = ui.global::<Params>();
    if p.get_fixed_seed() {
        p.get_seed_text().trim().parse().ok()
    } else {
        None
    }
}

pub fn scramble_options(ui: &AppWindow) -> ScrambleOptions {
    let p = ui.global::<Params>();
    let intensity = p.get_intensity() / 100.0;

    let scramble_type = match p.get_method() {
        1 => ScrambleType::Fourier(FourierOptions {
            intensity,
            frequency_range: match p.get_frequency_range() {
                1 => FrequencyRange::LowPass(p.get_cutoff()),
                2 => FrequencyRange::HighPass(p.get_cutoff()),
                3 => FrequencyRange::BandPass { low: p.get_band_low(), high: p.get_band_high() },
                _ => FrequencyRange::All,
            },
            unit: frequency_unit(ui),
            remove_outside_range: p.get_remove_outside_range(),
            phase_scramble: p.get_phase_scramble(),
            padding_mode: padding_mode(p.get_fourier_padding()),
            grayscale: p.get_grayscale(),
        }),
        2 => ScrambleType::Block(BlockOptions {
            block_size: (p.get_block_width().round().max(1.0) as u32, p.get_block_height().round().max(1.0) as u32),
            interpolate_edges: p.get_interpolate_edges(),
        }),
        3 => ScrambleType::Blur(BlurOptions { sigma: p.get_blur_sigma() }),
        4 => ScrambleType::Diffeomorphic(DiffeomorphicOptions {
            max_distortion: p.get_max_distortion(),
            n_steps: p.get_warp_steps().round() as u32,
            n_comp: p.get_dct_components().round() as u32,
        }),
        5 => ScrambleType::LocalPhase(LocalPhaseOptions {
            intensity,
            scales: p.get_local_scales().round().max(1.0) as u32,
            orientations: p.get_local_orientations().round().max(1.0) as u32,
            padding_mode: padding_mode(p.get_local_padding()),
        }),
        _ => ScrambleType::Pixel,
    };

    let face_detection = p.get_face_detection().then(|| FaceDetectionOptions {
        background_mode: if p.get_background_mode() == 1 {
            BackgroundMode::Exclude
        } else {
            BackgroundMode::Include
        },
        shape: match p.get_face_shape() {
            1 => MaskShape::Ellipse,
            2 => MaskShape::Outline,
            _ => MaskShape::Rectangle,
        },
        feather: p.get_face_feather() / 100.0,
        expansion_factor: p.get_face_size() / 100.0,
        ..Default::default()
    });

    let aperture_on_faces = face_detection.is_some() && p.get_aperture_centre() == 1;
    ScrambleOptions {
        scramble_type,
        intensity,
        seed: fixed_seed(ui),
        face_detection,
        matching: match_options(ui),
        aperture: p.get_aperture().then(|| ApertureOptions {
            shape: if p.get_aperture_shape() == 1 { ApertureShape::Ellipse } else { ApertureShape::Circle },
            size: if aperture_on_faces { p.get_aperture_face_size() } else { p.get_aperture_size() } / 100.0,
            feather: p.get_aperture_feather() / 100.0,
            surround: match p.get_aperture_surround() {
                1 => Surround::MeanColor,
                2 => Surround::Gray(p.get_aperture_gray()),
                _ => Surround::MeanLuminance,
            },
            centre: if aperture_on_faces { ApertureCentre::Faces } else { ApertureCentre::Image },
        }),
        linear_light: p.get_linear_light(),
        out_of_range: if p.get_out_of_range() == 1 { RangeHandling::CompressContrast } else { RangeHandling::Clip },
    }
}

pub fn match_options(ui: &AppWindow) -> Option<MatchOptions> {
    let p = ui.global::<Params>();
    let property = match p.get_match_property() {
        1 => MatchProperty::Luminance,
        2 => MatchProperty::Histogram,
        3 => MatchProperty::Spectrum,
        _ => return None,
    };
    let target = match p.get_match_target() {
        1 => MatchTarget::Fixed { mean: p.get_match_mean() as f64, rms_contrast: p.get_match_contrast() as f64 },
        2 => MatchTarget::SetAverage,
        _ => MatchTarget::Original,
    };
    Some(MatchOptions { property, target })
}

/// Sets the controls from `options`, e.g. when a recipe is loaded.
pub fn apply_options(ui: &AppWindow, options: &ScrambleOptions) {
    let p = ui.global::<Params>();
    p.set_intensity(options.intensity * 100.0);
    match &options.scramble_type {
        ScrambleType::Pixel => p.set_method(0),
        ScrambleType::Fourier(o) => {
            p.set_method(1);
            p.set_intensity(o.intensity * 100.0);
            p.set_frequency_unit(match o.unit {
                FrequencyUnit::CyclesPerPixel => 0,
                FrequencyUnit::CyclesPerImage => 1,
                FrequencyUnit::CyclesPerDegree { pixels_per_degree } => {
                    p.set_pixels_per_degree(pixels_per_degree);
                    2
                }
            });
            // Set after the unit, whose change handler resets the cutoffs.
            match o.frequency_range {
                FrequencyRange::All => p.set_frequency_range(0),
                FrequencyRange::LowPass(c) => {
                    p.set_frequency_range(1);
                    p.set_cutoff(c);
                }
                FrequencyRange::HighPass(c) => {
                    p.set_frequency_range(2);
                    p.set_cutoff(c);
                }
                FrequencyRange::BandPass { low, high } => {
                    p.set_frequency_range(3);
                    p.set_band_low(low);
                    p.set_band_high(high);
                }
            }
            p.set_remove_outside_range(o.remove_outside_range);
            p.set_phase_scramble(o.phase_scramble);
            p.set_fourier_padding(padding_index(o.padding_mode));
            p.set_grayscale(o.grayscale);
        }
        ScrambleType::Block(o) => {
            p.set_method(2);
            p.set_block_width(o.block_size.0 as f32);
            p.set_block_height(o.block_size.1 as f32);
            p.set_interpolate_edges(o.interpolate_edges);
        }
        ScrambleType::Blur(o) => {
            p.set_method(3);
            p.set_blur_sigma(o.sigma);
        }
        ScrambleType::Diffeomorphic(o) => {
            p.set_method(4);
            p.set_max_distortion(o.max_distortion);
            p.set_warp_steps(o.n_steps as f32);
            p.set_dct_components(o.n_comp as f32);
        }
        ScrambleType::LocalPhase(o) => {
            p.set_method(5);
            p.set_intensity(o.intensity * 100.0);
            p.set_local_scales(o.scales as f32);
            p.set_local_orientations(o.orientations as f32);
            p.set_local_padding(padding_index(o.padding_mode));
        }
    }
    p.set_face_detection(options.face_detection.is_some());
    if let Some(f) = &options.face_detection {
        p.set_background_mode(if f.background_mode == BackgroundMode::Exclude { 1 } else { 0 });
        p.set_face_shape(match f.shape {
            MaskShape::Rectangle => 0,
            MaskShape::Ellipse => 1,
            MaskShape::Outline => 2,
        });
        p.set_face_feather(f.feather * 100.0);
        p.set_face_size(f.expansion_factor * 100.0);
    }
    match options.matching {
        None => p.set_match_property(0),
        Some(m) => {
            p.set_match_property(match m.property {
                MatchProperty::Luminance => 1,
                MatchProperty::Histogram => 2,
                MatchProperty::Spectrum => 3,
            });
            p.set_match_target(match m.target {
                MatchTarget::Original => 0,
                MatchTarget::Fixed { mean, rms_contrast } => {
                    p.set_match_mean(mean as f32);
                    p.set_match_contrast(rms_contrast as f32);
                    1
                }
                MatchTarget::SetAverage => 2,
            });
        }
    }
    p.set_aperture(options.aperture.is_some());
    if let Some(a) = &options.aperture {
        p.set_aperture_shape(if a.shape == ApertureShape::Ellipse { 1 } else { 0 });
        if a.centre == ApertureCentre::Faces {
            p.set_aperture_centre(1);
            p.set_aperture_face_size(a.size * 100.0);
        } else {
            p.set_aperture_centre(if options.face_detection.is_some() { 0 } else { 1 });
            p.set_aperture_size(a.size * 100.0);
        }
        p.set_aperture_feather(a.feather * 100.0);
        p.set_aperture_surround(match a.surround {
            Surround::MeanLuminance => 0,
            Surround::MeanColor => 1,
            Surround::Gray(level) => {
                p.set_aperture_gray(level);
                2
            }
        });
    }
    p.set_linear_light(options.linear_light);
    p.set_out_of_range(if options.out_of_range == RangeHandling::CompressContrast { 1 } else { 0 });
    p.set_fixed_seed(options.seed.is_some());
    if let Some(seed) = options.seed {
        p.set_seed_text(seed.to_string().into());
    }
}

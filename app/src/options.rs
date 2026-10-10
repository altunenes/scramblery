use engine::{
    BackgroundMode, BlockOptions, BlurOptions, DiffeomorphicOptions, FaceDetectionOptions,
    FourierOptions, FrequencyRange, PaddingMode, ScrambleOptions, ScrambleType,
};
use slint::ComponentHandle;

use crate::{AppWindow, Params};

fn padding_mode(index: i32) -> PaddingMode {
    match index {
        0 => PaddingMode::Zero,
        2 => PaddingMode::Wrap,
        _ => PaddingMode::Reflect,
    }
}

pub fn scramble_options(ui: &AppWindow) -> ScrambleOptions {
    let p = ui.global::<Params>();
    let intensity = p.get_intensity() / 100.0;

    let scramble_type = match p.get_method() {
        1 => ScrambleType::Fourier(FourierOptions {
            frequency_range: match p.get_frequency_range() {
                1 => FrequencyRange::LowPass(p.get_cutoff()),
                2 => FrequencyRange::HighPass(p.get_cutoff()),
                3 => FrequencyRange::BandPass { low: p.get_band_low(), high: p.get_band_high() },
                _ => FrequencyRange::All,
            },
            phase_scramble: p.get_phase_scramble(),
            magnitude_scramble: false,
            padding_mode: padding_mode(p.get_fourier_padding()),
            intensity,
            grayscale: p.get_grayscale(),
        }),
        2 => ScrambleType::Block(BlockOptions {
            block_size: (p.get_block_width().round().max(1.0) as u32, p.get_block_height().round().max(1.0) as u32),
            interpolate_edges: p.get_interpolate_edges(),
            padding_mode: padding_mode(p.get_block_padding()),
        }),
        3 => ScrambleType::Blur(BlurOptions { sigma: p.get_blur_sigma() }),
        4 => ScrambleType::Diffeomorphic(DiffeomorphicOptions {
            max_distortion: p.get_max_distortion(),
            n_steps: p.get_warp_steps().round() as u32,
            n_comp: p.get_dct_components().round() as u32,
        }),
        _ => ScrambleType::Pixel,
    };

    let face_detection = p.get_face_detection().then(|| FaceDetectionOptions {
        background_mode: if p.get_background_mode() == 1 {
            BackgroundMode::Exclude
        } else {
            BackgroundMode::Include
        },
        ..Default::default()
    });

    ScrambleOptions { scramble_type, intensity, seed: None, face_detection }
}

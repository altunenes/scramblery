//! Checks the properties each method is documented to preserve.

use engine::analysis::{analyze_planes, mean_sd, ssim_planes};
use engine::normalize::{normalize, NormalizeMethod};
use engine::planes::Planes;
use engine::spectral::{fft2, radial_frequency, to_complex};
use engine::*;
use image::{DynamicImage, ImageBuffer, Rgb};

const W: usize = 67;
const H: usize = 45;

/// Deterministic test image with structure and noise, kept inside 0.2..0.8 to avoid clipping.
fn test_planes(width: usize, height: usize, variant: u32) -> Planes {
    let mut state = 0x1234_5678u32 ^ variant.wrapping_mul(0x9E37_79B9);
    let mut noise = move || {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        (state as f32 / u32::MAX as f32) - 0.5
    };
    let color = (0..3)
        .map(|c| {
            (0..width * height)
                .map(|i| {
                    let (x, y) = ((i % width) as f32, (i / width) as f32);
                    let structure = 0.15 * ((x * 0.21 + c as f32).sin() * (y * 0.13).cos());
                    0.5 + structure + 0.1 * noise()
                })
                .collect()
        })
        .collect();
    Planes { width, height, color, alpha: None, sixteen_bit: false }
}

fn test_image(width: usize, height: usize) -> DynamicImage {
    test_planes(width, height, 0).to_image()
}

fn spectrum(plane: &[f32], width: usize, height: usize) -> Vec<num_complex::Complex64> {
    let mut data = to_complex(plane);
    fft2(&mut data, width, height, false);
    data
}

fn fourier(options: FourierOptions) -> ScrambleOptions {
    ScrambleOptions { scramble_type: ScrambleType::Fourier(options), ..Default::default() }
}

fn sorted_samples(image: &DynamicImage) -> Vec<Vec<u8>> {
    let rgb = image.to_rgb8();
    (0..3)
        .map(|c| {
            let mut v: Vec<u8> = rgb.pixels().map(|p| p.0[c]).collect();
            v.sort_unstable();
            v
        })
        .collect()
}

#[test]
fn pixel_scrambling_preserves_histograms_exactly() {
    let image = test_image(W, H);
    let options = ScrambleOptions { intensity: 1.0, seed: Some(1), ..Default::default() };
    let out = scramble_image(&image, &options).unwrap();
    assert_eq!(sorted_samples(&image), sorted_samples(&out.image));
    assert_ne!(image.to_rgb8(), out.image.to_rgb8());
}

#[test]
fn pixel_intensity_is_fraction_of_pixels_moved() {
    // Distinct values so that every moved pixel is detectable.
    let n = 100 * 100;
    let planes = Planes {
        width: 100,
        height: 100,
        color: vec![(0..n).map(|i| i as f32 / n as f32).collect()],
        alpha: None,
        sixteen_bit: false,
    };
    let options = ScrambleOptions { intensity: 0.5, ..Default::default() };
    let out = scramble_planes(&planes, &options, 7);
    let moved = planes.color[0].iter().zip(&out.color[0]).filter(|(a, b)| a != b).count();
    let fraction = moved as f64 / n as f64;
    assert!((0.49..=0.5).contains(&fraction), "moved fraction {fraction}");
}

#[test]
fn zero_intensity_returns_the_original() {
    let image = test_image(W, H);
    for options in [
        ScrambleOptions { intensity: 0.0, ..Default::default() },
        fourier(FourierOptions { intensity: 0.0, ..Default::default() }),
    ] {
        let out = scramble_image(&image, &options).unwrap();
        assert_eq!(image.to_rgb8(), out.image.to_rgb8());
    }
}

#[test]
fn phase_scrambling_preserves_amplitude_spectrum_and_mean() {
    let planes = test_planes(W, H, 0);
    let out = scramble_planes(&planes, &fourier(FourierOptions::default()), 3);
    for (a, b) in planes.color.iter().zip(&out.color) {
        let (sa, sb) = (spectrum(a, W, H), spectrum(b, W, H));
        for (x, y) in sa.iter().zip(&sb) {
            assert!((x.norm() - y.norm()).abs() < 1e-3, "{} vs {}", x.norm(), y.norm());
        }
        assert!((mean_sd(a).0 - mean_sd(b).0).abs() < 1e-6);
    }
    assert!(ssim_planes(&planes, &out).unwrap() < 0.5);
}

#[test]
fn phase_scrambling_keeps_gray_images_gray() {
    let mut planes = test_planes(W, H, 0);
    planes.color[1] = planes.color[0].clone();
    planes.color[2] = planes.color[0].clone();
    let out = scramble_planes(&planes, &fourier(FourierOptions::default()), 11);
    for i in 0..planes.len() {
        assert!((out.color[0][i] - out.color[1][i]).abs() < 1e-5);
        assert!((out.color[0][i] - out.color[2][i]).abs() < 1e-5);
    }
}

#[test]
fn frequency_range_leaves_other_frequencies_untouched() {
    let planes = test_planes(W, H, 0);
    let cutoff = 0.1;
    let options = fourier(FourierOptions { frequency_range: FrequencyRange::LowPass(cutoff), ..Default::default() });
    let out = scramble_planes(&planes, &options, 5);
    let (sa, sb) = (spectrum(&planes.color[0], W, H), spectrum(&out.color[0], W, H));
    let mut changed_inside = 0;
    for ky in 0..H {
        for kx in 0..W {
            let i = ky * W + kx;
            if radial_frequency(kx, ky, W, H) > cutoff as f64 {
                assert!((sa[i] - sb[i]).norm() < 1e-3);
            } else if (sa[i] - sb[i]).norm() > 1e-3 {
                changed_inside += 1;
            }
        }
    }
    assert!(changed_inside > 0);
}

#[test]
fn removing_outside_range_zeroes_those_frequencies_but_keeps_the_mean() {
    let planes = test_planes(W, H, 0);
    let options = fourier(FourierOptions {
        frequency_range: FrequencyRange::HighPass(0.2),
        remove_outside_range: true,
        ..Default::default()
    });
    let out = scramble_planes(&planes, &options, 5);
    let sb = spectrum(&out.color[0], W, H);
    for ky in 0..H {
        for kx in 0..W {
            let f = radial_frequency(kx, ky, W, H);
            if f > 0.0 && f < 0.2 {
                assert!(sb[ky * W + kx].norm() < 1e-3);
            }
        }
    }
    assert!((mean_sd(&planes.color[0]).0 - mean_sd(&out.color[0]).0).abs() < 1e-6);
}

#[test]
fn frequency_units_are_equivalent() {
    let planes = test_planes(W, H, 0);
    let cpp = 0.12f32;
    let ppd = 40.0f32;
    let run = |unit: FrequencyUnit, cutoff: f32| {
        let options = fourier(FourierOptions { frequency_range: FrequencyRange::LowPass(cutoff), unit, ..Default::default() });
        scramble_planes(&planes, &options, 9)
    };
    let reference = run(FrequencyUnit::CyclesPerPixel, cpp);
    assert_eq!(reference, run(FrequencyUnit::CyclesPerImage, cpp * W as f32));
    assert_eq!(reference, run(FrequencyUnit::CyclesPerDegree { pixels_per_degree: ppd }, cpp * ppd));
}

#[test]
fn padding_modes_run_and_are_reproducible() {
    let image = test_image(W, H);
    for padding_mode in [PaddingMode::Wrap, PaddingMode::Reflect, PaddingMode::Mean] {
        let options = ScrambleOptions { seed: Some(4), ..fourier(FourierOptions { padding_mode, ..Default::default() }) };
        let a = scramble_image(&image, &options).unwrap();
        let b = scramble_image(&image, &options).unwrap();
        assert_eq!(a.image.to_rgb8(), b.image.to_rgb8());
    }
}

#[test]
fn every_method_is_reproducible_with_a_seed() {
    let image = test_image(W, H);
    let methods = [
        ScrambleType::Pixel,
        ScrambleType::Fourier(FourierOptions::default()),
        ScrambleType::Block(BlockOptions { block_size: (8, 8), interpolate_edges: false }),
        ScrambleType::Diffeomorphic(DiffeomorphicOptions::default()),
    ];
    for scramble_type in methods {
        let options = |seed| ScrambleOptions { scramble_type: scramble_type.clone(), seed: Some(seed), ..Default::default() };
        let a = scramble_image(&image, &options(42)).unwrap();
        let b = scramble_image(&image, &options(42)).unwrap();
        let c = scramble_image(&image, &options(43)).unwrap();
        assert_eq!(a.report.seed, 42);
        assert_eq!(a.image.to_rgb8(), b.image.to_rgb8(), "{scramble_type:?}");
        assert_ne!(a.image.to_rgb8(), c.image.to_rgb8(), "{scramble_type:?}");
    }
}

#[test]
fn unseeded_runs_report_a_seed_that_reproduces_them() {
    let image = test_image(W, H);
    let first = scramble_image(&image, &ScrambleOptions::default()).unwrap();
    let again = ScrambleOptions { seed: Some(first.report.seed), ..Default::default() };
    assert_eq!(first.image.to_rgb8(), scramble_image(&image, &again).unwrap().image.to_rgb8());
}

#[test]
fn block_scrambling_keeps_every_pixel_when_size_does_not_divide() {
    let image = test_image(W, H);
    let options = ScrambleOptions {
        scramble_type: ScrambleType::Block(BlockOptions { block_size: (16, 16), interpolate_edges: false }),
        seed: Some(2),
        ..Default::default()
    };
    let out = scramble_image(&image, &options).unwrap();
    assert_eq!(sorted_samples(&image), sorted_samples(&out.image));
    assert_ne!(image.to_rgb8(), out.image.to_rgb8());
}

#[test]
fn blur_preserves_mean_and_bit_depth() {
    let planes = Planes { sixteen_bit: true, ..test_planes(W, H, 0) };
    let image = planes.to_image();
    let options = ScrambleOptions { scramble_type: ScrambleType::Blur(BlurOptions { sigma: 3.0 }), ..Default::default() };
    let out = scramble_image(&image, &options).unwrap();
    assert!(matches!(out.image, DynamicImage::ImageRgb16(_)));
    let (a, b) = (Planes::from_image(&image), Planes::from_image(&out.image));
    assert!((mean_sd(&a.luminance()).0 - mean_sd(&b.luminance()).0).abs() < 2e-3);
    assert!(mean_sd(&b.luminance()).1 < mean_sd(&a.luminance()).1);
}

#[test]
fn small_diffeomorphic_warps_stay_close_to_the_original() {
    let planes = test_planes(W, H, 0);
    let options = ScrambleOptions {
        scramble_type: ScrambleType::Diffeomorphic(DiffeomorphicOptions { max_distortion: 0.05, n_steps: 5, n_comp: 3 }),
        ..Default::default()
    };
    let out = scramble_planes(&planes, &options, 1);
    let diff = planes.color[0].iter().zip(&out.color[0]).map(|(a, b)| (a - b).abs()).sum::<f32>() / planes.len() as f32;
    assert!(diff < 0.01, "mean absolute difference {diff}");
}

#[test]
fn sixteen_bit_images_round_trip_exactly() {
    let image = DynamicImage::ImageRgb16(ImageBuffer::from_fn(31, 17, |x, y| {
        Rgb([(x * 2000 + y) as u16, (y * 3000) as u16, 65535 - (x * y) as u16])
    }));
    assert_eq!(Planes::from_image(&image).to_image(), image);
}

#[test]
fn spectrum_peak_of_a_grating_is_at_its_frequency() {
    let (w, h) = (128, 96);
    let f0 = 0.125; // 8-pixel period
    let plane: Vec<f32> = (0..w * h)
        .map(|i| 0.5 + 0.25 * (2.0 * std::f32::consts::PI * f0 * (i % w) as f32).sin())
        .collect();
    let planes = Planes { width: w, height: h, color: vec![plane], alpha: None, sixteen_bit: false };
    let stats = analyze_planes(&planes);
    let peak = stats
        .spectrum_amplitudes
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.total_cmp(b.1))
        .map(|(i, _)| stats.spectrum_frequencies[i])
        .unwrap();
    let bin_width = 0.5 / (w.min(h) / 2) as f64;
    assert!((peak - f0 as f64).abs() <= bin_width, "peak at {peak}");
    assert!((stats.mean_luminance - 0.5).abs() < 1e-4);
    assert!((stats.rms_contrast - 0.25 / 2f64.sqrt()).abs() < 1e-3);
}

#[test]
fn ssim_of_identical_images_is_one() {
    let planes = test_planes(W, H, 0);
    assert!((ssim_planes(&planes, &planes).unwrap() - 1.0).abs() < 1e-9);
}

#[test]
fn luminance_matching_equalises_mean_and_contrast() {
    let mut set: Vec<Planes> = (0..4).map(|v| test_planes(W, H, v)).collect();
    for (i, p) in set.iter_mut().enumerate() {
        p.color.iter_mut().flatten().for_each(|v| *v = *v * (0.6 + 0.1 * i as f32) + 0.02 * i as f32);
    }
    normalize(&mut set, NormalizeMethod::Luminance { target: None }).unwrap();
    let stats: Vec<(f64, f64)> = set.iter().map(|p| mean_sd(&p.luminance())).collect();
    for s in &stats[1..] {
        assert!((s.0 - stats[0].0).abs() < 1e-5);
        assert!((s.1 - stats[0].1).abs() < 1e-5);
    }
}

#[test]
fn histogram_matching_equalises_histograms() {
    let mut set: Vec<Planes> = (0..3).map(|v| test_planes(W, H, v)).collect();
    set[1].color.iter_mut().flatten().for_each(|v| *v = v.powf(1.5));
    normalize(&mut set, NormalizeMethod::Histogram).unwrap();
    for c in 0..3 {
        let sorted: Vec<Vec<f32>> = set
            .iter()
            .map(|p| {
                let mut v = p.color[c].clone();
                v.sort_by(f32::total_cmp);
                v
            })
            .collect();
        for s in &sorted[1..] {
            assert!(s.iter().zip(&sorted[0]).all(|(a, b)| (a - b).abs() < 1e-6));
        }
    }
}

#[test]
fn spectrum_matching_equalises_amplitude_spectra() {
    let mut set: Vec<Planes> = (0..3).map(|v| test_planes(W, H, v)).collect();
    normalize(&mut set, NormalizeMethod::Spectrum).unwrap();
    let spectra: Vec<_> = set.iter().map(|p| spectrum(&p.color[0], W, H)).collect();
    for s in &spectra[1..] {
        for (a, b) in s.iter().zip(&spectra[0]) {
            assert!((a.norm() - b.norm()).abs() < 1e-3);
        }
    }
}

#[test]
fn compressing_contrast_avoids_clipping_and_keeps_channel_means() {
    // A high-contrast image clips heavily when phase scrambled.
    let mut planes = test_planes(W, H, 0);
    planes.color.iter_mut().flatten().for_each(|v| *v = if *v > 0.5 { 0.95 } else { 0.05 });
    let image = planes.to_image();
    let base = ScrambleOptions { seed: Some(8), ..fourier(FourierOptions::default()) };

    let clipped = scramble_image(&image, &base).unwrap();
    assert!(clipped.report.clipped_fraction > 0.01);

    let options = ScrambleOptions { out_of_range: RangeHandling::CompressContrast, ..base };
    let out = scramble_image(&image, &options).unwrap();
    assert_eq!(out.report.clipped_fraction, 0.0);
    assert!(out.report.contrast_scale < 1.0);
    let (a, b) = (Planes::from_image(&image), Planes::from_image(&out.image));
    for (pa, pb) in a.color.iter().zip(&b.color) {
        assert!((mean_sd(pa).0 - mean_sd(pb).0).abs() < 2.0 / 255.0);
    }
}

fn with_matching(options: ScrambleOptions, property: matching::MatchProperty, target: matching::MatchTarget) -> ScrambleOptions {
    ScrambleOptions { matching: Some(matching::MatchOptions { property, target }), ..options }
}

#[test]
fn matching_to_the_original_restores_its_histogram_after_phase_scrambling() {
    let image = test_image(W, H);
    let options = with_matching(
        ScrambleOptions { seed: Some(3), ..fourier(FourierOptions::default()) },
        matching::MatchProperty::Histogram,
        matching::MatchTarget::Original,
    );
    let out = scramble_image(&image, &options).unwrap();
    assert_eq!(sorted_samples(&image), sorted_samples(&out.image));
    assert_eq!(out.report.clipped_fraction, 0.0);
}

#[test]
fn matching_luminance_to_the_original_keeps_mean_and_contrast() {
    let image = test_image(W, H);
    let options = with_matching(
        ScrambleOptions {
            scramble_type: ScrambleType::Diffeomorphic(DiffeomorphicOptions::default()),
            seed: Some(3),
            ..Default::default()
        },
        matching::MatchProperty::Luminance,
        matching::MatchTarget::Original,
    );
    let out = scramble_image_with(&image, &options, None).unwrap();
    let (a, b) = (Planes::from_image(&image).luminance(), Planes::from_image(&out.image).luminance());
    let ((ma, sa), (mb, sb)) = (mean_sd(&a), mean_sd(&b));
    assert!((ma - mb).abs() < 1e-3 && (sa - sb).abs() < 1e-3, "{ma} {sa} vs {mb} {sb}");
}

#[test]
fn fixed_luminance_targets_are_reached() {
    let planes = test_planes(W, H, 1);
    let options = with_matching(
        ScrambleOptions { seed: Some(1), ..Default::default() },
        matching::MatchProperty::Luminance,
        matching::MatchTarget::Fixed { mean: 0.45, rms_contrast: 0.08 },
    );
    let out = scramble_image(&planes.to_image(), &options).unwrap();
    let (m, sd) = mean_sd(&Planes::from_image(&out.image).luminance());
    assert!((m - 0.45).abs() < 2e-3 && (sd - 0.08).abs() < 2e-3, "{m} {sd}");
}

#[test]
fn set_average_matching_needs_a_target_and_uses_it() {
    let image = test_image(W, H);
    let options = with_matching(
        ScrambleOptions { seed: Some(1), ..Default::default() },
        matching::MatchProperty::Luminance,
        matching::MatchTarget::SetAverage,
    );
    assert!(scramble_image(&image, &options).is_err());
    let target = matching::Target::Luminance { mean: 0.3, rms_contrast: 0.1 };
    let out = scramble_image_with(&image, &options, Some(&target)).unwrap();
    let (m, _) = mean_sd(&Planes::from_image(&out.image).luminance());
    assert!((m - 0.3).abs() < 2e-3);
}

#[test]
fn pixel_intensity_series_is_nested() {
    let n = 60 * 60;
    let planes = Planes {
        width: 60,
        height: 60,
        color: vec![(0..n).map(|i| i as f32 / n as f32).collect()],
        alpha: None,
        sixteen_bit: false,
    };
    let moved = |level: f32| -> Vec<bool> {
        let options = with_level(&ScrambleOptions::default(), level).unwrap();
        let out = scramble_planes(&planes, &options, 21);
        planes.color[0].iter().zip(&out.color[0]).map(|(a, b)| a != b).collect()
    };
    let (low, high) = (moved(30.0), moved(60.0));
    assert!(low.iter().zip(&high).all(|(l, h)| !*l || *h));
}

#[test]
fn fourier_series_levels_share_the_same_noise() {
    let planes = test_planes(W, H, 0);
    let base = fourier(FourierOptions::default());
    let half = scramble_planes(&planes, &with_level(&base, 50.0).unwrap(), 5);
    let full = scramble_planes(&planes, &with_level(&base, 100.0).unwrap(), 5);
    // Same phase field: the half-strength phase offset is exactly half of the full one.
    let (s0, sh, sf) = (spectrum(&planes.color[0], W, H), spectrum(&half.color[0], W, H), spectrum(&full.color[0], W, H));
    let wrap = |a: f64| (a + std::f64::consts::PI).rem_euclid(2.0 * std::f64::consts::PI) - std::f64::consts::PI;
    for i in 1..W * H {
        if s0[i].norm() > 1e-6 {
            let dh = wrap(sh[i].arg() - s0[i].arg());
            let df = wrap(sf[i].arg() - s0[i].arg());
            assert!((wrap(2.0 * dh) - df).abs() < 1e-3 || (wrap(2.0 * dh) - df).abs() > 2.0 * std::f64::consts::PI - 1e-3);
        }
    }
}

#[test]
fn batch_writes_every_exemplar_and_level() {
    let dir = std::env::temp_dir().join(format!("scramblery-test-{}", std::process::id()));
    let (input, output) = (dir.join("in"), dir.join("out"));
    std::fs::create_dir_all(&input).unwrap();
    for i in 0..2 {
        test_planes(32, 24, i).to_image().save(input.join(format!("img{i}.png"))).unwrap();
    }
    let options = BatchProcessingOptions {
        exemplars: 3,
        levels: Some(vec![0.0, 50.0, 100.0]),
        ..BatchProcessingOptions::new(input.clone(), output.clone(), fourier(FourierOptions::default()))
    };
    let results = process_directory_with_progress(&options, |_| {}).unwrap();
    assert_eq!(results.len(), 2 * 3 * 3);
    assert!(results.iter().all(|r| r.success()));
    assert!(output.join("img0_s02_l050.png").exists());
    let report = std::fs::read_to_string(output.join(REPORT_FILE)).unwrap();
    assert_eq!(report.lines().count(), 1 + 18);
    // Level 0 leaves the image unchanged.
    let original = image::open(input.join("img1.png")).unwrap().to_rgb8();
    assert_eq!(original, image::open(output.join("img1_s01_l000.png")).unwrap().to_rgb8());
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn srgb_transfer_round_trips_including_out_of_range_values() {
    for i in -20..=140 {
        let v = i as f32 / 100.0;
        assert!((engine::planes::linear_to_srgb(engine::planes::srgb_to_linear(v)) - v).abs() < 1e-5, "{v}");
    }
}

#[test]
fn linear_light_phase_scrambling_preserves_mean_linear_light() {
    let image = test_image(W, H);
    let options = ScrambleOptions { linear_light: true, seed: Some(4), ..fourier(FourierOptions::default()) };
    let out = scramble_image(&image, &options).unwrap();
    let (a, b) = (Planes::from_image(&image).to_linear(), Planes::from_image(&out.image).to_linear());
    assert!(out.report.clipped_fraction < 0.001);
    for (pa, pb) in a.color.iter().zip(&b.color) {
        assert!((mean_sd(pa).0 - mean_sd(pb).0).abs() < 2e-3, "{} vs {}", mean_sd(pa).0, mean_sd(pb).0);
    }
}

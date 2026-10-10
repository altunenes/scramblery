use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use engine::analysis::{analyze, ssim, ImageStats};
use engine::normalize::{normalize_directory, LuminanceTarget, NormalizeMethod};
use engine::*;

#[derive(Parser)]
#[command(name = "scramblery-cli", version, about = "Image and video scrambling for research, privacy and creative work")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Scramble an image, or every image in a folder.
    Scramble {
        /// Input image or folder.
        input: PathBuf,
        /// Output image or folder.
        #[arg(short, long)]
        output: PathBuf,
        /// Scramble options as JSON (see `scramblery-cli config`). Defaults to the method's defaults.
        #[arg(short, long)]
        config: Option<PathBuf>,
        /// Method used when no config file is given.
        #[arg(short, long, value_enum, default_value_t = Method::Fourier)]
        method: Method,
        /// Seed; overrides the config. Without one a random seed is chosen and reported.
        #[arg(short, long)]
        seed: Option<u64>,
        #[command(flatten)]
        matching: MatchArgs,
        /// Folder of reference images for `--match-target set` when scrambling a single image.
        /// For a folder input, the input images themselves are the reference set.
        #[arg(long)]
        reference_dir: Option<PathBuf>,
        /// Differently seeded outputs per image (folder input).
        #[arg(long, default_value_t = 1)]
        exemplars: usize,
        /// Comma-separated strength levels for an intensity series (folder input): percent for
        /// pixel and Fourier, sigma in px for blur, maximum distortion in px for diffeomorphic.
        #[arg(long, value_delimiter = ',')]
        levels: Option<Vec<f32>>,
    },
    /// Print the default options for a method as JSON, as a starting point for a config file.
    Config {
        #[arg(value_enum)]
        method: Method,
    },
    /// Print low-level statistics of one or more images.
    Analyze {
        images: Vec<PathBuf>,
        /// Also report SSIM against this reference image.
        #[arg(short, long)]
        reference: Option<PathBuf>,
        /// Print JSON including histograms and spectra.
        #[arg(long)]
        json: bool,
    },
    /// Equate luminance, histograms or amplitude spectra across every image in a folder.
    Normalize {
        input: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
        #[arg(short, long, value_enum)]
        method: NormalizeKind,
        /// Target mean luminance (0..1) for `luminance`; defaults to the set's average.
        #[arg(long, requires = "contrast")]
        mean: Option<f64>,
        /// Target RMS contrast (0..1) for `luminance`; defaults to the set's average.
        #[arg(long, requires = "mean")]
        contrast: Option<f64>,
    },
    /// Scramble every frame of a video.
    Video {
        input: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
        #[arg(short, long)]
        config: Option<PathBuf>,
        #[arg(short, long, value_enum, default_value_t = Method::Fourier)]
        method: Method,
        #[arg(short, long)]
        seed: Option<u64>,
        #[arg(long, value_enum, default_value_t = Fidelity::Lossless)]
        fidelity: Fidelity,
        /// `fixed` applies the same scramble to every frame; `per-frame` makes dynamic noise.
        #[arg(long, value_enum, default_value_t = Seeds::Fixed)]
        frame_seeds: Seeds,
        /// Follow the original motion with optical flow, with a fresh scramble every N frames.
        #[arg(long, value_name = "KEYFRAME_INTERVAL", conflicts_with = "spatiotemporal")]
        follow_motion: Option<usize>,
        /// Phase-scramble blocks of frames as spatiotemporal volumes with this intensity (0..1),
        /// instead of applying the method frame by frame.
        #[arg(long, value_name = "INTENSITY")]
        spatiotemporal: Option<f32>,
        #[command(flatten)]
        matching: MatchArgs,
        /// Folder of reference images for `--match-target set`.
        #[arg(long)]
        reference_dir: Option<PathBuf>,
        /// Colour standard of the input, replacing what the decoder reports (for files whose
        /// colour description is missing or wrong).
        #[arg(long, value_enum)]
        color_standard: Option<Standard>,
        /// Memory for spatiotemporal volumes in GB (default 4).
        #[arg(long)]
        memory_gb: Option<f32>,
    },
}

#[derive(Clone, Copy, ValueEnum)]
enum Standard {
    Bt601,
    Bt709,
    Bt2020,
}

#[derive(clap::Args)]
struct MatchArgs {
    /// Match a property of each output to a target; overrides the config.
    #[arg(long = "match", value_enum)]
    property: Option<MatchKind>,
    #[arg(long, value_enum, default_value_t = TargetKind::Original)]
    match_target: TargetKind,
    /// Mean luminance for `--match-target fixed`.
    #[arg(long, required_if_eq("match_target", "fixed"))]
    target_mean: Option<f64>,
    /// RMS contrast for `--match-target fixed`.
    #[arg(long, required_if_eq("match_target", "fixed"))]
    target_contrast: Option<f64>,
}

impl MatchArgs {
    fn options(&self) -> Option<matching::MatchOptions> {
        let property = match self.property? {
            MatchKind::Luminance => matching::MatchProperty::Luminance,
            MatchKind::Histogram => matching::MatchProperty::Histogram,
            MatchKind::Spectrum => matching::MatchProperty::Spectrum,
        };
        let target = match self.match_target {
            TargetKind::Original => matching::MatchTarget::Original,
            TargetKind::Set => matching::MatchTarget::SetAverage,
            TargetKind::Fixed => matching::MatchTarget::Fixed {
                mean: self.target_mean.unwrap_or(0.5),
                rms_contrast: self.target_contrast.unwrap_or(0.2),
            },
        };
        Some(matching::MatchOptions { property, target })
    }
}

#[derive(Clone, Copy, ValueEnum)]
enum MatchKind {
    Luminance,
    Histogram,
    Spectrum,
}

#[derive(Clone, Copy, ValueEnum)]
enum TargetKind {
    Original,
    Fixed,
    Set,
}

#[derive(Clone, Copy, ValueEnum)]
enum Method {
    Pixel,
    Fourier,
    Block,
    Blur,
    Diffeomorphic,
}

impl Method {
    fn default_options(self) -> ScrambleOptions {
        let scramble_type = match self {
            Method::Pixel => ScrambleType::Pixel,
            Method::Fourier => ScrambleType::Fourier(FourierOptions::default()),
            Method::Block => ScrambleType::Block(BlockOptions::default()),
            Method::Blur => ScrambleType::Blur(BlurOptions::default()),
            Method::Diffeomorphic => ScrambleType::Diffeomorphic(DiffeomorphicOptions::default()),
        };
        ScrambleOptions { scramble_type, ..Default::default() }
    }
}

#[derive(Clone, Copy, ValueEnum)]
enum NormalizeKind {
    Luminance,
    Histogram,
    Spectrum,
}

#[derive(Clone, Copy, ValueEnum)]
enum Seeds {
    Fixed,
    PerFrame,
}

#[derive(Clone, Copy, ValueEnum)]
enum Fidelity {
    Lossless,
    HighQuality,
    Fast,
}

fn load_options(config: Option<&Path>, method: Method, seed: Option<u64>) -> Result<ScrambleOptions> {
    let mut options = match config {
        Some(path) => {
            let text = std::fs::read_to_string(path).with_context(|| format!("Cannot read {}", path.display()))?;
            let value: serde_json::Value =
                serde_json::from_str(&text).with_context(|| format!("Invalid JSON in {}", path.display()))?;
            // Settings files written next to outputs hold the options under "scramble_options".
            let options = value.get("scramble_options").cloned().unwrap_or(value);
            serde_json::from_value(options).with_context(|| format!("Invalid options in {}", path.display()))?
        }
        None => method.default_options(),
    };
    if seed.is_some() {
        options.seed = seed;
    }
    Ok(options)
}

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
    match Cli::parse().command {
        Command::Scramble { input, output, config, method, seed, matching, reference_dir, exemplars, levels } => {
            let mut options = load_options(config.as_deref(), method, seed)?;
            if let Some(m) = matching.options() {
                options.matching = Some(m);
            }
            if input.is_dir() {
                let batch = BatchProcessingOptions { exemplars, levels, ..BatchProcessingOptions::new(input, output.clone(), options) };
                scramble_folder(&batch)
            } else {
                let set_target = match (options.matching, reference_dir) {
                    (Some(m), Some(dir)) if m.target == matching::MatchTarget::SetAverage => {
                        Some(engine::normalize::set_target(&dir, m.property, options.linear_light)?)
                    }
                    _ => None,
                };
                scramble_file(&input, &output, &options, set_target.as_ref())
            }
        }
        Command::Config { method } => {
            println!("{}", serde_json::to_string_pretty(&method.default_options())?);
            Ok(())
        }
        Command::Analyze { images, reference, json } => analyze_images(&images, reference.as_deref(), json),
        Command::Normalize { input, output, method, mean, contrast } => {
            let method = match method {
                NormalizeKind::Luminance => NormalizeMethod::Luminance {
                    target: mean.zip(contrast).map(|(mean, rms_contrast)| LuminanceTarget { mean, rms_contrast }),
                },
                NormalizeKind::Histogram => NormalizeMethod::Histogram,
                NormalizeKind::Spectrum => NormalizeMethod::Spectrum,
            };
            let results = normalize_directory(&input, &output, method, |_, _| {})?;
            for r in &results {
                println!(
                    "{}  mean {:.4} -> {:.4}  contrast {:.4} -> {:.4}  clipped {:.2}%",
                    r.input_path.file_name().unwrap_or_default().to_string_lossy(),
                    r.before.0,
                    r.after.0,
                    r.before.1,
                    r.after.1,
                    r.clipped_fraction * 100.0
                );
            }
            println!("Report written to {}", output.join(engine::normalize::NORMALIZE_REPORT_FILE).display());
            Ok(())
        }
        Command::Video { input, output, config, method, seed, fidelity, frame_seeds, follow_motion, spatiotemporal, matching, reference_dir, color_standard, memory_gb } => {
            let mut scramble_options = load_options(config.as_deref(), method, seed)?;
            if let Some(m) = matching.options() {
                scramble_options.matching = Some(m);
            }
            let set_target = match (scramble_options.matching, reference_dir) {
                (Some(m), Some(dir)) if m.target == matching::MatchTarget::SetAverage => {
                    Some(engine::normalize::set_target(&dir, m.property, scramble_options.linear_light)?)
                }
                _ => None,
            };
            let options = VideoProcessingOptions {
                input_path: input,
                output_path: output.clone(),
                scramble_options,
                frame_seeds: match frame_seeds {
                    Seeds::Fixed => FrameSeeds::Fixed,
                    Seeds::PerFrame => FrameSeeds::PerFrame,
                },
                temporal_coherence: follow_motion.map(|keyframe_interval| TemporalCoherenceOptions {
                    export_flow: false,
                    flow_output_dir: None,
                    keyframe_interval,
                    blend_frames: 0,
                }),
                spatiotemporal: spatiotemporal.map(|intensity| engine::video::spatiotemporal::SpatiotemporalOptions { intensity }),
                color_standard: color_standard.map(|s| match s {
                    Standard::Bt601 => ColorStandard::Bt601,
                    Standard::Bt709 => ColorStandard::Bt709,
                    Standard::Bt2020 => ColorStandard::Bt2020,
                }),
                memory_budget_gb: memory_gb,
                fidelity: match fidelity {
                    Fidelity::Lossless => VideoFidelity::Lossless,
                    Fidelity::HighQuality => VideoFidelity::HighQuality,
                    Fidelity::Fast => VideoFidelity::Fast,
                },
            };
            let report = process_video_with(&options, set_target, |p| eprint!("\r{p:5.1}%"))?;
            eprintln!();
            let faces = report.frames_without_faces.map(|n| format!("  frames without faces {n}")).unwrap_or_default();
            let chunks = report.chunks.map(|n| format!("  volumes {n}")).unwrap_or_default();
            println!(
                "{}  seed {}  frames {}  clipped {:.3}%{faces}{chunks}",
                output.display(),
                report.seed,
                report.frames,
                report.clipped_fraction * 100.0
            );
            Ok(())
        }
    }
}

fn scramble_file(input: &Path, output: &Path, options: &ScrambleOptions, set_target: Option<&matching::Target>) -> Result<()> {
    let image = engine::io::open_image(input)?;
    let result = scramble_image_with(&image, options, set_target)?;
    save_scrambled(&result, options, Some(input), output)?;
    print_report(output, &result.report);
    Ok(())
}

fn scramble_folder(batch: &BatchProcessingOptions) -> Result<()> {
    let output = &batch.output_dir;
    let results = process_directory_with_progress(batch, |p| eprint!("\r{}/{}", p.processed_files, p.total_files))?;
    eprintln!();
    let failed: Vec<_> = results.iter().filter(|r| !r.success()).collect();
    for r in &failed {
        eprintln!("{}: {}", r.input_path.display(), r.error.as_deref().unwrap_or(""));
    }
    println!(
        "{} of {} images scrambled. Report: {}",
        results.len() - failed.len(),
        results.len(),
        output.join(REPORT_FILE).display()
    );
    if !failed.is_empty() {
        bail!("{} images failed", failed.len());
    }
    Ok(())
}

fn print_report(output: &Path, report: &ScrambleReport) {
    let faces = report.faces_detected.map(|f| format!("  faces {f}")).unwrap_or_default();
    let contrast = if report.contrast_scale < 1.0 { format!("  contrast x{:.3}", report.contrast_scale) } else { String::new() };
    println!(
        "{}  seed {}  clipped {:.3}%{contrast}{faces}",
        output.display(),
        report.seed,
        report.clipped_fraction * 100.0
    );
}

fn analyze_images(paths: &[PathBuf], reference: Option<&Path>, json: bool) -> Result<()> {
    let reference = reference.map(engine::io::open_image).transpose()?;
    let mut all = Vec::new();
    for path in paths {
        let image = engine::io::open_image(path)?;
        let stats = analyze(&image);
        let similarity = reference.as_ref().and_then(|r| ssim(r, &image));
        if json {
            all.push(serde_json::json!({ "file": path, "stats": stats, "ssim": similarity }));
        } else {
            print_stats(path, &stats, similarity);
        }
    }
    if json {
        println!("{}", serde_json::to_string_pretty(&all)?);
    }
    Ok(())
}

fn print_stats(path: &Path, s: &ImageStats, similarity: Option<f64>) {
    println!("{}", path.display());
    println!("  size              {} x {}", s.width, s.height);
    println!("  mean luminance    {:.4}", s.mean_luminance);
    println!("  RMS contrast      {:.4}", s.rms_contrast);
    println!("  spectral slope    {:.3}", s.spectral_slope);
    println!("  at 0 or 1         {:.3}%", s.saturated_fraction * 100.0);
    if let Some(v) = similarity {
        println!("  SSIM to reference {v:.4}");
    }
}

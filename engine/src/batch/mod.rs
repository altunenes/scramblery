use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::analysis::{analyze_planes, ssim_planes};
use crate::io;
use crate::planes::Planes;
use crate::matching::{MatchTarget, Target};
use crate::scramble::{scramble_image_with, with_level, ScrambleOptions, ScrambleReport};
use crate::seed;

pub const REPORT_FILE: &str = "scramblery_report.csv";
pub const SETTINGS_FILE: &str = "scramblery_settings.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchProcessingOptions {
    pub input_dir: PathBuf,
    pub output_dir: PathBuf,
    pub scramble_options: ScrambleOptions,
    /// Number of differently seeded outputs per input image.
    #[serde(default = "one")]
    pub exemplars: usize,
    /// Strength levels of an intensity series (see `scramble::with_level`). Every level of an
    /// exemplar uses the same seed, so only the strength differs.
    #[serde(default)]
    pub levels: Option<Vec<f32>>,
}

fn one() -> usize {
    1
}

impl BatchProcessingOptions {
    pub fn new(input_dir: PathBuf, output_dir: PathBuf, scramble_options: ScrambleOptions) -> Self {
        Self { input_dir, output_dir, scramble_options, exemplars: 1, levels: None }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingResult {
    pub input_path: PathBuf,
    pub output_path: PathBuf,
    pub exemplar: usize,
    pub level: Option<f32>,
    pub error: Option<String>,
    pub report: Option<ScrambleReport>,
    pub comparison: Option<Comparison>,
}

impl ProcessingResult {
    pub fn success(&self) -> bool {
        self.error.is_none()
    }
}

/// Statistics of an input image and its scrambled version.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Comparison {
    pub mean_luminance: (f64, f64),
    pub rms_contrast: (f64, f64),
    pub spectral_slope: (f64, f64),
    pub ssim: Option<f64>,
}

impl Comparison {
    pub fn new(original: &Planes, scrambled: &Planes) -> Self {
        let (a, b) = (analyze_planes(original), analyze_planes(scrambled));
        Self {
            mean_luminance: (a.mean_luminance, b.mean_luminance),
            rms_contrast: (a.rms_contrast, b.rms_contrast),
            spectral_slope: (a.spectral_slope, b.spectral_slope),
            ssim: ssim_planes(original, scrambled),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BatchProgress {
    pub total_files: usize,
    pub processed_files: usize,
}

/// Settings written next to the outputs so a run can be reproduced.
#[derive(Debug, Serialize, Deserialize)]
pub struct RunSettings {
    pub scramblery_version: String,
    pub scramble_options: ScrambleOptions,
    #[serde(default = "one")]
    pub exemplars: usize,
    #[serde(default)]
    pub levels: Option<Vec<f32>>,
}

impl RunSettings {
    pub fn new(options: &ScrambleOptions) -> Self {
        Self { scramblery_version: crate::VERSION.to_string(), scramble_options: options.clone(), exemplars: 1, levels: None }
    }
}

/// File name for one output: the input name, then `_sNN` for the exemplar when there are
/// several, then `_lLEVEL` for a series level (a decimal point is written as `p`).
pub fn output_name(input: &Path, exemplar: usize, exemplars: usize, level: Option<f32>) -> PathBuf {
    let stem = input.file_stem().unwrap_or_default().to_string_lossy();
    let ext = input.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
    let mut name = stem.to_string();
    if exemplars > 1 {
        name.push_str(&format!("_s{:02}", exemplar + 1));
    }
    if let Some(level) = level {
        let text = if level.fract() == 0.0 { format!("{:03}", level as i64) } else { format!("{level}").replace('.', "p") };
        name.push_str(&format!("_l{text}"));
    }
    PathBuf::from(format!("{name}{ext}"))
}

/// Scrambles every image in `input_dir` into `output_dir`.
///
/// Files are processed in name order. Exemplar `e` of file `i` uses `seed::derive(base_seed, i)`
/// for `e = 0` and `seed::derive(that, e)` otherwise, where the base seed is
/// `scramble_options.seed` or a random one. With a set-average match target, the target is the
/// average over the input images. A CSV report and the settings (including the base seed) are
/// written to the output folder.
pub fn process_directory_with_progress(
    options: &BatchProcessingOptions,
    progress: impl Fn(BatchProgress) + Send + Sync,
) -> crate::Result<Vec<ProcessingResult>> {
    std::fs::create_dir_all(&options.output_dir)?;
    let files = io::list_images(&options.input_dir)?;
    let base_seed = seed::resolve(options.scramble_options.seed);
    let exemplars = options.exemplars.max(1);
    let levels: Vec<Option<f32>> = match &options.levels {
        Some(levels) if !levels.is_empty() => levels.iter().copied().map(Some).collect(),
        _ => vec![None],
    };
    if levels[0].is_some() {
        with_level(&options.scramble_options, 0.0)?;
    }

    let set_target: Option<Target> = match options.scramble_options.matching {
        Some(m) if m.target == MatchTarget::SetAverage && !files.is_empty() => {
            Some(crate::normalize::set_target_of_files(&files, m.property, options.scramble_options.linear_light, |_| {})?)
        }
        _ => None,
    };

    let mut jobs: Vec<(usize, usize, Option<f32>)> = Vec::new();
    for i in 0..files.len() {
        for e in 0..exemplars {
            jobs.extend(levels.iter().map(|&l| (i, e, l)));
        }
    }
    let total_files = jobs.len();
    let done = AtomicUsize::new(0);
    progress(BatchProgress { total_files, processed_files: 0 });

    let results: Vec<ProcessingResult> = jobs
        .par_iter()
        .map(|&(index, exemplar, level)| {
            let input_path = &files[index];
            let output_path = options.output_dir.join(output_name(input_path, exemplar, exemplars, level));
            let file_seed = seed::derive(base_seed, index as u64);
            let seed = if exemplar == 0 { file_seed } else { seed::derive(file_seed, exemplar as u64) };
            let outcome = (|| {
                let base = match level {
                    Some(l) => with_level(&options.scramble_options, l)?,
                    None => options.scramble_options.clone(),
                };
                let file_options = ScrambleOptions { seed: Some(seed), ..base };
                process_file(input_path, &output_path, &file_options, set_target.as_ref())
            })();
            progress(BatchProgress { total_files, processed_files: done.fetch_add(1, Ordering::Relaxed) + 1 });
            let (report, comparison, error) = match outcome {
                Ok((r, c)) => (Some(r), Some(c), None),
                Err(e) => (None, None, Some(format!("{e:#}"))),
            };
            ProcessingResult { input_path: input_path.clone(), output_path, exemplar, level, error, report, comparison }
        })
        .collect();

    let settings = RunSettings {
        scramblery_version: crate::VERSION.to_string(),
        scramble_options: ScrambleOptions { seed: Some(base_seed), ..options.scramble_options.clone() },
        exemplars,
        levels: options.levels.clone(),
    };
    io::write_json(&settings, &options.output_dir.join(SETTINGS_FILE))?;
    write_report(&results, &options.output_dir.join(REPORT_FILE))?;
    Ok(results)
}

fn process_file(
    input: &Path,
    output: &Path,
    options: &ScrambleOptions,
    set_target: Option<&Target>,
) -> crate::Result<(ScrambleReport, Comparison)> {
    let image = io::open_image(input)?;
    let scrambled = scramble_image_with(&image, options, set_target)?;
    io::save_image(&scrambled.image, output)?;
    let comparison = Comparison::new(&Planes::from_image(&image), &Planes::from_image(&scrambled.image));
    Ok((scrambled.report, comparison))
}

fn csv_field(value: &str) -> String {
    if value.contains([',', '"', '\n']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

pub fn write_report(results: &[ProcessingResult], path: &Path) -> crate::Result<()> {
    let mut out = String::from(
        "file,output,exemplar,level,seed,clipped_fraction,contrast_scale,faces_detected,mean_luminance_in,\
         mean_luminance_out,rms_contrast_in,rms_contrast_out,spectral_slope_in,spectral_slope_out,ssim,error\n",
    );
    for r in results {
        let name = r.input_path.file_name().unwrap_or_default().to_string_lossy();
        let output = r.output_path.file_name().unwrap_or_default().to_string_lossy();
        let mut fields = vec![
            csv_field(&name),
            csv_field(&output),
            (r.exemplar + 1).to_string(),
            r.level.map(|l| l.to_string()).unwrap_or_default(),
        ];
        match (&r.report, &r.comparison) {
            (Some(rep), Some(cmp)) => {
                fields.push(rep.seed.to_string());
                fields.push(format!("{:.6}", rep.clipped_fraction));
                fields.push(format!("{:.6}", rep.contrast_scale));
                fields.push(rep.faces_detected.map(|f| f.to_string()).unwrap_or_default());
                for (a, b) in [cmp.mean_luminance, cmp.rms_contrast, cmp.spectral_slope] {
                    fields.push(format!("{a:.6}"));
                    fields.push(format!("{b:.6}"));
                }
                fields.push(cmp.ssim.map(|s| format!("{s:.6}")).unwrap_or_default());
                fields.push(String::new());
            }
            _ => {
                fields.extend(std::iter::repeat_n(String::new(), 11));
                fields.push(csv_field(r.error.as_deref().unwrap_or("")));
            }
        }
        out.push_str(&fields.join(","));
        out.push('\n');
    }
    std::fs::write(path, out)?;
    Ok(())
}

/// Settings and report written next to a single scrambled image.
#[derive(Debug, Serialize, Deserialize)]
pub struct ImageRunSettings {
    pub scramblery_version: String,
    pub input: Option<PathBuf>,
    pub scramble_options: ScrambleOptions,
    pub report: ScrambleReport,
}

/// Path of the settings file written next to `output`: the same name with a `.json` extension.
pub fn settings_path(output: &Path) -> PathBuf {
    output.with_extension("json")
}

/// Saves a scrambled image and its settings file. The saved options carry the seed that was used.
pub fn save_scrambled(
    output: &crate::ScrambleOutput,
    options: &ScrambleOptions,
    input: Option<&Path>,
    path: &Path,
) -> crate::Result<()> {
    io::save_image(&output.image, path)?;
    let settings = ImageRunSettings {
        scramblery_version: crate::VERSION.to_string(),
        input: input.map(Path::to_path_buf),
        scramble_options: ScrambleOptions { seed: Some(output.report.seed), ..options.clone() },
        report: output.report.clone(),
    };
    io::write_json(&settings, &settings_path(path))
}

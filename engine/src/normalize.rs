//! Equating low-level properties across a set of images. The methods follow those of the SHINE
//! toolbox (Willenbockel et al., 2010).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::analysis::mean_sd;
use crate::matching::{self, MatchProperty, SetAccumulator, Target};
use crate::planes::Planes;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum NormalizeMethod {
    /// Give every image the same mean luminance and RMS contrast. Without a target,
    /// the averages over the set are used.
    Luminance { target: Option<LuminanceTarget> },
    /// Give every image the same histogram (per channel): the average of the set's histograms.
    Histogram,
    /// Give every image the same amplitude spectrum (per channel): the average of the set's
    /// spectra, keeping each image's phase. All images must have the same size.
    Spectrum,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LuminanceTarget {
    pub mean: f64,
    pub rms_contrast: f64,
}

impl NormalizeMethod {
    fn property(self) -> MatchProperty {
        match self {
            NormalizeMethod::Luminance { .. } => MatchProperty::Luminance,
            NormalizeMethod::Histogram => MatchProperty::Histogram,
            NormalizeMethod::Spectrum => MatchProperty::Spectrum,
        }
    }

    fn fixed_target(self) -> Option<Target> {
        match self {
            NormalizeMethod::Luminance { target: Some(t) } => Some(Target::Luminance { mean: t.mean, rms_contrast: t.rms_contrast }),
            _ => None,
        }
    }
}

/// Normalises `images` in place and returns the fraction of samples clipped in each.
pub fn normalize(images: &mut [Planes], method: NormalizeMethod) -> crate::Result<Vec<f64>> {
    if images.is_empty() {
        return Ok(Vec::new());
    }
    let target = match method.fixed_target() {
        Some(t) => t,
        None => {
            let resolution = images.iter().map(Planes::len).max().unwrap_or(1);
            let mut acc = SetAccumulator::new(method.property(), resolution);
            for image in images.iter() {
                acc.add(image)?;
            }
            acc.finish()?
        }
    };
    images
        .par_iter_mut()
        .map(|image| {
            matching::apply(image, &target)?;
            Ok(image.clip())
        })
        .collect()
}

/// Computes the average target of the images in `dir` for `property`, reading one image at a time.
/// With `linear`, the target is computed in linear light (see `ScrambleOptions::linear_light`).
pub fn set_target(dir: &Path, property: MatchProperty, linear: bool) -> crate::Result<Target> {
    let files = crate::io::list_images(dir)?;
    if files.is_empty() {
        anyhow::bail!("No images found in {}", dir.display());
    }
    set_target_of_files(&files, property, linear, |_| {})
}

pub(crate) fn set_target_of_files(files: &[PathBuf], property: MatchProperty, linear: bool, on_image: impl Fn(&Planes)) -> crate::Result<Target> {
    let resolution = files
        .iter()
        .filter_map(|p| image::image_dimensions(p).ok())
        .map(|(w, h)| w as usize * h as usize)
        .max()
        .unwrap_or(1);
    let mut acc = SetAccumulator::new(property, resolution);
    for path in files {
        let planes = Planes::from_image(&crate::io::open_image(path)?);
        let planes = if linear { planes.to_linear() } else { planes };
        on_image(&planes);
        acc.add(&planes)?;
    }
    acc.finish()
}

pub const NORMALIZE_REPORT_FILE: &str = "scramblery_normalize_report.csv";
pub const NORMALIZE_SETTINGS_FILE: &str = "scramblery_normalize_settings.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedFile {
    pub input_path: PathBuf,
    pub output_path: PathBuf,
    /// Mean luminance and RMS contrast before normalisation.
    pub before: (f64, f64),
    /// Mean luminance and RMS contrast after normalisation.
    pub after: (f64, f64),
    pub clipped_fraction: f64,
}

#[derive(Debug, Serialize, Deserialize)]
struct NormalizeSettings {
    scramblery_version: String,
    method: NormalizeMethod,
    files: usize,
}

/// Normalises every image in `input_dir` as one set and writes the results, a CSV report
/// and the settings to `output_dir`. Images are read twice (once to build the target, once
/// to apply it) so that large sets do not need to fit in memory. `progress` receives
/// (steps done, total steps).
pub fn normalize_directory(
    input_dir: &Path,
    output_dir: &Path,
    method: NormalizeMethod,
    progress: impl Fn(usize, usize) + Send + Sync,
) -> crate::Result<Vec<NormalizedFile>> {
    let files = crate::io::list_images(input_dir)?;
    if files.is_empty() {
        anyhow::bail!("No images found in {}", input_dir.display());
    }
    std::fs::create_dir_all(output_dir)?;
    let total = files.len() * 2;
    let step = AtomicUsize::new(0);
    let tick = || progress(step.fetch_add(1, Ordering::Relaxed) + 1, total);

    let target = match method.fixed_target() {
        Some(t) => {
            files.iter().for_each(|_| tick());
            t
        }
        None => set_target_of_files(&files, method.property(), false, |_| tick())?,
    };

    let results: Vec<NormalizedFile> = files
        .par_iter()
        .map(|input| {
            let mut planes = Planes::from_image(&crate::io::open_image(input)?);
            let before = mean_sd(&planes.luminance());
            matching::apply(&mut planes, &target)?;
            let clipped_fraction = planes.clip();
            let output = output_dir.join(input.file_name().unwrap_or_default());
            crate::io::save_image(&planes.to_image(), &output)?;
            tick();
            Ok(NormalizedFile {
                input_path: input.clone(),
                output_path: output,
                before,
                after: mean_sd(&planes.luminance()),
                clipped_fraction,
            })
        })
        .collect::<crate::Result<_>>()?;

    let settings = NormalizeSettings { scramblery_version: crate::VERSION.to_string(), method, files: results.len() };
    crate::io::write_json(&settings, &output_dir.join(NORMALIZE_SETTINGS_FILE))?;

    let mut csv = String::from("file,mean_luminance_before,mean_luminance_after,rms_contrast_before,rms_contrast_after,clipped_fraction\n");
    for r in &results {
        let name = r.input_path.file_name().unwrap_or_default().to_string_lossy().replace('"', "\"\"");
        csv.push_str(&format!(
            "\"{name}\",{:.6},{:.6},{:.6},{:.6},{:.6}\n",
            r.before.0, r.after.0, r.before.1, r.after.1, r.clipped_fraction
        ));
    }
    std::fs::write(output_dir.join(NORMALIZE_REPORT_FILE), csv)?;
    Ok(results)
}

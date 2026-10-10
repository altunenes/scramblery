use std::path::PathBuf;

use engine::BatchProcessingOptions;
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};

use crate::{options, overview, AppWindow, BatchState};

fn pick_folder(title: &str) -> Option<String> {
    rfd::FileDialog::new()
        .set_title(title)
        .pick_folder()
        .map(|p| p.display().to_string())
}

/// The intensity series levels, `None` when the series is off.
pub fn levels(ui: &AppWindow) -> Result<Option<Vec<f32>>, String> {
    let state = ui.global::<BatchState>();
    if !state.get_series() {
        return Ok(None);
    }
    let levels: Vec<f32> = state
        .get_levels_text()
        .split([',', ';', ' '])
        .filter(|t| !t.is_empty())
        .map(|t| t.parse::<f32>().map_err(|_| format!("\"{t}\" is not a number")))
        .collect::<Result<_, _>>()?;
    if levels.is_empty() {
        return Err("Enter at least one level".into());
    }
    Ok(Some(levels))
}

fn start(ui: &AppWindow) {
    let state = ui.global::<BatchState>();
    let levels = match levels(ui) {
        Ok(levels) => levels,
        Err(e) => {
            state.set_error(format!("Intensity series: {e}").into());
            return;
        }
    };
    let options = BatchProcessingOptions {
        exemplars: state.get_exemplars().round().max(1.0) as usize,
        levels,
        ..BatchProcessingOptions::new(
            PathBuf::from(state.get_input_dir().as_str()),
            PathBuf::from(state.get_output_dir().as_str()),
            options::scramble_options(ui),
        )
    };

    state.set_busy(true);
    state.set_done(false);
    state.set_processed(0);
    state.set_total(0);
    state.set_error("".into());

    let progress_ui = ui.as_weak();
    let done_ui = ui.as_weak();
    std::thread::spawn(move || {
        let outcome = engine::process_directory_with_progress(&options, move |progress| {
            let _ = progress_ui.upgrade_in_event_loop(move |ui| {
                let state = ui.global::<BatchState>();
                state.set_processed(progress.processed_files as i32);
                state.set_total(progress.total_files as i32);
            });
        });

        let _ = done_ui.upgrade_in_event_loop(move |ui| {
            let state = ui.global::<BatchState>();
            state.set_busy(false);
            match outcome {
                Ok(results) if results.is_empty() => {
                    state.set_error("No supported images found in the input folder.".into());
                }
                Ok(results) => {
                    let failures: Vec<SharedString> = results
                        .iter()
                        .filter(|r| !r.success())
                        .map(|r| {
                            let name = r.input_path.file_name().unwrap_or_default().to_string_lossy();
                            format!("{name}: {}", r.error.as_deref().unwrap_or("unknown error")).into()
                        })
                        .collect();
                    state.set_total(results.len() as i32);
                    state.set_processed(results.len() as i32);
                    state.set_succeeded((results.len() - failures.len()) as i32);
                    state.set_failed(failures.len() as i32);
                    state.set_failures(ModelRc::new(VecModel::from(failures)));
                    show_overview(&state, &results);
                    let settings = PathBuf::from(state.get_output_dir().as_str()).join(engine::SETTINGS_FILE);
                    if let Some(seed) = std::fs::read_to_string(&settings)
                        .ok()
                        .and_then(|t| serde_json::from_str::<engine::RunSettings>(&t).ok())
                        .and_then(|s| s.scramble_options.seed)
                    {
                        state.set_seed(seed.to_string().into());
                    }
                    state.set_done(true);
                }
                Err(e) => state.set_error(format!("Batch processing failed: {e}").into()),
            }
        });
    });
}

fn show_overview(state: &BatchState, results: &[engine::ProcessingResult]) {
    let mut before = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut after = Vec::new();
    for c in results.iter().filter_map(|r| r.comparison.as_ref().map(|c| (r, c))) {
        if seen.insert(c.0.input_path.clone()) {
            before.push((c.1.mean_luminance.0, c.1.rms_contrast.0));
        }
        after.push((c.1.mean_luminance.1, c.1.rms_contrast.1));
    }
    let o = overview::build(&before, &after);
    state.set_overview(ModelRc::new(VecModel::from(o.points)));
    state.set_overview_x_min(o.x_range.0.into());
    state.set_overview_x_max(o.x_range.1.into());
    state.set_overview_y_min(o.y_range.0.into());
    state.set_overview_y_max(o.y_range.1.into());
    state.set_spread(
        format!(
            "Spread of mean luminance and RMS contrast: inputs ±{:.3} · ±{:.3}, outputs ±{:.3} · ±{:.3}",
            o.spread_before.0, o.spread_before.1, o.spread_after.0, o.spread_after.1
        )
        .into(),
    );
}

pub fn setup(ui: &AppWindow) {
    let state = ui.global::<BatchState>();

    let weak = ui.as_weak();
    state.on_pick_input(move || {
        if let Some(dir) = pick_folder("Select input folder") {
            weak.unwrap().global::<BatchState>().set_input_dir(dir.into());
        }
    });

    let weak = ui.as_weak();
    state.on_pick_output(move || {
        if let Some(dir) = pick_folder("Select output folder") {
            weak.unwrap().global::<BatchState>().set_output_dir(dir.into());
        }
    });

    let weak = ui.as_weak();
    state.on_start(move || start(&weak.unwrap()));

    let weak = ui.as_weak();
    state.on_open_output(move || {
        let ui = weak.unwrap();
        let state = ui.global::<BatchState>();
        if let Err(e) = open::that(state.get_output_dir().as_str()) {
            state.set_error(format!("Could not open folder: {e}").into());
        }
    });

    let weak = ui.as_weak();
    state.on_dismiss_error(move || weak.unwrap().global::<BatchState>().set_error("".into()));
}

use std::path::PathBuf;

use engine::normalize::{normalize_directory, LuminanceTarget, NormalizeMethod, NormalizedFile};
use slint::{ComponentHandle, ModelRc, VecModel};

use crate::{overview, AppWindow, NormalizeState, NormalizedRow};

fn spread(values: impl Iterator<Item = (f64, f64)> + Clone) -> String {
    let range = |it: &mut dyn Iterator<Item = f64>| {
        it.fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| (lo.min(v), hi.max(v)))
    };
    let (m0, m1) = range(&mut values.clone().map(|v| v.0));
    let (c0, c1) = range(&mut values.map(|v| v.1));
    format!("±{:.3} · ±{:.3}", (m1 - m0) / 2.0, (c1 - c0) / 2.0)
}

fn show_results(state: &NormalizeState, results: &[NormalizedFile]) {
    let rows: Vec<NormalizedRow> = results
        .iter()
        .map(|r| NormalizedRow {
            file: r.input_path.file_name().unwrap_or_default().to_string_lossy().to_string().into(),
            mean: format!("{:.3} → {:.3}", r.before.0, r.after.0).into(),
            contrast: format!("{:.3} → {:.3}", r.before.1, r.after.1).into(),
            clipped: format!("{:.2}%", r.clipped_fraction * 100.0).into(),
        })
        .collect();
    state.set_rows(ModelRc::new(VecModel::from(rows)));
    state.set_spread_before(spread(results.iter().map(|r| r.before)).into());
    state.set_spread_after(spread(results.iter().map(|r| r.after)).into());
    let before: Vec<(f64, f64)> = results.iter().map(|r| r.before).collect();
    let after: Vec<(f64, f64)> = results.iter().map(|r| r.after).collect();
    let o = overview::build(&before, &after);
    state.set_overview(ModelRc::new(VecModel::from(o.points)));
    state.set_overview_x_min(o.x_range.0.into());
    state.set_overview_x_max(o.x_range.1.into());
    state.set_overview_y_min(o.y_range.0.into());
    state.set_overview_y_max(o.y_range.1.into());
}

fn start(ui: &AppWindow) {
    let state = ui.global::<NormalizeState>();
    let input = PathBuf::from(state.get_input_dir().as_str());
    let output = PathBuf::from(state.get_output_dir().as_str());
    let method = match state.get_method() {
        1 => NormalizeMethod::Histogram,
        2 => NormalizeMethod::Spectrum,
        _ => NormalizeMethod::Luminance {
            target: state.get_custom_target().then(|| LuminanceTarget {
                mean: state.get_target_mean() as f64,
                rms_contrast: state.get_target_contrast() as f64,
            }),
        },
    };
    state.set_busy(true);
    state.set_done(false);
    state.set_progress(0.0);
    state.set_error("".into());

    let progress_ui = ui.as_weak();
    let done_ui = ui.as_weak();
    std::thread::spawn(move || {
        let outcome = normalize_directory(&input, &output, method, move |done, total| {
            let _ = progress_ui.upgrade_in_event_loop(move |ui| {
                ui.global::<NormalizeState>().set_progress(done as f32 / total.max(1) as f32);
            });
        });
        let _ = done_ui.upgrade_in_event_loop(move |ui| {
            let state = ui.global::<NormalizeState>();
            state.set_busy(false);
            match outcome {
                Ok(results) => {
                    show_results(&state, &results);
                    state.set_done(true);
                }
                Err(e) => state.set_error(format!("{e:#}").into()),
            }
        });
    });
}

pub fn setup(ui: &AppWindow) {
    let state = ui.global::<NormalizeState>();

    let weak = ui.as_weak();
    state.on_pick_input(move || {
        if let Some(dir) = rfd::FileDialog::new().set_title("Select input folder").pick_folder() {
            weak.unwrap().global::<NormalizeState>().set_input_dir(dir.display().to_string().into());
        }
    });

    let weak = ui.as_weak();
    state.on_pick_output(move || {
        if let Some(dir) = rfd::FileDialog::new().set_title("Select output folder").pick_folder() {
            weak.unwrap().global::<NormalizeState>().set_output_dir(dir.display().to_string().into());
        }
    });

    let weak = ui.as_weak();
    state.on_start(move || start(&weak.unwrap()));

    let weak = ui.as_weak();
    state.on_open_output(move || {
        let ui = weak.unwrap();
        let state = ui.global::<NormalizeState>();
        if let Err(e) = open::that(state.get_output_dir().as_str()) {
            state.set_error(format!("Could not open folder: {e}").into());
        }
    });

    let weak = ui.as_weak();
    state.on_dismiss_error(move || weak.unwrap().global::<NormalizeState>().set_error("".into()));
}

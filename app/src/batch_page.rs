use std::path::PathBuf;

use engine::BatchProcessingOptions;
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};

use crate::{options, AppWindow, BatchState};

fn pick_folder(title: &str) -> Option<String> {
    rfd::FileDialog::new()
        .set_title(title)
        .pick_folder()
        .map(|p| p.display().to_string())
}

fn start(ui: &AppWindow) {
    let state = ui.global::<BatchState>();
    let options = BatchProcessingOptions {
        input_dir: PathBuf::from(state.get_input_dir().as_str()),
        output_dir: PathBuf::from(state.get_output_dir().as_str()),
        scramble_options: options::scramble_options(ui),
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
                    state.set_error("No supported images (JPEG, PNG, GIF, WebP) found in the input folder.".into());
                }
                Ok(results) => {
                    let failures: Vec<SharedString> = results
                        .iter()
                        .filter(|r| !r.success)
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
                    state.set_done(true);
                }
                Err(e) => state.set_error(format!("Batch processing failed: {e}").into()),
            }
        });
    });
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

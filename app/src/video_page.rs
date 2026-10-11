use std::path::{Path, PathBuf};

use engine::{FrameSeeds, TemporalCoherenceOptions, VideoFidelity, VideoProcessingOptions};
use slint::ComponentHandle;

use crate::{options, AppWindow, VideoState};

const VIDEO_EXTENSIONS: &[&str] = &["mp4", "avi", "mov", "mkv"];

fn default_output_path(input: &Path) -> PathBuf {
    let stem = input.file_stem().unwrap_or_default().to_string_lossy();
    input.with_file_name(format!("{stem}_scrambled.mp4"))
}

fn start(ui: &AppWindow) {
    let state = ui.global::<VideoState>();
    let input_path = PathBuf::from(state.get_input_path().as_str());
    let output_path = match state.get_output_path().as_str() {
        "" => default_output_path(&input_path),
        path => PathBuf::from(path),
    };

    let temporal_coherence = if state.get_temporal_mode() == 2 {
        let export_flow = state.get_export_flow();
        let flow_dir = state.get_flow_dir();
        if export_flow && flow_dir.is_empty() {
            state.set_error("Select a folder for the exported flow files.".into());
            return;
        }
        Some(TemporalCoherenceOptions {
            export_flow,
            flow_output_dir: export_flow.then(|| PathBuf::from(flow_dir.as_str())),
            keyframe_interval: state.get_keyframe_interval().round() as usize,
            blend_frames: state.get_blend_frames().round() as usize,
        })
    } else {
        None
    };

    let options = VideoProcessingOptions {
        input_path,
        output_path: output_path.clone(),
        scramble_options: options::scramble_options(ui),
        frame_seeds: if state.get_temporal_mode() == 1 { FrameSeeds::PerFrame } else { FrameSeeds::Fixed },
        temporal_coherence,
        spatiotemporal: (state.get_temporal_mode() == 3)
            .then(|| engine::video::spatiotemporal::SpatiotemporalOptions { intensity: state.get_spatiotemporal_intensity() / 100.0 }),
        color_standard: match state.get_color_standard() {
            1 => Some(engine::ColorStandard::Bt601),
            2 => Some(engine::ColorStandard::Bt709),
            3 => Some(engine::ColorStandard::Bt2020),
            _ => None,
        },
        memory_budget_gb: Some(state.get_memory_gb()),
        fidelity: match state.get_fidelity() {
            0 => VideoFidelity::Lossless,
            1 => VideoFidelity::HighQuality,
            _ => VideoFidelity::Fast,
        },
    };

    state.set_output_path(output_path.display().to_string().into());
    state.set_busy(true);
    state.set_done(false);
    state.set_progress(0.0);
    state.set_error("".into());
    state.set_device("".into());

    let progress_ui = ui.as_weak();
    let done_ui = ui.as_weak();
    // With a set-average match target, the target is the average of the reference folder.
    let reference = match options.scramble_options.matching {
        Some(m) if m.target == engine::matching::MatchTarget::SetAverage => {
            let dir = ui.global::<crate::Params>().get_reference_dir().to_string();
            if dir.is_empty() {
                state.set_busy(false);
                state.set_error("Choose a reference folder in the Matching section to match to a set average.".into());
                return;
            }
            Some((PathBuf::from(dir), m.property, options.scramble_options.linear_light))
        }
        _ => None,
    };

    std::thread::spawn(move || {
        let set_target = match reference {
            Some((dir, property, linear)) => match engine::normalize::set_target(&dir, property, linear) {
                Ok(target) => Some(target),
                Err(e) => {
                    let _ = done_ui.upgrade_in_event_loop(move |ui| {
                        let state = ui.global::<VideoState>();
                        state.set_busy(false);
                        state.set_error(format!("{e:#}").into());
                    });
                    return;
                }
            },
            None => None,
        };
        if options.temporal_coherence.is_some() {
            if let Ok(device) = engine::optical_flow_device() {
                let _ = progress_ui.upgrade_in_event_loop(move |ui| {
                    ui.global::<VideoState>().set_device(device.into());
                });
            }
        }
        // A panic must end the run with an error; otherwise the page waits forever.
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            engine::process_video_with(&options, set_target, move |progress| {
                let _ = progress_ui.upgrade_in_event_loop(move |ui| {
                    ui.global::<VideoState>().set_progress(progress);
                });
            })
        }))
        .unwrap_or_else(|panic| {
            let message = panic
                .downcast_ref::<String>()
                .map(String::as_str)
                .or_else(|| panic.downcast_ref::<&str>().copied())
                .unwrap_or("unexpected internal error");
            Err(anyhow::anyhow!("{message}"))
        });

        let _ = done_ui.upgrade_in_event_loop(move |ui| {
            let state = ui.global::<VideoState>();
            state.set_busy(false);
            match outcome {
                Ok(report) => {
                    state.set_seed(report.seed.to_string().into());
                    let mut text = format!("Seed {} · {} frames · settings saved next to the video as .json", report.seed, report.frames);
                    if let Some(n) = report.frames_without_faces.filter(|n| *n > 0) {
                        text.push_str(&format!(" · no face found in {n} frames"));
                    }
                    if let Some(n) = report.chunks.filter(|n| *n > 1) {
                        text.push_str(&format!(" · scrambled in {n} crossfaded volumes"));
                    }
                    if report.clipped_fraction > 0.01 {
                        text.push_str(&format!(" · {:.1}% of values outside 0 to 1 were clipped", report.clipped_fraction * 100.0));
                    }
                    if report.output_bit_depth == Some(10) {
                        text.push_str(" · 10-bit output");
                    }
                    state.set_report(text.into());
                    state.set_progress(100.0);
                    state.set_done(true);
                }
                Err(e) => state.set_error(format!("Video processing failed: {e}").into()),
            }
        });
    });
}

pub fn setup(ui: &AppWindow) {
    let state = ui.global::<VideoState>();

    let weak = ui.as_weak();
    state.on_pick_input(move || {
        let Some(path) = rfd::FileDialog::new()
            .set_title("Select video")
            .add_filter("Video", VIDEO_EXTENSIONS)
            .pick_file()
        else {
            return;
        };
        let ui = weak.unwrap();
        let state = ui.global::<VideoState>();
        state.set_output_path(default_output_path(&path).display().to_string().into());
        state.set_input_path(path.display().to_string().into());
        state.set_done(false);
    });

    let weak = ui.as_weak();
    state.on_pick_output(move || {
        let ui = weak.unwrap();
        let state = ui.global::<VideoState>();
        let current = PathBuf::from(state.get_output_path().as_str());
        let mut dialog = rfd::FileDialog::new()
            .set_title("Save scrambled video as")
            .add_filter("MP4", &["mp4"]);
        if let Some(name) = current.file_name() {
            dialog = dialog.set_file_name(name.to_string_lossy());
        }
        if let Some(dir) = current.parent().filter(|d| d.is_dir()) {
            dialog = dialog.set_directory(dir);
        }
        if let Some(mut path) = dialog.save_file() {
            path.set_extension("mp4");
            state.set_output_path(path.display().to_string().into());
        }
    });

    let weak = ui.as_weak();
    state.on_pick_flow_dir(move || {
        if let Some(dir) = rfd::FileDialog::new().set_title("Select flow output folder").pick_folder() {
            weak.unwrap().global::<VideoState>().set_flow_dir(dir.display().to_string().into());
        }
    });

    let weak = ui.as_weak();
    state.on_start(move || start(&weak.unwrap()));

    let weak = ui.as_weak();
    state.on_open_output(move || {
        let ui = weak.unwrap();
        let state = ui.global::<VideoState>();
        let output = PathBuf::from(state.get_output_path().as_str());
        let folder = output.parent().map(Path::to_path_buf).unwrap_or(output);
        if let Err(e) = open::that(&folder) {
            state.set_error(format!("Could not open folder: {e}").into());
        }
    });

    let weak = ui.as_weak();
    state.on_dismiss_error(move || weak.unwrap().global::<VideoState>().set_error("".into()));
}

//! Scrambling runs on a worker thread that always skips to the newest queued request.

use std::cell::RefCell;
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use engine::analysis::{analyze_planes, ssim_planes, ImageStats};
use engine::matching::{MatchProperty, MatchTarget, Target};
use engine::planes::Planes;
use engine::{ScrambleOptions, ScrambleOutput};
use image::DynamicImage;
use slint::{ComponentHandle, ModelRc, Rgba8Pixel, SharedPixelBuffer, Timer, TimerMode, VecModel};

use crate::{charts, options, AppWindow, AxisTick, ImageState, Params, StatRow};

struct Job {
    generation: u64,
    image: Arc<DynamicImage>,
    original: Option<Arc<(Planes, ImageStats)>>,
    options: ScrambleOptions,
    set_target: Option<Arc<Target>>,
}

/// The average target of the reference folder, for set-average matching.
#[derive(Default)]
struct Reference {
    key: Option<(String, MatchProperty, bool)>,
    target: Option<Arc<Target>>,
    loading: bool,
}

struct Finished {
    output: ScrambleOutput,
    options: ScrambleOptions,
    buffer: SharedPixelBuffer<Rgba8Pixel>,
    original: Arc<(Planes, ImageStats)>,
    stats: ImageStats,
    ssim: Option<f64>,
    elapsed: Duration,
}

#[derive(Default)]
struct Session {
    source: Option<Arc<DynamicImage>>,
    original: Option<Arc<(Planes, ImageStats)>>,
    path: Option<PathBuf>,
    result: Option<(ScrambleOutput, ScrambleOptions, ImageStats)>,
    generation: u64,
    /// Options of the last request, used to detect setting changes for live preview.
    last_options: Option<String>,
    /// Unit the charts were last drawn in.
    chart_unit: Option<String>,
}

thread_local! {
    // Only touched from the UI thread.
    static SESSION: RefCell<Session> = RefCell::default();
    static REFERENCE: RefCell<Reference> = RefCell::default();
}

fn to_pixel_buffer(img: &DynamicImage) -> SharedPixelBuffer<Rgba8Pixel> {
    let rgba = img.to_rgba8();
    SharedPixelBuffer::clone_from_slice(rgba.as_raw(), rgba.width(), rgba.height())
}

fn run_job(job: Job) -> anyhow::Result<Finished> {
    let started = Instant::now();
    let output = engine::scramble_image_with(&job.image, &job.options, job.set_target.as_deref())?;
    let elapsed = started.elapsed();
    let original = job.original.unwrap_or_else(|| {
        let planes = Planes::from_image(&job.image);
        let stats = analyze_planes(&planes);
        Arc::new((planes, stats))
    });
    let planes = Planes::from_image(&output.image);
    let stats = analyze_planes(&planes);
    let ssim = ssim_planes(&original.0, &planes);
    let buffer = to_pixel_buffer(&output.image);
    Ok(Finished { output, options: job.options, buffer, original, stats, ssim, elapsed })
}

fn spawn_worker(ui: slint::Weak<AppWindow>) -> mpsc::Sender<Job> {
    let (tx, rx) = mpsc::channel::<Job>();
    std::thread::spawn(move || {
        while let Ok(mut job) = rx.recv() {
            while let Ok(newer) = rx.try_recv() {
                job = newer;
            }
            let generation = job.generation;
            let outcome = run_job(job).map_err(|e| format!("{e:#}"));
            let _ = ui.upgrade_in_event_loop(move |ui| {
                if SESSION.with_borrow(|s| s.generation != generation) {
                    return;
                }
                let state = ui.global::<ImageState>();
                state.set_busy(false);
                match outcome {
                    Ok(done) => show_result(&ui, done),
                    Err(e) => state.set_error(format!("Scrambling failed: {e}").into()),
                }
            });
        }
    });
    tx
}

fn percent(v: f64) -> String {
    if v > 0.0 && v < 0.001 {
        "<0.1%".into()
    } else {
        format!("{:.1}%", v * 100.0)
    }
}

fn show_result(ui: &AppWindow, done: Finished) {
    let state = ui.global::<ImageState>();
    let report = &done.output.report;
    state.set_result(slint::Image::from_rgba8(done.buffer));
    state.set_has_result(true);
    ui.global::<Params>().set_last_seed(report.seed.to_string().into());

    let mut status = format!("seed {} · {} ms", report.seed, done.elapsed.as_millis());
    if report.contrast_scale < 1.0 {
        status.push_str(&format!(" · contrast ×{:.2}", report.contrast_scale));
    }
    state.set_status(status.into());

    let warning = if report.faces_detected == Some(0) {
        "No faces were found, so the image was left unchanged.".to_string()
    } else if report.clipped_fraction > 0.01 {
        format!(
            "{} of values fell outside 0 to 1 and were clipped, which changes mean and contrast. \"Compress contrast\" avoids this.",
            percent(report.clipped_fraction)
        )
    } else {
        String::new()
    };
    state.set_warning(warning.into());

    let original = &done.original.1;
    let s = &done.stats;
    let row = |label: &str, a: String, b: String| StatRow { label: label.into(), original: a.into(), result: b.into() };
    let rows = vec![
        row("Mean luminance", format!("{:.4}", original.mean_luminance), format!("{:.4}", s.mean_luminance)),
        row("RMS contrast", format!("{:.4}", original.rms_contrast), format!("{:.4}", s.rms_contrast)),
        row("Spectral slope", format!("{:.2}", original.spectral_slope), format!("{:.2}", s.spectral_slope)),
        row("Values at 0 or 1", percent(original.saturated_fraction), percent(s.saturated_fraction)),
        row("Clipped", "-".into(), percent(report.clipped_fraction)),
        row("SSIM to original", "1.000".into(), done.ssim.map(|v| format!("{v:.3}")).unwrap_or("-".into())),
    ];
    state.set_stats(ModelRc::new(VecModel::from(rows)));

    SESSION.with_borrow_mut(|session| {
        session.original = Some(done.original.clone());
        session.result = Some((done.output, done.options, done.stats));
        session.chart_unit = None;
    });
    update_charts(ui);
}

/// Redraws the analysis charts when a result exists and the frequency unit has changed.
fn update_charts(ui: &AppWindow) {
    let unit = options::frequency_unit(ui);
    let key = format!("{unit:?}");
    SESSION.with_borrow_mut(|session| {
        if session.chart_unit.as_ref() == Some(&key) {
            return;
        }
        let (Some(original), Some((_, _, stats))) = (&session.original, &session.result) else { return };
        let state = ui.global::<ImageState>();
        let (ha, hb) = charts::histograms(&original.1, stats);
        let (sa, sb, ticks) = charts::spectra(&original.1, stats, unit);
        state.set_hist_original(ha.into());
        state.set_hist_result(hb.into());
        state.set_spectrum_original(sa.into());
        state.set_spectrum_result(sb.into());
        let ticks: Vec<AxisTick> = ticks.into_iter().map(|t| AxisTick { position: t.position, label: t.label.into() }).collect();
        state.set_spectrum_ticks(ModelRc::new(VecModel::from(ticks)));
        state.set_has_analysis(true);
        session.chart_unit = Some(key);
    });
}

/// Keeps the reference target in step with the reference folder and match property.
/// Returns the target when one is needed and ready.
fn reference_target(ui: &AppWindow) -> Option<Arc<Target>> {
    let matching = options::match_options(ui).filter(|m| m.target == MatchTarget::SetAverage)?;
    let params = ui.global::<Params>();
    let dir = params.get_reference_dir().to_string();
    if dir.is_empty() {
        return None;
    }
    let key = (dir.clone(), matching.property, ui.global::<Params>().get_linear_light());
    REFERENCE.with_borrow_mut(|r| {
        if r.key.as_ref() == Some(&key) {
            return r.target.clone();
        }
        if r.loading {
            return None;
        }
        r.loading = true;
        r.target = None;
        let count = engine::io::list_images(std::path::Path::new(&dir)).map(|f| f.len()).unwrap_or(0);
        params.set_reference_status(format!("Averaging {count} images…").into());
        let weak = ui.as_weak();
        std::thread::spawn(move || {
            let outcome = engine::normalize::set_target(std::path::Path::new(&key.0), key.1, key.2);
            let _ = weak.upgrade_in_event_loop(move |ui| {
                let params = ui.global::<Params>();
                REFERENCE.with_borrow_mut(|r| {
                    r.loading = false;
                    match outcome {
                        Ok(target) => {
                            r.key = Some(key);
                            r.target = Some(Arc::new(target));
                            params.set_reference_status(format!("Average of {count} images").into());
                        }
                        Err(e) => {
                            r.key = Some(key);
                            params.set_reference_status("Reference folder could not be used".into());
                            ui.global::<ImageState>().set_error(format!("{e:#}").into());
                        }
                    }
                });
                // Re-run the preview with the new target.
                SESSION.with_borrow_mut(|s| s.last_options = None);
            });
        });
        None
    })
}

fn submit(ui: &AppWindow, worker: &mpsc::Sender<Job>) {
    let options = options::scramble_options(ui);
    let key = format!("{options:?}");
    let set_target = reference_target(ui);
    let needs_reference = matches!(options.matching, Some(m) if m.target == MatchTarget::SetAverage);
    if needs_reference && set_target.is_none() {
        SESSION.with_borrow_mut(|s| {
            s.last_options = Some(key);
            s.result = None;
            s.generation += 1;
        });
        let state = ui.global::<ImageState>();
        state.set_has_result(false);
        state.set_has_analysis(false);
        state.set_busy(false);
        if ui.global::<Params>().get_reference_dir().is_empty() {
            state.set_warning("Choose a reference folder to match to a set average.".into());
        }
        return;
    }
    let job = SESSION.with_borrow_mut(|s| {
        let image = s.source.clone()?;
        s.generation += 1;
        s.last_options = Some(key);
        Some(Job { generation: s.generation, image, original: s.original.clone(), options, set_target })
    });
    if let Some(job) = job {
        let state = ui.global::<ImageState>();
        state.set_busy(true);
        state.set_error("".into());
        let _ = worker.send(job);
    }
}

fn open_image(ui: &AppWindow) {
    let Some(path) = rfd::FileDialog::new()
        .set_title("Open image")
        .add_filter("Images", engine::io::IMAGE_EXTENSIONS)
        .pick_file()
    else {
        return;
    };
    load_image(ui, path);
}

pub fn load_image(ui: &AppWindow, path: PathBuf) {
    let state = ui.global::<ImageState>();
    match engine::io::open_image(&path) {
        Ok(img) => {
            state.set_original(slint::Image::from_rgba8(to_pixel_buffer(&img)));
            let format = path.extension().map(|e| e.to_string_lossy().to_uppercase()).unwrap_or_default();
            let depth = if Planes::from_image(&img).sixteen_bit { " · 16-bit" } else { "" };
            state.set_dimensions(format!("{} × {} px · {format}{depth}", img.width(), img.height()).into());
            state.set_file_name(path.file_name().unwrap_or_default().to_string_lossy().to_string().into());
            state.set_has_image(true);
            state.set_has_result(false);
            state.set_has_analysis(false);
            state.set_status("".into());
            state.set_warning("".into());
            state.set_error("".into());
            SESSION.with_borrow_mut(|s| {
                s.source = Some(Arc::new(img));
                s.original = None;
                s.path = Some(path);
                s.result = None;
                s.last_options = None;
                s.chart_unit = None;
                // Invalidate any result still being computed for the previous image.
                s.generation += 1;
            });
            state.set_busy(false);
        }
        Err(e) => state.set_error(format!("{e:#}").into()),
    }
}

fn save_result(ui: &AppWindow) {
    let state = ui.global::<ImageState>();
    let default_name = SESSION.with_borrow(|s| {
        s.path
            .as_ref()
            .and_then(|p| p.file_stem())
            .map(|stem| format!("{}_scrambled.png", stem.to_string_lossy()))
            .unwrap_or_else(|| "scrambled.png".to_string())
    });
    let Some(mut path) = rfd::FileDialog::new()
        .set_title("Save scrambled image")
        .set_file_name(default_name)
        .add_filter("PNG", &["png"])
        .add_filter("TIFF", &["tif", "tiff"])
        .save_file()
    else {
        return;
    };
    if path.extension().is_none() {
        path.set_extension("png");
    }

    let saved = SESSION.with_borrow(|s| {
        s.result
            .as_ref()
            .map(|(output, options, _)| engine::save_scrambled(output, options, s.path.as_deref(), &path))
    });
    match saved {
        Some(Ok(())) => {
            let settings = engine::settings_path(&path);
            let name = settings.file_name().unwrap_or_default().to_string_lossy().to_string();
            state.set_status(format!("Saved with settings in {name}").into());
        }
        Some(Err(e)) => state.set_error(format!("{e:#}").into()),
        None => {}
    }
}

/// The returned timer drives live preview and must be kept alive.
pub fn setup(ui: &AppWindow) -> Timer {
    let worker = spawn_worker(ui.as_weak());
    let state = ui.global::<ImageState>();

    let weak = ui.as_weak();
    state.on_open_image(move || open_image(&weak.unwrap()));

    let weak = ui.as_weak();
    state.on_save(move || save_result(&weak.unwrap()));

    let weak = ui.as_weak();
    state.on_dismiss_error(move || weak.unwrap().global::<ImageState>().set_error("".into()));

    let weak = ui.as_weak();
    let tx = worker.clone();
    state.on_scramble(move || submit(&weak.unwrap(), &tx));

    let weak = ui.as_weak();
    ui.global::<Params>().on_pick_reference(move || {
        if let Some(dir) = rfd::FileDialog::new().set_title("Select reference folder").pick_folder() {
            let ui = weak.unwrap();
            ui.global::<Params>().set_reference_dir(dir.display().to_string().into());
            ui.global::<ImageState>().set_warning("".into());
            reference_target(&ui);
        }
    });

    let timer = Timer::default();
    let weak = ui.as_weak();
    timer.start(TimerMode::Repeated, Duration::from_millis(150), move || {
        let Some(ui) = weak.upgrade() else { return };
        update_charts(&ui);
        if !ui.global::<ImageState>().get_live_preview() {
            return;
        }
        let key = format!("{:?}", options::scramble_options(&ui));
        let changed = SESSION.with_borrow(|s| s.source.is_some() && s.last_options.as_ref() != Some(&key));
        if changed {
            submit(&ui, &worker);
        }
    });
    timer
}

//! Scrambling runs on a worker thread that always skips to the newest queued request.

use std::cell::RefCell;
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use engine::ScrambleOptions;
use image::DynamicImage;
use slint::{ComponentHandle, Rgba8Pixel, SharedPixelBuffer, Timer, TimerMode};

use crate::{options, AppWindow, ImageState};

const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "webp", "tif", "tiff", "bmp", "gif"];

struct Job {
    generation: u64,
    image: Arc<DynamicImage>,
    options: ScrambleOptions,
}

#[derive(Default)]
struct Session {
    source: Option<Arc<DynamicImage>>,
    path: Option<PathBuf>,
    result: Option<DynamicImage>,
    generation: u64,
    /// Options of the last request, used to detect setting changes for live preview.
    last_options: Option<String>,
}

thread_local! {
    // Only touched from the UI thread.
    static SESSION: RefCell<Session> = RefCell::default();
}

fn to_pixel_buffer(img: &DynamicImage) -> SharedPixelBuffer<Rgba8Pixel> {
    let rgba = img.to_rgba8();
    SharedPixelBuffer::clone_from_slice(rgba.as_raw(), rgba.width(), rgba.height())
}

fn spawn_worker(ui: slint::Weak<AppWindow>) -> mpsc::Sender<Job> {
    let (tx, rx) = mpsc::channel::<Job>();
    std::thread::spawn(move || {
        while let Ok(mut job) = rx.recv() {
            while let Ok(newer) = rx.try_recv() {
                job = newer;
            }
            let started = Instant::now();
            let outcome = engine::scramble_image(&job.image, &job.options)
                .map(|img| {
                    let buffer = to_pixel_buffer(&img);
                    (img, buffer)
                })
                .map_err(|e| e.to_string());
            let elapsed = started.elapsed();
            let generation = job.generation;

            let _ = ui.upgrade_in_event_loop(move |ui| {
                let is_latest = SESSION.with_borrow(|s| s.generation == generation);
                if !is_latest {
                    return;
                }
                let state = ui.global::<ImageState>();
                state.set_busy(false);
                match outcome {
                    Ok((img, buffer)) => {
                        state.set_result(slint::Image::from_rgba8(buffer));
                        state.set_has_result(true);
                        state.set_status(format!("{} ms", elapsed.as_millis()).into());
                        SESSION.with_borrow_mut(|s| s.result = Some(img));
                    }
                    Err(e) => state.set_error(format!("Scrambling failed: {e}").into()),
                }
            });
        }
    });
    tx
}

fn submit(ui: &AppWindow, worker: &mpsc::Sender<Job>) {
    let options = options::scramble_options(ui);
    let key = format!("{options:?}");
    let job = SESSION.with_borrow_mut(|s| {
        let image = s.source.clone()?;
        s.generation += 1;
        s.last_options = Some(key);
        Some(Job { generation: s.generation, image, options })
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
        .add_filter("Images", IMAGE_EXTENSIONS)
        .pick_file()
    else {
        return;
    };
    load_image(ui, path);
}

pub fn load_image(ui: &AppWindow, path: PathBuf) {
    let state = ui.global::<ImageState>();
    match image::open(&path) {
        Ok(img) => {
            state.set_original(slint::Image::from_rgba8(to_pixel_buffer(&img)));
            let format = path.extension().map(|e| e.to_string_lossy().to_uppercase()).unwrap_or_default();
            state.set_dimensions(format!("{} × {} px · {format}", img.width(), img.height()).into());
            state.set_file_name(path.file_name().unwrap_or_default().to_string_lossy().to_string().into());
            state.set_has_image(true);
            state.set_has_result(false);
            state.set_status("".into());
            state.set_error("".into());
            SESSION.with_borrow_mut(|s| {
                s.source = Some(Arc::new(img));
                s.path = Some(path);
                s.result = None;
                s.last_options = None;
                // Invalidate any result still being computed for the previous image.
                s.generation += 1;
            });
            state.set_busy(false);
        }
        Err(e) => state.set_error(format!("Could not open {}: {e}", path.display()).into()),
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

    let saved = SESSION.with_borrow(|s| s.result.as_ref().map(|img| img.save(&path)));
    match saved {
        Some(Ok(())) => state.set_status(format!("Saved to {}", path.display()).into()),
        Some(Err(e)) => state.set_error(format!("Could not save {}: {e}", path.display()).into()),
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

    let timer = Timer::default();
    let weak = ui.as_weak();
    timer.start(TimerMode::Repeated, Duration::from_millis(150), move || {
        let Some(ui) = weak.upgrade() else { return };
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

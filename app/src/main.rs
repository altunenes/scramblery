// Prevents an additional console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod batch_page;
mod charts;
mod image_page;
mod normalize_page;
mod options;
mod overview;
mod recipe;
mod video_page;

use slint::ComponentHandle;

slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info,ort=error")).init();

    let ui = AppWindow::new()?;

    let info = ui.global::<AppInfo>();
    info.set_version(env!("CARGO_PKG_VERSION").into());
    info.on_open_url(|url| {
        if let Err(e) = open::that(url.as_str()) {
            log::error!("Failed to open {url}: {e}");
        }
    });

    let _live_preview = image_page::setup(&ui);
    batch_page::setup(&ui);
    video_page::setup(&ui);
    normalize_page::setup(&ui);
    recipe::setup(&ui, |ui, message| {
        let message = slint::SharedString::from(message);
        match ui.get_page() {
            1 => ui.global::<BatchState>().set_error(message),
            2 => ui.global::<VideoState>().set_error(message),
            3 => ui.global::<NormalizeState>().set_error(message),
            _ => ui.global::<ImageState>().set_error(message),
        }
    });

    ui.run()
}

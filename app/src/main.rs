// Prevents an additional console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod batch_page;
mod image_page;
mod options;
mod video_page;

use slint::ComponentHandle;

slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

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

    ui.run()
}

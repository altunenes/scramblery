use engine::scramble::ScrambleOptions;
use engine::batch::{BatchProcessingOptions, ProcessingResult};
use engine::video::VideoProcessingOptions;
use tauri::Emitter;
#[tauri::command]
async fn scramble_image(image_data: String, options: ScrambleOptions) -> Result<String, String> {
    engine::process_base64_image(&image_data, &options).map_err(|e| e.to_string())
}

#[tauri::command]
async fn process_directory(options: BatchProcessingOptions, window: tauri::Window) -> Result<Vec<ProcessingResult>, String> {
    engine::batch::process_directory_with_progress(
        &options,
        move |progress| {
            let _ = window.emit("batch-progress", progress);
        }
    ).map_err(|e| e.to_string())
}

#[tauri::command]
async fn process_video(options: VideoProcessingOptions, window: tauri::Window) -> Result<(), String> {
    engine::video::process_video(&options, move |progress| {
        let _ = window.emit("video-progress", progress);
    })
    .map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            scramble_image,
            process_directory,
            process_video 
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
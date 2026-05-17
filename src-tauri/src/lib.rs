pub mod config;
pub mod errors;
pub mod media;
pub mod models;
pub mod platform;
pub mod storage;
pub mod task;

pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("failed to run video downloader");
}

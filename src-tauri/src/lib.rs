pub mod config;
pub mod errors;
pub mod models;
pub mod storage;

pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("failed to run video downloader");
}

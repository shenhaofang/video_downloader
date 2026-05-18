pub mod auth;
pub mod commands;
pub mod config;
pub mod errors;
pub mod media;
pub mod models;
pub mod platform;
pub mod storage;
pub mod task;

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::get_config,
            commands::create_task,
            commands::list_platform_logins,
            commands::start_bilibili_login,
            commands::clear_bilibili_login,
            commands::get_tool_status
        ])
        .run(tauri::generate_context!())
        .expect("failed to run video downloader");
}

pub mod app_state;
pub mod auth;
pub mod commands;
pub mod config;
pub mod errors;
pub mod media;
pub mod models;
pub mod platform;
pub mod storage;
pub mod task;

use std::path::PathBuf;

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            use tauri::Manager;

            let handle = app.handle().clone();
            let dir = handle.path().app_data_dir()?;
            let state = init_app_state(dir)?;
            handle.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_config,
            commands::save_config,
            commands::create_task,
            commands::list_task_groups,
            commands::run_task,
            commands::list_platform_logins,
            commands::start_bilibili_login,
            commands::poll_bilibili_login,
            commands::clear_bilibili_login,
            commands::get_tool_status
        ])
        .run(tauri::generate_context!())
        .expect("failed to run video downloader");
}

fn init_app_state(data_dir: PathBuf) -> Result<app_state::AppState, Box<dyn std::error::Error>> {
    Ok(tauri::async_runtime::block_on(app_state::AppState::new(
        data_dir,
    ))?)
}

#[cfg(test)]
mod tests {
    use super::init_app_state;
    use std::fs;

    #[test]
    fn init_app_state_returns_error_for_invalid_data_dir() {
        let dir =
            std::env::temp_dir().join(format!("vd-invalid-app-state-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        let file_path = dir.join("data-file");
        fs::write(&file_path, b"not a directory").unwrap();

        let result = init_app_state(file_path);

        assert!(result.is_err());
        fs::remove_dir_all(dir).unwrap();
    }
}

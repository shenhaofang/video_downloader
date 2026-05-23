use crate::errors::{AppError, AppResult, ErrorCode};
use serde::Serialize;
use tauri::AppHandle;
use tauri_plugin_updater::UpdaterExt;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct AppUpdateStatus {
    pub available: bool,
    pub current_version: String,
    pub latest_version: Option<String>,
    pub notes: Option<String>,
    pub pub_date: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppUpdateMetadata {
    pub version: String,
    pub notes: Option<String>,
    pub pub_date: Option<String>,
}

pub async fn check_app_update(app: &AppHandle) -> AppResult<AppUpdateStatus> {
    let current_version = app.package_info().version.to_string();
    let update = app
        .updater()
        .map_err(update_error)?
        .check()
        .await
        .map_err(update_error)?
        .map(|update| AppUpdateMetadata {
            version: update.version,
            notes: update.body,
            pub_date: update.date.map(|date| date.to_string()),
        });

    Ok(status_from_update(current_version, update))
}

pub async fn install_app_update(app: &AppHandle) -> AppResult<()> {
    let update = app
        .updater()
        .map_err(update_error)?
        .check()
        .await
        .map_err(update_error)?
        .ok_or_else(|| AppError::structured(ErrorCode::UpdateError, "no update available"))?;

    update
        .download_and_install(|_, _| {}, || {})
        .await
        .map_err(update_error)?;
    app.restart()
}

pub fn status_from_update(
    current_version: impl Into<String>,
    update: Option<AppUpdateMetadata>,
) -> AppUpdateStatus {
    let current_version = current_version.into();
    match update {
        Some(update) => AppUpdateStatus {
            available: true,
            current_version,
            latest_version: Some(update.version),
            notes: update.notes,
            pub_date: update.pub_date,
        },
        None => AppUpdateStatus {
            available: false,
            current_version,
            latest_version: None,
            notes: None,
            pub_date: None,
        },
    }
}

fn update_error(error: tauri_plugin_updater::Error) -> AppError {
    AppError::structured(ErrorCode::UpdateError, error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_from_update_reports_available_release() {
        let status = status_from_update(
            "0.1.0",
            Some(AppUpdateMetadata {
                version: "0.1.1".into(),
                notes: Some("修复下载恢复".into()),
                pub_date: Some("2026-05-23T00:00:00Z".into()),
            }),
        );

        assert_eq!(
            status,
            AppUpdateStatus {
                available: true,
                current_version: "0.1.0".into(),
                latest_version: Some("0.1.1".into()),
                notes: Some("修复下载恢复".into()),
                pub_date: Some("2026-05-23T00:00:00Z".into()),
            }
        );
    }

    #[test]
    fn status_from_update_reports_current_release() {
        let status = status_from_update("0.1.0", None);

        assert_eq!(
            status,
            AppUpdateStatus {
                available: false,
                current_version: "0.1.0".into(),
                latest_version: None,
                notes: None,
                pub_date: None,
            }
        );
    }
}

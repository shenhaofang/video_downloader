use crate::errors::{AppError, AppResult, ErrorCode};
use serde::Serialize;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter};
use tauri_plugin_updater::UpdaterExt;

pub const APP_UPDATE_PROGRESS_EVENT: &str = "app-update-progress";

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

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct AppUpdateProgress {
    pub downloaded: u64,
    pub total: Option<u64>,
    pub percent: Option<u8>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct AppUpdateProgressTracker {
    downloaded: u64,
    total: Option<u64>,
}

impl AppUpdateProgressTracker {
    pub fn record_chunk(
        &mut self,
        chunk_length: usize,
        content_length: Option<u64>,
    ) -> AppUpdateProgress {
        self.downloaded = self.downloaded.saturating_add(chunk_length as u64);
        self.total = content_length.or(self.total);
        app_update_progress(self.downloaded, self.total)
    }

    pub fn finish(&self) -> AppUpdateProgress {
        let downloaded = self.total.unwrap_or(self.downloaded).max(self.downloaded);
        app_update_progress(downloaded, self.total.or(Some(downloaded)))
    }
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

    let progress = Arc::new(Mutex::new(AppUpdateProgressTracker::default()));
    let progress_app = app.clone();
    let progress_state = Arc::clone(&progress);
    let finish_app = app.clone();
    let finish_state = Arc::clone(&progress);

    update
        .download_and_install(
            move |chunk_length, content_length| {
                if let Ok(mut tracker) = progress_state.lock() {
                    let payload = tracker.record_chunk(chunk_length, content_length);
                    let _ = progress_app.emit(APP_UPDATE_PROGRESS_EVENT, payload);
                }
            },
            move || {
                if let Ok(tracker) = finish_state.lock() {
                    let _ = finish_app.emit(APP_UPDATE_PROGRESS_EVENT, tracker.finish());
                }
            },
        )
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

fn app_update_progress(downloaded: u64, total: Option<u64>) -> AppUpdateProgress {
    let percent = total.filter(|total| *total > 0).map(|total| {
        let value = ((downloaded as u128) * 100 / (total as u128)).min(100);
        value as u8
    });
    AppUpdateProgress {
        downloaded,
        total,
        percent,
    }
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

    #[test]
    fn update_progress_tracker_accumulates_chunks_and_percent() {
        let mut tracker = AppUpdateProgressTracker::default();

        let first = tracker.record_chunk(25, Some(100));
        let second = tracker.record_chunk(25, Some(100));

        assert_eq!(
            first,
            AppUpdateProgress {
                downloaded: 25,
                total: Some(100),
                percent: Some(25),
            }
        );
        assert_eq!(
            second,
            AppUpdateProgress {
                downloaded: 50,
                total: Some(100),
                percent: Some(50),
            }
        );
    }

    #[test]
    fn update_progress_tracker_reports_finished_download() {
        let mut tracker = AppUpdateProgressTracker::default();
        tracker.record_chunk(40, Some(100));

        assert_eq!(
            tracker.finish(),
            AppUpdateProgress {
                downloaded: 100,
                total: Some(100),
                percent: Some(100),
            }
        );
    }
}

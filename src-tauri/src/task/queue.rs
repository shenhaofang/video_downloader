use crate::models::DownloadTask;

pub fn should_auto_retry(task: &DownloadTask) -> bool {
    matches!(task.error_code.as_deref(), Some("network_error"))
        && task.retry_count < task.max_retries
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{DownloadEngine, DownloadTask, TaskState};
    use uuid::Uuid;

    fn failed_task(code: &str, retry_count: u8) -> DownloadTask {
        DownloadTask {
            id: Uuid::new_v4(),
            group_id: Uuid::new_v4(),
            title: "sample".into(),
            output_file: "sample.mp4".into(),
            state: TaskState::Failed,
            engine: DownloadEngine::Native,
            quality: None,
            used_login: false,
            bytes_downloaded: 0,
            bytes_total: None,
            retry_count,
            max_retries: 3,
            error_code: Some(code.into()),
            error_message: Some("failed".into()),
            bvid: None,
            cid: None,
            page: None,
        }
    }

    #[test]
    fn network_error_under_retry_limit_retries() {
        assert!(should_auto_retry(&failed_task("network_error", 2)));
    }

    #[test]
    fn network_error_at_retry_limit_does_not_retry() {
        assert!(!should_auto_retry(&failed_task("network_error", 3)));
    }

    #[test]
    fn login_required_does_not_retry() {
        assert!(!should_auto_retry(&failed_task("login_required", 0)));
    }
}

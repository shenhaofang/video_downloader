use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    NetworkError,
    LoginRequired,
    LoginExpired,
    PermissionDenied,
    UnsupportedContent,
    EngineMissing,
    FfmpegError,
    FilesystemError,
    PlatformChanged,
    UpdateError,
    UnknownError,
}

#[derive(Debug, Error)]
pub enum AppError {
    #[error("{code:?}: {message}")]
    Structured { code: ErrorCode, message: String },
}

impl AppError {
    pub fn structured(code: ErrorCode, message: impl Into<String>) -> Self {
        Self::Structured {
            code,
            message: message.into(),
        }
    }

    pub fn code(&self) -> ErrorCode {
        match self {
            Self::Structured { code, .. } => *code,
        }
    }
}

impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            AppError::Structured { code, message } => {
                #[derive(Serialize)]
                struct WireError<'a> {
                    code: ErrorCode,
                    message: &'a str,
                }
                WireError {
                    code: *code,
                    message,
                }
                .serialize(serializer)
            }
        }
    }
}

pub type AppResult<T> = Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_error_code_for_frontend() {
        let err = AppError::structured(
            ErrorCode::UnsupportedContent,
            "native cannot expand this link",
        );
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("unsupported_content"));
        assert!(json.contains("native cannot expand this link"));
    }

    #[test]
    fn serializes_update_error_code_for_frontend() {
        let err = AppError::structured(ErrorCode::UpdateError, "update failed");
        let json = serde_json::to_string(&err).unwrap();

        assert!(json.contains("update_error"));
        assert!(json.contains("update failed"));
    }
}

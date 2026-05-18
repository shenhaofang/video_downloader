use super::session_store::{SessionStore, StoredSession};
use crate::errors::AppResult;
use chrono::Utc;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LoginQr {
    pub qrcode_key: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LoginStatus {
    pub platform: String,
    pub status: String,
}

#[derive(Clone)]
pub struct BilibiliAuth {
    store: SessionStore,
}

impl BilibiliAuth {
    pub fn new(store: SessionStore) -> Self {
        Self { store }
    }

    pub fn create_mock_qr(&self) -> LoginQr {
        LoginQr {
            qrcode_key: "mock-qrcode-key".into(),
            url: "https://passport.bilibili.com/qrcode/mock".into(),
        }
    }

    pub fn save_cookie_string(&self, cookies: String) -> AppResult<()> {
        self.store.save(&StoredSession {
            platform: "bilibili".into(),
            cookies,
            expires_at: None,
            last_verified_at: Some(Utc::now().to_rfc3339()),
        })
    }

    pub fn load_cookie_string(&self) -> AppResult<Option<String>> {
        Ok(self.store.load("bilibili")?.map(|session| session.cookies))
    }

    pub fn status(&self) -> AppResult<LoginStatus> {
        let status = if self.load_cookie_string()?.is_some() {
            "已登录"
        } else {
            "未登录"
        };
        Ok(LoginStatus {
            platform: "bilibili".into(),
            status: status.into(),
        })
    }

    pub fn clear(&self) -> AppResult<()> {
        self.store.clear("bilibili")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn unique_temp_dir() -> std::path::PathBuf {
        std::env::temp_dir().join(format!("vd-bilibili-auth-{}", uuid::Uuid::new_v4()))
    }

    #[test]
    fn bilibili_auth_saves_loads_and_clears_cookie_string() {
        let dir = unique_temp_dir();
        let auth = BilibiliAuth::new(SessionStore::new(dir.clone()));

        auth.save_cookie_string("SESSDATA=auth-cookie".into())
            .unwrap();
        assert_eq!(
            auth.load_cookie_string().unwrap(),
            Some("SESSDATA=auth-cookie".into())
        );

        auth.clear().unwrap();
        assert!(auth.load_cookie_string().unwrap().is_none());

        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn mock_qr_has_key_and_bilibili_passport_url() {
        let dir = unique_temp_dir();
        let auth = BilibiliAuth::new(SessionStore::new(dir.clone()));

        let qr = auth.create_mock_qr();

        assert_eq!(qr.qrcode_key, "mock-qrcode-key");
        assert!(qr.url.starts_with("https://passport.bilibili.com/"));

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn status_reports_logged_in_only_when_cookie_exists() {
        let dir = unique_temp_dir();
        let auth = BilibiliAuth::new(SessionStore::new(dir.clone()));

        assert_eq!(
            auth.status().unwrap(),
            LoginStatus {
                platform: "bilibili".into(),
                status: "未登录".into(),
            }
        );

        auth.save_cookie_string("SESSDATA=auth-cookie".into())
            .unwrap();

        assert_eq!(
            auth.status().unwrap(),
            LoginStatus {
                platform: "bilibili".into(),
                status: "已登录".into(),
            }
        );

        fs::remove_dir_all(dir).unwrap();
    }
}

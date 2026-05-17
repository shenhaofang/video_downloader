use super::session_store::{SessionStore, StoredSession};
use crate::errors::AppResult;
use chrono::Utc;

#[derive(Clone)]
pub struct BilibiliAuth {
    store: SessionStore,
}

impl BilibiliAuth {
    pub fn new(store: SessionStore) -> Self {
        Self { store }
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
}

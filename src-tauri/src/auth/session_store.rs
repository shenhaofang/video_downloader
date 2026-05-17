use crate::errors::{AppError, AppResult, ErrorCode};
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use base64::{engine::general_purpose, Engine};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::{fmt, fs, io};

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoredSession {
    pub platform: String,
    pub cookies: String,
    pub expires_at: Option<String>,
    pub last_verified_at: Option<String>,
}

impl fmt::Debug for StoredSession {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("StoredSession")
            .field("platform", &self.platform)
            .field("cookies", &"<redacted>")
            .field("expires_at", &self.expires_at)
            .field("last_verified_at", &self.last_verified_at)
            .finish()
    }
}

#[derive(Debug, Clone)]
pub struct SessionStore {
    dir: PathBuf,
}

impl SessionStore {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    pub fn save(&self, session: &StoredSession) -> AppResult<()> {
        fs::create_dir_all(&self.dir)
            .map_err(|err| AppError::structured(ErrorCode::FilesystemError, err.to_string()))?;
        let key = self.load_or_create_key()?;
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key));
        let mut nonce_bytes = [0_u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        let plaintext = serde_json::to_vec(session)
            .map_err(|err| AppError::structured(ErrorCode::UnknownError, err.to_string()))?;
        let ciphertext = cipher.encrypt(nonce, plaintext.as_ref()).map_err(|_| {
            AppError::structured(ErrorCode::UnknownError, "failed to encrypt session")
        })?;
        let payload = format!(
            "{}:{}",
            general_purpose::STANDARD.encode(nonce_bytes),
            general_purpose::STANDARD.encode(ciphertext)
        );
        fs::write(self.session_path(&session.platform), payload)
            .map_err(|err| AppError::structured(ErrorCode::FilesystemError, err.to_string()))?;
        Ok(())
    }

    pub fn load(&self, platform: &str) -> AppResult<Option<StoredSession>> {
        let path = self.session_path(platform);
        if !path.exists() {
            return Ok(None);
        }

        let payload = match fs::read_to_string(&path) {
            Ok(payload) => payload,
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(err) => {
                return Err(AppError::structured(
                    ErrorCode::FilesystemError,
                    err.to_string(),
                ));
            }
        };
        let (nonce_text, cipher_text) = payload
            .split_once(':')
            .ok_or_else(|| AppError::structured(ErrorCode::LoginExpired, "invalid session file"))?;
        let nonce_bytes = general_purpose::STANDARD
            .decode(nonce_text)
            .map_err(|_| AppError::structured(ErrorCode::LoginExpired, "invalid session nonce"))?;
        if nonce_bytes.len() != 12 {
            return Err(AppError::structured(
                ErrorCode::LoginExpired,
                "invalid session nonce",
            ));
        }
        let ciphertext = general_purpose::STANDARD.decode(cipher_text).map_err(|_| {
            AppError::structured(ErrorCode::LoginExpired, "invalid session payload")
        })?;
        let key = self.load_or_create_key()?;
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key));
        let plaintext = cipher
            .decrypt(Nonce::from_slice(&nonce_bytes), ciphertext.as_ref())
            .map_err(|_| {
                AppError::structured(ErrorCode::LoginExpired, "failed to decrypt session")
            })?;
        let session: StoredSession = serde_json::from_slice(&plaintext)
            .map_err(|_| AppError::structured(ErrorCode::LoginExpired, "invalid session json"))?;
        if session.platform != platform {
            return Err(AppError::structured(
                ErrorCode::LoginExpired,
                "session platform mismatch",
            ));
        }
        Ok(Some(session))
    }

    pub fn clear(&self, platform: &str) -> AppResult<()> {
        let path = self.session_path(platform);
        if path.exists() {
            fs::remove_file(path)
                .map_err(|err| AppError::structured(ErrorCode::FilesystemError, err.to_string()))?;
        }
        Ok(())
    }

    fn session_path(&self, platform: &str) -> PathBuf {
        self.dir
            .join(format!("{}.session.enc", safe_platform_file_stem(platform)))
    }

    fn key_path(&self) -> PathBuf {
        self.dir.join("local.key")
    }

    fn load_or_create_key(&self) -> AppResult<[u8; 32]> {
        fs::create_dir_all(&self.dir)
            .map_err(|err| AppError::structured(ErrorCode::FilesystemError, err.to_string()))?;
        let path = self.key_path();
        if path.exists() {
            let bytes = fs::read(path)
                .map_err(|err| AppError::structured(ErrorCode::FilesystemError, err.to_string()))?;
            if bytes.len() != 32 {
                return Err(AppError::structured(
                    ErrorCode::LoginExpired,
                    "invalid local session key",
                ));
            }
            let mut key = [0_u8; 32];
            key.copy_from_slice(&bytes);
            return Ok(key);
        }

        let mut key = [0_u8; 32];
        rand::thread_rng().fill_bytes(&mut key);
        fs::write(path, key)
            .map_err(|err| AppError::structured(ErrorCode::FilesystemError, err.to_string()))?;
        Ok(key)
    }
}

fn safe_platform_file_stem(platform: &str) -> String {
    let stem: String = platform
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect();
    if stem.is_empty() {
        "session".to_string()
    } else {
        stem
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::ErrorCode;
    use std::fs;

    fn unique_temp_dir() -> PathBuf {
        std::env::temp_dir().join(format!("vd-session-{}", uuid::Uuid::new_v4()))
    }

    fn bilibili_session() -> StoredSession {
        StoredSession {
            platform: "bilibili".into(),
            cookies: "SESSDATA=secret-cookie".into(),
            expires_at: None,
            last_verified_at: Some("2026-05-17T00:00:00Z".into()),
        }
    }

    #[test]
    fn round_trips_encrypted_session() {
        let dir = unique_temp_dir();
        let store = SessionStore::new(dir.clone());
        let session = bilibili_session();

        store.save(&session).unwrap();

        let raw = fs::read_to_string(dir.join("bilibili.session.enc")).unwrap();
        assert!(!raw.contains(&session.cookies));

        let loaded = store.load("bilibili").unwrap().unwrap();
        assert_eq!(loaded.cookies, session.cookies);

        store.clear("bilibili").unwrap();
        assert!(store.load("bilibili").unwrap().is_none());

        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn load_missing_session_returns_none() {
        let dir = unique_temp_dir();
        let store = SessionStore::new(dir.clone());

        assert!(store.load("bilibili").unwrap().is_none());

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn corrupt_session_file_returns_login_expired() {
        let dir = unique_temp_dir();
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("bilibili.session.enc"), "not-a-valid-session").unwrap();
        let store = SessionStore::new(dir.clone());

        let err = store.load("bilibili").unwrap_err();

        assert_eq!(err.code(), ErrorCode::LoginExpired);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn malformed_key_file_does_not_panic() {
        let dir = unique_temp_dir();
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("local.key"), [1_u8, 2, 3]).unwrap();
        fs::write(dir.join("bilibili.session.enc"), "invalid").unwrap();
        let store = SessionStore::new(dir.clone());

        let result = std::panic::catch_unwind(|| store.load("bilibili"));

        assert!(result.is_ok());
        let err = result.unwrap().unwrap_err();
        assert!(matches!(
            err.code(),
            ErrorCode::LoginExpired | ErrorCode::FilesystemError
        ));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn session_path_sanitizes_platform_file_name() {
        let dir = unique_temp_dir();
        let store = SessionStore::new(dir.clone());

        let path = store.session_path("..\\evil/platform");

        assert!(path.starts_with(&dir));
        assert_eq!(path.parent(), Some(dir.as_path()));
        assert_ne!(
            path.file_name().and_then(|name| name.to_str()),
            Some("bilibili.session.enc")
        );
        assert_eq!(
            store.session_path("bilibili"),
            dir.join("bilibili.session.enc")
        );
    }

    #[test]
    fn unreadable_session_path_returns_filesystem_error() {
        let dir = unique_temp_dir();
        let store = SessionStore::new(dir.clone());
        fs::create_dir_all(store.session_path("bilibili")).unwrap();

        let err = store.load("bilibili").unwrap_err();

        assert_eq!(err.code(), ErrorCode::FilesystemError);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn rejects_session_payload_for_wrong_platform() {
        let dir = unique_temp_dir();
        let store = SessionStore::new(dir.clone());
        let session = bilibili_session();
        store.save(&session).unwrap();
        let raw = fs::read(store.session_path("bilibili")).unwrap();
        fs::write(store.session_path("other"), raw).unwrap();

        let err = store.load("other").unwrap_err();

        assert_eq!(err.code(), ErrorCode::LoginExpired);
        fs::remove_dir_all(dir).unwrap();
    }
}

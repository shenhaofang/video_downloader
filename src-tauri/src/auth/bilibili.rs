use super::session_store::{SessionStore, StoredSession};
use crate::errors::{AppError, AppResult, ErrorCode};
use chrono::Utc;
use reqwest::header::{HeaderMap, SET_COOKIE};
use reqwest::Url;
use serde::{Deserialize, Serialize};

const QR_GENERATE_URL: &str = "https://passport.bilibili.com/x/passport-login/web/qrcode/generate";
const QR_POLL_URL: &str = "https://passport.bilibili.com/x/passport-login/web/qrcode/poll";

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LoginPollResult {
    pub status: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginPollOutcome {
    pub result: LoginPollResult,
    pub cookies: Option<String>,
}

#[derive(Debug, Deserialize)]
struct QrGenerateResponse {
    code: i32,
    message: String,
    data: Option<QrGenerateData>,
}

#[derive(Debug, Deserialize)]
struct QrGenerateData {
    url: String,
    qrcode_key: String,
}

#[derive(Debug, Deserialize)]
struct QrPollResponse {
    code: i32,
    message: String,
    data: Option<QrPollData>,
}

#[derive(Debug, Deserialize)]
struct QrPollData {
    code: i32,
    message: String,
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

pub fn parse_qr_generate(json: &str) -> AppResult<LoginQr> {
    let parsed: QrGenerateResponse = serde_json::from_str(json)
        .map_err(|err| AppError::structured(ErrorCode::PlatformChanged, err.to_string()))?;
    if parsed.code != 0 {
        return Err(AppError::structured(
            ErrorCode::PlatformChanged,
            parsed.message,
        ));
    }
    let data = parsed
        .data
        .ok_or_else(|| AppError::structured(ErrorCode::PlatformChanged, "missing QR login data"))?;

    Ok(LoginQr {
        qrcode_key: data.qrcode_key,
        url: data.url,
    })
}

pub fn parse_qr_poll(json: &str) -> AppResult<LoginPollResult> {
    let parsed: QrPollResponse = serde_json::from_str(json)
        .map_err(|err| AppError::structured(ErrorCode::PlatformChanged, err.to_string()))?;
    if parsed.code != 0 {
        return Err(AppError::structured(
            ErrorCode::PlatformChanged,
            parsed.message,
        ));
    }
    let data = parsed
        .data
        .ok_or_else(|| AppError::structured(ErrorCode::PlatformChanged, "missing QR poll data"))?;
    let status = match data.code {
        0 => "confirmed",
        86090 => "scanned",
        86101 => "pending",
        86038 => "expired",
        _ => {
            return Err(AppError::structured(
                ErrorCode::PlatformChanged,
                format!("unknown QR poll status {}", data.code),
            ));
        }
    };
    let message = if data.message.is_empty() && data.code == 0 {
        "登录成功".to_string()
    } else {
        data.message
    };

    Ok(LoginPollResult {
        status: status.into(),
        message,
    })
}

pub async fn request_login_qr(client: &reqwest::Client) -> AppResult<LoginQr> {
    request_login_qr_from_url(client, QR_GENERATE_URL).await
}

async fn request_login_qr_from_url(client: &reqwest::Client, url: &str) -> AppResult<LoginQr> {
    let text = client
        .get(url)
        .send()
        .await
        .map_err(|err| AppError::structured(ErrorCode::NetworkError, err.to_string()))?
        .error_for_status()
        .map_err(|err| AppError::structured(ErrorCode::NetworkError, err.to_string()))?
        .text()
        .await
        .map_err(|err| AppError::structured(ErrorCode::NetworkError, err.to_string()))?;

    parse_qr_generate(&text)
}

pub async fn poll_login_qr(
    client: &reqwest::Client,
    qrcode_key: &str,
) -> AppResult<LoginPollOutcome> {
    let mut url = Url::parse(QR_POLL_URL)
        .map_err(|err| AppError::structured(ErrorCode::UnknownError, err.to_string()))?;
    url.query_pairs_mut().append_pair("qrcode_key", qrcode_key);
    poll_login_qr_from_url(client, url.as_str()).await
}

async fn poll_login_qr_from_url(
    client: &reqwest::Client,
    url: &str,
) -> AppResult<LoginPollOutcome> {
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|err| AppError::structured(ErrorCode::NetworkError, err.to_string()))?
        .error_for_status()
        .map_err(|err| AppError::structured(ErrorCode::NetworkError, err.to_string()))?;
    let cookies = cookie_string_from_set_cookie_headers(response.headers());
    let text = response
        .text()
        .await
        .map_err(|err| AppError::structured(ErrorCode::NetworkError, err.to_string()))?;
    let result = parse_qr_poll(&text)?;

    if result.status == "confirmed" && cookies.is_none() {
        return Err(AppError::structured(
            ErrorCode::PlatformChanged,
            "missing login cookies",
        ));
    }

    Ok(LoginPollOutcome { result, cookies })
}

fn cookie_string_from_set_cookie_headers(headers: &HeaderMap) -> Option<String> {
    let cookies = headers
        .get_all(SET_COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .filter_map(|value| value.split(';').next())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();

    if cookies.is_empty() {
        None
    } else {
        Some(cookies.join("; "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::header::{HeaderMap, HeaderValue, SET_COOKIE};
    use std::fs;
    use std::io::{Read, Write};
    use std::net::TcpListener;

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
    fn parses_qr_generate_response() {
        let json = r#"{"code":0,"message":"OK","data":{"url":"https://account.bilibili.com/h5/account-h5/auth/scan-web?qrcode_key=abc","qrcode_key":"abc"}}"#;

        let qr = parse_qr_generate(json).unwrap();

        assert_eq!(qr.qrcode_key, "abc");
        assert!(qr.url.contains("account.bilibili.com"));
    }

    #[test]
    fn rejects_qr_generate_error_response() {
        let err = parse_qr_generate(r#"{"code":-1,"message":"failed","data":null}"#).unwrap_err();

        assert_eq!(err.code(), crate::errors::ErrorCode::PlatformChanged);
    }

    #[test]
    fn parses_qr_poll_pending_status() {
        let result = parse_qr_poll(
            r#"{"code":0,"message":"OK","data":{"url":"","refresh_token":"","timestamp":0,"code":86101,"message":"未扫码"}}"#,
        )
        .unwrap();

        assert_eq!(result.status, "pending");
        assert_eq!(result.message, "未扫码");
    }

    #[test]
    fn parses_qr_poll_scanned_status() {
        let result = parse_qr_poll(
            r#"{"code":0,"message":"OK","data":{"url":"","refresh_token":"","timestamp":0,"code":86090,"message":"已扫码未确认"}}"#,
        )
        .unwrap();

        assert_eq!(result.status, "scanned");
    }

    #[test]
    fn parses_qr_poll_expired_status() {
        let result = parse_qr_poll(
            r#"{"code":0,"message":"OK","data":{"url":"","refresh_token":"","timestamp":0,"code":86038,"message":"二维码已失效"}}"#,
        )
        .unwrap();

        assert_eq!(result.status, "expired");
    }

    #[test]
    fn parses_qr_poll_confirmed_status() {
        let result = parse_qr_poll(
            r#"{"code":0,"message":"OK","data":{"url":"https://www.bilibili.com","refresh_token":"rt","timestamp":1,"code":0,"message":""}}"#,
        )
        .unwrap();

        assert_eq!(result.status, "confirmed");
        assert_eq!(result.message, "登录成功");
    }

    #[test]
    fn rejects_unknown_qr_poll_code() {
        let err = parse_qr_poll(
            r#"{"code":0,"message":"OK","data":{"url":"","refresh_token":"","timestamp":0,"code":12345,"message":"new status"}}"#,
        )
        .unwrap_err();

        assert_eq!(err.code(), crate::errors::ErrorCode::PlatformChanged);
    }

    #[test]
    fn extracts_cookie_pairs_from_set_cookie_headers() {
        let mut headers = HeaderMap::new();
        headers.append(
            SET_COOKIE,
            HeaderValue::from_static("SESSDATA=secret; Path=/; HttpOnly"),
        );
        headers.append(
            SET_COOKIE,
            HeaderValue::from_static("bili_jct=csrf; Path=/; Secure"),
        );

        let cookies = cookie_string_from_set_cookie_headers(&headers).unwrap();

        assert_eq!(cookies, "SESSDATA=secret; bili_jct=csrf");
    }

    #[tokio::test]
    async fn request_login_qr_from_url_uses_response_parser() {
        let url = one_shot_http_response(
            "200 OK",
            vec![("Content-Type", "application/json")],
            r#"{"code":0,"message":"OK","data":{"url":"https://account.bilibili.com/scan?qrcode_key=abc","qrcode_key":"abc"}}"#,
        );

        let qr = request_login_qr_from_url(&reqwest::Client::new(), &url)
            .await
            .unwrap();

        assert_eq!(qr.qrcode_key, "abc");
    }

    #[tokio::test]
    #[ignore]
    async fn live_request_login_qr_returns_key() {
        let qr = request_login_qr(&reqwest::Client::new()).await.unwrap();

        assert!(!qr.qrcode_key.is_empty());
        assert!(qr.url.contains("qrcode_key="));
    }

    #[tokio::test]
    async fn poll_login_qr_from_url_collects_success_cookies() {
        let url = one_shot_http_response(
            "200 OK",
            vec![
                ("Content-Type", "application/json"),
                ("Set-Cookie", "SESSDATA=secret; Path=/; HttpOnly"),
                ("Set-Cookie", "bili_jct=csrf; Path=/; Secure"),
            ],
            r#"{"code":0,"message":"OK","data":{"url":"https://www.bilibili.com","refresh_token":"rt","timestamp":1,"code":0,"message":""}}"#,
        );

        let outcome = poll_login_qr_from_url(&reqwest::Client::new(), &url)
            .await
            .unwrap();

        assert_eq!(outcome.result.status, "confirmed");
        assert_eq!(
            outcome.cookies,
            Some("SESSDATA=secret; bili_jct=csrf".into())
        );
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

    fn one_shot_http_response(
        status: &str,
        headers: Vec<(&'static str, &'static str)>,
        body: &'static str,
    ) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let status = status.to_string();
        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buffer = [0_u8; 1024];
            let _ = stream.read(&mut buffer);
            write!(
                stream,
                "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n",
                body.len()
            )
            .unwrap();
            for (name, value) in headers {
                write!(stream, "{name}: {value}\r\n").unwrap();
            }
            write!(stream, "\r\n{body}").unwrap();
        });
        format!("http://{address}/qr")
    }
}

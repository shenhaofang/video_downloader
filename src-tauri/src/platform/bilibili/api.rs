use crate::errors::{AppError, AppResult, ErrorCode};
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoPage {
    pub cid: u64,
    pub page: u32,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewInfo {
    pub title: String,
    pub pages: Vec<VideoPage>,
}

#[derive(Debug, Deserialize)]
struct ViewResponse {
    code: i32,
    message: String,
    data: Option<ViewData>,
}

#[derive(Debug, Deserialize)]
struct ViewData {
    title: String,
    pages: Vec<ViewPage>,
}

#[derive(Debug, Deserialize)]
struct ViewPage {
    cid: u64,
    page: u32,
    part: String,
}

pub fn parse_view_info(json: &str) -> AppResult<ViewInfo> {
    let parsed: ViewResponse = serde_json::from_str(json)
        .map_err(|err| AppError::structured(ErrorCode::PlatformChanged, err.to_string()))?;
    if parsed.code != 0 {
        return Err(AppError::structured(
            ErrorCode::PlatformChanged,
            parsed.message,
        ));
    }
    let data = parsed
        .data
        .ok_or_else(|| AppError::structured(ErrorCode::PlatformChanged, "missing view data"))?;
    if data.pages.is_empty() {
        return Err(AppError::structured(
            ErrorCode::PlatformChanged,
            "missing video pages",
        ));
    }

    let group_title = data.title;
    let is_single_page = data.pages.len() == 1;
    Ok(ViewInfo {
        title: group_title.clone(),
        pages: data
            .pages
            .into_iter()
            .map(|page| {
                let title = if page.part.trim().is_empty() {
                    if is_single_page {
                        group_title.clone()
                    } else {
                        format!("P{}", page.page)
                    }
                } else {
                    page.part
                };
                VideoPage {
                    cid: page.cid,
                    page: page.page,
                    title,
                }
            })
            .collect(),
    })
}

pub fn view_info_url(bvid: &str) -> String {
    format!("https://api.bilibili.com/x/web-interface/view?bvid={bvid}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::ErrorCode;

    #[test]
    fn parses_multi_part_view_response() {
        let json = r#"{
          "code": 0,
          "message": "0",
          "data": {
            "title": "Rust 桌面应用入门",
            "pages": [
              {"cid": 111, "page": 1, "part": "安装 Tauri"},
              {"cid": 222, "page": 2, "part": "Rust 命令与事件"}
            ]
          }
        }"#;

        let info = parse_view_info(json).unwrap();

        assert_eq!(info.title, "Rust 桌面应用入门");
        assert_eq!(
            info.pages,
            vec![
                VideoPage {
                    cid: 111,
                    page: 1,
                    title: "安装 Tauri".into(),
                },
                VideoPage {
                    cid: 222,
                    page: 2,
                    title: "Rust 命令与事件".into(),
                },
            ]
        );
    }

    #[test]
    fn uses_video_title_when_single_page_part_is_empty() {
        let json = r#"{
          "code": 0,
          "message": "OK",
          "data": {
            "title": "字幕君交流场所",
            "pages": [
              {"cid": 62131, "page": 1, "part": ""}
            ]
          }
        }"#;

        let info = parse_view_info(json).unwrap();

        assert_eq!(info.pages[0].title, "字幕君交流场所");
    }

    #[test]
    fn rejects_view_response_without_pages() {
        let json = r#"{
          "code": 0,
          "message": "OK",
          "data": {
            "title": "空视频",
            "pages": []
          }
        }"#;

        let err = parse_view_info(json).unwrap_err();

        assert_eq!(err.code(), ErrorCode::PlatformChanged);
    }

    #[test]
    fn rejects_error_view_response() {
        let err = parse_view_info(r#"{"code": -400, "message": "请求错误"}"#).unwrap_err();

        assert_eq!(err.code(), ErrorCode::PlatformChanged);
    }

    #[test]
    fn builds_view_info_url_for_bvid() {
        assert_eq!(
            view_info_url("BV1xx411c7mD"),
            "https://api.bilibili.com/x/web-interface/view?bvid=BV1xx411c7mD"
        );
    }
}

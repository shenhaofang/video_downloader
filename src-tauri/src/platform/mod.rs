pub mod bilibili;
pub mod mock;

use crate::errors::AppResult;
use crate::models::DownloadEngine;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProbeInput {
    pub url: String,
    pub engine: DownloadEngine,
    pub has_login: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DownloadItemMetadata {
    pub bvid: String,
    pub cid: u64,
    pub page: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DownloadItem {
    pub title: String,
    pub output_file: String,
    pub quality: Option<String>,
    pub requires_login: bool,
    pub bytes_total: Option<u64>,
    pub metadata: Option<DownloadItemMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProbeResult {
    pub group_title: String,
    pub items: Vec<DownloadItem>,
    pub used_login: bool,
}

#[derive(Debug, Clone)]
pub struct DownloadInput {
    pub item: DownloadItem,
    pub output_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DownloadOutput {
    pub output_path: String,
    pub quality: Option<String>,
    pub used_login: bool,
    pub bytes_total: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DownloadEvent {
    Log(String),
    Progress { downloaded: u64, total: Option<u64> },
    State(String),
}

pub trait EventSink: Send + Sync {
    fn emit(&self, event: DownloadEvent);
}

pub trait PlatformDownloader: Send + Sync {
    fn probe<'a>(
        &'a self,
        input: ProbeInput,
    ) -> Pin<Box<dyn Future<Output = AppResult<ProbeResult>> + Send + 'a>>;

    fn download<'a>(
        &'a self,
        input: DownloadInput,
        sink: &'a dyn EventSink,
    ) -> Pin<Box<dyn Future<Output = AppResult<DownloadOutput>> + Send + 'a>>;
}

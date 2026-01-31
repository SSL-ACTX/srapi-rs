// src/core/provider.rs
use crate::core::error::ProviderError;
use bytes::Bytes;
use reqwest::Body;
use rkyv::{Archive, Deserialize, Serialize};

#[derive(Debug, Clone, Archive, Serialize, Deserialize)]
#[archive(check_bytes)]
pub struct FileMetadata {
    pub filename: String,
    pub size: u64,
    pub content_type: String,
}

#[derive(Debug, Clone, Archive, Serialize, Deserialize)]
#[archive(check_bytes)]
pub struct BinInfo {
    pub id: String,
    pub file_count: usize,
    /// Human-readable expiration string (e.g., "1 week")
    pub expiration: String,
    pub files: Vec<FileMetadata>,
}

#[derive(Debug, Clone, Archive, Serialize, Deserialize)]
#[archive(check_bytes)]
pub struct UploadedFile {
    pub url: String,
    pub filename: String,
    pub size: u64,
    pub content_type: String,
    pub expires_at: Option<String>,
}

#[async_trait::async_trait]
pub trait FileProvider: Send + Sync {
    async fn create_bin(&self) -> Result<String, ProviderError>;

    async fn upload_file(
        &self,
        bin_id: &str,
        filename: &str,
        data: Body,
        len: u64,
    ) -> Result<FileMetadata, ProviderError>;

    async fn download_file(
        &self,
        bin_id: &str,
        filename: &str,
    ) -> Result<Box<dyn futures_core::Stream<Item = reqwest::Result<Bytes>> + Send + Unpin>, ProviderError>;

    async fn get_bin_details(&self, bin_id: &str) -> Result<BinInfo, ProviderError>;

    async fn delete_file(&self, bin_id: &str, filename: &str) -> Result<(), ProviderError>;

    async fn delete_bin(&self, bin_id: &str) -> Result<(), ProviderError>;
}

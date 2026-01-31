// src/backends/filebin.rs
use crate::core::error::ProviderError;
use crate::core::provider::{BinInfo, FileMetadata, FileProvider};
use async_trait::async_trait;
use bytes::Bytes;
use regex::Regex;
use reqwest::{header, Body, Client, StatusCode};
use std::time::Duration;

pub struct FilebinProvider {
    client: Client,
    base_url: String,
}

impl FilebinProvider {
    /// Creates a new Filebin provider with headers mimicking a browser to avoid bot detection.
    pub fn new() -> Self {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::USER_AGENT,
            header::HeaderValue::from_static(
                "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/136.0.0.0 Safari/537.36",
            ),
        );
        headers.insert(
            header::ACCEPT,
            header::HeaderValue::from_static("*/*"),
        );
        headers.insert(
            "X-Requested-With",
            header::HeaderValue::from_static("XMLHttpRequest"),
        );

        let client = Client::builder()
        .default_headers(headers)
        .timeout(Duration::from_secs(30))
        .build()
        .expect("Failed to build HTTP client");

        Self {
            client,
            base_url: "https://filebin.net".to_string(),
        }
    }

    pub fn with_base_url(base_url: String) -> Self {
        let mut s = Self::new();
        s.base_url = base_url;
        s
    }

    /// Helper to parse the bin ID from the landing page HTML.
    fn scrape_bin_id(html: &str) -> Option<String> {
        // Regex: var bin = "kjsdflkjsdf"
        let re = Regex::new(r#"var bin = "([^"]+)""#).ok()?;
        re.captures(html)
        .and_then(|cap| cap.get(1).map(|m| m.as_str().to_string()))
    }
}

#[async_trait]
impl FileProvider for FilebinProvider {
    async fn create_bin(&self) -> Result<String, ProviderError> {
        let res = self.client.get(&self.base_url).send().await?;
        let html = res.text().await?;

        Self::scrape_bin_id(&html)
        .ok_or_else(|| ProviderError::Parse("Failed to scrape Bin ID from homepage".to_string()))
    }

    async fn upload_file(
        &self,
        bin_id: &str,
        filename: &str,
        data: Body,
        len: u64,
    ) -> Result<FileMetadata, ProviderError> {
        let url = format!("{}/{}/{}", self.base_url, bin_id, filename);

        let res = self.client.post(&url)
        .header("Bin", bin_id)
        .header("Content-Type", "application/octet-stream")
        .header(header::CONTENT_LENGTH, len.to_string())
        .header("Size", len.to_string())
        .body(data)
        .send()
        .await?;

        if !res.status().is_success() {
            return Err(ProviderError::Api(format!("Upload failed: {}", res.status())));
        }

        // Filebin does not return JSON metadata on upload, so we reconstruct it from input
        Ok(FileMetadata {
            filename: filename.to_string(),
           size: len,
           content_type: "application/octet-stream".to_string(),
        })
    }

    async fn download_file(
        &self,
        bin_id: &str,
        filename: &str,
    ) -> Result<Box<dyn futures_core::Stream<Item = reqwest::Result<Bytes>> + Send + Unpin>, ProviderError> {
        let url = format!("{}/{}/{}", self.base_url, bin_id, filename);

        let res = self.client.get(&url)
        .header("Bin", bin_id)
        .send()
        .await?;

        if res.status() == StatusCode::NOT_FOUND {
            return Err(ProviderError::NotFound);
        }

        if !res.status().is_success() {
            return Err(ProviderError::Api(format!("Download failed: {}", res.status())));
        }

        Ok(Box::new(res.bytes_stream()))
    }

    async fn get_bin_details(&self, bin_id: &str) -> Result<BinInfo, ProviderError> {
        let url = format!("{}/{}", self.base_url, bin_id);

        let res = self.client.get(&url)
        .header("Bin", bin_id)
        .header("Accept", "text/html")
        .header("X-Requested-With", "XMLHttpRequest")
        .send()
        .await?;

        if res.status() == StatusCode::NOT_FOUND {
            return Err(ProviderError::NotFound);
        }

        let html = res.text().await?;

        // 1. Parse Summary Stats
        let re_count = Regex::new(r"It contains (\d+) uploaded").map_err(|e| ProviderError::Parse(e.to_string()))?;
        let file_count = re_count.captures(&html)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().parse::<usize>().unwrap_or(0))
        .unwrap_or(0);

        // 2. Parse Expiration
        let re_exp = Regex::new(r"expires ([\d\s\w]+) from now").map_err(|e| ProviderError::Parse(e.to_string()))?;
        let expiration = re_exp.captures(&html)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
        .unwrap_or_else(|| "Unknown".to_string());

        // 3. Parse File List
        let mut files = Vec::new();
        // Regex to capture size and filename
        let re_files = Regex::new(r#"<tr[^>]*>[\s\S]*?sorttable_customkey="(\d+)"[\s\S]*?<a[^>]+>([^<]+)</a>"#)
        .map_err(|e| ProviderError::Parse(e.to_string()))?;

        for cap in re_files.captures_iter(&html) {
            if let (Some(size_match), Some(name_match)) = (cap.get(1), cap.get(2)) {
                let size = size_match.as_str().parse::<u64>().unwrap_or(0);
                let name = name_match.as_str().to_string();

                if !files.iter().any(|f: &FileMetadata| f.filename == name) {
                    files.push(FileMetadata {
                        filename: name,
                        size,
                        content_type: "application/octet-stream".to_string(),
                    });
                }
            }
        }

        Ok(BinInfo {
            id: bin_id.to_string(),
           file_count,
           expiration, // Now used correctly
           files,
        })
    }

    async fn delete_file(&self, bin_id: &str, filename: &str) -> Result<(), ProviderError> {
        let url = format!("{}/{}/{}", self.base_url, bin_id, filename);
        let res = self.client.delete(&url)
        .header("Bin", bin_id)
        .send()
        .await?;

        if !res.status().is_success() {
            return Err(ProviderError::Api(format!("Failed to delete file: {}", res.status())));
        }
        Ok(())
    }

    async fn delete_bin(&self, bin_id: &str) -> Result<(), ProviderError> {
        let url = format!("{}/{}", self.base_url, bin_id);
        let res = self.client.delete(&url)
        .header("Bin", bin_id)
        .send()
        .await?;

        if !res.status().is_success() {
            return Err(ProviderError::Api(format!("Failed to delete bin: {}", res.status())));
        }
        Ok(())
    }
}

use crate::core::error::ProviderError;
use crate::core::provider::UploadedFile;
use regex::Regex;
use reqwest::{header, multipart, Client};
use std::time::Duration;

pub struct TmpfilesProvider {
    client: Client,
    base_url: String,
}

impl TmpfilesProvider {
    pub fn new() -> Self {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::USER_AGENT,
            header::HeaderValue::from_static(
                "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/136.0.0.0 Safari/537.36",
            ),
        );
        headers.insert(header::ACCEPT, header::HeaderValue::from_static("*/*"));

        let client = Client::builder()
            .default_headers(headers)
            .cookie_store(true)
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to build HTTP client");

        Self {
            client,
            base_url: "https://tmpfiles.org".to_string(),
        }
    }

    pub fn with_base_url(base_url: String) -> Self {
        let mut s = Self::new();
        s.base_url = base_url;
        s
    }

    pub async fn upload_bytes(&self, filename: &str, bytes: Vec<u8>) -> Result<UploadedFile, ProviderError> {
        let size = bytes.len() as u64;
        let token = self.fetch_token().await?;

        let part = multipart::Part::bytes(bytes)
            .file_name(filename.to_string())
            .mime_str("application/octet-stream")
            .map_err(|e| ProviderError::Parse(e.to_string()))?;

        let form = multipart::Form::new()
            .text("_token", token)
            .part("file", part)
            .text("upload", "Upload");

        let res = self.client.post(&self.base_url).multipart(form).send().await?;
        if !res.status().is_success() {
            return Err(ProviderError::Api(format!("tmpfiles upload failed: {}", res.status())));
        }
        let html = res.text().await?;

        let url = extract_download_url(&html)
            .ok_or_else(|| ProviderError::Parse("tmpfiles upload URL not found".to_string()))?;
        let expires_at = extract_table_value(&html, "Expires at");

        Ok(UploadedFile {
            url,
            filename: filename.to_string(),
            size,
            content_type: "application/octet-stream".to_string(),
            expires_at,
        })
    }

    pub async fn get_file_info(&self, url: &str) -> Result<UploadedFile, ProviderError> {
        let info_url = normalize_info_url(url);
        let res = self.client.get(&info_url).send().await?;
        if res.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(ProviderError::NotFound);
        }
        if !res.status().is_success() {
            return Err(ProviderError::Api(format!("tmpfiles info failed: {}", res.status())));
        }
        let html = res.text().await?;

        let filename = extract_table_value(&html, "Filename")
            .ok_or_else(|| ProviderError::Parse("tmpfiles filename not found".to_string()))?;
        let size_str = extract_table_value(&html, "Size").unwrap_or_else(|| "0".to_string());
        let expires_at = extract_table_value(&html, "Expires at");

        Ok(UploadedFile {
            url: extract_download_url(&html).unwrap_or_else(|| url.to_string()),
            filename,
            size: parse_size_to_bytes(&size_str),
            content_type: "application/octet-stream".to_string(),
            expires_at,
        })
    }

    async fn fetch_token(&self) -> Result<String, ProviderError> {
        let res = self.client.get(&self.base_url).send().await?;
        if !res.status().is_success() {
            return Err(ProviderError::Api(format!("tmpfiles token fetch failed: {}", res.status())));
        }
        let html = res.text().await?;

        let re = Regex::new(r#"name=\"_token\" value=\"([^\"]+)\""#)
            .map_err(|e| ProviderError::Parse(e.to_string()))?;
        re.captures(&html)
            .and_then(|c| c.get(1).map(|m| m.as_str().to_string()))
            .ok_or_else(|| ProviderError::Parse("tmpfiles _token not found".to_string()))
    }
}

fn extract_download_url(html: &str) -> Option<String> {
    let re = Regex::new(r#"https?://tmpfiles\.org/dl/[^\"'\s]+"#).ok()?;
    re.find(html).map(|m| m.as_str().to_string())
}

fn normalize_info_url(url: &str) -> String {
    if let Some(idx) = url.find("/dl/") {
        let (head, tail) = url.split_at(idx);
        let tail = tail.trim_start_matches("/dl/");
        format!("{}/{}", head.trim_end_matches('/'), tail)
    } else {
        url.to_string()
    }
}

fn extract_table_value(html: &str, key: &str) -> Option<String> {
    let pattern = format!(r"(?s)<th[^>]*>\s*{}\s*</th>\s*<td>\s*([^<]+)\s*</td>", regex::escape(key));
    let re = Regex::new(&pattern).ok()?;
    re.captures(html)
        .and_then(|c| c.get(1).map(|m| m.as_str().trim().to_string()))
}

fn parse_size_to_bytes(size: &str) -> u64 {
    let s = size.trim();
    if let Ok(v) = s.parse::<f64>() {
        return v as u64;
    }

    let re = Regex::new(r"(?i)([\d\.]+)\s*(KB|MB|GB|B)").ok();
    if let Some(re) = re {
        if let Some(caps) = re.captures(s) {
            let value = caps.get(1).and_then(|m| m.as_str().parse::<f64>().ok()).unwrap_or(0.0);
            let unit = caps.get(2).map(|m| m.as_str().to_uppercase()).unwrap_or_else(|| "B".to_string());
            return match unit.as_str() {
                "KB" => (value * 1024.0) as u64,
                "MB" => (value * 1024.0 * 1024.0) as u64,
                "GB" => (value * 1024.0 * 1024.0 * 1024.0) as u64,
                _ => value as u64,
            };
        }
    }

    0
}

use crate::core::error::ProviderError;
use crate::core::provider::UploadedFile;
use regex::Regex;
use reqwest::{header, multipart, Client};
use std::time::Duration;

pub struct TempShProvider {
    client: Client,
    base_url: String,
}

impl TempShProvider {
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
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to build HTTP client");

        Self {
            client,
            base_url: "https://temp.sh".to_string(),
        }
    }

    pub fn with_base_url(base_url: String) -> Self {
        let mut s = Self::new();
        s.base_url = base_url;
        s
    }

    pub async fn upload_bytes(&self, filename: &str, bytes: Vec<u8>) -> Result<UploadedFile, ProviderError> {
        let url = format!("{}/upload", self.base_url);
        let size = bytes.len() as u64;
        let part = multipart::Part::bytes(bytes)
            .file_name(filename.to_string())
            .mime_str("application/octet-stream")
            .map_err(|e| ProviderError::Parse(e.to_string()))?;

        let form = multipart::Form::new().part("file", part).text("submit", "Upload!");

        let res = self.client.post(&url).multipart(form).send().await?;
        if !res.status().is_success() {
            return Err(ProviderError::Api(format!("Temp.sh upload failed: {}", res.status())));
        }
        let text = res.text().await?;

        let re = Regex::new(r"https?://\S+").map_err(|e| ProviderError::Parse(e.to_string()))?;
        let url = re
            .find(&text)
            .map(|m| m.as_str().trim().to_string())
            .ok_or_else(|| ProviderError::Parse("Temp.sh upload URL not found".to_string()))?;

        Ok(UploadedFile {
            url,
            filename: filename.to_string(),
            size,
            content_type: "application/octet-stream".to_string(),
            expires_at: None,
        })
    }

    pub async fn get_file_info(&self, url: &str) -> Result<UploadedFile, ProviderError> {
        let res = self.client.get(url).send().await?;
        if res.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(ProviderError::NotFound);
        }
        if !res.status().is_success() {
            return Err(ProviderError::Api(format!("Temp.sh info failed: {}", res.status())));
        }
        let html = res.text().await?;

        let filename = extract_table_value(&html, "Filename")
            .ok_or_else(|| ProviderError::Parse("Temp.sh filename not found".to_string()))?;
        let expires_at = extract_table_value(&html, "Expire Time");
        let size_str = extract_table_value(&html, "File Size").unwrap_or_else(|| "0".to_string());
        let content_type = extract_table_value(&html, "Mime Type").unwrap_or_else(|| "application/octet-stream".to_string());

        Ok(UploadedFile {
            url: url.to_string(),
            filename,
            size: size_str.trim().parse::<u64>().unwrap_or(0),
            content_type,
            expires_at,
        })
    }
}

fn extract_table_value(html: &str, key: &str) -> Option<String> {
    let pattern = format!(r"(?s)<th>\s*{}\s*</th>\s*<td>\s*([^<]+)\s*</td>", regex::escape(key));
    let re = Regex::new(&pattern).ok()?;
    re.captures(html)
        .and_then(|c| c.get(1).map(|m| m.as_str().trim().to_string()))
}

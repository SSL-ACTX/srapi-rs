use crate::core::error::ProviderError;
use crate::core::provider::UploadedFile;
use rand::Rng;
use reqwest::cookie::{CookieStore, Jar};
use reqwest::{header, Client, Url};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

#[derive(Serialize, Deserialize, Debug)]
struct CreateBucketResponse {
    status: i32,
    bucket_id: Option<String>,
    code: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct InitMultipartResponse {
    #[serde(rename = "uploadId")]
    upload_id: String,
    key: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct SignPartResponse {
    url: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct CompleteData {
    download_url: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct CompleteResponse {
    status: i32,
    msg: Option<String>,
    data: CompleteData,
    link: String,
}

pub struct JumpshareProvider {
    client: Client,
    base_url: String,
    cookie_jar: Arc<Jar>,
}

impl JumpshareProvider {
    pub fn new() -> Self {
        let cookie_jar = Arc::new(Jar::default());
        let mut headers = header::HeaderMap::new();

        // 1. Random User-Agent
        let uas = [
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119.0.0.0 Safari/537.36",
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/118.0.0.0 Safari/537.36",
            "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
            "Mozilla/5.0 (iPhone; CPU iPhone OS 17_1 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.1 Mobile/15E148 Safari/604.1"
        ];
        let mut rng = rand::rng();
        let ua_idx = rng.random_range(0..uas.len());
        let ua = uas[ua_idx];

        headers.insert(
            header::USER_AGENT,
            header::HeaderValue::from_str(ua).unwrap(),
        );

        // 2. Random Fake IP for X-Forwarded-For
        let fake_ip = format!(
            "{}.{}.{}.{}",
            rng.random_range(1..255),
            rng.random_range(1..255),
            rng.random_range(1..255),
            rng.random_range(1..255)
        );
        headers.insert(
            "X-Forwarded-For",
            header::HeaderValue::from_str(&fake_ip).unwrap(),
        );

        headers.insert(
            header::ORIGIN,
            header::HeaderValue::from_static("https://jumpshare.com"),
        );
        // Initial referer, though logic might require specific referers per request (handled in Python by changing it?)
        // Python: Referer: f"{self.base_url}/file-sharing/{self.category}"
        // We set a default here, usually fine.
        headers.insert(
            header::REFERER,
            header::HeaderValue::from_static("https://jumpshare.com/file-sharing/video"),
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
            .cookie_provider(cookie_jar.clone())
            .timeout(Duration::from_secs(300))
            .build()
            .expect("Failed to build HTTP client");


        Self {
            client,
            base_url: "https://jumpshare.com".to_string(),
            cookie_jar,
        }
    }

    async fn get_token(&self) -> Result<String, ProviderError> {
        // Visit landing page to get cookies
        let url = format!("{}/file-sharing/video", self.base_url);
        self.client
            .get(&url)
            .send()
            .await
            .map_err(ProviderError::Network)?;

        // Extract _jsactnk from cookie jar
        let url_obj = Url::parse(&url).map_err(|e| ProviderError::Api(format!("Internal URL error: {}", e)))?;
        let cookies_str = self
            .cookie_jar
            .cookies(&url_obj)
            .ok_or(ProviderError::Api("No cookies found".to_string()))?;
        
        // cookies_str is like "name=value; name2=value2"
        // We need to find _jsactnk
        let header_str = cookies_str.to_str().map_err(|e| ProviderError::Parse(format!("Cookie encoding error: {}", e)))?;
        let token = header_str
            .split(';')
            .find_map(|pair| {
                let mut parts = pair.trim().split('=');
                let name = parts.next()?;
                let value = parts.next()?;
                if name == "_jsactnk" {
                    Some(value.to_string())
                } else {
                    None
                }
            })
            .ok_or(ProviderError::Api("Token _jsactnk not found in cookies".to_string()))?;

        Ok(token)
    }

    pub async fn upload_bytes(&self, filename: &str, bytes: Vec<u8>) -> Result<UploadedFile, ProviderError> {
        let size = bytes.len() as u64;
        let token = self.get_token().await?;

        // 1. Create Bucket
        let create_bucket_url = format!("{}/viewer/create-bucket", self.base_url);
        let ext = std::path::Path::new(filename)
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("txt");

        let form = [
            ("form_info[jsactnk]", token.clone()),
            ("file_extensions", ext.to_string()),
            ("first_file_name", filename.to_string()),
            ("files_size", size.to_string()),
            ("files_count", "1".to_string()),
            ("is_ai_page", "0".to_string()),
            ("upload_source", "programmatic-pages".to_string()),
            ("media_duration", "0".to_string()),
            ("jsactnk", token.clone()),
        ];

        let bucket_resp_text = self.client.post(&create_bucket_url)
            .form(&form)
            .send()
            .await
            .map_err(ProviderError::Network)?
            .text()
            .await
            .map_err(ProviderError::Network)?;
            
        let bucket_resp: CreateBucketResponse = serde_json::from_str(&bucket_resp_text)
            .map_err(|e| ProviderError::Parse(format!("Failed to parse bucket response: {} - {}", e, bucket_resp_text)))?;

        if bucket_resp.status != 1 {
            return Err(ProviderError::Api(format!(
                "Failed to create bucket: code={:?}", bucket_resp.code
            )));
        }
        let bucket_id = bucket_resp.bucket_id.ok_or(ProviderError::Parse("No bucket_id in response".to_string()))?;

        // 2. Initialize Multipart Upload
        let init_url = format!("{}/inbox/create_multipart_upload", self.base_url);
        // Determine type roughly
        let mime_type = if filename.ends_with(".mp4") { "video/mp4" }
                       else if filename.ends_with(".mp3") { "audio/mpeg" }
                       else { "application/octet-stream" };

        let init_form = [
            ("bucket_id", bucket_id.clone()),
            ("fileInfo[name]", filename.to_string()),
            ("fileInfo[type]", mime_type.to_string()),
            ("fileInfo[size]", size.to_string()),
            ("fileInfo[upload_source]", "programmatic-pages".to_string()),
            ("fileInfo[media_duration]", "0".to_string()),
            ("jsactnk", token.clone()),
        ];

        let init_resp: InitMultipartResponse = self.client.post(&init_url)
            .form(&init_form)
            .send()
            .await
            .map_err(ProviderError::Network)?
            .json()
            .await
            .map_err(|e| ProviderError::Parse(format!("Failed to parse init upload response: {}", e)))?;

        let upload_id = init_resp.upload_id;
        let key = init_resp.key;

        // 3. Sign Part
        let sign_url = format!("{}/inbox/sign_upload_part", self.base_url);
        let sign_form = [
            ("sendBackData[uploadId]", upload_id.clone()),
            ("sendBackData[key]", key.clone()),
            ("partNumber", "1".to_string()),
            ("contentLength", size.to_string()),
            ("jsactnk", token.clone()),
        ];

        let sign_resp: SignPartResponse = self.client.post(&sign_url)
            .form(&sign_form)
            .send()
            .await
            .map_err(ProviderError::Network)?
            .json()
            .await
            .map_err(|e| ProviderError::Parse(format!("Failed to parse sign part response: {}", e)))?;

        // 4. Upload to S3 (PUT)
        self.client.put(&sign_resp.url)
            .body(bytes)
            .send()
            .await
            .map_err(ProviderError::Network)?;

        // 5. Complete Upload
        let complete_url = format!("{}/inbox/complete_multipart_upload", self.base_url);
        let complete_form = [
            ("command", "CompleteMultipartUpload".to_string()),
            ("sendBackData[uploadId]", upload_id),
            ("sendBackData[key]", key),
            ("bucket_id", bucket_id),
            ("file_info[name]", filename.to_string()),
            ("file_info[type]", mime_type.to_string()),
            ("file_info[size]", size.to_string()),
            ("file_info[upload_source]", "programmatic-pages".to_string()),
            ("file_info[media_duration]", "0".to_string()),
            ("upload_source", "programmatic-pages".to_string()),
            ("jsactnk", token),
        ];

        let complete_resp: CompleteResponse = self.client.post(&complete_url)
            .form(&complete_form)
            .send()
            .await
            .map_err(ProviderError::Network)?
            .json()
            .await
            .map_err(|e| ProviderError::Parse(format!("Failed to parse complete response: {}", e)))?;

        if complete_resp.status != 1 {
            return Err(ProviderError::Api(format!("Upload completion failed: {}", complete_resp.msg.unwrap_or("Unknown".to_string()))));
        }

        // Return the direct download URL as preferred by the user
        let direct_url = complete_resp.data.download_url;
        
        Ok(UploadedFile {
            url: direct_url,
            filename: filename.to_string(),
            size: size,
            content_type: mime_type.to_string(),
            expires_at: Some("1 day".to_string()),
        })
    }

    pub async fn get_file_info(&self, _url: &str) -> Result<UploadedFile, ProviderError> {
        // Implement info scraping if needed, for now return Unsupported or minimal info
        // We can just check if it's a valid link and return what we know
        Err(ProviderError::Api("Info lookup not implemented for Jumpshare yet".to_string()))
    }
}

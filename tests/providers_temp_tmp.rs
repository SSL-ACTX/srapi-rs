use srapi_rs::{TempShProvider, TmpfilesProvider};
use wiremock::{Mock, MockServer, ResponseTemplate};
use wiremock::matchers::{method, path};

#[tokio::test]
async fn temp_sh_upload_parses_url() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/upload"))
        .respond_with(ResponseTemplate::new(200).set_body_string("https://temp.sh/abc123/test.txt\n"))
        .mount(&mock_server)
        .await;

    let provider = TempShProvider::with_base_url(mock_server.uri());
    let result = provider
        .upload_bytes("test.txt", b"hello".to_vec())
        .await
        .unwrap();

    assert_eq!(result.url, "https://temp.sh/abc123/test.txt");
    assert_eq!(result.filename, "test.txt");
    assert_eq!(result.size, 5);
}

#[tokio::test]
async fn temp_sh_info_parses_html() {
    let mock_server = MockServer::start().await;
    let html = r#"
        <table>
            <tr><th>Filename</th><td>test.txt</td></tr>
            <tr><th>Expire Time</th><td>2026-02-03 15:17:00</td></tr>
            <tr><th>File Size</th><td>0</td></tr>
            <tr><th>Mime Type</th><td>ASCII text</td></tr>
        </table>
    "#;

    Mock::given(method("GET"))
        .and(path("/abc123/test.txt"))
        .respond_with(ResponseTemplate::new(200).set_body_string(html))
        .mount(&mock_server)
        .await;

    let provider = TempShProvider::with_base_url(mock_server.uri());
    let url = format!("{}/abc123/test.txt", mock_server.uri());
    let info = provider.get_file_info(&url).await.unwrap();

    assert_eq!(info.filename, "test.txt");
    assert_eq!(info.expires_at.as_deref(), Some("2026-02-03 15:17:00"));
    assert_eq!(info.size, 0);
    assert_eq!(info.content_type, "ASCII text");
}

#[tokio::test]
async fn tmpfiles_upload_parses_url() {
    let mock_server = MockServer::start().await;

    let homepage = r#"
        <form action="https://tmpfiles.org" method="post" enctype="multipart/form-data">
            <input type="hidden" name="_token" value="abc-token">
        </form>
    "#;

    let upload_response = r#"
        <table>
            <tr><th>URL</th><td><a href="https://tmpfiles.org/dl/21990396/test.txt">http://tmpfiles.org/dl/21990396/test.txt</a></td></tr>
            <tr><th>Expires at</th><td>2026-01-31 16:23 UTC</td></tr>
        </table>
    "#;

    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_string(homepage))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_string(upload_response))
        .mount(&mock_server)
        .await;

    let provider = TmpfilesProvider::with_base_url(mock_server.uri());
    let result = provider
        .upload_bytes("test.txt", b"hello".to_vec())
        .await
        .unwrap();

    assert_eq!(result.url, "https://tmpfiles.org/dl/21990396/test.txt");
    assert_eq!(result.filename, "test.txt");
    assert_eq!(result.size, 5);
    assert_eq!(result.expires_at.as_deref(), Some("2026-01-31 16:23 UTC"));
}

#[tokio::test]
async fn tmpfiles_info_parses_html() {
    let mock_server = MockServer::start().await;
    let html = r#"
        <table>
            <tr><th>Filename</th><td>test.txt</td></tr>
            <tr><th>Size</th><td>0.01 KB</td></tr>
            <tr><th>URL</th><td><a href="https://tmpfiles.org/dl/21990396/test.txt">http://tmpfiles.org/dl/21990396/test.txt</a></td></tr>
            <tr><th>Expires at</th><td>2026-01-31 16:23 UTC</td></tr>
        </table>
    "#;

    Mock::given(method("GET"))
        .and(path("/21990396/test.txt"))
        .respond_with(ResponseTemplate::new(200).set_body_string(html))
        .mount(&mock_server)
        .await;

    let provider = TmpfilesProvider::with_base_url(mock_server.uri());
    let url = format!("{}/21990396/test.txt", mock_server.uri());
    let info = provider.get_file_info(&url).await.unwrap();

    assert_eq!(info.filename, "test.txt");
    assert_eq!(info.expires_at.as_deref(), Some("2026-01-31 16:23 UTC"));
    assert_eq!(info.size, 10);
    assert_eq!(info.content_type, "application/octet-stream");
}

use wiremock::{Mock, MockServer, ResponseTemplate};
use wiremock::matchers::{method, path};
use srapi_rs::{FilebinProvider, FileProvider};

#[tokio::test]
async fn test_create_bin_success() {
    // Start a mock server
    let mock_server = MockServer::start().await;

    // The HTML returned by filebin.net contains a script variable with the bin ID
    let mock_html = r#"
        <html>
            <body>
                <script>
                    var bin = "test-bin-123";
                </script>
            </body>
        </html>
    "#;

    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_string(mock_html))
        .mount(&mock_server)
        .await;

    let provider = FilebinProvider::with_base_url(mock_server.uri());
    let bin_id = provider.create_bin().await.unwrap();

    assert_eq!(bin_id, "test-bin-123");
}

#[tokio::test]
async fn test_get_bin_details_success() {
    let mock_server = MockServer::start().await;
    let bin_id = "test-bin-456";

    let mock_html = r#"
        <tr>
            <td sorttable_customkey="1024"><a href="/test-bin-456/file1.txt">file1.txt</a></td>
        </tr>
        <p>It contains 1 uploaded file at 1.0 KB and expires 6 days from now.</p>
    "#;

    Mock::given(method("GET"))
        .and(path(format!("/{}", bin_id)))
        .respond_with(ResponseTemplate::new(200).set_body_string(mock_html))
        .mount(&mock_server)
        .await;

    let provider = FilebinProvider::with_base_url(mock_server.uri());
    let details = provider.get_bin_details(bin_id).await.unwrap();

    assert_eq!(details.id, bin_id);
    assert_eq!(details.file_count, 1);
    assert_eq!(details.expiration, "6 days");
    assert_eq!(details.files.len(), 1);
    assert_eq!(details.files[0].filename, "file1.txt");
    assert_eq!(details.files[0].size, 1024);
}

#[tokio::test]
async fn test_upload_file_success() {
    let mock_server = MockServer::start().await;
    let bin_id = "test-bin-789";
    let filename = "test.txt";
    let content = "Hello World";
    let len = content.len() as u64;

    Mock::given(method("POST"))
        .and(path(format!("/{}/{}", bin_id, filename)))
        .and(wiremock::matchers::header("Content-Length", len.to_string().as_str()))
        .and(wiremock::matchers::header("Bin", bin_id))
        .respond_with(ResponseTemplate::new(201))
        .mount(&mock_server)
        .await;

    let provider = FilebinProvider::with_base_url(mock_server.uri());
    let body = reqwest::Body::from(content);
    let result = provider.upload_file(bin_id, filename, body, len).await;

    assert!(result.is_ok());
}

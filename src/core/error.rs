use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProviderError {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Parsing error: {0}")]
    Parse(String),

    #[error("API error: {0}")]
    Api(String),

    #[error("Resource not found")]
    NotFound,
}

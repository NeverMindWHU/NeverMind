use thiserror::Error;

#[derive(Debug, Error)]
pub enum HarnessError {
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("foreground capture is only implemented on Windows")]
    UnsupportedPlatform,
    #[error("windows API error: {0}")]
    Windows(String),
    #[error("time: {0}")]
    InvalidTime(String),
    #[error("path: {0}")]
    InvalidPath(String),
}

pub type Result<T> = std::result::Result<T, HarnessError>;

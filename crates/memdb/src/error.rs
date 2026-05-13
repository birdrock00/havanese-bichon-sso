use thiserror::Error;

#[derive(Error, Debug)]
pub enum DbError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("record not found: collection={0}, key={1}")]
    NotFound(String, String),

    #[error("duplicate key: collection={0}, key={1}")]
    DuplicateKey(String, String),
}

pub type Result<T> = std::result::Result<T, DbError>;

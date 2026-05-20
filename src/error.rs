use std::fmt::Display;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("key already exists: {0}")]
    AlreadyExists(String),

    #[error("key not found: {0}")]
    NotFound(String),

    #[error("backend error: {0}")]
    Backend(#[from] BackendError),

    #[error("serialization error: {0}")]
    Codec(#[from] CodecError),
}

#[derive(Debug, thiserror::Error)]
pub struct BackendError(Box<dyn std::error::Error + Send + Sync>);

impl Display for BackendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, thiserror::Error)]
pub struct CodecError(Box<dyn std::error::Error + Send + Sync>);

impl Display for CodecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

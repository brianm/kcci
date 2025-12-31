use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum OokError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("ONNX error: {0}")]
    Onnx(String),

    #[error("Tokenizer error: {0}")]
    Tokenizer(String),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Webarchive parse error: {0}")]
    Webarchive(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

impl From<tokenizers::Error> for OokError {
    fn from(err: tokenizers::Error) -> Self {
        OokError::Tokenizer(err.to_string())
    }
}

impl From<ort::Error> for OokError {
    fn from(err: ort::Error) -> Self {
        OokError::Onnx(err.to_string())
    }
}

// Tauri requires serializable errors for commands
impl Serialize for OokError {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

pub type Result<T> = std::result::Result<T, OokError>;

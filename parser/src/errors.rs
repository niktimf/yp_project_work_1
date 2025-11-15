use thiserror::Error;

pub type Result<T> = std::result::Result<T, ParseError>;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("CSV parsing error: {0}")]
    Csv(#[from] csv::Error),

    #[error("Text format parsing error at line {line}: {message}")]
    TextFormat { line: usize, message: String },

    #[error("Binary format error: {0}")]
    BinaryFormat(String),

    #[error("Invalid field value: {field} = {value}")]
    InvalidField { field: String, value: String },

    #[error("Missing required field: {0}")]
    MissingField(String),

    #[error("Invalid transaction type: {0}")]
    InvalidTransactionType(String),

    #[error("Invalid status: {0}")]
    InvalidStatus(String),

    #[error("UTF-8 conversion error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("MessagePack encoding error: {0}")]
    RmpEncode(#[from] rmp_serde::encode::Error),

    #[error("MessagePack decoding error: {0}")]
    RmpDecode(#[from] rmp_serde::decode::Error),
}


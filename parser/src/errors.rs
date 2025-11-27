//! Error types for the parser library.

use thiserror::Error;

/// Type alias for Results with ParseError.
pub type Result<T> = std::result::Result<T, ParseError>;

/// Comprehensive error type for all parsing operations.
///
/// This enum represents all possible errors that can occur during
/// parsing, serialization, and processing of YPBank transaction records.
#[derive(Error, Debug)]
pub enum ParseError {
    /// I/O operation error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// CSV parsing error.
    #[error("CSV parsing error: {0}")]
    Csv(#[from] csv::Error),

    /// Text format parsing error with line number and message.
    #[error("Text format parsing error at line {line}: {message}")]
    TextFormat {
        /// Line number where the error occurred.
        line: usize,
        /// Error message describing what went wrong.
        message: String,
    },

    /// Binary format error.
    #[error("Binary format error: {0}")]
    BinaryFormat(String),

    /// Invalid field value error.
    #[error("Invalid field value: {field} = {value}")]
    InvalidField {
        /// Name of the invalid field.
        field: String,
        /// Value that was invalid.
        value: String,
    },

    /// UTF-8 conversion error.
    #[error("UTF-8 conversion error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),

    /// JSON serialization/deserialization error.
    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    /// Integer conversion error.
    #[error("Integer conversion error: {0}")]
    TryFromInt(#[from] std::num::TryFromIntError),
}

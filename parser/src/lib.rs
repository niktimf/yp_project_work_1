//! Parser library for YPBank transaction records.
//!
//! This library provides functionality to parse, serialize, and process
//! transaction records from YPBank in various formats including CSV, text, and binary.
//!
//! # Example
//!
//! ```
//! use parser::formats::{YPBankRecord, TransactionType, TransactionStatus};
//! use parser::formats::csv::YPBankCSV;
//! use parser::formats::Parser;
//!
//! # fn example() -> parser::Result<()> {
//! let csv_data = "TX_ID,TX_TYPE,FROM_USER_ID,TO_USER_ID,AMOUNT,TIMESTAMP,STATUS,DESCRIPTION
//! 1,TRANSFER,100,200,5000,1234567890,SUCCESS,Payment";
//!
//! let records = YPBankCSV::parse(csv_data.as_bytes())?;
//! assert_eq!(records.len(), 1);
//! assert_eq!(records[0].tx_id, 1);
//! # Ok(())
//! # }
//! ```

#![warn(missing_docs)]

/// Error handling module for the parser library.
pub mod errors;
/// Formats module containing parsers and serializers for various data formats.
pub mod formats;

pub use errors::{ParseError, Result};

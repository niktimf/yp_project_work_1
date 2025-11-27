/// Binary format support for `YPBank` records.
pub mod binary;
/// CSV format support for `YPBank` records.
pub mod csv;
/// Text format support for `YPBank` records.
pub mod txt;

use crate::ParseError;
use crate::errors::Result;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use field_names::FieldNames;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::str::FromStr;
use strum::{Display, EnumString, IntoStaticStr};

use std::sync::LazyLock;

static YP_BANK_RECORD_UPPERCASE_FIELDS: LazyLock<Vec<String>> =
    LazyLock::new(|| {
        YPBankRecord::FIELDS
            .iter()
            .map(|s| s.to_uppercase())
            .collect()
    });

/// Represents a `YPBank` transaction record.
///
/// This struct contains all the fields of a single transaction
/// in the `YPBank` system and can be serialized/deserialized to/from
/// various formats including CSV, text, and binary.
///
/// # Example
///
/// ```
/// use parser::formats::{YPBankRecord, TransactionType, TransactionStatus};
///
/// let record = YPBankRecord {
///     tx_id: 12345,
///     tx_type: TransactionType::Transfer,
///     from_user_id: 100,
///     to_user_id: 200,
///     amount: 5000,
///     timestamp: 1234567890,
///     status: TransactionStatus::Success,
///     description: "Payment for services".to_string(),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, FieldNames)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub struct YPBankRecord {
    /// Unique transaction identifier.
    pub tx_id: u64,
    /// Type of transaction (deposit, transfer, withdrawal).
    pub tx_type: TransactionType,
    /// ID of the user initiating the transaction.
    pub from_user_id: u64,
    /// ID of the user receiving the transaction.
    pub to_user_id: u64,
    /// Transaction amount in minimal currency units (e.g., cents).
    pub amount: u64,
    /// Unix timestamp of when the transaction occurred.
    pub timestamp: u64,
    /// Current status of the transaction.
    pub status: TransactionStatus,
    /// Human-readable description of the transaction.
    pub description: String,
}

impl YPBankRecord {
    const MAGIC: &'static [u8; 4] = b"YPBN";

    // Field sizes according to specification
    const TX_ID_SIZE: usize = 8;
    const TX_TYPE_SIZE: usize = 1;
    const FROM_USER_ID_SIZE: usize = 8;
    const TO_USER_ID_SIZE: usize = 8;
    const AMOUNT_SIZE: usize = 8;
    const TIMESTAMP_SIZE: usize = 8;
    const STATUS_SIZE: usize = 1;
    const DESC_LEN_SIZE: usize = 4;

    // Fixed record body size (without description)
    const FIXED_SIZE: usize = Self::TX_ID_SIZE
        + Self::TX_TYPE_SIZE
        + Self::FROM_USER_ID_SIZE
        + Self::TO_USER_ID_SIZE
        + Self::AMOUNT_SIZE
        + Self::TIMESTAMP_SIZE
        + Self::STATUS_SIZE
        + Self::DESC_LEN_SIZE;

    fn read_from<R: Read>(reader: &mut R) -> Result<Self> {
        let mut header = [0_u8; 4];
        reader.read_exact(&mut header)?;
        if &header != Self::MAGIC {
            return Err(ParseError::BinaryFormat(format!(
                "Expected YPBN header, got: {:?}",
                &header
            )));
        }

        let _record_size = reader.read_u32::<BigEndian>()?;

        let tx_id = reader.read_u64::<BigEndian>()?;

        let mut tx_type_byte = [0_u8; 1];
        reader.read_exact(&mut tx_type_byte)?;
        let tx_type = TransactionType::try_from(tx_type_byte[0])?;

        let from_user_id = reader.read_u64::<BigEndian>()?;
        let to_user_id = reader.read_u64::<BigEndian>()?;
        let amount = reader.read_u64::<BigEndian>()?;
        let timestamp = reader.read_u64::<BigEndian>()?;

        let mut status_byte = [0_u8; 1];
        reader.read_exact(&mut status_byte)?;
        let status = TransactionStatus::try_from(status_byte[0])?;

        let desc_len = reader.read_u32::<BigEndian>()? as usize;
        let mut desc_bytes = vec![0_u8; desc_len];
        reader.read_exact(&mut desc_bytes)?;
        let mut description = String::from_utf8(desc_bytes).map_err(|e| {
            ParseError::BinaryFormat(format!(
                "Invalid UTF-8 in description: {e}"
            ))
        })?;

        if description.starts_with('"') && description.ends_with('"') {
            description = description[1..description.len() - 1].to_string();
        }

        Ok(Self {
            tx_id,
            tx_type,
            from_user_id,
            to_user_id,
            amount,
            timestamp,
            status,
            description,
        })
    }

    fn write_to<W: Write>(&self, writer: &mut W) -> Result<()> {
        let desc_bytes = self.description.as_bytes();
        let record_size = Self::FIXED_SIZE + desc_bytes.len();

        writer.write_all(Self::MAGIC)?;
        writer.write_u32::<BigEndian>(u32::try_from(record_size)?)?;

        writer.write_u64::<BigEndian>(self.tx_id)?;
        writer.write_all(&[self.tx_type as u8])?;
        writer.write_u64::<BigEndian>(self.from_user_id)?;
        writer.write_u64::<BigEndian>(self.to_user_id)?;
        writer.write_u64::<BigEndian>(self.amount)?;
        writer.write_u64::<BigEndian>(self.timestamp)?;
        writer.write_all(&[self.status as u8])?;
        writer.write_u32::<BigEndian>(u32::try_from(desc_bytes.len())?)?;
        writer.write_all(desc_bytes)?;

        Ok(())
    }
}

impl From<YPBankRecord> for serde_json::Value {
    fn from(record: YPBankRecord) -> Self {
        serde_json::json!({
            "TX_ID": record.tx_id,
            "TX_TYPE": record.tx_type.to_string(),
            "FROM_USER_ID": record.from_user_id,
            "TO_USER_ID": record.to_user_id,
            "AMOUNT": record.amount,
            "TIMESTAMP": record.timestamp,
            "STATUS": record.status.to_string(),
            "DESCRIPTION": record.description
        })
    }
}

/// Represents the type of a transaction.
///
/// # Example
///
/// ```
/// use parser::formats::TransactionType;
/// use std::str::FromStr;
///
/// let tx_type = TransactionType::from_str("TRANSFER").unwrap();
/// assert_eq!(tx_type, TransactionType::Transfer);
/// assert_eq!(tx_type.to_string(), "TRANSFER");
/// ```
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Display, EnumString, IntoStaticStr,
)]
#[repr(u8)]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum TransactionType {
    /// Money deposited into an account.
    Deposit = 0,
    /// Money transferred between accounts.
    Transfer = 1,
    /// Money withdrawn from an account.
    Withdrawal = 2,
}

impl Serialize for TransactionType {
    fn serialize<S>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.to_string().as_str())
    }
}

impl<'de> Deserialize<'de> for TransactionType {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::from_str(&s).map_err(serde::de::Error::custom)
    }
}

impl TryFrom<u8> for TransactionType {
    type Error = ParseError;

    fn try_from(value: u8) -> Result<Self> {
        match value {
            0 => Ok(Self::Deposit),
            1 => Ok(Self::Transfer),
            2 => Ok(Self::Withdrawal),
            _ => Err(ParseError::BinaryFormat(format!(
                "Invalid TX_TYPE: {value}"
            ))),
        }
    }
}

/// Represents the status of a transaction.
///
/// # Example
///
/// ```
/// use parser::formats::TransactionStatus;
/// use std::str::FromStr;
///
/// let status = TransactionStatus::from_str("SUCCESS").unwrap();
/// assert_eq!(status, TransactionStatus::Success);
/// assert_eq!(status.to_string(), "SUCCESS");
/// ```
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Display, EnumString, IntoStaticStr,
)]
#[repr(u8)]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum TransactionStatus {
    /// Transaction completed successfully.
    Success = 0,
    /// Transaction failed.
    Failure = 1,
    /// Transaction is pending processing.
    Pending = 2,
}

impl Serialize for TransactionStatus {
    fn serialize<S>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.to_string().as_str())
    }
}

impl<'de> Deserialize<'de> for TransactionStatus {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::from_str(&s).map_err(serde::de::Error::custom)
    }
}

impl TryFrom<u8> for TransactionStatus {
    type Error = ParseError;

    fn try_from(value: u8) -> Result<Self> {
        match value {
            0 => Ok(Self::Success),
            1 => Ok(Self::Failure),
            2 => Ok(Self::Pending),
            _ => Err(ParseError::BinaryFormat(format!(
                "Invalid STATUS: {value}"
            ))),
        }
    }
}

/// Trait for parsing data from a reader into a collection of items.
///
/// Implementors of this trait define how to parse their specific format
/// from raw input data.
pub trait Parser {
    /// The type of item produced by the parser.
    type Item;

    /// Parses data from a reader into a vector of items.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The input data cannot be read due to I/O issues
    /// - The data format is invalid or corrupted
    /// - Required fields are missing or have invalid values
    /// - Data deserialization fails
    fn parse<R: Read>(reader: R) -> Result<Vec<Self::Item>>;
}

/// Trait for serializing items to a writer.
///
/// Implementors of this trait define how to serialize their items
/// to a specific output format.
pub trait Serializer {
    /// The type of item to be serialized.
    type Item;

    /// Serializes a slice of items to a writer.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Writing to the output fails due to I/O issues
    /// - Data serialization fails
    /// - The writer cannot be flushed
    fn serialize<W: Write>(data: &[Self::Item], writer: W) -> Result<()>;
}

/// Marker trait for types that can both parse and serialize data.
///
/// Automatically implemented for all types that implement both Parser and Serializer.
pub trait Format: Parser + Serializer {}
impl<F: Parser + Serializer> Format for F {}

#[cfg(test)]
mod tests {
    use crate::formats::{TransactionStatus, TransactionType, YPBankRecord};
    use proptest::prelude::Strategy;
    use proptest::prop_oneof;
    use proptest::strategy::Just;
    use rstest::*;

    pub struct RecordBuilder {
        record: YPBankRecord,
    }

    impl RecordBuilder {
        pub fn new() -> Self {
            Self {
                record: YPBankRecord {
                    tx_id: 1_000_000_000_000_000,
                    tx_type: TransactionType::Deposit,
                    from_user_id: 0,
                    to_user_id: 9_223_372_036_854_775_807,
                    amount: 100,
                    timestamp: 1_633_036_860_000,
                    status: TransactionStatus::Failure,
                    description: "Record number 1".to_string(),
                },
            }
        }

        pub fn with_tx_id(mut self, tx_id: u64) -> Self {
            self.record.tx_id = tx_id;
            self
        }

        pub fn with_description(mut self, desc: impl Into<String>) -> Self {
            self.record.description = desc.into();
            self
        }

        pub fn build(self) -> YPBankRecord {
            self.record
        }
    }

    #[fixture]
    pub fn base_record() -> YPBankRecord {
        RecordBuilder::new().build()
    }

    pub fn arb_transaction_type() -> impl Strategy<Value = TransactionType> {
        prop_oneof![
            Just(TransactionType::Deposit),
            Just(TransactionType::Withdrawal),
            Just(TransactionType::Transfer),
        ]
    }

    /// Генератор для `TransactionStatus`
    pub fn arb_transaction_status() -> impl Strategy<Value = TransactionStatus>
    {
        prop_oneof![
            Just(TransactionStatus::Success),
            Just(TransactionStatus::Failure),
            Just(TransactionStatus::Pending),
        ]
    }

    /// Генератор для валидных строк (без переносов строк и двоеточий)
    pub fn arb_safe_string() -> impl Strategy<Value = String> {
        "[a-zA-Z0-9 ]{1,50}"
            .prop_filter("не должно быть двоеточий", |s| {
                !s.contains(':')
            })
    }

    /// Генератор для случайной записи
    pub fn arb_record() -> impl Strategy<Value = YPBankRecord> {
        (
            0_u64..=i64::MAX as u64, // Ограничиваем до i64::MAX для совместимости с JSON
            arb_transaction_type(),
            0_u64..=i64::MAX as u64,
            0_u64..=i64::MAX as u64,
            0_u64..=i64::MAX as u64,
            0_u64..=i64::MAX as u64,
            arb_transaction_status(),
            arb_safe_string(),
        )
            .prop_map(
                |(
                    tx_id,
                    tx_type,
                    from_user_id,
                    to_user_id,
                    amount,
                    timestamp,
                    status,
                    desc,
                )| {
                    YPBankRecord {
                        tx_id,
                        tx_type,
                        from_user_id,
                        to_user_id,
                        amount,
                        timestamp,
                        status,
                        description: desc,
                    }
                },
            )
    }
}

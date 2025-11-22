pub mod binary;
pub mod csv;
pub mod txt;

use crate::ParseError;
use crate::errors::Result;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::io::{Read, Write};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub struct YPBankRecord {
    pub tx_id: u64,
    pub tx_type: TransactionType,
    pub from_user_id: u64,
    pub to_user_id: u64,
    pub amount: u64,
    pub timestamp: u64,
    pub status: TransactionStatus,
    pub description: String,
}

impl YPBankRecord {
    const MAGIC: &'static [u8; 4] = b"YPBN";

    // Размеры полей согласно спецификации
    const TX_ID_SIZE: usize = 8;
    const TX_TYPE_SIZE: usize = 1;
    const FROM_USER_ID_SIZE: usize = 8;
    const TO_USER_ID_SIZE: usize = 8;
    const AMOUNT_SIZE: usize = 8;
    const TIMESTAMP_SIZE: usize = 8;
    const STATUS_SIZE: usize = 1;
    const DESC_LEN_SIZE: usize = 4;

    // Фиксированный размер тела записи (без description)
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

impl TryFrom<&serde_json::Map<String, serde_json::Value>> for YPBankRecord {
    type Error = ParseError;

    fn try_from(
        obj: &serde_json::Map<String, serde_json::Value>,
    ) -> Result<Self> {
        Ok(Self {
            tx_id: obj
                .get("TX_ID")
                .and_then(serde_json::Value::as_u64)
                .ok_or_else(|| {
                    ParseError::BinaryFormat(
                        "Missing or invalid TX_ID".to_string(),
                    )
                })?,
            tx_type: obj
                .get("TX_TYPE")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    ParseError::BinaryFormat(
                        "Missing or invalid TX_TYPE".to_string(),
                    )
                })
                .and_then(TransactionType::try_from)?,
            from_user_id: obj
                .get("FROM_USER_ID")
                .and_then(serde_json::Value::as_u64)
                .ok_or_else(|| {
                    ParseError::BinaryFormat(
                        "Missing or invalid FROM_USER_ID".to_string(),
                    )
                })?,
            to_user_id: obj
                .get("TO_USER_ID")
                .and_then(serde_json::Value::as_u64)
                .ok_or_else(|| {
                    ParseError::BinaryFormat(
                        "Missing or invalid TO_USER_ID".to_string(),
                    )
                })?,
            amount: obj
                .get("AMOUNT")
                .and_then(serde_json::Value::as_u64)
                .ok_or_else(|| {
                    ParseError::BinaryFormat(
                        "Missing or invalid AMOUNT".to_string(),
                    )
                })?,
            timestamp: obj
                .get("TIMESTAMP")
                .and_then(serde_json::Value::as_u64)
                .ok_or_else(|| {
                    ParseError::BinaryFormat(
                        "Missing or invalid TIMESTAMP".to_string(),
                    )
                })?,
            status: obj
                .get("STATUS")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    ParseError::BinaryFormat(
                        "Missing or invalid STATUS".to_string(),
                    )
                })
                .and_then(TransactionStatus::try_from)?,
            description: obj
                .get("DESCRIPTION")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    ParseError::BinaryFormat(
                        "Missing or invalid DESCRIPTION".to_string(),
                    )
                })?
                .to_string(),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TransactionType {
    Deposit = 0,
    Transfer = 1,
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
        Self::try_from(s.as_str()).map_err(serde::de::Error::custom)
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

impl TryFrom<&str> for TransactionType {
    type Error = ParseError;

    fn try_from(s: &str) -> Result<Self> {
        match s {
            "DEPOSIT" => Ok(Self::Deposit),
            "TRANSFER" => Ok(Self::Transfer),
            "WITHDRAWAL" => Ok(Self::Withdrawal),
            _ => Err(ParseError::BinaryFormat(format!("Unknown TX_TYPE: {s}"))),
        }
    }
}

impl fmt::Display for TransactionType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Deposit => "DEPOSIT",
                Self::Transfer => "TRANSFER",
                Self::Withdrawal => "WITHDRAWAL",
            }
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TransactionStatus {
    Success = 0,
    Failure = 1,
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
        Self::try_from(s.as_str()).map_err(serde::de::Error::custom)
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

impl TryFrom<&str> for TransactionStatus {
    type Error = ParseError;

    fn try_from(s: &str) -> Result<Self> {
        match s {
            "SUCCESS" => Ok(Self::Success),
            "FAILURE" => Ok(Self::Failure),
            "PENDING" => Ok(Self::Pending),
            _ => Err(ParseError::BinaryFormat(format!("Unknown STATUS: {s}"))),
        }
    }
}

impl fmt::Display for TransactionStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Success => "SUCCESS",
                Self::Failure => "FAILURE",
                Self::Pending => "PENDING",
            }
        )
    }
}

/// Трейт для парсинга данных из потока
pub trait Parser<T> {
    /// Парсит данные из reader в вектор записей типа T
    ///
    /// # Arguments
    /// * `reader` - Источник данных, реализующий трейт Read
    ///
    /// # Returns
    /// Возвращает вектор распарсенных записей
    fn parse<R: Read>(reader: R) -> Result<Vec<T>>;
}

/// Трейт для сериализации данных в поток
pub trait Serializer<T> {
    /// Сериализует вектор записей в writer
    ///
    /// # Arguments
    /// * `data` - Слайс записей для сериализации
    /// * `writer` - Приемник данных, реализующий трейт Write
    ///
    /// # Returns
    /// Возвращает Ok(()) при успешной сериализации или ошибку `ParseError`
    fn serialize<W: Write>(data: &[T], writer: W) -> Result<()>;
}

/// Трейт для полного формата данных (парсинг + сериализация)
pub trait Format<T>: Parser<T> + Serializer<T> {}
impl<T, F> Format<T> for F where F: Parser<T> + Serializer<T> {}

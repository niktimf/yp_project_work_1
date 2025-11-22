use crate::errors::{ParseError, Result};
use crate::formats::{Parser, Serializer, YPBankRecord};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};

pub struct YPBankBinaryFormat;

impl<T> Parser<T> for YPBankBinaryFormat
where
    T: for<'de> Deserialize<'de>,
{
    fn parse<R: Read>(mut reader: R) -> Result<Vec<T>> {
        let mut records = Vec::new();

        loop {
            match YPBankRecord::read_from(&mut reader) {
                Ok(record) => {
                    let json_obj = serde_json::json!({
                        "TX_ID": record.tx_id,
                        "TX_TYPE": record.tx_type.to_string(),
                        "FROM_USER_ID": record.from_user_id,
                        "TO_USER_ID": record.to_user_id,
                        "AMOUNT": record.amount,
                        "TIMESTAMP": record.timestamp,
                        "STATUS": record.status.to_string(),
                        "DESCRIPTION": record.description
                    });

                    let item: T = serde_json::from_value(json_obj)?;
                    records.push(item);
                }
                Err(ParseError::Io(e))
                    if e.kind() == std::io::ErrorKind::UnexpectedEof =>
                {
                    break;
                }
                Err(e) => return Err(e),
            }
        }

        Ok(records)
    }
}

impl<T> Serializer<T> for YPBankBinaryFormat
where
    T: Serialize,
{
    fn serialize<W: Write>(data: &[T], mut writer: W) -> Result<()> {
        for item in data {
            let json_value = serde_json::to_value(item)?;
            let obj = json_value.as_object().ok_or_else(|| {
                ParseError::BinaryFormat(
                    "Expected object for serialization".to_string(),
                )
            })?;

            let record = YPBankRecord::try_from(obj)?;
            record.write_to(&mut writer)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formats::{TransactionStatus, TransactionType, YPBankRecord};
    use std::fs::File;

    #[test]
    fn test_binary_parsing() {
        let file = File::open("src/test_data/records_example.bin").unwrap();
        let records: Vec<YPBankRecord> =
            YPBankBinaryFormat::parse(file).unwrap();

        // Проверяем что файл содержит данные
        assert!(!records.is_empty());

        // Проверяем первую запись
        let first_record = &records[0];
        assert_eq!(first_record.tx_id, 1000000000000000);
        assert_eq!(first_record.tx_type, TransactionType::Deposit);
        assert_eq!(first_record.from_user_id, 0);
        assert_eq!(first_record.to_user_id, 9223372036854775807);
        assert_eq!(first_record.amount, 100);
        assert_eq!(first_record.timestamp, 1633036860000);
        assert_eq!(first_record.status, TransactionStatus::Failure);
        assert_eq!(first_record.description, "Record number 1");

        println!("Загружено {} записей из Binary файла", records.len());
        println!("Первая запись: {:?}", first_record);
    }

    #[test]
    fn test_binary_serialization() {
        let records = vec![YPBankRecord {
            tx_id: 1000000000000000,
            tx_type: TransactionType::Deposit,
            from_user_id: 0,
            to_user_id: 9223372036854775807,
            amount: 100,
            timestamp: 1633036860000,
            status: TransactionStatus::Failure,
            description: "Record number 1".to_string(),
        }];

        let mut buffer = Vec::new();
        YPBankBinaryFormat::serialize(&records, &mut buffer).unwrap();

        // Проверяем что данные сериализованы (бинарные данные не пусты)
        assert!(!buffer.is_empty());

        // Проверяем что можем обратно десериализовать
        let parsed_records: Vec<YPBankRecord> =
            YPBankBinaryFormat::parse(buffer.as_slice()).unwrap();
        assert_eq!(parsed_records.len(), 1);
        assert_eq!(parsed_records[0], records[0]);

        println!("Сериализовано {} байт в binary формате", buffer.len());
        println!("Round-trip тест прошел успешно");
    }
}

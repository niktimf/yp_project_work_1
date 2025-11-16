use crate::errors::Result;
use crate::formats::{Parser, Serializer};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};

/// CSV формат данных
pub struct CsvFormat;

impl<T> Parser<T> for CsvFormat
where
    T: for<'de> Deserialize<'de>,
{
    fn parse<R: Read>(reader: R) -> Result<Vec<T>> {
        let mut csv_reader = csv::ReaderBuilder::new()
            .has_headers(true)
            .from_reader(reader);

        let mut records = Vec::new();
        for result in csv_reader.deserialize() {
            records.push(result?);
        }

        Ok(records)
    }
}

impl<T> Serializer<T> for CsvFormat
where
    T: Serialize,
{
    fn serialize<W: Write>(data: &[T], writer: W) -> Result<()> {
        let mut csv_writer = csv::Writer::from_writer(writer);

        for item in data {
            csv_writer.serialize(item)?;
        }

        csv_writer.flush()?;
        Ok(())
    }
}

/// Удобная функция для сериализации в строку
pub fn serialize_to_string<T>(data: &[T]) -> Result<String>
where
    T: Serialize,
{
    let mut buffer = Vec::new();
    CsvFormat::serialize(data, &mut buffer)?;
    Ok(String::from_utf8(buffer)?)
}

#[cfg(test)]
mod tests {
    use crate::formats::csv::CsvFormat;
    use crate::formats::{Parser, Serializer, TransactionStatus, TransactionType, YPBankRecord};
    use std::fs::File;

    #[test]
    fn test_csv_parsing() {
        let file = File::open("src/test_data/records_example.csv").unwrap();
        let records: Vec<YPBankRecord> = CsvFormat::parse(file).unwrap();

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

        println!("Загружено {} записей из CSV файла", records.len());
        println!("Первая запись: {:?}", first_record);
    }

    #[test]
    fn test_csv_serialization() {
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
        CsvFormat::serialize(&records, &mut buffer).unwrap();
        let csv_string = String::from_utf8(buffer).unwrap();
        assert!(csv_string.contains("1000000000000000,DEPOSIT,0,9223372036854775807,100,1633036860000,FAILURE,Record number 1"));
        println!("Сериализованный CSV:\n{}", csv_string);
    }
}

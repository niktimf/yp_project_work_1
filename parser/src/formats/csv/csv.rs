use std::io::{Cursor, Read, Write};
use crate::errors::Result;
use crate::formats::{Parser, Serializer};
use serde::{Deserialize, Serialize};

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
    use std::fs::File;
    use super::*;
    use serde::{Deserialize, Serialize};
    use crate::formats::{Parser, Serializer};
    use crate::formats::csv::CsvFormat;

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct BankRecord {
        #[serde(rename = "TX_ID")]
        pub tx_id: i64,
        #[serde(rename = "TX_TYPE")]
        pub tx_type: String,
        #[serde(rename = "FROM_USER_ID")]
        pub from_user_id: i64,
        #[serde(rename = "TO_USER_ID")]
        pub to_user_id: i64,
        #[serde(rename = "AMOUNT")]
        pub amount: i64,
        #[serde(rename = "TIMESTAMP")]
        pub timestamp: i64,
        #[serde(rename = "STATUS")]
        pub status: String,
        #[serde(rename = "DESCRIPTION")]
        pub description: String,
    }

    #[test]
    fn test_csv_parsing() {
        let file = File::open("src/test_data/records_example.csv").unwrap();
        let records: Vec<BankRecord> = CsvFormat::parse(file).unwrap();

        // Проверяем первую запись
        let first_record = &records[0];
        assert_eq!(first_record.tx_id, 1000000000000000);
        assert_eq!(first_record.tx_type, "DEPOSIT");
        assert_eq!(first_record.from_user_id, 0);
        assert_eq!(first_record.to_user_id, 9223372036854775807);
        assert_eq!(first_record.amount, 100);
        assert_eq!(first_record.timestamp, 1633036860000);
        assert_eq!(first_record.status, "FAILURE");
        assert_eq!(first_record.description, "Record number 1");

        println!("Загружено {} записей из CSV файла", records.len());
        println!("Первая запись: {:?}", first_record);
    }

    #[test]
    fn test_csv_serialization() {
        let records = vec![
            BankRecord {
                tx_id: 1000000000000000,
                tx_type: "DEPOSIT".to_string(),
                from_user_id: 0,
                to_user_id: 9223372036854775807,
                amount: 100,
                timestamp: 1633036860000,
                status: "FAILURE".to_string(),
                description: "Record number 1".to_string(),
            },
        ];

        let mut buffer = Vec::new();
        CsvFormat::serialize(&records, &mut buffer).unwrap();
        let csv_string = String::from_utf8(buffer).unwrap();

        // Проверяем что CSV содержит ожидаемые данные
        assert!(csv_string.contains("TX_ID,TX_TYPE,FROM_USER_ID,TO_USER_ID,AMOUNT,TIMESTAMP,STATUS,DESCRIPTION"));
        assert!(csv_string.contains("1000000000000000,DEPOSIT,0,9223372036854775807,100,1633036860000,FAILURE,\"Record number 1\""));

        println!("Сериализованный CSV:\n{}", csv_string);
    }
}

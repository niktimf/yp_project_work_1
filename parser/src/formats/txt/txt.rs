use crate::errors::Result;
use crate::formats::{Parser, Serializer};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Read, Write};

/// Text формат данных
pub struct TextFormat;

#[derive(Debug)]
enum ParseState {
    WaitingForRecord,
    InRecord(BTreeMap<String, String>),
}

impl<T> Parser<T> for TextFormat
where
    T: for<'de> Deserialize<'de>,
{
    fn parse<R: Read>(reader: R) -> Result<Vec<T>> {
        let mut buf_reader = BufReader::new(reader);
        let mut records = Vec::new();
        let mut state = ParseState::WaitingForRecord;

        let mut line = String::new();
        while buf_reader.read_line(&mut line)? > 0 {
            let trimmed = line.trim();

            if trimmed.is_empty() {
                // Пустая строка - завершаем текущую запись если есть
                if let ParseState::InRecord(fields) = state {
                    if !fields.is_empty() {
                        let record = deserialize_fields(fields)?;
                        records.push(record);
                    }
                    state = ParseState::WaitingForRecord;
                }
            } else if trimmed.starts_with("# Record") {
                // Начало новой записи
                match state {
                    ParseState::InRecord(fields) => {
                        // Сохраняем предыдущую запись
                        if !fields.is_empty() {
                            let record = deserialize_fields(fields)?;
                            records.push(record);
                        }
                    }
                    ParseState::WaitingForRecord => {}
                }
                state = ParseState::InRecord(BTreeMap::new());
            } else if let ParseState::InRecord(ref mut fields) = state {
                // Парсим поле только если мы внутри записи
                if let Some((key, value)) = parse_field(trimmed) {
                    fields.insert(key.to_string(), clean_value(value));
                }
            }

            line.clear();
        }

        // Обрабатываем последнюю запись если EOF
        if let ParseState::InRecord(fields) = state
            && !fields.is_empty()
        {
            let record = deserialize_fields(fields)?;
            records.push(record);
        }

        Ok(records)
    }
}

fn parse_field(line: &str) -> Option<(&str, &str)> {
    let parts: Vec<&str> = line.splitn(2, ':').collect();
    if parts.len() == 2 {
        Some((parts[0].trim(), parts[1].trim()))
    } else {
        None
    }
}

fn clean_value(value: &str) -> String {
    value.trim_matches('"').to_string()
}

fn deserialize_fields<T>(fields: BTreeMap<String, String>) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    // Преобразуем строковые значения в правильные типы JSON
    let mut json_map = serde_json::Map::new();

    for (key, value) in fields {
        let json_value = if value.starts_with('"') && value.ends_with('"') {
            // Это строка
            serde_json::Value::String(value.trim_matches('"').to_string())
        } else if let Ok(num) = value.parse::<i64>() {
            // Это число
            serde_json::Value::Number(serde_json::Number::from(num))
        } else {
            // Оставляем как строку
            serde_json::Value::String(value)
        };
        json_map.insert(key, json_value);
    }

    let json_value = serde_json::Value::Object(json_map);
    let item = serde_json::from_value(json_value)?;
    Ok(item)
}

impl<T> Serializer<T> for TextFormat
where
    T: Serialize,
{
    fn serialize<W: Write>(data: &[T], mut writer: W) -> Result<()> {
        for (index, item) in data.iter().enumerate() {
            // Сериализуем в JSON, затем в текстовый формат
            let json_value = serde_json::to_value(item)?;

            if let serde_json::Value::Object(map) = json_value {
                writeln!(writer, "# Record {}", index + 1)?;

                for (key, value) in map {
                    let value_str = match value {
                        serde_json::Value::String(s) => format!("\"{s}\""),
                        _ => value.to_string(),
                    };
                    writeln!(writer, "{}: {}", key.to_uppercase(), value_str)?;
                }

                writeln!(writer)?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use std::fs::File;

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct BankRecord {
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
    fn test_txt_parsing() {
        let file = File::open("src/test_data/records_example.txt").unwrap();
        let records: Vec<BankRecord> = TextFormat::parse(file).unwrap();

        // Проверяем что файл содержит данные
        assert!(!records.is_empty());

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

        println!("Загружено {} записей из TXT файла", records.len());
        println!("Первая запись: {:?}", first_record);
    }

    #[test]
    fn test_txt_serialization() {
        let records = vec![BankRecord {
            tx_id: 1000000000000000,
            tx_type: "DEPOSIT".to_string(),
            from_user_id: 0,
            to_user_id: 9223372036854775807,
            amount: 100,
            timestamp: 1633036860000,
            status: "FAILURE".to_string(),
            description: "Record number 1".to_string(),
        }];

        let mut buffer = Vec::new();
        TextFormat::serialize(&records, &mut buffer).unwrap();
        let txt_string = String::from_utf8(buffer).unwrap();

        // Проверяем что TXT содержит ожидаемые данные
        assert!(txt_string.contains("# Record 1"));
        assert!(txt_string.contains("TX_ID: 1000000000000000"));
        assert!(txt_string.contains("TX_TYPE: \"DEPOSIT\""));
        assert!(txt_string.contains("DESCRIPTION: \"Record number 1\""));

        println!("Сериализованный TXT:\n{}", txt_string);
    }
}

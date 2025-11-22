use crate::errors::Result;
use crate::formats::{Parser, Serializer};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};

/// CSV формат данных
pub struct YPBankCSV;

impl<T> Parser<T> for YPBankCSV
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

impl<T> Serializer<T> for YPBankCSV
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formats::{TransactionStatus, TransactionType, YPBankRecord};
    use rstest::*;
    use std::fs::File;

    #[fixture]
    fn simple_csv() -> &'static str {
        r#"TX_ID,TX_TYPE,FROM_USER_ID,TO_USER_ID,AMOUNT,TIMESTAMP,STATUS,DESCRIPTION
1000000000000000,DEPOSIT,0,9223372036854775807,100,1633036860000,FAILURE,Record number 1
"#
    }

    #[fixture]
    fn multi_record_csv() -> &'static str {
        r#"TX_ID,TX_TYPE,FROM_USER_ID,TO_USER_ID,AMOUNT,TIMESTAMP,STATUS,DESCRIPTION
1001,DEPOSIT,0,501,50000,1672531200000,SUCCESS,Initial account funding
1002,TRANSFER,501,502,15000,1672534800000,FAILURE,"Payment for services, invoice #123"
1003,WITHDRAWAL,502,0,1000,1672538400000,PENDING,ATM withdrawal
"#
    }

    mod parsing {
        use super::*;
        use crate::formats::tests::base_record;

        #[rstest]
        fn test_simple_parsing(simple_csv: &str, base_record: YPBankRecord) {
            let records: Vec<YPBankRecord> =
                YPBankCSV::parse(simple_csv.as_bytes()).unwrap();

            assert_eq!(records.len(), 1);
            assert_eq!(records[0], base_record);
        }

        #[test]
        fn test_parsing_from_real_file() {
            let file = File::open("src/test_data/records_example.csv").unwrap();
            let records: Vec<YPBankRecord> = YPBankCSV::parse(file).unwrap();

            assert!(!records.is_empty());
            assert_eq!(records[0].tx_id, 1000000000000000);
            assert_eq!(records[0].tx_type, TransactionType::Deposit);
        }

        #[rstest]
        fn test_multiple_records(multi_record_csv: &str) {
            let records: Vec<YPBankRecord> =
                YPBankCSV::parse(multi_record_csv.as_bytes()).unwrap();

            assert_eq!(records.len(), 3);

            // Первая запись
            assert_eq!(records[0].tx_id, 1001);
            assert_eq!(records[0].tx_type, TransactionType::Deposit);
            assert_eq!(records[0].status, TransactionStatus::Success);

            // Вторая запись
            assert_eq!(records[1].tx_id, 1002);
            assert_eq!(records[1].tx_type, TransactionType::Transfer);
            assert_eq!(
                records[1].description,
                "Payment for services, invoice #123"
            );

            // Третья запись
            assert_eq!(records[2].tx_id, 1003);
            assert_eq!(records[2].tx_type, TransactionType::Withdrawal);
        }

        /// Пустой CSV (только заголовок)
        #[rstest]
        fn test_empty_csv() {
            let csv = "TX_ID,TX_TYPE,FROM_USER_ID,TO_USER_ID,AMOUNT,TIMESTAMP,STATUS,DESCRIPTION\n";
            let records: Vec<YPBankRecord> =
                YPBankCSV::parse(csv.as_bytes()).unwrap();

            assert_eq!(records.len(), 0);
        }

        /// ВАЖНО: Явно проверяем каждый тип транзакции
        #[rstest]
        #[case(TransactionType::Deposit, "DEPOSIT")]
        #[case(TransactionType::Withdrawal, "WITHDRAWAL")]
        #[case(TransactionType::Transfer, "TRANSFER")]
        fn test_all_transaction_types(
            #[case] tx_type: TransactionType,
            #[case] tx_type_str: &str,
        ) {
            let csv = format!(
                "TX_ID,TX_TYPE,FROM_USER_ID,TO_USER_ID,AMOUNT,TIMESTAMP,STATUS,DESCRIPTION\n\
                 1,{},0,1,100,1633036860000,SUCCESS,Test\n",
                tx_type_str
            );

            let records: Vec<YPBankRecord> =
                YPBankCSV::parse(csv.as_bytes()).unwrap();

            assert_eq!(records[0].tx_type, tx_type);
        }

        /// ВАЖНО: Явно проверяем каждый статус
        #[rstest]
        #[case(TransactionStatus::Success, "SUCCESS")]
        #[case(TransactionStatus::Failure, "FAILURE")]
        #[case(TransactionStatus::Pending, "PENDING")]
        fn test_all_statuses(
            #[case] status: TransactionStatus,
            #[case] status_str: &str,
        ) {
            let csv = format!(
                "TX_ID,TX_TYPE,FROM_USER_ID,TO_USER_ID,AMOUNT,TIMESTAMP,STATUS,DESCRIPTION\n\
                 1,DEPOSIT,0,1,100,1633036860000,{},Test\n",
                status_str
            );

            let records: Vec<YPBankRecord> =
                YPBankCSV::parse(csv.as_bytes()).unwrap();

            assert_eq!(records[0].status, status);
        }

        /// ВАЖНО: Граничные значения для чисел
        #[rstest]
        #[case(i64::MAX as u64, "максимум i64")]
        #[case(u64::MIN, "минимум")]
        #[case(0, "ноль")]
        fn test_numeric_boundaries(#[case] value: u64, #[case] _desc: &str) {
            let csv = format!(
                "TX_ID,TX_TYPE,FROM_USER_ID,TO_USER_ID,AMOUNT,TIMESTAMP,STATUS,DESCRIPTION\n\
                 {},DEPOSIT,0,1,100,1633036860000,SUCCESS,Test\n",
                value
            );

            let records: Vec<YPBankRecord> =
                YPBankCSV::parse(csv.as_bytes()).unwrap();

            assert_eq!(records[0].tx_id, value);
        }
    }

    // ============================================================================
    // Integration Tests - Сериализация
    // ============================================================================

    mod serialization {
        use super::*;
        use crate::formats::tests::RecordBuilder;
        use crate::formats::tests::base_record;

        #[rstest]
        fn test_basic_serialization(base_record: YPBankRecord) {
            let records = vec![base_record];
            let mut buffer = Vec::new();

            YPBankCSV::serialize(&records, &mut buffer).unwrap();
            let csv_string = String::from_utf8(buffer).unwrap();

            // Проверяем наличие заголовка
            assert!(csv_string.contains("TX_ID,TX_TYPE,FROM_USER_ID,TO_USER_ID,AMOUNT,TIMESTAMP,STATUS,DESCRIPTION"));

            // Проверяем наличие данных
            assert!(csv_string.contains("1000000000000000"));
            assert!(csv_string.contains("DEPOSIT"));
            assert!(csv_string.contains("FAILURE"));
        }

        #[rstest]
        fn test_multiple_records_serialization() {
            let records = vec![
                RecordBuilder::new().with_tx_id(1).build(),
                RecordBuilder::new().with_tx_id(2).build(),
                RecordBuilder::new().with_tx_id(3).build(),
            ];

            let mut buffer = Vec::new();
            YPBankCSV::serialize(&records, &mut buffer).unwrap();
            let csv_string = String::from_utf8(buffer).unwrap();
            let lines: Vec<&str> = csv_string.lines().collect();

            // Заголовок + 3 записи
            assert_eq!(lines.len(), 4);
        }

        #[rstest]
        fn test_empty_serialization() {
            let records: Vec<YPBankRecord> = vec![];
            let mut buffer = Vec::new();
            YPBankCSV::serialize(&records, &mut buffer).unwrap();
            let csv_string = String::from_utf8(buffer).unwrap();

            // Только заголовок
            assert!(csv_string.starts_with("TX_ID,TX_TYPE"));
            assert_eq!(csv_string.lines().count(), 1);
        }
    }

    // ============================================================================
    // Edge Cases - CSV-специфичные случаи
    // ============================================================================

    mod edge_cases {
        use super::*;
        use crate::formats::tests::RecordBuilder;

        /// КРИТИЧНО для CSV: запятые в данных
        #[rstest]
        fn test_commas_in_description() {
            let record = RecordBuilder::new()
                .with_description("Payment for services, invoice #123, urgent")
                .build();

            let mut buffer = Vec::new();
            YPBankCSV::serialize(&vec![record], &mut buffer).unwrap();
            let parsed: Vec<YPBankRecord> =
                YPBankCSV::parse(&buffer[..]).unwrap();

            assert_eq!(
                parsed[0].description,
                "Payment for services, invoice #123, urgent"
            );
        }

        /// КРИТИЧНО для CSV: кавычки в данных
        #[rstest]
        fn test_quotes_in_description() {
            let record = RecordBuilder::new()
                .with_description(r#"Transaction "urgent" priority"#)
                .build();

            let mut buffer = Vec::new();
            YPBankCSV::serialize(&vec![record], &mut buffer).unwrap();
            let parsed: Vec<YPBankRecord> =
                YPBankCSV::parse(&buffer[..]).unwrap();

            assert_eq!(
                parsed[0].description,
                r#"Transaction "urgent" priority"#
            );
        }

        /// Переносы строк в описании
        #[rstest]
        fn test_newlines_in_description() {
            let record = RecordBuilder::new()
                .with_description("Line 1\nLine 2\nLine 3")
                .build();

            let mut buffer = Vec::new();
            YPBankCSV::serialize(&vec![record], &mut buffer).unwrap();
            let parsed: Vec<YPBankRecord> =
                YPBankCSV::parse(&buffer[..]).unwrap();

            assert_eq!(parsed[0].description, "Line 1\nLine 2\nLine 3");
        }

        /// Пустое описание
        #[rstest]
        fn test_empty_description() {
            let record = RecordBuilder::new().with_description("").build();

            let mut buffer = Vec::new();
            YPBankCSV::serialize(&vec![record], &mut buffer).unwrap();
            let parsed: Vec<YPBankRecord> =
                YPBankCSV::parse(&buffer[..]).unwrap();

            assert_eq!(parsed[0].description, "");
        }

        /// Юникод символы
        #[rstest]
        fn test_unicode_in_description() {
            let record = RecordBuilder::new()
                .with_description("Перевод средств 💰 на сумму €100")
                .build();

            let mut buffer = Vec::new();
            YPBankCSV::serialize(&vec![record], &mut buffer).unwrap();
            let parsed: Vec<YPBankRecord> =
                YPBankCSV::parse(&buffer[..]).unwrap();

            assert_eq!(
                parsed[0].description,
                "Перевод средств 💰 на сумму €100"
            );
        }

        /// CSV с пустыми строками между записями (должны игнорироваться)
        #[rstest]
        fn test_empty_lines_in_csv() {
            let csv = r#"TX_ID,TX_TYPE,FROM_USER_ID,TO_USER_ID,AMOUNT,TIMESTAMP,STATUS,DESCRIPTION
1,DEPOSIT,0,1,100,1633036860000,SUCCESS,First

2,DEPOSIT,0,1,200,1633036860000,SUCCESS,Second
"#;

            let records: Vec<YPBankRecord> =
                YPBankCSV::parse(csv.as_bytes()).unwrap();

            // Библиотека csv может игнорировать пустые строки или нет - проверяем
            assert!(records.len() >= 2);
            assert_eq!(records[0].tx_id, 1);
            assert_eq!(records[1].tx_id, 2);
        }

        /// Очень длинное описание
        #[rstest]
        fn test_long_description() {
            let long_desc = "A".repeat(1000);
            let record =
                RecordBuilder::new().with_description(&long_desc).build();

            let mut buffer = Vec::new();
            YPBankCSV::serialize(&vec![record], &mut buffer).unwrap();
            let parsed: Vec<YPBankRecord> =
                YPBankCSV::parse(&buffer[..]).unwrap();

            assert_eq!(parsed[0].description, long_desc);
        }

        /// Специальные CSV символы в описании
        #[rstest]
        fn test_special_csv_characters() {
            let special_chars = r#"Comma,Quote",Newline
,Tab	,Carriage Return"#;
            let record =
                RecordBuilder::new().with_description(special_chars).build();

            let mut buffer = Vec::new();
            YPBankCSV::serialize(&vec![record], &mut buffer).unwrap();
            let parsed: Vec<YPBankRecord> =
                YPBankCSV::parse(&buffer[..]).unwrap();

            assert_eq!(parsed[0].description, special_chars);
        }
    }

    // ============================================================================
    // Error Handling - Невалидные CSV
    // ============================================================================

    mod error_handling {
        use super::*;

        /// CSV без заголовка
        #[rstest]
        fn test_missing_header() {
            let csv = "1,DEPOSIT,0,1,100,1633036860000,SUCCESS,Test\n";
            let result: Result<Vec<YPBankRecord>> =
                YPBankCSV::parse(csv.as_bytes());

            // Должна быть ошибка (библиотека csv попытается использовать первую строку как заголовок)
            assert!(result.is_err());
        }

        /// CSV с неправильным заголовком
        #[rstest]
        fn test_wrong_header() {
            let csv = "WRONG,HEADERS,HERE\n1,DEPOSIT,0\n";
            let result: Result<Vec<YPBankRecord>> =
                YPBankCSV::parse(csv.as_bytes());

            assert!(result.is_err());
        }

        /// CSV с недостаточным количеством полей
        #[rstest]
        fn test_insufficient_fields() {
            let csv = r#"TX_ID,TX_TYPE,FROM_USER_ID,TO_USER_ID,AMOUNT,TIMESTAMP,STATUS,DESCRIPTION
1,DEPOSIT,0
"#;
            let result: Result<Vec<YPBankRecord>> =
                YPBankCSV::parse(csv.as_bytes());

            assert!(result.is_err());
        }

        /// CSV с неправильным типом транзакции
        #[rstest]
        fn test_invalid_transaction_type() {
            let csv = r#"TX_ID,TX_TYPE,FROM_USER_ID,TO_USER_ID,AMOUNT,TIMESTAMP,STATUS,DESCRIPTION
1,INVALID_TYPE,0,1,100,1633036860000,SUCCESS,Test
"#;
            let result: Result<Vec<YPBankRecord>> =
                YPBankCSV::parse(csv.as_bytes());

            assert!(result.is_err());
        }

        /// CSV с неправильным статусом
        #[rstest]
        fn test_invalid_status() {
            let csv = r#"TX_ID,TX_TYPE,FROM_USER_ID,TO_USER_ID,AMOUNT,TIMESTAMP,STATUS,DESCRIPTION
1,DEPOSIT,0,1,100,1633036860000,INVALID_STATUS,Test
"#;
            let result: Result<Vec<YPBankRecord>> =
                YPBankCSV::parse(csv.as_bytes());

            assert!(result.is_err());
        }

        /// CSV с нечисловым значением в числовом поле
        #[rstest]
        fn test_invalid_numeric_field() {
            let csv = r#"TX_ID,TX_TYPE,FROM_USER_ID,TO_USER_ID,AMOUNT,TIMESTAMP,STATUS,DESCRIPTION
not_a_number,DEPOSIT,0,1,100,1633036860000,SUCCESS,Test
"#;
            let result: Result<Vec<YPBankRecord>> =
                YPBankCSV::parse(csv.as_bytes());

            assert!(result.is_err());
        }
    }

    // ============================================================================
    // Property-Based Tests
    // ============================================================================

    #[cfg(test)]
    mod property_tests {
        use super::*;
        use proptest::prelude::*;

        fn arb_transaction_type() -> impl Strategy<Value = TransactionType> {
            prop_oneof![
                Just(TransactionType::Deposit),
                Just(TransactionType::Withdrawal),
                Just(TransactionType::Transfer),
            ]
        }

        fn arb_transaction_status() -> impl Strategy<Value = TransactionStatus>
        {
            prop_oneof![
                Just(TransactionStatus::Success),
                Just(TransactionStatus::Failure),
                Just(TransactionStatus::Pending),
            ]
        }

        /// Для CSV нужны строки БЕЗ переносов строк (т.к. они имеют специальное значение)
        /// Но запятые и кавычки допустимы - библиотека csv должна их правильно обрабатывать
        fn arb_csv_safe_string() -> impl Strategy<Value = String> {
            // Разрешаем запятые и кавычки - библиотека должна с ними справиться
            r#"[a-zA-Z0-9 ,"']{1,100}"#
        }

        fn arb_record() -> impl Strategy<Value = YPBankRecord> {
            (
                0_u64..=i64::MAX as u64,
                arb_transaction_type(),
                0_u64..=i64::MAX as u64,
                0_u64..=i64::MAX as u64,
                0_u64..=i64::MAX as u64,
                0_u64..=i64::MAX as u64,
                arb_transaction_status(),
                arb_csv_safe_string(),
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

        proptest! {
            /// Основное свойство: serialize -> parse -> должно вернуть исходные данные
            #[test]
            fn prop_roundtrip(records in prop::collection::vec(arb_record(), 1..50)) {
                let mut buffer = Vec::new();
                YPBankCSV::serialize(&records, &mut buffer)?;
                let parsed: Vec<YPBankRecord> = YPBankCSV::parse(&buffer[..])?;
                prop_assert_eq!(records, parsed);
            }

            /// Парсер не должен паниковать ни на каких данных
            #[test]
            fn prop_parser_robust(data in prop::collection::vec(any::<u8>(), 0..1000)) {
                let _: Result<Vec<YPBankRecord>> = YPBankCSV::parse(&data[..]);
            }

            /// Количество записей сохраняется
            #[test]
            fn prop_record_count_preserved(
                records in prop::collection::vec(arb_record(), 1..50)
            ) {
                let mut buffer = Vec::new();
                YPBankCSV::serialize(&records, &mut buffer)?;
                let parsed: Vec<YPBankRecord> = YPBankCSV::parse(&buffer[..])?;
                prop_assert_eq!(records.len(), parsed.len());
            }

            /// Сериализация идемпотентна
            #[test]
            fn prop_serialization_idempotent(record in arb_record()) {
                let records = vec![record];

                let mut buffer1 = Vec::new();
                YPBankCSV::serialize(&records, &mut buffer1)?;

                let mut buffer2 = Vec::new();
                YPBankCSV::serialize(&records, &mut buffer2)?;

                prop_assert_eq!(buffer1, buffer2);
            }
        }
    }
}

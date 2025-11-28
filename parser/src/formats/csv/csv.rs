use crate::ParseError;
use crate::errors::Result;
use crate::formats::{
    Parser, Serializer, YP_BANK_RECORD_UPPERCASE_FIELDS, YPBankRecord,
};
use std::io::{Read, Write};

/// CSV format parser and serializer for `YPBank` records.
///
/// Handles parsing and serialization of transaction records in CSV format
/// with specific header requirements and field validation.
///
/// # Example
///
/// ```
/// use parser::formats::csv::YPBankCSV;
/// use parser::formats::{Parser, Serializer};
///
/// # fn example() -> parser::Result<()> {
/// let csv_data = "TX_ID,TX_TYPE,FROM_USER_ID,TO_USER_ID,AMOUNT,TIMESTAMP,STATUS,DESCRIPTION
/// 1,TRANSFER,100,200,5000,1234567890,SUCCESS,Payment";
///
/// let records = YPBankCSV::parse(csv_data.as_bytes())?;
///
/// let mut output = Vec::new();
/// YPBankCSV::serialize(&records, &mut output)?;
/// # Ok(())
/// # }
/// ```
pub struct YPBankCSV;

impl Parser for YPBankCSV {
    type Item = YPBankRecord;
    fn parse<R: Read>(reader: R) -> Result<Vec<YPBankRecord>> {
        let mut csv_reader = csv::ReaderBuilder::new()
            .has_headers(true)
            .from_reader(reader);

        let headers = csv_reader.headers()?;
        let expected = YP_BANK_RECORD_UPPERCASE_FIELDS.to_vec();

        if headers.len() != expected.len() {
            return Err(ParseError::InvalidField {
                field: "headers".to_string(),
                value: format!(
                    "Expected {} columns, got {}",
                    expected.len(),
                    headers.len()
                ),
            });
        }

        for (i, (got, exp)) in headers.iter().zip(&expected).enumerate() {
            if got != *exp {
                return Err(ParseError::InvalidField {
                    field: format!("column_{i}"),
                    value: format!("expected '{exp}', got '{got}'"),
                });
            }
        }

        let mut records = Vec::new();
        for result in csv_reader.deserialize() {
            records.push(result?);
        }

        Ok(records)
    }
}

impl Serializer for YPBankCSV {
    type Item = YPBankRecord;
    fn serialize<W: Write>(data: &[YPBankRecord], writer: W) -> Result<()> {
        let mut csv_writer = csv::WriterBuilder::new()
            .has_headers(false)
            .from_writer(writer);

        let headers = YP_BANK_RECORD_UPPERCASE_FIELDS.to_vec();
        csv_writer.write_record(headers)?;

        for record in data {
            csv_writer.serialize(record)?;
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

    #[fixture]
    fn simple_csv() -> &'static str {
        r"TX_ID,TX_TYPE,FROM_USER_ID,TO_USER_ID,AMOUNT,TIMESTAMP,STATUS,DESCRIPTION
1000000000000000,DEPOSIT,0,9223372036854775807,100,1633036860000,FAILURE,Record number 1
"
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

        #[rstest]
        fn test_multiple_records(multi_record_csv: &str) {
            let records: Vec<YPBankRecord> =
                YPBankCSV::parse(multi_record_csv.as_bytes()).unwrap();

            assert_eq!(records.len(), 3);

            assert_eq!(records[0].tx_id, 1001);
            assert_eq!(records[0].tx_type, TransactionType::Deposit);
            assert_eq!(records[0].status, TransactionStatus::Success);

            assert_eq!(records[1].tx_id, 1002);
            assert_eq!(records[1].tx_type, TransactionType::Transfer);
            assert_eq!(
                records[1].description,
                "Payment for services, invoice #123"
            );

            assert_eq!(records[2].tx_id, 1003);
            assert_eq!(records[2].tx_type, TransactionType::Withdrawal);
        }

        #[rstest]
        fn test_empty_csv() {
            let csv = "TX_ID,TX_TYPE,FROM_USER_ID,TO_USER_ID,AMOUNT,TIMESTAMP,STATUS,DESCRIPTION\n";
            let records: Vec<YPBankRecord> =
                YPBankCSV::parse(csv.as_bytes()).unwrap();

            assert_eq!(records.len(), 0);
        }

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
                 1,{tx_type_str},0,1,100,1633036860000,SUCCESS,Test\n"
            );

            let records: Vec<YPBankRecord> =
                YPBankCSV::parse(csv.as_bytes()).unwrap();

            assert_eq!(records[0].tx_type, tx_type);
        }

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
                 1,DEPOSIT,0,1,100,1633036860000,{status_str},Test\n"
            );

            let records: Vec<YPBankRecord> =
                YPBankCSV::parse(csv.as_bytes()).unwrap();

            assert_eq!(records[0].status, status);
        }

        #[rstest]
        #[case(i64::MAX as u64, "i64 maximum")]
        #[case(u64::MIN, "minimum")]
        #[case(0, "zero")]
        fn test_numeric_boundaries(#[case] value: u64, #[case] desc: &str) {
            let _ = desc;
            let csv = format!(
                "TX_ID,TX_TYPE,FROM_USER_ID,TO_USER_ID,AMOUNT,TIMESTAMP,STATUS,DESCRIPTION\n\
                 {value},DEPOSIT,0,1,100,1633036860000,SUCCESS,Test\n"
            );

            let records: Vec<YPBankRecord> =
                YPBankCSV::parse(csv.as_bytes()).unwrap();

            assert_eq!(records[0].tx_id, value);
        }
    }

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

            assert!(csv_string.contains("TX_ID,TX_TYPE,FROM_USER_ID,TO_USER_ID,AMOUNT,TIMESTAMP,STATUS,DESCRIPTION"));

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

            assert!(lines.len() >= 4);
            assert!(lines.len() <= 5);
        }

        #[rstest]
        fn test_empty_serialization() {
            let records: Vec<YPBankRecord> = vec![];
            let mut buffer = Vec::new();
            YPBankCSV::serialize(&records, &mut buffer).unwrap();
            let csv_string = String::from_utf8(buffer).unwrap();

            assert!(csv_string.starts_with("TX_ID,TX_TYPE"));
            assert_eq!(csv_string.lines().count(), 1);
        }
    }

    mod edge_cases {
        use super::*;
        use crate::formats::tests::RecordBuilder;

        #[rstest]
        fn test_commas_in_description() {
            let record = RecordBuilder::new()
                .with_description("Payment for services, invoice #123, urgent")
                .build();

            let mut buffer = Vec::new();
            YPBankCSV::serialize(&[record], &mut buffer).unwrap();
            let parsed: Vec<YPBankRecord> =
                YPBankCSV::parse(&buffer[..]).unwrap();

            assert_eq!(
                parsed[0].description,
                "Payment for services, invoice #123, urgent"
            );
        }

        #[rstest]
        fn test_quotes_in_description() {
            let record = RecordBuilder::new()
                .with_description(r#"Transaction "urgent" priority"#)
                .build();

            let mut buffer = Vec::new();
            YPBankCSV::serialize(&[record], &mut buffer).unwrap();
            let parsed: Vec<YPBankRecord> =
                YPBankCSV::parse(&buffer[..]).unwrap();

            assert_eq!(
                parsed[0].description,
                r#"Transaction "urgent" priority"#
            );
        }

        #[rstest]
        fn test_newlines_in_description() {
            let record = RecordBuilder::new()
                .with_description("Line 1\nLine 2\nLine 3")
                .build();

            let mut buffer = Vec::new();
            YPBankCSV::serialize(&[record], &mut buffer).unwrap();
            let parsed: Vec<YPBankRecord> =
                YPBankCSV::parse(&buffer[..]).unwrap();

            assert_eq!(parsed[0].description, "Line 1\nLine 2\nLine 3");
        }

        #[rstest]
        fn test_empty_description() {
            let record = RecordBuilder::new().with_description("").build();

            let mut buffer = Vec::new();
            YPBankCSV::serialize(&[record], &mut buffer).unwrap();

            let parsed: Vec<YPBankRecord> =
                YPBankCSV::parse(&buffer[..]).unwrap();

            assert_eq!(parsed[0].description, "");
        }

        #[rstest]
        fn test_unicode_in_description() {
            let record = RecordBuilder::new()
                .with_description("Money transfer 💰 for €100")
                .build();

            let mut buffer = Vec::new();
            YPBankCSV::serialize(&[record], &mut buffer).unwrap();
            let parsed: Vec<YPBankRecord> =
                YPBankCSV::parse(&buffer[..]).unwrap();

            assert_eq!(parsed[0].description, "Money transfer 💰 for €100");
        }

        #[rstest]
        fn test_empty_lines_in_csv() {
            let csv = r"TX_ID,TX_TYPE,FROM_USER_ID,TO_USER_ID,AMOUNT,TIMESTAMP,STATUS,DESCRIPTION
1,DEPOSIT,0,1,100,1633036860000,SUCCESS,First

2,DEPOSIT,0,1,200,1633036860000,SUCCESS,Second
";

            let records: Vec<YPBankRecord> =
                YPBankCSV::parse(csv.as_bytes()).unwrap();

            assert!(records.len() >= 2);
            assert_eq!(records[0].tx_id, 1);
            assert_eq!(records[1].tx_id, 2);
        }

        #[rstest]
        fn test_long_description() {
            let long_desc = "A".repeat(1000);
            let record =
                RecordBuilder::new().with_description(&long_desc).build();

            let mut buffer = Vec::new();
            YPBankCSV::serialize(&[record], &mut buffer).unwrap();
            let parsed: Vec<YPBankRecord> =
                YPBankCSV::parse(&buffer[..]).unwrap();

            assert_eq!(parsed[0].description, long_desc);
        }

        #[rstest]
        fn test_special_csv_characters() {
            let special_chars = r#"Comma,Quote",Newline
,Tab	,Carriage Return"#;
            let record =
                RecordBuilder::new().with_description(special_chars).build();

            let mut buffer = Vec::new();
            YPBankCSV::serialize(&[record], &mut buffer).unwrap();
            let parsed: Vec<YPBankRecord> =
                YPBankCSV::parse(&buffer[..]).unwrap();

            assert_eq!(parsed[0].description, special_chars);
        }
    }

    mod error_handling {
        use super::*;

        #[rstest]
        fn test_missing_header() {
            let csv = "1,DEPOSIT,0,1,100,1633036860000,SUCCESS,Test\n";
            let result: Result<Vec<YPBankRecord>> =
                YPBankCSV::parse(csv.as_bytes());

            assert!(result.is_err());
        }

        #[rstest]
        fn test_wrong_header() {
            let csv = "WRONG,HEADERS,HERE\n1,DEPOSIT,0\n";
            let result: Result<Vec<YPBankRecord>> =
                YPBankCSV::parse(csv.as_bytes());

            assert!(result.is_err());
        }

        #[rstest]
        fn test_insufficient_fields() {
            let csv = r"TX_ID,TX_TYPE,FROM_USER_ID,TO_USER_ID,AMOUNT,TIMESTAMP,STATUS,DESCRIPTION
1,DEPOSIT,0
";
            let result: Result<Vec<YPBankRecord>> =
                YPBankCSV::parse(csv.as_bytes());

            assert!(result.is_err());
        }

        #[rstest]
        fn test_invalid_transaction_type() {
            let csv = r"TX_ID,TX_TYPE,FROM_USER_ID,TO_USER_ID,AMOUNT,TIMESTAMP,STATUS,DESCRIPTION
1,INVALID_TYPE,0,1,100,1633036860000,SUCCESS,Test
";
            let result: Result<Vec<YPBankRecord>> =
                YPBankCSV::parse(csv.as_bytes());

            assert!(result.is_err());
        }

        #[rstest]
        fn test_invalid_status() {
            let csv = r"TX_ID,TX_TYPE,FROM_USER_ID,TO_USER_ID,AMOUNT,TIMESTAMP,STATUS,DESCRIPTION
1,DEPOSIT,0,1,100,1633036860000,INVALID_STATUS,Test
";
            let result: Result<Vec<YPBankRecord>> =
                YPBankCSV::parse(csv.as_bytes());

            assert!(result.is_err());
        }

        #[rstest]
        fn test_invalid_numeric_field() {
            let csv = r"TX_ID,TX_TYPE,FROM_USER_ID,TO_USER_ID,AMOUNT,TIMESTAMP,STATUS,DESCRIPTION
not_a_number,DEPOSIT,0,1,100,1633036860000,SUCCESS,Test
";
            let result: Result<Vec<YPBankRecord>> =
                YPBankCSV::parse(csv.as_bytes());

            assert!(result.is_err());
        }
    }

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

        fn arb_csv_safe_string() -> impl Strategy<Value = String> {
            "[a-zA-Z0-9 ,.!?;:_+=(){}*&^%$#@~`-]{1,100}"
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
            #[test]
            fn prop_roundtrip(records in prop::collection::vec(arb_record(), 1..50)) {
                let mut buffer = Vec::new();
                YPBankCSV::serialize(&records, &mut buffer)?;
                let parsed: Vec<YPBankRecord> = YPBankCSV::parse(&buffer[..])?;
                prop_assert_eq!(records, parsed);
            }

            #[test]
            fn prop_parser_robust(data in prop::collection::vec(any::<u8>(), 0..1000)) {
                let _: Result<Vec<YPBankRecord>> = YPBankCSV::parse(&data[..]);
            }

            #[test]
            fn prop_record_count_preserved(
                records in prop::collection::vec(arb_record(), 1..50)
            ) {
                let mut buffer = Vec::new();
                YPBankCSV::serialize(&records, &mut buffer)?;
                let parsed: Vec<YPBankRecord> = YPBankCSV::parse(&buffer[..])?;
                prop_assert_eq!(records.len(), parsed.len());
            }

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

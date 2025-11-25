use crate::errors::Result;
use crate::formats::{Parser, Serializer, YPBankRecord};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Read, Write};

pub struct YPBankText;

#[derive(Debug)]
enum ParseState {
    WaitingForRecord,
    InRecord(BTreeMap<String, String>),
}

impl Parser for YPBankText {
    type Item = YPBankRecord;
    fn parse<R: Read>(reader: R) -> Result<Vec<YPBankRecord>> {
        let mut buf_reader = BufReader::new(reader);
        let mut records = Vec::new();
        let mut state = ParseState::WaitingForRecord;

        let mut line = String::new();
        while buf_reader.read_line(&mut line)? > 0 {
            let trimmed = line.trim();

            if trimmed.is_empty() {
                // Empty line - finish current record if exists
                if let ParseState::InRecord(fields) = state {
                    if !fields.is_empty() {
                        let record = deserialize_fields(fields)?;
                        records.push(record);
                    }
                    state = ParseState::WaitingForRecord;
                }
            } else if trimmed.starts_with('#') {
                // Ignore any comments
            } else {
                // This should be a record field
                match state {
                    ParseState::WaitingForRecord => {
                        // Start a new record
                        state = ParseState::InRecord(BTreeMap::new());
                    }
                    ParseState::InRecord(_) => {}
                }

                if let ParseState::InRecord(ref mut fields) = state
                    && let Some((key, value)) = parse_field(trimmed)
                {
                    fields.insert(key.to_string(), clean_value(value));
                }
            }

            line.clear();
        }

        // Process last record if EOF
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
    let mut json_map = serde_json::Map::new();

    for (key, value) in fields {
        let json_value = if value.starts_with('"') && value.ends_with('"') {
            serde_json::Value::String(value.trim_matches('"').to_string())
        } else {
            infer_field_type(&key, &value)
        };
        json_map.insert(key, json_value);
    }

    let json_value = serde_json::Value::Object(json_map);
    let item = serde_json::from_value(json_value)?;
    Ok(item)
}

/// Determines JSON value type for a field based on its name and value.
/// This approach is based on knowledge of the `YPBankRecord` schema and is
/// a pragmatic solution for the text format.
fn infer_field_type(field_name: &str, value: &str) -> serde_json::Value {
    let upper_field = field_name.to_uppercase();

    // Fields with numeric types (u64) - they should always be numbers
    if matches!(
        upper_field.as_str(),
        "TX_ID" | "FROM_USER_ID" | "TO_USER_ID" | "AMOUNT" | "TIMESTAMP"
    ) && let Ok(num) = value.parse::<u64>()
    {
        return serde_json::Value::Number(serde_json::Number::from(num));
    }

    // Fields with enum types and strings - they should be strings for custom deserializers
    if matches!(upper_field.as_str(), "TX_TYPE" | "STATUS" | "DESCRIPTION") {
        return serde_json::Value::String(value.to_string());
    }

    value.parse::<u64>().map_or_else(
        |_| serde_json::Value::String(value.to_string()),
        |num| serde_json::Value::Number(serde_json::Number::from(num)),
    )
}

impl Serializer for YPBankText {
    type Item = YPBankRecord;
    fn serialize<W: Write>(data: &[YPBankRecord], mut writer: W) -> Result<()> {
        for (index, record) in data.iter().enumerate() {
            let json_value = serde_json::to_value(record)?;

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
    use crate::formats::{TransactionStatus, TransactionType, YPBankRecord};
    use rstest::*;

    #[fixture]
    fn simple_txt() -> &'static str {
        r#"TX_ID: 1000000000000000
TX_TYPE: DEPOSIT
FROM_USER_ID: 0
TO_USER_ID: 9223372036854775807
AMOUNT: 100
TIMESTAMP: 1633036860000
STATUS: FAILURE
DESCRIPTION: "Record number 1"
"#
    }

    #[fixture]
    fn txt_with_comments() -> &'static str {
        r#"# This is a comment
# Another comment line
TX_ID: 1234567890123456
TX_TYPE: DEPOSIT
FROM_USER_ID: 0
TO_USER_ID: 9876543210987654
AMOUNT: 10000
# Comment in the middle
TIMESTAMP: 1633036800000
STATUS: SUCCESS
DESCRIPTION: "Terminal deposit"

# Comment between records
TX_ID: 2312321321321321
TIMESTAMP: 1633056800000
STATUS: FAILURE
TX_TYPE: TRANSFER
FROM_USER_ID: 1231231231231231
TO_USER_ID: 9876543210987654
AMOUNT: 1000
DESCRIPTION: "User transfer"
"#
    }

    mod field_parsing {
        use super::*;

        #[rstest]
        #[case("KEY: value", Some(("KEY", "value")))]
        #[case("TX_ID: 123", Some(("TX_ID", "123")))]
        #[case("AMOUNT: 1000", Some(("AMOUNT", "1000")))]
        #[case("  SPACED  :  value  ", Some(("SPACED", "value")))]
        #[case("NO_COLON", None)]
        #[case("MULTIPLE:COLONS:HERE", Some(("MULTIPLE", "COLONS:HERE")))]
        #[case("", None)]
        fn test_parse_field(
            #[case] input: &str,
            #[case] expected: Option<(&str, &str)>,
        ) {
            assert_eq!(parse_field(input), expected);
        }

        #[rstest]
        #[case(r#""quoted""#, "quoted")]
        #[case("not_quoted", "not_quoted")]
        #[case(r#""with spaces""#, "with spaces")]
        #[case(r#""""#, "")]
        fn test_clean_value(#[case] input: &str, #[case] expected: &str) {
            assert_eq!(clean_value(input), expected);
        }
    }

    mod parsing {
        use super::*;
        use crate::formats::tests::base_record;

        #[rstest]
        fn test_simple_parsing(simple_txt: &str, base_record: YPBankRecord) {
            let records: Vec<YPBankRecord> =
                YPBankText::parse(simple_txt.as_bytes()).unwrap();

            assert_eq!(records.len(), 1);
            assert_eq!(records[0], base_record);
        }

        #[rstest]
        fn test_parsing_with_comments(txt_with_comments: &str) {
            let records: Vec<YPBankRecord> =
                YPBankText::parse(txt_with_comments.as_bytes()).unwrap();

            assert_eq!(records.len(), 2);

            assert_eq!(records[0].tx_id, 1_234_567_890_123_456);
            assert_eq!(records[0].tx_type, TransactionType::Deposit);
            assert_eq!(records[0].status, TransactionStatus::Success);
            assert_eq!(records[0].description, "Terminal deposit");

            assert_eq!(records[1].tx_id, 2_312_321_321_321_321);
            assert_eq!(records[1].tx_type, TransactionType::Transfer);
            assert_eq!(records[1].status, TransactionStatus::Failure);
        }

        #[rstest]
        #[case("", vec![])]
        #[case("# Only comments\n# More comments", vec![])]
        #[case("\n\n\n", vec![])]
        fn test_empty_inputs(
            #[case] input: &str,
            #[case] expected: Vec<YPBankRecord>,
        ) {
            let records: Vec<YPBankRecord> =
                YPBankText::parse(input.as_bytes()).unwrap();
            assert_eq!(records, expected);
        }

        #[rstest]
        #[case(TransactionType::Deposit, "DEPOSIT")]
        #[case(TransactionType::Withdrawal, "WITHDRAWAL")]
        #[case(TransactionType::Transfer, "TRANSFER")]
        fn test_transaction_type_parsing(
            #[case] tx_type: TransactionType,
            #[case] tx_type_str: &str,
        ) {
            let txt = format!(
                r#"TX_ID: 1
TX_TYPE: {tx_type_str}
FROM_USER_ID: 0
TO_USER_ID: 1
AMOUNT: 100
TIMESTAMP: 1633036860000
STATUS: SUCCESS
DESCRIPTION: "Test"
"#
            );

            let records: Vec<YPBankRecord> =
                YPBankText::parse(txt.as_bytes()).unwrap();

            assert_eq!(records[0].tx_type, tx_type);
        }

        #[rstest]
        #[case(TransactionStatus::Success, "SUCCESS")]
        #[case(TransactionStatus::Failure, "FAILURE")]
        #[case(TransactionStatus::Pending, "PENDING")]
        fn test_status_parsing(
            #[case] status: TransactionStatus,
            #[case] status_str: &str,
        ) {
            let txt = format!(
                r#"TX_ID: 1
TX_TYPE: DEPOSIT
FROM_USER_ID: 0
TO_USER_ID: 1
AMOUNT: 100
TIMESTAMP: 1633036860000
STATUS: {status_str}
DESCRIPTION: "Test"
"#
            );

            let records: Vec<YPBankRecord> =
                YPBankText::parse(txt.as_bytes()).unwrap();

            assert_eq!(records[0].status, status);
        }

        #[rstest]
        #[case(i64::MAX as u64)]
        #[case(u64::MIN)]
        #[case(0)]
        #[case(1_000_000)]
        fn test_boundary_values(#[case] value: u64) {
            let txt = format!(
                r#"TX_ID: {value}
TX_TYPE: DEPOSIT
FROM_USER_ID: 0
TO_USER_ID: 1
AMOUNT: 100
TIMESTAMP: 1633036860000
STATUS: SUCCESS
DESCRIPTION: "Test"
"#
            );

            let records: Vec<YPBankRecord> =
                YPBankText::parse(txt.as_bytes()).unwrap();

            assert_eq!(records[0].tx_id, value);
        }
    }

    mod serialization {
        use super::*;
        use crate::formats::tests::base_record;

        #[rstest]
        fn test_basic_serialization(base_record: YPBankRecord) {
            let records = vec![base_record];
            let mut buffer = Vec::new();

            YPBankText::serialize(&records, &mut buffer).unwrap();
            let txt_string = String::from_utf8(buffer).unwrap();

            assert!(txt_string.contains("# Record 1"));
            assert!(txt_string.contains("TX_ID: 1000000000000000"));
            assert!(txt_string.contains("TX_TYPE: \"DEPOSIT\""));
            assert!(txt_string.contains("DESCRIPTION: \"Record number 1\""));
        }

        #[rstest]
        fn test_empty_serialization() {
            let records: Vec<YPBankRecord> = vec![];
            let mut buffer = Vec::new();

            YPBankText::serialize(&records, &mut buffer).unwrap();

            assert_eq!(buffer.len(), 0);
        }
    }

    #[cfg(test)]
    mod property_tests {
        use super::*;
        use crate::formats::tests::arb_record;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn prop_roundtrip_single_record(record in arb_record()) {
                let original = vec![record];

                let mut buffer = Vec::new();
                YPBankText::serialize(&original, &mut buffer)?;

                let parsed: Vec<YPBankRecord> = YPBankText::parse(&buffer[..])?;

                prop_assert_eq!(original, parsed);
            }

            #[test]
            fn prop_roundtrip_multiple_records(
                records in prop::collection::vec(arb_record(), 0..100)
            ) {
                if records.is_empty() {
                    return Ok(());
                }

                let mut buffer = Vec::new();
                YPBankText::serialize(&records, &mut buffer)?;
                let parsed: Vec<YPBankRecord> = YPBankText::parse(&buffer[..])?;

                prop_assert_eq!(records, parsed);
            }
        }

        proptest! {
            #[test]
            fn prop_parser_never_panics(data in prop::collection::vec(any::<u8>(), 0..1000)) {
                let _: Result<Vec<YPBankRecord>> = YPBankText::parse(&data[..]);
            }
        }

        proptest! {
            #[test]
            fn prop_record_count_preserved(
                records in prop::collection::vec(arb_record(), 1..50)
            ) {
                let original_count = records.len();

                let mut buffer = Vec::new();
                YPBankText::serialize(&records, &mut buffer)?;
                let parsed: Vec<YPBankRecord> = YPBankText::parse(&buffer[..])?;

                prop_assert_eq!(original_count, parsed.len());
            }
        }

        proptest! {
            #[test]
            fn prop_field_order_irrelevant(record in arb_record()) {
                let txt1 = format!(
                    "TX_ID: {}\nTX_TYPE: {}\nAMOUNT: {}\nSTATUS: {}\nDESCRIPTION: \"{}\"\nFROM_USER_ID: {}\nTO_USER_ID: {}\nTIMESTAMP: {}\n\n",
                    record.tx_id, record.tx_type, record.amount, record.status,
                    record.description, record.from_user_id, record.to_user_id, record.timestamp
                );

                let txt2 = format!(
                    "DESCRIPTION: \"{}\"\nTIMESTAMP: {}\nSTATUS: {}\nAMOUNT: {}\nTO_USER_ID: {}\nFROM_USER_ID: {}\nTX_TYPE: {}\nTX_ID: {}\n\n",
                    record.description, record.timestamp, record.status, record.amount,
                    record.to_user_id, record.from_user_id, record.tx_type, record.tx_id
                );

                let parsed1: Vec<YPBankRecord> = YPBankText::parse(txt1.as_bytes())?;
                let parsed2: Vec<YPBankRecord> = YPBankText::parse(txt2.as_bytes())?;

                prop_assert_eq!(parsed1, parsed2);
            }
        }

        proptest! {
            #[test]
            fn prop_comments_ignored(
                record in arb_record(),
                comment in "# [a-zA-Z0-9 ]{0,50}"
            ) {
                let records = vec![record];

                let mut buffer1 = Vec::new();
                YPBankText::serialize(&records, &mut buffer1)?;
                let parsed1: Vec<YPBankRecord> = YPBankText::parse(&buffer1[..])?;

                let mut buffer2 = Vec::new();
                writeln!(&mut buffer2, "{comment}")?;
                YPBankText::serialize(&records, &mut buffer2)?;
                let parsed2: Vec<YPBankRecord> = YPBankText::parse(&buffer2[..])?;

                prop_assert_eq!(parsed1, parsed2);
            }
        }
    }

    mod edge_cases {
        use super::*;
        use crate::formats::tests::RecordBuilder;

        #[rstest]
        fn test_record_without_trailing_newline() {
            let txt = r#"TX_ID: 1000000000000000
TX_TYPE: DEPOSIT
FROM_USER_ID: 0
TO_USER_ID: 9223372036854775807
AMOUNT: 100
TIMESTAMP: 1633036860000
STATUS: FAILURE
DESCRIPTION: "No trailing newline""#;

            let records: Vec<YPBankRecord> =
                YPBankText::parse(txt.as_bytes()).unwrap();

            assert_eq!(records.len(), 1);
        }

        #[rstest]
        fn test_multiple_empty_lines_between_records() {
            let txt = r#"TX_ID: 1
TX_TYPE: DEPOSIT
FROM_USER_ID: 0
TO_USER_ID: 1
AMOUNT: 100
TIMESTAMP: 1633036860000
STATUS: SUCCESS
DESCRIPTION: "First"



TX_ID: 2
TX_TYPE: DEPOSIT
FROM_USER_ID: 0
TO_USER_ID: 1
AMOUNT: 200
TIMESTAMP: 1633036860000
STATUS: SUCCESS
DESCRIPTION: "Second"
"#;

            let records: Vec<YPBankRecord> =
                YPBankText::parse(txt.as_bytes()).unwrap();

            assert_eq!(records.len(), 2);
        }

        #[rstest]
        fn test_special_characters_in_description() {
            let special_desc =
                r#"Special: "quotes", \backslash, newline\n, tab\t"#;
            let record =
                RecordBuilder::new().with_description(special_desc).build();

            let original = vec![record];

            let mut buffer = Vec::new();
            YPBankText::serialize(&original, &mut buffer).unwrap();
            let parsed: Vec<YPBankRecord> =
                YPBankText::parse(&buffer[..]).unwrap();

            assert_eq!(original[0].description, parsed[0].description);
        }
    }
}

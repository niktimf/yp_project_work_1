use crate::errors::{ParseError, Result};
use crate::formats::{Parser, Serializer, YPBankRecord};
use std::io::{Read, Write};

/// Binary format parser and serializer for YPBank records.
///
/// Implements a custom binary format with the following structure:
/// - Magic number: "YPBN" (4 bytes)
/// - Record size: u32 big-endian (4 bytes)
/// - Transaction fields in binary format
///
/// This format is more efficient for storage and transmission compared
/// to text formats but is not human-readable.
///
/// # Example
///
/// ```
/// use parser::formats::binary::YPBankBin;
/// use parser::formats::{Parser, Serializer, YPBankRecord, TransactionType, TransactionStatus};
///
/// # fn example() -> parser::Result<()> {
/// let record = YPBankRecord {
///     tx_id: 1,
///     tx_type: TransactionType::Transfer,
///     from_user_id: 100,
///     to_user_id: 200,
///     amount: 5000,
///     timestamp: 1234567890,
///     status: TransactionStatus::Success,
///     description: "Payment".to_string(),
/// };
///
/// let mut buffer = Vec::new();
/// YPBankBin::serialize(&[record], &mut buffer)?;
///
/// let parsed = YPBankBin::parse(buffer.as_slice())?;
/// assert_eq!(parsed.len(), 1);
/// # Ok(())
/// # }
/// ```
pub struct YPBankBin;

impl Parser for YPBankBin {
    type Item = YPBankRecord;
    fn parse<R: Read>(mut reader: R) -> Result<Vec<YPBankRecord>> {
        let mut records = Vec::new();

        loop {
            match YPBankRecord::read_from(&mut reader) {
                Ok(record) => records.push(record),
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

impl Serializer for YPBankBin {
    type Item = YPBankRecord;
    fn serialize<W: Write>(data: &[YPBankRecord], mut writer: W) -> Result<()> {
        for record in data {
            record.write_to(&mut writer)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formats::{TransactionStatus, TransactionType, YPBankRecord};
    use proptest::prelude::*;

    #[test]
    fn test_binary_parsing_generated_data() {
        let test_records = vec![
            YPBankRecord {
                tx_id: 1_000_000_000_000_000,
                tx_type: TransactionType::Deposit,
                from_user_id: 0,
                to_user_id: 9_223_372_036_854_775_807,
                amount: 100,
                timestamp: 1_633_036_860_000,
                status: TransactionStatus::Failure,
                description: "Record number 1".to_string(),
            },
            YPBankRecord {
                tx_id: 1_000_000_000_000_001,
                tx_type: TransactionType::Transfer,
                from_user_id: 9_223_372_036_854_775_807,
                to_user_id: 1_234_567_890,
                amount: 50000,
                timestamp: 1_633_036_870_000,
                status: TransactionStatus::Success,
                description: "Transfer between accounts".to_string(),
            },
        ];

        let mut buffer = Vec::new();
        YPBankBin::serialize(&test_records, &mut buffer).unwrap();

        let parsed_records: Vec<YPBankRecord> =
            YPBankBin::parse(buffer.as_slice()).unwrap();

        assert_eq!(parsed_records.len(), test_records.len());

        assert_eq!(parsed_records[0].tx_id, 1_000_000_000_000_000);
        assert_eq!(parsed_records[0].tx_type, TransactionType::Deposit);
        assert_eq!(parsed_records[0].from_user_id, 0);
        assert_eq!(parsed_records[0].to_user_id, 9_223_372_036_854_775_807);
        assert_eq!(parsed_records[0].amount, 100);
        assert_eq!(parsed_records[0].timestamp, 1_633_036_860_000);
        assert_eq!(parsed_records[0].status, TransactionStatus::Failure);
        assert_eq!(parsed_records[0].description, "Record number 1");
    }

    #[test]
    fn test_binary_serialization() {
        let records = vec![YPBankRecord {
            tx_id: 1_000_000_000_000_000,
            tx_type: TransactionType::Deposit,
            from_user_id: 0,
            to_user_id: 9_223_372_036_854_775_807,
            amount: 100,
            timestamp: 1_633_036_860_000,
            status: TransactionStatus::Failure,
            description: "Record number 1".to_string(),
        }];

        let mut buffer = Vec::new();
        YPBankBin::serialize(&records, &mut buffer).unwrap();

        assert!(!buffer.is_empty());

        let parsed_records: Vec<YPBankRecord> =
            YPBankBin::parse(buffer.as_slice()).unwrap();
        assert_eq!(parsed_records.len(), 1);
        assert_eq!(parsed_records[0], records[0]);
    }

    proptest! {
        #[test]
        fn test_binary_roundtrip_single_record(record in crate::formats::tests::arb_record()) {
            let records = vec![record.clone()];

            let mut buffer = Vec::new();
            YPBankBin::serialize(&records, &mut buffer).unwrap();

            let parsed_records: Vec<YPBankRecord> = YPBankBin::parse(buffer.as_slice()).unwrap();

            assert_eq!(parsed_records.len(), 1);
            assert_eq!(parsed_records[0], record);
        }

        #[test]
        fn test_binary_roundtrip_multiple_records(records in prop::collection::vec(crate::formats::tests::arb_record(), 1..20)) {
            let mut buffer = Vec::new();
            YPBankBin::serialize(&records, &mut buffer).unwrap();

            let parsed_records: Vec<YPBankRecord> = YPBankBin::parse(buffer.as_slice()).unwrap();

            assert_eq!(parsed_records.len(), records.len());
            for (original, parsed) in records.iter().zip(parsed_records.iter()) {
                assert_eq!(original, parsed);
            }
        }

        #[test]
        fn test_binary_format_structure(record in crate::formats::tests::arb_record()) {
            let records = vec![record];

            let mut buffer = Vec::new();
            YPBankBin::serialize(&records, &mut buffer).unwrap();

            assert!(buffer.len() >= 8, "Buffer should contain at least a header");

            assert_eq!(&buffer[0..4], b"YPBN", "Should start with YPBN magic number");

            let record_size = u32::from_be_bytes([buffer[4], buffer[5], buffer[6], buffer[7]]);
            let expected_min_size = 8 + 1 + 8 + 8 + 8 + 8 + 1 + 4; // Minimum size without description
            assert!(record_size as usize >= expected_min_size,
                   "Record size should be at least {expected_min_size}, got {record_size}");

            assert_eq!(buffer.len(), 8 + record_size as usize,
                      "Buffer size should match header + record size");
        }

        #[test]
        fn test_binary_empty_description(
            tx_id in any::<u64>(),
            tx_type in crate::formats::tests::arb_transaction_type(),
            from_user_id in any::<u64>(),
            to_user_id in any::<u64>(),
            amount in any::<u64>(),
            timestamp in any::<u64>(),
            status in crate::formats::tests::arb_transaction_status()
        ) {
            let record = YPBankRecord {
                tx_id,
                tx_type,
                from_user_id,
                to_user_id,
                amount,
                timestamp,
                status,
                description: String::new(),
            };

            let records = vec![record.clone()];
            let mut buffer = Vec::new();
            YPBankBin::serialize(&records, &mut buffer).unwrap();

            let parsed_records: Vec<YPBankRecord> = YPBankBin::parse(buffer.as_slice()).unwrap();
            assert_eq!(parsed_records[0], record);
        }

        #[test]
        fn test_binary_large_description(
            tx_id in any::<u64>(),
            description in "[a-zA-Z0-9 ]{500,1000}"
        ) {
            let record = YPBankRecord {
                tx_id,
                tx_type: TransactionType::Transfer,
                from_user_id: 1000,
                to_user_id: 2000,
                amount: 5000,
                timestamp: 1_633_036_860_000,
                status: TransactionStatus::Success,
                description,
            };

            let records = vec![record.clone()];
            let mut buffer = Vec::new();
            YPBankBin::serialize(&records, &mut buffer).unwrap();

            let parsed_records: Vec<YPBankRecord> = YPBankBin::parse(buffer.as_slice()).unwrap();
            assert_eq!(parsed_records[0], record);
        }

        #[test]
        fn test_binary_with_special_chars_in_description(
            description in prop::string::string_regex("[!@#$%^&*()_+={};:<>,.?/|\\-]{1,100}").unwrap()
        ) {
            let record = YPBankRecord {
                tx_id: 999_999,
                tx_type: TransactionType::Deposit,
                from_user_id: 1,
                to_user_id: 2,
                amount: 1000,
                timestamp: 1_700_000_000_000,
                status: TransactionStatus::Success,
                description,
            };

            let records = vec![record.clone()];
            let mut buffer = Vec::new();

            YPBankBin::serialize(&records, &mut buffer).unwrap();
            let parsed_records: Vec<YPBankRecord> = YPBankBin::parse(buffer.as_slice()).unwrap();

            assert_eq!(parsed_records.len(), 1);
            assert_eq!(parsed_records[0].description, record.description);
        }

        #[test]
        fn test_binary_edge_cases(
            records_count in 0usize..100usize
        ) {
            let records: Vec<YPBankRecord> = (0..records_count)
                .map(|i| YPBankRecord {
                    tx_id: i as u64,
                    tx_type: match i % 3 {
                        0 => TransactionType::Deposit,
                        1 => TransactionType::Transfer,
                        _ => TransactionType::Withdrawal,
                    },
                    from_user_id: if i % 3 == 0 { 0 } else { i as u64 * 100 },
                    to_user_id: if i % 3 == 2 { 0 } else { i as u64 * 200 },
                    amount: (i as u64 + 1) * 1000,
                    timestamp: 1_700_000_000_000 + i as u64 * 1000,
                    status: match i % 3 {
                        0 => TransactionStatus::Success,
                        1 => TransactionStatus::Pending,
                        _ => TransactionStatus::Failure,
                    },
                    description: format!("Transaction {i}"),
                })
                .collect();

            let mut buffer = Vec::new();
            YPBankBin::serialize(&records, &mut buffer).unwrap();

            let parsed_records: Vec<YPBankRecord> = YPBankBin::parse(buffer.as_slice()).unwrap();

            assert_eq!(parsed_records.len(), records.len());
            for (original, parsed) in records.iter().zip(parsed_records.iter()) {
                assert_eq!(original, parsed);
            }
        }
    }
}

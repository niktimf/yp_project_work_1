# YP Bank Data Processing Tools

A set of Rust utilities for processing YP Bank transaction records in multiple formats (CSV, TXT, Binary).

## Project Structure

This is a Cargo workspace containing three crates:
- `parser` - Core library for parsing and serializing YP Bank records
- `converter` - CLI tool for converting between different formats
- `comparer` - CLI tool for comparing records from two files

## Building

### Build all crates
```bash
cargo build --all
```

### Build in release mode (optimized)
```bash
cargo build --all --release
```

### Build specific binary
```bash
cargo build --bin converter
cargo build --bin comparer
```

## Running

### Converter

Convert YP Bank records between different formats (CSV, TXT, Binary).

#### Using cargo run
```bash
cargo run --bin converter -- --input <FILE> --input-format <FORMAT> --output-format <FORMAT>
```

#### Using compiled binary
```bash
./target/debug/converter --input <FILE> --input-format <FORMAT> --output-format <FORMAT>
```

#### Examples
```bash
# Convert CSV to TXT (output to stdout)
cargo run --bin converter -- --input data.csv --input-format csv --output-format txt

# Convert from stdin
cat data.csv | cargo run --bin converter -- --input - --input-format csv --output-format binary

# Using release binary
./target/release/converter --input records.txt --input-format txt --output-format csv
```

### Comparer

Compare YP Bank records from two files in any supported format.

#### Using cargo run
```bash
cargo run --bin comparer -- --first-file <FILE1> --first-file-format <FORMAT1> --second-file <FILE2> --second-file-format <FORMAT2> [--verbose]
```

#### Using compiled binary
```bash
./target/debug/comparer --first-file <FILE1> --first-file-format <FORMAT1> --second-file <FILE2> --second-file-format <FORMAT2> [--verbose]
```

#### Examples
```bash
# Compare two CSV files
cargo run --bin comparer -- --first-file file1.csv --first-file-format csv --second-file file2.csv --second-file-format csv

# Compare CSV with TXT format, show detailed differences
cargo run --bin comparer -- --first-file data.csv --first-file-format csv --second-file data.txt --second-file-format txt --verbose

# Using release binary
./target/release/comparer --first-file old.bin --first-file-format binary --second-file new.bin --second-file-format binary
```

## Supported Formats

- `csv` - Comma-separated values with headers
- `txt` - Custom text format with pipe-separated fields
- `binary` - MessagePack binary format

## Testing

Run all tests:
```bash
cargo test --all
```

Run tests for specific crate:
```bash
cargo test -p parser
cargo test -p converter
cargo test -p comparer
```

## Help

Get help for any tool:
```bash
cargo run --bin converter -- --help
cargo run --bin comparer -- --help
```

## Record Structure

YP Bank records contain the following fields:
- `TX_ID` - Transaction ID (u64)
- `TX_TYPE` - Transaction type (DEPOSIT, WITHDRAWAL, TRANSFER)
- `FROM_USER_ID` - Source user ID (u64)
- `TO_USER_ID` - Destination user ID (u64)
- `AMOUNT` - Transaction amount (u64)
- `TIMESTAMP` - Unix timestamp in milliseconds (u64)
- `STATUS` - Transaction status (SUCCESS, FAILURE, PENDING)
- `DESCRIPTION` - Transaction description (String)
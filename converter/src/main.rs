use clap::Parser;
use parser::formats::{
    YPBankRecord, binary::YPBankBin, csv::YPBankCSV, txt::YPBankText,
};
use parser::{ParseError, Result};
use std::fs::File;
use std::io::{self, BufReader, BufWriter, Read, Write};

#[derive(Parser, Debug)]
#[command(name = "ypbank_converter")]
#[command(about = "Bank records converter between formats", long_about = None)]
struct Args {
    /// Input file (or - for stdin)
    #[arg(long)]
    input: String,

    /// Input file format (csv, txt, binary)
    #[arg(long, value_enum)]
    input_format: YPBankFormat,

    /// Output file format (csv, txt, binary)
    #[arg(long, value_enum)]
    output_format: YPBankFormat,
}

#[derive(Debug, Clone, clap::ValueEnum)]
enum YPBankFormat {
    Csv,
    Txt,
    Binary,
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let args = Args::parse();

    let reader: Box<dyn Read> = if args.input == "-" {
        Box::new(io::stdin())
    } else {
        Box::new(File::open(&args.input).map_err(|e| {
            ParseError::Io(io::Error::new(
                e.kind(),
                format!("Failed to open input file '{}': {}", args.input, e),
            ))
        })?)
    };

    let records = parse_input(BufReader::new(reader), &args.input_format)?;

    if records.is_empty() {
        eprintln!("Warning: input file contains no records");
    } else {
        eprintln!("Parsed {} records", records.len());
    }

    let mut writer = BufWriter::new(io::stdout());
    serialize_output(&mut writer, &args.output_format, &records)?;
    writer.flush().map_err(ParseError::Io)?;

    eprintln!("Conversion completed successfully");
    Ok(())
}

fn parse_input<R: Read>(
    reader: BufReader<R>,
    format: &YPBankFormat,
) -> Result<Vec<YPBankRecord>> {
    use parser::formats::Parser as FormatParser;

    match format {
        YPBankFormat::Csv => YPBankCSV::parse(reader),
        YPBankFormat::Txt => YPBankText::parse(reader),
        YPBankFormat::Binary => YPBankBin::parse(reader),
    }
}

fn serialize_output<W: Write>(
    writer: W,
    format: &YPBankFormat,
    records: &[YPBankRecord],
) -> Result<()> {
    use parser::formats::Serializer;

    match format {
        YPBankFormat::Csv => YPBankCSV::serialize(records, writer),
        YPBankFormat::Txt => YPBankText::serialize(records, writer),
        YPBankFormat::Binary => YPBankBin::serialize(records, writer),
    }
}

use clap::Parser;
use parser::formats::{
    YPBankRecord, binary::YPBankBin, csv::YPBankCSV, txt::YPBankText,
};
use parser::{ParseError, Result};
use std::fs::File;
use std::io::{self, BufReader, BufWriter, Read, Write};

#[derive(Parser, Debug)]
#[command(name = "ypbank_converter")]
#[command(about = "Конвертер банковских записей между форматами", long_about = None)]
struct Args {
    /// Входной файл (или - для stdin)
    #[arg(long)]
    input: String,

    /// Формат входного файла (csv, txt, binary)
    #[arg(long, value_enum)]
    input_format: YPBankFormat,

    /// Формат выходного файла (csv, txt, binary)
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
        eprintln!("Ошибка: {e}");
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
                format!(
                    "Не удалось открыть входной файл '{}': {}",
                    args.input, e
                ),
            ))
        })?)
    };

    let records = parse_input(BufReader::new(reader), &args.input_format)?;

    if records.is_empty() {
        eprintln!("Предупреждение: входной файл не содержит записей");
    } else {
        eprintln!("Распарсено {} записей", records.len());
    }

    let mut writer = BufWriter::new(io::stdout());
    serialize_output(&mut writer, &args.output_format, &records)?;
    writer.flush().map_err(ParseError::Io)?;

    eprintln!("Конвертация завершена успешно");
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

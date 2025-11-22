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
    input_format: Format,

    /// Формат выходного файла (csv, txt, binary)
    #[arg(long, value_enum)]
    output_format: Format,
}

#[derive(Debug, Clone, clap::ValueEnum)]
enum Format {
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
    format: &Format,
) -> Result<Vec<YPBankRecord>> {
    use parser::formats::Parser as FormatParser;

    match format {
        Format::Csv => YPBankCSV::parse(reader),
        Format::Txt => YPBankText::parse(reader),
        Format::Binary => YPBankBin::parse(reader),
    }
}

fn serialize_output<W: Write>(
    writer: W,
    format: &Format,
    records: &[YPBankRecord],
) -> Result<()> {
    use parser::formats::Serializer;

    match format {
        Format::Csv => YPBankCSV::serialize(records, writer),
        Format::Txt => YPBankText::serialize(records, writer),
        Format::Binary => YPBankBin::serialize(records, writer),
    }
}

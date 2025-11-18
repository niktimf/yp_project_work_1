use clap::Parser;
use parser::formats::{YPBankRecord, binary::YPBankBinaryFormat, csv::CsvFormat, txt::TextFormat};
use parser::{ParseError, Result};
use std::fs::File;
use std::io::{self, BufReader, Read};

#[derive(Parser, Debug)]
#[command(name = "ypbank_compare")]
#[command(about = "Сравнение банковских записей из двух файлов", long_about = None)]
struct Args {
    /// Первый файл для сравнения
    #[arg(long)]
    file1: String,

    /// Формат первого файла (csv, txt, binary)
    #[arg(long, value_enum)]
    format1: Format,

    /// Второй файл для сравнения
    #[arg(long)]
    file2: String,

    /// Формат второго файла (csv, txt, binary)
    #[arg(long, value_enum)]
    format2: Format,

    /// Показывать детальную информацию о различиях
    #[arg(long, default_value_t = false)]
    verbose: bool,
}

#[derive(Debug, Clone, clap::ValueEnum)]
enum Format {
    Csv,
    Txt,
    Binary,
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Ошибка: {}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let args = Args::parse();

    let file1 = File::open(&args.file1).map_err(|e| {
        ParseError::Io(io::Error::new(
            e.kind(),
            format!("Не удалось открыть файл '{}': {}", args.file1, e),
        ))
    })?;

    let file2 = File::open(&args.file2).map_err(|e| {
        ParseError::Io(io::Error::new(
            e.kind(),
            format!("Не удалось открыть файл '{}': {}", args.file2, e),
        ))
    })?;

    let records1 = parse_file(BufReader::new(file1), &args.format1)?;
    let records2 = parse_file(BufReader::new(file2), &args.format2)?;

    compare_records(&records1, &records2, &args)?;
    Ok(())
}

fn parse_file<R: Read>(reader: BufReader<R>, format: &Format) -> Result<Vec<YPBankRecord>> {
    use parser::formats::Parser as FormatParser;

    match format {
        Format::Csv => CsvFormat::parse(reader),
        Format::Txt => TextFormat::parse(reader),
        Format::Binary => YPBankBinaryFormat::parse(reader),
    }
}

fn compare_records(
    records1: &[YPBankRecord],
    records2: &[YPBankRecord],
    args: &Args,
) -> Result<()> {
    if records1.len() != records2.len() {
        println!(
            "Файлы содержат разное количество записей: {} в '{}' и {} в '{}'",
            records1.len(),
            args.file1,
            records2.len(),
            args.file2
        );

        if args.verbose {
            if records1.len() > records2.len() {
                println!(
                    "В файле '{}' на {} записей больше",
                    args.file1,
                    records1.len() - records2.len()
                );
            } else {
                println!(
                    "В файле '{}' на {} записей больше",
                    args.file2,
                    records2.len() - records1.len()
                );
            }
        }

        return Ok(());
    }

    let mut differences = Vec::new();
    for (index, (rec1, rec2)) in records1.iter().zip(records2.iter()).enumerate() {
        if rec1 != rec2 {
            differences.push((index, rec1, rec2));
        }
    }

    if differences.is_empty() {
        println!(
            "Записи транзакций в '{}' и '{}' идентичны.",
            args.file1, args.file2
        );
    } else {
        println!(
            "Найдено {} различий между '{}' и '{}'",
            differences.len(),
            args.file1,
            args.file2
        );

        if args.verbose {
            for (index, rec1, rec2) in &differences {
                println!("\nРазличие в записи #{}", index + 1);
                print_diff(rec1, rec2);
            }
        }
    }

    Ok(())
}

fn print_diff(rec1: &YPBankRecord, rec2: &YPBankRecord) {
    if rec1.tx_id != rec2.tx_id {
        println!("  TX_ID: {} != {}", rec1.tx_id, rec2.tx_id);
    }
    if rec1.tx_type != rec2.tx_type {
        println!("  TX_TYPE: {:?} != {:?}", rec1.tx_type, rec2.tx_type);
    }
    if rec1.from_user_id != rec2.from_user_id {
        println!(
            "  FROM_USER_ID: {} != {}",
            rec1.from_user_id, rec2.from_user_id
        );
    }
    if rec1.to_user_id != rec2.to_user_id {
        println!("  TO_USER_ID: {} != {}", rec1.to_user_id, rec2.to_user_id);
    }
    if rec1.amount != rec2.amount {
        println!("  AMOUNT: {} != {}", rec1.amount, rec2.amount);
    }
    if rec1.timestamp != rec2.timestamp {
        println!("  TIMESTAMP: {} != {}", rec1.timestamp, rec2.timestamp);
    }
    if rec1.status != rec2.status {
        println!("  STATUS: {:?} != {:?}", rec1.status, rec2.status);
    }
    if rec1.description != rec2.description {
        println!(
            "  DESCRIPTION: '{}' != '{}'",
            rec1.description, rec2.description
        );
    }
}

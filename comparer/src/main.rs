use clap::Parser;
use parser::formats::{
    Parser as YPBankParser, YPBankRecord, binary::YPBankBinaryFormat, csv::CsvFormat,
    txt::TextFormat,
};
use std::fs::File;
use std::io::{BufReader, Read};

#[derive(Parser, Debug)]
#[command(name = "ypbank_compare")]
#[command(about = "Сравнение банковских записей из двух файлов", long_about = None)]
struct Args {
    /// Первый файл для сравнения
    #[arg(long)]
    first_file: String,

    /// Формат первого файла (csv, txt, binary)
    #[arg(long, value_enum)]
    first_file_format: YpBankFormat,

    /// Второй файл для сравнения
    #[arg(long)]
    second_file: String,

    /// Формат второго файла (csv, txt, binary)
    #[arg(long, value_enum)]
    second_file_format: YpBankFormat,

    /// Показывать детальную информацию о различиях
    #[arg(long, default_value_t = false)]
    verbose: bool,
}

#[derive(Debug, Clone, clap::ValueEnum)]
enum YpBankFormat {
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

fn run() -> anyhow::Result<()> {
    let args = Args::parse();

    let file1 = File::open(&args.first_file).map_err(|e| {
        anyhow::Error::new(e).context(format!("Не удалось открыть файл '{}'", args.first_file))
    })?;

    let file2 = File::open(&args.second_file).map_err(|e| {
        anyhow::Error::new(e).context(format!("Не удалось открыть файл '{}'", args.second_file))
    })?;

    let records1 = parse_file(BufReader::new(file1), &args.first_file_format)?;
    let records2 = parse_file(BufReader::new(file2), &args.second_file_format)?;

    if records1 == records2 {
        println!("Файлы идентичны");
    } else {
        println!("Файлы различаются");
        if args.verbose {
            println!("Записей в '{}': {}", args.first_file, records1.len());
            println!("Записей в '{}': {}", args.second_file, records2.len());
        }
    }
    Ok(())
}

fn parse_file<R: Read>(
    reader: BufReader<R>,
    format: &YpBankFormat,
) -> anyhow::Result<Vec<YPBankRecord>> {
    match format {
        YpBankFormat::Csv => CsvFormat::parse(reader).map_err(anyhow::Error::from),
        YpBankFormat::Txt => TextFormat::parse(reader).map_err(anyhow::Error::from),
        YpBankFormat::Binary => YPBankBinaryFormat::parse(reader).map_err(anyhow::Error::from),
    }
}

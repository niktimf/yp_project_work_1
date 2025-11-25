use clap::Parser;
use parser::formats::{
    Parser as YpBankParser, YPBankRecord, binary::YPBankBin, csv::YPBankCSV,
    txt::YPBankText,
};
use std::fs::File;
use std::io::{BufReader, Read};

#[derive(Parser, Debug)]
#[command(name = "ypbank_compare")]
#[command(about = "Compare bank records from two files", long_about = None)]
struct Args {
    /// First file to compare
    #[arg(long)]
    first_file: String,

    /// First file format (csv, txt, binary)
    #[arg(long, value_enum)]
    first_file_format: YpBankFormat,

    /// Second file to compare
    #[arg(long)]
    second_file: String,

    /// Second file format (csv, txt, binary)
    #[arg(long, value_enum)]
    second_file_format: YpBankFormat,

    /// Show detailed information about differences
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
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

fn run() -> anyhow::Result<()> {
    let args = Args::parse();

    let file1 = File::open(&args.first_file).map_err(|e| {
        anyhow::Error::new(e)
            .context(format!("Failed to open file '{}'", args.first_file))
    })?;

    let file2 = File::open(&args.second_file).map_err(|e| {
        anyhow::Error::new(e)
            .context(format!("Failed to open file '{}'", args.second_file))
    })?;

    let records1 = parse_file(BufReader::new(file1), &args.first_file_format)?;
    let records2 = parse_file(BufReader::new(file2), &args.second_file_format)?;

    if records1 == records2 {
        println!("Files are identical");
    } else {
        println!("Files differ");
        if args.verbose {
            println!("Records in '{}': {}", args.first_file, records1.len());
            println!("Records in '{}': {}", args.second_file, records2.len());
        }
    }
    Ok(())
}

fn parse_file<R: Read>(
    reader: BufReader<R>,
    format: &YpBankFormat,
) -> anyhow::Result<Vec<YPBankRecord>> {
    match format {
        YpBankFormat::Csv => {
            YPBankCSV::parse(reader).map_err(anyhow::Error::from)
        }
        YpBankFormat::Txt => {
            YPBankText::parse(reader).map_err(anyhow::Error::from)
        }
        YpBankFormat::Binary => {
            YPBankBin::parse(reader).map_err(anyhow::Error::from)
        }
    }
}

//! CLI-утилита для конвертации банковских выписок между форматами.

use clap::{Parser, ValueEnum};
use std::fs::File;
use std::io::{self, Read, Write};
use std::process;

use ypbank_parser::{Camt053Statement, Format, Mt940Statement, Statement, parse_statement};

/// Поддерживаемые форматы выписок.
#[derive(Clone, Copy, ValueEnum)]
enum FormatArg {
    /// MT940 (SWIFT)
    Mt940,
    /// CAMT.053 (ISO 20022 XML)
    Camt053,
    /// CSV
    Csv,
}

impl From<FormatArg> for Format {
    fn from(arg: FormatArg) -> Self {
        match arg {
            FormatArg::Mt940 => Format::Mt940,
            FormatArg::Camt053 => Format::Camt053,
            FormatArg::Csv => Format::Csv,
        }
    }
}

/// YPBank Converter - конвертер банковских выписок.
///
/// Поддерживает конвертацию между форматами MT940, CAMT.053 и CSV.
#[derive(Parser)]
#[command(name = "ypbank-converter")]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Входной файл (по умолчанию stdin)
    #[arg(short, long)]
    input: Option<String>,

    /// Выходной файл (по умолчанию stdout)
    #[arg(short, long)]
    output: Option<String>,

    /// Формат входных данных
    #[arg(short = 'f', long = "input-format", value_enum)]
    input_format: FormatArg,

    /// Формат выходных данных
    #[arg(short = 't', long = "output-format", value_enum)]
    output_format: FormatArg,
}

fn read_input(args: &Args) -> Result<String, String> {
    let mut content = String::new();

    if let Some(ref path) = args.input {
        let mut file = File::open(path)
            .map_err(|e| format!("Не удалось открыть файл '{}': {}", path, e))?;
        file.read_to_string(&mut content)
            .map_err(|e| format!("Не удалось прочитать файл '{}': {}", path, e))?;
    } else {
        io::stdin()
            .read_to_string(&mut content)
            .map_err(|e| format!("Не удалось прочитать stdin: {}", e))?;
    }

    Ok(content)
}

fn convert_and_write<W: Write>(
    content: &str,
    input_format: Format,
    output_format: Format,
    writer: &mut W,
) -> Result<(), String> {
    if input_format == output_format {
        writer
            .write_all(content.as_bytes())
            .map_err(|e| format!("Ошибка записи: {}", e))?;
        return Ok(());
    }

    match (input_format, output_format) {
        (Format::Mt940, Format::Camt053) => {
            let statements = Mt940Statement::parse(content)
                .map_err(|e| format!("Ошибка парсинга MT940: {}", e))?;
            for mt940 in statements {
                let camt: Camt053Statement = mt940.into();
                camt.write_to(writer)
                    .map_err(|e| format!("Ошибка записи CAMT.053: {}", e))?;
            }
        }
        (Format::Camt053, Format::Mt940) => {
            let camt = Camt053Statement::parse(content)
                .map_err(|e| format!("Ошибка парсинга CAMT.053: {}", e))?;
            let mt940: Mt940Statement = camt
                .try_into()
                .map_err(|e| format!("Ошибка конвертации CAMT.053 в MT940: {}", e))?;
            mt940
                .write_to(writer)
                .map_err(|e| format!("Ошибка записи MT940: {}", e))?;
        }
        (Format::Mt940, Format::Csv) | (Format::Camt053, Format::Csv) => {
            let statement = parse_statement(content, input_format)
                .map_err(|e| format!("Ошибка парсинга: {}", e))?;
            write_csv(&statement, writer)?;
        }
        (Format::Csv, Format::Mt940) | (Format::Csv, Format::Camt053) => {
            return Err("Конвертация из CSV в MT940/CAMT.053 не поддерживается".to_string());
        }
        (Format::Mt940, Format::Mt940)
        | (Format::Camt053, Format::Camt053)
        | (Format::Csv, Format::Csv) => {
            unreachable!("Равные форматы обрабатываются выше");
        }
    }

    Ok(())
}

fn write_csv<W: Write>(statement: &Statement, writer: &mut W) -> Result<(), String> {
    writeln!(writer, "Дата,Сумма,Валюта,Тип,Референс,Описание")
        .map_err(|e| format!("Ошибка записи: {}", e))?;

    for tx in &statement.transactions {
        let tx_type = if tx.is_credit { "Поступление" } else { "Списание" };
        let reference = tx.reference.as_deref().unwrap_or("");
        let description = tx.description.replace(',', ";").replace('\n', " ");

        writeln!(
            writer,
            "{},{:.2},{},{},{},\"{}\"",
            tx.date,
            tx.amount.as_float(),
            tx.amount.currency,
            tx_type,
            reference,
            description
        )
        .map_err(|e| format!("Ошибка записи: {}", e))?;
    }

    Ok(())
}

fn main() {
    let args = Args::parse();

    let content = match read_input(&args) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Ошибка: {}", e);
            process::exit(1);
        }
    };

    let input_format: Format = args.input_format.into();
    let output_format: Format = args.output_format.into();

    let result = if let Some(ref path) = args.output {
        let mut file = match File::create(path) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("Ошибка: Не удалось создать файл '{}': {}", path, e);
                process::exit(1);
            }
        };
        convert_and_write(&content, input_format, output_format, &mut file)
    } else {
        let mut stdout = io::stdout();
        convert_and_write(&content, input_format, output_format, &mut stdout)
    };

    if let Err(e) = result {
        eprintln!("Ошибка: {}", e);
        process::exit(1);
    }
}

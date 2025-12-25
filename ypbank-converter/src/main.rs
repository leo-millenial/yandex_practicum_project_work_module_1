//! CLI-утилита для конвертации банковских выписок между форматами.

use std::env;
use std::fs::File;
use std::io::{self, Read, Write};
use std::process;

use ypbank_parser::{
    Camt053Statement, Format, Mt940Statement, Statement, parse_statement,
};

struct Args {
    input: Option<String>,
    output: Option<String>,
    input_format: Format,
    output_format: Format,
}

fn print_usage() {
    eprintln!("YPBank Converter - конвертер банковских выписок");
    eprintln!();
    eprintln!("Использование:");
    eprintln!("  ypbank-converter [опции]");
    eprintln!();
    eprintln!("Опции:");
    eprintln!("  --input, -i <файл>         Входной файл (по умолчанию stdin)");
    eprintln!("  --output, -o <файл>        Выходной файл (по умолчанию stdout)");
    eprintln!("  --input-format, -if <формат>   Формат входных данных (mt940, camt053, csv)");
    eprintln!("  --output-format, -of <формат>  Формат выходных данных (mt940, camt053, csv)");
    eprintln!("  --help, -h                 Показать справку");
    eprintln!();
    eprintln!("Примеры:");
    eprintln!("  ypbank-converter -i statement.mt940 -if mt940 -of csv > output.csv");
    eprintln!("  ypbank-converter -if mt940 -of camt053 < input.mt940 > output.xml");
}

fn parse_args() -> Result<Args, String> {
    let args: Vec<String> = env::args().collect();

    let mut input = None;
    let mut output = None;
    let mut input_format = None;
    let mut output_format = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--help" | "-h" => {
                print_usage();
                process::exit(0);
            }
            "--input" | "-i" => {
                i += 1;
                if i >= args.len() {
                    return Err("Отсутствует значение для --input".to_string());
                }
                input = Some(args[i].clone());
            }
            "--output" | "-o" => {
                i += 1;
                if i >= args.len() {
                    return Err("Отсутствует значение для --output".to_string());
                }
                output = Some(args[i].clone());
            }
            "--input-format" | "-if" => {
                i += 1;
                if i >= args.len() {
                    return Err("Отсутствует значение для --input-format".to_string());
                }
                input_format = Format::parse(&args[i]);
                if input_format.is_none() {
                    return Err(format!("Неизвестный формат: {}", args[i]));
                }
            }
            "--output-format" | "-of" => {
                i += 1;
                if i >= args.len() {
                    return Err("Отсутствует значение для --output-format".to_string());
                }
                output_format = Format::parse(&args[i]);
                if output_format.is_none() {
                    return Err(format!("Неизвестный формат: {}", args[i]));
                }
            }
            arg => {
                return Err(format!("Неизвестный аргумент: {}", arg));
            }
        }
        i += 1;
    }

    let input_format = input_format.ok_or("Не указан формат входных данных (--input-format)")?;
    let output_format = output_format.ok_or("Не указан формат выходных данных (--output-format)")?;

    Ok(Args {
        input,
        output,
        input_format,
        output_format,
    })
}

fn read_input(args: &Args) -> Result<String, String> {
    let mut content = String::new();

    if let Some(ref path) = args.input {
        let mut file = File::open(path).map_err(|e| format!("Не удалось открыть файл '{}': {}", path, e))?;
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
            let mt940: Mt940Statement = camt.into();
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
    let args = match parse_args() {
        Ok(args) => args,
        Err(e) => {
            eprintln!("Ошибка: {}", e);
            eprintln!();
            print_usage();
            process::exit(1);
        }
    };

    let content = match read_input(&args) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Ошибка: {}", e);
            process::exit(1);
        }
    };

    let result = if let Some(ref path) = args.output {
        let mut file = match File::create(path) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("Ошибка: Не удалось создать файл '{}': {}", path, e);
                process::exit(1);
            }
        };
        convert_and_write(&content, args.input_format, args.output_format, &mut file)
    } else {
        let mut stdout = io::stdout();
        convert_and_write(&content, args.input_format, args.output_format, &mut stdout)
    };

    if let Err(e) = result {
        eprintln!("Ошибка: {}", e);
        process::exit(1);
    }
}

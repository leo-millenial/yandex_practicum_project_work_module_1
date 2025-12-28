//! CLI-утилита для сравнения транзакций из двух банковских выписок.

use clap::{Parser, ValueEnum};
use std::fs::File;
use std::io::Read;
use std::process;

use ypbank_parser::{Format, Statement, Transaction, parse_statement};

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

/// YPBank Comparer - сравнение банковских выписок.
///
/// Сравнивает транзакции из двух файлов выписок и показывает различия.
#[derive(Parser)]
#[command(name = "ypbank-comparer")]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Первый файл выписки
    #[arg(long = "file1", short = '1')]
    file1: String,

    /// Формат первого файла
    #[arg(long = "format1", short = 'a', value_enum)]
    format1: FormatArg,

    /// Второй файл выписки
    #[arg(long = "file2", short = '2')]
    file2: String,

    /// Формат второго файла
    #[arg(long = "format2", short = 'b', value_enum)]
    format2: FormatArg,

    /// Подробный вывод
    #[arg(short, long)]
    verbose: bool,
}

fn read_file(path: &str) -> Result<String, String> {
    let mut content = String::new();
    let mut file =
        File::open(path).map_err(|e| format!("Не удалось открыть файл '{}': {}", path, e))?;
    file.read_to_string(&mut content)
        .map_err(|e| format!("Не удалось прочитать файл '{}': {}", path, e))?;
    Ok(content)
}

struct ComparisonResult {
    matched: Vec<(usize, usize)>,
    only_in_first: Vec<usize>,
    only_in_second: Vec<usize>,
}

fn transactions_match(tx1: &Transaction, tx2: &Transaction) -> bool {
    tx1.date == tx2.date
        && tx1.amount.value == tx2.amount.value
        && tx1.is_credit == tx2.is_credit
}

fn calculate_match_score(tx1: &Transaction, tx2: &Transaction) -> u32 {
    let mut score = 0;

    if tx1.date == tx2.date {
        score += 10;
    }
    if tx1.amount.value == tx2.amount.value {
        score += 10;
    }
    if tx1.is_credit == tx2.is_credit {
        score += 5;
    }

    if let (Some(ref1), Some(ref2)) = (&tx1.reference, &tx2.reference) {
        if ref1 == ref2 {
            score += 15;
        }
    }

    if !tx1.description.is_empty()
        && !tx2.description.is_empty()
        && (tx1.description.contains(&tx2.description)
            || tx2.description.contains(&tx1.description))
    {
        score += 5;
    }

    score
}

fn compare_statements(stmt1: &Statement, stmt2: &Statement) -> ComparisonResult {
    let mut matched = Vec::new();
    let mut used_second = vec![false; stmt2.transactions.len()];

    for (i, tx1) in stmt1.transactions.iter().enumerate() {
        let mut best_match: Option<(usize, u32)> = None;

        for (j, tx2) in stmt2.transactions.iter().enumerate() {
            if used_second[j] {
                continue;
            }

            if transactions_match(tx1, tx2) {
                let score = calculate_match_score(tx1, tx2);
                if best_match.is_none() || score > best_match.as_ref().map(|(_, s)| *s).unwrap_or(0)
                {
                    best_match = Some((j, score));
                }
            }
        }

        if let Some((j, _)) = best_match {
            matched.push((i, j));
            used_second[j] = true;
        }
    }

    let matched_first: std::collections::HashSet<_> = matched.iter().map(|(i, _)| *i).collect();
    let only_in_first: Vec<_> = (0..stmt1.transactions.len())
        .filter(|i| !matched_first.contains(i))
        .collect();

    let only_in_second: Vec<_> = used_second
        .iter()
        .enumerate()
        .filter_map(|(i, &used)| if !used { Some(i) } else { None })
        .collect();

    ComparisonResult {
        matched,
        only_in_first,
        only_in_second,
    }
}

fn format_transaction(tx: &Transaction) -> String {
    let tx_type = if tx.is_credit { "+" } else { "-" };
    let reference = tx.reference.as_deref().unwrap_or("-");
    let description = if tx.description.len() > 50 {
        format!("{}...", &tx.description[..47])
    } else {
        tx.description.clone()
    };

    format!(
        "{} {} {:.2} {} | {} | {}",
        tx.date,
        tx_type,
        tx.amount.as_float(),
        tx.amount.currency,
        reference,
        description
    )
}

fn percent(part: usize, total: usize) -> f64 {
    if total == 0 {
        0.0
    } else {
        part as f64 / total as f64 * 100.0
    }
}

fn print_results(result: &ComparisonResult, stmt1: &Statement, stmt2: &Statement, verbose: bool) {
    println!("=== Результаты сравнения ===");
    println!();

    let total1 = stmt1.transactions.len();
    let total2 = stmt2.transactions.len();

    println!("Транзакций в файле 1: {}", total1);
    println!("Транзакций в файле 2: {}", total2);
    println!();
    println!(
        "Совпадающих транзакций: {} ({:.1}%)",
        result.matched.len(),
        percent(result.matched.len(), total1)
    );
    println!(
        "Только в файле 1: {} ({:.1}%)",
        result.only_in_first.len(),
        percent(result.only_in_first.len(), total1)
    );
    println!(
        "Только в файле 2: {} ({:.1}%)",
        result.only_in_second.len(),
        percent(result.only_in_second.len(), total2)
    );

    if verbose {
        if !result.matched.is_empty() {
            println!();
            println!("--- Совпадающие транзакции ---");
            for (i, j) in &result.matched {
                let tx1 = &stmt1.transactions[*i];
                let tx2 = &stmt2.transactions[*j];
                println!("[1] {}", format_transaction(tx1));
                println!("[2] {}", format_transaction(tx2));
                println!();
            }
        }

        if !result.only_in_first.is_empty() {
            println!();
            println!("--- Только в файле 1 ---");
            for &i in &result.only_in_first {
                let tx = &stmt1.transactions[i];
                println!("{}", format_transaction(tx));
            }
        }

        if !result.only_in_second.is_empty() {
            println!();
            println!("--- Только в файле 2 ---");
            for &i in &result.only_in_second {
                let tx = &stmt2.transactions[i];
                println!("{}", format_transaction(tx));
            }
        }
    }
}

fn main() {
    let args = Args::parse();

    let content1 = match read_file(&args.file1) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Ошибка: {}", e);
            process::exit(1);
        }
    };

    let format1: Format = args.format1.into();
    let stmt1 = match parse_statement(&content1, format1) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Ошибка в файле 1: {}", e);
            process::exit(1);
        }
    };

    let content2 = match read_file(&args.file2) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Ошибка: {}", e);
            process::exit(1);
        }
    };

    let format2: Format = args.format2.into();
    let stmt2 = match parse_statement(&content2, format2) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Ошибка в файле 2: {}", e);
            process::exit(1);
        }
    };

    let result = compare_statements(&stmt1, &stmt2);

    print_results(&result, &stmt1, &stmt2, args.verbose);

    if !result.only_in_first.is_empty() || !result.only_in_second.is_empty() {
        process::exit(1);
    }
}

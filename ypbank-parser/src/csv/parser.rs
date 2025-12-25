//! Парсер CSV формата банковских выписок.

use crate::error::{Error, Result};
use crate::types::{Account, Amount, Balance, Counterparty, Date, Statement, Transaction};
use std::io::Read;

/// Выписка в формате CSV.
#[derive(Debug, Clone)]
pub struct CsvStatement {
    pub account_number: String,
    pub account_name: String,
    pub currency: String,
    pub transactions: Vec<CsvTransaction>,
}

/// Транзакция в формате CSV.
#[derive(Debug, Clone)]
pub struct CsvTransaction {
    pub date: Date,
    pub debit_account: Option<String>,
    pub credit_account: Option<String>,
    pub debit_amount: Option<i64>,
    pub credit_amount: Option<i64>,
    pub document_number: String,
    pub bank_info: String,
    pub description: String,
}

impl CsvStatement {
    /// Парсит CSV из любого источника, реализующего трейт Read.
    pub fn from_read<R: Read>(reader: &mut R) -> Result<Self> {
        let mut content = String::new();
        reader.read_to_string(&mut content)?;
        Self::parse(&content)
    }

    /// Парсит CSV из строки.
    pub fn parse(content: &str) -> Result<Self> {
        let lines: Vec<&str> = content.lines().collect();

        if lines.len() < 12 {
            return Err(Error::InvalidFormat(
                "CSV файл слишком короткий, ожидается минимум 12 строк".to_string(),
            ));
        }

        let (account_number, account_name) = Self::parse_header(&lines)?;
        let currency = "RUB".to_string();
        let transactions = Self::parse_transactions(&lines[12..])?;

        Ok(CsvStatement {
            account_number,
            account_name,
            currency,
            transactions,
        })
    }

    fn parse_header(lines: &[&str]) -> Result<(String, String)> {
        let mut account_number = String::new();
        let mut account_name = String::new();

        for line in lines.iter().take(10) {
            if line.contains("40702") || line.contains("40703") || line.contains("40817") {
                let parts: Vec<&str> = line.split(',').collect();
                for part in parts {
                    let trimmed = part.trim().trim_matches('"');
                    if trimmed.len() == 20 && trimmed.chars().all(|c| c.is_ascii_digit()) {
                        account_number = trimmed.to_string();
                        break;
                    }
                }
            }

            if line.contains("ООО") || line.contains("ИП") || line.contains("АО") {
                let parts: Vec<&str> = line.split(',').collect();
                for part in parts {
                    let trimmed = part.trim().trim_matches('"');
                    if trimmed.contains("ООО") || trimmed.contains("ИП") || trimmed.contains("АО") {
                        account_name = trimmed.to_string();
                        break;
                    }
                }
            }
        }

        if account_number.is_empty() && lines.len() > 5 {
            let parts: Vec<&str> = lines[5].split(',').collect();
            for part in parts {
                let trimmed = part.trim().trim_matches('"');
                if trimmed.len() == 20 && trimmed.chars().all(|c| c.is_ascii_digit()) {
                    account_number = trimmed.to_string();
                    break;
                }
            }
        }

        if account_number.is_empty() {
            account_number = "UNKNOWN".to_string();
        }

        if account_name.is_empty() {
            account_name = "Неизвестно".to_string();
        }

        Ok((account_number, account_name))
    }

    fn parse_transactions(lines: &[&str]) -> Result<Vec<CsvTransaction>> {
        let mut transactions = Vec::new();
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i];

            if line.trim().is_empty()
                || line.contains("Количество операций")
                || line.contains("Входящий остаток")
                || line.contains("Исходящий остаток")
                || line.contains("Итого оборотов")
            {
                i += 1;
                continue;
            }

            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() > 1 {
                let date_str = parts[1].trim().trim_matches('"');
                if Self::is_valid_date(date_str) {
                    let mut full_record = line.to_string();
                    let mut j = i + 1;

                    while j < lines.len() && Self::has_unclosed_quotes(&full_record) {
                        full_record.push('\n');
                        full_record.push_str(lines[j]);
                        j += 1;
                    }

                    match Self::parse_transaction_record(&full_record) {
                        Ok(tx) => transactions.push(tx),
                        Err(e) => {
                            eprintln!("Предупреждение: не удалось распарсить транзакцию: {}", e);
                        }
                    }

                    i = j;
                    continue;
                }
            }

            i += 1;
        }

        Ok(transactions)
    }

    fn is_valid_date(s: &str) -> bool {
        if s.len() != 10 {
            return false;
        }

        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 3 {
            return false;
        }

        parts[0].len() == 2
            && parts[1].len() == 2
            && parts[2].len() == 4
            && parts.iter().all(|p| p.chars().all(|c| c.is_ascii_digit()))
    }

    fn has_unclosed_quotes(s: &str) -> bool {
        !s.matches('"').count().is_multiple_of(2)
    }

    fn parse_transaction_record(record: &str) -> Result<CsvTransaction> {
        let fields = Self::parse_csv_fields(record);

        if fields.len() < 20 {
            return Err(Error::Parse(format!(
                "Недостаточно полей в записи: {} (ожидается >= 20)",
                fields.len()
            )));
        }

        let date = Self::parse_date(&fields[1])?;

        let debit_account = if !fields[4].is_empty() {
            Some(fields[4].lines().next().unwrap_or("").to_string())
        } else {
            None
        };

        let credit_account = if fields.len() > 8 && !fields[8].is_empty() {
            Some(fields[8].lines().next().unwrap_or("").to_string())
        } else {
            None
        };

        let debit_amount = Self::parse_amount_field(&fields[9]);

        let credit_amount = if fields.len() > 13 {
            Self::parse_amount_field(&fields[13])
        } else {
            None
        };

        let document_number = if fields.len() > 14 {
            fields[14].clone()
        } else {
            String::new()
        };

        let bank_info = if fields.len() > 17 {
            fields[17].clone()
        } else {
            String::new()
        };

        let description = if fields.len() > 20 {
            fields[20].clone()
        } else {
            String::new()
        };

        Ok(CsvTransaction {
            date,
            debit_account,
            credit_account,
            debit_amount,
            credit_amount,
            document_number,
            bank_info,
            description,
        })
    }

    fn parse_csv_fields(record: &str) -> Vec<String> {
        let mut fields = Vec::new();
        let mut current_field = String::new();
        let mut in_quotes = false;
        let mut chars = record.chars().peekable();

        while let Some(c) = chars.next() {
            match c {
                '"' => {
                    if in_quotes && chars.peek() == Some(&'"') {
                        current_field.push('"');
                        chars.next();
                    } else {
                        in_quotes = !in_quotes;
                    }
                }
                ',' if !in_quotes => {
                    fields.push(current_field.trim().to_string());
                    current_field = String::new();
                }
                _ => {
                    current_field.push(c);
                }
            }
        }

        fields.push(current_field.trim().to_string());
        fields
    }

    fn parse_date(date_str: &str) -> Result<Date> {
        let date_str = date_str.trim();

        let parts: Vec<&str> = date_str.split('.').collect();
        if parts.len() != 3 {
            return Err(Error::Parse(format!("Некорректный формат даты: {}", date_str)));
        }

        let day: u8 = parts[0]
            .parse()
            .map_err(|_| Error::Parse(format!("Некорректный день: {}", parts[0])))?;

        let month: u8 = parts[1]
            .parse()
            .map_err(|_| Error::Parse(format!("Некорректный месяц: {}", parts[1])))?;

        let year: u16 = parts[2]
            .parse()
            .map_err(|_| Error::Parse(format!("Некорректный год: {}", parts[2])))?;

        Ok(Date::new(year, month, day))
    }

    fn parse_amount_field(s: &str) -> Option<i64> {
        let s = s.trim();
        if s.is_empty() {
            return None;
        }

        let cleaned: String = s
            .chars()
            .filter(|c| c.is_ascii_digit() || *c == '.' || *c == ',')
            .collect();

        let amount_str = cleaned.replace(',', ".");

        amount_str
            .parse::<f64>()
            .ok()
            .map(|a| (a * 100.0).round() as i64)
    }
}

impl From<CsvStatement> for Statement {
    fn from(csv: CsvStatement) -> Self {
        let account = Account {
            iban: None,
            number: csv.account_number,
            currency: csv.currency.clone(),
            name: Some(csv.account_name),
            owner: None,
        };

        let mut balance: i64 = 0;
        let first_date = csv
            .transactions
            .first()
            .map(|t| t.date.clone())
            .unwrap_or_else(|| Date::new(2024, 1, 1));
        let last_date = csv
            .transactions
            .last()
            .map(|t| t.date.clone())
            .unwrap_or_else(|| Date::new(2024, 12, 31));

        let transactions: Vec<Transaction> = csv
            .transactions
            .iter()
            .map(|tx| {
                let (amount, is_credit) = if let Some(credit) = tx.credit_amount {
                    balance += credit;
                    (credit, true)
                } else if let Some(debit) = tx.debit_amount {
                    balance -= debit;
                    (debit, false)
                } else {
                    (0, true)
                };

                let counterparty_account = if is_credit {
                    tx.debit_account.clone()
                } else {
                    tx.credit_account.clone()
                };

                let counterparty = Some(Counterparty {
                    name: None,
                    account: counterparty_account,
                    bank_code: CsvStatement::extract_bik(&tx.bank_info),
                    bank_name: Some(tx.bank_info.clone()),
                });

                Transaction {
                    date: tx.date.clone(),
                    value_date: None,
                    amount: Amount::new(amount, &csv.currency),
                    is_credit,
                    reference: Some(tx.document_number.clone()),
                    description: tx.description.clone(),
                    counterparty,
                }
            })
            .collect();

        let opening_balance = Balance {
            amount: Amount::new(0, &csv.currency),
            date: first_date,
            is_credit: true,
        };

        let closing_balance = Balance {
            amount: Amount::new(balance, &csv.currency),
            date: last_date,
            is_credit: balance >= 0,
        };

        Statement {
            account,
            opening_balance,
            closing_balance,
            transactions,
            statement_number: None,
            reference: None,
        }
    }
}

impl CsvStatement {
    fn extract_bik(bank_info: &str) -> Option<String> {
        if let Some(pos) = bank_info.find("БИК") {
            let start = pos + 4;
            let bik: String = bank_info[start..]
                .chars()
                .take_while(|c| c.is_ascii_digit())
                .collect();
            if bik.len() == 9 {
                return Some(bik);
            }
        }
        None
    }
}

//! Парсер формата MT940.

use crate::error::{Error, Result};
use crate::types::{Account, Amount, Balance, Counterparty, Date, Statement, Transaction};
use std::io::Read;

/// Выписка в формате MT940.
#[derive(Debug, Clone)]
pub struct Mt940Statement {
    /// Референс выписки (поле :20:).
    pub reference: String,
    /// Идентификатор счета (поле :25:).
    pub account_id: String,
    /// Номер выписки (поле :28C:).
    pub statement_number: String,
    /// Начальный баланс (поле :60F: или :60M:).
    pub opening_balance: Mt940Balance,
    /// Конечный баланс (поле :62F: или :62M:).
    pub closing_balance: Mt940Balance,
    /// Список транзакций (поля :61: и :86:).
    pub transactions: Vec<Mt940Transaction>,
}

/// Баланс в формате MT940.
#[derive(Debug, Clone)]
pub struct Mt940Balance {
    /// Индикатор кредит/дебет ('C' или 'D').
    pub credit_debit: char,
    /// Дата баланса.
    pub date: Date,
    /// Код валюты (EUR, USD, RUB и т.д.).
    pub currency: String,
    /// Сумма в минимальных единицах (копейки, центы).
    pub amount: i64,
}

/// Транзакция в формате MT940.
#[derive(Debug, Clone)]
pub struct Mt940Transaction {
    /// Дата проводки.
    pub date: Date,
    /// Дата валютирования.
    pub value_date: Option<Date>,
    /// Индикатор кредит/дебет ('C' или 'D').
    pub credit_debit: char,
    /// Сумма в минимальных единицах.
    pub amount: i64,
    /// Тип транзакции (NTRF, NMSC и т.д.).
    pub transaction_type: String,
    /// Референс транзакции.
    pub reference: Option<String>,
    /// Детали/описание транзакции.
    pub details: String,
}

impl Mt940Statement {
    /// Парсит MT940 из любого источника, реализующего трейт Read.
    pub fn from_read<R: Read>(reader: &mut R) -> Result<Vec<Self>> {
        let mut content = String::new();
        reader.read_to_string(&mut content)?;
        Self::parse(&content)
    }

    /// Парсит MT940 из строки.
    pub fn parse(content: &str) -> Result<Vec<Self>> {
        let mut statements = Vec::new();
        let blocks: Vec<&str> = content.split("{4:").collect();

        for block in blocks.iter().skip(1) {
            let end_pos = block.find("-}").unwrap_or(block.len());
            let block_content = &block[..end_pos];

            match Self::parse_single_statement(block_content) {
                Ok(stmt) => statements.push(stmt),
                Err(e) => {
                    eprintln!("Предупреждение: не удалось распарсить блок MT940: {}", e);
                }
            }
        }

        if statements.is_empty() {
            return Err(Error::InvalidFormat(
                "Не найдено ни одной валидной выписки MT940".to_string(),
            ));
        }

        Ok(statements)
    }

    fn parse_single_statement(content: &str) -> Result<Self> {
        let reference = Self::extract_tag(content, ":20:")?;
        let account_id = Self::extract_tag(content, ":25:")?;
        let statement_number = Self::extract_tag(content, ":28C:").unwrap_or_default();

        let opening_balance = Self::parse_balance(content, ":60F:")
            .or_else(|_| Self::parse_balance(content, ":60M:"))?;

        let closing_balance = Self::parse_balance(content, ":62F:")
            .or_else(|_| Self::parse_balance(content, ":62M:"))?;

        let transactions = Self::parse_transactions(content)?;

        Ok(Mt940Statement {
            reference,
            account_id,
            statement_number,
            opening_balance,
            closing_balance,
            transactions,
        })
    }

    fn extract_tag(content: &str, tag: &str) -> Result<String> {
        let start = content
            .find(tag)
            .ok_or_else(|| Error::MissingField(tag.to_string()))?;

        let value_start = start + tag.len();
        let value_end = content[value_start..]
            .find('\n')
            .map(|pos| value_start + pos)
            .unwrap_or(content.len());

        Ok(content[value_start..value_end].trim().to_string())
    }

    fn parse_balance(content: &str, tag: &str) -> Result<Mt940Balance> {
        let value = Self::extract_tag(content, tag)?;

        if value.len() < 10 {
            return Err(Error::Parse(format!("Некорректный формат баланса: {}", value)));
        }

        let credit_debit = value.chars().next().ok_or_else(|| {
            Error::Parse("Отсутствует индикатор кредит/дебет".to_string())
        })?;

        let date_str = &value[1..7];
        let date = Self::parse_date(date_str)?;
        let currency = value[7..10].to_string();
        let amount_str = value[10..].replace(',', ".");
        let amount = Self::parse_amount(&amount_str)?;

        Ok(Mt940Balance {
            credit_debit,
            date,
            currency,
            amount,
        })
    }

    fn parse_date(date_str: &str) -> Result<Date> {
        if date_str.len() != 6 {
            return Err(Error::Parse(format!("Некорректный формат даты: {}", date_str)));
        }

        let year: u16 = date_str[0..2]
            .parse()
            .map_err(|_| Error::Parse(format!("Некорректный год: {}", &date_str[0..2])))?;

        let year = if year > 50 { 1900 + year } else { 2000 + year };

        let month: u8 = date_str[2..4]
            .parse()
            .map_err(|_| Error::Parse(format!("Некорректный месяц: {}", &date_str[2..4])))?;

        let day: u8 = date_str[4..6]
            .parse()
            .map_err(|_| Error::Parse(format!("Некорректный день: {}", &date_str[4..6])))?;

        Ok(Date::new(year, month, day))
    }

    fn parse_amount(amount_str: &str) -> Result<i64> {
        let amount: f64 = amount_str
            .trim()
            .parse()
            .map_err(|_| Error::Parse(format!("Некорректная сумма: {}", amount_str)))?;

        Ok((amount * 100.0).round() as i64)
    }

    fn parse_transactions(content: &str) -> Result<Vec<Mt940Transaction>> {
        let mut transactions = Vec::new();
        let mut current_pos = 0;

        while let Some(tag_pos) = content[current_pos..].find(":61:") {
            let abs_pos = current_pos + tag_pos;
            let value_start = abs_pos + 4;

            let line_end = content[value_start..]
                .find('\n')
                .map(|pos| value_start + pos)
                .unwrap_or(content.len());

            let transaction_line = &content[value_start..line_end];

            let details_start = content[line_end..].find(":86:");
            let details = if let Some(pos) = details_start {
                let details_value_start = line_end + pos + 4;
                let details_end = content[details_value_start..]
                    .find("\n:")
                    .map(|p| details_value_start + p)
                    .unwrap_or_else(|| {
                        content[details_value_start..]
                            .find("-}")
                            .map(|p| details_value_start + p)
                            .unwrap_or(content.len())
                    });

                content[details_value_start..details_end]
                    .trim()
                    .replace('\n', " ")
            } else {
                String::new()
            };

            match Self::parse_transaction_line(transaction_line, &details) {
                Ok(tx) => transactions.push(tx),
                Err(e) => {
                    eprintln!("Предупреждение: не удалось распарсить транзакцию: {}", e);
                }
            }

            current_pos = line_end + 1;
        }

        Ok(transactions)
    }

    fn parse_transaction_line(line: &str, details: &str) -> Result<Mt940Transaction> {
        let line = line.trim();

        if line.len() < 16 {
            return Err(Error::Parse(format!(
                "Строка транзакции слишком короткая: {}",
                line
            )));
        }

        let value_date = Self::parse_date(&line[0..6])?;

        let (entry_date, cd_pos) = if line.chars().nth(6).map(|c| c.is_ascii_digit()).unwrap_or(false) {
            let month: u8 = line[6..8].parse().unwrap_or(value_date.month);
            let day: u8 = line[8..10].parse().unwrap_or(value_date.day);
            (Some(Date::new(value_date.year, month, day)), 10)
        } else {
            (None, 6)
        };

        let credit_debit = line.chars().nth(cd_pos).ok_or_else(|| {
            Error::Parse("Отсутствует индикатор кредит/дебет в транзакции".to_string())
        })?;

        let amount_start = if line.chars().nth(cd_pos + 1) == Some('R') {
            cd_pos + 2
        } else {
            cd_pos + 1
        };

        let amount_end = line[amount_start..]
            .find(|c: char| c.is_ascii_alphabetic())
            .map(|pos| amount_start + pos)
            .unwrap_or(line.len());

        let amount_str = line[amount_start..amount_end].replace(',', ".");
        let amount = Self::parse_amount(&amount_str)?;

        let type_start = amount_end;
        let type_end = (type_start + 4).min(line.len());
        let transaction_type = line[type_start..type_end].to_string();

        let reference = line.find("//").map(|pos| line[pos + 2..].to_string());

        Ok(Mt940Transaction {
            date: entry_date.unwrap_or_else(|| value_date.clone()),
            value_date: Some(value_date),
            credit_debit,
            amount,
            transaction_type,
            reference,
            details: details.to_string(),
        })
    }
}

impl From<Mt940Statement> for Statement {
    fn from(mt940: Mt940Statement) -> Self {
        let account = Account {
            iban: if mt940.account_id.starts_with("NL")
                || mt940.account_id.starts_with("DE")
                || mt940.account_id.starts_with("DK")
            {
                Some(mt940.account_id.clone())
            } else {
                None
            },
            number: mt940.account_id,
            currency: mt940.opening_balance.currency.clone(),
            name: None,
            owner: None,
        };

        let opening_balance = Balance {
            amount: Amount::new(
                if mt940.opening_balance.credit_debit == 'D' {
                    -mt940.opening_balance.amount
                } else {
                    mt940.opening_balance.amount
                },
                &mt940.opening_balance.currency,
            ),
            date: mt940.opening_balance.date,
            is_credit: mt940.opening_balance.credit_debit == 'C',
        };

        let closing_balance = Balance {
            amount: Amount::new(
                if mt940.closing_balance.credit_debit == 'D' {
                    -mt940.closing_balance.amount
                } else {
                    mt940.closing_balance.amount
                },
                &mt940.closing_balance.currency,
            ),
            date: mt940.closing_balance.date,
            is_credit: mt940.closing_balance.credit_debit == 'C',
        };

        let transactions = mt940
            .transactions
            .into_iter()
            .map(|tx| {
                let counterparty = if !tx.details.is_empty() {
                    Some(Counterparty {
                        name: Some(tx.details.clone()),
                        account: tx.reference.clone(),
                        bank_code: None,
                        bank_name: None,
                    })
                } else {
                    None
                };

                Transaction {
                    date: tx.date,
                    value_date: tx.value_date,
                    amount: Amount::new(tx.amount, &mt940.opening_balance.currency),
                    is_credit: tx.credit_debit == 'C',
                    reference: tx.reference,
                    description: tx.details,
                    counterparty,
                }
            })
            .collect();

        Statement {
            account,
            opening_balance,
            closing_balance,
            transactions,
            statement_number: Some(mt940.statement_number),
            reference: Some(mt940.reference),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_date() {
        let date = Mt940Statement::parse_date("200101").unwrap();
        assert_eq!(date.year, 2020);
        assert_eq!(date.month, 1);
        assert_eq!(date.day, 1);
    }

    #[test]
    fn test_parse_amount() {
        let amount = Mt940Statement::parse_amount("444.29").unwrap();
        assert_eq!(amount, 44429);
    }
}

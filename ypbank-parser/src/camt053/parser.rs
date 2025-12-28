//! Парсер формата CAMT.053 (ISO 20022 XML).

use crate::error::{Error, Result};
use crate::types::{
    Account, Amount, Balance, BalanceType, Counterparty, CreditDebit, Date, Statement, Transaction,
    CREDIT_INDICATOR,
};
use std::io::Read;

/// Выписка в формате CAMT.053.
#[derive(Debug, Clone)]
pub struct Camt053Statement {
    /// Идентификатор сообщения (MsgId).
    pub message_id: String,
    /// Дата и время создания (CreDtTm).
    pub creation_date_time: String,
    /// Идентификатор выписки (Id в Stmt).
    pub statement_id: String,
    /// Информация о счете.
    pub account: Camt053Account,
    /// Список балансов (начальный, конечный и др.).
    pub balances: Vec<Camt053Balance>,
    /// Список записей (транзакций).
    pub entries: Vec<Camt053Entry>,
}

/// Счет в формате CAMT.053.
#[derive(Debug, Clone)]
pub struct Camt053Account {
    /// IBAN счета.
    pub iban: Option<String>,
    /// Код валюты (EUR, USD, RUB и т.д.).
    pub currency: String,
    /// Название счета.
    pub name: Option<String>,
    /// Имя владельца счета.
    pub owner_name: Option<String>,
}

/// Баланс в формате CAMT.053.
#[derive(Debug, Clone)]
pub struct Camt053Balance {
    /// Тип баланса (начальный, конечный и др.).
    pub balance_type: BalanceType,
    /// Сумма в минимальных единицах.
    pub amount: i64,
    /// Код валюты.
    pub currency: String,
    /// Индикатор кредит/дебет.
    pub credit_debit: CreditDebit,
    /// Дата баланса.
    pub date: Date,
}

/// Запись (транзакция) в формате CAMT.053.
#[derive(Debug, Clone)]
pub struct Camt053Entry {
    /// Референс записи (NtryRef).
    pub entry_ref: Option<String>,
    /// Сумма в минимальных единицах.
    pub amount: i64,
    /// Код валюты.
    pub currency: String,
    /// Индикатор кредит/дебет.
    pub credit_debit: CreditDebit,
    /// Дата проводки.
    pub booking_date: Date,
    /// Дата валютирования.
    pub value_date: Option<Date>,
    /// Референс от банка (AcctSvcrRef).
    pub account_servicer_ref: Option<String>,
    /// Детали транзакций.
    pub transaction_details: Vec<Camt053TransactionDetails>,
}

/// Детали транзакции.
#[derive(Debug, Clone)]
pub struct Camt053TransactionDetails {
    /// End-to-end идентификатор.
    pub end_to_end_id: Option<String>,
    /// Идентификатор транзакции.
    pub transaction_id: Option<String>,
    /// Сумма в минимальных единицах.
    pub amount: Option<i64>,
    /// Код валюты.
    pub currency: Option<String>,
    /// Имя плательщика.
    pub debtor_name: Option<String>,
    /// Счет плательщика.
    pub debtor_account: Option<String>,
    /// Имя получателя.
    pub creditor_name: Option<String>,
    /// Счет получателя.
    pub creditor_account: Option<String>,
    /// Информация о назначении платежа.
    pub remittance_info: Vec<String>,
}

impl Camt053Statement {
    /// Парсит CAMT.053 из любого источника, реализующего трейт Read.
    pub fn from_read<R: Read>(reader: &mut R) -> Result<Self> {
        let mut content = String::new();
        reader.read_to_string(&mut content)?;
        Self::parse(&content)
    }

    /// Парсит CAMT.053 из строки.
    pub fn parse(content: &str) -> Result<Self> {
        let content = content.trim();

        if !content.contains("<BkToCstmrStmt>") {
            return Err(Error::InvalidFormat(
                "Не найден элемент BkToCstmrStmt".to_string(),
            ));
        }

        let message_id = Self::extract_element_value(content, "MsgId").ok_or_else(|| {
            Error::MissingField("Не найден обязательный элемент MsgId".to_string())
        })?;
        let creation_date_time = Self::extract_element_value(content, "CreDtTm").ok_or_else(|| {
            Error::MissingField("Не найден обязательный элемент CreDtTm".to_string())
        })?;

        let stmt_start = content.find("<Stmt>").ok_or_else(|| {
            Error::InvalidFormat("Не найден элемент Stmt".to_string())
        })?;
        let stmt_end = content.find("</Stmt>").ok_or_else(|| {
            Error::InvalidFormat("Не найден закрывающий тег Stmt".to_string())
        })?;
        let stmt_content = &content[stmt_start..stmt_end + 7];

        let statement_id = Self::extract_element_value(stmt_content, "Id").ok_or_else(|| {
            Error::MissingField("Не найден обязательный элемент Id в Stmt".to_string())
        })?;
        let account = Self::parse_account(stmt_content)?;
        let balances = Self::parse_balances(stmt_content)?;
        let entries = Self::parse_entries(stmt_content)?;

        Ok(Camt053Statement {
            message_id,
            creation_date_time,
            statement_id,
            account,
            balances,
            entries,
        })
    }

    fn extract_element_value(content: &str, tag: &str) -> Option<String> {
        let open_tag = format!("<{}>", tag);
        let close_tag = format!("</{}>", tag);

        let start = content.find(&open_tag)?;
        let value_start = start + open_tag.len();
        let end = content[value_start..].find(&close_tag)?;

        Some(content[value_start..value_start + end].trim().to_string())
    }

    fn parse_account(content: &str) -> Result<Camt053Account> {
        let acct_start = content.find("<Acct>").unwrap_or(0);
        let acct_end = content.find("</Acct>").unwrap_or(content.len());
        let acct_content = &content[acct_start..acct_end];

        let iban = Self::extract_element_value(acct_content, "IBAN");
        let currency = Self::extract_element_value(acct_content, "Ccy").unwrap_or_else(|| "EUR".to_string());
        let name = Self::extract_element_value(acct_content, "Nm");

        let owner_name = if let Some(ownr_start) = acct_content.find("<Ownr>") {
            let ownr_end = acct_content.find("</Ownr>").unwrap_or(acct_content.len());
            Self::extract_element_value(&acct_content[ownr_start..ownr_end], "Nm")
        } else {
            None
        };

        Ok(Camt053Account {
            iban,
            currency,
            name,
            owner_name,
        })
    }

    fn parse_balances(content: &str) -> Result<Vec<Camt053Balance>> {
        let mut balances = Vec::new();
        let mut pos = 0;

        while let Some(bal_start) = content[pos..].find("<Bal>") {
            let abs_start = pos + bal_start;
            let bal_end = content[abs_start..].find("</Bal>").unwrap_or(content.len() - abs_start);
            let bal_content = &content[abs_start..abs_start + bal_end + 6];

            match Self::parse_single_balance(bal_content) {
                Ok(balance) => balances.push(balance),
                Err(e) => {
                    tracing::warn!("Не удалось распарсить баланс: {}", e);
                }
            }

            pos = abs_start + bal_end + 6;
        }

        Ok(balances)
    }

    fn parse_single_balance(content: &str) -> Result<Camt053Balance> {
        let balance_type_str = Self::extract_element_value(content, "Cd").unwrap_or_default();
        let balance_type = BalanceType::from_code(&balance_type_str);
        let (amount, currency) = Self::parse_amount_with_currency(content, "Amt")?;
        let credit_debit_str =
            Self::extract_element_value(content, "CdtDbtInd").unwrap_or_else(|| CREDIT_INDICATOR.to_string());
        let credit_debit = CreditDebit::from_code(&credit_debit_str);
        let date = Self::parse_date_element(content)?;

        Ok(Camt053Balance {
            balance_type,
            amount,
            currency,
            credit_debit,
            date,
        })
    }

    fn parse_amount_with_currency(content: &str, tag: &str) -> Result<(i64, String)> {
        let open_tag = format!("<{}", tag);

        let start = content.find(&open_tag).ok_or_else(|| {
            Error::MissingField(format!("Не найден элемент {}", tag))
        })?;

        let tag_end = content[start..].find('>').ok_or_else(|| {
            Error::Parse("Некорректный XML".to_string())
        })?;

        let tag_content = &content[start..start + tag_end];
        let currency = if let Some(ccy_pos) = tag_content.find("Ccy=\"") {
            let ccy_start = ccy_pos + 5;
            let ccy_end = tag_content[ccy_start..].find('"').unwrap_or(3);
            tag_content[ccy_start..ccy_start + ccy_end].to_string()
        } else {
            "EUR".to_string()
        };

        let value_start = start + tag_end + 1;
        let close_tag = format!("</{}>", tag);
        let value_end = content[value_start..].find(&close_tag).ok_or_else(|| {
            Error::Parse(format!("Не найден закрывающий тег {}", tag))
        })?;

        let amount_str = content[value_start..value_start + value_end].trim();
        let amount = Self::parse_decimal_amount(amount_str)?;

        Ok((amount, currency))
    }

    /// Парсит сумму из строки без использования f64.
    /// Поддерживает форматы: "123.45", "123,45", "123"
    fn parse_decimal_amount(amount_str: &str) -> Result<i64> {
        let amount_str = amount_str.trim();

        if amount_str.is_empty() {
            return Err(Error::Parse("Пустая сумма".to_string()));
        }

        let is_negative = amount_str.starts_with('-');
        let amount_str = amount_str.trim_start_matches('-');

        let normalized = amount_str.replace(',', ".");

        let (whole_str, frac_str) = if let Some(dot_pos) = normalized.find('.') {
            (&normalized[..dot_pos], &normalized[dot_pos + 1..])
        } else {
            (normalized.as_str(), "")
        };

        let whole: i64 = if whole_str.is_empty() {
            0
        } else {
            whole_str.parse().map_err(|_| {
                Error::Parse(format!("Некорректная целая часть суммы: {}", whole_str))
            })?
        };

        let frac: i64 = if frac_str.is_empty() {
            0
        } else {
            let frac_padded = match frac_str.len() {
                0 => "00".to_string(),
                1 => format!("{}0", frac_str),
                2 => frac_str.to_string(),
                _ => frac_str[..2].to_string(),
            };
            frac_padded.parse().map_err(|_| {
                Error::Parse(format!("Некорректная дробная часть суммы: {}", frac_str))
            })?
        };

        let amount = whole
            .checked_mul(100)
            .and_then(|w| w.checked_add(frac))
            .ok_or_else(|| Error::Parse("Переполнение при парсинге суммы".to_string()))?;

        Ok(if is_negative { -amount } else { amount })
    }

    fn parse_date_element(content: &str) -> Result<Date> {
        // Ищем самый внутренний <Dt> элемент, который содержит только дату
        let mut date_str = Self::extract_element_value(content, "Dt").ok_or_else(|| {
            Error::MissingField("Не найден элемент Dt".to_string())
        })?;

        // Если внутри есть вложенный <Dt>, извлекаем его рекурсивно
        while date_str.contains("<Dt>") {
            if let Some(inner) = Self::extract_element_value(&date_str, "Dt") {
                date_str = inner;
            } else {
                // Если не можем извлечь вложенный тег, извлекаем дату напрямую
                // из содержимого после тега
                if let Some(start) = date_str.find("<Dt>") {
                    let value_start = start + 4;
                    if let Some(end) = date_str[value_start..].find('<') {
                        date_str = date_str[value_start..value_start + end].trim().to_string();
                    } else {
                        date_str = date_str[value_start..].trim().to_string();
                    }
                }
                break;
            }
        }

        Self::parse_iso_date(&date_str)
    }

    fn parse_iso_date(date_str: &str) -> Result<Date> {
        let date_str = date_str.trim();

        let parts: Vec<&str> = date_str.split('-').collect();
        if parts.len() != 3 {
            return Err(Error::Parse(format!("Некорректный формат даты: {}", date_str)));
        }

        let year: u16 = parts[0].parse().map_err(|_| {
            Error::Parse(format!("Некорректный год: {}", parts[0]))
        })?;

        let month: u8 = parts[1].parse().map_err(|_| {
            Error::Parse(format!("Некорректный месяц: {}", parts[1]))
        })?;

        let day: u8 = parts[2].parse().map_err(|_| {
            Error::Parse(format!("Некорректный день: {}", parts[2]))
        })?;

        Ok(Date::new(year, month, day))
    }

    fn parse_entries(content: &str) -> Result<Vec<Camt053Entry>> {
        let mut entries = Vec::new();
        let mut pos = 0;

        while let Some(ntry_start) = content[pos..].find("<Ntry>") {
            let abs_start = pos + ntry_start;
            let ntry_end = content[abs_start..].find("</Ntry>").unwrap_or(content.len() - abs_start);
            let ntry_content = &content[abs_start..abs_start + ntry_end + 7];

            match Self::parse_single_entry(ntry_content) {
                Ok(entry) => entries.push(entry),
                Err(e) => {
                    tracing::warn!("Не удалось распарсить запись: {}", e);
                }
            }

            pos = abs_start + ntry_end + 7;
        }

        Ok(entries)
    }

    fn parse_single_entry(content: &str) -> Result<Camt053Entry> {
        let entry_ref = Self::extract_element_value(content, "NtryRef");
        let (amount, currency) = Self::parse_amount_with_currency(content, "Amt")?;
        let credit_debit_str =
            Self::extract_element_value(content, "CdtDbtInd").unwrap_or_else(|| CREDIT_INDICATOR.to_string());
        let credit_debit = CreditDebit::from_code(&credit_debit_str);

        let booking_date = if let Some(bookg_start) = content.find("<BookgDt>") {
            let bookg_end = content.find("</BookgDt>").unwrap_or(content.len());
            Self::parse_date_element(&content[bookg_start..bookg_end])?
        } else {
            Date::new(2024, 1, 1)
        };

        let value_date = if let Some(val_start) = content.find("<ValDt>") {
            let val_end = content.find("</ValDt>").unwrap_or(content.len());
            Self::parse_date_element(&content[val_start..val_end]).ok()
        } else {
            None
        };

        let account_servicer_ref = Self::extract_element_value(content, "AcctSvcrRef");
        let transaction_details = Self::parse_transaction_details(content)?;

        Ok(Camt053Entry {
            entry_ref,
            amount,
            currency,
            credit_debit,
            booking_date,
            value_date,
            account_servicer_ref,
            transaction_details,
        })
    }

    fn parse_transaction_details(content: &str) -> Result<Vec<Camt053TransactionDetails>> {
        let mut details = Vec::new();
        let mut pos = 0;

        while let Some(tx_start) = content[pos..].find("<TxDtls>") {
            let abs_start = pos + tx_start;
            let tx_end = content[abs_start..].find("</TxDtls>").unwrap_or(content.len() - abs_start);
            let tx_content = &content[abs_start..abs_start + tx_end + 9];

            let end_to_end_id = Self::extract_element_value(tx_content, "EndToEndId");
            let transaction_id = Self::extract_element_value(tx_content, "TxId");

            let (amount, currency) = Self::parse_amount_with_currency(tx_content, "Amt")
                .map(|(a, c)| (Some(a), Some(c)))
                .unwrap_or((None, None));

            let (debtor_name, debtor_account) = Self::parse_party_info(tx_content, "Dbtr");
            let (creditor_name, creditor_account) = Self::parse_party_info(tx_content, "Cdtr");
            let remittance_info = Self::parse_remittance_info(tx_content);

            details.push(Camt053TransactionDetails {
                end_to_end_id,
                transaction_id,
                amount,
                currency,
                debtor_name,
                debtor_account,
                creditor_name,
                creditor_account,
                remittance_info,
            });

            pos = abs_start + tx_end + 9;
        }

        Ok(details)
    }

    fn parse_party_info(content: &str, party_tag: &str) -> (Option<String>, Option<String>) {
        let open_tag = format!("<{}>", party_tag);
        let close_tag = format!("</{}>", party_tag);

        if let Some(start) = content.find(&open_tag) {
            let end = content.find(&close_tag).unwrap_or(content.len());
            let party_content = &content[start..end];

            let name = Self::extract_element_value(party_content, "Nm");

            let account = Self::extract_element_value(party_content, "IBAN")
                .or_else(|| {
                    if let Some(id_start) = party_content.find("<Id>") {
                        let id_end = party_content.find("</Id>").unwrap_or(party_content.len());
                        Self::extract_element_value(&party_content[id_start..id_end], "Id")
                    } else {
                        None
                    }
                });

            (name, account)
        } else {
            (None, None)
        }
    }

    fn parse_remittance_info(content: &str) -> Vec<String> {
        let mut info = Vec::new();
        let mut pos = 0;

        while let Some(start) = content[pos..].find("<Ustrd>") {
            let abs_start = pos + start;
            if let Some(end) = content[abs_start..].find("</Ustrd>") {
                let value = &content[abs_start + 7..abs_start + end];
                info.push(value.trim().to_string());
                pos = abs_start + end + 8;
            } else {
                break;
            }
        }

        info
    }
}

impl From<Camt053Statement> for Statement {
    fn from(camt: Camt053Statement) -> Self {
        let account = Account {
            iban: camt.account.iban.clone(),
            number: camt.account.iban.clone().unwrap_or_else(|| "UNKNOWN".to_string()),
            currency: camt.account.currency.clone(),
            name: camt.account.name.clone(),
            owner: camt.account.owner_name.clone(),
        };

        let opening_balance = camt
            .balances
            .iter()
            .find(|b| b.balance_type == BalanceType::Opening)
            .map(|b| Balance {
                amount: Amount::new(
                    if b.credit_debit == CreditDebit::Debit {
                        -b.amount
                    } else {
                        b.amount
                    },
                    &b.currency,
                ),
                date: b.date.clone(),
                is_credit: b.credit_debit.is_credit(),
            })
            .unwrap_or_else(|| Balance {
                amount: Amount::new(0, &camt.account.currency),
                date: Date::new(2024, 1, 1),
                is_credit: true,
            });

        let closing_balance = camt
            .balances
            .iter()
            .find(|b| b.balance_type == BalanceType::Closing)
            .map(|b| Balance {
                amount: Amount::new(
                    if b.credit_debit == CreditDebit::Debit {
                        -b.amount
                    } else {
                        b.amount
                    },
                    &b.currency,
                ),
                date: b.date.clone(),
                is_credit: b.credit_debit.is_credit(),
            })
            .unwrap_or_else(|| Balance {
                amount: Amount::new(0, &camt.account.currency),
                date: Date::new(2024, 12, 31),
                is_credit: true,
            });

        let transactions = camt
            .entries
            .into_iter()
            .map(|entry| {
                let is_credit = entry.credit_debit.is_credit();

                let (counterparty, description) = if let Some(details) = entry.transaction_details.first() {
                    let counterparty = if is_credit {
                        Counterparty {
                            name: details.debtor_name.clone(),
                            account: details.debtor_account.clone(),
                            bank_code: None,
                            bank_name: None,
                        }
                    } else {
                        Counterparty {
                            name: details.creditor_name.clone(),
                            account: details.creditor_account.clone(),
                            bank_code: None,
                            bank_name: None,
                        }
                    };

                    let description = details.remittance_info.join(" ");

                    (Some(counterparty), description)
                } else {
                    (None, String::new())
                };

                Transaction {
                    date: entry.booking_date,
                    value_date: entry.value_date,
                    amount: Amount::new(entry.amount, &entry.currency),
                    is_credit,
                    reference: entry.account_servicer_ref,
                    description,
                    counterparty,
                }
            })
            .collect();

        Statement {
            account,
            opening_balance,
            closing_balance,
            transactions,
            statement_number: Some(camt.statement_id),
            reference: Some(camt.message_id),
        }
    }
}

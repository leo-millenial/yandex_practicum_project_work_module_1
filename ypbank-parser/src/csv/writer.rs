//! Сериализация формата CSV.

use crate::csv::parser::{CsvStatement, CsvTransaction};
use crate::error::Result;
use std::io::Write;

/// Writer для формата CSV.
pub struct CsvWriter;

impl CsvWriter {
    /// Записывает выписку CSV в любой приемник, реализующий трейт Write.
    pub fn write_to<W: Write>(statement: &CsvStatement, writer: &mut W) -> Result<()> {
        writeln!(
            writer,
            "Дата,Счет дебета,Счет кредита,Сумма дебета,Сумма кредита,№ документа,Банк,Назначение платежа"
        )?;

        for tx in &statement.transactions {
            Self::write_transaction(writer, tx)?;
        }

        Ok(())
    }

    fn write_transaction<W: Write>(writer: &mut W, tx: &CsvTransaction) -> Result<()> {
        let date_str = format!("{:02}.{:02}.{}", tx.date.day, tx.date.month, tx.date.year);

        let debit_account = tx.debit_account.as_deref().unwrap_or("");
        let credit_account = tx.credit_account.as_deref().unwrap_or("");

        let debit_amount = tx
            .debit_amount
            .map(Self::format_amount)
            .unwrap_or_default();

        let credit_amount = tx
            .credit_amount
            .map(Self::format_amount)
            .unwrap_or_default();

        let description = Self::escape_csv_field(&tx.description);
        let bank_info = Self::escape_csv_field(&tx.bank_info);

        writeln!(
            writer,
            "{},{},{},{},{},{},{},{}",
            date_str,
            debit_account,
            credit_account,
            debit_amount,
            credit_amount,
            tx.document_number,
            bank_info,
            description
        )?;

        Ok(())
    }

    fn format_amount(amount: i64) -> String {
        let whole = amount / 100;
        let frac = (amount % 100).abs();
        format!("{}.{:02}", whole, frac)
    }

    fn escape_csv_field(s: &str) -> String {
        if s.contains(',') || s.contains('"') || s.contains('\n') {
            format!("\"{}\"", s.replace('"', "\"\""))
        } else {
            s.to_string()
        }
    }
}

impl CsvStatement {
    /// Записывает выписку в любой приемник, реализующий трейт Write.
    pub fn write_to<W: Write>(&self, writer: &mut W) -> Result<()> {
        CsvWriter::write_to(self, writer)
    }
}

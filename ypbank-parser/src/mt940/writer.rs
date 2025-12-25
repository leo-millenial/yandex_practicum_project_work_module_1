//! Сериализация формата MT940.

use crate::error::Result;
use crate::mt940::parser::{Mt940Balance, Mt940Statement, Mt940Transaction};
use std::io::Write;

/// Writer для формата MT940.
pub struct Mt940Writer;

impl Mt940Writer {
    /// Записывает выписку MT940 в любой приемник, реализующий трейт Write.
    pub fn write_to<W: Write>(statement: &Mt940Statement, writer: &mut W) -> Result<()> {
        writeln!(writer, "{{1:F01BANKXXXX0000000000}}")?;
        writeln!(writer, "{{2:O940BANKXXXXN}}")?;
        writeln!(writer, "{{3:}}")?;
        writeln!(writer, "{{4:")?;
        writeln!(writer, ":20:{}", statement.reference)?;
        writeln!(writer, ":25:{}", statement.account_id)?;
        writeln!(writer, ":28C:{}", statement.statement_number)?;

        Self::write_balance(writer, ":60F:", &statement.opening_balance)?;

        for transaction in &statement.transactions {
            Self::write_transaction(writer, transaction)?;
        }

        Self::write_balance(writer, ":62F:", &statement.closing_balance)?;

        writeln!(writer, "-}}")?;
        writeln!(writer, "{{5:}}")?;

        Ok(())
    }

    fn write_balance<W: Write>(writer: &mut W, tag: &str, balance: &Mt940Balance) -> Result<()> {
        let date_str = format!(
            "{:02}{:02}{:02}",
            balance.date.year % 100,
            balance.date.month,
            balance.date.day
        );

        let amount_str = Self::format_amount(balance.amount);

        writeln!(
            writer,
            "{}{}{}{}{}",
            tag, balance.credit_debit, date_str, balance.currency, amount_str
        )?;

        Ok(())
    }

    fn write_transaction<W: Write>(writer: &mut W, transaction: &Mt940Transaction) -> Result<()> {
        let value_date_str = format!(
            "{:02}{:02}{:02}",
            transaction.date.year % 100,
            transaction.date.month,
            transaction.date.day
        );

        let entry_date_str = transaction
            .value_date
            .as_ref()
            .map(|d| format!("{:02}{:02}", d.month, d.day))
            .unwrap_or_default();

        let amount_str = Self::format_amount(transaction.amount);

        let reference_str = transaction
            .reference
            .as_ref()
            .map(|r| format!("//{}", r))
            .unwrap_or_default();

        writeln!(
            writer,
            ":61:{}{}{}{}{}{}",
            value_date_str,
            entry_date_str,
            transaction.credit_debit,
            amount_str,
            transaction.transaction_type,
            reference_str
        )?;

        if !transaction.details.is_empty() {
            let details = &transaction.details;
            let mut pos = 0;
            writeln!(writer, ":86:{}", &details[pos..(pos + 65).min(details.len())])?;
            pos += 65;

            while pos < details.len() {
                writeln!(writer, "{}", &details[pos..(pos + 65).min(details.len())])?;
                pos += 65;
            }
        }

        Ok(())
    }

    fn format_amount(amount: i64) -> String {
        let whole = amount / 100;
        let frac = (amount % 100).abs();
        format!("{},{:02}", whole, frac)
    }
}

impl Mt940Statement {
    /// Записывает выписку в любой приемник, реализующий трейт Write.
    pub fn write_to<W: Write>(&self, writer: &mut W) -> Result<()> {
        Mt940Writer::write_to(self, writer)
    }
}

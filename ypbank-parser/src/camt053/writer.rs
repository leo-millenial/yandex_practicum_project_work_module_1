//! Сериализация формата CAMT.053 (ISO 20022 XML).

use crate::camt053::parser::{Camt053Balance, Camt053Entry, Camt053Statement, Camt053TransactionDetails};
use crate::error::Result;
use std::io::{BufWriter, Write};

/// Writer для формата CAMT.053.
pub struct Camt053Writer;

impl Camt053Writer {
    /// Записывает выписку CAMT.053 в любой приемник, реализующий трейт Write.
    ///
    /// Использует внутреннюю буферизацию для уменьшения количества syscalls.
    pub fn write_to<W: Write>(statement: &Camt053Statement, writer: &mut W) -> Result<()> {
        let mut buf_writer = BufWriter::new(writer);
        Self::write_to_buffered(statement, &mut buf_writer)?;
        buf_writer.flush()?;
        Ok(())
    }

    fn write_to_buffered<W: Write>(statement: &Camt053Statement, writer: &mut W) -> Result<()> {
        writeln!(writer, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>")?;
        writeln!(
            writer,
            "<Document xmlns=\"urn:iso:std:iso:20022:tech:xsd:camt.053.001.02\">"
        )?;
        writeln!(writer, "<BkToCstmrStmt>")?;

        writeln!(writer, "<GrpHdr>")?;
        writeln!(writer, "<MsgId>{}</MsgId>", Self::escape_xml(&statement.message_id))?;
        writeln!(writer, "<CreDtTm>{}</CreDtTm>", Self::escape_xml(&statement.creation_date_time))?;
        writeln!(writer, "</GrpHdr>")?;

        writeln!(writer, "<Stmt>")?;
        writeln!(writer, "<Id>{}</Id>", Self::escape_xml(&statement.statement_id))?;

        Self::write_account(writer, statement)?;

        for balance in &statement.balances {
            Self::write_balance(writer, balance)?;
        }

        for entry in &statement.entries {
            Self::write_entry(writer, entry)?;
        }

        writeln!(writer, "</Stmt>")?;
        writeln!(writer, "</BkToCstmrStmt>")?;
        writeln!(writer, "</Document>")?;

        Ok(())
    }

    fn write_account<W: Write>(writer: &mut W, statement: &Camt053Statement) -> Result<()> {
        writeln!(writer, "<Acct>")?;
        writeln!(writer, "<Id>")?;

        if let Some(ref iban) = statement.account.iban {
            writeln!(writer, "<IBAN>{}</IBAN>", Self::escape_xml(iban))?;
        }

        writeln!(writer, "</Id>")?;
        writeln!(writer, "<Ccy>{}</Ccy>", Self::escape_xml(&statement.account.currency))?;

        if let Some(ref name) = statement.account.name {
            writeln!(writer, "<Nm>{}</Nm>", Self::escape_xml(name))?;
        }

        if let Some(ref owner) = statement.account.owner_name {
            writeln!(writer, "<Ownr>")?;
            writeln!(writer, "<Nm>{}</Nm>", Self::escape_xml(owner))?;
            writeln!(writer, "</Ownr>")?;
        }

        writeln!(writer, "</Acct>")?;

        Ok(())
    }

    fn write_balance<W: Write>(writer: &mut W, balance: &Camt053Balance) -> Result<()> {
        writeln!(writer, "<Bal>")?;
        writeln!(writer, "<Tp>")?;
        writeln!(writer, "<CdOrPrtry>")?;
        writeln!(writer, "<Cd>{}</Cd>", balance.balance_type.as_code())?;
        writeln!(writer, "</CdOrPrtry>")?;
        writeln!(writer, "</Tp>")?;

        writeln!(
            writer,
            "<Amt Ccy=\"{}\">{}</Amt>",
            Self::escape_xml(&balance.currency),
            Self::format_amount(balance.amount)
        )?;

        writeln!(
            writer,
            "<CdtDbtInd>{}</CdtDbtInd>",
            balance.credit_debit.as_code()
        )?;

        writeln!(writer, "<Dt>")?;
        writeln!(writer, "<Dt>{}</Dt>", Self::format_date(&balance.date))?;
        writeln!(writer, "</Dt>")?;

        writeln!(writer, "</Bal>")?;

        Ok(())
    }

    fn write_entry<W: Write>(writer: &mut W, entry: &Camt053Entry) -> Result<()> {
        writeln!(writer, "<Ntry>")?;

        if let Some(ref entry_ref) = entry.entry_ref {
            writeln!(writer, "<NtryRef>{}</NtryRef>", Self::escape_xml(entry_ref))?;
        }

        writeln!(
            writer,
            "<Amt Ccy=\"{}\">{}</Amt>",
            Self::escape_xml(&entry.currency),
            Self::format_amount(entry.amount)
        )?;

        writeln!(
            writer,
            "<CdtDbtInd>{}</CdtDbtInd>",
            entry.credit_debit.as_code()
        )?;

        writeln!(writer, "<Sts>BOOK</Sts>")?;

        writeln!(writer, "<BookgDt>")?;
        writeln!(writer, "<Dt>{}</Dt>", Self::format_date(&entry.booking_date))?;
        writeln!(writer, "</BookgDt>")?;

        if let Some(ref value_date) = entry.value_date {
            writeln!(writer, "<ValDt>")?;
            writeln!(writer, "<Dt>{}</Dt>", Self::format_date(value_date))?;
            writeln!(writer, "</ValDt>")?;
        }

        if let Some(ref acct_ref) = entry.account_servicer_ref {
            writeln!(writer, "<AcctSvcrRef>{}</AcctSvcrRef>", Self::escape_xml(acct_ref))?;
        }

        if !entry.transaction_details.is_empty() {
            writeln!(writer, "<NtryDtls>")?;
            for details in &entry.transaction_details {
                Self::write_transaction_details(writer, details)?;
            }
            writeln!(writer, "</NtryDtls>")?;
        }

        writeln!(writer, "</Ntry>")?;

        Ok(())
    }

    fn write_transaction_details<W: Write>(
        writer: &mut W,
        details: &Camt053TransactionDetails,
    ) -> Result<()> {
        writeln!(writer, "<TxDtls>")?;

        writeln!(writer, "<Refs>")?;
        if let Some(ref e2e_id) = details.end_to_end_id {
            writeln!(writer, "<EndToEndId>{}</EndToEndId>", Self::escape_xml(e2e_id))?;
        }
        if let Some(ref tx_id) = details.transaction_id {
            writeln!(writer, "<TxId>{}</TxId>", Self::escape_xml(tx_id))?;
        }
        writeln!(writer, "</Refs>")?;

        if let (Some(amount), Some(ref currency)) = (details.amount, &details.currency) {
            writeln!(writer, "<AmtDtls>")?;
            writeln!(writer, "<TxAmt>")?;
            writeln!(
                writer,
                "<Amt Ccy=\"{}\">{}</Amt>",
                Self::escape_xml(currency),
                Self::format_amount(amount)
            )?;
            writeln!(writer, "</TxAmt>")?;
            writeln!(writer, "</AmtDtls>")?;
        }

        writeln!(writer, "<RltdPties>")?;

        if details.debtor_name.is_some() || details.debtor_account.is_some() {
            writeln!(writer, "<Dbtr>")?;
            if let Some(ref name) = details.debtor_name {
                writeln!(writer, "<Nm>{}</Nm>", Self::escape_xml(name))?;
            }
            writeln!(writer, "</Dbtr>")?;

            if let Some(ref account) = details.debtor_account {
                writeln!(writer, "<DbtrAcct>")?;
                writeln!(writer, "<Id>")?;
                writeln!(writer, "<IBAN>{}</IBAN>", Self::escape_xml(account))?;
                writeln!(writer, "</Id>")?;
                writeln!(writer, "</DbtrAcct>")?;
            }
        }

        if details.creditor_name.is_some() || details.creditor_account.is_some() {
            writeln!(writer, "<Cdtr>")?;
            if let Some(ref name) = details.creditor_name {
                writeln!(writer, "<Nm>{}</Nm>", Self::escape_xml(name))?;
            }
            writeln!(writer, "</Cdtr>")?;

            if let Some(ref account) = details.creditor_account {
                writeln!(writer, "<CdtrAcct>")?;
                writeln!(writer, "<Id>")?;
                writeln!(writer, "<IBAN>{}</IBAN>", Self::escape_xml(account))?;
                writeln!(writer, "</Id>")?;
                writeln!(writer, "</CdtrAcct>")?;
            }
        }

        writeln!(writer, "</RltdPties>")?;

        if !details.remittance_info.is_empty() {
            writeln!(writer, "<RmtInf>")?;
            for info in &details.remittance_info {
                writeln!(writer, "<Ustrd>{}</Ustrd>", Self::escape_xml(info))?;
            }
            writeln!(writer, "</RmtInf>")?;
        }

        writeln!(writer, "</TxDtls>")?;

        Ok(())
    }

    fn format_amount(amount: i64) -> String {
        let whole = amount / 100;
        let frac = (amount % 100).abs();
        format!("{}.{:02}", whole, frac)
    }

    fn format_date(date: &crate::types::Date) -> String {
        format!("{:04}-{:02}-{:02}", date.year, date.month, date.day)
    }

    fn escape_xml(s: &str) -> String {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&apos;")
    }
}

impl Camt053Statement {
    /// Записывает выписку в любой приемник, реализующий трейт Write.
    pub fn write_to<W: Write>(&self, writer: &mut W) -> Result<()> {
        Camt053Writer::write_to(self, writer)
    }
}

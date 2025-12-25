//! Интеграционные тесты для ypbank-parser.

use std::io::Cursor;
use ypbank_parser::{Camt053Statement, Mt940Statement, Statement};

const SAMPLE_MT940: &str = r#"{1:F01ASNBNL21XXXX0000000000}{2:O940ASNBNL21XXXXN}{3:}{4:
:20:0000000000
:25:NL81ASNB9999999999
:28C:1/1
:60F:C200101EUR444,29
:61:2001010101D65,00NOVBNL47INGB9999999999
hr gjlm paulissen
:86:NL47INGB9999999999 hr gjlm paulissen

Betaling sieraden



:62F:C200101EUR379,29
-}{5:}
"#;

const SAMPLE_CAMT053: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<Document xmlns="urn:iso:std:iso:20022:tech:xsd:camt.053.001.02">
<BkToCstmrStmt>
<GrpHdr>
<MsgId>SAMPLE001</MsgId>
<CreDtTm>2024-01-01T00:00:00</CreDtTm>
</GrpHdr>
<Stmt>
<Id>STMT001</Id>
<Acct>
<Id>
<IBAN>DK8030000001234567</IBAN>
</Id>
<Ccy>DKK</Ccy>
<Nm>Sample Account</Nm>
</Acct>
<Bal>
<Tp>
<CdOrPrtry>
<Cd>OPBD</Cd>
</CdOrPrtry>
</Tp>
<Amt Ccy="DKK">10000.00</Amt>
<CdtDbtInd>CRDT</CdtDbtInd>
<Dt>
<Dt>2024-01-01</Dt>
</Dt>
</Bal>
<Bal>
<Tp>
<CdOrPrtry>
<Cd>CLBD</Cd>
</CdOrPrtry>
</Tp>
<Amt Ccy="DKK">10591.15</Amt>
<CdtDbtInd>CRDT</CdtDbtInd>
<Dt>
<Dt>2024-01-31</Dt>
</Dt>
</Bal>
<Ntry>
<Amt Ccy="DKK">591.15</Amt>
<CdtDbtInd>CRDT</CdtDbtInd>
<BookgDt>
<Dt>2024-01-15</Dt>
</BookgDt>
<NtryDtls>
<TxDtls>
<Refs>
<EndToEndId>E2E001</EndToEndId>
</Refs>
<RmtInf>
<Ustrd>Payment for invoice</Ustrd>
</RmtInf>
</TxDtls>
</NtryDtls>
</Ntry>
</Stmt>
</BkToCstmrStmt>
</Document>
"#;

#[test]
fn test_mt940_parse() {
    let mut cursor = Cursor::new(SAMPLE_MT940);
    let statements = Mt940Statement::from_read(&mut cursor).unwrap();

    assert_eq!(statements.len(), 1);

    let stmt = &statements[0];
    assert_eq!(stmt.reference, "0000000000");
    assert_eq!(stmt.account_id, "NL81ASNB9999999999");
    assert_eq!(stmt.opening_balance.amount, 44429);
    assert_eq!(stmt.opening_balance.currency, "EUR");
    assert_eq!(stmt.closing_balance.amount, 37929);
    assert_eq!(stmt.transactions.len(), 1);
}

#[test]
fn test_mt940_to_statement() {
    let statements = Mt940Statement::parse(SAMPLE_MT940).unwrap();
    let mt940 = statements.into_iter().next().unwrap();
    let statement: Statement = mt940.into();

    assert_eq!(statement.account.number, "NL81ASNB9999999999");
    assert_eq!(statement.opening_balance.amount.value, 44429);
    assert_eq!(statement.closing_balance.amount.value, 37929);
    assert_eq!(statement.transactions.len(), 1);
    assert!(!statement.transactions[0].is_credit);
}

#[test]
fn test_camt053_parse() {
    let mut cursor = Cursor::new(SAMPLE_CAMT053);
    let statement = Camt053Statement::from_read(&mut cursor).unwrap();

    assert_eq!(statement.message_id, "SAMPLE001");
    assert_eq!(statement.statement_id, "STMT001");
    assert_eq!(statement.account.iban, Some("DK8030000001234567".to_string()));
    assert_eq!(statement.account.currency, "DKK");
    let _ = statement.balances.len();
    let _ = statement.entries.len();
}

#[test]
fn test_camt053_to_statement() {
    let camt = Camt053Statement::parse(SAMPLE_CAMT053).unwrap();
    let statement: Statement = camt.into();

    assert_eq!(statement.account.iban, Some("DK8030000001234567".to_string()));
    assert!(!statement.account.number.is_empty());
}

#[test]
fn test_mt940_to_camt053_conversion() {
    let statements = Mt940Statement::parse(SAMPLE_MT940).unwrap();
    let mt940 = statements.into_iter().next().unwrap();

    let camt: Camt053Statement = mt940.into();

    assert!(camt.account.iban.is_some());
    assert_eq!(camt.balances.len(), 2);
    let opening = camt.balances.iter().find(|b| b.balance_type == "OPBD").unwrap();
    assert_eq!(opening.amount, 44429);
}

#[test]
fn test_camt053_to_mt940_conversion() {
    let camt = Camt053Statement::parse(SAMPLE_CAMT053).unwrap();

    let mt940: Mt940Statement = camt.into();

    assert_eq!(mt940.account_id, "DK8030000001234567");
    assert_eq!(mt940.opening_balance.currency, "DKK");
}

#[test]
fn test_mt940_write() {
    let statements = Mt940Statement::parse(SAMPLE_MT940).unwrap();
    let mt940 = statements.into_iter().next().unwrap();

    let mut output = Vec::new();
    mt940.write_to(&mut output).unwrap();

    let output_str = String::from_utf8(output).unwrap();

    assert!(output_str.contains(":20:"));
    assert!(output_str.contains(":25:"));
    assert!(output_str.contains(":60F:"));
    assert!(output_str.contains(":62F:"));
}

#[test]
fn test_camt053_write() {
    let camt = Camt053Statement::parse(SAMPLE_CAMT053).unwrap();

    let mut output = Vec::new();
    camt.write_to(&mut output).unwrap();

    let output_str = String::from_utf8(output).unwrap();

    assert!(output_str.contains("<Document"));
    assert!(output_str.contains("<BkToCstmrStmt>"));
    assert!(output_str.contains("<IBAN>"));
}

#[test]
fn test_date_parsing() {
    use ypbank_parser::Date;

    let date = Date::new(2024, 1, 15);
    assert_eq!(date.year, 2024);
    assert_eq!(date.month, 1);
    assert_eq!(date.day, 15);
}

#[test]
fn test_amount() {
    use ypbank_parser::Amount;

    let amount = Amount::new(12345, "EUR");
    assert_eq!(amount.value, 12345);
    assert_eq!(amount.currency, "EUR");
    assert!((amount.as_float() - 123.45).abs() < 0.01);
}

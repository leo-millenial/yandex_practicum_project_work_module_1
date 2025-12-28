//! Модуль конвертации между форматами.

use crate::camt053::parser::{
    Camt053Account, Camt053Balance, Camt053Entry, Camt053Statement, Camt053TransactionDetails,
};
use crate::error::Error;
use crate::mt940::parser::{Mt940Balance, Mt940Statement, Mt940Transaction};

impl From<Mt940Statement> for Camt053Statement {
    fn from(mt940: Mt940Statement) -> Self {
        let account = Camt053Account {
            iban: if mt940.account_id.len() > 10 {
                Some(mt940.account_id.clone())
            } else {
                None
            },
            currency: mt940.opening_balance.currency.clone(),
            name: None,
            owner_name: None,
        };

        let opening_balance = Camt053Balance {
            balance_type: "OPBD".to_string(),
            amount: mt940.opening_balance.amount,
            currency: mt940.opening_balance.currency.clone(),
            credit_debit_indicator: if mt940.opening_balance.credit_debit == 'C' {
                "CRDT".to_string()
            } else {
                "DBIT".to_string()
            },
            date: mt940.opening_balance.date.clone(),
        };

        let closing_balance = Camt053Balance {
            balance_type: "CLBD".to_string(),
            amount: mt940.closing_balance.amount,
            currency: mt940.closing_balance.currency.clone(),
            credit_debit_indicator: if mt940.closing_balance.credit_debit == 'C' {
                "CRDT".to_string()
            } else {
                "DBIT".to_string()
            },
            date: mt940.closing_balance.date.clone(),
        };

        let entries: Vec<Camt053Entry> = mt940
            .transactions
            .iter()
            .enumerate()
            .map(|(idx, tx)| {
                let transaction_details = vec![Camt053TransactionDetails {
                    end_to_end_id: Some("NOTPROVIDED".to_string()),
                    transaction_id: tx.reference.clone(),
                    amount: Some(tx.amount),
                    currency: Some(mt940.opening_balance.currency.clone()),
                    debtor_name: if tx.credit_debit == 'C' {
                        Some(tx.details.clone())
                    } else {
                        None
                    },
                    debtor_account: if tx.credit_debit == 'C' {
                        tx.reference.clone()
                    } else {
                        None
                    },
                    creditor_name: if tx.credit_debit == 'D' {
                        Some(tx.details.clone())
                    } else {
                        None
                    },
                    creditor_account: if tx.credit_debit == 'D' {
                        tx.reference.clone()
                    } else {
                        None
                    },
                    remittance_info: if tx.details.is_empty() {
                        vec![]
                    } else {
                        vec![tx.details.clone()]
                    },
                }];

                Camt053Entry {
                    entry_ref: Some(format!("{}", idx + 1)),
                    amount: tx.amount,
                    currency: mt940.opening_balance.currency.clone(),
                    credit_debit_indicator: if tx.credit_debit == 'C' {
                        "CRDT".to_string()
                    } else {
                        "DBIT".to_string()
                    },
                    booking_date: tx.date.clone(),
                    value_date: tx.value_date.clone(),
                    account_servicer_ref: tx.reference.clone(),
                    transaction_details,
                }
            })
            .collect();

        let now = "2024-01-01T00:00:00";

        Camt053Statement {
            message_id: format!("MT940-{}", mt940.reference),
            creation_date_time: now.to_string(),
            statement_id: mt940.statement_number.clone(),
            account,
            balances: vec![opening_balance, closing_balance],
            entries,
        }
    }
}

impl TryFrom<Camt053Statement> for Mt940Statement {
    type Error = Error;

    fn try_from(camt: Camt053Statement) -> Result<Self, Self::Error> {
        let account_id = camt
            .account
            .iban
            .clone()
            .unwrap_or_else(|| "UNKNOWN".to_string());

        let opening_balance = camt
            .balances
            .iter()
            .find(|b| b.balance_type == "OPBD")
            .map(|b| Mt940Balance {
                credit_debit: if b.credit_debit_indicator == "CRDT" {
                    'C'
                } else {
                    'D'
                },
                date: b.date.clone(),
                currency: b.currency.clone(),
                amount: b.amount,
            })
            .ok_or_else(|| {
                Error::MissingField("Отсутствует начальный баланс (OPBD)".to_string())
            })?;

        let closing_balance = camt
            .balances
            .iter()
            .find(|b| b.balance_type == "CLBD")
            .map(|b| Mt940Balance {
                credit_debit: if b.credit_debit_indicator == "CRDT" {
                    'C'
                } else {
                    'D'
                },
                date: b.date.clone(),
                currency: b.currency.clone(),
                amount: b.amount,
            })
            .ok_or_else(|| {
                Error::MissingField("Отсутствует конечный баланс (CLBD)".to_string())
            })?;

        let transactions: Vec<Mt940Transaction> = camt
            .entries
            .into_iter()
            .map(|entry| {
                let (reference, details) = if let Some(tx_details) = entry.transaction_details.first()
                {
                    let ref_str = tx_details
                        .transaction_id
                        .clone()
                        .or_else(|| entry.account_servicer_ref.clone());

                    let mut details_parts: Vec<String> = Vec::new();

                    if let Some(ref name) = tx_details.debtor_name {
                        details_parts.push(name.clone());
                    }
                    if let Some(ref name) = tx_details.creditor_name {
                        details_parts.push(name.clone());
                    }
                    if let Some(ref acct) = tx_details.debtor_account {
                        details_parts.push(acct.clone());
                    }
                    if let Some(ref acct) = tx_details.creditor_account {
                        details_parts.push(acct.clone());
                    }

                    for info in &tx_details.remittance_info {
                        details_parts.push(info.clone());
                    }

                    let details_str = details_parts.join(" ");

                    (ref_str, details_str)
                } else {
                    (entry.account_servicer_ref.clone(), String::new())
                };

                Mt940Transaction {
                    date: entry.booking_date.clone(),
                    value_date: entry.value_date,
                    credit_debit: if entry.credit_debit_indicator == "CRDT" {
                        'C'
                    } else {
                        'D'
                    },
                    amount: entry.amount,
                    transaction_type: "NTRF".to_string(),
                    reference,
                    details,
                }
            })
            .collect();

        Ok(Mt940Statement {
            reference: camt.message_id.replace("MT940-", ""),
            account_id,
            statement_number: camt.statement_id,
            opening_balance,
            closing_balance,
            transactions,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Date;

    #[test]
    fn test_mt940_to_camt053_conversion() {
        let mt940 = Mt940Statement {
            reference: "TEST001".to_string(),
            account_id: "NL81ASNB9999999999".to_string(),
            statement_number: "1/1".to_string(),
            opening_balance: Mt940Balance {
                credit_debit: 'C',
                date: Date::new(2024, 1, 1),
                currency: "EUR".to_string(),
                amount: 10000,
            },
            closing_balance: Mt940Balance {
                credit_debit: 'C',
                date: Date::new(2024, 1, 31),
                currency: "EUR".to_string(),
                amount: 15000,
            },
            transactions: vec![Mt940Transaction {
                date: Date::new(2024, 1, 15),
                value_date: Some(Date::new(2024, 1, 15)),
                credit_debit: 'C',
                amount: 5000,
                transaction_type: "NTRF".to_string(),
                reference: Some("REF001".to_string()),
                details: "Test payment".to_string(),
            }],
        };

        let camt: Camt053Statement = mt940.into();

        assert_eq!(camt.account.iban, Some("NL81ASNB9999999999".to_string()));
        assert_eq!(camt.balances.len(), 2);
        assert_eq!(camt.entries.len(), 1);
        assert_eq!(camt.entries[0].credit_debit_indicator, "CRDT");
    }

    #[test]
    fn test_camt053_to_mt940_conversion() {
        let camt = Camt053Statement {
            message_id: "MSG001".to_string(),
            creation_date_time: "2024-01-01T00:00:00".to_string(),
            statement_id: "STMT001".to_string(),
            account: Camt053Account {
                iban: Some("DK8030000001234567".to_string()),
                currency: "DKK".to_string(),
                name: Some("Test Account".to_string()),
                owner_name: Some("Test Owner".to_string()),
            },
            balances: vec![
                Camt053Balance {
                    balance_type: "OPBD".to_string(),
                    amount: 100000,
                    currency: "DKK".to_string(),
                    credit_debit_indicator: "CRDT".to_string(),
                    date: Date::new(2024, 1, 1),
                },
                Camt053Balance {
                    balance_type: "CLBD".to_string(),
                    amount: 150000,
                    currency: "DKK".to_string(),
                    credit_debit_indicator: "CRDT".to_string(),
                    date: Date::new(2024, 1, 31),
                },
            ],
            entries: vec![],
        };

        let mt940: Mt940Statement = camt.try_into().unwrap();

        assert_eq!(mt940.account_id, "DK8030000001234567");
        assert_eq!(mt940.opening_balance.credit_debit, 'C');
        assert_eq!(mt940.opening_balance.amount, 100000);
    }

    #[test]
    fn test_camt053_to_mt940_missing_balance() {
        let camt = Camt053Statement {
            message_id: "MSG001".to_string(),
            creation_date_time: "2024-01-01T00:00:00".to_string(),
            statement_id: "STMT001".to_string(),
            account: Camt053Account {
                iban: Some("DK8030000001234567".to_string()),
                currency: "DKK".to_string(),
                name: None,
                owner_name: None,
            },
            balances: vec![],
            entries: vec![],
        };

        let result: Result<Mt940Statement, _> = camt.try_into();
        assert!(result.is_err());
    }
}

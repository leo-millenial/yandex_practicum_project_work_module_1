//! Модуль конвертации между форматами.

use crate::camt053::parser::{
    Camt053Account, Camt053Balance, Camt053Entry, Camt053Statement, Camt053TransactionDetails,
};
use crate::error::Error;
use crate::mt940::parser::{Mt940Balance, Mt940Statement, Mt940Transaction};
use crate::types::{
    BALANCE_TYPE_CLOSING, BALANCE_TYPE_OPENING, CREDIT_INDICATOR, DEBIT_INDICATOR,
    END_TO_END_NOT_PROVIDED, TRANSACTION_TYPE_TRANSFER,
};

impl From<Mt940Statement> for Camt053Statement {
    fn from(mt940: Mt940Statement) -> Self {
        let currency = mt940.opening_balance.currency.clone();

        let account = Camt053Account {
            iban: if mt940.account_id.len() > 10 {
                Some(mt940.account_id.clone())
            } else {
                None
            },
            currency: currency.clone(),
            name: None,
            owner_name: None,
        };

        let opening_balance = Camt053Balance {
            balance_type: BALANCE_TYPE_OPENING.to_string(),
            amount: mt940.opening_balance.amount,
            currency: currency.clone(),
            credit_debit_indicator: if mt940.opening_balance.credit_debit == 'C' {
                CREDIT_INDICATOR.to_string()
            } else {
                DEBIT_INDICATOR.to_string()
            },
            date: mt940.opening_balance.date,
        };

        let closing_balance = Camt053Balance {
            balance_type: BALANCE_TYPE_CLOSING.to_string(),
            amount: mt940.closing_balance.amount,
            currency: mt940.closing_balance.currency,
            credit_debit_indicator: if mt940.closing_balance.credit_debit == 'C' {
                CREDIT_INDICATOR.to_string()
            } else {
                DEBIT_INDICATOR.to_string()
            },
            date: mt940.closing_balance.date,
        };

        let entries: Vec<Camt053Entry> = mt940
            .transactions
            .into_iter()
            .enumerate()
            .map(|(idx, tx)| {
                let (debtor_name, debtor_account, creditor_name, creditor_account, remittance_info) =
                    if tx.credit_debit == 'C' {
                        (
                            Some(tx.details.clone()),
                            tx.reference.clone(),
                            None,
                            None,
                            if tx.details.is_empty() {
                                vec![]
                            } else {
                                vec![tx.details.clone()]
                            },
                        )
                    } else {
                        (
                            None,
                            None,
                            Some(tx.details.clone()),
                            tx.reference.clone(),
                            if tx.details.is_empty() {
                                vec![]
                            } else {
                                vec![tx.details]
                            },
                        )
                    };

                let transaction_details = vec![Camt053TransactionDetails {
                    end_to_end_id: Some(END_TO_END_NOT_PROVIDED.to_string()),
                    transaction_id: tx.reference.clone(),
                    amount: Some(tx.amount),
                    currency: Some(currency.clone()),
                    debtor_name,
                    debtor_account,
                    creditor_name,
                    creditor_account,
                    remittance_info,
                }];

                Camt053Entry {
                    entry_ref: Some(format!("{}", idx + 1)),
                    amount: tx.amount,
                    currency: currency.clone(),
                    credit_debit_indicator: if tx.credit_debit == 'C' {
                        CREDIT_INDICATOR.to_string()
                    } else {
                        DEBIT_INDICATOR.to_string()
                    },
                    booking_date: tx.date,
                    value_date: tx.value_date,
                    account_servicer_ref: tx.reference,
                    transaction_details,
                }
            })
            .collect();

        Camt053Statement {
            message_id: format!("MT940-{}", mt940.reference),
            creation_date_time: "2024-01-01T00:00:00".to_string(),
            statement_id: mt940.statement_number,
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
            .unwrap_or_else(|| "UNKNOWN".to_string());

        let mut opening_balance_opt = None;
        let mut closing_balance_opt = None;

        for b in camt.balances {
            let balance = Mt940Balance {
                credit_debit: if b.credit_debit_indicator == CREDIT_INDICATOR {
                    'C'
                } else {
                    'D'
                },
                date: b.date,
                currency: b.currency,
                amount: b.amount,
            };

            match b.balance_type.as_str() {
                BALANCE_TYPE_OPENING => opening_balance_opt = Some(balance),
                BALANCE_TYPE_CLOSING => closing_balance_opt = Some(balance),
                _ => {}
            }
        }

        let opening_balance = opening_balance_opt.ok_or_else(|| {
            Error::MissingField("Отсутствует начальный баланс (OPBD)".to_string())
        })?;

        let closing_balance = closing_balance_opt.ok_or_else(|| {
            Error::MissingField("Отсутствует конечный баланс (CLBD)".to_string())
        })?;

        let transactions: Vec<Mt940Transaction> = camt
            .entries
            .into_iter()
            .map(|entry| {
                let (reference, details) = if let Some(tx_details) =
                    entry.transaction_details.into_iter().next()
                {
                    let ref_str = tx_details
                        .transaction_id
                        .or(entry.account_servicer_ref.clone());

                    let mut details_parts: Vec<String> = Vec::new();

                    if let Some(name) = tx_details.debtor_name {
                        details_parts.push(name);
                    }
                    if let Some(name) = tx_details.creditor_name {
                        details_parts.push(name);
                    }
                    if let Some(acct) = tx_details.debtor_account {
                        details_parts.push(acct);
                    }
                    if let Some(acct) = tx_details.creditor_account {
                        details_parts.push(acct);
                    }

                    details_parts.extend(tx_details.remittance_info);

                    let details_str = details_parts.join(" ");

                    (ref_str, details_str)
                } else {
                    (entry.account_servicer_ref, String::new())
                };

                Mt940Transaction {
                    date: entry.booking_date,
                    value_date: entry.value_date,
                    credit_debit: if entry.credit_debit_indicator == CREDIT_INDICATOR {
                        'C'
                    } else {
                        'D'
                    },
                    amount: entry.amount,
                    transaction_type: TRANSACTION_TYPE_TRANSFER.to_string(),
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

//! Базовые типы данных для представления банковских выписок.

// =============================================================================
// Константы для CAMT.053 формата
// =============================================================================

/// Тип баланса: начальный (Opening Booked).
pub const BALANCE_TYPE_OPENING: &str = "OPBD";
/// Тип баланса: конечный (Closing Booked).
pub const BALANCE_TYPE_CLOSING: &str = "CLBD";

/// Индикатор кредита (поступление).
pub const CREDIT_INDICATOR: &str = "CRDT";
/// Индикатор дебета (списание).
pub const DEBIT_INDICATOR: &str = "DBIT";

/// Тип транзакции по умолчанию (перевод).
pub const TRANSACTION_TYPE_TRANSFER: &str = "NTRF";

/// End-to-end идентификатор по умолчанию.
pub const END_TO_END_NOT_PROVIDED: &str = "NOTPROVIDED";

// =============================================================================
// Структуры данных
// =============================================================================

/// Дата в формате год-месяц-день.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Date {
    /// Год (например, 2024).
    pub year: u16,
    /// Месяц (1-12).
    pub month: u8,
    /// День месяца (1-31).
    pub day: u8,
}

impl Date {
    /// Создает новую дату.
    pub fn new(year: u16, month: u8, day: u8) -> Self {
        Self { year, month, day }
    }
}

impl std::fmt::Display for Date {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

/// Денежная сумма с валютой.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Amount {
    /// Значение в минимальных единицах (копейки, центы).
    pub value: i64,
    /// Код валюты (EUR, USD, RUB и т.д.).
    pub currency: String,
}

impl Amount {
    /// Создает новую сумму.
    pub fn new(value: i64, currency: impl Into<String>) -> Self {
        Self {
            value,
            currency: currency.into(),
        }
    }

    /// Возвращает значение в основных единицах (рубли, евро).
    pub fn as_float(&self) -> f64 {
        self.value as f64 / 100.0
    }
}

/// Информация о контрагенте.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Counterparty {
    /// Название контрагента.
    pub name: Option<String>,
    /// IBAN или номер счета.
    pub account: Option<String>,
    /// БИК банка.
    pub bank_code: Option<String>,
    /// Название банка.
    pub bank_name: Option<String>,
}

/// Банковский счет.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    /// IBAN (если есть).
    pub iban: Option<String>,
    /// Номер счета.
    pub number: String,
    /// Код валюты.
    pub currency: String,
    /// Название счета.
    pub name: Option<String>,
    /// Владелец счета.
    pub owner: Option<String>,
}

/// Баланс счета.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Balance {
    /// Сумма баланса.
    pub amount: Amount,
    /// Дата баланса.
    pub date: Date,
    /// true = кредит (положительный), false = дебет (отрицательный).
    pub is_credit: bool,
}

/// Банковская транзакция.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transaction {
    /// Дата проводки.
    pub date: Date,
    /// Дата валютирования (если отличается).
    pub value_date: Option<Date>,
    /// Сумма транзакции.
    pub amount: Amount,
    /// true = поступление, false = списание.
    pub is_credit: bool,
    /// Референс/идентификатор транзакции.
    pub reference: Option<String>,
    /// Описание/назначение платежа.
    pub description: String,
    /// Информация о контрагенте.
    pub counterparty: Option<Counterparty>,
}

/// Банковская выписка.
#[derive(Debug, Clone)]
pub struct Statement {
    /// Информация о счете.
    pub account: Account,
    /// Начальный баланс.
    pub opening_balance: Balance,
    /// Конечный баланс.
    pub closing_balance: Balance,
    /// Список транзакций.
    pub transactions: Vec<Transaction>,
    /// Номер выписки.
    pub statement_number: Option<String>,
    /// Референс выписки.
    pub reference: Option<String>,
}


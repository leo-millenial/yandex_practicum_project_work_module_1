//! # YPBank Parser
//!
//! Библиотека для парсинга и сериализации банковских выписок
//! в форматах MT940, CAMT.053 (ISO 20022) и CSV.
//!
//! ## Поддерживаемые форматы
//!
//! - **MT940** - текстовый формат SWIFT для банковских выписок
//! - **CAMT.053** - XML формат ISO 20022
//! - **CSV** - формат банковских выгрузок (СберБизнес и др.)
//!
//! ## Пример использования
//!
//! ```rust,ignore
//! use ypbank_parser::{Mt940Statement, Statement};
//! use std::fs::File;
//!
//! let mut file = File::open("statement.mt940")?;
//! let mt940 = Mt940Statement::from_read(&mut file)?;
//! let statement: Statement = mt940.into();
//! ```

pub mod error;
pub mod types;
pub mod mt940;
pub mod csv;
pub mod camt053;
pub mod convert;

pub use error::{Error, Result};
pub use types::*;
pub use mt940::{Mt940Statement, Mt940Writer};
pub use csv::{CsvStatement, CsvWriter};
pub use camt053::{Camt053Statement, Camt053Writer};

/// Поддерживаемые форматы.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// MT940 (SWIFT)
    Mt940,
    /// CAMT.053 (ISO 20022 XML)
    Camt053,
    /// CSV
    Csv,
}

impl std::str::FromStr for Format {
    type Err = ();

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "mt940" => Ok(Format::Mt940),
            "camt053" | "camt" | "xml" => Ok(Format::Camt053),
            "csv" => Ok(Format::Csv),
            _ => Err(()),
        }
    }
}

impl Format {
    /// Парсит формат из строки.
    pub fn parse(s: &str) -> Option<Self> {
        s.parse().ok()
    }
}

/// Парсит выписку из строки в универсальный формат Statement.
pub fn parse_statement(content: &str, format: Format) -> Result<Statement> {
    match format {
        Format::Mt940 => {
            let statements = Mt940Statement::parse(content)?;
            let mt940 = statements
                .into_iter()
                .next()
                .ok_or(Error::InvalidFormat("Пустой файл MT940".into()))?;
            Ok(mt940.into())
        }
        Format::Camt053 => {
            let camt = Camt053Statement::parse(content)?;
            Ok(camt.into())
        }
        Format::Csv => {
            let csv = CsvStatement::parse(content)?;
            Ok(csv.into())
        }
    }
}


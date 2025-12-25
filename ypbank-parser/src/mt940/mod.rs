//! Модуль парсинга и сериализации формата MT940.
//!
//! MT940 - текстовый формат SWIFT для банковских выписок.

pub mod parser;
pub mod writer;

pub use parser::{Mt940Balance, Mt940Statement, Mt940Transaction};
pub use writer::Mt940Writer;


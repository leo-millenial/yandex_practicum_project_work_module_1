//! Модуль обработки ошибок библиотеки.

use std::fmt;

/// Основной тип ошибки библиотеки.
#[derive(Debug)]
pub enum Error {
    /// Ошибка ввода/вывода
    Io(std::io::Error),
    /// Ошибка парсинга
    Parse(String),
    /// Неверный формат данных
    InvalidFormat(String),
    /// Отсутствует обязательное поле
    MissingField(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(err) => write!(f, "Ошибка ввода/вывода: {}", err),
            Error::Parse(msg) => write!(f, "Ошибка парсинга: {}", msg),
            Error::InvalidFormat(msg) => write!(f, "Неверный формат: {}", msg),
            Error::MissingField(field) => write!(f, "Отсутствует обязательное поле: {}", field),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Io(err)
    }
}

/// Тип Result с ошибкой библиотеки.
pub type Result<T> = std::result::Result<T, Error>;


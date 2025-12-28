//! Модуль обработки ошибок библиотеки.

use thiserror::Error;

/// Основной тип ошибки библиотеки.
#[derive(Debug, Error)]
pub enum Error {
    /// Ошибка ввода/вывода
    #[error("Ошибка ввода/вывода: {0}")]
    Io(#[from] std::io::Error),

    /// Ошибка парсинга
    #[error("Ошибка парсинга: {0}")]
    Parse(String),

    /// Неверный формат данных
    #[error("Неверный формат: {0}")]
    InvalidFormat(String),

    /// Отсутствует обязательное поле
    #[error("Отсутствует обязательное поле: {0}")]
    MissingField(String),
}

/// Тип Result с ошибкой библиотеки.
pub type Result<T> = std::result::Result<T, Error>;

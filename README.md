<p>
  <img src="images/Яндекс.svg" height="40" alt="Яндекс">
  <img src="images/new_logo_icon.svg" height="40" alt="Logo">
  <img src="images/Практикум.svg" height="40" alt="Практикум">
</p>

# Rust для действующих разработчиков: погружение в блокчейн

### Проектная работа №1

---

# YPBank — Парсер банковских выписок

Проект для парсинга, сериализации и конвертации банковских выписок в форматах MT940, CAMT.053 (ISO 20022) и CSV.

## Структура проекта

Проект организован как Cargo workspace с тремя крейтами:

```
project_work_module_1/
├── Cargo.toml                    # workspace manifest
├── ypbank-parser/                # библиотека парсинга/сериализации
│   ├── src/
│   │   ├── lib.rs               # публичный API
│   │   ├── error.rs             # типы ошибок
│   │   ├── types.rs             # Transaction, Account и др.
│   │   ├── mt940/               # парсер/writer MT940
│   │   ├── camt053/             # парсер/writer CAMT.053
│   │   ├── csv/                 # парсер/writer CSV
│   │   └── convert.rs           # конвертация между форматами
│   └── tests/
│       └── integration_tests.rs
├── ypbank-converter/             # CLI-утилита конвертации
│   └── src/main.rs
├── ypbank-comparer/              # CLI-утилита сравнения
│   └── src/main.rs
└── examples/                     # примеры файлов
    ├── sample.mt940
    ├── sample.camt053.xml
    └── sample.csv
```

## Поддерживаемые форматы

| Формат | Описание |
|--------|----------|
| **MT940** | Текстовый формат SWIFT для банковских выписок |
| **CAMT.053** | XML формат ISO 20022 |
| **CSV** | Формат банковских выгрузок (СберБизнес и др.) |

## Сборка

```bash
cargo build --release
```

## Использование

### Библиотека ypbank-parser

```rust
use ypbank_parser::{Mt940Statement, Statement};
use std::fs::File;

// Парсинг MT940
let mut file = File::open("statement.mt940")?;
let statements = Mt940Statement::from_read(&mut file)?;

// Конвертация в универсальный тип Statement
let statement: Statement = statements.into_iter().next().unwrap().into();

// Конвертация MT940 -> CAMT.053
use ypbank_parser::Camt053Statement;
let mt940 = Mt940Statement::parse(&content)?[0].clone();
let camt: Camt053Statement = mt940.into();

// Запись в файл
let mut output = std::io::stdout();
camt.write_to(&mut output)?;
```

### CLI: ypbank-converter

Конвертация между форматами:

```bash
# MT940 -> CSV
ypbank-converter -i statement.mt940 -if mt940 -of csv > output.csv

# CAMT.053 -> MT940
ypbank-converter -i statement.xml -if camt053 -of mt940 > output.mt940

# Из stdin в stdout
cat input.mt940 | ypbank-converter -if mt940 -of camt053 > output.xml
```

Опции:
- `--input, -i <файл>` — входной файл (по умолчанию stdin)
- `--output, -o <файл>` — выходной файл (по умолчанию stdout)
- `--input-format, -if <формат>` — формат входных данных (mt940, camt053, csv)
- `--output-format, -of <формат>` — формат выходных данных (mt940, camt053, csv)

### CLI: ypbank-comparer

Сравнение транзакций из двух выписок:

```bash
ypbank-comparer -f1 a.mt940 -fmt1 mt940 -f2 b.csv -fmt2 csv
ypbank-comparer -f1 a.xml -fmt1 camt053 -f2 b.mt940 -fmt2 mt940 -v
```

Опции:
- `--file1, -f1 <файл>` — первый файл выписки
- `--format1, -fmt1 <формат>` — формат первого файла
- `--file2, -f2 <файл>` — второй файл выписки
- `--format2, -fmt2 <формат>` — формат второго файла
- `--verbose, -v` — подробный вывод

## Тестирование

```bash
cargo test
```

## Примеры файлов

В директории `examples/` находятся примеры файлов:
- `sample.mt940` — пример выписки MT940
- `sample.camt053.xml` — пример выписки CAMT.053
- `sample.csv` — пример выписки CSV

## API документация

```bash
cargo doc --open
```

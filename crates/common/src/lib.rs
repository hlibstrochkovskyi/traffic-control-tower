// crates/common/src/lib.rs

// 1. Подключаем модуль proto
pub mod proto;

// 2. Экспортируем всё содержимое модуля proto наружу.
// Так как сгенерированные структуры лежат сразу внутри,
// мы используем * (wildcard).
pub use proto::*;

// Остальное без изменений
pub mod config;
pub use config::Config;

pub mod error;
pub use error::{Result, TrafficError};

pub mod telemetry;
pub mod map;

pub use telemetry::init_tracing;
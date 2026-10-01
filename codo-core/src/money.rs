//! お金

use thiserror::Error;

/// 金額(円)
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Money(pub i32);

#[derive(Debug, Error, PartialEq, Eq)]
pub enum MoneyError {
    #[error("金額として解釈できない文字列です: {0}")]
    Parse(String),
    #[error("金額が表現できる範囲を超えました")]
    Overflow,
}

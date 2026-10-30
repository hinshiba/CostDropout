//! 取引の金額

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use thiserror::Error;

use crate::money::{Money, MoneyError};

/// 取引の金額.
///
/// 0を含まない正の値だけをもつ.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "Money", into = "Money")]
pub struct Amount(Money);

impl TryFrom<Money> for Amount {
    type Error = AmountError;

    /// 金額を `Amount` にする.
    ///
    /// # Errors
    ///
    /// 金額が0以下の場合は `AmountError::NonPositive` エラー.
    fn try_from(money: Money) -> Result<Self, Self::Error> {
        if !money.is_positive() {
            return Err(AmountError::NonPositive(money));
        }
        Ok(Self(money))
    }
}

impl From<Amount> for Money {
    fn from(amount: Amount) -> Self {
        amount.0
    }
}

impl FromStr for Amount {
    type Err = AmountError;

    /// 取引の金額にパースする.
    ///
    /// # Errors
    ///
    /// `Money` として解釈できない場合は `AmountError::Parse` エラー.
    /// 金額が0以下の場合は `AmountError::NonPositive` エラー.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_from(s.parse::<Money>()?)
    }
}

impl fmt::Display for Amount {
    /// `Money` と同じ書式で出力する.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(test)]
impl Amount {
    /// テスト用に正の金額を作る.
    ///
    /// # Panics
    ///
    /// `value` が0以下の場合はパニック.
    pub(crate) fn of(value: i32) -> Self {
        Self::try_from(Money::from(value)).unwrap()
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum AmountError {
    /// 金額として解釈できない場合.
    #[error(transparent)]
    Parse(#[from] MoneyError),
    /// 金額が0以下の場合.
    #[error("金額は0円より大きい値を入力してください: {0}")]
    NonPositive(Money),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn try_from_test() {
        for value in [1, 2, 1000, i32::MAX] {
            let amount = Amount::try_from(Money::from(value)).unwrap();
            assert_eq!(Money::from(amount), Money::from(value), "{value}");
        }
    }

    #[test]
    fn try_from_error_test() {
        for value in [i32::MIN, -1000, -1, 0] {
            assert_eq!(
                Amount::try_from(Money::from(value)),
                Err(AmountError::NonPositive(Money::from(value))),
                "{value}"
            );
        }
    }

    #[test]
    fn from_str_test() {
        assert_eq!("¥1,234".parse::<Amount>(), Ok(Amount::of(1234)));
        for (s, expected) in [
            (
                " abc ",
                AmountError::Parse(MoneyError::Parse(" abc ".to_string())),
            ),
            ("2147483648", AmountError::Parse(MoneyError::Overflow)),
            ("0", AmountError::NonPositive(Money::from(0))),
            ("-1", AmountError::NonPositive(Money::from(-1))),
        ] {
            assert_eq!(s.parse::<Amount>(), Err(expected), "{s}");
        }
    }

    #[test]
    fn deserialize_test() {
        #[derive(Debug, Deserialize)]
        struct Row {
            amount: Amount,
        }

        let row: Row = toml::from_str("amount = 1").unwrap();
        assert_eq!(row.amount, Amount::of(1));
        for s in ["amount = 0", "amount = -1"] {
            assert!(toml::from_str::<Row>(s).is_err(), "{s}");
        }
    }
}

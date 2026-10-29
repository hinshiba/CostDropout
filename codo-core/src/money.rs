//! 金額のモデルと表示

use serde::{Deserialize, Serialize};
use std::fmt;
use std::iter::Sum;
use std::ops::{Add, AddAssign, Sub, SubAssign};
use std::str::FromStr;
use thiserror::Error;

/// 金額を表現する.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct Money(i32);

impl From<i32> for Money {
    fn from(value: i32) -> Self {
        Money(value)
    }
}

impl Add for Money {
    type Output = Money;

    /// 金額を加算する.
    ///
    /// # Panics
    ///
    /// 加算結果が `i32` の範囲を超える場合はパニック.
    fn add(self, rhs: Money) -> Money {
        self.checked_add(rhs).expect("overflowしました")
    }
}

impl Sub for Money {
    type Output = Money;

    /// 金額を減算する.
    ///
    /// # Panics
    ///
    /// 減算結果が `i32` の範囲を超える場合はパニック.
    fn sub(self, rhs: Money) -> Money {
        self.checked_sub(rhs).expect("overflowしました")
    }
}

impl Money {
    /// 金額を加算する.
    ///
    /// # Errors
    ///
    /// 加算結果が `i32` の範囲を超える場合は `MoneyError::Overflow` エラー.
    pub fn checked_add(self, rhs: Money) -> Result<Money, MoneyError> {
        match self.0.checked_add(rhs.0) {
            Some(value) => Ok(Money(value)),
            None => Err(MoneyError::Overflow),
        }
    }

    /// 金額を減算する.
    ///
    /// # Errors
    ///
    /// 減算結果が `i32` の範囲を超える場合は `MoneyError::Overflow` エラー.
    pub fn checked_sub(self, rhs: Money) -> Result<Money, MoneyError> {
        match self.0.checked_sub(rhs.0) {
            Some(value) => Ok(Money(value)),
            None => Err(MoneyError::Overflow),
        }
    }

    /// `base` に対する比率を返す.
    ///
    /// `base` が0の場合は `None`.
    pub fn ratio(self, base: Money) -> Option<f64> {
        if base.0 == 0 {
            return None;
        }
        Some(f64::from(self.0) / f64::from(base.0))
    }

    /// 金額が0より大きいかを判定する.
    ///
    /// 0は正の値に含まれない.
    pub fn is_positive(&self) -> bool {
        0 < self.0
    }

    /// 金額が0より小さいかを判定する.
    ///
    /// 0は負の値に含まれない.
    pub fn is_negative(&self) -> bool {
        self.0 < 0
    }
}

impl AddAssign for Money {
    /// 金額を加算して代入する.
    ///
    /// # Panics
    ///
    /// 加算結果が `i32` の範囲を超える場合はパニック.
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs
    }
}

impl SubAssign for Money {
    /// 金額を減算して代入する.
    ///
    /// # Panics
    ///
    /// 減算結果が `i32` の範囲を超える場合はパニック.
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs
    }
}

impl Sum<Money> for Money {
    /// 金額を合計する.
    ///
    /// # Panics
    ///
    /// 途中の合計が `i32` の範囲を超える場合はパニック.
    fn sum<I: Iterator<Item = Money>>(iter: I) -> Self {
        iter.fold(Money(0), |total, item| total + item)
    }
}

impl<'a> Sum<&'a Money> for Money {
    /// 金額を合計する.
    ///
    /// # Panics
    ///
    /// 途中の合計が `i32` の範囲を超える場合はパニック.
    fn sum<I: Iterator<Item = &'a Money>>(iter: I) -> Self {
        iter.fold(Money(0), |total, item| total + *item)
    }
}

impl fmt::Display for Money {
    /// 金額を整形して出力する.
    ///
    /// Unicode CLDR の ja ロケールの通貨書式にできるだけ準拠する.
    /// ただし通貨記号として U+00A5 `¥` を用いる.
    ///
    /// # Examples
    ///
    /// ```
    /// use codo_core::money::Money;
    ///
    /// assert_eq!(Money::from(1234567).to_string(), "¥1,234,567");
    /// assert_eq!(Money::from(-1234).to_string(), "-¥1,234");
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let digits = self.0.unsigned_abs().to_string();

        // 残りの桁数が3の倍数になる位置の手前に桁区切りを挿入する
        let mut grouped = String::with_capacity(digits.len() + digits.len() / 3);
        for (i, c) in digits.chars().enumerate() {
            if 0 < i && (digits.len() - i).is_multiple_of(3) {
                grouped.push(',');
            }
            grouped.push(c);
        }

        let sign = if self.is_negative() { "-" } else { "" };
        write!(f, "{sign}¥{grouped}")
    }
}

/// 金額の表記で無視する文字か.
fn is_ignored(c: char) -> bool {
    matches!(c, '¥' | '\\' | ',') || c.is_whitespace()
}

/// 空でなく, ASCII の数字だけからなるか.
fn is_digits(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

impl FromStr for Money {
    type Err = MoneyError;

    /// `Money` にパースする.
    ///
    /// 先頭の符号 `+` / `-` と ASCII の数字からなる金額として解釈する.
    /// `¥`, `\` (0x5C), `,`, 空白は位置によらず存在しないものとして扱う.
    ///
    /// 全角文字, `円`, 小数点を含む入力に対する結果は未定義.
    ///
    /// # Examples
    ///
    /// ```
    /// use codo_core::money::Money;
    ///
    /// assert_eq!("¥1,234".parse::<Money>(), Ok(Money::from(1234)));
    /// assert_eq!("-¥1,234".parse::<Money>(), Ok(Money::from(-1234)));
    /// assert_eq!("¥-1,234".parse::<Money>(), Ok(Money::from(-1234)));
    /// assert_eq!("\\1 234".parse::<Money>(), Ok(Money::from(1234)));
    /// ```
    ///
    /// # Errors
    ///
    /// 不正な形式の場合は `MoneyError::Parse` エラー.
    /// 表現可能な範囲を超える場合は `MoneyError::Overflow` エラー.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let significant: String = s.chars().filter(|&c| !is_ignored(c)).collect();
        let digits = significant.strip_prefix(['+', '-']).unwrap_or(&significant);

        if !is_digits(digits) {
            return Err(MoneyError::Parse(s.to_string()));
        }

        // 形式は検証済みなので, 失敗するのは範囲外の場合に限る
        significant
            .parse::<i32>()
            .map(Money)
            .map_err(|_| MoneyError::Overflow)
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum MoneyError {
    /// 金額として解釈できない文字列である場合.
    #[error("金額として解釈できない文字列です: {0}")]
    Parse(String),
    /// 金額が表現可能な範囲を超えた場合.
    #[error("金額が表現できる範囲を超えました")]
    Overflow,
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_test() {
        assert_eq!(Money(1234).to_string(), "¥1,234");
        assert_eq!(Money(0).to_string(), "¥0");
        assert_eq!(Money(1000000).to_string(), "¥1,000,000");
        assert_eq!(Money(-5000).to_string(), "-¥5,000");
    }

    #[test]
    fn from_str_test() {
        assert_eq!("1234".parse::<Money>().unwrap(), Money(1234));
        assert_eq!("1,234".parse::<Money>().unwrap(), Money(1234));
        assert_eq!("¥1,234".parse::<Money>().unwrap(), Money(1234));
        assert_eq!("-500".parse::<Money>().unwrap(), Money(-500));
        assert_eq!("  1,234  ".parse::<Money>().unwrap(), Money(1234));
    }

    #[test]
    fn from_str_error_test() {
        assert!(matches!("".parse::<Money>(), Err(MoneyError::Parse(_))));

        assert!(matches!("abc".parse::<Money>(), Err(MoneyError::Parse(_))));

        assert!(matches!(
            "1,,234".parse::<Money>(),
            Err(MoneyError::Parse(_))
        ));

        assert!(matches!(
            "12,34".parse::<Money>(),
            Err(MoneyError::Parse(_))
        ));
    }

    #[test]
    fn from_str_overflow_test() {
        assert!(matches!(
            "2147483648".parse::<Money>(),
            Err(MoneyError::Overflow)
        ));
        assert!(matches!(
            "-2147483649".parse::<Money>(),
            Err(MoneyError::Overflow)
        ));
        assert!(matches!(
            "99999999999999999999".parse::<Money>(),
            Err(MoneyError::Overflow)
        ));
        assert!(matches!(
            "2,147,483,648".parse::<Money>(),
            Err(MoneyError::Overflow)
        ));
    }

    #[test]
    fn is_negative_test() {
        assert!(Money(-500).is_negative());
        assert!(!Money(0).is_negative());
        assert!(!Money(500).is_negative());
    }

    #[test]
    fn is_positive_test() {
        assert!(Money(500).is_positive());
        assert!(!Money(0).is_positive());
        assert!(!Money(-500).is_positive());
    }

    #[test]
    fn is_positive_negative_test() {
        assert!(matches!("--5".parse::<Money>(), Err(MoneyError::Parse(_))));
        assert!(matches!("-+5".parse::<Money>(), Err(MoneyError::Parse(_))));
        assert!(matches!(
            "¥-500".parse::<Money>(),
            Err(MoneyError::Parse(_))
        ));
    }

    #[test]
    fn display_boundary_test() {
        assert_eq!(Money(-500).to_string(), "-¥500");
        assert_eq!(Money(999).to_string(), "¥999");
        assert_eq!(Money(1000).to_string(), "¥1,000");
        assert_eq!(Money(-1).to_string(), "-¥1");
        assert_eq!(Money(i32::MAX).to_string(), "¥2,147,483,647");
        assert_eq!(Money(i32::MIN).to_string(), "-¥2,147,483,648");
    }

    #[test]
    fn from_str_sign_test() {
        assert_eq!("+5".parse::<Money>().unwrap(), Money(5));
        assert_eq!("+¥5".parse::<Money>().unwrap(), Money(5));
        assert_eq!("-¥500".parse::<Money>().unwrap(), Money(-500));
        assert_eq!("-¥1,234".parse::<Money>().unwrap(), Money(-1234));
        assert_eq!("-0".parse::<Money>().unwrap(), Money(0));
        assert_eq!("007".parse::<Money>().unwrap(), Money(7));
    }

    #[test]
    fn from_str_comma_error_test() {
        for s in [
            ",123", "123,", "1234,567", "1,2345", "1,23", "0,123", "00,123", "01,234",
        ] {
            assert!(
                matches!(s.parse::<Money>(), Err(MoneyError::Parse(_))),
                "{s}"
            );
        }
    }

    #[test]
    fn from_str_invalid_error_test() {
        for s in [
            "   ", "¥", "-", "-¥", "12a", "1 234", "- 500", "¥ 500", "1234.0", "--5", "-+5", "+-5",
            "¥-500", "¥+5",
        ] {
            assert!(
                matches!(s.parse::<Money>(), Err(MoneyError::Parse(_))),
                "{s}"
            );
        }
    }

    #[test]
    fn from_str_error_keeps_input_test() {
        for s in ["", "   ", "  -¥5,00 ", "¥-500", "abc"] {
            assert_eq!(s.parse::<Money>(), Err(MoneyError::Parse(s.to_string())));
        }
    }
}

//! お金

use std::fmt;
use std::iter::Sum;
use std::ops::{Add, AddAssign, Sub, SubAssign};
use std::str::FromStr;
use thiserror::Error;

/// 金額(円)
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Money(pub i32);

impl Add for Money {
    type Output = Money;

    fn add(self, rhs: Money) -> Money {
        self.checked_add(rhs).expect("overflowしました")
    }
}

impl Sub for Money {
    type Output = Money;

    fn sub(self, rhs: Money) -> Money {
        self.checked_sub(rhs).expect("overflowしました")
    }
}

impl Money {
    pub fn checked_add(self, rhs: Money) -> Result<Money, MoneyError> {
        match self.0.checked_add(rhs.0) {
            Some(value) => Ok(Money(value)),
            None => Err(MoneyError::Overflow),
        }
    }

    pub fn checked_sub(self, rhs: Money) -> Result<Money, MoneyError> {
        match self.0.checked_sub(rhs.0) {
            Some(value) => Ok(Money(value)),
            None => Err(MoneyError::Overflow),
        }
    }

    pub fn is_positive(&self) -> bool {
        0 < self.0
    }

    pub fn is_negative(&self) -> bool {
        self.0 < 0
    }
}

impl AddAssign for Money {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs
    }
}

impl SubAssign for Money {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs
    }
}

impl Sum<Money> for Money {
    fn sum<I: Iterator<Item = Money>>(iter: I) -> Self {
        iter.fold(Money(0), |total, item| total + item)
    }
}

impl<'a> Sum<&'a Money> for Money {
    fn sum<I: Iterator<Item = &'a Money>>(iter: I) -> Self {
        iter.fold(Money(0), |total, item| total + *item)
    }
}

impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let negative = self.0 < 0;
        let amount = (self.0 as i64).abs();
        let digits = amount.to_string();
        let mut result = String::new();
        for (count, c) in digits.chars().rev().enumerate() {
            if count > 0 && count % 3 == 0 {
                result.push(',');
            }
            result.push(c);
        }
        let formatted = result.chars().rev().collect::<String>();
        if !negative {
            write!(f, "¥{}", formatted)
        } else {
            write!(f, "-¥{}", formatted)
        }
    }
}

fn is_digits(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

fn is_valid_amount(s: &str) -> bool {
    match s.split_once(',') {
        None => is_digits(s),

        Some((head, tail)) => {
            is_digits(head)
                && (1..4).contains(&head.len())
                && !head.starts_with('0')
                && tail
                    .split(',')
                    .all(|part| is_digits(part) && part.len() == 3)
        }
    }
}

impl FromStr for Money {
    type Err = MoneyError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let input = s;
        let s = s.trim();

        if s.is_empty() {
            return Err(MoneyError::Parse(input.to_string()));
        }
        let mut s = s;
        let mut negative = false;

        if let Some(rest) = s.strip_prefix('-') {
            negative = true;
            s = rest;
        } else if let Some(rest) = s.strip_prefix('+') {
            s = rest;
        }

        if let Some(rest) = s.strip_prefix('¥') {
            s = rest;
        }

        if !is_valid_amount(s) {
            return Err(MoneyError::Parse(input.to_string()));
        }

        let digits = s.replace(',', "");

        let mut value = match digits.parse::<i64>() {
            Ok(value) => value,
            Err(_) => return Err(MoneyError::Overflow),
        };

        if negative {
            value = -value;
        }

        match i32::try_from(value) {
            Ok(value) => Ok(Money(value)),
            Err(_) => Err(MoneyError::Overflow),
        }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum MoneyError {
    #[error("金額として解釈できない文字列です: {0}")]
    Parse(String),
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

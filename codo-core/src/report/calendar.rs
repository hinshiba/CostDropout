//! 支出カレンダーの色の段階

use crate::money::Money;

/// 色の段階の数
pub const EXPENSE_LEVELS: u8 = 5;

/// その日の支出から，色の段階（0から EXPENSE_LEVELS まで）を求める
///
/// 支出が0の日は0．それ以外は，月の最大支出を EXPENSE_LEVELS 等分して切り上げる
pub fn expense_level(expense: Money, max: Money) -> u8 {
    if !expense.is_positive() || !max.is_positive() {
        return 0;
    }

    let expense = expense.0 as i64;
    let max = max.0 as i64;
    let levels = EXPENSE_LEVELS as i64;

    ((expense * levels + max - 1) / max).min(levels) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expense_level_is_zero_when_no_expense() {
        assert_eq!(expense_level(Money(0), Money(1000)), 0);
    }

    #[test]
    fn expense_level_is_zero_when_max_is_zero() {
        // 取引の無い月でも 0 で割らない
        assert_eq!(expense_level(Money(0), Money(0)), 0);
        // 最大が 0 なら，支出があっても 0 で割らない
        assert_eq!(expense_level(Money(100), Money(0)), 0);
    }

    #[test]
    fn expense_level_is_zero_when_expense_is_negative() {
        // 返金などで支出がマイナスの日は色を付けない
        assert_eq!(expense_level(Money(-1000), Money(1000)), 0);
    }

    #[test]
    fn expense_level_is_highest_at_max() {
        assert_eq!(expense_level(Money(1000), Money(1000)), EXPENSE_LEVELS);
    }

    #[test]
    fn expense_level_is_one_for_smallest_expense() {
        // 少しでも使った日は色を付ける
        assert_eq!(expense_level(Money(1), Money(1000)), 1);
    }

    #[test]
    fn expense_level_rounds_up_at_boundaries() {
        // 最大 1000 を5等分すると 200 ごとの段階になる
        assert_eq!(expense_level(Money(200), Money(1000)), 1);
        assert_eq!(expense_level(Money(201), Money(1000)), 2);
        assert_eq!(expense_level(Money(400), Money(1000)), 2);
        assert_eq!(expense_level(Money(401), Money(1000)), 3);
        assert_eq!(expense_level(Money(800), Money(1000)), 4);
        assert_eq!(expense_level(Money(801), Money(1000)), 5);
    }

    #[test]
    fn expense_level_does_not_overflow_with_large_amounts() {
        // i32 のまま 5 倍するとあふれる金額
        assert_eq!(
            expense_level(Money(i32::MAX), Money(i32::MAX)),
            EXPENSE_LEVELS
        );
    }
}

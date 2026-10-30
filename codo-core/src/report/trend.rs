//! 支出の推移

use chrono::Datelike;

use crate::item::ItemId;
use crate::ledger::Ledger;
use crate::money::Money;
use crate::period::{Period, YearMonth};
use crate::report::{Flow, expense_total, flows_in};

/// 期間と重なる月ごとの支出の合計を返す.
///
/// 支出のない月は0として含める.
/// `item` が `Some` ならその品目の支出だけを数える.
///
/// # Panics
///
/// 月の合計が `i32` の範囲を超える場合はパニック.
pub fn monthly_expense(
    ledger: &Ledger,
    period: Period,
    item: Option<ItemId>,
) -> Vec<(YearMonth, Money)> {
    period
        .months()
        .map(|month| {
            let total = flows_in(ledger, Period::month(month))
                .filter(|&(tx, flow)| {
                    flow == Flow::Expense
                        && period.contains(tx.date())
                        && item.is_none_or(|item| tx.item() == item)
                })
                .map(|(tx, _)| Money::from(tx.amount()))
                .sum();
            (month, total)
        })
        .collect()
}

/// 最初の取引の年から最後の取引の年までの, 年ごとの支出の合計を返す.
///
/// 支出のない年は0として含める.
/// 取引がなければ空.
///
/// # Panics
///
/// 年の合計が `i32` の範囲を超える場合はパニック.
pub fn yearly_expense(ledger: &Ledger) -> Vec<(i32, Money)> {
    let mut transactions = ledger.transactions().iter();
    let Some(first) = transactions.next() else {
        return Vec::new();
    };
    let last = transactions.next_back().unwrap_or(first);

    (first.date().year()..=last.date().year())
        .map(|year| (year, expense_total(ledger, Period::year(year))))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::fixture::{Book, date};
    use chrono::NaiveDate;

    fn ym(year: i32, month: u32) -> YearMonth {
        YearMonth::new(year, month).unwrap()
    }

    fn ymd(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    #[test]
    fn monthly_expense_test() {
        let mut book = Book::new();
        // 期間の前後の日は数えない
        book.add(date(10, 9), 1, book.food);
        book.add(date(10, 10), 3_000, book.food);
        book.add(date(10, 20), 4_000, book.fun);
        // 収入と負債は数えない
        book.add(date(10, 21), 10_000, book.salary);
        book.add(date(10, 22), 5_000, book.loan);
        book.add(ymd(2027, 1, 10), 5_000, book.food);
        book.add(ymd(2027, 1, 20), 1, book.food);
        let period = Period::new(date(10, 10), ymd(2027, 1, 19)).unwrap();

        let expected = vec![
            (ym(2026, 10), Money::from(7_000)),
            (ym(2026, 11), Money::from(0)),
            (ym(2026, 12), Money::from(0)),
            (ym(2027, 1), Money::from(5_000)),
        ];
        assert_eq!(monthly_expense(&book.ledger, period, None), expected);
    }

    #[test]
    fn monthly_expense_with_item_test() {
        let mut book = Book::new();
        book.add(date(10, 5), 3_000, book.food);
        book.add(date(10, 10), 4_000, book.food);
        book.add(date(10, 10), 10_000, book.fun);
        let period = Period::month(ym(2026, 10));

        for (item, expected) in [(book.food, 7_000), (book.fun, 10_000), (book.salary, 0)] {
            assert_eq!(
                monthly_expense(&book.ledger, period, Some(item)),
                vec![(ym(2026, 10), Money::from(expected))],
                "{item:?}"
            );
        }
    }

    #[test]
    #[should_panic]
    fn monthly_expense_overflow_test() {
        let mut book = Book::new();
        book.add(date(10, 10), i32::MAX, book.food);
        book.add(date(10, 10), 1, book.food);

        monthly_expense(&book.ledger, Period::month(ym(2026, 10)), None);
    }

    #[test]
    fn yearly_expense_test() {
        let mut book = Book::new();
        book.add(ymd(2025, 1, 1), 3_000, book.food);
        book.add(ymd(2025, 12, 31), 1_000, book.fun);
        // 2026年は収入と負債だけ
        book.add(ymd(2026, 10, 10), 10_000, book.salary);
        book.add(ymd(2026, 11, 10), 5_000, book.loan);
        book.add(ymd(2027, 10, 10), 5_000, book.food);

        let expected = vec![
            (2025, Money::from(4_000)),
            (2026, Money::from(0)),
            (2027, Money::from(5_000)),
        ];
        assert_eq!(yearly_expense(&book.ledger), expected);
    }

    #[test]
    fn yearly_expense_one_or_no_transaction_test() {
        let mut book = Book::new();
        assert_eq!(yearly_expense(&book.ledger), vec![]);

        book.add(date(10, 10), 3_000, book.food);
        assert_eq!(
            yearly_expense(&book.ledger),
            vec![(2026, Money::from(3_000))]
        );
    }

    #[test]
    #[should_panic]
    fn yearly_expense_overflow_test() {
        let mut book = Book::new();
        book.add(date(1, 1), i32::MAX, book.food);
        book.add(date(12, 31), 1, book.food);

        yearly_expense(&book.ledger);
    }
}

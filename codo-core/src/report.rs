//! 帳簿の集計

pub mod budget;
pub mod calendar;
pub mod category;
pub mod monthly;
pub mod trend;

use crate::item::Tag;
use crate::ledger::Ledger;
use crate::money::Money;
use crate::period::Period;
use crate::transaction::Transaction;

/// 取引の収支の分類.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flow {
    /// 収入.
    Income,
    /// 支出.
    Expense,
    /// 収入にも支出にも数えない取引.
    Other,
}

impl From<Tag> for Flow {
    /// 取引の品目のタグから収支を分類する.
    fn from(tag: Tag) -> Self {
        match tag {
            Tag::Income => Flow::Income,
            Tag::Expense => Flow::Expense,
            Tag::Asset | Tag::Liability | Tag::OpeningBalance => Flow::Other,
        }
    }
}

/// 期間内の取引を, 収支の分類とともに日付順に返す.
pub fn flows_in(
    ledger: &Ledger,
    period: Period,
) -> impl DoubleEndedIterator<Item = (&Transaction, Flow)> {
    ledger.transactions().iter_in(period).map(move |tx| {
        // Ledger は存在しない品目を参照する取引をもたない
        let item = ledger.items().get(tx.item()).expect("取引の品目は存在する");
        (tx, Flow::from(item.tag()))
    })
}

/// 期間内で分類が `flow` である取引の金額を合計する.
///
/// # Panics
///
/// 合計が `i32` の範囲を超える場合はパニック.
pub fn total(ledger: &Ledger, period: Period, flow: Flow) -> Money {
    flows_in(ledger, period)
        .filter(|&(_, f)| f == flow)
        .map(|(tx, _)| Money::from(tx.amount()))
        .sum()
}

/// 期間内の収入を合計する.
///
/// # Panics
///
/// 合計が `i32` の範囲を超える場合はパニック.
pub fn income_total(ledger: &Ledger, period: Period) -> Money {
    total(ledger, period, Flow::Income)
}

/// 期間内の支出を合計する.
///
/// # Panics
///
/// 合計が `i32` の範囲を超える場合はパニック.
pub fn expense_total(ledger: &Ledger, period: Period) -> Money {
    total(ledger, period, Flow::Expense)
}

/// 集計のテストで共有する帳簿.
#[cfg(test)]
mod fixture {
    use chrono::NaiveDate;

    use crate::item::{ItemId, Tag};
    use crate::ledger::Ledger;
    use crate::transaction::{Amount, TransactionDraft};

    /// 食費, 娯楽 (支出), 給与 (収入), 借入 (負債), 現金 (資産) を登録した帳簿.
    pub(super) struct Book {
        pub ledger: Ledger,
        pub food: ItemId,
        pub fun: ItemId,
        pub salary: ItemId,
        pub loan: ItemId,
        pub cash: ItemId,
    }

    impl Book {
        pub fn new() -> Self {
            let mut ledger = Ledger::new();
            let mut add = |name, tag| ledger.add_item(name, tag).unwrap();
            let [food, fun, salary, loan, cash] = [
                add("食費", Tag::Expense),
                add("娯楽", Tag::Expense),
                add("給与", Tag::Income),
                add("借入", Tag::Liability),
                add("現金", Tag::Asset),
            ];
            Self {
                ledger,
                food,
                fun,
                salary,
                loan,
                cash,
            }
        }

        /// 品目が `item`, 相手先が現金の取引を追加する.
        pub fn add(&mut self, date: NaiveDate, amount: i32, item: ItemId) {
            let draft =
                TransactionDraft::new(date, Amount::of(amount), item, self.cash, String::new());
            self.ledger.add_transaction(draft).unwrap();
        }
    }

    /// 2026年の `month` 月 `day` 日.
    pub(super) fn date(month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, month, day).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::fixture::{Book, date};
    use super::*;
    use crate::period::YearMonth;

    #[test]
    fn flow_from_tag_test() {
        for (tag, expected) in [
            (Tag::Asset, Flow::Other),
            (Tag::Liability, Flow::Other),
            (Tag::OpeningBalance, Flow::Other),
            (Tag::Income, Flow::Income),
            (Tag::Expense, Flow::Expense),
        ] {
            assert_eq!(Flow::from(tag), expected, "{tag:?}");
        }
    }

    #[test]
    fn total_test() {
        let mut book = Book::new();
        book.add(date(9, 30), 1, book.food);
        book.add(date(10, 1), 1000, book.food);
        book.add(date(10, 15), 500, book.fun);
        book.add(date(10, 2), 5000, book.salary);
        book.add(date(10, 3), 3000, book.loan);
        book.add(date(10, 31), 200, book.food);
        book.add(date(11, 1), 1, book.salary);
        let period = Period::month(YearMonth::new(2026, 10).unwrap());

        // 期間の両端を含み, 負債は収入にも支出にも数えない
        assert_eq!(expense_total(&book.ledger, period), Money::from(1700));
        assert_eq!(income_total(&book.ledger, period), Money::from(5000));
        assert_eq!(total(&book.ledger, period, Flow::Other), Money::from(3000));
    }

    #[test]
    #[should_panic]
    fn total_overflow_test() {
        let mut book = Book::new();
        book.add(date(10, 1), i32::MAX, book.food);
        book.add(date(10, 2), 1, book.food);

        expense_total(
            &book.ledger,
            Period::month(YearMonth::new(2026, 10).unwrap()),
        );
    }
}

//! 品目別の集計

use std::collections::BTreeMap;

use crate::ledger::Ledger;
use crate::ledger::item::ItemId;
use crate::ledger::money::Money;
use crate::ledger::period::Period;
use crate::ledger::report::{Flow, flows_in};

/// 品目ごとの合計.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ItemShare {
    item: ItemId,
    amount: Money,
    ratio: f64,
}

impl ItemShare {
    /// 品目を返す.
    pub fn item(&self) -> ItemId {
        self.item
    }

    /// 品目の合計を返す.
    pub fn amount(&self) -> Money {
        self.amount
    }

    /// 期間内で同じ分類の総額に対する割合を返す.
    pub fn ratio(&self) -> f64 {
        self.ratio
    }
}

/// 期間内で分類が `flow` である取引を品目ごとに集計する.
///
/// 金額の大きい順, 同じ金額なら品目の ID の小さい順に並ぶ.
/// 該当する取引のない品目は含めない.
///
/// # Panics
///
/// 合計が `i32` の範囲を超える場合はパニック.
pub fn by_item(ledger: &Ledger, period: Period, flow: Flow) -> Vec<ItemShare> {
    let mut amounts: BTreeMap<ItemId, Money> = BTreeMap::new();
    for (tx, _) in flows_in(ledger, period).filter(|&(_, f)| f == flow) {
        *amounts.entry(tx.item()).or_default() += Money::from(tx.amount());
    }

    let total: Money = amounts.values().sum();
    let mut result: Vec<ItemShare> = amounts
        .into_iter()
        .map(|(item, amount)| ItemShare {
            item,
            amount,
            // 取引の金額は正なので, 1件でもあれば総額は0にならない
            ratio: amount.ratio(total).expect("総額は0でない"),
        })
        .collect();
    result.sort_by(|a, b| b.amount.cmp(&a.amount).then(a.item.cmp(&b.item)));
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ledger::period::YearMonth;
    use crate::ledger::report::fixture::{Book, date};

    fn october() -> Period {
        Period::month(YearMonth::new(2026, 10).unwrap())
    }

    fn summary(shares: &[ItemShare]) -> Vec<(ItemId, Money)> {
        shares.iter().map(|s| (s.item(), s.amount())).collect()
    }

    #[test]
    fn by_item_expense_test() {
        let mut book = Book::new();
        book.add(date(10, 1), 3_000, book.food);
        book.add(date(10, 2), 5_000, book.fun);
        book.add(date(10, 31), 5_000, book.fun);
        book.add(date(10, 3), 7_000, book.food);
        // 収入, 負債, 期間外の支出は数えない
        book.add(date(10, 4), 100_000, book.salary);
        book.add(date(10, 5), 50_000, book.loan);
        book.add(date(9, 30), 1, book.food);
        book.add(date(11, 1), 1, book.fun);

        let result = by_item(&book.ledger, october(), Flow::Expense);

        // 同じ金額なら品目の ID の小さい順
        assert_eq!(
            summary(&result),
            vec![
                (book.food, Money::from(10_000)),
                (book.fun, Money::from(10_000))
            ]
        );
        for share in &result {
            assert!((share.ratio() - 0.5).abs() < 1e-10);
        }
    }

    #[test]
    fn by_item_expense_sorts_larger_amount_first_test() {
        let mut book = Book::new();
        book.add(date(10, 1), 3_000, book.food);
        book.add(date(10, 2), 9_000, book.fun);

        let result = by_item(&book.ledger, october(), Flow::Expense);

        assert_eq!(
            summary(&result),
            vec![
                (book.fun, Money::from(9_000)),
                (book.food, Money::from(3_000))
            ]
        );
        assert!((result[0].ratio() - 0.75).abs() < 1e-10);
        assert!((result[1].ratio() - 0.25).abs() < 1e-10);
    }

    #[test]
    fn by_item_without_expense_test() {
        let mut book = Book::new();
        book.add(date(10, 1), 5_000, book.salary);
        book.add(date(9, 30), 3_000, book.food);

        assert_eq!(by_item(&book.ledger, october(), Flow::Expense), vec![]);
    }

    #[test]
    fn by_item_income_test() {
        let mut book = Book::new();
        book.add(date(10, 25), 200_000, book.salary);
        book.add(date(10, 1), 3_000, book.food);
        book.add(date(10, 5), 50_000, book.loan);

        let result = by_item(&book.ledger, october(), Flow::Income);

        assert_eq!(summary(&result), vec![(book.salary, Money::from(200_000))]);
        assert!((result[0].ratio() - 1.0).abs() < 1e-10);
    }
}

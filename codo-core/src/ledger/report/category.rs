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
    ratio: Option<f64>,
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

    /// 期間内で同じ分類の, 合計が正である品目の総額に対する割合を返す.
    ///
    /// 合計が0以下の品目は `None`.
    pub fn ratio(&self) -> Option<f64> {
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
        *amounts.entry(tx.item()).or_default() += tx.amount();
    }

    // 負の合計を混ぜると割合が100%を超えるので, 正の品目だけで総額を求める
    let total: Money = amounts.values().filter(|a| a.is_positive()).sum();
    let mut result: Vec<ItemShare> = amounts
        .into_iter()
        .map(|(item, amount)| ItemShare {
            item,
            amount,
            ratio: if amount.is_positive() {
                amount.ratio(total)
            } else {
                None
            },
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
            assert_eq!(share.ratio(), Some(0.5));
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
        assert_eq!(result[0].ratio(), Some(0.75));
        assert_eq!(result[1].ratio(), Some(0.25));
    }

    #[test]
    fn by_item_excludes_non_positive_item_from_ratio_test() {
        let mut book = Book::new();
        book.add(date(10, 1), 1_000, book.food);
        book.add(date(10, 2), -500, book.fun);
        book.add(date(10, 3), 300, book.salary);
        book.add(date(10, 4), -300, book.salary);

        let expense = by_item(&book.ledger, october(), Flow::Expense);
        assert_eq!(
            summary(&expense),
            vec![
                (book.food, Money::from(1_000)),
                (book.fun, Money::from(-500))
            ]
        );
        assert_eq!(expense[0].ratio(), Some(1.0));
        assert_eq!(expense[1].ratio(), None);

        // 合計が0の品目も, 取引があるので結果に残る
        let income = by_item(&book.ledger, october(), Flow::Income);
        assert_eq!(summary(&income), vec![(book.salary, Money::from(0))]);
        assert_eq!(income[0].ratio(), None);
    }

    #[test]
    fn by_item_of_offsetting_items_test() {
        let mut book = Book::new();
        book.add(date(10, 1), 1_000, book.food);
        book.add(date(10, 2), -1_000, book.fun);

        let result = by_item(&book.ledger, october(), Flow::Expense);

        // 総額が0でも, 正の品目の割合は求まる
        assert_eq!(result[0].ratio(), Some(1.0));
        assert_eq!(result[1].ratio(), None);
    }

    #[test]
    fn by_item_of_only_negative_items_test() {
        let mut book = Book::new();
        book.add(date(10, 1), -1_000, book.food);

        let result = by_item(&book.ledger, october(), Flow::Expense);

        assert_eq!(summary(&result), vec![(book.food, Money::from(-1_000))]);
        assert_eq!(result[0].ratio(), None);
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
        assert_eq!(result[0].ratio(), Some(1.0));
    }
}

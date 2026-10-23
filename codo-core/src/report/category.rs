use std::collections::BTreeMap;

use crate::item::ItemId;
use crate::ledger::Ledger;
use crate::money::Money;
use crate::period::Period;
use crate::report::Flow;
use crate::report::flow_of;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ItemExpense {
    item: ItemId,
    amount: Money,
    ratio: f64,
}

impl ItemExpense {
    pub fn item(&self) -> ItemId {
        self.item
    }

    pub fn amount(&self) -> Money {
        self.amount
    }

    pub fn ratio(&self) -> f64 {
        self.ratio
    }
}

pub fn expense_by_item(ledger: &Ledger, period: Period) -> Vec<ItemExpense> {
    let mut amounts: BTreeMap<ItemId, Money> = BTreeMap::new();

    for tx in ledger.transactions().iter() {
        // 期間外の取引は処理しない
        if !period.contains(tx.date()) {
            continue;
        }

        // 支出以外の取引は処理しない
        if flow_of(ledger, tx) != Flow::Expense {
            continue;
        }

        let item = tx.item();
        let amount = tx.amount();

        // ItemIdごとに合計を集計
        amounts
            .entry(item)
            .and_modify(|total| *total += amount)
            .or_insert(amount);
    }

    // 支出が1件もなければ空のVecを返す
    if amounts.is_empty() {
        return Vec::new();
    }

    // 期間内の支出総額
    let total = amounts
        .values()
        .copied()
        .fold(Money(0), |acc, amount| acc + amount);

    // ItemExpenseに変換
    let mut result: Vec<ItemExpense> = amounts
        .into_iter()
        .map(|(item, amount)| ItemExpense {
            item,
            amount,
            ratio: amount.0 as f64 / total.0 as f64,
        })
        .collect();

    // 金額の大きい順。
    // 同額ならItemIdの小さい順。
    result.sort_by(|a, b| b.amount.cmp(&a.amount).then_with(|| a.item.cmp(&b.item)));

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::item::{ItemList, Tag};
    use crate::ledger::Ledger;
    use crate::money::Money;
    use crate::period::Period;
    use crate::period::YearMonth;
    use crate::transaction::{TransactionDraft, TransactionList};
    use chrono::NaiveDate;

    fn date(day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, day).unwrap()
    }

    #[test]
    fn expense_by_item_calculates_ratio_and_sorts() {
        // 品目を作る
        let mut items = ItemList::new();

        let food = items.add("食費", Tag::Expense).unwrap();
        let entertainment = items.add("娯楽", Tag::Expense).unwrap();
        let cash = items.add("現金", Tag::Asset).unwrap();

        // 取引を作る
        let mut transactions = TransactionList::new();

        // 食費: 10,000円
        transactions
            .add(TransactionDraft::new(
                date(1),
                Money(10_000),
                food,
                cash,
                "食費".to_string(),
            ))
            .unwrap();

        // 娯楽: 5,000円
        transactions
            .add(TransactionDraft::new(
                date(2),
                Money(5_000),
                entertainment,
                cash,
                "娯楽".to_string(),
            ))
            .unwrap();

        // 娯楽: 5,000円
        // → 娯楽の合計は10,000円
        transactions
            .add(TransactionDraft::new(
                date(3),
                Money(5_000),
                entertainment,
                cash,
                "娯楽".to_string(),
            ))
            .unwrap();

        let ledger = Ledger::from_parts(items, transactions);

        let period = Period::new(date(1), date(31)).unwrap();

        let result = expense_by_item(&ledger, period);

        // 支出合計は20,000円
        assert_eq!(result.len(), 2);

        // 金額の大きい順
        // 食費     10,000円
        // 娯楽     10,000円
        //
        // 同額なので ItemId の小さい方が先
        assert_eq!(result[0].amount(), Money(10_000));
        assert_eq!(result[1].amount(), Money(10_000));

        assert_eq!(result[0].item(), food.min(entertainment));
        assert_eq!(result[1].item(), food.max(entertainment));

        // それぞれの割合は 10,000 / 20,000 = 0.5
        assert!((result[0].ratio() - 0.5).abs() < 1e-10);
        assert!((result[1].ratio() - 0.5).abs() < 1e-10);

        // 割合の合計は 1.0
        let ratio_sum: f64 = result.iter().map(|expense| expense.ratio()).sum();

        assert!((ratio_sum - 1.0).abs() < 1e-10);
    }

    #[test]
    fn expense_by_item_sorts_larger_amount_first() {
        let mut items = ItemList::new();

        let food = items.add("食費", Tag::Expense).unwrap();
        let entertainment = items.add("娯楽", Tag::Expense).unwrap();
        let cash = items.add("現金", Tag::Asset).unwrap();

        let mut transactions = TransactionList::new();

        // 食費: 3,000円
        transactions
            .add(TransactionDraft::new(
                date(1),
                Money(3_000),
                food,
                cash,
                "食費".to_string(),
            ))
            .unwrap();

        // 娯楽: 10,000円
        transactions
            .add(TransactionDraft::new(
                date(2),
                Money(10_000),
                entertainment,
                cash,
                "娯楽".to_string(),
            ))
            .unwrap();

        let ledger = Ledger::from_parts(items, transactions);

        let period = Period::month(YearMonth::new(2026, 10).unwrap());

        let result = expense_by_item(&ledger, period);

        // 大きい金額が先
        assert_eq!(result[0].item(), entertainment);
        assert_eq!(result[0].amount(), Money(10_000));

        assert_eq!(result[1].item(), food);
        assert_eq!(result[1].amount(), Money(3_000));
    }

    #[test]
    fn expense_by_item_returns_empty_when_there_is_no_expense() {
        let mut items = ItemList::new();

        let food = items.add("食費", Tag::Expense).unwrap();
        let cash = items.add("現金", Tag::Asset).unwrap();

        let mut transactions = TransactionList::new();

        // 期間外の支出しかない
        transactions
            .add(TransactionDraft::new(
                date(1),
                Money(10_000),
                food,
                cash,
                "食費".to_string(),
            ))
            .unwrap();

        let ledger = Ledger::from_parts(items, transactions);

        // 10月20日〜10月31日
        // 10月1日の取引は期間外
        let period = Period::new(date(20), date(31)).unwrap();

        let result = expense_by_item(&ledger, period);

        assert!(result.is_empty());
    }
}

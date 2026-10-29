//! 品目別の支出集計

use crate::item::ItemId;
use crate::ledger::Ledger;
use crate::money::Money;
use crate::period::Period;
use crate::report::Flow;
use crate::report::flow_of;
use std::collections::BTreeMap;

/// 品目ごとの支出情報
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

/// 期間内の支出を品目ごとに集計する。
/// 各品目の支出額と、全支出額に対する割合を返す。
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
        let amount = Money::from(tx.amount());

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
        .fold(Money::from(0), |acc, amount| acc + amount);

    // ItemExpenseに変換
    let mut result: Vec<ItemExpense> = amounts
        .into_iter()
        .map(|(item, amount)| ItemExpense {
            item,
            amount,
            // 支出は正の金額に限られ, 1件以上あるので総額は0にならない
            ratio: amount.ratio(total).expect("支出の総額が0になっています"),
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

    use crate::budget::BudgetList;
    use crate::item::{ItemList, Tag};
    use crate::ledger::Ledger;
    use crate::money::Money;
    use crate::period::Period;
    use crate::period::YearMonth;
    use crate::transaction::{Amount, TransactionDraft, TransactionList};
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
        transactions.add(TransactionDraft::new(
            date(1),
            Amount::of(10_000),
            food,
            cash,
            "食費".to_string(),
        ));

        // 娯楽: 5,000円
        transactions.add(TransactionDraft::new(
            date(2),
            Amount::of(5_000),
            entertainment,
            cash,
            "娯楽".to_string(),
        ));

        // 娯楽: 5,000円
        // → 娯楽の合計は10,000円
        transactions.add(TransactionDraft::new(
            date(3),
            Amount::of(5_000),
            entertainment,
            cash,
            "娯楽".to_string(),
        ));

        let ledger = Ledger::from_parts(items, transactions, BudgetList::new());

        let period = Period::new(date(1), date(31)).unwrap();

        let result = expense_by_item(&ledger, period);

        // 支出合計は20,000円
        assert_eq!(result.len(), 2);

        // 金額の大きい順
        // 食費     10,000円
        // 娯楽     10,000円
        //
        // 同額なので ItemId の小さい方が先
        assert_eq!(result[0].amount(), Money::from(10_000));
        assert_eq!(result[1].amount(), Money::from(10_000));

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
        transactions.add(TransactionDraft::new(
            date(1),
            Amount::of(3_000),
            food,
            cash,
            "食費".to_string(),
        ));

        // 娯楽: 10,000円
        transactions.add(TransactionDraft::new(
            date(2),
            Amount::of(10_000),
            entertainment,
            cash,
            "娯楽".to_string(),
        ));

        let ledger = Ledger::from_parts(items, transactions, BudgetList::new());

        let period = Period::month(YearMonth::new(2026, 10).unwrap());

        let result = expense_by_item(&ledger, period);

        // 大きい金額が先
        assert_eq!(result[0].item(), entertainment);
        assert_eq!(result[0].amount(), Money::from(10_000));

        assert_eq!(result[1].item(), food);
        assert_eq!(result[1].amount(), Money::from(3_000));

        // 割合
        assert!((result[0].ratio() - 10.0 / 13.0).abs() < 1e-10);
        assert!((result[1].ratio() - 3.0 / 13.0).abs() < 1e-10);
    }

    #[test]
    fn expense_by_item_returns_empty_when_there_is_no_expense() {
        let mut items = ItemList::new();

        let food = items.add("食費", Tag::Expense).unwrap();
        let cash = items.add("現金", Tag::Asset).unwrap();

        let mut transactions = TransactionList::new();

        // 期間外の支出しかない
        transactions.add(TransactionDraft::new(
            date(1),
            Amount::of(10_000),
            food,
            cash,
            "食費".to_string(),
        ));

        let ledger = Ledger::from_parts(items, transactions, BudgetList::new());

        // 10月20日〜10月31日
        // 10月1日の取引は期間外
        let period = Period::new(date(20), date(31)).unwrap();

        let result = expense_by_item(&ledger, period);

        assert!(result.is_empty());
    }

    #[test]
    fn expense_by_item_excludes_income_and_liability() {
        let mut items = ItemList::new();

        let food = items.add("食費", Tag::Expense).unwrap();
        let income = items.add("給与", Tag::Income).unwrap();
        let liability = items.add("借入", Tag::Liability).unwrap();
        let cash = items.add("現金", Tag::Asset).unwrap();

        let mut transactions = TransactionList::new();

        // 支出
        transactions.add(TransactionDraft::new(
            date(1),
            Amount::of(3_000),
            food,
            cash,
            "食費".to_string(),
        ));

        // 収入
        transactions.add(TransactionDraft::new(
            date(2),
            Amount::of(100_000),
            income,
            cash,
            "給与".to_string(),
        ));

        // 負債
        transactions.add(TransactionDraft::new(
            date(3),
            Amount::of(50_000),
            liability,
            cash,
            "借入".to_string(),
        ));

        let ledger = Ledger::from_parts(items, transactions, BudgetList::new());

        let period = Period::month(YearMonth::new(2026, 10).unwrap());

        let result = expense_by_item(&ledger, period);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].item(), food);
        assert_eq!(result[0].amount(), Money::from(3_000));
    }

    #[test]
    fn expense_by_item_includes_period_end_date() {
        let mut items = ItemList::new();

        let food = items.add("食費", Tag::Expense).unwrap();
        let cash = items.add("現金", Tag::Asset).unwrap();

        let mut transactions = TransactionList::new();

        // 期間開始日
        transactions.add(TransactionDraft::new(
            date(1),
            Amount::of(3_000),
            food,
            cash,
            "食費".to_string(),
        ));

        // 期間終了日
        transactions.add(TransactionDraft::new(
            date(31),
            Amount::of(2_000),
            food,
            cash,
            "食費".to_string(),
        ));

        let ledger = Ledger::from_parts(items, transactions, BudgetList::new());

        let period = Period::month(YearMonth::new(2026, 10).unwrap());

        let result = expense_by_item(&ledger, period);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].item(), food);
        assert_eq!(result[0].amount(), Money::from(5_000));
    }

    #[test]
    fn expense_by_item_excludes_expenses_outside_period() {
        let mut items = ItemList::new();

        let food = items.add("食費", Tag::Expense).unwrap();
        let cash = items.add("現金", Tag::Asset).unwrap();

        let mut transactions = TransactionList::new();

        // 期間内
        transactions.add(TransactionDraft::new(
            date(10),
            Amount::of(3_000),
            food,
            cash,
            "期間内".to_string(),
        ));

        // 期間外
        transactions.add(TransactionDraft::new(
            date(31),
            Amount::of(100_000),
            food,
            cash,
            "期間外".to_string(),
        ));

        let ledger = Ledger::from_parts(items, transactions, BudgetList::new());

        // 10月1日〜10月30日を対象
        let period = Period::new(date(1), date(30)).unwrap();

        let result = expense_by_item(&ledger, period);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].item(), food);
        assert_eq!(result[0].amount(), Money::from(3_000));
    }
}

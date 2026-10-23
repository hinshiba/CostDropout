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

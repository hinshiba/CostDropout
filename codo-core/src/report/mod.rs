pub mod calendar;
pub mod category;
pub mod monthly;
pub mod trend;
use crate::item::Tag;
use crate::ledger::Ledger;
use crate::money::Money;
use crate::period::Period;
use crate::transaction::Transaction;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flow {
    Income,
    Expense,
    Other,
}

pub fn flow_of(ledger: &Ledger, tx: &Transaction) -> Flow {
    let selected_item = ledger.items().get(tx.item()).expect("存在しない品目");

    match selected_item.tag() {
        Tag::Expense => Flow::Expense,
        Tag::Income => Flow::Income,
        Tag::Asset => Flow::Other,
        Tag::Liability => Flow::Other,
        Tag::OpeningBalance => Flow::Other,
    }
}

pub fn flows_in(ledger: &Ledger, period: Period) -> impl Iterator<Item = (&Transaction, Flow)> {
    ledger
        .transactions()
        .iter()
        .filter(move |tx| period.contains(tx.date()))
        .map(move |tx| (tx, flow_of(ledger, tx)))
}

pub fn total(ledger: &Ledger, period: Period, target_flow: Flow) -> Money {
    flows_in(ledger, period)
        .filter(|(_, flow)| *flow == target_flow)
        .map(|(tx, _)| tx.amount())
        .fold(Money::from(0), |acc, amount| acc + amount)
}

pub fn income_total(ledger: &Ledger, period: Period) -> Money {
    total(ledger, period, Flow::Income)
}

pub fn expense_total(ledger: &Ledger, period: Period) -> Money {
    total(ledger, period, Flow::Expense)
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::budget::BudgetList;
    use crate::item::ItemList;
    use crate::transaction::{TransactionDraft, TransactionList};
    use chrono::NaiveDate;

    fn create_test_ledger() -> Ledger {
        let mut items = ItemList::new();

        let expense_id = items.add("食費", Tag::Expense).unwrap();
        let income_id = items.add("給与", Tag::Income).unwrap();
        let asset_id = items.add("現金", Tag::Asset).unwrap();
        let liability_id = items.add("クレジットカード", Tag::Liability).unwrap();

        let mut transactions = TransactionList::new();

        // 支出 1000
        transactions
            .add(TransactionDraft::new(
                NaiveDate::from_ymd_opt(2026, 10, 1).unwrap(),
                Money::from(1000),
                expense_id,
                asset_id,
                String::new(),
            ))
            .unwrap();

        // 収入 5000
        transactions
            .add(TransactionDraft::new(
                NaiveDate::from_ymd_opt(2026, 10, 2).unwrap(),
                Money::from(5000),
                income_id,
                asset_id,
                String::new(),
            ))
            .unwrap();

        // 負債 3000（集計対象外）
        transactions
            .add(TransactionDraft::new(
                NaiveDate::from_ymd_opt(2026, 10, 3).unwrap(),
                Money::from(3000),
                liability_id,
                asset_id,
                String::new(),
            ))
            .unwrap();

        Ledger::from_parts(items, transactions, BudgetList::new())
    }

    #[test]
    fn totals_ignore_liability() {
        let ledger = create_test_ledger();
        let period = Period::new(
            NaiveDate::from_ymd_opt(2026, 10, 1).unwrap(),
            NaiveDate::from_ymd_opt(2026, 11, 1).unwrap(),
        )
        .unwrap();
        assert_eq!(expense_total(&ledger, period), Money::from(1000),);
        assert_eq!(income_total(&ledger, period), Money::from(5000),);
    }
}

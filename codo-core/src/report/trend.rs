//! 品目の支出推移

use crate::item::ItemId;
use crate::ledger::Ledger;
use crate::money::Money;
use crate::period::{Period, YearMonth};
use crate::report::{Flow, flow_of};

/// 月ごとの支出合計を返す
/// 品目で絞らない場合はNoneを引数とする
pub fn monthly_expense(
    ledger: &Ledger,
    period: Period,
    item: Option<ItemId>,
) -> Vec<(YearMonth, Money)> {
    period
        .months()
        .into_iter()
        .map(|month| {
            let total = ledger
                .transactions()
                .iter()
                .filter(|tx| month.contains(tx.date()))
                .filter(|tx| flow_of(ledger, tx) == Flow::Expense)
                .filter(|tx| item.is_none_or(|item_id| tx.item() == item_id))
                .fold(Money(0), |total, tx| total + tx.amount());
            (month, total)
        })
        .collect()
}

/// 年ごとの支出合計を返す
/// 品目では絞らない
pub fn yearly_expense(ledger: &Ledger) -> Vec<(i32, Money)> {
    let Some(first_year) = ledger
        .transactions()
        .iter()
        .map(|tx| YearMonth::from_date(tx.date()).year())
        .min()
    else {
        return Vec::new();
    };

    let Some(last_year) = ledger
        .transactions()
        .iter()
        .map(|tx| YearMonth::from_date(tx.date()).year())
        .max()
    else {
        return Vec::new();
    };

    (first_year..=last_year)
        .map(|year| {
            let total = ledger
                .transactions()
                .iter()
                .filter(|tx| YearMonth::from_date(tx.date()).year() == year)
                .filter(|tx| flow_of(ledger, tx) == Flow::Expense)
                .fold(Money(0), |total, tx| total + tx.amount());

            (year, total)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item::{ItemList, Tag};
    use crate::transaction::{TransactionDraft, TransactionList};
    use chrono::NaiveDate;

    fn date(day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, day).unwrap()
    }

    fn draft(day: u32, amount: i32, item: ItemId, counterpart: ItemId) -> TransactionDraft {
        TransactionDraft::new(
            date(day),
            Money(amount),
            item,
            counterpart,
            String::from("テスト"),
        )
    }

    #[test]
    fn monthly_expense_fills_zero_for_months_without_expenses() {
        let mut items = ItemList::new();

        let food = items.add("食費", Tag::Expense).unwrap();
        let cash = items.add("現金", Tag::Asset).unwrap();

        let mut transactions = TransactionList::new();

        // 10月だけ支出がある
        transactions.add(draft(10, 3000, food, cash)).unwrap();

        let ledger = Ledger::from_parts(items, transactions);

        // 2026年10月1日〜12月31日
        let period = Period::new(
            NaiveDate::from_ymd_opt(2026, 10, 1).unwrap(),
            NaiveDate::from_ymd_opt(2026, 12, 31).unwrap(),
        )
        .unwrap();

        let result = monthly_expense(&ledger, period, None);

        assert_eq!(
            result,
            vec![
                (YearMonth::new(2026, 10).unwrap(), Money(3000)),
                (YearMonth::new(2026, 11).unwrap(), Money(0)),
                (YearMonth::new(2026, 12).unwrap(), Money(0)),
            ]
        );
    }

    #[test]
    fn monthly_expense_filters_by_item() {
        let mut items = ItemList::new();

        let food = items.add("食費", Tag::Expense).unwrap();
        let entertainment = items.add("娯楽", Tag::Expense).unwrap();
        let cash = items.add("現金", Tag::Asset).unwrap();

        let mut transactions = TransactionList::new();

        transactions.add(draft(5, 3000, food, cash)).unwrap();

        transactions.add(draft(10, 4000, food, cash)).unwrap();

        transactions
            .add(draft(10, 10000, entertainment, cash))
            .unwrap();

        let ledger = Ledger::from_parts(items, transactions);

        let period = Period::new(date(1), date(31)).unwrap();

        let result = monthly_expense(&ledger, period, Some(food));

        assert_eq!(
            result,
            vec![(YearMonth::new(2026, 10).unwrap(), Money(7000)),]
        );
    }

    #[test]
    fn yearly_expense_fills_zero_for_years_without_expenses() {
        let mut items = ItemList::new();

        let food = items.add("食費", Tag::Expense).unwrap();
        let cash = items.add("現金", Tag::Asset).unwrap();

        let mut transactions = TransactionList::new();

        // 2025年：支出あり
        transactions
            .add(TransactionDraft::new(
                NaiveDate::from_ymd_opt(2025, 10, 10).unwrap(),
                Money(3000),
                food,
                cash,
                "テスト".to_string(),
            ))
            .unwrap();

        // 2027年：支出あり
        transactions
            .add(TransactionDraft::new(
                NaiveDate::from_ymd_opt(2027, 10, 10).unwrap(),
                Money(5000),
                food,
                cash,
                "テスト".to_string(),
            ))
            .unwrap();

        let ledger = Ledger::from_parts(items, transactions);

        let result = yearly_expense(&ledger);

        assert_eq!(
            result,
            vec![(2025, Money(3000)), (2026, Money(0)), (2027, Money(5000)),]
        );
    }
}

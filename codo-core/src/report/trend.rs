//! 品目の支出推移

use crate::item::ItemId;
use crate::ledger::Ledger;
use crate::money::{Money, MoneyError};
use crate::period::{Period, YearMonth};
use crate::report::{Flow, flows_in};
use chrono::Datelike;

/// 月ごとの支出合計を返す
///
/// 期間の最初と最後の月は，期間内の日の取引だけを数えること (Period の両端を含む)
/// 月の合計が i32 を超えると MoneyError::Overflow を返すこと
/// 品目で絞らない場合はNoneを引数とする
pub fn monthly_expense(
    ledger: &Ledger,
    period: Period,
    item: Option<ItemId>,
) -> Result<Vec<(YearMonth, Money)>, MoneyError> {
    period
        .months()
        .into_iter()
        .map(|month| {
            let total = flows_in(ledger, period)
                .filter(|(tx, flow)| *flow == Flow::Expense && month.contains(tx.date()))
                .filter(|(tx, _)| item.is_none_or(|item_id| tx.item() == item_id))
                .try_fold(Money::from(0), |total, (tx, _)| {
                    total.checked_add(tx.amount())
                })?;

            Ok((month, total))
        })
        .collect()
}

/// 年ごとの支出合計を返す
///
/// 支出のない年はMoney::from(0)として含める。
/// 年の合計が i32 を超えると MoneyError::Overflow を返すこと
/// 品目では絞らない
pub fn yearly_expense(ledger: &Ledger) -> Result<Vec<(i32, Money)>, MoneyError> {
    let mut transactions = ledger.transactions().iter();

    let Some(first) = transactions.next() else {
        return Ok(Vec::new());
    };

    let last = transactions.last().unwrap_or(first);

    (first.date().year()..=last.date().year())
        .map(|year| {
            let total = flows_in(ledger, Period::year(year))
                .filter(|(_, flow)| *flow == Flow::Expense)
                .try_fold(Money::from(0), |total, (tx, _)| {
                    total.checked_add(tx.amount())
                })?;

            Ok((year, total))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::budget::BudgetList;
    use crate::item::{ItemList, Tag};
    use crate::transaction::{TransactionDraft, TransactionList};
    use chrono::NaiveDate;

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    fn draft(date: NaiveDate, amount: i32, item: ItemId, counterpart: ItemId) -> TransactionDraft {
        TransactionDraft::new(
            date,
            Money::from(amount),
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
        let income = items.add("給与", Tag::Income).unwrap();
        let loan = items.add("借入金", Tag::Liability).unwrap();

        let mut transactions = TransactionList::new();

        // 10月だけ支出がある
        transactions
            .add(draft(date(2026, 10, 10), 3000, food, cash))
            .unwrap();

        // 10月に収入がある
        transactions
            .add(draft(date(2026, 10, 20), 10000, income, cash))
            .unwrap();

        // 10月に負債がある
        transactions
            .add(draft(date(2026, 10, 15), 5000, loan, cash))
            .unwrap();

        let ledger = Ledger::from_parts(items, transactions, BudgetList::new());

        // 2026年10月1日〜12月31日
        let period = Period::new(date(2026, 10, 1), date(2026, 12, 31)).unwrap();

        let result = monthly_expense(&ledger, period, None).unwrap();

        assert_eq!(
            result,
            vec![
                (YearMonth::new(2026, 10).unwrap(), Money::from(3000)),
                (YearMonth::new(2026, 11).unwrap(), Money::from(0)),
                (YearMonth::new(2026, 12).unwrap(), Money::from(0)),
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

        transactions
            .add(draft(date(2026, 10, 5), 3000, food, cash))
            .unwrap();

        transactions
            .add(draft(date(2026, 10, 10), 4000, food, cash))
            .unwrap();

        transactions
            .add(draft(date(2026, 10, 10), 10000, entertainment, cash))
            .unwrap();

        let ledger = Ledger::from_parts(items, transactions, BudgetList::new());

        let period = Period::new(date(2026, 10, 1), date(2026, 10, 31)).unwrap();

        let result = monthly_expense(&ledger, period, Some(food)).unwrap();

        assert_eq!(
            result,
            vec![(YearMonth::new(2026, 10).unwrap(), Money::from(7000)),]
        );
    }

    #[test]
    fn yearly_expense_fills_zero_for_years_without_expenses() {
        let mut items = ItemList::new();

        let food = items.add("食費", Tag::Expense).unwrap();
        let cash = items.add("現金", Tag::Asset).unwrap();
        let income = items.add("給与", Tag::Income).unwrap();
        let loan = items.add("借入金", Tag::Liability).unwrap();

        let mut transactions = TransactionList::new();

        // 2025年：支出あり
        transactions
            .add(draft(date(2025, 10, 10), 3000, food, cash))
            .unwrap();

        // 2026年：収入・負債のみ
        transactions
            .add(draft(date(2026, 10, 10), 10000, income, cash))
            .unwrap();

        transactions
            .add(draft(date(2026, 11, 10), 5000, loan, cash))
            .unwrap();

        // 2027年：支出あり
        transactions
            .add(draft(date(2027, 10, 10), 5000, food, cash))
            .unwrap();

        let ledger = Ledger::from_parts(items, transactions, BudgetList::new());

        let result = yearly_expense(&ledger).unwrap();

        assert_eq!(
            result,
            vec![
                (2025, Money::from(3000)),
                (2026, Money::from(0)),
                (2027, Money::from(5000)),
            ]
        );
    }

    #[test]
    fn monthly_expense_excludes_transactions_outside_period() {
        let mut items = ItemList::new();

        let food = items.add("食費", Tag::Expense).unwrap();
        let cash = items.add("現金", Tag::Asset).unwrap();

        let mut transactions = TransactionList::new();

        // 期間開始前なので集計されない
        transactions
            .add(draft(date(2026, 10, 5), 3000, food, cash))
            .unwrap();

        // 期間内なので集計される
        transactions
            .add(draft(date(2026, 10, 15), 5000, food, cash))
            .unwrap();

        let ledger = Ledger::from_parts(items, transactions, BudgetList::new());

        // 10月10日〜10月31日だけを期間にする
        let period = Period::new(date(2026, 10, 10), date(2026, 10, 31)).unwrap();

        let result = monthly_expense(&ledger, period, None).unwrap();

        assert_eq!(
            result,
            vec![(YearMonth::new(2026, 10).unwrap(), Money::from(5000)),]
        );
    }

    #[test]
    fn monthly_expense_returns_error_on_overflow() {
        let mut items = ItemList::new();

        let food = items.add("食費", Tag::Expense).unwrap();
        let cash = items.add("現金", Tag::Asset).unwrap();

        let mut transactions = TransactionList::new();

        transactions
            .add(draft(date(2026, 10, 10), 1_500_000_000, food, cash))
            .unwrap();

        transactions
            .add(draft(date(2026, 10, 10), 1_500_000_000, food, cash))
            .unwrap();

        let ledger = Ledger::from_parts(items, transactions, BudgetList::new());

        let period = Period::new(date(2026, 10, 1), date(2026, 10, 31)).unwrap();

        let result = monthly_expense(&ledger, period, None);

        assert_eq!(result, Err(MoneyError::Overflow));
    }

    #[test]
    fn yearly_expense_returns_error_on_overflow() {
        let mut items = ItemList::new();

        let food = items.add("食費", Tag::Expense).unwrap();
        let cash = items.add("現金", Tag::Asset).unwrap();

        let mut transactions = TransactionList::new();

        transactions
            .add(draft(date(2026, 10, 10), 1_500_000_000, food, cash))
            .unwrap();

        transactions
            .add(draft(date(2026, 10, 20), 1_500_000_000, food, cash))
            .unwrap();

        let ledger = Ledger::from_parts(items, transactions, BudgetList::new());

        let result = yearly_expense(&ledger);

        assert_eq!(result, Err(MoneyError::Overflow));
    }

    #[test]
    fn yearly_expense_returns_empty_when_there_are_no_transactions() {
        let ledger = Ledger::from_parts(ItemList::new(), TransactionList::new(), BudgetList::new());

        let result = yearly_expense(&ledger);

        assert_eq!(result, Ok(Vec::new()));
    }

    #[test]
    fn yearly_expense_returns_one_year_when_transactions_are_in_one_year() {
        let mut items = ItemList::new();

        let food = items.add("食費", Tag::Expense).unwrap();
        let cash = items.add("現金", Tag::Asset).unwrap();

        let mut transactions = TransactionList::new();

        transactions
            .add(draft(date(2026, 10, 10), 3000, food, cash))
            .unwrap();

        let ledger = Ledger::from_parts(items, transactions, BudgetList::new());

        let result = yearly_expense(&ledger);

        assert_eq!(result, Ok(vec![(2026, Money::from(3000)),]));
    }

    #[test]
    fn monthly_expense_works_across_year_boundary() {
        let mut items = ItemList::new();

        let food = items.add("食費", Tag::Expense).unwrap();
        let cash = items.add("現金", Tag::Asset).unwrap();

        let mut transactions = TransactionList::new();

        transactions
            .add(draft(date(2026, 12, 10), 3000, food, cash))
            .unwrap();

        transactions
            .add(draft(date(2027, 1, 10), 5000, food, cash))
            .unwrap();

        let ledger = Ledger::from_parts(items, transactions, BudgetList::new());

        let period = Period::new(date(2026, 12, 1), date(2027, 2, 28)).unwrap();

        let result = monthly_expense(&ledger, period, None);

        assert_eq!(
            result,
            Ok(vec![
                (YearMonth::new(2026, 12).unwrap(), Money::from(3000)),
                (YearMonth::new(2027, 1).unwrap(), Money::from(5000)),
                (YearMonth::new(2027, 2).unwrap(), Money::from(0)),
            ])
        );
    }
}

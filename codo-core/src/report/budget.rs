//! 予算の使用状況

use crate::budget::{Budget, BudgetError, BudgetId};
use crate::ledger::Ledger;
use crate::money::Money;
use crate::report::expense_total;

/// 予算の使用状況
#[derive(Debug, Clone, PartialEq)]
pub struct BudgetStatus {
    budget: Budget,
    spent: Money,
    remaining: Money,
    ratio: Option<f64>,
}

impl BudgetStatus {
    pub fn budget(&self) -> &Budget {
        &self.budget
    }

    pub fn spent(&self) -> Money {
        self.spent
    }

    pub fn remaining(&self) -> Money {
        self.remaining
    }

    pub fn ratio(&self) -> Option<f64> {
        self.ratio
    }

    /// 予算額が0円の場合は使用率を0.0とする
    pub fn calculate(ledger: &Ledger, id: BudgetId) -> Result<BudgetStatus, BudgetError> {
        // IDから予算を取得
        let budget = ledger.budgets().get(id).ok_or(BudgetError::NotFound(id))?;
        // 予算期間内の支出合計を取得
        let spent = expense_total(ledger, budget.period());
        // 残額を計算
        let remaining = budget.amount() - spent;
        // 使用率を計算
        let ratio = spent.ratio(budget.amount());
        Ok(BudgetStatus {
            budget: budget.clone(),
            spent,
            remaining,
            ratio,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item::Tag;
    use crate::period::Period;
    use crate::transaction::{Amount, TransactionDraft};
    use chrono::NaiveDate;

    fn ledger_with_budget_and_expense(
        budget_amount: i32,
        expense_amount: i32,
    ) -> (Ledger, BudgetId) {
        let mut ledger = Ledger::new();

        let period = Period::new(
            NaiveDate::from_ymd_opt(2025, 4, 1).unwrap(),
            NaiveDate::from_ymd_opt(2025, 4, 30).unwrap(),
        )
        .unwrap();

        let budget_id = ledger
            .add_budget(Money::from(budget_amount), period)
            .unwrap();

        let expense_id = ledger.add_item("食費", Tag::Expense).unwrap();
        let asset_id = ledger.add_item("現金", Tag::Asset).unwrap();

        if 0 < expense_amount {
            let draft = TransactionDraft::new(
                NaiveDate::from_ymd_opt(2025, 4, 15).unwrap(),
                Amount::of(expense_amount),
                expense_id,
                asset_id,
                String::new(),
            );

            ledger.add_transaction(draft).unwrap();
        }

        (ledger, budget_id)
    }

    /// budget_status関連のテスト
    /// 支出が予算内の場合
    #[test]
    fn budget_status_under_budget_test() {
        let (ledger, budget_id) = ledger_with_budget_and_expense(5000, 3000);

        let status = BudgetStatus::calculate(&ledger, budget_id).unwrap();

        assert_eq!(status.budget().id(), budget_id);
        assert_eq!(status.spent(), Money::from(3000));
        assert_eq!(status.remaining(), Money::from(2000));
        assert_eq!(status.ratio(), Some(0.6));
    }

    /// 支出が予算額と同じ場合
    #[test]
    fn budget_status_equal_budget_test() {
        let (ledger, budget_id) = ledger_with_budget_and_expense(5000, 5000);

        let status = BudgetStatus::calculate(&ledger, budget_id).unwrap();

        assert_eq!(status.spent(), Money::from(5000));
        assert_eq!(status.remaining(), Money::from(0));
        assert_eq!(status.ratio(), Some(1.0));
    }

    /// 支出が予算を超過した場合
    #[test]
    fn budget_status_over_budget_test() {
        let (ledger, budget_id) = ledger_with_budget_and_expense(5000, 7000);

        let status = BudgetStatus::calculate(&ledger, budget_id).unwrap();

        let ratio = status.ratio().unwrap();
        assert_eq!(status.spent(), Money::from(7000));
        assert_eq!(status.remaining(), Money::from(-2000));
        assert!((ratio - 1.4).abs() < 1e-10);
    }

    /// 支出が存在しない場合
    #[test]
    fn budget_status_no_expense_test() {
        let (ledger, _budget_id) = ledger_with_budget_and_expense(5000, 0);

        let status = BudgetStatus::calculate(&ledger, BudgetId(1)).unwrap();

        assert_eq!(status.spent(), Money::from(0));
        assert_eq!(status.remaining(), Money::from(5000));
        assert_eq!(status.ratio(), Some(0.0));
    }

    /// 予算額が0円の場合
    #[test]
    fn budget_status_zero_budget_test() {
        let (ledger, budget_id) = ledger_with_budget_and_expense(0, 0);

        let status = BudgetStatus::calculate(&ledger, budget_id).unwrap();

        assert_eq!(status.spent(), Money::from(0));
        assert_eq!(status.remaining(), Money::from(0));
        assert_eq!(status.ratio(), None);
    }

    /// 存在しない予算IDを指定した場合
    #[test]
    fn budget_status_not_found_test() {
        let ledger = Ledger::new();
        let missing_id = BudgetId(5);

        assert_eq!(
            BudgetStatus::calculate(&ledger, missing_id),
            Err(BudgetError::NotFound(missing_id))
        );
    }

    #[test]
    fn budget_status_zero_budget_with_expense_test() {
        let (ledger, budget_id) = ledger_with_budget_and_expense(0, 3000);

        let status = BudgetStatus::calculate(&ledger, budget_id).unwrap();

        assert_eq!(status.spent(), Money::from(3000));
        assert_eq!(status.remaining(), Money::from(-3000));
        assert_eq!(status.ratio(), None);
    }
}

use crate::ledger::Ledger;
use crate::money::Money;
use crate::period::Period;
use thiserror::Error;

/// 予算を識別するID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BudgetId(pub u32);

/// 予算
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Budget {
    id: BudgetId,
    amount: Money,
    period: Period,
}

/// 複数の予算を管理する
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BudgetList {
    budgets: Vec<Budget>,
}

/// 予算の使用状況
#[derive(Debug, Clone, PartialEq)]
pub struct BudgetStatus {
    pub budget: Budget,
    pub spent: Money,
    pub remaining: Money,
    pub ratio: f64,
}

/// 予算操作で発生するエラー
#[derive(Debug, Error, PartialEq, Eq)]
pub enum BudgetError {
    #[error("同じ期間の予算がすでに存在します")]
    DuplicatePeriod,

    #[error("予算IDが上限に達しました")]
    IdOverflow,

    #[error("指定された予算が存在しません: {0:?}")]
    NotFound(BudgetId),
}

impl Budget {
    pub fn new(id: BudgetId, amount: Money, period: Period) -> Self {
        Self { id, amount, period }
    }

    pub fn id(&self) -> BudgetId {
        self.id
    }

    pub fn amount(&self) -> Money {
        self.amount
    }

    pub fn period(&self) -> Period {
        self.period
    }
}

impl BudgetList {
    pub fn new() -> Self {
        Self {
            budgets: Vec::new(),
        }
    }

    pub fn get(&self, id: BudgetId) -> Option<&Budget> {
        self.budgets.iter().find(|budget| budget.id == id)
    }

    pub fn add(&mut self, amount: Money, period: Period) -> Result<BudgetId, BudgetError> {
        if self.budgets.iter().any(|budget| budget.period == period) {
            return Err(BudgetError::DuplicatePeriod);
        }
        let max_id = self
            .budgets
            .iter()
            .map(|budget| budget.id().0)
            .max()
            .unwrap_or(0);

        let new_id = max_id.checked_add(1).ok_or(BudgetError::IdOverflow)?;
        self.budgets
            .push(Budget::new(BudgetId(new_id), amount, period));
        Ok(BudgetId(new_id))
    }

    pub fn update(
        &mut self,
        id: BudgetId,
        amount: Money,
        period: Period,
    ) -> Result<BudgetId, BudgetError> {
        if self.get(id).is_none() {
            return Err(BudgetError::NotFound(id));
        }
        if self
            .budgets
            .iter()
            .any(|budget| budget.period() == period && budget.id() != id)
        {
            return Err(BudgetError::DuplicatePeriod);
        }
        if let Some(budget) = self.budgets.iter_mut().find(|budget| budget.id == id) {
            budget.amount = amount;
            budget.period = period;
            Ok(id)
        } else {
            Err(BudgetError::NotFound(id))
        }
    }

    pub fn remove(&mut self, id: BudgetId) -> Result<(), BudgetError> {
        if self.get(id).is_none() {
            return Err(BudgetError::NotFound(id));
        }

        self.budgets.retain(|budget| budget.id != id);

        Ok(())
    }
}

impl BudgetStatus {
    /// 予算額が0円の場合は使用率を0.0とする
    pub fn calculate(ledger: &Ledger, id: BudgetId) -> Result<BudgetStatus, BudgetError> {
        // IDから予算を取得
        let budget = ledger.budgets().get(id).ok_or(BudgetError::NotFound(id))?;
        // 予算期間内の支出合計を取得
        let spent = crate::report::expense_total(ledger, budget.period());
        // 残額を計算
        let remaining = budget.amount() - spent;
        // 使用率を計算
        let spent_value = spent.0 as f64;
        let budget_value = budget.amount().0 as f64;
        let ratio = if budget_value == 0.0 {
            0.0
        } else {
            spent_value / budget_value
        };
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
    use crate::transaction::TransactionDraft;
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

        let budget_id = ledger.add_budget(Money(budget_amount), period).unwrap();

        let expense_id = ledger.add_item("食費", Tag::Expense).unwrap();
        let asset_id = ledger.add_item("現金", Tag::Asset).unwrap();

        if expense_amount > 0 {
            let draft = TransactionDraft::new(
                NaiveDate::from_ymd_opt(2025, 4, 15).unwrap(),
                Money(expense_amount),
                expense_id,
                asset_id,
                String::new(),
            );

            ledger.add_transaction(draft).unwrap();
        }

        (ledger, budget_id)
    }

    #[test]
    fn add_test() {
        let mut budgets = BudgetList::new();
        let start = NaiveDate::from_ymd_opt(2025, 4, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2025, 4, 30).unwrap();
        let period = Period::new(start, end).unwrap();

        let id = budgets.add(Money(5000), period).unwrap();

        assert_eq!(id, BudgetId(1));
        let budget = budgets.get(BudgetId(1)).unwrap();

        assert_eq!(budget.amount(), Money(5000));
    }

    #[test]
    fn duplicate_period_test() {
        let mut budgets = BudgetList::new();
        let start = NaiveDate::from_ymd_opt(2025, 4, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2025, 4, 30).unwrap();
        let period = Period::new(start, end).unwrap();

        let first_outcome = budgets.add(Money(5000), period);
        assert_eq!(first_outcome, Ok(BudgetId(1)));

        let second_outcome = budgets.add(Money(7000), period);
        assert_eq!(second_outcome, Err(BudgetError::DuplicatePeriod));
    }

    #[test]
    fn update_test() {
        let mut budgets = BudgetList::new();
        let start = NaiveDate::from_ymd_opt(2025, 4, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2025, 4, 30).unwrap();
        let period = Period::new(start, end).unwrap();

        let target_id = budgets.add(Money(5000), period).unwrap();

        let update_budget_id = budgets.update(target_id, Money(7000), period).unwrap();
        let update_budget = budgets.get(update_budget_id).unwrap();

        assert_eq!(update_budget.amount(), Money(7000));
    }

    #[test]
    fn update_duplicate_period_test() {
        let mut budgets = BudgetList::new();
        // 1つ目の追加
        let start = NaiveDate::from_ymd_opt(2025, 4, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2025, 4, 30).unwrap();
        let period = Period::new(start, end).unwrap();

        let first_outcome = budgets.add(Money(5000), period);
        assert_eq!(first_outcome, Ok(BudgetId(1)));

        // 2つ目の追加
        let update_start = NaiveDate::from_ymd_opt(2026, 4, 1).unwrap();
        let update_end = NaiveDate::from_ymd_opt(2026, 4, 30).unwrap();
        let update_period = Period::new(update_start, update_end).unwrap();
        let second_outcome = budgets.add(Money(5000), update_period);
        assert_eq!(second_outcome, Ok(BudgetId(2)));

        // 1つ目の変更
        let deplicate_outcome = budgets.update(BudgetId(1), Money(7000), update_period);
        assert_eq!(deplicate_outcome, Err(BudgetError::DuplicatePeriod));

        // 1つ目の内容が変更されていないことを確認
        let budget = budgets.get(BudgetId(1)).unwrap();
        assert_eq!(budget.amount(), Money(5000));
        assert_eq!(budget.period(), period);
    }

    #[test]
    fn update_not_found_test() {
        let mut budgets = BudgetList::new();
        // 1つ目の追加
        let start = NaiveDate::from_ymd_opt(2025, 4, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2025, 4, 30).unwrap();
        let period = Period::new(start, end).unwrap();

        let first_outcome = budgets.add(Money(5000), period);
        assert_eq!(first_outcome, Ok(BudgetId(1)));

        // 不正なIdの変更
        let deplicate_outcome = budgets.update(BudgetId(2), Money(7000), period);
        assert_eq!(deplicate_outcome, Err(BudgetError::NotFound(BudgetId(2))));

        // 1つ目の内容が変更されていないことを確認
        let budget = budgets.get(BudgetId(1)).unwrap();
        assert_eq!(budget.amount(), Money(5000));
        assert_eq!(budget.period(), period);
    }

    #[test]
    fn remove_test() {
        let mut budgets = BudgetList::new();
        let start = NaiveDate::from_ymd_opt(2025, 4, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2025, 4, 30).unwrap();
        let period = Period::new(start, end).unwrap();
        let period2 = Period::new(
            NaiveDate::from_ymd_opt(2025, 5, 1).unwrap(),
            NaiveDate::from_ymd_opt(2025, 5, 31).unwrap(),
        )
        .unwrap();

        let period3 = Period::new(
            NaiveDate::from_ymd_opt(2025, 6, 1).unwrap(),
            NaiveDate::from_ymd_opt(2025, 6, 30).unwrap(),
        )
        .unwrap();
        // 予算の追加
        let add_outcome = budgets.add(Money(5000), period);
        assert_eq!(add_outcome, Ok(BudgetId(1)));
        // いくつか予算を追加する
        assert_eq!(budgets.add(Money(1000), period2), Ok(BudgetId(2)));
        assert_eq!(budgets.add(Money(2000), period3), Ok(BudgetId(3)));
        // 削除
        let remove_outcome = budgets.remove(BudgetId(2));
        assert_eq!(remove_outcome, Ok(()));
        // 対象となっているidの予算のみが削除されていることを確認する
        assert!(budgets.get(BudgetId(1)).is_some());
        assert!(budgets.get(BudgetId(2)).is_none());
        assert!(budgets.get(BudgetId(3)).is_some());
        assert_eq!(budgets.budgets.len(), 2);
    }

    #[test]
    fn remove_not_found_test() {
        let mut budgets = BudgetList::new();
        let start = NaiveDate::from_ymd_opt(2025, 4, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2025, 4, 30).unwrap();
        let period = Period::new(start, end).unwrap();
        // 予算の追加
        let add_outcome = budgets.add(Money(5000), period);
        assert_eq!(add_outcome, Ok(BudgetId(1)));
        // 削除
        let remove_outcome = budgets.remove(BudgetId(5));
        assert_eq!(remove_outcome, Err(BudgetError::NotFound(BudgetId(5))));
        assert_eq!(budgets.budgets.len(), 1);
        assert_eq!(budgets.get(BudgetId(1)).unwrap().amount(), Money(5000));
    }

    // budget_status関連のテスト

    /// 支出が予算内の場合
    #[test]
    fn budget_status_under_budget_test() {
        let (ledger, budget_id) = ledger_with_budget_and_expense(5000, 3000);

        let status = BudgetStatus::calculate(&ledger, budget_id).unwrap();

        assert_eq!(status.budget.id(), budget_id);
        assert_eq!(status.spent, Money(3000));
        assert_eq!(status.remaining, Money(2000));
        assert_eq!(status.ratio, 0.6);
    }

    /// 支出が予算額と同じ場合
    #[test]
    fn budget_status_equal_budget_test() {
        let (ledger, budget_id) = ledger_with_budget_and_expense(5000, 5000);

        let status = BudgetStatus::calculate(&ledger, budget_id).unwrap();

        assert_eq!(status.spent, Money(5000));
        assert_eq!(status.remaining, Money(0));
        assert_eq!(status.ratio, 1.0);
    }

    /// 支出が予算を超過した場合
    #[test]
    fn budget_status_over_budget_test() {
        let (ledger, budget_id) = ledger_with_budget_and_expense(5000, 7000);

        let status = BudgetStatus::calculate(&ledger, budget_id).unwrap();

        assert_eq!(status.spent, Money(7000));
        assert_eq!(status.remaining, Money(-2000));
        assert!((status.ratio - 1.4).abs() < 1e-10);
    }

    /// 支出が存在しない場合
    #[test]
    fn budget_status_no_expense_test() {
        let ledger = {
            let mut ledger = Ledger::new();

            let period = Period::new(
                NaiveDate::from_ymd_opt(2025, 4, 1).unwrap(),
                NaiveDate::from_ymd_opt(2025, 4, 30).unwrap(),
            )
            .unwrap();

            ledger.add_budget(Money(5000), period).unwrap();
            ledger
        };

        let status = BudgetStatus::calculate(&ledger, BudgetId(1)).unwrap();

        assert_eq!(status.spent, Money(0));
        assert_eq!(status.remaining, Money(5000));
        assert_eq!(status.ratio, 0.0);
    }

    /// 予算額が0円の場合
    #[test]
    fn budget_status_zero_budget_test() {
        let (ledger, budget_id) = ledger_with_budget_and_expense(0, 0);

        let status = BudgetStatus::calculate(&ledger, budget_id).unwrap();

        assert_eq!(status.spent, Money(0));
        assert_eq!(status.remaining, Money(0));
        assert_eq!(status.ratio, 0.0);
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
}

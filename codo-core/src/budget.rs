use crate::budget;
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
            return Err(BudgetError::NotFound(id));
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

impl BudgetStatus {}

#[cfg(test)]
mod tests {
    use crate::{
        budget::{self, Budget, BudgetError, BudgetId, BudgetList},
        money::Money,
        period::{self, Period},
    };
    use chrono::NaiveDate;

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
        // 予算の追加
        let add_outcome = budgets.add(Money(5000), period);
        assert_eq!(add_outcome, Ok(BudgetId(1)));
        // いくつか予算を追加する
        budgets.add(Money(1000), period);
        budgets.add(Money(2000), period);
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
}

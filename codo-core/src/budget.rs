use crate::money::Money;
use crate::period::Period;
use std::collections::HashSet;
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

/// 予算操作で発生するエラー
#[derive(Debug, Error, PartialEq, Eq)]
pub enum BudgetError {
    #[error("同じ期間の予算がすでに存在します")]
    DuplicatePeriod,

    #[error("予算IDが上限に達しました")]
    IdOverflow,

    #[error("指定された予算が存在しません: {0:?}")]
    NotFound(BudgetId),

    #[error("予算IDが重複しています: {0:?}")]
    DuplicateId(BudgetId),
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

    /// 予算の追加
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

    /// 既存の予算の編集
    pub fn update(
        &mut self,
        id: BudgetId,
        amount: Money,
        period: Period,
    ) -> Result<(), BudgetError> {
        let Some(index) = self.budgets.iter().position(|budget| budget.id == id) else {
            return Err(BudgetError::NotFound(id));
        };

        if self
            .budgets
            .iter()
            .any(|budget| budget.period() == period && budget.id() != id)
        {
            return Err(BudgetError::DuplicatePeriod);
        }

        self.budgets[index].amount = amount;
        self.budgets[index].period = period;

        Ok(())
    }

    /// 既存の予算の削除
    pub fn remove(&mut self, id: BudgetId) -> Result<(), BudgetError> {
        if self.get(id).is_none() {
            return Err(BudgetError::NotFound(id));
        }

        self.budgets.retain(|budget| budget.id != id);

        Ok(())
    }

    /// 保存済みのIDを変更せずに予算一覧を復元する
    /// IDの重複を検知した場合はエラーを返す
    ///
    /// IDまたは期間が重複している場合はエラーを返す
    pub fn from_budgets(mut budgets: Vec<Budget>) -> Result<Self, BudgetError> {
        let mut ids = HashSet::new();
        let mut periods = HashSet::new();

        for budget in &budgets {
            if !periods.insert(budget.period()) {
                return Err(BudgetError::DuplicatePeriod);
            }
            if !ids.insert(budget.id()) {
                return Err(BudgetError::DuplicateId(budget.id()));
            }
        }
        // IDの昇順に並び替える
        budgets.sort_by_key(|budget| budget.id().0);

        Ok(Self { budgets })
    }

    pub fn iter(&self) -> impl Iterator<Item = &Budget> {
        self.budgets.iter()
    }
}

impl BudgetId {
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    pub fn value(&self) -> u32 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::period::YearMonth;
    use chrono::NaiveDate;

    #[test]
    fn add_test() {
        let mut budgets = BudgetList::new();
        let start = NaiveDate::from_ymd_opt(2025, 4, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2025, 4, 30).unwrap();
        let period = Period::new(start, end).unwrap();

        let id = budgets.add(Money::from(5000), period).unwrap();

        assert_eq!(id, BudgetId(1));
        let budget = budgets.get(BudgetId(1)).unwrap();

        assert_eq!(budget.amount(), Money::from(5000));
    }

    #[test]
    fn duplicate_period_test() {
        let mut budgets = BudgetList::new();
        let start = NaiveDate::from_ymd_opt(2025, 4, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2025, 4, 30).unwrap();
        let period = Period::new(start, end).unwrap();

        let first_outcome = budgets.add(Money::from(5000), period);
        assert_eq!(first_outcome, Ok(BudgetId(1)));

        let second_outcome = budgets.add(Money::from(7000), period);
        assert_eq!(second_outcome, Err(BudgetError::DuplicatePeriod));
    }

    #[test]
    fn update_test() {
        let mut budgets = BudgetList::new();
        let start = NaiveDate::from_ymd_opt(2025, 4, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2025, 4, 30).unwrap();
        let period = Period::new(start, end).unwrap();

        let target_id = budgets.add(Money::from(5000), period).unwrap();

        let update_budget = budgets.update(target_id, Money::from(7000), period);
        let budget = budgets.get(target_id).unwrap();
        assert_eq!(budget.amount(), Money::from(7000));
        assert_eq!(budget.period(), period);

        assert_eq!(update_budget, Ok(()));
    }

    #[test]
    fn update_duplicate_period_test() {
        let mut budgets = BudgetList::new();
        // 1つ目の追加
        let start = NaiveDate::from_ymd_opt(2025, 4, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2025, 4, 30).unwrap();
        let period = Period::new(start, end).unwrap();

        let first_outcome = budgets.add(Money::from(5000), period);
        assert_eq!(first_outcome, Ok(BudgetId(1)));

        // 2つ目の追加
        let update_start = NaiveDate::from_ymd_opt(2026, 4, 1).unwrap();
        let update_end = NaiveDate::from_ymd_opt(2026, 4, 30).unwrap();
        let update_period = Period::new(update_start, update_end).unwrap();
        let second_outcome = budgets.add(Money::from(5000), update_period);
        assert_eq!(second_outcome, Ok(BudgetId(2)));

        // 1つ目の変更
        let duplicate_outcome = budgets.update(BudgetId(1), Money::from(7000), update_period);
        assert_eq!(duplicate_outcome, Err(BudgetError::DuplicatePeriod));

        // 1つ目の内容が変更されていないことを確認
        let budget = budgets.get(BudgetId(1)).unwrap();
        assert_eq!(budget.amount(), Money::from(5000));
        assert_eq!(budget.period(), period);
    }

    #[test]
    fn update_not_found_test() {
        let mut budgets = BudgetList::new();
        // 1つ目の追加
        let start = NaiveDate::from_ymd_opt(2025, 4, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2025, 4, 30).unwrap();
        let period = Period::new(start, end).unwrap();

        let first_outcome = budgets.add(Money::from(5000), period);
        assert_eq!(first_outcome, Ok(BudgetId(1)));

        // 不正なIdの変更
        let not_found_outcome = budgets.update(BudgetId(2), Money::from(7000), period);
        assert_eq!(not_found_outcome, Err(BudgetError::NotFound(BudgetId(2))));

        // 1つ目の内容が変更されていないことを確認
        let budget = budgets.get(BudgetId(1)).unwrap();
        assert_eq!(budget.amount(), Money::from(5000));
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
        let add_outcome = budgets.add(Money::from(5000), period);
        assert_eq!(add_outcome, Ok(BudgetId(1)));
        // いくつか予算を追加する
        assert_eq!(budgets.add(Money::from(1000), period2), Ok(BudgetId(2)));
        assert_eq!(budgets.add(Money::from(2000), period3), Ok(BudgetId(3)));
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
        let add_outcome = budgets.add(Money::from(5000), period);
        assert_eq!(add_outcome, Ok(BudgetId(1)));
        // 削除
        let remove_outcome = budgets.remove(BudgetId(5));
        assert_eq!(remove_outcome, Err(BudgetError::NotFound(BudgetId(5))));
        assert_eq!(budgets.budgets.len(), 1);
        assert_eq!(
            budgets.get(BudgetId(1)).unwrap().amount(),
            Money::from(5000)
        );
    }

    /// IDが重複した場合のテスト
    #[test]
    fn from_budgets_duplicate_id_test() {
        let period1 = Period::month(YearMonth::new(2026, 10).unwrap());
        let period2 = Period::month(YearMonth::new(2026, 11).unwrap());

        let budget1 = Budget::new(BudgetId(1), Money::from(50_000), period1);
        let budget2 = Budget::new(BudgetId(1), Money::from(30_000), period2);

        let result = BudgetList::from_budgets(vec![budget1, budget2]);

        assert_eq!(result, Err(BudgetError::DuplicateId(BudgetId(1))));
    }

    #[test]
    fn from_budgets_duplicate_period_test() {
        let period = Period::new(
            NaiveDate::from_ymd_opt(2026, 10, 1).unwrap(),
            NaiveDate::from_ymd_opt(2026, 10, 31).unwrap(),
        )
        .unwrap();

        let budget1 = Budget::new(BudgetId(1), Money::from(50_000), period);
        let budget2 = Budget::new(BudgetId(2), Money::from(30_000), period);

        let result = BudgetList::from_budgets(vec![budget1, budget2]);

        assert_eq!(result, Err(BudgetError::DuplicatePeriod));
    }

    #[test]
    fn add_id_overflow_test() {
        let period1 = Period::month(YearMonth::new(2026, 10).unwrap());

        let period2 = Period::month(YearMonth::new(2026, 11).unwrap());

        let max_budget = Budget::new(BudgetId(u32::MAX), Money::from(50_000), period1);

        let mut budgets = BudgetList::from_budgets(vec![max_budget]).unwrap();

        let result = budgets.add(Money::from(30_000), period2);

        assert_eq!(result, Err(BudgetError::IdOverflow));

        assert_eq!(budgets.iter().count(), 1);
        assert!(budgets.get(BudgetId(u32::MAX)).is_some());
    }

    #[test]
    fn budget_id_value_test() {
        let id = BudgetId::new(42);

        assert_eq!(id.value(), 42);
    }
}

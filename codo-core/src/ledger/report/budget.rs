//! 予算の使用状況
use crate::ledger::Ledger;
use crate::ledger::budget::{Budget, BudgetError, BudgetId};
use crate::ledger::money::Money;
use crate::ledger::report::expense_total;

/// 予算の使用状況.
#[derive(Debug, Clone, PartialEq)]
pub struct BudgetStatus {
    budget: Budget,
    spent: Money,
    remaining: Money,
    ratio: Option<f64>,
}

impl BudgetStatus {
    /// 対象の予算を返す.
    pub fn budget(&self) -> &Budget {
        &self.budget
    }

    /// 予算の期間内の支出の合計を返す.
    pub fn spent(&self) -> Money {
        self.spent
    }

    /// 予算額から支出を引いた残額を返す. 超過した場合は負の値.
    pub fn remaining(&self) -> Money {
        self.remaining
    }

    /// 予算額に対する支出の割合を返す.
    ///
    /// 予算額が0円の場合は `None`.
    pub fn ratio(&self) -> Option<f64> {
        self.ratio
    }

    /// ID が `id` である予算の使用状況を求める.
    ///
    /// # Errors
    ///
    /// 予算が存在しない場合は `BudgetError::NotFound` エラー.
    ///
    /// # Panics
    ///
    /// 支出の合計または残額が `i32` の範囲を超える場合はパニック.
    pub fn calculate(ledger: &Ledger, id: BudgetId) -> Result<BudgetStatus, BudgetError> {
        let budget = ledger.budgets().get(id).ok_or(BudgetError::NotFound(id))?;
        let spent = expense_total(ledger, budget.period());
        Ok(BudgetStatus {
            budget: budget.clone(),
            spent,
            remaining: budget.amount() - spent,
            ratio: spent.ratio(budget.amount()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ledger::period::{Period, YearMonth};
    use crate::ledger::report::fixture::{Book, date};

    /// 10月の予算 `budget` 円と, 10月の支出 `expenses` をもつ帳簿.
    fn ledger(budget: i32, expenses: &[i32]) -> (Ledger, BudgetId) {
        let mut book = Book::new();
        let october = Period::month(YearMonth::new(2026, 10).unwrap());
        let id = book
            .ledger
            .add_budget(Money::from(budget), october)
            .unwrap();
        for &amount in expenses {
            book.add(date(10, 15), amount, book.food);
        }
        // 期間外の支出と収入は数えない
        book.add(date(9, 30), 1, book.food);
        book.add(date(10, 15), 1, book.salary);
        (book.ledger, id)
    }

    #[test]
    fn calculate_test() {
        for (budget, expenses, spent, remaining, ratio) in [
            (5000, &[1000, 2000][..], 3000, 2000, Some(0.6)),
            (5000, &[5000], 5000, 0, Some(1.0)),
            (5000, &[7000], 7000, -2000, Some(1.4)),
            (5000, &[], 0, 5000, Some(0.0)),
            (0, &[], 0, 0, None),
            (0, &[3000], 3000, -3000, None),
        ] {
            let (ledger, id) = ledger(budget, expenses);

            let status = BudgetStatus::calculate(&ledger, id).unwrap();

            assert_eq!(status.budget().id(), id);
            assert_eq!(status.spent(), Money::from(spent), "{budget} {expenses:?}");
            assert_eq!(
                status.remaining(),
                Money::from(remaining),
                "{budget} {expenses:?}"
            );
            match (status.ratio(), ratio) {
                (Some(actual), Some(expected)) => {
                    assert!((actual - expected).abs() < 1e-10, "{budget} {expenses:?}")
                }
                (actual, expected) => assert_eq!(actual, expected, "{budget} {expenses:?}"),
            }
        }
    }

    #[test]
    fn calculate_error_test() {
        let missing = BudgetId::new(5);

        assert_eq!(
            BudgetStatus::calculate(&Ledger::new(), missing),
            Err(BudgetError::NotFound(missing))
        );
    }
}

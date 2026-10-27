//! 月の収支と日別内訳の集計

use chrono::NaiveDate;

use crate::money::Money;
use crate::period::YearMonth;

/// 1日分の収支
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DailySummary {
    date: NaiveDate,
    income: Money,
    expense: Money,
}

impl DailySummary {
    /// 日付
    pub fn date(&self) -> NaiveDate {
        self.date
    }

    /// その日の収入
    pub fn income(&self) -> Money {
        self.income
    }

    /// その日の支出
    pub fn expense(&self) -> Money {
        self.expense
    }
}

/// 一か月分の収支
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonthlySummary {
    month: YearMonth,
    income: Money,
    expense: Money,
    days: Vec<DailySummary>,
}

impl MonthlySummary {
    /// 月
    pub fn month(&self) -> YearMonth {
        self.month
    }

    /// 月の収入
    pub fn income(&self) -> Money {
        self.income
    }

    /// 月の支出
    pub fn expense(&self) -> Money {
        self.expense
    }

    /// 1日から末日までの日ごとの収支
    pub fn days(&self) -> &[DailySummary] {
        &self.days
    }
}

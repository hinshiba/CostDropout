//! 月の収支と日別内訳の集計

use chrono::{Datelike, NaiveDate};

use crate::ledger::Ledger;
use crate::money::Money;
use crate::period::{Period, YearMonth};
use crate::report::{Flow, expense_total, flows_in, income_total};

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

/// 1か月の収支と，1日から末日までの日ごとの内訳を集計する
///
/// 取引がない日も0として含める
pub fn monthly_summary(ledger: &Ledger, ym: YearMonth) -> MonthlySummary {
    let mut days = Vec::new();
    for day in 1..=ym.days_in_month() {
        let date = NaiveDate::from_ymd_opt(ym.year(), ym.month(), day)
            .expect("YearMonthの日付は常に正しい");
        days.push(DailySummary {
            date,
            income: Money(0),
            expense: Money(0),
        });
    }
    let period = Period::month(ym);
    for (tx, flow) in flows_in(ledger, period) {
        let index = (tx.date().day() - 1) as usize;
        match flow {
            Flow::Income => days[index].income += tx.amount(),
            Flow::Expense => days[index].expense += tx.amount(),
            Flow::Other => {}
        }
    }
    let income = income_total(ledger, period);
    let expense = expense_total(ledger, period);
    MonthlySummary {
        month: ym,
        income,
        expense,
        days,
    }
}

//! 月の収支と日別内訳の集計

use chrono::{Datelike, NaiveDate};

use crate::ledger::Ledger;
use crate::ledger::money::Money;
use crate::ledger::period::{Period, YearMonth};
use crate::ledger::report::{Flow, flows_in};

/// 1日分の収支.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DailySummary {
    date: NaiveDate,
    income: Money,
    expense: Money,
}

impl DailySummary {
    /// 日付を返す.
    pub fn date(&self) -> NaiveDate {
        self.date
    }

    /// その日の収入を返す.
    pub fn income(&self) -> Money {
        self.income
    }

    /// その日の支出を返す.
    pub fn expense(&self) -> Money {
        self.expense
    }
}

/// 1か月分の収支.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonthlySummary {
    month: YearMonth,
    income: Money,
    expense: Money,
    days: Vec<DailySummary>,
}

impl MonthlySummary {
    /// 集計した年月を返す.
    pub fn month(&self) -> YearMonth {
        self.month
    }

    /// 月の収入を返す.
    pub fn income(&self) -> Money {
        self.income
    }

    /// 月の支出を返す.
    pub fn expense(&self) -> Money {
        self.expense
    }

    /// 1日から末日までの日ごとの収支を返す.
    pub fn days(&self) -> &[DailySummary] {
        &self.days
    }
}

/// 1か月の収支と, 1日から末日までの日ごとの内訳を集計する.
///
/// 取引がない日も0として含める.
///
/// # Panics
///
/// 合計が `i32` の範囲を超える場合はパニック.
pub fn monthly_summary(ledger: &Ledger, ym: YearMonth) -> MonthlySummary {
    let period = Period::month(ym);

    // 1日から末日までの収入と支出が0の日を並べる
    let mut days: Vec<DailySummary> = period
        .days()
        .map(|date| DailySummary {
            date,
            income: Money::from(0),
            expense: Money::from(0),
        })
        .collect();

    // 月内の取引の金額を, その日に足す
    for (tx, flow) in flows_in(ledger, period) {
        let index = tx.date().day0() as usize;
        match flow {
            Flow::Income => days[index].income += Money::from(tx.amount()),
            Flow::Expense => days[index].expense += Money::from(tx.amount()),
            Flow::Other => {}
        }
    }

    let income = days.iter().map(|day| day.income).sum();
    let expense = days.iter().map(|day| day.expense).sum();
    MonthlySummary {
        month: ym,
        income,
        expense,
        days,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ledger::report::fixture::Book;

    /// テスト用の年月
    fn ym(year: i32, month: u32) -> YearMonth {
        YearMonth::new(year, month).unwrap()
    }

    /// テスト用の日付(2026年の指定した月日)
    fn date(month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, month, day).unwrap()
    }

    /// 2026年10月を中心に, 支出, 収入, 負債と前後の月の取引をもつ帳簿.
    ///
    /// 10月の収入は 5000, 支出は 1000 + 500 + 700 = 2200.
    fn sample_ledger() -> Ledger {
        let mut book = Book::new();
        // 月の初日に2件
        book.add(date(10, 1), 1000, book.food);
        book.add(date(10, 1), 500, book.food);
        book.add(date(10, 15), 5000, book.salary);
        // 負債は収入にも支出にも数えない
        book.add(date(10, 20), 3000, book.loan);
        // 月の末日
        book.add(date(10, 31), 700, book.food);
        // 前後の月の取引は含めない
        book.add(date(9, 30), 9999, book.food);
        book.add(date(11, 1), 8888, book.salary);
        book.ledger
    }

    #[test]
    fn monthly_summary_has_every_day_of_month() {
        let summary = monthly_summary(&sample_ledger(), ym(2026, 10));

        assert_eq!(summary.month(), ym(2026, 10));
        assert_eq!(summary.days().len(), 31);
        for (i, day) in summary.days().iter().enumerate() {
            assert_eq!(day.date(), date(10, i as u32 + 1));
        }
    }

    #[test]
    fn monthly_summary_has_28_or_29_days_in_february() {
        let ledger = Ledger::new();

        assert_eq!(monthly_summary(&ledger, ym(2026, 2)).days().len(), 28);
        // 2028年はうるう年
        assert_eq!(monthly_summary(&ledger, ym(2028, 2)).days().len(), 29);
    }

    #[test]
    fn monthly_summary_adds_amounts_to_each_day() {
        let summary = monthly_summary(&sample_ledger(), ym(2026, 10));
        let day = |d: usize| summary.days()[d - 1];

        assert_eq!(day(1).expense(), Money::from(1500));
        assert_eq!(day(1).income(), Money::from(0));
        assert_eq!(day(15).income(), Money::from(5000));
        assert_eq!(day(20).income(), Money::from(0));
        assert_eq!(day(20).expense(), Money::from(0));
        assert_eq!(day(31).expense(), Money::from(700));
        // 取引の無い日は 0
        assert_eq!(day(2).income(), Money::from(0));
        assert_eq!(day(2).expense(), Money::from(0));
    }

    #[test]
    fn monthly_summary_totals_match_sum_of_days() {
        let summary = monthly_summary(&sample_ledger(), ym(2026, 10));

        let income: Money = summary.days().iter().map(|day| day.income()).sum();
        let expense: Money = summary.days().iter().map(|day| day.expense()).sum();

        assert_eq!(summary.income(), Money::from(5000));
        assert_eq!(summary.expense(), Money::from(2200));
        assert_eq!(income, summary.income());
        assert_eq!(expense, summary.expense());
    }

    #[test]
    fn monthly_summary_of_month_without_transactions_is_all_zero() {
        let summary = monthly_summary(&sample_ledger(), ym(2026, 12));

        assert_eq!(summary.income(), Money::from(0));
        assert_eq!(summary.expense(), Money::from(0));
        assert_eq!(summary.days().len(), 31);
        for day in summary.days() {
            assert_eq!(day.income(), Money::from(0));
            assert_eq!(day.expense(), Money::from(0));
        }
    }

    #[test]
    #[should_panic]
    fn monthly_summary_overflow_test() {
        let mut book = Book::new();
        book.add(date(10, 1), i32::MAX, book.food);
        book.add(date(10, 2), 1, book.food);

        monthly_summary(&book.ledger, ym(2026, 10));
    }
}

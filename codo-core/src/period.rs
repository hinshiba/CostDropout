//! 年月と期間

use std::fmt;

use chrono::{Datelike, NaiveDate};
use thiserror::Error;

/// 年月
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct YearMonth {
    year: i32,
    /// 1から12
    month: u32,
}

impl YearMonth {
    /// 月が1から12以外なら `InvalidMonth`
    pub fn new(year: i32, month: u32) -> Result<Self, PeriodError> {
        if !(1..=12).contains(&month) {
            return Err(PeriodError::InvalidMonth(month));
        }
        Ok(Self { year, month })
    }

    /// 日付が属する年月
    pub fn from_date(date: NaiveDate) -> Self {
        Self {
            year: date.year(),
            month: date.month(),
        }
    }

    /// 年
    pub fn year(&self) -> i32 {
        self.year
    }

    /// 月 (1から12)
    pub fn month(&self) -> u32 {
        self.month
    }

    /// 月の初日
    pub fn first_day(&self) -> NaiveDate {
        NaiveDate::from_ymd_opt(self.year, self.month, 1)
            .expect("Year and month in YearMonth are always valid")
    }

    /// 月の末日．末日を含む
    pub fn last_day(&self) -> NaiveDate {
        self.next()
            .first_day()
            .pred_opt()
            .expect("Previous day of next month's first day should exist")
    }

    /// その月の日数
    pub fn days_in_month(&self) -> u32 {
        self.last_day().day()
    }

    /// 翌月．12月なら翌年1月
    pub fn next(&self) -> Self {
        if self.month == 12 {
            Self {
                year: self.year + 1,
                month: 1,
            }
        } else {
            Self {
                year: self.year,
                month: self.month + 1,
            }
        }
    }

    /// 前月．1月なら前年12月
    pub fn prev(&self) -> Self {
        if self.month == 1 {
            Self {
                year: self.year - 1,
                month: 12,
            }
        } else {
            Self {
                year: self.year,
                month: self.month - 1,
            }
        }
    }

    /// 日付がこの年月に属するか
    pub fn contains(&self, date: NaiveDate) -> bool {
        date.year() == self.year && date.month() == self.month
    }
}

impl fmt::Display for YearMonth {
    /// `2026-12` のように，月をゼロ埋め2桁で書く
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}", self.year, self.month)
    }
}

/// 期間(start と end を両方含む)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Period {
    start: NaiveDate,
    end: NaiveDate,
}

impl Period {
    /// `end < start` なら `StartAfterEnd`．`start == end` は1日だけの期間
    pub fn new(start: NaiveDate, end: NaiveDate) -> Result<Self, PeriodError> {
        if end < start {
            return Err(PeriodError::StartAfterEnd { start, end });
        }
        Ok(Self { start, end })
    }

    /// 月の1日から末日までの期間
    pub fn month(ym: YearMonth) -> Self {
        Self {
            start: ym.first_day(),
            end: ym.last_day(),
        }
    }

    /// 1月1日から12月31日までの期間
    ///
    /// 年が chrono の扱える範囲外のときは panic する
    pub fn year(year: i32) -> Self {
        let jan = YearMonth::new(year, 1).expect("1 is a valid month");
        let dec = YearMonth::new(year, 12).expect("12 is a valid month");
        Self {
            start: jan.first_day(),
            end: dec.last_day(),
        }
    }

    /// 開始日
    pub fn start(&self) -> NaiveDate {
        self.start
    }

    /// 終了日
    pub fn end(&self) -> NaiveDate {
        self.end
    }

    /// 日付が期間内か．`start` と `end` を含む
    pub fn contains(&self, date: NaiveDate) -> bool {
        self.start <= date && date <= self.end
    }

    /// 期間に含まれる年月を古い順に返す
    pub fn months(&self) -> Vec<YearMonth> {
        let start_ym = YearMonth::from_date(self.start);
        let end_ym = YearMonth::from_date(self.end);

        let mut result = Vec::new();
        let mut current = start_ym;

        while current <= end_ym {
            result.push(current);
            current = current.next();
        }

        result
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PeriodError {
    #[error("不正な月です: {0}")]
    InvalidMonth(u32),
    #[error("開始日 {start} が終了日 {end} より後です")]
    StartAfterEnd { start: NaiveDate, end: NaiveDate },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_year_month_invalid() {
        assert_eq!(YearMonth::new(2026, 0), Err(PeriodError::InvalidMonth(0)));
        assert_eq!(YearMonth::new(2026, 13), Err(PeriodError::InvalidMonth(13)));
        assert!(YearMonth::new(2026, 10).is_ok());
    }

    #[test]
    fn test_leap_year_days() {
        let ym_leap = YearMonth::new(2024, 2).unwrap();
        assert_eq!(ym_leap.days_in_month(), 29);
        assert_eq!(
            ym_leap.last_day(),
            NaiveDate::from_ymd_opt(2024, 2, 29).unwrap()
        );

        let ym_normal = YearMonth::new(2026, 2).unwrap();
        assert_eq!(ym_normal.days_in_month(), 28);
        assert_eq!(
            ym_normal.last_day(),
            NaiveDate::from_ymd_opt(2026, 2, 28).unwrap()
        );
    }

    #[test]
    fn test_year_boundary_next_prev() {
        let ym_dec = YearMonth::new(2026, 12).unwrap();
        assert_eq!(ym_dec.next(), YearMonth::new(2027, 1).unwrap());

        let ym_jan = YearMonth::new(2026, 1).unwrap();
        assert_eq!(ym_jan.prev(), YearMonth::new(2025, 12).unwrap());
    }

    #[test]
    fn test_year_month_display() {
        let ym = YearMonth::new(2026, 10).unwrap();
        assert_eq!(ym.to_string(), "2026-10");
    }

    #[test]
    fn test_period_boundary_and_invalid() {
        let d1 = NaiveDate::from_ymd_opt(2026, 10, 1).unwrap();
        let d2 = NaiveDate::from_ymd_opt(2026, 10, 31).unwrap();

        assert_eq!(
            Period::new(d2, d1),
            Err(PeriodError::StartAfterEnd { start: d2, end: d1 })
        );

        let period = Period::new(d1, d2).unwrap();
        let day_before = NaiveDate::from_ymd_opt(2026, 9, 30).unwrap();
        let day_after = NaiveDate::from_ymd_opt(2026, 11, 1).unwrap();

        assert!(period.contains(d1));
        assert!(period.contains(d2));
        assert!(!period.contains(day_before));
        assert!(!period.contains(day_after));
    }

    #[test]
    fn test_period_months_with_year_boundary() {
        let start = NaiveDate::from_ymd_opt(2026, 11, 15).unwrap();
        let end = NaiveDate::from_ymd_opt(2027, 1, 10).unwrap();
        let period = Period::new(start, end).unwrap();

        let expected = vec![
            YearMonth::new(2026, 11).unwrap(),
            YearMonth::new(2026, 12).unwrap(),
            YearMonth::new(2027, 1).unwrap(),
        ];
        assert_eq!(period.months(), expected);
    }

    #[test]
    fn test_period_constructors_month_and_year() {
        let ym = YearMonth::new(2026, 10).unwrap();
        let p_month = Period::month(ym);
        assert_eq!(
            p_month.start(),
            NaiveDate::from_ymd_opt(2026, 10, 1).unwrap()
        );
        assert_eq!(
            p_month.end(),
            NaiveDate::from_ymd_opt(2026, 10, 31).unwrap()
        );

        let p_year = Period::year(2026);
        assert_eq!(p_year.start(), NaiveDate::from_ymd_opt(2026, 1, 1).unwrap());
        assert_eq!(p_year.end(), NaiveDate::from_ymd_opt(2026, 12, 31).unwrap());
    }

    #[test]
    fn test_year_month_from_date_and_contains() {
        let date = NaiveDate::from_ymd_opt(2026, 10, 15).unwrap();
        let ym = YearMonth::from_date(date);
        assert_eq!(ym, YearMonth::new(2026, 10).unwrap());

        assert!(ym.contains(date));
        assert!(ym.contains(NaiveDate::from_ymd_opt(2026, 10, 1).unwrap()));
        assert!(ym.contains(NaiveDate::from_ymd_opt(2026, 10, 31).unwrap()));
        assert!(!ym.contains(NaiveDate::from_ymd_opt(2026, 9, 30).unwrap()));
        assert!(!ym.contains(NaiveDate::from_ymd_opt(2026, 11, 1).unwrap()));
    }

    #[test]
    fn test_non_december_next_prev() {
        let ym = YearMonth::new(2026, 3).unwrap();
        assert_eq!(ym.next(), YearMonth::new(2026, 4).unwrap());
        assert_eq!(ym.prev(), YearMonth::new(2026, 2).unwrap());
    }

    #[test]
    fn test_single_day_period() {
        let d = NaiveDate::from_ymd_opt(2026, 10, 15).unwrap();
        let period = Period::new(d, d).unwrap();
        assert!(period.contains(d));
        assert_eq!(period.months(), vec![YearMonth::new(2026, 10).unwrap()]);
    }

    #[test]
    fn test_same_month_period_months() {
        let start = NaiveDate::from_ymd_opt(2026, 10, 5).unwrap();
        let end = NaiveDate::from_ymd_opt(2026, 10, 20).unwrap();
        let period = Period::new(start, end).unwrap();
        assert_eq!(period.months(), vec![YearMonth::new(2026, 10).unwrap()]);
    }

    #[test]
    fn test_various_months_last_day() {
        let ym_apr = YearMonth::new(2026, 4).unwrap();
        assert_eq!(ym_apr.days_in_month(), 30);
        assert_eq!(
            ym_apr.last_day(),
            NaiveDate::from_ymd_opt(2026, 4, 30).unwrap()
        );

        let ym_may = YearMonth::new(2026, 5).unwrap();
        assert_eq!(ym_may.days_in_month(), 31);
        assert_eq!(
            ym_may.last_day(),
            NaiveDate::from_ymd_opt(2026, 5, 31).unwrap()
        );
    }

    #[test]
    fn test_year_month_valid_bounds_and_getters() {
        let jan = YearMonth::new(2026, 1).unwrap();
        let dec = YearMonth::new(2026, 12).unwrap();
        assert_eq!((jan.year(), jan.month()), (2026, 1));
        assert_eq!((dec.year(), dec.month()), (2026, 12));
    }

    #[test]
    fn test_year_month_display_pads_zero() {
        assert_eq!(YearMonth::new(2026, 1).unwrap().to_string(), "2026-01");
        assert_eq!(YearMonth::new(2026, 9).unwrap().to_string(), "2026-09");
        assert_eq!(YearMonth::new(2026, 12).unwrap().to_string(), "2026-12");
        assert_eq!(YearMonth::new(999, 5).unwrap().to_string(), "0999-05");
    }

    #[test]
    fn test_days_in_month_all_months() {
        let expected = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
        for (i, days) in expected.into_iter().enumerate() {
            let ym = YearMonth::new(2026, i as u32 + 1).unwrap();
            assert_eq!(ym.days_in_month(), days, "{ym}");
        }
    }

    #[test]
    fn test_period_adjacent_days() {
        let d1 = NaiveDate::from_ymd_opt(2026, 10, 14).unwrap();
        let d2 = NaiveDate::from_ymd_opt(2026, 10, 15).unwrap();
        assert!(Period::new(d1, d2).is_ok());
        assert_eq!(
            Period::new(d2, d1),
            Err(PeriodError::StartAfterEnd { start: d2, end: d1 })
        );
    }

    #[test]
    fn test_period_year_leap() {
        let p = Period::year(2024);
        assert_eq!(p.start(), NaiveDate::from_ymd_opt(2024, 1, 1).unwrap());
        assert_eq!(p.end(), NaiveDate::from_ymd_opt(2024, 12, 31).unwrap());
        assert_eq!(p.months().len(), 12);
    }
}

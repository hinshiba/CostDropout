//! 年月と期間

use std::fmt;

use chrono::{Datelike, NaiveDate};
use thiserror::Error;

/// 年月
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct YearMonth {
    pub year: i32,
    /// 1から12
    pub month: u32,
}

/// 月の判定
impl YearMonth {
    pub fn new(year: i32, month: u32) -> Result<Self, PeriodError> {
        if !(1..=12).contains(&month) {
            return Err(PeriodError::InvalidMonth(month));
        }
        Ok(Self { year, month })
    }

    /// 年月をインポート
    pub fn from_date(date: NaiveDate) -> Self {
        Self {
            year: date.year(),
            month: date.month(),
        }
    }

    /// 月の初めの日と翌月の前日
    pub fn first_day(&self) -> NaiveDate {
        NaiveDate::from_ymd_opt(self.year, self.month, 1)
            .expect("Year and month in YearMonth are always valid")
    }

    pub fn last_day(&self) -> NaiveDate {
        self.next()
            .first_day()
            .pred_opt()
            .expect("Previous day of next month's first day should exist")
    }

    /// 各月の末日
    pub fn days_in_month(&self) -> u32 {
        self.last_day().day()
    }

    /// 前月，来月
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

    /// 年月の一致の確認
    pub fn contains(&self, date: NaiveDate) -> bool {
        date.year() == self.year && date.month() == self.month
    }
}

///2026-12のようなフォーマット化
impl fmt::Display for YearMonth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}", self.year, self.month)
    }
}

/// 期間(start と end を両方含む)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Period {
    pub start: NaiveDate,
    pub end: NaiveDate,
}

/// 開始時間＜＝終了時間の確認
impl Period {
    pub fn new(start: NaiveDate, end: NaiveDate) -> Result<Self, PeriodError> {
        if start > end {
            return Err(PeriodError::StartAfterEnd { start, end });
        }
        Ok(Self { start, end })
    }

    /// 月全体，年全体の構築
    pub fn month(ym: YearMonth) -> Self {
        Self {
            start: ym.first_day(),
            end: ym.last_day(),
        }
    }

    pub fn year(year: i32) -> Self {
        let start = NaiveDate::from_ymd_opt(year, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(year, 12, 31).unwrap();
        Self { start, end }
    }

    /// 日付の確認
    pub fn contains(&self, date: NaiveDate) -> bool {
        self.start <= date && date <= self.end
    }

    /// 期間内の年月の列挙
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
        // 閏年 (2024-02 は 29日)
        let ym_leap = YearMonth::new(2024, 2).unwrap();
        assert_eq!(ym_leap.days_in_month(), 29);
        assert_eq!(
            ym_leap.last_day(),
            NaiveDate::from_ymd_opt(2024, 2, 29).unwrap()
        );

        // 平年 (2026-02 は 28日)
        let ym_normal = YearMonth::new(2026, 2).unwrap();
        assert_eq!(ym_normal.days_in_month(), 28);
        assert_eq!(
            ym_normal.last_day(),
            NaiveDate::from_ymd_opt(2026, 2, 28).unwrap()
        );
    }

    #[test]
    fn test_year_boundary_next_prev() {
        // 年またぎの next / prev
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

        // start > end のエラーテスト
        assert_eq!(
            Period::new(d2, d1),
            Err(PeriodError::StartAfterEnd { start: d2, end: d1 })
        );

        // 境界の日(初日・末日・その前後)のチェック
        let period = Period::new(d1, d2).unwrap();
        let day_before = NaiveDate::from_ymd_opt(2026, 9, 30).unwrap();
        let day_after = NaiveDate::from_ymd_opt(2026, 11, 1).unwrap();

        assert!(period.contains(d1)); // 初日
        assert!(period.contains(d2)); // 末日
        assert!(!period.contains(day_before)); // 初日の前日
        assert!(!period.contains(day_after)); // 末日の翌日
    }

    #[test]
    fn test_period_months_with_year_boundary() {
        // 2026-11-15 .. 2027-01-10 の年またぎ期間における月一覧の算出
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
}

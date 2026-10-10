//! 年月と期間

use std::fmt;

use chrono::{Datelike,NaiveDate};
use thiserror::Error;

/// 年月
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct YearMonth {
    pub year: i32,
    /// 1から12
    pub month: u32,
}

/// 月の判定
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
        Self { year: self.year + 1, month: 1 }
    } else {
        Self { year: self.year, month: self.month + 1 }
    }
}

pub fn prev(&self) -> Self {
    if self.month == 1 {
        Self { year: self.year - 1, month: 12 }
    } else {
        Self { year: self.year, month: self.month - 1 }
    }
}

/// 年月の一致の確認
pub fn contains(&self, date: NaiveDate) -> bool {
    date.year() == self.year && date.month() == self.month
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
pub fn new(start: NaiveDate, end: NaiveDate) -> Result<Self, PeriodError> {
    if start > end {
        return Err(PeriodError::StartAfterEnd);
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

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PeriodError {
    #[error("不正な月です: {0}")]
    InvalidMonth(u32),
    #[error("開始日 {start} が終了日 {end} より後です")]
    StartAfterEnd { start: NaiveDate, end: NaiveDate },
}


#[cfg(test)]
mod tests { ... }
//! 年月と期間

use chrono::NaiveDate;
use thiserror::Error;

/// 年月
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct YearMonth {
    pub year: i32,
    /// 1から12
    pub month: u32,
}

/// 期間(start と end を両方含む)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Period {
    pub start: NaiveDate,
    pub end: NaiveDate,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PeriodError {
    #[error("不正な月です: {0}")]
    InvalidMonth(u32),
    #[error("開始日 {start} が終了日 {end} より後です")]
    StartAfterEnd { start: NaiveDate, end: NaiveDate },
}

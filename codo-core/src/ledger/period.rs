//! 年月と期間のモデルと表示

use chrono::format::{self, Parsed, StrftimeItems};
use chrono::{Datelike, Months, NaiveDate};
use std::fmt;
use std::iter;
use std::str::FromStr;
use thiserror::Error;

/// `YearMonth` の書式.
const YEAR_MONTH_FORMAT: &str = "%Y-%m";

/// 年月を表現する.
///
/// `NaiveDate` の日付を取り出せなくすることで実現している.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct YearMonth(
    /// 月の初日.
    NaiveDate,
);

impl YearMonth {
    /// 年と月から生成する.
    ///
    /// # Errors
    ///
    /// 月が1以上12以下でない場合と,
    /// 年が chrono で表現できる範囲を超える場合は `PeriodError::InvalidYearMonth` エラー.
    pub fn new(year: i32, month: u32) -> Result<Self, PeriodError> {
        NaiveDate::from_ymd_opt(year, month, 1)
            .map(YearMonth)
            .ok_or(PeriodError::InvalidYearMonth { year, month })
    }

    /// 日付を含む年月を返す.
    pub fn from_date(date: NaiveDate) -> Self {
        YearMonth(
            date.with_day(1)
                .expect("月の初日は常に存在するため到達不能"),
        )
    }

    /// 年を返す.
    pub fn year(&self) -> i32 {
        self.0.year()
    }

    /// 月を返す.
    pub fn month(&self) -> u32 {
        self.0.month()
    }

    /// 月の初日を返す.
    pub fn first_day(&self) -> NaiveDate {
        self.0
    }

    /// 月の末日を返す.
    pub fn last_day(&self) -> NaiveDate {
        let days = u32::from(self.0.num_days_in_month());
        self.0
            .with_day(days)
            .expect("月の末日は常に存在するため到達不能")
    }

    /// 翌月を返す.
    ///
    /// # Panics
    ///
    /// 翌月が chrono で表現できる範囲を超える場合はパニック.
    pub fn next(&self) -> Self {
        self.checked_next().expect("表現できる範囲を超えました")
    }

    /// 前月を返す.
    ///
    /// # Panics
    ///
    /// 前月が chrono で表現できる範囲を超える場合はパニック.
    pub fn prev(&self) -> Self {
        self.checked_prev().expect("表現できる範囲を超えました")
    }

    /// 翌月を返す.
    ///
    /// 翌月が chrono で表現できる範囲を超える場合は `None`.
    pub fn checked_next(&self) -> Option<Self> {
        self.0.checked_add_months(Months::new(1)).map(YearMonth)
    }

    /// 前月を返す.
    ///
    /// 前月が chrono で表現できる範囲を超える場合は `None`.
    pub fn checked_prev(&self) -> Option<Self> {
        self.0.checked_sub_months(Months::new(1)).map(YearMonth)
    }

    /// 日付がこの年月に含まれるかを判定する.
    pub fn contains(&self, date: NaiveDate) -> bool {
        Self::from_date(date) == *self
    }
}

impl fmt::Display for YearMonth {
    /// 年月を整形して出力する.
    ///
    /// ISO 8601 の年月の書式に従う.
    /// 年は4桁で, 0年より前と9999年より後は符号を付けて桁を増やす.
    ///
    /// # Examples
    ///
    /// ```
    /// use codo_core::ledger::period::YearMonth;
    ///
    /// assert_eq!(YearMonth::new(2026, 10).unwrap().to_string(), "2026-10");
    /// assert_eq!(YearMonth::new(800, 1).unwrap().to_string(), "0800-01");
    /// assert_eq!(YearMonth::new(10000, 1).unwrap().to_string(), "+10000-01");
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.format(YEAR_MONTH_FORMAT))
    }
}

impl FromStr for YearMonth {
    type Err = PeriodError;

    /// `YearMonth` にパースする.
    ///
    /// 年と月を `-` でつないだものとして解釈する.
    /// 年と月の桁数は問わず, 年には符号を付けてもよい.
    ///
    /// # Examples
    ///
    /// ```
    /// use codo_core::ledger::period::YearMonth;
    ///
    /// assert_eq!("2026-10".parse(), YearMonth::new(2026, 10));
    /// assert_eq!("2026-1".parse(), YearMonth::new(2026, 1));
    /// assert_eq!("+10000-01".parse(), YearMonth::new(10000, 1));
    /// ```
    ///
    /// # Errors
    ///
    /// 年月として解釈できない場合は `PeriodError::Parse` エラー.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // 書式は日を含まないので, 日付として組み立てるために初日を補う
        let mut parsed = Parsed::new();
        format::parse(&mut parsed, s, StrftimeItems::new(YEAR_MONTH_FORMAT))
            .and_then(|()| parsed.set_day(1))
            .and_then(|()| parsed.to_naive_date())
            .map(YearMonth)
            .map_err(|_| PeriodError::Parse(s.to_string()))
    }
}

/// 期間を表現する.
///
/// 閉区間であり範囲の境界を常に含む.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Period {
    start: NaiveDate,
    end: NaiveDate,
}

impl Period {
    /// 開始日と終了日から生成する.
    ///
    /// # Errors
    ///
    /// 開始日が終了日より後の場合は `PeriodError::StartAfterEnd` エラー.
    pub fn new(start: NaiveDate, end: NaiveDate) -> Result<Self, PeriodError> {
        if end < start {
            return Err(PeriodError::StartAfterEnd { start, end });
        }
        Ok(Self { start, end })
    }

    /// 月の初日から末日までの期間を生成する.
    pub fn month(ym: YearMonth) -> Self {
        Self {
            start: ym.first_day(),
            end: ym.last_day(),
        }
    }

    /// 年の1月1日から12月31日までの期間を生成する.
    ///
    /// # Panics
    ///
    /// 年が chrono で表現できる範囲を超える場合はパニック.
    pub fn year(year: i32) -> Self {
        let date = |month, day| {
            NaiveDate::from_ymd_opt(year, month, day).expect("表現できる範囲を超えました")
        };
        Self {
            start: date(1, 1),
            end: date(12, 31),
        }
    }

    /// 開始日を返す.
    pub fn start(&self) -> NaiveDate {
        self.start
    }

    /// 終了日を返す.
    pub fn end(&self) -> NaiveDate {
        self.end
    }

    /// 日付が期間に含まれるかを判定する.
    pub fn contains(&self, date: NaiveDate) -> bool {
        (self.start..=self.end).contains(&date)
    }

    /// 期間内の日付を昇順に返す.
    pub fn days(&self) -> impl Iterator<Item = NaiveDate> {
        let end = self.end;
        iter::successors(Some(self.start), NaiveDate::succ_opt).take_while(move |&date| date <= end)
    }

    /// 期間と重なる年月を昇順に返す.
    pub fn months(&self) -> impl Iterator<Item = YearMonth> {
        let last = YearMonth::from_date(self.end);
        // 表現できる最後の年月でパニックしないよう, 翌月がなければ終える
        iter::successors(
            Some(YearMonth::from_date(self.start)),
            YearMonth::checked_next,
        )
        .take_while(move |&ym| ym <= last)
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PeriodError {
    /// 年月が存在しない場合.
    #[error("存在しない年月です: {year}年{month}月")]
    InvalidYearMonth { year: i32, month: u32 },
    /// 年月として解釈できない文字列である場合.
    #[error("年月として解釈できない文字列です: {0}")]
    Parse(String),
    /// 開始日が終了日より後である場合.
    #[error("開始日 {start} が終了日 {end} より後です")]
    StartAfterEnd { start: NaiveDate, end: NaiveDate },
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    fn ym(year: i32, month: u32) -> YearMonth {
        YearMonth::new(year, month).unwrap()
    }

    #[test]
    fn new_test() {
        for (year, month) in [(2026, 1), (2026, 12), (NaiveDate::MIN.year(), 1)] {
            let ym = YearMonth::new(year, month).unwrap();
            assert_eq!((ym.year(), ym.month()), (year, month), "{year}-{month}");
        }
    }

    #[test]
    fn new_error_test() {
        for (year, month) in [
            (2026, 0),
            (2026, 13),
            (2026, u32::MAX),
            (NaiveDate::MIN.year() - 1, 12),
            (NaiveDate::MAX.year() + 1, 1),
            (i32::MIN, 1),
            (i32::MAX, 1),
        ] {
            assert_eq!(
                YearMonth::new(year, month),
                Err(PeriodError::InvalidYearMonth { year, month })
            );
        }
    }

    #[test]
    fn from_str_test() {
        for (s, expected) in [
            ("2026-10", ym(2026, 10)),
            ("2026-01", ym(2026, 1)),
            ("2026-1", ym(2026, 1)),
            ("2026-12", ym(2026, 12)),
            ("0000-01", ym(0, 1)),
            ("-0001-12", ym(-1, 12)),
            ("9999-12", ym(9999, 12)),
            ("+10000-01", ym(10000, 1)),
        ] {
            assert_eq!(s.parse::<YearMonth>(), Ok(expected), "{s}");
        }
    }

    #[test]
    fn from_str_parse_error_test() {
        for s in [
            "",
            "-",
            "2026",
            "2026-",
            "-10",
            "2026-0",
            "2026-13",
            "2026-10-01",
            "2026/10",
            "202610",
            "abc",
            "+999999-01",
            "-999999-01",
        ] {
            assert_eq!(
                s.parse::<YearMonth>(),
                Err(PeriodError::Parse(s.to_string())),
                "{s}"
            );
        }
    }

    #[test]
    fn display_from_str_roundtrip_test() {
        for ym in [
            YearMonth::from_date(NaiveDate::MIN),
            ym(-1, 12),
            ym(0, 1),
            ym(9999, 12),
            ym(10000, 1),
            YearMonth::from_date(NaiveDate::MAX),
        ] {
            assert_eq!(ym.to_string().parse(), Ok(ym), "{ym}");
        }
    }

    #[test]
    fn period_new_test() {
        let d = date(2026, 10, 15);
        assert!(Period::new(d, d).is_ok());

        let next = d.succ_opt().unwrap();
        assert!(Period::new(d, next).is_ok());
        assert_eq!(
            Period::new(next, d),
            Err(PeriodError::StartAfterEnd {
                start: next,
                end: d
            })
        );
    }

    #[test]
    fn days_test() {
        for (start, end, expected) in [
            (
                date(2026, 10, 15),
                date(2026, 10, 15),
                vec![date(2026, 10, 15)],
            ),
            (
                date(2026, 12, 31),
                date(2027, 1, 2),
                vec![date(2026, 12, 31), date(2027, 1, 1), date(2027, 1, 2)],
            ),
            (NaiveDate::MIN, NaiveDate::MIN, vec![NaiveDate::MIN]),
            (NaiveDate::MAX, NaiveDate::MAX, vec![NaiveDate::MAX]),
        ] {
            let period = Period::new(start, end).unwrap();
            assert_eq!(period.days().collect::<Vec<_>>(), expected, "{period:?}");
        }
    }

    #[test]
    fn months_test() {
        for (start, end, expected) in [
            (date(2026, 10, 15), date(2026, 10, 15), vec![ym(2026, 10)]),
            (date(2026, 10, 1), date(2026, 10, 31), vec![ym(2026, 10)]),
            (
                date(2026, 10, 31),
                date(2026, 11, 1),
                vec![ym(2026, 10), ym(2026, 11)],
            ),
            (
                date(2026, 11, 15),
                date(2027, 1, 10),
                vec![ym(2026, 11), ym(2026, 12), ym(2027, 1)],
            ),
            (
                NaiveDate::MIN,
                NaiveDate::MIN,
                vec![YearMonth::from_date(NaiveDate::MIN)],
            ),
            (
                NaiveDate::MAX,
                NaiveDate::MAX,
                vec![YearMonth::from_date(NaiveDate::MAX)],
            ),
        ] {
            let period = Period::new(start, end).unwrap();
            assert_eq!(period.months().collect::<Vec<_>>(), expected, "{period:?}");
        }
    }

    #[test]
    fn checked_prev_returns_previous_month() {
        let month = YearMonth::new(2024, 5).unwrap();

        assert_eq!(month.checked_prev(), Some(YearMonth::new(2024, 4).unwrap()));
    }

    #[test]
    fn contains_returns_true_only_for_same_month() {
        let month = YearMonth::new(2024, 5).unwrap();

        assert!(month.contains(NaiveDate::from_ymd_opt(2024, 5, 15).unwrap()));

        assert!(!month.contains(NaiveDate::from_ymd_opt(2024, 6, 1).unwrap()));
    }
}

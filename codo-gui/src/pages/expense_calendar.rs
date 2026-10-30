//! 支出カレンダー画面

use crate::app::AppState;
use chrono::Datelike;
use codo_core::ledger::money::Money;
use codo_core::ledger::period::YearMonth;
use codo_core::ledger::report::calendar::{EXPENSE_LEVELS, expense_level};
use codo_core::ledger::report::monthly::{DailySummary, monthly_summary};
use gpui_kit::component::button::Button;
use gpui_kit::*;

/// 支出カレンダー画面
pub struct ExpenseCalendar {
    // 共有されているAppState
    state: Entity<AppState>,

    // 表示されている年月
    yearmonth: YearMonth,
}

impl ExpenseCalendar {
    pub fn new(state: Entity<AppState>) -> Self {
        let yearmonth = YearMonth::from_date(chrono::Local::now().date_naive());
        Self { state, yearmonth }
    }

    /// 前の月を表示する
    fn prev_month(&mut self, cx: &mut Context<Self>) {
        self.yearmonth = self.yearmonth.prev();
        cx.notify();
    }

    /// 次の月を表示する
    fn next_month(&mut self, cx: &mut Context<Self>) {
        self.yearmonth = self.yearmonth.next();
        cx.notify();
    }
}

// 画面開発者は自由に変更してよい
impl Render for ExpenseCalendar {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let month_label = format!("{}年{}月", self.yearmonth.year(), self.yearmonth.month());

        // 月の集計
        let summary = monthly_summary(self.state.read(cx).ledger(), self.yearmonth);

        // 月の一番多い日の支出
        let max_expense = summary
            .days()
            .iter()
            .map(|day| day.expense())
            .max()
            .unwrap_or_default();

        // 1日の前の空きマスの数（日曜始まり）
        let blank_days = self.yearmonth.first_day().weekday().num_days_from_sunday() as usize;

        // カレンダーのマス（空きマスは None）
        let mut cells: Vec<Option<DailySummary>> = Vec::new();
        for _ in 0..blank_days {
            cells.push(None);
        }
        for day in summary.days() {
            cells.push(Some(*day));
        }
        while !cells.len().is_multiple_of(7) {
            cells.push(None);
        }

        // 曜日の見出し
        let weekday_header = div().flex().flex_row().children(
            ["日", "月", "火", "水", "木", "金", "土"]
                .into_iter()
                .map(|weekday| div().w(px(100.)).flex().justify_center().child(weekday)),
        );

        // 週ごとの行
        let week_rows = cells
            .chunks(7)
            .map(|week| {
                div().flex().flex_row().children(week.iter().map(|cell| {
                    match cell {
                        None => div()
                            .w(px(100.))
                            .h(px(80.))
                            .border_1()
                            .border_color(rgb(0xe0e0e0)),
                        Some(day) => {
                            let level = expense_level(day.expense(), max_expense);
                            let text_color = if EXPENSE_LEVELS <= level {
                                rgb(0xffffff)
                            } else {
                                rgb(0x000000)
                            };
                            div()
                                .w(px(100.))
                                .h(px(80.))
                                .border_1()
                                .border_color(rgb(0xe0e0e0))
                                .bg(level_color(level))
                                .text_color(text_color)
                                .p_1()
                                .flex()
                                .flex_col()
                                .child(day.date().day().to_string())
                                .child(day.expense().to_string())
                        }
                    }
                }))
            })
            .collect::<Vec<_>>();

        // 凡例
        let legend = div()
            .flex()
            .flex_row()
            .gap_4()
            .children((0..=EXPENSE_LEVELS).map(|level| {
                let label = if level == 0 {
                    "¥0".to_string()
                } else {
                    format!(
                        "~{}",
                        Money::from(
                            (f64::from(max_expense) * f64::from(level) / f64::from(EXPENSE_LEVELS))
                                as i32
                        )
                    )
                };
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_1()
                    .child(
                        div()
                            .w(px(20.))
                            .h(px(20.))
                            .bg(level_color(level))
                            .border_1()
                            .border_color(rgb(0xe0e0e0)),
                    )
                    .child(label)
            }));

        let prev_button =
            Button::new("prev_button")
                .label("◀")
                .on_click(cx.listener(|this, _, _, cx| {
                    this.prev_month(cx);
                }));

        let next_button =
            Button::new("next_button")
                .label("▶")
                .on_click(cx.listener(|this, _, _, cx| {
                    this.next_month(cx);
                }));

        // ボタンを設置
        let month_selector = div()
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .justify_center()
            .py_2()
            .bg(rgb(0xc62828))
            .w_full()
            .text_color(rgb(0xffffff))
            .child(prev_button)
            .child(month_label)
            .child(next_button);

        div()
            .id("expense-calendar")
            .size_full()
            .overflow_scroll()
            .bg(rgb(0xffffff))
            .text_xl()
            .text_color(rgb(0x000000))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .w_full()
                    .min_w(px(700.))
                    .child(month_selector)
                    .child(weekday_header)
                    .child(div().flex().flex_col().children(week_rows))
                    .child(legend),
            )
    }
}

/// 支出の段階に対応するマスの色
fn level_color(level: u8) -> Rgba {
    match level {
        0 => rgb(0xffffff),
        1 => rgb(0xffebee),
        2 => rgb(0xffcdd2),
        3 => rgb(0xef9a9a),
        4 => rgb(0xe57373),
        _ => rgb(0xc62828),
    }
}

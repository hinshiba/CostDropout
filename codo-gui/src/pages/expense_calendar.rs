//! 支出カレンダー画面

use crate::app::AppState;
use chrono::Datelike;
use codo_core::period::YearMonth;
use codo_core::report::monthly::{DailySummary, monthly_summary};
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
        let summary = monthly_summary(&self.state.read(cx).ledger, self.yearmonth);

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
            .bg(rgb(0xe57373))
            .w_full()
            .text_color(rgb(0xffffff))
            .child(prev_button)
            .child(month_label)
            .child(next_button);

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(0xffffff))
            .items_center()
            .text_xl()
            .text_color(rgb(0x000000))
            .child(month_selector)
            .child(weekday_header)
    }
}

//! 支出カレンダー画面

use crate::app::AppState;
use chrono::{Datelike, NaiveDate};
use codo_core::period::YearMonth;
use codo_core::report::monthly::monthly_summary;
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
    }
}

//! 月の収支表示画面

use crate::app::AppState;
use chrono::{Datelike, NaiveDate};
use codo_core::period::YearMonth;
use codo_core::report::monthly::monthly_summary;
use gpui_kit::component::button::Button;
use gpui_kit::*;

/// 月の収支画面
pub struct MonthlyReport {
    // 共有されているAppState
    state: Entity<AppState>,

    // 表示されている年月
    yearmonth: YearMonth,

    // 選択している日
    selected_day: Option<NaiveDate>,
}

impl MonthlyReport {
    pub fn new(state: Entity<AppState>) -> Self {
        let yearmonth = YearMonth::from_date(chrono::Local::now().date_naive());
        let selected_day = None;
        Self {
            state,
            yearmonth,
            selected_day,
        }
    }

    /// 前の月を表示する
    fn prev_month(&mut self, cx: &mut Context<Self>) {
        self.yearmonth = self.yearmonth.prev();
        self.selected_day = None;
        cx.notify();
    }

    /// 次の月を表示する
    fn next_month(&mut self, cx: &mut Context<Self>) {
        self.yearmonth = self.yearmonth.next();
        self.selected_day = None;
        cx.notify();
    }

    /// 日を選ぶ
    fn select_day(&mut self, date: NaiveDate, cx: &mut Context<Self>) {
        self.selected_day = Some(date);
        cx.notify();
    }
}

// 画面開発者は自由に変更してよい
impl Render for MonthlyReport {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let month_label = format!("{}年{}月", self.yearmonth.year(), self.yearmonth.month());

        let summary = monthly_summary(&self.state.read(cx).ledger, self.yearmonth);

        let summary_income = format!("収入: {}", summary.income());
        let summary_expense = format!("支出: {}", summary.expense());
        let summary_difference = format!("差額: {}", summary.income() - summary.expense());

        let day_list_height = (window.viewport_size().height - px(200.)).max(px(0.));

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
            .bg(rgb(0x9370db))
            .w_full()
            .text_color(rgb(0xffffff))
            .child(prev_button)
            .child(month_label)
            .child(next_button);

        // 月の合計
        let totals = div()
            .flex()
            .flex_col()
            .items_center()
            .gap_2()
            .child(summary_income)
            .child(summary_expense)
            .child(summary_difference);

        // 日ごとの収支の行
        let day_rows = summary
            .days()
            .iter()
            .map(|day| {
                let date = day.date();
                let day_day = format!("{}日", date.day());
                let day_income = format!("収入: {}", day.income());
                let day_expense = format!("支出: {}", day.expense());
                let row_color = if self.selected_day == Some(date) {
                    rgb(0xe6e0f8)
                } else {
                    rgb(0xffffff)
                };
                div()
                    .id(format!("day-{}", date))
                    .flex()
                    .flex_row()
                    .border_b_1()
                    .border_color(rgb(0xe0e0e0))
                    .px_4()
                    .bg(row_color)
                    .gap_4()
                    .child(div().w(px(70.)).flex().justify_end().child(day_day))
                    .child(div().w(px(200.)).child(day_income))
                    .child(div().w(px(200.)).child(day_expense))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.select_day(date, cx);
                    }))
            })
            .collect::<Vec<_>>();

        // 日の表示
        let day_list = div()
            .id("day-list")
            .flex()
            .flex_col()
            .h(day_list_height)
            .overflow_y_scroll()
            .children(day_rows);

        // 選んだ日の取引の詳細
        let detail = match self.selected_day {
            None => div().child("日を選ぶと，その日の取引が表示されます"),
            Some(date) => {
                let ledger = &self.state.read(cx).ledger;
                let tx_rows = ledger
                    .monthly_transactions(self.yearmonth)
                    .filter(|tx| tx.date() == date)
                    .map(|tx| {
                        let item_name = |id| {
                            ledger
                                .items()
                                .get_item(id)
                                .map(|item| item.name().to_string())
                                .unwrap_or_else(|| "削除された品目".to_string())
                        };
                        let name = item_name(tx.item());
                        let amount = tx.amount().to_string();
                        let comment = tx.comment().to_string();
                        div()
                            .flex()
                            .flex_row()
                            .gap_4()
                            .child(div().w(px(120.)).child(name))
                            .child(div().w(px(120.)).child(amount))
                            .child(div().w(px(120.)).child(comment))
                    })
                    .collect::<Vec<_>>();
                if tx_rows.is_empty() {
                    div().child("この日の取引はありません")
                } else {
                    div().flex().flex_col().children(tx_rows)
                }
            }
        };

        // 日と詳細の表示
        let body = div()
            .flex()
            .flex_row()
            .gap_4()
            .child(day_list)
            .child(detail);

        // 実際の収支表示
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(0xffffff))
            .items_center()
            .text_xl()
            .text_color(rgb(0x000000))
            .child(month_selector)
            .child(totals)
            .child(body)
    }
}

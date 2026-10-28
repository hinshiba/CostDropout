//! 支出カレンダー画面

use crate::app::AppState;
use gpui_kit::*;

/// 支出カレンダー画面
pub struct ExpenseCalendar {
    // 共有されているAppState
    state: Entity<AppState>,
}

impl ExpenseCalendar {
    pub fn new(state: Entity<AppState>) -> Self {
        Self { state }
    }
}

// 画面開発者は自由に変更してよい
impl Render for ExpenseCalendar {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = self.state.read(cx);
        let _ledger = &state.ledger;

        div()
            .size_full()
            .flex()
            .bg(rgb(0xe57373))
            .justify_center()
            .items_center()
            .text_xl()
            .text_color(rgb(0xffffff))
            .child("支出カレンダー")
    }
}

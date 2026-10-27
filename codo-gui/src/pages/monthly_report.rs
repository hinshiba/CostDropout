//! 月の収支表示画面

use crate::app::AppState;
use gpui_kit::*;

/// 月の収支画面
pub struct MonthlyReport {
    // 共有されているAppState
    state: Entity<AppState>,
}

impl MonthlyReport {
    pub fn new(state: Entity<AppState>) -> Self {
        Self { state }
    }
}

// 画面開発者は自由に変更してよい
impl Render for MonthlyReport {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = self.state.read(cx);

        let _ledger = &state.ledger;

        // 実際の収支表示
        div()
            .size_full()
            .flex()
            .bg(rgb(0x9370db))
            .justify_center()
            .items_center()
            .text_xl()
            .text_color(rgb(0xffffff))
            .child("月の収支")
    }
}

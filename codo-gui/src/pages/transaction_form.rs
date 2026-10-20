//! 取引入力表示画面

use crate::app::AppState;
use gpui_kit::*;

/// 取引入力画面
pub struct TransactionForm {
    // 共有されているAppState
    state: Entity<AppState>,
}

impl TransactionForm {
    pub fn new(state: Entity<AppState>) -> Self {
        Self { state }
    }
}

// 画面開発者は自由に変更してよい
impl Render for TransactionForm {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = self.state.read(cx);

        let _ledger = &state.ledger;

        // 実際の取引フォーム表示
        div()
            .size_full()
            .flex()
            .bg(rgb(0x2e7d87))
            .justify_center()
            .items_center()
            .text_xl()
            .text_color(rgb(0xffffff))
            .child("取引入力")
    }
}

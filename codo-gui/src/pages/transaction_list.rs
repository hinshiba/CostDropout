//! 取引リスト表示画面

use crate::app::AppState;
use gpui_kit::*;

/// 取引リスト画面
pub struct TransactionList {
    // 共有されているAppState
    state: Entity<AppState>,
}

impl TransactionList {
    pub fn new(state: Entity<AppState>) -> Self {
        Self { state }
    }
}

// 画面開発者は自由に変更してよい
impl Render for TransactionList {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = self.state.read(cx);

        let _ledger = &state.ledger;

        // 実際の取引リスト表示
        div()
            .size_full()
            .flex()
            .bg(rgb(0x5e7d32))
            .justify_center()
            .items_center()
            .text_xl()
            .text_color(rgb(0xffffff))
            .child("取引一覧")
    }
}

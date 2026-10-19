//! 品目管理表示画面

use crate::app::AppState;
use gpui_kit::*;

/// 品目画面
pub struct Items {
    // 共有されているAppState
    state: Entity<AppState>,
}

impl Items {
    pub fn new(state: Entity<AppState>) -> Self {
        Self { state }
    }
}

// 画面開発者は自由に変更してよい
impl Render for Items {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = self.state.read(cx);

        let _ledger = &state.ledger;

        // 実際の品目画面表示
        div()
            .flex()
            .bg(rgb(0x2e7d32))
            .size_full()
            .justify_center()
            .items_center()
            .text_xl()
            .text_color(rgb(0xffffff))
            .child("品目")
    }
}

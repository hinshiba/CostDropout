//! 品目管理表示画面

use crate::app::AppState;
use crate::item::Item;
use gpui_kit::*;

// 品目画面
pub struct Items {
    // 共有されているAppState
    state: Entity<AppState>,
}

impl Items {
    pub fn new(state: Entity<AppState>) -> Self {
        Self { state }
    }
}

impl Render for Items {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let item_count = self.state.read(cx).ledger.items().iter();

        div()
            .flex()
            .bg(rgb(0x2e7d32))
            .size_full()
            .justify_center()
            .items_center()
            .text_xl()
            .text_color(rgb(0xffffff))
            .child(format!("品目"))
    }
}

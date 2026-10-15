use crate::app::AppState;
use gpui_kit::*;
pub struct TransactionList {
    state: Entity<AppState>,
}

impl TransactionList {
    pub fn new(state: Entity<AppState>) -> Self {
        Self { state }
    }
}

impl Render for TransactionList {
    /// TransactionListを画面に表示する
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .bg(rgb(0x5e7d32))
            .justify_center()
            .items_center()
            .text_xl()
            .text_color(rgb(0xffffff))
            .child(format!("取引一覧"))
    }
}

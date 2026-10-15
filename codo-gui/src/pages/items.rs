use crate::app::AppState;
use gpui_kit::*;
pub struct Items {
    state: Entity<AppState>,
}

impl Items {
    pub fn new(state: Entity<AppState>) -> Self {
        Self { state }
    }
}

impl Render for Items {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
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

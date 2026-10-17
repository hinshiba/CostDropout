//! 画面切り替え
use crate::pages::{
    items::Items, transaction_form::TransactionForm, transaction_list::TransactionList,
};
use codo_core::ledger::Ledger;
use gpui_kit::component::button::Button;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

pub struct AppState {
    pub ledger: Ledger,
}
/// 現在の画面のenum
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Page {
    TransactionForm,
    TransactionList,
    Items,
}

pub struct App {
    state: Entity<AppState>,
    transaction_form: Entity<TransactionForm>,
    transaction_list: Entity<TransactionList>,
    items: Entity<Items>,
    page: Page,
    sidebar_open: bool,
}

impl App {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let state = cx.new(|_| AppState {
            ledger: Ledger::with_presets(),
        });
        let transaction_form = cx.new(|_| TransactionForm::new(state.clone()));

        let transaction_list = cx.new(|_| TransactionList::new(state.clone()));

        let items = cx.new(|_| Items::new(state.clone()));

        Self {
            state,
            transaction_form,
            transaction_list,
            items,
            page: Page::TransactionForm,
            sidebar_open: true,
        }
    }

    fn show_page(&mut self, page: Page) {
        self.page = page;
    }

    fn toggle_sidebar(&mut self) {
        self.sidebar_open = !self.sidebar_open;
    }
}

impl Render for App {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .child(
                // 上側：画面切り替えバー
                div()
                    .w(px(if self.sidebar_open { 200.0 } else { 70.0 }))
                    .h_full()
                    .flex()
                    .flex_col()
                    .p_4()
                    .gap_4()
                    .child(
                        // サイドバー開閉ボタン
                        Button::new("toggle-sidebar")
                            .label("☰")
                            .bg(rgb(0xafdfe4))
                            .on_click(cx.listener(|app, _, _, _| {
                                app.toggle_sidebar();
                            })),
                    )
                    .when(self.sidebar_open, |sidebar| {
                        sidebar
                            .child(Button::new("transaction-form").label("取引入力").on_click(
                                cx.listener(|app, _, _, _| {
                                    app.show_page(Page::TransactionForm);
                                }),
                            ))
                            .child(Button::new("transaction-list").label("取引一覧").on_click(
                                cx.listener(|app, _, _, _| {
                                    app.show_page(Page::TransactionList);
                                }),
                            ))
                            .child(Button::new("items").label("品目").on_click(cx.listener(
                                |app, _, _, _| {
                                    app.show_page(Page::Items);
                                },
                            )))
                    }),
            )
            .child(div().flex_1().h_full().child(
                // 下側：選択された画面
                div().flex_1().child(match self.page {
                    Page::TransactionForm => div().child(self.transaction_form.clone()),
                    Page::TransactionList => div().child(self.transaction_list.clone()),
                    Page::Items => div().child(self.items.clone()),
                }),
            ))
    }
}

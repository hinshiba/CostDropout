//! 画面切り替えと共有画面の作成

use crate::pages::{
    items::Items, transaction_form::TransactionForm, transaction_list::TransactionList,
};
use codo_core::ledger::Ledger;
use gpui_kit::component::button::Button;
use gpui_kit::component::sidebar::{Sidebar, SidebarMenu, SidebarMenuItem};
use gpui_kit::*;

/// ledgerの共有
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

/// 画面の管理
pub struct App {
    transaction_form: Entity<TransactionForm>,
    transaction_list: Entity<TransactionList>,
    items: Entity<Items>,
    page: Page,
    sidebar_open: bool,
}

impl App {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let state = cx.new(|_| AppState {
            ledger: Ledger::with_presets(),
        });
        let transaction_form = cx.new(|cx| TransactionForm::new(state.clone(), window, cx));

        let transaction_list = cx.new(|cx| TransactionList::new(state.clone(), cx));

        let items = cx.new(|_| Items::new(state.clone()));

        Self {
            transaction_form,
            transaction_list,
            items,
            page: Page::TransactionForm,
            sidebar_open: true,
        }
    }

    /// 開いているページを切り替える
    fn show_page(&mut self, page: Page, cx: &mut Context<Self>) {
        self.page = page;
        cx.notify();
    }

    /// サイドバーの開閉を操作
    fn toggle_sidebar(&mut self, cx: &mut Context<Self>) {
        self.sidebar_open = !self.sidebar_open;
        cx.notify();
    }
}

impl Render for App {
    /// 画面表示の処理
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // 自分自身のEntityを取得する
        let app = cx.entity();

        // 取引入力
        let transaction_form = SidebarMenuItem::new("取引入力")
            .active(self.page == Page::TransactionForm)
            .on_click({
                let app = app.clone();

                move |_, _, cx| {
                    app.update(cx, |app, cx| {
                        app.show_page(Page::TransactionForm, cx);
                    });
                }
            });

        // 取引一覧
        let transaction_list = SidebarMenuItem::new("取引一覧")
            .active(self.page == Page::TransactionList)
            .on_click({
                let app = app.clone();

                move |_, _, cx| {
                    app.update(cx, |app, cx| {
                        app.show_page(Page::TransactionList, cx);
                    });
                }
            });

        // 品目
        let items = SidebarMenuItem::new("品目")
            .active(self.page == Page::Items)
            .on_click({
                let app = app.clone();

                move |_, _, cx| {
                    app.update(cx, |app, cx| {
                        app.show_page(Page::Items, cx);
                    });
                }
            });

        div()
            .size_full()
            .flex()
            // サイドバー
            .child(
                Sidebar::new("sidebar")
                    .collapsed(!self.sidebar_open)
                    .collapsible(true)
                    .header(Button::new("toggle-sidebar").label("☰").on_click({
                        let app = app.clone();

                        move |_, _, cx| {
                            app.update(cx, |app, cx| {
                                app.toggle_sidebar(cx);
                            });
                        }
                    }))
                    .child(
                        SidebarMenu::new()
                            .gap_4()
                            .py_4()
                            .child(transaction_form)
                            .child(transaction_list)
                            .child(items),
                    ),
            )
            // メイン画面
            .child(div().flex_1().h_full().child(
                // 画面選択
                div().flex_1().child(match self.page {
                    Page::TransactionForm => div().child(self.transaction_form.clone()),
                    Page::TransactionList => div().child(self.transaction_list.clone()),
                    Page::Items => div().child(self.items.clone()),
                }),
            ))
    }
}

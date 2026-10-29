//! 画面切り替えと共有画面の作成

use crate::pages::{
    expense_pie::ExpensePie, items::Items, transaction_form::TransactionForm,
    transaction_list::TransactionList,
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
    ExpensePie,
    Items,
}

/// 画面の管理
pub struct App {
    transaction_form: Entity<TransactionForm>,
    transaction_list: Entity<TransactionList>,
    expense_pie: Entity<ExpensePie>,
    items: Entity<Items>,
    page: Page,
    sidebar_open: bool,
}

impl App {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self::with_ledger(Ledger::with_presets(), window, cx)
    }

    pub fn with_ledger(ledger: Ledger, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let state = cx.new(|_| AppState { ledger });
        let transaction_form = cx.new(|cx| TransactionForm::new(state.clone(), window, cx));

        let transaction_list = cx.new(|cx| TransactionList::new(state.clone(), cx));

        let expense_pie = cx.new(|cx| ExpensePie::new(state.clone(), cx));

        let items = cx.new(|_| Items::new(state.clone()));

        Self {
            transaction_form,
            transaction_list,
            expense_pie,
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

        let expense_pie = SidebarMenuItem::new("支出の内訳")
            .active(self.page == Page::ExpensePie)
            .on_click(cx.listener(|app, _, _, cx| {
                app.show_page(Page::ExpensePie, cx);
            }));

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
                            .child(expense_pie)
                            .child(items),
                    ),
            )
            // メイン画面
            .child(div().flex_1().h_full().child(
                // 画面選択
                div().size_full().child(match self.page {
                    Page::TransactionForm => div().child(self.transaction_form.clone()),
                    Page::TransactionList => div().child(self.transaction_list.clone()),
                    Page::ExpensePie => div().size_full().child(self.expense_pie.clone()),
                    Page::Items => div().child(self.items.clone()),
                }),
            ))
    }
}

#[cfg(test)]
mod tests {
    use super::{App, Page};
    use chrono::Local;
    use codo_core::period::YearMonth;
    use gpui_kit::test::TestWindowExt;
    use gpui_kit::{AppContext, Bounds, TestAppContext, WindowBounds, WindowOptions, px, size};

    #[gpui_kit::test]
    fn sidebar_opens_expense_pie(cx: &mut TestAppContext) {
        let month = YearMonth::from_date(Local::now().date_naive());
        cx.update(gpui_kit::init);
        let (handle, app) = cx.update(|cx| {
            cx.set_reduce_motion(true);
            gpui_kit::open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds {
                        origin: Default::default(),
                        size: size(px(900.), px(700.)),
                    })),
                    ..Default::default()
                },
                cx,
                |window, cx| cx.new(|cx| App::with_ledger(crate::demo::ledger(), window, cx)),
            )
            .unwrap()
        });
        cx.update_window(handle, |_, window, cx| {
            window.render_frame(cx);
            // Sidebar が割り当てる「グループ番号-項目番号」の ID でクリックする。
            assert!(window.find("0-2").visible());
            window.click("0-2", cx);
            assert!(app.read(cx).page == Page::ExpensePie);
            window.render_frame(cx);
            assert!(window.find("previous-month").visible());
            assert!(window.find("next-month").visible());
            let assert_month = |window: &gpui_kit::Window, expected: YearMonth, is_empty: bool| {
                let label = window.find("expense-month-label");
                assert!(label.visible());
                let expected_label = format!("{}年{}月", expected.year(), expected.month());
                assert_eq!(label.label(), Some(expected_label.as_str()));
                if is_empty {
                    assert!(window.find("expense-empty-state").visible());
                } else {
                    assert!(window.try_find("expense-empty-state").is_none());
                }
            };
            assert_month(window, month, false);
            window.click("previous-month", cx);
            window.render_frame(cx);
            assert_month(window, month.prev(), false);
            window.click("next-month", cx);
            window.render_frame(cx);
            assert_month(window, month, false);
            window.click("next-month", cx);
            window.render_frame(cx);
            assert_month(window, month.next(), true);
            window.click("previous-month", cx);
            window.render_frame(cx);
            assert_month(window, month, false);
        })
        .unwrap();
    }
}

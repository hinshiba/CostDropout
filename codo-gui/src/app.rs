//! 画面切り替えと共有画面の作成

use crate::pages::{
    expense_calendar::ExpenseCalendar, expense_pie::ExpensePie, items::Items,
    monthly_report::MonthlyReport, transaction_form::TransactionForm,
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
    MonthlyReport,
    ExpenseCalendar,
}

/// 画面の管理
pub struct App {
    transaction_form: Entity<TransactionForm>,
    transaction_list: Entity<TransactionList>,
    expense_pie: Entity<ExpensePie>,
    items: Entity<Items>,
    monthly_report: Entity<MonthlyReport>,
    expense_calendar: Entity<ExpenseCalendar>,
    page: Page,
    sidebar_open: bool,
}

impl App {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self::with_ledger(Ledger::with_presets(), window, cx)
    }

    fn with_ledger(ledger: Ledger, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let state = cx.new(|_| AppState { ledger });
        let transaction_form = cx.new(|cx| TransactionForm::new(state.clone(), window, cx));

        let transaction_list = cx.new(|cx| TransactionList::new(state.clone(), cx));

        let expense_pie = cx.new(|cx| ExpensePie::new(state.clone(), cx));

        let items = cx.new(|_| Items::new(state.clone()));

        let monthly_report = cx.new(|_| MonthlyReport::new(state.clone()));

        let expense_calendar = cx.new(|_| ExpenseCalendar::new(state.clone()));

        Self {
            transaction_form,
            transaction_list,
            expense_pie,
            items,
            monthly_report,
            expense_calendar,
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

        // 月の収支
        let monthly_report = SidebarMenuItem::new("月の収支")
            .active(self.page == Page::MonthlyReport)
            .on_click({
                let app = app.clone();

                move |_, _, cx| {
                    app.update(cx, |app, cx| {
                        app.show_page(Page::MonthlyReport, cx);
                    });
                }
            });

        // 支出カレンダー
        let expense_calendar = SidebarMenuItem::new("支出カレンダー")
            .active(self.page == Page::ExpenseCalendar)
            .on_click({
                let app = app.clone();

                move |_, _, cx| {
                    app.update(cx, |app, cx| {
                        app.show_page(Page::ExpenseCalendar, cx);
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
                            .child(items)
                            .child(monthly_report)
                            .child(expense_calendar),
                    ),
            )
            // メイン画面
            .child(div().flex_1().h_full().min_w_0().child(
                // 画面選択
                div().size_full().child(match self.page {
                    Page::TransactionForm => div().child(self.transaction_form.clone()),
                    Page::TransactionList => div().child(self.transaction_list.clone()),
                    Page::ExpensePie => div().size_full().child(self.expense_pie.clone()),
                    Page::Items => div().child(self.items.clone()),
                    Page::MonthlyReport => div().child(self.monthly_report.clone()),
                    Page::ExpenseCalendar => div().size_full().child(self.expense_calendar.clone()),
                }),
            ))
    }
}

#[cfg(test)]
mod tests {
    use super::{App, Page};
    use chrono::Local;
    use codo_core::period::YearMonth;
    use codo_core::{ledger::Ledger, money::Money, transaction::TransactionDraft};
    use gpui_kit::test::TestWindowExt;
    use gpui_kit::{AppContext, Bounds, TestAppContext, WindowBounds, WindowOptions, px, size};

    fn ledger_for_month_navigation(month: YearMonth) -> Ledger {
        let mut ledger = Ledger::with_presets();
        let food = ledger.items().find_by_name("食費").unwrap().id();
        let cash = ledger.items().find_by_name("現金").unwrap().id();
        for (month, amount) in [(month, 1200), (month.prev(), 800)] {
            ledger
                .add_transaction(TransactionDraft::new(
                    month.first_day(),
                    Money(amount),
                    food,
                    cash,
                    String::new(),
                ))
                .unwrap();
        }
        ledger
    }

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
                |window, cx| {
                    cx.new(|cx| App::with_ledger(ledger_for_month_navigation(month), window, cx))
                },
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

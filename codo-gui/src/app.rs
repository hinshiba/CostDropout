//! 画面切り替えと共有画面の作成

use crate::pages::{
    budget::BudgetPage, expense_bar_chart::ExpenseBarChart, expense_calendar::ExpenseCalendar,
    expense_pie::ExpensePie, items::Items, monthly_report::MonthlyReport,
    transaction_form::TransactionForm, transaction_list::TransactionList,
};
use chrono::Local;
use codo_core::ledger::Ledger;
use gpui_kit::component::button::Button;
use gpui_kit::component::date_picker::DatePickerState;
use gpui_kit::component::input::InputState;
use gpui_kit::component::sidebar::{Sidebar, SidebarMenu, SidebarMenuItem};
use gpui_kit::*;

/// ledgerの共有
pub struct AppState {
    pub ledger: Ledger,
}

/// 現在の画面のenum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    TransactionForm,
    TransactionList,
    ExpensePie,
    Items,
    ExpenseBarChart,
    MonthlyReport,
    Budget,
    ExpenseCalendar,
}

/// 画面の管理
pub struct App {
    transaction_form: Entity<TransactionForm>,
    transaction_list: Entity<TransactionList>,
    expense_pie: Entity<ExpensePie>,
    items: Entity<Items>,
    expense_bar_chart: Entity<ExpenseBarChart>,
    monthly_report: Entity<MonthlyReport>,
    expense_calendar: Entity<ExpenseCalendar>,
    page: Page,
    sidebar_open: bool,
    budget: Entity<BudgetPage>,
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

        let expense_bar_chart = cx.new(|_| ExpenseBarChart::new(state.clone()));

        let monthly_report = cx.new(|_| MonthlyReport::new(state.clone()));

        let budget = cx.new(|cx| {
            let amount = cx.new(|cx| InputState::new(window, cx).placeholder("予算額"));

            let start_date = cx.new(|cx| {
                let mut state_start = DatePickerState::new(window, cx);
                state_start.set_date(Local::now().date_naive(), window, cx);
                state_start
            });

            let end_date = cx.new(|cx| {
                let mut state_end = DatePickerState::new(window, cx);
                state_end.set_date(Local::now().date_naive(), window, cx);
                state_end
            });

            BudgetPage::new(state.clone(), amount, start_date, end_date)
        });
        let expense_calendar = cx.new(|_| ExpenseCalendar::new(state.clone()));

        Self {
            transaction_form,
            transaction_list,
            expense_pie,
            items,
            monthly_report,
            expense_calendar,
            page: Page::TransactionForm,
            expense_bar_chart,
            sidebar_open: true,
            budget,
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

        // 支出推移
        let expense_bar_chart = SidebarMenuItem::new("支出推移（棒グラフ）")
            .active(self.page == Page::ExpenseBarChart)
            .on_click({
                let app = app.clone();

                move |_, _, cx| {
                    app.update(cx, |app, cx| {
                        app.show_page(Page::ExpenseBarChart, cx);
                    });
                }
            });
        //予算
        let budget = SidebarMenuItem::new("予算")
            .active(self.page == Page::Budget)
            .on_click({
                let app = app.clone();

                move |_, _, cx| {
                    app.update(cx, |app, cx| {
                        app.show_page(Page::Budget, cx);
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
                        app.show_page(Page::ExpenseBarChart, cx);
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
                            .child(items)
                            .child(expense_bar_chart)
                            .child(expense_pie)
                            .child(monthly_report)
                            .child(budget)
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
                    Page::ExpenseBarChart => div().child(self.expense_bar_chart.clone()),
                    Page::MonthlyReport => div().child(self.monthly_report.clone()),
                    Page::Budget => div().child(self.budget.clone()),
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
    use codo_core::{
        item::Tag,
        ledger::Ledger,
        money::Money,
        transaction::{Amount, TransactionDraft},
    };
    use gpui_kit::test::TestWindowExt;
    use gpui_kit::{AppContext, Bounds, TestAppContext, WindowBounds, WindowOptions, px, size};

    fn ledger_for_month_navigation(month: YearMonth) -> Ledger {
        let mut ledger = Ledger::new();
        let food = ledger.add_item("食費", Tag::Expense).unwrap();
        let cash = ledger.add_item("現金", Tag::Asset).unwrap();
        for (month, amount) in [(month, 1200), (month.prev(), 800)] {
            ledger
                .add_transaction(TransactionDraft::new(
                    month.first_day(),
                    Amount::try_from(Money::from(amount)).unwrap(),
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
            window.click("0-4", cx);
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

    #[gpui_kit::test]
    fn sidebar_opens_expense_bar_chart_and_switches_total_modes(cx: &mut TestAppContext) {
        let mut ledger = Ledger::new();
        let food = ledger.add_item("食費", Tag::Expense).unwrap();
        let cash = ledger.add_item("現金", Tag::Asset).unwrap();

        let transactions = [
            // 2025年
            (chrono::NaiveDate::from_ymd_opt(2025, 1, 10).unwrap(), 1_000),
            (
                chrono::NaiveDate::from_ymd_opt(2025, 12, 10).unwrap(),
                3_000,
            ),
            // 2026年
            (chrono::NaiveDate::from_ymd_opt(2026, 1, 10).unwrap(), 4_000),
            (chrono::NaiveDate::from_ymd_opt(2026, 6, 10).unwrap(), 5_000),
            (
                chrono::NaiveDate::from_ymd_opt(2026, 12, 10).unwrap(),
                6_000,
            ),
        ];

        for (date, amount) in transactions {
            ledger
                .add_transaction(TransactionDraft::new(
                    date,
                    Amount::try_from(Money::from(amount)).unwrap(),
                    food,
                    cash,
                    String::new(),
                ))
                .unwrap();
        }

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
                |window, cx| cx.new(|cx| App::with_ledger(ledger, window, cx)),
            )
            .unwrap()
        });

        cx.update_window(handle, |_, window, cx| {
            window.render_frame(cx);

            // Sidebar の「支出推移」を開く。
            window.click("0-3", cx);
            window.render_frame(cx);

            assert_eq!(app.read(cx).page, Page::ExpenseBarChart);

            // 「合計」に切り替える。
            window.click("chart-mode-total", cx);
            window.render_frame(cx);

            assert!(window.find("total-mode-yearly").visible());
            assert!(window.find("total-mode-monthly").visible());

            // 初期状態は「年別」。
            assert!(window.find("total-mode-yearly").visible());

            // 「月別」に切り替える。
            window.click("total-mode-monthly", cx);
            window.render_frame(cx);

            // 月別では前年・翌年ボタンが表示される。
            assert!(window.find("previous-year").visible());
            assert!(window.find("next-year").visible());

            // 翌年へ移動できる。
            window.click("next-year", cx);
            window.render_frame(cx);

            // 前年へ戻ることができる。
            window.click("previous-year", cx);
            window.render_frame(cx);

            // 「年別」に戻せる。
            window.click("total-mode-yearly", cx);
            window.render_frame(cx);

            assert!(window.find("total-mode-yearly").visible());
            assert!(window.find("total-mode-monthly").visible());
        })
        .unwrap();
    }
}

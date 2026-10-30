//! 今日を含む予算の残額と支出ペースを表示する.
use crate::app::AppState;
use chrono::{Local, NaiveDate};
use codo_core::{budget::BudgetStatus, ledger::Ledger, period::Period};
use gpui_kit::base::TestSupportExt;
use gpui_kit::component::{ActiveTheme, progress::Progress};
use gpui_kit::*;

pub struct BudgetDashboard {
    state: Entity<AppState>,
    _subscription: Subscription,
}

impl BudgetDashboard {
    pub fn new(state: Entity<AppState>, cx: &mut Context<Self>) -> Self {
        let subscription = cx.observe(&state, |_, _, cx| cx.notify());
        Self {
            state,
            _subscription: subscription,
        }
    }
}

/// 開始日・終了日・今日を含めて数える.
fn elapsed_days(period: Period, today: NaiveDate) -> (i64, i64) {
    let total = (period.end() - period.start()).num_days() + 1;
    let elapsed = ((today - period.start()).num_days() + 1).clamp(0, total);
    (elapsed, total)
}

fn active_budgets(ledger: &Ledger, today: NaiveDate) -> Vec<BudgetStatus> {
    ledger
        .budgets()
        .iter()
        .filter(|budget| budget.period().contains(today))
        .map(|budget| {
            BudgetStatus::calculate(ledger, budget.id())
                .expect("帳簿から取得した予算は必ず存在する")
        })
        .collect()
}

fn budget_card(status: &BudgetStatus, today: NaiveDate, cx: &App) -> impl IntoElement {
    let budget = status.budget();
    let id = budget.id().value();
    let period = budget.period();
    let (elapsed, total) = elapsed_days(period, today);
    let elapsed_ratio = elapsed as f64 / total as f64;
    let is_over = status.remaining().is_negative();
    let is_fast = status.ratio().is_some_and(|ratio| elapsed_ratio < ratio);
    let (color, message) = if is_over {
        (rgb(0xb42318), "予算を超過しています")
    } else if is_fast {
        (rgb(0x946200), "支出ペースが期間の経過を上回っています")
    } else {
        (rgb(0x18794e), "予算内のペースです")
    };
    let spending_label = match status.ratio() {
        Some(ratio) => format!("支出割合  {:.0}%", ratio * 100.),
        None => "支出割合  —（予算額が0円）".to_string(),
    };

    let spending_progress = status
        .ratio()
        .unwrap_or(if is_over { 1. } else { 0. })
        .clamp(0., 1.)
        * 100.;
    let metric = |label: &str, value: String, value_color: Hsla| {
        div()
            .flex_1()
            .min_w(px(120.))
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(label.to_string()),
            )
            .child(div().text_2xl().text_color(value_color).child(value))
    };
    div()
        .id(format!("budget-card-{id}"))
        .test_support()
        .flex()
        .flex_col()
        .flex_shrink_0()
        .gap_4()
        .p_5()
        .rounded_lg()
        .border_1()
        .border_color(if is_over {
            color.into()
        } else {
            cx.theme().border
        })
        .bg(cx.theme().background)
        .child(
            div()
                .text_lg()
                .child(format!("{} 〜 {}", period.start(), period.end())),
        )
        .child(
            div()
                .flex()
                .flex_wrap()
                .gap_4()
                .child(metric(
                    "予算額",
                    budget.amount().to_string(),
                    cx.theme().foreground,
                ))
                .child(metric(
                    "支出済み",
                    status.spent().to_string(),
                    cx.theme().foreground,
                ))
                .child(metric("残り", status.remaining().to_string(), color.into())),
        )
        .child(
            div()
                .flex()
                .flex_wrap()
                .gap_4()
                .child(
                    div()
                        .flex_1()
                        .min_w(px(240.))
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(format!("期間の経過  {:.0}%", elapsed_ratio * 100.))
                        .child(
                            Progress::new(format!("budget-time-{id}"))
                                .accessibility_label("期間の経過割合")
                                .value((elapsed_ratio * 100.) as f32)
                                .color(rgb(0x64748b)),
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(cx.theme().muted_foreground)
                                .child(format!("{total}日中{elapsed}日経過（今日を含む）")),
                        ),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w(px(240.))
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(
                            div()
                                .id(format!("budget-percentage-{id}"))
                                .test_support()
                                .aria_label(spending_label.clone())
                                .child(spending_label),
                        )
                        .child(
                            Progress::new(format!("budget-spent-{id}"))
                                .accessibility_label("予算の支出割合")
                                .value(spending_progress as f32)
                                .color(color),
                        ),
                ),
        )
        .child(
            div()
                .id(format!("budget-status-{id}"))
                .test_support()
                .aria_label(message)
                .text_sm()
                .text_color(color)
                .child(message),
        )
}

impl Render for BudgetDashboard {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let today = Local::now().date_naive();
        let statuses = active_budgets(&self.state.read(cx).ledger, today);
        let content = if statuses.is_empty() {
            div()
                .id("budget-empty-state")
                .test_support()
                .p_6()
                .rounded_lg()
                .border_1()
                .border_color(cx.theme().border)
                .flex()
                .flex_col()
                .gap_2()
                .child(div().text_lg().child("予算がありません"))
                .child("予算設定画面で、期間と予算額を設定してください。")
                .into_any_element()
        } else {
            div()
                .flex()
                .flex_col()
                .gap_4()
                .children(statuses.iter().map(|status| budget_card(status, today, cx)))
                .into_any_element()
        };
        div()
            .id("budget-dashboard")
            .size_full()
            .overflow_y_scroll()
            .p_6()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .child(div().text_2xl().child("予算ダッシュボード"))
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!("{} 予算の節約状況", today)),
                    )
                    .child(content),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::{active_budgets, elapsed_days};
    use chrono::NaiveDate;
    use codo_core::{ledger::Ledger, money::Money, period::Period};

    #[test]
    fn counts_inclusive_days_and_clamps_outside_period() {
        let date = |day| NaiveDate::from_ymd_opt(2026, 10, day).unwrap();
        let period = Period::new(date(1), date(31)).unwrap();
        assert_eq!(elapsed_days(period, date(1)), (1, 31));
        assert_eq!(elapsed_days(period, date(10)), (10, 31));
        assert_eq!(elapsed_days(period, date(31)), (31, 31));
        let single = Period::new(date(10), date(10)).unwrap();
        assert_eq!(elapsed_days(single, date(9)), (0, 1));
        assert_eq!(elapsed_days(single, date(10)), (1, 1));
        assert_eq!(elapsed_days(single, date(11)), (1, 1));
    }

    #[test]
    fn shows_all_overlapping_budgets_including_boundaries() {
        let date = |day| NaiveDate::from_ymd_opt(2026, 10, day).unwrap();
        let mut ledger = Ledger::new();
        for (start, end) in [(1, 9), (1, 10), (10, 10), (10, 31), (11, 31)] {
            ledger
                .add_budget(
                    Money::from(1000),
                    Period::new(date(start), date(end)).unwrap(),
                )
                .unwrap();
        }
        let ids: Vec<_> = active_budgets(&ledger, date(10))
            .iter()
            .map(|status| status.budget().id().value())
            .collect();
        assert_eq!(ids, vec![2, 3, 4]);
        assert!(active_budgets(&Ledger::new(), date(10)).is_empty());
    }

    #[gpui_kit::test]
    fn renders_status_and_updates_after_ledger_changes(cx: &mut gpui_kit::TestAppContext) {
        use super::BudgetDashboard;
        use crate::app::AppState;
        use chrono::{Days, Local};
        use codo_core::transaction::TransactionDraft;
        use gpui_kit::test::TestWindowExt;
        use gpui_kit::{AppContext, Bounds, WindowBounds, WindowOptions, px, size};

        let today = Local::now().date_naive();
        let period = Period::new(today - Days::new(9), today + Days::new(21)).unwrap();
        let mut ledger = Ledger::with_presets();
        let budget_id = ledger.add_budget(Money::from(10000), period).unwrap();
        let food = ledger.items().find_by_name("食費").unwrap().id();
        let cash = ledger.items().find_by_name("現金").unwrap().id();
        ledger
            .add_transaction(TransactionDraft::new(
                today,
                Money::from(2000),
                food,
                cash,
                String::new(),
            ))
            .unwrap();
        cx.update(gpui_kit::init);
        let state = cx.new(|_| AppState { ledger });
        let (handle, _) = cx.update(|cx| {
            cx.set_reduce_motion(true);
            gpui_kit::open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds {
                        origin: Default::default(),
                        size: size(px(800.), px(600.)),
                    })),
                    ..Default::default()
                },
                cx,
                |_, cx| cx.new(|cx| BudgetDashboard::new(state.clone(), cx)),
            )
            .unwrap()
        });
        cx.update_window(handle, |_, window, cx| {
            for (amount, expected, percentage) in [
                (10000, "予算内のペースです", "支出割合  20%"),
                (
                    2500,
                    "支出ペースが期間の経過を上回っています",
                    "支出割合  80%",
                ),
                (1000, "予算を超過しています", "支出割合  200%"),
                (0, "予算を超過しています", "支出割合  —（予算額が0円）"),
            ] {
                state.update(cx, |state, cx| {
                    state
                        .ledger
                        .update_budget(budget_id, Money::from(amount), period)
                        .unwrap();
                    cx.notify();
                });
                window.render_frame(cx);
                assert!(window.find("budget-card-1").visible());
                assert_eq!(window.find("budget-status-1").label(), Some(expected));
                assert_eq!(window.find("budget-percentage-1").label(), Some(percentage));
            }
            state.update(cx, |state, cx| {
                state.ledger.remove_budget(budget_id).unwrap();
                cx.notify();
            });
            window.render_frame(cx);
            assert!(window.find("budget-empty-state").visible());
            assert!(window.try_find("budget-card-1").is_none());
        })
        .unwrap();
    }
}

//! 月ごとの支出の品目割合を表示する画面

use crate::app::AppState;
use chrono::Local;
use codo_core::ledger::Ledger;
use codo_core::money::Money;
use codo_core::period::{Period, YearMonth};
use codo_core::report::category::expense_by_item;
use gpui_kit::component::{ActiveTheme, button::Button, chart::PieChart};
use gpui_kit::*;

struct ExpenseSlice {
    name: String,
    amount: Money,
    percentage: f64,
    color: Hsla,
}

impl ExpenseSlice {
    fn for_month(ledger: &Ledger, month: YearMonth) -> Vec<Self> {
        expense_by_item(ledger, Period::month(month))
            .into_iter()
            .filter_map(|expense| {
                let id = expense.item();
                let item = ledger.items().get_item(id)?;
                Some(Self {
                    name: item.name().to_string(),
                    amount: expense.amount(),
                    percentage: expense.ratio() * 100.,
                    // 同じ品目は月を切り替えても同じ色にする。
                    color: hsla(
                        (f64::from(id.id()) * 0.618_034).fract() as f32,
                        0.65,
                        0.5,
                        1.,
                    ),
                })
            })
            .collect()
    }
}

pub struct ExpensePie {
    state: Entity<AppState>,
    month: YearMonth,
    _state_subscription: Subscription,
}

impl ExpensePie {
    pub fn new(state: Entity<AppState>, cx: &mut Context<Self>) -> Self {
        let subscription = cx.observe(&state, |_, _, cx| cx.notify());
        Self {
            state,
            month: YearMonth::from_date(Local::now().date_naive()),
            _state_subscription: subscription,
        }
    }
}

impl Render for ExpensePie {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // 再表示時にも必ず最新の帳簿から集計する。
        let slices = ExpenseSlice::for_month(&self.state.read(cx).ledger, self.month);
        let content = if slices.is_empty() {
            div()
                .py_8()
                .text_color(cx.theme().muted_foreground)
                .child("この月の支出はありません")
        } else {
            let legend = div()
                .flex()
                .flex_col()
                .gap_3()
                .children(slices.iter().map(|slice| {
                    div()
                        .flex()
                        .items_center()
                        .gap_3()
                        .child(
                            div()
                                .size_3()
                                .flex_shrink_0()
                                .rounded_full()
                                .bg(slice.color),
                        )
                        .child(div().flex_1().child(slice.name.clone()))
                        .child(slice.amount.to_string())
                        .child(
                            div()
                                .w(px(80.))
                                .text_right()
                                .child(format!("{:.1}%", slice.percentage)),
                        )
                }));
            div()
                .flex()
                .flex_col()
                .gap_6()
                .child(
                    div().w_full().h(px(320.)).child(
                        PieChart::new(slices)
                            .inner_radius(0.)
                            .value(|slice| slice.amount.0 as f32)
                            .color(|slice| slice.color)
                            .tooltip_name(|slice| slice.name.clone().into())
                            .tooltip_value(|slice, _, _| {
                                format!("{} ({:.1}%)", slice.amount, slice.percentage).into()
                            }),
                    ),
                )
                .child(legend)
        };

        div()
            .id("expense-pie-page")
            .size_full()
            .overflow_y_scroll()
            .p_6()
            .flex()
            .flex_col()
            .gap_6()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(div().text_xl().child("支出の内訳"))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_4()
                    .child(
                        Button::new("previous-month")
                            .label("前月")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.month = this.month.prev();
                                cx.notify();
                            })),
                    )
                    .child(format!("{}年{}月", self.month.year(), self.month.month()))
                    .child(
                        Button::new("next-month")
                            .label("翌月")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.month = this.month.next();
                                cx.notify();
                            })),
                    ),
            )
            .child(content.flex_shrink_0())
    }
}

#[cfg(test)]
mod tests {
    use super::{ExpensePie, ExpenseSlice};
    use crate::app::AppState;
    use codo_core::item::Tag;
    use codo_core::ledger::Ledger;
    use codo_core::money::Money;
    use codo_core::period::YearMonth;
    use codo_core::transaction::TransactionDraft;
    use gpui_kit::{AppContext, TestAppContext};
    use std::{cell::Cell, rc::Rc};

    #[gpui_kit::test]
    fn transaction_changes_notify_chart_and_refresh_monthly_slices(cx: &mut TestAppContext) {
        let mut ledger = Ledger::new();
        let food = ledger.add_item("食費", Tag::Expense).unwrap();
        let leisure = ledger.add_item("娯楽", Tag::Expense).unwrap();
        let cash = ledger.add_item("現金", Tag::Asset).unwrap();
        let month = YearMonth::new(2026, 12).unwrap();
        let state = cx.new(|_| AppState { ledger });
        let page = cx.new(|cx| ExpensePie::new(state.clone(), cx));
        page.update(cx, |page, _| page.month = month);
        let notified = Rc::new(Cell::new(false));
        let _subscription = cx.update(|cx| {
            cx.observe(&page, {
                let notified = notified.clone();
                move |_, _| notified.set(true)
            })
        });

        assert!(
            cx.read(|cx| { ExpenseSlice::for_month(&state.read(cx).ledger, month).is_empty() })
        );
        state.update(cx, |state, cx| {
            for (item, amount) in [(food, 100), (leisure, 300)] {
                state
                    .ledger
                    .add_transaction(TransactionDraft::new(
                        month.first_day(),
                        Money(amount),
                        item,
                        cash,
                        String::new(),
                    ))
                    .unwrap();
            }
            cx.notify();
        });
        cx.run_until_parked();
        assert!(notified.replace(false));
        let slices =
            cx.read(|cx| ExpenseSlice::for_month(&state.read(cx).ledger, page.read(cx).month));
        assert_eq!(
            (
                slices[0].name.as_str(),
                slices[0].amount,
                slices[0].percentage
            ),
            ("娯楽", Money(300), 75.)
        );
        assert_eq!(
            (
                slices[1].name.as_str(),
                slices[1].amount,
                slices[1].percentage
            ),
            ("食費", Money(100), 25.)
        );

        state.update(cx, |state, cx| {
            state
                .ledger
                .add_transaction(TransactionDraft::new(
                    month.last_day(),
                    Money(200),
                    food,
                    cash,
                    String::new(),
                ))
                .unwrap();
            cx.notify();
        });
        cx.run_until_parked();
        assert!(notified.get());
        let updated = cx.read(|cx| ExpenseSlice::for_month(&state.read(cx).ledger, month));
        assert!(
            updated
                .iter()
                .all(|slice| slice.amount == Money(300) && slice.percentage == 50.)
        );

        page.update(cx, |page, _| page.month = page.month.next());
        assert!(cx.read(|cx| {
            ExpenseSlice::for_month(&state.read(cx).ledger, page.read(cx).month).is_empty()
        }));
        page.update(cx, |page, _| page.month = page.month.prev());
        assert_eq!(
            cx.read(
                |cx| ExpenseSlice::for_month(&state.read(cx).ledger, page.read(cx).month).len()
            ),
            2
        );
    }
}

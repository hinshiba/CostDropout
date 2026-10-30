//! 取引の日付・金額・品目を入力して帳簿に追加する画面。

use crate::app::AppState;
use crate::transaction_fields::{FieldsChanged, FormError, TransactionFields, friendly_error};
use gpui_kit::component::button::Button;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;
/// 共有帳簿に取引を追加する画面。
pub struct TransactionForm {
    state: Entity<AppState>,
    fields: Entity<TransactionFields>,
    status: Option<Result<(), FormError>>,
    scroll_handle: ScrollHandle,
    _subscriptions: Vec<Subscription>,
}

impl TransactionForm {
    pub fn new(state: Entity<AppState>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let fields = cx.new(|cx| TransactionFields::new(state.clone(), window, cx));
        let subscriptions = vec![
            cx.observe(&state, |_, _, cx| cx.notify()),
            cx.subscribe(&fields, |this, _, _: &FieldsChanged, cx| {
                this.reset_feedback(cx);
            }),
        ];
        Self {
            state,
            fields,
            status: None,
            scroll_handle: ScrollHandle::new(),
            _subscriptions: subscriptions,
        }
    }

    /// 入力の変更前に表示していたエラーと追加成功メッセージを消す。
    fn reset_feedback(&mut self, cx: &mut Context<Self>) {
        self.status = None;
        cx.notify();
    }

    fn add_transaction(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let draft = self.fields.read(cx).draft(cx);
        let result = self.state.update(cx, |state, cx| {
            state.ledger_mut().add_transaction(draft?)?;
            cx.notify();
            Ok(())
        });
        let amount_error = match &result {
            Err(error @ FormError::Amount(_)) => Some(friendly_error(error)),
            _ => None,
        };
        let is_added = result.is_ok();
        self.status = Some(result);
        self.fields.update(cx, |fields, cx| {
            fields.set_amount_error(amount_error, cx);
            if is_added {
                fields.clear_amount_and_comment(window, cx);
            }
        });
        cx.notify();
    }
}

impl Render for TransactionForm {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let count = self.state.read(cx).ledger().transactions().iter().count();
        let count_label: SharedString = format!("登録済みの取引: {count}件").into();
        let form_error = match &self.status {
            Some(Err(FormError::Amount(_))) => None,
            Some(Err(error)) => Some(friendly_error(error)),
            _ => None,
        };
        div()
            .id("transaction-form-scroll")
            .size_full()
            .overflow_y_scroll()
            .track_scroll(&self.scroll_handle)
            .p_6()
            .child(
                div()
                    .w_full()
                    .max_w(px(560.))
                    .flex()
                    .flex_col()
                    .gap_4()
                    .child(div().text_xl().child("取引入力"))
                    .child(
                        div()
                            .id("transaction-count")
                            .test_support()
                            .aria_label(count_label.clone())
                            .child(count_label),
                    )
                    .child(self.fields.clone())
                    .when_some(form_error, |this, message| {
                        this.child(div().text_color(rgb(0xc62828)).child(message))
                    })
                    .when(matches!(&self.status, Some(Ok(()))), |this| {
                        this.child(div().text_color(rgb(0x2e7d32)).child("取引を追加しました"))
                    })
                    .child(Button::new("add-transaction").label("追加").on_click(
                        cx.listener(|this, _, window, cx| this.add_transaction(window, cx)),
                    )),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::TransactionForm;
    use crate::{app::AppState, pages::expense_pie::ExpensePie};
    use chrono::Local;
    use codo_core::ledger::{
        Ledger,
        item::Tag,
        money::Money,
        period::{Period, YearMonth},
        report::{Flow, category::by_item},
    };
    use gpui_kit::test::TestWindowExt;
    use gpui_kit::{AppContext, TestAppContext, WindowOptions};
    use std::{cell::Cell, rc::Rc};

    #[gpui_kit::test]
    fn adding_transaction_notifies_expense_chart(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        let mut ledger = Ledger::new();
        let food = ledger.add_item("食費", Tag::Expense).unwrap();
        let cash = ledger.add_item("現金", Tag::Asset).unwrap();
        let state = cx.new(|_| AppState::for_test(ledger));
        let chart = cx.new(|cx| ExpensePie::new(state.clone(), cx));
        let notified = Rc::new(Cell::new(false));
        let _subscription = cx.update(|cx| {
            cx.observe(&chart, {
                let notified = notified.clone();
                move |_, _| notified.set(true)
            })
        });
        let (handle, form) = cx.update(|cx| {
            gpui_kit::open_window(WindowOptions::default(), cx, |window, cx| {
                cx.new(|cx| TransactionForm::new(state.clone(), window, cx))
            })
            .unwrap()
        });
        cx.update_window(handle, |_, window, cx| {
            let fields = form.read(cx).fields.clone();
            fields.update(cx, |fields, cx| {
                fields.fill_for_test(food, cash, "1,200", window, cx);
            });
            window.render_frame(cx);
            window.click("add-transaction", cx);
            assert!(matches!(form.read(cx).status, Some(Ok(()))));
            // 追加に成功したら, 金額とコメントは空になる
            let values = fields.read(cx).values(cx);
            assert_eq!(values.amount, "");
            assert_eq!(values.comment, "");
            // 品目と相手先は続けて入力できるように残る
            assert_eq!(values.item, Some(food));
            assert_eq!(values.counterpart, Some(cash));
        })
        .unwrap();
        cx.run_until_parked();
        assert!(notified.get());
        cx.read(|cx| {
            let expenses = by_item(
                state.read(cx).ledger(),
                Period::month(YearMonth::from_date(Local::now().date_naive())),
                Flow::Expense,
            );
            assert_eq!(expenses.len(), 1);
            assert_eq!(expenses[0].amount(), Money::from(1200));
            assert_eq!(expenses[0].ratio(), 1.);
        });
    }
}

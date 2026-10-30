//! 取引の日付・金額・品目を入力して帳簿に追加する画面。

use crate::app::AppState;
use chrono::Local;
use codo_core::item::{Item, ItemId};
use codo_core::ledger::LedgerError;
use codo_core::transaction::{AmountError, TransactionDraft, TransactionError};
use gpui_kit::component::button::Button;
use gpui_kit::component::date_picker::{DatePicker, DatePickerEvent, DatePickerState};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::select::{Select, SelectEvent, SelectItem, SelectState};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

#[derive(Debug, Clone, PartialEq, Eq)]
struct ItemChoice {
    id: ItemId,
    name: SharedString,
}

impl SelectItem for ItemChoice {
    type Value = ItemId;

    fn title(&self) -> SharedString {
        self.name.clone()
    }

    fn value(&self) -> &ItemId {
        &self.id
    }
}

impl From<&Item> for ItemChoice {
    fn from(item: &Item) -> Self {
        Self {
            id: item.id(),
            name: item.to_string().into(),
        }
    }
}

/// 取引の追加で起こるエラー。
#[derive(Debug)]
enum FormError {
    Amount(AmountError),
    MissingDate,
    MissingItem,
    MissingCounterpart,
    Ledger(LedgerError),
}

impl From<AmountError> for FormError {
    fn from(error: AmountError) -> Self {
        Self::Amount(error)
    }
}

impl From<LedgerError> for FormError {
    fn from(error: LedgerError) -> Self {
        Self::Ledger(error)
    }
}

/// コアのエラーを、画面利用者向けのメッセージに変換する。
fn friendly_error(error: &FormError) -> String {
    match error {
        FormError::Ledger(error) => match error {
            LedgerError::TransactionItemNotFound(_) => {
                "選択した品目または相手先が見つかりません。選び直してください".into()
            }
            LedgerError::Transaction(TransactionError::InvalidItemTag(tag)) => format!(
                "{tag}の品目は取引の品目として選べません。収入・支出・負債から選んでください"
            ),
            LedgerError::Transaction(TransactionError::SameTag(tag)) => {
                format!("品目と相手先がどちらも{tag}です。異なる分類の品目を選んでください")
            }
            _ => "取引を追加できませんでした。品目と相手先を確認してください".into(),
        },
        FormError::Amount(error) => error.to_string(),
        FormError::MissingDate => "日付を選択してください".into(),
        FormError::MissingItem => "品目を選択してください".into(),
        FormError::MissingCounterpart => "相手先を選択してください".into(),
    }
}

struct FormControls {
    date: Entity<DatePickerState>,
    amount: Entity<InputState>,
    item: Entity<SelectState<Vec<ItemChoice>>>,
    counterpart: Entity<SelectState<Vec<ItemChoice>>>,
    comment: Entity<InputState>,
    item_choices: Vec<ItemChoice>,
    counterpart_choices: Vec<ItemChoice>,
}

/// 共有帳簿に取引を追加する画面。
pub struct TransactionForm {
    state: Entity<AppState>,
    controls: FormControls,
    status: Option<Result<(), FormError>>,
    scroll_handle: ScrollHandle,
    _subscriptions: Vec<Subscription>,
}

impl TransactionForm {
    pub fn new(state: Entity<AppState>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let date = cx.new(|cx| {
            let mut state = DatePickerState::new(window, cx).date_format("%Y-%m-%d");
            state.set_date(Local::now().date_naive(), window, cx);
            state
        });
        let amount = cx.new(|cx| InputState::new(window, cx).placeholder("例: 1,234"));
        let comment = cx.new(|cx| InputState::new(window, cx).placeholder("任意"));
        let item = cx.new(|cx| SelectState::new(Vec::<ItemChoice>::new(), None, window, cx));
        let counterpart = cx.new(|cx| SelectState::new(Vec::<ItemChoice>::new(), None, window, cx));
        let mut subscriptions = vec![cx.observe_in(&state, window, |this, _, window, cx| {
            this.refresh_choices(window, cx);
            cx.notify();
        })];
        subscriptions.push(cx.subscribe_in(
            &item,
            window,
            |this, _, _: &SelectEvent<Vec<ItemChoice>>, window, cx| {
                this.refresh_choices(window, cx);
                this.reset_feedback(cx);
            },
        ));
        subscriptions.push(cx.subscribe_in(
            &counterpart,
            window,
            |this, _, _: &SelectEvent<Vec<ItemChoice>>, _, cx| {
                this.reset_feedback(cx);
            },
        ));
        for input in [&amount, &comment] {
            subscriptions.push(cx.subscribe_in(
                input,
                window,
                |this, _, event: &InputEvent, _, cx| {
                    if matches!(event, InputEvent::Change) {
                        this.reset_feedback(cx);
                    }
                },
            ));
        }
        subscriptions.push(cx.subscribe_in(
            &date,
            window,
            |this, _, _: &DatePickerEvent, _, cx| {
                this.reset_feedback(cx);
            },
        ));
        let mut form = Self {
            state,
            controls: FormControls {
                date,
                amount,
                item,
                counterpart,
                comment,
                item_choices: Vec::new(),
                counterpart_choices: Vec::new(),
            },
            status: None,
            scroll_handle: ScrollHandle::new(),
            _subscriptions: subscriptions,
        };
        form.refresh_choices(window, cx);
        form
    }

    /// 入力の変更前に表示していたエラーと追加成功メッセージを消す。
    fn reset_feedback(&mut self, cx: &mut Context<Self>) {
        self.status = None;
        cx.notify();
    }

    /// 品目管理での編集・削除を反映し、無効になった選択を解除する。
    fn refresh_choices(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let controls = &mut self.controls;
        let ledger = &self.state.read(cx).ledger;
        let items = ledger
            .item_choices()
            .map(ItemChoice::from)
            .collect::<Vec<_>>();
        let selected_item = controls.item.read(cx).selected_value().copied();
        let counterparts = selected_item
            .map(|id| {
                ledger
                    .counterpart_choices(id)
                    .map(ItemChoice::from)
                    .collect()
            })
            .unwrap_or_default();
        for (select, cached, choices) in [
            (&controls.item, &mut controls.item_choices, items),
            (
                &controls.counterpart,
                &mut controls.counterpart_choices,
                counterparts,
            ),
        ] {
            if *cached == choices {
                continue;
            }
            let selected = select.read(cx).selected_value().copied();
            select.update(cx, |state, cx| {
                state.set_items(choices.clone(), window, cx);
                if let Some(id) = selected.filter(|id| choices.iter().any(|item| item.id == *id)) {
                    state.set_selected_value(&id, window, cx);
                } else {
                    state.set_selected_index(None, window, cx);
                }
                cx.notify();
            });
            *cached = choices;
        }
    }

    fn add_transaction(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let controls = &self.controls;
        let date = controls
            .date
            .read(cx)
            .date_time()
            .start()
            .map(|date| date.date());
        let amount = controls.amount.read(cx).value().to_string();
        let item = controls.item.read(cx).selected_value().copied();
        let counterpart = controls.counterpart.read(cx).selected_value().copied();
        let comment = controls.comment.read(cx).value().to_string();
        let result = self.state.update(cx, |state, cx| {
            let amount = amount.parse()?;
            let draft = TransactionDraft::new(
                date.ok_or(FormError::MissingDate)?,
                amount,
                item.ok_or(FormError::MissingItem)?,
                counterpart.ok_or(FormError::MissingCounterpart)?,
                comment,
            );
            state.ledger.add_transaction(draft)?;
            cx.notify();
            Ok(())
        });
        self.status = Some(result);
        if matches!(&self.status, Some(Ok(()))) {
            for input in [&controls.amount, &controls.comment] {
                input.update(cx, |state, cx| state.set_value("", window, cx));
            }
        }
        cx.notify();
    }
}

impl Render for TransactionForm {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let controls = &self.controls;
        let count = self.state.read(cx).ledger.transactions().iter().count();
        let amount_error = match &self.status {
            Some(Err(error @ FormError::Amount(_))) => Some(friendly_error(error)),
            _ => None,
        };
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
                    .child(format!("登録済みの取引: {count}件"))
                    .child(div().child("日付").child(DatePicker::new(&controls.date)))
                    .child(
                        div()
                            .child("金額（円）")
                            .child(Input::new(&controls.amount))
                            .when_some(amount_error, |this, message| {
                                this.child(div().text_color(rgb(0xc62828)).child(message))
                            }),
                    )
                    .child(
                        div()
                            .child("品目")
                            .child(Select::new(&controls.item).placeholder("品目を選択")),
                    )
                    .child(
                        div().child("相手先").child(
                            Select::new(&controls.counterpart)
                                .placeholder("相手先を選択")
                                .disabled(controls.item.read(cx).selected_value().is_none()),
                        ),
                    )
                    .child(div().child("コメント").child(Input::new(&controls.comment)))
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
    use codo_core::{
        item::Tag,
        ledger::Ledger,
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
        let state = cx.new(|_| AppState { ledger });
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
            form.update(cx, |form, cx| {
                form.controls.item.update(cx, |select, cx| {
                    select.set_selected_value(&food, window, cx)
                });
                form.refresh_choices(window, cx);
                form.controls.counterpart.update(cx, |select, cx| {
                    select.set_selected_value(&cash, window, cx)
                });
                form.controls
                    .amount
                    .update(cx, |input, cx| input.set_value("1,200", window, cx));
            });
            window.render_frame(cx);
            window.click("add-transaction", cx);
            assert!(matches!(form.read(cx).status, Some(Ok(()))));
        })
        .unwrap();
        cx.run_until_parked();
        assert!(notified.get());
        cx.read(|cx| {
            let expenses = by_item(
                &state.read(cx).ledger,
                Period::month(YearMonth::from_date(Local::now().date_naive())),
                Flow::Expense,
            );
            assert_eq!(expenses.len(), 1);
            assert_eq!(expenses[0].amount(), Money::from(1200));
            assert_eq!(expenses[0].ratio(), 1.);
        });
    }
}

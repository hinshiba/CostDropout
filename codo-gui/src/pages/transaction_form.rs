//! 取引の日付・金額・品目を入力して帳簿に追加する画面。

use crate::app::AppState;
use chrono::{Local, NaiveDate};
use codo_core::item::{ItemId, Tag};
use codo_core::ledger::{Ledger, LedgerError};
use codo_core::money::Money;
use codo_core::transaction::{TransactionDraft, TransactionError};
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
    tag: Tag,
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

/// 相手先の場合は選択中の品目と異なるタグだけを返す。
fn item_choices(ledger: &Ledger, item_tag: Option<Tag>, is_counterpart: bool) -> Vec<ItemChoice> {
    ledger
        .items()
        .iter()
        .filter(|item| {
            if is_counterpart {
                item_tag.is_some_and(|tag| item.tag() != tag)
            } else {
                item.tag().is_transaction_selectable()
            }
        })
        .map(|item| ItemChoice {
            id: item.id(),
            name: item.name().to_string().into(),
            tag: item.tag(),
        })
        .collect()
}

#[derive(Debug, PartialEq, Eq)]
enum FormError {
    Amount(String),
    Other(String),
}

/// 入力を検証し、成功した場合だけ帳簿を更新する。
fn submit_transaction(
    ledger: &mut Ledger,
    date: Option<NaiveDate>,
    amount: &str,
    item: Option<ItemId>,
    counterpart: Option<ItemId>,
    comment: String,
) -> Result<(), FormError> {
    let amount = amount
        .parse::<Money>()
        .map_err(|error| FormError::Amount(error.to_string()))?;
    let date = date.ok_or_else(|| FormError::Other("日付を選択してください".into()))?;
    let item = item.ok_or_else(|| FormError::Other("品目を選択してください".into()))?;
    let counterpart =
        counterpart.ok_or_else(|| FormError::Other("相手先を選択してください".into()))?;

    ledger
        .add_transaction(TransactionDraft::new(
            date,
            amount,
            item,
            counterpart,
            comment,
        ))
        .map(|_| ())
        .map_err(|error| match error {
            LedgerError::Transaction(TransactionError::NonPositiveAmount(_)) => {
                FormError::Amount(error.to_string())
            }
            error => FormError::Other(error.to_string()),
        })
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
    controls: Option<FormControls>,
    error: Option<FormError>,
    has_added: bool,
    scroll_handle: ScrollHandle,
    subscriptions: Vec<Subscription>,
}

impl TransactionForm {
    pub fn new(state: Entity<AppState>, cx: &mut Context<Self>) -> Self {
        let subscription = cx.observe(&state, |_, _, cx| cx.notify());
        Self {
            state,
            controls: None,
            error: None,
            has_added: false,
            scroll_handle: ScrollHandle::new(),
            subscriptions: vec![subscription],
        }
    }

    fn initialize_controls(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.controls.is_some() {
            return;
        }
        let date = cx.new(|cx| {
            let mut state = DatePickerState::new(window, cx).date_format("%Y-%m-%d");
            state.set_date(Local::now().date_naive(), window, cx);
            state
        });
        let amount = cx.new(|cx| InputState::new(window, cx).placeholder("例: 1,234"));
        let comment = cx.new(|cx| InputState::new(window, cx).placeholder("任意"));
        let item = cx.new(|cx| SelectState::new(Vec::<ItemChoice>::new(), None, window, cx));
        let counterpart = cx.new(|cx| SelectState::new(Vec::<ItemChoice>::new(), None, window, cx));
        self.subscriptions.push(cx.subscribe_in(
            &item,
            window,
            |this, _, _: &SelectEvent<Vec<ItemChoice>>, window, cx| {
                this.refresh_choices(window, cx);
                this.reset_feedback(cx);
            },
        ));
        self.subscriptions.push(cx.subscribe_in(
            &counterpart,
            window,
            |this, _, _: &SelectEvent<Vec<ItemChoice>>, _, cx| {
                this.reset_feedback(cx);
            },
        ));
        for input in [&amount, &comment] {
            self.subscriptions.push(cx.subscribe_in(
                input,
                window,
                |this, _, event: &InputEvent, _, cx| {
                    if matches!(event, InputEvent::Change) {
                        this.reset_feedback(cx);
                    }
                },
            ));
        }
        self.subscriptions.push(cx.subscribe_in(
            &date,
            window,
            |this, _, _: &DatePickerEvent, _, cx| {
                this.reset_feedback(cx);
            },
        ));
        self.controls = Some(FormControls {
            date,
            amount,
            item,
            counterpart,
            comment,
            item_choices: Vec::new(),
            counterpart_choices: Vec::new(),
        });
    }

    /// 入力の変更前に表示していたエラーと追加成功メッセージを消す。
    fn reset_feedback(&mut self, cx: &mut Context<Self>) {
        self.error = None;
        self.has_added = false;
        cx.notify();
    }

    /// 品目管理での編集・削除を反映し、無効になった選択を解除する。
    fn refresh_choices(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(controls) = &mut self.controls else {
            return;
        };
        let ledger = &self.state.read(cx).ledger;
        let items = item_choices(ledger, None, false);
        let selected_item = controls.item.read(cx).selected_value().copied();
        let selected_tag = items
            .iter()
            .find(|item| Some(item.id) == selected_item)
            .map(|item| item.tag);
        let counterparts = item_choices(ledger, selected_tag, true);
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
        self.refresh_choices(window, cx);
        let Some(controls) = &self.controls else {
            return;
        };
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
            submit_transaction(&mut state.ledger, date, &amount, item, counterpart, comment)?;
            cx.notify();
            Ok(())
        });
        self.has_added = result.is_ok();
        self.error = result.err();
        if self.has_added {
            for input in [&controls.amount, &controls.comment] {
                input.update(cx, |state, cx| state.set_value("", window, cx));
            }
        }
        cx.notify();
    }
}

impl Render for TransactionForm {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.initialize_controls(window, cx);
        self.refresh_choices(window, cx);
        let controls = self.controls.as_ref().unwrap();
        let count = self.state.read(cx).ledger.transactions().iter().count();
        let amount_error = match &self.error {
            Some(FormError::Amount(message)) => Some(message.clone()),
            _ => None,
        };
        let form_error = match &self.error {
            Some(FormError::Other(message)) => Some(message.clone()),
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
                    .when(self.has_added, |this| {
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
    use super::{FormError, item_choices, submit_transaction};
    use chrono::NaiveDate;
    use codo_core::item::{ItemId, Tag};
    use codo_core::ledger::{Ledger, LedgerError};
    use codo_core::money::{Money, MoneyError};
    use codo_core::transaction::TransactionError;

    fn ledger() -> (Ledger, ItemId, ItemId) {
        let mut ledger = Ledger::new();
        let item = ledger.add_item("食費", Tag::Expense).unwrap();
        let counterpart = ledger.add_item("現金", Tag::Asset).unwrap();
        (ledger, item, counterpart)
    }

    #[test]
    fn submit_transaction_preserves_date_amount_items_and_comment() {
        let (mut ledger, item, counterpart) = ledger();
        let date = NaiveDate::from_ymd_opt(2024, 2, 29).unwrap();
        for (input, expected) in [
            ("1", 1),
            ("9", 9),
            ("10", 10),
            (" ¥1,234 ", 1234),
            ("2147483647", i32::MAX),
        ] {
            submit_transaction(
                &mut ledger,
                Some(date),
                input,
                Some(item),
                Some(counterpart),
                format!("コメント {input}"),
            )
            .unwrap();
            let transaction = ledger
                .transactions()
                .iter()
                .find(|transaction| transaction.comment() == format!("コメント {input}"))
                .unwrap();
            assert_eq!(transaction.date(), date);
            assert_eq!(transaction.amount(), Money(expected));
            assert_eq!(transaction.item(), item);
            assert_eq!(transaction.counterpart(), counterpart);
        }
        assert_eq!(ledger.transactions().iter().count(), 5);
    }

    #[test]
    fn submit_transaction_rejects_bad_amount_without_changing_ledger() {
        let (mut ledger, item, counterpart) = ledger();
        let date = NaiveDate::from_ymd_opt(2026, 10, 27).unwrap();
        submit_transaction(
            &mut ledger,
            Some(date),
            "100",
            Some(item),
            Some(counterpart),
            String::new(),
        )
        .unwrap();
        let before = ledger.clone();
        let overflow = MoneyError::Overflow.to_string();
        for (input, expected) in [
            ("", MoneyError::Parse(String::new()).to_string()),
            (" abc ", MoneyError::Parse(" abc ".into()).to_string()),
            ("1,23", MoneyError::Parse("1,23".into()).to_string()),
            ("1.5", MoneyError::Parse("1.5".into()).to_string()),
            ("2147483648", overflow.clone()),
            ("-2147483649", overflow.clone()),
            ("999999999999999999999999999", overflow),
            (
                "0",
                TransactionError::NonPositiveAmount(Money(0)).to_string(),
            ),
            (
                "-1",
                TransactionError::NonPositiveAmount(Money(-1)).to_string(),
            ),
            (
                "-2147483648",
                TransactionError::NonPositiveAmount(Money(i32::MIN)).to_string(),
            ),
        ] {
            assert_eq!(
                submit_transaction(
                    &mut ledger,
                    Some(date),
                    input,
                    Some(item),
                    Some(counterpart),
                    "残す".into()
                ),
                Err(FormError::Amount(expected)),
                "{input}"
            );
            assert_eq!(ledger, before);
        }
    }

    #[test]
    fn submit_transaction_reports_missing_fields_and_invalid_combinations() {
        let (mut ledger, item, counterpart) = ledger();
        let other_expense = ledger.add_item("娯楽", Tag::Expense).unwrap();
        let removed = ledger.add_item("削除する品目", Tag::Income).unwrap();
        ledger.remove_item(removed).unwrap();
        let date = NaiveDate::from_ymd_opt(2026, 10, 27).unwrap();
        let before = ledger.clone();
        for (date, item, counterpart, message) in [
            (
                None,
                Some(item),
                Some(counterpart),
                "日付を選択してください".to_string(),
            ),
            (
                Some(date),
                None,
                Some(counterpart),
                "品目を選択してください".to_string(),
            ),
            (
                Some(date),
                Some(item),
                None,
                "相手先を選択してください".to_string(),
            ),
            (
                Some(date),
                Some(item),
                Some(other_expense),
                LedgerError::SameTag(Tag::Expense).to_string(),
            ),
            (
                Some(date),
                Some(item),
                Some(item),
                TransactionError::SameItemAndCounterpart(item).to_string(),
            ),
            (
                Some(date),
                Some(counterpart),
                Some(item),
                LedgerError::InvalidItemTag(Tag::Asset).to_string(),
            ),
            (
                Some(date),
                Some(removed),
                Some(counterpart),
                LedgerError::TransactionItemNotFound(removed).to_string(),
            ),
            (
                Some(date),
                Some(item),
                Some(removed),
                LedgerError::TransactionItemNotFound(removed).to_string(),
            ),
        ] {
            assert_eq!(
                submit_transaction(&mut ledger, date, "100", item, counterpart, String::new()),
                Err(FormError::Other(message))
            );
            assert_eq!(ledger, before);
        }
    }

    #[test]
    fn item_choices_filter_tags_and_reflect_renames_and_deletions() {
        let mut ledger = Ledger::new();
        let ids: Vec<_> = Tag::ALL_TAGS
            .into_iter()
            .map(|tag| ledger.add_item(tag.label(), tag).unwrap())
            .collect();
        let choices = item_choices(&ledger, None, false);
        assert_eq!(
            choices.iter().map(|choice| choice.id).collect::<Vec<_>>(),
            vec![ids[1], ids[3], ids[4]]
        );
        assert!(item_choices(&ledger, None, true).is_empty());
        for tag in Tag::ALL_TAGS {
            let counterparts = item_choices(&ledger, Some(tag), true);
            let expected: Vec<_> = ledger
                .items()
                .iter()
                .filter(|item| item.tag() != tag)
                .map(|item| item.id())
                .collect();
            assert_eq!(
                counterparts
                    .iter()
                    .map(|choice| choice.id)
                    .collect::<Vec<_>>(),
                expected
            );
        }
        ledger
            .update_item(ids[4], "新しい支出", Tag::Asset)
            .unwrap();
        ledger.remove_item(ids[1]).unwrap();
        assert_eq!(
            item_choices(&ledger, None, false)
                .iter()
                .map(|choice| choice.id)
                .collect::<Vec<_>>(),
            vec![ids[3]]
        );
        let counterparts = item_choices(&ledger, Some(Tag::Income), true);
        assert_eq!(
            counterparts
                .iter()
                .find(|choice| choice.id == ids[4])
                .unwrap()
                .name
                .as_ref(),
            "新しい支出"
        );
        assert!(!counterparts.iter().any(|choice| choice.id == ids[1]));
    }
}

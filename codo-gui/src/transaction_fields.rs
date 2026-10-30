//! 取引の入力欄. 新規の追加と訂正の両方で使う.

use crate::app::AppState;
use chrono::{Local, NaiveDate};
use codo_core::ledger::LedgerError;
use codo_core::ledger::item::{Item, ItemId};
use codo_core::ledger::money::MoneyError;
use codo_core::ledger::transaction::{Transaction, TransactionDraft, TransactionError};
use gpui_kit::component::date_picker::{DatePicker, DatePickerEvent, DatePickerState};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::select::{Select, SelectEvent, SelectItem, SelectState};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

/// 品目と相手先の選択肢.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemChoice {
    pub id: ItemId,
    pub name: SharedString,
}

/// 取引の追加や訂正で起こるエラー.
#[derive(Debug)]
pub enum FormError {
    Amount(MoneyError),
    MissingDate,
    MissingItem,
    MissingCounterpart,
    Ledger(LedgerError),
}

impl From<MoneyError> for FormError {
    fn from(error: MoneyError) -> Self {
        Self::Amount(error)
    }
}

impl From<LedgerError> for FormError {
    fn from(error: LedgerError) -> Self {
        Self::Ledger(error)
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

/// 入力欄から読み取った値.
pub struct FieldValues {
    pub date: Option<NaiveDate>,
    pub amount: String,
    pub item: Option<ItemId>,
    pub counterpart: Option<ItemId>,
    pub comment: String,
}

/// 入力欄の値が変わったことを知らせるイベント.
pub struct FieldsChanged;

/// 取引の入力欄. 日付・金額・品目・相手先・コメントをまとめて扱う.
pub struct TransactionFields {
    state: Entity<AppState>,
    controls: FormControls,
    amount_error: Option<String>,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<FieldsChanged> for TransactionFields {}

impl TransactionFields {
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
                this.notify_changed(cx);
            },
        ));
        subscriptions.push(cx.subscribe_in(
            &counterpart,
            window,
            |this, _, _: &SelectEvent<Vec<ItemChoice>>, _, cx| {
                this.notify_changed(cx);
            },
        ));
        for input in [&amount, &comment] {
            subscriptions.push(cx.subscribe_in(
                input,
                window,
                |this, _, event: &InputEvent, _, cx| {
                    if matches!(event, InputEvent::Change) {
                        this.notify_changed(cx);
                    }
                },
            ));
        }
        subscriptions.push(cx.subscribe_in(
            &date,
            window,
            |this, _, _: &DatePickerEvent, _, cx| {
                this.notify_changed(cx);
            },
        ));
        let mut fields = Self {
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
            amount_error: None,
            _subscriptions: subscriptions,
        };
        fields.refresh_choices(window, cx);
        fields
    }

    /// 品目管理での編集・削除を反映し, 無効になった選択を解除する.
    fn refresh_choices(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let controls = &mut self.controls;
        let ledger = self.state.read(cx).ledger();
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

    /// 入力が変わったときに, 金額のエラーを消して画面に知らせる.
    fn notify_changed(&mut self, cx: &mut Context<Self>) {
        self.amount_error = None;
        cx.emit(FieldsChanged);
        cx.notify();
    }

    /// 金額の下に出すエラーを設定する.
    pub fn set_amount_error(&mut self, error: Option<String>, cx: &mut Context<Self>) {
        self.amount_error = error;
        cx.notify();
    }

    /// 入力欄の値を読み取る.
    pub fn values(&self, cx: &App) -> FieldValues {
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
        FieldValues {
            date,
            amount,
            item,
            counterpart,
            comment,
        }
    }

    /// 入力欄の値を検証し, 取引の下書きを作る.
    ///
    /// # Errors
    ///
    /// 金額が読めない場合や, 日付・品目・相手先が選ばれていない場合はエラー.
    pub fn draft(&self, cx: &App) -> Result<TransactionDraft, FormError> {
        let values = self.values(cx);
        let amount = values.amount.parse()?;
        Ok(TransactionDraft::new(
            values.date.ok_or(FormError::MissingDate)?,
            amount,
            values.item.ok_or(FormError::MissingItem)?,
            values.counterpart.ok_or(FormError::MissingCounterpart)?,
            values.comment,
        ))
    }

    /// 金額とコメントを空にする. 追加に成功したあとに使う.
    pub fn clear_amount_and_comment(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        for input in [&self.controls.amount, &self.controls.comment] {
            input.update(cx, |state, cx| state.set_value("", window, cx));
        }
    }

    /// 取引の値を入力欄に入れる. 訂正のときに使う.
    pub fn set_values(
        &mut self,
        transaction: &Transaction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.controls.date.update(cx, |state, cx| {
            state.set_date(transaction.date(), window, cx)
        });
        self.controls.amount.update(cx, |state, cx| {
            state.set_value(transaction.amount().to_string(), window, cx)
        });
        self.controls.comment.update(cx, |state, cx| {
            state.set_value(transaction.comment().to_string(), window, cx)
        });
        self.controls.item.update(cx, |state, cx| {
            state.set_selected_value(&transaction.item(), window, cx)
        });
        // 品目に合わせて相手先の候補を作ってから, 相手先を選ぶ
        self.refresh_choices(window, cx);
        self.controls.counterpart.update(cx, |state, cx| {
            state.set_selected_value(&transaction.counterpart(), window, cx)
        });
    }
}

#[cfg(test)]
impl TransactionFields {
    /// テスト用. 金額の下に出しているエラーを返す.
    pub fn amount_error(&self) -> Option<&str> {
        self.amount_error.as_deref()
    }

    /// テスト用. 品目・相手先・金額を, 利用者が選んだのと同じ状態にする.
    pub fn fill_for_test(
        &mut self,
        item: ItemId,
        counterpart: ItemId,
        amount: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.controls.item.update(cx, |select, cx| {
            select.set_selected_value(&item, window, cx)
        });
        // 品目に合わせて相手先の候補を作ってから, 相手先を選ぶ
        self.refresh_choices(window, cx);
        self.controls.counterpart.update(cx, |select, cx| {
            select.set_selected_value(&counterpart, window, cx)
        });
        self.controls.amount.update(cx, |input, cx| {
            input.set_value(amount.to_string(), window, cx)
        });
    }
}

impl Render for TransactionFields {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let controls = &self.controls;
        div()
            .flex()
            .flex_col()
            .gap_4()
            .child(div().child("日付").child(DatePicker::new(&controls.date)))
            .child(
                div()
                    .child("金額（円）")
                    .child(Input::new(&controls.amount))
                    .when_some(self.amount_error.clone(), |this, message| {
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
    }
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

/// コアのエラーを, 画面利用者向けのメッセージに変換する.
pub fn friendly_error(error: &FormError) -> String {
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
            LedgerError::Transaction(TransactionError::NotFound(_)) => {
                "訂正する取引が見つかりません。一覧を確認してください".into()
            }
            _ => "取引を保存できませんでした。品目と相手先を確認してください".into(),
        },
        FormError::Amount(error) => error.to_string(),
        FormError::MissingDate => "日付を選択してください".into(),
        FormError::MissingItem => "品目を選択してください".into(),
        FormError::MissingCounterpart => "相手先を選択してください".into(),
    }
}

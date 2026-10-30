//! 月別の取引一覧と削除操作。

use crate::app::AppState;
use crate::transaction_fields::{TransactionFields, friendly_error};
use chrono::Local;
use codo_core::period::YearMonth;
use codo_core::transaction::TransactionId;
use codo_core::transaction_input::{FormError, submit_transaction_update};
use gpui_kit::component::WindowExt;
use gpui_kit::component::button::Button;
use gpui_kit::component::table::{Table, TableBody, TableCell, TableHead, TableHeader, TableRow};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

/// 共有帳簿の取引を月ごとに表示する画面。
pub struct TransactionList {
    state: Entity<AppState>,
    month: YearMonth,
    error_message: Option<String>,
    editing: Option<(TransactionId, Entity<TransactionFields>)>,
    edit_error: Option<String>,
    scroll_handle: ScrollHandle,
    _subscription: Subscription,
}

impl TransactionList {
    pub fn new(state: Entity<AppState>, cx: &mut Context<Self>) -> Self {
        let subscription = cx.observe(&state, |_, _, cx| cx.notify());
        Self {
            state,
            month: YearMonth::from_date(Local::now().date_naive()),
            error_message: None,
            editing: None,
            edit_error: None,
            scroll_handle: ScrollHandle::new(),
            _subscription: subscription,
        }
    }

    fn show_month(&mut self, month: YearMonth, cx: &mut Context<Self>) {
        self.month = month;
        self.error_message = None;
        self.scroll_handle.set_offset(point(px(0.), px(0.)));
        cx.notify();
    }

    /// 削除の確認ダイアログを開き、確定後に取引を削除する。
    fn confirm_remove_transaction(
        &mut self,
        id: TransactionId,
        label: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let weak_list = cx.weak_entity();

        window.open_alert_dialog(cx, move |alert, _, _| {
            let weak_list = weak_list.clone();
            let label = label.clone();

            alert
                .confirm()
                .title("取引を削除しますか？")
                .description(format!(
                    "「{}」を削除します。この操作は取り消せません。",
                    label
                ))
                .ok_text("削除")
                .cancel_text("キャンセル")
                .on_ok(move |_, _, cx| {
                    weak_list
                        .update(cx, |list, cx| list.remove_transaction(id, cx))
                        .is_ok()
                })
        });
    }

    fn remove_transaction(&mut self, id: TransactionId, cx: &mut Context<Self>) {
        let result = self.state.update(cx, |state, cx| {
            state.ledger.remove_transaction(id)?;
            cx.notify();
            Ok::<_, codo_core::ledger::LedgerError>(())
        });
        self.error_message = result.err().map(|error| error.to_string());
        cx.notify();
    }

    /// 取引の訂正を始める. 入力欄に今の値を入れて, 訂正パネルを開く.
    fn start_edit(&mut self, id: TransactionId, window: &mut Window, cx: &mut Context<Self>) {
        let Some(transaction) = self.state.read(cx).ledger.transactions().get(id).cloned() else {
            return;
        };
        let state = self.state.clone();
        let fields = cx.new(|cx| TransactionFields::new(state, window, cx));
        fields.update(cx, |fields, cx| fields.set_values(&transaction, window, cx));
        self.editing = Some((id, fields));
        self.edit_error = None;
        self.error_message = None;
        cx.notify();
    }

    /// 訂正をやめて, 訂正パネルを閉じる.
    fn cancel_edit(&mut self, cx: &mut Context<Self>) {
        self.editing = None;
        self.edit_error = None;
        cx.notify();
    }

    /// 訂正を確定する. 成功したら訂正パネルを閉じる.
    fn confirm_edit(&mut self, cx: &mut Context<Self>) {
        let Some((id, fields)) = self.editing.clone() else {
            return;
        };
        let values = fields.read(cx).values(cx);
        let result: Result<(), FormError> = self.state.update(cx, |state, cx| {
            submit_transaction_update(
                &mut state.ledger,
                id,
                values.date,
                &values.amount,
                values.item,
                values.counterpart,
                values.comment,
            )?;
            cx.notify();
            Ok(())
        });
        match result {
            Ok(()) => {
                self.editing = None;
                self.edit_error = None;
            }
            Err(error @ FormError::Amount(_)) => {
                fields.update(cx, |fields, cx| {
                    fields.set_amount_error(Some(friendly_error(&error)), cx)
                });
                self.edit_error = None;
            }
            Err(error) => self.edit_error = Some(friendly_error(&error)),
        }
        cx.notify();
    }
}

impl Render for TransactionList {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let ledger = &self.state.read(cx).ledger;
        let rows = ledger
            .monthly_transactions(self.month)
            .map(|transaction| {
                let label = format!("{} {}", transaction.date(), transaction.amount());
                let id = transaction.id();
                let item_name = |id| {
                    ledger
                        .items()
                        .get_item(id)
                        .map(|item| item.name().to_string())
                        .unwrap_or_else(|| "削除された品目".to_string())
                };
                TableRow::new()
                    .child(TableCell::new().child(transaction.date().to_string()))
                    .child(TableCell::new().child(item_name(transaction.item())))
                    .child(TableCell::new().child(item_name(transaction.counterpart())))
                    .child(
                        TableCell::new()
                            .text_right()
                            .child(transaction.amount().to_string()),
                    )
                    .child(TableCell::new().child(transaction.comment().to_string()))
                    .child(
                        TableCell::new().child(
                            div()
                                .flex()
                                .gap_2()
                                .child(
                                    Button::new(format!("edit-transaction-{id:?}"))
                                        .label("編集")
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            this.start_edit(id, window, cx)
                                        })),
                                )
                                .child(
                                    Button::new(format!("remove-transaction-{id:?}"))
                                        .label("削除")
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            this.confirm_remove_transaction(
                                                id,
                                                label.clone(),
                                                window,
                                                cx,
                                            )
                                        })),
                                ),
                        ),
                    )
            })
            .collect::<Vec<_>>();
        let count = rows.len();

        let edit_panel = self.editing.as_ref().map(|(_, fields)| {
            div()
                .flex()
                .flex_col()
                .gap_4()
                .p_4()
                .border_1()
                .border_color(rgb(0xe0e0e0))
                .rounded_md()
                .max_w(px(560.))
                .child(div().text_lg().child("取引を訂正"))
                .child(fields.clone())
                .when_some(self.edit_error.clone(), |this, message| {
                    this.child(div().text_color(rgb(0xc62828)).child(message))
                })
                .child(
                    div()
                        .flex()
                        .gap_2()
                        .child(
                            Button::new("confirm-edit")
                                .label("確定")
                                .on_click(cx.listener(|this, _, _, cx| this.confirm_edit(cx))),
                        )
                        .child(
                            Button::new("cancel-edit")
                                .label("キャンセル")
                                .on_click(cx.listener(|this, _, _, cx| this.cancel_edit(cx))),
                        ),
                )
        });
        let table = Table::new()
            .min_w(px(720.))
            .child(
                TableHeader::new().child(
                    TableRow::new().children(
                        ["日付", "品目名", "相手先名", "金額", "コメント", "操作"]
                            .into_iter()
                            .map(|label| TableHead::new().child(label)),
                    ),
                ),
            )
            .child(TableBody::new().children(rows));
        div()
            .size_full()
            .flex()
            .flex_col()
            .gap_4()
            .p_6()
            .child(div().text_xl().child("取引一覧"))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_4()
                    .child(Button::new("previous-month").label("前月").on_click(
                        cx.listener(|this, _, _, cx| this.show_month(this.month.prev(), cx)),
                    ))
                    .child(format!("{}年{}月", self.month.year(), self.month.month()))
                    .child(Button::new("next-month").label("翌月").on_click(
                        cx.listener(|this, _, _, cx| this.show_month(this.month.next(), cx)),
                    ))
                    .child(format!("{count}件")),
            )
            .when_some(self.error_message.clone(), |this, message| {
                this.child(div().text_color(rgb(0xc62828)).child(message))
            })
            .children(edit_panel)
            .child(
                div()
                    .id("transaction-list-scroll")
                    .h((window.viewport_size().height - px(180.)).max(px(0.)))
                    .overflow_scroll()
                    .track_scroll(&self.scroll_handle)
                    .child(table)
                    .when(count == 0, |this| {
                        this.child(div().p_4().child("この月の取引はありません"))
                    }),
            )
    }
}

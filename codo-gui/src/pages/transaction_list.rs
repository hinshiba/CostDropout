//! 月別の取引一覧と, 取引の訂正・削除操作.

use crate::app::AppState;
use crate::transaction_fields::{FieldsChanged, FormError, TransactionFields, friendly_error};
use chrono::Local;
use codo_core::ledger::period::{Period, YearMonth};
use codo_core::ledger::transaction::TransactionId;
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
    // 訂正中の入力欄の変化を見張る. 訂正していないときは None
    _edit_subscription: Option<Subscription>,
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
            _edit_subscription: None,
        }
    }

    fn show_month(&mut self, month: YearMonth, cx: &mut Context<Self>) {
        self.month = month;
        self.error_message = None;
        self.scroll_handle.set_offset(point(px(0.), px(0.)));
        cx.notify();
    }

    /// 削除の確認ダイアログを開き, 確定後に取引を削除する.
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
            state.ledger_mut().remove_transaction(id)?;
            cx.notify();
            Ok::<_, codo_core::ledger::LedgerError>(())
        });
        self.error_message = result.err().map(|error| error.to_string());
        cx.notify();
    }

    /// 取引の訂正を始める. 入力欄に今の値を入れて, 訂正パネルを開く.
    fn start_edit(&mut self, id: TransactionId, window: &mut Window, cx: &mut Context<Self>) {
        let Some(transaction) = self.state.read(cx).ledger().transactions().get(id).cloned() else {
            return;
        };
        let state = self.state.clone();
        let fields = cx.new(|cx| TransactionFields::new(state, window, cx));
        fields.update(cx, |fields, cx| fields.set_values(&transaction, window, cx));
        // 入力が変わったら, 表示中のエラーを消す
        self._edit_subscription = Some(cx.subscribe(&fields, |this, _, _: &FieldsChanged, cx| {
            this.edit_error = None;
            cx.notify();
        }));
        self.editing = Some((id, fields));
        self.edit_error = None;
        self.error_message = None;
        cx.notify();
    }

    /// 訂正パネルを閉じ, 入力欄の見張りもやめる.
    fn close_edit(&mut self) {
        self.editing = None;
        self.edit_error = None;
        self._edit_subscription = None;
    }

    /// 訂正をやめて, 訂正パネルを閉じる.
    fn cancel_edit(&mut self, cx: &mut Context<Self>) {
        self.close_edit();
        cx.notify();
    }

    /// 訂正を確定する. 成功したら訂正パネルを閉じる.
    fn confirm_edit(&mut self, cx: &mut Context<Self>) {
        let Some((id, fields)) = self.editing.clone() else {
            return;
        };
        let draft = fields.read(cx).draft(cx);
        let result: Result<(), FormError> = self.state.update(cx, |state, cx| {
            state.ledger_mut().update_transaction(id, draft?)?;
            cx.notify();
            Ok(())
        });
        match result {
            Ok(()) => self.close_edit(),
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
        let ledger = self.state.read(cx).ledger();
        let rows = ledger
            .transactions()
            .iter_in(Period::month(self.month))
            .rev()
            .map(|transaction| {
                let label = format!("{} {}", transaction.date(), transaction.amount());
                let id = transaction.id();
                let item_name = |id| {
                    ledger
                        .items()
                        .get(id)
                        .map(|item| item.to_string())
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
                    .child(
                        TableCell::new()
                            .text_right()
                            .child(ledger.counterpart_amount(transaction).to_string()),
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
            .min_w(px(840.))
            .child(
                TableHeader::new().child(
                    TableRow::new().children(
                        [
                            "日付",
                            "品目名",
                            "相手先名",
                            "金額",
                            "相手先の増減",
                            "コメント",
                            "操作",
                        ]
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

#[cfg(test)]
mod tests {
    use super::TransactionList;
    use crate::app::AppState;
    use crate::transaction_fields::FieldsChanged;
    use chrono::Local;
    use codo_core::ledger::Ledger;
    use codo_core::ledger::item::{ItemId, Tag};
    use codo_core::ledger::money::Money;
    use codo_core::ledger::transaction::{TransactionDraft, TransactionId};
    use gpui_kit::test::TestWindowExt;
    use gpui_kit::{
        AnyWindowHandle, App, AppContext, Entity, TestAppContext, Window, WindowOptions,
    };

    /// テストで使う取引の情報.
    struct Fixture {
        list: Entity<TransactionList>,
        state: Entity<AppState>,
        id: TransactionId,
        food: ItemId,
        cash: ItemId,
        _directory: tempfile::TempDir,
    }

    /// 今月の取引を1件持つ取引一覧を開く.
    fn open_list(cx: &mut TestAppContext) -> (Fixture, AnyWindowHandle) {
        cx.update(gpui_kit::init);
        let mut ledger = Ledger::new();
        let food = ledger.add_item("食費", Tag::Expense).unwrap();
        let cash = ledger.add_item("現金", Tag::Asset).unwrap();
        let id = ledger
            .add_transaction(TransactionDraft::new(
                Local::now().date_naive(),
                "1,200".parse().unwrap(),
                food,
                cash,
                "昼食".into(),
            ))
            .unwrap();
        let (state, directory) = AppState::for_test(ledger);
        let state = cx.new(|_| state);
        let (handle, list) = cx.update(|cx| {
            gpui_kit::open_window(WindowOptions::default(), cx, |_, cx| {
                cx.new(|cx| TransactionList::new(state.clone(), cx))
            })
            .unwrap()
        });
        let fixture = Fixture {
            list,
            state,
            id,
            food,
            cash,
            _directory: directory,
        };
        (fixture, handle)
    }

    /// 今月の取引を1件持つ取引一覧を開き, その中で `f` を実行する.
    fn with_list(cx: &mut TestAppContext, f: impl FnOnce(&Fixture, &mut Window, &mut App)) {
        let (fixture, handle) = open_list(cx);
        cx.update_window(handle, |_, window, cx| f(&fixture, window, cx))
            .unwrap();
    }

    #[gpui_kit::test]
    fn changing_fields_clears_edit_error(cx: &mut TestAppContext) {
        let (fixture, handle) = open_list(cx);
        let id = fixture.id;
        cx.update_window(handle, |_, window, cx| {
            fixture
                .list
                .update(cx, |list, cx| list.start_edit(id, window, cx));
            // 取引が消されていて, 確定するとパネルにエラーが出る状態にする
            fixture.state.update(cx, |state, _| {
                state.ledger_mut().remove_transaction(id).unwrap()
            });
            window.render_frame(cx);
            window.click("confirm-edit", cx);
        })
        .unwrap();
        cx.run_until_parked();
        assert!(cx.read(|cx| fixture.list.read(cx).edit_error.is_some()));
        cx.update(|cx| {
            let (_, fields) = fixture.list.read(cx).editing.clone().unwrap();
            // 利用者が入力欄を変えたときに部品が出すイベントを送る.
            // (コードからの set_value では入力イベントが起きないため, 直接送る)
            fields.update(cx, |_, cx| cx.emit(FieldsChanged));
        });
        cx.run_until_parked();
        // 入力を変えたら, パネルのエラーは消える
        cx.read(|cx| {
            let list = fixture.list.read(cx);
            assert!(list.editing.is_some());
            assert_eq!(list.edit_error, None);
        });
    }

    #[gpui_kit::test]
    fn start_edit_fills_fields_with_transaction(cx: &mut TestAppContext) {
        with_list(cx, |fixture, window, cx| {
            let id = fixture.id;
            fixture
                .list
                .update(cx, |list, cx| list.start_edit(id, window, cx));
            let (editing_id, fields) = fixture.list.read(cx).editing.clone().unwrap();
            assert_eq!(editing_id, id);
            let values = fields.read(cx).values(cx);
            assert_eq!(values.date, Some(Local::now().date_naive()));
            assert_eq!(values.amount, "¥1,200");
            assert_eq!(values.item, Some(fixture.food));
            assert_eq!(values.counterpart, Some(fixture.cash));
            assert_eq!(values.comment, "昼食");
        });
    }

    #[gpui_kit::test]
    fn confirm_edit_updates_transaction_and_closes_panel(cx: &mut TestAppContext) {
        with_list(cx, |fixture, window, cx| {
            let id = fixture.id;
            fixture
                .list
                .update(cx, |list, cx| list.start_edit(id, window, cx));
            let (_, fields) = fixture.list.read(cx).editing.clone().unwrap();
            fields.update(cx, |fields, cx| {
                fields.fill_for_test(fixture.food, fixture.cash, "3,000", window, cx)
            });
            window.render_frame(cx);
            window.click("confirm-edit", cx);
            let list = fixture.list.read(cx);
            assert!(list.editing.is_none());
            assert_eq!(list.edit_error, None);
            let ledger = fixture.state.read(cx).ledger();
            // 追加ではなく訂正なので, 件数は増えない
            assert_eq!(ledger.transactions().iter().count(), 1);
            let transaction = ledger.transactions().get(id).unwrap();
            assert_eq!(transaction.amount(), Money::from(3000));
            assert_eq!(transaction.comment(), "昼食");
        });
    }

    #[gpui_kit::test]
    fn confirm_edit_keeps_panel_and_ledger_on_amount_error(cx: &mut TestAppContext) {
        with_list(cx, |fixture, window, cx| {
            let id = fixture.id;
            let before = fixture.state.read(cx).ledger().clone();
            fixture
                .list
                .update(cx, |list, cx| list.start_edit(id, window, cx));
            let (_, fields) = fixture.list.read(cx).editing.clone().unwrap();
            fields.update(cx, |fields, cx| {
                fields.fill_for_test(fixture.food, fixture.cash, "abc", window, cx)
            });
            window.render_frame(cx);
            window.click("confirm-edit", cx);
            let list = fixture.list.read(cx);
            // 金額のエラーは入力欄の金額の下に出すので, パネルのエラーは空のまま
            assert!(list.editing.is_some());
            assert_eq!(list.edit_error, None);
            assert_eq!(
                fields.read(cx).amount_error(),
                Some("金額として解釈できない文字列です: abc")
            );
            assert_eq!(fixture.state.read(cx).ledger(), &before);
        });
    }

    #[gpui_kit::test]
    fn confirm_edit_shows_error_when_transaction_was_removed(cx: &mut TestAppContext) {
        with_list(cx, |fixture, window, cx| {
            let id = fixture.id;
            fixture
                .list
                .update(cx, |list, cx| list.start_edit(id, window, cx));
            // 訂正中に, 別の操作で取引が消された場合
            fixture.state.update(cx, |state, _| {
                state.ledger_mut().remove_transaction(id).unwrap()
            });
            window.render_frame(cx);
            window.click("confirm-edit", cx);
            let list = fixture.list.read(cx);
            assert!(list.editing.is_some());
            assert_eq!(
                list.edit_error.as_deref(),
                Some("訂正する取引が見つかりません。一覧を確認してください")
            );
        });
    }

    #[gpui_kit::test]
    fn cancel_edit_closes_panel_without_changes(cx: &mut TestAppContext) {
        with_list(cx, |fixture, window, cx| {
            let id = fixture.id;
            let before = fixture.state.read(cx).ledger().clone();
            fixture
                .list
                .update(cx, |list, cx| list.start_edit(id, window, cx));
            let (_, fields) = fixture.list.read(cx).editing.clone().unwrap();
            fields.update(cx, |fields, cx| {
                fields.fill_for_test(fixture.food, fixture.cash, "9,999", window, cx)
            });
            window.render_frame(cx);
            window.click("cancel-edit", cx);
            assert!(fixture.list.read(cx).editing.is_none());
            assert_eq!(fixture.state.read(cx).ledger(), &before);
        });
    }
}

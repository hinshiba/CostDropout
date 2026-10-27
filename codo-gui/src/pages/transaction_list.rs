//! 月別の取引一覧と削除操作。

use crate::app::AppState;
use chrono::Local;
use codo_core::period::YearMonth;
use codo_core::transaction::TransactionId;
use gpui_kit::component::button::Button;
use gpui_kit::component::table::{Table, TableBody, TableCell, TableHead, TableHeader, TableRow};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

/// 共有帳簿の取引を月ごとに表示する画面。
pub struct TransactionList {
    state: Entity<AppState>,
    month: YearMonth,
    error_message: Option<String>,
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

    fn remove_transaction(&mut self, id: TransactionId, cx: &mut Context<Self>) {
        let result = self.state.update(cx, |state, cx| {
            state.ledger.remove_transaction(id)?;
            cx.notify();
            Ok::<_, codo_core::ledger::LedgerError>(())
        });
        self.error_message = result.err().map(|error| error.to_string());
        cx.notify();
    }
}

impl Render for TransactionList {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let ledger = &self.state.read(cx).ledger;
        let rows = ledger
            .monthly_transactions(self.month)
            .map(|transaction| {
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
                            Button::new(format!("remove-transaction-{id:?}"))
                                .label("削除")
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.remove_transaction(id, cx)
                                })),
                        ),
                    )
            })
            .collect::<Vec<_>>();
        let count = rows.len();
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

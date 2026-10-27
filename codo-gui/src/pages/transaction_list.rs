//! 月別の取引一覧と削除操作。

use crate::app::AppState;
use chrono::Local;
use codo_core::ledger::Ledger;
use codo_core::period::YearMonth;
use codo_core::transaction::{Transaction, TransactionId};
use gpui_kit::component::button::Button;
use gpui_kit::component::table::{Table, TableBody, TableCell, TableHead, TableHeader, TableRow};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

/// 指定月の取引を日付の降順、同日ならIDの降順で返す。
fn monthly_transactions(ledger: &Ledger, month: YearMonth) -> Vec<&Transaction> {
    let mut transactions: Vec<_> = ledger
        .transactions()
        .iter()
        .filter(|transaction| month.contains(transaction.date()))
        .collect();
    transactions
        .sort_by_key(|transaction| std::cmp::Reverse((transaction.date(), transaction.id())));
    transactions
}

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

    fn change_month(&mut self, is_next: bool, cx: &mut Context<Self>) {
        self.month = if is_next {
            self.month.next()
        } else {
            self.month.prev()
        };
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
        let transactions = monthly_transactions(ledger, self.month);
        let count = transactions.len();
        let rows = transactions
            .into_iter()
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
                    .child(
                        Button::new("previous-month")
                            .label("前月")
                            .on_click(cx.listener(|this, _, _, cx| this.change_month(false, cx))),
                    )
                    .child(format!("{}年{}月", self.month.year(), self.month.month()))
                    .child(
                        Button::new("next-month")
                            .label("翌月")
                            .on_click(cx.listener(|this, _, _, cx| this.change_month(true, cx))),
                    )
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

#[cfg(test)]
mod tests {
    use super::monthly_transactions;
    use chrono::NaiveDate;
    use codo_core::item::Tag;
    use codo_core::ledger::Ledger;
    use codo_core::money::Money;
    use codo_core::period::YearMonth;
    use codo_core::transaction::TransactionDraft;

    #[test]
    fn monthly_transactions_filter_boundaries_sort_and_reflect_removal() {
        let mut ledger = Ledger::new();
        let item = ledger.add_item("食費", Tag::Expense).unwrap();
        let counterpart = ledger.add_item("現金", Tag::Asset).unwrap();
        let dates = [
            (2026, 12, 31),
            (2027, 1, 1),
            (2026, 11, 30),
            (2026, 12, 1),
            (2026, 12, 15),
            (2026, 12, 15),
        ];
        let mut ids = Vec::new();
        for (year, month, day) in dates {
            ids.push(
                ledger
                    .add_transaction(TransactionDraft::new(
                        NaiveDate::from_ymd_opt(year, month, day).unwrap(),
                        Money(1234),
                        item,
                        counterpart,
                        String::new(),
                    ))
                    .unwrap(),
            );
        }
        let month = YearMonth::new(2026, 12).unwrap();
        let mut same_day = [ids[4], ids[5]];
        same_day.sort_by(|left, right| right.cmp(left));
        assert_eq!(
            monthly_transactions(&ledger, month)
                .iter()
                .map(|transaction| transaction.id())
                .collect::<Vec<_>>(),
            vec![ids[0], same_day[0], same_day[1], ids[3]]
        );
        assert_eq!(
            monthly_transactions(&ledger, month.prev())
                .iter()
                .map(|transaction| transaction.id())
                .collect::<Vec<_>>(),
            vec![ids[2]]
        );
        assert_eq!(
            monthly_transactions(&ledger, month.next())
                .iter()
                .map(|transaction| transaction.id())
                .collect::<Vec<_>>(),
            vec![ids[1]]
        );
        ledger.remove_transaction(ids[0]).unwrap();
        assert_eq!(
            monthly_transactions(&ledger, month)
                .iter()
                .map(|transaction| transaction.id())
                .collect::<Vec<_>>(),
            vec![same_day[0], same_day[1], ids[3]]
        );
        assert!(monthly_transactions(&ledger, month.next().next()).is_empty());
    }

    #[test]
    fn monthly_transactions_include_leap_day_only_in_february() {
        let mut ledger = Ledger::new();
        let item = ledger.add_item("給与", Tag::Income).unwrap();
        let counterpart = ledger.add_item("銀行", Tag::Asset).unwrap();
        for (month, day) in [(2, 1), (2, 29), (3, 1)] {
            ledger
                .add_transaction(TransactionDraft::new(
                    NaiveDate::from_ymd_opt(2024, month, day).unwrap(),
                    Money(1),
                    item,
                    counterpart,
                    String::new(),
                ))
                .unwrap();
        }
        let transactions = monthly_transactions(&ledger, YearMonth::new(2024, 2).unwrap());
        assert_eq!(
            transactions
                .iter()
                .map(|transaction| transaction.date())
                .collect::<Vec<_>>(),
            vec![
                NaiveDate::from_ymd_opt(2024, 2, 29).unwrap(),
                NaiveDate::from_ymd_opt(2024, 2, 1).unwrap()
            ]
        );
    }
}

//! 取引入力の検証と、品目・相手先の候補取得。

use chrono::NaiveDate;
use thiserror::Error;

use crate::item::{Item, ItemId};
use crate::ledger::{Ledger, LedgerError};
use crate::transaction::{Amount, AmountError, TransactionDraft, check_item_tag, check_tags};

/// 取引の品目として選べる収入・支出・負債の品目を返す。
pub fn item_choices(ledger: &Ledger) -> impl Iterator<Item = &Item> {
    ledger
        .items()
        .iter()
        .filter(|item| check_item_tag(item.tag()).is_ok())
}

/// 品目 `item` と異なるタグの相手先候補を返す。
pub fn counterpart_choices(ledger: &Ledger, item: ItemId) -> impl Iterator<Item = &Item> {
    let tag = ledger.items().get(item).map(Item::tag);
    ledger
        .items()
        .iter()
        .filter(move |candidate| tag.is_some_and(|tag| check_tags(tag, candidate.tag()).is_ok()))
}

/// 取引入力の検証・登録エラー。
#[derive(Debug, Error, PartialEq, Eq)]
pub enum FormError {
    #[error(transparent)]
    Amount(#[from] AmountError),
    #[error("日付を選択してください")]
    MissingDate,
    #[error("品目を選択してください")]
    MissingItem,
    #[error("相手先を選択してください")]
    MissingCounterpart,
    #[error(transparent)]
    Ledger(#[from] LedgerError),
}

/// 入力を検証し、成功した場合だけ帳簿に取引を追加する。
///
/// 金額は0円を含まない正の値のみ受け付ける。
pub fn submit_transaction(
    ledger: &mut Ledger,
    date: Option<NaiveDate>,
    amount: &str,
    item: Option<ItemId>,
    counterpart: Option<ItemId>,
    comment: String,
) -> Result<(), FormError> {
    let amount = amount.parse::<Amount>()?;
    let date = date.ok_or(FormError::MissingDate)?;
    let item = item.ok_or(FormError::MissingItem)?;
    let counterpart = counterpart.ok_or(FormError::MissingCounterpart)?;
    ledger
        .add_transaction(TransactionDraft::new(
            date,
            amount,
            item,
            counterpart,
            comment,
        ))
        .map(|_| ())
        .map_err(FormError::Ledger)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item::Tag;
    use crate::money::{Money, MoneyError};
    use crate::transaction::TransactionError;

    fn ledger() -> (Ledger, ItemId, ItemId) {
        let mut ledger = Ledger::new();
        let item = ledger.add_item("食費", Tag::Expense).unwrap();
        let counterpart = ledger.add_item("現金", Tag::Asset).unwrap();
        (ledger, item, counterpart)
    }

    #[test]
    fn submit_transaction_preserves_fields_and_accepts_optional_comment() {
        let (mut ledger, item, counterpart) = ledger();
        let date = NaiveDate::from_ymd_opt(2026, 10, 27).unwrap();
        let existing_id = ledger
            .add_transaction(TransactionDraft::new(
                date,
                Amount::of(200),
                item,
                counterpart,
                "既存".into(),
            ))
            .unwrap();
        let existing = ledger.transactions().get(existing_id).unwrap().clone();
        for comment in ["食事", ""] {
            let previous_count = ledger.transactions().iter().count();
            submit_transaction(
                &mut ledger,
                Some(date),
                "100",
                Some(item),
                Some(counterpart),
                comment.into(),
            )
            .unwrap();
            let transaction = ledger
                .transactions()
                .iter()
                .find(|transaction| transaction.comment() == comment)
                .unwrap();
            assert_eq!(transaction.date(), date);
            assert_eq!(transaction.amount(), Amount::of(100));
            assert_eq!(transaction.item(), item);
            assert_eq!(transaction.counterpart(), counterpart);
            assert_eq!(ledger.transactions().iter().count(), previous_count + 1);
            assert_eq!(ledger.transactions().get(existing_id), Some(&existing));
        }
    }

    #[test]
    fn submit_transaction_routes_amount_errors_and_keeps_ledger() {
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
        for (input, expected) in [
            (
                " abc ",
                AmountError::Parse(MoneyError::Parse(" abc ".into())),
            ),
            ("0", AmountError::NonPositive(Money::from(0))),
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
                Err(FormError::Amount(expected))
            );
            assert_eq!(ledger, before);
        }
    }

    #[test]
    fn submit_transaction_reports_missing_fields_and_keeps_ledger() {
        let (mut ledger, item, counterpart) = ledger();
        let date = NaiveDate::from_ymd_opt(2026, 10, 27).unwrap();
        let before = ledger.clone();
        for (date, item, counterpart, expected) in [
            (None, Some(item), Some(counterpart), FormError::MissingDate),
            (Some(date), None, Some(counterpart), FormError::MissingItem),
            (Some(date), Some(item), None, FormError::MissingCounterpart),
        ] {
            assert_eq!(
                submit_transaction(&mut ledger, date, "100", item, counterpart, String::new()),
                Err(expected)
            );
            assert_eq!(ledger, before);
        }
    }

    #[test]
    fn submit_transaction_preserves_ledger_errors_and_state() {
        let (mut ledger, item, counterpart) = ledger();
        let same_tag = ledger.add_item("娯楽", Tag::Expense).unwrap();
        let removed = ledger.add_item("削除する品目", Tag::Income).unwrap();
        ledger.remove_item(removed).unwrap();
        let date = NaiveDate::from_ymd_opt(2026, 10, 27).unwrap();
        let before = ledger.clone();
        for (item, counterpart, expected) in [
            (
                item,
                same_tag,
                LedgerError::Transaction(TransactionError::SameTag(Tag::Expense)),
            ),
            (
                item,
                item,
                LedgerError::Transaction(TransactionError::SameTag(Tag::Expense)),
            ),
            (
                counterpart,
                item,
                LedgerError::Transaction(TransactionError::InvalidItemTag(Tag::Asset)),
            ),
            (
                removed,
                counterpart,
                LedgerError::TransactionItemNotFound(removed),
            ),
            (item, removed, LedgerError::TransactionItemNotFound(removed)),
        ] {
            assert_eq!(
                submit_transaction(
                    &mut ledger,
                    Some(date),
                    "100",
                    Some(item),
                    Some(counterpart),
                    String::new()
                ),
                Err(FormError::Ledger(expected))
            );
            assert_eq!(ledger, before);
        }
    }

    fn all_tags() -> (Ledger, [ItemId; 5]) {
        let mut ledger = Ledger::new();
        let ids = Tag::ALL.map(|tag| ledger.add_item(&tag.to_string(), tag).unwrap());
        (ledger, ids)
    }

    #[test]
    fn item_choices_include_only_selectable_tags() {
        let (ledger, ids) = all_tags();
        assert_eq!(
            item_choices(&ledger).map(Item::id).collect::<Vec<_>>(),
            vec![ids[1], ids[3], ids[4]]
        );
    }

    #[test]
    fn counterpart_choices_exclude_selected_item_tag() {
        let (mut ledger, ids) = all_tags();
        let removed = ledger.add_item("削除する品目", Tag::Expense).unwrap();
        ledger.remove_item(removed).unwrap();
        for (item, expected) in [
            (ids[0], vec![]),
            (ids[1], vec![ids[0], ids[2], ids[3], ids[4]]),
            (ids[2], vec![]),
            (ids[3], vec![ids[0], ids[1], ids[2], ids[4]]),
            (ids[4], vec![ids[0], ids[1], ids[2], ids[3]]),
            (removed, vec![]),
        ] {
            assert_eq!(
                counterpart_choices(&ledger, item)
                    .map(Item::id)
                    .collect::<Vec<_>>(),
                expected,
                "{item:?}"
            );
        }
    }

    #[test]
    fn choices_reflect_item_changes() {
        let (mut ledger, ids) = all_tags();
        ledger.update_item(ids[4], "変更後", Tag::Asset).unwrap();
        ledger.remove_item(ids[1]).unwrap();
        assert_eq!(
            item_choices(&ledger).map(Item::id).collect::<Vec<_>>(),
            vec![ids[3]]
        );
        let counterparts = counterpart_choices(&ledger, ids[3]).collect::<Vec<_>>();
        assert_eq!(
            counterparts
                .iter()
                .map(|item| item.id())
                .collect::<Vec<_>>(),
            vec![ids[0], ids[2], ids[4]]
        );
        assert_eq!(counterparts[2].to_string(), "変更後");
    }
}

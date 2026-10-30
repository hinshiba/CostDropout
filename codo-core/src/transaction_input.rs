//! 取引入力の検証と、品目・相手先の候補取得。

use chrono::NaiveDate;
use thiserror::Error;

use crate::item::{Item, ItemId, Tag};
use crate::ledger::{Ledger, LedgerError};
use crate::money::{Money, MoneyError};
use crate::transaction::{TransactionDraft, TransactionError};

/// 取引の品目として選べる収入・支出・負債の品目を返す。
pub fn item_choices(ledger: &Ledger) -> impl Iterator<Item = &Item> {
    ledger
        .items()
        .iter()
        .filter(|item| item.tag().is_transaction_selectable())
}

/// 指定したタグと異なるタグの相手先候補を返す。
pub fn counterpart_choices(ledger: &Ledger, tag: Tag) -> impl Iterator<Item = &Item> {
    ledger.items().iter().filter(move |item| item.tag() != tag)
}

/// 金額入力の変換・検証エラー。
#[derive(Debug, Error, PartialEq, Eq)]
pub enum AmountError {
    #[error(transparent)]
    Parse(#[from] MoneyError),
    #[error("金額は0円より大きい値を入力してください: {0}")]
    NonPositive(Money),
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
    let amount = amount.parse::<Money>().map_err(AmountError::Parse)?;
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
        .map_err(|error| match error {
            LedgerError::Transaction(TransactionError::NonPositiveAmount(amount)) => {
                FormError::Amount(AmountError::NonPositive(amount))
            }
            error => FormError::Ledger(error),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

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
                Money::from(200),
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
            assert_eq!(transaction.amount(), Money::from(100));
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
            (item, same_tag, LedgerError::SameTag(Tag::Expense)),
            (
                item,
                item,
                LedgerError::Transaction(TransactionError::SameItemAndCounterpart(item)),
            ),
            (counterpart, item, LedgerError::InvalidItemTag(Tag::Asset)),
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
        let ids = Tag::ALL_TAGS.map(|tag| ledger.add_item(tag.label(), tag).unwrap());
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
    fn counterpart_choices_exclude_each_selected_tag() {
        let (ledger, ids) = all_tags();
        for (tag, expected) in [
            (Tag::Asset, vec![ids[1], ids[2], ids[3], ids[4]]),
            (Tag::Liability, vec![ids[0], ids[2], ids[3], ids[4]]),
            (Tag::OpeningBalance, vec![ids[0], ids[1], ids[3], ids[4]]),
            (Tag::Income, vec![ids[0], ids[1], ids[2], ids[4]]),
            (Tag::Expense, vec![ids[0], ids[1], ids[2], ids[3]]),
        ] {
            assert_eq!(
                counterpart_choices(&ledger, tag)
                    .map(Item::id)
                    .collect::<Vec<_>>(),
                expected
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
        let counterparts = counterpart_choices(&ledger, Tag::Income).collect::<Vec<_>>();
        assert_eq!(
            counterparts
                .iter()
                .map(|item| item.id())
                .collect::<Vec<_>>(),
            vec![ids[0], ids[2], ids[4]]
        );
        assert_eq!(counterparts[2].name(), "変更後");
    }
}

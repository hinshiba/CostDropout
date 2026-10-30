//! 取引のモデルと取引一覧

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::item::{ItemId, Tag};

mod amount;
mod list;

pub use amount::{Amount, AmountError};
pub use list::TransactionList;

/// 取引を識別する ID.
///
/// 別の月ファイルへ同時に追加されても衝突しないよう uuid を使う.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TransactionId(Uuid);

/// 帳簿に登録した取引.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Transaction {
    id: TransactionId,
    #[serde(flatten)]
    draft: TransactionDraft,
}

impl Transaction {
    pub fn id(&self) -> TransactionId {
        self.id
    }

    pub fn date(&self) -> NaiveDate {
        self.draft.date
    }

    pub fn amount(&self) -> Amount {
        self.draft.amount
    }

    /// 収入, 支出, 負債のいずれかの品目を返す.
    pub fn item(&self) -> ItemId {
        self.draft.item
    }

    /// 相手先の品目を返す.
    pub fn counterpart(&self) -> ItemId {
        self.draft.counterpart
    }

    pub fn comment(&self) -> &str {
        &self.draft.comment
    }

    /// `TransactionDraft::check_tags` と同じ.
    pub(crate) fn check_tags<E: From<TransactionError>>(
        &self,
        tag_of: impl Fn(ItemId) -> Result<Tag, E>,
    ) -> Result<(), E> {
        self.draft.check_tags(tag_of)
    }

    /// 品目または相手先として `item` を使うかを判定する.
    pub(crate) fn uses(&self, item: ItemId) -> bool {
        self.draft.item == item || self.draft.counterpart == item
    }
}

/// ID のない取引の内容.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransactionDraft {
    #[serde(with = "crate::serialization::toml_date")]
    date: NaiveDate,
    amount: Amount,
    item: ItemId,
    counterpart: ItemId,
    comment: String,
}

impl TransactionDraft {
    pub fn new(
        date: NaiveDate,
        amount: Amount,
        item: ItemId,
        counterpart: ItemId,
        comment: String,
    ) -> Self {
        Self {
            date,
            amount,
            item,
            counterpart,
            comment,
        }
    }

    /// 品目と相手先のタグの組み合わせを検査する.
    ///
    /// `tag_of` で `ItemId` から `Tag` を解決する.
    ///
    /// # Errors
    ///
    /// `tag_of` のエラー.
    /// `codo_core::transaction::check_tags` のエラー.
    pub(crate) fn check_tags<E: From<TransactionError>>(
        &self,
        tag_of: impl Fn(ItemId) -> Result<Tag, E>,
    ) -> Result<(), E> {
        // 内部の関数に移譲する
        Ok(check_tags(tag_of(self.item)?, tag_of(self.counterpart)?)?)
    }
}

/// 取引の品目として選べるタグかを検査する.
///
/// 収入, 支出, 負債だけが選べる.
///
/// # Errors
///
/// 選べない場合は `TransactionError::InvalidItemTag` エラー.
pub(crate) fn check_item_tag(tag: Tag) -> Result<(), TransactionError> {
    if !matches!(tag, Tag::Income | Tag::Expense | Tag::Liability) {
        return Err(TransactionError::InvalidItemTag(tag));
    }
    Ok(())
}

/// 取引の品目と相手先のタグの組み合わせを検査する.
///
/// # Errors
///
/// 品目のタグが取引の品目として選べない場合は `TransactionError::InvalidItemTag` エラー.
/// 相手先のタグが品目のタグと同じ場合は `TransactionError::SameTag` エラー.
pub(crate) fn check_tags(item: Tag, counterpart: Tag) -> Result<(), TransactionError> {
    check_item_tag(item)?;
    if item == counterpart {
        return Err(TransactionError::SameTag(item));
    }
    Ok(())
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum TransactionError {
    /// 品目のタグが取引の品目として選べない場合.
    #[error("取引の品目のタグ {0} が収入・支出・負債ではありません")]
    InvalidItemTag(Tag),
    /// 相手先のタグが品目のタグと同じ場合.
    #[error("相手先のタグ {0} が品目のタグと同じです")]
    SameTag(Tag),
    /// 取引が存在しない場合.
    #[error("取引 {0:?} が見つかりません")]
    NotFound(TransactionId),
    /// 取引 ID が他の取引と同じ場合.
    #[error("取引 {0:?} が重複しています")]
    DuplicateId(TransactionId),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_tags_test() {
        for (item, counterpart) in [
            (Tag::Expense, Tag::Asset),
            (Tag::Expense, Tag::Liability),
            (Tag::Income, Tag::Asset),
            (Tag::Liability, Tag::Asset),
            (Tag::Liability, Tag::OpeningBalance),
        ] {
            assert_eq!(
                check_tags(item, counterpart),
                Ok(()),
                "{item:?} {counterpart:?}"
            );
        }
    }

    #[test]
    fn check_tags_error_test() {
        for (item, counterpart, expected) in [
            (
                Tag::Asset,
                Tag::Income,
                TransactionError::InvalidItemTag(Tag::Asset),
            ),
            (
                Tag::OpeningBalance,
                Tag::Asset,
                TransactionError::InvalidItemTag(Tag::OpeningBalance),
            ),
            // 品目として選べないことを先に判定する
            (
                Tag::Asset,
                Tag::Asset,
                TransactionError::InvalidItemTag(Tag::Asset),
            ),
            (
                Tag::Income,
                Tag::Income,
                TransactionError::SameTag(Tag::Income),
            ),
            (
                Tag::Expense,
                Tag::Expense,
                TransactionError::SameTag(Tag::Expense),
            ),
            (
                Tag::Liability,
                Tag::Liability,
                TransactionError::SameTag(Tag::Liability),
            ),
        ] {
            assert_eq!(
                check_tags(item, counterpart),
                Err(expected),
                "{item:?} {counterpart:?}"
            );
        }
    }
}

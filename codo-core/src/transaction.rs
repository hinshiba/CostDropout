//! 取引

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::item::{ItemId, Tag};

mod amount;
mod list;

pub use amount::{Amount, AmountError};
pub use list::TransactionList;

/// 取引 ID
///
/// 別の月ファイルへ同時に追加されても衝突しないよう uuid を使う
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TransactionId(Uuid);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Transaction {
    id: TransactionId,
    #[serde(flatten)]
    draft: TransactionDraft,
}

///取引の各値の読み込み
impl Transaction {
    ///取引のID
    pub fn id(&self) -> TransactionId {
        self.id
    }

    ///取引の日付
    pub fn date(&self) -> NaiveDate {
        self.draft.date
    }

    ///取引の金額
    pub fn amount(&self) -> Amount {
        self.draft.amount
    }

    ///取引の品目
    pub fn item(&self) -> ItemId {
        self.draft.item
    }

    ///取引の相手先
    pub fn counterpart(&self) -> ItemId {
        self.draft.counterpart
    }

    ///取引のコメント
    pub fn comment(&self) -> &str {
        &self.draft.comment
    }

    /// 品目と相手先のタグの組み合わせを検査する.
    ///
    /// `tag_of` で品目のタグを引く.
    ///
    /// # Errors
    ///
    /// `tag_of` のエラーと, `check_tags` のエラー.
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

///idなしの取引
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransactionDraft {
    #[serde(with = "crate::serialization::toml_date")]
    date: NaiveDate,
    amount: Amount,
    /// 収入/支出/負債の品目
    item: ItemId,
    /// 相手先の品目
    counterpart: ItemId,
    comment: String,
}

impl TransactionDraft {
    ///取引の下書きの作成
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

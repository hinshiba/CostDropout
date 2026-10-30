//! 取引のモデルと取引一覧

use chrono::NaiveDate;
use thiserror::Error;
use uuid::Uuid;

use crate::ledger::item::{ItemId, Tag};
use crate::ledger::money::Money;

mod list;

pub use list::TransactionList;

/// 取引を識別する ID.
///
/// 別の月ファイルへ同時に追加されても衝突しないよう uuid を使う.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TransactionId(pub(crate) Uuid);

/// 帳簿に登録した取引.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transaction {
    id: TransactionId,
    draft: TransactionDraft,
}

impl Transaction {
    pub(crate) fn new(id: TransactionId, draft: TransactionDraft) -> Self {
        Self { id, draft }
    }

    pub fn id(&self) -> TransactionId {
        self.id
    }

    pub fn date(&self) -> NaiveDate {
        self.draft.date
    }

    pub fn amount(&self) -> Money {
        self.draft.amount
    }

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

    /// `TransactionDraft::counterpart_amount` と同じ.
    pub(crate) fn counterpart_amount<E: From<TransactionError>>(
        &self,
        tag_of: impl Fn(ItemId) -> Result<Tag, E>,
    ) -> Result<Money, E> {
        self.draft.counterpart_amount(tag_of)
    }

    /// 品目または相手先として `item` を使うかを判定する.
    pub(crate) fn uses(&self, item: ItemId) -> bool {
        self.draft.item == item || self.draft.counterpart == item
    }
}

/// ID のない取引の内容.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransactionDraft {
    date: NaiveDate,
    amount: Money,
    item: ItemId,
    counterpart: ItemId,
    comment: String,
}

impl TransactionDraft {
    pub fn new(
        date: NaiveDate,
        amount: Money,
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

    /// 品目と相手先の組み合わせを検査し, 相手先の増減を求める.
    ///
    /// `tag_of` で `ItemId` から `Tag` を解決する.
    ///
    /// # Errors
    ///
    /// `tag_of` のエラー.
    /// 品目と相手先が同じ場合は `TransactionError::SameItem` エラー.
    /// `Direction::of` と `Direction::apply` のエラー.
    pub(crate) fn counterpart_amount<E: From<TransactionError>>(
        &self,
        tag_of: impl Fn(ItemId) -> Result<Tag, E>,
    ) -> Result<Money, E> {
        let (item, counterpart) = (tag_of(self.item)?, tag_of(self.counterpart)?);
        if self.item == self.counterpart {
            return Err(TransactionError::SameItem(self.item).into());
        }
        Ok(Direction::of(item, counterpart)?.apply(self.amount)?)
    }
}

/// 品目の増減に対する相手先の増減の向き.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Direction {
    /// 品目と同じ符号で増減する.
    Same,
    /// 品目と異なる符号で増減する.
    Opposite,
}

impl Direction {
    /// 品目と相手先のタグの組から向きを決める.
    ///
    /// # Errors
    ///
    /// 取引にできない組の場合は `TransactionError::InvalidPair` エラー.
    pub(crate) fn of(item: Tag, counterpart: Tag) -> Result<Self, TransactionError> {
        match (item, counterpart) {
            (Tag::Income | Tag::Liability | Tag::OpeningBalance, Tag::Asset)
            | (Tag::Expense, Tag::Liability) => Ok(Self::Same),
            (Tag::Expense | Tag::Asset, Tag::Asset) | (Tag::OpeningBalance, Tag::Liability) => {
                Ok(Self::Opposite)
            }
            _ => Err(TransactionError::InvalidPair(item, counterpart)),
        }
    }

    /// 品目の増減 `amount` から相手先の増減を求める.
    ///
    /// # Errors
    ///
    /// 符号を反転できない場合は `TransactionError::CounterpartOverflow` エラー.
    fn apply(self, amount: Money) -> Result<Money, TransactionError> {
        match self {
            Self::Same => Ok(amount),
            Self::Opposite => amount
                .checked_neg()
                .map_err(|_| TransactionError::CounterpartOverflow(amount)),
        }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum TransactionError {
    /// 品目と相手先のタグが取引にできない組の場合.
    #[error("品目のタグ {0} と相手先のタグ {1} の組は取引にできません")]
    InvalidPair(Tag, Tag),
    /// 品目と相手先が同じ場合.
    #[error("品目と相手先がどちらも {0:?} です")]
    SameItem(ItemId),
    /// 相手先の増減が金額の範囲を超える場合.
    #[error("金額 {0} の符号を反転した相手先の増減は範囲を超えています")]
    CounterpartOverflow(Money),
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

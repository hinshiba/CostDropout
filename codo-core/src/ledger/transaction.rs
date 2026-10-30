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

    /// 取引にできる組と, その向き.
    const PAIRS: [(Tag, Tag, Direction); 7] = [
        (Tag::Income, Tag::Asset, Direction::Same),
        (Tag::Expense, Tag::Asset, Direction::Opposite),
        (Tag::Expense, Tag::Liability, Direction::Same),
        (Tag::Liability, Tag::Asset, Direction::Same),
        (Tag::Asset, Tag::Asset, Direction::Opposite),
        (Tag::OpeningBalance, Tag::Asset, Direction::Same),
        (Tag::OpeningBalance, Tag::Liability, Direction::Opposite),
    ];

    /// `item` と `counterpart` の取引の相手先の増減を求める.
    fn counterpart_amount(
        item: Tag,
        counterpart: Tag,
        amount: i32,
    ) -> Result<Money, TransactionError> {
        let [item_id, counterpart_id] = [ItemId(1), ItemId(2)];
        let draft = TransactionDraft::new(
            NaiveDate::from_ymd_opt(2026, 10, 12).unwrap(),
            Money::from(amount),
            item_id,
            counterpart_id,
            String::new(),
        );
        draft.counterpart_amount(|id| Ok(if id == item_id { item } else { counterpart }))
    }

    #[test]
    fn direction_of_test() {
        for (item, counterpart, expected) in PAIRS {
            assert_eq!(
                Direction::of(item, counterpart),
                Ok(expected),
                "{item:?} {counterpart:?}"
            );
        }
    }

    #[test]
    fn direction_of_error_test() {
        for item in Tag::ALL {
            for counterpart in Tag::ALL {
                if PAIRS.iter().any(|&(i, c, _)| (i, c) == (item, counterpart)) {
                    continue;
                }
                assert_eq!(
                    Direction::of(item, counterpart),
                    Err(TransactionError::InvalidPair(item, counterpart)),
                    "{item:?} {counterpart:?}"
                );
            }
        }
    }

    #[test]
    fn counterpart_amount_test() {
        for (item, counterpart, direction) in PAIRS {
            for amount in [-1000, 0, 1000, i32::MAX] {
                let expected = match direction {
                    Direction::Same => amount,
                    Direction::Opposite => -amount,
                };
                assert_eq!(
                    counterpart_amount(item, counterpart, amount),
                    Ok(Money::from(expected)),
                    "{item:?} {counterpart:?} {amount}"
                );
            }
        }
    }

    /// 同じ符号なら `i32::MIN` も相手先の増減として表せるが, 異なる符号では表せない.
    #[test]
    fn counterpart_amount_of_min_test() {
        let min = Money::from(i32::MIN);
        assert_eq!(
            counterpart_amount(Tag::Income, Tag::Asset, i32::MIN),
            Ok(min)
        );
        assert_eq!(
            counterpart_amount(Tag::Expense, Tag::Asset, i32::MIN),
            Err(TransactionError::CounterpartOverflow(min))
        );
    }

    #[test]
    fn counterpart_amount_rejects_same_item_test() {
        let cash = ItemId(1);
        let draft = TransactionDraft::new(
            NaiveDate::from_ymd_opt(2026, 10, 12).unwrap(),
            Money::from(1000),
            cash,
            cash,
            String::new(),
        );

        assert_eq!(
            draft.counterpart_amount(|_| Ok::<_, TransactionError>(Tag::Asset)),
            Err(TransactionError::SameItem(cash))
        );
    }
}

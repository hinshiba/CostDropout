//! 取引

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::item::{ItemId, Tag};

mod amount;

pub use amount::{Amount, AmountError};

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

/// 取引の一覧
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TransactionList {
    transactions: Vec<Transaction>,
}

impl TransactionList {
    ///空の取引一覧を作る
    pub fn new() -> Self {
        Self::default()
    }

    ///ファイルの復元
    pub fn from_transactions(transactions: Vec<Transaction>) -> Self {
        let mut list = Self { transactions };
        list.sort();
        list
    }

    ///取引の登録
    pub fn add(&mut self, draft: TransactionDraft) -> Result<TransactionId, TransactionError> {
        let new_id = TransactionId(Uuid::new_v4());
        self.transactions.push(Transaction { id: new_id, draft });
        self.sort();
        Ok(new_id)
    }

    ///特定の品目を使っている取引があるかの確認
    pub fn uses_item(&self, item: ItemId) -> bool {
        self.transactions.iter().any(|t| t.uses(item))
    }

    ///IDで取引を探す
    pub fn get(&self, id: TransactionId) -> Option<&Transaction> {
        self.transactions.iter().find(|t| t.id == id)
    }

    ///取引の削除
    pub fn remove(&mut self, id: TransactionId) -> Result<Transaction, TransactionError> {
        let i = self.index_of(id)?;
        Ok(self.transactions.remove(i))
    }

    ///取引の訂正
    pub fn update(
        &mut self,
        id: TransactionId,
        draft: TransactionDraft,
    ) -> Result<(), TransactionError> {
        let i = self.index_of(id)?;
        self.transactions[i].draft = draft;
        self.sort();
        Ok(())
    }

    ///日付順に取引を取り出す
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &Transaction> {
        self.transactions.iter()
    }

    ///日付順・ID順に並び替える
    fn sort(&mut self) {
        self.transactions.sort_by_key(|t| (t.draft.date, t.id))
    }

    ///対象が何番目にあるかを探す
    fn index_of(&self, id: TransactionId) -> Result<usize, TransactionError> {
        for i in 0..self.transactions.len() {
            if self.transactions[i].id == id {
                return Ok(i);
            }
        }
        Err(TransactionError::NotFound(id))
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum TransactionError {
    /// 品目のタグが取引の品目として選べない場合.
    #[error("取引の品目のタグ {0} が収入・支出・負債ではありません")]
    InvalidItemTag(Tag),
    /// 相手先のタグが品目のタグと同じ場合.
    #[error("相手先のタグ {0} が品目のタグと同じです")]
    SameTag(Tag),
    #[error("取引 {0:?} が見つかりません")]
    NotFound(TransactionId),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item::{ItemList, Tag};

    /// テスト用の日付(2026年10月の指定した日)
    fn date(day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, day).unwrap()
    }

    /// テスト用の draft を作る(コメントは "テスト" で固定)
    fn draft(day: u32, amount: i32, item: ItemId, counterpart: ItemId) -> TransactionDraft {
        TransactionDraft {
            date: date(day),
            amount: Amount::of(amount),
            item,
            counterpart,
            comment: String::from("テスト"),
        }
    }

    ///テスト用の品目を3つ登録して、その ID を返す
    fn items() -> (ItemId, ItemId, ItemId) {
        let mut list = ItemList::new();
        let a = list.add("食費", Tag::Expense).unwrap();
        let b = list.add("現金", Tag::Asset).unwrap();
        let c = list.add("娯楽", Tag::Expense).unwrap();
        (a, b, c)
    }

    #[test]
    fn add_then_get() {
        let (a, b, _) = items();
        let mut list = TransactionList::new();

        let id = list.add(draft(1, 1000, a, b)).unwrap();

        let t = list.get(id).unwrap();
        assert_eq!(t.id, id);
        assert_eq!(t.date(), date(1));
        assert_eq!(t.amount(), Amount::of(1000));
        assert_eq!(t.item(), a);
        assert_eq!(t.counterpart(), b);
        assert_eq!(t.comment(), "テスト");
    }

    #[test]
    fn add_accepts_one_yen() {
        let (a, b, _) = items();
        let mut list = TransactionList::new();

        let id = list.add(draft(1, 1, a, b)).unwrap();

        let t = list.get(id).unwrap();
        assert_eq!(t.amount(), Amount::of(1));
    }

    #[test]
    fn remove_deletes() {
        let (a, b, _) = items();
        let mut list = TransactionList::new();
        let id = list.add(draft(1, 1000, b, a)).unwrap();
        let removed = list.remove(id).unwrap();
        assert_eq!(removed.id, id);
        assert_eq!(list.get(id), None);
    }

    #[test]
    fn remove_not_found() {
        let mut list = TransactionList::new();
        let missing = TransactionId(Uuid::new_v4());

        let result = list.remove(missing);

        assert_eq!(result, Err(TransactionError::NotFound(missing)));
    }

    #[test]
    fn update_changes() {
        let (a, b, c) = items();
        let mut list = TransactionList::new();
        let id = list.add(draft(1, 1000, a, b)).unwrap();

        let result = list.update(id, draft(5, 2000, c, a));

        assert_eq!(result, Ok(()));
        let t = list.get(id).unwrap();
        assert_eq!(t.id, id);
        assert_eq!(t.date(), date(5));
        assert_eq!(t.amount(), Amount::of(2000));
        assert_eq!(t.item(), c);
        assert_eq!(t.counterpart(), a);
    }

    #[test]
    fn update_not_found() {
        let (a, b, _) = items();
        let mut list = TransactionList::new();
        let missing = TransactionId(Uuid::new_v4());

        let result = list.update(missing, draft(1, 1000, a, b));

        assert_eq!(result, Err(TransactionError::NotFound(missing)));
    }

    #[test]
    fn uses_item_checks_item_and_counterpart() {
        let (a, b, c) = items();
        let mut list = TransactionList::new();
        list.add(draft(1, 1000, a, b)).unwrap();

        // 品目として使っている
        assert!(list.uses_item(a));
        // 相手先として使っている
        assert!(list.uses_item(b));
        // 使っていない
        assert!(!list.uses_item(c));
    }

    #[test]
    fn iter_sorted_by_date() {
        let (a, b, _) = items();
        let mut list = TransactionList::new();
        list.add(draft(3, 1000, a, b)).unwrap();
        list.add(draft(1, 1000, a, b)).unwrap();
        list.add(draft(2, 1000, a, b)).unwrap();

        let mut dates = Vec::new();
        for t in list.iter() {
            dates.push(t.date());
        }

        assert_eq!(dates, vec![date(1), date(2), date(3)]);
    }

    #[test]
    fn iter_same_date_sorted_by_id() {
        let (a, b, _) = items();
        let mut list = TransactionList::new();
        let id1 = list.add(draft(1, 1000, a, b)).unwrap();
        let id2 = list.add(draft(1, 2000, a, b)).unwrap();
        let id3 = list.add(draft(1, 3000, a, b)).unwrap();

        let mut ids = Vec::new();
        for t in list.iter() {
            ids.push(t.id);
        }

        // ID はランダムなので、期待する順番は自分で並べ替えて作る
        let mut expected = vec![id1, id2, id3];
        expected.sort();
        assert_eq!(ids, expected);
    }

    #[test]
    fn update_keeps_date_order() {
        let (a, b, _) = items();
        let mut list = TransactionList::new();
        let id = list.add(draft(1, 1000, a, b)).unwrap();
        list.add(draft(2, 1000, a, b)).unwrap();

        // 10/1 の取引を 10/3 に訂正すると、一番後ろに移る
        list.update(id, draft(3, 1000, a, b)).unwrap();

        let mut dates = Vec::new();
        for t in list.iter() {
            dates.push(t.date());
        }
        assert_eq!(dates, vec![date(2), date(3)]);
    }
}

//! 取引

use chrono::NaiveDate;
use thiserror::Error;
use uuid::Uuid;

use crate::item::ItemId;
use crate::money::Money;

/// 取引 ID
///
/// 別の月ファイルへ同時に追加されても衝突しないよう uuid を使う
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TransactionId(pub Uuid);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transaction {
    pub id: TransactionId,
    pub date: NaiveDate,
    /// 正の値のみ
    pub amount: Money,
    /// 収入/支出/負債の品目
    pub item: ItemId,
    /// 相手先の品目
    pub counterpart: ItemId,
    pub comment: String,
}

///idなしの取引
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransactionDraft {
    pub date: NaiveDate,
    pub amount: Money,
    pub item: ItemId,
    pub counterpart: ItemId,
    pub comment: String,
}

impl TransactionDraft {
    ///金額と品目のチェック
    fn validate(&self) -> Result<(), TransactionError> {
        //金額が0以下の場合のエラー確認
        if self.amount.0 <= 0 {
            return Err(TransactionError::NonPositiveAmount(self.amount));
        }
        //品目が相手先と同じの場合のエラー確認
        if self.item == self.counterpart {
            return Err(TransactionError::SameItemAndCounterpart(self.item));
        }
        Ok(())
    }
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
        //金額，品目のチェック
        draft.validate()?;
        let new_id = TransactionId(Uuid::new_v4());
        let transaction = Transaction {
            id: new_id,
            date: draft.date,
            amount: draft.amount,
            item: draft.item,
            counterpart: draft.counterpart,
            comment: draft.comment,
        };
        self.transactions.push(transaction);
        self.sort();
        Ok(new_id)
    }

    ///特定の品目を使っている取引があるかの確認
    pub fn uses_item(&self, item: ItemId) -> bool {
        for t in &self.transactions {
            if t.item == item || t.counterpart == item {
                return true;
            }
        }
        false
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
        //金額,品目のチェック
        draft.validate()?;

        //探して書き換える
        let i = self.index_of(id)?;
        self.transactions[i].date = draft.date;
        self.transactions[i].amount = draft.amount;
        self.transactions[i].item = draft.item;
        self.transactions[i].counterpart = draft.counterpart;
        self.transactions[i].comment = draft.comment;
        self.sort();
        Ok(())
    }

    ///日付順に取引を取り出す
    pub fn iter(&self) -> impl Iterator<Item = &Transaction> {
        self.transactions.iter()
    }

    ///日付順・ID順に並び替える
    fn sort(&mut self) {
        self.transactions.sort_by_key(|t| (t.date, t.id))
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
    #[error("金額は正の値でなければなりません: {0:?}")]
    NonPositiveAmount(Money),
    #[error("品目と相手先が同じです: {0:?}")]
    SameItemAndCounterpart(ItemId),
    #[error("取引 {0:?} が見つかりません")]
    NotFound(TransactionId),
}

#[cfg(test)]
mod tests {
    use super::*;

    /// テスト用の日付(2026年10月の指定した日)
    fn date(day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, day).unwrap()
    }

    /// テスト用の draft を作る(コメントは "テスト" で固定)
    fn draft(day: u32, amount: i32, item: u32, counterpart: u32) -> TransactionDraft {
        TransactionDraft {
            date: date(day),
            amount: Money(amount),
            item: ItemId(item),
            counterpart: ItemId(counterpart),
            comment: String::from("テスト"),
        }
    }

    #[test]
    fn add_then_get() {
        let mut list = TransactionList::new();

        let id = list.add(draft(1, 1000, 1, 2)).unwrap();

        let t = list.get(id).unwrap();
        assert_eq!(t.id, id);
        assert_eq!(t.date, date(1));
        assert_eq!(t.amount, Money(1000));
        assert_eq!(t.item, ItemId(1));
        assert_eq!(t.counterpart, ItemId(2));
        assert_eq!(t.comment, "テスト");
    }

    #[test]
    fn add_rejects_zero_amount() {
        let mut list = TransactionList::new();
        let result = list.add(draft(1, 0, 1, 2));
        assert_eq!(result, Err(TransactionError::NonPositiveAmount(Money(0))));
    }

    #[test]
    fn add_rejects_same_item_and_counterpart() {
        let mut list = TransactionList::new();
        let result = list.add(draft(1, 1000, 1, 1));
        assert_eq!(
            result,
            Err(TransactionError::SameItemAndCounterpart(ItemId(1)))
        );
    }

    #[test]
    fn remove_deletes() {
        let mut list = TransactionList::new();
        let id = list.add(draft(1, 1000, 2, 1)).unwrap();
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
    fn add_rejects_negative_amount() {
        let mut list = TransactionList::new();

        let result = list.add(draft(1, -500, 1, 2));

        assert_eq!(
            result,
            Err(TransactionError::NonPositiveAmount(Money(-500)))
        );
    }

    #[test]
    fn update_changes() {
        let mut list = TransactionList::new();
        let id = list.add(draft(1, 1000, 1, 2)).unwrap();

        let result = list.update(id, draft(5, 2000, 3, 4));

        assert_eq!(result, Ok(()));
        let t = list.get(id).unwrap();
        assert_eq!(t.id, id);
        assert_eq!(t.date, date(5));
        assert_eq!(t.amount, Money(2000));
        assert_eq!(t.item, ItemId(3));
        assert_eq!(t.counterpart, ItemId(4));
    }

    #[test]
    fn update_not_found() {
        let mut list = TransactionList::new();
        let missing = TransactionId(Uuid::new_v4());

        let result = list.update(missing, draft(1, 1000, 1, 2));

        assert_eq!(result, Err(TransactionError::NotFound(missing)));
    }

    #[test]
    fn update_rejects_zero_amount() {
        let mut list = TransactionList::new();
        let id = list.add(draft(1, 1000, 1, 2)).unwrap();

        let result = list.update(id, draft(1, 0, 1, 2));

        assert_eq!(result, Err(TransactionError::NonPositiveAmount(Money(0))));
        // 失敗したときは元の取引のまま
        assert_eq!(list.get(id).unwrap().amount, Money(1000));
    }

    #[test]
    fn update_rejects_same_item_and_counterpart() {
        let mut list = TransactionList::new();
        let id = list.add(draft(1, 1000, 1, 2)).unwrap();

        let result = list.update(id, draft(1, 1000, 3, 3));

        assert_eq!(
            result,
            Err(TransactionError::SameItemAndCounterpart(ItemId(3)))
        );
    }

    #[test]
    fn uses_item_checks_item_and_counterpart() {
        let mut list = TransactionList::new();
        list.add(draft(1, 1000, 1, 2)).unwrap();

        // 品目として使っている
        assert!(list.uses_item(ItemId(1)));
        // 相手先として使っている
        assert!(list.uses_item(ItemId(2)));
        // 使っていない
        assert!(!list.uses_item(ItemId(3)));
    }

    #[test]
    fn iter_sorted_by_date() {
        let mut list = TransactionList::new();
        list.add(draft(3, 1000, 1, 2)).unwrap();
        list.add(draft(1, 1000, 1, 2)).unwrap();
        list.add(draft(2, 1000, 1, 2)).unwrap();

        let mut dates = Vec::new();
        for t in list.iter() {
            dates.push(t.date);
        }

        assert_eq!(dates, vec![date(1), date(2), date(3)]);
    }

    #[test]
    fn iter_same_date_sorted_by_id() {
        let mut list = TransactionList::new();
        let a = list.add(draft(1, 1000, 1, 2)).unwrap();
        let b = list.add(draft(1, 2000, 1, 2)).unwrap();
        let c = list.add(draft(1, 3000, 1, 2)).unwrap();

        let mut ids = Vec::new();
        for t in list.iter() {
            ids.push(t.id);
        }

        // ID はランダムなので、期待する順番は自分で並べ替えて作る
        let mut expected = vec![a, b, c];
        expected.sort();
        assert_eq!(ids, expected);
    }

    #[test]
    fn update_keeps_date_order() {
        let mut list = TransactionList::new();
        let id = list.add(draft(1, 1000, 1, 2)).unwrap();
        list.add(draft(2, 1000, 1, 2)).unwrap();

        // 10/1 の取引を 10/3 に訂正すると、一番後ろに移る
        list.update(id, draft(3, 1000, 1, 2)).unwrap();

        let mut dates = Vec::new();
        for t in list.iter() {
            dates.push(t.date);
        }
        assert_eq!(dates, vec![date(2), date(3)]);
    }

    #[test]
    fn from_transactions_sorts_without_validation() {
        let transaction = |day: u32, amount: i32| Transaction {
            id: TransactionId(Uuid::new_v4()),
            date: date(day),
            amount: Money(amount),
            item: ItemId(1),
            counterpart: ItemId(2),
            comment: String::from("テスト"),
        };
        // 金額 0 の取引も検証せずに受け入れる
        let list = TransactionList::from_transactions(vec![
            transaction(3, 1000),
            transaction(1, 0),
            transaction(2, 1000),
        ]);

        let mut dates = Vec::new();
        for t in list.iter() {
            dates.push(t.date);
        }
        assert_eq!(dates, vec![date(1), date(2), date(3)]);
    }
}

//! 取引一覧

use std::collections::HashSet;
use uuid::Uuid;

use super::{Transaction, TransactionDraft, TransactionError, TransactionId};
use crate::item::ItemId;
use crate::period::Period;

/// 取引一覧.
///
/// 次の要件を満たす.
/// * 日付の昇順, 同じ日付なら ID の昇順に並ぶ
/// * 互いに異なる ID をもつ
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TransactionList {
    transactions: Vec<Transaction>,
}

impl TransactionList {
    /// `Vec<Transaction>` から取引一覧を作成する.
    ///
    /// # Errors
    ///
    /// ID が重複する場合は `TransactionError::DuplicateId` エラー.
    pub(crate) fn from_transactions(
        transactions: Vec<Transaction>,
    ) -> Result<Self, TransactionError> {
        let mut ids = HashSet::new();
        if let Some(t) = transactions.iter().find(|t| !ids.insert(t.id)) {
            return Err(TransactionError::DuplicateId(t.id));
        }

        let mut list = Self { transactions };
        list.sort();
        Ok(list)
    }

    /// 取引を走査するイテレーターを返す.
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &Transaction> {
        self.transactions.iter()
    }

    /// 日付が `period` に含まれる取引を走査するイテレーターを返す.
    pub fn iter_in(&self, period: Period) -> impl DoubleEndedIterator<Item = &Transaction> {
        // 期間の両端を二分探索する
        let start = self
            .transactions
            .partition_point(|t| t.draft.date < period.start());
        let end = self
            .transactions
            .partition_point(|t| t.draft.date <= period.end());
        self.transactions[start..end].iter()
    }

    /// ID が `id` である取引を返す.
    pub fn get(&self, id: TransactionId) -> Option<&Transaction> {
        self.transactions.iter().find(|t| t.id == id)
    }

    /// 品目または相手先として `item` を使う取引があるかを判定する.
    pub(crate) fn uses_item(&self, item: ItemId) -> bool {
        self.transactions.iter().any(|t| t.uses(item))
    }

    /// 取引を追加し, 新しい ID を割り当てる.
    pub(crate) fn add(&mut self, draft: TransactionDraft) -> TransactionId {
        let id = TransactionId(Uuid::new_v4());
        self.transactions.push(Transaction { id, draft });
        self.sort();
        id
    }

    /// 取引の内容を `draft` に置き換える.
    ///
    /// # Errors
    ///
    /// 取引が存在しない場合は `TransactionError::NotFound` エラー.
    pub(crate) fn update(
        &mut self,
        id: TransactionId,
        draft: TransactionDraft,
    ) -> Result<(), TransactionError> {
        let index = self.index_of(id)?;
        self.transactions[index].draft = draft;
        self.sort();
        Ok(())
    }

    /// 取引を削除する.
    ///
    /// # Errors
    ///
    /// 取引が存在しない場合は `TransactionError::NotFound` エラー.
    pub(crate) fn remove(&mut self, id: TransactionId) -> Result<(), TransactionError> {
        let index = self.index_of(id)?;
        self.transactions.remove(index);
        Ok(())
    }

    fn sort(&mut self) {
        self.transactions.sort_by_key(|t| (t.draft.date, t.id));
    }

    fn index_of(&self, id: TransactionId) -> Result<usize, TransactionError> {
        self.transactions
            .iter()
            .position(|t| t.id == id)
            .ok_or(TransactionError::NotFound(id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item::{ItemList, Tag};
    use crate::transaction::Amount;
    use chrono::NaiveDate;

    /// 2026年10月の `day` 日.
    fn date(day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, day).unwrap()
    }

    /// 食費 (支出), 現金 (資産), 娯楽 (支出) の ID.
    fn items() -> [ItemId; 3] {
        let mut list = ItemList::new();
        ["食費", "現金", "娯楽"]
            .into_iter()
            .zip([Tag::Expense, Tag::Asset, Tag::Expense])
            .map(|(name, tag)| list.add(name, tag).unwrap())
            .collect::<Vec<_>>()
            .try_into()
            .unwrap()
    }

    fn draft(day: u32, amount: i32, item: ItemId, counterpart: ItemId) -> TransactionDraft {
        TransactionDraft::new(
            date(day),
            Amount::of(amount),
            item,
            counterpart,
            String::from("テスト"),
        )
    }

    fn transaction(id: u128, date: NaiveDate) -> Transaction {
        let [food, cash, _] = items();
        Transaction {
            id: TransactionId(Uuid::from_u128(id)),
            draft: TransactionDraft::new(date, Amount::of(1000), food, cash, String::new()),
        }
    }

    fn ids(list: &TransactionList) -> Vec<TransactionId> {
        list.iter().map(Transaction::id).collect()
    }

    fn id(value: u128) -> TransactionId {
        TransactionId(Uuid::from_u128(value))
    }

    #[test]
    fn from_transactions_test() {
        let list = TransactionList::from_transactions(vec![
            transaction(3, date(1)),
            transaction(1, date(2)),
            transaction(2, date(1)),
        ])
        .unwrap();

        assert_eq!(ids(&list), [2, 3, 1].map(id));
    }

    #[test]
    fn from_transactions_error_test() {
        assert_eq!(
            TransactionList::from_transactions(vec![
                transaction(1, date(1)),
                transaction(2, date(1)),
                transaction(1, date(2)),
            ]),
            Err(TransactionError::DuplicateId(id(1)))
        );
    }

    #[test]
    fn iter_in_test() {
        let list = TransactionList::from_transactions(vec![
            transaction(1, NaiveDate::from_ymd_opt(2026, 9, 30).unwrap()),
            transaction(2, date(1)),
            transaction(3, date(2)),
            transaction(4, date(15)),
            transaction(5, date(31)),
            transaction(6, NaiveDate::from_ymd_opt(2026, 11, 1).unwrap()),
        ])
        .unwrap();
        let in_period = |start, end| {
            let period = Period::new(date(start), date(end)).unwrap();
            list.iter_in(period)
                .map(Transaction::id)
                .collect::<Vec<_>>()
        };

        // 両端の日を含み, 期間の外は含まない
        assert_eq!(in_period(1, 31), [2, 3, 4, 5].map(id));
        assert_eq!(in_period(2, 15), [3, 4].map(id));
        assert_eq!(in_period(3, 14), []);
        let period = Period::new(date(1), date(31)).unwrap();
        let reversed = list.iter_in(period).rev().map(Transaction::id);
        assert_eq!(reversed.collect::<Vec<_>>(), [5, 4, 3, 2].map(id));
    }

    #[test]
    fn add_test() {
        let [food, cash, _] = items();
        let mut list = TransactionList::default();

        let added = list.add(draft(1, 1000, food, cash));

        let expected = Transaction {
            id: added,
            draft: draft(1, 1000, food, cash),
        };
        assert_eq!(list.get(added), Some(&expected));
    }

    #[test]
    fn add_keeps_order_test() {
        let [food, cash, _] = items();
        let mut list = TransactionList::default();

        let added = [3, 1, 2, 1].map(|day| list.add(draft(day, 1000, food, cash)));

        let dates = list.iter().map(Transaction::date).collect::<Vec<_>>();
        assert_eq!(dates, vec![date(1), date(1), date(2), date(3)]);
        // 同じ日付なら ID 順
        let (first, second) = (added[1].min(added[3]), added[1].max(added[3]));
        assert_eq!(ids(&list)[..2], [first, second]);
    }

    #[test]
    fn update_test() {
        let [food, cash, fun] = items();
        let mut list = TransactionList::default();
        let target = list.add(draft(1, 1000, food, cash));
        let other = list.add(draft(2, 1000, food, cash));

        list.update(target, draft(3, 2000, fun, cash)).unwrap();

        assert_eq!(list.get(target).map(Transaction::comment), Some("テスト"));
        assert_eq!(list.get(target).unwrap().draft, draft(3, 2000, fun, cash));
        // 10/1 の取引を 10/3 に訂正すると後ろに移る
        assert_eq!(ids(&list), vec![other, target]);
    }

    #[test]
    fn update_error_test() {
        let [food, cash, _] = items();
        let mut list = TransactionList::default();
        list.add(draft(1, 1000, food, cash));
        let before = list.clone();
        let missing = id(0);

        assert_eq!(
            list.update(missing, draft(2, 2000, food, cash)),
            Err(TransactionError::NotFound(missing))
        );
        assert_eq!(list, before);
    }

    #[test]
    fn remove_test() {
        let [food, cash, _] = items();
        let mut list = TransactionList::default();
        let target = list.add(draft(1, 1000, food, cash));
        let other = list.add(draft(2, 1000, food, cash));

        list.remove(target).unwrap();

        assert_eq!(ids(&list), vec![other]);
    }

    #[test]
    fn remove_error_test() {
        let [food, cash, _] = items();
        let mut list = TransactionList::default();
        list.add(draft(1, 1000, food, cash));
        let before = list.clone();
        let missing = id(0);

        assert_eq!(
            list.remove(missing),
            Err(TransactionError::NotFound(missing))
        );
        assert_eq!(list, before);
    }

    #[test]
    fn uses_item_test() {
        let [food, cash, fun] = items();
        let mut list = TransactionList::default();
        list.add(draft(1, 1000, food, cash));

        for (item, expected) in [(food, true), (cash, true), (fun, false)] {
            assert_eq!(list.uses_item(item), expected, "{item:?}");
        }
    }
}

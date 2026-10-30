//! 帳簿とその操作
use crate::budget::{BudgetError, BudgetId, BudgetList};
use crate::item::{Item, ItemError, ItemId, ItemList, Tag};
use crate::money::Money;
use crate::period::Period;
use crate::preset::preset_items;
use crate::transaction::{
    TransactionDraft, TransactionError, TransactionId, TransactionList, check_item_tag, check_tags,
};
use thiserror::Error;

/// 帳簿.
///
/// 品目一覧, 取引一覧, 予算一覧をもち, 次の要件を満たす.
/// * 取引の品目と相手先は品目一覧に存在する
/// * 取引の品目と相手先のタグの組み合わせは `check_tags` を満たす
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Ledger {
    items: ItemList,
    transactions: TransactionList,
    budgets: BudgetList,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum LedgerError {
    /// 品目一覧の操作に失敗した場合.
    #[error(transparent)]
    Item(#[from] ItemError),
    /// 取引一覧の操作に失敗した場合と, 取引のタグの組み合わせが不正な場合.
    #[error(transparent)]
    Transaction(#[from] TransactionError),
    /// 予算一覧の操作に失敗した場合.
    #[error(transparent)]
    Budget(#[from] BudgetError),
    /// 取引から使われている品目を削除しようとした場合.
    #[error("取引から参照されている品目 {0:?} は削除できません")]
    ItemUse(ItemId),
    /// 取引の品目または相手先が存在しない場合.
    #[error("存在しない取引の品目 {0:?} が指定されています")]
    TransactionItemNotFound(ItemId),
}

impl Ledger {
    /// 新しい空の帳簿を作成する.
    pub fn new() -> Self {
        Self::default()
    }

    /// プリセット品目を登録した帳簿を作成する.
    pub fn with_presets() -> Self {
        Self {
            items: preset_items(),
            ..Self::default()
        }
    }

    /// 品目一覧, 取引一覧, 予算一覧から帳簿を作成する.
    ///
    /// # Errors
    ///
    /// 取引の品目または相手先が存在しない場合は `LedgerError::TransactionItemNotFound` エラー.
    /// 取引のタグの組み合わせが不正な場合は `LedgerError::Transaction` エラー.
    pub(crate) fn from_parts(
        items: ItemList,
        transactions: TransactionList,
        budgets: BudgetList,
    ) -> Result<Self, LedgerError> {
        let ledger = Self {
            items,
            transactions,
            budgets,
        };
        for t in ledger.transactions.iter() {
            t.check_tags(|id| ledger.tag_of(id))?;
        }
        Ok(ledger)
    }

    /// 品目一覧を返す.
    pub fn items(&self) -> &ItemList {
        &self.items
    }

    /// 取引一覧を返す.
    pub fn transactions(&self) -> &TransactionList {
        &self.transactions
    }

    /// 予算一覧を返す.
    pub fn budgets(&self) -> &BudgetList {
        &self.budgets
    }

    /// 取引の品目として選べる品目を返す.
    pub fn item_choices(&self) -> impl Iterator<Item = &Item> {
        self.items
            .iter()
            .filter(|item| check_item_tag(item.tag()).is_ok())
    }

    /// 品目が `item` である取引の相手先として選べる品目を返す.
    ///
    /// `item` が存在しないか, 取引の品目として選べない場合は空.
    pub fn counterpart_choices(&self, item: ItemId) -> impl Iterator<Item = &Item> {
        let tag = self.items.get(item).map(Item::tag);
        self.items.iter().filter(move |candidate| {
            tag.is_some_and(|tag| check_tags(tag, candidate.tag()).is_ok())
        })
    }

    /// 品目を追加する.
    ///
    /// # Errors
    ///
    /// `ItemList::add` のエラー.
    pub fn add_item(&mut self, name: &str, tag: Tag) -> Result<ItemId, LedgerError> {
        Ok(self.items.add(name, tag)?)
    }

    /// 品目の名前とタグを更新する.
    ///
    /// # Errors
    ///
    /// 品目が存在しない場合は `ItemError::NotFound` エラー.
    /// この品目を使う取引のタグの組み合わせが不正になる場合は `LedgerError::Transaction` エラー.
    /// そのほか `ItemList::update` のエラー.
    pub fn update_item(&mut self, id: ItemId, name: &str, tag: Tag) -> Result<(), LedgerError> {
        let item = self.items.get(id).ok_or(ItemError::NotFound(id))?;

        if item.tag() != tag {
            let tag_after = |item_id| {
                if item_id == id {
                    Ok(tag)
                } else {
                    self.tag_of(item_id)
                }
            };
            for t in self.transactions.iter().filter(|t| t.uses(id)) {
                t.check_tags(tag_after)?;
            }
        }
        Ok(self.items.update(id, name, tag)?)
    }

    /// 品目を削除する.
    ///
    /// # Errors
    ///
    /// 取引の品目または相手先として使われている場合は `LedgerError::ItemUse` エラー.
    /// 品目が存在しない場合は `ItemError::NotFound` エラー.
    pub fn remove_item(&mut self, id: ItemId) -> Result<(), LedgerError> {
        if self.transactions.uses_item(id) {
            return Err(LedgerError::ItemUse(id));
        }
        Ok(self.items.remove(id)?)
    }

    fn tag_of(&self, id: ItemId) -> Result<Tag, LedgerError> {
        self.items
            .get(id)
            .map(Item::tag)
            .ok_or(LedgerError::TransactionItemNotFound(id))
    }

    /// 取引を追加する.
    ///
    /// # Errors
    ///
    /// 品目または相手先が存在しない場合は `LedgerError::TransactionItemNotFound` エラー.
    /// タグの組み合わせが不正な場合は `LedgerError::Transaction` エラー.
    pub fn add_transaction(
        &mut self,
        draft: TransactionDraft,
    ) -> Result<TransactionId, LedgerError> {
        draft.check_tags(|id| self.tag_of(id))?;
        Ok(self.transactions.add(draft))
    }

    /// 取引の内容を `draft` に置き換える.
    ///
    /// # Errors
    ///
    /// 品目または相手先が存在しない場合は `LedgerError::TransactionItemNotFound` エラー.
    /// タグの組み合わせが不正な場合と, 取引が存在しない場合は `LedgerError::Transaction` エラー.
    pub fn update_transaction(
        &mut self,
        id: TransactionId,
        draft: TransactionDraft,
    ) -> Result<(), LedgerError> {
        draft.check_tags(|id| self.tag_of(id))?;
        Ok(self.transactions.update(id, draft)?)
    }

    /// 取引を削除する.
    ///
    /// # Errors
    ///
    /// 取引が存在しない場合は `TransactionError::NotFound` エラー.
    pub fn remove_transaction(&mut self, id: TransactionId) -> Result<(), LedgerError> {
        Ok(self.transactions.remove(id)?)
    }

    /// 予算を追加する.
    ///
    /// # Errors
    ///
    /// `BudgetList::add` のエラー.
    pub fn add_budget(&mut self, amount: Money, period: Period) -> Result<BudgetId, LedgerError> {
        Ok(self.budgets.add(amount, period)?)
    }

    /// 予算の金額と期間を更新する.
    ///
    /// # Errors
    ///
    /// `BudgetList::update` のエラー.
    pub fn update_budget(
        &mut self,
        id: BudgetId,
        amount: Money,
        period: Period,
    ) -> Result<(), LedgerError> {
        Ok(self.budgets.update(id, amount, period)?)
    }

    /// 予算を削除する.
    ///
    /// # Errors
    ///
    /// `BudgetList::remove` のエラー.
    pub fn remove_budget(&mut self, id: BudgetId) -> Result<(), LedgerError> {
        Ok(self.budgets.remove(id)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::money::Money;
    use crate::period::YearMonth;
    use crate::transaction::Amount;
    use chrono::NaiveDate;

    fn draft(item: ItemId, counterpart: ItemId) -> TransactionDraft {
        TransactionDraft::new(
            NaiveDate::from_ymd_opt(2026, 10, 12).unwrap(),
            Amount::of(1000),
            item,
            counterpart,
            String::new(),
        )
    }

    /// 給与 (収入), 現金 (資産), 食費 (支出), 娯楽 (支出), 登録してから削除した品目.
    fn ledger() -> (Ledger, [ItemId; 5]) {
        let mut ledger = Ledger::new();
        let removed = ledger.add_item("削除する品目", Tag::Expense).unwrap();
        let [salary, cash, food, fun] = [
            ("給与", Tag::Income),
            ("現金", Tag::Asset),
            ("食費", Tag::Expense),
            ("娯楽", Tag::Expense),
        ]
        .map(|(name, tag)| ledger.add_item(name, tag).unwrap());
        ledger.remove_item(removed).unwrap();
        (ledger, [salary, cash, food, fun, removed])
    }

    /// 品目と相手先の組み合わせが不正な取引と, そのエラー.
    fn invalid_pairs(ids: [ItemId; 5]) -> [(ItemId, ItemId, LedgerError); 5] {
        let [salary, cash, food, fun, removed] = ids;
        [
            (removed, cash, LedgerError::TransactionItemNotFound(removed)),
            (food, removed, LedgerError::TransactionItemNotFound(removed)),
            (
                cash,
                salary,
                TransactionError::InvalidItemTag(Tag::Asset).into(),
            ),
            (food, fun, TransactionError::SameTag(Tag::Expense).into()),
            (food, food, TransactionError::SameTag(Tag::Expense).into()),
        ]
    }

    /// すべてのタグの品目を1つずつ表示順に登録した帳簿.
    fn all_tags() -> (Ledger, [ItemId; 5]) {
        let mut ledger = Ledger::new();
        let ids = Tag::ALL.map(|tag| ledger.add_item(&tag.to_string(), tag).unwrap());
        (ledger, ids)
    }

    #[test]
    fn from_parts_error_test() {
        let (ledger, ids) = ledger();
        for (item, counterpart, expected) in invalid_pairs(ids) {
            let mut transactions = TransactionList::default();
            transactions.add(draft(item, counterpart));
            assert_eq!(
                Ledger::from_parts(ledger.items.clone(), transactions, BudgetList::new()),
                Err(expected),
                "{item:?} {counterpart:?}"
            );
        }
    }

    #[test]
    fn item_choices_test() {
        let (ledger, ids) = all_tags();

        let choices = ledger.item_choices().map(Item::id).collect::<Vec<_>>();
        assert_eq!(choices, vec![ids[1], ids[3], ids[4]]);
    }

    #[test]
    fn counterpart_choices_test() {
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
            let choices = ledger.counterpart_choices(item).map(Item::id);
            assert_eq!(choices.collect::<Vec<_>>(), expected, "{item:?}");
        }
    }

    #[test]
    fn choices_reflect_item_changes_test() {
        let (mut ledger, ids) = all_tags();

        ledger.update_item(ids[4], "変更後", Tag::Asset).unwrap();
        ledger.remove_item(ids[1]).unwrap();

        let choices = ledger.item_choices().map(Item::id).collect::<Vec<_>>();
        assert_eq!(choices, vec![ids[3]]);
        let counterparts = ledger.counterpart_choices(ids[3]).map(Item::id);
        assert_eq!(
            counterparts.collect::<Vec<_>>(),
            vec![ids[0], ids[2], ids[4]]
        );
    }

    #[test]
    fn update_item_test() {
        let (mut ledger, [salary, cash, food, fun, _]) = ledger();
        ledger.add_transaction(draft(salary, cash)).unwrap();

        // 取引の組み合わせを壊さないタグへの変更
        ledger.update_item(cash, "カード", Tag::Liability).unwrap();
        // 取引に使われていない品目のタグの変更
        ledger.update_item(fun, "娯楽", Tag::Asset).unwrap();

        let tags = [cash, fun].map(|id| ledger.items.get(id).unwrap().tag());
        assert_eq!(tags, [Tag::Liability, Tag::Asset]);
        assert_eq!(ledger.items.get(cash).unwrap().to_string(), "カード");
        assert!(ledger.add_transaction(draft(food, cash)).is_ok());
    }

    #[test]
    fn update_item_error_test() {
        let (mut ledger, [salary, cash, _, _, removed]) = ledger();
        ledger.add_transaction(draft(salary, cash)).unwrap();
        let before = ledger.clone();

        for (id, tag, expected) in [
            (
                salary,
                Tag::Asset,
                TransactionError::InvalidItemTag(Tag::Asset).into(),
            ),
            (
                cash,
                Tag::Income,
                TransactionError::SameTag(Tag::Income).into(),
            ),
            (removed, Tag::Income, ItemError::NotFound(removed).into()),
        ] {
            assert_eq!(
                ledger.update_item(id, "変更後", tag),
                Err(expected),
                "{id:?}"
            );
            assert_eq!(ledger, before, "{id:?}");
        }
    }

    #[test]
    fn remove_item_error_test() {
        let (mut ledger, [salary, cash, _, _, removed]) = ledger();
        let transaction = ledger.add_transaction(draft(salary, cash)).unwrap();
        let before = ledger.clone();

        for (id, expected) in [
            // 品目と相手先のどちらとして使われていても削除できない
            (salary, LedgerError::ItemUse(salary)),
            (cash, LedgerError::ItemUse(cash)),
            (removed, ItemError::NotFound(removed).into()),
        ] {
            assert_eq!(ledger.remove_item(id), Err(expected), "{id:?}");
            assert_eq!(ledger, before, "{id:?}");
        }

        // 取引から使われなくなれば削除できる
        ledger.remove_transaction(transaction).unwrap();
        assert_eq!(ledger.remove_item(salary), Ok(()));
        assert_eq!(ledger.remove_item(cash), Ok(()));
    }

    #[test]
    fn add_transaction_test() {
        let (mut ledger, [salary, cash, food, _, _]) = ledger();

        for (item, counterpart) in [(salary, cash), (food, cash), (food, salary)] {
            let id = ledger.add_transaction(draft(item, counterpart)).unwrap();

            let transaction = ledger.transactions().get(id).unwrap();
            assert_eq!(
                (transaction.item(), transaction.counterpart()),
                (item, counterpart)
            );
        }
        assert_eq!(ledger.transactions().iter().count(), 3);
    }

    #[test]
    fn add_transaction_error_test() {
        let (mut ledger, ids) = ledger();
        let before = ledger.clone();

        for (item, counterpart, expected) in invalid_pairs(ids) {
            assert_eq!(
                ledger.add_transaction(draft(item, counterpart)),
                Err(expected),
                "{item:?} {counterpart:?}"
            );
            assert_eq!(ledger, before, "{item:?} {counterpart:?}");
        }
    }

    #[test]
    fn update_transaction_test() {
        let (mut ledger, [salary, cash, food, _, _]) = ledger();
        let id = ledger.add_transaction(draft(salary, cash)).unwrap();

        ledger.update_transaction(id, draft(food, cash)).unwrap();

        let transaction = ledger.transactions().get(id).unwrap();
        assert_eq!(
            (transaction.item(), transaction.counterpart()),
            (food, cash)
        );
        assert_eq!(ledger.transactions().iter().count(), 1);
    }

    #[test]
    fn update_transaction_error_test() {
        let (mut ledger, ids) = ledger();
        let [salary, cash, ..] = ids;
        let id = ledger.add_transaction(draft(salary, cash)).unwrap();
        let missing = ledger.add_transaction(draft(salary, cash)).unwrap();
        ledger.remove_transaction(missing).unwrap();
        let before = ledger.clone();

        for (item, counterpart, expected) in invalid_pairs(ids) {
            assert_eq!(
                ledger.update_transaction(id, draft(item, counterpart)),
                Err(expected),
                "{item:?} {counterpart:?}"
            );
            assert_eq!(ledger, before, "{item:?} {counterpart:?}");
        }
        assert_eq!(
            ledger.update_transaction(missing, draft(salary, cash)),
            Err(TransactionError::NotFound(missing).into())
        );
        assert_eq!(ledger, before);
    }

    #[test]
    fn remove_transaction_error_test() {
        let (mut ledger, [salary, cash, ..]) = ledger();
        let id = ledger.add_transaction(draft(salary, cash)).unwrap();
        ledger.remove_transaction(id).unwrap();
        let before = ledger.clone();

        assert_eq!(
            ledger.remove_transaction(id),
            Err(TransactionError::NotFound(id).into())
        );
        assert_eq!(ledger, before);
    }

    /// budget関連のテスト
    ///
    /// LedgerError::Budgetを確認するテスト
    #[test]
    fn budget_error_is_converted_to_ledger_error() {
        let mut ledger = Ledger::new();
        let missing_id = BudgetId(5);

        let result = ledger.remove_budget(missing_id);

        assert_eq!(
            result,
            Err(LedgerError::Budget(BudgetError::NotFound(missing_id)))
        );
    }

    /// ミューテーションテスト
    /// 更新後の金額と期間を確認する
    #[test]
    fn update_budget_delegates_to_budget_list() {
        let mut ledger = Ledger::new();

        let old_period = Period::month(YearMonth::new(2026, 10).unwrap());

        let new_period = Period::month(YearMonth::new(2026, 11).unwrap());

        let budget_id = ledger.add_budget(Money::from(5000), old_period).unwrap();

        ledger
            .update_budget(budget_id, Money::from(7000), new_period)
            .unwrap();

        let updated = ledger.budgets().get(budget_id).unwrap();

        assert_eq!(updated.amount(), Money::from(7000));
        assert_eq!(updated.period(), new_period);
    }
}

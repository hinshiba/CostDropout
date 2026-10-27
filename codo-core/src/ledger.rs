//! ledgerの整合性

use crate::budget::{BudgetError, BudgetId, BudgetList};
use crate::item::{ItemError, ItemId, ItemList, Tag};
use crate::money::Money;
use crate::period::Period;
use crate::preset::preset_items;
use crate::transaction::{TransactionDraft, TransactionError, TransactionId, TransactionList};
use thiserror::Error;

/// 登録された品目と取引を保持し、
/// 品目や取引に対する操作を提供する。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Ledger {
    items: ItemList,
    transactions: TransactionList,
    budgets: BudgetList,
}

/// Ledgerの操作中に発生するエラー
#[derive(Debug, Error, PartialEq, Eq)]
pub enum LedgerError {
    #[error(transparent)]
    Item(#[from] ItemError),

    #[error(transparent)]
    Transaction(#[from] TransactionError),

    #[error(transparent)]
    Budget(#[from] BudgetError),

    #[error("取引から参照されている品目 {0:?} は削除できません")]
    ItemUse(ItemId),

    #[error("取引の品目のタグ {0:?} が収入・支出・負債ではありません")]
    InvalidItemTag(Tag),

    #[error("相手先のタグ {0:?} が品目のタグと同じです")]
    SameTag(Tag),

    #[error("存在しない取引の品目 {0:?} が指定されています")]
    TransactionItemNotFound(ItemId),
}

impl Ledger {
    pub fn new() -> Self {
        Self {
            items: ItemList::new(),
            transactions: TransactionList::new(),
            budgets: BudgetList::new(),
        }
    }

    /// ItemListにプリセット品目を入れる関数
    pub fn with_presets() -> Self {
        Self {
            items: ItemList::from_items(preset_items()),
            transactions: TransactionList::new(),
            budgets: BudgetList::new(),
        }
    }

    /// ファイル復元のため
    /// エラー検証は行わない
    pub fn from_parts(items: ItemList, transactions: TransactionList, budgets: BudgetList) -> Self {
        Self {
            items,
            transactions,
            budgets,
        }
    }

    pub fn items(&self) -> &ItemList {
        &self.items
    }

    pub fn transactions(&self) -> &TransactionList {
        &self.transactions
    }

    pub fn budgets(&self) -> &BudgetList {
        &self.budgets
    }

    /// 品目のadd処理を委譲
    pub fn add_item(&mut self, name: &str, tag: Tag) -> Result<ItemId, LedgerError> {
        Ok(self.items.add(name, tag)?)
    }

    /// 品目のupdate処理を委譲
    /// タグを変えると，この品目を参照している取引が制約を満たさなくなる場合はエラー
    pub fn update_item(&mut self, id: ItemId, name: &str, tag: Tag) -> Result<(), LedgerError> {
        // 更新対象の品目を確認
        let Some(item) = self.items.get_item(id) else {
            return Err(LedgerError::Item(ItemError::NotFound(id)));
        };

        if item.tag() != tag {
            for t in self.transactions.iter() {
                // この品目が「品目」として使われている場合
                if t.item() == id {
                    if !tag.is_transaction_selectable() {
                        return Err(LedgerError::InvalidItemTag(tag));
                    }
                    let Some(counterpart) = self.items.get_item(t.counterpart()) else {
                        return Err(LedgerError::TransactionItemNotFound(t.counterpart()));
                    };
                    if counterpart.tag() == tag {
                        return Err(LedgerError::SameTag(tag));
                    }
                }

                // この品目が「相手先」として使われている場合
                // 取引の品目と相手先のタグが同じにならないか確認する
                if t.counterpart() == id {
                    let Some(transaction_item) = self.items.get_item(t.item()) else {
                        return Err(LedgerError::TransactionItemNotFound(t.item()));
                    };

                    if transaction_item.tag() == tag {
                        return Err(LedgerError::SameTag(tag));
                    }
                }
            }
        }
        // ItemList に更新を委譲する
        self.items.update(id, name, tag)?;
        Ok(())
    }

    /// 品目のremove処理を委譲
    /// 取引から品目または相手先として参照されていればエラー
    pub fn remove_item(&mut self, id: ItemId) -> Result<(), LedgerError> {
        if self.transactions.uses_item(id) {
            return Err(LedgerError::ItemUse(id));
        }

        // ItemList に削除を委譲する
        self.items.remove(id)?;
        Ok(())
    }

    fn validate_transaction_draft(&self, draft: &TransactionDraft) -> Result<(), LedgerError> {
        draft.validate()?;

        let Some(item) = self.items.get_item(draft.item()) else {
            return Err(LedgerError::TransactionItemNotFound(draft.item()));
        };

        let Some(counterpart) = self.items.get_item(draft.counterpart()) else {
            return Err(LedgerError::TransactionItemNotFound(draft.counterpart()));
        };

        if !item.tag().is_transaction_selectable() {
            return Err(LedgerError::InvalidItemTag(item.tag()));
        }

        if item.tag() == counterpart.tag() {
            return Err(LedgerError::SameTag(item.tag()));
        }
        Ok(())
    }

    /// 取引のadd処理を委譲
    /// 品目と相手先が存在しなければエラー
    /// 品目のタグが Tag::is_transaction_selectable を満たさなければエラー
    /// 相手先のタグが品目のタグと異なればエラー
    pub fn add_transaction(
        &mut self,
        draft: TransactionDraft,
    ) -> Result<TransactionId, LedgerError> {
        self.validate_transaction_draft(&draft)?;

        Ok(self.transactions.add(draft)?)
    }

    /// 取引のupdate処理を委譲
    /// 品目と相手先が存在しなければエラー
    /// 品目のタグが Tag::is_transaction_selectable を満たさなければエラー
    /// 相手先のタグが品目のタグと異なればエラー
    pub fn update_transaction(
        &mut self,
        id: TransactionId,
        draft: TransactionDraft,
    ) -> Result<(), LedgerError> {
        self.validate_transaction_draft(&draft)?;
        self.transactions.update(id, draft)?;
        Ok(())
    }

    /// 取引のremove処理を委譲
    /// 指定した取引が存在しない場合は、そのエラーをそのまま返す。
    pub fn remove_transaction(&mut self, id: TransactionId) -> Result<(), LedgerError> {
        self.transactions.remove(id)?;
        Ok(())
    }

    /// 予算のadd処理を委譲
    pub fn add_budget(&mut self, amount: Money, period: Period) -> Result<BudgetId, LedgerError> {
        Ok(self.budgets.add(amount, period)?)
    }

    /// 予算のupdate処理を委譲
    pub fn update_budget(
        &mut self,
        id: BudgetId,
        amount: Money,
        period: Period,
    ) -> Result<(), LedgerError> {
        self.budgets.update(id, amount, period)?;
        Ok(())
    }

    /// 予算のremove処理を委譲
    pub fn remove_budget(&mut self, id: BudgetId) -> Result<(), LedgerError> {
        self.budgets.remove(id)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::money::Money;

    fn draft(item: ItemId, counterpart: ItemId) -> TransactionDraft {
        TransactionDraft::new(
            chrono::NaiveDate::from_ymd_opt(2026, 10, 12).unwrap(),
            Money(1000),
            item,
            counterpart,
            String::new(),
        )
    }

    fn ledger_with_items() -> (Ledger, ItemId, ItemId) {
        let mut ledger = Ledger::new();

        let income_id: ItemId = ledger.add_item("収入", Tag::Income).unwrap();
        let asset_id = ledger.add_item("現金", Tag::Asset).unwrap();

        (ledger, income_id, asset_id)
    }

    fn period(year: i32, month: u32, start_day: u32, end_day: u32) -> Period {
        Period::new(
            chrono::NaiveDate::from_ymd_opt(year, month, start_day).unwrap(),
            chrono::NaiveDate::from_ymd_opt(year, month, end_day).unwrap(),
        )
        .unwrap()
    }

    fn ledger_with_budget() -> (Ledger, BudgetId) {
        let mut ledger = Ledger::new();

        let period = period(2025, 4, 1, 30);
        let budget_id = ledger.add_budget(Money(5000), period).unwrap();

        (ledger, budget_id)
    }

    // item関連のテスト

    /// 取引にある場合は削除できない，
    /// 取引がない場合は削除が成功する
    #[test]
    fn remove_item_fails_when_item_is_used_by_transaction() {
        let (mut ledger, income_id, asset_id) = ledger_with_items();

        let transaction_id = ledger.add_transaction(draft(income_id, asset_id)).unwrap();
        let before = ledger.transactions().get(transaction_id).unwrap().clone();

        assert_eq!(
            ledger.remove_item(income_id),
            Err(LedgerError::ItemUse(income_id))
        );

        assert_eq!(
            ledger.remove_item(asset_id),
            Err(LedgerError::ItemUse(asset_id))
        );

        // 削除に失敗したため、元の取引は変わっていないか検証
        assert_eq!(ledger.transactions().get(transaction_id).unwrap(), &before);

        ledger.remove_transaction(transaction_id).unwrap();

        // Transactionから参照されなくなったので、Itemを削除できる
        assert!(ledger.remove_item(income_id).is_ok());
        assert!(ledger.remove_item(asset_id).is_ok());
    }

    #[test]
    fn update_item_fails_when_item_tag_becomes_invalid() {
        let (mut ledger, income_id, asset_id) = ledger_with_items();

        let transaction_id = ledger.add_transaction(draft(income_id, asset_id)).unwrap();
        let before = ledger.transactions().get(transaction_id).unwrap().clone();

        assert_eq!(
            ledger.update_item(income_id, "収入", Tag::Asset),
            Err(LedgerError::InvalidItemTag(Tag::Asset))
        );

        // 更新に失敗したため、元の取引は変わっていない
        assert_eq!(ledger.transactions().get(transaction_id).unwrap(), &before);
    }

    #[test]
    fn update_item_fails_when_counterpart_tag_becomes_same() {
        let (mut ledger, income_id, asset_id) = ledger_with_items();

        ledger.add_transaction(draft(income_id, asset_id)).unwrap();

        assert_eq!(
            ledger.update_item(asset_id, "現金", Tag::Income),
            Err(LedgerError::SameTag(Tag::Income))
        );

        // 更新に失敗したときは，現金のタグはAssetのまま
        assert_eq!(ledger.items.get_item(asset_id).unwrap().tag(), Tag::Asset);
    }

    // transaction関連のテスト
    #[test]
    fn add_transaction_fails_when_item_or_counterpart_does_not_exist() {
        let mut ledger = Ledger::new();

        let income_id = ledger.add_item("収入", Tag::Income).unwrap();
        let missing_id = ledger.add_item("削除する品目", Tag::Asset).unwrap();

        ledger.remove_item(missing_id).unwrap();

        assert_eq!(
            ledger.add_transaction(draft(missing_id, income_id)),
            Err(LedgerError::TransactionItemNotFound(missing_id))
        );

        assert_eq!(
            ledger.add_transaction(draft(income_id, missing_id)),
            Err(LedgerError::TransactionItemNotFound(missing_id))
        );
    }

    #[test]
    fn add_transaction_fails_when_item_tag_is_not_selectable() {
        let mut ledger = Ledger::new();

        let asset_id = ledger.add_item("現金", Tag::Asset).unwrap();
        let income_id = ledger.add_item("収入", Tag::Income).unwrap();

        assert_eq!(
            ledger.add_transaction(draft(asset_id, income_id)),
            Err(LedgerError::InvalidItemTag(Tag::Asset))
        );
    }

    #[test]
    fn add_transaction_fails_when_item_and_counterpart_have_same_tag() {
        let mut ledger = Ledger::new();

        let income1 = ledger.add_item("収入1", Tag::Income).unwrap();
        let income2 = ledger.add_item("収入2", Tag::Income).unwrap();

        assert_eq!(
            ledger.add_transaction(draft(income1, income2)),
            Err(LedgerError::SameTag(Tag::Income))
        );
    }

    // update_transaction 関連のテスト

    #[test]
    fn update_transaction_fails_when_item_or_counterpart_does_not_exist() {
        let (mut ledger, income_id, asset_id) = ledger_with_items();

        let transaction_id = ledger.add_transaction(draft(income_id, asset_id)).unwrap();
        let before = ledger.transactions().get(transaction_id).unwrap().clone();

        // 品目を登録してから削除し、存在しないIDを用意する
        let missing_id = ledger.add_item("削除する品目", Tag::Asset).unwrap();
        ledger.remove_item(missing_id).unwrap();

        // 取引の品目が存在しない場合
        assert_eq!(
            ledger.update_transaction(transaction_id, draft(missing_id, asset_id)),
            Err(LedgerError::TransactionItemNotFound(missing_id))
        );

        // 取引の相手先が存在しない場合
        assert_eq!(
            ledger.update_transaction(transaction_id, draft(income_id, missing_id),),
            Err(LedgerError::TransactionItemNotFound(missing_id))
        );

        // 更新に失敗したため、元の取引は変わっていない
        assert_eq!(ledger.transactions().get(transaction_id).unwrap(), &before);
    }

    #[test]
    fn update_transaction_fails_when_item_tag_is_not_selectable() {
        let (mut ledger, income_id, asset_id) = ledger_with_items();

        let transaction_id = ledger.add_transaction(draft(income_id, asset_id)).unwrap();
        let before = ledger.transactions().get(transaction_id).unwrap().clone();

        assert_eq!(
            ledger.update_transaction(transaction_id, draft(asset_id, income_id)),
            Err(LedgerError::InvalidItemTag(Tag::Asset))
        );

        // 更新に失敗したため、元の取引は変わっていない
        assert_eq!(ledger.transactions().get(transaction_id).unwrap(), &before);
    }

    #[test]
    fn update_transaction_fails_when_item_and_counterpart_have_same_tag() {
        let (mut ledger, income_id, asset_id) = ledger_with_items();

        let transaction_id = ledger.add_transaction(draft(income_id, asset_id)).unwrap();
        let before = ledger.transactions().get(transaction_id).unwrap().clone();

        let income2 = ledger.add_item("収入2", Tag::Income).unwrap();

        assert_eq!(
            ledger.update_transaction(transaction_id, draft(income_id, income2)),
            Err(LedgerError::SameTag(Tag::Income))
        );

        // 更新に失敗したため、元の取引は変わっていない
        assert_eq!(ledger.transactions().get(transaction_id).unwrap(), &before);
    }

    #[test]
    fn update_transaction_fails_when_transaction_does_not_exist() {
        let (mut ledger, income_id, asset_id) = ledger_with_items();

        // 取引を登録した後に削除し、存在しない TransactionId を用意する
        let missing_id = ledger.add_transaction(draft(income_id, asset_id)).unwrap();

        ledger.remove_transaction(missing_id).unwrap();

        assert_eq!(
            ledger.update_transaction(missing_id, draft(income_id, asset_id)),
            Err(LedgerError::Transaction(TransactionError::NotFound(
                missing_id
            )))
        );
    }

    /// update_transactionが成功した場合
    #[test]
    fn update_transaction_succeeds_when_new_item_and_counterpart_tags_are_valid() {
        let (mut ledger, income_id, asset_id) = ledger_with_items();

        let transaction_id = ledger.add_transaction(draft(income_id, asset_id)).unwrap();
        let expense_id = ledger.add_item("支出", Tag::Expense).unwrap();

        assert!(
            ledger
                .update_transaction(transaction_id, draft(expense_id, asset_id))
                .is_ok()
        );
        let after = ledger.transactions().get(transaction_id).unwrap();

        assert_eq!(after.item(), expense_id);
        assert_eq!(after.counterpart(), asset_id);
    }

    #[test]
    fn remove_transaction_fails_when_transaction_does_not_exist() {
        let (mut ledger, income_id, asset_id) = ledger_with_items();

        let transaction_id = ledger.add_transaction(draft(income_id, asset_id)).unwrap();

        // 一度削除する
        assert!(ledger.remove_transaction(transaction_id).is_ok());

        // すでに削除されているので、もう一度削除するとエラーになる
        assert_eq!(
            ledger.remove_transaction(transaction_id),
            Err(LedgerError::Transaction(TransactionError::NotFound(
                transaction_id
            )))
        );
    }

    #[test]
    fn add_item_after_with_presets_gets_next_id() {
        let mut ledger = Ledger::with_presets();

        let new_id = ledger.add_item("新しい品目", Tag::Expense).unwrap();

        assert_eq!(new_id.id(), 13);
        assert_eq!(
            ledger.items().get_item(new_id).unwrap().name(),
            "新しい品目"
        );
    }

    // budget関連のテスト

    /// 予算の追加が成功する場合
    #[test]
    fn add_budget_succeeds_when_period_is_unique() {
        let mut ledger = Ledger::new();
        let target_period = period(2025, 4, 1, 30);

        let budget_id = ledger.add_budget(Money(5000), target_period).unwrap();

        assert_eq!(budget_id, BudgetId(1));

        let budget = ledger.budgets().get(budget_id).unwrap();
        assert_eq!(budget.amount(), Money(5000));
        assert_eq!(budget.period(), target_period);
    }

    /// 同じ期間の予算を追加できない場合
    #[test]
    fn add_budget_fails_when_period_is_duplicate() {
        let (mut ledger, budget_id) = ledger_with_budget();
        let target_period = period(2025, 4, 1, 30);

        let before = ledger.budgets().get(budget_id).unwrap().clone();

        assert_eq!(
            ledger.add_budget(Money(7000), target_period),
            Err(LedgerError::Budget(BudgetError::DuplicatePeriod))
        );

        // 追加に失敗しても元の予算は変更されない
        assert_eq!(ledger.budgets().get(budget_id).unwrap(), &before);
    }

    /// 予算の更新が成功する場合
    #[test]
    fn update_budget_succeeds_when_id_exists() {
        let (mut ledger, budget_id) = ledger_with_budget();
        let new_period = period(2025, 5, 1, 31);

        assert_eq!(
            ledger.update_budget(budget_id, Money(7000), new_period),
            Ok(())
        );

        let budget = ledger.budgets().get(budget_id).unwrap();

        assert_eq!(budget.amount(), Money(7000));
        assert_eq!(budget.period(), new_period);
    }

    /// 存在しない予算を更新できない場合
    #[test]
    fn update_budget_fails_when_id_does_not_exist() {
        let (mut ledger, budget_id) = ledger_with_budget();
        let missing_id = BudgetId(5);
        let target_period = period(2025, 5, 1, 31);

        let before = ledger.budgets().get(budget_id).unwrap().clone();

        assert_eq!(
            ledger.update_budget(missing_id, Money(7000), target_period),
            Err(LedgerError::Budget(BudgetError::NotFound(missing_id)))
        );

        // 更新に失敗しても元の予算は変更されない
        assert_eq!(ledger.budgets().get(budget_id).unwrap(), &before);
    }

    /// 更新先の期間が既存の予算と重複する場合
    #[test]
    fn update_budget_fails_when_period_is_duplicate() {
        let (mut ledger, budget_id) = ledger_with_budget();

        let second_period = period(2025, 5, 1, 31);
        let second_id = ledger.add_budget(Money(7000), second_period).unwrap();

        let before = ledger.budgets().get(budget_id).unwrap().clone();

        assert_eq!(
            ledger.update_budget(budget_id, Money(9000), second_period),
            Err(LedgerError::Budget(BudgetError::DuplicatePeriod))
        );

        // 更新対象の予算が変更されていないことを確認
        assert_eq!(ledger.budgets().get(budget_id).unwrap(), &before);

        // 重複先の予算も変更されていないことを確認
        assert_eq!(
            ledger.budgets().get(second_id).unwrap().amount(),
            Money(7000)
        );
    }

    /// 予算の削除が成功する場合
    #[test]
    fn remove_budget_succeeds_when_id_exists() {
        let (mut ledger, budget_id) = ledger_with_budget();

        assert_eq!(ledger.remove_budget(budget_id), Ok(()));

        assert!(ledger.budgets().get(budget_id).is_none());
    }

    /// 存在しない予算を削除できない場合
    #[test]
    fn remove_budget_fails_when_id_does_not_exist() {
        let (mut ledger, budget_id) = ledger_with_budget();
        let missing_id = BudgetId(5);

        let before = ledger.budgets().get(budget_id).unwrap().clone();

        assert_eq!(
            ledger.remove_budget(missing_id),
            Err(LedgerError::Budget(BudgetError::NotFound(missing_id)))
        );

        // 削除に失敗しても元の予算は残っている
        assert_eq!(ledger.budgets().get(budget_id).unwrap(), &before);
    }
}

//! ledgerの整合性

use crate::budget::{BudgetError, BudgetId, BudgetList};
use crate::item::{Item, ItemError, ItemId, ItemList, Tag};
use crate::money::Money;
use crate::period::Period;
use crate::period::YearMonth;
use crate::preset::preset_items;
use crate::transaction::{
    Transaction, TransactionDraft, TransactionError, TransactionId, TransactionList,
};
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
            items: preset_items(),
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
    /// 指定月の取引を日付の降順、同日ならIDの降順で返す。
    ///
    /// 月の初日と末日の両方を含む。
    pub fn monthly_transactions(&self, month: YearMonth) -> impl Iterator<Item = &Transaction> {
        self.transactions
            .iter()
            .rev()
            .filter(move |transaction| month.contains(transaction.date()))
    }

    /// 品目のadd処理を委譲
    pub fn add_item(&mut self, name: &str, tag: Tag) -> Result<ItemId, LedgerError> {
        Ok(self.items.add(name, tag)?)
    }

    /// 品目のupdate処理を委譲
    /// タグを変えると，この品目を参照している取引が制約を満たさなくなる場合はエラー
    pub fn update_item(&mut self, id: ItemId, name: &str, tag: Tag) -> Result<(), LedgerError> {
        let Some(item) = self.items.get(id) else {
            return Err(LedgerError::Item(ItemError::NotFound(id)));
        };

        if item.tag() != tag {
            // 更新後のタグで，この品目を参照している取引を検証する
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

    /// 取引の品目と相手先として参照する品目のタグを返す
    fn tag_of(&self, id: ItemId) -> Result<Tag, LedgerError> {
        self.items
            .get(id)
            .map(Item::tag)
            .ok_or(LedgerError::TransactionItemNotFound(id))
    }

    /// 取引のadd処理を委譲
    /// 品目と相手先が存在しなければエラー
    /// 品目のタグが Tag::is_transaction_selectable を満たさなければエラー
    /// 相手先のタグが品目のタグと異なればエラー
    pub fn add_transaction(
        &mut self,
        draft: TransactionDraft,
    ) -> Result<TransactionId, LedgerError> {
        draft.check_tags(|id| self.tag_of(id))?;

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
        draft.check_tags(|id| self.tag_of(id))?;
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
    use crate::transaction::Amount;

    fn draft(item: ItemId, counterpart: ItemId) -> TransactionDraft {
        TransactionDraft::new(
            chrono::NaiveDate::from_ymd_opt(2026, 10, 12).unwrap(),
            Amount::of(1000),
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
            Err(LedgerError::Transaction(TransactionError::InvalidItemTag(
                Tag::Asset
            )))
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
            Err(LedgerError::Transaction(TransactionError::SameTag(
                Tag::Income
            )))
        );

        // 更新に失敗したときは，現金のタグはAssetのまま
        assert_eq!(ledger.items.get(asset_id).unwrap().tag(), Tag::Asset);
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
            Err(LedgerError::Transaction(TransactionError::InvalidItemTag(
                Tag::Asset
            )))
        );
    }

    #[test]
    fn add_transaction_fails_when_item_and_counterpart_have_same_tag() {
        let mut ledger = Ledger::new();

        let income1 = ledger.add_item("収入1", Tag::Income).unwrap();
        let income2 = ledger.add_item("収入2", Tag::Income).unwrap();

        assert_eq!(
            ledger.add_transaction(draft(income1, income2)),
            Err(LedgerError::Transaction(TransactionError::SameTag(
                Tag::Income
            )))
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
            Err(LedgerError::Transaction(TransactionError::InvalidItemTag(
                Tag::Asset
            )))
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
            Err(LedgerError::Transaction(TransactionError::SameTag(
                Tag::Income
            )))
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

        let old_period = Period::month(crate::period::YearMonth::new(2026, 10).unwrap());

        let new_period = Period::month(crate::period::YearMonth::new(2026, 11).unwrap());

        let budget_id = ledger.add_budget(Money::from(5000), old_period).unwrap();

        ledger
            .update_budget(budget_id, Money::from(7000), new_period)
            .unwrap();

        let updated = ledger.budgets().get(budget_id).unwrap();

        assert_eq!(updated.amount(), Money::from(7000));
        assert_eq!(updated.period(), new_period);
    }
}

#[cfg(test)]
mod monthly_transaction_tests {
    use crate::item::Tag;
    use crate::ledger::Ledger;
    use crate::period::YearMonth;
    use crate::transaction::{Amount, TransactionDraft};
    use chrono::NaiveDate;

    #[test]
    fn monthly_transactions_filter_boundaries_sort_and_reflect_removal() {
        let mut ledger = Ledger::new();
        let item = ledger.add_item("食費", Tag::Expense).unwrap();
        let counterpart = ledger.add_item("現金", Tag::Asset).unwrap();
        let dates = [
            (2026, 12, 31),
            (2027, 1, 1),
            (2026, 11, 30),
            (2026, 12, 1),
            (2026, 12, 15),
            (2026, 12, 15),
        ];
        let mut ids = Vec::new();
        for (year, month, day) in dates {
            ids.push(
                ledger
                    .add_transaction(TransactionDraft::new(
                        NaiveDate::from_ymd_opt(year, month, day).unwrap(),
                        Amount::of(1234),
                        item,
                        counterpart,
                        String::new(),
                    ))
                    .unwrap(),
            );
        }
        let month = YearMonth::new(2026, 12).unwrap();
        let mut same_day = [ids[4], ids[5]];
        same_day.sort_by(|left, right| right.cmp(left));
        assert_eq!(
            ledger
                .monthly_transactions(month)
                .map(|transaction| transaction.id())
                .collect::<Vec<_>>(),
            vec![ids[0], same_day[0], same_day[1], ids[3]]
        );
        assert_eq!(
            ledger
                .monthly_transactions(month.prev())
                .map(|transaction| transaction.id())
                .collect::<Vec<_>>(),
            vec![ids[2]]
        );
        assert_eq!(
            ledger
                .monthly_transactions(month.next())
                .map(|transaction| transaction.id())
                .collect::<Vec<_>>(),
            vec![ids[1]]
        );
        ledger.remove_transaction(ids[0]).unwrap();
        assert_eq!(
            ledger
                .monthly_transactions(month)
                .map(|transaction| transaction.id())
                .collect::<Vec<_>>(),
            vec![same_day[0], same_day[1], ids[3]]
        );
        assert!(
            ledger
                .monthly_transactions(month.next().next())
                .next()
                .is_none()
        );
    }

    #[test]
    fn monthly_transactions_include_leap_day_only_in_february() {
        let mut ledger = Ledger::new();
        let item = ledger.add_item("給与", Tag::Income).unwrap();
        let counterpart = ledger.add_item("銀行", Tag::Asset).unwrap();
        for (month, day) in [(2, 1), (2, 29), (3, 1)] {
            ledger
                .add_transaction(TransactionDraft::new(
                    NaiveDate::from_ymd_opt(2024, month, day).unwrap(),
                    Amount::of(1),
                    item,
                    counterpart,
                    String::new(),
                ))
                .unwrap();
        }
        let transactions = ledger.monthly_transactions(YearMonth::new(2024, 2).unwrap());
        assert_eq!(
            transactions
                .map(|transaction| transaction.date())
                .collect::<Vec<_>>(),
            vec![
                NaiveDate::from_ymd_opt(2024, 2, 29).unwrap(),
                NaiveDate::from_ymd_opt(2024, 2, 1).unwrap()
            ]
        );
    }
}

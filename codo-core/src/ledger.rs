use crate::item::{ItemError, ItemId, ItemList, Tag};
use crate::preset::preset_items;
use crate::transaction::{TransactionDraft, TransactionError, TransactionId, TransactionList};
use thiserror::Error;

#[derive(Debug)]
pub struct Ledger {
    items: ItemList,
    transactions: TransactionList,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum LedgerError {
    #[error(transparent)]
    Item(#[from] ItemError),

    #[error(transparent)]
    Transaction(#[from] TransactionError),

    #[error("取引から参照されている品目は削除できません")]
    ItemUse(ItemId),

    #[error("取引の品目のタグが収入・支出・負債ではありません")]
    InvalidItemTag(Tag),

    #[error("相手先のタグが品目のタグと同じです")]
    SameTag(Tag),

    #[error("存在しない取引の品目が指定されています")]
    TransactionItemNotFound(ItemId),
}

impl Default for Ledger {
    fn default() -> Self {
        Self::new()
    }
}

impl Ledger {
    pub fn new() -> Self {
        Self {
            items: ItemList::new(),
            transactions: TransactionList::new(),
        }
    }

    /// ItemListにプリセット品目を入れる関数
    pub fn with_presets() -> Self {
        Self {
            items: ItemList::from_items(preset_items()),
            transactions: TransactionList::new(),
        }
    }

    /// ファイル復元のため
    /// エラー検証は行わない
    pub fn from_parts(items: ItemList, transactions: TransactionList) -> Self {
        Self {
            items,
            transactions,
        }
    }

    pub fn items(&self) -> &ItemList {
        &self.items
    }

    pub fn transactions(&self) -> &TransactionList {
        &self.transactions
    }

    /// 品目のadd処理を委譲
    pub fn add_item(&mut self, name: &str, tag: Tag) -> Result<ItemId, LedgerError> {
        Ok(self.items.add(name, tag)?)
    }

    /// 品目のupdate処理を委譲
    pub fn update_item(&mut self, id: ItemId, name: &str, tag: Tag) -> Result<(), LedgerError> {
        // 更新対象の品目を確認
        let Some(item) = self.items.get_item(id) else {
            return Err(LedgerError::Item(ItemError::NotFound(id)));
        };

        // タグが変わらないなら、取引への影響はない
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
        // ItemList に更新を移譲する
        self.items.update(id, name, tag)?;
        Ok(())
    }

    /// 品目のremove処理を委譲
    pub fn remove_item(&mut self, id: ItemId) -> Result<(), LedgerError> {
        if self.transactions.uses_item(id) {
            return Err(LedgerError::ItemUse(id));
        }

        // ItemList に削除を移譲する
        self.items.remove(id)?;
        Ok(())
    }

    fn validate_transaction_draft(&self, draft: &TransactionDraft) -> Result<(), LedgerError> {
        let Some(item) = self.items.get_item(draft.item()) else {
            return Err(LedgerError::Item(ItemError::NotFound(draft.item())));
        };

        let Some(counterpart) = self.items.get_item(draft.counterpart()) else {
            return Err(LedgerError::Item(ItemError::NotFound(draft.counterpart())));
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
    pub fn add_transaction(
        &mut self,
        draft: TransactionDraft,
    ) -> Result<TransactionId, LedgerError> {
        self.validate_transaction_draft(&draft)?;

        Ok(self.transactions.add(draft)?)
    }

    /// 取引のupdate処理を委譲
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
    pub fn remove_transaction(&mut self, id: TransactionId) -> Result<(), LedgerError> {
        self.transactions.remove(id)?;
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

    /// item関連のテスト
    #[test]
    fn remove_item_fails_when_item_is_used_by_transaction() {
        let (mut ledger, income_id, asset_id) = ledger_with_items();

        ledger.add_transaction(draft(income_id, asset_id)).unwrap();

        assert_eq!(
            ledger.remove_item(income_id),
            Err(LedgerError::ItemUse(income_id))
        );

        assert_eq!(
            ledger.remove_item(asset_id),
            Err(LedgerError::ItemUse(asset_id))
        );
    }

    #[test]
    fn update_item_fails_when_item_tag_becomes_invalid() {
        let (mut ledger, income_id, asset_id) = ledger_with_items();

        ledger.add_transaction(draft(income_id, asset_id)).unwrap();

        assert_eq!(
            ledger.update_item(income_id, "収入", Tag::Asset),
            Err(LedgerError::InvalidItemTag(Tag::Asset))
        );
    }

    #[test]
    fn update_item_fails_when_counterpart_tag_becomes_same() {
        let (mut ledger, income_id, asset_id) = ledger_with_items();

        ledger.add_transaction(draft(income_id, asset_id)).unwrap();

        assert_eq!(
            ledger.update_item(asset_id, "現金", Tag::Income),
            Err(LedgerError::SameTag(Tag::Income))
        );
    }

    /// transaction関連のテスト
    #[test]
    fn add_transaction_fails_when_item_or_counterpart_does_not_exist() {
        let mut ledger = Ledger::new();

        let income_id = ledger.add_item("収入", Tag::Income).unwrap();
        let missing_id = ledger.add_item("削除する品目", Tag::Asset).unwrap();

        ledger.remove_item(missing_id).unwrap();

        assert_eq!(
            ledger.add_transaction(draft(missing_id, income_id)),
            Err(LedgerError::Item(ItemError::NotFound(missing_id)))
        );

        assert_eq!(
            ledger.add_transaction(draft(income_id, missing_id)),
            Err(LedgerError::Item(ItemError::NotFound(missing_id)))
        );
    }

    #[test]
    fn validate_transaction_draft_fails_when_item_tag_is_not_selectable() {
        let mut ledger = Ledger::new();

        let asset_id = ledger.add_item("現金", Tag::Asset).unwrap();
        let income_id = ledger.add_item("収入", Tag::Income).unwrap();

        assert_eq!(
            ledger.add_transaction(draft(asset_id, income_id)),
            Err(LedgerError::InvalidItemTag(Tag::Asset))
        );
    }

    #[test]
    fn validate_transaction_draft_fails_when_item_and_counterpart_have_same_tag() {
        let mut ledger = Ledger::new();

        let income1 = ledger.add_item("収入1", Tag::Income).unwrap();
        let income2 = ledger.add_item("収入2", Tag::Income).unwrap();

        assert_eq!(
            ledger.add_transaction(draft(income1, income2)),
            Err(LedgerError::SameTag(Tag::Income))
        );
    }
}

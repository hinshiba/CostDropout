use crate::item::{ItemError, ItemId, ItemList, Tag};
use crate::preset::preset_items;
use crate::transaction::{TransactionError, TransactionList};
use thiserror::Error;

/// 品目 ID(連番)
// #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]

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

    #[error("取引の品目のタグが不正です")]
    InvalidItemTag(Tag),

    #[error("相手先のタグが品目のタグと同じです")]
    SameTag(Tag),

    #[error("存在しない取引の品目が指定されています")]
    TransactionItemNotFound(ItemId),
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

    pub fn add_item(&mut self, name: &str, tag: Tag) -> Result<ItemId, LedgerError> {
        Ok(self.items.add_item(name, tag)?)
    }

    pub fn update_item(&mut self, id: ItemId, name: &str, tag: Tag) -> Result<(), LedgerError> {
        // 更新対象の品目を確認
        let Some(item) = self.items.get_item(id) else {
            return Err(LedgerError::Item(ItemError::NotFound(id)));
        };

        // タグが変わらないなら、取引への影響はない
        if item.tag() != tag {
            for t in self.transactions.iter() {
                // この品目が「品目」として使われている場合
                if t.item() == id && !tag.is_transaction_selectable() {
                    return Err(LedgerError::InvalidItemTag(tag));
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
        self.items.update_item(id, name, tag)?;
        Ok(())
    }

    pub fn remove_item(&mut self, id: ItemId) -> Result<(), LedgerError> {
        if self.transactions.uses_item(id) {
            return Err(LedgerError::ItemUse(id));
        }

        // ItemList に削除を移譲する
        self.items.remove_item(id)?;
        Ok(())
    }
}

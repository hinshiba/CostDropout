use crate::item::ItemList;
use crate::item::{ItemError, ItemId, Tag};
use crate::preset::preset_items;
use crate::transaction::TransactionError;
use crate::transaction::TransactionList;
use thiserror::Error;

/// 品目 ID(連番)
// #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]

pub struct Ledger {
    items: ItemList,
    transactions: TransactionList,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum LegerError {
    #[error(transparent)]
    Item(#[from] ItemError),

    #[error(transparent)]
    Transaction(#[from] TransactionError),

    #[error("取引から参照されている品目は削除できません")]
    ItemUse(ItemId),

    #[error("取引の品目のタグが不正です")]
    InValidItemTag(Tag),

    #[error("相手先のタグが品目のタグと同じです")]
    SameTag(Tag),

    #[error("存在しない品目が指定されています")]
    ItemNotFound(ItemId),
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
}

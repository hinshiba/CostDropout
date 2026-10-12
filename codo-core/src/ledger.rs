use crate::item::ItemError;
use crate::item::ItemList;
use crate::preset::preset_items;
use crate::transaction::TransactionError;
use crate::transaction::TransactionList;
use thiserror::Error;

/// 品目 ID(連番)
// #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
use crate::transaction::TransactionList;

pub struct Ledger {
    items: ItemList,
    transactions: TransactionList,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum LegerError {
    #[error("品目エラー: {0}")]
    Item(#[from] ItemError),

    #[error("取引エラー: {0}")]
    Transaction(#[from] TransactionError),

    #[error("取引から参照されている品目は削除できません")]
    ItemUse,

    #[error("取引の品目のタグが不正です")]
    InValidItemTag,

    #[error("相手先のタグが品目のタグと同じです")]
    SameTag,

    #[error("存在しない品目が指定されています")]
    ItemNotFound,
}

impl Ledger {
    pub fn new() -> Self {
        Self {
            items: ItemList::new(),
            transactions: TransactionList::new(),
        }
    }

    pub fn with_presets() -> Self {
        Self {
            items: ItemList::from_items(preset_items()),
            transactions: TransactionList::new(),
        }
    }

    pub fn from_parts(items: ItemList, transactions: TransactionList) -> Self {
        Self {
            items,
            transactions,
        }
    }
}

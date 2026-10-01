//! 品目

use thiserror::Error;

/// 品目 ID(連番)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ItemId(pub u32);

/// 分類タグ
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tag {
    /// 資産
    Asset,
    /// 負債
    Liability,
    /// 初期残高(純資産)
    OpeningBalance,
    /// 収入(収益)
    Income,
    /// 支出(費用)
    Expense,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub id: ItemId,
    pub name: String,
    pub tag: Tag,
}

/// 品目の一覧
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ItemList {
    items: Vec<Item>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ItemError {
    #[error("品目名が空です")]
    EmptyName,
    #[error("品目名 {0} は既に使われています")]
    DuplicateName(String),
    #[error("品目 {0:?} が見つかりません")]
    NotFound(ItemId),
}

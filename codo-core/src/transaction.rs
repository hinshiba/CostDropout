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

/// 取引の一覧
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TransactionList {
    transactions: Vec<Transaction>,
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

//! 帳簿のファイル形式

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::ledger::budget::{Budget, BudgetId};
use crate::ledger::item::{Item, ItemId, Tag};
use crate::ledger::money::Money;
use crate::ledger::period::{Period, PeriodError};
use crate::ledger::transaction::{Transaction, TransactionDraft, TransactionId};

/// 1か月分の取引ファイル
#[derive(Serialize, Deserialize)]
pub(super) struct MonthFile {
    pub(super) transactions: Vec<TransactionRecord>,
}

/// 品目ファイル
#[derive(Serialize, Deserialize)]
pub(super) struct ItemsFile {
    pub(super) items: Vec<ItemRecord>,
}

/// 予算ファイル
#[derive(Serialize, Deserialize)]
pub(super) struct BudgetsFile {
    pub(super) budgets: Vec<BudgetRecord>,
}

/// TOMLに保存する品目のタグ
#[derive(Serialize, Deserialize)]
#[serde(remote = "Tag", rename_all = "snake_case")]
enum TagRecord {
    Asset,
    Liability,
    OpeningBalance,
    Income,
    Expense,
}

/// TOMLに保存する品目1件分のデータ
#[derive(Serialize, Deserialize)]
pub(super) struct ItemRecord {
    id: u32,
    name: String,
    #[serde(with = "TagRecord")]
    tag: Tag,
}

impl From<&Item> for ItemRecord {
    fn from(item: &Item) -> Self {
        Self {
            id: item.id().0,
            name: item.name().to_string(),
            tag: item.tag(),
        }
    }
}

impl From<ItemRecord> for Item {
    fn from(record: ItemRecord) -> Self {
        Item::new(ItemId(record.id), record.name, record.tag)
    }
}

/// TOMLに保存する取引1件分のデータ
#[derive(Serialize, Deserialize)]
pub(super) struct TransactionRecord {
    id: Uuid,
    #[serde(with = "toml_date")]
    date: NaiveDate,
    amount: i32,
    item: u32,
    counterpart: u32,
    comment: String,
}

impl From<&Transaction> for TransactionRecord {
    fn from(transaction: &Transaction) -> Self {
        Self {
            id: transaction.id().0,
            date: transaction.date(),
            amount: transaction.amount().to_i32(),
            item: transaction.item().0,
            counterpart: transaction.counterpart().0,
            comment: transaction.comment().to_string(),
        }
    }
}

impl From<TransactionRecord> for Transaction {
    fn from(record: TransactionRecord) -> Self {
        let draft = TransactionDraft::new(
            record.date,
            Money::from(record.amount),
            ItemId(record.item),
            ItemId(record.counterpart),
            record.comment,
        );
        Transaction::new(TransactionId(record.id), draft)
    }
}

/// TOMLに保存する予算1件分のデータ
#[derive(Serialize, Deserialize)]
pub(super) struct BudgetRecord {
    pub(super) id: u32,
    amount: i32,

    #[serde(with = "toml_date")]
    start: NaiveDate,

    #[serde(with = "toml_date")]
    end: NaiveDate,
}

/// BudgetをBudgetRecordに変換する
impl From<&Budget> for BudgetRecord {
    fn from(budget: &Budget) -> Self {
        Self {
            id: budget.id().0,
            amount: budget.amount().to_i32(),
            start: budget.period().start(),
            end: budget.period().end(),
        }
    }
}

/// BudgetRecordからBudgetを復元する
impl TryFrom<BudgetRecord> for Budget {
    type Error = PeriodError;

    fn try_from(record: BudgetRecord) -> Result<Self, Self::Error> {
        let period = Period::new(record.start, record.end)?;

        Ok(Budget::new(
            BudgetId(record.id),
            Money::from(record.amount),
            period,
        ))
    }
}

/// `NaiveDate` を TOML の日付型で読み書きする.
///
/// `#[serde(with = "toml_date")]` で使う.
mod toml_date {
    use chrono::NaiveDate;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use toml::value::Datetime;

    pub fn serialize<S: Serializer>(date: &NaiveDate, serializer: S) -> Result<S::Ok, S::Error> {
        let datetime = date
            .to_string()
            .parse::<Datetime>()
            .map_err(serde::ser::Error::custom)?;
        datetime.serialize(serializer)
    }

    /// # Errors
    ///
    /// TOML の日付型でない場合と, 時刻を含む場合はエラー.
    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<NaiveDate, D::Error> {
        Datetime::deserialize(deserializer)?
            .to_string()
            .parse::<NaiveDate>()
            .map_err(serde::de::Error::custom)
    }
}

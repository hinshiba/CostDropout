//!帳簿の保存・読み込み

use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::PathBuf;
use thiserror::Error;

use crate::item::ItemList;
use crate::ledger::Ledger;
use crate::period::YearMonth;
use crate::transaction::{Transaction, TransactionList};

///1か月分の取引ファイル
#[derive(Serialize, Deserialize)]
struct MonthFile {
    transactions: Vec<Transaction>,
}

///帳簿の保存先
pub trait LedgerStore {
    ///帳簿を読み込む
    fn load(&self) -> Result<Ledger, StorageError>;

    ///帳簿を保存する
    fn save(&self, ledger: &Ledger) -> Result<(), StorageError>;
}

///TOMLファイルに帳簿を保存する保存先
#[derive(Debug, Clone)]
pub struct TomlStore {
    dir: PathBuf,
}

impl TomlStore {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }
}

impl LedgerStore for TomlStore {
    ///ファイルの読み込みを行う
    /// ファイルがなければ空の帳簿を返す
    fn load(&self) -> Result<Ledger, StorageError> {
        let items_path = self.dir.join("items.toml");
        let items = if items_path.exists() {
            let items_toml = std::fs::read_to_string(items_path)?;
            toml::from_str(&items_toml)?
        } else {
            ItemList::new()
        };

        //取引の読み込み
        let transactions_dir = self.dir.join("transactions");
        let mut all = Vec::new();
        if transactions_dir.exists() {
            for entry in std::fs::read_dir(transactions_dir)? {
                let entry = entry?;
                let path = entry.path();
                let month_toml = std::fs::read_to_string(&path)?;
                let month_file: MonthFile = toml::from_str(&month_toml)?;
                all.extend(month_file.transactions);
            }
        }
        let transactions = TransactionList::from_transactions(all);

        Ok(Ledger::from_parts(items, transactions))
    }

    ///ファイルの保存を行う
    fn save(&self, ledger: &Ledger) -> Result<(), StorageError> {
        std::fs::create_dir_all(&self.dir)?;
        let items_path = self.dir.join("items.toml");
        let items_toml = toml::to_string(ledger.items())?;
        let mut by_month: BTreeMap<YearMonth, Vec<Transaction>> = BTreeMap::new();
        std::fs::write(items_path, items_toml)?;
        for t in ledger.transactions().iter() {
            let month = YearMonth::from_date(t.date());
            by_month.entry(month).or_default().push(t.clone());
        }

        let transactions_dir = self.dir.join("transactions");
        std::fs::create_dir_all(&transactions_dir)?;

        let mut written = Vec::new();
        for (month, transactions) in by_month {
            let month_path = transactions_dir.join(format!("{month}.toml"));
            let month_toml = toml::to_string(&MonthFile { transactions })?;
            std::fs::write(&month_path, month_toml)?;
            written.push(month_path);
        }

        //取引がなくなった月のファイルの削除
        for entry in std::fs::read_dir(&transactions_dir)? {
            let path = entry?.path();

            if !written.contains(&path) {
                std::fs::remove_file(&path)?;
            }
        }
        Ok(())
    }
}

///帳簿の保存先(テスト用)
#[derive(Debug, Default)]
pub struct MemoryStore {
    ledger: RefCell<Option<Ledger>>,
}

impl LedgerStore for MemoryStore {
    ///帳簿を保存する
    fn save(&self, ledger: &Ledger) -> Result<(), StorageError> {
        *self.ledger.borrow_mut() = Some(ledger.clone());
        Ok(())
    }

    ///帳簿を読み込む
    fn load(&self) -> Result<Ledger, StorageError> {
        Ok(self.ledger.borrow().clone().unwrap_or_default())
    }
}

///帳簿の保存・読み込みで発生するエラー
#[derive(Debug, Error)]
pub enum StorageError {
    #[error("ファイルの読み書きに失敗しました: {0}")]
    Io(#[from] std::io::Error),

    #[error("ファイルの中身が正しくないです: {0}")]
    FailToReadToml(#[from] toml::de::Error),

    #[error("変換することができないです: {0}")]
    FailToWriteToml(#[from] toml::ser::Error),
}

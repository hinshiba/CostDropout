//! 帳簿の保存・読み込み

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use thiserror::Error;

use crate::item::ItemList;
use crate::ledger::Ledger;
use crate::period::YearMonth;
use crate::transaction::{Transaction, TransactionList};

/// 1か月分の取引ファイル
#[derive(Serialize, Deserialize)]
struct MonthFile {
    transactions: Vec<Transaction>,
}

/// 拡張子がtomlかの判断
fn is_toml(path: &Path) -> bool {
    Some(OsStr::new("toml")) == path.extension()
}

/// 帳簿の保存先
pub trait LedgerStore {
    /// 帳簿を読み込む
    fn load(&self) -> Result<Ledger, StorageError>;

    /// 帳簿を保存する
    fn save(&self, ledger: &Ledger) -> Result<(), StorageError>;
}

/// TOMLファイルに帳簿を保存する保存先
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
    /// ファイルの読み込みを行う
    ///
    /// ファイルがなければ空の帳簿を返す
    fn load(&self) -> Result<Ledger, StorageError> {
        let items_path = self.dir.join("items.toml");
        let items = if items_path.exists() {
            let items_toml = std::fs::read_to_string(items_path)?;
            toml::from_str(&items_toml)?
        } else {
            ItemList::new()
        };

        // 取引の読み込み
        // TODO: Pathを外部から注入する
        let transactions_dir = self.dir.join("transactions");
        let mut all = Vec::new();
        if transactions_dir.exists() {
            for entry in std::fs::read_dir(transactions_dir)? {
                let path = entry?.path();
                if !is_toml(&path) {
                    continue;
                }
                let month_toml = std::fs::read_to_string(&path)?;
                let month_file: MonthFile = toml::from_str(&month_toml)?;
                all.extend(month_file.transactions);
            }
        }
        let transactions = TransactionList::from_transactions(all);

        Ok(Ledger::from_parts(items, transactions))
    }

    /// ファイルの保存を行う
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

        // 取引がなくなった月のファイルの削除
        for entry in std::fs::read_dir(&transactions_dir)? {
            let path = entry?.path();

            if !written.contains(&path) && is_toml(&path) {
                std::fs::remove_file(&path)?;
            }
        }
        Ok(())
    }
}

/// 帳簿の保存・読み込みで発生するエラー
#[derive(Debug, Error)]
pub enum StorageError {
    #[error("ファイルの読み書きに失敗しました: {0}")]
    Io(#[from] std::io::Error),

    #[error("TOMLの解析に失敗しました: {0}")]
    ParseToml(#[from] toml::de::Error),

    #[error("TOMLへの変換に失敗しました: {0}")]
    SerializeToml(#[from] toml::ser::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use std::cell::RefCell;

    use crate::item::Tag;
    use crate::money::Money;
    use crate::transaction::{TransactionDraft, TransactionId};

    /// テスト用の日付(2026年の指定した月日)
    fn date(month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, month, day).unwrap()
    }

    /// メモリ上の帳簿に置く保存先
    #[derive(Debug, Default)]
    struct MemoryStore {
        ledger: RefCell<Option<Ledger>>,
    }

    impl LedgerStore for MemoryStore {
        /// 帳簿を保存する
        fn save(&self, ledger: &Ledger) -> Result<(), StorageError> {
            *self.ledger.borrow_mut() = Some(ledger.clone());
            Ok(())
        }

        /// 帳簿を読み込む
        fn load(&self) -> Result<Ledger, StorageError> {
            Ok(self.ledger.borrow().clone().unwrap_or_default())
        }
    }

    /// 品目2つと，10月に2件・11月に1件の取引を持つ帳簿を作る
    ///
    /// 11月の取引の ID も返す
    fn sample_ledger() -> (Ledger, TransactionId) {
        let mut ledger = Ledger::new();
        let food = ledger.add_item("食費", Tag::Expense).unwrap();
        let cash = ledger.add_item("現金", Tag::Asset).unwrap();
        let draft = |month, day, amount, comment: &str| {
            TransactionDraft::new(
                date(month, day),
                Money(amount),
                food,
                cash,
                comment.to_string(),
            )
        };

        ledger.add_transaction(draft(10, 2, 1200, "昼食")).unwrap();
        ledger.add_transaction(draft(10, 5, 800, "夕食")).unwrap();
        let november = ledger.add_transaction(draft(11, 1, 3000, "外食")).unwrap();
        (ledger, november)
    }

    /// 保存先にある全てのファイルの場所と中身を，場所の順に読む
    fn read_all_files(dir: &Path) -> Vec<(PathBuf, String)> {
        let mut paths = vec![dir.join("items.toml")];
        for entry in std::fs::read_dir(dir.join("transactions")).unwrap() {
            paths.push(entry.unwrap().path());
        }
        paths.sort();

        let mut files = Vec::new();
        for path in paths {
            let content = std::fs::read_to_string(&path).unwrap();
            files.push((path, content));
        }
        files
    }

    #[test]
    fn toml_store_save_then_load_returns_same_ledger() {
        let dir = tempfile::tempdir().unwrap();
        let store = TomlStore::new(dir.path().to_path_buf());
        let (ledger, _) = sample_ledger();

        store.save(&ledger).unwrap();

        assert_eq!(store.load().unwrap(), ledger);
    }

    #[test]
    fn toml_store_save_twice_keeps_same_content() {
        let dir = tempfile::tempdir().unwrap();
        let store = TomlStore::new(dir.path().to_path_buf());
        let (ledger, _) = sample_ledger();

        store.save(&ledger).unwrap();
        let first = read_all_files(dir.path());
        store.save(&ledger).unwrap();
        let second = read_all_files(dir.path());

        assert_eq!(first, second);
    }

    #[test]
    fn toml_store_writes_files_by_month() {
        let dir = tempfile::tempdir().unwrap();
        let store = TomlStore::new(dir.path().to_path_buf());
        let (ledger, _) = sample_ledger();

        store.save(&ledger).unwrap();

        let transactions_dir = dir.path().join("transactions");
        assert!(transactions_dir.join("2026-10.toml").exists());
        assert!(transactions_dir.join("2026-11.toml").exists());
        // タグは design.md の形式どおり小文字で書き出す
        let items_toml = std::fs::read_to_string(dir.path().join("items.toml")).unwrap();
        assert!(items_toml.contains("tag = \"expense\""));
    }

    #[test]
    fn toml_store_load_empty_dir_returns_empty_ledger() {
        let dir = tempfile::tempdir().unwrap();
        let store = TomlStore::new(dir.path().to_path_buf());

        assert_eq!(store.load().unwrap(), Ledger::new());
    }

    #[test]
    fn toml_store_load_missing_dir_returns_empty_ledger() {
        let dir = tempfile::tempdir().unwrap();
        let store = TomlStore::new(dir.path().join("missing"));

        assert_eq!(store.load().unwrap(), Ledger::new());
    }

    #[test]
    fn toml_store_save_removes_empty_month_file() {
        let dir = tempfile::tempdir().unwrap();
        let store = TomlStore::new(dir.path().to_path_buf());
        let (mut ledger, november) = sample_ledger();
        store.save(&ledger).unwrap();

        // 11月の取引を消すと，11月のファイルも消える
        ledger.remove_transaction(november).unwrap();
        store.save(&ledger).unwrap();

        let transactions_dir = dir.path().join("transactions");
        assert!(transactions_dir.join("2026-10.toml").exists());
        assert!(!transactions_dir.join("2026-11.toml").exists());
        assert_eq!(store.load().unwrap(), ledger);
    }

    #[test]
    fn toml_store_load_ignores_non_toml_file() {
        let dir = tempfile::tempdir().unwrap();
        let store = TomlStore::new(dir.path().to_path_buf());
        let (ledger, _) = sample_ledger();
        store.save(&ledger).unwrap();

        // TOMLとして読めない中身のファイルを置いても，読み込みは失敗しない
        let memo_path = dir.path().join("transactions").join("memo.txt");
        std::fs::write(&memo_path, "TOMLではないメモ").unwrap();

        assert_eq!(store.load().unwrap(), ledger);
    }

    #[test]
    fn toml_store_save_keeps_non_toml_file() {
        let dir = tempfile::tempdir().unwrap();
        let store = TomlStore::new(dir.path().to_path_buf());
        let (ledger, _) = sample_ledger();
        store.save(&ledger).unwrap();

        let memo_path = dir.path().join("transactions").join("memo.txt");
        std::fs::write(&memo_path, "TOMLではないメモ").unwrap();
        store.save(&ledger).unwrap();

        assert!(memo_path.exists());
    }

    #[test]
    fn memory_store_load_before_save_returns_empty_ledger() {
        let store = MemoryStore::default();

        assert_eq!(store.load().unwrap(), Ledger::new());
    }

    #[test]
    fn memory_store_save_then_load_returns_same_ledger() {
        let store = MemoryStore::default();
        let (ledger, _) = sample_ledger();

        store.save(&ledger).unwrap();

        assert_eq!(store.load().unwrap(), ledger);
    }
}

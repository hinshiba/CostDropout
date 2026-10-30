//! 保存すると commit される帳簿
use std::path::Path;
use thiserror::Error;

use crate::infra::filestore::{LedgerStore, StorageError, TomlStore};
use crate::infra::gitrepo::{GitError, GitRepo, History};
use crate::ledger::Ledger;

/// 保存先と変更履歴をもつ帳簿.
///
/// 保存すると保存先に書き出してから commit する.
pub struct Book<S: LedgerStore, H: History> {
    store: S,
    history: H,
    ledger: Ledger,
    /// 最後に保存した帳簿
    saved: Ledger,
}

/// 帳簿の保存, 読み込み, commit で発生するエラー
#[derive(Debug, Error)]
pub enum BookError {
    #[error(transparent)]
    Storage(#[from] StorageError),

    #[error(transparent)]
    History(#[from] GitError),
}

impl<S: LedgerStore, H: History> Book<S, H> {
    /// 保存先から帳簿を読み込む.
    ///
    /// # Errors
    ///
    /// `LedgerStore::load` のエラー.
    pub fn load(store: S, history: H) -> Result<Self, BookError> {
        let ledger = store.load()?;
        Ok(Self {
            store,
            history,
            saved: ledger.clone(),
            ledger,
        })
    }

    /// 帳簿を返す.
    pub fn ledger(&self) -> &Ledger {
        &self.ledger
    }

    /// 変更できる帳簿を返す.
    pub fn ledger_mut(&mut self) -> &mut Ledger {
        &mut self.ledger
    }

    /// 最後に保存した帳簿と内容が異なるか.
    pub fn is_dirty(&self) -> bool {
        self.ledger != self.saved
    }

    /// 帳簿を保存してから commit する.
    ///
    /// 失敗した場合は保存していないものとして扱い, 次の `save` で書き直す.
    ///
    /// # Errors
    ///
    /// `LedgerStore::save` と `History::commit_all` のエラー.
    pub fn save(&mut self, message: &str) -> Result<(), BookError> {
        self.store.save(&self.ledger)?;
        self.history.commit_all(message)?;
        self.saved = self.ledger.clone();
        Ok(())
    }
}

impl Book<TomlStore, GitRepo> {
    /// ディレクトリの帳簿を開く.
    ///
    /// 空のディレクトリなら git リポジトリにし, プリセット品目の帳簿を保存して commit する.
    ///
    /// # Errors
    ///
    /// ディレクトリを読めない場合は `BookError::Storage` エラー.
    /// 空でなく git リポジトリのルートでもない場合は `BookError::History` エラー.
    /// そのほか `TomlStore` と `GitRepo` のエラー.
    pub fn open(dir: &Path) -> Result<Self, BookError> {
        let store = TomlStore::new(dir.to_path_buf());
        let is_empty = dir.read_dir().map_err(StorageError::from)?.next().is_none();
        if !is_empty {
            return Self::load(store, GitRepo::open(dir.to_path_buf())?);
        }

        let mut book = Self {
            store,
            history: GitRepo::init(dir.to_path_buf())?,
            ledger: Ledger::with_presets(),
            saved: Ledger::new(),
        };
        book.save("帳簿を作成")?;
        Ok(book)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    use crate::infra::gitrepo::git_command;

    use crate::ledger::LedgerError;
    use crate::ledger::item::{ItemError, Tag};

    /// メモリ上の帳簿に置く保存先
    #[derive(Debug, Default)]
    struct MemoryStore {
        ledger: RefCell<Option<Ledger>>,
    }

    impl LedgerStore for MemoryStore {
        fn save(&self, ledger: &Ledger) -> Result<(), StorageError> {
            *self.ledger.borrow_mut() = Some(ledger.clone());
            Ok(())
        }

        fn load(&self) -> Result<Ledger, StorageError> {
            Ok(self.ledger.borrow().clone().unwrap_or_default())
        }
    }

    #[test]
    fn memory_store_load_before_save_returns_empty_ledger() {
        let store = MemoryStore::default();

        assert_eq!(store.load().unwrap(), Ledger::new());
    }

    #[test]
    fn memory_store_save_then_load_returns_same_ledger() {
        let store = MemoryStore::default();
        let ledger = Ledger::with_presets();

        store.save(&ledger).unwrap();

        assert_eq!(store.load().unwrap(), ledger);
    }

    /// commit したメッセージを記録する変更履歴
    #[derive(Debug, Default)]
    struct FakeHistory {
        messages: RefCell<Vec<String>>,
        is_broken: bool,
    }

    impl History for FakeHistory {
        fn commit_all(&self, message: &str) -> Result<(), GitError> {
            if self.is_broken {
                return Err(GitError::CommandFailed(
                    "commit".to_string(),
                    "壊れている".to_string(),
                ));
            }
            self.messages.borrow_mut().push(message.to_string());
            Ok(())
        }
    }

    /// 書き込めない保存先
    struct BrokenStore;

    impl LedgerStore for BrokenStore {
        fn load(&self) -> Result<Ledger, StorageError> {
            Ok(Ledger::with_presets())
        }

        fn save(&self, _: &Ledger) -> Result<(), StorageError> {
            Err(std::io::Error::other("壊れている").into())
        }
    }

    /// プリセット品目の帳簿を保存した保存先から開いた帳簿
    fn book(history: FakeHistory) -> Book<MemoryStore, FakeHistory> {
        let store = MemoryStore::default();
        store.save(&Ledger::with_presets()).unwrap();
        Book::load(store, history).unwrap()
    }

    #[test]
    fn load_returns_clean_book() {
        let book = book(FakeHistory::default());

        assert_eq!(book.ledger(), &Ledger::with_presets());
        assert!(!book.is_dirty());
    }

    #[test]
    fn is_dirty_compares_with_saved_ledger() {
        let mut book = book(FakeHistory::default());

        let id = book.ledger_mut().add_item("書籍", Tag::Expense).unwrap();
        assert!(book.is_dirty());

        // 元に戻せば保存済みの内容と同じになる
        book.ledger_mut().remove_item(id).unwrap();
        assert!(!book.is_dirty());

        // 失敗した操作は帳簿を変えない
        let error = book.ledger_mut().add_item("食費", Tag::Expense);
        assert_eq!(
            error,
            Err(LedgerError::Item(ItemError::DuplicateName(
                "食費".to_string()
            )))
        );
        assert!(!book.is_dirty());
    }

    #[test]
    fn save_stores_then_commits() {
        let mut book = book(FakeHistory::default());
        book.ledger_mut().add_item("書籍", Tag::Expense).unwrap();

        book.save("書籍を追加").unwrap();

        assert!(!book.is_dirty());
        assert_eq!(&book.store.load().unwrap(), book.ledger());
        assert_eq!(*book.history.messages.borrow(), ["書籍を追加"]);
    }

    #[test]
    fn save_keeps_dirty_when_commit_fails() {
        let mut book = book(FakeHistory {
            is_broken: true,
            ..FakeHistory::default()
        });
        book.ledger_mut().add_item("書籍", Tag::Expense).unwrap();
        let edited = book.ledger().clone();

        let result = book.save("書籍を追加");

        assert!(matches!(
            result,
            Err(BookError::History(GitError::CommandFailed(command, _))) if command == "commit"
        ));
        assert!(book.is_dirty());
        assert_eq!(book.ledger(), &edited);
    }

    #[test]
    fn save_does_not_commit_when_store_fails() {
        let mut book = Book::load(BrokenStore, FakeHistory::default()).unwrap();
        book.ledger_mut().add_item("書籍", Tag::Expense).unwrap();
        let edited = book.ledger().clone();

        let result = book.save("書籍を追加");

        assert!(matches!(
            result,
            Err(BookError::Storage(StorageError::Io(e))) if e.to_string() == "壊れている"
        ));
        assert!(book.history.messages.borrow().is_empty());
        assert!(book.is_dirty());
        assert_eq!(book.ledger(), &edited);
    }

    #[test]
    fn open_loads_repository() {
        let dir = tempfile::tempdir().unwrap();
        GitRepo::init(dir.path().to_path_buf()).unwrap();
        let mut ledger = Ledger::new();
        ledger.add_item("書籍", Tag::Expense).unwrap();
        TomlStore::new(dir.path().to_path_buf())
            .save(&ledger)
            .unwrap();

        let book = Book::open(dir.path()).unwrap();

        assert_eq!(book.ledger(), &ledger);
        assert!(!book.is_dirty());
    }

    #[test]
    fn save_commits_to_repository() {
        let dir = tempfile::tempdir().unwrap();
        let repo = GitRepo::init(dir.path().to_path_buf()).unwrap();
        for (key, value) in [("user.name", "test"), ("user.email", "test@example.com")] {
            let output = git_command(dir.path())
                .args(["config", key, value])
                .output()
                .unwrap();
            assert!(output.status.success(), "{key}");
        }
        let mut book = Book::open(dir.path()).unwrap();
        book.ledger_mut().add_item("書籍", Tag::Expense).unwrap();

        book.save("書籍を追加").unwrap();

        assert!(!repo.has_changes().unwrap());
        assert_eq!(
            &TomlStore::new(dir.path().to_path_buf()).load().unwrap(),
            book.ledger()
        );
    }

    #[test]
    fn open_rejects_missing_dir() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("missing");

        let result = Book::open(&missing);

        assert!(matches!(
            result,
            Err(BookError::Storage(StorageError::Io(e))) if e.kind() == std::io::ErrorKind::NotFound
        ));
        assert!(!missing.exists());
    }

    #[test]
    fn open_rejects_non_repository() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("memo.txt"), "メモ").unwrap();

        let result = Book::open(dir.path());

        assert!(matches!(
            result,
            Err(BookError::History(GitError::CommandFailed(command, _)))
                if command == "git -C <path> rev-parse --show-cdup"
        ));
        assert!(!dir.path().join(".git").exists());
    }
}

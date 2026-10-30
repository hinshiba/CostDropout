//! 保存すると commit される帳簿
use crate::git::{GitError, GitRepo};
use crate::ledger::Ledger;
use crate::storage::{LedgerStore, StorageError, TomlStore};
use std::path::Path;
use thiserror::Error;

/// 帳簿の変更履歴
pub trait History {
    /// すべての変更を commit する.
    ///
    /// 変更がなければ何もしない.
    fn commit_all(&self, message: &str) -> Result<(), GitError>;

    fn has_remote(&self) -> Result<bool, GitError>;

    fn push(&self) -> Result<(), GitError>;

    fn pull_rebase(&self) -> Result<(), GitError>;
}

impl History for GitRepo {
    fn commit_all(&self, message: &str) -> Result<(), GitError> {
        GitRepo::commit_all(self, message).map(drop)
    }

    fn has_remote(&self) -> Result<bool, GitError> {
        GitRepo::has_remote(self)
    }

    fn push(&self) -> Result<(), GitError> {
        GitRepo::push(self)
    }

    fn pull_rebase(&self) -> Result<(), GitError> {
        GitRepo::pull_rebase(self)
    }
}

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

    #[error("未保存の変更があります")]
    UnsavedChanges,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncOutcome {
    Pushed,
    PulledAndPushed,
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

    //保存した最新の帳簿を読み込んで反映する．
    fn reload(&mut self) -> Result<(), BookError> {
        self.ledger = self.store.load()?;
        self.saved = self.ledger.clone();
        Ok(())
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

    pub fn sync(&mut self) -> Result<SyncOutcome, BookError> {
        if self.is_dirty() {
            return Err(BookError::UnsavedChanges);
        }

        if !self.history.has_remote()? {
            return Ok(SyncOutcome::Pushed);
        }

        match self.history.push() {
            Ok(()) => return Ok(SyncOutcome::Pushed),

            Err(GitError::PushRejected) => {}

            Err(error) => return Err(error.into()),
        }

        self.history.pull_rebase()?;

        self.reload()?;

        self.history.push()?;

        Ok(SyncOutcome::PulledAndPushed)
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
    use crate::git::git_command;
    use crate::item::{ItemError, Tag};
    use crate::ledger::LedgerError;
    use crate::storage::MemoryStore;
    use std::cell::RefCell;
    use std::path::Path;
    use std::process::Command;

    /// commit したメッセージを記録する変更履歴
    #[derive(Debug, Default)]
    struct FakeHistory {
        messages: RefCell<Vec<String>>,
        is_broken: bool,
        has_remote: bool,
        push_rejected: bool,
        pull_rebased: RefCell<bool>,
        push_count: RefCell<u32>,
        pull_rebase_fails: bool,
    }

    fn run_git(dir: &Path, args: &[&str]) {
        let output = git_command(dir).args(args).output().unwrap();

        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn configure_user(dir: &Path) {
        run_git(dir, &["config", "user.name", "test"]);
        run_git(dir, &["config", "user.email", "test@example.com"]);
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

        fn has_remote(&self) -> Result<bool, GitError> {
            Ok(self.has_remote)
        }

        fn push(&self) -> Result<(), GitError> {
            let mut count = self.push_count.borrow_mut();

            *count += 1;

            if *count == 1 && self.push_rejected {
                return Err(GitError::PushRejected);
            }

            Ok(())
        }

        fn pull_rebase(&self) -> Result<(), GitError> {
            *self.pull_rebased.borrow_mut() = true;
            if self.pull_rebase_fails {
                return Err(GitError::Conflict);
            }
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

    #[test]
    fn sync_returns_error_when_dirty() {
        let mut book = book(FakeHistory::default());

        book.ledger_mut().add_item("書籍", Tag::Expense).unwrap();

        assert!(matches!(book.sync(), Err(BookError::UnsavedChanges)));
    }

    #[test]
    fn sync_succeeds_without_remote() {
        let mut book = book(FakeHistory::default());

        let result = book.sync();

        assert!(matches!(result, Ok(SyncOutcome::Pushed)));
    }

    #[test]
    fn sync_pulls_then_pushes_when_rejected() {
        let mut book = book(FakeHistory {
            has_remote: true,
            push_rejected: true,
            ..FakeHistory::default()
        });

        let result = book.sync();

        assert!(matches!(result, Ok(SyncOutcome::PulledAndPushed)));
        //pull_rebaseが呼ばれた
        assert!(*book.history.pull_rebased.borrow());
        //pushが２回呼ばれた
        assert_eq!(*book.history.push_count.borrow(), 2);
    }

    #[test]
    fn sync_returns_conflict_when_pull_rebase_fails() {
        let mut book = book(FakeHistory {
            has_remote: true,
            push_rejected: true,
            pull_rebase_fails: true,
            ..FakeHistory::default()
        });

        let result = book.sync();

        assert!(matches!(
            result,
            Err(BookError::History(GitError::Conflict))
        ));
    }

    #[test]
    fn sync_pushes_when_remote_exists() {
        let mut book = book(FakeHistory {
            has_remote: true,
            ..FakeHistory::default()
        });

        let result = book.sync();

        assert!(matches!(result, Ok(SyncOutcome::Pushed)));

        assert_eq!(*book.history.push_count.borrow(), 1);

        assert!(!*book.history.pull_rebased.borrow());
    }

    #[test]
    fn sync_propagates_has_remote_error() {
        #[derive(Default)]
        struct BrokenHistory;

        impl History for BrokenHistory {
            fn commit_all(&self, _: &str) -> Result<(), GitError> {
                Ok(())
            }

            fn has_remote(&self) -> Result<bool, GitError> {
                Err(GitError::CommandFailed(
                    "remote".to_string(),
                    "error".to_string(),
                ))
            }

            fn push(&self) -> Result<(), GitError> {
                Ok(())
            }

            fn pull_rebase(&self) -> Result<(), GitError> {
                Ok(())
            }
        }

        let store = MemoryStore::default();
        store.save(&Ledger::with_presets()).unwrap();

        let mut book = Book::load(store, BrokenHistory).unwrap();

        assert!(matches!(
            book.sync(),
            Err(BookError::History(GitError::CommandFailed(_, _)))
        ));
    }

    #[test]
    fn sync_pulls_remote_changes_into_book() {
        let root = tempfile::tempdir().unwrap();

        let remote = root.path().join("remote.git");
        let init_dir = root.path().join("init");
        let a_dir = root.path().join("a");
        let b_dir = root.path().join("b");

        // bare remote
        let output = Command::new("git")
            .args(["init", "--bare"])
            .arg(&remote)
            .output()
            .unwrap();

        assert!(output.status.success());

        //
        // 初期帳簿を作成して remote に push
        //
        let output = Command::new("git")
            .arg("clone")
            .arg(&remote)
            .arg(&init_dir)
            .output()
            .unwrap();

        assert!(output.status.success());

        configure_user(&init_dir);

        let repo = GitRepo::open(init_dir.clone()).unwrap();

        let store = TomlStore::new(init_dir.clone());
        store.save(&Ledger::with_presets()).unwrap();

        repo.commit_all("帳簿を作成").unwrap();
        repo.push().unwrap();

        //
        // A/B clone
        //
        let output = Command::new("git")
            .arg("clone")
            .arg(&remote)
            .arg(&a_dir)
            .output()
            .unwrap();

        assert!(output.status.success());

        let output = Command::new("git")
            .arg("clone")
            .arg(&remote)
            .arg(&b_dir)
            .output()
            .unwrap();

        assert!(output.status.success());

        configure_user(&a_dir);
        configure_user(&b_dir);

        let mut book_a = Book::load(
            TomlStore::new(a_dir.clone()),
            GitRepo::open(a_dir.clone()).unwrap(),
        )
        .unwrap();

        let mut book_b = Book::load(
            TomlStore::new(b_dir.clone()),
            GitRepo::open(b_dir.clone()).unwrap(),
        )
        .unwrap();

        //
        // A が変更して同期
        //
        book_a
            .ledger_mut()
            .add_item("A専用品目", Tag::Expense)
            .unwrap();

        book_a.save("A追加").unwrap();

        assert_eq!(book_a.sync().unwrap(), SyncOutcome::Pushed);

        //
        // B は変更せず同期
        //
        assert_eq!(book_b.sync().unwrap(), SyncOutcome::PulledAndPushed);

        assert!(!book_b.is_dirty());
        //
        // reload() が実行されていれば
        // A の変更が帳簿へ反映される
        //
        assert!(
            book_b
                .ledger()
                .items()
                .iter()
                .any(|item| item.to_string() == "A専用品目")
        );
    }
}

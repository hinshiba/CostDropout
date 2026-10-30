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

    #[error("未保存の変更があります")]
    UnsavedChanges,

    #[error("取り込むと帳簿が不正になります: {0}")]
    InvalidMerge(#[source] StorageError),
}

/// 同期の結果
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncOutcome {
    /// 同期先がないので何もしなかった
    NoRemote,
    /// push した. push するものがなかった場合を含む
    Pushed,
    /// 同期先を取り込んでから push した
    PulledAndPushed,
}

/// 帳簿に触れずに同期する処理.
///
/// バックグラウンドで実行できるよう, 保存先と履歴の複製だけを持つ.
#[must_use]
pub struct SyncJob<S, H> {
    store: S,
    history: H,
}

impl<S: LedgerStore, H: History> SyncJob<S, H> {
    /// push し, 拒否されたら取り込んでからもう一度 push する.
    ///
    /// 終わったら成否に関わらず `Book::reload` を呼ぶこと.
    ///
    /// # Errors
    ///
    /// 取り込んだ結果を読み込めなければ, 取り込む前に戻して `BookError::InvalidMerge` エラー.
    /// そのほか `History` のエラー.
    pub fn run(self) -> Result<SyncOutcome, BookError> {
        if !self.history.has_remote()? {
            return Ok(SyncOutcome::NoRemote);
        }

        match self.history.push() {
            Ok(()) => return Ok(SyncOutcome::Pushed),
            Err(GitError::PushRejected) => {}
            Err(error) => return Err(error.into()),
        }

        let head = self.history.head()?;
        self.history.pull_rebase()?;

        // 別々のファイルの変更は git の競合にならないが, 帳簿としては不正になり得る
        if let Err(error) = self.store.load() {
            self.history.reset_hard(&head)?;
            return Err(BookError::InvalidMerge(error));
        }

        self.history.push()?;
        Ok(SyncOutcome::PulledAndPushed)
    }
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

    /// 保存先から帳簿を読み直す. 未保存の変更は失われる.
    ///
    /// 失敗した場合は帳簿を変えない.
    ///
    /// # Errors
    ///
    /// `LedgerStore::load` のエラー.
    pub fn reload(&mut self) -> Result<(), BookError> {
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
}

impl<S: LedgerStore + Clone, H: History + Clone> Book<S, H> {
    /// 同期する処理を作る.
    ///
    /// # Errors
    ///
    /// 未保存の変更があれば `BookError::UnsavedChanges` エラー.
    pub fn sync_job(&self) -> Result<SyncJob<S, H>, BookError> {
        if self.is_dirty() {
            return Err(BookError::UnsavedChanges);
        }

        Ok(SyncJob {
            store: self.store.clone(),
            history: self.history.clone(),
        })
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
    use crate::ledger::LedgerError;
    use std::cell::{Cell, RefCell};
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::rc::Rc;

    use crate::infra::gitrepo::{CommitId, git_command};
    use crate::ledger::item::{ItemError, ItemId, Tag};
    use crate::ledger::money::Money;
    use crate::ledger::transaction::TransactionDraft;

    /// メモリ上の帳簿に置く保存先. 複製は同じ帳簿を共有する
    #[derive(Debug, Default, Clone)]
    struct MemoryStore {
        ledger: Rc<RefCell<Option<Ledger>>>,
        is_load_broken: Rc<Cell<bool>>,
    }

    impl LedgerStore for MemoryStore {
        fn save(&self, ledger: &Ledger) -> Result<(), StorageError> {
            *self.ledger.borrow_mut() = Some(ledger.clone());
            Ok(())
        }

        fn load(&self) -> Result<Ledger, StorageError> {
            if self.is_load_broken.get() {
                return Err(std::io::Error::other("壊れている").into());
            }
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

    /// 呼び出しを記録する変更履歴. 複製は同じ記録を共有する
    #[derive(Debug, Default, Clone)]
    struct FakeHistory {
        messages: Rc<RefCell<Vec<String>>>,
        is_broken: bool,
        has_remote: bool,
        is_has_remote_broken: bool,
        is_push_rejected: bool,
        is_pulled: Rc<Cell<bool>>,
        push_count: Rc<Cell<u32>>,
        is_pull_conflicted: bool,
        reset_to: Rc<RefCell<Option<CommitId>>>,
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
            if self.is_has_remote_broken {
                return Err(GitError::CommandFailed(
                    "remote".to_string(),
                    "壊れている".to_string(),
                ));
            }
            Ok(self.has_remote)
        }

        fn push(&self) -> Result<(), GitError> {
            self.push_count.set(self.push_count.get() + 1);
            if self.push_count.get() == 1 && self.is_push_rejected {
                return Err(GitError::PushRejected);
            }
            Ok(())
        }

        fn pull_rebase(&self) -> Result<(), GitError> {
            self.is_pulled.set(true);
            if self.is_pull_conflicted {
                return Err(GitError::Conflict);
            }
            Ok(())
        }

        fn head(&self) -> Result<CommitId, GitError> {
            Ok(CommitId("取り込む前".to_string()))
        }

        fn reset_hard(&self, commit: &CommitId) -> Result<(), GitError> {
            *self.reset_to.borrow_mut() = Some(commit.clone());
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

    /// GUI と同じ順に同期する
    fn sync<S, H>(book: &mut Book<S, H>) -> Result<SyncOutcome, BookError>
    where
        S: LedgerStore + Clone,
        H: History + Clone,
    {
        let result = book.sync_job()?.run();
        book.reload()?;
        result
    }

    #[test]
    fn sync_job_returns_error_when_dirty() {
        let mut book = book(FakeHistory::default());

        book.ledger_mut().add_item("書籍", Tag::Expense).unwrap();

        assert!(matches!(book.sync_job(), Err(BookError::UnsavedChanges)));
        assert!(book.is_dirty());
    }

    #[test]
    fn sync_does_nothing_without_remote() {
        let mut book = book(FakeHistory::default());

        assert_eq!(sync(&mut book).unwrap(), SyncOutcome::NoRemote);
        assert_eq!(book.history.push_count.get(), 0);
    }

    #[test]
    fn sync_pulls_then_pushes_when_rejected() {
        let mut book = book(FakeHistory {
            has_remote: true,
            is_push_rejected: true,
            ..FakeHistory::default()
        });

        assert_eq!(sync(&mut book).unwrap(), SyncOutcome::PulledAndPushed);
        assert!(book.history.is_pulled.get());
        assert_eq!(book.history.push_count.get(), 2);
        assert_eq!(*book.history.reset_to.borrow(), None);
    }

    #[test]
    fn sync_returns_conflict_when_pull_rebase_fails() {
        let mut book = book(FakeHistory {
            has_remote: true,
            is_push_rejected: true,
            is_pull_conflicted: true,
            ..FakeHistory::default()
        });

        assert!(matches!(
            sync(&mut book),
            Err(BookError::History(GitError::Conflict))
        ));
        assert_eq!(book.history.push_count.get(), 1);
    }

    #[test]
    fn sync_pushes_when_remote_exists() {
        let mut book = book(FakeHistory {
            has_remote: true,
            ..FakeHistory::default()
        });

        assert_eq!(sync(&mut book).unwrap(), SyncOutcome::Pushed);
        assert_eq!(book.history.push_count.get(), 1);
        assert!(!book.history.is_pulled.get());
    }

    #[test]
    fn sync_propagates_has_remote_error() {
        let mut book = book(FakeHistory {
            is_has_remote_broken: true,
            ..FakeHistory::default()
        });

        assert!(matches!(
            sync(&mut book),
            Err(BookError::History(GitError::CommandFailed(command, _))) if command == "remote"
        ));
    }

    #[test]
    fn job_rolls_back_and_does_not_push_when_merged_ledger_is_invalid() {
        let history = FakeHistory {
            has_remote: true,
            is_push_rejected: true,
            ..FakeHistory::default()
        };
        let store = MemoryStore::default();
        store.is_load_broken.set(true);
        let job = SyncJob {
            store,
            history: history.clone(),
        };

        assert!(matches!(
            job.run(),
            Err(BookError::InvalidMerge(StorageError::Io(e))) if e.to_string() == "壊れている"
        ));
        assert_eq!(
            *history.reset_to.borrow(),
            Some(CommitId("取り込む前".to_string()))
        );
        assert_eq!(history.push_count.get(), 1);
    }

    #[test]
    fn reload_replaces_ledger_and_marks_clean() {
        let mut book = book(FakeHistory::default());
        book.ledger_mut().add_item("書籍", Tag::Expense).unwrap();
        let mut stored = Ledger::with_presets();
        stored.add_item("雑誌", Tag::Expense).unwrap();
        book.store.save(&stored).unwrap();

        book.reload().unwrap();

        assert_eq!(book.ledger(), &stored);
        assert!(!book.is_dirty());
    }

    #[test]
    fn reload_keeps_ledger_when_load_fails() {
        let mut book = book(FakeHistory::default());
        book.ledger_mut().add_item("書籍", Tag::Expense).unwrap();
        let edited = book.ledger().clone();
        book.store.is_load_broken.set(true);

        assert!(matches!(
            book.reload(),
            Err(BookError::Storage(StorageError::Io(_)))
        ));
        assert_eq!(book.ledger(), &edited);
        assert!(book.is_dirty());
    }

    fn git_clone(remote: &Path, dir: &Path) {
        let output = Command::new("git")
            .arg("clone")
            .arg(remote)
            .arg(dir)
            .output()
            .unwrap();
        assert!(output.status.success());
        configure_user(dir);
    }

    /// `ledger` を push した bare リポジトリと, その2つの clone を作る
    fn shared_clones(root: &Path, ledger: &Ledger) -> (PathBuf, PathBuf, PathBuf) {
        let remote = root.join("remote.git");
        let output = Command::new("git")
            .args(["init", "--bare"])
            .arg(&remote)
            .output()
            .unwrap();
        assert!(output.status.success());

        let init_dir = root.join("init");
        git_clone(&remote, &init_dir);
        TomlStore::new(init_dir.clone()).save(ledger).unwrap();
        let repo = GitRepo::open(init_dir).unwrap();
        repo.commit_all("帳簿を作成").unwrap();
        repo.push().unwrap();

        let a_dir = root.join("a");
        let b_dir = root.join("b");
        git_clone(&remote, &a_dir);
        git_clone(&remote, &b_dir);
        (remote, a_dir, b_dir)
    }

    fn open_clone(dir: &Path) -> Book<TomlStore, GitRepo> {
        Book::load(
            TomlStore::new(dir.to_path_buf()),
            GitRepo::open(dir.to_path_buf()).unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn sync_does_nothing_for_repository_without_origin() {
        let dir = tempfile::tempdir().unwrap();
        TomlStore::new(dir.path().to_path_buf())
            .save(&Ledger::new())
            .unwrap();
        GitRepo::init(dir.path().to_path_buf()).unwrap();
        let mut book = Book::open(dir.path()).unwrap();

        assert_eq!(sync(&mut book).unwrap(), SyncOutcome::NoRemote);
    }

    #[test]
    fn sync_pulls_remote_changes_into_book() {
        let root = tempfile::tempdir().unwrap();
        let (_remote, a_dir, b_dir) = shared_clones(root.path(), &Ledger::with_presets());
        let mut book_a = open_clone(&a_dir);
        let mut book_b = open_clone(&b_dir);

        book_a
            .ledger_mut()
            .add_item("A専用品目", Tag::Expense)
            .unwrap();
        book_a.save("A追加").unwrap();
        assert_eq!(sync(&mut book_a).unwrap(), SyncOutcome::Pushed);

        // B は変更していなくても push が拒否され, A の変更を取り込む
        assert_eq!(sync(&mut book_b).unwrap(), SyncOutcome::PulledAndPushed);

        assert!(!book_b.is_dirty());
        assert_eq!(book_b.ledger(), book_a.ledger());
    }

    #[test]
    fn sync_rejects_merge_that_breaks_ledger() {
        let root = tempfile::tempdir().unwrap();
        let mut ledger = Ledger::new();
        let unused = ledger.add_item("未使用", Tag::Expense).unwrap();
        let cash = ledger.add_item("現金", Tag::Asset).unwrap();
        let (remote, a_dir, b_dir) = shared_clones(root.path(), &ledger);
        let mut book_a = open_clone(&a_dir);
        let mut book_b = open_clone(&b_dir);

        // 別々のファイルの変更なので git の競合にはならない
        book_a.ledger_mut().remove_item(unused).unwrap();
        book_a.save("品目を削除").unwrap();
        sync(&mut book_a).unwrap();
        add_transaction(&mut book_b, unused, cash);
        book_b.save("取引を追加").unwrap();
        let edited = book_b.ledger().clone();
        let head = book_b.history.head().unwrap();

        assert!(matches!(
            sync(&mut book_b),
            Err(BookError::InvalidMerge(StorageError::InvalidLedger(
                LedgerError::TransactionItemNotFound(id)
            ))) if id == unused
        ));

        assert_eq!(book_b.history.head().unwrap(), head);
        assert!(!book_b.history.has_changes().unwrap());
        assert_eq!(book_b.ledger(), &edited);

        // 同期先には不正な帳簿が届いていない
        let c_dir = root.path().join("c");
        git_clone(&remote, &c_dir);
        assert_eq!(open_clone(&c_dir).ledger(), book_a.ledger());
    }

    fn add_transaction(book: &mut Book<TomlStore, GitRepo>, item: ItemId, counterpart: ItemId) {
        book.ledger_mut()
            .add_transaction(TransactionDraft::new(
                chrono::NaiveDate::from_ymd_opt(2026, 10, 2).unwrap(),
                Amount::try_from(Money::from(1200)).unwrap(),
                item,
                counterpart,
                String::new(),
            ))
            .unwrap();
    }

    #[test]
    fn history_has_remote_returns_false() {
        let dir = tempfile::tempdir().unwrap();

        let repo = GitRepo::init(dir.path().to_path_buf()).unwrap();

        assert!(!<GitRepo as History>::has_remote(&repo).unwrap());
    }

    #[test]
    fn save_creates_git_commit() {
        let dir = tempfile::tempdir().unwrap();

        let repo = GitRepo::init(dir.path().to_path_buf()).unwrap();

        configure_user(dir.path());

        let store = TomlStore::new(dir.path().to_path_buf());

        store.save(&Ledger::with_presets()).unwrap();

        let mut book = Book::load(store, repo).unwrap();

        book.ledger_mut().add_item("書籍", Tag::Expense).unwrap();

        book.save("書籍を追加").unwrap();

        let output = git_command(dir.path())
            .args(["rev-list", "--count", "HEAD"])
            .output()
            .unwrap();

        assert!(output.status.success());

        let count = String::from_utf8(output.stdout)
            .unwrap()
            .trim()
            .parse::<u32>()
            .unwrap();

        assert_eq!(count, 1);
    }
}

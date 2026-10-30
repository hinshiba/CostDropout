//! Gitリポジトリの操作と帳簿の変更履歴

pub mod audit;

use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use thiserror::Error;

/// 同期先のリモート名
const REMOTE: &str = "origin";

/// Gitコマンドを生成する。
/// 指定したパスを使い、リポジトリを切り替える環境変数を除去する。
/// GUI からは端末の問い合わせに答えられないので, 待たずに失敗させる。
pub(crate) fn git_command(path: &Path) -> Command {
    let mut command = Command::new("git");

    command
        .args(["-C"])
        .arg(path)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env("GIT_TERMINAL_PROMPT", "0");

    command
}

/// 指定されたパス自体がGitリポジトリのルートか判定する。
fn is_repository(path: &Path) -> Result<bool, GitError> {
    let output = git_command(path)
        .args(["rev-parse", "--show-cdup"])
        .output()
        .map_err(GitError::Command)?;

    if !output.status.success() {
        return Ok(false);
    }

    // ルートでは空行を出力する
    Ok(output.stdout.trim_ascii().is_empty())
}

/// commit のハッシュ
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitId(pub(crate) String);

/// Gitリポジトリを操作するためのラッパー。
#[derive(Debug, Clone)]
pub struct GitRepo {
    path: PathBuf,
}

impl GitRepo {
    /// 指定されたパスがGitリポジトリのルートであることを確認して開く。
    pub fn open(path: PathBuf) -> Result<GitRepo, GitError> {
        if !is_repository(&path)? {
            return Err(GitError::CommandFailed(
                "git -C <path> rev-parse --show-cdup".to_string(),
                "指定されたパスはGitリポジトリのルートではありません".to_string(),
            ));
        }

        Ok(GitRepo { path })
    }

    /// Gitリポジトリのパスを参照として返す。
    fn path(&self) -> &Path {
        &self.path
    }

    /// 指定されたパスにGitリポジトリを初期化する。
    /// すでにGitリポジトリなら、そのまま開く。
    pub fn init(path: PathBuf) -> Result<GitRepo, GitError> {
        if is_repository(&path)? {
            return Ok(GitRepo { path });
        }

        let output = git_command(&path)
            .arg("init")
            .output()
            .map_err(GitError::Command)?;

        if !output.status.success() {
            return Err(GitError::CommandFailed(
                "git -C <path> init".to_string(),
                String::from_utf8_lossy(&output.stderr).trim().to_string(),
            ));
        }

        Ok(GitRepo { path })
    }

    /// 作業ツリーに変更があるか確認する。
    pub fn has_changes(&self) -> Result<bool, GitError> {
        let output = git_command(&self.path)
            .args(["status", "--porcelain"])
            .output()
            .map_err(GitError::Command)?;

        if !output.status.success() {
            return Err(GitError::CommandFailed(
                "git -C <path> status --porcelain".to_string(),
                String::from_utf8_lossy(&output.stderr).trim().to_string(),
            ));
        }

        Ok(!output.stdout.is_empty())
    }

    /// すべての変更をステージングしてコミットする。
    /// 変更がない場合は `Ok(None)` を返す。
    pub fn commit_all(&self, message: &str) -> Result<Option<String>, GitError> {
        if !self.has_changes()? {
            return Ok(None);
        }

        // すべての変更をステージングする
        let output = git_command(&self.path)
            .args(["add", "-A"])
            .output()
            .map_err(GitError::Command)?;

        if !output.status.success() {
            return Err(GitError::CommandFailed(
                "git -C <path> add -A".to_string(),
                String::from_utf8_lossy(&output.stderr).trim().to_string(),
            ));
        }

        // コミットする
        let output = git_command(&self.path)
            .args(["commit", "--allow-empty-message", "-m", message])
            .output()
            .map_err(GitError::Command)?;

        if !output.status.success() {
            return Err(GitError::CommandFailed(
                "git -C <path> commit -m <message>".to_string(),
                String::from_utf8_lossy(&output.stderr).trim().to_string(),
            ));
        }

        Ok(Some(self.head()?.0))
    }

    /// git を実行し, 成功したら標準出力を返す。
    fn run(&self, args: &[&str]) -> Result<String, GitError> {
        let output = git_command(&self.path)
            .args(args)
            .output()
            .map_err(GitError::Command)?;

        if !output.status.success() {
            return Err(GitError::CommandFailed(
                format!("git -C <path> {}", args.join(" ")),
                String::from_utf8_lossy(&output.stderr).trim().to_string(),
            ));
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    /// 同期先のリモート `origin` が設定されているか調べる。
    pub fn has_remote(&self) -> Result<bool, GitError> {
        let output = git_command(&self.path)
            .args(["remote", "get-url", REMOTE])
            .output()
            .map_err(GitError::Command)?;

        // リモートがないときの終了コードは 2
        match output.status.code() {
            Some(0) => Ok(true),
            Some(2) => Ok(false),
            _ => Err(GitError::CommandFailed(
                format!("git -C <path> remote get-url {REMOTE}"),
                String::from_utf8_lossy(&output.stderr).trim().to_string(),
            )),
        }
    }

    /// 現在のブランチを `origin` へ push する。
    ///
    /// `git remote add` の直後は upstream がないので毎回指定する。
    ///
    /// # Errors
    ///
    /// fast-forward でなければ `GitError::PushRejected` エラー。
    pub fn push(&self) -> Result<(), GitError> {
        let output = git_command(&self.path)
            .args(["push", "--porcelain", "--set-upstream", REMOTE, "HEAD"])
            .output()
            .map_err(GitError::Command)?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        if stdout.lines().any(|line| line.starts_with('!')) {
            return Err(GitError::PushRejected);
        }

        if !output.status.success() {
            return Err(GitError::CommandFailed(
                format!("git -C <path> push --porcelain --set-upstream {REMOTE} HEAD"),
                String::from_utf8_lossy(&output.stderr).trim().to_string(),
            ));
        }

        Ok(())
    }

    /// `origin` の現在のブランチを取り込み, その上に rebase する。
    ///
    /// # Errors
    ///
    /// 競合したら rebase 前に戻して `GitError::Conflict` エラー。
    pub fn pull_rebase(&self) -> Result<(), GitError> {
        let branch = self.run(&["symbolic-ref", "--short", "HEAD"])?;
        let error = match self.run(&["pull", "--rebase", REMOTE, &branch]) {
            Ok(_) => return Ok(()),
            Err(error) => error,
        };

        // git の文言は翻訳で変わるので, 未解決のファイルの有無で判定する
        if self.run(&["ls-files", "--unmerged"])?.is_empty() {
            return Err(error);
        }

        self.run(&["rebase", "--abort"])?;
        Err(GitError::Conflict)
    }

    /// HEAD の commit を返す。
    pub fn head(&self) -> Result<CommitId, GitError> {
        self.run(&["rev-parse", "HEAD"]).map(CommitId)
    }

    /// 作業ツリーごと指定の commit まで戻す。
    pub fn reset_hard(&self, commit: &CommitId) -> Result<(), GitError> {
        self.run(&["reset", "--hard", &commit.0]).map(drop)
    }
}

/// 帳簿の変更履歴
pub trait History {
    /// すべての変更を commit する.
    ///
    /// 変更がなければ何もしない.
    fn commit_all(&self, message: &str) -> Result<(), GitError>;

    /// 同期先があるか.
    fn has_remote(&self) -> Result<bool, GitError>;

    /// 同期先へ push する.
    fn push(&self) -> Result<(), GitError>;

    /// 同期先を取り込み, その上に rebase する.
    fn pull_rebase(&self) -> Result<(), GitError>;

    /// HEAD の commit を返す.
    fn head(&self) -> Result<CommitId, GitError>;

    /// 指定の commit まで戻す.
    fn reset_hard(&self, commit: &CommitId) -> Result<(), GitError>;
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

    fn head(&self) -> Result<CommitId, GitError> {
        GitRepo::head(self)
    }

    fn reset_hard(&self, commit: &CommitId) -> Result<(), GitError> {
        GitRepo::reset_hard(self, commit)
    }
}

/// Git操作中に発生するエラー。
#[derive(Debug, Error)]
pub enum GitError {
    #[error("gitを実行できません")]
    Command(#[source] std::io::Error),

    #[error("gitコマンドが失敗しました: {0}, stderr: {1}")]
    CommandFailed(String, String),

    #[error("git push が拒否されました")]
    PushRejected,

    #[error("rebase中に競合が発生しました")]
    Conflict,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    /// commit する利用者を設定したリポジトリを作る
    fn init_with_identity(dir: &Path) -> GitRepo {
        let repo = GitRepo::init(dir.to_path_buf()).unwrap();

        let output = git_command(dir)
            .args(["config", "user.name", "test"])
            .output()
            .unwrap();

        assert!(
            output.status.success(),
            "user.name設定に失敗: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let output = git_command(dir)
            .args(["config", "user.email", "test@example.com"])
            .output()
            .unwrap();

        assert!(
            output.status.success(),
            "user.email設定に失敗: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        repo
    }

    #[test]
    fn commit_all_creates_commit_and_returns_hash() {
        let dir = tempdir().unwrap();
        let repo = init_with_identity(dir.path());

        fs::write(dir.path().join("test.txt"), "hello").unwrap();

        let result = repo.commit_all("test commit").unwrap();

        assert!(result.is_some());

        let hash = result.unwrap();

        assert!(!hash.is_empty());
    }

    #[test]
    fn commit_all_accepts_empty_message() {
        let dir = tempdir().unwrap();
        let repo = init_with_identity(dir.path());

        fs::write(dir.path().join("test.txt"), "hello").unwrap();

        assert!(repo.commit_all("").unwrap().is_some());
        assert!(!repo.has_changes().unwrap());
    }

    #[test]
    fn commit_all_returns_none_when_no_changes() {
        let dir = tempdir().unwrap();

        let repo = GitRepo::init(dir.path().to_path_buf()).unwrap();

        let result = repo.commit_all("test commit").unwrap();

        assert_eq!(result, None);
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

    #[test]
    fn push_returns_rejected_when_remote_has_new_commit() {
        let root = tempdir().unwrap();

        let remote = root.path().join("remote.git");
        let a_dir = root.path().join("a");
        let b_dir = root.path().join("b");

        let output = Command::new("git")
            .args(["init", "--bare"])
            .arg(&remote)
            .output()
            .unwrap();

        assert!(output.status.success());

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

        let repo_a = GitRepo::open(a_dir.clone()).unwrap();
        let repo_b = GitRepo::open(b_dir.clone()).unwrap();

        // A が commit -> push
        fs::write(a_dir.join("a.txt"), "A").unwrap();
        repo_a.commit_all("A").unwrap();
        repo_a.push().unwrap();

        // B は A の変更を知らない
        fs::write(b_dir.join("b.txt"), "B").unwrap();
        repo_b.commit_all("B").unwrap();

        let result = repo_b.push();

        assert!(matches!(result, Err(GitError::PushRejected)));
    }

    #[test]
    fn pull_rebase_returns_conflict() {
        let root = tempdir().unwrap();

        let remote = root.path().join("remote.git");
        let a_dir = root.path().join("a");
        let b_dir = root.path().join("b");

        let output = Command::new("git")
            .args(["init", "--bare"])
            .arg(&remote)
            .output()
            .unwrap();
        assert!(output.status.success());

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

        let repo_a = GitRepo::open(a_dir.clone()).unwrap();
        let repo_b = GitRepo::open(b_dir.clone()).unwrap();

        // 初回コミット
        fs::write(a_dir.join("data.txt"), "hello\n").unwrap();
        repo_a.commit_all("initial").unwrap();
        repo_a.push().unwrap();

        repo_b.pull_rebase().unwrap();

        // 同じ行を別々に変更
        fs::write(a_dir.join("data.txt"), "alice\n").unwrap();
        repo_a.commit_all("alice").unwrap();
        repo_a.push().unwrap();

        fs::write(b_dir.join("data.txt"), "bob\n").unwrap();
        repo_b.commit_all("bob").unwrap();

        let result = repo_b.pull_rebase();

        assert!(matches!(result, Err(GitError::Conflict)));

        assert!(!b_dir.join(".git/rebase-merge").exists());
        assert!(!b_dir.join(".git/rebase-apply").exists());
    }

    #[test]
    fn has_remote_returns_true() {
        let root = tempdir().unwrap();

        let remote = root.path().join("remote.git");
        let local = root.path().join("local");

        let output = Command::new("git")
            .args(["init", "--bare"])
            .arg(&remote)
            .output()
            .unwrap();

        assert!(output.status.success());

        let output = Command::new("git")
            .arg("clone")
            .arg(&remote)
            .arg(&local)
            .output()
            .unwrap();

        assert!(output.status.success());

        let repo = GitRepo::open(local).unwrap();

        assert!(repo.has_remote().unwrap());
    }

    /// bare リポジトリを `origin` として登録した, 初回 commit だけのリポジトリを作る
    fn init_with_origin(root: &Path) -> GitRepo {
        let remote = root.join("remote.git");
        let local = root.join("local");
        let output = Command::new("git")
            .args(["init", "--bare"])
            .arg(&remote)
            .output()
            .unwrap();
        assert!(output.status.success());

        fs::create_dir(&local).unwrap();
        let repo = init_with_identity(&local);
        run_git(&local, &["remote", "add", "origin", "../remote.git"]);
        fs::write(local.join("data.txt"), "hello").unwrap();
        repo.commit_all("initial").unwrap();
        repo
    }

    #[test]
    fn push_sets_upstream_regardless_of_user_config() {
        let root = tempdir().unwrap();
        let repo = init_with_origin(root.path());
        run_git(repo.path(), &["config", "push.autoSetupRemote", "false"]);

        repo.push().unwrap();

        let branch = repo.run(&["symbolic-ref", "--short", "HEAD"]).unwrap();
        assert_eq!(
            repo.run(&["rev-parse", "--abbrev-ref", "@{upstream}"])
                .unwrap(),
            format!("origin/{branch}")
        );
    }

    #[test]
    fn push_returns_command_failed_when_remote_is_unreachable() {
        let root = tempdir().unwrap();
        let repo = init_with_origin(root.path());
        run_git(repo.path(), &["remote", "set-url", "origin", "../missing"]);

        assert!(matches!(
            repo.push(),
            Err(GitError::CommandFailed(command, _))
                if command == "git -C <path> push --porcelain --set-upstream origin HEAD"
        ));
    }

    #[test]
    fn pull_rebase_works_without_upstream() {
        let root = tempdir().unwrap();
        let repo = init_with_origin(root.path());
        repo.push().unwrap();
        let other = root.path().join("other");
        fs::create_dir(&other).unwrap();
        let other_repo = init_with_identity(&other);
        run_git(&other, &["remote", "add", "origin", "../remote.git"]);
        // 取り込む先のブランチ名を揃える
        let branch = repo.run(&["symbolic-ref", "--short", "HEAD"]).unwrap();
        run_git(
            &other,
            &["symbolic-ref", "HEAD", &format!("refs/heads/{branch}")],
        );
        fs::write(other.join("other.txt"), "other").unwrap();
        other_repo.commit_all("other").unwrap();

        other_repo.pull_rebase().unwrap();

        assert!(other.join("data.txt").exists());
        assert!(other.join("other.txt").exists());
    }

    #[test]
    fn pull_rebase_returns_command_failed_when_remote_is_unreachable() {
        let root = tempdir().unwrap();
        let repo = init_with_origin(root.path());
        run_git(repo.path(), &["remote", "set-url", "origin", "../missing"]);

        assert!(matches!(
            repo.pull_rebase(),
            Err(GitError::CommandFailed(command, _)) if command.starts_with("git -C <path> pull")
        ));
    }

    #[test]
    fn reset_hard_returns_to_commit() {
        let dir = tempdir().unwrap();
        let repo = init_with_identity(dir.path());
        let file = dir.path().join("test.txt");
        fs::write(&file, "before").unwrap();
        repo.commit_all("before").unwrap();
        let head = repo.head().unwrap();
        fs::write(&file, "after").unwrap();
        repo.commit_all("after").unwrap();
        assert_ne!(repo.head().unwrap(), head);

        repo.reset_hard(&head).unwrap();

        assert_eq!(repo.head().unwrap(), head);
        assert_eq!(fs::read_to_string(file).unwrap(), "before");
    }

    #[test]
    fn reset_hard_returns_command_failed_for_unknown_commit() {
        let dir = tempdir().unwrap();
        let repo = init_with_identity(dir.path());

        assert!(matches!(
            repo.reset_hard(&CommitId("missing".to_string())),
            Err(GitError::CommandFailed(command, _))
                if command == "git -C <path> reset --hard missing"
        ));
    }

    #[test]
    fn has_remote_ignores_remote_other_than_origin() {
        let root = tempdir().unwrap();
        let dir = root.path().join("local");
        fs::create_dir(&dir).unwrap();
        let repo = init_with_identity(&dir);
        run_git(&dir, &["remote", "add", "share", "../remote.git"]);

        assert!(!repo.has_remote().unwrap());
    }

    #[test]
    fn has_remote_returns_command_failed_outside_repository() {
        let dir = tempdir().unwrap();
        let repo = GitRepo {
            path: dir.path().to_path_buf(),
        };

        assert!(matches!(
            repo.has_remote(),
            Err(GitError::CommandFailed(command, _))
                if command == "git -C <path> remote get-url origin"
        ));
    }

    #[test]
    fn has_remote_returns_false_without_remote() {
        let dir = tempdir().unwrap();

        let repo = init_with_identity(dir.path());

        assert!(!repo.has_remote().unwrap());
    }
}

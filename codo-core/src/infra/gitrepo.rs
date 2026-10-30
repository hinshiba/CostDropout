//! Gitリポジトリの操作と帳簿の変更履歴

pub mod audit;

use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use thiserror::Error;

/// Gitコマンドを生成する。
/// 指定したパスを使い、リポジトリを切り替える環境変数を除去する。
pub(crate) fn git_command(path: &Path) -> Command {
    let mut command = Command::new("git");

    command
        .args(["-C"])
        .arg(path)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE");

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

/// Gitリポジトリを操作するためのラッパー。
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

        // コミットハッシュを取得する
        let output = git_command(&self.path)
            .args(["rev-parse", "HEAD"])
            .output()
            .map_err(GitError::Command)?;

        if !output.status.success() {
            return Err(GitError::CommandFailed(
                "git -C <path> rev-parse HEAD".to_string(),
                String::from_utf8_lossy(&output.stderr).trim().to_string(),
            ));
        }

        let hash = String::from_utf8_lossy(&output.stdout).trim().to_string();

        Ok(Some(hash))
    }

    //Gitリポジトリにremoteなどが設定されているか調べる
    pub fn has_remote(&self) -> Result<bool, GitError> {
        let output = git_command(&self.path)
            .arg("remote")
            .output()
            .map_err(GitError::Command)?;

        if !output.status.success() {
            return Err(GitError::CommandFailed(
                "git -C <path> remote".to_string(),
                String::from_utf8_lossy(&output.stderr).trim().to_string(),
            ));
        }

        Ok(!String::from_utf8_lossy(&output.stdout).trim().is_empty())
    }

    //git pushを実行して成功・失敗．その他の失敗を判定
    pub fn push(&self) -> Result<(), GitError> {
        let output = git_command(&self.path)
            .args(["push", "--porcelain"])
            .output()
            .map_err(GitError::Command)?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        if stdout.lines().any(|line| line.starts_with('!')) {
            return Err(GitError::PushRejected);
        }

        if !output.status.success() {
            return Err(GitError::CommandFailed(
                "git -C <path> push --porcelain".to_string(),
                String::from_utf8_lossy(&output.stderr).trim().to_string(),
            ));
        }

        Ok(())
    }

    //git pull --rebaseを実行して成功・コンフリクト・その他失敗を判定
    pub fn pull_rebase(&self) -> Result<(), GitError> {
        let output = git_command(&self.path)
            .args(["pull", "--rebase"])
            .output()
            .map_err(GitError::Command)?;

        if output.status.success() {
            return Ok(());
        }

        let stderr = String::from_utf8_lossy(&output.stderr);

        if stderr.contains("CONFLICT") || stderr.contains("could not apply") {
            let _ = git_command(&self.path).args(["rebase", "--abort"]).output();

            return Err(GitError::Conflict);
        }

        Err(GitError::CommandFailed(
            "git -C <path> pull --rebase".to_string(),
            stderr.trim().to_string(),
        ))
    }
}

/// 帳簿の変更履歴
pub trait History {
    /// すべての変更を commit する.
    ///
    /// 変更がなければ何もしない.
    fn commit_all(&self, message: &str) -> Result<(), GitError>;
}

impl History for GitRepo {
    fn commit_all(&self, message: &str) -> Result<(), GitError> {
        GitRepo::commit_all(self, message).map(drop)
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

    #[test]
    fn has_remote_returns_false_without_remote() {
        let dir = tempdir().unwrap();

        let repo = init_with_identity(dir.path());

        assert!(!repo.has_remote().unwrap());
    }
}

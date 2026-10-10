use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use thiserror::Error;

/// Gitコマンドを生成する。
/// 指定したパスを使い、リポジトリを切り替える環境変数を除去する。
fn git_command(path: &Path) -> Command {
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

    Ok(output.stdout.is_empty())
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
        let output = Command::new("git")
            .args(["-C"])
            .arg(&self.path)
            .args(["commit", "-m", message])
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
}

/// Git操作中に発生するエラー。
#[derive(Debug, Error)]
pub enum GitError {
    #[error("gitを実行できません")]
    Command(#[source] std::io::Error),

    #[error("gitコマンドが失敗しました: {0}, stderr: {1}")]
    CommandFailed(String, String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn commit_all_creates_commit_and_returns_hash() {
        let dir = tempdir().unwrap();

        let repo = GitRepo::init(dir.path().to_path_buf()).unwrap();

        let output = git_command(dir.path())
            .args(["config", "user.name", "test"])
            .output()
            .unwrap();

        assert!(
            output.status.success(),
            "user.name設定に失敗: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let output = git_command(dir.path())
            .args(["config", "user.email", "test@example.com"])
            .output()
            .unwrap();

        assert!(
            output.status.success(),
            "user.email設定に失敗: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        fs::write(dir.path().join("test.txt"), "hello").unwrap();

        let result = repo.commit_all("test commit").unwrap();

        assert!(result.is_some());

        let hash = result.unwrap();

        assert!(!hash.is_empty());
    }

    #[test]
    fn commit_all_returns_none_when_no_changes() {
        let dir = tempdir().unwrap();

        let repo = GitRepo::init(dir.path().to_path_buf()).unwrap();

        let result = repo.commit_all("test commit").unwrap();

        assert_eq!(result, None);
    }
}

use std::path::PathBuf;
use std::process::Command;
use thiserror::Error;

pub struct GitRepo {
    path: PathBuf,
}

impl GitRepo {
    pub fn open(path: PathBuf) -> Result<GitRepo, GitError> {
        let output = Command::new("git")
            .args(["-C", path.to_str().unwrap(), "status"])
            .output()
            .map_err(GitError::Command)?;

        if !output.status.success() {
            return Err(GitError::CommandFailed(
                "git -C <path> status".to_string(),
                String::from_utf8_lossy(&output.stderr).trim().to_string(),
            ));
        }
        Ok(GitRepo { path })
    }

    pub fn init(path: PathBuf) -> Result<GitRepo, GitError> {
        if Self::open(path.clone()).is_ok() {
            return Ok(GitRepo { path });
        }
        let output = Command::new("git")
            .args(["-C", path.to_str().unwrap(), "init"])
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

    pub fn has_changes(&self) -> Result<bool, GitError> {
        let output = Command::new("git")
            .args(["-C"])
            .arg(&self.path)
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

    pub fn commit_all(&self, message: &str) -> Result<Option<String>, GitError> {
        if !self.has_changes()? {
            return Ok(None);
        }

        let output = Command::new("git")
            .args(["-C"])
            .arg(&self.path)
            .args(["add", "-A"])
            .output()
            .map_err(GitError::Command)?;

        if !output.status.success() {
            return Err(GitError::CommandFailed(
                "git -C <path> add -A".to_string(),
                String::from_utf8_lossy(&output.stderr).trim().to_string(),
            ));
        }

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

        let output = Command::new("git")
            .args(["-C"])
            .arg(&self.path)
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

#[derive(Debug, Error)]
pub enum GitError {
    #[error("gitを実行できません: {0}")]
    Command(std::io::Error),
    #[error("gitコマンドが失敗しました: {0}, stderr: {1}")]
    CommandFailed(String, String),
}

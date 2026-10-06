use std::path::PathBuf;
use std::process::Command;

pub struct GitRepo {
    path: PathBuf,
}

impl GitRepo {
    fn open(path: PathBuf) -> Result<GitRepo, GitError> {}

    fn init(path: PathBuf) -> Result<GitRepo, GitError> {}

    fn has_changes(&self) -> Result<bool, GitError> {}

    fn commit_all(&self, message: &str) -> Result<Option<String>, GitError> {}
}

use crate::git::{GitRepo, git_command};
use chrono::{DateTime, FixedOffset};
use thiserror::Error;

/// Gitのコミット情報を保持する
pub struct CommitInfo {
    /// コミットのハッシュ値
    pub hash: String,
    /// コミットを作成したユーザー名
    pub author: String,
    /// コミット日時とタイムゾーン
    pub date: DateTime<FixedOffset>,
    /// コミットメッセージ
    pub message: String,
}

///gitにハッシュ・作者・日時・メッセージを出力させる.
pub fn log(repo: &GitRepo, limit: usize) -> Result<Vec<CommitInfo>, AuditError> {
    let output = git_command(repo.get_pathbuf())
        .arg("log")
        .arg("--pretty=tformat:%H%x00%an%x00%aI%x00%B%x00")
        .arg("-n")
        .arg(limit.to_string())
        .output()
        .map_err(AuditError::Command)?;

    if !output.status.success() {
        return Err(AuditError::CommandFailed(
            "git log -n".to_string(),
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let mut fields = text.split('\0');
    let mut logs = Vec::new();

    while let Some(hash) = fields.next() {
        let hash = hash.trim();
        if hash.is_empty() {
            continue;
        }
        let author = fields.next().ok_or(AuditError::OutputDataFailed)?;
        let date_text = fields.next().ok_or(AuditError::OutputDataFailed)?;
        let date = DateTime::parse_from_rfc3339(date_text).map_err(AuditError::DateTimeFailed)?;
        let message = fields.next().ok_or(AuditError::OutputDataFailed)?;

        logs.push(CommitInfo {
            hash: hash.to_string(),
            author: author.to_string(),
            date,
            message: message.trim_end_matches('\n').to_string(),
        });
    }

    Ok(logs)
}

/// 指定したコミットで変更されたファイル名の一覧を取得する。
pub fn changed_files(repo: &GitRepo, hash: &str) -> Result<Vec<String>, AuditError> {
    let output = git_command(repo.get_pathbuf())
        .args([
            "diff-tree",
            "--root",
            "--no-commit-id",
            "--name-only",
            "-r",
            "-z",
        ])
        .arg(hash)
        .output()
        .map_err(AuditError::Command)?;

    if !output.status.success() {
        return Err(AuditError::CommandFailed(
            format!(
                "git diff-tree --root --no-commit-id --name-only -r -z {}",
                hash
            ),
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }

    let text = String::from_utf8_lossy(&output.stdout);

    let files: Vec<String> = text
        .split('\0')
        .filter(|file_name| !file_name.is_empty())
        .map(|file| file.to_string())
        .collect();

    Ok(files)
}

/// 指定したコミットの差分を取得する
pub fn show_diff(repo: &GitRepo, hash: &str) -> Result<String, AuditError> {
    let output = git_command(repo.get_pathbuf())
        .args(["show", "--format="])
        .arg(hash)
        .output()
        .map_err(AuditError::Command)?;

    if !output.status.success() {
        return Err(AuditError::CommandFailed(
            format!("git show --format= {}", hash),
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

#[derive(Debug, Error)]
pub enum AuditError {
    /// Gitコマンドの起動に失敗した場合
    #[error("gitを実行できません")]
    Command(#[source] std::io::Error),

    /// Gitコマンドが失敗した場合
    #[error("gitコマンドが失敗しました: {0}, stderr: {1}")]
    CommandFailed(String, String),

    /// Gitコマンドの出力データが不足している場合
    #[error("出力データが不足しています")]
    OutputDataFailed,

    /// コミット日時の解析に失敗した場合
    #[error("日時の解析に失敗しました")]
    DateTimeFailed(#[source] chrono::ParseError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn show_diff_returns_added_content() {
        let dir = tempdir().unwrap();
        let repo = GitRepo::init(dir.path().to_path_buf()).unwrap();

        for (key, value) in [("user.name", "test"), ("user.email", "test@example.com")] {
            let output = git_command(dir.path())
                .args(["config", key, value])
                .output()
                .unwrap();

            assert!(output.status.success());
        }
        fs::write(dir.path().join("hello.txt"), "Hello, Rust!\n").unwrap();

        let hash = repo
            .commit_all("add hello.txt")
            .unwrap()
            .expect("コミットが作成されるはずです");

        let diff = show_diff(&repo, &hash).unwrap();

        assert!(diff.contains("hello.txt"));
        assert!(diff.contains("+Hello, Rust!"));
    }

    #[test]
    fn changed_files_returns_changed_file_names() {
        let dir = tempdir().unwrap();
        let repo = GitRepo::init(dir.path().to_path_buf()).unwrap();

        for (key, value) in [("user.name", "test"), ("user.email", "test@example.com")] {
            let output = git_command(dir.path())
                .args(["config", key, value])
                .output()
                .unwrap();

            assert!(output.status.success());
        }
        fs::write(dir.path().join("hello.txt"), "Hello, Rust!\n").unwrap();
        fs::write(dir.path().join("memo.txt"), "Hello, Rust!\n").unwrap();

        let hash = repo
            .commit_all("feat:add two files")
            .unwrap()
            .expect("コミットが作成されるはず");

        let files = changed_files(&repo, &hash).unwrap();

        assert_eq!(files.len(), 2);
        assert!(files.iter().any(|file| file == "hello.txt"));
        assert!(files.iter().any(|file| file == "memo.txt"));
    }

    #[test]
    fn log_returns_commits_in_newest_first_order() {
        let (_dir, repo) = setup_repo();

        // 1回目のコミット
        fs::write(repo.get_pathbuf().join("hello.txt"), "Hello, Rust!\n").unwrap();

        let hash1 = repo
            .commit_all("first commit")
            .unwrap()
            .expect("1回目のコミットが作成されるはず");

        // 2回目のコミットでは作者を変更する
        let output = git_command(repo.get_pathbuf())
            .args(["config", "user.name", "second-author"])
            .output()
            .unwrap();
        assert!(output.status.success());

        let output = git_command(repo.get_pathbuf())
            .args(["config", "user.email", "second@example.com"])
            .output()
            .unwrap();
        assert!(output.status.success());

        fs::write(repo.get_pathbuf().join("memo.txt"), "Second commit\n").unwrap();

        let hash2 = repo
            .commit_all("second commit")
            .unwrap()
            .expect("2回目のコミットが作成されるはず");

        // 履歴を取得する
        let logs = log(&repo, 2).unwrap();

        assert_eq!(logs.len(), 2);

        // 新しいコミットが先頭
        assert_eq!(logs[0].hash, hash2);
        assert_eq!(logs[1].hash, hash1);

        // 作者とメッセージ
        assert_eq!(logs[0].author, "second-author");
        assert_eq!(logs[0].message, "second commit");

        assert_eq!(logs[1].author, "test");
        assert_eq!(logs[1].message, "first commit");

        // limit による件数制限
        let limited_logs = log(&repo, 1).unwrap();
        assert_eq!(limited_logs.len(), 1);
        assert_eq!(limited_logs[0].hash, hash2);
    }

    fn setup_repo() -> (tempfile::TempDir, GitRepo) {
        let dir = tempdir().unwrap();
        let repo = GitRepo::init(dir.path().to_path_buf()).unwrap();

        for (key, value) in [("user.name", "test"), ("user.email", "test@example.com")] {
            let output = git_command(dir.path())
                .args(["config", key, value])
                .output()
                .unwrap();

            assert!(output.status.success());
        }

        (dir, repo)
    }

    #[test]
    fn log_preserves_multiline_message_and_pipe() {
        let (_dir, repo) = setup_repo();

        let message = "feat: 集計 | 初版\n\n月次集計を実装\nエラー処理も追加";

        fs::write(repo.get_pathbuf().join("report.txt"), "aggregation\n").unwrap();

        repo.commit_all(message)
            .unwrap()
            .expect("コミットが作成されるはず");

        let logs = log(&repo, 1).unwrap();

        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].message, message);
    }

    #[test]
    fn changed_files_returns_files_for_each_commit() {
        let (_dir, repo) = setup_repo();

        // 1回目：hello.txt を追加
        fs::write(repo.get_pathbuf().join("hello.txt"), "Hello\n").unwrap();

        let hash1 = repo
            .commit_all("add hello.txt")
            .unwrap()
            .expect("1回目のコミットが作成されるはず");

        // 2回目：memo.txt を追加
        fs::write(repo.get_pathbuf().join("memo.txt"), "Memo\n").unwrap();

        let hash2 = repo
            .commit_all("add memo.txt")
            .unwrap()
            .expect("2回目のコミットが作成されるはず");

        let files1 = changed_files(&repo, &hash1).unwrap();
        let files2 = changed_files(&repo, &hash2).unwrap();

        assert_eq!(files1, vec!["hello.txt"]);
        assert_eq!(files2, vec!["memo.txt"]);
    }
}

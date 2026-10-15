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
        let stderr = String::from_utf8_lossy(&output.stderr);

        if stderr.contains("does not have any commits yet") {
            return Ok(Vec::new());
        }

        return Err(AuditError::CommandFailed(
            "git log -n".to_string(),
            stderr.trim().to_string(),
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
        let date = DateTime::parse_from_rfc3339(date_text).map_err(|source| {
            AuditError::DateTimeFailed {
                date_text: date_text.to_string(),
                source,
            }
        })?;
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

/// 指定したコミットの第1親を取得する
fn first_parent(repo: &GitRepo, hash: &str) -> Result<Option<String>, AuditError> {
    let output = git_command(repo.get_pathbuf())
        .args(["rev-list", "--parents", "-n", "1"])
        .arg(hash)
        .output()
        .map_err(AuditError::Command)?;

    if !output.status.success() {
        return Err(AuditError::CommandFailed(
            format!("git rev-list --parents -n 1 {}", hash),
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let mut hashes = text.split_whitespace();

    hashes.next().ok_or(AuditError::OutputDataFailed)?;

    Ok(hashes.next().map(str::to_string))
}

/// 指定したコミットで変更されたファイル名の一覧を取得する。
pub fn changed_files(repo: &GitRepo, hash: &str) -> Result<Vec<String>, AuditError> {
    let output = match first_parent(repo, hash)? {
        Some(parent) => git_command(repo.get_pathbuf())
            .args(["diff", "--name-only", "-z"])
            .arg(&parent)
            .arg(hash)
            .output()
            .map_err(AuditError::Command)?,
        None => git_command(repo.get_pathbuf())
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
            .map_err(AuditError::Command)?,
    };

    if !output.status.success() {
        return Err(AuditError::CommandFailed(
            format!("git diff --name-only -z {}", hash),
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }

    let text = String::from_utf8_lossy(&output.stdout);

    let files: Vec<String> = text
        .split('\0')
        .filter(|file_name| !file_name.is_empty())
        .map(str::to_string)
        .collect();

    Ok(files)
}

/// 指定したコミットの差分を取得する
pub fn show_diff(repo: &GitRepo, hash: &str) -> Result<String, AuditError> {
    let output = match first_parent(repo, hash)? {
        Some(parent) => git_command(repo.get_pathbuf())
            .arg("diff")
            .arg(&parent)
            .arg(hash)
            .output()
            .map_err(AuditError::Command)?,
        None => git_command(repo.get_pathbuf())
            .args(["show", "--root", "--format="])
            .arg(hash)
            .output()
            .map_err(AuditError::Command)?,
    };

    if !output.status.success() {
        return Err(AuditError::CommandFailed(
            format!("git diff/show {}", hash),
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
    #[error("日時の解析に失敗しました: {date_text}")]
    DateTimeFailed {
        date_text: String,
        #[source]
        source: chrono::ParseError,
    },
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

    #[test]
    fn merge_commit_uses_first_parent_for_audit() {
        let (_dir, repo) = setup_repo();

        // 最初のコミットを作る
        fs::write(repo.get_pathbuf().join("base.txt"), "base\n").unwrap();

        repo.commit_all("base commit")
            .unwrap()
            .expect("最初のコミットが作成されるはず");

        let output = git_command(repo.get_pathbuf())
            .args(["symbolic-ref", "--short", "HEAD"])
            .output()
            .unwrap();

        assert!(output.status.success());

        let base_branch = String::from_utf8(output.stdout).unwrap().trim().to_string();

        // feature ブランチを作り、feature.txt を追加
        let output = git_command(repo.get_pathbuf())
            .args(["checkout", "-b", "feature"])
            .output()
            .unwrap();

        assert!(output.status.success());

        fs::write(repo.get_pathbuf().join("feature.txt"), "feature\n").unwrap();

        repo.commit_all("feature commit")
            .unwrap()
            .expect("feature のコミットが作成されるはず");

        // 元のブランチに戻り、main.txt を追加
        let output = git_command(repo.get_pathbuf())
            .args(["checkout", &base_branch])
            .output()
            .unwrap();

        assert!(output.status.success());

        fs::write(repo.get_pathbuf().join("main.txt"), "main\n").unwrap();

        repo.commit_all("main commit")
            .unwrap()
            .expect("main のコミットが作成されるはず");

        // feature をマージしてマージコミットを作る
        let output = git_command(repo.get_pathbuf())
            .args(["merge", "--no-ff", "feature", "-m", "merge feature"])
            .output()
            .unwrap();

        assert!(
            output.status.success(),
            "merge failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        // マージコミットのハッシュを取得する
        let output = git_command(repo.get_pathbuf())
            .args(["rev-parse", "HEAD"])
            .output()
            .unwrap();

        assert!(output.status.success());

        let merge_hash = String::from_utf8(output.stdout).unwrap().trim().to_string();

        // 第1親との差分には feature.txt が含まれる
        let files = changed_files(&repo, &merge_hash).unwrap();

        assert_eq!(files, vec!["feature.txt"]);

        // 差分の内容も同じ比較方針になっている
        let diff = show_diff(&repo, &merge_hash).unwrap();

        assert!(diff.contains("feature.txt"));
        assert!(diff.contains("+feature"));
        assert!(!diff.contains("main.txt"));
    }

    #[test]
    fn log_with_zero_limit_returns_empty_vec() {
        let (_dir, repo) = setup_repo();

        fs::write(repo.get_pathbuf().join("hello.txt"), "Hello\n").unwrap();

        repo.commit_all("first commit")
            .unwrap()
            .expect("コミットが作成されるはず");

        let logs = log(&repo, 0).unwrap();

        assert!(logs.is_empty());
    }

    #[test]
    fn log_on_empty_repository_returns_empty_vec() {
        let (_dir, repo) = setup_repo();

        let logs = log(&repo, 10).unwrap();

        assert!(logs.is_empty());
    }

    #[test]
    fn changed_files_returns_error_for_invalid_hash() {
        let (_dir, repo) = setup_repo();

        let result = changed_files(&repo, "invalid-hash");

        assert!(result.is_err());
    }

    #[test]
    fn show_diff_returns_error_for_invalid_hash() {
        let (_dir, repo) = setup_repo();

        let result = show_diff(&repo, "invalid-hash");

        assert!(result.is_err());
    }
}

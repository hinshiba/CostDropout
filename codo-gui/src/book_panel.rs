//! サイドバー下部の保存と同期の操作.

use chrono::{Local, NaiveDateTime};
use codo_core::book::{BookError, SyncOutcome};
use codo_core::infra::gitrepo::GitError;
use gpui_kit::component::button::Button;
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::component::notification::Notification;
use gpui_kit::component::{Disableable, WindowExt};
use gpui_kit::*;

use crate::app::AppState;

/// 空白だけの入力を日時入りの既定メッセージに置き換える.
fn commit_message(input: &str, now: NaiveDateTime) -> String {
    let message = input.trim();
    if message.is_empty() {
        return now.format("%Y-%m-%d %H:%M の記録").to_string();
    }
    message.to_string()
}

/// 未保存なら確認を出して閉じるのを止める.
fn confirm_close(state: &Entity<AppState>, window: &mut Window, cx: &mut App) -> bool {
    if !state.read(cx).is_dirty() {
        return true;
    }
    if !window.has_active_dialog(cx) {
        window.open_alert_dialog(cx, |alert, _, _| {
            alert
                .confirm()
                .title("未保存の変更があります")
                .description("保存せずに閉じると、変更は失われます。")
                .ok_text("保存せず閉じる")
                .cancel_text("キャンセル")
                .on_ok(|_, window, _| {
                    window.remove_window();
                    true
                })
        });
    }
    false
}

/// 閉じるボタンしかない案内のダイアログを開く.
fn open_guide(title: &'static str, description: String, window: &mut Window, cx: &mut App) {
    window.open_alert_dialog(cx, move |alert, _, _| {
        alert.title(title).description(description.clone())
    });
}

/// 編集を止めるため, 利用者が閉じられないダイアログを開く.
fn open_blocking(title: &'static str, description: String, window: &mut Window, cx: &mut App) {
    window.open_dialog(cx, move |dialog, _, _| {
        dialog
            .title(title)
            .child(description.clone())
            .keyboard(false)
            .close_button(false)
            .overlay_closable(false)
    });
}

/// 帳簿の保存と同期を操作する.
pub struct BookPanel {
    state: Entity<AppState>,
    message: Entity<InputState>,
    /// 読み直しに失敗した場合は, 開き直すまで立ったままにする
    is_syncing: bool,
    _subscription: Subscription,
}

impl BookPanel {
    /// 未保存のまま閉じるときの確認も登録する.
    pub fn new(state: Entity<AppState>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let subscription = cx.observe(&state, |_, _, cx| cx.notify());
        window.on_window_should_close(cx, {
            let state = state.clone();
            move |window, cx| confirm_close(&state, window, cx)
        });
        Self {
            state,
            message: cx.new(|cx| InputState::new(window, cx).placeholder("commit メッセージ")),
            is_syncing: false,
            _subscription: subscription,
        }
    }

    /// 帳簿を保存して結果を通知する. 失敗したら入力を残す.
    fn save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let message = commit_message(&self.message.read(cx).value(), Local::now().naive_local());
        let result = self.state.update(cx, |state, cx| {
            let result = state.save(&message);
            cx.notify();
            result
        });
        match result {
            Ok(()) => {
                self.message
                    .update(cx, |input, cx| input.set_value("", window, cx));
                window.push_notification(Notification::success("保存しました。"), cx);
            }
            Err(error) => window.push_notification(
                Notification::error(format!("保存できませんでした: {error}")),
                cx,
            ),
        }
    }

    /// git をバックグラウンドで動かし, 終わるまで編集を止める.
    fn sync(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let job = match self.state.read(cx).sync_job() {
            Ok(job) => job,
            Err(error) => {
                window.push_notification(
                    Notification::warning(format!("{error}。先に保存してください。")),
                    cx,
                );
                return;
            }
        };
        self.is_syncing = true;
        open_blocking(
            "同期中",
            "同期が終わるまでお待ちください。".to_string(),
            window,
            cx,
        );
        cx.spawn_in(window, async move |this, cx| {
            let result = cx.background_spawn(async move { job.run() }).await;
            // ウィンドウを閉じた後なら表示する先がない
            let _ = this.update_in(cx, |this, window, cx| {
                this.finish_sync(result, window, cx);
            });
        })
        .detach();
        cx.notify();
    }

    /// 帳簿を読み直して同期の結果を表示する.
    fn finish_sync(
        &mut self,
        result: Result<SyncOutcome, BookError>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.close_dialog(cx);
        let reloaded = self.state.update(cx, |state, cx| {
            let reloaded = state.reload();
            cx.notify();
            reloaded
        });
        if let Err(error) = reloaded {
            // 古い帳簿のまま保存すると, 取り込んだ変更を上書きしてしまう
            open_blocking(
                "帳簿を読み直せませんでした",
                format!("アプリを開き直してください。\n{error}"),
                window,
                cx,
            );
            return;
        }
        self.is_syncing = false;
        cx.notify();
        match result {
            Ok(SyncOutcome::Pushed) => {
                window.push_notification(Notification::success("同期しました。"), cx);
            }
            Ok(SyncOutcome::PulledAndPushed) => {
                window.push_notification(
                    Notification::success("他の変更を取り込んで同期しました。"),
                    cx,
                );
            }
            Ok(SyncOutcome::NoRemote) => open_guide(
                "同期先が設定されていません",
                "帳簿フォルダで次のコマンドを実行してください。\ngit remote add origin <URL>"
                    .to_string(),
                window,
                cx,
            ),
            Err(BookError::History(GitError::Conflict)) => open_guide(
                "競合を自動で解決できませんでした",
                "帳簿フォルダで手動で解決してください。\n\
                 1. git pull --rebase origin <現在のブランチ> を実行する\n\
                 2. 競合を直して git add し, git rebase --continue を実行する\n\
                 3. アプリに戻り, 編集せずに同期ボタンを押す"
                    .to_string(),
                window,
                cx,
            ),
            Err(BookError::InvalidMerge(error)) => open_guide(
                "取り込むと帳簿が不正になります",
                format!(
                    "取り込む前の状態に戻しました。他の人と調整して帳簿を直してください。\n{error}"
                ),
                window,
                cx,
            ),
            Err(error) => open_guide("同期できませんでした", error.to_string(), window, cx),
        }
    }
}

impl Render for BookPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_dirty = self.state.read(cx).is_dirty();
        let status: SharedString = if is_dirty {
            "未保存の変更があります"
        } else {
            "保存済み"
        }
        .into();
        div()
            .w_full()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .id("book-status")
                    .test_support()
                    .aria_label(status.clone())
                    .child(status),
            )
            .child(
                Button::new("save-book")
                    .label("保存")
                    .disabled(!is_dirty || self.is_syncing)
                    .on_click(cx.listener(|this, _, window, cx| this.save(window, cx))),
            )
            .child(Input::new(&self.message))
            .child(
                Button::new("sync-book")
                    .label("同期")
                    .disabled(self.is_syncing)
                    .on_click(cx.listener(|this, _, window, cx| this.sync(window, cx))),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::{BookPanel, commit_message};
    use crate::app::App;
    use chrono::NaiveDate;
    use codo_core::{
        book::Book,
        infra::{filestore::TomlStore, gitrepo::GitRepo},
        ledger::{
            Ledger,
            item::{ItemId, Tag},
            money::Money,
            transaction::{Amount, TransactionDraft},
        },
    };
    use gpui_kit::component::WindowExt;
    use gpui_kit::test::TestWindowExt;
    use gpui_kit::{
        AnyWindowHandle, AppContext, Bounds, Entity, TestAppContext, VisualTestContext,
        WindowBounds, WindowOptions, px, size,
    };
    use std::path::{Path, PathBuf};
    use std::process::Command;

    fn git(directory: &Path, args: &[&str]) -> String {
        let output = Command::new("git")
            .arg("-C")
            .arg(directory)
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }

    fn set_identity(directory: &Path) {
        git(directory, &["config", "user.name", "test"]);
        git(directory, &["config", "user.email", "test@example.com"]);
    }

    fn commit_count(directory: &Path) -> usize {
        git(directory, &["rev-list", "--count", "HEAD"])
            .parse()
            .unwrap()
    }

    /// 品目と相手先だけの帳簿を commit したリポジトリを作る.
    fn init_book(directory: &Path) -> (ItemId, ItemId) {
        let mut ledger = Ledger::new();
        let food = ledger.add_item("食費", Tag::Expense).unwrap();
        let cash = ledger.add_item("現金", Tag::Asset).unwrap();
        let repository = GitRepo::init(directory.to_path_buf()).unwrap();
        set_identity(directory);
        let store = TomlStore::new(directory.to_path_buf());
        codo_core::infra::filestore::LedgerStore::save(&store, &ledger).unwrap();
        repository.commit_all("帳簿を作成").unwrap();
        (food, cash)
    }

    /// bare リポジトリへ push した帳簿の clone を2つ作る.
    fn clones(root: &Path) -> (PathBuf, PathBuf, ItemId, ItemId) {
        let remote = root.join("remote.git");
        let origin = root.join("origin");
        std::fs::create_dir(&origin).unwrap();
        let (food, cash) = init_book(&origin);
        git(root, &["init", "--bare", "remote.git"]);
        git(&origin, &["remote", "add", "origin", "../remote.git"]);
        GitRepo::open(origin).unwrap().push().unwrap();
        let [a, b] = ["a", "b"].map(|name| root.join(name));
        for directory in [&a, &b] {
            git(
                root,
                &[
                    "clone",
                    remote.to_str().unwrap(),
                    directory.to_str().unwrap(),
                ],
            );
            set_identity(directory);
        }
        (a, b, food, cash)
    }

    fn open_app(directory: &Path, cx: &mut TestAppContext) -> (AnyWindowHandle, Entity<App>) {
        cx.update(gpui_kit::init);
        let book = Book::open(directory).unwrap();
        cx.update(|cx| {
            cx.set_reduce_motion(true);
            gpui_kit::open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds {
                        origin: Default::default(),
                        size: size(px(900.), px(900.)),
                    })),
                    ..Default::default()
                },
                cx,
                |window, cx| cx.new(|cx| App::new(book, window, cx)),
            )
            .unwrap()
        })
    }

    fn add_transaction(app: &Entity<App>, food: ItemId, cash: ItemId, cx: &mut TestAppContext) {
        cx.update(|cx| {
            let state = app.read(cx).book_panel().read(cx).state.clone();
            state.update(cx, |state, cx| {
                state
                    .ledger_mut()
                    .add_transaction(TransactionDraft::new(
                        NaiveDate::from_ymd_opt(2026, 10, 2).unwrap(),
                        Amount::try_from(Money::from(1200)).unwrap(),
                        food,
                        cash,
                        String::new(),
                    ))
                    .unwrap();
                cx.notify();
            });
        });
    }

    fn transaction_count(app: &Entity<App>, cx: &mut TestAppContext) -> usize {
        cx.update(|cx| {
            app.read(cx)
                .book_panel()
                .read(cx)
                .state
                .read(cx)
                .ledger()
                .transactions()
                .iter()
                .count()
        })
    }

    fn click(handle: AnyWindowHandle, id: &'static str, cx: &mut TestAppContext) {
        cx.update_window(handle, |_, window, cx| {
            window.render_frame(cx);
            window.click(id, cx);
        })
        .unwrap();
        cx.run_until_parked();
    }

    /// ダイアログの有無と通知の数を返す.
    fn feedback(handle: AnyWindowHandle, cx: &mut TestAppContext) -> (bool, usize) {
        cx.update_window(handle, |_, window, cx| {
            window.render_frame(cx);
            (window.has_active_dialog(cx), window.notifications(cx).len())
        })
        .unwrap()
    }

    fn status(handle: AnyWindowHandle, cx: &mut TestAppContext) -> String {
        cx.update_window(handle, |_, window, cx| {
            window.render_frame(cx);
            window.find("book-status").label().unwrap().to_string()
        })
        .unwrap()
    }

    fn panel(app: &Entity<App>, cx: &mut TestAppContext) -> Entity<BookPanel> {
        cx.update(|cx| app.read(cx).book_panel())
    }

    #[test]
    fn commit_message_uses_datetime_for_blank_input() {
        let now = NaiveDate::from_ymd_opt(2026, 10, 2)
            .unwrap()
            .and_hms_opt(12, 34, 56)
            .unwrap();

        for input in ["", "  ", "\t"] {
            assert_eq!(commit_message(input, now), "2026-10-02 12:34 の記録");
        }
        assert_eq!(commit_message(" 昼食 ", now), "昼食");
    }

    #[gpui_kit::test]
    fn save_commits_with_message_and_clears_input(cx: &mut TestAppContext) {
        let directory = tempfile::tempdir().unwrap();
        let (food, cash) = init_book(directory.path());
        let (handle, app) = open_app(directory.path(), cx);
        let before = commit_count(directory.path());

        // 変更がなければ保存しない
        click(handle, "save-book", cx);
        assert_eq!(commit_count(directory.path()), before);

        add_transaction(&app, food, cash, cx);
        assert_eq!(status(handle, cx), "未保存の変更があります");
        let panel = panel(&app, cx);
        cx.update_window(handle, |_, window, cx| {
            panel.update(cx, |panel, cx| {
                panel
                    .message
                    .update(cx, |input, cx| input.set_value("昼食", window, cx));
            });
        })
        .unwrap();

        click(handle, "save-book", cx);

        assert_eq!(commit_count(directory.path()), before + 1);
        assert_eq!(git(directory.path(), &["log", "-1", "--format=%s"]), "昼食");
        assert_eq!(status(handle, cx), "保存済み");
        assert_eq!(feedback(handle, cx), (false, 1));
        cx.update(|cx| assert!(panel.read(cx).message.read(cx).value().is_empty()));
    }

    #[gpui_kit::test]
    fn save_failure_keeps_input_and_stays_dirty(cx: &mut TestAppContext) {
        let directory = tempfile::tempdir().unwrap();
        let (food, cash) = init_book(directory.path());
        let (handle, app) = open_app(directory.path(), cx);
        add_transaction(&app, food, cash, cx);
        let panel = panel(&app, cx);
        cx.update_window(handle, |_, window, cx| {
            panel.update(cx, |panel, cx| {
                panel
                    .message
                    .update(cx, |input, cx| input.set_value("昼食", window, cx));
            });
        })
        .unwrap();
        // 帳簿フォルダを git リポジトリでなくして commit を失敗させる
        let git_directory = directory.path().join(".git");
        std::fs::remove_dir_all(&git_directory).unwrap();

        click(handle, "save-book", cx);

        assert_eq!(status(handle, cx), "未保存の変更があります");
        assert_eq!(feedback(handle, cx), (false, 1));
        cx.update(|cx| assert_eq!(panel.read(cx).message.read(cx).value(), "昼食"));
    }

    #[gpui_kit::test]
    fn close_asks_only_when_dirty(cx: &mut TestAppContext) {
        let directory = tempfile::tempdir().unwrap();
        let (food, cash) = init_book(directory.path());
        let (handle, app) = open_app(directory.path(), cx);

        assert!(VisualTestContext::from_window(handle, cx).simulate_close());
        assert_eq!(feedback(handle, cx), (false, 0));

        add_transaction(&app, food, cash, cx);
        assert!(!VisualTestContext::from_window(handle, cx).simulate_close());
        assert_eq!(feedback(handle, cx), (true, 0));

        // キャンセルなら開いたまま
        click(handle, "cancel", cx);
        assert_eq!(feedback(handle, cx), (false, 0));
        assert_eq!(status(handle, cx), "未保存の変更があります");

        assert!(!VisualTestContext::from_window(handle, cx).simulate_close());
        click(handle, "ok", cx);
        assert!(cx.update_window(handle, |_, _, _| ()).is_err());
    }

    #[gpui_kit::test]
    fn sidebar_hides_panel_when_collapsed(cx: &mut TestAppContext) {
        let directory = tempfile::tempdir().unwrap();
        init_book(directory.path());
        let (handle, _app) = open_app(directory.path(), cx);

        cx.update_window(handle, |_, window, cx| {
            window.render_frame(cx);
            assert!(window.find("save-book").visible());
            window.click("toggle-sidebar", cx);
            window.render_frame(cx);
            assert!(window.try_find("save-book").is_none());
        })
        .unwrap();
    }

    #[gpui_kit::test]
    fn sync_with_unsaved_changes_only_asks_to_save(cx: &mut TestAppContext) {
        let directory = tempfile::tempdir().unwrap();
        let (food, cash) = init_book(directory.path());
        let (handle, app) = open_app(directory.path(), cx);
        add_transaction(&app, food, cash, cx);

        click(handle, "sync-book", cx);

        assert_eq!(feedback(handle, cx), (false, 1));
        assert_eq!(transaction_count(&app, cx), 1);
        let panel = panel(&app, cx);
        cx.update(|cx| assert!(!panel.read(cx).is_syncing));
    }

    #[gpui_kit::test]
    fn sync_blocks_until_finished_then_guides_without_remote(cx: &mut TestAppContext) {
        let directory = tempfile::tempdir().unwrap();
        init_book(directory.path());
        let (handle, app) = open_app(directory.path(), cx);
        let panel = panel(&app, cx);

        cx.update_window(handle, |_, window, cx| {
            window.render_frame(cx);
            window.click("sync-book", cx);
            window.render_frame(cx);
            assert!(window.has_active_dialog(cx));
            // 同期中のダイアログにボタンはない
            assert!(window.try_find("ok").is_none());
        })
        .unwrap();
        cx.update(|cx| assert!(panel.read(cx).is_syncing));

        cx.run_until_parked();

        cx.update(|cx| assert!(!panel.read(cx).is_syncing));
        assert_eq!(feedback(handle, cx), (true, 0));
        click(handle, "ok", cx);
        assert_eq!(feedback(handle, cx), (false, 0));
    }

    #[gpui_kit::test]
    fn sync_shares_transactions_between_clones(cx: &mut TestAppContext) {
        let root = tempfile::tempdir().unwrap();
        let (a, b, food, cash) = clones(root.path());
        let (handle_a, app_a) = open_app(&a, cx);
        let (handle_b, app_b) = open_app(&b, cx);

        add_transaction(&app_a, food, cash, cx);
        click(handle_a, "save-book", cx);
        click(handle_a, "sync-book", cx);
        assert_eq!(feedback(handle_a, cx), (false, 2));

        click(handle_b, "sync-book", cx);
        assert_eq!(feedback(handle_b, cx), (false, 1));
        assert_eq!(transaction_count(&app_b, cx), 1);

        add_transaction(&app_b, food, cash, cx);
        click(handle_b, "save-book", cx);
        click(handle_b, "sync-book", cx);
        assert_eq!(transaction_count(&app_b, cx), 2);

        click(handle_a, "sync-book", cx);
        assert_eq!(transaction_count(&app_a, cx), 2);
        assert_eq!(status(handle_a, cx), "保存済み");
    }

    #[gpui_kit::test]
    fn sync_conflict_guides_and_keeps_local_book(cx: &mut TestAppContext) {
        let root = tempfile::tempdir().unwrap();
        let (a, b, food, cash) = clones(root.path());
        let (handle_a, app_a) = open_app(&a, cx);
        let (handle_b, app_b) = open_app(&b, cx);
        add_transaction(&app_a, food, cash, cx);
        click(handle_a, "save-book", cx);
        click(handle_a, "sync-book", cx);
        add_transaction(&app_b, food, cash, cx);
        click(handle_b, "save-book", cx);
        let notifications = feedback(handle_b, cx).1;
        let head = git(&b, &["rev-parse", "HEAD"]);

        click(handle_b, "sync-book", cx);

        assert_eq!(feedback(handle_b, cx), (true, notifications));
        assert_eq!(git(&b, &["rev-parse", "HEAD"]), head);
        assert_eq!(transaction_count(&app_b, cx), 1);
        assert_eq!(git(&b, &["status", "--porcelain"]), "");
    }

    #[gpui_kit::test]
    fn reload_failure_blocks_until_reopened(cx: &mut TestAppContext) {
        let directory = tempfile::tempdir().unwrap();
        init_book(directory.path());
        let (handle, app) = open_app(directory.path(), cx);
        std::fs::write(directory.path().join("items.toml"), "壊れている").unwrap();

        click(handle, "sync-book", cx);

        assert_eq!(feedback(handle, cx), (true, 0));
        cx.update_window(handle, |_, window, cx| {
            window.render_frame(cx);
            assert!(window.try_find("ok").is_none());
        })
        .unwrap();
        let panel = panel(&app, cx);
        cx.update(|cx| assert!(panel.read(cx).is_syncing));
        assert!(VisualTestContext::from_window(handle, cx).simulate_close());
    }
}

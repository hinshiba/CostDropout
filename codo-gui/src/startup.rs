//! 起動時の帳簿選択と読み込み.

use std::path::PathBuf;

use codo_core::book::Book;
use gpui_kit::component::{Disableable, button::Button};
use gpui_kit::*;

use crate::app::App;

/// 起動中に同時には成立しない状態を区別する.
enum StartupState {
    Waiting,
    Selecting,
    Failed(String),
    Opened(Entity<App>),
}

/// 帳簿が開かれるまで選択画面を表示する.
pub struct Startup {
    state: StartupState,
}

impl Startup {
    /// 引数のフォルダを開くか, 引数がなければフォルダ選択を表示する.
    pub fn new(directory: Option<PathBuf>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut startup = Self {
            state: StartupState::Waiting,
        };
        if let Some(directory) = directory {
            startup.open(directory, window, cx);
        } else {
            startup.select_directory(window, cx);
        }
        startup
    }

    /// 指定したフォルダを開き, 失敗した場合は選び直せる状態にする.
    fn open(&mut self, directory: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        self.state = match Book::open(&directory) {
            Ok(book) => {
                let directory = directory.canonicalize().unwrap_or(directory);
                let name = directory.file_name().unwrap_or(directory.as_os_str());
                window.set_window_title(&format!("{} - CostDropout", name.to_string_lossy()));
                StartupState::Opened(cx.new(|cx| App::new(book, window, cx)))
            }
            Err(error) => StartupState::Failed(format!(
                "帳簿を開けませんでした: {}\n{error}",
                directory.display()
            )),
        };
        cx.notify();
    }

    /// フォルダ選択の結果を待ち, キャンセル時は選択前のエラーを保持する.
    fn select_directory(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let previous_error = match &self.state {
            StartupState::Waiting => None,
            StartupState::Failed(error) => Some(error.clone()),
            StartupState::Selecting | StartupState::Opened(_) => return,
        };
        self.state = StartupState::Selecting;
        let selection = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("帳簿フォルダを選ぶ".into()),
        });
        cx.spawn_in(window, async move |view, cx| {
            let result = selection.await;
            let _ = cx.update(|window, cx| {
                view.update(cx, |this, cx| {
                    this.state = match result {
                        Ok(Ok(paths)) => {
                            if let Some(directory) =
                                paths.and_then(|paths| paths.into_iter().next())
                            {
                                this.open(directory, window, cx);
                                return;
                            }
                            previous_error
                                .map(StartupState::Failed)
                                .unwrap_or(StartupState::Waiting)
                        }
                        Ok(Err(error)) => {
                            StartupState::Failed(format!("フォルダ選択を開けませんでした: {error}"))
                        }
                        Err(error) => StartupState::Failed(format!(
                            "フォルダ選択の結果を取得できませんでした: {error}"
                        )),
                    };
                    cx.notify();
                })
            });
        })
        .detach();
        cx.notify();
    }
}

impl Render for Startup {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let error = match &self.state {
            StartupState::Opened(app) => return div().size_full().child(app.clone()),
            StartupState::Failed(error) => Some(error.clone()),
            StartupState::Waiting | StartupState::Selecting => None,
        };
        div()
            .size_full()
            .flex()
            .flex_col()
            .gap_4()
            .p_6()
            .child("帳簿フォルダを選んでください")
            .children(error)
            .child(
                Button::new("select-book-directory")
                    .label("フォルダを選ぶ")
                    .disabled(matches!(self.state, StartupState::Selecting))
                    .on_click(cx.listener(|this, _, window, cx| this.select_directory(window, cx))),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::{Startup, StartupState};
    use codo_core::{
        book::Book,
        infra::filestore::{LedgerStore, TomlStore},
        infra::gitrepo::GitRepo,
        ledger::Ledger,
    };
    use gpui_kit::test::TestWindowExt;
    use gpui_kit::{AppContext, TestAppContext, VisualTestContext, WindowOptions};
    use std::path::PathBuf;

    #[gpui_kit::test]
    fn cancelled_selection_can_be_reopened(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        let (handle, startup) = cx.update(|cx| {
            gpui_kit::open_window(WindowOptions::default(), cx, |window, cx| {
                cx.new(|cx| Startup::new(None, window, cx))
            })
            .unwrap()
        });
        startup.read_with(cx, |startup, _| {
            assert!(matches!(startup.state, StartupState::Selecting));
        });
        cx.update_window(handle, |_, window, cx| {
            startup.update(cx, |startup, cx| startup.select_directory(window, cx));
        })
        .unwrap();
        assert!(cx.did_prompt_for_paths());
        cx.simulate_path_prompt_response(|options| {
            assert!(!options.files);
            assert!(options.directories);
            assert!(!options.multiple);
            None
        });
        cx.run_until_parked();
        startup.read_with(cx, |startup, _| {
            assert!(matches!(startup.state, StartupState::Waiting));
        });
        cx.update_window(handle, |_, window, cx| {
            window.render_frame(cx);
            window.click("select-book-directory", cx);
        })
        .unwrap();
        assert!(cx.did_prompt_for_paths());
        cx.simulate_path_prompt_response(|_| None);
        cx.run_until_parked();
    }

    #[gpui_kit::test]
    fn invalid_directory_can_be_replaced_with_existing_book(cx: &mut TestAppContext) {
        let directory = tempfile::tempdir().unwrap();
        let missing = directory.path().join("missing");
        let mut ledger = Ledger::with_presets();
        ledger
            .add_item("読み込み確認", codo_core::ledger::item::Tag::Expense)
            .unwrap();
        TomlStore::new(directory.path().to_path_buf())
            .save(&ledger)
            .unwrap();
        GitRepo::init(directory.path().to_path_buf()).unwrap();
        cx.update(gpui_kit::init);
        let (handle, startup) = cx.update(|cx| {
            gpui_kit::open_window(WindowOptions::default(), cx, |window, cx| {
                cx.new(|cx| Startup::new(Some(missing.clone()), window, cx))
            })
            .unwrap()
        });
        let error = Book::open(&missing).err().unwrap();
        let expected_error = format!("帳簿を開けませんでした: {}\n{error}", missing.display());
        startup.read_with(cx, |startup, _| {
            assert!(
                matches!(&startup.state, StartupState::Failed(error) if error == &expected_error)
            );
        });
        assert!(!missing.exists());
        assert!(!cx.did_prompt_for_paths());
        cx.update_window(handle, |_, window, cx| {
            window.render_frame(cx);
            window.click("select-book-directory", cx);
        })
        .unwrap();
        assert!(cx.did_prompt_for_paths());
        cx.simulate_path_prompt_response(|_| None);
        cx.run_until_parked();
        startup.read_with(cx, |startup, _| {
            assert!(
                matches!(&startup.state, StartupState::Failed(error) if error == &expected_error)
            );
        });
        cx.update_window(handle, |_, window, cx| {
            window.render_frame(cx);
            window.click("select-book-directory", cx);
        })
        .unwrap();
        assert!(cx.did_prompt_for_paths());
        cx.simulate_path_prompt_response(|_| Some(vec![directory.path().to_path_buf()]));
        cx.run_until_parked();
        startup.read_with(cx, |startup, _| {
            assert!(matches!(startup.state, StartupState::Opened(_)));
        });
        let name = directory.path().file_name().unwrap().to_string_lossy();
        assert_eq!(
            VisualTestContext::from_window(handle, cx).window_title(),
            Some(format!("{name} - CostDropout")),
        );
        assert_eq!(
            TomlStore::new(directory.path().to_path_buf())
                .load()
                .unwrap(),
            ledger
        );
    }

    #[gpui_kit::test]
    fn directory_argument_loads_saved_transactions(cx: &mut TestAppContext) {
        use codo_core::{
            ledger::item::Tag,
            ledger::money::Money,
            ledger::transaction::{Amount, TransactionDraft},
        };
        let directory = tempfile::tempdir().unwrap();
        let mut ledger = Ledger::new();
        let food = ledger.add_item("食費", Tag::Expense).unwrap();
        let cash = ledger.add_item("現金", Tag::Asset).unwrap();
        ledger
            .add_transaction(TransactionDraft::new(
                chrono::Local::now().date_naive(),
                Amount::try_from(Money::from(1200)).unwrap(),
                food,
                cash,
                "保存済みの取引".into(),
            ))
            .unwrap();
        TomlStore::new(directory.path().to_path_buf())
            .save(&ledger)
            .unwrap();
        GitRepo::init(directory.path().to_path_buf()).unwrap();
        cx.update(gpui_kit::init);
        let (handle, startup) = cx.update(|cx| {
            gpui_kit::open_window(WindowOptions::default(), cx, |window, cx| {
                cx.new(|cx| Startup::new(Some(directory.path().to_path_buf()), window, cx))
            })
            .unwrap()
        });
        cx.update_window(handle, |_, window, cx| {
            startup.update(cx, |startup, cx| startup.select_directory(window, cx));
        })
        .unwrap();
        assert!(!cx.did_prompt_for_paths());
        startup.read_with(cx, |startup, _| {
            assert!(matches!(startup.state, StartupState::Opened(_)));
        });
        cx.update_window(handle, |_, window, cx| {
            window.render_frame(cx);
            assert!(window.find("budget-empty-state").visible());
            window.click("0-1", cx);
            window.render_frame(cx);
            assert_eq!(
                window.find("transaction-count").label(),
                Some("登録済みの取引: 1件")
            );
        })
        .unwrap();
    }

    #[gpui_kit::test]
    fn empty_directory_argument_initializes_book(cx: &mut TestAppContext) {
        // 初回 commit 用の設定は子プロセスに限定し, 他のテストと利用者の設定を変えない.
        let Some(directory) = std::env::var_os("CODO_GUI_INITIALIZE_TEST_DIR") else {
            let fixture = tempfile::tempdir().unwrap();
            let directory = fixture.path().join("book");
            let config = fixture.path().join("gitconfig");
            let hooks = fixture.path().join("hooks");
            std::fs::create_dir(&directory).unwrap();
            std::fs::create_dir(&hooks).unwrap();
            std::fs::write(&config, "").unwrap();
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "startup::tests::empty_directory_argument_initializes_book",
                    "--nocapture",
                ])
                .env("CODO_GUI_INITIALIZE_TEST_DIR", &directory)
                .env("GIT_AUTHOR_NAME", "book test")
                .env("GIT_AUTHOR_EMAIL", "book-test@example.com")
                .env("GIT_COMMITTER_NAME", "book test")
                .env("GIT_COMMITTER_EMAIL", "book-test@example.com")
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .env("GIT_CONFIG_GLOBAL", &config)
                .env("GIT_CONFIG_COUNT", "2")
                .env("GIT_CONFIG_KEY_0", "commit.gpgsign")
                .env("GIT_CONFIG_VALUE_0", "false")
                .env("GIT_CONFIG_KEY_1", "core.hooksPath")
                .env("GIT_CONFIG_VALUE_1", &hooks)
                .env_remove("GIT_CONFIG_PARAMETERS")
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "stdout: {}\nstderr: {}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            return;
        };
        let directory = PathBuf::from(directory);
        cx.update(gpui_kit::init);
        let (handle, startup) = cx.update(|cx| {
            gpui_kit::open_window(WindowOptions::default(), cx, |window, cx| {
                cx.new(|cx| Startup::new(Some(directory.clone()), window, cx))
            })
            .unwrap()
        });
        startup.read_with(cx, |startup, _| {
            assert!(matches!(startup.state, StartupState::Opened(_)));
        });
        assert_eq!(
            TomlStore::new(directory.clone()).load().unwrap(),
            Ledger::with_presets()
        );
        let repository = GitRepo::open(directory).unwrap();
        assert!(!repository.has_changes().unwrap());
        cx.update_window(handle, |_, window, cx| {
            window.render_frame(cx);
            assert!(window.find("budget-empty-state").visible());
            window.click("0-1", cx);
            window.render_frame(cx);
            assert_eq!(
                window.find("transaction-count").label(),
                Some("登録済みの取引: 0件")
            );
        })
        .unwrap();
    }
}

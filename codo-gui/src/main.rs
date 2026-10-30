//! 家計簿アプリケーションの起動.

mod app;
mod book_panel;
mod pages;
mod startup;
mod transaction_fields;

use crate::startup::Startup;
use gpui_kit::*;

fn main() {
    let directory = std::env::args_os().nth(1).map(std::path::PathBuf::from);
    gpui_kit::application().run(move |cx| {
        gpui_kit::init(cx);
        gpui_kit::open_window(WindowOptions::default(), cx, |window, cx| {
            cx.new(|cx| Startup::new(directory, window, cx))
        })
        .expect("ウィンドウを開けませんでした");
    });
}

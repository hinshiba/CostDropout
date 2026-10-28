//! 家計簿アプリケーションの起動。

mod app;
mod demo;
mod pages;
use crate::app::App;
use gpui_kit::*;

fn main() {
    let is_demo = std::env::args().any(|argument| argument == "--demo");
    gpui_kit::application().run(move |cx| {
        gpui_kit::init(cx);
        gpui_kit::open_window(WindowOptions::default(), cx, |window, cx| {
            if is_demo {
                cx.new(|cx| App::with_ledger(demo::ledger(), window, cx))
            } else {
                cx.new(|cx| App::new(window, cx))
            }
        })
        .expect("ウィンドウを開けませんでした");
    });
}

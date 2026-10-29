//! 家計簿アプリケーションの起動。

mod app;
mod pages;
use crate::app::App;
use gpui_kit::*;

fn main() {
    gpui_kit::application().run(|cx| {
        gpui_kit::init(cx);
        gpui_kit::open_window(WindowOptions::default(), cx, |window, cx| {
            cx.new(|cx| App::new(window, cx))
        })
        .expect("ウィンドウを開けませんでした");
    });
}

mod app;
mod pages;
use crate::app::App;
use gpui_kit::*;

fn main() {
    gpui_kit::application().run(|cx| {
        gpui_kit::init(cx);
        gpui_kit::open_window(WindowOptions::default(), cx, |_, cx| {
            cx.new(|cx| App::new(cx))
        })
        .expect("ウィンドウを開けませんでした");
    });
}

//! 品目管理表示画面

use crate::app::AppState;
use codo_core::item::{ItemId, Tag};
use gpui_kit::component::button::Button;
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

/// 品目画面
pub struct Items {
    // 共有されているAppState
    state: Entity<AppState>,
    new_item_name: String,
    new_item_tag: Tag,
    error_message: Option<String>,
    name_input: Option<Entity<InputState>>,
    editing_item: Option<ItemId>,
}

impl Items {
    pub fn new(state: Entity<AppState>) -> Self {
        Self {
            state,
            new_item_name: String::new(),
            new_item_tag: Tag::Expense,
            error_message: None,
            name_input: None,
            editing_item: None,
        }
    }

    fn start_editing(
        &mut self,
        id: ItemId,
        name: &str,
        tag: Tag,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.editing_item = Some(id);
        self.new_item_tag = tag;
        self.error_message = None;

        if let Some(input) = &self.name_input {
            input.update(cx, |state, cx| {
                state.set_value(name, window, cx);
            });
        }

        cx.notify();
    }

    fn add_item(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let name = self
            .name_input
            .as_ref()
            .map(|input| input.read(cx).value().to_string())
            .unwrap_or_default();

        let name = name.trim().to_string();
        let tag = self.new_item_tag;

        let result = self.state.update(cx, |state, _| {
            if let Some(id) = self.editing_item {
                state.ledger.update_item(id, &name, tag)
            } else {
                state.ledger.add_item(&name, tag).map(|_| ())
            }
        });

        match result {
            Ok(()) => {
                self.new_item_name.clear();
                self.editing_item = None;
                self.error_message = None;

                if let Some(input) = &self.name_input {
                    input.update(cx, |state, cx| {
                        state.set_value("", window, cx);
                    });
                }

                cx.notify();
            }
            Err(error) => {
                self.error_message = Some(error.to_string());
                cx.notify();
            }
        }
    }

    fn select_tag(&mut self, tag: Tag, cx: &mut Context<Self>) {
        self.new_item_tag = tag;
        self.error_message = None;
        cx.notify();
    }
}

// 画面開発者は自由に変更してよい
impl Render for Items {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.name_input.is_none() {
            self.name_input = Some(cx.new(|cx| InputState::new(window, cx)));
        }

        let name_input = self.name_input.as_ref().unwrap().clone();

        let state = self.state.read(cx);
        let _ledger = &state.ledger;

        // 実際の品目画面表示
        div()
            .flex()
            .flex_col()
            .bg(rgb(0xffffff))
            .size_full()
            .text_color(rgb(0x000000))
            // タイトルバー
            .child(
                div()
                    .w_full()
                    .flex()
                    .justify_center()
                    .items_center()
                    .bg(rgb(0x2e7d32))
                    .text_color(rgb(0xffffff))
                    .text_xl()
                    .child("品目"),
            )
            // スクロール領域
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    //.overflow_y_scrollbar()
                    // 品目一覧
                    .child(div().flex().flex_col().p_4().gap_4().children(
                        Tag::ALL_TAGS.into_iter().map(|tag| {
                            let items = _ledger
                                .items()
                                .iter()
                                .filter(|item| item.tag() == tag)
                                .map(|item| {
                                    let id = item.id();
                                    let name = item.name().to_string();
                                    let item_tag = item.tag();
                                    let edit_name = name.clone();

                                    div()
                                        .flex()
                                        .flex_row()
                                        .items_center()
                                        .justify_between()
                                        .p_2()
                                        .child(name)
                                        .child(
                                            Button::new(format!("edit-{}", id.id()))
                                                .label("編集")
                                                .on_click(cx.listener(
                                                    move |this, _, window, cx| {
                                                        this.start_editing(
                                                            id, &edit_name, item_tag, window, cx,
                                                        );
                                                    },
                                                )),
                                        )
                                });

                            div()
                                .flex()
                                .flex_col()
                                .gap_1()
                                .child(div().text_lg().child(tag.label()))
                                .children(items)
                        }),
                    ))
                    // 品目の追加
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .p_4()
                            .gap_2()
                            // 見出し
                            .child(div().text_lg().child("品目の追加"))
                            .child(Input::new(&name_input))
                            .child(
                                div().when_some(self.error_message.clone(), |this, message| {
                                    this.text_color(rgb(0xc62828)).child(message)
                                }),
                            )
                            // タグ選択欄
                            .child(div().flex().flex_row().gap_2().children(
                                Tag::ALL_TAGS.into_iter().map(|tag| {
                                    let selected = self.new_item_tag == tag;

                                    Button::new(format!("tag-{}", tag.label()))
                                        .label(tag.label())
                                        .bg(if selected {
                                            rgb(0x2e7d32)
                                        } else {
                                            rgb(0xe0e0e0)
                                        })
                                        .text_color(if selected {
                                            rgb(0xffffff)
                                        } else {
                                            rgb(0x000000)
                                        })
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            this.select_tag(tag, cx);
                                        }))
                                }),
                            ))
                            .child(
                                Button::new("add_item")
                                    .label(if self.editing_item.is_some() {
                                        "更新"
                                    } else {
                                        "追加"
                                    })
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.add_item(window, cx);
                                    })),
                            ),
                    ),
            )
    }
}

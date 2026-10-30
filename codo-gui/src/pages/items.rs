//! 品目管理表示画面

use crate::app::AppState;
use codo_core::ledger::LedgerError;
use codo_core::ledger::item::{ItemError, ItemId, Tag};
use gpui_kit::component::Selectable;
use gpui_kit::component::WindowExt;
use gpui_kit::component::button::Button;
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

/// 品目の追加・編集・削除を行う画面。
pub struct Items {
    // 共有されているAppState
    state: Entity<AppState>,
    new_item_tag: Tag,
    error_message: Option<String>,
    name_input: Option<Entity<InputState>>,
    editing_item: Option<ItemId>,
    item_list_scroll_handle: ScrollHandle,
}

impl Items {
    /// 品目画面を作成する。
    pub fn new(state: Entity<AppState>) -> Self {
        Self {
            state,
            new_item_tag: Tag::Expense,
            error_message: None,
            name_input: None,
            editing_item: None,
            item_list_scroll_handle: ScrollHandle::new(),
        }
    }

    /// 選択した品目を編集フォームに読み込む。
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

    /// 入力された名前とタグで品目を追加または更新する。
    fn add_item(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let name = self
            .name_input
            .as_ref()
            .map(|input| input.read(cx).value().to_string())
            .unwrap_or_default();
        let tag = self.new_item_tag;

        let result = self.state.update(cx, |state, cx| {
            if let Some(id) = self.editing_item {
                state.ledger_mut().update_item(id, &name, tag)?;
            } else {
                state.ledger_mut().add_item(&name, tag)?;
            }
            cx.notify();
            Ok::<_, LedgerError>(())
        });

        match result {
            Ok(()) => {
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
                self.error_message = Some(Self::friendly_error(error));
                cx.notify();
            }
        }
    }

    /// 削除の確認ダイアログを開き、確定後に品目を削除する。
    fn confirm_remove_item(
        &mut self,
        id: ItemId,
        name: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let item_name = name;
        let weak_items = cx.weak_entity();

        window.open_alert_dialog(cx, move |alert, _, _| {
            let weak_items = weak_items.clone();
            let item_name_for_message = item_name.clone();

            alert
                .confirm()
                .title("品目を削除しますか？")
                .description(format!(
                    "「{item_name_for_message}」を削除します。この操作は取り消せません。"
                ))
                .ok_text("削除")
                .cancel_text("キャンセル")
                .on_ok(move |_, _, cx| {
                    weak_items
                        .update(cx, |items, cx| items.remove_item(id, cx))
                        .is_ok()
                })
        });
    }

    /// Ledger の削除処理を実行し、結果を画面に反映する。
    fn remove_item(&mut self, id: ItemId, cx: &mut Context<Self>) {
        let result = self.state.update(cx, |state, cx| {
            state.ledger_mut().remove_item(id)?;
            cx.notify();
            Ok::<_, LedgerError>(())
        });

        match result {
            Ok(()) => {
                if self.editing_item == Some(id) {
                    self.editing_item = None;
                }
                self.error_message = None;
            }
            Err(error) => self.error_message = Some(Self::friendly_error(error)),
        }
        cx.notify();
    }

    /// Ledger のエラーを画面利用者向けの日本語に変換する。
    fn friendly_error(error: LedgerError) -> String {
        match error {
            LedgerError::Item(ItemError::EmptyName) => "品目名を入力してください".to_string(),
            LedgerError::Item(ItemError::DuplicateName(name)) => {
                format!("「{name}」という品目はすでに登録されています")
            }
            LedgerError::ItemUse(_) => {
                "この品目を使っている取引があるため削除できません。先に取引で使う品目を変更してください".to_string()
            }
            LedgerError::ItemTagInUse(_) => {
                "この品目を使っている取引があるためタグを変更できません。先に取引で使う品目を変更してください".to_string()
            }
            error => error.to_string(),
        }
    }

    /// 品目名と編集・削除操作を1行にまとめて描画する。
    fn render_item_row(id: ItemId, name: String, tag: Tag, cx: &mut Context<Self>) -> AnyElement {
        let edit_name = name.clone();
        let delete_name = name.clone();

        let edit_button = Button::new(format!("edit-{id:?}"))
            .label("編集")
            .bg(rgb(0xe8f5e9))
            .text_color(rgb(0x1b5e20))
            .on_click(cx.listener(move |this, _, window, cx| {
                this.start_editing(id, &edit_name, tag, window, cx);
            }));

        let delete_button = Button::new(format!("delete-{id:?}"))
            .label("削除")
            .bg(rgb(0xffebee))
            .text_color(rgb(0xb71c1c))
            .on_click(cx.listener(move |this, _, window, cx| {
                this.confirm_remove_item(id, delete_name.clone(), window, cx);
            }));

        let action_buttons = div()
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .child(edit_button)
            .child(delete_button);

        div()
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .p_2()
            .child(name)
            .child(action_buttons)
            .into_any_element()
    }

    /// 追加・編集フォームで選択中のタグを切り替える。
    fn select_tag(&mut self, tag: Tag, cx: &mut Context<Self>) {
        self.new_item_tag = tag;
        self.error_message = None;
        cx.notify();
    }
}

impl Render for Items {
    /// タグ別の品目一覧と、画面下部に固定する入力フォームを描画する。
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.name_input.is_none() {
            self.name_input = Some(cx.new(|cx| InputState::new(window, cx)));
        }

        let name_input = self.name_input.as_ref().unwrap().clone();

        let item_groups = {
            let state = self.state.read(cx);
            Tag::ALL
                .into_iter()
                .map(|tag| {
                    let items = state
                        .ledger()
                        .items()
                        .iter_with_tag(tag)
                        .map(|item| (item.id(), item.to_string()))
                        .collect::<Vec<_>>();

                    (tag, items)
                })
                .collect::<Vec<_>>()
        };
        let item_form_height = px(120.);
        let page_title_height = px(32.);
        let item_list_height =
            (window.viewport_size().height - item_form_height - page_title_height).max(px(0.));

        let mut item_sections = Vec::with_capacity(item_groups.len());
        for (tag, items) in item_groups {
            let item_rows = items
                .into_iter()
                .map(|(id, name)| Self::render_item_row(id, name, tag, cx))
                .collect::<Vec<_>>();

            let section_heading = div()
                .w_full()
                .px_3()
                .py_2()
                .bg(rgb(0xe8f5e9))
                .text_color(rgb(0x1b5e20))
                .text_xl()
                .child(tag.to_string());

            let section = div()
                .flex()
                .flex_col()
                .gap_2()
                .child(section_heading)
                .children(item_rows)
                .into_any_element();

            item_sections.push(section);
        }

        let item_list = div()
            .id("items-list")
            .flex()
            .flex_col()
            .h(item_list_height)
            .flex_shrink_0()
            .overflow_y_scroll()
            .track_scroll(&self.item_list_scroll_handle)
            .p_4()
            .gap_4()
            .children(item_sections);

        let is_editing_item = self.editing_item.is_some();
        let add_button_label = if is_editing_item { "更新" } else { "追加" };
        let add_button = Button::new("add_item")
            .label(add_button_label)
            .on_click(cx.listener(|this, _, window, cx| {
                this.add_item(window, cx);
            }));

        let name_and_submit = div()
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .child(div().flex_1().child(Input::new(&name_input)))
            .child(add_button);

        let error_message = self.error_message.clone();
        let tag_buttons = Tag::ALL.into_iter().map(|tag| {
            let is_selected = self.new_item_tag == tag;
            let button_background = if is_selected {
                rgb(0x2e7d32)
            } else {
                rgb(0xe0e0e0)
            };
            let button_text_color = if is_selected {
                rgb(0xffffff)
            } else {
                rgb(0x000000)
            };

            Button::new(format!("tag-{tag:?}"))
                .label(tag.to_string())
                .selected(is_selected)
                .bg(button_background)
                .text_color(button_text_color)
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.select_tag(tag, cx);
                }))
        });

        let tag_selector = div().flex().flex_row().gap_2().children(tag_buttons);

        let add_form = div()
            .flex()
            .flex_col()
            .flex_shrink_0()
            .h(item_form_height)
            .px_4()
            .py_1()
            .gap_1()
            .border_t_1()
            .border_color(rgb(0xe0e0e0))
            .bg(rgb(0xffffff))
            .child(name_and_submit)
            .when_some(error_message, |form, message| {
                form.child(div().text_color(rgb(0xc62828)).child(message))
            })
            .child(tag_selector);

        let page_title = div()
            .w_full()
            .flex()
            .justify_center()
            .items_center()
            .bg(rgb(0x2e7d32))
            .text_color(rgb(0xffffff))
            .text_xl()
            .child("品目");

        let page_content = div()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .overflow_hidden()
            .child(item_list)
            .child(add_form);

        div()
            .flex()
            .flex_col()
            .bg(rgb(0xffffff))
            .size_full()
            .min_h_0()
            .overflow_hidden()
            .text_color(rgb(0x000000))
            .child(page_title.flex_shrink_0())
            .child(page_content)
    }
}

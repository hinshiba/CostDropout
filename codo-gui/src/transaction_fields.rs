//! 取引の入力欄. 新規の追加と訂正の両方で使う.

use codo_core::item::{Item, ItemId, Tag};
use gpui_kit::SharedString;
use gpui_kit::component::select::SelectItem;

/// 品目と相手先の選択肢.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemChoice {
    pub id: ItemId,
    pub name: SharedString,
    pub tag: Tag,
}

impl SelectItem for ItemChoice {
    type Value = ItemId;

    fn title(&self) -> SharedString {
        self.name.clone()
    }

    fn value(&self) -> &ItemId {
        &self.id
    }
}

impl From<&Item> for ItemChoice {
    fn from(item: &Item) -> Self {
        Self {
            id: item.id(),
            name: item.name().to_string().into(),
            tag: item.tag(),
        }
    }
}

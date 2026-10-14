//! プリセット品目
//!
//! 新しい帳簿を作るときに最初から登録しておく品目を提供する
use crate::item::{Item, ItemId, Tag};

pub fn preset_items() -> Vec<Item> {
    vec![
        Item::new(ItemId::new(1), "食費".into(), Tag::Expense),
        Item::new(ItemId::new(2), "娯楽".into(), Tag::Expense),
        Item::new(ItemId::new(3), "日用雑貨".into(), Tag::Expense),
        Item::new(ItemId::new(4), "衣服/美容".into(), Tag::Expense),
        Item::new(ItemId::new(5), "定期支出".into(), Tag::Expense),
        Item::new(ItemId::new(6), "その他".into(), Tag::Expense),
        Item::new(ItemId::new(7), "給与".into(), Tag::Income),
        Item::new(ItemId::new(8), "その他収入".into(), Tag::Income),
        Item::new(ItemId::new(9), "現金".into(), Tag::Asset),
        Item::new(ItemId::new(10), "銀行口座".into(), Tag::Asset),
        Item::new(ItemId::new(11), "クレジットカード".into(), Tag::Liability),
        Item::new(ItemId::new(12), "初期残高".into(), Tag::OpeningBalance),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn preset_items_has_all_tags() {
        let items = preset_items();

        assert!(items.iter().any(|item| item.tag() == Tag::Expense));
        assert!(items.iter().any(|item| item.tag() == Tag::Income));
        assert!(items.iter().any(|item| item.tag() == Tag::Asset));
        assert!(items.iter().any(|item| item.tag() == Tag::Liability));
        assert!(items.iter().any(|item| item.tag() == Tag::OpeningBalance));
    }

    #[test]
    fn preset_items_has_unique_ids_names() {
        let items = preset_items();

        let mut ids = HashSet::new();
        let mut names = HashSet::new();
        for item in items {
            assert!(ids.insert(item.id()));
            assert!(names.insert(item.name().to_string()));
        }
    }
}

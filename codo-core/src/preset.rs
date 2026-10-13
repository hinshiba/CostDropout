//! プリセット品目
//!
//! 新しい帳簿を作るときに最初から登録しておく品目を提供する

use crate::item::{Item, ItemId, Tag};

pub fn preset_items() -> Vec<Item> {
    let item = vec![
        Item {
            id: ItemId(1),
            name: "食費".into(),
            tag: Tag::Expense,
        },
        Item {
            id: ItemId(2),
            name: "娯楽".into(),
            tag: Tag::Expense,
        },
        Item {
            id: ItemId(3),
            name: "日用雑貨".into(),
            tag: Tag::Expense,
        },
        Item {
            id: ItemId(4),
            name: "衣服/美容".into(),
            tag: Tag::Expense,
        },
        Item {
            id: ItemId(5),
            name: "定期支出".into(),
            tag: Tag::Expense,
        },
        Item {
            id: ItemId(6),
            name: "その他".into(),
            tag: Tag::Expense,
        },
        Item {
            id: ItemId(7),
            name: "給与".into(),
            tag: Tag::Income,
        },
        Item {
            id: ItemId(8),
            name: "その他収入".into(),
            tag: Tag::Income,
        },
        Item {
            id: ItemId(9),
            name: "現金".into(),
            tag: Tag::Asset,
        },
        Item {
            id: ItemId(10),
            name: "銀行口座".into(),
            tag: Tag::Asset,
        },
        Item {
            id: ItemId(11),
            name: "クレジットカード".into(),
            tag: Tag::Liability,
        },
        Item {
            id: ItemId(12),
            name: "初期残高".into(),
            tag: Tag::OpeningBalance,
        },
    ];

    item
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn preset_items_has_all_tags() {
        let items = preset_items();

        assert!(items.iter().any(|item| item.tag == Tag::Expense));
        assert!(items.iter().any(|item| item.tag == Tag::Income));
        assert!(items.iter().any(|item| item.tag == Tag::Asset));
        assert!(items.iter().any(|item| item.tag == Tag::Liability));
        assert!(items.iter().any(|item| item.tag == Tag::OpeningBalance));
    }

    #[test]
    fn preset_items_has_unique_ids_names() {
        let items = preset_items();

        let mut ids = HashSet::new();
        let mut names = HashSet::new();
        for item in items {
            assert!(ids.insert(item.id));
            assert!(names.insert(item.name));
        }
    }
}

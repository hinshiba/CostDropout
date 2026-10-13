//! プリセット品目
//!
//! 新しい帳簿を作るときに最初から登録しておく品目を提供する
use std::collections::HashSet;

#[derive(Debug, PartialEq)]

pub enum Tag {
    Asset,
    Liability,
    OpeningBalance,
    Income,
    Expense,
}

pub struct Item {
    id: u32,
    name: String,
    tag: Tag,
}

fn validate_items(item: &[Item]) {
    let required_tags = [
        Tag::Expense,
        Tag::Income,
        Tag::Asset,
        Tag::Liability,
        Tag::OpeningBalance,
    ];

    for tag in required_tags {
        let has_tag = item.iter().any(|item| item.tag == tag);
        if !has_tag {
            panic!("エラー: {:?} が登録されていません", tag);
        }
    }

    let mut ids = HashSet::new();
    let mut names = HashSet::new();

    for item in item {
        if !ids.insert(&item.id) {
            panic!("ID {} が重複しています", item.id)
        }
        if !names.insert(&item.name) {
            panic!("{} が重複しています", item.name)
        }
    }
}

pub fn preset_items() -> Vec<Item> {
    let item = vec![
        Item {
            id: 1,
            name: "食費".into(),
            tag: Tag::Expense,
        },
        Item {
            id: 2,
            name: "娯楽".into(),
            tag: Tag::Expense,
        },
        Item {
            id: 3,
            name: "日用雑貨".into(),
            tag: Tag::Expense,
        },
        Item {
            id: 4,
            name: "衣服/美容".into(),
            tag: Tag::Expense,
        },
        Item {
            id: 5,
            name: "定期支出".into(),
            tag: Tag::Expense,
        },
        Item {
            id: 6,
            name: "その他".into(),
            tag: Tag::Expense,
        },
        Item {
            id: 7,
            name: "給与".into(),
            tag: Tag::Income,
        },
        Item {
            id: 8,
            name: "その他収入".into(),
            tag: Tag::Income,
        },
        Item {
            id: 9,
            name: "現金".into(),
            tag: Tag::Asset,
        },
        Item {
            id: 10,
            name: "銀行口座".into(),
            tag: Tag::Asset,
        },
        Item {
            id: 11,
            name: "クレジットカード".into(),
            tag: Tag::Liability,
        },
        Item {
            id: 12,
            name: "初期残高".into(),
            tag: Tag::OpeningBalance,
        },
    ];

    validate_items(&item);
    item
}

#[cfg(test)]
mod tests {
    use super::*;

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

#[test]
#[should_panic(expected = "ID 1 が重複しています")]
fn duplicate_id_panic() {
    let items = vec![
        Item {
            id: 1,
            name: "食費".into(),
            tag: Tag::Expense,
        },
        Item {
            id: 1,
            name: "給与".into(),
            tag: Tag::Income,
        },
        Item {
            id: 9,
            name: "現金".into(),
            tag: Tag::Asset,
        },
        Item {
            id: 11,
            name: "クレジットカード".into(),
            tag: Tag::Liability,
        },
        Item {
            id: 12,
            name: "初期残高".into(),
            tag: Tag::OpeningBalance,
        },
    ];

    validate_items(&items);
}

#[test]
#[should_panic(expected = "現金 が重複しています")]
fn duplicate_name_panic() {
    let items = vec![
        Item {
            id: 1,
            name: "食費".into(),
            tag: Tag::Expense,
        },
        Item {
            id: 2,
            name: "現金".into(),
            tag: Tag::Income,
        },
        Item {
            id: 9,
            name: "現金".into(),
            tag: Tag::Asset,
        },
        Item {
            id: 11,
            name: "クレジットカード".into(),
            tag: Tag::Liability,
        },
        Item {
            id: 12,
            name: "初期残高".into(),
            tag: Tag::OpeningBalance,
        },
    ];

    validate_items(&items);
}

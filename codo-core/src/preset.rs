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
    pub id: u32,
    pub name: String,
    pub tag: Tag,
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
    let required_tags = [
        Tag::Expense,
        Tag::Income,
        Tag::Asset,
        Tag::Liability,
        Tag::OpeningBalance,
    ];
    for tag in required_tags {
        let exists = item.iter().any(|item| item.tag == tag);
        if !exists {
            panic!("エラー: {:?} が登録されていません", tag);
        }
    }
    let mut ids = HashSet::new();
    let mut names = HashSet::new();
    for item in &item {
        if !ids.insert(&item.id) {
            panic!("ID {} が重複しています", item.id)
        }
        if !names.insert(&item.name) {
            panic!("{} が重複しています", item.name)
        }
    }
    item
}

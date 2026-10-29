//! プリセット品目
//!
//! 新しい帳簿を作るときに最初から登録しておく品目を提供する.

use crate::item::{ItemList, Tag};

/// プリセット品目の名前とタグ. 登録順に ID が割り当てられる.
const PRESETS: [(&str, Tag); 12] = [
    ("食費", Tag::Expense),
    ("娯楽", Tag::Expense),
    ("日用雑貨", Tag::Expense),
    ("衣服/美容", Tag::Expense),
    ("定期支出", Tag::Expense),
    ("その他", Tag::Expense),
    ("給与", Tag::Income),
    ("その他収入", Tag::Income),
    ("現金", Tag::Asset),
    ("銀行口座", Tag::Asset),
    ("クレジットカード", Tag::Liability),
    ("初期残高", Tag::OpeningBalance),
];

/// プリセット品目を登録した一覧を返す.
pub fn preset_items() -> ItemList {
    let mut items = ItemList::new();
    for (name, tag) in PRESETS {
        items
            .add(name, tag)
            .expect("プリセット品目は有効なので到達不能");
    }
    items
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `PRESETS` の名前が空だったり重複したりしていればパニックする.
    #[test]
    fn preset_items_test() {
        preset_items();
    }
}

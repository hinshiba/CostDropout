//! 品目

use thiserror::Error;

/// 品目 ID(連番)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ItemId(pub u32);

/// 分類タグ
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tag {
    /// 資産
    Asset,
    /// 負債
    Liability,
    /// 初期残高(純資産)
    OpeningBalance,
    /// 収入(収益)
    Income,
    /// 支出(費用)
    Expense,
}

impl Tag {
    pub const ALL_TAGS: [Tag; 5] = [
        Tag::Asset,
        Tag::Liability,
        Tag::OpeningBalance,
        Tag::Income,
        Tag::Expense,
    ];

    /// タグの表示名
    /// Tag::Income.label()のように使用
    pub fn label(&self) -> &'static str {
        match self {
            Tag::Asset => "資産",
            Tag::Liability => "負債",
            Tag::OpeningBalance => "初期残高",
            Tag::Income => "収入",
            Tag::Expense => "支出",
        }
    }

    /// 取引の品目として選択できるか
    pub fn is_transaction_selectable(&self) -> bool {
        matches!(self, Tag::Income | Tag::Expense | Tag::Liability)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub id: ItemId,
    pub name: String,
    pub tag: Tag,
}

/// 品目の一覧
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ItemList {
    items: Vec<Item>,
}

impl ItemList {
    /// 新しい空の品目リストを作成する
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    /// アプリ再起動時にファイルを復元する
    /// エラー検証はしない
    pub fn from_items(items: Vec<Item>) -> Self {
        Self { items }
    }

    /// Vec<&Item>をId順にして返す
    /// Vec<Item>は並び替えてないので注意
    pub fn iter(&self) -> impl Iterator<Item = &Item> {
        let mut items: Vec<&Item> = self.items.iter().collect();
        items.sort_by_key(|item| item.id);
        items.into_iter()
    }

    /// 同じIDの品目があれば返す
    pub fn get_item(&self, id: ItemId) -> Option<&Item> {
        self.items.iter().find(|item| item.id == id)
    }

    /// 同じ名前の品目があれば返す
    /// tagが等しくない場合でも通る
    pub fn find_by_name(&self, name: &str) -> Option<&Item> {
        let trim_name = name.trim();
        self.items.iter().find(|item| item.name == trim_name)
    }

    /// 品目を追加する
    /// # Returns
    /// nameが空の場合と，名前が被る場合はエラーを返す
    /// - Ok(ItemId) : 追加した品目のID
    pub fn add_item(&mut self, name: &str, tag: Tag) -> Result<ItemId, ItemError> {
        let trim_name = name.trim().to_string();

        if trim_name.is_empty() {
            return Err(ItemError::EmptyName);
        }

        if self.items.iter().any(|item| item.name == trim_name) {
            Err(ItemError::DuplicateName(trim_name))
        } else if self.items.is_empty() {
            self.items.push(Item {
                id: ItemId(1),
                name: trim_name,
                tag,
            });
            Ok(ItemId(1))
        }
        // 既存の品目がある場合は、最大のIDに1を足して新しいIdを作成(削除後に追加する場合を考慮)
        else {
            let new_id: u32 = self.items.iter().map(|item| item.id).max().unwrap().0 + 1;
            self.items.push(Item {
                id: ItemId(new_id),
                name: trim_name,
                tag,
            });
            Ok(ItemId(new_id))
        }
    }

    /// 品目を更新する
    /// nameが空の場合と，名前が被る場合はエラーを返す
    /// nameが変更なければそのまま返す
    pub fn update_item(&mut self, id: ItemId, name: &str, tag: Tag) -> Result<(), ItemError> {
        let trim_name = name.trim().to_string();

        if trim_name.is_empty() {
            return Err(ItemError::EmptyName);
        }

        if self
            .items
            .iter()
            .any(|item| item.name == trim_name && item.id != id)
        {
            return Err(ItemError::DuplicateName(trim_name));
        }

        // 更新対象を探す
        if let Some(item) = self.items.iter_mut().find(|item| item.id == id) {
            // 名前もタグも同じなら更新不要
            if item.name == trim_name && item.tag == tag {
                return Ok(());
            }

            // 更新
            item.name = trim_name;
            item.tag = tag;

            Ok(())
        } else {
            Err(ItemError::NotFound(id))
        }
    }

    /// 指定されたIDの品目を削除する
    pub fn remove_item(&mut self, id: ItemId) -> Result<(), ItemError> {
        if let Some(delete_item) = self.items.iter().position(|item| item.id == id) {
            self.items.remove(delete_item);
            Ok(())
        } else {
            Err(ItemError::NotFound(id))
        }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ItemError {
    #[error("品目名が空です")]
    EmptyName,
    #[error("品目名 {0} は既に使われています")]
    DuplicateName(String),
    #[error("品目 {0:?} が見つかりません")]
    NotFound(ItemId),
}

/// 以下全てテスト用
#[cfg(test)]
mod tests {
    use super::*;

    // =========================
    // Tag のテスト
    // =========================

    #[test]
    fn tag_label_returns_correct_label() {
        assert_eq!(Tag::Asset.label(), "資産");
        assert_eq!(Tag::Liability.label(), "負債");
        assert_eq!(Tag::OpeningBalance.label(), "初期残高");
        assert_eq!(Tag::Income.label(), "収入");
        assert_eq!(Tag::Expense.label(), "支出");
    }

    #[test]
    fn tag_is_transaction_selectable_returns_correct_result() {
        assert!(!Tag::Asset.is_transaction_selectable());
        assert!(Tag::Liability.is_transaction_selectable());
        assert!(!Tag::OpeningBalance.is_transaction_selectable());
        assert!(Tag::Income.is_transaction_selectable());
        assert!(Tag::Expense.is_transaction_selectable());
    }

    // =========================
    // ItemList::new のテスト
    // =========================

    #[test]
    fn new_creates_empty_item_list() {
        let list = ItemList::new();

        assert_eq!(list.iter().count(), 0);
    }

    // =========================
    // from_items のテスト
    // =========================

    #[test]
    fn from_items_restores_items_without_validation() {
        let items = vec![
            Item {
                id: ItemId(2),
                name: "みかん".to_string(),
                tag: Tag::Expense,
            },
            Item {
                id: ItemId(1),
                name: "".to_string(),
                tag: Tag::Asset,
            },
        ];

        let list = ItemList::from_items(items.clone());

        assert_eq!(list.iter().count(), 2);
        assert_eq!(list.get_item(ItemId(1)), Some(&items[1]));
        assert_eq!(list.get_item(ItemId(2)), Some(&items[0]));
    }

    // =========================
    // iter のテスト
    // =========================

    #[test]
    fn iter_returns_items_in_id_order() {
        let items = vec![
            Item {
                id: ItemId(3),
                name: "みかん".to_string(),
                tag: Tag::Expense,
            },
            Item {
                id: ItemId(1),
                name: "りんご".to_string(),
                tag: Tag::Asset,
            },
            Item {
                id: ItemId(2),
                name: "バナナ".to_string(),
                tag: Tag::Income,
            },
        ];

        let list = ItemList::from_items(items);

        let ids: Vec<ItemId> = list.iter().map(|item| item.id).collect();

        assert_eq!(ids, vec![ItemId(1), ItemId(2), ItemId(3)]);
    }

    // =========================
    // get_item のテスト
    // =========================

    #[test]
    fn get_item_returns_item_when_id_exists() {
        let mut list = ItemList::new();

        list.add_item("りんご", Tag::Asset).unwrap();

        let item = list.get_item(ItemId(1));

        assert_eq!(
            item,
            Some(&Item {
                id: ItemId(1),
                name: "りんご".to_string(),
                tag: Tag::Asset,
            })
        );
    }

    #[test]
    fn get_item_returns_none_when_id_does_not_exist() {
        let list = ItemList::new();

        assert_eq!(list.get_item(ItemId(999)), None);
    }

    // =========================
    // find_by_name のテスト
    // =========================

    #[test]
    fn find_by_name_returns_item_when_name_exists() {
        let mut list = ItemList::new();

        list.add_item("りんご", Tag::Asset).unwrap();

        let item = list.find_by_name("りんご");

        assert_eq!(item.unwrap().name, "りんご");
    }

    #[test]
    fn find_by_name_trims_input_name() {
        let mut list = ItemList::new();

        list.add_item("りんご", Tag::Asset).unwrap();

        let item = list.find_by_name("  りんご  ");

        assert_eq!(item.unwrap().name, "りんご");
    }

    // =========================
    // add_item の正常系テスト
    // =========================

    #[test]
    fn add_item_adds_item_with_id_one() {
        let mut list = ItemList::new();

        let result = list.add_item("　　りんご　　", Tag::Asset);

        assert_eq!(result, Ok(ItemId(1)));

        let item = list.get_item(ItemId(1)).unwrap();

        assert_eq!(item.name, "りんご");
        assert_eq!(item.tag, Tag::Asset);
    }

    #[test]
    fn add_item_assigns_next_id() {
        let mut list = ItemList::new();

        list.add_item("りんご", Tag::Asset).unwrap();
        list.add_item("みかん", Tag::Expense).unwrap();

        assert!(list.get_item(ItemId(1)).is_some());
        assert!(list.get_item(ItemId(2)).is_some());
    }

    #[test]
    fn add_item_uses_max_id_plus_one_after_deletion() {
        let mut list = ItemList::new();

        list.add_item("りんご", Tag::Asset).unwrap();
        list.add_item("みかん", Tag::Expense).unwrap();
        list.add_item("バナナ", Tag::Income).unwrap();

        list.remove_item(ItemId(2)).unwrap();

        let result = list.add_item("ぶどう", Tag::Asset);

        assert_eq!(result, Ok(ItemId(4)));
    }

    // =========================
    // add_item のエラー系テスト
    // =========================

    #[test]
    fn add_item_returns_empty_name_error() {
        let mut list = ItemList::new();

        let result = list.add_item("", Tag::Asset);

        assert_eq!(result, Err(ItemError::EmptyName));
    }

    #[test]
    fn add_item_returns_empty_name_error_when_only_spaces() {
        let mut list = ItemList::new();

        let result = list.add_item("   ", Tag::Asset);

        assert_eq!(result, Err(ItemError::EmptyName));
    }

    #[test]
    fn add_item_returns_duplicate_name_error() {
        let mut list = ItemList::new();

        list.add_item("りんご", Tag::Asset).unwrap();

        let result = list.add_item("りんご", Tag::Expense);

        assert_eq!(result, Err(ItemError::DuplicateName("りんご".to_string())));
    }

    // =========================
    // update_item の正常系テスト
    // =========================

    #[test]
    fn update_item_updates_name_and_tag() {
        let mut list = ItemList::new();

        list.add_item("りんご", Tag::Asset).unwrap();

        let result = list.update_item(ItemId(1), "みかん", Tag::Expense);

        assert_eq!(result, Ok(()));

        let item = list.get_item(ItemId(1)).unwrap();

        assert_eq!(item.name, "みかん");
        assert_eq!(item.tag, Tag::Expense);
    }

    #[test]
    fn update_item_trims_name() {
        let mut list = ItemList::new();

        list.add_item("りんご", Tag::Asset).unwrap();

        list.update_item(ItemId(1), "  みかん  ", Tag::Expense)
            .unwrap();

        assert_eq!(list.get_item(ItemId(1)).unwrap().name, "みかん");
    }

    #[test]
    fn update_item_allows_same_name_for_same_item() {
        let mut list = ItemList::new();

        list.add_item("りんご", Tag::Asset).unwrap();

        let result = list.update_item(ItemId(1), "食費", Tag::Asset);

        assert_eq!(result, Ok(()));
    }

    // =========================
    // update_item のエラー系テスト
    // =========================

    #[test]
    fn update_item_returns_empty_name_error() {
        let mut list = ItemList::new();

        list.add_item("りんご", Tag::Asset).unwrap();

        let result = list.update_item(ItemId(1), "", Tag::Asset);

        assert_eq!(result, Err(ItemError::EmptyName));
    }

    #[test]
    fn update_item_returns_empty_name_error_when_only_spaces() {
        let mut list = ItemList::new();

        list.add_item("りんご", Tag::Asset).unwrap();

        let result = list.update_item(ItemId(1), "   ", Tag::Asset);

        assert_eq!(result, Err(ItemError::EmptyName));
    }

    #[test]
    fn update_item_returns_duplicate_name_error() {
        let mut list = ItemList::new();

        list.add_item("りんご", Tag::Asset).unwrap();
        list.add_item("みかん", Tag::Expense).unwrap();

        let result = list.update_item(ItemId(2), "りんご", Tag::Expense);

        assert_eq!(result, Err(ItemError::DuplicateName("りんご".to_string())));
    }

    #[test]
    fn update_item_returns_not_found_error() {
        let mut list = ItemList::new();

        let result = list.update_item(ItemId(999), "りんご", Tag::Asset);

        assert_eq!(result, Err(ItemError::NotFound(ItemId(999))));
    }

    // =========================
    // remove_item の正常系テスト
    // =========================

    #[test]
    fn remove_item_removes_item() {
        let mut list = ItemList::new();

        list.add_item("りんご", Tag::Asset).unwrap();

        let result = list.remove_item(ItemId(1));

        assert_eq!(result, Ok(()));
        assert_eq!(list.get_item(ItemId(1)), None);
    }

    #[test]
    fn remove_item_does_not_change_other_items() {
        let mut list = ItemList::new();

        list.add_item("りんご", Tag::Asset).unwrap();
        list.add_item("みかん", Tag::Expense).unwrap();

        list.remove_item(ItemId(1)).unwrap();

        assert!(list.get_item(ItemId(2)).is_some());
        assert_eq!(list.get_item(ItemId(2)).unwrap().name, "みかん");
    }

    // =========================
    // remove_item のエラー系テスト
    // =========================

    #[test]
    fn remove_item_returns_not_found_error() {
        let mut list = ItemList::new();

        let result = list.remove_item(ItemId(999));

        assert_eq!(result, Err(ItemError::NotFound(ItemId(999))));
    }
}

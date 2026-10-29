//! 品目のモデルと品目一覧

use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

/// 品目を識別する連番.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ItemId(u32);

/// 品目の分類.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tag {
    /// 資産.
    Asset,
    /// 負債.
    Liability,
    /// 初期残高 (純資産).
    OpeningBalance,
    /// 収入 (収益).
    Income,
    /// 支出 (費用).
    Expense,
}

impl Tag {
    /// すべてのタグを表示順に並べたもの.
    pub const ALL: [Tag; 5] = [
        Tag::Asset,
        Tag::Liability,
        Tag::OpeningBalance,
        Tag::Income,
        Tag::Expense,
    ];

    /// 取引の品目として選べるかを判定する.
    ///
    /// 収入, 支出, 負債だけが選べる.
    pub fn is_transaction_selectable(&self) -> bool {
        matches!(self, Tag::Income | Tag::Expense | Tag::Liability)
    }
}

impl fmt::Display for Tag {
    /// タグの表示名を出力する.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Tag::Asset => "資産",
            Tag::Liability => "負債",
            Tag::OpeningBalance => "初期残高",
            Tag::Income => "収入",
            Tag::Expense => "支出",
        };
        f.write_str(label)
    }
}

/// 品目.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Item {
    id: ItemId,
    name: String,
    tag: Tag,
}

impl Item {
    pub fn id(&self) -> ItemId {
        self.id
    }

    pub(crate) fn tag(&self) -> Tag {
        self.tag
    }
}

impl fmt::Display for Item {
    /// 品目名を出力する.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)
    }
}

/// 品目一覧.
///
/// 次の要件を満たす.
/// * ID の昇順に並ぶ
/// * 名前は前後に空白がなく空でない
/// * 互いに異なる ID と名前をもつ
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ItemList {
    items: Vec<Item>,
}

impl ItemList {
    /// 新しい空の品目リストを作成する
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    /// `Vec<Item>` から品目一覧を作成する.
    ///
    /// 名前の前後の空白は取り除く.
    ///
    /// # Errors
    ///
    /// 空白を取り除いた名前が空の場合は `ItemError::EmptyName` エラー.
    /// 空白を取り除いた名前が重複する場合は `ItemError::DuplicateName` エラー.
    /// ID が重複する場合は `ItemError::DuplicateId` エラー.
    pub(crate) fn from_items(mut items: Vec<Item>) -> Result<Self, ItemError> {
        for item in &mut items {
            item.name = item.name.trim().to_string();
        }

        let mut names = HashSet::new();
        for item in &items {
            if item.name.is_empty() {
                return Err(ItemError::EmptyName);
            }
            if !names.insert(item.name.as_str()) {
                return Err(ItemError::DuplicateName(item.name.clone()));
            }
        }

        items.sort_by_key(|item| item.id);
        if let Some(pair) = items.windows(2).find(|pair| pair[0].id == pair[1].id) {
            return Err(ItemError::DuplicateId(pair[0].id));
        }
        Ok(Self { items })
    }

    /// 品目を走査するイテレーターを返す.
    pub fn iter(&self) -> impl Iterator<Item = &Item> {
        self.items.iter()
    }

    /// タグが `tag` である品目を走査するイテレーターを返す.
    pub fn iter_with_tag(&self, tag: Tag) -> impl Iterator<Item = &Item> {
        self.items.iter().filter(move |item| item.tag == tag)
    }

    /// ID が `id` である品目を返す.
    pub fn get(&self, id: ItemId) -> Option<&Item> {
        self.items.iter().find(|item| item.id == id)
    }

    /// 品目を追加し, 最大の ID に1を足した ID を割り当てる.
    ///
    /// 名前の前後の空白は取り除く.
    ///
    /// # Errors
    ///
    /// 空白を取り除いた名前が空の場合は `ItemError::EmptyName` エラー.
    /// 他の品目と名前が同じ場合は `ItemError::DuplicateName` エラー.
    /// 割り当てる ID が `u32` の範囲を超える場合は `ItemError::Overflow` エラー.
    pub fn add(&mut self, name: &str, tag: Tag) -> Result<ItemId, ItemError> {
        let name = self.validate_name(name, None)?;
        let max_id = self.items.iter().map(|item| item.id.0).max().unwrap_or(0);
        let id = ItemId(max_id.checked_add(1).ok_or(ItemError::Overflow)?);

        self.items.push(Item {
            id,
            name: name.to_string(),
            tag,
        });
        Ok(id)
    }

    /// 品目の名前とタグを更新する.
    ///
    /// 名前の前後の空白は取り除く.
    ///
    /// # Errors
    ///
    /// 空白を取り除いた名前が空の場合は `ItemError::EmptyName` エラー.
    /// 他の品目と名前が同じ場合は `ItemError::DuplicateName` エラー.
    /// 品目が存在しない場合は `ItemError::NotFound` エラー.
    pub fn update(&mut self, id: ItemId, name: &str, tag: Tag) -> Result<(), ItemError> {
        let name = self.validate_name(name, Some(id))?;
        let index = self.index_of(id)?;

        let item = &mut self.items[index];
        item.name = name.to_string();
        item.tag = tag;
        Ok(())
    }

    /// 品目を削除する.
    ///
    /// # Errors
    ///
    /// 品目が存在しない場合は `ItemError::NotFound` エラー.
    pub fn remove(&mut self, id: ItemId) -> Result<(), ItemError> {
        let index = self.index_of(id)?;
        self.items.remove(index);
        Ok(())
    }

    fn index_of(&self, id: ItemId) -> Result<usize, ItemError> {
        self.items
            .iter()
            .position(|item| item.id == id)
            .ok_or(ItemError::NotFound(id))
    }

    /// 前後の空白を取り除いた名前を返す.
    ///
    /// `id` の品目は重複の判定から除く.
    ///
    /// # Errors
    ///
    /// 空白を取り除いた名前が空の場合は `ItemError::EmptyName` エラー.
    /// 他の品目と名前が同じ場合は `ItemError::DuplicateName` エラー.
    fn validate_name<'a>(&self, name: &'a str, id: Option<ItemId>) -> Result<&'a str, ItemError> {
        let name = name.trim();

        if name.is_empty() {
            return Err(ItemError::EmptyName);
        }

        // 重複だと気づかせるために, 特別に空白を取り除いた名前を入れる
        if self
            .items
            .iter()
            .any(|item| item.name == name && Some(item.id) != id)
        {
            return Err(ItemError::DuplicateName(name.to_string()));
        }
        Ok(name)
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
    #[error("品目IDがオーバーフローしました")]
    Overflow,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---------------------
    // from_items のテスト
    // ---------------------

    #[test]
    fn from_items_sorts_by_id() {
        let food = Item::new(ItemId(1), "食費".to_string(), Tag::Expense);
        let cash = Item::new(ItemId(2), "現金".to_string(), Tag::Asset);

        let reversed = ItemList::from_items(vec![cash.clone(), food.clone()]);
        let sorted = ItemList::from_items(vec![food, cash]);

        assert_eq!(reversed, sorted);
    }

    // ---------------------
    // get_item のテスト
    // ---------------------

    #[test]
    fn get_item_returns_item_when_id_exists() {
        let mut list = ItemList::new();

        list.add("日用品", Tag::Asset).unwrap();
        list.add("食費", Tag::Expense).unwrap();
        list.add("給与", Tag::Income).unwrap();

        let item = list.get_item(ItemId(1));

        assert_eq!(
            item,
            Some(&Item {
                id: ItemId(1),
                name: "日用品".to_string(),
                tag: Tag::Asset,
            })
        );
    }

    #[test]
    fn get_item_returns_none_when_id_does_not_exist() {
        let list = ItemList::new();

        assert_eq!(list.get_item(ItemId(999)), None);
    }

    // ---------------------
    // find_by_name のテスト
    // ---------------------

    #[test]
    fn find_by_name_trims_input_name() {
        let mut list = ItemList::new();

        list.add("日用品", Tag::Asset).unwrap();
        list.add("食費", Tag::Expense).unwrap();
        list.add("給与", Tag::Income).unwrap();

        let item = list.find_by_name("  日用品  ");

        assert_eq!(item.unwrap().name, "日用品");
    }

    // ---------
    // add_item
    // ---------

    #[test]
    fn add_item_adds_item_with_id_one() {
        let mut list = ItemList::new();

        let result = list.add("　　食費　　", Tag::Asset);

        assert_eq!(result, Ok(ItemId(1)));

        let item = list.get_item(ItemId(1)).unwrap();

        assert_eq!(item.name, "食費");
        assert_eq!(item.tag, Tag::Asset);
    }

    #[test]
    fn add_item_uses_max_id_plus_one_after_deletion() {
        let mut list = ItemList::new();

        list.add("日用品", Tag::Asset).unwrap();
        list.add("娯楽", Tag::Expense).unwrap();
        list.add("食費", Tag::Income).unwrap();

        list.remove(ItemId(2)).unwrap();

        let result = list.add("住宅", Tag::Asset);

        assert_eq!(result, Ok(ItemId(4)));
    }

    #[test]
    fn add_item_returns_empty_name_error_when_only_spaces() {
        let mut list = ItemList::new();

        let result = list.add("   ", Tag::Asset);

        assert_eq!(result, Err(ItemError::EmptyName));
    }

    #[test]
    fn add_item_returns_duplicate_name_error() {
        let mut list = ItemList::new();

        list.add("食費", Tag::Asset).unwrap();

        let result = list.add("食費", Tag::Expense);

        assert_eq!(result, Err(ItemError::DuplicateName("食費".to_string())));
    }

    // -----------
    // update_item
    // -----------

    #[test]
    fn update_item_trims_name() {
        let mut list = ItemList::new();

        list.add("食費", Tag::Asset).unwrap();

        list.update(ItemId(1), "  娯楽  ", Tag::Expense).unwrap();

        assert_eq!(list.get_item(ItemId(1)).unwrap().name, "娯楽");
    }

    #[test]
    fn update_item_allows_same_name_for_same_item() {
        let mut list = ItemList::new();

        list.add("食費", Tag::Asset).unwrap();

        let result = list.update(ItemId(1), "食費", Tag::Asset);

        assert_eq!(result, Ok(()));
    }

    #[test]
    fn update_item_returns_empty_name_error_when_only_spaces() {
        let mut list = ItemList::new();

        list.add("食費", Tag::Asset).unwrap();

        let result = list.update(ItemId(1), "   ", Tag::Asset);

        assert_eq!(result, Err(ItemError::EmptyName));
    }

    #[test]
    fn update_item_returns_duplicate_name_error() {
        let mut list = ItemList::new();

        list.add("食費", Tag::Asset).unwrap();
        list.add("娯楽", Tag::Expense).unwrap();

        let result = list.update(ItemId(2), "食費", Tag::Expense);

        assert_eq!(result, Err(ItemError::DuplicateName("食費".to_string())));
    }

    #[test]
    fn update_item_returns_not_found_error() {
        let mut list = ItemList::new();

        let result = list.update(ItemId(999), "食費", Tag::Asset);

        assert_eq!(result, Err(ItemError::NotFound(ItemId(999))));
    }

    // -----------
    // remove_item
    // -----------

    #[test]
    fn remove_item_removes_item() {
        let mut list = ItemList::new();

        list.add("食費", Tag::Asset).unwrap();
        list.add("娯楽", Tag::Expense).unwrap();
        let result = list.remove(ItemId(1));

        assert_eq!(result, Ok(()));
        let remaining_ids = list.iter().map(Item::id).collect::<Vec<_>>();
        assert_eq!(remaining_ids, vec![ItemId(2)]);
        assert_eq!(list.get_item(ItemId(2)).unwrap().name, "娯楽");
    }

    #[test]
    fn remove_item_returns_not_found_error() {
        let mut list = ItemList::new();

        let result = list.remove(ItemId(999));

        assert_eq!(result, Err(ItemError::NotFound(ItemId(999))));
    }
}

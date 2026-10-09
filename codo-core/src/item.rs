//! 品目

use thiserror::Error;

/// 品目 ID(連番)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ItemId(u32);

impl ItemId {
    pub const fn id(&self) -> u32 {
        self.0
    }
}
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
    id: ItemId,
    name: String,
    tag: Tag,
}

impl Item {
    pub fn id(&self) -> ItemId {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn tag(&self) -> Tag {
        self.tag
    }
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

    /// 同じIDの品目があれば返す
    pub fn get_item(&self, id: ItemId) -> Option<&Item> {
        self.items.iter().find(|item| item.id == id)
    }

    /// 同じ名前の品目があれば返す
    pub fn find_by_name(&self, name: &str) -> Option<&Item> {
        let trim_name = name.trim();
        self.items.iter().find(|item| item.name == trim_name)
    }

    /// 品目を追加する
    ///
    /// # Returns
    ///
    /// nameが空の場合と，名前が被る場合はエラーを返す
    pub fn add_item(&mut self, name: &str, tag: Tag) -> Result<ItemId, ItemError> {
        let trim_name = name.trim();

        if trim_name.is_empty() {
            return Err(ItemError::EmptyName);
        }

        // 特別に，重複だと気づかせるためにtrim_nameを使う
        if self.find_by_name(trim_name).is_some() {
            return Err(ItemError::DuplicateName(trim_name.to_string()));
        }

        // 既存の品目がある場合は、新しくIdを作り，ない場合はItemId(0)となる
        let max_id = self
            .items
            .iter()
            .map(|item| item.id)
            .max()
            .unwrap_or(ItemId(0))
            .0;

        if max_id == u32::MAX {
            return Err(ItemError::Overflow);
        }
        let new_id = max_id + 1;
        self.items.push(Item {
            id: ItemId(new_id),
            name: trim_name.to_string(),
            tag,
        });
        Ok(ItemId(new_id))
    }

    /// 品目を更新する
    /// nameが空の場合と，名前が被る場合はエラーを返す
    /// nameが変更なければそのまま返す
    pub fn update_item(&mut self, id: ItemId, name: &str, tag: Tag) -> Result<(), ItemError> {
        let trim_name = name.trim();

        if trim_name.is_empty() {
            return Err(ItemError::EmptyName);
        }

        // まず更新対象が存在するか確認
        if self.get_item(id).is_none() {
            return Err(ItemError::NotFound(id));
        }

        if self
            .find_by_name(trim_name)
            .is_some_and(|item| item.id != id)
        {
            return Err(ItemError::DuplicateName(trim_name.to_string()));
        }

        // 更新対象を探す
        let Some(item) = self.items.iter_mut().find(|item| item.id == id) else {
            return Err(ItemError::NotFound(id));
        };
        // 名前もタグも同じなら更新不要
        if item.name == trim_name && item.tag == tag {
            return Ok(());
        }

        // 更新
        item.name = trim_name.to_string();
        item.tag = tag;

        Ok(())
    }

    /// 指定されたIDの品目を削除する
    pub fn remove_item(&mut self, id: ItemId) -> Result<(), ItemError> {
        if let Some(delete_pos) = self.items.iter().position(|item| item.id == id) {
            self.items.remove(delete_pos);
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
    #[error("品目IDがオーバーフローしました")]
    Overflow,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---------------------
    // get_item のテスト
    // ---------------------

    #[test]
    fn get_item_returns_item_when_id_exists() {
        let mut list = ItemList::new();

        list.add_item("日用品", Tag::Asset).unwrap();
        list.add_item("食費", Tag::Expense).unwrap();
        list.add_item("給与", Tag::Income).unwrap();

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

        list.add_item("日用品", Tag::Asset).unwrap();
        list.add_item("食費", Tag::Expense).unwrap();
        list.add_item("給与", Tag::Income).unwrap();

        let item = list.find_by_name("  日用品  ");

        assert_eq!(item.unwrap().name, "日用品");
    }

    // ---------
    // add_item
    // ---------

    #[test]
    fn add_item_adds_item_with_id_one() {
        let mut list = ItemList::new();

        let result = list.add_item("　　食費　　", Tag::Asset);

        assert_eq!(result, Ok(ItemId(1)));

        let item = list.get_item(ItemId(1)).unwrap();

        assert_eq!(item.name, "食費");
        assert_eq!(item.tag, Tag::Asset);
    }

    #[test]
    fn add_item_uses_max_id_plus_one_after_deletion() {
        let mut list = ItemList::new();

        list.add_item("日用品", Tag::Asset).unwrap();
        list.add_item("娯楽", Tag::Expense).unwrap();
        list.add_item("食費", Tag::Income).unwrap();

        list.remove_item(ItemId(2)).unwrap();

        let result = list.add_item("住宅", Tag::Asset);

        assert_eq!(result, Ok(ItemId(4)));
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

        list.add_item("食費", Tag::Asset).unwrap();

        let result = list.add_item("食費", Tag::Expense);

        assert_eq!(result, Err(ItemError::DuplicateName("食費".to_string())));
    }

    // -----------
    // update_item
    // -----------

    #[test]
    fn update_item_trims_name() {
        let mut list = ItemList::new();

        list.add_item("食費", Tag::Asset).unwrap();

        list.update_item(ItemId(1), "  娯楽  ", Tag::Expense)
            .unwrap();

        assert_eq!(list.get_item(ItemId(1)).unwrap().name, "娯楽");
    }

    #[test]
    fn update_item_allows_same_name_for_same_item() {
        let mut list = ItemList::new();

        list.add_item("食費", Tag::Asset).unwrap();

        let result = list.update_item(ItemId(1), "食費", Tag::Asset);

        assert_eq!(result, Ok(()));
    }

    #[test]
    fn update_item_returns_empty_name_error_when_only_spaces() {
        let mut list = ItemList::new();

        list.add_item("食費", Tag::Asset).unwrap();

        let result = list.update_item(ItemId(1), "   ", Tag::Asset);

        assert_eq!(result, Err(ItemError::EmptyName));
    }

    #[test]
    fn update_item_returns_duplicate_name_error() {
        let mut list = ItemList::new();

        list.add_item("食費", Tag::Asset).unwrap();
        list.add_item("娯楽", Tag::Expense).unwrap();

        let result = list.update_item(ItemId(2), "食費", Tag::Expense);

        assert_eq!(result, Err(ItemError::DuplicateName("食費".to_string())));
    }

    #[test]
    fn update_item_returns_not_found_error() {
        let mut list = ItemList::new();

        let result = list.update_item(ItemId(999), "食費", Tag::Asset);

        assert_eq!(result, Err(ItemError::NotFound(ItemId(999))));
    }

    // -----------
    // remove_item
    // -----------

    #[test]
    fn remove_item_removes_item() {
        let mut list = ItemList::new();

        list.add_item("食費", Tag::Asset).unwrap();
        list.add_item("娯楽", Tag::Expense).unwrap();
        let result = list.remove_item(ItemId(1));

        assert_eq!(result, Ok(()));
        assert_eq!(list.get_item(ItemId(1)), None);
        assert_eq!(list.get_item(ItemId(2)).unwrap().name, "娯楽");
    }

    #[test]
    fn remove_item_returns_not_found_error() {
        let mut list = ItemList::new();

        let result = list.remove_item(ItemId(999));

        assert_eq!(result, Err(ItemError::NotFound(ItemId(999))));
    }
}

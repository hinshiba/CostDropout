//! 品目のモデルと品目一覧

use std::collections::HashSet;
use std::fmt;
use thiserror::Error;

/// 品目を識別する連番.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ItemId(pub(crate) u32);

/// 品目の分類.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    id: ItemId,
    name: String,
    tag: Tag,
}

impl Item {
    pub(crate) fn new(id: ItemId, name: String, tag: Tag) -> Self {
        Self { id, name, tag }
    }

    pub fn id(&self) -> ItemId {
        self.id
    }

    pub(crate) fn name(&self) -> &str {
        &self.name
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
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ItemList {
    items: Vec<Item>,
}

impl ItemList {
    /// 新しい空の品目一覧を作成する.
    pub(crate) fn new() -> Self {
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
    pub(crate) fn add(&mut self, name: &str, tag: Tag) -> Result<ItemId, ItemError> {
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
    pub(crate) fn update(&mut self, id: ItemId, name: &str, tag: Tag) -> Result<(), ItemError> {
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
    pub(crate) fn remove(&mut self, id: ItemId) -> Result<(), ItemError> {
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
    /// 品目名が空の場合.
    #[error("品目名が空です")]
    EmptyName,
    /// 品目名が他の品目と同じ場合.
    #[error("品目名 {0} は既に使われています")]
    DuplicateName(String),
    /// 品目 ID が他の品目と同じ場合.
    #[error("品目 {0:?} が重複しています")]
    DuplicateId(ItemId),
    /// 品目が存在しない場合.
    #[error("品目 {0:?} が見つかりません")]
    NotFound(ItemId),
    /// 割り当てる ID が `u32` の範囲を超える場合.
    #[error("品目IDがオーバーフローしました")]
    Overflow,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: u32, name: &str, tag: Tag) -> Item {
        Item {
            id: ItemId(id),
            name: name.to_string(),
            tag,
        }
    }

    /// 食費 (1), 娯楽 (2), 現金 (3) の一覧.
    fn list() -> ItemList {
        ItemList::from_items(vec![
            item(1, "食費", Tag::Expense),
            item(2, "娯楽", Tag::Expense),
            item(3, "現金", Tag::Asset),
        ])
        .unwrap()
    }

    #[test]
    fn from_items_test() {
        let food = item(1, "食費", Tag::Expense);
        let cash = item(2, "現金", Tag::Asset);

        let list = ItemList::from_items(vec![
            item(2, "現金", Tag::Asset),
            item(1, "　 食費 \t", Tag::Expense),
        ])
        .unwrap();

        assert_eq!(list.iter().collect::<Vec<_>>(), vec![&food, &cash]);
        assert_eq!(
            list.iter_with_tag(Tag::Asset).collect::<Vec<_>>(),
            vec![&cash]
        );
    }

    #[test]
    fn from_items_error_test() {
        for (items, expected) in [
            (vec![item(1, "　 \t", Tag::Expense)], ItemError::EmptyName),
            (
                vec![item(1, "食費", Tag::Expense), item(2, " 食費 ", Tag::Asset)],
                ItemError::DuplicateName("食費".to_string()),
            ),
            (
                vec![item(1, "食費", Tag::Expense), item(1, "現金", Tag::Asset)],
                ItemError::DuplicateId(ItemId(1)),
            ),
        ] {
            assert_eq!(ItemList::from_items(items), Err(expected));
        }
    }

    #[test]
    fn add_test() {
        let mut list = ItemList::new();

        for (name, tag, expected) in [
            ("食費", Tag::Expense, item(1, "食費", Tag::Expense)),
            ("　 給与 \t", Tag::Income, item(2, "給与", Tag::Income)),
            ("現 金", Tag::Asset, item(3, "現 金", Tag::Asset)),
        ] {
            let id = list.add(name, tag).unwrap();
            assert_eq!(list.get(id), Some(&expected), "{name}");
        }
    }

    #[test]
    fn add_after_remove_test() {
        let mut list = list();

        list.remove(ItemId(2)).unwrap();
        assert_eq!(list.add("住宅", Tag::Expense), Ok(ItemId(4)));
    }

    #[test]
    fn add_max_id_test() {
        let mut list =
            ItemList::from_items(vec![item(u32::MAX - 1, "食費", Tag::Expense)]).unwrap();

        assert_eq!(list.add("現金", Tag::Asset), Ok(ItemId(u32::MAX)));

        let before = list.clone();
        assert_eq!(list.add("娯楽", Tag::Expense), Err(ItemError::Overflow));
        assert_eq!(list, before);
    }

    #[test]
    fn add_error_test() {
        let mut list = list();
        let before = list.clone();

        for (name, expected) in [
            ("", ItemError::EmptyName),
            ("　 \t", ItemError::EmptyName),
            ("食費", ItemError::DuplicateName("食費".to_string())),
            (" 現金 ", ItemError::DuplicateName("現金".to_string())),
        ] {
            assert_eq!(list.add(name, Tag::Income), Err(expected), "{name}");
            assert_eq!(list, before, "{name}");
        }
    }

    #[test]
    fn update_test() {
        for (name, tag, expected) in [
            ("  外食  ", Tag::Expense, item(1, "外食", Tag::Expense)),
            // 自分自身の名前とは重複しない
            ("食費", Tag::Income, item(1, "食費", Tag::Income)),
        ] {
            let mut list = list();

            list.update(ItemId(1), name, tag).unwrap();

            assert_eq!(list.get(ItemId(1)), Some(&expected), "{name}");
            assert_eq!(list.iter().count(), 3, "{name}");
        }
    }

    #[test]
    fn update_error_test() {
        let mut list = list();
        let before = list.clone();

        for (id, name, expected) in [
            (ItemId(1), "", ItemError::EmptyName),
            (ItemId(1), "　 \t", ItemError::EmptyName),
            (
                ItemId(1),
                " 娯楽 ",
                ItemError::DuplicateName("娯楽".to_string()),
            ),
            (ItemId(4), "住宅", ItemError::NotFound(ItemId(4))),
        ] {
            assert_eq!(list.update(id, name, Tag::Income), Err(expected), "{name}");
            assert_eq!(list, before, "{name}");
        }
    }

    #[test]
    fn remove_test() {
        let mut list = list();

        list.remove(ItemId(2)).unwrap();

        let ids = list.iter().map(Item::id).collect::<Vec<_>>();
        assert_eq!(ids, vec![ItemId(1), ItemId(3)]);
    }

    #[test]
    fn remove_error_test() {
        let mut list = list();
        let before = list.clone();

        assert_eq!(list.remove(ItemId(4)), Err(ItemError::NotFound(ItemId(4))));
        assert_eq!(list, before);
    }
}

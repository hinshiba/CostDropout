//! 画面切り替え
use crate::pages::{
    items::Items, transaction_form::TransactionForm, transaction_list::TransactionList,
};
use crate::state::AppState;

/// 現在の画面のenum
#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    TransactionForm,
    TransactionList,
    Items,
}

pub struct App {
    state: Entity<AppState>,
    page: Page,
}

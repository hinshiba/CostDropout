//! アプリ全体で共有するデータ
use codo-core::ledger::Ledger;

pub struct AppState {
    pub ledger: Ledger,
}

//! `--demo` 起動で表示確認に使う、メモリ上のサンプル帳簿。

use chrono::Local;
use codo_core::{ledger::Ledger, money::Money, period::YearMonth, transaction::TransactionDraft};

pub fn ledger() -> Ledger {
    let mut ledger = Ledger::with_presets();
    let month = YearMonth::from_date(Local::now().date_naive());
    let cash = ledger.items().find_by_name("現金").unwrap().id();

    for (month, name, amount) in [
        (month, "食費", 30_000),
        (month, "娯楽", 15_000),
        (month, "日用雑貨", 5_000),
        (month.prev(), "食費", 20_000),
        (month.prev(), "娯楽", 20_000),
    ] {
        let item = ledger.items().find_by_name(name).unwrap().id();
        ledger
            .add_transaction(TransactionDraft::new(
                month.first_day(),
                Money(amount),
                item,
                cash,
                "表示確認用".into(),
            ))
            .expect("サンプル取引を追加できませんでした");
    }
    ledger
}

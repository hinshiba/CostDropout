# 設計メモ

docs/plan.md を実装するにあたって決めたことと，そのトレードオフ．

## crate 構成

| crate       | 役割                                                               |
| ----------- | ------------------------------------------------------------------ |
| `codo-core` | ドメインモデル，集計，帳簿の保存と読込，git 操作．GUI に依存しない |
| `codo-gui`  | gpui による GUI．実行ファイル名は `cost-dropout`                   |

- GUI から使う機能はすべて `codo-core` に置き，テストも `codo-core` で書く
- エラー型は集約しない．各モジュールが自分のエラー型を持つ(例: `item::ItemError`)．上位のモジュールは下位のエラーを `#[from]` で包む

## GUI

`gpui-kit` 0.7 を使う．中身は gpui の fork である `gpui-pre` と，部品集の `gpui-component`．

- 採用理由: 素の gpui にはテキスト入力，セレクト，表，日付選択，グラフの部品がなく，取引入力や円グラフを自作することになる．gpui-component には `BarChart`，`PieChart`，`Calendar`，`DatePicker`，`Table`，`Input`，`Select` がある
- トレードオフ: zed 本体の gpui (crates.io の `gpui`) とはバージョンがずれる．gpui 公式のサンプルがそのまま動かないことがある
- ドキュメント: <https://docs.rs/gpui-kit>，<https://docs.rs/gpui-component>

## ID

| 型              | 中身       | 理由                                                                                                |
| --------------- | ---------- | --------------------------------------------------------------------------------------------------- |
| `ItemId`        | `u32` 連番 | 品目は `items.toml` 1ファイルにあるので，同時に追加すると git の競合として必ず検出される            |
| `BudgetId`      | `u32` 連番 | 同上(`budgets.toml`)                                                                                |
| `TransactionId` | uuid v4    | 取引は月ごとのファイルに分かれる．別の月に同時追加すると連番は重複するのに git は競合を検出できない |

連番は既存の最大値 + 1 とする．

## 取引

- 金額は正の値のみ．収入か支出かは品目の分類タグで決まる
- 品目に選べるのは収入/支出/負債のタグを持つものだけ
- 相手先は品目と異なる分類タグの品目(例: 食費(支出)を現金(資産)で払う)

## 集計の定義

- 支出: 品目のタグが支出である取引の金額
- 収入: 品目のタグが収入である取引の金額
- 品目のタグが負債である取引(借入や返済)は支出にも収入にも数えない
- 予算と比較するのは期間内の支出の合計

## 品目の削除

取引から参照されている品目は削除できない．エラーを返し，GUI では先に取引の品目を付け替えるよう案内する．

## 帳簿のファイル形式

帳簿は1つの git リポジトリで，次のファイルを TOML で持つ．

```
<帳簿ディレクトリ>/
  items.toml
  budgets.toml
  transactions/
    2026-09.toml
    2026-10.toml
```

```toml
# items.toml
[[items]]
id = 1
name = "食費"
tag = "expense"   # asset / liability / opening_balance / income / expense
```

```toml
# transactions/2026-10.toml
[[transactions]]
id = "0b6f6a4e-3a59-4f5e-9d0e-2f7d1b0c9a11"
date = 2026-10-02
amount = 1200
item = 1
counterpart = 7
comment = "昼食"
```

```toml
# budgets.toml
[[budgets]]
id = 1
amount = 50000
start = 2026-10-01
end = 2026-10-31
```

- 書き出す順序は固定する(品目と予算は ID 順，取引は日付順，同じ日付なら ID 順)．順序が変わると無関係な行まで diff に出て監査しにくくなるため
- 取引を月ごとに分けるので diff は読みやすいが，同じ月に複数人が同時に追記すると pull 時に競合し得る

## git

`git2` crate ではなく `std::process::Command` で `git` コマンドを呼ぶ．

- push の認証を各自の環境の credential helper に任せられる
- 複数人編集の競合は `git push` が fast-forward でないとして拒否されることで検出する
- 実行環境に git がインストールされている必要がある

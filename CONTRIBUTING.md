# 作業ガイド

チームメンバー向けの作業ルールです．

- 仕様: [docs/plan.md](docs/plan.md)
- コーディング規約: [docs/coding.md](docs/coding.md)

## 作業の流れ

- `main` から作業ブランチを切る
- コミットを積む
- PR を作成し，テンプレートに沿って記入する
- レビューと CI 成功を確認してからマージする

`main` へ直接 push しないでください．

## ブランチ

`<prefix>/<内容>` の形式で命名します．内容は英小文字の kebab-case で書きます．

prefix はコミットと共通です (後述の一覧を参照)．

例

```
feat/transaction-delete
fix/budget-period-overflow
doc/contributing
refactor/money-arithmetic
chore/update-deps
```

## コミット

メッセージの先頭に prefix をつけます．

```
<prefix>: <要約>
```

- 要約は日本語で可，末尾に句点はつけない
- 1コミット1目的を心がける

### prefix 一覧

| prefix     | 用途                                     |
| ---------- | ---------------------------------------- |
| `feat`     | 機能の追加                               |
| `fix`      | バグ修正                                 |
| `doc`      | ドキュメントのみの変更                   |
| `refactor` | 振る舞いを変えないコードの整理           |
| `test`     | テストの追加や修正                       |
| `style`    | フォーマットなど，意味を変えない変更     |
| `chore`    | 依存関係の更新やビルド設定など，上記以外 |

### 例

```
feat: 取引の削除を追加
fix: 予算期間の月末判定を修正
feat: 品目一覧画面を追加
refactor: Money の演算を trait 実装に置き換え
test: 品目削除の制約に関するテストを追加
doc: plan.md に予算の仕様を追記
chore: uuid を 1.26 に更新
```

## 依存関係

### バージョン指定

- v1 以上: マイナーバージョンまで指定する (`"1.26"`)
- v1 未満: パッチバージョンまで指定する (`"0.4.45"`)

```toml
# OK
serde = "1.0"
uuid = "1.26"
chrono = "0.4.45"

# NG
serde = "1"          # マイナーがない
uuid = "1.26.0"      # v1以上なのにパッチまで書いている
chrono = "0.4"       # v1未満なのにパッチがない
```

`cargo add` はパッチまで書き込むため，v1 以上のクレートは手動でマイナーまでに直してください．

`Cargo.lock` はコミットに含めます．


## PR

画面右側の事項も埋めてください

- Reviewers: 基本的には hinshiba を
- Assignees: 自分を登録してください
- Labels: 適切なものを
- Milestone: 適切なものを
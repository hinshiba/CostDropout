# コーディング規約

`cargo fmt` と `cargo clippy` は前提とします．

## 空行

次のものの間には空行を1行入れます．

- `impl` ブロック
- 関数 (テスト関数を含む)
- 関連型 (`type Output` など) と関数

```rust
impl Add for HogeCount {
    type Output = HogeCount;

    fn add(self, rhs: HogeCount) -> Self {
        HogeCount(self.0 + rhs.0)
    }
}

impl HogeCount {
    fn foo() {
        // ...
    }

    fn bar() {}
}
```

## 比較

不等号は原則 `<` と `<=` を使い，左が小さくなるように書きます．
数直線上の並びと一致させるためです．

```rust
// OK
0 < x
lo <= x && x < hi

// NG
x > 0
```

また，範囲はできるだけ `<` で表せる半開区間として扱います．
スライスなど多くのデータ構造と同じ扱いにするためです．

## 命名

`bool` の変数は `is_` や `has_` などの助動詞で始めます (`is_empty`，`has_next`，`can_read`，`need_sync`)．
`bool` だとわかるような名前ならなんでもよいですが，慣れないうちは気にしてください．

## 変数

### 不変の変数を同じ名前の可変の変数として再束縛しない

別の名前にするか，`mut` なしで書いてください．
不変として宣言した変数は以降変わらないものとして読まれるので，同じ名前のまま `let mut` で再束縛すると誤読を招くためです．

文字列の前後の空白を除いてから，先頭の `+` を外す場面を例にします．

```rust
// NG: 不変だった s が途中から可変になる
let s = input.trim();
// ...
let mut s = s;
if let Some(rest) = s.strip_prefix('+') {
    s = rest;
}

// OK: 別の名前にする
let s = input.trim();
// ...
let mut body = s;
if let Some(rest) = body.strip_prefix('+') {
    body = rest;
}

// Good: mut なしで書く
let s = input.trim();
// ...
let body = s.strip_prefix('+').unwrap_or(s);
```

### 中身が嘘になる瞬間を作らない

`match` や `if` 式で値を確定させてから一度に束縛してください．
初期値を入れてから条件に応じて書き換えると，書き換えるまでの間は変数が誤った値を持つためです．

```rust
// NG: x が負のとき，if を通るまで sign が誤っている
let mut sign = 1;
if x < 0 {
    sign = -1;
}

// OK
let sign = if x < 0 { -1 } else { 1 };
```

## 制御フロー

真のときの式が空の `if` を書かないでください．

```rust
// NG
if cond {
} else {
    // 何かしらの処理
    foo();
}

// OK
if !cond {
    // 何かしらの処理
    foo();
}
```

## 重複を避ける

同じ処理が既にあるならそれを呼びます (DRY 原則)．
例えば，panic する版は `Result` を返す版を呼んで `expect` します．

## 簡潔に

演算子を実装済みなら，`Add::add(a, b)` のように trait のメソッドを明示せず `a + b` と書きます．

## ドキュメントコメント

境界を含むかどうかを明記します．

```rust
/// 正の値か．0 は含まない
fn is_positive(x: i32) -> bool {
    0 < x
}
```

## エラー

エラーに入力を含めるときは，trim などの加工をする前の値を入れます．
利用者が入力したものをそのまま見せるためです．

## テスト

仕様に書かれた例だけでなく，次のものもテストします．

- 境界値: 0，±1，桁が変わる値 (9 と 10 など)，型の最大値と最小値
- 範囲外: 型の範囲をわずかに超える値と，大きく超える値 (変換の途中で使う型にも収まらない値)
- 拒否すべき入力: 規則ごとに，それに違反する入力を1つ以上
- エラーの中身: エラーの種類だけでなく，保持する値も確かめる

```rust
#[test]
fn parse_error_test() {
    for s in ["", "a", "1a"] {
        assert!(s.parse::<i32>().is_err(), "{s}");
    }
}
```

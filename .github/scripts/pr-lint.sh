#!/usr/bin/env bash
# PR のブランチ名，コミット，リンクした Issue，Cargo.lock の差分を CONTRIBUTING.md の規則に照らす
set -euo pipefail

prefixes='feat|fix|doc|refactor|test|style|chore'
status=0

fail() {
  echo "::error::$1"
  status=1
}

# agent/ は AGENTS.md でエージェントの作業ブランチに使う．
# ブランチ名は PR を作り直さないと直せないので警告に留める
if ! grep -qE "^($prefixes|agent)/[a-z0-9]+(-[a-z0-9]+)*$" <<<"$HEAD_REF"; then
  echo "::warning::ブランチ名 '$HEAD_REF' が '<prefix>/<英小文字の kebab-case>' の形式ではありません"
fi

while read -r subject; do
  if ! grep -qE "^(($prefixes): \S|Merge )" <<<"$subject"; then
    echo "::warning::コミット '$subject' が '<prefix>: <要約>' の形式ではありません"
  fi
done < <(gh pr view "$PR_NUMBER" --json commits --jq '.commits[].messageHeadline')

# Issue のない PR も認める (PR テンプレート)．書いたつもりで無効になっている場合に気づけるよう警告は出す
# 別リポジトリの Issue も閉じられるので，番号ではなく URL で辿る
issues=$(gh pr view "$PR_NUMBER" --json closingIssuesReferences --jq '.closingIssuesReferences[].url')
if [ -z "$issues" ]; then
  echo "::warning::閉じる Issue がありません．Issue に対応する PR なら本文に 'Closes #番号' を書いてください．HTML コメントの中や 'Closes #1,#2' の2つ目以降は無効です"
fi

{
  echo "### この PR が閉じる Issue"
  echo
  if [ -z "$issues" ]; then
    echo "なし"
  else
    echo "変更内容と一致しているか確認してください．"
    echo
    for url in $issues; do
      echo "- $url $(gh issue view "$url" --json title --jq .title)"
    done
  fi
} >>"$GITHUB_STEP_SUMMARY"

files=$(gh pr view "$PR_NUMBER" --json files --jq '.files[].path')
# 依存関係の更新は chore/ で行う (CONTRIBUTING.md)
if grep -qx 'Cargo.lock' <<<"$files" && ! grep -qE '(^|/)Cargo\.toml$' <<<"$files" && [[ $HEAD_REF != chore/* ]]; then
  fail "Cargo.toml を変えずに Cargo.lock が変わっています．git checkout origin/main -- Cargo.lock で戻してください"
fi

exit "$status"

"""CONTRIBUTING.md と docs/coding.md のうち，機械で判定できる規則を検査する

python .github/scripts/check_repo.py で手元でも実行できる．Python 3.11 以上
"""

import re
import subprocess
import sys
import tomllib
from collections.abc import Iterator
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DEP_TABLES = ("dependencies", "dev-dependencies", "build-dependencies")


def tracked(pattern: str) -> list[Path]:
    out = subprocess.run(
        ["git", "ls-files", "--cached", "--others", "--exclude-standard", pattern],
        cwd=ROOT, capture_output=True, text=True, check=True,
    ).stdout
    return [ROOT / line for line in out.splitlines()]


def version_of(spec: object) -> str | None:
    if isinstance(spec, str):
        return spec
    if isinstance(spec, dict):
        return spec.get("version")
    return None


def check_version(req: str) -> str | None:
    """CONTRIBUTING.md の依存関係の規則に反していればその理由を返す"""
    m = re.fullmatch(r"(\d+)\.(\d+)(?:\.(\d+))?", req)
    if not m:
        return "数字とドットだけで書いてください"
    is_patch_given = m.group(3) is not None
    if m.group(1) == "0" and not is_patch_given:
        return "v1 未満はパッチバージョンまで書いてください"
    if m.group(1) != "0" and is_patch_given:
        return "v1 以上はマイナーバージョンまで書いてください"
    return None


def dependency_errors() -> Iterator[str]:
    for manifest in tracked("**Cargo.toml"):
        data = tomllib.loads(manifest.read_text(encoding="utf-8"))
        tables = [data.get(t, {}) for t in DEP_TABLES]
        tables.append(data.get("workspace", {}).get("dependencies", {}))
        for table in tables:
            for name, spec in table.items():
                req = version_of(spec)
                if req is None:
                    continue
                reason = check_version(req)
                if reason:
                    rel = manifest.relative_to(ROOT).as_posix()
                    yield f'{rel}: {name} = "{req}": {reason}'


def declares_mod(parent: Path, name: str) -> bool:
    pattern = rf"^\s*(?:pub(?:\([^)]*\))?\s+)?mod\s+{re.escape(name)}\s*;"
    return re.search(pattern, parent.read_text(encoding="utf-8"), re.M) is not None


def orphan_errors() -> Iterator[str]:
    """どの mod 宣言からも辿れず，コンパイルされない .rs ファイルを返す"""
    for path in tracked("*/src/**.rs"):
        src = ROOT / path.relative_to(ROOT).parts[0] / "src"
        rel = path.relative_to(src)
        if rel.parts[0] == "bin" or rel.as_posix() in ("lib.rs", "main.rs"):
            continue
        parts = rel.with_suffix("").parts
        if parts[-1] == "mod":
            parts = parts[:-1]
        name, parent_parts = parts[-1], parts[:-1]
        if parent_parts:
            base = src.joinpath(*parent_parts[:-1])
            parents = [base / f"{parent_parts[-1]}.rs", base / parent_parts[-1] / "mod.rs"]
        else:
            parents = [src / "lib.rs", src / "main.rs"]
        if not any(p.exists() and declares_mod(p, name) for p in parents):
            rel_path = path.relative_to(ROOT).as_posix()
            yield f"{rel_path}: `mod {name};` がどこにもなく，コンパイルされていません"


def main() -> int:
    sys.stdout.reconfigure(encoding="utf-8")
    errors = [*dependency_errors(), *orphan_errors()]
    for e in errors:
        print(f"::error::{e}")
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())

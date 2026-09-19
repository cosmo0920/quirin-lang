#!/usr/bin/env python3
"""Constraint-14 census for card `OWNED-FIELD-ROUTE-REACH`, run rather than
assumed.

The card reserves **no word and no token**: the copy bind `let NAME = NAME;`
is four things the lexer already produces. What it *does* do is claim a token
shape that was a `Q051` refusal before, so the question the census has to
answer is not "is a word free" but **"does any program in this tree spell the
shape"** — a program that did would change meaning, and one that does not
cannot.

Three counts, over every `.qcore`/`.qc` file in the tree (comments stripped):

1. `let NAME = NAME ;` — the claimed shape, anywhere.
2. `: string` in field-type position — the `AGGREGATE-VIEW-FIELD` spelling.
3. a record field whose declared type is a record type the same file declares
   — the `AGGREGATE-NESTED` shape.

Usage:  python3 docs/reports/owned-field-route-reach/scripts/census.py .
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

COMMENT = re.compile(r"//[^\n]*")
COPY_BIND = re.compile(r"\blet\s+([A-Za-z_][A-Za-z0-9_]*)\s*=\s*([A-Za-z_][A-Za-z0-9_]*)\s*;")
VIEW_FIELD = re.compile(r":\s*string\b")
RECORD_DECL = re.compile(r"\btype\s+([A-Za-z_][A-Za-z0-9_]*)\s*\{([^}]*)\}")
FIELD = re.compile(r"([A-Za-z_][A-Za-z0-9_]*)\s*:\s*([A-Za-z_][A-Za-z0-9_]*)")
# `resource type NAME {` is a different declaration head and is not a record.
RESOURCE_TYPE = re.compile(r"\bresource\s+type\b")


def main() -> int:
    root = Path(sys.argv[1] if len(sys.argv) > 1 else ".").resolve()
    files = sorted(
        p
        for p in root.rglob("*")
        if p.suffix in {".qcore", ".qc"}
        and ".git" not in p.parts
        and "target" not in p.parts
        and "measure" not in p.parts
        and "private" not in p.parts
    )
    copy_hits: list[str] = []
    view_hits: list[str] = []
    nested_hits: list[str] = []
    for path in files:
        text = COMMENT.sub("", path.read_text(encoding="utf-8", errors="replace"))
        rel = path.relative_to(root)
        for match in COPY_BIND.finditer(text):
            line = text[: match.start()].count("\n") + 1
            copy_hits.append(f"{rel}:{line}: {match.group(0)}")
        for match in VIEW_FIELD.finditer(text):
            line = text[: match.start()].count("\n") + 1
            view_hits.append(f"{rel}:{line}: {match.group(0).strip()}")
        declared = {
            m.group(1)
            for m in RECORD_DECL.finditer(text)
            if not RESOURCE_TYPE.search(text[max(0, m.start() - 12) : m.start() + 4])
        }
        for m in RECORD_DECL.finditer(text):
            if RESOURCE_TYPE.search(text[max(0, m.start() - 12) : m.start() + 4]):
                continue
            for field in FIELD.finditer(m.group(2)):
                if field.group(2) in declared:
                    line = text[: m.start()].count("\n") + 1
                    nested_hits.append(f"{rel}:{line}: {field.group(0)}")

    print(f"files scanned: {len(files)}")
    for label, hits in (
        ("`let NAME = NAME;` (the claimed shape)", copy_hits),
        ("`: string` at a field type (AGGREGATE-VIEW-FIELD)", view_hits),
        ("a record-typed field (AGGREGATE-NESTED)", nested_hits),
    ):
        print(f"\n{label}: {len(hits)}")
        for hit in hits:
            print(f"  {hit}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

#!/usr/bin/env python3
"""Constraint-14 census for card AGGREGATE-OWNED-FIELD.

Scans every `.qcore`/`.qv`/`.qr`/`.qc` file in the tree (comments stripped) for the
spellings the card claims or rejected: `bytes` and `Buffer` in field-type
position, a record declaration with a `bytes` field, the dotted store/load
shape `NAME.NAME[`, and `drop NAME.NAME`. Run from the repository root; the
card's own example is expected to be the only file spelling the new forms.
"""
import pathlib
import re

SKIP = {"target", ".lake", "measure", "private"}
files = [
    p
    for p in pathlib.Path(".").rglob("*")
    if p.suffix in (".qcore", ".qv", ".qr", ".qc") and not (SKIP & set(p.parts))
]
print("files scanned:", len(files))


def code_lines(path):
    text = path.read_text(encoding="utf-8", errors="replace")
    for number, line in enumerate(text.split("\n"), 1):
        stripped = line.split("//")[0]
        if stripped.strip():
            yield number, stripped


PATTERNS = [
    ("field-type `bytes` (`: bytes`)", r":\s*bytes\b"),
    ("field-type `Buffer` (`: Buffer`)", r":\s*Buffer\b"),
    ("record declaration with a `bytes` field", r"^\s*type\s+\w+\s*\{.*\bbytes\b"),
    ("dotted store or load `NAME.NAME[`", r"\b\w+\.\w+\["),
    ("`drop NAME.NAME`", r"\bdrop\s+\w+\.\w+"),
    ("`extent(NAME.NAME)`", r"\bextent\s*\(\s*\w+\.\w+\s*\)"),
]
for label, pattern in PATTERNS:
    hits = []
    for path in sorted(files):
        for number, line in code_lines(path):
            if re.search(pattern, line):
                hits.append((str(path), number, line.strip()))
    print(f"\n{label}: {len(hits)} occurrence(s) in code")
    for hit in hits[:20]:
        print("   ", hit)

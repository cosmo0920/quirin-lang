#!/usr/bin/env python3
"""Constraint-14 census for card W48-QR-FLOATING-POINT's spellings.

Usage (from the repository root):

    python3 docs/reports/w48-qr-floating-point/census_float_spelling.py [OUT.json]

Reads every tracked `.qr`, `.qv`, `.qrlib`, `.qcore` and `.qc` file (from
`git ls-files`; constraint 29) and tokenizes it the way
`crates/qcore-frontend/src/lexer.rs` does at the card's base: `//` and
`/* */` comments are trivia, `"..."` is one string token, `'x'` a char
literal, words are `[A-Za-z_][A-Za-z0-9_]*`, and a number is `DIGITS` or
`DIGITS . DIGITS` (the lexer's existing `Float` token). It counts, per file:

* the words `f32`, `f64`, `partial_eq` and `partial_ord` in any position, and
  each of them after `as` (the conversion position), after `:` (the
  annotation position) and inside an `effects { ... }` row;
* every `DIGITS . DIGITS` token (the existing `Float` token, which the card
  gives a meaning on the routes it widens);
* every exponent spelling the card's lexer change would read as one float
  literal: a number immediately followed (no space) by `e`/`E`, an optional
  `+`/`-`, and a digit -- today that is a number, then a word (`1e5` is
  `1` `e5`) or a number, a word `e`, a sign and a number (`1.5e-3`).

It also reads `is_reserved_source_word` out of `frontend.rs` and reports the
reserved-word count and which of the card's words are reserved (the card
reserves none). Output is deterministic JSON.
"""

import json
import re
import subprocess
import sys
from pathlib import Path

SUFFIXES = (".qr", ".qv", ".qrlib", ".qcore", ".qc")
TWO = {"[|", "|]", "|>", "||", "&&", "->", "=>", "::", ":=", "==", "!=", "<=", ">="}
WORD = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")
CARD_WORDS = ["f32", "f64", "partial_eq", "partial_ord"]
EXPONENT = re.compile(r"[0-9]+(?:\.[0-9]+)?[eE][+-]?[0-9]")


def tokens(text: str):
    """Yield (kind, value, start, end). Unterminated literals end the file."""
    i, n = 0, len(text)
    while i < n:
        ch = text[i]
        if ch.isspace():
            i += 1
            continue
        if text.startswith("//", i):
            j = text.find("\n", i)
            i = n if j < 0 else j
            continue
        if text.startswith("/*", i):
            j = text.find("*/", i + 2)
            i = n if j < 0 else j + 2
            continue
        m = WORD.match(text, i)
        if m:
            yield ("word", m.group(0), i, m.end())
            i = m.end()
            continue
        if ch.isdigit():
            j = i
            while j < n and text[j].isdigit():
                j += 1
            kind = "number"
            if j + 1 < n and text[j] == "." and text[j + 1].isdigit():
                j += 1
                while j < n and text[j].isdigit():
                    j += 1
                kind = "float"
            yield (kind, text[i:j], i, j)
            i = j
            continue
        if ch == '"':
            j, escaped = i + 1, False
            while j < n:
                if not escaped and text[j] == '"':
                    break
                escaped = not escaped and text[j] == "\\"
                j += 1
            yield ("string", text[i + 1 : j], i, j + 1)
            i = j + 1
            continue
        if ch == "'" and i + 2 < n and text[i + 2] == "'" and text[i + 1] not in "\n\r'":
            yield ("char", text[i + 1], i, i + 3)
            i += 3
            continue
        pair = text[i : i + 2]
        if pair in TWO:
            yield ("symbol", pair, i, i + 2)
            i += 2
            continue
        yield ("symbol", ch, i, i + 1)
        i += 1


def reserved_words(root: Path) -> set:
    source = (root / "crates/qcore-frontend/src/frontend.rs").read_text()
    start = source.index("fn is_reserved_source_word(word: &str) -> bool {")
    body = source[start : source.index("\n}\n", start)]
    return set(re.findall(r'"([^"]+)"', body))


def main() -> int:
    root = Path(
        subprocess.run(
            ["git", "rev-parse", "--show-toplevel"], check=True, capture_output=True
        ).stdout.decode().strip()
    )
    listed = subprocess.run(
        ["git", "ls-files", "-z"], cwd=root, check=True, capture_output=True
    ).stdout.decode()
    files = sorted(p for p in listed.split("\0") if p.endswith(SUFFIXES))
    if not files:
        print("no tracked source files", file=sys.stderr)
        return 1
    hits = {f"word_{w}": [] for w in CARD_WORDS}
    hits.update({f"as_{w}": [] for w in CARD_WORDS})
    hits.update({f"annotation_{w}": [] for w in CARD_WORDS})
    hits.update({f"effect_row_{w}": [] for w in CARD_WORDS})
    hits["float_token"] = []
    hits["exponent_spelling"] = []
    for path in files:
        text = (root / path).read_text(errors="replace")
        toks = list(tokens(text))
        in_row = False
        for k, (kind, value, start, end) in enumerate(toks):
            line = f"{path}:{text.count(chr(10), 0, start) + 1}"
            if kind == "word" and value == "effects" and k + 1 < len(toks) and toks[k + 1][1] == "{":
                in_row = True
            elif in_row and kind == "symbol" and value == "}":
                in_row = False
            if kind == "word" and value in CARD_WORDS:
                hits[f"word_{value}"].append(line)
                if k > 0 and toks[k - 1][:2] == ("word", "as"):
                    hits[f"as_{value}"].append(line)
                if k > 0 and toks[k - 1][:2] == ("symbol", ":"):
                    hits[f"annotation_{value}"].append(line)
                if in_row:
                    hits[f"effect_row_{value}"].append(line)
            if kind == "float":
                hits["float_token"].append(line)
            if kind in ("number", "float") and EXPONENT.match(text, start):
                # Only a spelling that starts at the number's own first digit
                # and runs on without a space is an exponent candidate.
                m = EXPONENT.match(text, start)
                if m and m.end() > end:
                    hits["exponent_spelling"].append(line)
    reserved = reserved_words(root)
    report = {
        "files": len(files),
        "counts": {key: len(value) for key, value in sorted(hits.items())},
        "occurrences": {key: value for key, value in sorted(hits.items()) if value},
        "reserved_word_count": len(reserved),
        "card_words_reserved": sorted(w for w in CARD_WORDS if w in reserved),
    }
    text = json.dumps(report, indent=2, sort_keys=True) + "\n"
    if len(sys.argv) > 1:
        Path(sys.argv[1]).write_text(text)
    else:
        sys.stdout.write(text)
    return 0


if __name__ == "__main__":
    sys.exit(main())

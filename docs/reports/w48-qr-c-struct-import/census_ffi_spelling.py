#!/usr/bin/env python3
"""Constraint-14 census for card W48-QR-C-STRUCT-IMPORT's spellings.

Usage (from the repository root):

    python3 docs/reports/w48-qr-c-struct-import/census_ffi_spelling.py [OUT.json]

Reads every tracked `.qr`, `.qv`, `.qrlib`, `.qcore` and `.qc` file (from
`git ls-files`; constraint 29) and tokenizes it the way
`crates/qcore-frontend/src/lexer.rs` does: `//` and `/* */` comments are
trivia, `"…"` is one string token (backslash escapes), `'x'` a char literal,
words are `[A-Za-z_][A-Za-z0-9_]*`, and the two-byte symbols (`[|`, `|]`,
`|>`, `||`, `&&`, `->`, `=>`, `::`, `:=`, `==`, `!=`, `<=`, `>=`) win over
one-byte ones. It counts, per file:

* `struct` followed by `!` — the declaration head this card claims;
* every `struct` word token at all;
* the word `c` immediately followed (no byte between) by a string literal —
  the `c"…"` C-type spelling;
* the `[|` and `|]` tokens, and `[|` followed by `quant # c_ffi` — the
  contract attribute;
* the word `c_ffi`, and the import path `std.ffi` in any `import` line;
* `|` followed by `copy` — card 4's closure capture-list opener, to show the
  lexical space is still free.

It also reads `is_reserved_source_word` out of `frontend.rs` and reports
whether any word this card uses (`struct`, `c`, `from`, `fields`, `c_ffi`,
`ffi`) is reserved: none is, and none is added. Output is deterministic JSON.
"""

import json
import re
import subprocess
import sys
from collections import Counter
from pathlib import Path

SUFFIXES = (".qr", ".qv", ".qrlib", ".qcore", ".qc")
TWO = {"[|", "|]", "|>", "||", "&&", "->", "=>", "::", ":=", "==", "!=", "<=", ">="}
WORD = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")
CARD_WORDS = ["struct", "c", "from", "fields", "c_ffi", "ffi"]


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
            while j < n and (text[j].isdigit()):
                j += 1
            yield ("number", text[i:j], i, j)
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
    hits = {
        "struct_bang": [],
        "c_string": [],
        "quant_open": [],
        "quant_close": [],
        "c_ffi_attribute": [],
        "c_ffi_word": [],
        "std_ffi_import": [],
        "pipe_copy": [],
    }
    struct_words = Counter()
    for path in files:
        text = (root / path).read_text(errors="replace")
        toks = list(tokens(text))
        for k, (kind, value, start, end) in enumerate(toks):
            nxt = toks[k + 1] if k + 1 < len(toks) else None
            line = text.count("\n", 0, start) + 1
            if kind == "word" and value == "struct":
                struct_words[path] += 1
                if nxt and nxt[0] == "symbol" and nxt[1] == "!":
                    hits["struct_bang"].append(f"{path}:{line}")
            if kind == "word" and value == "c" and nxt and nxt[0] == "string" and nxt[2] == end:
                hits["c_string"].append(f"{path}:{line}")
            if kind == "symbol" and value == "[|":
                hits["quant_open"].append(f"{path}:{line}")
                window = [t[1] for t in toks[k + 1 : k + 4]]
                if window == ["quant", "#", "c_ffi"]:
                    hits["c_ffi_attribute"].append(f"{path}:{line}")
            if kind == "symbol" and value == "|]":
                hits["quant_close"].append(f"{path}:{line}")
            if kind == "word" and value == "c_ffi":
                hits["c_ffi_word"].append(f"{path}:{line}")
            if kind == "word" and value == "import":
                dotted = []
                j = k + 1
                while j < len(toks) and toks[j][0] in ("word", "symbol") and toks[j][1] != ";":
                    dotted.append(toks[j][1])
                    j += 1
                if "".join(dotted).startswith("std.ffi"):
                    hits["std_ffi_import"].append(f"{path}:{line}")
            if kind == "symbol" and value == "|" and nxt and nxt[:2] == ("word", "copy"):
                hits["pipe_copy"].append(f"{path}:{line}")
    reserved = reserved_words(root)
    report = {
        "files": len(files),
        "counts": {name: len(found) for name, found in hits.items()},
        "hits": hits,
        "struct_word_files": len(struct_words),
        "struct_word_tokens": sum(struct_words.values()),
        "reserved_word_count": len(reserved),
        "card_words_reserved": sorted(w for w in CARD_WORDS if w in reserved),
    }
    text = json.dumps(report, indent=1, sort_keys=True) + "\n"
    if len(sys.argv) == 2:
        Path(sys.argv[1]).write_text(text)
    print(text, end="")
    return 0


if __name__ == "__main__":
    sys.exit(main())

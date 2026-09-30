#!/usr/bin/env python3
"""Constraint-14 census for card W48-QR-CLOSURES' spellings.

Usage (from the repository root):

    python3 docs/reports/w48-qr-closures/census_closure_spelling.py [OUT.json]

Reads every tracked `.qr`, `.qv`, `.qrlib`, `.qcore` and `.qc` file (from
`git ls-files`; constraint 29) and tokenizes it the way
`crates/qcore-frontend/src/lexer.rs` does (the tokenizer of card
W48-QR-C-STRUCT-IMPORT's census, copied): `//` and `/* */` comments are
trivia, `"…"` is one string token, `'x'` a char literal, words are
`[A-Za-z_][A-Za-z0-9_]*`, and the two-byte symbols (`[|`, `|]`, `|>`, `||`,
`&&`, `->`, `=>`, `::`, `:=`, `==`, `!=`, `<=`, `>=`) win over one-byte ones.
It counts, per file:

* `spawn NAME |`, `spawn NAME ||` and `spawn NAME [|` -- the closure head this
  card claims (a capture list, the empty capture list, and the
  bracket-adjacent spelling the card refuses);
* `|` followed by a capture-mode word (`copy`, `move`, `read`, `write`,
  `lend`) anywhere, and `|` or `||` followed by `->` anywhere -- the two edges
  of a capture list in any position;
* every `|`, `||`, `[|`, `|]` and `|>` token;
* `as` followed by a name inside a named spawn's parenthesized capture list --
  the capture binder `as NAME` in the position a named worker's list has.

It also reads `is_reserved_source_word` out of `frontend.rs` and reports the
reserved-word count and which of the words this card uses are reserved (the
card reserves none). Output is deterministic JSON.
"""

import json
import re
import subprocess
import sys
from pathlib import Path

SUFFIXES = (".qr", ".qv", ".qrlib", ".qcore", ".qc")
TWO = {"[|", "|]", "|>", "||", "&&", "->", "=>", "::", ":=", "==", "!=", "<=", ">="}
WORD = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")
CARD_WORDS = ["spawn", "copy", "move", "read", "write", "lend", "share", "as", "effects"]
MODES = {"copy", "move", "read", "write", "lend"}


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
        "spawn_name_pipe": [],
        "spawn_name_pipepipe": [],
        "spawn_name_quant_open": [],
        "pipe_before_mode_word": [],
        "pipe_before_arrow": [],
        "pipepipe_before_arrow": [],
        "as_binder_in_spawn_list": [],
    }
    totals = {"pipe": 0, "pipepipe": 0, "quant_open": 0, "quant_close": 0, "pipe_into": 0}
    for path in files:
        text = (root / path).read_text(errors="replace")
        toks = list(tokens(text))

        def at(j):
            return toks[j][:2] if 0 <= j < len(toks) else (None, None)

        spawn_list = None  # (open index, depth) of a named spawn's capture list
        for k, (kind, value, start, _end) in enumerate(toks):
            line = f"{path}:{text.count(chr(10), 0, start) + 1}"
            if kind == "word" and value == "spawn" and at(k + 1)[0] == "word":
                follow = at(k + 2)
                if follow == ("symbol", "|"):
                    hits["spawn_name_pipe"].append(line)
                if follow == ("symbol", "||"):
                    hits["spawn_name_pipepipe"].append(line)
                if follow == ("symbol", "[|"):
                    hits["spawn_name_quant_open"].append(line)
                if at(k + 3) == ("symbol", "("):
                    spawn_list = (k + 3, 0)
            if kind == "symbol" and value == "|":
                totals["pipe"] += 1
                if at(k + 1)[0] == "word" and at(k + 1)[1] in MODES:
                    hits["pipe_before_mode_word"].append(line)
                if at(k + 1) == ("symbol", "->"):
                    hits["pipe_before_arrow"].append(line)
            if kind == "symbol" and value == "||":
                totals["pipepipe"] += 1
                if at(k + 1) == ("symbol", "->"):
                    hits["pipepipe_before_arrow"].append(line)
            if kind == "symbol" and value == "[|":
                totals["quant_open"] += 1
            if kind == "symbol" and value == "|]":
                totals["quant_close"] += 1
            if kind == "symbol" and value == "|>":
                totals["pipe_into"] += 1
            if spawn_list is not None and k > spawn_list[0]:
                opened, depth = spawn_list
                if value == "(":
                    spawn_list = (opened, depth + 1)
                elif value == ")":
                    spawn_list = None if depth == 0 else (opened, depth - 1)
                elif kind == "word" and value == "as" and at(k + 1)[0] == "word":
                    hits["as_binder_in_spawn_list"].append(line)
    reserved = reserved_words(root)
    report = {
        "files": len(files),
        "counts": {name: len(found) for name, found in hits.items()},
        "hits": hits,
        "token_totals": totals,
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

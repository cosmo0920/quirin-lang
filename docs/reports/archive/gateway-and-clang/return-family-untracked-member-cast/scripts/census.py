#!/usr/bin/env python3
"""Card RETURN-FAMILY-UNTRACKED-MEMBER-THROUGH-CAST: census of `return`
expressions in which a name reaches `->` or `[` only through closing
parentheses whose groups hold nothing before it but opening parentheses and
parenthesised type groups (identifiers, then `*`s), the spelling the card's
new member/subscript base test reads.

    python3 census.py COMPILE_COMMANDS.json [COMPILE_COMMANDS.json ...]

Reads every distinct translation unit the compile databases name (the route's
units; headers are not read).

Prints one line per hit (file:line, the names, the expression's tokens) and a
count. A syntactic census only: whether family S is reached (a scalar result,
every older route declined) and whether the name is untracked is not decided
here.
"""
import json
import re
import sys
from pathlib import Path

TOKEN = re.compile(r"[A-Za-z_]\w*|\d[\w.]*|->|\S")
IDENT = re.compile(r"[A-Za-z_]\w*$")
KEYWORDS = {"return", "sizeof", "if", "while", "for", "switch"}


def strip(text):
    text = re.sub(r"/\*.*?\*/", lambda m: "\n" * m.group(0).count("\n"), text, flags=re.S)
    text = re.sub(r"//[^\n]*", "", text)
    text = re.sub(r'"(\\.|[^"\\\n])*"', '""', text)
    return re.sub(r"'(\\.|[^'\\\n])*'", "''", text)


def type_group_open(toks, close):
    if toks[close] != ")":
        return None
    opens = [i for i in range(close) if toks[i] == "("]
    if not opens:
        return None
    open_ = opens[-1]
    group = toks[open_ + 1:close]
    while group and group[-1] == "*":
        group = group[:-1]
    if group and all(IDENT.match(t) for t in group):
        return open_
    return None


def hits(toks):
    out = []
    for i, t in enumerate(toks):
        if not IDENT.match(t) or t in KEYWORDS:
            continue
        if i + 1 < len(toks) and toks[i + 1] == "(":
            continue
        at, opened = i, 0
        while at > 0:
            p = toks[at - 1]
            if p == "(":
                opened += 1
                at -= 1
                continue
            o = type_group_open(toks, at - 1) if p == ")" else None
            if o is None:
                break
            at = o
        after, closes = i + 1, 0
        while after < len(toks) and toks[after] == ")" and closes < opened:
            after += 1
            closes += 1
        if closes >= 1 and after < len(toks) and toks[after] in ("->", "["):
            out.append(t)
    return out


def main():
    total = 0
    files = set()
    for database in sys.argv[1:]:
        for entry in json.loads(Path(database).read_text()):
            files.add(str(Path(entry["directory"], entry["file"]).resolve()))
    for name in sorted(files):
        path = Path(name)
        if path.is_file():
            text = strip(path.read_text(errors="replace"))
            for m in re.finditer(r"\breturn\b([^;{}]*);", text):
                toks = TOKEN.findall(m.group(1))
                names = hits(toks)
                if names:
                    total += 1
                    line = text.count("\n", 0, m.start()) + 1
                    print(f"{path}:{line}\t{','.join(names)}\t{' '.join(toks)}")
    print(f"total {total}")


if __name__ == "__main__":
    main()

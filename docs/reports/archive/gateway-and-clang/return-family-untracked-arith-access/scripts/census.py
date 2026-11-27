#!/usr/bin/env python3
"""Card RETURN-FAMILY-UNTRACKED-ACCESS-THROUGH-ARITHMETIC: census of `return`
expressions in which a name reaches `->`, `[` or a unary `*` only through
parentheses, parenthesised type groups and at least one parenthesised
additive group (`( p + n )`, `( n + p )`, `( ( T * ) p - n )`), the spelling
the card's new test reads. RETURN-FAMILY-UNTRACKED-MEMBER-THROUGH-CAST's
census.py with the walk changed.

    python3 census.py COMPILE_COMMANDS.json [COMPILE_COMMANDS.json ...]

Reads every distinct translation unit the compile databases name (the route's
units; headers are not read). Prints one line per hit (file:line, the names,
the expression's tokens) and a count. A syntactic superset: the other
operands of an additive group are not classified, and whether family S is
reached (a scalar result, every older route declined, no call) and whether
the name is untracked is not decided here.
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
    if close < 0 or toks[close] != ")":
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


def enclosing(toks, lo):
    depth = 0
    for i in range(lo - 1, -1, -1):
        if toks[i] in (")", "]"):
            depth += 1
        elif toks[i] in ("(", "["):
            if depth == 0:
                if toks[i] != "(":
                    return None
                d = 0
                for j in range(i, len(toks)):
                    if toks[j] == "(":
                        d += 1
                    elif toks[j] == ")":
                        d -= 1
                        if d == 0:
                            return i, j
                return None
            depth -= 1
    return None


def accessed(toks, i):
    lo = hi = i
    crossed = False
    while True:
        o = type_group_open(toks, lo - 1)
        if o is not None:
            lo = o
            continue
        left = toks[lo - 1] if lo > 0 else None
        right = toks[hi + 1] if hi + 1 < len(toks) else None
        if left == "(" and right == ")":
            lo, hi = lo - 1, hi + 1
            continue
        if left in ("(", "+", "-") and right in (")", "+", "-"):
            group = enclosing(toks, lo)
            if group is None:
                return False
            lo, hi = group
            crossed = True
            continue
        break
    if not crossed:
        return False
    after = toks[hi + 1] if hi + 1 < len(toks) else None
    before = toks[lo - 1] if lo > 0 else None
    before2 = toks[lo - 2] if lo > 1 else None
    star = before == "*" and not (
        before2 is not None and (IDENT.match(before2) or before2[0].isdigit() or before2 in (")", "]")))
    return after in ("->", "[") or star


def hits(toks):
    out = []
    for i, t in enumerate(toks):
        if not IDENT.match(t) or t in KEYWORDS:
            continue
        if i + 1 < len(toks) and toks[i + 1] == "(":
            continue
        if i > 0 and toks[i - 1] in ("->", "."):
            continue
        if accessed(toks, i):
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

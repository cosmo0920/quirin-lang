#!/usr/bin/env python3
"""Card RETURN-FAMILY-UNTRACKED-CONDITIONAL-ACCESS: census of `return`
expressions in which a parenthesised group whose top level holds a `?`
(a conditional, `( c ? p : q )`) reaches `->`, `[` or a unary `*` through
parentheses, parenthesised type groups and parenthesised additive groups,
the spelling the card's new test reads. RETURN-FAMILY-UNTRACKED-ACCESS-
THROUGH-ARITHMETIC's census.py with the starting span changed from a name
to a conditional group.

    python3 census.py COMPILE_COMMANDS.json [COMPILE_COMMANDS.json ...]

Reads every distinct translation unit the compile databases name (the route's
units; headers are not read). Prints one line per hit (file:line, whether the
expression holds a call, the expression's tokens) and counts. A syntactic
superset: the arms are not classified, and whether family S is reached (a
scalar result, every older route declined, no call) and whether a root is
untracked is not decided here.
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


def close_of(toks, open_):
    d = 0
    for j in range(open_, len(toks)):
        if toks[j] in ("(", "["):
            d += 1
        elif toks[j] in (")", "]"):
            d -= 1
            if d == 0:
                return j
    return None


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
                j = close_of(toks, i)
                return None if j is None else (i, j)
            depth -= 1
    return None


def top_question(toks, lo, hi):
    d = 0
    for i in range(lo, hi + 1):
        if toks[i] in ("(", "["):
            d += 1
        elif toks[i] in (")", "]"):
            d -= 1
        elif toks[i] == "?" and d == 0:
            return True
    return False


def accessed(toks, lo, hi):
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
            continue
        break
    after = toks[hi + 1] if hi + 1 < len(toks) else None
    before = toks[lo - 1] if lo > 0 else None
    before2 = toks[lo - 2] if lo > 1 else None
    star = before == "*" and not (
        before2 is not None and (IDENT.match(before2) or before2[0].isdigit() or before2 in (")", "]")))
    return after in ("->", "[") or star


def hit(toks):
    for i, t in enumerate(toks):
        if t != "(":
            continue
        j = close_of(toks, i)
        if j is None or toks[j] != ")":
            continue
        if top_question(toks, i + 1, j - 1) and accessed(toks, i, j):
            return True
    return False


def has_call(toks):
    return any(
        toks[i + 1] == "("
        and ((IDENT.match(toks[i]) and toks[i] not in KEYWORDS) or toks[i] in (")", "]"))
        for i in range(len(toks) - 1)
    )


def main():
    total = calls = 0
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
                if hit(toks):
                    total += 1
                    call = has_call(toks)
                    calls += call
                    line = text.count("\n", 0, m.start()) + 1
                    kind = "call" if call else "no-call"
                    print(f"{path}:{line}\t{kind}\t{' '.join(toks)}")
    print(f"total {total} with-call {calls} without-call {total - calls}")


if __name__ == "__main__":
    main()

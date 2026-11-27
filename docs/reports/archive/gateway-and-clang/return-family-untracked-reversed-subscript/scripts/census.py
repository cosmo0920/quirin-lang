#!/usr/bin/env python3
"""Card RETURN-FAMILY-UNTRACKED-REVERSED-SUBSCRIPT: census of `return`
expressions in which a name fills the brackets of a subscript `X [ ... ]`
through parentheses, parenthesised type groups and additive terms, the
spelling the card's new test reads. RETURN-FAMILY-UNTRACKED-ACCESS-THROUGH-
ARITHMETIC's census.py with the walk changed.

    python3 census.py COMPILE_COMMANDS.json [COMPILE_COMMANDS.json ...]

Reads every distinct translation unit the compile databases name (headers are
not read). Prints one line per hit: file:line, the class (`call` when the
expression holds a call-shaped `NAME (`, which family S refuses before any
base is read; `literal` when X is a numeric literal; `other` otherwise), the
names, and the expression's tokens; then the counts. A syntactic superset:
whether X is a declared pointer, whether the name is a declared scalar or
untracked, and whether family S is reached is not decided here.
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
    depth = 0
    for i in range(close, -1, -1):
        if toks[i] == ")":
            depth += 1
        elif toks[i] == "(":
            depth -= 1
            if depth == 0:
                group = toks[i + 1:close]
                while group and group[-1] == "*":
                    group = group[:-1]
                if group and all(IDENT.match(t) for t in group):
                    return i
                return None
    return None


def enclosing(toks, lo):
    """The innermost bracket around lo: (open, close, kind)."""
    depth = 0
    for i in range(lo - 1, -1, -1):
        if toks[i] in (")", "]"):
            depth += 1
        elif toks[i] in ("(", "["):
            if depth == 0:
                opener, closer = toks[i], ")" if toks[i] == "(" else "]"
                d = 0
                for j in range(i, len(toks)):
                    if toks[j] == opener:
                        d += 1
                    elif toks[j] == closer:
                        d -= 1
                        if d == 0:
                            return i, j, opener
                return None
            depth -= 1
    return None


def fills_subscript(toks, i):
    lo = hi = i
    while lo > 0 and toks[lo - 1] == "*":
        lo -= 1
    while True:
        o = type_group_open(toks, lo - 1)
        if o is not None:
            lo = o
            continue
        left = toks[lo - 1] if lo > 0 else None
        right = toks[hi + 1] if hi + 1 < len(toks) else None
        if left == "[" and right == "]":
            return lo - 1
        if left == "(" and right == ")":
            lo, hi = lo - 1, hi + 1
            continue
        if left in ("(", "[", "+", "-") and right in (")", "]", "+", "-"):
            group = enclosing(toks, lo)
            if group is None:
                return None
            open_, close, kind = group
            if kind == "[":
                return open_
            lo, hi = open_, close
            continue
        return None


def hits(toks):
    out = []
    for i, t in enumerate(toks):
        if not IDENT.match(t) or t in KEYWORDS:
            continue
        if i + 1 < len(toks) and toks[i + 1] == "(":
            continue
        if i > 0 and toks[i - 1] in ("->", "."):
            continue
        bracket = fills_subscript(toks, i)
        if bracket is not None and bracket > 0:
            out.append((t, toks[bracket - 1]))
    return out


def main():
    counts = {"call": 0, "literal": 0, "other": 0}
    files = set()
    for database in sys.argv[1:]:
        for entry in json.loads(Path(database).read_text()):
            files.add(str(Path(entry["directory"], entry["file"]).resolve()))
    for name in sorted(files):
        path = Path(name)
        if not path.is_file():
            continue
        text = strip(path.read_text(errors="replace"))
        for m in re.finditer(r"\breturn\b([^;{}]*);", text):
            toks = TOKEN.findall(m.group(1))
            found = hits(toks)
            if not found:
                continue
            call = any(
                IDENT.match(t) and t not in KEYWORDS and j + 1 < len(toks) and toks[j + 1] == "("
                for j, t in enumerate(toks)
            )
            if call:
                kind = "call"
            elif any(outside[0].isdigit() for _, outside in found):
                kind = "literal"
            else:
                kind = "other"
            counts[kind] += 1
            line = text.count("\n", 0, m.start()) + 1
            names = ",".join(f"{n}@{o}" for n, o in found)
            print(f"{path}:{line}\t{kind}\t{names}\t{' '.join(toks)}")
    print("counts " + " ".join(f"{k}={v}" for k, v in counts.items()))


if __name__ == "__main__":
    main()

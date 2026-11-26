#!/usr/bin/env python3
"""Card RETURN-STRICT-MEMBER-NULL-UNTRACKED-ROOT: census of the `return`
spellings B01's opaque member read admits, `return R -> F ;` (one `->`
selection of a name, possibly under one parenthesised cast), in the route's
units, joined to the base report's row status.

    python3 census.py REPORT.json SOURCE_ROOT [MONKEY_ROOT]

REPORT.json is the base run's `resource-diagnostics.json`; only the units it
lists are read (headers are not). For each hit the enclosing function
definition is found by a brace-depth scan and the root is classified:

* `formal`: a parameter of the function the body never assigns
  (`R =`, `R +=`, `&R`, `R++`), so the base obligation sees it unbound;
* `formal-written` / `local-null`: a parameter that is written, or a local
  whose declaration or an assignment gives it a null constant (`0`, `NULL`,
  `(void *) 0`): a binding the flow check may read as possibly null;
* `local-other`: any other local or file-scope root (bound from a member
  read, a formal, a call, or never written).

A syntactic census only: whether the row reaches B01 (every older route
declined) and whether the flow check discharges the base is not decided here.
Prints one line per hit and the totals by class and base status.
"""
import collections
import json
import re
import sys
from pathlib import Path

TOKEN = re.compile(r"[A-Za-z_]\w*|\d[\w.]*|->|\S")
IDENT = re.compile(r"[A-Za-z_]\w*$")
NULLS = {"0", "NULL", "nullptr"}
KEYWORDS = {"if", "while", "for", "switch", "return", "sizeof", "do", "else"}


def strip(text):
    text = re.sub(r"/\*.*?\*/", lambda m: "\n" * m.group(0).count("\n"), text, flags=re.S)
    text = re.sub(r"//[^\n]*", "", text)
    text = re.sub(r'"(\\.|[^"\\\n])*"', '""', text)
    text = re.sub(r"'(\\.|[^'\\\n])*'", "''", text)
    text = re.sub(r"^\s*#.*?(?<!\\)$", lambda m: "\n" * m.group(0).count("\n"), text,
                  flags=re.M | re.S)
    return text


def tokens(text):
    out = []
    for number, line in enumerate(text.split("\n"), 1):
        for match in TOKEN.finditer(line):
            out.append((match.group(0), number))
    return out


def functions(toks):
    """(name, params, body_start, body_end) for every top-level `NAME ( … ) {`."""
    out = []
    depth = 0
    index = 0
    while index < len(toks):
        text = toks[index][0]
        if text == "{" and depth == 0 and index > 0 and toks[index - 1][0] == ")":
            close = index - 1
            level = 0
            open_ = None
            for back in range(close, -1, -1):
                if toks[back][0] == ")":
                    level += 1
                elif toks[back][0] == "(":
                    level -= 1
                    if level == 0:
                        open_ = back
                        break
            end = index
            level = 0
            for forward in range(index, len(toks)):
                if toks[forward][0] == "{":
                    level += 1
                elif toks[forward][0] == "}":
                    level -= 1
                    if level == 0:
                        end = forward
                        break
            if open_ and open_ > 0 and IDENT.match(toks[open_ - 1][0]):
                params = {t for t, _ in toks[open_ + 1 : close] if IDENT.match(t)}
                out.append((toks[open_ - 1][0], params, index, end))
            index = end + 1
            continue
        if text == "{":
            depth += 1
        elif text == "}":
            depth = max(0, depth - 1)
        index += 1
    return out


def written(body, root):
    for i, (text, _) in enumerate(body):
        if text != root:
            continue
        nxt = body[i + 1][0] if i + 1 < len(body) else ""
        nxt2 = body[i + 2][0] if i + 2 < len(body) else ""
        prev = body[i - 1][0] if i else ""
        if nxt == "=" and nxt2 != "=":
            return True
        if nxt in "+-*/|&^" and nxt2 == "=":
            return True
        if prev == "&" or (nxt == nxt2 and nxt in "+-") or (prev in "+-" and i > 1 and body[i - 2][0] == prev):
            return True
    return False


def null_bound(body, root):
    for i, (text, _) in enumerate(body):
        if text == root and i + 2 < len(body) and body[i + 1][0] == "=" and body[i + 2][0] != "=":
            value = []
            for t, _ in body[i + 2 :]:
                if t in (";", ","):
                    break
                value.append(t)
            core = [t for t in value if t not in ("(", ")", "void", "*")]
            if core and all(t in NULLS for t in core):
                return True
    return False


def hits(path):
    toks = tokens(strip(path.read_text(errors="replace")))
    for name, params, start, end in functions(toks):
        body = toks[start : end + 1]
        for i, (text, line) in enumerate(body):
            if text != "return":
                continue
            expr = []
            for t, _ in body[i + 1 :]:
                if t == ";":
                    break
                expr.append(t)
            # strip one parenthesised cast `( IDENT… *… )` and outer parentheses
            core = list(expr)
            while len(core) > 2 and core[0] == "(" and core[-1] == ")":
                core = core[1:-1]
            if core and core[0] == "(" and ")" in core:
                close = core.index(")")
                inner = core[1:close]
                if inner and all(IDENT.match(t) or t == "*" for t in inner) and "*" in inner:
                    core = core[close + 1 :]
            if len(core) == 3 and IDENT.match(core[0]) and core[1] == "->" and IDENT.match(core[2]) \
                    and core[0] not in KEYWORDS:
                root = core[0]
                if root in params:
                    klass = "formal-written" if written(body, root) else "formal"
                elif null_bound(body, root):
                    klass = "local-null"
                else:
                    klass = "local-other"
                yield name, line, root, klass, " ".join(expr)


def main() -> int:
    report = json.load(open(sys.argv[1]))
    roots = [Path(p) for p in sys.argv[2:]]
    totals = collections.Counter()
    for unit in report["units"]:
        status = collections.defaultdict(list)
        for function in unit.get("functions", []):
            status[function["function"]].append(function["status"])
        path = None
        for root in roots:
            candidate = root / unit["file"]
            if candidate.is_file():
                path = candidate
                break
        if path is None:
            continue
        for name, line, root, klass, expr in hits(path):
            row = ",".join(status.get(name, ["absent"]))
            totals[(klass, row)] += 1
            print(f"{unit['file']}:{line} {name} [{row}] {klass} {root}: return {expr} ;")
    print("totals:")
    for (klass, row), count in sorted(totals.items()):
        print(f"  {klass:15} {row:12} {count}")
    return 0


if __name__ == "__main__":
    sys.exit(main())

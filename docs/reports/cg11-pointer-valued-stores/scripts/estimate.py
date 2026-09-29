#!/usr/bin/env python3
"""CG11-POINTER-VALUED-STORES: a static upper bound on the functions rule 10v
could free, frozen before the rule ran on the corpus.

Usage: estimate.py RUN_DIR PARITY_JSONL

Reads the BEFORE run's resource-diagnostics.json and its parity stream. A
refused statement is a *candidate* when its tokens are one simple store
`LHS = RHS` whose left side selects a member, an element or a pointee
(`->`, `.`, `[`, a leading `*`) and whose right side is spelled as one of the
value classes the rule admits: the null constant (`NULL`, `0`, `( ( void * ) 0 )`
and its parenthesised forms), a string literal, the left side's root itself,
or the address `&` of a place spelled from the same root. A name of
function-pointer type or a function designator cannot be told from tokens;
they are not counted. The owner premise (a never-bound root) is not checked.

An Unknown function is an upper-bound candidate when every one of its
diagnostics is an `unsupported` leaf with a store refusal on a line whose
refused statements are all candidates. Prints the candidate functions.
"""

import collections
import json
import pathlib
import re
import sys

STORE_REFUSALS = {
    "indirect memory access is outside the resource projection",
    "unrecognized assignment destination",
    "unsupported assignment expression",
}


def strip_parens(tokens):
    while len(tokens) >= 2 and tokens[0] == "(" and tokens[-1] == ")":
        depth = 0
        for index, token in enumerate(tokens):
            depth += token == "("
            depth -= token == ")"
            if depth == 0 and index != len(tokens) - 1:
                return tokens
        tokens = tokens[1:-1]
    return tokens


def root_of(tokens):
    for token in tokens:
        if re.fullmatch(r"[A-Za-z_]\w*", token):
            return token
    return None


def candidate(texts):
    if texts.count("=") != 1:
        return False
    equal = texts.index("=")
    left, right = texts[:equal], strip_parens(texts[equal + 1:])
    if not left or not right or any(t in ("==", "!=", "<=", ">=") for t in texts):
        return False
    selects = "-" in left and ">" in left or "." in left or "[" in left or left[0] == "*"
    if not selects:
        return False
    root = root_of(left)
    if right in (["NULL"], ["0"], ["(", "void", "*", ")", "0"]):
        return True
    if len(right) == 1 and right[0].startswith('"'):
        return True
    if right == [root]:
        return True
    if right[0] == "&" and root_of(right[1:]) == root:
        return True
    return False


def main() -> int:
    run, parity = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2])
    data = json.loads((run / "resource-diagnostics.json").read_text())
    lines = collections.defaultdict(lambda: [0, 0])  # (function, line) -> [all, candidates]
    seen = set()
    with parity.open() as stream:
        for raw in stream:
            row = json.loads(raw)
            if row.get("position") != "elaboration-statement":
                continue
            key = (row["function"], row["line"], tuple(row["texts"]))
            if key in seen:
                continue
            seen.add(key)
            entry = lines[(row["function"], row["line"])]
            entry[0] += 1
            entry[1] += candidate(row["texts"])
    freed = []
    for unit in data["units"]:
        for function in unit["functions"]:
            if function["status"] != "Unknown":
                continue
            diagnostics = function.get("diagnostics", [])
            if not diagnostics:
                continue
            ok = True
            for diagnostic in diagnostics:
                if diagnostic.get("category") != "unsupported":
                    ok = False
                    break
                if diagnostic.get("description") not in STORE_REFUSALS:
                    ok = False
                    break
                entry = lines.get((function["function"], diagnostic.get("line")))
                if not entry or entry[0] != entry[1]:
                    ok = False
                    break
            if ok:
                freed.append(f"{unit['file']}::{function['function']}")
    for name in sorted(freed):
        print(name)
    print(f"{len(freed)} upper-bound candidate functions")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

#!/usr/bin/env python3
"""CG11-POINTER-VALUED-STORES: a lexical census of the function-pointer
parameter finding (`CG11-FUNCTION-POINTER-PARAMETER-CLOSURE`).

Usage: fp_param_census.py RUN_DIR

The probe (private/cg11/probe) showed `void subject(struct S *s, void
(*g)(void)) { s->a = 1; }` Unknown with `unknown_alias s`, while the same body
without the function-pointer parameter is Safe. This census reads the pinned
source tree lexically: a function definition whose parameter list holds a
declarator `( * NAME ) (`, and whose row in RUN_DIR is Unknown with at least
one `unknown_alias` naming another of its parameters. It is an upper bound on
the functions the finding could touch (a function may carry other blockers),
and a lexical reading (macros and multi-line signatures are read as written).
"""

import collections
import json
import pathlib
import re
import sys

SOURCE = pathlib.Path(
    "/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source"
)
FUNCTION_POINTER = re.compile(r"\(\s*\*\s*([A-Za-z_]\w*)\s*\)\s*\(")


def parameters(text):
    names = []
    depth = 0
    current = ""
    for character in text:
        if character == "(":
            depth += 1
        elif character == ")":
            depth -= 1
        if character == "," and depth == 0:
            names.append(current)
            current = ""
        else:
            current += character
    names.append(current)
    found = []
    for declaration in names:
        pointer = FUNCTION_POINTER.search(declaration)
        if pointer:
            found.append((pointer.group(1), True))
            continue
        words = re.findall(r"[A-Za-z_]\w*", declaration)
        if words:
            found.append((words[-1], False))
    return found


def definitions(text):
    """Every `NAME ( PARAMS ) {` at file scope, found by one linear scan: a
    `{` at brace depth 0 whose previous non-space character is `)`, the
    matching `(` found by counting back."""
    depth = 0
    index = 0
    while index < len(text):
        character = text[index]
        if character == "{":
            if depth == 0:
                back = index - 1
                while back >= 0 and text[back].isspace():
                    back -= 1
                if back >= 0 and text[back] == ")":
                    close = back
                    level = 0
                    start = None
                    while back >= 0:
                        if text[back] == ")":
                            level += 1
                        elif text[back] == "(":
                            level -= 1
                            if level == 0:
                                start = back
                                break
                        back -= 1
                    if start is not None:
                        head = text[max(0, start - 200):start].rstrip()
                        name = re.search(r"([A-Za-z_]\w*)$", head)
                        if name:
                            yield name.group(1), parameters(text[start + 1:close])
            depth += 1
        elif character == "}":
            depth = max(0, depth - 1)
        index += 1


def main() -> int:
    run = pathlib.Path(sys.argv[1])
    data = json.loads((run / "resource-diagnostics.json").read_text())
    signatures = {}
    for path in list((SOURCE / "lib").rglob("*.c")) + list((SOURCE / "src").rglob("*.c")):
        try:
            text = path.read_text(errors="replace")
        except OSError:
            continue
        for name, params in definitions(text):
            if any(is_fp for _, is_fp in params):
                signatures.setdefault(name, params)
    hits = []
    counts = collections.Counter()
    for unit in data["units"]:
        for function in unit["functions"]:
            params = signatures.get(function["function"])
            if params is None:
                continue
            counts[function["status"]] += 1
            if function["status"] != "Unknown":
                continue
            others = {name for name, is_fp in params if not is_fp}
            aliases = {
                match.group(1)
                for diagnostic in function.get("diagnostics", [])
                if diagnostic.get("category") == "unknown_alias"
                for match in [re.search(r"`([^`]+)`", diagnostic.get("description") or "")]
                if match
            }
            if others & aliases:
                hits.append(f"{unit['file']}::{function['function']}")
    print(f"functions with a function-pointer parameter: {dict(counts)}")
    print(f"Unknown with unknown_alias on another parameter: {len(hits)}")
    for hit in sorted(hits)[:40]:
        print(f"  {hit}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

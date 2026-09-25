#!/usr/bin/env python3
"""Read every tail and nested-call refusal against its analysed line.

Usage: tail_nested_lines.py DIAGNOSTICS(.json|.json.gz) UNIT_TEXTS_DIR OUT.json

UNIT_TEXTS_DIR is the output of `../../w43-libc-contracts/scripts/unit_texts.py`
(the analysed `-E -P` text of each unit, with `pairing.json`). A leaf's `line`
indexes that text.

**Tail leaves** (`arguments beyond the declared parameters of `X``). Every call
of X on the leaf's line is parsed lexically; its actuals beyond X's declared
parameter count are its tail. A tail actual is *literal-or-predefined* when
every token of it is a string, character or numeric literal, or one of C11
6.4.2.2's predefined identifier `__func__` and its Clang keyword aliases
`__FUNCTION__` and `__PRETTY_FUNCTION__`. A call is *literal-only* when every
tail actual is. A leaf is counted **removable by a literal-only widening** up
to the number of literal-only calls of X on its line (one line can carry a
whole macro expansion, so several calls and several leaves).

**Nested leaves** (`nested call `Y` inside a modeled call`). The enclosing call
on the line whose argument list contains `Y (` is its outer callee; the leaf is
counted against that outer callee (and against every one, marked ambiguous,
when the line has several).

The bound is recomputed with these classes: the non-Safe functions whose every
leaf the literal-only widening would remove, and the non-Safe functions whose
every leaf is a printf-family nested refusal or a removable tail refusal.
"""

import collections
import gzip
import json
import pathlib
import re
import sys

TAIL = re.compile(r"arguments beyond the declared parameters of `([^`]+)` are outside the resource projection$")
NESTED = re.compile(r"nested call `([^`]+)` inside a modeled call is outside the resource projection$")
TOKEN = re.compile(r'[A-Za-z_][A-Za-z0-9_]*|0[xX][0-9a-fA-F]+[uUlL]*|\d+(?:\.\d*)?(?:[eE][-+]?\d+)?[uUlLfF]*|"(?:[^"\\]|\\.)*"|\'(?:[^\'\\]|\\.)*\'|->|\+\+|--|<<|>>|&&|\|\||[<>=!]=|\S')
PREDEFINED = {"__func__", "__FUNCTION__", "__PRETTY_FUNCTION__"}
# Declared parameter count of each tail-declaring callee in the shipped corpus.
NAMED = {"printf": 1, "fprintf": 2, "snprintf": 3, "sprintf": 2, "mk_print": 2}
PRINTF = {"printf", "fprintf", "snprintf", "sprintf"}


def load(path):
    opener = gzip.open if str(path).endswith(".gz") else open
    with opener(path, "rt") as handle:
        return json.load(handle)


def is_literal(token):
    return token[0] in "\"'" or token[0].isdigit() or token in PREDEFINED


def calls(tokens, name):
    """Every call of `name` on the line: (start, [argument token lists])."""
    found = []
    for index, token in enumerate(tokens):
        if token != name or index + 1 >= len(tokens) or tokens[index + 1] != "(":
            continue
        depth = 0
        arguments = [[]]
        for cursor in range(index + 1, len(tokens)):
            text = tokens[cursor]
            if text == "(":
                depth += 1
                if depth == 1:
                    continue
            elif text == ")":
                depth -= 1
                if depth == 0:
                    break
            elif text == "," and depth == 1:
                arguments.append([])
                continue
            arguments[-1].append(text)
        found.append((index, cursor, arguments))
    return found


def main() -> int:
    data = load(sys.argv[1])
    texts_dir = pathlib.Path(sys.argv[2])
    pairing = json.loads((texts_dir / "pairing.json").read_text())
    cache = {}
    tail_rows = []
    nested_rows = []
    functions = []
    for unit in data["units"]:
        key = f"{unit['library']}/{unit['file']}"
        text_name = pairing.get(key, {}).get("text")
        lines = None
        if text_name:
            if text_name not in cache:
                cache[text_name] = (texts_dir / text_name).read_text(errors="replace").split("\n")
            lines = cache[text_name]
        for entry in unit["functions"]:
            classes = []
            per_line = collections.Counter()
            for diagnostic in entry.get("diagnostics") or []:
                description = diagnostic["description"]
                line = diagnostic["line"]
                tokens = TOKEN.findall(lines[line - 1]) if lines and 0 < line <= len(lines) else None
                tail = TAIL.match(description)
                nested = NESTED.match(description)
                if tail:
                    callee = tail.group(1)
                    per_line[("tail", callee, line)] += 1
                    ordinal = per_line[("tail", callee, line)]
                    shapes = []
                    if tokens is not None:
                        for _, _, arguments in calls(tokens, callee):
                            rest = arguments[NAMED.get(callee, len(arguments)) :]
                            if rest and any(rest):
                                shapes.append(all(arg and all(is_literal(t) for t in arg) for arg in rest))
                    literal_calls = sum(shapes)
                    removable = ordinal <= literal_calls
                    row = {
                        "unit": key,
                        "function": entry["function"],
                        "status": entry["status"],
                        "line": line,
                        "origin": diagnostic.get("origin"),
                        "callee": callee,
                        "calls_with_tail_on_line": len(shapes),
                        "literal_only_calls_on_line": literal_calls,
                        "rdut_line": bool(lines and "RDUT:" in lines[line - 1]),
                        "removable_by_literal_widening": removable,
                        "paired": tokens is not None,
                    }
                    tail_rows.append(row)
                    classes.append("tail-literal" if removable else "tail-other")
                elif nested:
                    inner = nested.group(1)
                    outers = []
                    if tokens is not None:
                        for index, token in enumerate(tokens):
                            if token == inner or index + 1 >= len(tokens) or tokens[index + 1] != "(":
                                continue
                            if not re.match(r"[A-Za-z_]", token):
                                continue
                            for start, end, arguments in calls(tokens, token):
                                span = tokens[start + 2 : end]
                                if any(span[i] == inner and i + 1 < len(span) and span[i + 1] == "(" for i in range(len(span))):
                                    outers.append(token)
                    outers = sorted(set(outers))
                    row = {
                        "unit": key,
                        "function": entry["function"],
                        "status": entry["status"],
                        "line": line,
                        "origin": diagnostic.get("origin"),
                        "inner": inner,
                        "outers": outers,
                        "printf_family_outer": bool(set(outers) & PRINTF),
                        "paired": tokens is not None,
                    }
                    nested_rows.append(row)
                    classes.append("nested-printf" if row["printf_family_outer"] else "nested-other")
                else:
                    classes.append("other")
            functions.append({"unit": key, "function": entry["function"], "status": entry["status"], "classes": classes})

    def bound(selector):
        moved = [f for f in functions if f["status"] != "Safe" and f["classes"] and all(selector(c) for c in f["classes"])]
        return {"functions": len(moved), "moved": [(f["unit"], f["function"], f["status"]) for f in moved]}

    summary = {
        "tail": {
            "leaves": len(tail_rows),
            "by_callee": dict(collections.Counter(r["callee"] for r in tail_rows)),
            "unknown_only_leaves": sum(1 for r in tail_rows if r["status"] == "Unknown"),
            "functions": len({(r["unit"], r["function"]) for r in tail_rows}),
            "unknown_only_functions": len({(r["unit"], r["function"]) for r in tail_rows if r["status"] == "Unknown"}),
            "rdut_line_leaves": sum(r["rdut_line"] for r in tail_rows),
            "removable_by_literal_widening": sum(r["removable_by_literal_widening"] for r in tail_rows),
            "removable_on_rdut_lines": sum(r["removable_by_literal_widening"] and r["rdut_line"] for r in tail_rows),
            "functions_touched_by_literal_widening": len({(r["unit"], r["function"]) for r in tail_rows if r["removable_by_literal_widening"]}),
            "unpaired_leaves": sum(not r["paired"] for r in tail_rows),
        },
        "nested": {
            "leaves": len(nested_rows),
            "printf_family_outer_leaves": sum(r["printf_family_outer"] for r in nested_rows),
            "printf_family_outer_functions": len({(r["unit"], r["function"]) for r in nested_rows if r["printf_family_outer"]}),
            "by_inner_printf_family": dict(collections.Counter(r["inner"] for r in nested_rows if r["printf_family_outer"]).most_common()),
            "by_inner_other": dict(collections.Counter(r["inner"] for r in nested_rows if not r["printf_family_outer"]).most_common()),
            "unpaired_leaves": sum(not r["paired"] for r in nested_rows),
        },
        "bounds": {
            "literal_tail_widening_alone": bound(lambda c: c == "tail-literal"),
            "every_tail_refusal": bound(lambda c: c in {"tail-literal", "tail-other"}),
            "printf_nested_alone": bound(lambda c: c == "nested-printf"),
            "literal_tail_and_printf_nested": bound(lambda c: c in {"tail-literal", "nested-printf"}),
            "every_tail_and_printf_nested": bound(lambda c: c in {"tail-literal", "tail-other", "nested-printf"}),
        },
    }
    out = {"summary": summary, "tail": tail_rows, "nested": nested_rows}
    with open(sys.argv[3], "w") as handle:
        json.dump(out, handle, indent=1, sort_keys=True)
        handle.write("\n")
    print(json.dumps(summary, indent=1, sort_keys=True))
    return 0


if __name__ == "__main__":
    sys.exit(main())

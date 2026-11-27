#!/usr/bin/env python3
"""CG26-TAIL-RESIDUAL-ACTUALS: the declared-tail statements rule 11w still
refuses, with the facts that decide them.

Usage: residuals.py DIAGNOSTICS PARITY.jsonl OUT.json

For each `variadic-tail-statement` parity row (de-duplicated by unit source,
function, line and tokens) that is not admitted: the callee, the trailing
actuals (split from the tokens after the callee's named parameters), the
enclosing function's `root-declarations` row (`lexical_readable`: whether the
lexical declaration reader read it; `ast_answers`: whether rule 10l's Clang
reader answers it) and, per plain-name actual, rule 10l's facts for that name
(`served_never_bound` as the typed positions read it, `ast_answered`). Rows
carrying this card's `tail_readings` (the AFTER build) list them. The tail
leaves (`arguments beyond the declared parameters of `C` …`) are counted per
function from the report, with the function's status and its other leaves.
"""

import collections
import json
import pathlib
import re
import sys

TAIL = re.compile(r"^arguments beyond the declared parameters of `([^`]+)` are outside the resource projection$")
NAMED = {"printf": 1, "fprintf": 2, "snprintf": 3, "sprintf": 2, "mk_print": 2,
         "flb_log_print": 4}
IDENT = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*$")


def split_call(texts):
    """The top-level actuals of the first `callee ( … )` in the tokens."""
    for start in range(len(texts) - 1):
        if IDENT.match(texts[start]) and texts[start + 1] == "(" and texts[start] in NAMED:
            break
    else:
        return None, None
    out, depth, current = [], 0, []
    for token in texts[start + 2:]:
        if token in "([{":
            depth += 1
        elif token in ")]}":
            if depth == 0:
                out.append(current)
                return texts[start], out
            depth -= 1
        if token == "," and depth == 0:
            out.append(current)
            current = []
        else:
            current.append(token)
    return texts[start], None


def main():
    diagnostics, parity, out = sys.argv[1:4]
    path = pathlib.Path(diagnostics)
    if path.is_dir():
        path = path / "resource-diagnostics.json"
    data = json.loads(path.read_text())
    roots, statements, seen = {}, [], set()
    for line in open(parity, encoding="utf-8"):
        if '"root-declarations"' in line:
            row = json.loads(line)
            roots[(row.get("source_sha256"), row["function"])] = row
        elif '"variadic-tail-statement"' in line:
            row = json.loads(line)
            key = (row.get("source_sha256"), row["function"], row["line"], tuple(row["texts"]))
            if key in seen:
                continue
            seen.add(key)
            statements.append(row)
    leaves = collections.defaultdict(lambda: {"status": None, "tail": 0, "other": 0})
    for unit in data["units"]:
        for entry in unit["functions"]:
            record = leaves[entry["function"]]
            order = {"Safe": 0, "Pending": 1, "Unknown": 2, "Unsafe": 3}
            if record["status"] is None or order.get(entry["status"], 2) > order.get(record["status"], 2):
                record["status"] = entry["status"]
            for d in entry.get("diagnostics") or []:
                if TAIL.match(d.get("description", "")):
                    record["tail"] += 1
                else:
                    record["other"] += 1
    rows = []
    for row in statements:
        if row["new_admitted"]:
            continue
        callee, arguments = split_call(row["texts"])
        named = NAMED.get(row["callee"], 0)
        tail = [" ".join(a) for a in arguments[named:]] if arguments else None
        root = roots.get((row.get("source_sha256"), row["function"]), {})
        facts = {n["name"]: n for n in root.get("names", [])}
        names = {}
        for actual in (arguments[named:] if arguments else []):
            if len(actual) == 1 and IDENT.match(actual[0]) and actual[0] != "__qcore_literal__":
                fact = facts.get(actual[0], {})
                names[actual[0]] = {"served_never_bound": fact.get("served_never_bound"),
                                    "ast_answered": fact.get("ast_answered"),
                                    "formal": fact.get("formal")}
        rows.append({
            "callee": row["callee"], "function": row["function"], "line": row["line"],
            "texts": " ".join(row["texts"]), "tail": tail,
            "promoted": row["promoted_scalar_or_pointer"],
            "lexical_readable": root.get("lexical_readable"),
            "ast_answers": root.get("ast_answers"), "names": names,
            "tail_readings": row.get("tail_readings"),
            "function_leaves": leaves.get(row["function"]),
        })
    summary = {
        "statements": len(rows),
        "lexical_readable": dict(collections.Counter(str(r["lexical_readable"]) for r in rows)),
        "functions": len({r["function"] for r in rows}),
        "tail_leaves": {f: v["tail"] for f, v in sorted(leaves.items()) if v["tail"]},
    }
    summary["tail_leaf_total"] = sum(summary["tail_leaves"].values())
    pathlib.Path(out).write_text(json.dumps({"summary": summary, "rows": rows}, indent=1,
                                            ensure_ascii=False) + "\n")
    print(json.dumps(summary, indent=1))
    for r in rows:
        print(r["callee"], r["function"], r["line"], "readable", r["lexical_readable"],
              "ast", r["ast_answers"], "|", r["tail"], r["names"] or "", r["tail_readings"] or "",
              "|", r["function_leaves"])
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

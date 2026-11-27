#!/usr/bin/env python3
"""Card K3: the lexical ceiling of the same-object derived reading on the 11-24
report, before any code change.

usage: derived_yield.py REPORT_JSON SHAPE_LEX_JSON OUT_JSON

A `pointer-valued-store` leaf is a *candidate* when its statement is
`P = [casts] Q ± E` (or `E + Q`) with `Q` the same spelling as `P`, or `P` and `Q`
both `R->m` / `R.m` over one root name `R` (one hop). A row is freed when every
diagnostic it carries is such a candidate leaf whose 11-07 tags are exactly
{pointer-valued-store}. Lexical: the AST reading (`pointer_store_derived.rs`)
decides; this is the prediction's ceiling, not a verdict.
"""
import collections
import json
import pathlib
import re
import sys

CAST = r"(?:\(\s*(?:const\s+)?[A-Za-z_][\w\s]*\*+\s*\)\s*)*"
ONE_HOP = re.compile(r"^([A-Za-z_]\w*)\s*(->|\.)\s*([A-Za-z_]\w*)$")


def candidate(text):
    match = re.match(r"^\s*(.+?)\s*=\s*(.+?);", text)
    if not match or "==" in text.split(";")[0]:
        return None
    lhs, rhs = match.group(1).strip(), match.group(2).strip()
    rhs = re.sub(r"^" + CAST, "", rhs).strip()
    if rhs.startswith("(") and rhs.endswith(")"):
        rhs = rhs[1:-1].strip()
    for pattern in (r"^" + CAST + r"\(?(?P<q>.+?)\)?\s*[-+]\s*[^-+>]+$", r"^[^+]+\+\s*" + CAST + r"(?P<q>.+)$"):
        found = re.match(pattern, rhs)
        if not found:
            continue
        q = re.sub(r"^" + CAST, "", found.group("q").strip()).strip("() ")
        if q == lhs:
            return "same-place"
        a, b = ONE_HOP.match(lhs), ONE_HOP.match(q)
        if a and b and a.group(1) == b.group(1) and a.group(2) == b.group(2):
            return "same-object"
    return None


def main():
    report = json.load(open(sys.argv[1]))
    lex = json.load(open(sys.argv[2]))
    flagged = {}
    for leaf in lex["leaves"]:
        kind = candidate(leaf["text"]) if leaf["tags"] == ["pointer-valued-store"] else None
        flagged[(leaf["file"], leaf["function"], leaf["line"], leaf["family"])] = kind
    by_kind = collections.Counter(v for v in flagged.values() if v)
    freed, touched = [], set()
    from recensus import family  # noqa: E402
    for unit in report["units"]:
        for function in unit.get("functions", []):
            if function["status"] != "Unknown" or not function["diagnostics"]:
                continue
            ok, any_candidate = True, False
            for d in function["diagnostics"]:
                key = (unit["file"], function["function"], d["line"], family(d))
                if flagged.get(key):
                    any_candidate = True
                else:
                    ok = False
            name = f"{unit.get('library')}::{unit['file']}::{function['function']}"
            if any_candidate:
                touched.add(name)
            if ok:
                freed.append(name)
    out = {"candidate_leaves": dict(by_kind), "unknown_rows_touched": len(touched),
           "freed_rows": len(freed), "freed": sorted(freed), "touched": sorted(touched)}
    pathlib.Path(sys.argv[3]).write_text(json.dumps(out, indent=1, sort_keys=True) + "\n")
    print(json.dumps({k: out[k] for k in ("candidate_leaves", "unknown_rows_touched", "freed_rows", "freed")},
                     indent=1))


if __name__ == "__main__":
    sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
    main()

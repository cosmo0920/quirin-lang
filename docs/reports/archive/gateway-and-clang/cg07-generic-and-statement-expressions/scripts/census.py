#!/usr/bin/env python3
"""CG07-GENERIC-AND-STATEMENT-EXPRESSIONS: census of the statement-expression
consultations in a QCORE_TYPED_PATH_PARITY stream, and of the E17/E19 leaves of
a corpus run.

Usage: census.py PARITY.jsonl DIAGNOSTICS [--json OUT]

Parity rows with `position` `statement-expression-statement` / `-return` are
deduplicated by (function, line, texts) (a function's CFG is built more than
once per run) and counted by outcome, refusal and, per statement expression,
by position, family and admission. The diagnostics give the leaf counts the
card names: the braced-construct refusal, `_Generic` and `bsearch`/`memchr`
uncontracted calls, and `__assert_fail`.
"""

import argparse
import collections
import gzip
import json
import pathlib

BRACED = "braced initializer, expression, or declaration is not modeled"


def rows(path):
    seen = set()
    with open(path, encoding="utf-8") as handle:
        for line in handle:
            row = json.loads(line)
            if not str(row.get("position", "")).startswith("statement-expression"):
                continue
            key = (row.get("function"), row.get("line"), tuple(row.get("texts", [])), row["position"])
            if key in seen:
                continue
            seen.add(key)
            yield row


def leaves(path):
    path = pathlib.Path(path)
    if path.is_dir():
        path = path / "resource-diagnostics.json"
    raw = path.read_bytes()
    if path.suffix == ".gz":
        raw = gzip.decompress(raw)
    data = json.loads(raw)
    counts = collections.Counter()
    functions = collections.defaultdict(set)
    for unit in data["units"]:
        for function in unit["functions"]:
            for diagnostic in function.get("diagnostics", []):
                description = diagnostic.get("description") or ""
                for label, match in (
                    ("braced", description == BRACED),
                    ("_Generic call", "`_Generic`" in description),
                    ("bsearch call", "function `bsearch`" in description),
                    ("memchr call", "function `memchr`" in description),
                    ("__assert_fail call", "`__assert_fail`" in description),
                ):
                    if match:
                        counts[label] += 1
                        functions[label].add((unit["file"], function["function"]))
    return {label: {"leaves": counts[label], "functions": len(functions[label])} for label in counts}


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("parity")
    parser.add_argument("diagnostics")
    parser.add_argument("--json", type=pathlib.Path)
    args = parser.parse_args()
    outcome = collections.Counter()
    refusals = collections.Counter()
    placements = collections.Counter()
    families = collections.defaultdict(collections.Counter)
    functions = collections.defaultdict(set)
    for row in rows(args.parity):
        admitted = bool(row.get("new_admitted"))
        outcome[(row["position"], admitted)] += 1
        functions[admitted].add(row.get("function"))
        if not admitted:
            refusals[row.get("new_refusal")] += 1
        for placement in row.get("statement_expressions") or []:
            placements[(placement.get("position"), placement.get("admitted"))] += 1
            family = placement.get("family") or "?"
            families[family]["admitted" if placement.get("admitted") else "refused"] += 1
            if not placement.get("admitted"):
                families[family][placement.get("refusal") or "?"] += 1
    record = {
        "consultations": {f"{p} admitted={a}": n for (p, a), n in sorted(outcome.items())},
        "functions_with_an_admitted_consultation": len(functions[True]),
        "functions_with_a_declined_consultation": len(functions[False]),
        "declined_by_reason": dict(refusals.most_common()),
        "statement_expressions_by_position": {
            f"{p} admitted={a}": n for (p, a), n in sorted(placements.items(), key=str)
        },
        "families": {
            family: dict(counts)
            for family, counts in sorted(
                families.items(), key=lambda item: -sum(v for k, v in item[1].items() if k in ("admitted", "refused"))
            )[:25]
        },
        "leaves": leaves(args.diagnostics),
    }
    print(json.dumps(record, indent=1, ensure_ascii=False))
    if args.json:
        args.json.write_text(json.dumps(record, indent=1, ensure_ascii=False) + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

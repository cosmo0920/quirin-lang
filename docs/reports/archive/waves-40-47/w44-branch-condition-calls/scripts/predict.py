#!/usr/bin/env python3
"""Card W44-BRANCH-CONDITION-CALLS: split the condition families and forecast.

Input: one census run directory (`run_audit.py ... --census`) holding
`resource-diagnostics.json` and `condition-call-census.jsonl`. The census binary
is the census-instrumentation commit, whose desugaring recogniser is compiled
in but not consulted (`ADMIT = false`); every record carries the rule's own
answer (`admitted` or its refusal code) for one controlling expression that
every pre-existing route refused, including call-free ones (`no-call`).

Dedup: the analyzer parses a function several times (derivation rounds, the
final analysis, a certificate recheck). Only the **last** record per
(unit_seq, function, line, position, texts) is kept, which is the final
analysis's answer.

Join: `unit_seq` is matched to a document unit by greatest overlap of
(function, line) keys (W43-INDIRECT-ACCESS-GENERAL's `match_units`). A leaf
`(unit, function, line, family)` of the five condition families is explained by
every distinct final record at that (function, line) of the family's position,
and is forecast to leave its family only if every such record is `admitted`.

Splits (Unknown functions): by outcome code; for the call-bearing leaves, by
operator shape, by callee contract status and by result kind of the callees.

Function forecast: an Unknown function whose every leaf is a forecast-admitted
condition leaf is an upper bound on Unknown -> Safe; the flow half (a leak or an
unresolved alias the admitted calls expose) is not visible here.

Usage: predict.py RUN_DIR OUT_JSON OUT_TXT
"""

import collections
import json
import pathlib
import re
import sys

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parents[1] / "w43-indirect-access-general" / "scripts"))

import shape_census as census  # noqa: E402

FAMILY = re.compile(
    r"unmodeled call, aggregate or indirect access in "
    r"(branch condition|loop condition|for-loop condition|do-while condition|switch expression)$"
)
POSITION = {
    "branch condition": "if",
    "loop condition": "while",
    "for-loop condition": "for",
    "do-while condition": "do-while",
    "switch expression": "switch",
}


def load(path):
    """seq -> {(function, line, position): [final records]}."""
    last = {}
    for line in open(path):
        record = json.loads(line)
        key = (record["unit_seq"], record["function"], record["line"], record["position"],
               tuple(record["texts"]))
        last[key] = record
    stream = collections.defaultdict(lambda: collections.defaultdict(list))
    for (seq, function, line, position, _), record in last.items():
        stream[seq][(function, line, position)].append(record)
    return stream


def shape(texts):
    joined = " ".join(texts)
    parts = []
    if "& &" in joined:
        parts.append("and")
    if "| |" in joined:
        parts.append("or")
    if "?" in texts:
        parts.append("ternary")
    if "!" in texts and "! =" not in joined.replace("! =", "!="):
        parts.append("not")
    if any(t == "=" for i, t in enumerate(texts)
           if (i + 1 >= len(texts) or texts[i + 1] != "=")
           and (i == 0 or texts[i - 1] not in ("=", "!", "<", ">"))):
        parts.append("assign")
    return "+".join(parts) or "plain"


def callee_status(record):
    callees = record["callees"]
    if not callees:
        return "no-call"
    if any(not c["contracted"] for c in callees):
        return "some-uncontracted"
    if any(c["local"] for c in callees):
        return "contracted-with-local"
    return "all-contracted-external"


def result_kind(record):
    kinds = sorted({c["result"] or "uncontracted" for c in record["callees"]})
    return "+".join(kinds) or "none"


def main():
    run = pathlib.Path(sys.argv[1])
    out_json = pathlib.Path(sys.argv[2])
    out_txt = pathlib.Path(sys.argv[3])
    _, functions, unit_keys = census.load_document(run / "resource-diagnostics.json")
    stream = load(run / "condition-call-census.jsonl")
    assignment = census.match_units(
        [{seq: {(f, l, None): [] for (f, l, _) in keys} for seq, keys in stream.items()}],
        unit_keys,
    )

    totals = collections.Counter()
    admitted_leaves = collections.Counter()
    unexplained = collections.Counter()
    by_outcome = collections.defaultdict(lambda: [0, set()])
    by_shape = collections.defaultdict(lambda: [0, set()])
    by_status = collections.defaultdict(lambda: [0, set()])
    by_result = collections.defaultdict(lambda: [0, set()])
    by_call_refusal = collections.defaultdict(lambda: [0, set()])
    family_functions = collections.defaultdict(set)
    forecast_admitted = {}
    function_moves = []
    sole_moves = collections.Counter()
    for fkey, data in functions.items():
        if data["status"] != "Unknown":
            continue
        ukey, function = fkey[:2], fkey[2]
        seq = assignment.get(ukey)
        records = stream.get(seq, {})
        leaves = set(data["leaves"])
        all_forecast = True
        for line, description in sorted(leaves):
            match = FAMILY.match(description)
            if not match:
                all_forecast = False
                continue
            family = match.group(1)
            totals[family] += 1
            family_functions[family].add(fkey)
            found = records.get((function, line, POSITION[family]), [])
            if not found:
                unexplained[family] += 1
                all_forecast = False
                continue
            outcomes = sorted({r["outcome"] for r in found})
            admitted = outcomes == ["admitted"]
            forecast_admitted[(fkey, line, description)] = admitted
            if admitted:
                admitted_leaves[family] += 1
            else:
                all_forecast = False
            label = "admitted" if admitted else "+".join(o for o in outcomes if o != "admitted")
            by_outcome[(family, label)][0] += 1
            by_outcome[(family, label)][1].add(fkey)
            called = [r for r in found if r["outcome"] != "no-call"]
            if called:
                shp = "+".join(sorted({shape(r["texts"]) for r in called}))
                st = "+".join(sorted({callee_status(r) for r in called}))
                rk = "+".join(sorted({result_kind(r) for r in called}))
                for table, key in ((by_shape, shp), (by_status, st), (by_result, rk)):
                    table[(family, key, admitted)][0] += 1
                    table[(family, key, admitted)][1].add(fkey)
                for r in called:
                    if r["outcome"] == "call-refused":
                        reason = (r.get("reason") or "")[:80]
                        by_call_refusal[reason][0] += 1
                        by_call_refusal[reason][1].add(fkey)
        if all_forecast and leaves:
            function_moves.append("::".join(str(x) for x in fkey))
        blockers = {normalize(d) for _, d in leaves}
        if len(blockers) == 1:
            only = next(iter(blockers))
            if FAMILY.match(only):
                sole_moves[FAMILY.match(only).group(1)] += 1

    def rows(table):
        out = []
        for key, (leaves, funcs) in sorted(table.items(), key=lambda kv: (-kv[1][0], str(kv[0]))):
            out.append({"key": list(key) if isinstance(key, tuple) else key,
                        "leaves": leaves, "functions": len(funcs)})
        return out

    result = {
        "run": str(run),
        "family_leaves": dict(totals),
        "family_functions": {k: len(v) for k, v in family_functions.items()},
        "family_sole_blocked": dict(sole_moves),
        "forecast_admitted_leaves": dict(admitted_leaves),
        "unexplained_leaves": dict(unexplained),
        "by_outcome": rows(by_outcome),
        "by_shape": rows(by_shape),
        "by_callee_status": rows(by_status),
        "by_result_kind": rows(by_result),
        "call_refusal_reasons": rows(by_call_refusal),
        "forecast_function_moves_upper_bound": sorted(function_moves),
    }
    out_json.write_text(json.dumps(result, indent=1) + "\n")
    with open(out_txt, "w") as text:
        text.write("W44-BRANCH-CONDITION-CALLS census forecast (Unknown functions)\n\n")
        text.write("family: leaves functions sole | forecast admitted leaves | unexplained\n")
        for family in POSITION:
            text.write(f"  {family:20} {totals[family]:6} {len(family_functions[family]):6} "
                       f"{sole_moves[family]:4} | {admitted_leaves[family]:6} | {unexplained[family]}\n")
        text.write(f"\nforecast function moves (upper bound, every leaf admitted): {len(function_moves)}\n")
        for name, table in (("outcome", by_outcome), ("operator shape (call-bearing)", by_shape),
                            ("callee contract status (call-bearing)", by_status),
                            ("callee result kind (call-bearing)", by_result),
                            ("classifier refusal of a refused call", by_call_refusal)):
            text.write(f"\nby {name}: leaves functions key\n")
            for row in rows(table)[:40]:
                text.write(f"  {row['leaves']:6} {row['functions']:6}  {row['key']}\n")
    print(out_txt.read_text())
    return 0


def normalize(text):
    return re.sub(r"`[^`]*`", "`_`", str(text))


if __name__ == "__main__":
    sys.exit(main())

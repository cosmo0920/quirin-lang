#!/usr/bin/env python3
"""Card W44-INDIRECT-CALL-ARGUMENT-ACCESS: the call-argument census.

Input: one instrumented run directory (`run_audit.py --census` with an
`indirect-census` build of the census commit) holding

* `resource-diagnostics.json` - the authority for which leaves exist;
* `indirect-census.jsonl` and `statement-census.jsonl` - W43-INDIRECT-ACCESS-
  GENERAL's streams, read through its `residuals.py` classifier so that the
  family is exactly its `call-argument` class;
* `call-argument-census.jsonl` - this card's stream: one record per simple
  statement the lexical gate refused whose first token run carries a call,
  with every actual's position (from the callee's declaration), the
  recogniser's answer, the bases' declaration facts, and, where at least one
  actual is a candidate, the classifier's own answer when the statement is
  classified again with the candidates admitted (`outcome`, `used`).

Every leaf of the family (in an Unknown function) is put in one row of each
split. A leaf is **forecast removed** when every census record at its key is a
probe whose outcome is a contract call or an element store that used a
candidate, and every W43 indirect record at that key has the same token text
(so no other statement on the line keeps the refusal).

Usage: call_argument_census.py RUN_DIR OUT_JSON OUT_TXT [--union]
"""

import collections
import json
import pathlib
import sys

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parents[1] / "w43-indirect-access-general" / "scripts"))

import predict  # noqa: E402
import residuals  # noqa: E402
import shape_census as census  # noqa: E402

ADMITTED_OUTCOMES = {"contract-call", "element-store"}
UNION = "--union" in sys.argv[4:]


def load_calls(path):
    stream = collections.defaultdict(lambda: collections.defaultdict(list))
    seen = set()
    if not path.exists():
        return stream
    for line in open(path):
        record = json.loads(line)
        seq = record.pop("unit_seq")
        key = (record["function"], record["line"], census.INDIRECT)
        blob = json.dumps(record, sort_keys=True)
        if (seq, blob) in seen:
            continue
        seen.add((seq, blob))
        stream[seq][key].append(record)
    return stream


def base_kind(base):
    if base["formal"]:
        where = "formal"
    else:
        where = "local"
    if base["never_bound"]:
        binding = "never-bound"
    elif base["may_bind"]:
        binding = "may-bind"
    else:
        binding = "bound-by-closure"
    return f"{where}/{binding}"


def record_class(record):
    """(statement class, per-refusing-actual rows) of one call record."""
    refusing = [a for a in record["arguments"] if a["access"]]
    rows = []
    for argument in refusing:
        if argument["recognised"]:
            shape = argument["recognised"]
            kinds = sorted({base_kind(b) for b in argument["bases"]}) or ["no-base"]
        else:
            shape = "refused:" + (argument["reason"] or "?")
            kinds = ["-"]
        rows.append((argument["position"], shape, "+".join(kinds)))
    if record.get("descriptor"):
        cls = "descriptor"
    elif all(a["position"] == "uncontracted" for a in record["arguments"]) or not record["arguments"]:
        cls = "uncontracted" if record["arguments"] else "no-actual"
    elif record.get("outcome") in ADMITTED_OUTCOMES and record.get("used"):
        cls = "admitted:" + record["outcome"]
    elif record.get("outcome") is not None:
        cls = "probe-refused:" + census.normalize(record["outcome"])
    else:
        admissible = [a for a in refusing if a["position"] in ("borrowed", "tail") and a["recognised"]]
        if not admissible:
            cls = "no-candidate"
        else:
            cls = "no-probe"
    return cls, rows


def main():
    run = pathlib.Path(sys.argv[1])
    _, functions, unit_keys = census.load_document(run / "resource-diagnostics.json")
    indirect = census.load_stream(run / "indirect-census.jsonl", lambda record: census.INDIRECT)
    statements = census.load_stream(
        run / "statement-census.jsonl", lambda record: census.outcome_family(record["outcome"])
    )
    calls = load_calls(run / "call-argument-census.jsonl")
    assignment = census.match_units([indirect, statements, calls], unit_keys)

    family = {}  # leaf -> (records of W43 streams, call records)
    for fkey, data in functions.items():
        if data["status"] != "Unknown":
            continue
        seq = assignment.get(fkey[:2])
        for line, description in data["leaves"]:
            if description != census.INDIRECT:
                continue
            key = (fkey[2], line, description)
            records = []
            if seq is not None:
                records = indirect.get(seq, {}).get(key, []) or statements.get(seq, {}).get(key, [])
            classes = {residuals.classify(r) for r in records} or {"other"}
            call_records = calls.get(seq, {}).get(key, []) if seq is not None else []
            in_window = min(classes, key=residuals.ORDER.index) == "call-argument"
            # `--union`: also every leaf whose statement this card's call stream
            # recorded. W43's shape classifier files some call statements under
            # `chain` (a typed declaration `T * x = f(s.f)`, measured by
            # W44-INDIRECT-CHAIN-ACCESS); the frozen forecast used the W43
            # window, so the default keeps it.
            if not in_window and not (UNION and call_records):
                continue
            family[(fkey, line)] = (records, call_records)

    unknown = {f for f, d in functions.items() if d["status"] == "Unknown"}

    def blockers(fkey):
        return {census.normalize(d) for _, d in functions[fkey]["leaves"]}

    leaves = len(family)
    touched = {fkey for fkey, _ in family}
    sole = {f for f in touched if blockers(f) == {census.normalize(census.INDIRECT)}}

    statement_split = collections.Counter()
    statement_functions = collections.defaultdict(set)
    actual_split = collections.Counter()
    forecast = set()
    exchange_risk = set()
    for (fkey, line), (records, call_records) in family.items():
        if not call_records:
            cls = "not-a-simple-statement"
        else:
            classes = sorted({record_class(r)[0] for r in call_records})
            cls = classes[0] if len(classes) == 1 else "mixed:" + "|".join(classes)
            for record in call_records:
                for row in record_class(record)[1]:
                    actual_split[row] += 1
        statement_split[cls] += 1
        statement_functions[cls].add(fkey)
        texts_w43 = {tuple(r["texts"]) for r in records if "texts" in r}
        texts_calls = {tuple(r["texts"]) for r in call_records}
        admitted = (
            call_records
            and all(record_class(r)[0].startswith("admitted:") for r in call_records)
            and texts_w43 <= texts_calls
        )
        if admitted:
            forecast.add((fkey, line))
            for record in call_records:
                for argument in record["arguments"]:
                    if argument["index"] in record["used"]:
                        if any(not b["never_bound"] for b in argument["bases"]):
                            exchange_risk.add((fkey, line))

    def decided(fkey, cleared):
        return all(
            description == census.INDIRECT and (fkey, line) in cleared
            for line, description in functions[fkey]["leaves"]
        )

    decided_functions = sorted(f for f in unknown if decided(f, forecast))
    decided_without_risk = sorted(f for f in unknown if decided(f, forecast - exchange_risk))
    # Functions whose only remaining blockers would be forecast leaves of other
    # families: reported as "pairs" as the W43 census did.
    remaining_after = collections.Counter()
    for f in touched:
        rest = {
            census.normalize(d)
            for line, d in functions[f]["leaves"]
            if not (d == census.INDIRECT and (f, line) in forecast)
        }
        if len(rest) == 1:
            remaining_after[next(iter(rest))] += 1

    result = {
        "run": str(run),
        "units_matched": len(assignment),
        "family": {"leaves": leaves, "unknown_functions": len(touched), "sole_blocked": len(sole)},
        "statement_split": {
            k: {"leaves": v, "functions": len(statement_functions[k])} for k, v in statement_split.most_common()
        },
        "actual_split": [
            {"position": p, "shape": s, "bases": b, "actuals": n} for (p, s, b), n in actual_split.most_common()
        ],
        "forecast": {
            "leaves_removed": len(forecast),
            "leaves_with_exchange_risk": len(exchange_risk),
            "functions_decided_upper": len(decided_functions),
            "functions_decided_without_risk": len(decided_without_risk),
            "decided_upper": ["/".join(str(p) for p in f) for f in decided_functions],
            "decided_without_risk": ["/".join(str(p) for p in f) for f in decided_without_risk],
            "one_family_left_after": dict(remaining_after.most_common(10)),
            "leaves": sorted(["/".join(str(p) for p in f), line] for f, line in forecast),
            "exchange_risk_leaves": sorted(["/".join(str(p) for p in f), line] for f, line in exchange_risk),
        },
    }
    pathlib.Path(sys.argv[2]).write_text(json.dumps(result, indent=1) + "\n")
    lines = [
        f"run: {run}",
        f"units matched {len(assignment)} / {len(unit_keys)}",
        "",
        f"family ({'W43 class call-argument widened by the call stream' if UNION else 'W43 residuals.py class call-argument'}, Unknown functions): {leaves} leaves / {len(touched)} functions / {len(sole)} sole-blocked",
        "",
        "statement split (leaves, functions):",
    ]
    for cls, n in statement_split.most_common():
        lines.append(f"  {n:6} {len(statement_functions[cls]):5}  {cls}")
    lines += ["", "refusing actuals of simple statements: position x shape x base kind (actuals)"]
    for (p, s, b), n in actual_split.most_common(60):
        lines.append(f"  {n:6}  {p:12} {s:26} {b}")
    lines += [
        "",
        f"forecast: {len(forecast)} leaves removed ({len(exchange_risk)} of them over a base that is not never-bound, exchange risk)",
        f"functions decided: at most {len(decided_functions)}; {len(decided_without_risk)} without any exchange-risk leaf",
    ]
    for f in decided_functions:
        mark = "" if f in decided_without_risk else "  (exchange risk)"
        lines.append(f"  {'/'.join(str(p) for p in f)}{mark}")
    lines += ["", "functions left with one other family after the forecast:"]
    for fam, n in remaining_after.most_common(10):
        lines.append(f"  {n:5}  {fam}")
    pathlib.Path(sys.argv[3]).write_text("\n".join(lines) + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())

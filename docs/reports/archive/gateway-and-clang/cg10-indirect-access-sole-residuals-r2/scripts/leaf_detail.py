#!/usr/bin/env python3
"""leaf_detail.py CLASSIFY.json CENSUS_PREFIX RUN-PARITY.jsonl OUT.json -- card
CG10-INDIRECT-ACCESS-SOLE-RESIDUALS round 2: every leaf of the Unknown
functions the access refusal blocks alone, with what the readers recorded at
its line, for reading by hand and for the ranking (rank.py).

Per leaf: the class (classify.py), the statement tokens, the first call's
callee and, per actual, its contract position and the recogniser's answer
(the call-argument census, last row at (function, line)); rule 12k's answer at
the line (`position: "uncontracted-access"` in the parity stream); the return
census row's expression; and the parity positions recorded at the line."""
import collections
import json
import pathlib
import sys


def last_rows(path, names, kind=None):
    rows = {}
    with open(path, encoding="utf-8") as stream:
        for line in stream:
            try:
                row = json.loads(line)
            except json.JSONDecodeError:
                continue
            if kind and row.get("kind") != kind:
                continue
            if row.get("function") in names:
                rows[(row.get("function"), row.get("line"))] = row
    return rows


def main() -> int:
    classes = json.loads(pathlib.Path(sys.argv[1]).read_text())
    prefix = sys.argv[2]
    names = {leaf["function"] for leaf in classes["leaves"]}
    calls = last_rows(prefix + "-call-argument.jsonl", names, "call_argument")
    gate = last_rows(prefix + "-indirect.jsonl", names)
    run = pathlib.Path(prefix)
    returns = last_rows(run / "return-census.jsonl", names)
    positions = collections.defaultdict(list)
    attribution = {}
    with open(sys.argv[3], encoding="utf-8") as stream:
        for line in stream:
            row = json.loads(line)
            if row.get("function") not in names:
                continue
            key = (row["function"], row.get("line"))
            positions[key].append(row.get("position"))
            if row.get("position") == "uncontracted-access":
                attribution[key] = {"callee": row.get("callee"), "answer": row.get("answer"),
                                    "form": row.get("form")}
    out = []
    for leaf in classes["leaves"]:
        key = (leaf["function"], leaf["line"])
        call = calls.get(key)
        record = dict(leaf)
        record["gate_texts"] = " ".join((gate.get(key) or {}).get("texts") or [])
        if call is not None:
            record["callee"] = call.get("callee")
            record["actuals"] = [
                {"index": a.get("index"), "position": a.get("position"),
                 "access": a.get("access"), "recognised": a.get("recognised"),
                 "reason": a.get("reason"), "texts": " ".join(a.get("texts") or [])}
                for a in call.get("arguments") or []]
            record["probe"] = call.get("outcome") or call.get("result")
        if key in attribution:
            record["rule_12k"] = attribution[key]
        if key in returns:
            record["return"] = returns[key].get("expression") or returns[key].get("texts")
        record["parity_positions"] = sorted(set(p for p in positions.get(key, []) if p))
        out.append(record)
    pathlib.Path(sys.argv[4]).write_text(json.dumps(out, indent=1, ensure_ascii=False) + "\n")
    print(len(out), "leaves")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

#!/usr/bin/env python3
"""Predict the arbiter from the widened-scope census, before implementation.

Usage: predict.py DIAGNOSTICS.json[.gz] SCOPE_CENSUS.jsonl OUT.json

SCOPE_CENSUS is the `QCORE_CALLEE_SCOPE_CENSUS` stream of the census build: per
unit, what the widened derivation decided for each unselected definition the
selection reaches. A `validated` row is a summary the route will install.

The prediction is mechanical and says how it is made:

* **removed leaves**: every `call to uncontracted function `C`` leaf, in any
  function of a unit, whose callee `C` validated in that unit. A modeled call
  can still refuse on its argument obligations, so a removed leaf may be
  *replaced* by another refusal on the same line; the prediction does not
  guess which, and the arbiter attributes it;
* **movers**: an Unknown function every one of whose diagnostics is such a
  leaf. It is `neutral` when every summary it relies on is all-`Borrowed` with
  result `None` (nothing for the call to discharge or establish), otherwise
  `effectful` and less certain;
* **transitive movers**: a mover defined in the unit is itself offered to W4,
  so its callers' leaves naming it are re-examined with it added to the unit's
  validated set, to a fixed point. These are *candidates*: the mover's own
  summary must still validate.
"""

import collections
import gzip
import json
import re
import sys

CALL = re.compile(r"call to uncontracted function `([^`]+)`")
NEUTRAL = re.compile(r"^ResourceFunctionContract \{ parameters: \[(Borrowed(, Borrowed)*)?\], result: None \}$")


def main() -> int:
    diagnostics, scope_path, out = sys.argv[1:4]
    opener = gzip.open if diagnostics.endswith(".gz") else open
    data = json.load(opener(diagnostics))
    rows = {}
    for line in open(scope_path):
        row = json.loads(line)
        rows.setdefault(tuple(sorted(row["selected"])), row)
    validated = {}
    for unit in data["units"]:
        key = unit["library"] + "/" + unit["file"]
        row = rows.get(tuple(sorted(f["function"] for f in unit["functions"])))
        validated[key] = {}
        if row is None:
            continue
        for callee in row["callees"]:
            if callee["outcome"] == "validated":
                validated[key][callee["name"]] = callee["summary"]
    removed = []
    movers = []
    transitive = []
    for unit in data["units"]:
        key = unit["library"] + "/" + unit["file"]
        summaries = dict(validated[key])
        moved = set()
        first_round = True
        while True:
            added = False
            for entry in unit["functions"]:
                name = entry["function"]
                if name in moved:
                    continue
                ds = entry.get("diagnostics") or []
                hits = [CALL.fullmatch(d["description"]) for d in ds]
                decided = [m.group(1) for m in hits if m and m.group(1) in summaries]
                if first_round:
                    for d, m in zip(ds, hits):
                        if m and m.group(1) in summaries:
                            removed.append({"unit": key, "function": name, "status": entry["status"],
                                            "line": d["line"], "callee": m.group(1)})
                if entry["status"] != "Unknown" or not ds:
                    continue
                if all(m and m.group(1) in summaries for m in hits):
                    relied = sorted(set(decided))
                    neutral = all(summaries[c] is not None and NEUTRAL.match(summaries[c]) for c in relied)
                    record = {"unit": key, "function": name, "leaves": len(ds), "callees": relied,
                              "kind": "neutral" if neutral else "effectful"}
                    (movers if first_round else transitive).append(record)
                    moved.add(name)
                    summaries[name] = None
                    added = True
            first_round = False
            if not added:
                break
    by_callee = collections.Counter(r["callee"] for r in removed)
    record = {
        "validated_rows": sum(len(v) for v in validated.values()),
        "validated_callees": sorted({c for v in validated.values() for c in v}),
        "removed_leaves": len(removed),
        "removed_in_unknown_functions": sum(1 for r in removed if r["status"] == "Unknown"),
        "removed_by_status": dict(collections.Counter(r["status"] for r in removed)),
        "removed_by_callee": dict(by_callee.most_common()),
        "movers": movers,
        "transitive_candidates": transitive,
        "removed": removed,
    }
    with open(out, "w") as handle:
        json.dump(record, handle, indent=1)
        handle.write("\n")
    print(json.dumps({k: record[k] for k in ("validated_rows", "removed_leaves",
                      "removed_in_unknown_functions", "removed_by_status", "removed_by_callee")}, indent=1))
    for m in movers:
        print("mover", m)
    for m in transitive:
        print("transitive", m)
    return 0


if __name__ == "__main__":
    sys.exit(main())

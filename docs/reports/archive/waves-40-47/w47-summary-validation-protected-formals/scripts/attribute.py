#!/usr/bin/env python3
"""attribute.py BASELINE_DIR AFTER_DIR CENSUS_JSON [--out FILE]

Diffs the AFTER run's function rows against the baseline archive and
attributes every changed row to this card's mechanism, using the record-only
census (scripts/census.py over the MEASURE run):

* `withdrawn`   the function whose own summary no longer validates;
* `caller`      a function whose unit graph reaches a withdrawn summary;
* otherwise     `unattributed` (the script exits 1).

Every status move is listed with its old and new status; a Safe -> non-Safe
move is flagged `correction`. Leaf deltas are (category, description) multisets.
Exit 0 only when every changed row is attributed and no leak, double release,
use after release or Unsafe row is added.
"""
import argparse
import collections
import gzip
import json
import pathlib
import sys


def load(path):
    path = pathlib.Path(path)
    for candidate in (path / "resource-diagnostics.json", path / "resource-diagnostics.json.gz"):
        if candidate.exists():
            opener = gzip.open if candidate.suffix == ".gz" else open
            with opener(candidate, "rt") as handle:
                return json.load(handle)
    raise SystemExit(f"no resource-diagnostics.json in {path}")


def rows(report):
    out = {}
    for unit in report["units"]:
        for row in unit.get("functions", []):
            leaves = collections.Counter(
                (d.get("category"), d.get("description") or d.get("variable") or "")
                for d in row.get("diagnostics", []))
            out[(unit["file"], row["function"])] = (row["status"], leaves)
    return out, {u["file"]: u.get("status") for u in report["units"]}, report.get("diagnostic_counts")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("baseline")
    parser.add_argument("after")
    parser.add_argument("census")
    parser.add_argument("--out")
    args = parser.parse_args()
    base, base_units, base_counts = rows(load(args.baseline))
    after, after_units, after_counts = rows(load(args.after))
    census = json.loads(pathlib.Path(args.census).read_text())
    withdrawn = {(w["file"], w["function"]) for w in census["withdrawn"]}
    reaching = {(u["file"], name) for u in census["units"] for name in u["reaching_withdrawn"]}
    changed = []
    bad = False
    for key in sorted(set(base) | set(after)):
        b = base.get(key, (None, collections.Counter()))
        a = after.get(key, (None, collections.Counter()))
        if b == a:
            continue
        added = a[1] - b[1]
        removed = b[1] - a[1]
        cause = "withdrawn" if key in withdrawn else "caller" if key in reaching else "unattributed"
        entry = {
            "file": key[0], "function": key[1], "cause": cause,
            "status": [b[0], a[0]],
            "added": sorted([list(k) + [v] for k, v in added.items()]),
            "removed": sorted([list(k) + [v] for k, v in removed.items()]),
        }
        if b[0] == "Safe" and a[0] != "Safe":
            entry["correction"] = True
        if cause == "unattributed":
            bad = True
        if any(k[0] in ("leak", "double_release", "use_after_release") for k in added) or (
                a[0] == "Unsafe" and b[0] != "Unsafe"):
            entry["violation_added"] = True
            bad = True
        changed.append(entry)
    unit_moves = {f: [base_units.get(f), after_units.get(f)] for f in set(base_units) | set(after_units)
                  if base_units.get(f) != after_units.get(f)}
    result = {
        "functions_before": dict(collections.Counter(v[0] for v in base.values())),
        "functions_after": dict(collections.Counter(v[0] for v in after.values())),
        "units_before": dict(collections.Counter(base_units.values())),
        "units_after": dict(collections.Counter(after_units.values())),
        "diagnostic_counts_before": base_counts,
        "diagnostic_counts_after": after_counts,
        "unit_moves": unit_moves,
        "changed_rows": changed,
        "causes": dict(collections.Counter(c["cause"] for c in changed)),
        "status_moves": [c for c in changed if c["status"][0] != c["status"][1]],
    }
    text = json.dumps(result, indent=1, sort_keys=True)
    if args.out:
        pathlib.Path(args.out).write_text(text + "\n")
    summary = {k: result[k] for k in ("functions_before", "functions_after", "units_before", "units_after",
                                       "diagnostic_counts_before", "diagnostic_counts_after", "unit_moves",
                                       "causes")}
    summary["status_moves"] = [(c["file"], c["function"], c["status"]) for c in result["status_moves"]]
    print(json.dumps(summary, indent=1))
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())

#!/usr/bin/env python3
"""summarize.py CENSUS_DIR [BEFORE_DIR] -- the strict-pair figures of one
census run (census.sh): the clause table, the observations outside the
contract, the callers' refusals (per site and root), the release and C4
tallies, and for the pause-collectors roots every release's kept-path
decision. With BEFORE_DIR, every release and C4 site whose verdict label moved
and whether any other key besides the lifecycle ones moved. Read-only; prints
JSON.
"""
import gzip
import json
import pathlib
import sys
from collections import Counter

TREES = ["fixed", "previous", "system", "corpus"]
LIFECYCLE_KEYS = {"lifecycle_violations", "lifecycle_stops", "lifecycle_paths"}


def load(directory, tree):
    path = pathlib.Path(directory) / f"{tree}.json"
    if not path.exists():
        path = path.with_suffix(".json.gz")
        return json.load(gzip.open(path))
    return json.load(open(path))


def release_key(r):
    return (r["root"], r["file"], r["line"], r["function"], r["what"])


def site_key(s):
    return (s["file"], s["line"], s["function"], s.get("callee"), s.get("kind"))


def strip(entry):
    return {k: v for k, v in entry.items() if k not in LIFECYCLE_KEYS}


def summary(doc):
    lc = doc["lifecycle"]
    calls = lc["calls"]
    refused = [c for c in calls if c["violations"] or c.get("not_established")]
    per_site = Counter(f'{c["callee"]} {c["file"]}:{c["line"]}' for c in refused)
    per_clause = Counter(v.split("(")[-1].rstrip(")") for c in refused for v in c["violations"])
    roots = {r: v["result"] for r, v in lc["conformance"].items()}
    pause = {}
    for r in doc["releases"]:
        if "pause_collectors" not in r["root"]:
            continue
        pause[f'{r["root"].split("::")[-1]} {r["what"]} {r["file"]}:{r["line"]}'] = {
            "result": r["result"],
            "lifecycle_paths": {k: v for k, v in r.get("lifecycle_paths", {}).items()
                                if k in ("conforming", "undetermined", "refused", "conforming_result", "conforming_premises",
                                         "conforming_live", "conforming_descendants")},
            "stops": r.get("lifecycle_stops"),
        }
    return {
        "tally": doc["tally"],
        "clauses": [(s["clause"], s["function"], s["result"]) for s in lc["status"]],
        "outside_contract": [
            {"forbidden_by": o["forbidden_by"], "every_path_joins": o["every_path_joins"],
             "paths": [p["joins"] for p in o["paths"]]}
            for o in lc.get("outside_contract", [])],
        "calls": len(calls),
        "refused_readings": len(refused),
        "violating_readings": sum(1 for c in calls if c["violations"]),
        "not_established_readings": sum(1 for c in calls if c.get("not_established")),
        "undetermined_readings": sum(1 for c in calls if c["undetermined"]),
        "refused_by_site": dict(sorted(per_site.items())),
        "violations_by_clause": dict(per_clause),
        "roots": dict(Counter(roots.values())),
        "states": dict(Counter(f'{c["callee"]} {sorted(c["states"])}' for c in calls)),
        "pause_collectors_releases": pause,
    }


def moves(before, after):
    rb = {release_key(r): r for r in before["releases"]}
    ra = {release_key(r): r for r in after["releases"]}
    moved, reason_changed, other = [], 0, []
    for key in sorted(set(rb) | set(ra)):
        b, a = rb.get(key), ra.get(key)
        if b is None or a is None:
            other.append({"release": key, "missing": "before" if b is None else "after"})
            continue
        if b["result"]["verdict"] != a["result"]["verdict"]:
            moved.append({"release": key, "before": b["result"], "after": a["result"]})
        elif b["result"] != a["result"]:
            reason_changed += 1
        if {k: v for k, v in strip(b).items() if k != "result"} != \
                {k: v for k, v in strip(a).items() if k != "result"}:
            other.append({"release": key, "keys": sorted(
                k for k in set(strip(a)) | set(strip(b)) if k != "result" and strip(a).get(k) != strip(b).get(k))})
    sb = {site_key(s): s for s in before["sites"]}
    sa = {site_key(s): s for s in after["sites"]}
    site_moves = [k for k in set(sb) | set(sa)
                  if (sb.get(k) or {}).get("result", {}).get("verdict")
                  != (sa.get(k) or {}).get("result", {}).get("verdict")]
    site_reason = sum(1 for k in set(sb) & set(sa) if sb[k] != sa[k])
    top = sorted(k for k in set(before) | set(after)
                 if k not in ("releases", "sites", "lifecycle", "tally") and before.get(k) != after.get(k))
    return {
        "release_verdict_moves": moved,
        "release_reason_changes": reason_changed,
        "release_other_key_changes": other,
        "site_verdict_moves": site_moves,
        "site_changes_any": site_reason,
        "tally_equal": before["tally"] == after["tally"],
        "other_top_level_keys_changed": top,
    }


def main():
    run = sys.argv[1]
    before = sys.argv[2] if len(sys.argv) > 2 else None
    out = {}
    for tree in TREES:
        doc = load(run, tree)
        out[tree] = summary(doc)
        if before:
            out[tree]["against_before"] = moves(load(before, tree), doc)
    json.dump(out, sys.stdout, indent=1, sort_keys=True, default=str)
    print()


if __name__ == "__main__":
    main()

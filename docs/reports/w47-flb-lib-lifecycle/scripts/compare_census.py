#!/usr/bin/env python3
"""compare_census.py BEFORE.json AFTER.json -- compare a condvar/join census
run under the lifecycle obligations (card W47-FLB-LIB-LIFECYCLE) with the
run before them: with the added `lifecycle` section and each release's
`lifecycle_violations` removed, is every other byte the same? Lists every
release or site whose verdict or reason moved, and the lifecycle summary:
clause results per body, calls, violations, undetermined calls and each
root's conformance. Read-only; prints JSON.
"""
import json
import sys


def strip(doc):
    doc = dict(doc)
    doc.pop("lifecycle", None)
    doc["releases"] = [
        {k: v for k, v in r.items() if k not in ("lifecycle_violations", "lifecycle_stops")} for r in doc["releases"]
    ]
    return doc


def key(entry):
    return (entry.get("root"), entry["file"], entry["line"], entry["function"],
            entry.get("what") or entry.get("callee"))


def main():
    before = json.load(open(sys.argv[1]))
    after = json.load(open(sys.argv[2]))
    stripped = strip(after)
    moved = []
    for kind in ("releases", "sites"):
        old = {key(e): e["result"] for e in before[kind]}
        new = {key(e): e["result"] for e in stripped[kind]}
        for k in sorted(set(old) | set(new), key=str):
            if old.get(k) != new.get(k):
                moved.append({"kind": kind, "at": list(k), "before": old.get(k), "after": new.get(k)})
    lifecycle = after.get("lifecycle", {})
    violating = [r for r in after["releases"] if r.get("lifecycle_violations")]
    summary = {
        "identical_without_lifecycle": stripped == before,
        "tally": [before["tally"], after["tally"]],
        "moved": moved,
        "releases_with_violations": [
            {"root": r["root"], "at": f"{r['file']}:{r['line']}", "what": r["what"],
             "violations": r["lifecycle_violations"]} for r in violating
        ],
        "clause_status": lifecycle.get("status"),
        "calls": len(lifecycle.get("calls", [])),
        "call_violations": [c for c in lifecycle.get("calls", []) if c["violations"]],
        "undetermined_calls": sorted({f"{c['file']}:{c['line']} {c['callee']} ({c['caller']})"
                                      for c in lifecycle.get("calls", []) if c["undetermined"]}),
        "conformance": {
            result: sorted(root for root, v in lifecycle.get("conformance", {}).items()
                           if v["result"] == result)
            for result in ("conforms", "conforms where derived", "violates")
        },
    }
    json.dump(summary, sys.stdout, indent=1, sort_keys=True)
    print()


if __name__ == "__main__":
    main()

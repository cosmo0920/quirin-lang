#!/usr/bin/env python3
"""SUP03-BOUNDS-TALLY-ON-INTEGRATED-MAIN: a summary of one bounds-selected
arbiter run, read only from the run's own files.

usage: summarize.py NAME OUT.json

Reads private/sup03/NAME-bounds.jsonl (rows) and NAME-bounds.summary.json (the
run's own tally). Writes: the run's tally and per-function reason codes as the
run printed them (copied, not recomputed), and from the rows: the
custody x bounds cross table, the reason codes of the bounds-Unknown rows only
(a function counts once per code; a function may carry several codes), the
number of Unknown rows by their exact code set, obligations by kind and
outcome, the bounds Unsafe rows, and the custody-Unknown x bounds-Safe
functions. Checks that the row tally equals the run's own tally."""
import collections
import json
import pathlib
import sys

REPO = pathlib.Path(__file__).resolve().parents[4]
OUT = REPO / "private/sup03"


def code(reason):
    return (reason or "").split(":")[0]


def main():
    name, target = sys.argv[1], sys.argv[2]
    rows = [json.loads(line) for line in (OUT / f"{name}-bounds.jsonl").read_text().splitlines()]
    own = json.loads((OUT / f"{name}-bounds.summary.json").read_text())
    cross = collections.Counter()
    tally = {k: collections.Counter() for k in ("bounds", "custody", "combined")}
    unknown_codes = collections.Counter()
    unknown_code_sets = collections.Counter()
    obligations = collections.Counter()
    unsafe = []
    custody_unknown_bounds_safe = []
    for row in rows:
        b = row.get("bounds") or {}
        bv = b.get("verdict") or "absent"
        cv = row.get("custody") or "absent"
        tally["bounds"][bv] += 1
        tally["custody"][cv] += 1
        tally["combined"][row.get("combined") or "absent"] += 1
        cross[f"{cv} x {bv}"] += 1
        codes = sorted({code(r) for r in b.get("reasons") or []})
        if bv == "Unknown":
            for c in codes:
                unknown_codes[c] += 1
            unknown_code_sets[" + ".join(codes) or "(none)"] += 1
        if bv == "Unsafe":
            unsafe.append({"file": row.get("file"), "function": row.get("function")})
        if bv == "Safe" and cv != "Safe":
            custody_unknown_bounds_safe.append(
                {"file": row.get("file"), "function": row.get("function"), "custody": cv})
        for access in b.get("accesses") or []:
            outcome = code(access.get("actual")) or "?"
            obligations[f"{access.get('kind')} / {outcome}"] += 1
    rows_tally = {k: dict(sorted(v.items())) for k, v in tally.items()}
    assert rows_tally == {k: dict(sorted(v.items())) for k, v in own["tally"].items()}, (rows_tally, own["tally"])
    summary = {
        "rows": len(rows),
        "run_summary": own,
        "custody_x_bounds": dict(sorted(cross.items())),
        "unknown_rows_by_reason_code": dict(unknown_codes.most_common()),
        "unknown_rows_by_code_set": dict(unknown_code_sets.most_common()),
        "obligations_by_kind_and_outcome": dict(sorted(obligations.items())),
        "bounds_unsafe": unsafe,
        "custody_not_safe_x_bounds_safe": sorted(custody_unknown_bounds_safe, key=lambda r: (r["file"], r["function"])),
    }
    pathlib.Path(target).write_text(json.dumps(summary, indent=1, sort_keys=True) + "\n")
    print(json.dumps({k: summary[k] for k in ("rows", "custody_x_bounds", "unknown_rows_by_reason_code")}, indent=1))
    print("run tally:", json.dumps(own["tally"]))


if __name__ == "__main__":
    main()

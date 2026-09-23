#!/usr/bin/env python3
"""Attribute every per-row change between the W42 and W43 lifecycle results.

Card W43-CHUNK-SOURCE-PROFILE. Both inputs are Rust-decided published results;
this script decides nothing. It pairs rows by (library, file, function),
refuses any row present in one result and not the other, and reports each
row's role, status, outstanding legs, reached legs and derivation before and
after, plus the distribution of all rows on each side.

``--check PATH`` compares the rendering with a committed file instead of
printing it.
"""
import argparse
import json
import pathlib
import sys
from collections import Counter

ROOT = pathlib.Path(__file__).resolve().parents[4]
BEFORE = ROOT / "docs/reports/w42-chunk-lifecycle-cert/lifecycle-coverage.json"
AFTER = ROOT / "docs/reports/w43-chunk-source-profile/lifecycle-coverage.json"


def key(row):
    return (row["library"], row["file"], row["function"])


def summary(row):
    model = row.get("model")
    derivation = row.get("derivation")
    return {
        "role": row["role"],
        "status": row["lifecycle"]["status"],
        "outstanding_legs": len(row["lifecycle"].get("outstanding_legs", [])),
        "composition": model["kind"] if model else None,
        "reached_legs": sorted(model.get("reached_legs", model.get("discharged", [])))
        if model else [],
        "retained_live": model["retained_live"] if model else [],
        "derived_from_own_pinned_unit": bool(derivation),
        "checked_paths": len(derivation["checked_paths"]) if derivation else 0,
        "uncovered_paths": len(derivation["uncovered_paths"]) if derivation else 0,
    }


def distribution(rows):
    counts = Counter((r["role"], r["status"], r["outstanding_legs"],
                      r["derived_from_own_pinned_unit"]) for r in rows)
    return [
        {"role": role, "status": status, "outstanding_legs": legs,
         "derived_from_own_pinned_unit": derived, "functions": count}
        for (role, status, legs, derived), count in sorted(counts.items())
    ]


def render(before_path, after_path):
    before = json.loads(before_path.read_text())
    after = json.loads(after_path.read_text())
    left = {key(row): row for row in before["functions"]}
    right = {key(row): row for row in after["functions"]}
    if set(left) != set(right) or len(left) != len(before["functions"]) or \
            len(right) != len(after["functions"]):
        raise SystemExit("the two results do not account for the same selected functions")
    rows, before_rows, after_rows = [], [], []
    for identity in sorted(left):
        was, now = summary(left[identity]), summary(right[identity])
        before_rows.append(was)
        after_rows.append(now)
        if was != now:
            rows.append({"library": identity[0], "file": identity[1], "function": identity[2],
                         "before": was, "after": now})
    result = {
        "card": "W43-CHUNK-SOURCE-PROFILE",
        "before": {"result": before_path.relative_to(ROOT).as_posix(),
                   "evidence_version": before["evidence_version"],
                   "selected_functions": before["selected_functions"],
                   "safe_functions": before["safe_functions"],
                   "unknown_functions": before["unknown_functions"],
                   "distribution": distribution(before_rows)},
        "after": {"result": after_path.relative_to(ROOT).as_posix(),
                  "evidence_version": after["evidence_version"],
                  "selected_functions": after["selected_functions"],
                  "safe_functions": after["safe_functions"],
                  "unknown_functions": after["unknown_functions"],
                  "distribution": distribution(after_rows)},
        "unchanged_rows": len(left) - len(rows),
        "changed_rows": rows,
    }
    return json.dumps(result, indent=2) + "\n"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--before", type=pathlib.Path, default=BEFORE)
    parser.add_argument("--after", type=pathlib.Path, default=AFTER)
    parser.add_argument("--check", type=pathlib.Path)
    args = parser.parse_args()
    rendered = render(args.before.resolve(), args.after.resolve())
    if args.check:
        if args.check.read_text() != rendered:
            print(f"{args.check} differs from the rendered delta", file=sys.stderr)
            return 1
        print(f"{args.check} matches the rendered delta")
        return 0
    sys.stdout.write(rendered)
    return 0


if __name__ == "__main__":
    sys.exit(main())

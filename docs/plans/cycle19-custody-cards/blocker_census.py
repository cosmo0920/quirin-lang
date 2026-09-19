"""Blocker census over the pinned Cycle-19 corpus: what actually keeps a function Unknown.

Reads the current corpus baseline's verdict-bearing artifact and answers, for every Unknown
function, which refusal reasons it shows and which reason - if any - is the *only* one it shows.
The distinction is the point: a reason's leaf count says how loud it is, its sole-blocker count
says how many functions it alone holds back, and the two rank differently. Cards sized by the
first have been returning zero unit movement.

This is the measurement `W36-COVER-HIDDEN-BLOCKER` asks for in its discharge - a cover over the
projection's reachability rather than over its emitted leaves - archived here so the figures in
this directory's card definitions have a computed source rather than a remembered one.

Usage:  python3 docs/plans/cycle19-custody-cards/blocker_census.py [BASELINE_DIR]

BASELINE_DIR defaults to the baseline named by docs/STATUS.md. The counts are of the *pinned*
corpus and move with it; re-run at every arbiter rather than carrying these numbers forward.
"""

import collections
import gzip
import json
import pathlib
import re
import sys

DEFAULT = "docs/reports/w37-corpus-contracts-4/artifacts"


def normalise(text):
    """Collapse the backticked operand so `ptr` and `buf` are one reason, not two."""
    return re.sub(r"`[^`]*`", "`_`", str(text))[:90]


def census(artifact):
    data = json.load(gzip.open(artifact))
    leaves = collections.Counter()
    functions = collections.defaultdict(set)
    sole = collections.Counter()
    pairs = collections.Counter()
    unknown = 0
    for unit in data["units"]:
        entries = unit["functions"]
        entries = entries.values() if isinstance(entries, dict) else entries
        for entry in entries:
            if entry.get("status") != "Unknown":
                continue
            unknown += 1
            reasons = []
            for diagnostic in entry.get("diagnostics") or []:
                reason = normalise(diagnostic.get("description", ""))
                leaves[reason] += 1
                functions[reason].add((unit["file"], entry["function"]))
                reasons.append(reason)
            distinct = set(reasons)
            if len(distinct) == 1:
                sole[next(iter(distinct))] += 1
            elif len(distinct) == 2:
                pairs[tuple(sorted(distinct))] += 1
    return data, unknown, leaves, functions, sole, pairs


def main():
    root = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else DEFAULT)
    data, unknown, leaves, functions, sole, pairs = census(root / "diagnostics.json.gz")
    print(f"baseline: {root}")
    print(f"corpus diagnostic counts: {data['diagnostic_counts']}")
    print(f"Unknown functions: {unknown}")
    print(f"functions with exactly one distinct blocker: {sum(sole.values())}")
    print(f"functions with exactly two: {sum(pairs.values())}\n")
    print(f"{'leaves':>8} {'functions':>10} {'sole':>6}  reason")
    for reason, count in leaves.most_common(20):
        print(f"{count:8,} {len(functions[reason]):10,} {sole.get(reason, 0):6}  {reason}")
    print("\nthe unmodeled-call family, by position (the B series' subject)")
    family = [r for r in leaves if "unmodeled call, aggregate or indirect access in" in r or "nested call" in r]
    for reason in sorted(family, key=lambda r: -leaves[r]):
        position = reason.split("in ", 1)[-1] if "in " in reason else reason
        print(f"{leaves[reason]:8,} {len(functions[reason]):10,} {sole.get(reason, 0):6}  {position}")
    print("\ndominant two-blocker pairs (a card landing only one of a pair moves neither function)")
    for (first, second), count in pairs.most_common(10):
        short = lambda r: (r.split("in ", 1)[-1] if "in " in r else r)[:46]
        print(f"{count:8}  {short(first)}  +  {short(second)}")


if __name__ == "__main__":
    main()

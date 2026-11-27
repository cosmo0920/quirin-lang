#!/usr/bin/env python3
"""Predict the reason_code distribution of the four pinned 185-unit artifacts.

Card REASON-CODES, §7. Strictly read-only: it opens the archived gzip
artifacts and applies the same rule `ReasonCategory::classify` implements
(`crates/qcore-schemas/src/reason.rs`). It writes nothing and regenerates
nothing. The artifacts themselves move at CORPUS-REPIN.

Run from the repository root:

    python3 -B docs/reports/remaining-reason-codes/predict_185.py
"""
import collections
import gzip
import json
import pathlib
import sys

REPO = pathlib.Path(__file__).resolve().parents[3]
BASE = REPO / "docs/reports/w5-cmetrics-unit-provider"

PROJECTION_PREFIX = "resource projection unresolved: "

# The subset of `REASON_RULES` reachable from these artifacts, in table order.
MARKERS = [
    ("production C parser is unavailable", "environment-unavailable"),
    ("Clang identity changed during Boolean source import", "environment-unavailable"),
    ("was not found", "incomplete-obligation-coverage"),
    ("expected one selected definition after preprocessing", "incomplete-obligation-coverage"),
    ("cannot answer a filename-sensitive construct", "incomplete-obligation-coverage"),
    ("absent from the selected typed import", "incomplete-obligation-coverage"),
    ("exceeds the import depth limit", "incomplete-obligation-coverage"),
    ("could not project body for C function", "unsupported-syntax"),
    ("production C parser rejected the translation unit", "unsupported-syntax"),
    ("outside single borrowed chunk", "unsupported-syntax"),
    ("helper signature or parameter binding mismatch", "unsupported-syntax"),
    ("unmodeled ", "unsupported-syntax"),
]


def classify(reason):
    """Mirror of `ReasonCategory::classify` over the reachable subset."""
    if reason is None:
        return None
    if reason.startswith(PROJECTION_PREFIX):
        head = reason[len(PROJECTION_PREFIX) :].split(" ")[0]
        try:
            unsupported = int(head)
        except ValueError:
            return "UNCLASSIFIED"
        return "residual-custody" if unsupported == 0 else "unsupported-syntax"
    for marker, category in MARKERS:
        if marker in reason:
            return category
    return "UNCLASSIFIED"


def main():
    unclassified = 0
    for name in ("diagnostics", "source", "chunk-contracts", "source-effects"):
        path = BASE / f"{name}.json.gz"
        if not path.exists():
            print(f"== {name}.json ==  ABSENT at {path}")
            continue
        doc = json.loads(gzip.decompress(path.read_bytes()))
        print(f"== {name}.json ==")
        for label, rows in (("units", doc.get("units")), ("results", doc.get("results"))):
            if not rows:
                continue
            histogram = collections.Counter(classify(row.get("reason")) for row in rows)
            unclassified += histogram.get("UNCLASSIFIED", 0)
            print(f"   {label}: {len(rows)}  reason_code prediction: {dict(histogram)}")
            for row in rows:
                if classify(row.get("reason")) == "UNCLASSIFIED":
                    print("   UNCLASSIFIED:", repr(row.get("reason"))[:140])
        if not doc.get("units") and not doc.get("results"):
            print("   no reason-bearing rows")
        print()
    return 1 if unclassified else 0


if __name__ == "__main__":
    sys.exit(main())

#!/usr/bin/env python3
"""Compare REASON-CODES' read-only prediction against the re-measured artifacts.

Card CORPUS-REPIN, deliverable 5. `docs/reports/remaining-reason-codes/predict_185.py`
applied `ReasonCategory::classify` to the *archived* bytes and predicted the
`reason_code` distribution the re-pin would carry. This script recomputes the
same classification over the *re-measured* bytes, reads the `reason_code` values
the artifacts actually carry, and reports both comparisons.

    python3 -B docs/reports/corpus-repin/compare_prediction.py \
        --measured docs/reports/corpus-repin/artifacts

Read-only. Exit code 1 if any row classifies to UNCLASSIFIED.
"""
import argparse
import collections
import gzip
import json
import pathlib

PROJECTION_PREFIX = "resource projection unresolved: "

# Identical to predict_185.py's table (the reachable subset of REASON_RULES).
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

# REASON-CODES README section 7, "Predicted distribution".
PREDICTED = {
    "diagnostics": {"unsupported-syntax": 136, "null": 48, "incomplete-obligation-coverage": 1},
    "source-effects": {"unsupported-syntax": 91, "null": 3, "incomplete-obligation-coverage": 2},
    "chunk-contracts": {"unsupported-syntax": 91, "null": 3, "incomplete-obligation-coverage": 2},
    "source": {},
}


def classify(reason):
    if reason is None:
        return None
    if reason.startswith(PROJECTION_PREFIX):
        head = reason[len(PROJECTION_PREFIX):].split(" ")[0]
        try:
            unsupported = int(head)
        except ValueError:
            return "UNCLASSIFIED"
        return "residual-custody" if unsupported == 0 else "unsupported-syntax"
    for marker, category in MARKERS:
        if marker in reason:
            return category
    return "UNCLASSIFIED"


def load(path):
    if path.suffix == ".gz":
        return json.loads(gzip.decompress(path.read_bytes()))
    return json.loads(path.read_text())


def label(value):
    return "null" if value is None else value


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--measured", type=pathlib.Path,
                        default=pathlib.Path("docs/reports/corpus-repin/artifacts"))
    arguments = parser.parse_args()
    unclassified = 0
    for stem in ("diagnostics", "source", "chunk-contracts", "source-effects"):
        path = arguments.measured / f"{stem}.json.gz"
        if not path.exists():
            path = arguments.measured / f"{stem}.json"
        if not path.exists():
            print(f"== {stem} ==  ABSENT")
            continue
        document = load(path)
        rows = document.get("units") or document.get("results") or []
        print(f"== {stem} ==  rows: {len(rows)}")
        if not rows:
            print("   no reason-bearing rows")
            continue
        computed = collections.Counter(label(classify(row.get("reason"))) for row in rows)
        carried = collections.Counter(label(row.get("reason_code")) for row in rows)
        unclassified += computed.get("UNCLASSIFIED", 0)
        print(f"   predicted (REASON-CODES section 7): {PREDICTED.get(stem, {})}")
        print(f"   classifier over re-measured reasons: {dict(computed)}")
        print(f"   reason_code actually carried:        {dict(carried)}")
        if dict(computed) == PREDICTED.get(stem, {}):
            print("   distribution: MATCHES the prediction")
        else:
            print("   distribution: DIVERGES from the prediction")
        if not any(key != "null" for key in carried):
            print("   carrier: this artifact carries NO reason_code field")
        elif dict(carried) == dict(computed):
            print("   carrier: present and equal to the classifier")
        else:
            print("   carrier: present but DIFFERENT from the classifier")
        for row in rows:
            if classify(row.get("reason")) == "UNCLASSIFIED":
                print("   UNCLASSIFIED:", repr(row.get("reason"))[:140])
        print()
    return 1 if unclassified else 0


if __name__ == "__main__":
    raise SystemExit(main())

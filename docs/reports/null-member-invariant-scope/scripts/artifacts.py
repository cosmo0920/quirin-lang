#!/usr/bin/env python3
"""artifacts.py -- NULL-MEMBER-INVARIANT-SCOPE: copy the report's artifacts
from the BEFORE (b0), stable-only (a1) and AFTER (a2) runs in private/nmis/
(written there by routes.sh, member_counts.py and scope_candidates.py)."""
import pathlib
import shutil

REPO = pathlib.Path(__file__).resolve().parents[4]
PRIVATE = REPO / "private/nmis"
OUT = REPO / "docs/reports/null-member-invariant-scope/artifacts"
OUT.mkdir(parents=True, exist_ok=True)
COPIES = {
    "compare-corpus-a2.json": "compare-corpus.json",
    "compare-system-a2.json": "compare-system.json",
    "compare-slice-a2.json": "compare-slice.json",
    "compare-corpus-a1.json": "compare-corpus-stable-only.json",
    "parity-corpus-a2.json": "parity-corpus.json",
    "parity-system-a2.json": "parity-system.json",
    "parity-slice-a2.json": "parity-slice.json",
    "counts-corpus-b0.json": "counts-corpus-before.json",
    "counts-corpus-a2.json": "counts-corpus-after.json",
    "a1-counts.json": "counts-corpus-stable-only.json",
    "counts-system-a2.json": "counts-system-after.json",
    "counts-slice-a2.json": "counts-slice-after.json",
    "scope-choice.json": "scope-choice.json",
    "b0-null.summary.json": "null-summary-before-corpus.json",
    "a1-null.summary.json": "null-summary-stable-only-corpus.json",
    "a2-null.summary.json": "null-summary-after-corpus.json",
    "b0-system-null.summary.json": "null-summary-before-system.json",
    "a2-system-null.summary.json": "null-summary-after-system.json",
    "b0-slice-null.summary.json": "null-summary-before-slice.json",
    "a2-slice-null.summary.json": "null-summary-after-slice.json",
}
for source, target in COPIES.items():
    shutil.copyfile(PRIVATE / source, OUT / target)
    print(target, (OUT / target).stat().st_size)

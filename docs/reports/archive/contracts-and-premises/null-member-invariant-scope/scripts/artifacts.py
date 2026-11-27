#!/usr/bin/env python3
"""artifacts.py -- NULL-MEMBER-INVARIANT-SCOPE: copy the report's artifacts
from the BEFORE (b0), stable-only (a1) and AFTER (a2) runs in private/nmis/
(written there by routes.sh, member_counts.py, scope_candidates.py and
rename_identity.py); AFTER is a3, after the `nonnull_` rename."""
import pathlib
import shutil

REPO = pathlib.Path(__file__).resolve().parents[4]
PRIVATE = REPO / "private/nmis"
OUT = REPO / "docs/reports/null-member-invariant-scope/artifacts"
OUT.mkdir(parents=True, exist_ok=True)
COPIES = {
    "compare-corpus-a3.json": "compare-corpus.json",
    "compare-system-a3.json": "compare-system.json",
    "compare-slice-a3.json": "compare-slice.json",
    "compare-corpus-a1.json": "compare-corpus-stable-only.json",
    "parity-corpus-a3.json": "parity-corpus.json",
    "parity-system-a3.json": "parity-system.json",
    "parity-slice-a3.json": "parity-slice.json",
    "counts-corpus-b0.json": "counts-corpus-before.json",
    "counts-corpus-a3.json": "counts-corpus-after.json",
    "a1-counts.json": "counts-corpus-stable-only.json",
    "counts-system-a3.json": "counts-system-after.json",
    "counts-slice-a3.json": "counts-slice-after.json",
    "scope-choice.json": "scope-choice.json",
    "rename-identity.json": "rename-identity-corpus.json",
    "rename-identity-system.json": "rename-identity-system.json",
    "rename-identity-slice.json": "rename-identity-slice.json",
    "b0-null.summary.json": "null-summary-before-corpus.json",
    "a1-null.summary.json": "null-summary-stable-only-corpus.json",
    "a3-null.summary.json": "null-summary-after-corpus.json",
    "b0-system-null.summary.json": "null-summary-before-system.json",
    "a3-system-null.summary.json": "null-summary-after-system.json",
    "b0-slice-null.summary.json": "null-summary-before-slice.json",
    "a3-slice-null.summary.json": "null-summary-after-slice.json",
}
for source, target in COPIES.items():
    shutil.copyfile(PRIVATE / source, OUT / target)
    print(target, (OUT / target).stat().st_size)

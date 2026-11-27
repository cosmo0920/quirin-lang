#!/usr/bin/env python3
"""increment.py -- NULL-TRIAGE-UNRESOLVED-PATHS: prepend the card's increment
paragraph to the current wave-48 record (idempotent)."""
import pathlib

REPO = pathlib.Path(__file__).resolve().parents[4]
RECORD = REPO / "docs/development/increments/wave-48-part6.md"
ANCHOR = "**`NULL-CANDIDATE-TRIAGE` (2026-10-10"
PARAGRAPH = (
    "**`NULL-TRIAGE-UNRESOLVED-PATHS` (2026-10-11, base `c77efedbe`, rebased onto `d7b0882ac`; prediction `aa4e15a84`).** "
    "The null path search's unresolved candidates read by sound extensions only "
    "([report](../../reports/null-triage-unresolved-paths/README.md); gateway rules "
    "[15q](../../agents/gateway.md#constraint-15q)–[15u](../../agents/gateway.md#constraint-15u), ids taken "
    "past the router's `15a`–`15p`; Lean `QCore.NullTriagePaths`, 15 theorems). Priority 0: a call's integer "
    "result is a bound (its C type, a declared `nonnull_result_range`, named so by the user's decision of "
    "2026-10-11, or the unit's validated summary of the callee), never a free value, so `copy_label_values`' "
    "path through `cfl_list_size() < 0` and seven other findings resting on a callee's result are candidates "
    "again (named corrections); `cfl_list_size`'s range is pinned at both revisions and validated by 45 "
    "bodies. A two-phase search keeps rule 14r's exact reading for findings and explores every inexact "
    "condition only to prove infeasibility; exact offsets, sums, differences and enumeration ranges are "
    "followed (four new `null-on-normal-path` findings in `rd_kafka_toppar_op_serve`), and two widenings of "
    "the old search are corrected (a two-value comparison taken whatever the ranges; a cached `p->n` kept "
    "across `p = q`). Corpus: findings 33 → 29, provisional Unsafe rows 22 → 18, 9 infeasible candidates "
    "removed (2 rows Unknown → Safe), unresolved 104 → 101 (57 at a call's result, registered as "
    "`NULL-TRIAGE-CALL-RESULT-ACHIEVABILITY`; 30 correlated, left in the amended entry); custody "
    "byte-identical on all three routes, parity multiset-identical, the same triage before and after the rebase; corpus wall +10.6 %.\n\n"
)
text = RECORD.read_text()
if PARAGRAPH not in text:
    assert text.count(ANCHOR) == 1
    RECORD.write_text(text.replace(ANCHOR, PARAGRAPH + ANCHOR, 1))
print("ok")

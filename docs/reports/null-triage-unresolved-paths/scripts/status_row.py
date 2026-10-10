#!/usr/bin/env python3
"""status_row.py -- NULL-TRIAGE-UNRESOLVED-PATHS: restate STATUS's null row
from this card's AFTER run (a3 on the rebased tip). Idempotent: each edit
applies only while its old text is present."""
import pathlib

STATUS = pathlib.Path(__file__).resolve().parents[4] / "docs/STATUS.md"
EDITS = [
    ("Null-dereference property (separate from custody; rules 14k–14z, C09.3)",
     "Null-dereference property (separate from custody; rules 14k–14z, 15q–15u, C09.3)"),
    ("**Corpus 4,921 rows: 1,113 Safe unconditional, 1,433 Safe conditional on their own declared or "
     "inferred preconditions, 0 Pending, 2,353 Unknown, 22 provisional Unsafe** (NULL-CANDIDATE-TRIAGE: "
     "**33 findings**, each a path validated from entry to a NULL dereference under a named condition, 30 "
     "`null-on-allocation-failure` and 3 `null-on-normal-path`, published with path and excerpts and never "
     "an upstream defect; read by hand 32 real-looking, 1 resting on a callee result the callee never "
     "returns); **12,648 candidates**:",
     "**Corpus 4,921 rows: 1,113 Safe unconditional, 1,435 Safe conditional on their own declared or "
     "inferred preconditions, 0 Pending, 2,355 Unknown, 18 provisional Unsafe** (NULL-TRIAGE-UNRESOLVED-PATHS: "
     "**29 findings**, each an exact path validated from entry to a NULL dereference under a named condition, "
     "24 `null-on-allocation-failure` and 5 `null-on-normal-path`, published with path and excerpts and never "
     "an upstream defect; a call's integer result is a bound, never a free value, so the `copy_label_values` "
     "false finding and seven others resting on a callee's result are candidates again, named corrections; "
     "9 candidates proved infeasible by exhausting every path are removed); **12,643 candidates** (the "
     "breakdown that follows is NULL-MEMBER-INVARIANT-SCOPE's 12,648, before the 9 removals and the 8 + 4 "
     "finding moves):"),
    ("Of the 139 strong-source candidates: 33 findings, 2 infeasible, 104 unresolved.",
     "Of the 139 strong-source candidates: 29 findings, 9 infeasible (removed), 101 unresolved (57 stop at "
     "a call's result, 30 at a correlated condition)."),
    ("20 `quant null-deref` declarations, five member and one result declaration (bounds contract v1, "
     "pinned at both revisions).",
     "20 `quant null-deref` declarations, five member, one result and one result-range declaration "
     "(`nonnull_result_range`, `cfl_list_size`, validated by 45 bodies; bounds contract v1, pinned at both "
     "revisions)."),
    ("the triage adds +0.6 % ([report](reports/null-candidate-triage/README.md);",
     "the triage adds +0.6 %, its two-phase search (NULL-TRIAGE-UNRESOLVED-PATHS) a further +10.6 % "
     "([report](reports/null-triage-unresolved-paths/README.md); [triage](reports/null-candidate-triage/README.md);"),
]
text = STATUS.read_text()
for old, new in EDITS:
    if new in text:
        continue
    assert text.count(old) == 1, old[:70]
    text = text.replace(old, new)
STATUS.write_text(text)
print("ok")

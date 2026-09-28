#!/usr/bin/env python3
"""Record the user's 2026-09-28 decision on W47-ARENA-INTERIOR-READING (idempotent).

Usage: record_arena_decision.py   (then docs/reports/remaining-d02/build_matrix.py .)

* amends W47-ARENA-INTERIOR-READING: the interior reading is accepted as the interim
  declaration; the entry stays open until the pool/arena mode replaces it;
* registers W47-ARENA-POOL-RESULT-MODE (the future treatment of an arena as an
  allocation pool).
"""
import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[4]
MATRIX = ROOT / "docs/reviews/r0_3/acceptance_matrix.json"
MARK = "USER DECISION (2026-09-28):"
AMEND = (
    " " + MARK + " the user accepted the interior reading as reasonable for now, and stated that "
    "the arena should in future be treated as a memory allocation pool; how to contract that is "
    "open. The entry stays open as the interim declaration and is discharged by "
    "W47-ARENA-POOL-RESULT-MODE."
)
NEW = {
    "id": "W47-ARENA-POOL-RESULT-MODE",
    "title": "An arena is an allocation pool: its results should be invalidated by the pool's destroy and reset",
    "why": (
        "User decision 2026-09-28 on W47-ARENA-INTERIOR-READING: cfl_arena_calloc's "
        "resource_return_interior(arena, nullable) declaration (quirin-provider-cfl "
        "cfl/sds_lifetime.qv) is accepted as the interim reading, and the arena should in future be "
        "treated as a memory allocation pool. Today the interior mode ties each result to the arena "
        "argument's release, but cfl_arena_destroy is declared only in cfl/arena_lifetime.qv, which "
        "is not a v1 resource-view module, and cfl_arena_reset (which invalidates every result "
        "without releasing the arena) is undeclared; a caller of either is refused (Unknown, "
        "fail-closed), so no result is shown invalid after destroy or reset. DISCHARGE: a pool "
        "resource kind (or result mode) in the contract spec (C04) whose allocations depend on the "
        "pool, are invalidated by the pool's destroy and by reset, are never released individually "
        "and whose nullness is their own; cfl_arena_create/destroy/reset declared in that kind in "
        "quirin-provider-cfl, checked against their bodies and pinned in revision mode; gateway "
        "rule, matching audited Lean, negative controls (use after destroy, use after reset, free "
        "of a pool result, a pool result outliving its pool through a store), and a measured "
        "corpus/system re-pin; W47-ARENA-INTERIOR-READING retired when this lands."
    ),
    "source": "docs/reports/w47-callee-closure-r2/README.md",
    "recorded_by": "INTEGRATOR (user decision on W47-CALLEE-CLOSURE-R2)",
}

matrix = json.loads(MATRIX.read_text())
entries = matrix["named_followups"]
ids = {entry["id"]: entry for entry in entries}
before = len(entries)
entry = ids["W47-ARENA-INTERIOR-READING"]
if MARK not in entry["why"]:
    entry["why"] += AMEND
if NEW["id"] not in ids:
    entries.append(NEW)
MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
print(f"follow-ups {before} -> {len(entries)}")

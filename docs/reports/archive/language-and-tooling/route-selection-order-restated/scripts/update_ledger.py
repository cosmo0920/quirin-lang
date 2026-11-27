#!/usr/bin/env python3
"""ROUTE-SELECTION-ORDER-RESTATED: retire its own entry (both DISCHARGE
branches hold) and register the residual it found, in the acceptance matrix
(indent=1, ensure_ascii=False; `dimensions.named_followups` = list length).
Idempotent."""
import json
import pathlib

REPO = pathlib.Path(__file__).resolve().parents[4]
MATRIX = REPO / "docs/reviews/r0_3/acceptance_matrix.json"
REPORT = "docs/reports/route-selection-order-restated/README.md"
CARD = "ROUTE-SELECTION-ORDER-RESTATED"
RETIRE = CARD
REGISTER = [
    {
        "id": "ROUTE-TABLE-CLOSURE-GENERAL-RULE-UNDRIVEN",
        "title": "The route table's general closure refusal (every row after the two "
        "scoped-task rows) is reached by no fixture, and its unreachability rests on an "
        "untested parser property",
        "why": (
            "Recorded by " + CARD + " (base 1a399e6a1). The closure refusal of "
            "compile_elaborated_source now reads the selected row's closure rule "
            "(crates/qcore-verification/src/driver/source_routes.rs, SOURCE_ROUTES): rows 0-4 "
            "carry the FFI, field-footprint, epoch (two rows) and mutex refusals, rows 5-6 (the "
            "composed-task and scoped-task routes) host closures, and rows 7-21 carry the general "
            "refusal `a closure is a spawn target of the scoped-task routes ...` (Q600), which was "
            "the closure if-chain's `else` arm on the base. The FFI and field-footprint rules are "
            "now driven (compiler/tests/route_selection_table.rs), the epoch and mutex rules by "
            "compiler/tests/closures.rs; the general rule is driven by nothing. By reading it looks "
            "unreachable: the parser lifts a closure only at a `spawn SCOPE |...|` head "
            "(frontend_tasks.rs find_closures / is_closure_head), and source_tasks::has_source_tasks "
            "holds for every module with a task expression or a `thread scope`, so row 6 or an "
            "earlier row selects a module with a closure whenever its spawn is a recorded task "
            "expression or stands in a `thread scope`. Nothing tests that every lifted closure is "
            "(stability-review item 1: a stated boundary, not a claim). Fail-closed either way. DISCHARGE: a test or Lean argument that every "
            "parsed closure makes has_source_tasks hold (then the rule is unreachable by "
            "construction and can be stated so), or a fixture that reaches it."
        ),
        "source": REPORT,
        "recorded_by": CARD,
    },
]


def main():
    matrix = json.loads(MATRIX.read_text(encoding="utf-8"))
    followups = matrix["named_followups"]
    followups[:] = [f for f in followups if f.get("id") != RETIRE]
    by_id = {f["id"]: f for f in followups}
    for entry in REGISTER:
        if entry["id"] in by_id:
            by_id[entry["id"]].update(entry)
        else:
            followups.append(entry)
    matrix["dimensions"]["named_followups"] = len(followups)
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
    print(len(followups))


if __name__ == "__main__":
    main()

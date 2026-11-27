#!/usr/bin/env python3
"""update_ledger.py -- card CG09-TYPED-SOURCE-CONTROL: the ledger edits.

Retires `CG09-TYPED-SOURCE-CONTROL`, amends `CG01-T-CONSUMER-ADMISSIONS` and
`CG27-LOOPS-RECURSION-BUDGETS`, registers `CG09-CONTROL-SHAPE-RESIDUALS`, and
keeps `dimensions.named_followups` equal to the list length. Idempotent;
writes with indent=1 and ensure_ascii=False, as the hand-maintained sections
require.
"""
import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[4]
MATRIX = ROOT / "docs/reviews/r0_3/acceptance_matrix.json"
CARD = "CG09-TYPED-SOURCE-CONTROL"
REPORT = "docs/reports/cg09-typed-source-control/README.md"

CG01_AMENDMENT = (
    " AMENDED by CG09-TYPED-SOURCE-CONTROL (gateway rule 11l): route T also reads the"
    " control statements, and admits an acyclic form (`switch` with fallthrough, `break`,"
    " `continue` out of `do … while (0)`, forward `goto` and labels) only through the"
    " importer's second admission, `ControlAdmission`. The same 19 LEGACY import sites"
    " refuse each of the four by name (`typed-source control `…` is not admitted by the"
    " legacy typed-source consumer`); only `import_monkey_wrappers` lists them"
    " (`MONKEY_WRAPPER_CONTROL`). An admission census per site now covers the 20 CG01"
    " nodes, CG02's value forms and these 4 forms. A lowered form emits only existing"
    " operations (guards `selector == constant`, `LocalEnd`), but no reader was read for"
    " the path shapes it adds: several not-taken guards on one path, and fallthrough"
    " paths that repeat a later case's operations (so a release consumer would see a"
    " second release there). Size: 0 functions. Every Monkey wrapper and callback the"
    " corpus arbiter imports through T imports at base, so none holds a control form"
    " outside T's legacy shapes."
)

CG27_AMENDMENT = (
    " AMENDED by CG09-TYPED-SOURCE-CONTROL (gateway rule 11l): route T now reads `while`,"
    " a `for` that is not a canonical scan, a `do` whose condition is not the literal"
    " zero, and a `goto` to a label already read. It refuses each by name, in every"
    " consumer, as a back-edge that carries no invariant or summary."
    " `ControlAdmission::validate` refuses an admission row for a back-edge form, and"
    " `QCore.TypedSourceControl.back_edge_refused` proves the refusal. A route-T loop"
    " invariant or summary lands here: an admission class the importer checks, with"
    " the projection of the loop's iteration."
)

RESIDUAL = {
    "id": "CG09-CONTROL-SHAPE-RESIDUALS",
    "title": "Route T control shapes CG09 refuses by name",
    "why": (
        "Found by CG09-TYPED-SOURCE-CONTROL (gateway rule 11l(5)). Route T reads"
        " `switch`, `break`, `continue`, forward `goto` and labels, but it refuses these"
        " shapes by name, in every consumer:"
        " (1) a `goto` into a scope it does not enclose (an `if` arm, a loop or a"
        " `switch` body);"
        " (2) a `goto` that skips a declaration into its own scope;"
        " (3) a declaration or an ordinary label at a `switch` body's top level;"
        " (4) a case label below that top level (Duff's device);"
        " (5) a GNU case range;"
        " (6) a selector that is not a promoted integer;"
        " (7) a switch with an initializer or condition variable."
        " An indirect `goto` keeps the generic refusal. Each shape is fail-closed; none"
        " is a precision loss on a pinned function. Size: 0 functions. No Monkey wrapper"
        " or callback the arbiter imports holds one, since all import at base."
        " DISCHARGE: a shape admitted with its scope and lifetime premise stated, a"
        " refusal control per remaining shape, and the Lean model"
        " (`QCore.TypedSourceControl`) extended with it."
    ),
    "source": REPORT,
    "recorded_by": CARD,
}


def main() -> int:
    matrix = json.loads(MATRIX.read_text())
    entries = matrix["named_followups"]
    entries[:] = [entry for entry in entries if entry["id"] != CARD]
    for entry in entries:
        if entry["id"] == "CG01-T-CONSUMER-ADMISSIONS" and "CG09-TYPED" not in entry["why"]:
            entry["why"] += CG01_AMENDMENT
        if entry["id"] == "CG27-LOOPS-RECURSION-BUDGETS" and "CG09-TYPED" not in entry["why"]:
            entry["why"] += CG27_AMENDMENT
    if not any(entry["id"] == RESIDUAL["id"] for entry in entries):
        entries.append(RESIDUAL)
    matrix["dimensions"]["named_followups"] = len(entries)
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
    print(len(entries))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

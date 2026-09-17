#!/usr/bin/env python3
"""Append card C-GATEWAY-RETURN-CALL-W4's four residuals to the acceptance matrix.

The wave-34 close found these UNAPPLIED. The arbiter's §11 item 5 asked for them
by name; DOCS-COMPACT-6 merged afterwards and moved `named_followups` 74 -> 80
without them (its six additions are platform and constraint names), so the four
reached no tracker. `scripts/reconciliation_sweep.py --residuals` names all four
independently, which is the same defect's other half.

`named_followups` is `hand_maintained` in the generator's `section_ownership`
(`docs/reports/remaining-d02/build_matrix.py` carries it verbatim from the
committed matrix), so this appends rather than regenerates. It then sets
`dimensions.named_followups` from the list's own length and rewrites with the
generator's byte format - `indent=1`, `ensure_ascii=False`, trailing newline
(card HYGIENE-16) - so the diff stays reviewable.

Run the generator afterwards to confirm the append is a fixed point:

    python3 -B docs/reports/remaining-d02/build_matrix.py . \
        --gate docs/reports/wave33-close/gate.json

The wave-33 gate is deliberate: it is the gate the committed matrix already
carries, so this change moves `named_followups` and its derived count and
nothing else. The wave-34 close could not measure a gate of its own (see
`docs/reports/wave34-close/gate.json`), and substituting an unmeasured figure
would put a number in the matrix that no command produced.

Idempotent: an id already present is skipped.
"""

import json
import pathlib

MATRIX = pathlib.Path("docs/reviews/r0_3/acceptance_matrix.json")
REPORT = "docs/reports/c-gateway-return-call-w4/README.md"
CARD = "C-GATEWAY-RETURN-CALL-W4"


def entry(id_, title, why):
    return {"id": id_, "title": title, "why": why, "source": REPORT, "recorded_by": CARD}


NEW = [
    entry(
        "RETURN-CALL-W4-REACH-BLOCKED",
        "The W4 route yields 1 of 14 leaves, and no larger return-position card is available on it",
        "Card C-GATEWAY-RETURN-CALL-W4 (wave 34) was cut on a brief claiming the W4 custody-summary route would "
        "decide the bulk of the `return CALL(...)` refusals; its report REFUTES that brief and says so first. "
        "Measured over the 643 `return CALL(...)` refusal leaves of the pinned corpus, on one population so the "
        "shares are comparable: 379 (58.9%) have a callee defined in the same `.c`, but constraint 4's second "
        "route publishes a summary only after `observations_match_contract` re-runs the analyzer and finds the "
        "callee's own body FULLY MODELED AND DEFECT-FREE, and that holds for 4 (0.6%). The other 375 are 366 "
        "`Unknown` bodies and 9 `Unsafe`; 264 (41.1%) are external. Of the 14 leaves this card actually withdrew, "
        "13 were decided by a declared `.qcore` contract and exactly 1 by the W4 route "
        "(`destroy_data_point -> destroy_summary_data_point`). 'Defined in this translation unit' is not "
        "'W4-derivable', and a reach argument that counts definedness overstates this route by two orders of "
        "magnitude. DISCHARGE: a measured statement of which refusal classes dominate those 366 callees' own "
        "bodies - the real next question, deliberately not answered by that card.",
    ),
    entry(
        "RETURN-CALL-SYNTHETIC-PLACE-PRESENTED",
        "Ten published leaves name a reserved place that appears in no source",
        "Card C-GATEWAY-RETURN-CALL-W4 (wave 34) publishes 10 corpus leaves naming `__qcore_returned_value`, the "
        "synthetic place its return-position desugaring binds. The rows are true and `Unknown`-class, so this is "
        "PRESENTATION, not soundness: no verdict moves and no pin depends on the spelling. A reader of "
        "`source-effects.json` nonetheless sees a place that exists in no translation unit. DISCHARGE: render the "
        "reserved place as a phrase ('the value this call returned') wherever a diagnostic carries a place. "
        "Deliberately not taken in that card because it touches every diagnostic that carries a place, which is a "
        "presentation change across the whole gateway rather than a return-position one.",
    ),
    entry(
        "RETURN-CALL-NULLABLE-WRAPPER",
        "Five allocator wrappers are accused by a non-null contract for legitimately returning NULL",
        "Card C-GATEWAY-RETURN-CALL-W4 (wave 34) §5.3.1: five functions outside the 478 gain a new leaf reading "
        "`nullable resource ... used by a non-null contract`. They are allocator wrappers that legitimately return "
        "NULL on failure; the complaint arises from analysing them WITHOUT their own derived contract in force, so "
        "the caller-side non-null contract meets a nullable result. `Unknown`-class and in the accusing direction, "
        "so invariant 1 is not engaged and nothing is promoted. DISCHARGE: measure whether the derivation can infer "
        "`Owned { nullable: true }` for such a wrapper, AND whether doing so is a promotion this project wants - "
        "the second half is a disposition, not a measurement, because inferring nullability for a wrapper moves "
        "verdicts in the discharging direction.",
    ),
    entry(
        "RETURN-CALL-INCOMPLETE-LINE-455",
        "455 of 1,470 return leaves sit on a line where the return is not complete, and are classed shape-unknown",
        "Card C-GATEWAY-RETURN-CALL-W4 (wave 34) §3.2: 455 of 1,470 return leaves (31%) have an `origin` line on "
        "which the `return` statement is not complete - the classifier reads one preprocessed line, and a return "
        "spanning more than one cannot be shaped from it. Those leaves are classified SHAPE-UNKNOWN rather than "
        "guessed, which is why the card's own census makes no shape claim about them (the "
        "`ctr_span_set_trace_id_with_cid` exemplar falls in this bucket and is recorded as such rather than "
        "smoothed over). Fail-closed and correct, but it bounds every shape census this route publishes. "
        "DISCHARGE: a multi-line-aware classifier, or a measurement showing the class is uninteresting.",
    ),
]


def main():
    matrix = json.loads(MATRIX.read_text())
    followups = matrix["named_followups"]
    present = {e["id"] for e in followups if isinstance(e, dict)}

    added = []
    for e in NEW:
        if e["id"] in present:
            print(f"  skip (already present): {e['id']}")
            continue
        followups.append(e)
        added.append(e["id"])
        print(f"  +  {e['id']}")

    before = matrix["dimensions"]["named_followups"]
    matrix["dimensions"]["named_followups"] = len(followups)
    print(f"\ndimensions.named_followups {before} -> {len(followups)}")

    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
    print(f"wrote {MATRIX} ({len(added)} appended)")


if __name__ == "__main__":
    main()

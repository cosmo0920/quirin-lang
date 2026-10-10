#!/usr/bin/env python3
"""update_ledger.py -- NULL-DEREF-QTT-GROUNDING: register the card's residual
in docs/reviews/r0_3/acceptance_matrix.json `named_followups` (indent=1,
ensure_ascii=False; `dimensions.named_followups` = the list's length).
Idempotent: an entry already present is replaced, not duplicated."""
import json
import pathlib

REPO = pathlib.Path(__file__).resolve().parents[4]
MATRIX = REPO / "docs/reviews/r0_3/acceptance_matrix.json"

ENTRY = {
    "id": "NULL-GRADE-WALK-CORRESPONDENCE",
    "title": (
        "The null walk computes the grade's abstraction (`First`), not a section-L07 `Usage`; "
        "its correspondence to the grounded path model is a review premise"
    ),
    "why": (
        "Measured by NULL-DEREF-QTT-GROUNDING (USER DECISION 2026-10-11, verbatim: \"We need "
        "grounding.\"; contract specification C09.3 'Grounding', language specification L07 "
        "'Null-dereference demand', grammar G05d). QCore.NullDerefGrade proves, on its path model, "
        "that the walk's `First` abstraction is exactly the L07 grade reading of `quant null-deref "
        "#B;` (relevant_accepts_body_iff_inferred, necessary_iff_relevant, branch_join_matches, "
        "moved_question_is_sequential_addition, caller_obligation_is_grade_admission), and "
        "gradeVectors with exhaustive short-path tests (crates/qcore-core/tests/null_deref_grade.rs, "
        "5 tests) bind qcore_core::null_deref's First, inferred and clause_standing to "
        "qcore_core::usage::Usage. Not proved: that crates/qcore-c-gateway/src/null_deref/walk.rs's "
        "five `First` sites are that path model (trust boundary 28): the dereference mark in "
        "`check` (walk.rs:645), the NULL-test mark in `refine` (:687), the label widening that "
        "inserts `Other` (`widen_all`, :1774), `State::join`'s union (:159) and `exit`'s collection "
        "(:479). The walk never constructs a `Usage`, so the grade's upper bound is not computed "
        "(no admission reads it). DISCHARGE: compute each formal's entry grade with "
        "qcore_core::usage inside the walk (or a function it calls) beside `First`, and show the "
        "two agree on the corpus, system and slice with the null rows byte-identical; or record "
        "the review premise as permanent at a stability review."
    ),
    "source": "docs/reports/null-deref-qtt-grounding/README.md",
    "recorded_by": "NULL-DEREF-QTT-GROUNDING",
}


def main() -> int:
    matrix = json.loads(MATRIX.read_text())
    followups = [e for e in matrix["named_followups"] if e["id"] != ENTRY["id"]]
    followups.append(ENTRY)
    matrix["named_followups"] = followups
    matrix["dimensions"]["named_followups"] = len(followups)
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
    print(len(followups))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

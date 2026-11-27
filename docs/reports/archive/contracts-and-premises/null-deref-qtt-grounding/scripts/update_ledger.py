#!/usr/bin/env python3
"""update_ledger.py -- NULL-DEREF-QTT-GROUNDING: register the card's residuals
in docs/reviews/r0_3/acceptance_matrix.json `named_followups` (indent=1,
ensure_ascii=False; `dimensions.named_followups` = the list's length).
Idempotent: an entry already present is replaced, not duplicated."""
import json
import pathlib

REPO = pathlib.Path(__file__).resolve().parents[4]
MATRIX = REPO / "docs/reviews/r0_3/acceptance_matrix.json"
SOURCE = "docs/reports/null-deref-qtt-grounding/README.md"
CARD = "NULL-DEREF-QTT-GROUNDING"

ENTRIES = [
    {
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
        "source": SOURCE,
        "recorded_by": CARD,
    },
    {
        "id": "NULL-DEREF-FROM-INDEX-ZERO-BOUNDS",
        "title": (
            "A discharged `quant checks B, Idx #i: #i == 0` entails `quant null-deref #B` "
            "(proved), but no analysis uses it to discharge a NULL obligation"
        ),
        "why": (
            "Recorded by NULL-DEREF-QTT-GROUNDING on the user's direction of 2026-10-11 (verbatim: "
            "\"This can be derived from the proven lean. And it can be natural.\"): "
            "QCore.DerefDecomposition.checks_zero_entails_null_deref derives, from QCore.IndexExtent's "
            "element_access_sound and QCore.IndexQuant's call-side decision, that a caller whose "
            "index-0 check is discharged against a count sound for its actual passes a non-NULL value "
            "(NULL has no extent), which relevant admits and QCore.NullDeref may read non-null; the "
            "converse fails (null_deref_does_not_entail_checks_zero, an_unknown_extent_is_never_in_bounds). "
            "Out of the card's scope by the user's instruction: no consumer reads a bounds fact as a NULL "
            "proof. Size: no corpus declaration states an index-0 check today (the four quant checks on "
            "main, chunkio's file_st_bounds.qv and meta_len_bounds.qv, admit 22..23 and 10..13), and none "
            "uses quant deref. DISCHARGE: a behaviour card that lets the null property read a discharged "
            "index-0 check (or a source-established extent of at least one element) as a non-null proof, "
            "with the theorem as its basis, a Rust control binding the decision, and a measured corpus, "
            "system and slice movement attributed."
        ),
        "source": SOURCE,
        "recorded_by": CARD,
    },
    {
        "id": "QUANT-LIVE-CUSTODY-READING",
        "title": (
            "`quant live #B` reaches custody only as a checked restatement of a resource_owned "
            "position; custody never reads it, and resource_borrowed does not back it"
        ),
        "why": (
            "Recorded by NULL-DEREF-QTT-GROUNDING (USER DECISION 2026-10-11, \"Two levels\"; gateway "
            "rule 14q, contract specification C09.3, grammar G05d): the liveness part of a dereference "
            "is admitted in bounds contract v1 (with quant deref), refused by resource v1, and admitted "
            "on an audit route only where the function's resource v1 declaration in force puts the "
            "pointer at a resource_owned position (crates/qcore-contract/src/deref_parts.rs "
            "check_live_backing; quant-live-unbacked, quant-live-contradicts-custody). resource_borrowed "
            "does not back it: custody's Borrowed rule (QCore.BorrowLiveness) refuses only an identity "
            "released on every reaching path, weaker than 'not released'. Size: 0 corpus declarations "
            "use the new forms. DISCHARGE: either a custody reading of a liveness part (a must-live "
            "obligation at a Borrowed actual, with its Lean rule and a measured movement), or a decision "
            "that the checked restatement is the permanent design, recorded in C09.3."
        ),
        "source": SOURCE,
        "recorded_by": CARD,
    },
]


def main() -> int:
    matrix = json.loads(MATRIX.read_text())
    ids = {entry["id"] for entry in ENTRIES}
    followups = [e for e in matrix["named_followups"] if e["id"] not in ids]
    followups.extend(ENTRIES)
    matrix["named_followups"] = followups
    matrix["dimensions"]["named_followups"] = len(followups)
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
    print(len(followups))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

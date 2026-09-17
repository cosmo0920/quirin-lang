#!/usr/bin/env python3
"""Append card SWEEP-BLIND-SPOTS' residuals to the acceptance matrix.

`named_followups` is `hand_maintained` in the generator's `section_ownership`
(`docs/reports/remaining-d02/build_matrix.py` carries it verbatim from the
committed matrix), so this appends rather than regenerates. It then sets
`dimensions.named_followups` from the list's own length and rewrites with the
generator's byte format - `indent=1`, `ensure_ascii=False`, trailing newline
(card HYGIENE-16) - so the diff stays reviewable. Modelled on
`docs/reports/stability-reeval-1/scripts/add_followups.py`.

Run the generator afterwards to confirm the append is a fixed point, re-supplying
the gate the matrix was last built with so `validation.measurement_provenance` is
not downgraded:

    python3 -B docs/reports/remaining-d02/build_matrix.py . \
        --gate docs/reports/wave33-close/gate.json

Idempotent: an id already present is skipped.
"""

import json
import pathlib

MATRIX = pathlib.Path("docs/reviews/r0_3/acceptance_matrix.json")
REPORT = "docs/reports/sweep-blind-spots/README.md"
CARD = "SWEEP-BLIND-SPOTS"


def entry(id_, title, why):
    return {"id": id_, "title": title, "why": why, "source": REPORT, "recorded_by": CARD}


NEW = [
    entry(
        "W32-SWEEP-MATRIX-EDIT-LOST-AT-MERGE",
        "W32-RECONCILE-SWEEP's own matrix amendment was resolved away at its merge, and a recorded `applied` verdict cites it",
        "Card SWEEP-BLIND-SPOTS (wave 35) measured, while retro-sweeping wave 32, that the "
        "`docs-compact-5#the-unapplied-integrator-reconciliation-list#1` verdict in "
        "docs/reports/w32-reconcile-sweep/reconciliation-waves-29-31.json says the item is `applied` and cites "
        "'DOCS-COMPACT-O in the matrix now carries the passes 4 and 5 residual names'. It does not. Measured "
        "by `git show` at three points: at 16070450 (W32-RECONCILE-SWEEP's own commit) the DOCS-COMPACT-O "
        "named_followups row is 5,473 bytes and contains both DOCS-COMPACT-4-TABLE-SYNC and "
        "DOCS-COMPACT-5-RECONCILIATION-LIST; at the merge 3bbec4d0 (parents 1423b6dc and 16070450) it is 3,265 "
        "bytes and contains neither; HEAD's row is byte-equal to the post-merge one. It is the ONLY "
        "named_followups row that changed across that merge (-2,208 bytes), so this is a lost edit at a merge "
        "resolution rather than a later deliberate retirement. Consequence today: DOCS-COMPACT-4-TABLE-SYNC "
        "reaches the ledger NOWHERE, and DOCS-COMPACT-5-RECONCILIATION-LIST reaches it only through a wave-34 "
        "entry (CONSTRAINT-NAMED-BOUNDARIES-NOT-OPEN-WORK) - so the two findings that named the "
        "reconciliation defect were, once again, the untracked ones. The wider point is about the tool this "
        "card maintains: a `--check` green says a verdict was RECORDED, never that its 'where' still holds, "
        "and this is the first measured instance of an `applied` note whose where does not. Discharge: "
        "restore the pass-4 and pass-5 residual names to DOCS-COMPACT-O (or register "
        "DOCS-COMPACT-4-TABLE-SYNC as its own entry), and decide whether a close should re-verify the `where` "
        "of verdicts recorded by an earlier close.",
    ),
    entry(
        "CONSTRAINT-14-WAVE-32-CENSUS-INCOMPLETE",
        "Constraint 14 claims 'every card of waves 32, 33 and 34' over a list that omits three wave-32 cards",
        "AGENTS.md's constraint-14 keyword-census ledger, in the 'Cards that do not engage this constraint at "
        "all' bullet, was rewritten at DOCS-COMPACT-6 (wave 34) to read '**every card of waves 32, 33 and "
        "34**' and then enumerates ten: CT-WIRE-LANE-4, CT-COMPOSED-NATIVE, TRACE-ALPHABET-LEASE-BOUNDARY, "
        "W32-RECONCILE-SWEEP, C-GATEWAY-RETURN-POSITION, VERIFIER-HOST-CLOSED-INPUT-DEADLINE, the Lean "
        "proof-integrity pass, STABILITY-REEVAL #1, C-GATEWAY-RETURN-CALL-W4 and DOCS-COMPACT-6. Wave 32 "
        "merged SEVEN cards (docs/reports/wave32-close/gate.json names them): MUTEX-SHARE-CROSS, "
        "PROOF-LEG-BATCH and DOCS-COMPACT-5 are missing. So a claim of exhaustiveness now stands over an "
        "incomplete list, and 'Waves 28 through 34 each closed at zero reserved words' rests on it. The "
        "omission is not neutral for MUTEX-SHARE-CROSS specifically: that card's report SS13.2 explicitly asked "
        "the integrator for its '+0 / +0 / +0' census row, the card claimed a NEW POSITION for already-reserved "
        "words (`thread scope NAME share PLACE under MUTEX`), and its whole integrator list was never swept "
        "(WAVE-32-CLOSE-UNSWEPT). Measured by card SWEEP-BLIND-SPOTS (wave 35); AGENTS.md contains the string "
        "'MUTEX-SHARE-CROSS' zero times. Discharge: add the three cards' census rows, or narrow the "
        "exhaustiveness claim to the cards actually enumerated.",
    ),
    entry(
        "MUTEX-SHARE-CROSS-INTEGRATOR-LIST-UNLANDED",
        "MUTEX-SHARE-CROSS's entire integrator reconciliation list is unlanded three waves on",
        "Card SWEEP-BLIND-SPOTS (wave 35) retro-swept wave 32 and measured all five items of "
        "docs/reports/mutex-share-cross/README.md SS13 against the tree. Two landed (the derived lean_audits "
        "count, now 129 with AuditMutexShareCross.lean in validation.lean_audit_files; the public-API notice, "
        "verified at crates/qcore-core/src/transfer.rs:161,167). Three did not, and none by a decision: the "
        "proposed standing constraint (the opt-in `thread scope NAME share PLACE under MUTEX` clause and its "
        "three-way division of labour between qcore_core::transfer's classification, the crossing's "
        "absent/mismatched protocol gates, and the UNEDITED CheckedMutexInvariantV1 checker's Q611) is absent "
        "- AGENTS.md contains 'MUTEX-SHARE-CROSS' zero times; the Q600 namespace row (AGENTS.md:1441) still "
        "reads 'Four later cards move this row' and names four, not five; and the Q610-Q627 row "
        "(AGENTS.md:1442) is one unchanged line. The drift is visible because the AUTHORITY the table "
        "summarises DOES carry it: docs/QCORE_SPEC_r0_3.md:1004 reads 'Card MUTEX-SHARE-CROSS (wave 32) widens "
        "Q600's boundary condition and the Q610-Q627 sub-band's route'. AGENTS.md calls its own table 'a "
        "summary of it, not a second allocation', so the summary has drifted from its authority for one "
        "card. This is the concrete cost of WAVE-32-CLOSE-UNSWEPT. Discharge: land the constraint IN THE "
        "WORDS THE CARD PROPOSED (quoted verbatim in the source report SS6) or record it `declined` with its "
        "ground, per CLAUDE.md's close rule; and update the two namespace rows and the constraint-14 census "
        "row (CONSTRAINT-14-WAVE-32-CENSUS-INCOMPLETE). No code is challenged - the card's Rust, Lean and "
        "spec text are all in the tree.",
    ),
    entry(
        "WAVE-34-CLOSE-UNSWEPT",
        "Wave 34 has no close directory at all, so its three cards' integrator sections are swept by nothing",
        "Card SWEEP-BLIND-SPOTS (wave 35) measured, while discharging WAVE-32-CLOSE-UNSWEPT, that the same "
        "defect recurred two waves later: `docs/reports/wave34-close/` DOES NOT EXIST (wave23-close through "
        "wave33-close all do), so wave 34 wrote neither a gate.json nor a reconciliation.json - AGENTS.md's "
        "merged-gate row instead cites 'docs/reports/docs-compact-6/README.md SS8' for its numbers. Every "
        "reconciliation record on disk was enumerated: w32-reconcile-sweep/reconciliation-waves-29-31.json "
        "(20 cards, waves 29-31), wave32-close/reconciliation.json (7 cards, written by THIS card) and "
        "wave33-close/reconciliation.json (2 cards). None names stability-reeval-1, "
        "c-gateway-return-call-w4 or docs-compact-6, and a sweep of those three returns 25 "
        "integrator-addressed items with no verdict anywhere. This card does not sweep them: its scope is "
        "wave 32, and 25 items measured against the tree is a card's work rather than a footnote. Discharge: "
        "`python3 scripts/reconciliation_sweep.py . --range <wave-33 close>..<wave-34 tip> --check "
        "docs/reports/wave34-close/reconciliation.json`, with a human verdict measured against the tree for "
        "each of the 25 - noting that wave 34's own STABILITY-REEVAL #1 is the card that FOUND "
        "WAVE-32-CLOSE-UNSWEPT, so its own integrator list going unswept is the defect closing a loop on "
        "itself.",
    ),
    entry(
        "SWEEP-UNCARDED-RULE-APPROXIMATE",
        "The uncarded-commit rule is necessary but not sufficient, and its precision depends on an open decision",
        "Card SWEEP-BLIND-SPOTS (wave 35) added `--uncarded` to close the WAVE-BASE-UNSWEPT seam, and states "
        "its rule plainly in the module docstring rather than implying more than it measures: a commit is "
        "`claimed` when it writes some `docs/reports/<card>/README.md`. That is NECESSARY for being carded, "
        "NOT SUFFICIENT, so the listing is a human-eye list and the tool classifies nothing in it. Measured on "
        "the wave-32 range (8bf7fd33..474f27af): 15 of 38 commits are unclaimed, of which most are a card's "
        "own code commits (`MUTEX-SHARE-CROSS: ...`, `CT-WIRE-LANE-4: ...`) or the integrator's ledger and "
        "close commits - correct behaviour, and noise a human filters in seconds. On the wave-33 range "
        "(474f27af..37bc9004) it returns exactly one commit, 37bc9004, which is the live instance "
        "WAVE-BASE-UNSWEPT named. The residual is that the rule's precision is coupled to an open decision: "
        "under SYNC-RULE-TOOLING-COMMIT-SCOPE option (c) - move tooling reports out of docs/reports/ so that "
        "tree is one kind of thing - the rule becomes exact instead of approximate. Note also 37bc9004 itself "
        "writes THREE files under docs/reports/ (corpus-repin/diff_artifacts.py, remaining-d02/build_matrix.py, "
        "wave17-gen-ctreasons/backfill_contracts.py) without writing any README.md, which is why the rule "
        "keys on the report file rather than the directory. Discharge: re-measure the rule's precision once "
        "SYNC-RULE-TOOLING-COMMIT-SCOPE is decided, or record it as a permanent human-eye list.",
    ),
    entry(
        "SWEEP-SEPARATOR-ITEM-ARTIFACT",
        "A markdown horizontal rule inside an integrator section becomes an item with no content",
        "Card SWEEP-BLIND-SPOTS (wave 35) found while retro-sweeping wave 32 that "
        "docs/reports/trace-alphabet-lease-boundary/README.md SS5.4 ends with a `---` horizontal rule, which "
        "`_section_items` reads as a top-level block and emits as item "
        "`trace-alphabet-lease-boundary#for-the-integrator-at-the-wave-32-close#6`, carrying the text '---' "
        "and no request. It therefore demands a human verdict for nothing. It was given one (`noted`) rather "
        "than suppressed, and the extractor was NOT changed, deliberately: filtering it would mean the "
        "extractor deciding what is and is not content, which is the one judgment this module refuses to "
        "make, and a rule between two blocks is only distinguishable from a deliberate separator by reading "
        "intent. Low severity - one spurious item in 67 across seven cards. Discharge: either accept the "
        "artifact permanently and say so, or add a narrow syntactic exclusion for a block whose entire text "
        "is punctuation, with a test showing it cannot swallow a real item.",
    ),
]


def main() -> int:
    matrix = json.loads(MATRIX.read_text(encoding="utf-8"))
    followups = matrix["named_followups"]
    present = {item["id"] for item in followups}
    added = [item for item in NEW if item["id"] not in present]
    followups.extend(added)
    matrix["dimensions"]["named_followups"] = len(followups)
    MATRIX.write_text(
        json.dumps(matrix, indent=1, ensure_ascii=False) + "\n", encoding="utf-8"
    )
    for item in added:
        print("added", item["id"])
    for item in NEW:
        if item["id"] in present:
            print("already present, skipped", item["id"])
    print(f"named_followups now {len(followups)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

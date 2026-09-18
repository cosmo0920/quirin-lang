#!/usr/bin/env python3
"""WAVE-40-CLOSE: the tracker edits, in one place, on the wave-39 close's precedent.

`named_followups` is hand-maintained (the generator carries it verbatim), so this
appends / amends / removes and rewrites in the generator's byte format
(indent=1, ensure_ascii=False, trailing newline); `dimensions.named_followups`
is then re-derived by

    python3 -B docs/reports/remaining-d02/build_matrix.py . \
        --gate docs/reports/wave40-close/gate.json

Phase 1 (`--show`): print the full text of every entry this close proposes to
retire, so each discharge condition is read before anything moves.
Phase 2 (no flag): retire three, amend six, register two. Idempotent.
"""
import json
import pathlib
import sys

MATRIX = pathlib.Path("docs/reviews/r0_3/acceptance_matrix.json")
REPORT = "docs/reports/wave40-close/README.md"
CARD = "WAVE-40-CLOSE"

RETIRE = [
    # Discharge (its own words): "credit the rows in a stage-evidence record, or
    # state why the runner does not back them." MEASURED at this base from
    # docs/reevaluation/ct_manifest.json: the SIX rows run_composed_native backs
    # (CT22 and CT25, each c_emission / native_execution / native_sanitized) all
    # read `executed`, each naming compiler/tests/ct_corpus.rs::run_composed_native
    # as executor and that case's own fixture as artifact - constraint 23's
    # condition, met. STABILITY-REEVAL-4 measured the same and deliberately left
    # the retirement to this close (its section 6 item 5).
    "CT-COMPOSED-NATIVE-ROWS",
    # Discharge (its own words): "state the platform boundary in the CT ledger row
    # beside the sanitized-native count, so the number carries the host condition
    # it was measured on." Landed by this close in AGENTS.md's CT row, with the
    # entry's own wording corrected there rather than carried: the gate is a
    # runtime cfg! placed AFTER the reference-run and c_source assertions
    # (compiler/tests/ct_corpus.rs, run_composed_native), so the EMISSION row is
    # genuinely exercised on another host and "asserting nothing" overstated it.
    # All ten executed native_sanitized rows carry host_gate ["linux","macos"],
    # measured at this base.
    "CT-COMPOSED-NATIVE-NON-LINUX-VACUOUS-PASS",
    # Discharge (its own words), SECOND arm: "state the rule that a merge whose
    # message states a figure differing from the card's gate must publish one, and
    # apply it retrospectively to these two by a dated appended note
    # (constraint 21)." Both halves landed by this close: the rule is in CLAUDE.md
    # beside the wave-39 integration-gate convention, and dated appended notes
    # carrying each figure with its in-history merge sha are at
    # docs/reports/ct07-two-thread-proof-leg/README.md and
    # docs/reports/ct42-cell-route/README.md. The sibling
    # INTEGRATION-TIP-CITATION-UNREACHABLE stays OPEN on its own arm.
    "INTEGRATION-GATE-NOT-PUBLISHED-FOR-EVERY-MERGE",
]


def entry(id_, title, why, source):
    return {"id": id_, "title": title, "why": why, "source": source, "recorded_by": CARD}


NEW = [
    entry(
        "CONSTRAINT-14-WORD-COUNT-CASE-FILTERED",
        "Constraint 14's reserved-word count is a lowercase-only extraction of a case-sensitive set, and reads 112 where the function matches 113",
        "MEASURED by the wave-40 close. Constraint 14 names "
        "`qcore_frontend::frontend::is_reserved_source_word` as 'the one place to count them' and "
        "published 112. Extracting every string literal from that function's `matches!` arms with "
        "comments stripped returns 113 distinct words; the difference is `FULL`, which sits among "
        "the quantity and grade literals (`zero`, `one`, `affine`, `relevant`, `many`, `inf`, "
        "`FULL`) and is uppercase. DOCS-COMPACT-8's own artifact "
        "(docs/reports/docs-compact-8/artifacts/reserved-words.txt, 112 lines, 'extracted from the "
        "`matches!` arms') is lowercase-only and misses exactly that one word; the close diffed the "
        "two sets and the symmetric difference is {FULL} in one direction and empty in the other. "
        "The count is corrected to 113 in AGENTS.md with the case-sensitivity stated. This is the "
        "AGENTS-MAP-FIGURES-UNDERIVED family a third time - a figure whose stated authority is a "
        "piece of Rust that nothing derives it from - and the close notes the aggravating shape: "
        "the constraint's own sentence claims the derivation while the artifact beside it was "
        "filtered. DISCHARGE: a test or gate step that derives the count from "
        "`is_reserved_source_word` itself and fails when the prose disagrees, shown red by adding "
        "or removing one arm; until then the figure is one extraction's arithmetic and the "
        "case-sensitivity sentence is what keeps the next extraction honest.",
        REPORT,
    ),
    entry(
        "RESIDUAL-QUIET-BY-PROSE-MENTION",
        "reconciliation_sweep.py --residuals reads the whole tracker text, so a name mentioned inside another entry's why goes quiet without being registered",
        "MEASURED by the wave-40 close, five live instances in one wave. --residuals lists "
        "residual-shaped names the tracker 'does not carry', and it decides that by scanning the "
        "tracker's whole text rather than its entry ids. STABILITY-REEVAL-4 registered 23 entries "
        "whose `why` texts cite earlier names, and five names the wave-39 close had listed as "
        "untracked are therefore absent from this close's listing while still being absent from "
        "`named_followups` BY ID: CT46-NOTES-PREDATE-TSAN-ROW (retired at wave 39; its two live "
        "AGENTS.md instances are exactly what LEDGER-TSAN-BOUNDARY-SENTENCES-STALE was registered "
        "for), CT07-PROOF-LEG-PATTERN-UNAVAILABLE, CT42-CELL-FALLBACK-ARM-DIRECTION, "
        "WAVE-38-QUEUE-NAMES-UNTRACKED and MUTATION-HARNESS-MTIME-RESTORE - each appearing once or "
        "twice in the matrix's prose and zero times as an id. HTTP-V1-WINDOWS-RUN is the standing "
        "case (absent by id, twice in prose, silent in the listing for three waves; wave 39 said so "
        "in its section 8). The wave-39 close recorded the mechanism as a one-off 'quiet for the "
        "wrong reason'; this entry records it as a PROPERTY of the tool, because registering "
        "entries is the ordinary way a wave makes names quiet, so the listing shrinks fastest "
        "exactly when the most work is being recorded. DISCHARGE: --residuals matches against "
        "`named_followups` ids (and each entry's own id alone), with a separate 'mentioned in "
        "another entry's why' bucket so nothing is lost, shown red by mutation; OR a recorded "
        "decision that a prose mention is tracking, with the six names above given entries or an "
        "explicit statement that their tracker is the entry that cites them.",
        REPORT,
    ),
]

#: id -> (field, old substring that must be present, replacement text appended or
#: substituted). Each amendment is a stated, non-softening change to an entry the
#: cards' integrator lists asked the close to correct; every one ADDS a measured
#: sentence and none removes a discharge condition.
AMEND = {
    "LEDGER-TSAN-BOUNDARY-SENTENCES-STALE": (
        " PARTIALLY DISCHARGED at the wave-40 close, and DELIBERATELY NOT RETIRED "
        "(DOCS-COMPACT-8 section 9 item 10 and section 4.10 ask for exactly this). Two of the three "
        "parts are met: AGENTS.md's constraint 15h sentence and its constraint 15o sentence were "
        "both rewritten by DOCS-COMPACT-8 to the measured position, the second found only at that "
        "card's re-derivation because the first pass grepped `ThreadSanitizer` and that sentence "
        "spells it `TSan`. The THIRD part is unmet and is this entry's standing discharge "
        "condition: compiler/tests/ct07_proof_leg.rs and "
        "docs/reports/ct07-two-thread-proof-leg/README.md still cite the retired "
        "TASK-ROUTE-TSAN-SEGV. The close did not take it either - a crate-source comment and a "
        "frozen card report are a code card's and an appending card's work respectively, and the "
        "close records the omission rather than leaving the entry to read as fully open. Both "
        "citations reach no tracker except this entry."
    ),
    "LEASE-BOUNDARY-TRANSCRIPTION-PREMISE": (
        " CORRECTED at the wave-40 close on STABILITY-REEVAL-4's measurement (its section 6 item 7, "
        "and PROOF-LEG-DIGESTS-UNCROSSCHECKED's own why): the second arm of the discharge above "
        "cites a mechanism that DOES NOT EXIST. CT01's proof leg does no digest binding of a Lean "
        "term to a Rust pin; all four legs (ct12_ct14_proof_reach.rs, ct03_proof_leg.rs, "
        "ct37_proof_leg.rs, ct07_proof_leg.rs) carry a bare Rust literal that nothing compares with "
        "its Lean twin, and no test under compiler/tests/ or crates/ reads any .lean file. READ THE "
        "SECOND ARM AS NAMING NO EXISTING PATTERN: the discharge is either the module's own "
        "transcription sentence carried into constraint 15n, or the cross-check "
        "PROOF-LEG-DIGESTS-UNCROSSCHECKED asks for, which would have to be built first. The close "
        "DECLINED to build it - a gate script extracting each `theorem ct*_digest` and comparing it "
        "with the corresponding `const` is code with a mutation control, not close work - and names "
        "it as brief material for the next implementation wave."
    ),
    "INTEGRATION-TIP-CITATION-UNREACHABLE": (
        " STILL OPEN after the wave-40 close, with TWO NEW INSTANCES and one live repair. Both "
        "wave-40 merges cite an integration tip that is a commit object in this repository and a "
        "NON-ANCESTOR of main - f661b2aa (merge d6a57f66, STABILITY-REEVAL-4) and 3cb27d80 (merge "
        "0c7b0cea, DOCS-COMPACT-8), each measured with git merge-base --is-ancestor, exit 1 - so "
        "the wave-39 count of five is now seven. c188d3ab, the tip AGENTS.md's merged-gate row "
        "named at this base, is a non-ancestor too; the close REPAIRED that live instance by "
        "re-owning the row at its own in-history branch tips, which is the only instance a close "
        "can repair without editing a frozen record. The successor mapping for the wave-39 five and "
        "the wave-40 two is published in docs/reports/wave40-close/README.md rather than written "
        "INTO the three frozen integration-gate.json files (constraint 21's dual-publication "
        "shape), so the first discharge arm - the sha inside each such file - remains unmet. The "
        "sibling INTEGRATION-GATE-NOT-PUBLISHED-FOR-EVERY-MERGE was retired at that close on its "
        "own second arm, which carries the in-history sha for the two wave-39 merges that published "
        "no file; this entry is what the rest of the family still needs."
    ),
    "RECONCILIATION-VERIFY-CLAIM-PERISHABLE": (
        " DECIDED AND STILL OPEN at the wave-40 close, with the class measured far wider than this "
        "entry states. Re-running the wave-39 close's own command at 0c7b0cea fails 16 of its 57 "
        "verify claims, not one. Attributed commit by commit by testing each claim's normalised "
        "needle against `git show <rev>:<path>` at 6dc484ad, f11ccbba, d6a57f66 and 0c7b0cea: ONE "
        "(the increments-index pointer this entry names) broke at the wave-opening commit f11ccbba, "
        "and FIFTEEN broke at the DOCS-COMPACT-8 merge 0c7b0cea - fourteen in AGENTS.md, one in "
        "CLAUDE.md - none at the reeval merge. DURABILITY IS A PROPERTY OF THE TARGET FILE: every "
        "claim into the acceptance matrix (17), a frozen increment record (4), the spec (3), crate "
        "source (2), the CT manifest and its fixture (2) and a frozen card report (1) survived; 15 "
        "of the 27 into AGENTS.md/CLAUDE.md did not. So the FIRST discharge arm - 'quote durable "
        "text' - is INSUFFICIENT ON ITS OWN, measured: verifying an AGENTS.md amendment in "
        "AGENTS.md is the point of the arm, and a compaction card is chartered to rewrite exactly "
        "that prose. The close recorded the operative rule in CLAUDE.md (a --check is green at its "
        "own close and only there; a later red is evidence of supersession, not of a defect, and is "
        "never repaired by editing a frozen record; prefer the most durable location an amendment "
        "genuinely allows). DISCHARGE IS NOW THE SECOND ARM ALONE: a --check mode distinguishing "
        "'applied and later legitimately superseded' from 'never applied' - for instance a claim "
        "pinned to the commit it was verified at, or a per-close snapshot the check reads - with "
        "the wave-39 record's 16 claims reclassified under it."
    ),
    "INTEGRATION-GATE-CLANG-CLAIM-BORROWED": (
        " SECOND ARM TAKEN at the wave-40 close, first arm still open. STABILITY-REEVAL-4 measured "
        "this entry UNDER-SCOPED (its section 6 item 8): the reuse is disclosed in all three files' "
        "own notes, and the real defect is that the generator's check cannot tell a borrowed "
        "recording from an own one at all (CLANG-RECORDING-UNBOUND-TO-ITS-RUN). The close took the "
        "arm that is close work: AGENTS.md's standing-gate section now states, in those words, that "
        "the `clang` gate key is a DISCLOSURE CONVENTION AND NOT A CHECK, names what "
        "clang_recording() actually verifies and what it does not, and instructs that "
        "`clang --version` be run in the worktree that took the measurement. Borrowing is permitted "
        "only where the gate file's own note says so. BOTH ENTRIES STAY OPEN on the mechanical arm "
        "- a recording carrying a run identity the gate file also names, checked as a pair and "
        "shown red by mutation - which is generator work with a control, not close work."
    ),
    "CLANG-RECORDING-UNBOUND-TO-ITS-RUN": (
        " SECOND ARM TAKEN at the wave-40 close: AGENTS.md's standing-gate section now records that "
        "the key is a disclosure convention rather than a check, in those words, so a green "
        "regeneration is not read as evidence Clang was present for the run being recorded. The "
        "close's own gate recording was taken by running `clang --version` in its own worktree "
        "(exit 0 captured to its own file, both runs' recordings compared), which is the honesty "
        "the convention now asks for and which the check still cannot measure. THE FIRST ARM STAYS "
        "OPEN and is the real discharge."
    ),
}


def load():
    return json.loads(MATRIX.read_text(encoding="utf-8"))


def show():
    matrix = load()
    by_id = {e["id"]: e for e in matrix["named_followups"]}
    for name in RETIRE:
        entry_ = by_id.get(name)
        print("=" * 78)
        if entry_ is None:
            print(f"{name}: ABSENT from named_followups - nothing to retire")
            continue
        print(json.dumps(entry_, indent=1, ensure_ascii=False))
    print("=" * 78)
    for name in AMEND:
        print(f"AMEND {name}: {'present' if name in by_id else 'ABSENT'}")
    for new in NEW:
        print(f"REGISTER {new['id']}: {'ALREADY PRESENT' if new['id'] in by_id else 'new'}")
    print(f"\n{len(matrix['named_followups'])} entries before any edit.")


def main():
    matrix = load()
    followups = matrix["named_followups"]
    before = len(followups)
    by_id = {e["id"]: e for e in followups}

    for name in RETIRE:
        if name not in by_id:
            print(f"retire {name}: already absent (idempotent)")
    followups = [e for e in followups if e["id"] not in set(RETIRE)]

    for name, addition in AMEND.items():
        target = next((e for e in followups if e["id"] == name), None)
        if target is None:
            raise SystemExit(f"amend {name}: entry absent - refusing to invent one")
        if addition.strip() in target["why"]:
            print(f"amend {name}: already applied (idempotent)")
            continue
        target["why"] = target["why"].rstrip() + addition

    present = {e["id"] for e in followups}
    for new in NEW:
        if new["id"] in present:
            print(f"register {new['id']}: already present (idempotent)")
            continue
        followups.append(new)

    matrix["named_followups"] = followups
    MATRIX.write_text(
        json.dumps(matrix, indent=1, ensure_ascii=False) + "\n", encoding="utf-8"
    )
    print(f"named_followups {before} -> {len(followups)}")


if __name__ == "__main__":
    if "--show" in sys.argv:
        show()
    else:
        main()

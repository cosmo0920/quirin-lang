#!/usr/bin/env python3
"""WAVE-40-CLOSE: build docs/reports/wave40-close/reconciliation.json.

One entry per swept item, keyed `card#section#item` exactly as
`scripts/reconciliation_sweep.py` names it. Every `applied` verdict carries a
`verify` list of checkable text at a named location; every non-`applied`
verdict carries a note with its ground. The tool decides nothing: the verdicts
below are this close's, measured against the tree.
"""
import json
import pathlib

OUT = pathlib.Path("docs/reports/wave40-close/reconciliation.json")

M = "docs/reviews/r0_3/acceptance_matrix.json"
A = "AGENTS.md"
C = "CLAUDE.md"
W40 = "docs/development/increments/wave-40.md"
IDX = "docs/development/increments/README.md"


def V(verdict, note, verify=None):
    entry = {"verdict": verdict, "note": note}
    if verify:
        entry["verify"] = verify
    return entry


R = "stability-reeval-4#integrator-reconciliation-list#"
D = "docs-compact-8#for-the-integrator#"

verdicts = {}

# ---------------------------------------------------------------- STABILITY-REEVAL-4
verdicts[R + "1"] = V(
    "applied",
    "The retirement is the card's own, taken at its merge d6a57f66 and verified here rather than "
    "assumed: W36-W37-MUTATION-HARNESSES-UNSHIPPED is absent from named_followups by id at this "
    "close's tip, and CLAUDE.md still carries the convention line the retirement's discharge rests "
    "on, so nothing was orphaned by removing the entry. The sibling "
    "LEASE-BOUNDARY-MUTATION-EVIDENCE-UNSHIPPED is untouched and still registered by id, checked. "
    "The name is still written in CLAUDE.md, the wave-40 record and both cards' reports, which is "
    "the discharged class of README section 7 and expected.",
    [
        {"file": M, "absent": '"id": "W36-W37-MUTATION-HARNESSES-UNSHIPPED"'},
        {"file": M, "contains": '"id": "LEASE-BOUNDARY-MUTATION-EVIDENCE-UNSHIPPED"'},
        {"file": C, "contains": "a count whose harness lives under `measure/` or `private/` is written as"},
    ],
)
verdicts[R + "2"] = V(
    "deferred",
    "TWO OF THREE PARTS LANDED, THE THIRD NOT TAKEN, and the entry deliberately NOT retired - "
    "DOCS-COMPACT-8's section 9 item 10 asks for exactly this and the close agrees. Parts 1 and 2 "
    "(AGENTS.md's constraint 15h 'measured, not claimed' TSan sentence and constraint 15o's 'no "
    "TSan row exists anywhere') were rewritten by DOCS-COMPACT-8, which owns that prose; verified "
    "below. Part 3 - compiler/tests/ct07_proof_leg.rs and "
    "docs/reports/ct07-two-thread-proof-leg/README.md still citing the retired TASK-ROUTE-TSAN-SEGV "
    "- was NOT taken by this close either. GROUND: a crate-source comment is a code card's edit and "
    "a frozen card report takes an appending card's dated section; this close appended to that same "
    "report for a different residual and deliberately did not smuggle a second, unrelated "
    "correction into the same note. The omission is recorded in the entry's own `why` as its "
    "standing discharge condition rather than left for a reader to rediscover, and the entry stays "
    "OPEN - so the two AGENTS.md instances the reeval found now reach a tracker, which they did not "
    "before.",
    [
        {"file": M, "contains": "PARTIALLY DISCHARGED at the wave-40 close, and DELIBERATELY NOT RETIRED"},
        {"file": M, "contains": '"id": "LEDGER-TSAN-BOUNDARY-SENTENCES-STALE"'},
    ],
)
verdicts[R + "3"] = V(
    "applied",
    "DOCS-COMPACT-8 published the three numbers separately in the merged-gate row (238 strict "
    "summaries over 236 targets, TWO doubled sections, with 240 explained as the loose match). This "
    "close made that a THIRD independent measurement, on a third tree, with the reeval's own shipped "
    "script (docs/reports/stability-reeval-4/scripts/count_summaries.py, run against this close's "
    "base gate log): targets 236, strict 238, loose 240, two doubled sections (Doc-tests qcore_core, "
    "Doc-tests qcore_verifier_host), zero targets emitting no summary. Three measurements on three "
    "trees agree exactly, so the row now states an arithmetic three runs support.",
    [
        {"file": A, "contains": "238 result summaries over 236 test targets"},
        {"file": A, "contains": "made that a **third** independent measurement on a third tree"},
    ],
)
verdicts[R + "4"] = V(
    "applied",
    "Both halves taken. The qualifier is in AGENTS.md's CT row with the measurement behind it: all "
    "ten executed native_sanitized rows declare host_gate [\"linux\",\"macos\"] in "
    "docs/reevaluation/ct_manifest.json (CT03, CT07, CT09, CT17, CT22, CT25, CT26, CT34, CT46, "
    "CT49, read at this close). The entry's own wording is corrected in the same sentence rather "
    "than carried forward: the gate is a runtime cfg!(any(target_os = \"linux\", target_os = "
    "\"macos\")) placed AFTER the reference-run and c_source assertions in "
    "compiler/tests/ct_corpus.rs, so the EMISSION row is genuinely exercised on another host and "
    "'asserting nothing' overstated it. The entry is then RETIRED, named explicitly in README "
    "section 5 and verified registered by id first.",
    [
        {"file": A, "contains": "sanitized-native **10 of 47 — a Unix figure**"},
        {"file": A, "contains": "the *emission* row is genuinely exercised and the sanitizer row asserts nothing"},
        {"file": M, "absent": '"id": "CT-COMPOSED-NATIVE-NON-LINUX-VACUOUS-PASS"'},
    ],
)
verdicts[R + "5"] = V(
    "applied",
    "RETIRED at the close, which is the first of the two options the card offers, and the "
    "postponement's ground is respected rather than overridden: the reeval declined to retire "
    "another lane's entry out of an audit, and a close is the document that may. Re-measured here "
    "rather than taken on the card's word - the SIX stage rows run_composed_native backs (CT22 and "
    "CT25, each c_emission / native_execution / native_sanitized) all read `executed` in "
    "docs/reevaluation/ct_manifest.json, each naming compiler/tests/ct_corpus.rs::run_composed_native "
    "as executor and that case's own fixture as artifact, which is constraint 23's condition word "
    "for word. Named explicitly in README section 5; registered by id verified before removal.",
    [{"file": M, "absent": '"id": "CT-COMPOSED-NATIVE-ROWS"'}],
)
verdicts[R + "6"] = V(
    "deferred",
    "NOT DECIDED HERE, and the ground is that both halves belong to an older open decision. "
    "CT24-CLEANUP-TASK-COMPOSITION was classified a PROSPECTIVE name by the wave-35 close and "
    "CT46-NOTES-PREDATE-TSAN-ROW was retired by the wave-39 close against a discharge condition it "
    "met; registering a successor for either would be registering a name for a case whose reason "
    "the CT ledger row does not carry, which is CT10-REASON-ROW's question - deferred now for a "
    "SEVENTH wave and re-listed in this record's deferred_decisions with its ground. What the close "
    "DID take is the half it could measure: the general defect that made the reeval notice these "
    "two at all is registered as RESIDUAL-QUIET-BY-PROSE-MENTION, which names both of them and the "
    "four other names now silent in --residuals while absent by id. Neither the CT-ledger-row "
    "restatement nor a successor entry is landed, and this close does not pretend otherwise.",
    [
        {"file": M, "contains": '"id": "RESIDUAL-QUIET-BY-PROSE-MENTION"'},
        {"file": M, "contains": "CT46-NOTES-PREDATE-TSAN-ROW (retired at wave 39;"},
    ],
)
verdicts[R + "7"] = V(
    "applied",
    "The sentence is amended, which is the half the card offers first: "
    "LEASE-BOUNDARY-TRANSCRIPTION-PREMISE's `why` now records that its second discharge arm names "
    "NO EXISTING PATTERN - CT01's proof leg does no digest binding of a Lean term to a Rust pin, "
    "and all four legs carry a bare literal nothing compares - so a reader cannot follow it into a "
    "mechanism that does not exist. The card's alternative second half, taking "
    "PROOF-LEG-DIGESTS-UNCROSSCHECKED's discharge (a gate script extracting each `theorem "
    "ct*_digest` and comparing it with the corresponding `const`), is DECLINED WITH ITS GROUND, "
    "recorded in the same amendment: that is code with a mutation control, not close work, and it "
    "is named as brief material for the next implementation wave in README section 10. Both "
    "entries stay registered.",
    [
        {"file": M, "contains": "READ THE SECOND ARM AS NAMING NO EXISTING PATTERN"},
        {"file": M, "contains": '"id": "PROOF-LEG-DIGESTS-UNCROSSCHECKED"'},
    ],
)
verdicts[R + "8"] = V(
    "applied",
    "The SECOND arm taken, in the words the card proposed: AGENTS.md's standing-gate section now "
    "records that the `clang` key is a disclosure convention and not a check, names what "
    "clang_recording() actually verifies (a file named clang-version.txt beside the gate file or in "
    "a gate/ subdirectory; the claim parsing as present:/absent:; the banner equalling the "
    "recording's first line) and what it does not (no re-invocation, no host identity, no "
    "timestamp, no commit binding, and the clang-version.exit arm firing only if that file "
    "happens to exist), and instructs that `clang --version` be run in the worktree that took the "
    "measurement. This close did exactly that - exit 0 captured to its own file, both runs' "
    "recordings compared - rather than borrowing another report's. BOTH entries "
    "(INTEGRATION-GATE-CLANG-CLAIM-BORROWED, CLANG-RECORDING-UNBOUND-TO-ITS-RUN) are amended to "
    "record the arm taken and stay OPEN on the mechanical arm, which is generator work with a "
    "control.",
    [
        {"file": A, "contains": "**The `clang` gate key is a disclosure convention, not a check**"},
        {"file": M, "contains": "SECOND ARM TAKEN at the wave-40 close, first arm still open."},
    ],
)
verdicts[R + "9"] = V(
    "applied",
    "Both conventions taken, one by decision and one by repair. (1) "
    "INTEGRATION-GATE-NOT-PUBLISHED-FOR-EVERY-MERGE is DECIDED on its second arm and retired: the "
    "rule is in CLAUDE.md beside the wave-39 convention - a merge whose stated figure DIFFERS from "
    "the card's own gate.json owes a docs/reports/<card>/integration-gate.json, one whose figure "
    "EQUALS it does not - and it is applied retrospectively by dated appended sections on "
    "docs/reports/ct07-two-thread-proof-leg/README.md and docs/reports/ct42-cell-route/README.md, "
    "each carrying the merge's stated figure, the non-ancestor tip it names and the in-history "
    "merge sha beside it (constraint 21: appended, nothing above them edited). Measured, not "
    "assumed: both wave-40 merges state figures EQUAL to their cards' gates (4,727 workspace / 141 "
    "Lean / 303 Python in both docs/reports/{stability-reeval-4,docs-compact-8}/gate.json), so "
    "neither owed a file. (2) INTEGRATION-TIP-CITATION-UNREACHABLE stays OPEN, amended with two new "
    "instances - f661b2aa and 3cb27d80, both non-ancestors by git merge-base --is-ancestor, seven "
    "in all - and the one instance a close can repair without touching a frozen record IS repaired: "
    "AGENTS.md's merged-gate row named the non-ancestor c188d3ab and now names this close's own "
    "in-history branch tips.",
    [
        {"file": C, "contains": "**When the file is owed, decided at the wave-40 close**"},
        {"file": "docs/reports/ct07-two-thread-proof-leg/README.md", "contains": "Published beside by `WAVE-40-CLOSE` (2026-09-19)"},
        {"file": "docs/reports/ct42-cell-route/README.md", "contains": "Published beside by `WAVE-40-CLOSE` (2026-09-19)"},
        {"file": M, "contains": "STILL OPEN after the wave-40 close, with TWO NEW INSTANCES"},
    ],
)
verdicts[R + "10"] = V(
    "applied",
    "Both mechanisms addressed, neither tool repaired, and the close says which is which. "
    "(1) UNCARDED-BLIND-TO-MERGE-PROSE: the close ran the hand sweep the residual asks for - `git "
    "show --cc` and `git log --format=%B` over both wave-40 merges plus a classification of every "
    "--uncarded commit - and published what the merge prose asserts in README section 4, which is "
    "the first close able to act on the finding. The TOOL arm (a combined-diff mode) is not taken; "
    "the entry stays open. (2) RECONCILIATION-VERIFY-CLAIM-PERISHABLE is DECIDED, and the decision "
    "rests on a wider measurement than the card had: the wave-39 record fails 16 of its 57 claims "
    "at this base, not one - FIFTEEN broken by the DOCS-COMPACT-8 merge rewriting the very "
    "AGENTS.md/CLAUDE.md sentences that close verified, ONE by the wave-opening commit - so "
    "'quote durable text' cannot be sufficient, because verifying an AGENTS.md amendment IN "
    "AGENTS.md is the point of the arm. CLAUDE.md now carries the operative rule (a --check is "
    "green at its own close and only there; a later red is supersession, never repaired by editing "
    "a frozen record; prefer the most durable location an amendment genuinely allows), the entry is "
    "amended with the measurement and its discharge narrowed to the SECOND arm alone, and the "
    "wave-39 claim is reclassified under that rule rather than by editing wave 39's record.",
    [
        {"file": C, "contains": "**A `--check` is green at its own close and only there, and that is the"},
        {"file": M, "contains": "DECIDED AND STILL OPEN at the wave-40 close, with the class measured far wider"},
        {"file": M, "contains": '"id": "UNCARDED-BLIND-TO-MERGE-PROSE"'},
    ],
)
verdicts[R + "11"] = V(
    "applied",
    "Carried into README section 10 with each conclusion attached to its candidate, and into the "
    "wave-40 record's close paragraph, so a later brief can be written from either: CT-PROOF-LEG-3 "
    "(CT07's ledger pattern does NOT generalise, CT50 the only remaining F02 candidate, the "
    "numerator already counting two binding shapes), CT-NATIVE-CREDIT (the premise is spent - the "
    "rows read `executed` and this close retired CT-COMPOSED-NATIVE-ROWS for it - so what remained "
    "was the platform qualifier, which this close also landed, leaving the card thinner than when "
    "it was proposed), TSAN-FAILURE-ARMS (the forcing harness breaks first on the failed-spawn "
    "path, so the arms need outcome hooks and a gate-closed control before a catch-rate row means "
    "anything). CT10, CT24 and CT41 are carried with the same treatment. Both CT-PROOF-LEG-3 and "
    "CT-NATIVE-CREDIT are still absent from the tree outside the reeval's report and this record, "
    "and README section 7 classifies them as prospective card names rather than residuals.",
    [{"file": W40, "contains": "CT-PROOF-LEG-3"}, {"file": W40, "contains": "TSAN-FAILURE-ARMS"}],
)
verdicts[R + "12"] = V(
    "applied",
    "CLAUDE.md's cadence bullet is advanced rather than left stale - it still read '#4 is due now, "
    "after wave 39 ... not run' at this base, the sibling compaction card having updated its own "
    "bullet and not this one. It now records #4 in wave 40 with its report, names #5 as due after "
    "wave 41, and carries firing #4's own nomination of the highest-value unexamined ground (the "
    "transcription family - LOOPWINDOWS-LEAN-IS-A-TRANSCRIPTION, "
    "CT42-CELL-DERIVER-LEAN-PARITY-UNBOUND, PROOF-LEG-DIGESTS-UNCROSSCHECKED as one defect wearing "
    "three names - and the standing constraints' own sentences, this firing having found two of "
    "them false). Firing #4's §1.3 process finding is landed as operational advice in the same "
    "bullet, in its own terms: what gets a residual discharged is BRIEF-REACH, not the class of "
    "work it needs.",
    [
        {"file": C, "contains": "**#5 is due after wave\n  41**"},
        {"file": C, "contains": "brief-reach, not the class of work it needs"},
    ],
)

# ------------------------------------------------------------------- DOCS-COMPACT-8
verdicts[D + "1"] = V(
    "noted",
    "The section's own preamble, not an item: it states that every item below needs a recorded "
    "verdict, which is CLAUDE.md's rule and is what this record does. Nothing to apply.",
)
verdicts[D + "2"] = V(
    "applied",
    "The row is re-measured at this close's own tips and re-owned, which the card says is expected, "
    "and THE SENTENCE FORM IS KEPT as asked: the provenance sentence names the run that produced "
    "the figures, not a card. One thing is added rather than changed - the sentence now also says "
    "that both tips are ordinary commits of this close's own branch and therefore reachable from "
    "main once its merge lands, because the tip the row named at this base (c188d3ab) is a "
    "non-ancestor, which is INTEGRATION-TIP-CITATION-UNREACHABLE's live instance in the ledger "
    "itself.",
    [
        {"file": A, "contains": "**Measured by the wave-40 close in two detached runs at its own in-history tips, whose figures are identical**"},
        {"file": A, "contains": "the five wave-39 tips this row used to cite are not"},
    ],
)
verdicts[D + "3"] = V(
    "applied",
    "Carried forward exactly, and re-measured rather than adopted: this close's own base run gives "
    "236 targets, 238 strict summaries, 240 loose and two doubled sections, identical to both "
    "cards'. Since the card asked that a DIFFERENT measurement be reported as a finding rather than "
    "silently reconciled, the close records the converse explicitly - nothing differed, on a third "
    "tree, with the sibling's shipped script, and the agreement is stated in README section 5 with "
    "the command that produced it.",
    [{"file": A, "contains": "**two** of those sections print two summaries each"}],
)
verdicts[D + "4"] = V(
    "deferred",
    "NOT TAKEN, and the card is right that it is not close work. The proposed discharge is two more "
    "--gate keys that build_matrix.py derives from the run's own log and refuses when the row's "
    "arithmetic disagrees; that is generator code plus the mutation control that shows the refusal "
    "red, which is a tooling card's work with its own gate. GATE-ROW-SUMMARY-COUNT-UNCHECKED stays "
    "registered and is named in README section 10 as tooling-card material. What this close DID do "
    "is reduce the entry's exposure without touching its discharge: three independent measurements "
    "now support the published arithmetic, so the row is currently right even though nothing checks "
    "it.",
    [{"file": M, "contains": '"id": "GATE-ROW-SUMMARY-COUNT-UNCHECKED"'}],
)
verdicts[D + "5"] = V(
    "declined",
    "DECLINED, with its ground, and the entry left open unchanged. The card asks whether the "
    "repository map should stop carrying countable figures; the close does not restructure the map. "
    "Ground: (1) the map's figures were RE-DERIVED at this close and all are correct - "
    "contract/qcore 88 (`find contract/qcore -name '*.qcore' | wc -l`), "
    "crates/qcore-c-gateway/src 36 modules with ten named by standing constraints (counted in the "
    "row), crates/ 9 - so there is no stale figure to force the decision now; (2) which figures a "
    "navigation table should carry is a documentation-structure decision that the compaction "
    "cadence owns, and a close taking it would pre-empt DOCS-COMPACT-9 on a question its own card "
    "raised. RELATED FINDING, recorded rather than folded in: the close found a DIFFERENT stale "
    "figure of the same family - constraint 14's '112 reserved words' against the 113 its own named "
    "authority matches - and registered it separately as "
    "CONSTRAINT-14-WORD-COUNT-CASE-FILTERED, because the mechanism differs (a case-filtered "
    "extraction, not a second hand-kept copy of a derived number) while the family is the one this "
    "entry names. The new entry says so in its own `why`.",
    [{"file": M, "contains": '"id": "AGENTS-MAP-FIGURES-UNDERIVED"'}],
)
verdicts[D + "6"] = V(
    "applied",
    "GLANCED AT AS ASKED, AND MEASURED: the five stay, and the convention is now stated so the next "
    "pass does not have to guess. Measured at this close - of the five names added to constraint "
    "14's pointer list, THREE appear as card names in the constraint's own compacted body "
    "(MUTEX-SHARE-CROSS, AGGREGATE-OWNED-FIELD, CT42-CELL-ROUTE) and TWO do not "
    "(WRITE-LEND-TASKS-EXERCISE, LOOPS-ON-COMPOSED-ROUTES). The close's reading, recorded in README "
    "section 8: the pointer list is the constraint's bibliography and names every card that ran a "
    "census under it, whether or not the card reserved a word - the per-card ledger those two "
    "belong to having moved verbatim to the wave-40 checkpoint page - while the BODY names only the "
    "decisions a refactor could undo. That reading is what the constraint's own Method paragraph "
    "already warns about in the other direction (a wave-by-wave roll call in the body invites a "
    "false universal), so no name is removed.",
    [
        {"file": A, "contains": "`mutex-share-cross`, `write-lend-tasks-exercise`, `aggregate-owned-field`,"},
        {"file": A, "contains": "`ct42-cell-route`, `loops-on-composed-routes`."},
    ],
)
verdicts[D + "7"] = V(
    "applied",
    "NOT OVERRULED - the reading is confirmed and left as written. DOCS-COMPACT-8 fired in wave 40; "
    "the rule is every 2-3 waves; so the ninth pass is due around wave 42 or 43, which is the "
    "bullet's own arithmetic. The close adds nothing to it beyond recording DOCS-COMPACT-9 as due "
    "in README section 10 and in the wave-40 record, so the next close re-lists it rather than "
    "rediscovering it.",
    [{"file": C, "contains": "the\n  ninth is due around wave 42 or 43"}],
)
verdicts[D + "8"] = V(
    "applied",
    "DECISION TAKEN: KEEP THE SPLIT, and it becomes the precedent the card offers. Measured at this "
    "close - wave-40.md is 10,970 bytes and wave-40-compaction-checkpoints.md 51,145; inlining "
    "would make one 62,115-byte record page, against wave-37.md's 64,613, which is the size the "
    "card names as its reason and which AGENTS.md's own guidance ('new history pages should stay "
    "below 32 KiB where a complete record boundary permits it') already exceeds twice over. Ground "
    "beyond size: a compaction checkpoint is verbatim historical text with a different reading "
    "contract from an increment paragraph, and separating them keeps the record page a page a "
    "reader finishes. The split is therefore the convention for future compaction passes, recorded "
    "in README section 8 and in the wave-40 close paragraph.",
    [{"file": IDX, "contains": "[compaction checkpoints](wave-40-compaction-checkpoints.md)"}],
)
verdicts[D + "9"] = V(
    "applied",
    "NO COLLISION, measured rather than assumed: the two cards' registrations are disjoint sets of "
    "ids. STABILITY-REEVAL-4 recorded 23 entries and DOCS-COMPACT-8 two "
    "(GATE-ROW-SUMMARY-COUNT-UNCHECKED, AGENTS-MAP-FIGURES-UNDERIVED), counted by reading "
    "`recorded_by` off the committed matrix, and no id appears twice - the file cannot hold a "
    "duplicate id and the generator's own dimensions check would go red if the section and its "
    "count disagreed. Nothing needed merging, and this close's own two registrations were checked "
    "against both sets before being appended.",
    [
        {"file": M, "contains": '"recorded_by": "DOCS-COMPACT-8"'},
        {"file": M, "contains": '"recorded_by": "STABILITY-REEVAL-4"'},
    ],
)
verdicts[D + "10"] = V(
    "declined",
    "DECLINED to restore the count, and the card's own handling stands. The close does NOT know "
    "which wave the icebox decision landed in: the tree records it only as 'an "
    "integrator-recorded decision of 2026-09-12', and deriving a wave number from a date would be "
    "inference presented as a measurement - which is the defect this whole cadence exists to "
    "remove. The span the enumeration supports ('across waves 26-39') is what the tree actually "
    "carries and is kept. A second reason not to restore it: the figure 'the icebox has held "
    "thirteen waves' also lives inside the wave-39 close's frozen deferred-decision ground, where "
    "constraint 21 forbids editing it, so a restored count in AGENTS.md would have to agree with a "
    "frozen number nobody can re-derive either. What the close DID add is the current wave's own "
    "datum: wave 40 ran the consequence ZERO times and allocated zero codes, measured - `git diff "
    "--stat 6dc484ad 0c7b0cea -- crates/ compiler/ proof/ providers/` is empty, so neither card "
    "engages the rule at all.",
    [{"file": A, "contains": "**wave 40 added\nnothing to it**"}],
)
verdicts[D + "11"] = V(
    "applied",
    "AGREED AND DONE: LEDGER-TSAN-BOUNDARY-SENTENCES-STALE is NOT retired. Its `why` now records "
    "the entry as partially discharged, names which two parts are met and by which card, and states "
    "the third part - the two wave-39 citations of the retired TASK-ROUTE-TSAN-SEGV in crate source "
    "and in a frozen card report - as its standing discharge condition. This close did not take the "
    "third part either and says so rather than letting the entry read as untouched; see the verdict "
    "on stability-reeval-4 item 2 for the ground.",
    [{"file": M, "contains": "The THIRD part is unmet and is this entry's standing discharge"}],
)
verdicts[D + "12"] = V(
    "applied",
    "KEPT - the convention is not one link per wave. The wave-40 row names both the record page and "
    "the checkpoint page, which is the same decision as item 7's split: a reader who wants the "
    "wave's increments and a reader who wants the verbatim compacted blocks are looking for "
    "different documents, and the index is the place that distinction is cheapest to make. "
    "wave-40.md's own header also links the checkpoint page, so the two routes agree.",
    [{"file": IDX, "contains": "| 40 | [Wave 40 records](wave-40.md) · [compaction checkpoints](wave-40-compaction-checkpoints.md) |"}],
)
verdicts[D + "13"] = V(
    "noted",
    "Noted, and the mechanism is what made the close's own re-measurement worth taking: the entry "
    "now explains WHY 240 and 'four doubled sections' were both wrong (one loose regex, matching "
    "`test result` without its colon, which also catches two ordinary tests named result_..., with "
    "236 + 4 = 240 inferred to make the parenthetical self-consistent). The close adds a third "
    "agreeing measurement and takes no generator work; see the verdict on item 3 of this section "
    "and item 4 above.",
)
verdicts[D + "14"] = V(
    "applied",
    "THE READING IS THE ONE THE CLOSE WANTS, and it is checked rather than accepted: the compacted "
    "follow-on paragraph sends a reader to the increment records and each close for 'which card "
    "registered or retired which name', and W36-W37-MUTATION-HARNESSES-UNSHIPPED is named there - "
    "in docs/development/increments/wave-40.md, in STABILITY-REEVAL-4's own paragraph. So the "
    "paragraph's rule and the record agree and nothing is lost by the paragraph not naming it. The "
    "close updated the same paragraph's arithmetic for its own edits (258 -> 257, three retirements "
    "and two registrations), and the wave-40 record now carries three paragraphs rather than two.",
    [
        {"file": W40, "contains": "W36-W37-MUTATION-HARNESSES-UNSHIPPED"},
        {"file": A, "contains": "wave 40's three paragraphs carry this wave's,\nincluding its four retirements"},
    ],
)
verdicts[D + "15"] = V(
    "noted",
    "A horizontal rule closing the section, swept as an item because it is a list-level line. "
    "Nothing to apply.",
)

# ---- the two frozen wave-39 card reports, re-swept because this close appended to them
for i in range(1, 14):
    verdicts[f"ct42-cell-route#for-the-integrator#{i}"] = V(
        "noted",
        "A WAVE-39 item, re-swept only because this close appended a dated section to this frozen "
        "report (constraint 21, nothing above it edited) for "
        "INTEGRATION-GATE-NOT-PUBLISHED-FOR-EVERY-MERGE. The item was verdicted in full by the "
        "wave-39 close at the same item id (docs/reports/wave39-close/reconciliation.json); that "
        "measurement stands and this close takes none of its own. Re-verdicting it here would put "
        "this close's name on wave 39's measurement, which is the softening-while-reconciling "
        "failure CLAUDE.md's rule exists to prevent.",
    )
for i in range(1, 9):
    verdicts[f"ct07-two-thread-proof-leg#for-the-integrator#{i}"] = V(
        "noted",
        "A WAVE-39 item, re-swept only because this close appended a dated section to this frozen "
        "report (constraint 21, nothing above it edited) for "
        "INTEGRATION-GATE-NOT-PUBLISHED-FOR-EVERY-MERGE. The item was verdicted in full by the "
        "wave-39 close at the same item id (docs/reports/wave39-close/reconciliation.json); that "
        "measurement stands and this close takes none of its own. One live consequence is tracked "
        "elsewhere rather than re-opened here: this report's line 143 still cites the retired "
        "TASK-ROUTE-TSAN-SEGV, which is LEDGER-TSAN-BOUNDARY-SENTENCES-STALE's undischarged third "
        "part, recorded in that entry rather than repaired inside a wave-39 verdict.",
    )

NO_SECTION = {
    "wave39-close": (
        "The previous close's own report, swept because --since 6dc484ad starts at that close's "
        "merge and therefore includes its own branch commits. A close's report has no section "
        "addressed to a later integrator - it IS the integrator's document - and its items are its "
        "own reconciliation.json, which this close re-ran at this base and reports on in README "
        "section 3 (16 of 57 verify claims now red, 15 of them from the wave-40 compaction merge, "
        "one from the wave-opening commit; RECONCILIATION-VERIFY-CLAIM-PERISHABLE). That record is "
        "frozen and is NOT edited to make it green again - which is exactly the rule this close "
        "landed in CLAUDE.md."
    ),
    "wave40-close": (
        "This close's own report. It carries no section addressed to an integrator because it is "
        "the integrator's document; what it owes the next close is its deferred_decisions block "
        "and its registrations, both of which the sweep reads from this record and from the "
        "tracker. The wave-39 close recorded the same reason for its own directory."
    ),
}

DEFERRED = {
    "note": (
        "Every decision an earlier close left open, re-listed with a ground measured at this base - "
        "reconciliation_sweep.py --check refuses a record that drops one. Eleven were outstanding "
        "coming into this close (outstanding_decisions(root, 40)); none is resolved by a "
        "two-card cadence wave that moves no crate source, and the close says so rather than "
        "letting silence read as resolution. One NEW decision is opened, ended in the same breath "
        "because the tree resolved it."
    ),
    "items": [],
}


def DD(name, verdict, ground):
    DEFERRED["items"].append({"name": name, "verdict": verdict, "ground": ground})


DD(
    "CT10-REASON-ROW",
    "deferred",
    "Still deferred, an EIGHTH wave, and now with a second card asking for it: "
    "STABILITY-REEVAL-4's section 6 item 6 names CT24-CLEANUP-TASK-COMPOSITION and "
    "CT46-NOTES-PREDATE-TSAN-ROW as two names that reach no tracker precisely because the CT ledger "
    "row does not say where an unmapped or retired case's reason lives - which is this decision's "
    "question. NO NEW EVIDENCE ABOUT CT10 ITSELF: wave 40 moved no crate source at all (git diff "
    "--stat 6dc484ad 0c7b0cea over crates/ compiler/ proof/ providers/ is empty), so nothing "
    "touched compiler/tests/fixtures/ct/CT10/. The decision - whether to write such a row at all - "
    "is untaken and this close does not take it; what it did take is the general half, registered "
    "as RESIDUAL-QUIET-BY-PROSE-MENTION. Still double-carried as a named_followup "
    "(DECISION-ALSO-REGISTERED-AS-FOLLOWUP).",
)
DD(
    "DOCS-COMPACT-CADENCE-DOES-CORRECTION-WORK",
    "deferred",
    "Still deferred - a question raised for the user and not for a close to answer. NEW DATA POINT, "
    "offered as such and cutting BOTH ways this wave: DOCS-COMPACT-8 did fire, and it did nine "
    "corrections, which is the pattern the question is about; and this close STILL did correction "
    "work on top of it - constraint 14's reserved-word count was one short of what its own named "
    "authority matches (112 against 113), CLAUDE.md's stability-cadence bullet was a wave stale, "
    "and the merged-gate row's provenance named a non-ancestor tip. So correction work arrived from "
    "the compaction card AND from the close in the same wave. Neither reading is argued for here. "
    "DOCS-COMPACT-9 is recorded as due around wave 42 or 43 and not run.",
)
DD(
    "EXAMPLES-STYLE-SLICE-FANOUT",
    "deferred",
    "Still deferred, an EIGHTH wave, and re-listed so it does not expire in silence. Measured at "
    "this base: examples/ did not move this wave at all (git diff --name-only 6dc484ad 0c7b0cea -- "
    "examples/ is empty), and there are 12 example family directories, unchanged. A two-card "
    "documentation wave produces no evidence either way about the slice fan-out spelling of "
    "existing programs, which is what the decision is about.",
)
DD(
    "F02-WINDOWS-PIN-RUN",
    "deferred",
    "Still deferred, raised at wave 38 for the user: a user-validated Windows run of the F02 row at "
    "or after a re-pin, in the HTTP-V1-WINDOWS-RUN pattern. NEW EVIDENCE, which does not resolve "
    "it: STABILITY-REEVAL-4 re-read both pin sites this wave and reports the literals still "
    "disagree, so F02-WINDOWS-PIN-STALE-SINCE-WAVE-36 is red for a FIFTH wave; no wave-40 card "
    "claims a Windows or Apple run and neither could, both being documentation cards; this close's "
    "two gates report 10 ignored, the tenth being that #[ignore]d agreement test, the same count as "
    "wave 39's. No Linux gate can see this row, which is why the decision exists.",
)
DD(
    "INDIRECT-CENSUS-FEATURE-UNBUILT",
    "deferred",
    "Still deferred and not resolved. Measured: no wave-40 commit touched a Cargo.toml feature or a "
    "CI job - the wave's whole diff outside docs/ is empty - so nothing builds either feature "
    "still. Both arms stand as written at wave 36: a CI job building the features, or a recorded "
    "decision that the instrumentation is disposable and a future census re-derives it.",
)
DD(
    "MATRIX-RECORDED-BY-DECISION",
    "deferred",
    "Still deferred, raised at wave 38 for the user: back-fill the 67 entries without `recorded_by` "
    "with a five-key generator check, or record that `recorded_by` is optional and `source` is the "
    "provenance key. MEASURED at this close: still exactly 67 entries without the key, and every "
    "wave-40 registration carries it (23 STABILITY-REEVAL-4, 2 DOCS-COMPACT-8, 2 this close), so "
    "the unkeyed set is stable across another wave and the decision is untouched. Not decided here; "
    "no entry was back-filled by guess.",
)
DD(
    "MATRIX-UPDATED-STALE-AT-WAVE-27",
    "deferred",
    "Still deferred; untaken. NEW EVIDENCE: the matrix was regenerated at least four more times "
    "this wave (each card's own, each merge's, and this close's two), and the `updated` field still "
    "opens '2026-09-12 (wave 27 complete' - read off the committed key at this close - so the prose "
    "now sits beside a file regenerated thirteen waves later. Still double-carried as a "
    "named_followup.",
)
DD(
    "PLATFORM-RESIDUAL-FAMILY-PARTIALLY-TRACKED",
    "deferred",
    "Still deferred, and this close again did NOT resolve it by completing the registration, for "
    "the wave-35 ground: which arm to take IS the decision. NEW EVIDENCE: OWNED-FIELD-WINDOWS-RUN "
    "is still unregistered and now appears in CLAUDE.md, the wave-40 checkpoint page and "
    "DOCS-COMPACT-8's report as well as the wave-39 close - four documents, no tracker entry - and "
    "HTTP-V1-WINDOWS-RUN is still absent from named_followups by id while appearing twice in the "
    "matrix's prose, which is exactly the mechanism this close registered as "
    "RESIDUAL-QUIET-BY-PROSE-MENTION. The family is once more more partially tracked than before.",
)
DD(
    "Q-NAMESPACE-DECISION",
    "deferred",
    "OPEN, NOT RESOLVED, and RE-AFFIRMED FOR WAVE 41 BY THIS CLOSE only in the sense the wave-39 "
    "close meant it - the decision itself (open Q101+ around the legacy bands, or take a new "
    "prefix) is integrator/spec work and is not a close's to take. MEASURED for wave 40: the "
    "consequence ran ZERO times and zero codes were allocated, because neither cadence card moves "
    "crate source (git diff --stat 6dc484ad 0c7b0cea -- crates/ compiler/ proof/ providers/ is "
    "empty). The tally therefore stands at eighteen instances on ten bands across waves 26-39, and "
    "AGENTS.md now says wave 40 added nothing to it. Carried so a later close re-lists it rather "
    "than reading the AGENTS.md sentence as a resolution. The 'held for N waves' figure is NOT "
    "restored - see this record's verdict on docs-compact-8 item 9.",
)
DD(
    "SWEEP-RESIDUALS-PROSE-ONLY-ITEMS",
    "deferred",
    "Still deferred, raised at wave 37 for the user; NOT taken here, on the same ground: widening "
    "--residuals to prose needs a definition of the shape a test can make red by mutation, and none "
    "was designed or measured by this close. RELATED BUT DISTINCT, and registered separately so the "
    "two are not confused: RESIDUAL-QUIET-BY-PROSE-MENTION is the opposite direction - the tool "
    "reads TOO MUCH prose, matching a name inside another entry's `why` and calling it tracked - "
    "and it is a measured defect with a discharge condition rather than an open decision.",
)
DD(
    "SYNC-RULE-TOOLING-COMMIT-SCOPE",
    "deferred",
    "Still deferred, and wave 40 repeats the shape a FOURTH time with one new wrinkle. --uncarded "
    "returned 9 commits: six DOCS-COMPACT-8 branch commits, two WAVE-39-CLOSE branch commits "
    "reachable only through the inclusive base merge, and ONE structural integrator commit outside "
    "any merge - f11ccbba, 'Open wave 40: create the increment record and move the index pointer', "
    "the same convention as 05bbd149 (wave 38) and 477b733d (wave 37). So the answer to 'zero "
    "integrator commits outside merges' is NO this wave, and the one that exists is the conventional "
    "wave-opening commit, carried on STABILITY-REEVAL-4's branch. The wrinkle: that commit is also "
    "what broke one of the wave-39 close's verify claims, so a structural commit nobody reports is "
    "now measurably load-bearing. Everything else the integrator did this wave lives INSIDE the two "
    "merges, indexed by nothing but their messages, which is again the only reason this close could "
    "sweep it (README section 4). The question the decision asks is unchanged.",
)
DD(
    "CHECKPOINT-PAGE-SPLIT",
    "resolved",
    "OPENED AND ENDED IN THE SAME RECORD, because the tree resolved it: DOCS-COMPACT-8 section 9 "
    "item 7 and section 10.6 left the close two questions - whether compaction checkpoints belong "
    "on a separate page, and whether a wave's index row may carry two links - and both are decided "
    "here in favour of what the card did. KEEP THE SPLIT: inlining would make one 62,115-byte "
    "record page (wave-40.md 10,970 + wave-40-compaction-checkpoints.md 51,145, measured), against "
    "wave-37.md's 64,613 which is the size that occasioned the split, and a verbatim checkpoint has "
    "a different reading contract from an increment paragraph. KEEP BOTH LINKS: the index row is "
    "the cheapest place to make that distinction. Recorded as a decision rather than only as a "
    "verdict so the next compaction pass inherits it by the mechanism that re-lists decisions.",
)

record = {
    "wave": 40,
    "cards": [
        "ct07-two-thread-proof-leg",
        "ct42-cell-route",
        "docs-compact-8",
        "stability-reeval-4",
        "wave39-close",
        "wave40-close",
    ],
    "verify_applied": True,
    "note": (
        "WAVE-40-CLOSE. Base 0c7b0cea, the DOCS-COMPACT-8 merge (= origin/main at the start); a "
        "TWO-CARD CADENCE WAVE (STABILITY-REEVAL-4, DOCS-COMPACT-8), the close the only card "
        "running. 48 integrator-addressed items across four card reports, each given a verdict "
        "MEASURED AGAINST THE TREE rather than read off the card. 27 of the 48 are the wave's own "
        "two cards; the other 21 are wave-39 items on two frozen reports that re-entered the sweep "
        "only because this close appended a dated section to each (constraint 21) - those are "
        "`noted` and carry the wave-39 close's measurement, not this close's. no_section names the "
        "previous close's report and this close's own. Tracker: 258 -> 257 (three retirements, two "
        "registrations, six amendments), every retirement verified registered by id and its "
        "discharge condition read in full first. Two standing questions decided: the perishable "
        "verify claim (CLAUDE.md rule; the class measured at 16 of 57, far wider than the entry "
        "stated) and the checkpoint-page split (kept, with both index links). Gate: two detached "
        "full runs, figures identical, exits captured to files. This record's `--check` is green at "
        "this close's own tip and makes no claim about any later tree - which is the rule this "
        "close landed."
    ),
    "verdicts": verdicts,
    "no_section": NO_SECTION,
    "hand_sweep": {
        "note": (
            "What the sweep cannot reach, swept by hand and recorded so the close's coverage is not "
            "confused with the tool's. UNCARDED-BLIND-TO-MERGE-PROSE says --uncarded excludes every "
            "merge by construction; this is the first close able to act on that finding, so the "
            "wave's merge prose is swept by hand below."
        ),
        "items": [
            "--uncarded over the inclusive range returned 9 commits and every one is classified by "
            "eye, by prefix and files: SIX DOCS-COMPACT-8 branch commits (70f1cec3, 97166b99, "
            "c1eaf2a4, fa6b50c8, c97d4e5d, 55951751), TWO WAVE-39-CLOSE branch commits (11edf7eb, "
            "b1462e6d) reachable only through the inclusive base merge 6dc484ad, and ONE structural "
            "integrator commit outside any merge: f11ccbba, 'Open wave 40: create the increment "
            "record and move the index pointer', two files, carried on STABILITY-REEVAL-4's branch "
            "on the 05bbd149 (wave 38) / 477b733d (wave 37) convention. So the answer to the brief's "
            "'--uncarded should show no integrator commit outside merges' is NO, and the exception "
            "is that conventional commit - which is also the commit that broke one of the wave-39 "
            "close's verify claims. The full inclusive range 6dc484ad^..0c7b0cea is 22 commits "
            "(git rev-list --count): the two card merges, 9 card branch commits, the wave-opening "
            "commit, plus the wave-39 close merge and its 5 branch commits reachable only through "
            "it. Every one attributable.",
            "HAND SWEEP OF THE MERGE PROSE (git log --format=%B over both merges, and git show --cc "
            "for what is in neither parent). MERGE d6a57f66 (STABILITY-REEVAL-4) asserts: the "
            "four-step method over waves 38-39 with no Rust, no Lean, no semantic change and "
            "nothing promoted; 8 discharged / 2 partial / 4 open of firing #3's fourteen with none "
            "gone quiet; the correction of firing #3's own headline (mechanism 5/11 and sentence "
            "3/3, the discriminator being brief-reach); 23 findings registered; the retirement "
            "named explicitly; CT-COMPOSED-NATIVE-ROWS measured discharged and deliberately NOT "
            "retired; 'Matrix 234 -> 256'; and a per-merge 'Gate at f661b2aa (integration tip)' "
            "line. MERGE 0c7b0cea (DOCS-COMPACT-8) asserts: 56 numbered entries before and after "
            "with the reeval proved to have moved none; AGENTS.md 192,048 -> 173,072 (-9.9%); nine "
            "corrections; 47,344 bytes of checkpoints on a separate page; 'Registrations: "
            "GATE-ROW-SUMMARY-COUNT-UNCHECKED and AGENTS-MAP-FIGURES-UNDERIVED; nothing retired, no "
            "sibling entry touched; 256 -> 258'; 'Do not retire it at the close' for "
            "LEDGER-TSAN-BOUNDARY-SENTENCES-STALE; and its own 'Gate at 3cb27d80 (integration tip)' "
            "line. CHECKED BY THIS CLOSE: the arithmetic chains (234 + 23 - 1 = 256, 256 + 2 = 258, "
            "and 258 - 3 + 2 = 257 after this close) reproduce against the committed file; both "
            "merges' stated integration figures (4,727 / 141 / 303) EQUAL their cards' own "
            "gate.json figures, so neither owed an integration-gate.json under the rule this close "
            "landed; both cited integration tips are non-ancestors of main "
            "(INTEGRATION-TIP-CITATION-UNREACHABLE, two new instances); and no merge hand-edited "
            "the merged-gate row's Lean figure, the row at this base being DOCS-COMPACT-8's own "
            "measurement - which is MERGED-GATE-ROW-LEAN-HAND-EDIT's repair holding for a first "
            "wave. Nothing in either merge's prose is unattributable to a card's report or to a "
            "measurement this close could reproduce.",
            "THE INTEGRATOR'S OWN RECORD THIS WAVE, step-3 material, unsmoothed: two tooling "
            "repairs were made mid-wave and both are visible only in merge prose, not in any "
            "report. (1) The merged-gate row's Lean figure is no longer hand-edited at a merge - "
            "the merge line's own 'Gate at <tip> (integration tip)' sentence carries the Lean count "
            "as that run's measurement, and merge d6a57f66's line is the first under the new rule. "
            "Measured consequence: MERGED-GATE-ROW-LEAN-HAND-EDIT's mechanism did not recur this "
            "wave, and the entry stays open because the finding was that nothing PREVENTS it. "
            "(2) The follow-on-count checks were made whitespace-insensitive after the compaction "
            "rewrapped that paragraph - the count and the word `entries` now straddle a newline in "
            "AGENTS.md, which a literal check would have failed from here on. Both repairs are the "
            "shape SYNC-RULE-TOOLING-COMMIT-SCOPE asks about: committed-document tooling changed "
            "with no report and no residual name, indexed by nothing but a merge message. Recorded "
            "here because this is the only document that indexes them.",
        ],
    },
    "deferred_decisions": DEFERRED,
    "gate": "docs/reports/wave40-close/gate.json",
}

OUT.write_text(json.dumps(record, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
print(f"wrote {OUT}: {len(verdicts)} verdicts, {len(NO_SECTION)} no_section, "
      f"{len(DEFERRED['items'])} deferred decisions")

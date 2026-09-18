#!/usr/bin/env python3
"""WAVE-39-CLOSE: build docs/reports/wave39-close/reconciliation.json.

Every verdict below is a human judgement measured against the tree at the
close's base (`b6d1f101`, plus this close's own commits), never read off a
card's claim that something landed. The record is checked by

    python3 scripts/reconciliation_sweep.py . --since e878b362 \
        --check docs/reports/wave39-close/reconciliation.json

which verifies that every swept item carries a verdict and that every `applied`
verdict's quoted text is at the named location. The tool decides nothing.
"""
import json
import pathlib
from collections import Counter

HERE = pathlib.Path(__file__).resolve().parent.parent
MATRIX = "docs/reviews/r0_3/acceptance_matrix.json"
AGENTS = "AGENTS.md"
CLAUDE = "CLAUDE.md"
SPEC = "docs/QCORE_SPEC_r0_3.md"
REC = "docs/development/increments/wave-39.md"
INDEX = "docs/development/increments/README.md"
CT_CORPUS = "compiler/tests/ct_corpus.rs"
CT46_EXPECTED = "compiler/tests/fixtures/ct/CT46/expected.json"
MANIFEST = "docs/reevaluation/ct_manifest.json"
GATE = "docs/reports/wave39-close/gate.json"
README = "docs/reports/wave39-close/README.md"


def c(file, text):
    return {"file": file, "contains": text}


def a(file, text):
    return {"file": file, "absent": text}


V = {}


def verdict(item, kind, note, verify=None):
    entry = {"verdict": kind, "note": note}
    if verify:
        entry["verify"] = verify
    assert item not in V, item
    V[item] = entry


MERGED_MATRIX = (
    "Regenerated at the merge with the integrator's integration-gate.json rather than the card's "
    "gate.json (no card's figure could agree with a merged tree carrying sibling cards' Lean "
    "audits; the merge message says so), and regenerated again by this close with its own "
    "gate.json - the convention is verdicted in README section 4 and the borrowed Clang claim is "
    "registered as INTEGRATION-GATE-CLANG-CLAIM-BORROWED. The matrix's validation block now carries "
    "this close's gate note."
)

# ------------------------------------------------------------ ct07-two-thread-proof-leg (7)
P = "ct07-two-thread-proof-leg#for-the-integrator#"
verdict(
    P + "1",
    "applied",
    "Applied at the merge 0d9e922f: the CT row read proof 4 of 46 there and 4 of 47 after "
    "CT42-CELL-ROUTE's rebase (45a33285), with CT07 in the proof-leg sentence bound through the "
    "ledger; named_followups 228 -> 227 at that merge, re-summed at each later union and rewritten "
    "by this close as the wave's chain; Lean 137 -> 138 there (141 at this close's tip); the "
    "merged-gate row carried the card's own figure until this close replaced it under its own "
    "provenance (README section 6).",
    [
        c(AGENTS, "**proof 4 of 47**"),
        c(AGENTS, "CT07 (wave 39, `CT07-TWO-THREAD-PROOF-LEG`, the first leg bound **through the scoped-task ledger's lease set**"),
        c(AGENTS, "227 after `CT07-TWO-THREAD-PROOF-LEG` (−2 +1)"),
    ],
)
verdict(
    P + "2",
    "applied",
    "Constraint 15n's appended paragraph is in AGENTS.md in the card's words and the pointer list "
    "names the card; verified by reading the constraint at b6d1f101.",
    [
        c(AGENTS, "**Appended, card `CT07-TWO-THREAD-PROOF-LEG` (wave 39; it amends nothing above).**"),
        c(AGENTS, "`ct07-two-thread-proof-leg`."),
    ],
)
verdict(
    P + "3",
    "applied",
    "The two retirements and the registration are in the tracker (CT07-TWO-THREAD-PROOF-LEG and "
    "CT07-PROOF-LEG-PATTERN-UNAVAILABLE absent by id, CT07-PROOF-LEG-F02-PREMISE-PIN present with "
    "recorded_by the card); the count line was re-summed at each union and this close rewrote it "
    "as the wave's chain so each step is attributable.",
    [
        a(MATRIX, "\"id\": \"CT07-TWO-THREAD-PROOF-LEG\""),
        c(MATRIX, "\"id\": \"CT07-PROOF-LEG-F02-PREMISE-PIN\""),
        c(AGENTS, "227 after `CT07-TWO-THREAD-PROOF-LEG` (−2 +1)"),
    ],
)
verdict(
    P + "4",
    "applied",
    "wave-39.md exists with the card's paragraph (unioned with four siblings' at the merges, "
    "newest first) and the index's pointer and table row name wave 39.",
    [
        c(REC, "**`CT07-TWO-THREAD-PROOF-LEG` (2026-09-18).**"),
        c(INDEX, "Current record file: [wave-39.md](wave-39.md)"),
    ],
)
verdict(
    P + "5",
    "noted",
    "A statement of what the card did not do (no hand regeneration of the CT reasons map). "
    "Checked: this close ran scripts/ct_stage_summary.py --check at its tip (README section 7, "
    "exit 0) and the ct_corpus target in its gate, which are the two directions the item names.",
)
verdict(
    P + "6",
    "applied",
    "DECIDED by this close: the doc comment above ct_mapping_progress_is_pinned_and_counted no "
    "longer carries hand-kept figures. It had read 'sanitized-native evidence is 7 of 43' and "
    "'proof is 3 of 43' against a ledger at 10 of 47 and 4 of 47 - the third drift of the same "
    "sentence, which the comment itself predicted. The figures were removed rather than refreshed "
    "a fourth time; the sentence now points at the authority (PROOF_GATED_CASES, the runner "
    "registry and the manifest) and records the three drifts. A comment edit only; no test moved.",
    [
        c(CT_CORPUS, "figures were removed by the wave-39 close rather than refreshed a fourth"),
        a(CT_CORPUS, "and proof is 3 of 43"),
    ],
)
verdict(
    P + "7",
    "noted",
    "A boundary statement (no native, sanitizer, Windows or Apple observation; no conformance; no "
    "alphabet, reference_trace, checker or emitter change). Checked: the merge message and the "
    "wave-39 record paragraph carry the same boundary and this close claims nothing beyond it.",
)

# ------------------------------------------------------------ aggregate-owned-field (6)
P = "aggregate-owned-field#for-the-integrator#"
verdict(
    P + "1",
    "applied",
    "Applied at the merge e80b58d6: named_followups 227 -> 232 there (the card's own 228 -> 233 "
    "was measured on its pre-union base), Lean 139 at the tip, and the merged-gate row resolved to "
    "main's and replaced by this close. The chain step is in the count line this close wrote.",
    [c(AGENTS, "232 after `AGGREGATE-OWNED-FIELD` (−1 +6)")],
)
verdict(
    P + "2",
    "applied",
    "Constraint 15k's amendment, the constraint-14 +0 bullet, the Q-space sixteenth instance and "
    "the Q051-Q057 / Q100-Q103 rows are in AGENTS.md in the card's words; verified by reading each "
    "at b6d1f101.",
    [
        c(AGENTS, "**Amended, card `AGGREGATE-OWNED-FIELD` (wave 39; deliberate —"),
        c(AGENTS, "`AGGREGATE-OWNED-FIELD` (wave 39) **+0 words, +0 tokens, and no lexer"),
        c(AGENTS, "**Wave 39's `AGGREGATE-OWNED-FIELD` is the sixteenth instance**"),
    ],
)
verdict(
    P + "3",
    "applied",
    "Section L21's Q051-Q057 row and Q1xx row carry the wave-39 amendment.",
    [c(SPEC, "Card `AGGREGATE-OWNED-FIELD` (wave 39) grows")],
)
verdict(
    P + "4",
    "applied",
    "The card's paragraph is in wave-39.md, unioned at the merge.",
    [c(REC, "**`AGGREGATE-OWNED-FIELD` (2026-09-18).**")],
)
verdict(P + "5", "superseded", MERGED_MATRIX, [c(MATRIX, "WAVE-39-CLOSE")])
verdict(
    P + "6",
    "applied",
    "Three decisions, each taken by this close. (a) OWNED-FIELD-WINDOWS-RUN: named in CLAUDE.md's "
    "platform list on the family's existing footing (the SLICE-PERMISSIONS-WINDOWS-RUN shape) and "
    "deliberately NOT registered, because which arm the family takes is the open decision "
    "PLATFORM-RESIDUAL-FAMILY-PARTIALLY-TRACKED. (b) The Q103 sentence: kept as written - a "
    "vocabulary statement with OWNED-FIELD-SEQUENTIAL-UNREACHED naming it - and its 'not applicable "
    "when the namespace decision is taken' arm is DEFERRED with Q-NAMESPACE-DECISION, which is the "
    "decision it depends on. (c) A beside-the-archive note on AGGREGATES-V1's section 3.1 table: "
    "DECLINED - that table states a fact about f122cc03 that is still true of f122cc03; a "
    "publish-beside note is for a reading that was wrong, and constraint 15k's amendment already "
    "says where the rule moved.",
    [c(CLAUDE, "`OWNED-FIELD-WINDOWS-RUN` (wave 39, `AGGREGATE-OWNED-FIELD` §7)")],
)

# ------------------------------------------------------------ tsan-rows-2 (3 + 9)
P = "tsan-rows-2#left-registered-with-proposed-text-for-the-integrator-11#"
verdict(
    P + "1",
    "applied",
    "Applied at the merge 0150c904, which records the entry 'RETIRED on its own second arm - \"a "
    "DECISION, recorded as such, that this route's native evidence stays serial by construction\" - "
    "taken by the card in section 6 and accepted as a user decision; the card's proposed why text "
    "stands in its report as the decision's wording' (the merge message's own words). Measured: "
    "absent from named_followups by id at b6d1f101.",
    [a(MATRIX, "\"id\": \"MUTEX-SHARE-CROSS-EMISSION-NO-CONTENTION\"")],
)
verdict(
    P + "2",
    "noted",
    "Left registered, as the card proposed; its why was amended at the merge in the card's words "
    "(item 4 of the section below). Checked: present by id with the amendment text.",
    [c(MATRIX, "AMENDED by TSAN-ROWS-2 (wave 39): the second half of the discharge is met")],
)
verdict(
    P + "3",
    "noted",
    "Both entries unchanged and present by id, as the card states.",
    [
        c(MATRIX, "\"id\": \"TASK-ROUTE-TSAN-PTHREAD-BINDING\""),
        c(MATRIX, "\"id\": \"TASK-ROUTE-TSAN-WINDOWS-APPLE-RUN\""),
    ],
)
P = "tsan-rows-2#for-the-integrator#"
verdict(
    P + "1",
    "applied",
    "Applied at the merge 0150c904: the card's own 228 -> 231 became 232 -> 234 at the union "
    "(three card retirements plus the integrator's one, six registrations); the wave-39 record "
    "and the index carry the card. The chain step is in the count line this close wrote.",
    [c(AGENTS, "234 after `TSAN-ROWS-2` (−4 +6")],
)
verdict(P + "2", "superseded", MERGED_MATRIX, [c(MATRIX, "WAVE-39-CLOSE")])
verdict(
    P + "3",
    "applied",
    "The retire arm was taken at the merge 0150c904 and recorded there as the integrator's own "
    "retirement ('Item 3: MUTEX-SHARE-CROSS-EMISSION-NO-CONTENTION RETIRED on its own second arm "
    "... accepted as a user decision'). Measured absent by id. Carried in this record's "
    "deferred_decisions for accounting, in the same shape as CT07-TWO-THREAD-REPLAY-ORDER.",
    [a(MATRIX, "\"id\": \"MUTEX-SHARE-CROSS-EMISSION-NO-CONTENTION\"")],
)
verdict(
    P + "4",
    "applied",
    "The why amendment is in the entry in the card's words (ASCII hyphens, the matrix's "
    "convention), applied at the merge; verified by an id-scoped read at b6d1f101.",
    [c(MATRIX, "AMENDED by TSAN-ROWS-2 (wave 39): the second half of the discharge is met - every task-route subject runs 20 times per mutation under three named schedules")],
)
verdict(
    P + "5",
    "applied",
    "The retirement stands as the merge 0150c904 recorded it: 'Item 5: the card's retirement of "
    "CT07-TWO-THREAD-REPLAY-ORDER by statement (the deferred decision the wave-37 close left to the "
    "user) accepted as a user decision; the restated boundary is "
    "TSAN-FORCED-BODY-OVERLAP-NOT-FORCED.' Measured: absent by id; the restated boundary present. "
    "The wave-38 deferred decision of the same name is carried RESOLVED below, citing that merge.",
    [
        a(MATRIX, "\"id\": \"CT07-TWO-THREAD-REPLAY-ORDER\""),
        c(MATRIX, "\"id\": \"TSAN-FORCED-BODY-OVERLAP-NOT-FORCED\""),
    ],
)
verdict(
    P + "6",
    "applied",
    "Taken by this close: scripts/cite_ct46.py appends the two-text citation on the CT07 precedent "
    "to the manifest's CT46 semantic_profile and the fixture's notes, naming the row "
    "(ct46s_own_case_runs_clean_bound_and_the_forcing_header_finds_no_seam) and the recorded "
    "decision in the merge's words. CT46-NOTES-PREDATE-TSAN-ROW's discharge is met in its own words "
    "and the entry is retired (README section 5).",
    [
        c(CT46_EXPECTED, "TSAN CITATION (wave-39 close"),
        c(MANIFEST, "TSAN CITATION (wave-39 close"),
        a(MATRIX, "\"id\": \"CT46-NOTES-PREDATE-TSAN-ROW\""),
    ],
)
verdict(
    P + "7",
    "declined",
    "No standing constraint landed for 'a forced-schedule harness must be invisible to the race "
    "detector it serves'. Ground: there is one forcing header riding one seam, the rule is enforced "
    "by a shipped mutation that goes red by MISSING the race when the header is instrumented "
    "(mutate.sh's hook_visible_to_tsan), and TSAN-FORCED-HARNESS-SYNC-INVISIBILITY's why is indexed "
    "and carries the rule with its discharge ('keep the header uninstrumented and the reproduction "
    "mutation red whenever the header changes'). A numbered constraint restating a tracker entry "
    "for a surface with one instance is the accretion CLAUDE.md's compaction rule exists to bound; "
    "the condition for landing one is a second forcing harness or a second seam, at which point "
    "the entry's text is the constraint's, as the card said.",
)
verdict(
    P + "8",
    "applied",
    "The wave paragraph this close wrote says forcing buys executed-schedule coverage and names "
    "the two hazards, not 'forced schedules catch what unforced misses'; the card's own paragraph "
    "already said so.",
    [c(REC, "executed-schedule coverage")],
)
verdict(
    P + "9",
    "noted",
    "Checked: docs/reports/task-route-tsan/README.md carries the dated appended section and git "
    "diff over the range shows additions only below the frozen text.",
    [c("docs/reports/task-route-tsan/README.md", "## Published beside by `TSAN-ROWS-2` (2026-09-18)")],
)

# ------------------------------------------------------------ ct42-cell-route (12)
P = "ct42-cell-route#for-the-integrator#"
verdict(P + "1", "noted", "Introductory sentence; the items follow.")
verdict(
    P + "2",
    "applied",
    "The CT row reads 47/50 mapped, 3 unmapped, sanitized-native 10 of 47, proof 4 of 47 with both "
    "wave-39 sentences; verified at b6d1f101 and unchanged by this close (README section 6). No "
    "stale figure remains in AGENTS.md, README.md or the increments index (grep, README section 6).",
    [c(AGENTS, "**47/50 mapped**, 3 unmapped; sanitized-native **10 of 47**; **proof 4 of 47**")],
)
verdict(
    P + "3",
    "superseded",
    "140 was the count on the card's tip; LOOPS-ON-COMPOSED-ROUTES merged after it with "
    "AuditLoopWindows.lean, and this close's row records 141, measured on disk and carried under "
    "the lean_audits key the generator checks.",
    [c(AGENTS, "**141** Lean audits")],
)
verdict(
    P + "4",
    "applied",
    "Applied at the merge 45a33285 (234 -> 231 at the union: five retirements, two registrations, "
    "one amendment, one withdrawal); the chain step is in the count line this close wrote. "
    "Measured: the five retired names absent by id, the two registered present with recorded_by "
    "the card.",
    [
        c(AGENTS, "231 after `CT42-CELL-ROUTE` (−5 +2)"),
        a(MATRIX, "\"id\": \"CT42-CELL-UNROUTED\""),
        c(MATRIX, "\"id\": \"SOCKET-CELL-COUPLING-UNPROVED\""),
    ],
)
verdict(
    P + "5",
    "applied",
    "Constraint 15p is in AGENTS.md in the card's words; verified by reading it at b6d1f101.",
    [c(AGENTS, "15p. **A socket program's protocol cells are derived, never declared, and the")],
)
verdict(
    P + "6",
    "applied",
    "The constraint-14 +1 bullet and the amended run sentence are in AGENTS.md; this close added "
    "the wave line (README section 6).",
    [c(AGENTS, "`CT42-CELL-ROUTE` (wave 39) **+1**: `disjoint`")],
)
verdict(
    P + "7",
    "applied",
    "The three section L21 rows and the three AGENTS.md table rows carry the card's amendments; "
    "verified at b6d1f101.",
    [
        c(AGENTS, "Card `CT42-CELL-ROUTE` (wave 39) **widens `Q095`'s condition**"),
        c(SPEC, "`CT42-CELL-ROUTE` (wave 39): the disjointness question `disjoint NAME NAME;`"),
    ],
)
verdict(
    P + "8",
    "applied",
    "The card's paragraph is in wave-39.md, unioned at the merge.",
    [c(REC, "**`CT42-CELL-ROUTE` (2026-09-18).**")],
)
verdict(
    P + "9",
    "noted",
    "Pinned expectations moved deliberately by the card and verified at its rebase by ct_corpus "
    "and ct_stage_summary.py --check (its report section 7; the merge message records the union). "
    "This close's gate re-ran ct_corpus at its own tip and its --check exited 0 (README section 7).",
)
verdict(
    P + "10",
    "noted",
    "A file-ownership note; checked against the merge order - the proof-leg card's "
    "PROOF_GATED_CASES row and this card's three tables both survive in ct_corpus.rs at b6d1f101.",
)
verdict(
    P + "11",
    "noted",
    "CT-CTWIN-DESCRIPTOR is not a named_followups id; it is carried inside the roll-up entry "
    "CT-CTWIN-O's why ('int-typed resource identity refused by the contract loader'), which is why "
    "--residuals is silent about it (the whole-text scan). Left open where it is; the roll-up is "
    "its tracker and registering it separately would double-carry (README section 5).",
)
verdict(
    P + "12",
    "applied",
    "The card ran the icebox's consequence (five conditions widened, zero codes) and the Q-space "
    "note had NOT counted it: at b6d1f101 the note numbered AGGREGATE-OWNED-FIELD sixteenth and "
    "LOOPS-ON-COMPOSED-ROUTES seventeenth and did not name this card, although its own Q080 and "
    "Q090 rows below the note say 'widening three conditions' and 'widens Q095's condition'. This "
    "close counts it as the seventeenth and renumbers LOOPS the eighteenth, stating the correction "
    "in the note (Q-SPACE-EXTENSION-TALLY-HAND-KEPT's live instance; README section 6).",
    [c(AGENTS, "Wave 39's `CT42-CELL-ROUTE` is the **seventeenth**")],
)

# ------------------------------------------------------------ loops-on-composed-routes (12)
P = "loops-on-composed-routes#for-the-integrator#"
verdict(
    P + "1",
    "applied",
    "Applied at the merge b6d1f101 (231 -> 233 at the union with CT42 merged since the rebase "
    "base; Lean 141 at the tip; the gate row resolved to main's and replaced by this close).",
    [c(AGENTS, "233 after `LOOPS-ON-COMPOSED-ROUTES` (−1 +3)")],
)
verdict(
    P + "2",
    "applied",
    "Constraint 15h carries the amendment paragraph and the two superseded sentences quoted in "
    "place; verified by reading 15h at b6d1f101.",
    [
        c(AGENTS, "**Amended, card `LOOPS-ON-COMPOSED-ROUTES` (wave 39; deliberate — it"),
        c(AGENTS, "Superseded deliberately by card `LOOPS-ON-COMPOSED-ROUTES`"),
    ],
)
verdict(
    P + "3",
    "applied",
    "The +0 bullet is in the constraint-14 ledger with the census stated.",
    [c(AGENTS, "`LOOPS-ON-COMPOSED-ROUTES` (wave 39) **+0 words, +0 tokens, and the")],
)
verdict(
    P + "4",
    "superseded",
    "The card wrote 'sixteenth'; the merge renumbered it seventeenth after AGGREGATE-OWNED-FIELD "
    "(the card's own section 12 item 9 records that), and this close renumbers it EIGHTEENTH after "
    "counting CT42-CELL-ROUTE, which the note had missed (ct42-cell-route item 12 above). The "
    "note says so and names the card's report as the checkpoint that reads seventeenth.",
    [c(AGENTS, "Wave 39's `LOOPS-ON-COMPOSED-ROUTES` is therefore the **eighteenth**")],
)
verdict(
    P + "5",
    "applied",
    "The Q051, Q068 and Q077-Q079 rows of section L21 carry the card's narrowings.",
    [c(SPEC, "LOOPS-ON-COMPOSED-ROUTES` (wave 39): a `for` in an entry that lends is admitted")],
)
verdict(P + "6", "superseded", MERGED_MATRIX, [c(MATRIX, "WAVE-39-CLOSE")])
verdict(
    P + "7",
    "declined",
    "(a) Whether the composed route should accept an unrolling licence before a for: NOT taken by "
    "this close - it is a design choice for the card that lifts "
    "LOOP-ON-COMPOSED-ROUTE-BODY-VOCABULARY, where the card recorded it, and a close deciding a "
    "route's grammar without a card would be the shape invariant 6 forbids. (b) Whether "
    "SLICE-PERMISSIONS-LOOP-WINDOW's lifting should be recorded as a retired rule somewhere other "
    "than 15h: DECLINED - the constraint text is the authority for a rule and 15h quotes the "
    "superseded sentence in place with the lifting card named, which is exactly the wave-38 close's "
    "decision ('a rule ... the lifting card names it then'); a second record would be a second "
    "authority.",
)
verdict(
    P + "8",
    "applied",
    "Registered by this close as MUTATION-HARNESS-STALE-ARTIFACTS with a discharge condition, "
    "after measuring every shipped harness in the tree (seventeen: twelve rewrite, four cp from a "
    "backup, one mv plus touch - the card's own, repaired; none mtime-preserving), and the "
    "CLAUDE.md rule extended to name the mv / cp -p mechanism. MUTATION-HARNESS-MTIME-RESTORE is "
    "retired on both halves of its discharge and the family goes forward under the new entry "
    "(README section 5).",
    [
        c(MATRIX, "\"id\": \"MUTATION-HARNESS-STALE-ARTIFACTS\""),
        c(CLAUDE, "**`mv` of a `cp` backup and `cp -p` preserve the mtime the same way**"),
    ],
)
verdict(
    P + "9",
    "noted",
    "The rebase union, one line per file; the merge message records it and this close's gate "
    "re-ran the whole workspace at a tip carrying both cards' arms (README section 7). Checked that "
    "the count-line chain agrees: 231 -> 233 after the union.",
)
verdict(P + "10", "noted", "A heading (12.1) introducing the verbatim amendment; no action.")
verdict(
    P + "11",
    "applied",
    "The amendment paragraph is in 15h as quoted (verified by reading it at b6d1f101).",
    [c(AGENTS, "**Amended, card `LOOPS-ON-COMPOSED-ROUTES` (wave 39; deliberate — it")],
)
verdict(
    P + "12",
    "applied",
    "The superseded sentences are quoted in place with those words.",
    [c(AGENTS, "Superseded deliberately by card `LOOPS-ON-COMPOSED-ROUTES`")],
)

# ------------------------------------------------------------ re-swept frozen reports (57)
# --since e878b362 is inclusive of the wave-38 close merge, which APPENDED notes
# to five frozen reports (STABILITY-REEVAL-3 section 6 items 4 and 7), and
# TSAN-ROWS-2 appended a further note to task-route-tsan. Each of the 57 items
# was verdicted by the wave-36 or wave-37 close and carried `noted` by the
# wave-38 close; they are carried `noted` again, citing the ORIGINAL record,
# not re-measured.
PRIOR = {}
for wave in ("wave36-close", "wave37-close"):
    prior_record = json.loads((HERE.parent / wave / "reconciliation.json").read_text(encoding="utf-8"))
    for key, prior_verdict in prior_record["verdicts"].items():
        PRIOR.setdefault(key, (wave, prior_verdict["verdict"]))
RESWEPT = {
    "ct-lane-5": ("wave37-close", "the wave-38 close's appended note (STABILITY-REEVAL-3 section 6 items 4 and 7), inside the inclusive range"),
    "joined-provenance-round-bound": ("wave37-close", "the wave-38 close's appended note, inside the inclusive range"),
    "task-route-tsan": ("wave37-close", "the wave-38 close's appended note, inside the inclusive range, and TSAN-ROWS-2's own appended section (its item 9 above)"),
    "mutex-share-cross-emission": ("wave36-close", "the wave-38 close's appended note, inside the inclusive range"),
    "write-lend-tasks-exercise": ("wave36-close", "the wave-38 close's appended note, inside the inclusive range"),
}
for card, (expected_wave, why_swept) in RESWEPT.items():
    keys = [k for k in PRIOR if k.startswith(card + "#")]
    assert keys, card
    for key in keys:
        wave, prior_verdict = PRIOR[key]
        assert wave == expected_wave, (key, wave)
        verdict(
            key,
            "noted",
            f"Already verdicted `{prior_verdict}` by the {wave.upper()} "
            f"(docs/reports/{wave}/reconciliation.json, this key), measured against the tree at that "
            f"close, and carried `noted` by the WAVE38-CLOSE. Re-swept here because of {why_swept} "
            "(constraint 21, no archived line edited). Not re-measured by this close; the prior "
            "verdict stands.",
        )

record = {
    "wave": 39,
    "cards": [
        "ct07-two-thread-proof-leg",
        "aggregate-owned-field",
        "tsan-rows-2",
        "ct42-cell-route",
        "loops-on-composed-routes",
    ],
    "verify_applied": True,
    "note": (
        "WAVE-39-CLOSE. Base b6d1f101, the LOOPS-ON-COMPOSED-ROUTES merge (= origin/main at the "
        "start); the compiler-side wave of five cards, the only card running at the close. 49 "
        "integrator-addressed items across the five cards, each given a verdict MEASURED AGAINST THE "
        "TREE rather than read off the card's claim: applied 31 (most at their merges in the card's "
        "words, verified by reading the tree at b6d1f101; six by this close - the ct_corpus.rs doc "
        "comment, the OWNED-FIELD-WINDOWS-RUN name, the CT46 citation, the wave paragraph's "
        "executed-schedule wording, the Q-space tally correction, the harness residual), superseded "
        "5 (three integration-gate regenerations and this close's own; the Lean count; the Q-space "
        "numbering), declined 2 (a standing rule for harness invisibility; the loops card's two "
        "not-mine decisions), noted 11. Nothing was found unapplied at its merge; one thing was found "
        "UNCOUNTED at its merge (CT42-CELL-ROUTE's condition extensions missing from the Q-space tally, "
        "corrected here). A further 57 items belong to five frozen reports inside the inclusive range "
        "(ct-lane-5, joined-provenance-round-bound, task-route-tsan, mutex-share-cross-emission, "
        "write-lend-tasks-exercise), each verdicted by the wave-36 or wave-37 close and carried `noted` "
        "citing that record, not re-measured. The integrator's own record this wave is in "
        "hand_sweep: a summary-parser transcription defect in merge 0d9e922f's measured line "
        "(GATE-SUMMARY-PARSER-OK-ONLY), the deferred-regeneration convention and its three "
        "integration-gate.json files (verdicted; the borrowed Clang claim registered as "
        "INTEGRATION-GATE-CLANG-CLAIM-BORROWED), one prose union in constraint 14's ledger at "
        "b6d1f101 (verified reading correctly), two rebases ordered, one intermittent Python row at "
        "an amend (an instance of PYTHON-EVIDENCE-SUITE-INTERMITTENT-ROW, the row now named), and "
        "zero integrator commits outside merges."
    ),
    "verdicts": V,
    "no_section": {
        "ct42-protocol-cell": (
            "A frozen wave-31 card report, swept because two commits in the range appended dated "
            "'Published beside' sections to it: the wave-38 close (CT42-CELL-FALLBACK-ARM-DIRECTION, "
            "inside the inclusive range) and CT42-CELL-ROUTE at 62ff7943 (the corrected direction "
            "sentence, constraint 21, no archived line edited). Its README carries no "
            "integrator-addressed section - its follow-ons are section 6's plain named list, swept by "
            "its own wave's close - so nothing is owed here beyond the notes themselves; every "
            "CT42-CELL-* name it carries was retired or re-affirmed by CT42-CELL-ROUTE and is "
            "classified in README section 5."
        ),
        "remaining-d02": (
            "Card D02's own report directory, swept because the wave-38 close's merge e878b362 - "
            "inclusive in this range - changed docs/reports/remaining-d02/build_matrix.py (the "
            "required and checked lean_audits and clang gate keys). No wave-39 commit touched the "
            "directory (git log over e878b362..b6d1f101 is empty for it). Its README is a frozen card "
            "report from the D02 wave with no section addressed to this integrator; the generator "
            "lives beside it by history."
        ),
        "wave38-close": (
            "The previous close's own report, swept because --since e878b362 is inclusive of the "
            "wave-38 close merge and its report directory was created in that commit. It has no "
            "integrator-addressed section and must not: it is the previous integrator's record; its "
            "fifteen deferred decisions are re-listed below with grounds, two of them resolved by "
            "this wave's tree, and its two registrations and three retirements stand."
        ),
        "wave39-close": (
            "This close's own report. It has no integrator-addressed section and must not: it IS the "
            "integrator's record, and a close that addressed items to itself would be the "
            "reconciliation defect in miniature. Its findings go forward as the three registered "
            "residuals, the amended entries and the deferred decisions named in its README."
        ),
    },
    "hand_sweep": {
        "note": "What the sweep cannot reach, swept by hand and recorded so the close's coverage is not confused with the tool's.",
        "items": [
            "--uncarded over the range returned 15 commits and every one was classified by eye as a "
            "card's own branch commit, by its prefix and its files: CT07-TWO-THREAD-PROOF-LEG 3 "
            "(d13f568b, ea255a64, 81f783ce), AGGREGATE-OWNED-FIELD 5 (e60ad0c1, 2f27c652, c960a263, "
            "63bd3e85, 95460d9d), TSAN-ROWS-2 1 (170ec883), CT42-CELL-ROUTE 3 (2d1f8fa9, 7e64f3a5, "
            "ead4d5bf), LOOPS-ON-COMPOSED-ROUTES 3 (39cc9848, 6adaf1ef, 37f4219a). ZERO integrator "
            "commits outside merges this wave. The tool's inclusive range e878b362^..b6d1f101 is 44 "
            "commits (git rev-list --count): 39 after the base - the five card merges and 34 card "
            "branch commits (19 write a report and 15 do not) - plus the wave-38 close merge e878b362 "
            "and the 4 wave-38 close branch commits reachable only through it, every one "
            "attributable. The integrator's own edits live "
            "INSIDE the five merges - the matrix retirements and registrations (one the integrator's "
            "own, at 0150c904), the count lines, the regenerations with integration gates, the "
            "constraint-14 prose union at b6d1f101 - indexed by nothing but the merge messages "
            "(SYNC-RULE-TOOLING-COMMIT-SCOPE's wave-37/38 shape, a third time; restated under it below).",
            "The integrator's own record this wave, step-3 material, unsmoothed (README section 4): "
            "(1) merge 0d9e922f's measured line reads '4620 passed / 0 failed / 9 ignored' while naming "
            "three failed constraint-16 rows - the integrator's summary parser summed only `ok.` "
            "summaries; the brief to this close states the true totals as 4,641 / 3 / 9, which sum to "
            "the card's own 4,644 at 81f783ce, and the run's log is not in the tree; no ledger figure "
            "carried it (the merged-gate row at the base is the card's own) - registered as "
            "GATE-SUMMARY-PARSER-OK-ONLY; (2) three merges (e80b58d6, 0150c904, b6d1f101) could not "
            "regenerate with any card's gate.json and used a measured docs/reports/<card>/"
            "integration-gate.json, each saying so in its message and each taking its Clang claim from "
            "the card's recording beside it - the convention is ACCEPTED and written into CLAUDE.md, "
            "the three files are swept into README section 4 with their tips and figures, the generator "
            "is deliberately NOT taught a second layout, and the borrowed claim is registered as "
            "INTEGRATION-GATE-CLANG-CLAIM-BORROWED; (3) one hand-written prose union in constraint 14's "
            "ledger at b6d1f101 (CT42's and LOOPS' bullets after AGGREGATE's, CT42's re-worded run "
            "sentence kept) - verified reading correctly, and the wave line added after it; (4) the "
            "merged-gate row was resolved to main's at every merge and left naming the proof-leg card's "
            "own tip - re-measured and rewritten by this close under its own provenance; (5) CT42 was "
            "told to rebase twice and LOOPS once, each card's report recording its superseded runs "
            "(ct42-cell-route section 7: two earlier runs superseded and kept only in worktree-private "
            "directories; its session was cut once by a rate limit mid-rebase and resumed - the brief's "
            "record, the report's gate note being the citation); (6) one intermittent Python row at the "
            "TSAN-ROWS-2 amend (test_every_audit_runs_once_with_the_sequential_command, green on re-run; "
            "the brief's record, no log in the tree) - PYTHON-EVIDENCE-SUITE-INTERMITTENT-ROW's why "
            "amended to name the row.",
            "--residuals over the range returned 56 names at the base (this close's own run, kept "
            "under the session scratchpad's w39/ directory and re-run rather than copied) and 57 after "
            "the tracker edits and this report; the listings and the classification of every name are "
            "in README section 5 - card names, discharged names still written in the reports that "
            "discharged them (the MAIN-GUARD-UNACTIVATED mechanism: CT07-PROOF-LEG-PATTERN-UNAVAILABLE, "
            "CT42-CELL-UNROUTED, CT42-MAP, TSAN-SUBJECTS-ARE-TWINS-NOT-FIXTURES, "
            "WLT-EXERCISE-TSAN-TEST-SILENT-ARMS and the wave-38 close's three), rules named in "
            "constraints, verdict labels, deferred decisions, and the three names this close acted on "
            "(MUTATION-HARNESS-STALE-ARTIFACTS registered; OWNED-FIELD-WINDOWS-RUN named in CLAUDE.md's "
            "platform list on the family's footing; SOCKET-CELL-QUESTION-COMPOSED-ROUTES withdrawn by "
            "its own card before merge). Zero names were registered from any listing without a "
            "discharge condition; no third-party name was orphaned by the two retirements.",
            "TWO RETIREMENTS by this close, each named and printed in full by scripts/register.py --show "
            "and its discharge read before removal: CT46-NOTES-PREDATE-TSAN-ROW (discharge: 'a citation "
            "appended to both CT46 texts on the CT07 precedent, naming the row and the recorded decision' "
            "- done by scripts/cite_ct46.py, in those words) and MUTATION-HARNESS-MTIME-RESTORE "
            "(discharge: 'read the other harness, and land the proposed CLAUDE.md measurement-hygiene "
            "bullet' - the bullet is in CLAUDE.md, and docs/reports/c-gateway-return-call-w4/scripts/"
            "mutations.py was read by this close: it restores by write_text, a rewrite). THREE "
            "REGISTRATIONS, each with all five keys and a discharge condition: "
            "MUTATION-HARNESS-STALE-ARTIFACTS (LOOPS section 12 item 8, with the seventeen-harness "
            "survey), INTEGRATION-GATE-CLANG-CLAIM-BORROWED, GATE-SUMMARY-PARSER-OK-ONLY. ONE AMENDMENT: "
            "PYTHON-EVIDENCE-SUITE-INTERMITTENT-ROW (the row named). named_followups 233 -> 234 (+3 -2), "
            "AGENTS.md's count line rewritten as the wave's chain and kept in step.",
            "AGENTS.md's headline numbers were re-read from their sources rather than from the ledger: "
            "named_followups against the matrix file (234 after this close's edits, "
            "dimensions.named_followups derived to the same by the regeneration); the CT row against "
            "docs/reevaluation/ct_manifest.json through scripts/ct_stage_summary.py --check (47 mapped, "
            "3 unmapped, sanitized-native 10, proof 4 - no stale figure in AGENTS.md, README.md or the "
            "increments index, by grep); the contract corpus unmoved at 88 (git diff over the range is "
            "empty for contract/qcore); the corpus row unmoved (no arbiter ran); constraint 14's ledger "
            "measured against git diff over crates/qcore-frontend/src/ (six files, every one attributed "
            "to AGGREGATE, CT42 or LOOPS; lexer.rs and parser.rs unmoved; wave 39 closed at ONE reserved "
            "word, `disjoint`); the Q-space tally re-derived from the note's own enumeration plus the "
            "table rows (eighteen, not seventeen: CT42-CELL-ROUTE was uncounted - corrected in the note); "
            "the merged-gate row REPLACED by this close's own measurement at its own tip (README "
            "section 7).",
            "STABILITY-REEVAL #4 is due after this wave and DOCS-COMPACT-8 around wave 40: RECORDED as "
            "due (CLAUDE.md's cadence lines updated from '#3 is due now' and 'next due around wave 39'), "
            "NOT run. The Q-NAMESPACE-DECISION icebox held for wave 39 as re-affirmed (three condition-"
            "extension instances, zero codes); a re-affirmation for wave 40 is not this close's to make.",
        ],
    },
    "deferred_decisions": {
        "note": (
            "Fifteen decision ENTRIES were carried into this close from "
            "docs/reports/wave38-close/reconciliation.json (eleven open, three resolved or withdrawn "
            "for accounting, one - CT07-TWO-THREAD-REPLAY-ORDER - resolved by THIS wave's tree and "
            "recorded so, citing the merge that records the decision in its own words). One entry is "
            "ADDED for accounting in the same shape: MUTEX-SHARE-CROSS-EMISSION-NO-CONTENTION, never a "
            "deferred decision but retired at the same merge on its own second arm as a recorded "
            "decision. No new decision is raised for the user by this close; the decisions the cards "
            "left to the integrator were taken or declined with grounds in the verdicts above. No close "
            "may answer a decision raised for the user; each is carried until it carries resolved or "
            "withdrawn with a ground."
        ),
        "items": [
            {
                "name": "CT10-REASON-ROW",
                "verdict": "deferred",
                "ground": "Still deferred, a seventh wave. No wave-39 card touched CT10 (git diff --name-only e878b362 b6d1f101 -- compiler/tests/fixtures/ct/CT10/ is empty, measured), so this close produces NO new evidence about the row's content; the decision - whether to write such a row at all - is untaken and this close does not take it. Still double-carried as a named_followup (DECISION-ALSO-REGISTERED-AS-FOLLOWUP).",
            },
            {
                "name": "MATRIX-UPDATED-STALE-AT-WAVE-27",
                "verdict": "deferred",
                "ground": "Still deferred; untaken. NEW EVIDENCE: the matrix was regenerated at least nine times this wave (five cards' own regenerations, five merges' - three with integration gates - and this close's), and the `updated` field still opens '2026-09-12 (wave 27 complete' (measured by reading the key at b6d1f101), so the prose now sits beside a file regenerated twelve waves later. Still double-carried as a named_followup.",
            },
            {
                "name": "EXAMPLES-STYLE-SLICE-FANOUT",
                "verdict": "deferred",
                "ground": "Still deferred, a seventh wave. NEW EVIDENCE, which does not decide it: examples/ moved this wave for the first time since wave 33 - git diff --name-only e878b362 b6d1f101 -- examples/ names seven files: three new programs (examples/records/owned_field.qcore, examples/loops/lend_fill.qcore, examples/strings/looped_fill.qcore) and their four README rows; no existing example was respelled, and the decision is about the slice fan-out spelling of existing programs, which none of the three touches. Re-listed so it does not expire in silence.",
            },
            {
                "name": "SYNC-RULE-TOOLING-COMMIT-SCOPE",
                "verdict": "deferred",
                "ground": "Still deferred, and wave 39 repeats the SHAPE a third time at five times the volume: --uncarded returned 15 commits, every one a card's own; ZERO integrator commits outside merges; the integrator's edits (retirements and registrations including one of the integrator's own, five count lines, three integration-gate regenerations, one constraint-14 prose union) live INSIDE the five merges, indexed by nothing but their messages, which is again the only reason this close could verify them - and one in-merge measured line carried a transcription defect (GATE-SUMMARY-PARSER-OK-ONLY) that no card report indexes. The question the decision asks - what scope of committed-document change needs a report - is unchanged and now has a measured cost.",
            },
            {
                "name": "PLATFORM-RESIDUAL-FAMILY-PARTIALLY-TRACKED",
                "verdict": "deferred",
                "ground": "Still deferred, and this close again did NOT resolve it by completing the registration, for the wave-35 ground: which arm to take is the decision. NEW EVIDENCE: OWNED-FIELD-WINDOWS-RUN (AGGREGATE-OWNED-FIELD section 7) joins CLAUDE.md's platform list on the family's existing footing, unregistered, by this close's decision on that card's item 6; HTTP-V1-WINDOWS-RUN is still absent from named_followups (measured by id at b6d1f101); the family is once more more partially tracked than before, and the wave-39 cards' two further Linux-only sanitized examples (LOOPS) name no residual at all.",
            },
            {
                "name": "DOCS-COMPACT-CADENCE-DOES-CORRECTION-WORK",
                "verdict": "deferred",
                "ground": "Still deferred - a question raised for the user. NEW DATA POINT, offered as such: no compaction card fired in wave 39, and this close still did correction work - the Q-space note's hand-kept tally was short by one (CT42-CELL-ROUTE uncounted) and CLAUDE.md's cadence lines were stale by a wave ('#3 is due now', 'next due around wave 39') - so correction work arrived from the close again. Neither reading is argued for here. DOCS-COMPACT-8 is due around wave 40, recorded as due by this close and not run.",
            },
            {
                "name": "VERIFIER-HOST-ROW-DISPOSITION",
                "verdict": "resolved",
                "ground": "Ended at the wave-35 close; recorded so the carry is accounted. This wave's proof-leg merge saw three host rows red under four sibling builds and green in isolation (12/12 and 12/12, merge 0d9e922f), and this close's gates saw them green under full-suite load - constraint 16's recorded shape, not a reopening, and not a quiet-host measurement.",
            },
            {
                "name": "items",
                "verdict": "withdrawn",
                "ground": "Not a decision: the wave-34 reader artifact withdrawn at wave 35, carried for accounting only.",
            },
            {
                "name": "INDIRECT-CENSUS-FEATURE-UNBUILT",
                "verdict": "deferred",
                "ground": "Still deferred and not resolved: no wave-39 commit touched a Cargo.toml feature or a CI job (git diff --stat e878b362 b6d1f101 over the four Cargo.toml files and .github/ is empty, measured) and nothing builds either feature still. Both arms stand as written at wave 36: a CI job building the features, or a recorded decision that the instrumentation is disposable and a future census re-derives it.",
            },
            {
                "name": "TASK-ROUTE-TSAN-SEGV-PRIORITY",
                "verdict": "resolved",
                "ground": "Resolved by the tree at wave 37 (card TASK-ROUTE-TSAN, 3fb52b8c); recorded so the carry is accounted. TSAN-ROWS-2 built its forced rows on that binding this wave; what remains is registered, not deferred: TASK-ROUTE-TSAN-EXECUTED-SCHEDULES (amended), TASK-ROUTE-TSAN-PTHREAD-BINDING, TASK-ROUTE-TSAN-WINDOWS-APPLE-RUN, and the six TSAN-FORCED-* entries.",
            },
            {
                "name": "CT07-TWO-THREAD-REPLAY-ORDER",
                "verdict": "resolved",
                "ground": "RESOLVED by this wave's tree, and recorded here in the words of the merge that records the decision. Card TSAN-ROWS-2 retired the entry by statement on its second arm (a TSan row on CT07's own fixture, the boundary restated as TSAN-FORCED-BODY-OVERLAP-NOT-FORCED), and merge 0150c904 records: 'Item 5: the card's retirement of CT07-TWO-THREAD-REPLAY-ORDER by statement (the deferred decision the wave-37 close left to the user) accepted as a user decision; the restated boundary is TSAN-FORCED-BODY-OVERLAP-NOT-FORCED.' Measured at b6d1f101: absent from named_followups by id; TSAN-FORCED-BODY-OVERLAP-NOT-FORCED present with recorded_by TSAN-ROWS-2. The double-carry (DECISION-ALSO-REGISTERED-AS-FOLLOWUP) ends with both halves gone in one merge.",
            },
            {
                "name": "SWEEP-RESIDUALS-PROSE-ONLY-ITEMS",
                "verdict": "deferred",
                "ground": "Still deferred, raised at wave 37 for the user; NOT taken here, on the same ground: widening --residuals to prose needs a definition of the shape a test can make red by mutation, and none was designed or measured by this close. No new instance was found this wave: the one item a card named in prose as 'named here, not registered' (LOOPS section 12 item 8) spelled its name in backticks and --residuals returned it. Both arms stand as written.",
            },
            {
                "name": "Q-NAMESPACE-DECISION",
                "verdict": "deferred",
                "ground": "OPEN, NOT RESOLVED; the wave-38 re-affirmation for wave 39 HELD: the wave ran the condition-extension consequence three times (AGGREGATE-OWNED-FIELD the sixteenth with the first deliberate lift of Q055, CT42-CELL-ROUTE the seventeenth - uncounted by the note until this close - and LOOPS-ON-COMPOSED-ROUTES the eighteenth, the first whose movements are all narrowings), allocated zero codes, and the tally is re-derived in the note as eighteen on ten bands. The icebox has held thirteen waves; the namespace decision itself (open Q101+ around the legacy bands, or a new prefix) stays integrator/spec work when taken up, and a re-affirmation for wave 40 is not this close's to make. Carried so a later close re-lists it rather than reading the AGENTS.md sentence as a resolution.",
            },
            {
                "name": "F02-WINDOWS-PIN-RUN",
                "verdict": "deferred",
                "ground": "Still deferred, raised at wave 38 for the user: a user-validated Windows run of the F02 row at or after a re-pin, in the HTTP-V1-WINDOWS-RUN pattern. No wave-39 card claims a Windows or Apple run (each of the five reports says so), the two Windows literals in compiler/tests/windows_threads.rs were not moved, and compiler/tests/f02_windows_pin_agreement.rs's agreement test stays #[ignore]d and red by design (this close's gates report 9 ignored, the same count as wave 38's final run). F02-WINDOWS-PIN-STALE-SINCE-WAVE-36 names this decision and this decision names it.",
            },
            {
                "name": "MATRIX-RECORDED-BY-DECISION",
                "verdict": "deferred",
                "ground": "Still deferred, raised at wave 38 for the user: back-fill the 67 entries' recorded_by with a five-key generator check, or record that recorded_by is optional and source is the provenance key. MEASURED at b6d1f101: still exactly 67 entries without recorded_by, and every wave-39 registration (18 by the five cards, 3 by this close) carries it, so the set is stable and the decision is untouched. Not decided here; no entry was back-filled by guess.",
            },
            {
                "name": "MUTEX-SHARE-CROSS-EMISSION-NO-CONTENTION",
                "verdict": "resolved",
                "ground": "Added for accounting in the same shape as CT07-TWO-THREAD-REPLAY-ORDER: never a deferred decision (a registered followup since wave 36), retired at merge 0150c904 on its own second arm, which that merge records in these words: 'Item 3: MUTEX-SHARE-CROSS-EMISSION-NO-CONTENTION RETIRED on its own second arm - \"a DECISION, recorded as such, that this route's native evidence stays serial by construction\" - taken by the card in section 6 and accepted as a user decision; the card's proposed why text stands in its report as the decision's wording.' Measured absent by id at b6d1f101; the decision's wording is in docs/reports/tsan-rows-2/README.md section 6 and, since this close, cited in both CT46 texts.",
            },
        ],
    },
    "gate": GATE,
}

out = HERE / "reconciliation.json"
out.write_text(json.dumps(record, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
print("verdicts:", len(V), dict(Counter(v["verdict"] for v in V.values())))
print("verify claims:", sum(len(v.get("verify", [])) for v in V.values()))
print("deferred decisions:", len(record["deferred_decisions"]["items"]))
print("wrote", out)

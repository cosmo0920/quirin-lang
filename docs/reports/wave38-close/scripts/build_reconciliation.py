#!/usr/bin/env python3
"""WAVE-38-CLOSE: build docs/reports/wave38-close/reconciliation.json.

Every verdict below is a human judgement measured against the tree at the
close's base (`4f663934`, plus this close's own commits), never read off a
card's claim that something landed. The record is checked by

    python3 scripts/reconciliation_sweep.py . --since e69083a6 \
        --check docs/reports/wave38-close/reconciliation.json

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
GEN = "docs/reports/remaining-d02/build_matrix.py"
GEN_TESTS = "scripts/tests/test_build_matrix_sections.py"
GATE = "docs/reports/wave38-close/gate.json"
README = "docs/reports/wave38-close/README.md"


def c(file, text):
    return {"file": file, "contains": text}


V = {}


def verdict(item, kind, note, verify=None):
    entry = {"verdict": kind, "note": note}
    if verify:
        entry["verify"] = verify
    assert item not in V, item
    V[item] = entry


S = "stability-reeval-3#integrator-reconciliation-list#"

verdict(
    S + "1",
    "deferred",
    "F02-WINDOWS-PIN-STALE-SINCE-WAVE-36. NOT re-pinned: the item's own condition is a card that "
    "records a Windows run of windows_f02_renderings_satisfy_the_incoming_trace_relation_and_"
    "emission_pins (or states the native half was not run), and re-pinning a Windows expectation "
    "from Linux is the shape 37bc9004 was named for. Carried as the deferred decision "
    "F02-WINDOWS-PIN-RUN below, a user-validated platform run in the HTTP-V1-WINDOWS-RUN pattern. "
    "The Linux-side half the item names IS landed: compiler/tests/f02_windows_pin_agreement.rs "
    "reads both files' literals through include_str! and asserts they agree; it is RED at "
    "4f663934 by design, so it is #[ignore]d with the residual's name in its reason and a module "
    "comment saying why, and the standing gate stays green with the red reproducible on demand "
    "(--include-ignored). Measured: F0 red at the committed literals (failing test "
    "the_windows_f02_pins_agree_with_the_linux_ones), F1 green under a TEMPORARY edit of the two "
    "Windows literals to the Linux pair, windows_threads.rs restored sha256-equal "
    "(3dfabb8c...) and never committed (mutations.log). Its un-ignored companion "
    "both_f02_pin_sites_are_still_where_this_test_reads_them runs in the gate so the extraction "
    "cannot silently lose a site. The Windows literals were not changed. README section 3.",
)
verdict(
    S + "2",
    "applied",
    "Applied AT THE MERGE 4f663934 in the card's words modulo the matrix's ASCII convention (an "
    "ASCII hyphen for the card's em-dash; backticks kept on `ScopedTasks` and `Reachable`, dropped on "
    "the three code names the why already spells bare) - verified by reading the entry at the base, "
    "not the merge message. The deletion of 'unusually cheap' from THIS entry was verified by an "
    "id-scoped read (README section 2); a file-wide `absent` claim is impossible because "
    "CT07-PROOF-LEG-PATTERN-UNAVAILABLE's why quotes the phrase it retired.",
    [
        c(
            MATRIX,
            "binding CT07's checked `ScopedTasks` module's lease set to the Lean `Reachable` term of "
            "QCore.WriteLendTasks - a new binding pattern, since AdmittedCausalOrder::derive refuses "
            "this run at task_trace_relation.rs:551,563 by constraint 15n's decided boundary - with "
            "its own non-vacuity method",
        )
    ],
)
verdict(
    S + "3",
    "applied",
    "Applied AT THE MERGE 4f663934 in the card's words, with the LOCATION DEVIATION the merge "
    "recorded and this close verified: the anchor sentence ('so re-pointing a case can silently "
    "re-point another card's tests') is in the entry's TITLE, not its why, so the measurement was "
    "appended to the why before its DISCHARGE clause. No wording change; the discharge (an "
    "include_str! inventory check) stands.",
    [
        c(
            MATRIX,
            "the harness refuses a manifest re-point (ct_corpus.rs:2894-2905, :3021-3036), so the "
            "silent direction is the reverse one - the proof stage credited to a `file::test` string "
            "nobody read - now checked by "
            "proof_gated_cases_name_a_live_test_that_reads_the_cases_own_fixture",
        )
    ],
)
verdict(
    S + "4",
    "applied",
    "Three dated, labelled sections APPENDED (constraint 21: no archived line edited, each note "
    "opens 'Appended, not edited') - ct42-protocol-cell (the fallback arms at "
    "socket_footprint.rs:184,211 assign a singleton cell the unedited T09 checker judges disjoint, "
    "the admitting direction; lines re-read at 4f663934), task-route-tsan (section 4.1's 'two "
    "races' against the harness's halt_on_error=1 and races >= 1; 'CT46's emitted crossing' is "
    "fork_join_trace(), a CT46-shaped twin; DISJOINT_EXERCISE the logger twin), ct-lane-5 "
    "(section 2.4's mutations.json is the mutations key of ct_stage_evidence.json:237; measure/ is "
    "gitignored). README section 4.",
    [
        c("docs/reports/ct42-protocol-cell/README.md",
          "## Published beside by `WAVE-38-CLOSE` (2026-09-18) — `CT42-CELL-FALLBACK-ARM-DIRECTION`"),
        c("docs/reports/task-route-tsan/README.md",
          "## Published beside by `WAVE-38-CLOSE` (2026-09-18) — `TSAN-SUBJECTS-ARE-TWINS-NOT-FIXTURES`"),
        c("docs/reports/ct-lane-5/README.md",
          "## Published beside by `WAVE-38-CLOSE` (2026-09-18) — `mutations.json`, and the §2.4 harness"),
    ],
)
verdict(
    S + "5",
    "noted",
    "The card retires nothing, and the three entries it names as deliberately kept - "
    "GATE-ROW-FIGURE-ATTRIBUTION-DRIFT, F02-DIGEST-PIN-DUPLICATED, CT-FIXTURE-EXTERNAL-READERS - "
    "were each verified registered by id at 4f663934 (README section 8). This close's OWN "
    "retirements are three and are its own arithmetic, named there: WAVE-38-QUEUE-NAMES-UNTRACKED, "
    "LEAN-AUDIT-GATE-KEY-OPTIONAL, CLANG-GUARDED-GREEN-FAMILY-750; GATE-ROW-FIGURE-ATTRIBUTION-DRIFT "
    "stays PARTIAL (its why amended to say which arm is still unlanded).",
)
verdict(
    S + "6",
    "deferred",
    "MATRIX-RECORDED-BY-ABSENT-67 is a DECISION and is carried as MATRIX-RECORDED-BY-DECISION below, "
    "for the user; no entry was back-filled by guess. The cheap fact that decides it was MEASURED "
    "(private/wave38-close/recorded_by_measure.py, log in the report): of the 67 entries without "
    "recorded_by, 49 name exactly ONE docs/reports/<dir>/README.md in source and 18 name several "
    "(2 to 10 directories); 0 of the 49 directory names upper-case to a value any entry already "
    "uses as recorded_by. A back-fill is an inference for 49 and impossible by rule for 18. The "
    "entry's why now carries this measurement and names the decision, so ending either is visibly "
    "half the job (DECISION-ALSO-REGISTERED-AS-FOLLOWUP's discharge shape). README section 5.",
)
verdict(
    S + "7",
    "applied",
    "None of the four harnesses is in the tree (measure/ absent, private/ holds only this close's "
    "own files, and each report directory listed at 4f663934), so each count is RESTATED beside its "
    "report as 'measured once by the card, harness not shipped' in a dated appended section: "
    "joined-provenance-round-bound (section 7, six of seven), ct-lane-5 (section 2.4, seven of "
    "seven; the same note as item 4's), write-lend-tasks-exercise (section 8, ten of eleven with "
    "M5d's survival asserted), mutex-share-cross-emission (section 4, no script named). CLAUDE.md's "
    "card-report convention gained the one line the item asks for. README section 6.",
    [
        c(CLAUDE, "only with its script and its per-mutation log in that report directory"),
        c("docs/reports/joined-provenance-round-bound/README.md",
          "## Published beside by `WAVE-38-CLOSE` (2026-09-18) — the §7 harness"),
        c("docs/reports/write-lend-tasks-exercise/README.md",
          "## Published beside by `WAVE-38-CLOSE` (2026-09-18) — the §8 harness"),
        c("docs/reports/mutex-share-cross-emission/README.md",
          "## Published beside by `WAVE-38-CLOSE` (2026-09-18) — the §4 mutations"),
        c("docs/reports/ct-lane-5/README.md", "measured once by the card, harness not shipped"),
    ],
)
verdict(
    S + "8",
    "applied",
    "build_matrix.py now REQUIRES both checked keys in a --gate file - lean_audits (still checked "
    "against proof/Audit*.lean) and a new `clang` presence claim ('present: <banner>' or 'absent: "
    "<banner>') checked against the clang-version.txt and clang-version.exit recorded beside the "
    "gate (its directory or a gate/ subdirectory) - neither copied into the published block. The "
    "refusal lives only on the --gate regeneration path, so no frozen close's gate.json is ever "
    "read (every earlier close's lacks the keys; the generator opens none of them). Eight tests "
    "landed (27 in the module, exit 0), six arms shown RED BY MUTATION each by the test named for "
    "it, build_matrix.py restored sha256-equal (mutations.log). This close's own gate.json carries "
    "both keys with gate/clang-version.txt beside it. Discharges LEAN-AUDIT-GATE-KEY-OPTIONAL and, "
    "with RETURN-CALL-TESTS-CLANG-GUARDED-GREEN re-scoped to the family, "
    "CLANG-GUARDED-GREEN-FAMILY-750 - both retired, named in README section 8.",
    [
        c(GEN, 'CHECKED_GATE_KEYS = ["lean_audits", "clang"]'),
        c(GEN, "def clang_recording(gate_file):"),
        c(GEN_TESTS, "def test_a_gate_file_lacking_lean_audits_is_refused"),
        c(GEN_TESTS, "def test_a_gate_file_lacking_clang_is_refused"),
        c(GEN_TESTS, "def test_a_clang_claim_disagreeing_with_its_recording_is_refused"),
    ],
)
verdict(
    S + "9",
    "applied",
    "MAIN-GUARD-REFUSES-PACK-REFS's why AMENDED to name the second transaction (X->0000, the loose "
    "ref's deletion, byte-for-byte the shape test_deleting_main_is_refused protects) and the "
    "main-checkout refusal without the token; the PIN half (a committed test for both "
    "transactions, red by mutation) is card work and stays open under the already-registered "
    "MAIN-GUARD-PACK-REFS-UNPINNED, which the amendment names. Not queued for wave 39.",
    [
        c(MATRIX, "AMENDED by the wave-38 close (STABILITY-REEVAL-3 section 6 item 9, applied): the "
                  "exemption must cover BOTH transactions git pack-refs performs on refs/heads/main"),
    ],
)
verdict(
    S + "10",
    "noted",
    "Lines for the five queued cards' briefs, carried into README section 14 for the record only "
    "(the close briefs and runs none of them): CT42-CELL-ROUTE - fallback arms Err and the T09 "
    "edit first; AGGREGATE-OWNED-FIELD - an IR card first, Q055 the deliberate lift, under the "
    "re-affirmed Q-space icebox; CT07-TWO-THREAD-PROOF-LEG - ledger-side binding only; TSAN-ROWS-2 "
    "- a QCORE_TASK_CREATE harness card plus a recorded decision for CT46; LOOPS-ON-COMPOSED-ROUTES "
    "- rewrite last_use/last_view_uses over Repeat bodies, refusals kept in front, and its three "
    "names now classified: STRING-CONCAT-LOOP-WINDOW registered, SLICE-PERMISSIONS-LOOP-WINDOW a "
    "rule (CONSTRAINT-NAMED-BOUNDARIES-NOT-OPEN-WORK, the constraint text supports no other "
    "reading), WRITE-LEND-RELEND already present. WAVE-38-QUEUE-NAMES-UNTRACKED retired, verified "
    "registered first.",
)
verdict(
    S + "11",
    "noted",
    "STABILITY-REEVAL #4 is due after wave 39; the card's two named grounds - the Windows arm (this "
    "wave's one live red row, and the F02-WINDOWS-PIN-RUN decision now carrying it) and the "
    "tracker-only residuals (18 mechanism-needing entries, zero discharged in two waves) - are "
    "recorded in README section 14 for that firing's brief. Not acted on by this close.",
)

# ------------------------------------------------------------ re-swept frozen reports
# This close APPENDED a dated note to six frozen reports (items 4 and 7), so
# --since sees their directories move and sweeps their integrator sections
# again. Five of the six have such a section, and every item in it was already
# given a measured verdict by the close of the wave the card merged in. Those
# verdicts are not re-measured here: each is carried as `noted`, citing the
# prior record and its verdict, so the tool's completeness check is met by a
# human statement of WHY the item owes nothing new rather than by silence. The
# sixth, ct42-protocol-cell, has no integrator-addressed section (no_section).
PRIOR = {}
for wave in ("wave36-close", "wave37-close"):
    prior_record = json.loads(
        (HERE.parent / wave / "reconciliation.json").read_text(encoding="utf-8")
    )
    for key, prior_verdict in prior_record["verdicts"].items():
        PRIOR.setdefault(key, (wave, prior_verdict["verdict"]))
RESWEPT = {
    "ct-lane-5": "wave37-close",
    "joined-provenance-round-bound": "wave37-close",
    "task-route-tsan": "wave37-close",
    "mutex-share-cross-emission": "wave36-close",
    "write-lend-tasks-exercise": "wave36-close",
}
for card, expected_wave in RESWEPT.items():
    keys = [k for k in PRIOR if k.startswith(card + "#")]
    assert keys, card
    for key in keys:
        wave, prior_verdict = PRIOR[key]
        assert wave == expected_wave, (key, wave)
        verdict(
            key,
            "noted",
            f"Already verdicted `{prior_verdict}` by the {wave.upper()} "
            f"(docs/reports/{wave}/reconciliation.json, this key), measured against the tree at "
            "that close. Re-swept only because this close APPENDED a dated 'Published beside by "
            "WAVE-38-CLOSE' section to the card's frozen report (STABILITY-REEVAL-3 section 6 items "
            "4 and 7; constraint 21, no archived line edited). Not re-measured by this close; the "
            "prior verdict stands.",
        )

record = {
    "wave": 38,
    "cards": ["stability-reeval-3"],
    "verify_applied": True,
    "note": (
        "WAVE-38-CLOSE. Base 4f663934, the STABILITY-REEVAL-3 merge (= origin/main at the start); a "
        "one-card wave, the only card running. 11 integrator-addressed items in one card, each given "
        "a verdict MEASURED AGAINST THE TREE rather than read off the card's claim. Two of the eleven "
        "were applied at the merge itself in the card's words (items 2 and 3, item 3 with the "
        "recorded location deviation); four are applied by this close (4, 7, 8, 9); two are DEFERRED "
        "as decisions for the user with their grounds and their cheap measurements (1, 6); three are "
        "noted (5, 10, 11). Nothing was found unapplied at its merge. A further 57 items belong to "
        "five frozen reports this close only APPENDED a note to (ct-lane-5, "
        "joined-provenance-round-bound, task-route-tsan, mutex-share-cross-emission, "
        "write-lend-tasks-exercise); each was verdicted by the wave-36 or wave-37 close and is "
        "carried here as `noted` citing that record, not re-measured. The close's own findings: the "
        "fmt step of the base gate exited 1 on this close's OWN new test file (formatted, re-checked, "
        "exit 0 - the wave-37/38 'own unrepaired test' shape); a stale shared-scratchpad artifact "
        "nearly served as the --residuals before-list (caught, re-derived from this close's own run); "
        "and two retirements went quiet in --residuals for the wrong reason (cited inside other "
        "entries' why texts), recorded so no later close reads the silence as a registration."
    ),
    "verdicts": V,
    "no_section": {
        "wave37-close": (
            "The previous close's own report, swept because --since e69083a6 is inclusive of the "
            "wave-37 close merge and its report directory moved in that commit. It has no "
            "integrator-addressed section and must not: it is the previous integrator's record; its "
            "deferred decisions are re-listed below and its registered residual "
            "(REPORT-REGISTERED-CLAIM-UNCHECKED) stands."
        ),
        "wave38-close": (
            "This close's own report. It has no integrator-addressed section and must not: it IS the "
            "integrator's record, and a close that addressed items to itself would be the "
            "reconciliation defect in miniature. Its findings go forward as the deferred decisions "
            "below and the amended tracker entries named in its README."
        ),
        "ct42-protocol-cell": (
            "A frozen wave-33 card report, swept because this close APPENDED a dated 'Published "
            "beside by WAVE-38-CLOSE' section to it (STABILITY-REEVAL-3 section 6 item 4, "
            "CT42-CELL-FALLBACK-ARM-DIRECTION; constraint 21, no archived line edited). Its README "
            "carries no integrator-addressed section - its follow-ons are section 6's plain named "
            "list, swept by its own wave's close - so nothing is owed here beyond the note itself."
        ),
        "remaining-d02": (
            "Card D02's own report directory, swept because this close changed "
            "docs/reports/remaining-d02/build_matrix.py (the required and checked lean_audits and "
            "clang gate keys). Its README is a frozen card report from the D02 wave with no section "
            "addressed to this integrator; the generator lives beside it by history, and the change "
            "is this close's own, recorded in this close's report and verified by --check through "
            "item 8's claims."
        ),
    },
    "hand_sweep": {
        "note": "What the sweep cannot reach, swept by hand and recorded so the close's coverage is not confused with the tool's.",
        "items": [
            "--uncarded over the range returned 2 commits and both were classified by eye: 05bbd149 "
            "('Open wave 38: create the increment record and move the index pointer'), the card "
            "STABILITY-REEVAL-3's own wave-opening commit (its report says so, section 5); and 91e064c7, "
            "the WAVE-37-CLOSE's WIP commit, reachable only through the e69083a6 merge that opens this "
            "range inclusively. ZERO integrator commits outside merges this wave. The integrator's own "
            "edits of wave 38 live INSIDE the merge 4f663934 - the two matrix why amendments of items 2 "
            "and 3, the count line 215 -> 229 and the regeneration with the wave-37 close's gate - "
            "which is what 4f663934's body says and what --uncarded, by construction, cannot see "
            "(SYNC-RULE-TOOLING-COMMIT-SCOPE's wave-37 shape, restated under it below).",
            "The whole range e69083a6~1..4f663934 is 5 commits: 2 merges (the wave-37 close's and the "
            "card's), the card's 2 branch commits (6368cc7b the skeleton, 1052f8ce the card) and the "
            "wave-open commit 05bbd149 - every one attributable (README section 11).",
            "--residuals over the range returned 44 names before this close's tracker edits and 45 "
            "after, the two listings diffed under LC_ALL=C from this close's OWN runs (a stale 34-name "
            "file in the shared scratchpad, another session's, was nearly used as the before-list and "
            "was caught by its count). The 44 are the wave-37 close's final 43, each carried in the "
            "class that close gave it and re-read rather than copied, plus TSAN-ROWS-2, a wave-39 card "
            "name (the DOCS-COMPACT-8 future-card shape). The +1 is LEAN-AUDIT-GATE-KEY-OPTIONAL, retired "
            "here and still written in the report that discharged it (the MAIN-GUARD-UNACTIVATED "
            "mechanism). The other two retirements - WAVE-38-QUEUE-NAMES-UNTRACKED and "
            "CLANG-GUARDED-GREEN-FAMILY-750 - went quiet FOR THE WRONG REASON: each is cited inside other "
            "entries' why texts (the two registrations' and RETURN-CALL-TESTS-CLANG-GUARDED-GREEN's), and "
            "the scan reads the whole tracker text; recorded so no later close reads the drop as a "
            "registration. The final listing after the README was written is classified in README "
            "section 10, none unclassified.",
            "THREE RETIREMENTS by this close, each named and verified a registered entry before removal "
            "(register.py --show printed each entry's full text first): WAVE-38-QUEUE-NAMES-UNTRACKED "
            "(discharge: register STRING-CONCAT-LOOP-WINDOW and AGGREGATE-OWNED-FIELD with the discharge "
            "conditions above - done, in the constraints' words); LEAN-AUDIT-GATE-KEY-OPTIONAL (discharge: "
            "from the wave-38 close on a --gate file lacking lean_audits is refused, with a test red by "
            "mutation - done, M1 red); CLANG-GUARDED-GREEN-FAMILY-750 (discharge: a clang presence key "
            "checked by the generator the way lean_audits is, AND the 14-row entry re-scoped to the "
            "family - both done; RETURN-CALL-TESTS-CLANG-GUARDED-GREEN is now the family's tracker for "
            "the Clang-absence-reads-ok defect, so nothing is orphaned). TWO REGISTRATIONS: "
            "STRING-CONCAT-LOOP-WINDOW (constraint 15h) and AGGREGATE-OWNED-FIELD (constraint 15k), each "
            "with all five keys. SLICE-PERMISSIONS-LOOP-WINDOW deliberately NOT registered: 15h states it "
            "as one of 'three refusals that must survive refactors', a rule, and the lifting card names it "
            "then. named_followups 229 -> 228 (+2 -3), AGENTS.md's count line kept in step.",
            "AGENTS.md's headline numbers were re-read from their sources rather than from the ledger: "
            "named_followups against the matrix file (228 after this close's edits, "
            "dimensions.named_followups derived to the same by the regeneration), the contract corpus "
            "unmoved at 88 (no card touched contract/qcore this wave), the CT row unmoved at 46/50 with "
            "sanitized-native 10 and proof 3 (no lane ran), the corpus row unmoved (no arbiter ran), and "
            "the merged-gate row REPLACED by this close's own measurement at its own tip under its own "
            "provenance, with the Lean count and the Clang presence carried in this close's gate.json "
            "under keys the generator now REQUIRES and CHECKS.",
            "The integrator's own record this wave (README section 11) is step-3 material and is in the "
            "close unsmoothed: the edits at 4f663934 went no further than the merge script's computed "
            "resolutions and the two matrix amendments (its body says so; --uncarded agrees); the base "
            "gate's fmt exit 1 was this close's own unformatted new test file, added after the gate's "
            "start and before its fmt step - formatted with rustfmt, re-checked exit 0, both exits "
            "archived; the coordinator's mid-close addition (the Q-space icebox re-affirmation) was "
            "landed as one appended sentence in AGENTS.md's Q-space note, recorded as an "
            "integrator-recorded decision and carried open below.",
        ],
    },
    "deferred_decisions": {
        "note": (
            "Twelve decision ENTRIES were carried into this close from "
            "docs/reports/wave37-close/reconciliation.json (nine open, two resolved, one withdrawn "
            "reader artifact), restated here with grounds; NONE is resolved by the tree this wave "
            "(a one-card audit wave), and the three already ended stay recorded so the twelve-entry "
            "carry is accounted rather than silently reduced. THREE decisions are added: "
            "Q-NAMESPACE-DECISION (re-affirmed 2026-09-18 for wave 39, an integrator-recorded "
            "decision, open), F02-WINDOWS-PIN-RUN and MATRIX-RECORDED-BY-DECISION (both raised for the "
            "user by STABILITY-REEVAL-3 section 6 items 1 and 6). No close may answer a decision raised "
            "for the user; each is carried until it carries resolved or withdrawn with a ground."
        ),
        "items": [
            {
                "name": "CT10-REASON-ROW",
                "verdict": "deferred",
                "ground": "Still deferred, a sixth wave. No wave-38 card touched CT10 (the wave's one card is an audit that maps nothing), so this close produces NO new evidence about the row's content beyond wave 37's CT10-BYTE-ENCODING-UNSOUND; the decision - whether to write such a row at all - is untaken and this close does not take it. Still double-carried as a named_followup (DECISION-ALSO-REGISTERED-AS-FOLLOWUP).",
            },
            {
                "name": "MATRIX-UPDATED-STALE-AT-WAVE-27",
                "verdict": "deferred",
                "ground": "Still deferred; untaken. NEW EVIDENCE: the matrix was regenerated twice this wave (at the merge 4f663934 with the wave-37 close's gate, and by this close with its own gate carrying the two newly required keys), so the `updated` field's wave-27 prose now sits beside a file regenerated eleven waves later. Still double-carried as a named_followup.",
            },
            {
                "name": "EXAMPLES-STYLE-SLICE-FANOUT",
                "verdict": "deferred",
                "ground": "Still deferred, a sixth wave. No card of wave 38 touched examples/ (git diff --name-only e69083a6 HEAD -- examples/ is empty, measured), so this close produces NO new evidence about it. Re-listed so it does not expire in silence.",
            },
            {
                "name": "SYNC-RULE-TOOLING-COMMIT-SCOPE",
                "verdict": "deferred",
                "ground": "Still deferred, and wave 38 repeats wave 37's SHAPE exactly: --uncarded returned 2 commits, both a card's or the previous close's own; ZERO integrator commits outside merges; the integrator's edits (two matrix why amendments, a count line, a regeneration) live INSIDE 4f663934, indexed by nothing but the merge message, which is the only reason this close could verify them. The question the decision asks - what scope of committed-document change needs a report - is unchanged; a rule about the merge commit's own edits is the same decision in different clothes.",
            },
            {
                "name": "PLATFORM-RESIDUAL-FAMILY-PARTIALLY-TRACKED",
                "verdict": "deferred",
                "ground": "Still deferred, and this close again did NOT resolve it by completing the registration, for the wave-35 ground: which arm to take is the decision. NEW EVIDENCE: this close adds F02-WINDOWS-PIN-RUN to the platform-run family as a deferred decision in the HTTP-V1-WINDOWS-RUN pattern - and HTTP-V1-WINDOWS-RUN itself is still absent from named_followups (measured by id at 4f663934), so the family is once more more partially tracked than before; the CLAUDE.md platform list still carries its unregistered members.",
            },
            {
                "name": "DOCS-COMPACT-CADENCE-DOES-CORRECTION-WORK",
                "verdict": "deferred",
                "ground": "Still deferred - a question raised for the user. NEW DATA POINT, offered as such: no compaction card fired this wave, and this close still did correction work - three frozen reports' readings corrected beside them (ct42-protocol-cell's refusing/admitting direction, task-route-tsan's 'two races' and 'CT46's emitted crossing', ct-lane-5's mutations.json citation) - so correction work arrives from the STABILITY-REEVAL cadence and the close as well as from DOCS-COMPACT. Neither reading is argued for here. DOCS-COMPACT-8 is due around wave 40.",
            },
            {
                "name": "VERIFIER-HOST-ROW-DISPOSITION",
                "verdict": "resolved",
                "ground": "Ended at the wave-35 close; recorded so the twelve-entry carry is accounted. No wave-38 card touched the verifier host, and this close's gates saw the host rows green under full-suite load (gate.json), consistent with the resolution and not a quiet-host measurement.",
            },
            {
                "name": "items",
                "verdict": "withdrawn",
                "ground": "Not a decision: the wave-34 reader artifact withdrawn at wave 35, carried for accounting only.",
            },
            {
                "name": "INDIRECT-CENSUS-FEATURE-UNBUILT",
                "verdict": "deferred",
                "ground": "Still deferred and not resolved: no card of wave 38 touched either off-by-default feature (indirect-census, provenance-rounds-census) and nothing builds them still; three cards' measurement instrumentation sits behind two features no workspace member or CI job builds. Both arms stand as written at wave 36: a CI job building the features, or a recorded decision that the instrumentation is disposable and a future census re-derives it.",
            },
            {
                "name": "TASK-ROUTE-TSAN-SEGV-PRIORITY",
                "verdict": "resolved",
                "ground": "Resolved by the tree at wave 37 (card TASK-ROUTE-TSAN, 3fb52b8c); recorded so the carry is accounted. What remains is registered, not deferred: TASK-ROUTE-TSAN-EXECUTED-SCHEDULES, TASK-ROUTE-TSAN-PTHREAD-BINDING, TASK-ROUTE-TSAN-WINDOWS-APPLE-RUN, and since wave 38 TSAN-SUBJECTS-ARE-TWINS-NOT-FIXTURES.",
            },
            {
                "name": "CT07-TWO-THREAD-REPLAY-ORDER",
                "verdict": "deferred",
                "ground": "Still deferred, raised at wave 37 for the user. NEW EVIDENCE from STABILITY-REEVAL-3: CT07 has NO TSan row at all - the TSan subjects are twins, not the CT07 fixture (TSAN-SUBJECTS-ARE-TWINS-NOT-FIXTURES, published beside task-route-tsan by this close) - so the residual's own why still citing the retired TASK-ROUTE-TSAN-SEGV as the blocker is stale in the other direction; and a CT07 proof leg cannot bind through F02 at all (CT07-PROOF-LEG-PATTERN-UNAVAILABLE), so the trace-comparison sentence the decision is about would be the ONLY replay-order evidence a leg could cite. The closing-by-statement arm is still declined by this close on wave 37's ground: whoever writes the sentence is the one a later reader relies on. The work arm (a TSan row on CT07's own fixture) is TSAN-ROWS-2's material. Double-carried as a named_followup.",
            },
            {
                "name": "SWEEP-RESIDUALS-PROSE-ONLY-ITEMS",
                "verdict": "deferred",
                "ground": "Still deferred, raised at wave 37 for the user; NOT taken here, on the same ground: widening --residuals to prose needs a definition of the shape a test can make red by mutation, and none was designed or measured by this close. No new instance was found this wave; the one-card wave's report names every finding by a SCREAMING-KEBAB residual. Both arms stand as written.",
            },
            {
                "name": "Q-NAMESPACE-DECISION",
                "verdict": "deferred",
                "ground": "OPEN, RE-AFFIRMED, NOT RESOLVED. An integrator-recorded decision, landed during this close as one appended sentence in AGENTS.md's Q-space note: re-affirmed on 2026-09-18 for wave 39 - condition extensions only; no card allocates a new Q code, including the aggregate-custody card, which widens the existing StandardBuffer and Ownership bands with each widening stated in section L21. The icebox has held twelve waves; the namespace decision itself (open Q101+ around the legacy bands, or a new prefix) stays integrator/spec work when taken up. Carried here so a later close re-lists it rather than reading the AGENTS.md sentence as a resolution.",
            },
            {
                "name": "F02-WINDOWS-PIN-RUN",
                "verdict": "deferred",
                "ground": "RAISED for the user by STABILITY-REEVAL-3 section 6 item 1 and this close, in the HTTP-V1-WINDOWS-RUN pattern: a user-validated platform run. compiler/tests/windows_threads.rs:31,36 pins the fourth-move F02 digests (6df369991465940e / 0da423be2f16b8bb) where f02_trace_relation.rs:96-97 pins the fifth (bfa572914cb96413 / 9cbe67a70a420d52), so windows_f02_renderings_satisfy_the_incoming_trace_relation_and_emission_pins is red on every Windows host since fa48211a. DISCHARGE: a card that re-pins the two Windows literals AND records a Windows run of that test at or after the re-pin (or states in so many words that the native half was not run) - at which point compiler/tests/f02_windows_pin_agreement.rs's ignored test goes green, its #[ignore] comes off, and F02-WINDOWS-PIN-STALE-SINCE-WAVE-36 and F02-DIGEST-PIN-DUPLICATED retire together. This close deliberately did NOT re-pin from Linux (37bc9004's shape). The registered followup F02-WINDOWS-PIN-STALE-SINCE-WAVE-36 names this decision and this decision names it, so ending either is visibly half the job.",
            },
            {
                "name": "MATRIX-RECORDED-BY-DECISION",
                "verdict": "deferred",
                "ground": "RAISED for the user by STABILITY-REEVAL-3 section 6 item 6 (MATRIX-RECORDED-BY-ABSENT-67): either back-fill the 67 entries' recorded_by from their source reports and add a five-key generator check red by mutation, or record that recorded_by is optional and source is the provenance key with every recorded_by-keyed query restated on source. MEASURED by this close so the decision is taken on a fact: 49 of the 67 name exactly one docs/reports/<dir>/README.md in source, 18 name several (2 to 10), and 0 of the 49 directory names upper-case to a value any entry already uses as recorded_by - so a back-fill is an inference for 49 and impossible by rule for 18. Not decided here; no entry was back-filled by guess. The followup entry names this decision and this decision names it.",
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

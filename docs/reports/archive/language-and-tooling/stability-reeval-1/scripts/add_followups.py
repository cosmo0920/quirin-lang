#!/usr/bin/env python3
"""Append card STABILITY-REEVAL-1's residuals to the acceptance matrix.

`named_followups` is `hand_maintained` in the generator's `section_ownership`
(`docs/reports/remaining-d02/build_matrix.py` carries it verbatim from the
committed matrix), so this appends rather than regenerates. It then sets
`dimensions.named_followups` from the list's own length and rewrites with the
generator's byte format - `indent=1`, `ensure_ascii=False`, trailing newline
(card HYGIENE-16) - so the diff stays reviewable.

Run the generator afterwards to confirm the append is a fixed point:

    python3 -B docs/reports/remaining-d02/build_matrix.py . \
        --gate docs/reports/wave33-close/gate.json

Idempotent: an id already present is skipped.
"""

import json
import pathlib

MATRIX = pathlib.Path("docs/reviews/r0_3/acceptance_matrix.json")
REPORT = "docs/reports/stability-reeval-1/README.md"
CARD = "STABILITY-REEVAL-1"


def entry(id_, title, why):
    return {"id": id_, "title": title, "why": why, "source": REPORT, "recorded_by": CARD}


NEW = [
    entry(
        "VERIFIER-HOST-LARGE-RESPONSE-CONTROL-RED",
        "The wave-33 host repair's own admitting control is red on this host, behind a published 0 failed",
        "Card STABILITY-REEVAL-1 (wave 34) measured `closed_input_does_not_mask_descendant_timeout_or_accept_a_partial_request` "
        "FAILING deterministically 3/3 IN ISOLATION on the unmodified wave-33 close tip d3225f91, at "
        "crates/qcore-verifier-host/tests/standalone_host.rs:212 - the `assert_eq!(provider.verify(&request), Ok(response))` "
        "admitting control that the VERIFIER-HOST-CLOSED-INPUT-DEADLINE card itself ADDED - returning Err(Timeout). The repair is "
        "present, not missing: 4aa24e28 is an ancestor of HEAD and its progress-gated backoff stands at lib.rs:1705,1757-1758. "
        "Raising manifest.limits.timeout_ms from 1000 to 8000 makes it PASS, so this is a wall-clock throughput shortfall (about "
        "8 MiB in and 8 MiB out through a pipe inside 1,000 ms), not a hang. Independently corroborated by "
        "docs/plans/fluent-bit-system-coverage/README.md:244-254, which records the same assertion failing including on an isolated "
        "rerun and states it is not a flake. Against that, AGENTS.md's gate row publishes 4,544 passed / 0 failed and calls the "
        "defect 'repaired', and wave33-close/gate.json says all binaries green. The repair is sound and the assertion was verified "
        "NOT weakened; the finding is that 'repaired' is an OBSERVATION ON ONE MACHINE PRESENTED AS A PROPERTY, and this test is not "
        "among constraint 16's five load-sensitive rows, so nothing records an environment boundary for it. Discharge, one of: (a) "
        "record the row beside constraint 16's five with its measured host condition, which c-gateway-return-position's report "
        "already requested in those words; (b) make the control's budget a function of measured pipe throughput rather than a fixed "
        "1,000 ms; (c) an evidenced host investigation. NOT a discharge: shrinking the response, raising the deadline, or deleting "
        "the control (invariant 1).",
    ),
    entry(
        "MUTEX-SHARE-CROSS-UNEXERCISED-ARMS",
        "A card asserting it has no unexercised arms carries four, two of them provably dead",
        "docs/reports/mutex-share-cross/README.md:186, under a section headed 'the STABILITY-REEVAL discipline', states 'Every "
        "refusal this card adds is driven by a named test; there are no unexercised arms in the card's own rules.' Card "
        "STABILITY-REEVAL-1 (wave 34) measured four further refusal arms in `lower_declared_share` "
        "(crates/qcore-frontend/src/source_mutex.rs) that the card's §6 table does not list and no test drives. TWO ARE PROVABLY "
        "UNREACHABLE FROM ANY PROGRAM: (1) the `Ok(other)` arm at :1325-1336, because for TransferMode::ShareUnderMutex the judgment "
        "at crates/qcore-core/src/transfer.rs:634-635 returns only Ok(ShareUnderMutex(CheckedMutex)), which the PRECEDING arm "
        "matches, and transfer.rs:762 states that pair is the only one in the matrix yielding the mutex protocol; (2) the "
        "`state.resource_of.get(...) else` arm at :1348-1356, because `place` is ProtectedResource iff "
        "`state.resource_of.contains_key(resource_name)` (:1313-1319) and only ProtectedResource reaches that point - same map, same "
        "key. The other two (:1292-1296, :1298-1302, non-Word tokens at the payload and mutex positions) are reachable in principle "
        "but undriven; the three shapes in `a_malformed_crossing_clause_names_the_form` are all length-6 and stop at the length arm, "
        "and each refusal message occurs exactly once in the tree - its own definition. Dead fail-closed backstops are defensible; "
        "the sentence asserting they do not exist is not. Discharge: amend the card's §6 to list the four with their status, giving "
        "the two dead ones the RETURN-POSITION-ESCAPE-UNREACHABLE treatment (a backstop justified by an argument, never by the green "
        "suite); or drive the two reachable token arms with a test. NOT a discharge: deleting the arms - they fail closed, and "
        "FB-SYSTEM-EXTENSIONS makes both dead arms reachable.",
    ),
    entry(
        "LEASE-BOUNDARY-MUTATION-COUNT-OVERSTATED",
        "Constraint 15n says five of five tests were shown non-vacuous; the measurement is three of five",
        "AGENTS.md constraint 15n states that 'every one of its five tests was shown non-vacuous by mutating the arm'. Card "
        "STABILITY-REEVAL-1 (wave 34) re-derived all three mutations the card names against "
        "crates/qcore-core/src/task_trace_relation.rs:548-550 and measured 3 OF 5 red, not 5 of 5: silent-drop (`.filter` before "
        "`.enumerate()`) 3/5, admitting mis-encode as Loaned/Returned 3/5, wrong index 2/5; the union of arm-sensitive tests is 3. "
        "The two never-red tests are STRUCTURALLY incapable of falsification by an arm mutation rather than merely uncovered: "
        "`dropping_the_lease_events_would_have_produced_an_admitted_order` constructs the dropped encoding itself and calls "
        "reference_trace on an already-lease-free observation, and `the_rendered_lease_lines_are_outside_the_parsers_alphabet_too` "
        "exercises parse_observed_trace, a different route whose own doc says so. The card's own report is honest - it publishes the "
        "three counts - so this is the ledger's paraphrase inflating them. The discharge claim itself is NOT challenged: the arm is "
        "genuinely driven (see the report's §2.3). Discharge: correct 15n to 'three of its five tests, the other two exercising "
        "neighbouring routes by construction'.",
    ),
    entry(
        "LEASE-BOUNDARY-MUTATION-EVIDENCE-UNSHIPPED",
        "A non-vacuity claim whose mutation evidence is counts only, with no script, log or per-test names",
        "docs/reports/trace-alphabet-lease-boundary/ contains ONLY README.md - no logs/, no scripts/, no transcript - and its §3.1 "
        "records the three mutation results as a bare three-row count table with no per-test names. Card STABILITY-REEVAL-1 (wave 34) "
        "confirmed the counts reproduce, but only by re-deriving all three mutations by hand; nothing in the tree lets a later reader "
        "check the claim. The sibling card c-gateway-return-position ships logs/mutations.txt and scripts/mutations.sh naming every "
        "red test, which is the standard this one should meet. This matters more than it looks because of the measured boundary in "
        "the same report's §8.2: a card report's log citations are in general not checkable after the merge, which is the mechanism "
        "that let wave 33's six nonexistent logs stand. A committed mutation script is the cheap repair for that whole class. "
        "Discharge: ship the mutation script and log for this card's five tests, as the sibling card does.",
    ),
    entry(
        "AUDITALL-TRIPWIRE-CONTROLS-UNCOMMITTED",
        "The Lean audit's empty-count and warning tripwires were validated by uncommitted manual injections",
        "AuditAll.lean's `statements == 0` and `checked == 0` tripwires and lakefile.toml's `warningAsError = true` were validated at "
        "the Lean proof-integrity card by MANUAL, UNCOMMITTED injections - the wave-33 record says 'injected bad statements and a "
        "deprecated-lemma build fail as intended'. Card STABILITY-REEVAL-1 (wave 34) found nothing in the tree that re-runs them, so "
        "a refactor that silently zeroed the audit count would pass the gate: the tripwire that guards the 128-audit figure is itself "
        "unguarded. This is a green-suite-is-not-evidence case of exactly the cadence's step 1. Note the same file's statement "
        "detector DOES carry four committed elaborated controls, so the pattern is established and simply was not applied here. "
        "Discharge: commit the injections as controls the way the statement detector's own run_elab control is committed, or record "
        "them explicitly as a measured one-time observation rather than a standing gate.",
    ),
    entry(
        "CT-COMPOSED-NATIVE-NON-LINUX-VACUOUS-PASS",
        "The composed-epoch native runner passes green on Windows while asserting nothing",
        "`run_composed_native` in compiler/tests/ct_corpus.rs early-returns on non-Linux/macOS hosts, printing an eprintln, and the "
        "test still PASSES. Card STABILITY-REEVAL-1 (wave 34) records this as a disclosure gap rather than a defect: the restriction "
        "is documented (evidence_target=linux) and deliberate. But the ledger's CT row publishes 'sanitized-native 9 of 45' with NO "
        "platform qualifier, so a green suite on Windows reads as evidence for the CT22/CT25 sanitized rows when it is evidence for "
        "nothing. The cadence's rule is that an arm no test reaches is not verified by a green suite; here the suite is green "
        "precisely where it reaches nothing. Discharge: state the platform boundary in the CT ledger row beside the sanitized-native "
        "count, so the number carries the host condition it was measured on.",
    ),
    entry(
        "CT-SHARED-FIXTURE-DISCLOSURE",
        "CT01, CT12 and CT14 are one byte-identical fixture, and the CT denominator does not say so",
        "Named by card PROOF-LEG-BATCH (wave 32) and, until card STABILITY-REEVAL-1 (wave 34), reaching NO tracker - absent from "
        "AGENTS.md, CLAUDE.md and named_followups. Verified by sha256sum: compiler/tests/fixtures/ct/{CT01,CT12,CT14}/case.qcore all "
        "hash 131f635d9579a7da9833a7e51f305a5aa3306b2542808c17558ced982f5e909c. Across the corpus the 45 mapped cases rest on 43 "
        "DISTINCT ARTIFACTS (40 distinct .qcore programs plus 3 legacy_c C twins). The proof NUMERATOR is honest - CT01 "
        "(131f635d...), CT03 (7c75577b...) and CT37 (890d275e...) are pairwise distinct programs - and PROOF-LEG-BATCH behaved "
        "correctly, measuring this and WITHDRAWING two proof rows rather than banking them. What is understated is the denominator's "
        "independence, and ct_manifest.json compounds it by recording three distinct `source_fixture` PATHS, so path-identity stands "
        "in for program-identity in a schema whose own rule is that an executed entry names the case's own fixture. Discharge: one "
        "clause in the CT ledger row giving the artifact-vs-case count, or a content digest beside `source_fixture` so the manifest "
        "cannot present one program as three.",
    ),
    entry(
        "GATEWAY-LEAN-ABSTRACT-NOT-EXTRACTION",
        "The gateway's Lean modules are abstract rule theorems, and neither authoritative file says so",
        "docs/reports/c-gateway-wave/README.md states it plainly - the gateway's Lean modules are 'abstract rule theorems, not "
        "extraction of the Rust analyzer or whole-C correctness' - and every gateway-backing module repeats the disclaimer in its own "
        "docstring. Card STABILITY-REEVAL-1 (wave 34) surveyed proof/ and found NO extraction, refinement or simulation theorem "
        "anywhere, and found that grep -niE 'not extraction|abstract (rule|model)|faithful extraction' over AGENTS.md and CLAUDE.md "
        "returns NOTHING. The sentences a careful reader may take as the stronger claim are in the files agents are told are "
        "authoritative: the repository map's 'proof/ | Lean 4 executable core and safety theorems'; invariant 6's 'Lean parity', "
        "naming a correspondence nowhere proved; and constraint 4's 'its soundness is an induction on execution prefixes', where "
        "'its' reads as legacy_neutral.rs's. Most tellingly, constraint 28 - headed 'Trust boundaries that are true statements, not "
        "TODOs' - enumerates the premises and OMITS the two largest in the gateway. Discharge: add faithful C token/type extraction "
        "and the Lean-model-to-Rust correspondence to constraint 28's list, and qualify the repository-map row. Documentation only; "
        "moves no count and no verdict.",
    ),
    entry(
        "RETURN-POSITION-SOUNDNESS-PREMISE-ELIDED",
        "Constraint 5b presents a conditional theorem as covering the premise it is conditional on",
        "Constraint 5b says it is 'sound to answer None because `admitted_chain` refuses a bound base while every tracked -> place is "
        "rooted at a body-acquired name, which `may_bind` reports bound by construction - proved as "
        "QCore.ReturnPositionEscape.an_admitted_chain_names_no_tracked_place'. Card STABILITY-REEVAL-1 (wave 34) read the theorem at "
        "proof/QCore/ReturnPositionEscape.lean:84-93: it takes `(inv : TrackedBasesAreBound tracked)` as an EXPLICIT HYPOTHESIS, and "
        "the module states at line 34 'A premise, stated and not proved here: TrackedBasesAreBound'. The constraint joins the premise "
        "and the theorem in one sentence with 'because ... proved as', so the may_bind half - the load-bearing half - reads as "
        "covered by the proof. The Lean file is more honest than the ledger. This is not academic: FB-BACKLOG-CUSTODY's member-typing "
        "widening is the one move that can put a projection-bound place on an admitted chain, which is exactly the premise failing. "
        "Discharge: amend 5b to say the theorem is CONDITIONAL ON TrackedBasesAreBound, in the Lean module's own words.",
    ),
    entry(
        "LEASE-BOUNDARY-TRANSCRIPTION-PREMISE",
        "15n's index claim is proved of a hand-transcribed Lean list, and the constraint omits the transcription premise",
        "Card STABILITY-REEVAL-1 (wave 34) verified that all five theorems constraint 15n names exist in "
        "proof/QCore/TaskTraceLeaseBoundary.lean. But 15n's 'so the Rust pin index: 2 is an instance rather than a coincidence' is "
        "proved of `flagshipRun`, a HAND-TRANSCRIBED Lean list whose docstring says it was 'transcribed from the reference trace "
        "compiler/tests/write_lend_tasks.rs pins' and whose module adds that it says nothing 'about the Rust implementation's "
        "faithfulness beyond the transcription'. The module discloses the premise; the constraint does not. Discharge: carry the "
        "module's own transcription sentence into 15n, or bind the Lean term to the Rust pin by a generated digest the way CT01's "
        "proof leg binds its trace.",
    ),
    entry(
        "GATEWAY-CONSERVATIVE-UNATTACHED",
        "The project description's 'conservative checker' claim carries no theorem and no pointer",
        "Card STABILITY-REEVAL-1 (wave 34) swept every theorem-shaped identifier in AGENTS.md and CLAUDE.md (18 of them) against "
        "proof/ and found EVERY ONE resolves to a real theorem - a genuine positive result. The one unattached claim word is "
        "AGENTS.md's 'It also ships a conservative checker for existing C source (the unrestricted-C gateway)', which carries no "
        "theorem and no pointer. Conservativity of the gateway AS IMPLEMENTED is precisely what GATEWAY-LEAN-ABSTRACT-NOT-EXTRACTION "
        "shows is proved nowhere: the Lean modules prove rule-level properties of abstract models under a faithful-extraction "
        "premise. Every other use of 'conservative' in the file is attached to a named theorem. Discharge: point the phrase at the "
        "rule-level theorems with the abstract-model qualifier, or restate it as a design intent rather than a property.",
    ),
    entry(
        "LEDGER-CT-ROW-LEASE-ARM-STALE",
        "AGENTS.md's CT row contradicts constraint 15n in the same file, on the cadence's own occasioning case",
        "AGENTS.md line 131 (the CT01-CT50 ledger row) still says of the F02 lease arm that it 'is observed and not proved: no test "
        "drives a lease-carrying reference run into it, so the green suites are evidence about the paths they took "
        "(TRACE-ALPHABET-LEASE-REFUSAL-UNEXERCISED, the occasioning case for CLAUDE.md's STABILITY-REEVAL cadence)'. Constraint 15n, "
        "IN THE SAME FILE, says that card TRACE-ALPHABET-LEASE-BOUNDARY (wave 32) discharged the residual, that 'the arm is reachable "
        "today', and that compiler/tests/trace_alphabet_lease_boundary.rs drives it. Card STABILITY-REEVAL-1 (wave 34) MEASURED the "
        "arm genuinely driven, so line 131 is not merely stale but false. The matrix is correct: there is no live named_followups "
        "entry for the old id and its successor TRACE-ALPHABET-LEASE-GROWTH-UNDECIDED records the discharge. So this is a stale "
        "LEDGER SENTENCE, and its cause is measured: wave 32's close wrote no reconciliation.json and never swept "
        "TRACE-ALPHABET-LEASE-BOUNDARY, whose report §5.1 is headed 'fully discharged, retire it' and addressed to the integrator. "
        "That the cadence's own occasioning case is still mis-stated two closes later is the clearest argument that the cadence is "
        "load-bearing. Discharge: replace the clause with 15n's measured statement.",
    ),
    entry(
        "WAVE-32-CLOSE-UNSWEPT",
        "Wave 32 instituted the reconciliation record and did not write one; six of its seven cards were never swept",
        "Measured by ls in card STABILITY-REEVAL-1 (wave 34): docs/reports/wave32-close/ contains ONLY gate.json - no README.md, no "
        "reconciliation.json - while docs/reports/wave33-close/ contains all three. Wave 32 is the wave that LANDED the rule "
        "(W32-RECONCILE-SWEEP, after which CLAUDE.md reads 'The close WRITES docs/reports/wave<N>-close/reconciliation.json beside "
        "its gate.json'), so the mechanism was instituted and skipped in the same wave. Its sweep record "
        "w32-reconcile-sweep/reconciliation-waves-29-31.json covers 20 cards of waves 29-31; of wave 32's own seven merged cards "
        "(W32-RECONCILE-SWEEP, CT-WIRE-LANE-4, MUTEX-SHARE-CROSS, TRACE-ALPHABET-LEASE-BOUNDARY, PROOF-LEG-BATCH, "
        "CT-COMPOSED-NATIVE, DOCS-COMPACT-5) only docs-compact-5 appears - SIX OF SEVEN UNSWEPT. This is the measured cause of three "
        "separate findings on this card: LEDGER-CT-ROW-LEASE-ARM-STALE, LEASE-BOUNDARY-MUTATION-COUNT-OVERSTATED and "
        "MUTEX-SHARE-CROSS-UNEXERCISED-ARMS all trace to unswept integrator sections of those cards. Discharge: sweep wave 32's six "
        "unswept cards retrospectively and write docs/reports/wave32-close/reconciliation.json, as W32-RECONCILE-SWEEP itself did for "
        "waves 29-31. The tool decides nothing; a person must measure each item against the tree.",
    ),
    entry(
        "SWEEP-RESIDUALS-SCANS-REPORTS-ONLY",
        "The residual sweep reads only docs/reports, so a residual named anywhere else is structurally invisible",
        "scripts/reconciliation_sweep.py:288 discovers cards with `git diff --name-only ref HEAD -- docs/reports` and filters "
        "`line.startswith('docs/reports/')`, so --residuals can only ever see residual names introduced under docs/reports/. A "
        "residual introduced in docs/plans/, docs/development/increments/, docs/cycles/ or a source comment is invisible BY "
        "CONSTRUCTION. Card STABILITY-REEVAL-1 (wave 34) ran the tool since the wave-32 close: it reported 5 untracked names and "
        "MISSED two the card found by grep - CT-SHARED-FIXTURE-DISCLOSURE and, structurally, FB-PLAN-GATE-HOST-TIMEOUT, which lives "
        "in docs/plans/fluent-bit-system-coverage/README.md and is the name attached to this wave's one live test failure. So the "
        "wave-33 close ran the tool, got an answer, and the wave's most consequential residual was not in it. Discharge: widen "
        "--residuals' scan to the other documentation roots that can introduce a residual name, keeping the tool's 'candidates for a "
        "human eye, never a verdict' contract. Bounded and mechanical; it does not touch the --check path.",
    ),
    entry(
        "WAVE-BASE-UNSWEPT",
        "A commit that IS the sweep's --since base falls in a seam both the sweep and the increment records skip",
        "The close's sweep range is exclusive of its --since base, so a commit that is itself the base is invisible to the sweep. "
        "Card STABILITY-REEVAL-1 (wave 34) found a live instance: 37bc9004 'Fix Windows coherence gates' is the declared wave-33 base "
        "(docs/reports/wave33-close/README.md:30) AND lands after the wave-32 close commit, so wave-32.md never describes it, "
        "wave-33.md never describes it, the wave-33 sweep's exclusive range skips it, and no docs/reports entry indexes it. It is not "
        "a harmless commit (see WINDOWS-COHERENCE-GATES-UNCARDED: it moved two pinned F02 digests and removed four tests from the "
        "Windows provider gate). The structural point is that this seam RECURS EVERY WAVE and nothing currently closes it. Discharge: "
        "make the close's sweep range inclusive of its base commit, or require the declared base to be the previous close's merge "
        "commit so that nothing can land in the seam between two waves' records.",
    ),
    entry(
        "WINDOWS-COHERENCE-GATES-UNCARDED",
        "An uncarded, unindexed commit moved two pinned expectations and disabled four tests on Windows",
        "Commit 37bc9004 'Fix Windows coherence gates' appears in NO increment record, has NO docs/reports/<card>/ entry, and is in "
        "NO reconciliation record; its message is one line. Card STABILITY-REEVAL-1 (wave 34) separated its parts and EXONERATES the "
        "digest move on the merits: the two pins it changes in compiler/tests/windows_threads.rs:31,36 (6524b41d0ccb910f -> "
        "6df369991465940e and 0719a8dcb0192e7e -> 0da423be2f16b8bb) are exactly the values card WRITE-LEND-TASKS (wave 31) re-pinned "
        "deliberately (write-lend-tasks/README.md:246-247); that card moved the Linux copy in f02_trace_relation.rs:79-80 and MISSED "
        "the Windows-only twin, which #![cfg(target_os = \"windows\")] keeps out of every Linux gate. So this brings a stale duplicate "
        "into agreement - a repair, not a new pin. What stands as the finding: a pinned expectation moved in a commit whose message "
        "does not say so and which no report indexes, against CLAUDE.md's 'update the pin deliberately and say so in the commit "
        "message and report' (one of the two is CT01's digest, and CT01 is proof-gated); and the commit separately adds #![cfg(unix)] "
        "to three providers/conformance/tests/ files, removing four test functions from the Windows provider gate, where the three "
        "shared fixtures use no unix-only API so it is a behavioural exclusion rather than a compile fix. Discharge: a retrospective "
        "record for 37bc9004 stating the re-pin deliberately, as CARD-SCOPE-SELF-MERGE was retro-promoted. The code is not "
        "challenged.",
    ),
    entry(
        "F02-DIGEST-PIN-DUPLICATED",
        "Two F02 emission digests are bare literals in two files with no shared source and no cross-check",
        "The same two F02 emitted-C digests are pinned independently in compiler/tests/f02_trace_relation.rs:79-80 (Linux-reachable) "
        "and compiler/tests/windows_threads.rs:31,36 (behind #![cfg(target_os = \"windows\")]). Card STABILITY-REEVAL-1 (wave 34) "
        "measured the consequence: when card WRITE-LEND-TASKS (wave 31) re-pinned the pair deliberately it moved only the Linux copy, "
        "and the Windows twin DRIFTED FOR AN ENTIRE WAVE with no gate able to see it, until an uncarded commit repaired it "
        "(WINDOWS-COHERENCE-GATES-UNCARDED). The windows_threads.rs comment already asks the reader to 'Keep these pins paired with "
        "f02_trace_relation.rs', which is a convention where a mechanism is needed - and the same drift will recur at the next "
        "emission change. Discharge: give both files one source for the two constants, or add a host-unconditional test asserting the "
        "two pin sites agree, so a divergence is caught on the platform that runs CI rather than on the one that does not.",
    ),
    entry(
        "PROVIDER-CONFORMANCE-UNIX-ONLY",
        "The carried 122 provider figure became a Unix measurement with no platform qualifier",
        "Commit 37bc9004 added #![cfg(unix)] to providers/conformance/tests/{counter_effect,source_effect,teardown_capability}.rs, so "
        "on Windows those three binaries now run zero tests - four test functions removed from that platform's provider gate. Card "
        "STABILITY-REEVAL-1 (wave 34) verified the three shared fixtures "
        "(crates/qcore-verification/tests/fixtures/{counter,boolean,teardown}_source.rs) use no unix-only API, so this is a "
        "behavioural exclusion rather than a compile fix. It is invisible in the ledger because the Linux figure does not move: "
        "AGENTS.md's gate row still reads '122 provider (carried; unchanged)' with no platform qualifier, and constraint 18's rebuild "
        "was correctly not engaged since no provider source or manifest template moved. Discharge: qualify the ledger row to say what "
        "platform the 122 was measured on, or restore the four tests on Windows.",
    ),
    entry(
        "LEDGER-BINARY-COUNT-CORRECTED-IN-ONE-FILE-ONLY",
        "The 216/227 binary-count correction was applied to AGENTS.md alone and five other sites still say 227 binaries",
        "docs/reports/lean-proof-integrity/validation.json settles the split: workspace.test_binaries = 216, "
        "workspace.result_summaries = 227, doc_test_result_summaries = 11. So '216 test executables / 227 result summaries' is "
        "CORRECT and AGENTS.md's gate row is right. Card STABILITY-REEVAL-1 (wave 34) found the correction was applied in AGENTS.md "
        "ONLY: 'all 227 binaries green' or '227 test binaries' still stands in the gate.json note that AGENTS.md's own row cites, in "
        "docs/reports/wave33-close/README.md:169,181,184, and in docs/development/increments/wave-33.md:98,110. AGENTS.md also "
        "asserts the run reproduced the integrator's figure 'exactly, test for test', so the two labels cannot both be right. The "
        "wave-33 union merge is what put the corrected and uncorrected figures side by side in wave-33.md, where neither parent "
        "contained both. Discharge: propagate 216/227 to the five remaining sites, or state which figure each one counts. "
        "Frozen-archive care applies - gate.json is a close artifact, so publish provenance beside it rather than editing it "
        "(constraint 21).",
    ),
    entry(
        "WAVE-33-RECORD-FUSED-PARAGRAPH",
        "The wave-33 union merge dropped a blank line, fusing two increment entries into one rendered paragraph",
        "Card STABILITY-REEVAL-1 (wave 34) verified with cat -A that docs/development/increments/wave-33.md has NO blank line between "
        "line 45 (the Lean proof-integrity paragraph's last line, ending '[validation record](...validation.json).') and line 46 "
        "('Wave 33 close (2026-09-15): ...'). In Markdown the two increment entries therefore render as ONE fused paragraph, against "
        "the sync rule's 'one paragraph per increment, newest first' - which is the unit of record the whole increment-history "
        "mechanism is built on. The seam between the two same-parent paragraphs retains its blank line; only the parent-1/parent-2 "
        "seam of the d3225f91 union lost it. The same union also left two contradictory binary counts in the file (see "
        "LEDGER-BINARY-COUNT-CORRECTED-IN-ONE-FILE-ONLY). Discharge: insert the blank line and reconcile the 216/227 pair. Trivial to "
        "fix and invisible until rendered, which is why it survived a close.",
    ),
    entry(
        "DANGLING-THEOREM-CITATION-AFTER-LEAN-SWEEP",
        "Two frozen reports cite a theorem the Lean proof-integrity sweep removed",
        "The Lean proof-integrity sweep (commit d117b74f) removed QCore.BranchJoin.evaluation_is_state_free as a literal "
        "self-equality (`c.evaluate = c.evaluate := rfl`). Card STABILITY-REEVAL-1 (wave 34) verified the removal is sound - it lost "
        "nothing, and the sweep's 13 removals were all confirmed vacuous, with two genuine assumed-conclusion defects repaired rather "
        "than deleted and AuditAll's axiom-closure array byte-for-byte intact - but grepped the tree and found the theorem still "
        "cited as a GROUND by two frozen reports: docs/reports/wave19-or-and-lowering/README.md:209 ('literal comparison "
        "(BranchJoin.evaluation_is_state_free)') and docs/reports/wave19-if-statement/README.md:376. It now survives only in archived "
        "audit logs. Neither report appears in the superseding report's 'Relevant historical evidence' list, which does name nine "
        "others. Correctly, the frozen reports were NOT edited (constraint 21), so the repair belongs on the superseding side. "
        "Discharge: name both reports in docs/reports/lean-proof-integrity/README.md's supersession list, so a reader following the "
        "citation is not left at a theorem that no longer exists.",
    ),
    entry(
        "FB-PLAN-GATE-HOST-TIMEOUT",
        "A planning card recorded a failing standing gate and the residual reached no tracker",
        "Named in docs/plans/fluent-bit-system-coverage/README.md:244-254 (wave 33) and, until card STABILITY-REEVAL-1 (wave 34), "
        "reaching NO tracker - not named_followups, not AGENTS.md, not wave33-close/reconciliation.json - because the residual sweep "
        "reads only docs/reports/ (see SWEEP-RESIDUALS-SCANS-REPORTS-ONLY). The planning card recorded the standing gate exiting 101 "
        "with 4,543 passed / 1 failed / 8 ignored, the failure being "
        "closed_input_does_not_mask_descendant_timeout_or_accept_a_partial_request at "
        "crates/qcore-verifier-host/tests/standalone_host.rs:212, with an isolated rerun failing identically and an explicit "
        "statement that it is not classified as a flake. Card STABILITY-REEVAL-1 independently reproduced the failure 3/3 in "
        "isolation at the wave-33 close tip and measured it a throughput shortfall rather than a hang; see "
        "VERIFIER-HOST-LARGE-RESPONSE-CONTROL-RED, which carries the technical discharge condition. This entry records the TRACKING "
        "half: a live failing gate row named only inside a planning document. Discharge: resolved together with "
        "VERIFIER-HOST-LARGE-RESPONSE-CONTROL-RED; this entry closes when that one does.",
    ),
    entry(
        "LEAN-STATEMENT-SEMANTIC-REVIEW",
        "The Lean statement sweep is mechanical and no exhaustive semantic review is claimed",
        "Named by the Lean proof-integrity card (wave 33) and, until card STABILITY-REEVAL-1 (wave 34), reaching no tracker. "
        "AuditAll.lean's added check detects literal self-equalities and exact-assumed-conclusion statements, and the card removed ten "
        "of the former plus an existence-only theorem named as uniqueness, and repaired two genuine assumed-conclusion defects. "
        "8,884 public non-internal theorems and 42,938 declarations pass the mechanical check. What is NOT claimed, by that card or "
        "this one, is that every surviving statement says what its name claims: the detector is syntactic, and a theorem can be "
        "non-vacuous and still misnamed or weaker than its title suggests. Card STABILITY-REEVAL-1 verified the REMOVALS were vacuous "
        "(so the proof surface lost count, not content) and explicitly did NOT attempt the semantic review, recording it as "
        "postponed. Discharge: a bounded semantic review over a named subset - the theorems AGENTS.md's constraints cite as grounds "
        "are the highest-value subset, being the ones a reader treats as load-bearing - rather than an exhaustive pass.",
    ),
    entry(
        "WAVE-33-DEFERRED-DECISIONS-UNTRACKED",
        "The wave-33 close's four deferred decisions are named only in its own README",
        "The wave-33 close deliberately DEFERRED four decisions to the integrator and the user and named them in its README rather "
        "than taking them: CT10-REASON-ROW, DOCS-COMPACT-6, MATRIX-UPDATED-STALE-AT-WAVE-27 and SYNC-RULE-TOOLING-COMMIT-SCOPE "
        "(whether the sync rule binds a tooling commit publishing under docs/reports/); EXAMPLES-STYLE-SLICE-FANOUT is named there "
        "too. Deferring is a valid outcome and the close recorded it correctly. The defect is that all four reach no tracker, which "
        "card STABILITY-REEVAL-1 (wave 34) confirmed by running scripts/reconciliation_sweep.py --residuals - they are four of the "
        "five names it returned as being in no tracker. They are registered here as ONE entry so they stop being invisible without "
        "each being asserted as this card's own finding; this card takes none of them. Two are already live rather than theoretical: "
        "SYNC-RULE-TOOLING-COMMIT-SCOPE has an instance in commit 37bc9004 (see WINDOWS-COHERENCE-GATES-UNCARDED), and DOCS-COMPACT-6 "
        "is due 'around wave 34' by CLAUDE.md's own cadence text, while CLAUDE.md is byte-identical to the wave-32 tip. Discharge: "
        "split into individually tracked entries when each is taken up, or record each decision's outcome at the wave it is taken.",
    ),
    entry(
        "RETURN-POSITION-ESCAPE-UNREACHABLE",
        "The vacuous escape backstop, re-measured much more broadly, and narrowed to a terminal boundary",
        "Named by card C-GATEWAY-RETURN-POSITION (wave 33) and recorded in standing constraint 5b, but - until card "
        "STABILITY-REEVAL-1 (wave 34) - carrying NO entry in this ledger: it was named only inside an AGENTS.md constraint "
        "paragraph, which scripts/reconciliation_sweep.py counts as a tracker, so --residuals never flagged it (see "
        "RESIDUAL-NAMED-ONLY-IN-A-CONSTRAINT-IS-UNTRACKED-WORK). The arm is at crates/qcore-c-gateway/src/legacy_c.rs:3374-3386, "
        "forcing returned_place: None when the strict Returned rule refuses but the Decides rule admits; the mutation is "
        "returned_place: canonical_place(&texts), which compiles the node to EscapeIfOwned instead of NoOp. "
        "STABILITY-REEVAL-1 re-measured the vacuity far beyond the originating card's 3 binaries / 52 tests: under the mutation, "
        "qcore-c-gateway is 985 passed / 0 failed across 41 binaries, twelve further compiler binaries are 63 passed / 0 failed, both "
        "probes exit 0, and the FULL pinned 185-unit corpus audit reproduces the card's archived AFTER artifacts BYTE-IDENTICALLY "
        "(diagnostics.json, monkey-source.json, chunk-contracts.json). The arm demonstrably EXECUTES on real input during that run - "
        "the 33 withdrawn return-position leaves are exactly the untracked_pointer cases - and moves nothing. VERDICT: ACCURATE, and "
        "the vacuity is a GENUINE SEMANTIC CONSEQUENCE rather than missing coverage. Both consumers of EscapeIfOwned are guarded on "
        "state.pointers membership (legacy_c.rs:6757; observe_exit at :5827) and the NoOp node falls to the same "
        "ReturnedValue::None at :5844, so with the key absent the two answers are literally the same value; and canonical_place's "
        "string can never BE a key, since chain-shaped keys arise only through the aggregate-store route, which requires a base "
        "may_bind reports bound, while F4 admission requires !may_bind(base). CONSEQUENTLY the residual's first listed discharge - 'a "
        "fixture that makes the projection track a place an admitted chain can name' - is NOT AVAILABLE: it needs a source change "
        "weakening admitted_chain, not a fixture. The only available discharge is 'state it as a measured boundary', which the "
        "originating card already did. Discharge: mark it a CLOSED measured boundary rather than pending work, so the tracker stops "
        "implying work that cannot be done - EXCEPT that FB-BACKLOG-CUSTODY's member-typing widening (RETURN-POSITION-TYPED-MEMBER-REACH) "
        "is the one move that can put a projection-bound place on an admitted chain, at which point the None answer starts DECIDING "
        "escapes with a mutation-confirmed-vacuous arm as the decider, and this entry reopens.",
    ),
    entry(
        "RETURN-POSITION-ACCUSING-UNEXERCISED-IN-CORPUS",
        "The accusing return-position arm is proved and unit-tested but reaches no corpus function",
        "Named by card C-GATEWAY-RETURN-POSITION (wave 33) and recorded in standing constraint 5b, but - until card "
        "STABILITY-REEVAL-1 (wave 34) - carrying NO entry in this ledger, for the same reason as its sibling "
        "RETURN-POSITION-ESCAPE-UNREACHABLE (see RESIDUAL-NAMED-ONLY-IN-A-CONSTRAINT-IS-UNTRACKED-WORK). The arm is proved and "
        "driven by unit tests, but reaches NO function of the pinned 185-unit corpus: the wave-33 arbiter added ZERO leak leaves, so "
        "the unchanged leak pin of 272 records that ABSENCE rather than the arm's verification. STABILITY-REEVAL-1 confirms the "
        "naming accurate and supplies the step-4 half the originating card did not state: the queued card FB-BACKLOG-CUSTODY is what "
        "makes it load-bearing. Backlog wrappers return chunk pointers through field chains in acquiring bodies - precisely the shape "
        "this arm accuses - so when that card lands the leak pin moves, constraint 7 gains a link, and the arm is exercised IN "
        "PRODUCTION FIRST rather than in a fixture, which is the worst order in which to first exercise an accusing arm. Discharge: "
        "either a fixture-level corpus case that drives the accusing arm before FB-BACKLOG-CUSTODY lands, or an explicit statement in "
        "that card's brief that its first job is to exercise this arm on a known case and attribute the resulting leak movement leaf "
        "by leaf.",
    ),
    entry(
        "RESIDUAL-NAMED-ONLY-IN-A-CONSTRAINT-IS-UNTRACKED-WORK",
        "The sweep counts an AGENTS.md mention as tracking, so a residual named only in a constraint is invisible as open work",
        "scripts/reconciliation_sweep.py:248 defines TRACKERS as acceptance_matrix.json, AGENTS.md and CLAUDE.md, so --residuals "
        "treats a residual named anywhere in AGENTS.md - including inside a standing-constraint paragraph - as tracked. Card "
        "STABILITY-REEVAL-1 (wave 34) found the consequence: RETURN-POSITION-ESCAPE-UNREACHABLE and "
        "RETURN-POSITION-ACCUSING-UNEXERCISED-IN-CORPUS, the two vacuity residuals wave 33 found by mutation and reported as findings "
        "rather than smoothing over, are named in constraint 5b and had NO entry in named_followups. The sweep was therefore silent "
        "about both, and neither appeared in any open-work list. A constraint paragraph states a RULE that must survive refactors; "
        "the follow-on ledger states WORK that is open. A name can be in the first and still be untracked as the second, and these "
        "two are the clearest possible case because the wave that produced them did everything else right. Both are now registered. "
        "Discharge: either narrow the TRACKERS definition so an AGENTS.md mention alone does not count as tracking a residual, or "
        "make the close require that every residual a card's report names reaches named_followups specifically - the ledger the "
        "matrix exists to keep.",
    ),
]

AMEND = {
    "__DISABLED_RETURN-POSITION-ESCAPE-UNREACHABLE": (
        " Card STABILITY-REEVAL-1 (wave 34) RE-MEASURED this residual far more broadly than the originating card "
        "(985 + 63 tests across 53 binaries, both probes, and the full pinned 185-unit corpus audit, whose diagnostics.json, "
        "monkey-source.json and chunk-contracts.json all reproduce the card's archived AFTER artifacts BYTE-IDENTICALLY under the "
        "mutation) and confirms the residual ACCURATE. It also narrows it: the vacuity is a GENUINE SEMANTIC CONSEQUENCE, not missing "
        "coverage. Both consumers of EscapeIfOwned are guarded on state.pointers membership (legacy_c.rs:6757 and observe_exit at "
        ":5827) and the NoOp node falls to the same ReturnedValue::None at :5844, so with the key absent the two answers are "
        "literally the same value; and canonical_place's string can never BE a key, since chain-shaped keys arise only via the "
        "aggregate-store route, which requires a base may_bind reports bound, while F4 admission requires !may_bind(base). "
        "Consequently the residual's first listed discharge - 'a fixture that makes the projection track a place an admitted chain "
        "can name' - is NOT AVAILABLE: it needs a source change weakening admitted_chain, not a fixture. The only available discharge "
        "is 'state it as a measured boundary', which the originating card already did, so this residual is in its TERMINAL state and "
        "carrying it as pending follow-on work overstates what remains. It should be marked a closed measured boundary - EXCEPT that "
        "FB-BACKLOG-CUSTODY's member-typing widening is the one move that can put a projection-bound place on an admitted chain, at "
        "which point the None answer starts deciding escapes and this entry reopens."
    ),
    "D02-DIMENSIONS-FOLLOWUP-COUNT": (
        " PARTIALLY DISCHARGED by card STABILITY-REEVAL-1 (wave 34), and narrowed rather than closed. "
        "scripts/tests/test_build_matrix_sections.py gains class CommittedMatrixIsSelfConsistent with two tests against the COMMITTED "
        "matrix - which is the gap, since the two pre-existing tests compare generator output against generator output, both "
        "self-consistent by construction, so neither could ever see drift in the committed file. That is why this residual fired "
        "twice and was caught both times by a person. test_each_derived_dimension_matches_the_section_it_counts covers the two "
        "dimensions whose name IS a section name (requirements, named_followups); "
        "test_the_committed_matrix_keeps_the_generators_byte_format covers the second half, a hand edit in another serialisation - "
        "the exact condition of commit 1c7d8e45, which buried a single real line in a 12,673-line reserialisation diff. Both were "
        "shown non-vacuous by mutation: setting dimensions.named_followups to 45 (the exact historical drift) turns the first red, "
        "and re-serialising at indent=2 turns the second red; the matrix was restored and verified byte-identical by sha256 "
        "afterwards. STILL OPEN: the derived cell and product counts (requirement_evidence_cells, ct_cells, ct_rows, "
        "unverified_spec_items), which this card deliberately left alone because each needs its own derivation rule and a wrong one "
        "would pin a wrong number."
    ),
}


def main():
    raw = MATRIX.read_text(encoding="utf-8")
    doc = json.loads(raw)
    followups = doc["named_followups"]
    have = {e["id"] for e in followups}

    added = [e for e in NEW if e["id"] not in have]
    followups.extend(added)

    amended = []
    for entry_ in followups:
        extra = AMEND.get(entry_["id"])
        if extra and extra not in entry_["why"]:
            entry_["why"] += extra
            amended.append(entry_["id"])

    doc["dimensions"]["named_followups"] = len(followups)
    MATRIX.write_text(json.dumps(doc, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")

    print(f"appended {len(added)} (skipped {len(NEW) - len(added)} already present)")
    print(f"amended  {amended}")
    print(f"named_followups now {len(followups)}")


if __name__ == "__main__":
    main()

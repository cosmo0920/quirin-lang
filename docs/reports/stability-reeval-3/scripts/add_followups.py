#!/usr/bin/env python3
"""Append card STABILITY-REEVAL-3's residuals to the acceptance matrix.

`named_followups` is `hand_maintained` in the generator's `section_ownership`
(`docs/reports/remaining-d02/build_matrix.py` carries it verbatim from the
committed matrix), so this appends rather than regenerates, on the
`stability-reeval-1/scripts/add_followups.py` precedent. It then rewrites with
the generator's byte format (`indent=1`, `ensure_ascii=False`, trailing
newline) and leaves `dimensions.named_followups` to the generator:

    python3 -B docs/reports/remaining-d02/build_matrix.py . \
        --gate docs/reports/wave37-close/gate.json

Idempotent: an id already present is skipped. Every entry carries all five
keys (`id`, `title`, `why`, `source`, `recorded_by`), which is the point of
`MATRIX-RECORDED-BY-ABSENT-67` below.
"""

import json
import pathlib

MATRIX = pathlib.Path("docs/reviews/r0_3/acceptance_matrix.json")
REPORT = "docs/reports/stability-reeval-3/README.md"
CARD = "STABILITY-REEVAL-3"


def entry(id_, title, why):
    return {"id": id_, "title": title, "why": why, "source": REPORT, "recorded_by": CARD}


NEW = [
    entry(
        "MATRIX-RECORDED-BY-ABSENT-67",
        "67 of 215 named_followups entries carry no recorded_by, 28 of them registered in waves 36-37",
        "MEASURED by card STABILITY-REEVAL-3 (wave 38): the 215 entries have three key sets - 147 with "
        "(id, recorded_by, source, title, why), 67 with no recorded_by, one (wave3-CT) with no source. Of the "
        "67, 39 are pre-schema entries and 28 were registered in waves 36-37 by CARD-SCOPE-GUARD (6), "
        "JOINED-PROVENANCE-CALL-FRAMES (2), WRITE-LEND-TASKS-EXERCISE (5), BASE-TYPE-READER-CENSUS (5) and "
        "W37-CORPUS-CONTRACTS-4 (10), so the two wave-37 code arbiters' registrations sit on opposite sides of "
        "the split. This card's own brief defines its population as 'entries whose recorded_by starts with "
        "STABILITY-REEVAL', and the first script written against the matrix crashed on a KeyError at the "
        "eighth entry read: a sweep keyed on recorded_by misses 67 entries without saying so. Nothing is "
        "lost - every one carries source - but a tracker whose entries do not agree on their keys cannot be "
        "queried by provenance. DISCHARGE: a generator check that every named_followups entry carries the "
        "five keys, shown red by mutation, after back-filling the 67 from their source reports (an "
        "integrator edit); OR a recorded decision that recorded_by is optional and source is the provenance "
        "key, with every recorded_by-keyed query in the tree restated on source.",
    ),
    entry(
        "W36-W37-MUTATION-HARNESSES-UNSHIPPED",
        "Four waves-36/37 cards state a mutation count whose harness lives on a gitignored path",
        "MEASURED by card STABILITY-REEVAL-3 (wave 38) by ls and find over every waves-36/37 report "
        "directory. Three cards ship harness and per-test log (joined-provenance-call-frames, "
        "scalar-route-exit-edge, task-route-tsan) - the standard LEASE-BOUNDARY-MUTATION-EVIDENCE-UNSHIPPED "
        "set. Four do not: joined-provenance-round-bound (directory is README.md alone; harness at "
        "measure/joined-provenance-round-bound/mutations.py, seven mutations, no log - DOCS-COMPACT-7 "
        "noticed the same absence as 'published none'); ct-lane-5 (section 2.4 cites "
        "'measure/ct-lane-5/mutate.py; results in mutations.json' and the directory holds no "
        "mutations.json - the seven results are under the mutations key of ct_stage_evidence.json:236, so "
        "the citation names the wrong file and the harness is unshipped); write-lend-tasks-exercise "
        "(section 8: 'private/mutate.py (worktree-private), 11 mutations'; directory README.md + gate.json; "
        "the table names no catching test per row); mutex-share-cross-emission (section 4.2 names no "
        "script; README.md + ct_stage_evidence.json). Each states a non-vacuity count a later reader can "
        "check only by re-deriving every mutation by hand - the situation firing #1 named for one card is "
        "the majority case for the two waves' concurrency and CT cards, while every gateway card ships "
        "its harness. DISCHARGE: per card, ship the harness and a per-test log beside the report "
        "(append-only, no frozen text edited) or restate the count in the card's own record as "
        "measured-once-not-archived; and one line in the card-report convention saying which is required.",
    ),
    entry(
        "CLANG-GUARDED-GREEN-FAMILY-750",
        "The 14-row Clang early-return shape is a 750-guard family, and no gate.json records Clang presence",
        "MEASURED by card STABILITY-REEVAL-3 (wave 38): rg -c 'if !clang_available()' over crates/ and "
        "compiler/ totals 750 early-return guards across 79 test files (publication_custody.rs 36, "
        "neutral_expression_positions.rs 32, may_alias_boundary.rs 32, ... scalar_route_exit_edge.rs 11). "
        "RETURN-CALL-TESTS-CLANG-GUARDED-GREEN (firing #2) named the shape on one file of 14 rows; it is "
        "the crate convention. On a Clang-less host every one reports ok having measured nothing, and no "
        "close's gate.json records whether Clang was present when its workspace count was taken - the ten "
        "keys are counts and exits, so a 4,637-green figure taken without Clang would be indistinguishable "
        "from one taken with it. This card recorded clang-version.txt (Ubuntu clang 21.1.8, QCORE_CLANG "
        "unset) beside its own gate as the cheapest form of the discharge. DISCHARGE: a clang presence key "
        "(present/absent plus version) in the close's gate.json, checked by the generator the way "
        "lean_audits now is; OR one shared helper that reports Clang absence as ignored so the count "
        "moves visibly; and the 14-row entry re-scoped to the family or retired into this one.",
    ),
    entry(
        "LEGACY-CFG-NODE-FIXTURES-SPELL-EVERY-FIELD",
        "Every LegacyCfgNode test fixture spells every field, so each new field breaks sibling cards at integration",
        "MEASURED by card STABILITY-REEVAL-3 (wave 38), step 3, from the 797661f3 integration failure: "
        "three fixtures in crates/qcore-c-gateway/src/legacy_joined_provenance.rs (:553, :598, :973) and "
        "one in legacy_recursion.rs (:432) build LegacyCfgNode in a local closure that lists every field, "
        "and LegacyCfgNode has neither a constructor nor a Default. SCALAR-ROUTE-EXIT-EDGE added "
        "loop_exit_edge; JOINED-PROVENANCE-ROUND-BOUND's new fixture, written against the old struct, "
        "compiled green on its own base and red at the merge with no textual conflict, and the integrator "
        "added `loop_exit_edge: None,` by hand. The field is unread by the module under test (rg finds "
        "only the three fixture literals in legacy_joined_provenance.rs; derive/iterate read successors "
        "only), so the value was immaterial and the close's 'no residual' verdict stands for the edit. "
        "The mechanism is not discharged: the next field added to LegacyCfgNode reproduces it. "
        "DISCHARGE: a LegacyCfgNode::plain(successors) constructor (or Default) in the gateway crate that "
        "the fixtures use, so a new field has one home; OR a recorded decision that the integration gate "
        "is the intended catch.",
    ),
    entry(
        "F02-WINDOWS-PIN-STALE-SINCE-WAVE-36",
        "The Windows F02 twin pins the fourth-move digests while the Linux file pins the fifth; the Windows row is red at HEAD",
        "MEASURED by card STABILITY-REEVAL-3 (wave 38), re-verifying a Step-0 sub-audit line by line. "
        "compiler/tests/f02_trace_relation.rs:96-97 pins ROUND_TRIP_C_DIGEST bfa572914cb96413 and "
        "CONCURRENT_C_DIGEST 9cbe67a70a420d52 (its own comment: the fifth recorded move, card "
        "WRITE-LEND-TASKS-EXERCISE, 2026-09-18). compiler/tests/windows_threads.rs:31,36 pins "
        "6df369991465940e and 0da423be2f16b8bb - the fourth-move values (write-lend-tasks/README.md:247) - "
        "under the comment 'Keep these pins paired with f02_trace_relation.rs'. Both files compute the "
        "same FNV-1a (offset 0xcbf29ce484222325, prime 0x100000001b3) over "
        "diagnostic_sink::instrument(module.c_source) of the same two programs; the Windows one "
        "normalises CRLF to LF first, so it is the Linux function over the Linux bytes. Whatever emitted "
        "bytes are platform-independent - which the pairing comment presumes - the Windows test "
        "windows_f02_renderings_satisfy_the_incoming_trace_relation_and_emission_pins asserts a digest "
        "the emitter no longer produces and is red on every Windows host since fa48211a. The re-pinning "
        "card's report (write-lend-tasks-exercise/README.md section 4) does not mention windows_threads.rs; "
        "the last edit of that file was 37bc9004, the uncarded commit WINDOWS-COHERENCE-GATES-UNCARDED "
        "names. This is F02-DIGEST-PIN-DUPLICATED's predicted drift, now a live red row rather than a "
        "hazard, invisible to every Linux gate. NOT taken by this card: re-pinning a Windows expectation "
        "without a Windows run is the shape 37bc9004 was named for. DISCHARGE: re-pin windows_threads.rs "
        "to bfa572914cb96413 / 9cbe67a70a420d52 in a card that records a Windows run of that test (or "
        "states that the native half was not run); AND make the pair one shared constant or add a "
        "Linux-side test asserting the two files' literals agree, red by mutation - the original entry's "
        "own discharge, so this drift is the last.",
    ),
    entry(
        "CT42-CELL-FALLBACK-ARM-DIRECTION",
        "The CT42 deriver's two unreachable arms are described as refusing more; they assign a singleton cell the checker judges disjoint",
        "MEASURED by card STABILITY-REEVAL-3 (wave 38), step 4, re-verified against the lines: "
        "crates/qcore-core/src/socket_footprint.rs derive_socket_cells has two arms 'unreachable for an "
        "admitted program' - a connect the resolution cannot target (continue, :184) and an accept with "
        "nothing queued (None => acquire(entries, binding, None), :211) - and the second gives the "
        "binding cell = its own fresh descriptor (:167). In the unedited checker "
        "(crates/qcore-core/src/disjoint_footprint.rs:714-722) two Descriptor footprints with different "
        "cells return Ok((true, ...)) - proved disjoint - and refuse SharedProtocolCell only when the cells "
        "are equal. So the fallback arm ADMITS disjointness for an accepted binding whose real connector "
        "it failed to pair. docs/reports/ct42-protocol-cell/README.md:128-132 says the opposite: 'the "
        "direction that refuses more disjointness questions than it admits'. The sentence is harmless today "
        "only because check_socket_program (:149) refuses first, which is exactly the structure "
        "CT42-CELL-ROUTE (queued) makes load-bearing: once source routes the deriver, any disagreement "
        "between its FIFO pairing and std_socket's peer assignment (CT42-CELL-PENDING-PHASE) reaches an "
        "arm that admits. DISCHARGE: publish the correction beside the frozen report (constraint 21) and "
        "make both arms return Err (or prove pairing agreement with std_socket's resolution) before any "
        "source certificate carries a derived cell; the route card's brief carries this line.",
    ),
    entry(
        "CT07-PROOF-LEG-PATTERN-UNAVAILABLE",
        "CT07-TWO-THREAD-PROOF-LEG's discharge names the CT01/CT37/CT03 pattern, whose binding refuses CT07's run by a decided boundary",
        "MEASURED by card STABILITY-REEVAL-3 (wave 38), step 4. Each existing proof leg binds its case "
        "through AdmittedCausalOrder::derive over the reference run (compiler/tests/ct03_proof_leg.rs:381-384, "
        "ct01:220, ct37 the same), and derive calls reference_trace "
        "(crates/qcore-core/src/task_trace_relation.rs:852), which returns "
        "Err(UnsupportedObservedEvent) for BufferLeased/BufferLeaseReturned (:563-565) AND for "
        "BufferByteWritten/BufferByteRead (:551-553). CT07's fixture "
        "(compiler/tests/fixtures/ct/CT07/case.qr) spawns two writers with lend buffer 0..2 and 2..4 "
        "and each writes a byte, so its reference run carries both event kinds and derive refuses it at "
        "the first lease. Constraint 15n as amended by WRITE-LEND-TASKS-EXERCISE records that boundary "
        "as DECIDED (the alphabet does not grow; TRACE-ALPHABET-LEASE-GROWTH-UNDECIDED is retired), so "
        "the F02 route to a CT07 proof leg is closed by decision. The residual's text says the leg is "
        "'unusually cheap because the theorem already exists' and asks for 'a proof-leg card on the "
        "CT01/CT37/CT03 pattern' - a pattern its own artifact cannot take. The theorems "
        "(QCore.WriteLendTasks.no_byte_has_two_write_authorities, "
        "QCore.WriteLendTasksExercise.no_byte_is_written_by_two_parties) are about the ledger's lease "
        "list, not a trace, so a binding must go through the checked ScopedTasks module's lease set - a "
        "pattern with no precedent and no PROOF_GATED_CASES entry of its kind. DISCHARGE: amend "
        "CT07-TWO-THREAD-PROOF-LEG's discharge to a ledger-side binding with its own non-vacuity method, "
        "and drop 'unusually cheap'; the entry stays open.",
    ),
    entry(
        "WAVE-38-QUEUE-NAMES-UNTRACKED",
        "Two of the queued LOOPS/AGGREGATE cards' target names reach no tracker, and one is classified as a rule not work",
        "MEASURED by card STABILITY-REEVAL-3 (wave 38), step 4. The queued compiler cards name their "
        "targets by residual: LOOPS-ON-COMPOSED-ROUTES by STRING-CONCAT-LOOP-WINDOW, "
        "SLICE-PERMISSIONS-LOOP-WINDOW and WRITE-LEND-RELEND; AGGREGATE-OWNED-FIELD by its own name. In "
        "named_followups: WRITE-LEND-RELEND is PRESENT; STRING-CONCAT-LOOP-WINDOW and AGGREGATE-OWNED-FIELD "
        "are ABSENT (only mentioned inside other entries' why texts, which scripts/reconciliation_sweep.py "
        "says is not tracking); SLICE-PERMISSIONS-LOOP-WINDOW is one of the nine names "
        "CONSTRAINT-NAMED-BOUNDARIES-NOT-OPEN-WORK classifies as a RULE the constraint imposes, which "
        "'a card that lifts must name then'. So a card sized from the tracker finds one of its three "
        "targets, and a card sized from the constraint finds a rule. This is WAVE-36-CARDS-UNCHARTERED's "
        "shape one wave later. Step-4 measurement for the briefs: on both routes the window is computed "
        "over TOP-LEVEL event indices only (std_buffer.rs lend_windows :965-1054, last_use over "
        "Store/Load/Extent/Release/Lend with through = unwrap_or(index); std_string.rs last_view_uses "
        ":1057 and :1509), so a loan used only inside a Repeat body would close before the loop opens and "
        "every owner write in the body would be admitted - which is why both Repeat refusals "
        "(std_buffer.rs:1926-1944, std_string.rs:1814-1843) sit BEFORE the window computation and must "
        "not be reordered; and constraint 15k's 'a record contributes no ownership operation' is an "
        "explicit no-op arm (std_buffer.rs:914-918) over an owner vocabulary that is a plain String "
        "everywhere (ownership.rs:169-176, :242, :845), so AGGREGATE-OWNED-FIELD is an IR card before it "
        "is a route card and Q055's buffer-at-a-field-position refusal is the gate it must knowingly "
        "lift. DISCHARGE: register STRING-CONCAT-LOOP-WINDOW and AGGREGATE-OWNED-FIELD with the "
        "discharge conditions above, or extend CONSTRAINT-NAMED-BOUNDARIES-NOT-OPEN-WORK to name them "
        "and say why the queued cards are not their work.",
    ),
    entry(
        "TSAN-SUBJECTS-ARE-TWINS-NOT-FIXTURES",
        "The TSan rows run twins of CT07's and CT46's artifacts, not the fixtures the CT rows credit",
        "MEASURED by card STABILITY-REEVAL-3 (wave 38). (1) compiler/tests/task_route_tsan.rs's subjects "
        "are DISJOINT_EXERCISE (:65, the std.logger twin duplicated verbatim from "
        "write_lend_tasks_exercise.rs:45 with nothing asserting the two literals stay equal) plus the four "
        "examples/threads programs (:104-117); compiler/tests/fixtures/ct/CT07/case.qr is not among "
        "them, and the CT harness compiles with -fsanitize=address,undefined only, so CT07 has no TSan "
        "row while CT07-TWO-THREAD-REPLAY-ORDER's why still says the TSan row is blocked by "
        "TASK-ROUTE-TSAN-SEGV (retired). (2) The CT46 row "
        "(crates/qcore-core/tests/unit/mutex_c_emit.rs:1200) runs fork_join_trace() (:843), a hand-built "
        "MutexInvariantProgram used only inside that unit file; nothing asserts its events equal the "
        "lowering of fixtures/ct/CT46/case.qcore that ct_corpus.rs's run_mutex_crossing executes, so "
        "task-route-tsan/README.md's 'CT46's emitted crossing' overstates by that much. (3) The report's "
        "section 4.1 says the join mutation makes TSan report 'two races'; the committed harness runs "
        "under TSAN_OPTIONS=halt_on_error=1 (tests/support/thread_sanitizer.rs:183) and the test asserts "
        "races >= 1 (INFERRED: TSan semantics), so the sentence is not what the archived harness measures. "
        "None of this makes a row vacuous - each is green about the program it runs - but the boundary "
        "between 'the row' and 'the case' is a name, not a test. DISCHARGE: add CT07/case.qcore to the "
        "TSan subjects and a test asserting fork_join_trace().events equals CT46's lowered events (or "
        "reword both reports beside them to 'a CT46-shaped program' / 'the logger twin'); one shared "
        "fixture for DISJOINT_EXERCISE; and the 'two races' sentence corrected beside the report or a "
        "halt_on_error=0 run archived.",
    ),
    entry(
        "WLT-EXERCISE-TSAN-TEST-SILENT-ARMS",
        "Wave 36's 'cannot run under TSan' test has one arm that passes silently and one that cannot go red",
        "MEASURED by card STABILITY-REEVAL-3 (wave 38): "
        "compiler/tests/write_lend_tasks_exercise.rs:735-762 "
        "thread_sanitizer_cannot_run_this_routes_programs_lease_or_not matches on (exercised, lease_free): "
        "(Err, Err) asserts both stderrs contain 'SEGV' - any SEGV, not the 0x18 shape "
        "every_subject_dies_unbound_at_the_one_fault (task_route_tsan.rs:237) pins; (Ok, Ok) asserts "
        "!stderr.contains('data race'), which cannot fail because run_native sets "
        "TSAN_OPTIONS=halt_on_error=1 (:655) and returns Err on any non-zero exit (:660-667), so a race "
        "never reaches that arm; and (Ok(_), Err(_)) => {} passes silently. The precise boundary between "
        "this test and wave 37's: wave 36 asserts 'under the published thrd_create, TSan cannot run either "
        "program' (unbound build, this file's own C executable); wave 37 asserts 'under a pthread-bound "
        "build (-include bind.h), all five subjects are race-free' and, separately, the same unbound fault "
        "at the address. They share no compilation, executable or Rust binary; both are green today "
        "because the unbound fault fires at __tsan_func_entry before any QCore worker code runs, so the "
        "unbound rows are evidence about the toolchain and the bound rows about the pthread-substituted "
        "program (TASK-ROUTE-TSAN-PTHREAD-BINDING). Both could stay green while something visible only on "
        "the published thrd_create path is broken, and the bound rows read no stdout (0 occurrences in "
        "task_route_tsan.rs), so their correctness evidence is wave 36's separate ASan row. DISCHARGE: "
        "retire the wave-36 row in favour of every_subject_dies_unbound_at_the_one_fault, or tighten it "
        "to the 0x18 shape and make the (Ok, Err) arm a panic and the (Ok, Ok) arm honest about "
        "halt_on_error - either with the ground recorded.",
    ),
    entry(
        "MAIN-GUARD-PACK-REFS-UNPINNED",
        "The pack-refs refusal is reproduced twice and pinned by no test, and it is two transactions not one",
        "MEASURED by card STABILITY-REEVAL-3 (wave 38) through a read-only sub-audit's probe in a "
        "throwaway repository against the versioned .githooks (private log packrefs_probe.log, "
        "reproduced here: exit 128 refused-provenance from an agent worktree with and without the token; "
        "exit 128 refused-unauthorised from the MAIN checkout without the token; exit 0 with it). "
        "scripts/tests/test_main_guard_hooks.py (27 tests) drives no pack-refs, gc or maintenance shape "
        "(rg over the file: nothing); its closest row test_a_no_op_update_of_main_is_not_refused (:279) "
        "drives old == new, not null -> current. Two things MAIN-GUARD-REFUSES-PACK-REFS's own text does "
        "not carry: the refusal fires in the main checkout too whenever the token is absent, so the scope "
        "'every delegated worktree' understates; and with the token the guard log records TWO "
        "refs/heads/main transactions per pack-refs - 0000->X (the packed write) then X->0000 (the loose "
        "ref's deletion) - the second byte-for-byte the deletion shape test_deleting_main_is_refused "
        "(:260) and the deletion-reads-as-noop mutant protect, so an exemption of 'old = null OID' alone "
        "would still refuse the second and a bare old == ZERO exemption is the same syntactic shape as a "
        "real advance. DISCHARGE: a committed test running git pack-refs --all from an agent worktree and "
        "from the main checkout that pins whichever behaviour is chosen for BOTH transactions, red by "
        "mutation; and MAIN-GUARD-REFUSES-PACK-REFS's discharge amended to name the second transaction.",
    ),
    entry(
        "LEAN-AUDIT-GATE-KEY-OPTIONAL",
        "The checked lean_audits gate key is optional, so the drift it exists to catch recurs by omission",
        "MEASURED by card STABILITY-REEVAL-3 (wave 38) on a private copy of "
        "docs/reports/remaining-d02/build_matrix.py (original md5 unchanged): a --gate with lean_audits = "
        "139 against 137 proof/Audit*.lean is refused with the verbatim message naming "
        "GATE-ROW-FIGURE-ATTRIBUTION-DRIFT and writes nothing; 137 is accepted; with the comparison "
        "mutated to False the 139 gate is accepted and the matrix written, so the arm is load-bearing and "
        "test_build_matrix_sections.py:289 is the test that goes red. But GATE_CHECKED is populated only "
        "under 'if key in supplied' (:204): a gate.json that omits lean_audits passes without comment, as "
        "the wave-35 and wave-36 gates do. A close that forgets the key gets exactly the unchecked figure "
        "the key was added to prevent. Also: the published validation.lean_audits stays DERIVED, so the "
        "arm guards the gate record's honesty and not the AGENTS.md row (GATE-ROW-FIGURE-ATTRIBUTION-DRIFT "
        "stays partial on its second arm). DISCHARGE: from the wave-38 close on, a --gate file lacking "
        "lean_audits is refused (or its omission printed as provenance in the gate block), with a test red "
        "by mutation.",
    ),
    entry(
        "CALL-FRAMES-NO-SOURCE-FIXTURE",
        "No C-text fixture ties the call-frame allow-list's construction to the arm it feeds",
        "MEASURED by card STABILITY-REEVAL-3 (wave 38) through a read-only sub-audit, re-verified: the "
        "poisoned arm names.retain(|name| preserved.contains(name)) in Facts::step's ContractCall arm "
        "(crates/qcore-c-gateway/src/legacy_joined_provenance.rs:153-155) receives preserved = "
        "cfg.conditional_names from outgoing (:226), which re-applies the identical membership at "
        ":227-229, so deleting the retain changes no derive result and only the direct step test "
        "a_call_forgets_a_name_the_frame_list_does_not_carry (:893) goes red - which is what "
        "CALL-FRAMES-ALLOWLIST-LOCAL-ONLY already records accurately. The spared arm is not vacuous "
        "(restoring the pre-card clear() fails :870 and :969). What no test does is tie the allow-list's "
        "CONSTRUCTION - unique_local_names intersected with frame_protected_locals minus formals at "
        "legacy_c.rs:1839-1844, called from :1972-1973, derive at :6411 - to the arm through "
        "analyze_unrestricted_c11: no C-source fixture in crates/qcore-c-gateway/tests, compiler/tests or "
        "legacy_c.rs drives a local surviving an intervening contracted call end to end. DISCHARGE: one "
        "C-text fixture test in which a local p survives an intervening contracted call and an "
        "address-taken twin does not, red when legacy_c.rs:1840-1844 is widened.",
    ),
    entry(
        "W37-SUBSTITUTION-SET-EQUALITY-UNARCHIVED",
        "W37-CORPUS-CONTRACTS-4's leaf-neutrality claim is a set equality its archived scripts do not compute",
        "MEASURED by card STABILITY-REEVAL-3 (wave 38) through a read-only sub-audit's recomputation "
        "from the archived artifacts (private log recompute_substitution.log). "
        "docs/reports/w37-corpus-contracts-4/README.md section 5.1 says the eleven "
        "abi-unreadable-declaration rows for rd_kafka_topic_partition_list_sort are 'same-position "
        "substitutions ... verified as a set equality, not by counting' (:437-440); the archived "
        "scripts/attribution.py classifies a substitution by per-position leaf-count equality "
        "(:88-95) and census.py lists rows - neither computes the position sets. Recomputed: 9 BEFORE "
        "uncontracted-call positions are a subset of the 11 AFTER withdrawal positions; the two extras "
        "(rd_kafka_assignment_add L20630, rd_kafka_share_assignment_add L20567) carried 'indirect memory "
        "access' before; origins agree and per-position counts are equal at all 11. The claim is TRUE; "
        "its archived support is prose plus a counting script. Also recorded here: the withdrawal arm "
        "(legacy_declared_abi.rs:577-579 via project_parameter :1012-1014) was fixture-driven since "
        "5f3c3cf7 (loader_abi.rs:500-504, the inline function-pointer row), so the production firing is a "
        "transition from fixture-verified to corpus-observed, and LOADER-ABI-INLINE-FNPTR-DECLARATOR's "
        "eventual discharge must flip that pinned row deliberately (invariant 5). DISCHARGE: add the "
        "position-set check to the archived attribution.py or publish the recomputation beside the "
        "report so section 5.1's sentence has a computed source.",
    ),
]


def main():
    matrix = json.loads(MATRIX.read_text(encoding="utf-8"))
    present = {item["id"] for item in matrix["named_followups"]}
    added = []
    for item in NEW:
        if item["id"] in present:
            continue
        matrix["named_followups"].append(item)
        added.append(item["id"])
    MATRIX.write_text(
        json.dumps(matrix, indent=1, ensure_ascii=False) + "\n", encoding="utf-8"
    )
    print(f"added {len(added)}: {', '.join(added)}")
    print(f"named_followups now {len(matrix['named_followups'])}")


if __name__ == "__main__":
    main()

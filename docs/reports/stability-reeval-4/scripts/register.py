#!/usr/bin/env python3
"""Register card STABILITY-REEVAL-4's residuals and its one retirement.

`named_followups` is `hand_maintained` in the generator's `section_ownership`
(`docs/reports/remaining-d02/build_matrix.py` carries it verbatim from the
committed matrix), so this appends and removes in place rather than
regenerating, on the `stability-reeval-{1,2,3}/scripts/add_followups.py`
precedent. It then rewrites with the generator's byte format (`indent=1`,
`ensure_ascii=False`, trailing newline) and leaves `dimensions.named_followups`
to the generator:

    python3 -B docs/reports/remaining-d02/build_matrix.py . \
        --gate docs/reports/stability-reeval-4/gate.json

Idempotent: an id already present is skipped, and a retirement whose id is
already absent is reported as already done.

Every entry carries all five keys (`id`, `title`, `why`, `source`,
`recorded_by`), which is what `MATRIX-RECORDED-BY-ABSENT-67` asks of new
entries and which this card's brief requires.

`--show` prints what would change and writes nothing.
"""

import json
import pathlib
import sys

MATRIX = pathlib.Path("docs/reviews/r0_3/acceptance_matrix.json")
REPORT = "docs/reports/stability-reeval-4/README.md"
CARD = "STABILITY-REEVAL-4"


def entry(id_, title, why):
    return {"id": id_, "title": title, "why": why, "source": REPORT, "recorded_by": CARD}


# --- The one retirement -----------------------------------------------------
#
# Verified registered first (the brief's requirement): the id is read out of
# the committed matrix before it is removed, and the removal is refused if it
# is absent for any other reason than a previous run of this script.
RETIRE = {
    "W36-W37-MUTATION-HARNESSES-UNSHIPPED": (
        "DISCHARGED IN FULL and retired by STABILITY-REEVAL-4 (wave 40). The entry's discharge read: "
        "'per card, ship the harness and a per-test log beside the report (append-only) OR restate the count "
        "in the card's own record as measured-once-not-archived; AND one line in the card-report convention "
        "saying which.' MEASURED at 6dc484ad: all four named report directories carry the dated appended "
        "restatement in the discharge's own words - joined-provenance-round-bound/README.md:453,458; "
        "ct-lane-5/README.md:580,587; write-lend-tasks-exercise/README.md:577,581; "
        "mutex-share-cross-emission/README.md:567,571 - each opening 'Appended, not edited (constraint 21)' "
        "and each reading 'measured once by the card, harness not shipped'; and the convention line is "
        "CLAUDE.md:219-224. The wave-38 close applied the restatements (its README item 7); no close names "
        "the entry, which is why a fully met discharge sat registered for two waves. Wave 39 is additionally "
        "standard-compliant on the entry's FIRST arm: all three of its code cards ship harness and log "
        "(tsan-rows-2/artifacts/, ct07-two-thread-proof-leg/{mutations.log,scripts/}, "
        "ct42-cell-route/{mutations.log,scripts/}). RETIRED; the sibling entry "
        "LEASE-BOUNDARY-MUTATION-EVIDENCE-UNSHIPPED is untouched and stays open."
    )
}


NEW = [
    # ---- Step 1: AGGREGATE-OWNED-FIELD -----------------------------------
    entry(
        "OWNERSHIP-COPY-OP-UNPRODUCED",
        "OwnershipOp::Copy is constructed by no route in the tree, so the copy rules are decided by unit tests alone",
        "MEASURED by card STABILITY-REEVAL-4 (wave 40) with rg over crates and compiler: OwnershipOp::Copy is "
        "CONSTRUCTED at exactly four places, all in crates/qcore-core/tests/unit/ownership.rs (:80, :715, :727, "
        ":817). The one non-test occurrence, crates/qcore-frontend/src/whole_program.rs:716, is a CONSUMER - an "
        "arm of an exhaustive no-identity match. copy_owned has one caller (ownership.rs:466). So no route on "
        "any pipeline emits Copy, and BOTH the aggregate non-copyability rule AGGREGATE-OWNED-FIELD landed "
        "(ownership.rs:680, Binding::Aggregate arm :682) and the older plain-owner copy rule are decided by "
        "hand-built operation lists. The defect is inherited, not introduced by the card - but the card counts "
        "non-copyability among the four IR rules it landed, and a reader of 'four rules, all green' does not "
        "learn that one of them is reachable from nothing. This is evidence about the paths the suite took. "
        "DISCHARGE: emit Copy from a route (which is a semantic card, not an audit's edit), OR record the "
        "variant as model-only in ownership.rs's own doc and in constraint 15k, so the rule is read as a model "
        "property rather than as route coverage."
    ),
    entry(
        "OWNED-FIELD-IR-REFUSALS-UNREACHED",
        "AggregateAtOwnerPosition and NotAnAggregate are named in a standing constraint and reached by no source program",
        "MEASURED by card STABILITY-REEVAL-4 (wave 40). AggregateAtOwnerPosition has eight construction sites "
        "(crates/qcore-core/src/ownership.rs:617, :647, :697, :825, :920, :969, :1059, :1073) and NotAnAggregate "
        "one (:1047); both are asserted only at crates/qcore-core/tests/unit/ownership.rs:779 and :800, over "
        "hand-built operation lists. On the admitting route the record card's own Q052 arm (FieldIsNotABuffer, "
        "'read the field you mean first') fires first, as AGGREGATE-OWNED-FIELD's own section 5.2 table shows. "
        "Yet AGENTS.md's Q054 amendment states that the condition 'now carries ... the IR's own "
        "AggregateAtOwnerPosition and NotAnAggregate' - a standing-constraint sentence about a condition no "
        "source program reaches. Fail-closed in direction, so this is a coverage finding and not a soundness "
        "one. DISCHARGE: a source fixture that reaches either refusal through the real pipeline, OR an amended "
        "Q054 sentence saying the two IR errors are carried but unreached today and naming what would reach "
        "them."
    ),
    entry(
        "OWNED-FIELD-BOUNDARIES-ARE-ABSENCE",
        "AGGREGATE-VIEW-FIELD, AGGREGATE-NESTED and AGGREGATE-TASK-CAPTURE are absence, not refusals a test drives by name",
        "MEASURED by card STABILITY-REEVAL-4 (wave 40), the distinction this cadence exists to draw. "
        "AGGREGATE-VIEW-FIELD: a view spelling reaches FieldType::from_spelling "
        "(crates/qcore-core/src/source_record.rs:88, two constructors I32 and Bytes), returns None, and refuses "
        "as UnknownFieldType at crates/qcore-frontend/src/source_record.rs:447 with a Q051 message naming "
        "AGGREGATE-FIELD-TYPES, a DIFFERENT residual; the only tests on that arm use u8 (source_record.rs:291) "
        "and Buffer (aggregate_owned_field.rs:514), and NO test writes a view field. AGGREGATE-NESTED: "
        "'type A { b: B }' takes the identical arm and the identical message, and no nested-record declaration "
        "exists anywhere in the tree (every 'type NAME {' in compiler/tests and crates/*/tests was read). "
        "AGGREGATE-TASK-CAPTURE: pure absence - route dispatch selects the task route at "
        "crates/qcore-verification/src/driver.rs:1168 and the record/buffer route at :1277, so a module that "
        "spawns never reaches the record route, and no test combines spawn with a record declaration. So of "
        "three registered boundaries, ZERO are refused by their own name. DISCHARGE: per boundary, a test that "
        "drives the refusal and asserts the boundary's own name in the message, OR a restatement in the card's "
        "record and in constraint 15k that the boundary is an unspellable construct rather than a checked "
        "refusal."
    ),
    # ---- Step 1: LOOPS-ON-COMPOSED-ROUTES ---------------------------------
    entry(
        "LOOPWINDOWS-LEAN-IS-A-TRANSCRIPTION",
        "QCore.LoopWindows is an independently written model linked to std_buffer::last_uses by a doc comment only",
        "MEASURED by card STABILITY-REEVAL-4 (wave 40), the JOINED-PROVENANCE-BOUND-DESCENT-PREMISE shape. "
        "QCore.LoopWindows.lastUse (proof/QCore/LoopWindows.lean:122) is defined over its own inductive Stmt a "
        "(:75) with its own tagging (:89, :96, :106). The Rust std_buffer::last_uses "
        "(crates/qcore-core/src/std_buffer.rs:1219) is defined over BufferEvent with "
        "custody_names_at_any_depth (:1169). The link is a DOC COMMENT (std_buffer.rs:1213-1217) and nothing "
        "mechanical: no extraction, no differential evaluation, no digest, and no Rust test references "
        "QCore.LoopWindows. Two gaps this hides, both measured. (1) the_unrolling_is_loop_v1s "
        "(LoopWindows.lean:345) - the ONLY theorem tying the model to LOOP-V1 - is stated at "
        "QCore.StdBuffer.Event, whose eight constructors (proof/QCore/StdBuffer.lean:47) are "
        "bind/acquire/store/load/extent/release/line/ret; rg 'lend' over that file returns nothing, while the "
        "Rust BufferEvent has sixteen constructors including Lend, Repeat, Construct and the four Field*. So "
        "the theorem asserting 'this is LOOP-V1's unrolling' is proved over an alphabet with NO LOAN IN IT, "
        "and the general theorems keep `uses` abstract, which is what lets the gap through. (2) the "
        "non-vacuity witness a_top_level_only_window_misses_a_body_use (:387) is decided over "
        "[.leaf 0, .loop 2 [.leaf 1]] with uses = (. == 1) - the model's own scenario, not the route's events. "
        "PROVED: properties of the Lean model. OBSERVED: that the Rust function is that model. DISCHARGE: a "
        "correspondence mechanism (a generated digest, a differential test over shared vectors, or an "
        "extraction) binding last_uses to lastUse; OR an instantiation of the LOOP-V1 theorem at an alphabet "
        "carrying Lend; OR a sentence in constraint 15h and in std_buffer.rs naming the transcription premise."
    ),
    entry(
        "LOOPWINDOWS-ADMITTED-SHAPES-TAUTOLOGY",
        "the_admitted_body_shapes_are_exactly_these proves a two-constructor enum's constructors are in the list of its own constructors",
        "MEASURED by card STABILITY-REEVAL-4 (wave 40). proof/QCore/LoopWindows.lean:445 proves "
        "'for all s : AdmittedInBody, s is in [.access, .plain]' by 'cases s <;> simp'. AdmittedInBody has "
        "exactly those two constructors, so the statement is a tautology about a Lean enum and relates to "
        "check_buffer_program in no way. LOOPS-ON-COMPOSED-ROUTES' section 4 theorem table presents it as 'the "
        "body shapes admitted beside a loan, as one list'. Low severity and recorded so the theorem is not "
        "later cited as coverage of the route's admitted set. DISCHARGE: restate the theorem's entry in the "
        "card's record as a well-formedness check on the Lean enum, OR relate AdmittedInBody to the Rust "
        "admitted set by a checked correspondence, which is LOOPWINDOWS-LEAN-IS-A-TRANSCRIPTION's mechanism."
    ),
    entry(
        "COMPOSED-VIEW-WINDOW-INERT",
        "std_string::view_uses_at_any_depth returns the empty vector for every input, and constraint 15h names it as the composed route's window function",
        "MEASURED by card STABILITY-REEVAL-4 (wave 40), reading crates/qcore-core/src/std_string.rs:1130-1158. "
        "The inner buffer_view_uses returns Vec::new() for every BufferEvent arm except Repeat, whose arm "
        "flat_maps over a body that can hold only the same constructors, so the function returns the empty "
        "vector FOR EVERY POSSIBLE INPUT, by construction. last_view_uses (:1177) therefore computes exactly "
        "the map it computed before the card. The code says so in its own comment (:1125-1129) and "
        "LOOPS-ON-COMPOSED-ROUTES' report says so in its section 2 ('last_view_uses was exact as it stood ... "
        "the real admitting arm was the Repeat arm of check_string_program'), and the card's mutations M2/M5 "
        "target that real arm. AGENTS.md constraint 15h (lines 1266-1268) does NOT carry the caveat: it reads "
        "'on BOTH routes it runs from the loan's opening to the last top-level statement naming the loan AT ANY "
        "DEPTH (std_buffer::last_uses; std_string::view_uses_at_any_depth)'. The constraint is not wrong - it "
        "describes the real mechanism two sentences later - but the one clause that names a function is "
        "misleading, and a reader will believe the two routes have parallel window computations when one is a "
        "no-op. Neither function is named by any test (rg over compiler/tests and crates/*/tests returns zero "
        "hits for last_view_uses). Not repaired by this card: DOCS-COMPACT-8 owns that prose. DISCHARGE: amend "
        "15h to attribute the composed route's repair to check_string_program's Repeat arm and record "
        "view_uses_at_any_depth as written-ahead-of-its-grammar, OR land a body grammar that lets a view be "
        "read inside a loop, which makes the recursion live."
    ),
    entry(
        "WINDOW-FIELD-ARM-UNREACHED",
        "custody_names_at_any_depth's six aggregate arms are unreachable today and become load-bearing in the admitting direction the day OWNED-FIELD-LEND-WINDOW is lifted",
        "MEASURED by card STABILITY-REEVAL-4 (wave 40), the CLAUDE.md step-4 shape: a rule correct today "
        "because a construct is unreachable. custody_names_at_any_depth (crates/qcore-core/src/std_buffer.rs:1169) "
        "has an explicit arm returning nothing for Construct, Field, FieldStore, FieldLoad, FieldExtent and "
        "FieldRelease (:1181-1192), justified in-comment by the refusal OWNED-FIELD-LEND-WINDOW. The ordering "
        "was measured rather than trusted: check_buffer_program returns Err(OWNED-FIELD-LEND-WINDOW) at "
        ":2677-2691 for any program that composes an owned field and contains a Lend, and "
        "Err(OWNED-FIELD-LOOP-BODY) at :2713-2733 for an owned-field event in a Repeat body, BOTH BEFORE the "
        "block at :2737 that calls lend_windows -> last_uses. last_uses has exactly two callers (:1269 and the "
        "test at loops_on_composed_routes.rs:571). So the six arms are unreachable in every program the checker "
        "admits that far - and the day either refusal is lifted, the window silently FORGETS every field-path "
        "name, giving a SHORTER window, which is the ADMITTING direction invariant 1 constrains. Measured "
        "beside it: there is no admitted program anywhere in the tree with an owned field inside or beside a "
        "loop or a lend; the pair test the_owned_field_refusals_survive_a_for_beside_a_loan "
        "(compiler/tests/loops_on_composed_routes.rs:977) drives three combinations and all three are "
        "REFUSALS (:1006, :1030, :1048), so the two cards' interaction is pinned as mutual refusal only. "
        "DISCHARGE: the card that lifts OWNED-FIELD-LEND-WINDOW or OWNED-FIELD-LOOP-BODY must, in the same "
        "change, give those six arms their real names and a test that shows the window lengthening - or the "
        "arms must be made an explicit refusal rather than an empty vector."
    ),
    # ---- Step 1: CT42 ------------------------------------------------------
    entry(
        "CT42-CELL-DERIVER-LEAN-PARITY-UNBOUND",
        "The socket-cell deriver and its Lean model are bound by hand only, so every CT42 'proved' claim is proved of the Lean model",
        "MEASURED by card STABILITY-REEVAL-4 (wave 40). PROVED, in Lean, of a Lean re-model: the derivation's "
        "direction and its three refusals - proof/QCore/SocketFootprint.lean:234 "
        "(an_unresolved_connect_is_refused), :239 (an_unqueued_accept_is_refused), :245 "
        "(an_orphaned_connector_is_refused), :255, :184, and the composition with the unedited decideDisjoint "
        "at :273 and :280, plus the T09 narrowing in proof/QCore/SocketCellRoute.lean:74. OBSERVED, not "
        "proved: that the Rust deriver (crates/qcore-core/src/socket_footprint.rs:380) IS that model. There is "
        "no correspondence test, no digest and no shared term; no file under compiler/tests/ or crates/*/tests/ "
        "reads proof/, and socket_footprint.rs carries no transcription pin, unlike "
        "crates/qcore-core/tests/lifetime_correspondence.rs, which exists for another module. The registered "
        "SOCKET-CELL-COUPLING-UNPROVED accurately names a DIFFERENT gap (proof/QCore/StdSocket.lean:132-349 "
        "carries ledger and stream theorems and no theorem about peer coupling); it does not name this one. "
        "DISCHARGE: a transcription pin binding the Rust walk to the Lean term (the mechanism "
        "PROOF-LEG-DIGESTS-UNCROSSCHECKED asks for, applied here), OR one sentence in socket_footprint.rs's "
        "module doc and in SOCKET-CELL-COUPLING-UNPROVED's why naming the transcription premise."
    ),
    entry(
        "CT42-ARM-UNREACHABILITY-UNPINNED",
        "The two Err fallback arms are unreachable only because two hand-written walks agree, and nothing tests the agreement",
        "MEASURED by card STABILITY-REEVAL-4 (wave 40). Both arms are now Err - "
        "crates/qcore-core/src/socket_footprint.rs:419 (UnpairedConnect) and :448 (UnpairedAccept), each "
        "carrying Q085 (:215) and the offending statement index (:226) - which repairs what "
        "CT42-CELL-FALLBACK-ARM-DIRECTION found. Reachability re-measured: the only production path is "
        "crates/qcore-verification/src/driver.rs:5476 and :5657 -> check_socket_cells (socket_footprint.rs:507) "
        "-> derive_socket_cells (:361), whose FIRST LINE (:362) is check_socket_program(program), and the "
        "transport checker refuses both conditions first (crates/qcore-core/src/std_socket.rs:2022-2031 for the "
        "unqueued accept, :1476-1484 for the unresolved target). So neither arm is reachable from a source "
        "program. The arms ARE driven - crates/qcore-core/tests/unit/socket_footprint.rs:319 and :355 call "
        "derive_cells_of_admitted directly, a bypass published for exactly this at socket_footprint.rs:369-378 "
        "- so this is not an arm nothing verifies; it is one verified ONLY off-route. The unreachability rests "
        "on an UNSTATED and UNTESTED coupling: both walks read the same resolve_addresses "
        "(std_socket.rs:1742; socket_footprint.rs:363) and queue connectors identically "
        "(std_socket.rs:1981-1985; socket_footprint.rs:424-427). An edit to either walk (backlog depth, a new "
        "endpoint form) could make an admitted program reach an arm - fail-closed, so not a soundness hazard, "
        "but it would turn an admitted program into a Q085 with no test naming why. DISCHARGE: a test asserting "
        "derive_cells_of_admitted returns Ok for every admitted program of the route's fixture set, OR a proof "
        "that the two pending queues coincide."
    ),
    # ---- Step 1: TSAN ------------------------------------------------------
    entry(
        "TSAN-HEADER-INVISIBILITY-UNENFORCED",
        "Nothing in the workspace reads the forcing header's text, so TSAN-FORCED-HARNESS-SYNC-INVISIBILITY's discharge has no mechanism",
        "MEASURED by card STABILITY-REEVAL-4 (wave 40). The named reproduction hook_visible_to_tsan exists ONLY "
        "at docs/reports/tsan-rows-2/artifacts/mutate.sh:139-141 and nothing in the workspace runs it. Nothing "
        "reads the header's text: no test asserts that SCHEDULE_FORCING "
        "(compiler/tests/support/thread_sanitizer.rs:173) contains QCORE_FORCED_UNINSTRUMENTED (:199); there is "
        "no checksum and no generated-file pin. compiler/tests/task_route_tsan.rs:507 checks the EMITTED C for "
        "harness symbols, which is the opposite direction. The measured failure mode IS caught behaviourally: "
        "overlapping_writers_are_reported_under_every_schedule (task_route_tsan.rs:624) asserts races >= 1 over "
        "Schedule::ALL (thread_sanitizer.rs:386, which includes ForcedReverse) on two subjects, exactly the row "
        "the v1 pthread header made go unreported "
        "(docs/reports/tsan-rows-2/artifacts/catch-point-reverse-mutex-edge.log). But that is coverage by "
        "COINCIDENCE: nothing names that test as the header's invisibility guard, and a visible-sync design "
        "that synchronised only the gate-open and not between worker bodies would hide other races while "
        "leaving that row green. Its sibling TSAN-FORCED-GATE-OPENED-BY-THE-JOIN was checked at the same time "
        "and is NOT in this condition - it is pinned in both directions by "
        "a_join_deleted_without_opening_the_gate_hides_the_race_under_forcing (task_route_tsan.rs:711, asserting "
        "races == 0 and schedule.is_empty() at :719-729 and races >= 1 unforced at :733-738). DISCHARGE: a "
        "cheap textual tripwire (the header must carry no_sanitize(\"thread\") on every qcore_forced_ function) "
        "plus a doc line naming task_route_tsan.rs:624 as the behavioural guard; OR promote hook_visible_to_tsan "
        "to an #[ignore]d workspace mutation test on the measure_catch_rates_for_the_report precedent "
        "(task_route_tsan.rs:834)."
    ),
    # ---- Step 1: CT07 / proof legs ----------------------------------------
    entry(
        "PROOF-LEG-DIGESTS-UNCROSSCHECKED",
        "All four proof legs pin a Lean digest as a bare Rust literal that nothing compares with the Lean term, and an existing discharge cites a mechanism that does not exist",
        "MEASURED by card STABILITY-REEVAL-4 (wave 40). CT07's pin is "
        "compiler/tests/ct07_proof_leg.rs:75 (const CT07_LEAN_LEDGER_DIGEST: u64 = 8876998389998880787), checked "
        "at :532; its Lean twin is proof/QCore/Ct07LeaseLedger.lean:209 (theorem ct07_ledger_digest ... := by "
        "decide). A DISCRIMINATION control exists - a_single_writer_mutant_... (:752) ends with assert_ne! "
        "(:799), and the leg does not rest on the digest alone (:527, :534-539, and the LEFT_HALF/RIGHT_HALF "
        "literals at :237 and :239). A RE-TRANSCRIPTION control does not: the two literals are independent "
        "copies, nothing compares them, no test under compiler/tests/ or crates/ reads any .lean file (only doc "
        "comments mention them), and proof/AuditCt07LeaseLedger.lean only prints axioms (:15-40) - it proves the "
        "Lean digest matches the LEAN term, never the Rust constant. Someone who re-derived the digest from new "
        "bytes and updated both literals would be green in both gates. THIS IS THE WHOLE FAMILY: "
        "compiler/tests/ct12_ct14_proof_reach.rs:63 (CT01), ct03_proof_leg.rs:55, ct37_proof_leg.rs:49, "
        "ct07_proof_leg.rs:75 - four bare literals, zero cross-checks. SHARP CONSEQUENCE FOR AN EXISTING ENTRY: "
        "LEASE-BOUNDARY-TRANSCRIPTION-PREMISE's discharge prescribes 'bind the Lean term to the Rust pin by a "
        "generated digest the way CT01's proof leg binds its trace' - measured, CT01's leg does no such "
        "binding, so that discharge condition names a mechanism that does not exist. DISCHARGE: one script in "
        "the standing gate that extracts each 'theorem ct*_digest : ... = N' from proof/QCore/ and compares it "
        "with the corresponding const in compiler/tests/, shown red by mutation; AND correct "
        "LEASE-BOUNDARY-TRANSCRIPTION-PREMISE's discharge sentence (an integrator amendment, not this card's)."
    ),
    entry(
        "CT07-F02-PREMISE-PIN-IS-ERROR-BLIND",
        "The CT07 leg's F02 premise pin asserts only is_err(), so any other future refusal keeps it green while the premise has moved",
        "MEASURED by card STABILITY-REEVAL-4 (wave 40). compiler/tests/ct07_proof_leg.rs:588-595 asserts "
        "AdmittedCausalOrder::derive(&artifact.observation).is_err() with a message naming "
        "CT07-PROOF-LEG-PATTERN-UNAVAILABLE. It pins neither the error variant nor the refused index, so the "
        "premise it records - 'the F02 trace route is closed BECAUSE OF THE LEASE' - is not what it checks: any "
        "other refusal of this run (a changed observation shape, an unrelated derive rejection) keeps the "
        "assertion green while the premise has silently moved. The sharper assertion is available for free: "
        "constraint 15n's own proved facts refusal_points_at_a_refused_event and refusal_is_the_first, and the "
        "sibling pin index: 2 in compiler/tests/trace_alphabet_lease_boundary.rs, already establish which event "
        "and which position a lease refusal names. The registered CT07-PROOF-LEG-F02-PREMISE-PIN names the "
        "alphabet-growth staleness correctly and does not name this one. DISCHARGE: assert the refusal's "
        "variant and index rather than is_err(), so an unrelated refusal cannot stand in for the premise."
    ),
    entry(
        "PROOF-GATED-LIVENESS-CHECK-FILE-SCOPED",
        "The wave-38 proof-stage liveness check reads the fixture at file scope and rejects only ignore, not cfg",
        "MEASURED by card STABILITY-REEVAL-4 (wave 40). "
        "proof_gated_cases_name_a_live_test_that_reads_the_cases_own_fixture (compiler/tests/ct_corpus.rs:7823) "
        "is NON-VACUOUS for CT07: it reads the named file, finds fn "
        "ct07_artifact_is_the_lean_proved_disjoint_halves_shape (ct07_proof_leg.rs:492), checks the attribute "
        "block carries #[test] (:491) and no ignore, and requires the literal "
        "include_str!(\"fixtures/ct/CT07/case.qcore\"), present at :71 and used by the named test at :493. Two "
        "shape gaps in the CHECK, neither biting CT07 today: (a) it asserts that the FILE contains the "
        "include_str!, never that the NAMED test uses it, so a second test in the same file satisfies it; (b) "
        "its attribute scan rejects only ignore, so a #[cfg(target_os = ...)] on a proof-leg test would leave "
        "the proof numerator green while the stage ran nowhere off that host - which is "
        "CT-COMPOSED-NATIVE-NON-LINUX-VACUOUS-PASS's shape reaching the proof count. DISCHARGE: scope the "
        "include_str! check to the named function's body, and reject a #[cfg( in the attribute block (or "
        "require the case's evidence record to state the host scope), each arm shown red by mutation."
    ),
    entry(
        "CLANG-RECORDING-UNBOUND-TO-ITS-RUN",
        "The required clang gate key is satisfied by any sibling text file, so a borrowed recording is indistinguishable from an own one",
        "MEASURED by card STABILITY-REEVAL-4 (wave 40), sharpening INTEGRATION-GATE-CLANG-CLAIM-BORROWED from "
        "'three files reuse a recording' to 'the check cannot tell'. clang_recording() "
        "(docs/reports/remaining-d02/build_matrix.py:182-196) checks only that a file named clang-version.txt "
        "sits in the gate file's own directory or a gate/ subdirectory beside it (:187-189), that the claim "
        "parses as 'present: <banner>' or 'absent: <banner>' (:288-295), that the banner equals the recording's "
        "FIRST LINE (:302-307), and - only IF clang-version.exit happens to exist (:193) - that the recorded "
        "exit agrees with the presence (:308-313). There is no re-invocation of clang, no host identity, no "
        "timestamp, no tree or commit binding and no QCORE_CLANG cross-check, and the exit arm is OPTIONAL. A "
        "clang-version.txt copied from any other report directory passes every arm provided the clang string is "
        "copied with it, which is exactly what the three integration gates do (each disclosing it in its own "
        "note, so nothing was concealed; loops-on-composed-routes and tsan-rows-2 share a byte-identical "
        "recording). DISCHARGE: require the recording to carry a run identity that the gate file also names (the "
        "run directory, a timestamp, or the measured tip's sha) and check the pair, shown red by mutation; OR "
        "record that the key is a disclosure convention rather than a check, and say so in AGENTS.md's gate "
        "section so a green regeneration is not read as evidence Clang was present at that run."
    ),
    entry(
        "F02-AGREEMENT-TEST-SURFACED-BY-PROSE-ONLY",
        "No CI job, gate script or standing command passes --ignored, so the F02 Windows pin disagreement is discoverable only by reading a sentence",
        "MEASURED by card STABILITY-REEVAL-4 (wave 40), sharpening F02-WINDOWS-PIN-STALE-SINCE-WAVE-36 rather "
        "than replacing it. The test the wave-38 close landed, the_windows_f02_pins_agree_with_the_linux_ones "
        "(compiler/tests/f02_windows_pin_agreement.rs:101-111), is #[ignore]d by design (un-ignoring it would "
        "redden the standing gate) and still reproduces its red: the literals still disagree - "
        "compiler/tests/f02_trace_relation.rs:96-97 read bfa572914cb96413 / 9cbe67a70a420d52 and "
        "compiler/tests/windows_threads.rs:31,36 read 6df369991465940e / 0da423be2f16b8bb - and the extraction "
        "anchors are intact, so its un-ignored companion both_f02_pin_sites_are_still_where_this_test_reads_them "
        "(:75-97) is measuring what it claims. But rg over .github/, scripts/, AGENTS.md and CLAUDE.md finds NO "
        "invocation of --ignored or --include-ignored; .github/workflows/ci.yml runs plain cargo test "
        "--workspace. The only surfaces are a person typing the documented command, the two tracker entries, "
        "and prose in AGENTS.md and two close reports. A reader of a green gate sees nothing. DISCHARGE: a "
        "documented, non-gate command (a scripts/ entry or a CI job allowed to fail) that runs the ignored "
        "family and reports, OR one sentence beside every citation of the test saying that no automatic surface "
        "exists and the tracker entry is the record."
    ),
    # ---- Step 3: semantic edits without a card ----------------------------
    entry(
        "UNCARDED-BLIND-TO-MERGE-PROSE",
        "reconciliation_sweep.py --uncarded excludes every merge by construction, and the integrator's own prose lives only in merges",
        "MEASURED by card STABILITY-REEVAL-4 (wave 40). uncarded_commits "
        "(scripts/reconciliation_sweep.py:547-587) computes diff-tree -m --first-parent and SKIPS any commit "
        "whose file list contains a docs/reports/*/README.md (:580-584). Every wave-39 merge's first-parent "
        "diff carries several (verified for b6d1f101: five report READMEs), so all six merges of the wave are "
        "excluded - and the merges are exactly where the integrator's own edits live, as both the wave-38 and "
        "wave-39 closes say in their own hand_sweep notes ('the integrator's own edits live INSIDE the merges "
        "... which --uncarded, by construction, cannot see'). The tool's docstring says a merge carrying a "
        "report 'lands here'; it does not. This card's own run over e878b362..HEAD returned 18 commits, 16 "
        "card-branch WIP commits and two WAVE-39-CLOSE WIP commits plus one structural integrator commit "
        "(f11ccbba, this card's wave-opening commit, the 05bbd149 and 477b733d convention) - zero unexplained "
        "- while MERGED-GATE-ROW-LEAN-HAND-EDIT, found by hand at three merges, is invisible to it. DISCHARGE: "
        "a mode that lists each merge's combined-diff lines present in neither parent (git show --cc), or a "
        "close step that runs git show --cc <merge> -- AGENTS.md CLAUDE.md per merge and records a verdict on "
        "each such hunk."
    ),
    entry(
        "MERGED-GATE-ROW-LEAN-HAND-EDIT",
        "Three wave-39 merges hand-edited the Lean count inside a merged-gate row attributed to another card's named run",
        "MEASURED by card STABILITY-REEVAL-4 (wave 40) with git show --cc -U0 <merge> -- AGENTS.md, whose ++ "
        "lines are in neither parent and are therefore the integrator's own. At e80b58d6, 45a33285 and "
        "b6d1f101 the ++ line is CT07-TWO-THREAD-PROOF-LEG's merged-gate row VERBATIM with only the Lean number "
        "replaced by the current tree count (139, 140, 141) - the row still reading 'Measured by card "
        "CT07-TWO-THREAD-PROOF-LEG at its own tip 81f783ce', whose "
        "docs/reports/ct07-two-thread-proof-leg/gate.json records lean_audits: 138 and whose merge message says "
        "'Lean 138' (the workspace figure, 4644, does match the card's gate). So a figure the named run did not "
        "produce wore that run's label at three consecutive merges. This is exactly "
        "GATE-ROW-FIGURE-ATTRIBUTION-DRIFT, the defect the lean_audits gate key was created for in wave 37 and "
        "made REQUIRED in wave 38 - the key does not reach here, because it guards build_matrix.py --gate and "
        "not AGENTS.md's hand-maintained prose row. The wave-39 close REPAIRED both prose lines at 24505b83 "
        "and the row today reads 'Measured by the wave-39 close at its own tip 11edf7eb'. The finding is that "
        "the mechanism recurred three times within one wave and nothing prevents it recurring: a close is the "
        "only catch, and a close is not a gate. Checked and clean beside it: the follow-up counts are honest at "
        "every merge (232/234/231/233 against the matrix's own lengths), and constraint 14's ledger carries no "
        "hand-written prose union at b6d1f101 - both wave-39 census bullets came from their own branches. "
        "DISCHARGE: a checker that reads AGENTS.md's merged-gate row and refuses when its figures disagree with "
        "the gate.json it names, shown red by mutation; OR a recorded rule that a merge may only replace the "
        "whole row with a measurement it owns, never edit a figure inside a row labelled with another run."
    ),
    entry(
        "INTEGRATION-TIP-CITATION-UNREACHABLE",
        "All five cited wave-39 integration tips are non-ancestors of main, and two are the only record of an integration figure",
        "MEASURED by card STABILITY-REEVAL-4 (wave 40): git merge-base --is-ancestor <sha> HEAD exits 1 for "
        "4786ae4c, 3fa88fc6, 601acde6, f1fcd14d and ecfef946. All five are the PRE-AMENDMENT merges - e.g. "
        "4786ae4c and the in-history b6d1f101 have identical parents and differ in exactly three files "
        "(AGENTS.md by one line, the new integration-gate.json, the regenerated matrix) - so the code is "
        "identical and the test figures are honest. But the citations point at commits git log will never show "
        "and gc will eventually drop, AGENTS.md's own merged-gate row among them ('4,727 / 0 / 10 at "
        "4786ae4c'). The three published docs/reports/<card>/integration-gate.json files were checked against "
        "the tree at their in-history merges and AGREE: loops-on-composed-routes 4727 / 141 at b6d1f101 (141 "
        "proof/Audit*.lean), aggregate-owned-field 4677 / 139 at e80b58d6, tsan-rows-2 4683 / 139 at 0150c904, "
        "each matching its merge message. DISCHARGE: record the in-history merge sha beside the measured tip in "
        "each integration-gate.json and in any ledger row citing one, OR state in the gate note that the "
        "measured tip is the pre-amendment commit and name its successor."
    ),
    entry(
        "INTEGRATION-GATE-NOT-PUBLISHED-FOR-EVERY-MERGE",
        "Two wave-39 merges state an integration figure differing from their card's gate and publish no integration-gate.json",
        "MEASURED by card STABILITY-REEVAL-4 (wave 40). 0d9e922f (CT07) and 45a33285 (CT42) state integration "
        "figures - 4620 / 138 at f1fcd14d and 4703 / 140 at ecfef946 - each differing from that card's own "
        "gate.json (4644 / 138 and 4697 / 140), and neither publishes a docs/reports/<card>/integration-gate.json. "
        "Both cited tips are non-ancestors of main (INTEGRATION-TIP-CITATION-UNREACHABLE), so those two "
        "figures have no record outside an unreachable commit's message. AGENTS.md describes the convention as "
        "covering 'the three integration gates the integrator measured at the merges where no card's gate could "
        "agree with the merged tree', which is true of the three that were published and silent about these "
        "two. The CLAUDE.md convention the wave-39 close wrote says an integrator who cannot regenerate the "
        "matrix with any card's gate.json measures the tip and writes the file; it does not say what to do when "
        "the integrator measures a tip for another reason. DISCHARGE: publish an integration-gate.json for every "
        "merge that measures a tip, OR state the rule that a merge whose message states a figure differing from "
        "the card's gate must publish one, and apply it retrospectively to these two by a dated appended note "
        "(constraint 21)."
    ),
    entry(
        "GATE-SUMMARY-COUNT-LOOSE-GREP-ARTIFACT",
        "The ledger's merged-gate row publishes 240 summaries with a parenthetical that does not reproduce; the true strict count is 238 and two sections double, not four",
        "MEASURED by card STABILITY-REEVAL-4 (wave 40) on its own gate log by a shipped script "
        "(docs/reports/stability-reeval-4/scripts/count_summaries.py): 236 test targets (227 Running + 9 "
        "doc-test sections), 238 lines matching the strict '^test result: ', 240 matching the loose "
        "'^test result', and exactly TWO targets printing more than one summary - Doc-tests qcore_core and "
        "Doc-tests qcore_verifier_host. So 236 + 2 = 238 is the summary count, and the extra two in the loose "
        "count are the two tests whose NAMES begin result_ "
        "(result_handlers_collect_siblings_before_rethrowing_exact_worker_errors, "
        "result_arm_bindings_neither_leak_nor_resolve_their_own_scrutinee) - the artifact STABILITY-REEVAL-3 "
        "named for its own run ('a loose grep -c ^test result reads 235 for the two result_ test names'). "
        "AGENTS.md's merged-gate row publishes '240 result summaries over 236 test targets (227 Running plus 9 "
        "doc-test sections; FOUR of those sections print two summaries each)': the doubled-section count is "
        "wrong and 240 is the loose grep's number, with the parenthetical making 236 + 4 = 240 self-consistent, "
        "which is how the error survives reading. The counting method also CHANGED WITHIN WAVE 39: the row "
        "written at merge 45a33285 published '235 result summaries over 233 test targets ... TWO of those "
        "sections print two summaries each' - 233 + 2 = 235, the strict count, correct. No verdict, no ledger "
        "figure and no test depends on the number; it is a published measurement that does not reproduce. Not "
        "repaired by this card: DOCS-COMPACT-8 owns that prose and this card must not edit a figure into a "
        "document it does not own. DISCHARGE: re-derive the row's summary arithmetic from a gate log with a "
        "strict matcher and publish the three numbers separately (targets, summaries, doubled sections), OR "
        "drop the parenthetical and publish only what a gate.json key can carry."
    ),
    entry(
        "RECONCILIATION-VERIFY-CLAIM-PERISHABLE",
        "A --check verify claim may quote a mutable pointer, so a frozen close's record goes permanently red on an ordinary later action",
        "MEASURED by card STABILITY-REEVAL-4 (wave 40), a LIVE instance. Running the wave-39 close's own "
        "documented command at this base - python3 scripts/reconciliation_sweep.py . --since e878b362 --check "
        "docs/reports/wave39-close/reconciliation.json - exits 1 with the complaint "
        "'ct07-two-thread-proof-leg#for-the-integrator#4 verify[2]: docs/development/increments/README.md does "
        "NOT carry the asserted text \"Current record file: [wave-39.md](wave-39.md)\" - the verdict says "
        "applied and the named location disagrees'. The claim quotes a MUTABLE POINTER, and this card's own "
        "wave-opening commit f11ccbba - the same convention as 05bbd149 (wave 38) and 477b733d (wave 37) - "
        "moved it to wave-40.md. So the record is now permanently red through no fault of the close, and the "
        "next wave-opening commit was CERTAIN to happen when the claim was written. This is the failure mode "
        "_normalise's own docstring (scripts/reconciliation_sweep.py:713-717) names as the reason verification "
        "arms get switched off. The content --check itself is sound and non-vacuous: _verify_claim (:728-766) "
        "opens the named file, rejects path escapes (:748-751) and tests a whitespace-normalised substring "
        "(:758), and coverage is 100% in both waves (wave 38, 6 of 6 applied items carry a non-empty verify; "
        "wave 39, 31 of 31), with the unverifiable escape hatch (:797-816) unused. DISCHARGE: a recorded rule "
        "that a verify claim quotes DURABLE text (never a current-file pointer, a count or a date), OR a "
        "--check mode that distinguishes 'applied and later legitimately superseded' from 'never applied', "
        "with the wave-39 claim reclassified under whichever lands."
    ),
    # ---- Step 4: what a queued card would make load-bearing ---------------
    entry(
        "LEDGER-TSAN-BOUNDARY-SENTENCES-STALE",
        "AGENTS.md presents as 'measured, not claimed' that ThreadSanitizer cannot run any task-route program, and says no TSan row exists anywhere - both false since waves 37 and 39",
        "MEASURED by card STABILITY-REEVAL-4 (wave 40), two live false sentences in the standing constraints. "
        "(1) AGENTS.md:1252-1253 reads 'Three boundaries are MEASURED, NOT CLAIMED: ThreadSanitizer cannot run "
        "any program this route emits, lease or not (TASK-ROUTE-TSAN-SEGV)'. TASK-ROUTE-TSAN-SEGV was RETIRED "
        "in wave 37 against its own discharge condition, met word for word "
        "(docs/development/increments/wave-37.md:305, and :257 'It is not the route's' - the SEGV was diagnosed "
        "as ThreadSanitizer's missing thrd_create interceptor), it is ABSENT from named_followups, and "
        "compiler/tests/task_route_tsan.rs today runs TSan over SIX route-emitted subjects (SUBJECTS at :133, "
        "the CT07 and CT46 fixtures themselves at :103 and :106) through a -include pthread binding that "
        "changes no emitted byte. So a sentence labelled 'measured' asserts the opposite of what the suite "
        "measures. (2) AGENTS.md:1635 reads '...and no TSan row exists anywhere in this repository', false "
        "since TASK-ROUTE-TSAN (wave 37) and TSAN-ROWS-2 (wave 39). The manifest and "
        "compiler/tests/fixtures/ct/CT46/expected.json received the wave-39 close's TSan citation correction "
        "(retiring CT46-NOTES-PREDATE-TSAN-ROW, which is itself absent from named_followups), so these two "
        "AGENTS.md instances reach NO tracker; wave 39's own compiler/tests/ct07_proof_leg.rs:58 and "
        "docs/reports/ct07-two-thread-proof-leg/README.md:143 also still cite the retired name. Not repaired by "
        "this card: DOCS-COMPACT-8 owns that prose and an audit lands no semantic change. DISCHARGE: rewrite "
        "both sentences to the measured position (TSan runs these programs under the harness's pthread "
        "binding; the route's own emission is unchanged) and re-cite TASK-ROUTE-TSAN / TSAN-ROWS-2 in place of "
        "the retired name, in AGENTS.md and in the two wave-39 citations."
    ),
    entry(
        "CT-PROOF-FOURTH-CASE-UNTRACKED",
        "The measured survey's conclusion that CT50 is the only remaining F02-pattern proof-leg candidate reaches no tracker",
        "MEASURED by card STABILITY-REEVAL-4 (wave 40). The proof numerator is 4 of 47 and the four legs do NOT "
        "share a binding pattern: CT01, CT37 and CT03 bind through AdmittedCausalOrder::derive plus the F02 "
        "digest fold (compiler/tests/ct01_proof_leg.rs:41-44 via ct12_ct14_proof_reach.rs, ct37_proof_leg.rs:41-44, "
        "ct03_proof_leg.rs:47-50), while CT07 binds through the scoped-task LEDGER's lease replay "
        "(ct07_proof_leg.rs:64-70) - Rust ScopedTaskLedger states against Lean QCore.Ct07LeaseLedger.Step / "
        "replay. So 'proof N of 47' counts two kinds of evidence and no manifest sentence says so. CT07's "
        "pattern had three things for free that no other candidate has: an already-audited Lean model of a "
        "PRODUCTION data structure (wave 31's WriteLendTasks), a production type to drive step for step, and a "
        "decision procedure already in the checker - so the leg needed one new general theorem. Within the F02 "
        "pattern the measured survey is explicit that CT50 is the ONLY remaining candidate whose default "
        "reference run both derives and exhibits its scenario "
        "(docs/reports/proof-leg-batch/README.md:358-361; CT12/CT13/CT15 are refused at derive with "
        "UnattributableEvent { index: 1 }, :155-165). That sentence - the single most decision-relevant fact "
        "for the next proof-leg card - is in a report and in NO named_followup, the WAVE-38-QUEUE-NAMES-UNTRACKED "
        "shape one wave on. DISCHARGE: register the survey's conclusion (or a decision that a fourth pattern "
        "is to be invented instead), and state in the CT ledger row that the proof numerator counts more than "
        "one binding pattern."
    ),
]


def main():
    show = "--show" in sys.argv
    matrix = json.loads(MATRIX.read_text())
    followups = matrix["named_followups"]
    present = {e.get("id") for e in followups}

    for rid, ground in RETIRE.items():
        if rid in present:
            print("RETIRE  %s" % rid)
            print("        ground: %s" % ground[:120] + "...")
            followups[:] = [e for e in followups if e.get("id") != rid]
        else:
            print("RETIRE  %s - already absent (idempotent re-run)" % rid)

    added = 0
    for e in NEW:
        if e["id"] in present:
            print("SKIP    %s - already registered" % e["id"])
            continue
        followups.append(e)
        added += 1
        print("ADD     %s" % e["id"])

    print("\nnamed_followups: %d -> %d (+%d, -%d)"
          % (len(present), len(followups), added, len(present) - len(followups) + added))

    if show:
        print("\n--show: nothing written.")
        return

    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
    print("\nwrote %s; now run build_matrix.py --gate docs/reports/stability-reeval-4/gate.json" % MATRIX)


if __name__ == "__main__":
    main()

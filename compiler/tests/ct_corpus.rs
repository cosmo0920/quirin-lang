// SPDX-License-Identifier: Apache-2.0
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! W6 CT conformance harness (`docs/GENERALIZATION_PLAN.md` §W6 task 1).
//!
//! The harness is keyed to `docs/reevaluation/ct_manifest.json`, the tracking
//! table for the spec §55 language conformance corpus (CT01–CT50). Every case
//! the manifest claims is `mapped` must be backed by a real QCore source
//! fixture under `compiler/tests/fixtures/ct/<ID>/` and is executed through
//! the actual source pipeline (`compile_files`: parse → elaborate → check,
//! plus the CFG reference execution for accepted executables). Reevaluation
//! spec §5.1: hand-constructed Rust events do not mark a source conformance
//! case complete, so this harness never builds semantics by hand.
//!
//! Unmapped cases are counted explicitly, never silently skipped, and the
//! pinned `MAPPED_CASES` table below is an intentional expectation: advancing
//! a case requires updating the manifest, adding an executed fixture, and
//! editing this table deliberately. A manifest status the fixture set cannot
//! back (mapped without a fixture, a fixture without a mapped status, or an
//! unknown status vocabulary) fails the harness.
//!
//! Namespace discipline (§W6 task 3): CT is the language corpus only. WM/VP/LC
//! probe regressions and per-library provider cases must not be added here.
//!
//! All five declared routes now carry at least one mapped case. The task route
//! (`ScopedCopyTasksV1`,
//! `ScopedOwnedTasksV1`, and its `ScopedReadSharesV1` / `ScopedErrorPayloadV1`
//! extensions) is the original eleven cases plus CT03/CT04/CT05/CT17/CT18. The
//! mutex route (`source.mutex.guard.v1` lowering onto the unedited
//! `CheckedMutexInvariantV1` checker, cards T03 and T03-F) is
//! CT26/CT27/CT28/CT29/CT30/CT31/CT32/CT33/CT34/CT48. The epoch route
//! (`source.epoch.bound.v1` lowering onto the unedited `FlatEpochBoundV1`
//! checker, cards T08-EPOCH-SRC and T08-SRC-CT) is CT35/CT36/CT38/CT39. The
//! `legacy_c` route (card CT-CTWIN, joined by CT40 in the wave-17 CT lane after
//! card CTWIN-PUBLISH closed the hazard that kept it unmapped) is
//! CT40/CT43/CT45: those three cases have no QCore
//! source form at all — the constructs they name are C constructs — so their
//! artifact is a C twin analyzed through the real unrestricted-C gateway
//! (`validate_unrestricted_c11` then `analyze_unrestricted_c11`), which is why
//! the route's vocabulary carries `parse` and `legacy_c_twin` and no
//! `elaborate`. That route's evidence is gated on Clang rather than on a
//! `target_os`, which the stage entry records as a `tool_gate`. The
//! `sequential` route, declared and empty since schema 2, gained CT06 and CT11
//! (card `CT-WIRE-LANE`): `source.string.v1`'s `borrow_utf8` bridge is this
//! corpus's first **local** shared alias — a second name for an owned buffer's
//! own bytes, bound in the same frame, with no task and no second party — which
//! is the form the wave-3 lane recorded as missing when it left CT11 unmapped.
//! Card `CT-WIRE-LANE-2` (wave 28) grows the same route by four: CT07 and CT09
//! are its first **accepted** cases, each with a sanitized native run of its
//! own emitted C (`CT07-SLICE-PERMISSIONS`'s range-wise disjointness — CT07's
//! evidence since upgraded to `SLICE-PERMISSIONS-WRITE-LEND`'s two disjoint
//! write authorities by card `CT07-WRITE-REMAP`, wave 30 — and
//! `AGGREGATES-V1`'s distinct record fields), and CT08 and CT21 are two more
//! rejections (an overlapping-range write at `Q078`; `QUANT-REGION-SOURCE`'s
//! undischarged cleanup pack at `Q305`). The same card maps CT23 on the new
//! `composed_epoch` route — `source.epoch.tasks.v1`, a flat-epoch plan over a
//! checked task region — where `error(close);` abandoning a child's granted
//! credit on the creator's own error edge refuses at `Q635`
//! (card `CT23-CREDIT-ERROR-PATH`). Card `CT22-WIRE` (wave 29) puts that
//! route's first **accepted** case beside it: CT22's parent-local `error
//! region … quant open … discharge` after every join (card
//! `REGION-CREDIT-JOINS`), whose published abandoned/restored/spent
//! quantities are compared wholesale under the same `epoch_credit_split`
//! verdict mode, widened for the accepting shape.
//!
//! One case's native evidence is not a total transcript. CT49 asks for exactly
//! that (`validate partial order, not exact total trace`), and card
//! `OBS-REORDERING` supplies it: a source line in a worker body is written by
//! that worker's own thread, so the program declares `qcore.obs-reordering.v1`
//! and its `stderr` is compared window by window and emitter by emitter while
//! its `stdout` stays byte-compared. Two controls bound that: a parent-only
//! program keeps a strict relation, and a window whose emitters could write the
//! same bytes refuses the declaration outright.
//! Rejection cases on any route pin the whole ordered diagnostic cascade rather
//! than its head, because a refusal that silently dropped a custody obligation
//! would still satisfy a head-only assertion, and each carries at least one
//! control so the refusal is not merely "the profile cannot express this at
//! all". The epoch route's *accepted* cases carry the symmetric obligation: an
//! epoch that checks has said nothing until its published certificate is
//! measured, so `epoch_bound_figures` pins every figure on it.
//!
//! Manifest schema 2 (card T00-CT-SCHEMA, tasks T00.1-T00.9) makes the per-case
//! stage record derived rather than declared. Schema 1 wrote a bare word per
//! stage — and two of its six names, `cfg` and `type_checker`, conflated four
//! real passes and had to be *read differently per route*, so a task-route case
//! and a mutex-route case rendered identically while running different
//! machinery. Schema 2 names the passes that exist, gives each route its own
//! stage vocabulary, and requires an `executed` entry to name the runner, the
//! artifact, the invocation count and the exact `target_os` gate behind it.
//! `derive_stage_plan` recomputes every one of those from this harness's own
//! dispatch, the fixture's own scenario data and the diagnostic codes the
//! compiler really produced, and `run_mapped_case` refuses any disagreement, so
//! the manifest is checked evidence and not a self-certifying claim. A route
//! that has no such pass records it as *structurally absent*
//! (`not_applicable`): the mutex route emits directly from its checked event
//! list, so it has no CFG-shaped stage at all, which schema 1 could only spell
//! as an indefinite `cfg: unassessed`. Sanitizer evidence is gated: only a case
//! a runner really compiles with `-fsanitize=…` may read `native_sanitized:
//! executed`, and the five cases whose sanitizer evidence is an archived
//! transcript must read `recorded_not_gated` with a pointer to it.

#[cfg(target_os = "windows")]
#[path = "../../tests/support/windows_c.rs"]
mod windows_c;

#[path = "../../tests/support/diagnostic_sink.rs"]
mod diagnostic_sink;

#[path = "../../tests/support/mutex_observation.rs"]
mod mutex_observation;

#[path = "../../tests/support/tracked_files.rs"]
mod tracked_files;

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use qcore_compiler::{
    AnalysisStatus, C11TranslationUnit, CompileOptions, ExecutionOutcome, LegacyDiagnosticKind,
    LegacyProgramSummary, ResourceContracts, SourceFile, TraceEvent, UnrestrictedLegacyResult,
    analyze_unrestricted_c11, compile_files, format_diagnostic, validate_unrestricted_c11,
};
// Card `CT-CTWIN-LANE` (wave 35): CT46's declared crossing is asserted against
// the checked event list and against the transferability judgment's own
// published classification, so the fixture pins the model's vocabulary rather
// than this harness's prose.
use qcore_core::mutex_invariant::MutexInvariantEvent;
use qcore_core::transfer::MUTEX_SHARE_CROSSING;
use serde::Deserialize;

/// Pinned mapping expectation. Exactly these manifest entries carry
/// `mapping_status: "mapped"`; everything else must still read `unmapped`.
const MAPPED_CASES: &[&str] = &[
    "CT01", "CT02", "CT03", "CT04", "CT05", "CT06", "CT07", "CT08", "CT09", "CT10", "CT11", "CT12",
    "CT13", "CT14", "CT15", "CT16", "CT17", "CT18", "CT19", "CT20", "CT21", "CT22", "CT23", "CT24",
    "CT25", "CT26", "CT27", "CT28", "CT29", "CT30", "CT31", "CT32", "CT33", "CT34", "CT35", "CT36",
    "CT37", "CT38", "CT39", "CT40", "CT41", "CT42", "CT43", "CT44", "CT45", "CT46", "CT47", "CT48",
    "CT49", "CT50",
];
/// The `legacy_c` route (card CT-CTWIN, extended by the wave-17 CT lane). These
/// three cases name C constructs that
/// no QCore syntax expresses, so their artifact is a `.c` twin analyzed through
/// the real unrestricted-C gateway. Each pins the whole verdict — the unit
/// result, the complete per-function status map and the complete ordered
/// diagnostic inventory — and carries at least one control that removes exactly
/// the construct the scenario names, so the recorded `Unknown` is attributable
/// to that construct rather than to "the projection cannot express this at
/// all". CT40 additionally carries two controls pinned for *not* moving the
/// verdict, which is what bounds its mapping to the publication protocol rather
/// than to the atomicity its title names.
const LEGACY_C_TWIN_CASES: &[&str] = &["CT40", "CT43", "CT45"];
/// The `source.mutex.guard.v1` rejection cases (T03's lowering onto the unedited
/// `CheckedMutexInvariantV1` checker, extended by T03-F's bounded task
/// composition, `wait` announcement and guard-scoped facts, and by CT33-RANKS's
/// opt-in lock-rank discipline, and by CT34-GROWTH's opt-in protection growth).
/// Each pins its exact
/// ordered diagnostic code vector, not merely its head, and carries at least one
/// control that distinguishes the refused construct from the admitted one.
const MUTEX_REJECT_CASES: &[&str] = &[
    "CT27", "CT28", "CT29", "CT30", "CT31", "CT32", "CT33", "CT34", "CT48",
];
/// The one accepted mutex-guard case, executed end to end and re-run under
/// ASan/UBSan/LSan.
const MUTEX_SANITIZED_CASE: &str = "CT26";
/// The one `source.mutex.guard.v1` **declared-crossing** case (card
/// `MUTEX-SHARE-CROSS`, wave 32; mapped by card `CT-CTWIN-LANE`, wave 35).
///
/// This mode is an *accept* that deliberately stops at the checker. The module
/// is admitted — `compile_files` returns it on the checked mutex route — but
/// the C backend renders nothing for it and `interpret_mutex_program` refuses
/// its event list, because the `Spawn` the crossing introduces is outside the
/// emitted subset. So it is neither `checked_mutex_sanitized` (which requires a
/// sanitized native run) nor `checked_mutex_reject` (it is not a refusal), and
/// it carries its own mode rather than being squeezed into either. The emission
/// boundary is *pinned* rather than omitted: the case states the exact refusal
/// its emission produces, and one control is the same route without the
/// crossing clause, which does emit and does interpret — so the missing
/// emission is attributable to the crossing and not to the route.
const MUTEX_CROSSING_CASE: &str = "CT46";
/// Task-route rejection cases (cards T05 and T06; CT44 and CT47 added by card
/// `CT-WIRE-LANE-4`, wave 32). Same discipline as the mutex
/// rejections: the exact ordered code vector plus at least one control that
/// isolates the refused element from the surrounding program.
///
/// The two wave-32 additions each carry more than the minimum, because each
/// maps a case whose scenario has a word this language gives no referent:
/// CT47 supplies **every** contract its refusal message enumerates and pins the
/// modeless shape's grammar refusal beside them, and CT44 pins both disjuncts
/// of its scenario plus two accepted controls that separate the refused
/// construct from its position in both directions.
const SOURCE_REJECT_CASES: &[&str] = &["CT04", "CT05", "CT18", "CT44", "CT47"];
/// `sequential`-route rejection cases (card `CT-WIRE-LANE`, wave 26). The same
/// discipline one route over: the exact ordered code vector plus controls that
/// keep the refusal from being vacuous. These are the first mapped cases on the
/// `sequential` route, which has been declared and empty since schema 2.
///
/// Both are `source.string.v1`'s `borrow_utf8` bridge, which is the corpus's
/// first **local** shared alias — a second name for an owned buffer's bytes,
/// bound in the same frame, with no task and no other party. CT11 measures that
/// the alias blocks the reconstruction of exclusive authority; CT06 measures
/// that the alias's identity survives being copied into a differently named
/// variable.
///
/// CT42 joins them in wave 39 (card `CT42-CELL-ROUTE`): a `source.socket.v1`
/// program asking `disjoint client server;` of the two ends of one accepted
/// connection, refused by the **unedited** `DisjointFootprintsV1` checker as
/// `SharedProtocolCell` (`Q664`) — the socket route is a non-concurrent source
/// route, which is what `sequential` names in this harness.
const SEQUENTIAL_REJECT_CASES: &[&str] = &["CT06", "CT08", "CT11", "CT21", "CT42"];
/// CT10 is a check-time field-independence question over real task captures.
/// It has its own route because no scoped-task ledger is run and calling its
/// checker stage `task_ledger_check` would overstate the evidence.
const BITFIELD_REJECT_CASE: &str = "CT10";
/// `sequential`-route **accept** cases whose evidence is a sanitized native run
/// of the case's own emitted C compared per stream with the route's checked
/// observation (card `CT-WIRE-LANE-2`, wave 28). CT09
/// is card `AGGREGATES-V1`'s three-field record program, whose sanitized byte
/// comparison is the executed layout *observation* (`AGGREGATE-CT09-LAYOUT`:
/// an observation, never the layout proof the case's `proof` stage would need).
///
/// **CT07 left this list at card `CT07-TWO-THREAD-REMAP` (wave 37)** for
/// `HEAP_SCENARIO_CASES`, because its scenario's word "independently" now has a
/// two-thread artifact to rest on. Its sequential program did not leave the
/// corpus with it: it is retained as that case's `sequential_write_halves`
/// accept control, and its three sequential reject twins as that case's
/// sequential rejection controls, so the remap is a strict superset of the
/// evidence wave 30 published rather than a swap.
const SEQUENTIAL_SANITIZED_CASES: &[&str] = &["CT09"];
/// Card `CT01-PROOF-LEG`: the cases whose `proof` stage is executed, each
/// bound to the named workspace test that checks the case's own artifact
/// against the Lean-proved decision procedure. The gate is pinned per case and
/// fail-closed — every case outside this table keeps `unassessed`, and
/// advancing another case means editing this table deliberately (the
/// sanitizer-gate precedent). CT01's theorem covers the checked artifact's
/// reference run and its admitted renderings; native execution stays covered
/// by observation only, and a grown proof numerator is evidence about the
/// case's artifact, not language conformance.
const PROOF_GATED_CASES: &[(&str, &str)] = &[
    (
        "CT01",
        "compiler/tests/ct01_proof_leg.rs::ct01_artifact_is_the_lean_proved_owned_transfer_shape",
    ),
    // Card `CT-PROOF-LEG-2`: CT37's completion-capsule procedure
    // (QCore.Ct37CompletionCapsule, AuditCt37CompletionCapsule.lean).
    (
        "CT37",
        "compiler/tests/ct37_proof_leg.rs::ct37_artifact_is_the_lean_proved_completion_capsule_shape",
    ),
    // Card `CT03-PROOF-ALPHABET`: CT03's shared-reads procedure over the
    // extended F02 alphabet (QCore.Ct03ReadShares over QCore.TaskTraceWrites,
    // AuditCt03ReadShares.lean).
    (
        "CT03",
        "compiler/tests/ct03_proof_leg.rs::ct03_artifact_is_the_lean_proved_shared_reads_shape",
    ),
    // Card `CT07-TWO-THREAD-PROOF-LEG` (wave 39): CT07's disjointness
    // procedure over the scoped-task ledger's lease set
    // (QCore.Ct07LeaseLedger over QCore.WriteLendTasks,
    // AuditCt07LeaseLedger.lean). The first leg bound **through the ledger**
    // rather than the F02 trace relation, which refuses this run's lease
    // events by constraint 15n's decided boundary; the alphabet did not grow.
    (
        "CT07",
        "compiler/tests/ct07_proof_leg.rs::ct07_artifact_is_the_lean_proved_disjoint_halves_shape",
    ),
    // Card `CT-PROOF-LEG-3` (wave 41): CT50's lifetime-identity procedure over
    // the F02 observation alphabet (QCore.Ct50LifetimeIdentity,
    // AuditCt50LifetimeIdentity.lean). The fourth leg bound through the trace
    // relation, and the first whose closure over admitted renderings is the
    // **degenerate** case — the run is single-actor, so the relation admits
    // nothing but the run itself, proved as
    // `an_admitted_rendering_is_the_reference_run` and measured on the
    // artifact by enumerating `admitted_linearizations`. The alphabet did not
    // grow. Scope: the capability half of the scenario — the numeric address
    // reuse it names is invisible to this alphabet and stays with
    // `run_native_task_scenarios`' observation, unproved.
    (
        "CT50",
        "compiler/tests/ct50_proof_leg.rs::ct50_artifact_is_the_lean_proved_lifetime_identity_shape",
    ),
];
/// The `source.epoch.tasks.v1` composed cases, whose verdict mode pins
/// published credit quantities wholesale because an epoch that merely checks
/// has measured nothing. CT23 (cards `CT23-CREDIT-ERROR-PATH` and
/// `CT-WIRE-LANE-2`) is the refusal: the credit a flat epoch granted a
/// creation, abandoned on the creator's own error edge — the exact cascade
/// plus every admitting control's published split. CT22 (cards
/// `REGION-CREDIT-JOINS` and `CT22-WIRE`, wave 29) is the mode's first
/// **accepted** case: the parent's own `error region … quant open …
/// discharge` after every join, whose published evidence — the credit-recovery
/// split and the region's re-derived `abandoned_credit` — is compared
/// wholesale, with the duty-free pack and the join-before-region span fact
/// re-asserted on the published objects. CT25 (cards `REGION-JOINS-DUTIES`,
/// `REGION-JOINS-LOAN` and `CT-WIRE-LANE-3`, wave 31) is the mode's second
/// accepted case and its first **adopting** one: the parent lends the worker a
/// counted read share of its own owner, joins — the loan expires there, under
/// the unedited task ledger — and only then does its region adopt the owner
/// (`move owner;`) and its discharge destroy it, abandoning the remainder. The
/// widened mode pins the region evidence's `released` duty list and the
/// derived capture list beside the credit split, and the duty premise splits
/// in two: a case pinning no released owner keeps CT22's duty-free-pack
/// assertion (no `Acquire` in the published body), while a case pinning one
/// must publish exactly that duty as a body `Acquire` — the adoption, checked
/// on the published operations rather than trusted.
const EPOCH_CREDIT_SPLIT_CASES: &[&str] = &["CT22", "CT23", "CT24", "CT25"];
/// Card `CT-COMPOSED-NATIVE` (wave 32), discharging the follow-on CT22 and
/// CT25 named by their own `unassessed` emission and native rows: the
/// composed-epoch cases whose **own** emitted C this harness really compiles
/// and runs. Pinned and fail-closed on the `SEQUENTIAL_SANITIZED_CASES` /
/// `PROOF_GATED_CASES` precedent — a case outside this table is never compiled
/// or run by `run_composed_native`, and the table and the fixtures'
/// `composed_native` blocks must agree in both directions, so neither a block
/// without a table entry nor a table entry without a block can exist.
///
/// CT23 is deliberately absent and must stay absent: it is the route's
/// refusal, so `compile_files` never reaches the backend on it and there is no
/// emission of its own to run. That is a *stated* reason, not a sweep — its
/// `c_emission`/`native_execution` rows are `not_applicable` for exactly that
/// reason and this card does not touch them.
const COMPOSED_NATIVE_CASES: &[&str] = &["CT22", "CT25"];
/// The case whose native evidence is compared **up to a declared partial
/// order** rather than byte for byte (card `CT-WIRE-LANE`, wave 26). Exactly
/// one, and its own controls pin both boundaries: a program with no worker-body
/// line keeps a strict relation, and a window whose emitters could write the
/// same bytes refuses the declaration outright.
const REORDERING_CASES: &[&str] = &["CT49"];
/// The `source.epoch.bound.v1` rejection cases (card T08-EPOCH-SRC's lowering
/// onto the unedited `FlatEpochBoundV1` checker, mapped by card T08-SRC-CT).
/// Same discipline again: the exact ordered code vector plus at least one
/// control that distinguishes the refused epoch from an admitted one.
const EPOCH_REJECT_CASES: &[&str] = &["CT38", "CT39"];
/// The accepted `source.epoch.bound.v1` cases. Their evidence is not the bare
/// verdict — an epoch that checks says nothing on its own — but the exact
/// certificate figures the checker published, which is why they carry their own
/// verdict mode rather than reusing `accept`.
const EPOCH_FIGURE_CASES: &[&str] = &["CT35", "CT36"];
/// Task-route accept cases whose native evidence is a sanitized run with a heap
/// witness: every Buffer allocation and release is recorded by address, so
/// "counted once" and "reclaims nothing" are checked by identity rather than by
/// the emitted trace alone.
///
/// **CT07 joins at card `CT07-TWO-THREAD-REMAP` (wave 37)**, as CT03's write
/// twin: where CT03 is two readers over one allocation, CT07 is two *writers*
/// over disjoint byte ranges of one allocation, each exercising a range write
/// lease in its own worker body (card `WRITE-LEND-TASKS-EXERCISE`, wave 36).
/// The heap witness carries the same weight in both — a second allocation would
/// make "two parties, one object" false without changing a single trace line.
const HEAP_SCENARIO_CASES: &[&str] = &["CT03", "CT07", "CT17"];
const TOTAL_CASES: usize = 50;

/// Manifest schema 2 (card T00-CT-SCHEMA, tasks T00.1-T00.9). Schema 1's `cfg`
/// and `type_checker` are gone: they conflated four real passes and their
/// reading differed per route, so a mutex-route case and a task-route case could
/// render identically while executing different machinery. These are the passes
/// that exist, in pipeline order. Every mapped case declares its `route`, and a
/// stage outside that route's vocabulary is *structurally absent*
/// (`not_applicable`), never "unassessed forever".
const STAGE_NAMES: &[&str] = &[
    "parse",
    "elaborate",
    "footprint_lowering",
    "footprint_check",
    "task_lowering",
    "task_ledger_check",
    "mutex_lowering",
    "mutex_invariant_check",
    "epoch_lowering",
    "epoch_bound_check",
    "sequential_whole_program",
    "sequential_reference_execution",
    "task_reference_execution",
    "mutex_reference_execution",
    "c_emission",
    "native_execution",
    "native_sanitized",
    "legacy_c_twin",
    "atomic_publication_check",
    "proof",
];
/// `executed` needs an executor, an artifact and an observation count;
/// `recorded_not_gated` needs a `record` pointer and is never counted as
/// executed (T00 finding O4: the ASan transcripts are re-run by nothing);
/// `not_applicable` is a stage this route or this refusal provably never
/// reaches; `unassessed` is an open stage.
const STAGE_STATUSES: &[&str] = &[
    "executed",
    "recorded_not_gated",
    "not_applicable",
    "unassessed",
];
/// Per-route stage vocabulary. A route's list is exactly the stages that route
/// *can* execute; the harness refuses any other stage claiming anything but
/// `not_applicable`. `sequential` and `legacy_c` are declared and currently
/// carry no mapped case — declaring them is what makes "this route has no such
/// stage" a structural statement rather than a note.
const ROUTE_STAGES: &[(&str, &[&str])] = &[
    (
        "atomic_publication",
        &["parse", "atomic_publication_check", "proof"],
    ),
    (
        "bitfield_concurrency",
        &[
            "parse",
            "elaborate",
            "footprint_lowering",
            "footprint_check",
            "proof",
        ],
    ),
    // `source.epoch.tasks.v1` (card T08-COMPOSE; first mapped case in card
    // CT-WIRE-LANE-2): a flat-epoch plan over a real checked scoped-task
    // region. Both halves lower and check — the credit-recovery gate derives
    // the creator's error edges from the *checked* region, so neither half can
    // be skipped — and an admitted module emits the task route's own C
    // (byte-identical whatever the plan's error arm says), which is why the
    // emission and native stages are in this vocabulary even though no runner
    // executes them today. The composed module publishes no reference
    // observation, so no reference-execution stage exists here.
    (
        "composed_epoch",
        &[
            "parse",
            "elaborate",
            "epoch_lowering",
            "task_lowering",
            "task_ledger_check",
            "epoch_bound_check",
            "c_emission",
            "native_execution",
            "native_sanitized",
            "proof",
        ],
    ),
    // `source.epoch.bound.v1` is check-only by construction: an admitted epoch
    // is an accounting statement about declared shapes, so the driver publishes
    // it with `c_source: None` and `reference_observation: None`. There is
    // nothing to execute, nothing to emit and therefore nothing to sanitize, so
    // this route's vocabulary omits the reference/emission/native stages —
    // including `native_sanitized`, which is structurally absent here rather
    // than an open item. That is the one deviation from card T08-EPOCH-SRC's
    // parked rows, which proposed `native_sanitized: unassessed`; recorded in
    // `docs/reports/wave8-epoch-ct/README.md`.
    (
        "epoch",
        &[
            "parse",
            "elaborate",
            "epoch_lowering",
            "epoch_bound_check",
            "proof",
        ],
    ),
    ("legacy_c", &["parse", "legacy_c_twin", "proof"]),
    (
        "mutex_guard",
        &[
            "parse",
            "elaborate",
            "mutex_lowering",
            "mutex_invariant_check",
            "mutex_reference_execution",
            "c_emission",
            "native_execution",
            "native_sanitized",
            "proof",
        ],
    ),
    (
        "sequential",
        &[
            "parse",
            "elaborate",
            "sequential_whole_program",
            "sequential_reference_execution",
            "c_emission",
            "native_execution",
            "native_sanitized",
            "proof",
        ],
    ),
    (
        "task",
        &[
            "parse",
            "elaborate",
            "task_lowering",
            "task_ledger_check",
            "task_reference_execution",
            "c_emission",
            "native_execution",
            "native_sanitized",
            "proof",
        ],
    ),
];
/// The only executors a manifest stage entry may name. A stage cannot be
/// credited to a runner this file does not have.
const RUNNER_REGISTRY: &[&str] = &[
    "compiler/tests/ct_corpus.rs::atomic_publication::run",
    "compiler/tests/ct_corpus.rs::run_mapped_case",
    "compiler/tests/ct_corpus.rs::run_native_owned_transfer",
    "compiler/tests/ct_corpus.rs::run_native_create_recovery",
    "compiler/tests/ct_corpus.rs::run_native_join_failure",
    "compiler/tests/ct_corpus.rs::run_native_task_scenarios",
    "compiler/tests/ct_corpus.rs::run_native_heap_scenarios",
    "compiler/tests/ct_corpus.rs::run_checked_mutex_sanitized",
    "compiler/tests/ct_corpus.rs::run_legacy_c_twin",
    // Card `CT34-SANITIZED`: the one runner that executes a stage on a control
    // rather than on the case's own fixture.
    "compiler/tests/ct_corpus.rs::run_mutex_controls",
    // Card `CT-CTWIN-LANE` (wave 35): the declared-crossing runner. It asserts
    // the admitted crossing's whole lowered shape — the spawn/join edge, the
    // child's own lock, and the complete guarded-reach vector rebuilt from the
    // event list — then pins the emission refusal and runs the four controls
    // that make "only through the protocol" falsifiable.
    "compiler/tests/ct_corpus.rs::run_mutex_crossing",
    // Card `CT-WIRE-LANE`: the one runner whose native observation is compared
    // up to a declared partial order instead of byte for byte.
    "compiler/tests/ct_corpus.rs::run_reordering_partial_order",
    // Card `CT-WIRE-LANE-2`: the sequential accept runner — one sanitized
    // compile-and-run of the case's own emitted C, compared per stream with
    // the checked module's own observation.
    "compiler/tests/ct_corpus.rs::run_sequential_sanitized",
    // Card `CT01-PROOF-LEG`: the proof-stage runner — the named test that
    // lowers the case's own fixture and checks its artifact against the
    // Lean-proved decision procedure (host-unconditional; the theorems
    // themselves are checked by the standing Lean audit gate).
    "compiler/tests/ct01_proof_leg.rs::ct01_artifact_is_the_lean_proved_owned_transfer_shape",
    // Card `CT-PROOF-LEG-2`: CT37's proof-stage runner, the same shape.
    "compiler/tests/ct37_proof_leg.rs::ct37_artifact_is_the_lean_proved_completion_capsule_shape",
    // Card `CT03-PROOF-ALPHABET`: CT03's proof-stage runner, the same shape.
    "compiler/tests/ct03_proof_leg.rs::ct03_artifact_is_the_lean_proved_shared_reads_shape",
    // Card `CT07-TWO-THREAD-PROOF-LEG` (wave 39): CT07's proof-stage runner —
    // the same shape, bound through the checked module's lease set and the
    // production ledger rather than the F02 digest relation.
    "compiler/tests/ct07_proof_leg.rs::ct07_artifact_is_the_lean_proved_disjoint_halves_shape",
    // Card `CT-PROOF-LEG-3` (wave 41): CT50's proof-stage runner — the CT01
    // shape again, bound through the F02 digest relation, with the admitted
    // set enumerated on the artifact because this run has one actor.
    "compiler/tests/ct50_proof_leg.rs::ct50_artifact_is_the_lean_proved_lifetime_identity_shape",
    // Card `CT-COMPOSED-NATIVE` (wave 32): the composed-epoch accept runner —
    // one sanitized compile-and-run of the case's own emitted C, its standard
    // output compared byte for byte with the reference observation of the
    // module's own checked task half. Registered here deliberately, which is
    // what makes the name legal for a manifest stage entry to cite; the stage
    // rows that cite it are the integrator's lockstep edit with
    // `derive_stage_plan`'s composed-epoch arm, per this card's boundary.
    "compiler/tests/ct_corpus.rs::run_composed_native",
];
/// The only external tools a stage entry may be gated on. A `tool_gate` is not
/// a `host_gate`: the C-twin stages run on every target this workspace builds
/// for and are gated on Clang being present, so recording them as
/// host-unconditional with no further qualification would read as "this ran
/// wherever the tests ran" (T00 finding O3, in the tool direction).
const TOOL_REGISTRY: &[&str] = &["clang"];
/// The sanitizer gate (T00.3). A case may claim `native_sanitized: executed`
/// only if a harness runner really compiles its emitted C with `-fsanitize=…`
/// and runs it. Every other case's sanitizer evidence is a transcript.
const SANITIZED_GATED_CASES: &[&str] = &[
    // CT46 joins at card `MUTEX-SHARE-CROSS-EMISSION` (wave 36): the declared
    // crossing's own emitted C is compiled under ASan/UBSan/LSan and run, and
    // its standard output compared byte for byte with the reference
    // interpreter's observation of the same checked trace. The numerator this
    // moves is evidence about one case's artifact, never conformance
    // (constraint 23).
    "CT03", "CT07", "CT09", "CT17", "CT22", "CT25", "CT26", "CT34", "CT46", "CT49",
];
/// Cases whose sanitizer evidence is an archived transcript that no workspace
/// test re-runs. They must read `recorded_not_gated` with a `record` pointer,
/// never `executed`.
const SANITIZER_TRANSCRIPT_CASES: &[&str] = &["CT14", "CT16", "CT20", "CT37", "CT50"];
/// Depth rank per stage: the deepest executed stage of a case is the one with
/// the greatest rank. Ranks are shared across routes so a task-route ledger
/// check and a mutex-route invariant check sit at the same depth.
const STAGE_DEPTH: &[(&str, u8)] = &[
    ("parse", 0),
    ("elaborate", 1),
    ("footprint_lowering", 2),
    ("task_lowering", 2),
    ("mutex_lowering", 2),
    ("epoch_lowering", 2),
    ("sequential_whole_program", 2),
    ("task_ledger_check", 3),
    ("footprint_check", 3),
    ("mutex_invariant_check", 3),
    ("epoch_bound_check", 3),
    ("task_reference_execution", 4),
    ("mutex_reference_execution", 4),
    ("sequential_reference_execution", 4),
    ("legacy_c_twin", 4),
    ("atomic_publication_check", 4),
    ("c_emission", 5),
    ("native_execution", 6),
    ("native_sanitized", 7),
    ("proof", 8),
];

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    schema_version: u32,
    #[allow(dead_code)]
    note: String,
    stage_vocabulary: StageVocabulary,
    artifact_identity: ArtifactIdentity,
    cases: Vec<ManifestCase>,
}

/// Card `CT-SHARED-FIXTURE-DISCLOSURE`: how many independent programs back the
/// mapped cases, and the reviewed allow-list of byte-identical fixture groups.
/// Required rather than defaulted, so deleting the disclosure fails parsing.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ArtifactIdentity {
    #[allow(dead_code)]
    note: String,
    report: String,
    scan_root: String,
    mapped_cases: usize,
    distinct_source_fixture_digests: usize,
    shared_fixture_groups: Vec<SharedFixtureGroup>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SharedFixtureGroup {
    sha256: String,
    relation: String,
    decision: String,
    members: Vec<SharedFixtureMember>,
    #[allow(dead_code)]
    note: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SharedFixtureMember {
    path: String,
    role: String,
}

/// The vocabulary is carried in the manifest so a reader knows what the words
/// mean, and pinned against this file so data cannot widen it.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StageVocabulary {
    stages: Vec<String>,
    statuses: Vec<String>,
    routes: BTreeMap<String, Vec<String>>,
}

/// One per-case stage entry. A status is only as good as the evidence named
/// beside it, which is why `executed` cannot be written without an executor, an
/// artifact and a count, and why a host-gated runner must state its gate rather
/// than leaving the status host-unconditional (T00 finding O3).
#[derive(Deserialize, PartialEq, Eq, Debug)]
#[serde(deny_unknown_fields)]
struct StageRecord {
    status: String,
    #[serde(default)]
    executor: Option<String>,
    #[serde(default)]
    artifact: Option<String>,
    #[serde(default)]
    observations: Option<usize>,
    #[serde(default)]
    host_gate: Option<Vec<String>>,
    /// External tools the runner needs before it can execute this stage at all
    /// (card CT-CTWIN). The C-twin stages are gated on Clang; every other stage
    /// in this corpus needs no tool and leaves this absent.
    #[serde(default)]
    tool_gate: Option<Vec<String>>,
    #[serde(default)]
    record: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ManifestCase {
    id: String,
    #[allow(dead_code)]
    scenario: String,
    #[allow(dead_code)]
    expected: String,
    spec_section: String,
    mapping_status: String,
    route: Option<String>,
    source_fixture: Option<String>,
    semantic_profile: Option<String>,
    #[serde(default)]
    harness_verdict_mode: Option<String>,
    stages: BTreeMap<String, StageRecord>,
    #[serde(default)]
    evidence_target: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExpectedCase {
    id: String,
    verdict: String,
    entry: Option<String>,
    diagnostics: Vec<ExpectedDiagnostic>,
    outcome: Option<ExpectedOutcome>,
    trace: Option<Vec<String>>,
    stages_executed: Vec<String>,
    #[allow(dead_code)]
    notes: String,
    #[serde(default)]
    evidence_target: Option<String>,
    #[serde(default)]
    native_join_failure: Option<NativeJoinFailure>,
    #[serde(default)]
    native_create_recovery: Option<NativeCreateRecovery>,
    #[serde(default)]
    native_owned_transfer: Option<NativeOwnedTransfer>,
    #[serde(default)]
    positive_control: Option<PositiveControl>,
    #[serde(default)]
    native_task_scenarios: Option<NativeTaskScenarios>,
    #[serde(default)]
    checked_mutex_sanitized: Option<CheckedMutexSanitized>,
    #[serde(default)]
    checked_mutex_reject: Option<CheckedMutexReject>,
    #[serde(default)]
    checked_mutex_crossing: Option<CheckedMutexCrossing>,
    #[serde(default)]
    native_heap_scenarios: Option<NativeHeapScenarios>,
    #[serde(default)]
    source_reject: Option<SourceReject>,
    #[serde(default)]
    bitfield_reject: Option<SourceReject>,
    #[serde(default)]
    sequential_reject: Option<SourceReject>,
    #[serde(default)]
    reordering_partial_order: Option<ReorderingPartialOrder>,
    #[serde(default)]
    sequential_sanitized: Option<SequentialSanitized>,
    #[serde(default)]
    epoch_credit_split: Option<EpochCreditSplit>,
    #[serde(default)]
    composed_native: Option<ComposedNative>,
    #[serde(default)]
    epoch_bound_figures: Option<EpochBoundFigures>,
    #[serde(default)]
    epoch_reject: Option<EpochReject>,
    #[serde(default)]
    legacy_c_twin: Option<LegacyCTwin>,
}

/// A C twin analyzed through the real unrestricted-C gateway (card CT-CTWIN).
///
/// The evidence is the whole verdict, not a convenient part of it: the unit
/// result, its verbatim reason, the complete per-function status map and the
/// complete ordered diagnostic inventory, each rebuilt from the analysis and
/// compared wholesale so a fixture cannot pin only what suits it. `attributed_to`
/// names the construct the CT scenario is about, and the controls are what make
/// that attribution falsifiable: removing exactly that construct must move the
/// verdict.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyCTwin {
    /// Declared resource contracts in force for the case, fixture-relative.
    /// These are trusted premises about a synthetic API, not verified facts.
    contracts: Vec<String>,
    /// The C construct this case's scenario names. Recorded so the control
    /// requirement below has something to be a control *of*.
    attributed_to: String,
    /// `unknown`, `unsafe` or `safe`; any other spelling fails the harness.
    verdict: String,
    /// The unit-level reason, verbatim, for an `unknown` verdict.
    #[serde(default)]
    reason: Option<String>,
    /// Every analyzed function and its exact status.
    function_status: BTreeMap<String, String>,
    /// The whole ordered diagnostic inventory as `function:line:kind`.
    diagnostics: Vec<String>,
    controls: Vec<LegacyControl>,
}

/// One C-twin control. A control either changes the source (`source`) or
/// changes the declared contracts over the case's own bytes (`source` absent),
/// which is the form CT45's `expected` text — "require access contract or
/// report unknown" — actually asks about.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyControl {
    #[serde(default)]
    source: Option<String>,
    contracts: Vec<String>,
    /// `safe`, `unknown` or `unsafe`; any other spelling fails the harness.
    expectation: String,
    #[serde(default)]
    reason: Option<String>,
    function_status: BTreeMap<String, String>,
    diagnostics: Vec<String>,
    #[allow(dead_code)]
    note: String,
}

/// Accepted `source.epoch.bound.v1` evidence (cards T08, T08-EPOCH-SRC,
/// T08-SRC-CT). An admitted epoch is not itself the observation: the route
/// refuses nothing about a program that declares comfortable figures, so
/// "checked" alone would map a case without measuring what CT35 and CT36 ask.
/// What is measured is the certificate the checker published — every nonzero
/// axis of every quantity on it, keyed `field.axis`, plus the task count and
/// the exact premise set — so a changed union rule, a maximized peak or a
/// dropped premise moves a pinned number instead of passing silently.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EpochBoundFigures {
    /// The `SourceExecutionProfile` the module must be published under, so a
    /// case that silently changed route would not still pass.
    execution_profile: String,
    /// Exactly the certificate's premise set. These are declared premises, not
    /// facts this route establishes, and naming them here keeps that visible.
    assumptions: Vec<String>,
    task_count: u64,
    /// Exactly the nonzero `field.axis` entries of the published certificate —
    /// not a chosen subset, which the runner enforces by rebuilding the whole
    /// map from the certificate and comparing it wholesale.
    figures: BTreeMap<String, u64>,
    controls: Vec<EpochControl>,
}

/// A refused `source.epoch.bound.v1` fixture: the exact ordered diagnostic code
/// vector plus the controls that keep the refusal from being vacuous.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EpochReject {
    diagnostic_sequence: Vec<String>,
    controls: Vec<EpochControl>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EpochControl {
    source: String,
    /// `accept` or `reject`; any other spelling fails the harness.
    expectation: String,
    diagnostic_sequence: Vec<String>,
    /// An accepted control states its own certificate, on the same terms as the
    /// case: a control that merely "also checks" would not show what differs.
    #[serde(default)]
    task_count: Option<u64>,
    #[serde(default)]
    figures: BTreeMap<String, u64>,
    #[allow(dead_code)]
    note: String,
}

/// Sanitized task-route native evidence with a heap witness (cards T05, T06).
/// Every scenario compiles the case's own emitted C with the recorded sanitizer
/// flags, includes one fixture header, and compares its stdout with the
/// independently computed reference execution of the same checked module. The
/// header prints each Buffer allocation and release with its address, so a
/// scenario states how many objects existed and how many were reclaimed rather
/// than trusting the trace.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NativeHeapScenarios {
    /// The `SourceExecutionProfile` the module must be published under. A case
    /// that silently changed route would otherwise still pass.
    execution_profile: String,
    alloc_prefix: String,
    free_prefix: String,
    sanitizer_flags: Vec<String>,
    ubsan_options: String,
    rejection_controls: Vec<TaskRejectionControl>,
    /// Programs that must still **check**, carried beside the refusals (card
    /// `CT07-TWO-THREAD-REMAP`, wave 37). A mapping lane that moves a case onto
    /// a stronger artifact must be able to keep admitting the configuration the
    /// case used to rest on; without this, a remap would silently drop an
    /// accept control and read as an upgrade while being a narrowing. Defaulted,
    /// so CT03's and CT17's fixtures are untouched by its addition.
    #[serde(default)]
    accept_controls: Vec<HeapAcceptControl>,
    scenarios: Vec<HeapScenario>,
}

/// A control program that must compile and check. It carries no execution: the
/// claim is admission, and the case's own scenarios carry the running.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct HeapAcceptControl {
    source: String,
    /// The `SourceExecutionProfile` the control must be published under, so a
    /// control cannot silently change route and still read as the same
    /// admission.
    execution_profile: String,
    note: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct HeapScenario {
    name: String,
    /// Fixture header included with `-include`; other fixture headers are
    /// copied beside it so a header may `#include "track.h"`.
    header: String,
    /// `-D` options selecting the injected failure, if any.
    defines: Vec<String>,
    spawn_outcomes: Vec<bool>,
    allocation_outcomes: Vec<bool>,
    #[serde(default)]
    worker_allocation_outcomes: BTreeMap<u64, Vec<bool>>,
    /// LeakSanitizer. Disabled only where the retained allocation *is* the
    /// observation; at least one scenario per case must enable it.
    detect_leaks: bool,
    /// Process exit code, checked with both the quiet and diagnostic sinks.
    /// Zero means the diagnostic run's stdout must equal the reference; a
    /// nonzero code is a trapped abort with a truncated diagnostic observation.
    exit_code: i32,
    /// These stdout constraints apply only to the explicit diagnostic sink.
    /// The ordinary emitted program is separately required to remain quiet.
    stdout_contains: Vec<String>,
    stdout_absent: Vec<String>,
    /// Exact stdout lines that must appear in this relative order.
    stdout_order: Vec<String>,
    heap_allocations: usize,
    heap_releases: usize,
}

/// A refused task-route fixture: the exact ordered diagnostic code vector plus
/// the controls that keep the refusal from being vacuous.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceReject {
    diagnostic_sequence: Vec<String>,
    controls: Vec<SourceControl>,
}

/// Native evidence compared **up to a declared partial order** (card
/// `CT-WIRE-LANE`, wave 26; the relation is `qcore_core::obs_reordering`,
/// landed by card `OBS-REORDERING`).
///
/// Every other native mode in this corpus pins an exact total transcript. This
/// one cannot, and that is the point of the case it maps: the fixture's worker
/// bodies write from their own threads, so the emitted program's byte order is
/// the scheduler's and the reference execution's is one particular schedule.
///
/// What is pinned instead, and what makes the looser comparison evidence rather
/// than a weakening:
///
/// * the **structure** of the derived relation — its window sizes, its emitter
///   count, and which of the two streams is strict — so a relation that
///   silently became one big commuting window would move a number here;
/// * the declared rendering of **both** streams, so the reference itself is
///   still pinned exactly;
/// * `runs` sanitized executions, each of which must lie in the admitted set,
///   and the `stdout` of each compared byte for byte;
/// * two deterministic witnesses that the relation is neither vacuous nor total:
///   a regrouped stream that is admitted and is *not* the declared order, and a
///   reversal inside one emitter that is refused.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReorderingPartialOrder {
    /// The `SourceExecutionProfile` the module must be published under.
    execution_profile: String,
    /// How many times the one sanitized executable is run.
    runs: usize,
    sanitizer_flags: Vec<String>,
    asan_options: String,
    ubsan_options: String,
    /// The reference execution's own rendering of each stream, pinned exactly.
    declared_stdout: Vec<String>,
    declared_stderr: Vec<String>,
    /// The derived relation's shape on `stderr`: one entry per window, in
    /// order, giving that window's record count.
    stderr_window_sizes: Vec<usize>,
    /// How many distinct emitters write to `stderr`.
    stderr_emitters: usize,
    /// Whether each stream's declared relation is strict. `stdout` must be —
    /// every trace write of an emitted task program is the parent's — and
    /// `stderr` must not be, or this case would be measuring byte equality
    /// under another name.
    stdout_relation_strict: bool,
    stderr_relation_strict: bool,
    controls: Vec<ReorderingControl>,
}

/// One control of the partial-order case. Unlike every other control kind here,
/// these are about the **relation** a program declares rather than about a
/// checker verdict: both fixtures check, and what distinguishes them is what
/// their derived relation is.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReorderingControl {
    source: String,
    /// `strict_relation` — the control's own relation is strict on both
    /// streams, and its sanitized native run is compared byte for byte;
    /// `refuse_relation` — the relation cannot be derived at all.
    expectation: String,
    /// For `refuse_relation`: the `ReorderingRefusal` code, and a substring its
    /// message must carry, so a refusal for another reason is not accepted.
    #[serde(default)]
    refusal_code: Option<u32>,
    #[serde(default)]
    refusal_contains: Option<String>,
    #[allow(dead_code)]
    note: String,
}

/// A `sequential`-route **accept** case's evidence (card `CT-WIRE-LANE-2`):
/// the case's own fixture is admitted, its per-stream observation is pinned
/// exactly, and one binary built from its own emitted C with the recorded
/// sanitizer flags is run and compared per stream against that observation.
///
/// The observation this route publishes is per-stream (`out: `/`err: `
/// prefixed lines computed from the checked model, never from the emitted C),
/// which is what makes the native comparison a comparison. An accepted case
/// still needs at least one **refused** control, or the acceptance says only
/// "this route admits programs" — the wave-9 attribution rule read from the
/// accepting side.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SequentialSanitized {
    /// The `SourceExecutionProfile` the module must be published under, so a
    /// case that silently changed route would not still pass.
    execution_profile: String,
    sanitizer_flags: Vec<String>,
    asan_options: String,
    ubsan_options: String,
    /// The checked observation's per-stream projections, pinned exactly.
    declared_stdout: Vec<String>,
    declared_stderr: Vec<String>,
    controls: Vec<SourceControl>,
}

/// The `source.epoch.tasks.v1` credit-disposition evidence (cards
/// `CT23-CREDIT-ERROR-PATH` and `CT-WIRE-LANE-2`; widened for an accepted
/// case by card `CT22-WIRE`, wave 29). What is measured is never a bare
/// verdict: an epoch that merely checks has said nothing, so every admitted
/// module states the credit split the driver published and independently
/// re-derived, and the accounting law `restored + spent = credit` is asserted
/// on each disposition, with the mint refusal on the other side.
///
/// Two case shapes, decided by the cascade:
/// * a **refusal** (CT23) pins `diagnostic_sequence` and publishes no split of
///   its own — the split evidence lives on the admitting controls;
/// * an **acceptance** (CT22) leaves the cascade empty and pins the case's own
///   published figures instead: the credit-recovery split (`restored`/`spent`)
///   and the composed region's `abandoned` credit, each compared wholesale
///   against the driver's re-derived publication.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EpochCreditSplit {
    /// The composed route's profile; a control that silently changed route
    /// would otherwise still pass.
    execution_profile: String,
    /// Empty exactly when the case is accepted.
    diagnostic_sequence: Vec<String>,
    /// The grant the epoch makes the one creation, per axis. Every admitted
    /// module's split — the case's own where it accepts, and every accepted
    /// control's — must add back to exactly this.
    granted: BTreeMap<String, u64>,
    /// Accepted case only: its own published split, per axis.
    #[serde(default)]
    restored: BTreeMap<String, u64>,
    #[serde(default)]
    spent: BTreeMap<String, u64>,
    /// Accepted case only: the region evidence's abandoned parent-local
    /// credit (card `REGION-CREDIT-JOINS`). `Some` exactly when the case is
    /// accepted, because the accepted shape this mode admits is the composed
    /// region's — an acceptance with nothing measured would map nothing.
    #[serde(default)]
    abandoned: Option<u64>,
    /// Accepted case only (card `CT-WIRE-LANE-3`): the owners the discharge
    /// destroyed, by binding name in destruction order — compared wholesale
    /// against the region evidence's own re-derived `released` list. Empty
    /// for the duty-free shape (CT22), whose case then keeps the no-`Acquire`
    /// premise; non-empty for the adopting shape (CT25), whose published body
    /// must carry exactly that many `Acquire` operations — the adoptions.
    #[serde(default)]
    released: Vec<String>,
    /// Accepted case only (card `CT-WIRE-LANE-3`): the derived capture list of
    /// each task-ledger row, in row order, rendered as the plan's own clause
    /// words (`copy`/`move`/`read`). Pinned where the scenario's words hang on
    /// a capture — CT25's *loan expiry* is real only if the capture really is
    /// a loan — and compared against both the checked ledger and the declared
    /// rows, which the route has already reconciled elementwise.
    #[serde(default)]
    captures: Option<Vec<Vec<String>>>,
    controls: Vec<CreditControl>,
}

/// A composed-epoch **accept** case's native evidence (card
/// `CT-COMPOSED-NATIVE`, wave 32). The follow-on `CT22-WIRE` named and
/// `CT-WIRE-LANE-3` repeated is exactly this: an admitted `composed_epoch`
/// module really emits the task route's own C, and until this card no harness
/// runner compiled or ran it, so the emission and native stages were open
/// items.
///
/// What is compared, and it is one thing: the **standard output of the
/// sanitized binary built from this case's own emitted C** against
/// `render_task_observation` of the reference execution of **this same
/// module's own checked task half** (`ComposedEpochModule::tasks`, the
/// unedited `qcore_core::task_execution` interpreter). The rendering is also
/// pinned line by line here, so the comparison is against a stated transcript
/// rather than against whatever the two sides happen to agree on — the
/// `sequential_sanitized` discipline, one route over.
///
/// What it is **not**: the region and the flat-epoch plan erase from emission,
/// so no byte of this run is evidence about a credit quantity, an abandoned
/// remainder or an epoch bound. Those are check-time folds, measured by
/// `epoch_credit_split` and by nothing else. This run observes the program the
/// composed module lowers to, which is precisely why a discharge that lost an
/// adopted duty would show up here (the destroy is a real emitted
/// `qcore_buffer_drop`) while a mis-stated credit would not.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ComposedNative {
    sanitizer_flags: Vec<String>,
    asan_options: String,
    ubsan_options: String,
    /// The reference observation of the module's own checked task half, pinned
    /// line by line. This is the comparison basis, so it is stated rather than
    /// read off the model at run time.
    declared_observation: Vec<String>,
    /// The reference run's root outcome, as the `i32` the entry returns.
    root_outcome_i32: i32,
    /// How many task observations that run produced — the parent plus its
    /// children — so a reference run that silently lost a task would not still
    /// match a shorter transcript.
    reference_tasks: usize,
    #[allow(dead_code)]
    note: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CreditControl {
    source: String,
    /// `accept` or `reject`; any other spelling fails the harness.
    expectation: String,
    diagnostic_sequence: Vec<String>,
    /// An accepted control's published split, per axis; a refused control
    /// publishes none.
    #[serde(default)]
    restored: BTreeMap<String, u64>,
    #[serde(default)]
    spent: BTreeMap<String, u64>,
    /// An accepted control that writes a region pins the abandoned credit it
    /// publishes; one that writes none must publish no region evidence at all
    /// (card `CT22-WIRE`). Absent for every refused control.
    #[serde(default)]
    abandoned: Option<u64>,
    /// An accepted control that writes a region also pins the released duty
    /// list its region publishes (card `CT-WIRE-LANE-3`) — empty where the
    /// duty never reaches the pack, as in the body-release frame-rule control
    /// — so the case's released owner stays attributable to the adoption the
    /// discharge really destroyed. Empty for a refused control.
    #[serde(default)]
    released: Vec<String>,
    #[allow(dead_code)]
    note: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceControl {
    source: String,
    /// `accept` or `reject`; any other spelling fails the harness.
    expectation: String,
    diagnostic_sequence: Vec<String>,
    /// For an accepted control, the `SourceExecutionProfile` it must select.
    #[serde(default)]
    execution_profile: Option<String>,
    #[allow(dead_code)]
    note: String,
}

/// Accepted `source.mutex.guard.v1` evidence. The reference observation is
/// computed by `interpret_mutex_program` from the checked event list — it never
/// reads the emitted C — and the sanitized native run must reproduce it line
/// for line. This is the CT corpus's first gated sanitized-native case (T00's
/// finding O4 recorded that the earlier sanitizer evidence was transcript-only).
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CheckedMutexSanitized {
    sections: usize,
    guards: Vec<u64>,
    sanitizer_flags: Vec<String>,
    asan_options: String,
    ubsan_options: String,
    emitted_c_contains: Vec<String>,
    reference_observation: Vec<String>,
}

/// An admitted `source.mutex.guard.v1` **declared crossing** (card
/// `CT-CTWIN-LANE`, wave 35; the construct is card `MUTEX-SHARE-CROSS`'s).
///
/// The evidence is the lowered shape, not merely "it checked". `reaches` is
/// rebuilt whole from the checked event list rather than looked up by the keys
/// the fixture names, so a fixture cannot pin a convenient subset of the
/// payload accesses — and it is the vector that carries this case's `expected`
/// text, because every entry being guarded is exactly "accepted only through
/// the protocol". `emission_refusal` pins the boundary the mapping does *not*
/// cross: this module is admitted and emits nothing, and saying so in the
/// fixture is what keeps the three unassessed stages a measurement.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CheckedMutexCrossing {
    /// The construct the CT scenario is about, so the controls below have
    /// something to be controls *of*.
    attributed_to: String,
    /// The transferability judgment's classification of the shared payload and
    /// the protocol it admits. Recorded because CT46's own words (`non-Sync but
    /// Send`) have no referent in this language and these are what stand in for
    /// them; the runner checks them against `qcore_core::transfer`'s own data.
    payload: String,
    protocol: String,
    /// The task edge the crossing really lowers to.
    spawn: TaskEdge,
    join: TaskEdge,
    /// The child takes the generation itself; nothing is parked with it.
    child_lock_task: u32,
    /// Every access of the protected payload, in order, with whether it named a
    /// guard. Rebuilt whole from the event list.
    reaches: Vec<GuardedReach>,
    /// Card `MUTEX-SHARE-CROSS-EMISSION` (wave 36). The crossing's own
    /// rendering: what the emitted C must contain, and the observation its
    /// sanitized native run must reproduce byte for byte. This field **replaces**
    /// wave 35's `emission_refusal`, which pinned the boundary the backend has
    /// since crossed; the surviving boundary is pinned by the `accept_unrendered`
    /// control instead, so the fixture still measures both directions.
    emission: CrossingEmission,
    controls: Vec<MutexCrossingControl>,
}

/// The declared crossing's native evidence.
#[derive(Deserialize, PartialEq, Eq, Debug)]
#[serde(deny_unknown_fields)]
struct CrossingEmission {
    /// Fragments of the emitted C that must be present. These are the ones that
    /// make the crossing *real*: a child function, a `thrd_create` naming it, a
    /// `thrd_join`, and the child's own `mtx_lock`/`mtx_unlock` pair.
    emitted_c_contains: Vec<String>,
    /// Text that must not appear **in `main`**: the child's critical section
    /// runs in the child's own function, or the "crossing" would be a rendering
    /// artefact rather than a thread boundary. Checked against the emitted text
    /// from `int main(` onwards, which is exactly the creator's body.
    main_excludes: Vec<String>,
    sanitizer_flags: Vec<String>,
    asan_options: String,
    ubsan_options: String,
    /// The observation `interpret_mutex_program` computes, which the sanitized
    /// native run's standard output must equal line for line.
    reference_observation: Vec<String>,
}

#[derive(Deserialize, PartialEq, Eq, Debug)]
#[serde(deny_unknown_fields)]
struct TaskEdge {
    parent: u32,
    child: u32,
}

#[derive(Deserialize, PartialEq, Eq, Debug)]
#[serde(deny_unknown_fields)]
struct GuardedReach {
    task: u32,
    guarded: bool,
}

/// One declared-crossing control. `accept_emitting` is the emission-boundary
/// control — admitted, rendered, and interpreted to the pinned observation —
/// and `reject` is one of the ways of reaching the payload without the
/// protocol, pinning both its code vector and a distinguishing fragment of its
/// message, so a control cannot pass by reaching the right code for the wrong
/// reason.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MutexCrossingControl {
    source: String,
    /// `accept_emitting` or `reject`; any other spelling fails the harness.
    expectation: String,
    diagnostic_sequence: Vec<String>,
    message_fragment: Option<String>,
    /// `accept_emitting` only: the observation `interpret_mutex_program`
    /// computes from this control's own checked event list.
    reference_observation: Vec<String>,
    /// `accept_unrendered` only (card `MUTEX-SHARE-CROSS-EMISSION`): the
    /// verbatim refusal `interpret_mutex_program` returns for a control that
    /// checks and is still outside **both** rendered shapes. This is where
    /// wave 35's top-level `emission_refusal` pin went: the boundary is still
    /// measured, on a neighbouring shape the backend really does refuse.
    emission_refusal: Option<String>,
    #[allow(dead_code)]
    note: String,
}

/// A refused `source.mutex.guard.v1` fixture: the exact ordered diagnostic code
/// vector plus the controls that keep the refusal from being vacuous.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CheckedMutexReject {
    diagnostic_sequence: Vec<String>,
    controls: Vec<MutexControl>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MutexControl {
    source: String,
    /// `accept`, `accept_growth`, `accept_check_only` or `reject`; any other
    /// spelling fails the harness.
    expectation: String,
    diagnostic_sequence: Vec<String>,
    /// `accept_check_only` only: the exact ordered code vector `compile_files`
    /// produces for a control the *checker* admits but the C backend declines to
    /// render. Pinning it is what keeps a check-only admission from being
    /// reported as a full acceptance, and what makes a backend refusal a
    /// recorded decision rather than an omission. Card CT34-GROWTH's growth
    /// control carried one until card CT34-SANITIZED made the backend render an
    /// admitted growth; the expectation is retained because a future check-only
    /// route needs exactly it.
    #[serde(default)]
    backend_refusal: Vec<String>,
    /// `accept_growth` only (card `CT34-SANITIZED`): the sanitized native run of
    /// this control's own emitted C. A case's own fixture is a refusal that
    /// emits nothing, so the sanitized evidence for a mutex rejection lives on
    /// the control that renders, and the case's native stages name *that* file
    /// as their artifact.
    #[serde(default)]
    sanitized_run: Option<ControlSanitizedRun>,
    #[allow(dead_code)]
    note: String,
}

/// A control the harness compiles with `-fsanitize=…` and runs, comparing its
/// stdout line for line with `interpret_mutex_program` — which is written
/// against the checked event list and never reads the emitted C. At most one
/// control per case may carry this, because the case's native stages name a
/// single artifact.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ControlSanitizedRun {
    sanitizer_flags: Vec<String>,
    asan_options: String,
    ubsan_options: String,
    emitted_c_contains: Vec<String>,
    reference_observation: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NativeTaskScenarios {
    header: String,
    rejection_controls: Vec<TaskRejectionControl>,
    scenarios: Vec<TaskScenario>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TaskRejectionControl {
    source: String,
    diagnostic: ExpectedDiagnostic,
    /// The complete ordered code vector, where the control has one (card
    /// `CT07-TWO-THREAD-REMAP`, wave 37). Wave 30 pinned CT07's sequential
    /// controls this way — one of them refuses in *two* layers, `[Q054, Q053]`,
    /// and asserting only the head would have let either layer fall silent.
    /// Defaulted and empty for every control that pins a head diagnostic only.
    #[serde(default)]
    diagnostic_sequence: Vec<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TaskScenario {
    name: String,
    failed_allocation: usize,
    #[serde(default)]
    allocation_task: u64,
    #[serde(default)]
    worker_allocation_outcomes: BTreeMap<u64, Vec<bool>>,
    failed_create: usize,
    allocation_outcomes: Vec<bool>,
    spawn_outcomes: Vec<bool>,
    outcome: ExpectedOutcome,
    joined_children: usize,
    owned_child_results: Vec<u64>,
    native_stderr: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NativeOwnedTransfer {
    header: String,
    fail_create: bool,
    expected_i32: i32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NativeCreateRecovery {
    header: String,
    unjoined_source: String,
    success_i32: i32,
    recovery_i32: i32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NativeJoinFailure {
    header: String,
    control_i32: i32,
    abort_exit_code: i32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PositiveControl {
    source: String,
    evidence_target: String,
    expected_i32: i32,
    joined_children: usize,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExpectedDiagnostic {
    code: String,
    message_contains: String,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct ExpectedOutcome {
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    ok_value: Option<i32>,
    /// Completion-record identities still live at a normal exit (spec §43.5,
    /// §50.3). Mandatory for an `ok_value` outcome so that a case cannot claim
    /// a normal exit without stating what the capsule still carries; an error
    /// exit carries no completion payload in this model.
    #[serde(default)]
    payload: Option<Vec<u64>>,
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("compiler crate lives one level below the repository root")
        .to_path_buf()
}

fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/ct")
}

fn load_manifest() -> Manifest {
    let path = repo_root().join("docs/reevaluation/ct_manifest.json");
    let text = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
    serde_json::from_str(&text)
        .unwrap_or_else(|error| panic!("cannot parse {}: {error}", path.display()))
}

fn trace_token(event: &TraceEvent) -> String {
    match event {
        TraceEvent::Acquire(resource) => format!("acquire:{resource}"),
        TraceEvent::Release(resource) => format!("release:{resource}"),
        TraceEvent::Transfer(resource) => format!("transfer:{resource}"),
    }
}

/// What this harness derives for one stage of one case: the status, the runner
/// that produced it, how many times that runner invoked it, and the exact set of
/// `target_os` values on which it runs. `host_gate: None` means the stage runs
/// on every host.
#[derive(Clone)]
struct StagePlan {
    status: &'static str,
    executor: Option<&'static str>,
    observations: Option<usize>,
    host_gate: Option<Vec<&'static str>>,
    tool_gate: Option<Vec<&'static str>>,
    /// The file the runner really executed this stage on. `None` means the
    /// case's own fixture, which is what every stage of every case ran on until
    /// card `CT34-SANITIZED`; a stage a runner executed on one of the case's own
    /// declared controls names that file here, and the manifest must then name
    /// exactly it. This is a tighter check than the constant it replaces: the
    /// derived plan now decides the artifact rather than assuming it.
    artifact: Option<String>,
}

impl StagePlan {
    fn absent(status: &'static str) -> Self {
        StagePlan {
            status,
            executor: None,
            observations: None,
            host_gate: None,
            tool_gate: None,
            artifact: None,
        }
    }

    fn executed(
        executor: &'static str,
        observations: usize,
        host_gate: Option<Vec<&'static str>>,
    ) -> Self {
        StagePlan {
            status: "executed",
            executor: Some(executor),
            observations: Some(observations),
            host_gate,
            tool_gate: None,
            artifact: None,
        }
    }

    /// An executed stage a runner ran on a file other than the case's own
    /// fixture — today, only one of the case's own declared controls.
    fn on(mut self, artifact: String) -> Self {
        self.artifact = Some(artifact);
        self
    }

    /// An executed stage whose runner cannot start without an external tool.
    fn needing(mut self, tool: &'static str) -> Self {
        self.tool_gate = Some(vec![tool]);
        self
    }
}

fn route_stages(route: &str) -> &'static [&'static str] {
    ROUTE_STAGES
        .iter()
        .find(|(name, _)| *name == route)
        .unwrap_or_else(|| panic!("unknown CT route `{route}`"))
        .1
}

/// The route a verdict mode really takes. `reject` is the one mode all three
/// source routes share, and the pinned `MUTEX_REJECT_CASES` / `EPOCH_REJECT_CASES`
/// sets — each cross-checked against its fixture's own rejection metadata —
/// decide it.
fn route_for_case(case_id: &str, verdict: &str) -> &'static str {
    match verdict {
        "checked_mutex_sanitized" => "mutex_guard",
        // Card `CT-CTWIN-LANE`: the declared crossing is the same route, one
        // construct further in. It stops at the checker, which is a fact about
        // this construct's emission and not about the route's vocabulary.
        "checked_mutex_crossing" => "mutex_guard",
        "epoch_bound_figures" => "epoch",
        "legacy_c_twin" => "legacy_c",
        "bitfield_reject" => "bitfield_concurrency",
        "native_join_failure"
        | "native_create_recovery"
        | "native_owned_transfer"
        | "native_task_scenarios"
        | "native_heap_scenarios"
        // Card `CT-WIRE-LANE`: a composed task/std-io module, so the task route
        // exactly as `ComposedStdIoTasksV1`'s task half takes it.
        | "reordering_partial_order" => "task",
        // Card `CT-WIRE-LANE-2`: the sequential accept mode with a sanitized
        // native run, and the composed flat-epoch/scoped-task route.
        "sequential_sanitized" => "sequential",
        "epoch_credit_split" => "composed_epoch",
        "accept" => "sequential",
        "reject" => {
            if MUTEX_REJECT_CASES.contains(&case_id) {
                "mutex_guard"
            } else if EPOCH_REJECT_CASES.contains(&case_id) {
                "epoch"
            } else if SEQUENTIAL_REJECT_CASES.contains(&case_id) {
                "sequential"
            } else {
                "task"
            }
        }
        other => panic!("{case_id}: unknown verdict vocabulary `{other}`"),
    }
}

fn diagnostic_band(code: &str) -> u32 {
    code.trim_start_matches('Q')
        .parse::<u32>()
        .unwrap_or_else(|_| panic!("unparsable diagnostic code `{code}`"))
}

/// Derive every stage entry for one case from this harness's dispatch, the
/// fixture's own scenario data, and the diagnostic codes the compiler really
/// produced on the fixture. Nothing here reads the manifest, which is what
/// makes the manifest checkable rather than self-certifying.
fn derive_stage_plan(
    case_id: &str,
    expected: &ExpectedCase,
    observed_codes: &[&str],
) -> BTreeMap<&'static str, StagePlan> {
    let verdict = expected.verdict.as_str();
    let route = route_for_case(case_id, verdict);
    let base = "compiler/tests/ct_corpus.rs::run_mapped_case";
    let deep = match verdict {
        "native_owned_transfer" => Some("compiler/tests/ct_corpus.rs::run_native_owned_transfer"),
        "native_create_recovery" => Some("compiler/tests/ct_corpus.rs::run_native_create_recovery"),
        "native_join_failure" => Some("compiler/tests/ct_corpus.rs::run_native_join_failure"),
        "native_task_scenarios" => Some("compiler/tests/ct_corpus.rs::run_native_task_scenarios"),
        "native_heap_scenarios" => Some("compiler/tests/ct_corpus.rs::run_native_heap_scenarios"),
        "checked_mutex_sanitized" => {
            Some("compiler/tests/ct_corpus.rs::run_checked_mutex_sanitized")
        }
        // Card `MUTEX-SHARE-CROSS-EMISSION` (wave 36). The declared crossing
        // now renders, so its runner really executes the reference, emission,
        // native and sanitized stages on the case's **own** artifact — the
        // same shape `checked_mutex_sanitized` has had since `CT34-SANITIZED`.
        "checked_mutex_crossing" => Some("compiler/tests/ct_corpus.rs::run_mutex_crossing"),
        "reordering_partial_order" => {
            Some("compiler/tests/ct_corpus.rs::run_reordering_partial_order")
        }
        "sequential_sanitized" => Some("compiler/tests/ct_corpus.rs::run_sequential_sanitized"),
        _ => None,
    };
    // Invocation counts, read from the fixture data each runner loops over — an
    // observation count is a fact about the runner, not a number to be typed in.
    let (reference_runs, native_runs) = match verdict {
        "native_owned_transfer" => {
            let runs = if expected
                .native_owned_transfer
                .as_ref()
                .expect("mode metadata")
                .fail_create
            {
                2
            } else {
                1
            };
            (runs, runs)
        }
        "native_create_recovery" => (3, 3),
        // One reference execution of the control; two compile-and-run cycles,
        // the successful join and the injected failure.
        "native_join_failure" => (1, 2),
        "native_task_scenarios" => {
            let runs = expected
                .native_task_scenarios
                .as_ref()
                .expect("mode metadata")
                .scenarios
                .len();
            (runs, runs)
        }
        "native_heap_scenarios" => {
            let runs = expected
                .native_heap_scenarios
                .as_ref()
                .expect("mode metadata")
                .scenarios
                .len();
            (runs, runs)
        }
        "checked_mutex_sanitized" | "checked_mutex_crossing" => (1, 1),
        // One reference execution; one compile and `runs` executions of the one
        // sanitized binary. The native count is read from the fixture rather
        // than typed in, on the same rule as every other mode here.
        "reordering_partial_order" => (
            1,
            expected
                .reordering_partial_order
                .as_ref()
                .expect("mode metadata")
                .runs,
        ),
        // One per-stream reference comparison; one compile and one run of the
        // one sanitized binary.
        "sequential_sanitized" => (1, 1),
        _ => (0, 0),
    };
    // The emitted C is produced once per module, however many times the binary
    // built from it is executed. Every other mode recompiles per run, so its
    // emission count really is the run count; `reordering_partial_order` builds
    // one executable and runs it `runs` times, and saying `runs` emissions there
    // would inflate a count this harness is supposed to read off its own
    // dispatch.
    let emission_runs = if verdict == "reordering_partial_order" {
        1
    } else {
        native_runs
    };
    let all_three = vec!["linux", "macos", "windows"];
    // The sanitized runners return on Windows after asserting that the
    // emitted C exists, so emission is reached on all three hosts while the
    // sanitized compile-and-run is Unix-only.
    let deep_gate = if matches!(
        verdict,
        "native_heap_scenarios"
            | "checked_mutex_sanitized"
            | "checked_mutex_crossing"
            | "reordering_partial_order"
            | "sequential_sanitized"
    ) {
        vec!["linux", "macos"]
    } else {
        all_three.clone()
    };
    // CT50's native observation needs Linux `MAP_FIXED_NOREPLACE`; every other
    // stage of that case runs wherever the threads backend does.
    let native_gate = if case_id == "CT50" {
        vec!["linux"]
    } else {
        deep_gate.clone()
    };

    let refused = !observed_codes.is_empty();
    let refused_before_lowering = refused
        && observed_codes
            .iter()
            .all(|code| diagnostic_band(code) < 500);
    let reached_model_check = !refused
        || observed_codes
            .iter()
            .any(|code| diagnostic_band(code) >= 600);

    // `reference` is `None` for a route that has no reference-execution stage at
    // all. The epoch route is check-only, so it has none: an admitted epoch is a
    // checked accounting statement, not a program with an observation.
    let (lowering, check, reference) = match route {
        "task" => (
            "task_lowering",
            Some("task_ledger_check"),
            Some("task_reference_execution"),
        ),
        "mutex_guard" => (
            "mutex_lowering",
            Some("mutex_invariant_check"),
            Some("mutex_reference_execution"),
        ),
        "sequential" => (
            "sequential_whole_program",
            None,
            Some("sequential_reference_execution"),
        ),
        "epoch" => ("epoch_lowering", Some("epoch_bound_check"), None),
        "bitfield_concurrency" => ("footprint_lowering", Some("footprint_check"), None),
        // The composed route lowers a flat-epoch plan over a checked task
        // region; the epoch pair is the tuple here and the task pair is
        // inserted below, because both halves demonstrably run before the
        // credit-recovery band can refuse anything. No reference stage: the
        // composed module publishes no reference observation.
        "composed_epoch" => ("epoch_lowering", Some("epoch_bound_check"), None),
        other => panic!("{case_id}: route `{other}` has no mapped case shape"),
    };

    let mut plan: BTreeMap<&'static str, StagePlan> = STAGE_NAMES
        .iter()
        .map(|stage| (*stage, StagePlan::absent("not_applicable")))
        .collect();
    plan.insert("parse", StagePlan::executed(base, 1, None));
    plan.insert("elaborate", StagePlan::executed(base, 1, None));
    if !refused_before_lowering {
        plan.insert(lowering, StagePlan::executed(base, 1, None));
    }
    if let Some(check) = check
        && !refused_before_lowering
        && reached_model_check
    {
        plan.insert(check, StagePlan::executed(base, 1, None));
    }
    // Card `CT-WIRE-LANE-2`. The composed epoch route lowers and checks its
    // task half before the accounting half can say anything: the
    // credit-recovery gate derives the creator's error edges from the
    // *checked* region, so a refusal in the flat epoch's own band
    // (`Q630`–`Q641`) proves the task stages ran on this fixture.
    if route == "composed_epoch" && !refused_before_lowering {
        plan.insert("task_lowering", StagePlan::executed(base, 1, None));
        if reached_model_check {
            plan.insert("task_ledger_check", StagePlan::executed(base, 1, None));
        }
        // CT24 is refused by the task ledger while compiling the erased task
        // half. The epoch plan has been read syntactically, but neither epoch
        // lowering nor the bound checker runs: compile_composed_epoch_source
        // returns immediately from compile_task_source. A Q600 code alone
        // cannot distinguish that ordering from a later epoch refusal, so pin
        // this case's named composition boundary explicitly rather than
        // crediting stages its own artifact never reaches.
        if case_id == "CT24" && refused {
            plan.insert("epoch_lowering", StagePlan::absent("not_applicable"));
            plan.insert("epoch_bound_check", StagePlan::absent("not_applicable"));
        }
    }
    // Card `CT22-WIRE`. An *admitted* composed-epoch module really emits the
    // task route's own C (the runner asserts `c_source` is present), so the two
    // stages are open items rather than structurally absent: `unassessed`,
    // exactly as `native_sanitized` already reads on this route. A refused
    // composed-epoch fixture keeps `not_applicable` — its refusal provably never
    // reaches the backend.
    //
    // Card `CT-COMPOSED-NATIVE` (wave 32) **supplies the missing runner**:
    // `run_composed_native` now compiles and runs every
    // `COMPOSED_NATIVE_CASES` entry's own emitted C under the platform
    // sanitizers on Linux and macOS, on every `cargo test` of this workspace,
    // and compares its standard output byte for byte with the reference
    // execution of that module's own checked task half. These three rows
    // therefore **understate** what this harness executes today, which is the
    // safe direction and never the claiming one.
    //
    // They are deliberately not moved here. A stage status is checked against
    // `docs/reevaluation/ct_manifest.json` for exact equality, so the derivation
    // and the manifest rows must move in **one** change, and the manifest is
    // outside that card's boundary (the CT numerators it feeds —
    // sanitized-native `7 of 43` → `9 of 43` — are the integrator's wave-wide
    // reconciliation). Writing the `executed` arm here with the rows still
    // reading `unassessed` would leave an arm no test drives, which is a stated
    // boundary rather than a verified rule. The exact coupled edit is published
    // in `docs/reports/ct-composed-native/README.md` §4 for the integrator:
    // this block gains a `COMPOSED_NATIVE_CASES` arm crediting
    // `run_composed_native` with `c_emission` on all three hosts and
    // `native_execution`/`native_sanitized` on `["linux", "macos"]`, the case
    // ids join `SANITIZED_GATED_CASES`, and CT22/CT25's three manifest rows are
    // rewritten to match. CT23 moves in neither: it is the route's refusal.
    if route == "composed_epoch" && !refused {
        // Integrator, wave 32: the coupled edit card `CT-COMPOSED-NATIVE`
        // published in its report §4, applied together with the manifest rows
        // it names so the derivation and the record move in one change.
        if COMPOSED_NATIVE_CASES.contains(&case_id) {
            let runner = "compiler/tests/ct_corpus.rs::run_composed_native";
            plan.insert(
                "c_emission",
                StagePlan::executed(runner, 1, Some(vec!["linux", "macos", "windows"])),
            );
            plan.insert(
                "native_execution",
                StagePlan::executed(runner, 1, Some(vec!["linux", "macos"])),
            );
        } else {
            plan.insert("c_emission", StagePlan::absent("unassessed"));
            plan.insert("native_execution", StagePlan::absent("unassessed"));
        }
    }
    // Card `CT-CTWIN-LANE` (wave 35) planned a declared crossing's reference,
    // emission and native stages as `unassessed`, because the crossing was
    // *admitted* and still reached no backend: `compile_files` returned the
    // module with `c_source: None` and `interpret_mutex_program` refused its
    // event list at the `Spawn`. That block is **removed** by card
    // `MUTEX-SHARE-CROSS-EMISSION` (wave 36) rather than edited around: the
    // backend now renders a non-nested fork/join trace with real C11 children,
    // so the three stages are planned by the ordinary `deep` machinery above,
    // crediting `run_mutex_crossing` on the case's own artifact. The open item
    // that block named is the residual this card discharges.
    // The plain sequential accept mode runs its reference execution inside
    // `compile_files` itself, so it has no separate runner and no host gate.
    if verdict == "accept"
        && expected.outcome.is_some()
        && let Some(reference) = reference
    {
        plan.insert(reference, StagePlan::executed(base, 1, None));
    }
    if let Some(deep) = deep {
        plan.insert(
            reference.expect("a native runner implies a reference-execution stage"),
            StagePlan::executed(deep, reference_runs, Some(deep_gate.clone())),
        );
        plan.insert(
            "c_emission",
            StagePlan::executed(deep, emission_runs, Some(all_three.clone())),
        );
        plan.insert(
            "native_execution",
            StagePlan::executed(deep, native_runs, Some(native_gate)),
        );
    }
    // Card `CT34-SANITIZED`. A mutex rejection's own fixture is a refusal that
    // emits nothing, so a sanitized run for such a case can only be of a control
    // that renders. `run_mutex_controls` compiles and runs exactly the one
    // control carrying a `sanitized_run` block, so these four stages are
    // credited to that runner and name that control's own file — never the
    // case's fixture, which no compiler ever saw. A control that merely reaches
    // `compile_files` for a verdict assertion, as every other mutex control
    // does, is not an emission or execution observation and stays absent.
    let control_runner = "compiler/tests/ct_corpus.rs::run_mutex_controls";
    let sanitized_control = expected.checked_mutex_reject.as_ref().and_then(|reject| {
        reject
            .controls
            .iter()
            .find(|control| control.sanitized_run.is_some())
    });
    if let Some(control) = sanitized_control {
        let artifact = format!("compiler/tests/fixtures/ct/{case_id}/{}", control.source);
        let reference = reference.expect("the mutex route has a reference-execution stage");
        // The runner returns on Windows after asserting the emission exists, so
        // emission is reached on all three hosts and the compile-and-run pair is
        // Unix-only — the same split `run_checked_mutex_sanitized` has.
        let unix = vec!["linux", "macos"];
        plan.insert(
            reference,
            StagePlan::executed(control_runner, 1, Some(unix.clone())).on(artifact.clone()),
        );
        plan.insert(
            "c_emission",
            StagePlan::executed(control_runner, 1, Some(all_three.clone())).on(artifact.clone()),
        );
        plan.insert(
            "native_execution",
            StagePlan::executed(control_runner, 1, Some(unix)).on(artifact),
        );
    }
    plan.insert(
        "native_sanitized",
        if SANITIZED_GATED_CASES.contains(&case_id) {
            match (deep, sanitized_control) {
                (Some(deep), None) => StagePlan::executed(deep, native_runs, Some(deep_gate)),
                (None, Some(control)) => {
                    StagePlan::executed(control_runner, 1, Some(vec!["linux", "macos"])).on(
                        format!("compiler/tests/fixtures/ct/{case_id}/{}", control.source),
                    )
                }
                // Card `CT-COMPOSED-NATIVE`: the composed-epoch mode has no
                // `deep` runner and no sanitized control — its own runner
                // compiles and runs the case's emitted C under the sanitizers.
                (None, None) if COMPOSED_NATIVE_CASES.contains(&case_id) => StagePlan::executed(
                    "compiler/tests/ct_corpus.rs::run_composed_native",
                    1,
                    Some(vec!["linux", "macos"]),
                ),
                _ => panic!(
                    "{case_id}: a gated sanitized case has exactly one runner that compiles \
                     and runs it under the sanitizers"
                ),
            }
        } else if SANITIZER_TRANSCRIPT_CASES.contains(&case_id) {
            StagePlan::absent("recorded_not_gated")
        } else if !route_stages(route).contains(&"native_sanitized") {
            // A route that emits no C has nothing to sanitize: structurally
            // absent, not an open item somebody could close.
            StagePlan::absent("not_applicable")
        } else {
            StagePlan::absent("unassessed")
        },
    );
    // Card `CT01-PROOF-LEG` advanced this line deliberately (it was card
    // T00.7's open item): a case in the pinned table has its theorem-bound
    // runner; every other case keeps `unassessed`, fail-closed.
    plan.insert(
        "proof",
        match PROOF_GATED_CASES.iter().find(|(case, _)| *case == case_id) {
            Some((_, executor)) => StagePlan::executed(executor, 1, None),
            None => StagePlan::absent("unassessed"),
        },
    );
    plan
}

/// The `legacy_c` route's stage plan (card CT-CTWIN). The route has two
/// executable stages and no others: `parse` is the gateway's C admission —
/// Clang preprocessing plus syntax acceptance of the case's own `.c` bytes,
/// executed as `validate_unrestricted_c11` — and `legacy_c_twin` is the
/// resource projection and its unit verdict. There is no `elaborate` because
/// nothing here is QCore source, and no emission, native or sanitizer stage
/// because the gateway analyzes C rather than producing any, so
/// `native_sanitized` is structurally absent under the same route rule the
/// epoch route uses. Both stages need Clang and no particular `target_os`,
/// which is what `tool_gate` records.
fn derive_legacy_stage_plan(
    case_id: &str,
    twin: &LegacyCTwin,
) -> BTreeMap<&'static str, StagePlan> {
    let runner = "compiler/tests/ct_corpus.rs::run_legacy_c_twin";
    // One admission of the case's own bytes, then one analysis per contract
    // configuration those bytes are analyzed under: the case's own, plus every
    // control that varies the contracts rather than the source. Controls with
    // their own source are executed on *their* artifact, so they are not this
    // case's observations.
    let contract_controls = twin
        .controls
        .iter()
        .filter(|control| control.source.is_none())
        .count();
    let mut plan: BTreeMap<&'static str, StagePlan> = STAGE_NAMES
        .iter()
        .map(|stage| (*stage, StagePlan::absent("not_applicable")))
        .collect();
    assert!(
        route_stages("legacy_c").contains(&"parse")
            && route_stages("legacy_c").contains(&"legacy_c_twin"),
        "{case_id}: the legacy_c route vocabulary must carry the stages this plan executes"
    );
    plan.insert(
        "parse",
        StagePlan::executed(runner, 1, None).needing("clang"),
    );
    plan.insert(
        "legacy_c_twin",
        StagePlan::executed(runner, 1 + contract_controls, None).needing("clang"),
    );
    plan.insert("proof", StagePlan::absent("unassessed"));
    plan
}

#[path = "support/ct_atomic_publication.rs"]
mod atomic_publication;

fn run_mapped_case(case: &ManifestCase) {
    if case.id == "CT41" {
        atomic_publication::run(case);
        return;
    }
    let directory = fixture_root().join(&case.id);
    assert!(
        directory.is_dir(),
        "{}: manifest claims `mapped` but fixture directory {} does not exist",
        case.id,
        directory.display()
    );
    let expected_path = directory.join("expected.json");
    let expected: ExpectedCase = serde_json::from_str(
        &fs::read_to_string(&expected_path)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", expected_path.display())),
    )
    .unwrap_or_else(|error| panic!("cannot parse {}: {error}", expected_path.display()));
    assert_eq!(expected.id, case.id, "expected.json id mismatch");
    assert_eq!(
        expected.evidence_target, case.evidence_target,
        "{}: target metadata mismatch",
        case.id
    );
    assert_eq!(
        expected.native_task_scenarios.is_some(),
        expected.verdict == "native_task_scenarios"
    );
    assert_eq!(
        expected.native_join_failure.is_some(),
        expected.verdict == "native_join_failure",
        "{}: native fault mode and fixture metadata must agree",
        case.id
    );
    assert_eq!(
        expected.native_create_recovery.is_some(),
        expected.verdict == "native_create_recovery"
    );
    assert_eq!(
        expected.native_owned_transfer.is_some(),
        expected.verdict == "native_owned_transfer"
    );
    assert_eq!(
        expected.checked_mutex_sanitized.is_some(),
        expected.verdict == "checked_mutex_sanitized"
    );
    assert_eq!(
        expected.checked_mutex_sanitized.is_some(),
        case.id == MUTEX_SANITIZED_CASE,
        "{}: exactly one CT case carries gated sanitized mutex evidence",
        case.id
    );
    assert_eq!(
        expected.checked_mutex_reject.is_some(),
        MUTEX_REJECT_CASES.contains(&case.id.as_str()),
        "{}: mutex rejection metadata is pinned to its own case set",
        case.id
    );
    assert_eq!(
        expected.checked_mutex_crossing.is_some(),
        expected.verdict == "checked_mutex_crossing"
    );
    assert_eq!(
        expected.checked_mutex_crossing.is_some(),
        case.id == MUTEX_CROSSING_CASE,
        "{}: exactly one CT case carries declared-crossing evidence",
        case.id
    );
    assert_eq!(
        expected.native_heap_scenarios.is_some(),
        expected.verdict == "native_heap_scenarios"
    );
    assert_eq!(
        expected.native_heap_scenarios.is_some(),
        HEAP_SCENARIO_CASES.contains(&case.id.as_str()),
        "{}: sanitized heap-witness evidence is pinned to its own case set",
        case.id
    );
    assert_eq!(
        expected.source_reject.is_some(),
        SOURCE_REJECT_CASES.contains(&case.id.as_str()),
        "{}: task-route rejection metadata is pinned to its own case set",
        case.id
    );
    assert_eq!(
        expected.bitfield_reject.is_some(),
        case.id == BITFIELD_REJECT_CASE,
        "{}: exactly CT10 carries bit-field footprint rejection metadata",
        case.id
    );
    assert_eq!(
        expected.bitfield_reject.is_some(),
        expected.verdict == "bitfield_reject",
        "{}: bit-field route metadata and verdict mode must agree",
        case.id
    );
    assert_eq!(
        expected.sequential_reject.is_some(),
        SEQUENTIAL_REJECT_CASES.contains(&case.id.as_str()),
        "{}: sequential-route rejection metadata is pinned to its own case set",
        case.id
    );
    assert!(
        expected.source_reject.is_none() || expected.sequential_reject.is_none(),
        "{}: a rejection takes one route, so it carries one route's metadata",
        case.id
    );
    assert_eq!(
        expected.reordering_partial_order.is_some(),
        expected.verdict == "reordering_partial_order"
    );
    assert_eq!(
        expected.reordering_partial_order.is_some(),
        REORDERING_CASES.contains(&case.id.as_str()),
        "{}: partial-order native evidence is pinned to its own case set",
        case.id
    );
    assert_eq!(
        expected.sequential_sanitized.is_some(),
        expected.verdict == "sequential_sanitized"
    );
    assert_eq!(
        expected.sequential_sanitized.is_some(),
        SEQUENTIAL_SANITIZED_CASES.contains(&case.id.as_str()),
        "{}: sequential sanitized-accept evidence is pinned to its own case set",
        case.id
    );
    assert_eq!(
        expected.epoch_credit_split.is_some(),
        expected.verdict == "epoch_credit_split"
    );
    assert_eq!(
        expected.epoch_credit_split.is_some(),
        EPOCH_CREDIT_SPLIT_CASES.contains(&case.id.as_str()),
        "{}: composed-epoch credit-split evidence is pinned to its own case set",
        case.id
    );
    // Card `CT-COMPOSED-NATIVE`: the native block and the pinned table must
    // agree in both directions, so a fixture cannot buy itself a compile-and-run
    // and the table cannot name a case with nothing to run. A composed-epoch
    // *refusal* carries neither: it never reaches the backend, so there is no
    // emission of its own to execute.
    assert_eq!(
        expected.composed_native.is_some(),
        COMPOSED_NATIVE_CASES.contains(&case.id.as_str()),
        "{}: composed-epoch native evidence is pinned to its own case set",
        case.id
    );
    if expected.composed_native.is_some() {
        let split = expected.epoch_credit_split.as_ref().unwrap_or_else(|| {
            panic!(
                "{}: a composed-epoch native case is a composed-epoch case, so it \
                 carries the credit-split evidence too",
                case.id
            )
        });
        assert!(
            split.diagnostic_sequence.is_empty(),
            "{}: only an admitted composed-epoch module emits C a runner could \
             compile and run",
            case.id
        );
    }
    assert_eq!(
        expected.epoch_reject.is_some(),
        EPOCH_REJECT_CASES.contains(&case.id.as_str()),
        "{}: epoch-route rejection metadata is pinned to its own case set",
        case.id
    );
    assert_eq!(
        expected.epoch_bound_figures.is_some(),
        expected.verdict == "epoch_bound_figures"
    );
    assert_eq!(
        expected.epoch_bound_figures.is_some(),
        EPOCH_FIGURE_CASES.contains(&case.id.as_str()),
        "{}: accepted epoch-certificate evidence is pinned to its own case set",
        case.id
    );
    assert_eq!(
        expected.legacy_c_twin.is_some(),
        expected.verdict == "legacy_c_twin"
    );
    assert_eq!(
        expected.legacy_c_twin.is_some(),
        LEGACY_C_TWIN_CASES.contains(&case.id.as_str()),
        "{}: C-twin gateway evidence is pinned to its own case set",
        case.id
    );
    let legacy = expected.legacy_c_twin.is_some();
    // Target-scoped evidence: the native fault modes, the sanitized mutex run,
    // and any mutex rejection whose control compiles through the Linux-only
    // emission path. Recording the last of these repairs T00's finding O6 for
    // this card's cases rather than repeating it.
    assert_eq!(
        expected.evidence_target.as_deref(),
        (expected.native_join_failure.is_some()
            || expected.native_create_recovery.is_some()
            || expected.native_owned_transfer.is_some()
            || expected.native_task_scenarios.is_some()
            || expected.checked_mutex_sanitized.is_some()
            // Card `CT-CTWIN-LANE`: the declared crossing takes the mutex
            // route, which refuses a non-Linux/macOS/Windows host at `Q500`
            // before the checker is reached, and its `accept_emitting` control
            // compiles through the host-gated emission path.
            || expected.checked_mutex_crossing.is_some()
            || expected.native_heap_scenarios.is_some()
            // Card `CT-WIRE-LANE`: the sanitized compile-and-run half is
            // Unix-only, exactly as the two runners above.
            || expected.reordering_partial_order.is_some()
            // Card `CT-WIRE-LANE-2`: the same Unix-only sanitized half, on the
            // sequential accept runner.
            || expected.sequential_sanitized.is_some()
            // Card `CT-WIRE-LANE-2`: a composed-epoch accept control compiles
            // through the host-gated task emission path.
            || expected
                .epoch_credit_split
                .as_ref()
                .is_some_and(|split| split
                    .controls
                    .iter()
                    .any(|control| control.expectation == "accept"))
            || expected
                .checked_mutex_reject
                .as_ref()
                .is_some_and(|reject| reject.controls.iter().any(|control| matches!(
                    control.expectation.as_str(),
                    // Every admitted-control kind reaches `compile_files`, whose
                    // mutex route refuses a non-Linux/macOS/Windows host at
                    // `Q500` before it ever reaches the backend, so all of them
                    // are host-gated in exactly the same way.
                    "accept" | "accept_growth" | "accept_check_only"
                )))
            || expected.source_reject.as_ref().is_some_and(|reject| reject
                .controls
                .iter()
                .any(|control| control.expectation == "accept")))
        .then_some("linux"),
        "{}: a case carries target-scoped evidence exactly when a host-gated \
         runner or control executes for it",
        case.id
    );
    assert_eq!(
        expected.positive_control.is_some(),
        case.id == "CT19",
        "CT19 must retain its explicitly named joined-sibling positive control"
    );
    // The route decides what the case's own artifact is: a QCore source on the
    // three source routes, a C twin on `legacy_c`.
    let case_file = if legacy { "case.c" } else { "case.qr" };
    let mut required_files = BTreeSet::from([case_file.to_owned(), "expected.json".to_owned()]);
    if let Some(native) = &expected.native_task_scenarios {
        assert!(!native.scenarios.is_empty());
        assert!(required_files.insert(native.header.clone()));
        for control in &native.rejection_controls {
            assert!(required_files.insert(control.source.clone()));
        }
        let names = native
            .scenarios
            .iter()
            .map(|scenario| &scenario.name)
            .collect::<BTreeSet<_>>();
        assert_eq!(names.len(), native.scenarios.len());
        assert!(names.iter().all(|name| !name.is_empty()));
    }
    if let Some(native) = &expected.native_owned_transfer {
        assert!(["CT01", "CT12"].contains(&case.id.as_str()));
        assert_eq!(native.header, "owned_transfer.h");
        assert_eq!(native.fail_create, case.id == "CT12");
        assert_eq!(native.expected_i32, 42);
        required_files.insert(native.header.clone());
    }
    if let Some(native) = &expected.native_join_failure {
        assert_eq!(case.id, "CT15");
        assert_eq!(native.header, "join_failure.h");
        assert_eq!(native.abort_exit_code, 77);
        required_files.insert(native.header.clone());
    }
    if let Some(native) = &expected.native_create_recovery {
        assert_eq!(case.id, "CT13");
        assert_eq!(native.header, "create_failure.h");
        assert_eq!(native.unjoined_source, "unjoined_failure.qr");
        assert_eq!(native.success_i32, 7);
        assert_eq!(native.recovery_i32, 42);
        required_files.insert(native.header.clone());
        required_files.insert(native.unjoined_source.clone());
    }
    if let Some(mutex) = &expected.checked_mutex_sanitized {
        assert!(mutex.sections >= 2, "CT26 needs two critical sections");
        assert_eq!(mutex.guards.len(), mutex.sections);
        assert!(
            mutex
                .sanitizer_flags
                .iter()
                .any(|flag| flag == "-fsanitize=address,undefined"),
            "the sanitized mutex run must actually request ASan/UBSan"
        );
        assert!(
            mutex.asan_options.contains("detect_leaks=1"),
            "LeakSanitizer stays enabled: reclaiming every emitted allocation \
             is what this run must witness"
        );
        assert!(!mutex.reference_observation.is_empty());
        assert!(!mutex.emitted_c_contains.is_empty());
    }
    if let Some(crossing) = &expected.checked_mutex_crossing {
        // The classification half of the mapping is checked against the
        // judgment's own published data rather than against this fixture's
        // prose, so a rename there cannot leave the fixture asserting a
        // vocabulary the code no longer has.
        assert_eq!(
            crossing.payload,
            format!("TransferKind::{:?}", MUTEX_SHARE_CROSSING.kind),
            "{}: the declared payload must be the judgment's own kind",
            case.id
        );
        assert_eq!(
            crossing.protocol,
            format!("ShareProtocol::{:?}", MUTEX_SHARE_CROSSING.protocol),
            "{}: the declared protocol must be the judgment's own",
            case.id
        );
        // This mapping rests on the crossing being requestable from source at
        // all. That is a compile-time fact about the judgment's published data,
        // so it is pinned in a `const` block rather than at run time: if a
        // later card sets `requestable: false` — as `MUTEX_SHARING_GAP` still
        // is for the task route's own buffer — this file stops compiling,
        // instead of a green test reporting a case mapped onto a capability the
        // judgment no longer offers.
        const { assert!(MUTEX_SHARE_CROSSING.requestable) };
        assert!(
            !crossing.attributed_to.trim().is_empty(),
            "{}: a crossing names the construct its scenario is about",
            case.id
        );
        assert!(
            !crossing.reaches.is_empty() && crossing.reaches.iter().all(|reach| reach.guarded),
            "{}: `accept only through the protocol` means every reach names a guard",
            case.id
        );
        assert!(
            crossing
                .reaches
                .iter()
                .any(|reach| reach.task == crossing.spawn.child),
            "{}: the child must be one of the tasks that reaches the payload, or \
             the crossing carries nothing",
            case.id
        );
        // Card `MUTEX-SHARE-CROSS-EMISSION` (wave 36) moved this pin from the
        // case's own refusal to its rendering. The boundary is still pinned,
        // never omitted — it is now the *surviving* one, on a control the
        // backend really does refuse.
        assert!(
            !crossing.emission.reference_observation.is_empty()
                && !crossing.emission.emitted_c_contains.is_empty()
                && !crossing.emission.main_excludes.is_empty(),
            "{}: the crossing's rendering is pinned, never omitted",
            case.id
        );
        assert_eq!(
            crossing
                .controls
                .iter()
                .filter(|control| control.expectation == "accept_unrendered")
                .count(),
            1,
            "{}: exactly one control pins the emission boundary that survives, \
             so `the backend renders a crossing` is not read as `the backend \
             renders every mutex trace`",
            case.id
        );
        assert_eq!(
            crossing
                .controls
                .iter()
                .filter(|control| control.expectation == "accept_emitting")
                .count(),
            1,
            "{}: exactly one control pins the same route without the clause, so \
             the case's thread boundary is attributable to the clause",
            case.id
        );
        // The attribution is a real difference and not a restatement: the case
        // carries task structure in its observation and the control does not.
        let control_observation = &crossing
            .controls
            .iter()
            .find(|control| control.expectation == "accept_emitting")
            .expect("checked just above")
            .reference_observation;
        assert!(
            crossing
                .emission
                .reference_observation
                .iter()
                .any(|line| line.starts_with("spawn "))
                && !control_observation
                    .iter()
                    .any(|line| line.starts_with("spawn ")),
            "{}: the crossing's observation carries the task structure its \
             control's does not",
            case.id
        );
        assert!(
            crossing
                .controls
                .iter()
                .filter(|control| control.expectation == "reject")
                .count()
                >= 2,
            "{}: `only through the protocol` needs more than one way of \
             failing to reach it",
            case.id
        );
        for control in &crossing.controls {
            match control.expectation.as_str() {
                "accept_emitting" => assert!(
                    control.diagnostic_sequence.is_empty()
                        && control.message_fragment.is_none()
                        && !control.reference_observation.is_empty(),
                    "{}: the emitting control pins the observation that makes it \
                     the emission boundary",
                    case.id
                ),
                // Card `MUTEX-SHARE-CROSS-EMISSION`: the surviving boundary's
                // control is admitted and renders nothing, so it pins a refusal
                // instead of an observation — the exact mirror of the emitting
                // control above.
                "accept_unrendered" => assert!(
                    control.diagnostic_sequence.is_empty()
                        && control.message_fragment.is_none()
                        && control.reference_observation.is_empty()
                        && control
                            .emission_refusal
                            .as_ref()
                            .is_some_and(|refusal| !refusal.trim().is_empty()),
                    "{}: the unrendered control pins the refusal that makes it \
                     the surviving emission boundary",
                    case.id
                ),
                "reject" => assert!(
                    !control.diagnostic_sequence.is_empty()
                        && control.message_fragment.is_some()
                        && control.reference_observation.is_empty(),
                    "{}: a refused control pins its codes and a distinguishing \
                     fragment of its message",
                    case.id
                ),
                other => panic!("{}: unknown crossing control `{other}`", case.id),
            }
            assert!(required_files.insert(control.source.clone()));
        }
    }
    if let Some(reject) = &expected.checked_mutex_reject {
        assert!(
            !reject.diagnostic_sequence.is_empty(),
            "{}: a mutex rejection pins its exact ordered code vector",
            case.id
        );
        assert!(
            !reject.controls.is_empty(),
            "{}: a mutex rejection needs at least one control so the refusal \
             is not vacuous",
            case.id
        );
        assert!(
            reject
                .controls
                .iter()
                .any(|control| control.expectation != "reject"),
            "{}: a mutex rejection needs an admitted control, or the refusal is \
             only `the profile cannot express this at all`",
            case.id
        );
        // Card `CT34-SANITIZED`. The sanitized native evidence of a mutex
        // rejection is one control's own run, and the case's native stages name
        // that control's file, so there is exactly one of them or none. The two
        // directions are pinned together: a case cannot carry a sanitized
        // control without being in the gate, and cannot be in the gate on this
        // route without carrying one.
        let sanitized_controls = reject
            .controls
            .iter()
            .filter(|control| control.sanitized_run.is_some())
            .count();
        assert!(
            sanitized_controls <= 1,
            "{}: at most one control carries the case's sanitized native run",
            case.id
        );
        assert_eq!(
            sanitized_controls == 1,
            SANITIZED_GATED_CASES.contains(&case.id.as_str()),
            "{}: a mutex rejection's sanitized-native claim is exactly its sanitized \
             control",
            case.id
        );
        for control in &reject.controls {
            assert!(
                control.sanitized_run.is_none() || control.expectation == "accept_growth",
                "{}: only a control the backend renders can be compiled and run",
                case.id
            );
            match control.expectation.as_str() {
                "accept" => assert!(
                    control.diagnostic_sequence.is_empty() && control.backend_refusal.is_empty(),
                    "{}: an accepted control has no diagnostics and no backend refusal",
                    case.id
                ),
                // The checker admits it, it really performs a protection growth,
                // and the C backend renders that growth (card CT34-SANITIZED).
                "accept_growth" => assert!(
                    control.diagnostic_sequence.is_empty() && control.backend_refusal.is_empty(),
                    "{}: a rendered growth control has no diagnostics and no backend refusal",
                    case.id
                ),
                // The checker admits it; the C backend declines to render it.
                // Both halves are pinned, so neither can be quietly dropped.
                "accept_check_only" => assert!(
                    control.diagnostic_sequence.is_empty() && !control.backend_refusal.is_empty(),
                    "{}: a check-only control states the backend refusal that makes \
                     it check-only, so `it checks` is never the whole control",
                    case.id
                ),
                "reject" => assert!(
                    !control.diagnostic_sequence.is_empty() && control.backend_refusal.is_empty(),
                    "{}: a refused control pins its codes and reaches no backend",
                    case.id
                ),
                other => panic!("{}: unknown control expectation `{other}`", case.id),
            }
            assert!(required_files.insert(control.source.clone()));
        }
    }
    if let Some(native) = &expected.native_heap_scenarios {
        assert!(!native.scenarios.is_empty());
        assert!(
            native
                .sanitizer_flags
                .iter()
                .any(|flag| flag == "-fsanitize=address,undefined"),
            "{}: the sanitized heap runs must actually request ASan/UBSan",
            case.id
        );
        assert!(
            native.ubsan_options.contains("halt_on_error=1"),
            "{}: UBSan must halt rather than continue past a fault",
            case.id
        );
        assert!(
            !native.alloc_prefix.is_empty() && !native.free_prefix.is_empty(),
            "{}: the heap witness needs both an allocation and a release marker",
            case.id
        );
        assert!(
            native
                .scenarios
                .iter()
                .any(|scenario| scenario.detect_leaks),
            "{}: at least one scenario must run with LeakSanitizer enabled, so \
             `detect_leaks` cannot be switched off case-wide by editing data",
            case.id
        );
        let names = native
            .scenarios
            .iter()
            .map(|scenario| &scenario.name)
            .collect::<BTreeSet<_>>();
        assert_eq!(names.len(), native.scenarios.len());
        assert!(names.iter().all(|name| !name.is_empty()));
        for scenario in &native.scenarios {
            assert!(
                scenario.heap_releases <= scenario.heap_allocations,
                "{}: {} cannot release more objects than it allocated",
                case.id,
                scenario.name
            );
            assert!(
                scenario
                    .defines
                    .iter()
                    .all(|define| define.starts_with("-D")),
                "{}: {} may only inject failures through `-D` options",
                case.id,
                scenario.name
            );
            assert!(
                scenario.exit_code == 0 || !scenario.stdout_absent.is_empty(),
                "{}: {} is a trapped abort, so it must state what its truncated \
                 observation does not contain",
                case.id,
                scenario.name
            );
            assert!(
                scenario.exit_code == 0 || !scenario.detect_leaks,
                "{}: {} retains its allocations on purpose, so LeakSanitizer \
                 cannot also be asserted clean",
                case.id,
                scenario.name
            );
            required_files.insert(scenario.header.clone());
        }
        for control in &native.rejection_controls {
            assert!(required_files.insert(control.source.clone()));
        }
        // Card `CT07-TWO-THREAD-REMAP`: an accept control is an executed input
        // exactly as a rejection control is, so it enters the inventory the same
        // way and an unreferenced accept fixture is still a failure.
        for control in &native.accept_controls {
            assert!(required_files.insert(control.source.clone()));
            assert!(
                !control.note.is_empty(),
                "{}: accept control {} must say what it is controlling for",
                case.id,
                control.source
            );
        }
    }
    if let Some(reordering) = &expected.reordering_partial_order {
        assert!(
            reordering.runs >= 2,
            "{}: a partial-order case runs its executable more than once — one \
             run could not distinguish a relation from an equality",
            case.id
        );
        assert!(
            reordering
                .sanitizer_flags
                .iter()
                .any(|flag| flag == "-fsanitize=address,undefined"),
            "{}: the partial-order runs must actually request ASan/UBSan",
            case.id
        );
        assert!(
            reordering.asan_options.contains("detect_leaks=1"),
            "{}: LeakSanitizer stays enabled, so a threaded program that retains \
             an allocation is caught rather than tolerated",
            case.id
        );
        assert!(
            reordering.ubsan_options.contains("halt_on_error=1"),
            "{}: UBSan must halt rather than continue past a fault",
            case.id
        );
        assert!(
            reordering.stdout_relation_strict,
            "{}: every trace write of an emitted task program is performed by \
             the parent's thread, so the standard-output relation is strict",
            case.id
        );
        assert!(
            !reordering.stderr_relation_strict,
            "{}: if this case's own stream relation were strict it would be \
             measuring byte equality under another name",
            case.id
        );
        assert!(
            reordering.stderr_emitters >= 3,
            "{}: the interleaving this case is about needs both workers and the \
             parent writing to the same stream",
            case.id
        );
        assert!(
            reordering.stderr_window_sizes.len() >= 3,
            "{}: a relation with no barrier on either side of the concurrent \
             window would not pin where commuting stops",
            case.id
        );
        assert!(
            reordering
                .controls
                .iter()
                .any(|control| control.expectation == "strict_relation"),
            "{}: the partial order must be bounded by a control that keeps byte \
             equality, or the mapping reads as a corpus-wide weakening",
            case.id
        );
        assert!(
            reordering
                .controls
                .iter()
                .any(|control| control.expectation == "refuse_relation"),
            "{}: a relation that is never refused is not a premise",
            case.id
        );
        for control in &reordering.controls {
            match control.expectation.as_str() {
                "strict_relation" => assert!(
                    control.refusal_code.is_none() && control.refusal_contains.is_none(),
                    "{}: a strict control derives a relation, so it names no refusal",
                    case.id
                ),
                "refuse_relation" => assert!(
                    control.refusal_code.is_some() && control.refusal_contains.is_some(),
                    "{}: a refused relation pins the refusal it reaches, never \
                     merely that one happened",
                    case.id
                ),
                other => panic!("{}: unknown control expectation `{other}`", case.id),
            }
            assert!(required_files.insert(control.source.clone()));
        }
    }
    if let Some(mode) = &expected.sequential_sanitized {
        assert!(
            mode.sanitizer_flags
                .iter()
                .any(|flag| flag == "-fsanitize=address,undefined"),
            "{}: the sanitized run must actually request ASan/UBSan",
            case.id
        );
        assert!(
            mode.asan_options.contains("detect_leaks=1"),
            "{}: LeakSanitizer stays enabled on the Linux arm — Apple's ASan \
             ships no leak detector, and the runner records that boundary",
            case.id
        );
        assert!(
            mode.ubsan_options.contains("halt_on_error=1"),
            "{}: UBSan must halt rather than continue past a fault",
            case.id
        );
        assert!(
            !mode.declared_stdout.is_empty() || !mode.declared_stderr.is_empty(),
            "{}: an accepted case with no observable output would compare nothing",
            case.id
        );
        assert!(
            mode.controls
                .iter()
                .any(|control| control.expectation == "reject"),
            "{}: an accepted case needs a refused control, or the acceptance \
             says only `this route admits programs`",
            case.id
        );
        for control in &mode.controls {
            match control.expectation.as_str() {
                "accept" => {
                    assert!(
                        control.diagnostic_sequence.is_empty(),
                        "{}: an accepted control has no diagnostics",
                        case.id
                    );
                    assert!(
                        control.execution_profile.is_some(),
                        "{}: an accepted control must name the route it takes",
                        case.id
                    );
                }
                "reject" => {
                    assert!(
                        !control.diagnostic_sequence.is_empty(),
                        "{}: a refused control pins its codes",
                        case.id
                    );
                    assert!(control.execution_profile.is_none());
                }
                other => panic!("{}: unknown control expectation `{other}`", case.id),
            }
            assert!(required_files.insert(control.source.clone()));
        }
    }
    if let Some(split) = &expected.epoch_credit_split {
        // Card `CT22-WIRE`: the mode carries two case shapes. A rejection
        // (CT23) pins its exact ordered code vector and publishes no split of
        // its own; an acceptance (CT22) leaves the cascade empty and pins its
        // own published figures — the credit split and the composed region's
        // abandoned credit — so an acceptance can never be "it checks" alone.
        let case_accepts = split.diagnostic_sequence.is_empty();
        assert_eq!(
            case_accepts,
            split.abandoned.is_some(),
            "{}: an accepted credit-split case pins its published abandoned \
             figure; a rejection pins its cascade and no case figures",
            case.id
        );
        if case_accepts {
            // The law itself, on the case's declared figures, before anything
            // is compiled — exactly the accepted-control rule below.
            let mut sum: BTreeMap<&str, u64> = BTreeMap::new();
            for (axis, amount) in split.restored.iter().chain(split.spent.iter()) {
                *sum.entry(axis.as_str()).or_insert(0) += *amount;
            }
            let granted: BTreeMap<&str, u64> = split
                .granted
                .iter()
                .map(|(axis, amount)| (axis.as_str(), *amount))
                .collect();
            assert_eq!(
                sum, granted,
                "{}: the case states a split that does not add back to the grant",
                case.id
            );
        } else {
            assert!(
                split.restored.is_empty() && split.spent.is_empty(),
                "{}: a refused case publishes no split of its own",
                case.id
            );
            assert!(
                split.released.is_empty() && split.captures.is_none(),
                "{}: a refused case publishes no region duties and no captures",
                case.id
            );
        }
        assert!(
            !split.granted.is_empty(),
            "{}: the grant the split is measured against is part of the fixture",
            case.id
        );
        assert!(
            split
                .controls
                .iter()
                .any(|control| control.expectation == "accept"),
            "{}: a credit-split rejection needs an admitted control, or the \
             refusal is only `the profile cannot express this at all`",
            case.id
        );
        assert!(
            split
                .controls
                .iter()
                .any(|control| control.expectation == "reject"),
            "{}: the mint direction is part of the law — an admitted-only \
             control set would not pin the other refusal",
            case.id
        );
        for control in &split.controls {
            match control.expectation.as_str() {
                "accept" => {
                    assert!(
                        control.diagnostic_sequence.is_empty(),
                        "{}: an accepted control has no diagnostics",
                        case.id
                    );
                    // The law itself, on the declared figures: the split must
                    // add back to the grant, componentwise, before anything is
                    // compiled — a fixture stating figures that cannot conserve
                    // is wrong on its face.
                    let mut sum: BTreeMap<&str, u64> = BTreeMap::new();
                    for (axis, amount) in control.restored.iter().chain(control.spent.iter()) {
                        *sum.entry(axis.as_str()).or_insert(0) += *amount;
                    }
                    let granted: BTreeMap<&str, u64> = split
                        .granted
                        .iter()
                        .map(|(axis, amount)| (axis.as_str(), *amount))
                        .collect();
                    assert_eq!(
                        sum, granted,
                        "{}: control {} states a split that does not add back to \
                         the grant",
                        case.id, control.source
                    );
                    // Card `CT-WIRE-LANE-3`: a released duty is region
                    // evidence, so a control that publishes no region cannot
                    // pin one.
                    assert!(
                        control.abandoned.is_some() || control.released.is_empty(),
                        "{}: control {} pins released duties but publishes no region",
                        case.id,
                        control.source
                    );
                }
                "reject" => {
                    assert!(
                        !control.diagnostic_sequence.is_empty(),
                        "{}: a refused control pins its codes",
                        case.id
                    );
                    assert!(
                        control.restored.is_empty()
                            && control.spent.is_empty()
                            && control.abandoned.is_none()
                            && control.released.is_empty(),
                        "{}: a refused control publishes no split and no region figure",
                        case.id
                    );
                }
                other => panic!("{}: unknown control expectation `{other}`", case.id),
            }
            assert!(required_files.insert(control.source.clone()));
        }
    }
    if let Some(reject) = &expected.source_reject {
        assert!(
            !reject.diagnostic_sequence.is_empty(),
            "{}: a task-route rejection pins its exact ordered code vector",
            case.id
        );
        assert!(
            !reject.controls.is_empty(),
            "{}: a task-route rejection needs at least one control so the \
             refusal is not vacuous",
            case.id
        );
        for control in &reject.controls {
            match control.expectation.as_str() {
                "accept" => {
                    assert!(
                        control.diagnostic_sequence.is_empty(),
                        "{}: an accepted control has no diagnostics",
                        case.id
                    );
                    assert!(
                        control.execution_profile.is_some(),
                        "{}: an accepted control must name the route it takes, \
                         so a control cannot silently pass through another one",
                        case.id
                    );
                }
                "reject" => {
                    assert!(
                        !control.diagnostic_sequence.is_empty(),
                        "{}: a refused control pins its codes",
                        case.id
                    );
                    assert!(control.execution_profile.is_none());
                }
                other => panic!("{}: unknown control expectation `{other}`", case.id),
            }
            assert!(required_files.insert(control.source.clone()));
        }
    }
    if let Some(reject) = &expected.bitfield_reject {
        assert_eq!(reject.diagnostic_sequence, ["Q661"]);
        assert!(
            reject.controls.len() >= 2
                && reject
                    .controls
                    .iter()
                    .all(|control| control.expectation == "accept"),
            "{}: CT10 requires both admitting attribution controls",
            case.id
        );
        for control in &reject.controls {
            assert!(control.diagnostic_sequence.is_empty());
            assert_eq!(
                control.execution_profile.as_deref(),
                Some("BitfieldConcurrencyV1")
            );
            assert!(required_files.insert(control.source.clone()));
        }
    }
    if let Some(reject) = &expected.sequential_reject {
        assert!(
            !reject.diagnostic_sequence.is_empty(),
            "{}: a sequential-route rejection pins its exact ordered code vector",
            case.id
        );
        assert!(
            reject
                .controls
                .iter()
                .any(|control| control.expectation == "accept"),
            "{}: a sequential-route rejection needs an admitted control, or the \
             refusal is only `this route cannot express the program at all`",
            case.id
        );
        assert!(
            reject.controls.len() >= 2,
            "{}: one admitted control shows the refusal is not vacuous; the \
             second is what separates the construct from its position",
            case.id
        );
        for control in &reject.controls {
            match control.expectation.as_str() {
                "accept" => {
                    assert!(
                        control.diagnostic_sequence.is_empty(),
                        "{}: an accepted control has no diagnostics",
                        case.id
                    );
                    assert!(
                        control.execution_profile.is_some(),
                        "{}: an accepted control must name the route it takes",
                        case.id
                    );
                }
                "reject" => {
                    assert!(
                        !control.diagnostic_sequence.is_empty(),
                        "{}: a refused control pins its codes",
                        case.id
                    );
                    assert!(control.execution_profile.is_none());
                }
                other => panic!("{}: unknown control expectation `{other}`", case.id),
            }
            assert!(required_files.insert(control.source.clone()));
        }
    }
    if let Some(figures) = &expected.epoch_bound_figures {
        assert_eq!(
            figures.execution_profile, "FlatEpochBoundV1",
            "{}: the accepted epoch cases take the flat-epoch route",
            case.id
        );
        assert!(
            !figures.figures.is_empty(),
            "{}: an accepted epoch case states the certificate it measured",
            case.id
        );
        check_epoch_controls_shape(&case.id, &figures.controls, &mut required_files);
    }
    if let Some(reject) = &expected.epoch_reject {
        assert!(
            !reject.diagnostic_sequence.is_empty(),
            "{}: an epoch-route rejection pins its exact ordered code vector",
            case.id
        );
        assert!(
            reject
                .controls
                .iter()
                .any(|control| control.expectation == "accept"),
            "{}: an epoch rejection needs an accepted control, or the refusal is \
             only `the profile cannot express this at all`",
            case.id
        );
        check_epoch_controls_shape(&case.id, &reject.controls, &mut required_files);
    }
    if let Some(twin) = &expected.legacy_c_twin {
        check_legacy_twin_shape(&case.id, twin, &mut required_files);
    }
    if let Some(control) = &expected.positive_control {
        assert_eq!(control.source, "joined_sibling.qr");
        assert_eq!(control.evidence_target, "linux");
        assert_eq!(control.joined_children, 2);
        required_files.insert(control.source.clone());
    }
    let actual_files = fs::read_dir(&directory)
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            assert!(
                entry.file_type().unwrap().is_file(),
                "CT fixtures cannot hide nested artifacts"
            );
            entry.file_name().into_string().unwrap()
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(
        actual_files, required_files,
        "{}: fixture inventory differs from executed inputs",
        case.id
    );

    let source_path = directory.join(case_file);
    assert!(
        source_path.is_file(),
        "{}: fixture source {} is missing",
        case.id,
        source_path.display()
    );
    let recorded = case.source_fixture.as_deref().unwrap_or_else(|| {
        panic!(
            "{}: a mapped case must record its source fixture path in the manifest",
            case.id
        )
    });
    assert_eq!(
        repo_root().join(recorded).canonicalize().ok(),
        source_path.canonicalize().ok(),
        "{}: manifest source_fixture `{recorded}` does not name the fixture source",
        case.id
    );
    let profile = case.semantic_profile.as_deref().unwrap_or_else(|| {
        panic!(
            "{}: a mapped case must declare its semantic profile in the manifest",
            case.id
        )
    });
    assert!(
        !profile.trim().is_empty(),
        "{}: empty semantic profile",
        case.id
    );

    // The three source routes run the fixture through `compile_files`; the
    // `legacy_c` route runs its C twin through the gateway instead, inside its
    // own runner, so there is no compiled module here at all.
    let result = if legacy {
        None
    } else {
        let source = fs::read_to_string(&source_path)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", source_path.display()));
        let files = [SourceFile::new(recorded.to_string(), source)];
        Some(compile_files(
            &files,
            &CompileOptions {
                entry: expected.entry.clone(),
                ..CompileOptions::default()
            },
        ))
    };

    // Per-stage manifest evidence is derived here, from this harness's own
    // dispatch plus what the compiler actually reported on this fixture, and the
    // manifest must record exactly that. This is the T00.1/T00.4 repair: the
    // manifest no longer carries hand-written status strings whose reading
    // depended on the route.
    let plan = match &result {
        Some(result) => {
            let observed_codes: Vec<&str> = match result {
                Ok(_) => Vec::new(),
                Err(diagnostics) => diagnostics
                    .iter()
                    .map(|diagnostic| diagnostic.code.as_str())
                    .collect(),
            };
            derive_stage_plan(&case.id, &expected, &observed_codes)
        }
        None => derive_legacy_stage_plan(
            &case.id,
            expected.legacy_c_twin.as_ref().expect("mode metadata"),
        ),
    };
    let route = case
        .route
        .as_deref()
        .unwrap_or_else(|| panic!("{}: a mapped case must declare its route", case.id));
    assert_eq!(
        route,
        route_for_case(&case.id, &expected.verdict),
        "{}: the declared route must be the one this verdict mode really takes",
        case.id
    );
    assert_eq!(
        case.harness_verdict_mode.as_deref(),
        Some(expected.verdict.as_str()),
        "{}: the manifest must record the harness mode that produced its stage evidence",
        case.id
    );
    let route_vocabulary: BTreeSet<&str> = route_stages(route).iter().copied().collect();
    let mut claimed_stages: Vec<&str> = case.stages.keys().map(String::as_str).collect();
    claimed_stages.sort_unstable();
    let mut all_stages: Vec<&str> = STAGE_NAMES.to_vec();
    all_stages.sort_unstable();
    assert_eq!(
        claimed_stages, all_stages,
        "{}: every case states every stage; a missing key is not an absent stage",
        case.id
    );
    for stage in STAGE_NAMES {
        let record = &case.stages[*stage];
        let planned = &plan[stage];
        assert!(
            STAGE_STATUSES.contains(&record.status.as_str()),
            "{}: unknown stage status `{}` for `{stage}`",
            case.id,
            record.status
        );
        if !route_vocabulary.contains(stage) {
            assert_eq!(
                record.status, "not_applicable",
                "{}: `{stage}` is not a stage of route `{route}`, so it is structurally \
                 absent, never unassessed",
                case.id
            );
        }
        assert_eq!(
            record.status, planned.status,
            "{}: stage `{stage}` claims `{}` but this harness derives `{}`",
            case.id, record.status, planned.status
        );
        match record.status.as_str() {
            "executed" => {
                let executor = record.executor.as_deref().unwrap_or_else(|| {
                    panic!("{}: `{stage}` is executed but names no executor", case.id)
                });
                assert!(
                    RUNNER_REGISTRY.contains(&executor),
                    "{}: `{stage}` names executor `{executor}`, which is not a runner in \
                     this harness",
                    case.id
                );
                assert_eq!(
                    Some(executor),
                    planned.executor,
                    "{}: `{stage}` is executed by a different runner than it claims",
                    case.id
                );
                let artifact = record.artifact.as_deref().unwrap_or_else(|| {
                    panic!("{}: `{stage}` is executed but names no artifact", case.id)
                });
                assert_eq!(
                    artifact,
                    planned.artifact.as_deref().unwrap_or(recorded),
                    "{}: `{stage}` must name the exact file its runner executed — the \
                     case's own fixture, or the one control this harness derives a \
                     native run for",
                    case.id
                );
                assert!(
                    repo_root().join(artifact).is_file(),
                    "{}: `{stage}` names artifact `{artifact}`, which does not exist",
                    case.id
                );
                assert_eq!(
                    record.observations, planned.observations,
                    "{}: `{stage}` observation count must equal the runner's invocations",
                    case.id
                );
                assert_eq!(
                    record
                        .host_gate
                        .as_ref()
                        .map(|gate| gate.iter().map(String::as_str).collect::<Vec<_>>()),
                    planned.host_gate.clone(),
                    "{}: `{stage}` must record the exact host gate of its runner, so the \
                     status cannot be read as host-unconditional",
                    case.id
                );
                for tool in record.tool_gate.iter().flatten() {
                    assert!(
                        TOOL_REGISTRY.contains(&tool.as_str()),
                        "{}: `{stage}` names tool `{tool}`, which is not a gate this harness \
                         knows how to require",
                        case.id
                    );
                }
                assert_eq!(
                    record
                        .tool_gate
                        .as_ref()
                        .map(|gate| gate.iter().map(String::as_str).collect::<Vec<_>>()),
                    planned.tool_gate.clone(),
                    "{}: `{stage}` must record the exact external tools its runner needs, so \
                     an `executed` claim cannot be read as unconditional on a host that has \
                     none of them",
                    case.id
                );
                assert!(record.record.is_none());
            }
            "recorded_not_gated" => {
                assert_eq!(
                    *stage, "native_sanitized",
                    "{}: only sanitizer evidence is currently recorded rather than gated",
                    case.id
                );
                let pointer = record.record.as_deref().unwrap_or_else(|| {
                    panic!(
                        "{}: `{stage}` is recorded_not_gated but names no record",
                        case.id
                    )
                });
                assert!(
                    repo_root().join(pointer).is_file(),
                    "{}: `{stage}` names record `{pointer}`, which does not exist",
                    case.id
                );
                assert!(
                    record.executor.is_none()
                        && record.artifact.is_none()
                        && record.observations.is_none()
                        && record.host_gate.is_none()
                        && record.tool_gate.is_none(),
                    "{}: a recorded transcript has no executor, artifact, count or host \
                     gate — that is what distinguishes it from executed evidence",
                    case.id
                );
            }
            _ => assert!(
                record.executor.is_none()
                    && record.artifact.is_none()
                    && record.observations.is_none()
                    && record.host_gate.is_none()
                    && record.tool_gate.is_none()
                    && record.record.is_none(),
                "{}: `{stage}` is `{}` and may carry no evidence fields",
                case.id,
                record.status
            ),
        }
    }
    // T00.3: the sanitizer gate. `executed` here is admissible only for a case
    // a runner really compiles with `-fsanitize=…` and runs.
    assert_eq!(
        case.stages["native_sanitized"].status == "executed",
        SANITIZED_GATED_CASES.contains(&case.id.as_str()),
        "{}: a sanitized-native claim requires the harness-gated run",
        case.id
    );
    assert_eq!(
        case.stages["native_sanitized"].status == "recorded_not_gated",
        SANITIZER_TRANSCRIPT_CASES.contains(&case.id.as_str()),
        "{}: transcript-only sanitizer evidence is pinned to its own case set",
        case.id
    );

    // The fixture repeats its executed stage set in the same vocabulary, and it
    // must agree with what the manifest and this harness derived.
    let executed: BTreeSet<&str> = expected
        .stages_executed
        .iter()
        .map(String::as_str)
        .collect();
    for stage in &executed {
        assert!(
            STAGE_NAMES.contains(stage),
            "{}: unknown stage `{stage}` in expected.json",
            case.id
        );
    }
    let derived_executed: BTreeSet<&str> = STAGE_NAMES
        .iter()
        .copied()
        .filter(|stage| plan[stage].status == "executed")
        .collect();
    assert_eq!(
        executed, derived_executed,
        "{}: expected.json's stages_executed must match the derived plan",
        case.id
    );
    assert!(
        executed.contains("parse"),
        "{}: a mapped case must at least admit its own fixture",
        case.id
    );
    assert_eq!(
        executed.contains("elaborate"),
        !legacy,
        "{}: elaboration is a QCore-source stage — the `legacy_c` route has no QCore \
         source to elaborate, and every other route must reach it",
        case.id
    );

    if let Some(control) = &expected.positive_control {
        run_positive_control(&directory, control);
    }
    if let Some(reject) = &expected.checked_mutex_reject {
        run_mutex_controls(&directory, &case.id, reject);
    }
    if let Some(reject) = &expected.source_reject {
        run_source_controls(&directory, &case.id, reject);
    }
    if let Some(reject) = &expected.bitfield_reject {
        run_bitfield_controls(&directory, &case.id, reject);
    }
    if let Some(reject) = &expected.sequential_reject {
        run_sequential_controls(&directory, &case.id, reject);
    }
    if let Some(reject) = &expected.epoch_reject {
        run_epoch_controls(&directory, &case.id, &reject.controls);
    }
    if let Some(twin) = &expected.legacy_c_twin {
        assert!(
            expected.diagnostics.is_empty()
                && expected.outcome.is_none()
                && expected.trace.is_none(),
            "{}: a C twin has no QCore diagnostics and no execution post-state; its \
             evidence is the gateway's verdict",
            case.id
        );
        run_legacy_c_twin(&directory, &case.id, &expected, twin);
        return;
    }
    let result = result.expect("a source-route case compiles its own fixture");
    match expected.verdict.as_str() {
        "epoch_bound_figures" => {
            assert!(
                expected.diagnostics.is_empty()
                    && expected.outcome.is_none()
                    && expected.trace.is_none(),
                "{}: an admitted epoch has no execution post-state; its evidence is \
                 the certificate",
                case.id
            );
            run_epoch_bound_figures(
                &directory,
                &case.id,
                expected
                    .epoch_bound_figures
                    .as_ref()
                    .expect("mode metadata"),
                result,
            );
        }
        "native_heap_scenarios" => {
            assert!(
                expected.diagnostics.is_empty()
                    && expected.outcome.is_none()
                    && expected.trace.is_none()
            );
            run_native_heap_scenarios(&directory, &expected, result);
        }
        "checked_mutex_sanitized" => {
            assert!(
                expected.diagnostics.is_empty()
                    && expected.outcome.is_none()
                    && expected.trace.is_none()
            );
            run_checked_mutex_sanitized(&expected, result);
        }
        "checked_mutex_crossing" => {
            assert!(
                expected.diagnostics.is_empty()
                    && expected.outcome.is_none()
                    && expected.trace.is_none(),
                "{}: this mode's evidence is the lowered crossing and its \
                 controls, not an execution post-state",
                case.id
            );
            run_mutex_crossing(
                &directory,
                &case.id,
                expected
                    .checked_mutex_crossing
                    .as_ref()
                    .expect("mode metadata"),
                result,
            );
        }
        "reordering_partial_order" => {
            assert!(
                expected.diagnostics.is_empty()
                    && expected.outcome.is_none()
                    && expected.trace.is_none(),
                "{}: this mode's post-state is two stream relations, not a \
                 sequential outcome and trace",
                case.id
            );
            run_reordering_partial_order(&directory, &expected, result);
        }
        "sequential_sanitized" => {
            assert!(
                expected.diagnostics.is_empty()
                    && expected.outcome.is_none()
                    && expected.trace.is_none(),
                "{}: this mode's post-state is the per-stream observation, not \
                 a sequential outcome and trace",
                case.id
            );
            run_sequential_sanitized(&directory, &case.id, &expected, result);
        }
        "epoch_credit_split" => {
            assert!(
                expected.outcome.is_none() && expected.trace.is_none(),
                "{}: this mode's post-state is the published credit quantities, \
                 not a sequential outcome and trace",
                case.id
            );
            // A refused case (CT23) pins at least one diagnostic beside its
            // cascade; an accepted case (CT22, card `CT22-WIRE`) has none —
            // the two fields must agree with the mode metadata's own shape.
            assert_eq!(
                expected.diagnostics.is_empty(),
                expected
                    .epoch_credit_split
                    .as_ref()
                    .expect("mode metadata")
                    .diagnostic_sequence
                    .is_empty(),
                "{}: expected diagnostics accompany the cascade exactly when \
                 the case refuses",
                case.id
            );
            run_epoch_credit_split(&directory, &case.id, &expected, result);
        }
        "native_task_scenarios" => {
            assert!(
                expected.diagnostics.is_empty()
                    && expected.outcome.is_none()
                    && expected.trace.is_none()
            );
            run_native_task_scenarios(&directory, &expected, result);
        }
        "native_owned_transfer" => {
            assert!(
                expected.diagnostics.is_empty()
                    && expected.outcome.is_none()
                    && expected.trace.is_none()
            );
            run_native_owned_transfer(&directory, &expected, result);
        }
        "native_create_recovery" => {
            assert!(
                expected.diagnostics.is_empty()
                    && expected.outcome.is_none()
                    && expected.trace.is_none()
            );
            run_native_create_recovery(&directory, &expected, result);
        }
        "native_join_failure" => {
            assert!(
                expected.diagnostics.is_empty()
                    && expected.outcome.is_none()
                    && expected.trace.is_none(),
                "native fault mode has its own explicit outcome expectations"
            );
            run_native_join_failure(&directory, &expected, result);
        }
        "accept" => {
            assert!(
                expected.diagnostics.is_empty(),
                "{}: an accept case must not expect diagnostics",
                case.id
            );
            let module = result.unwrap_or_else(|diagnostics| {
                panic!(
                    "{}: expected acceptance but the checker rejected:\n{}",
                    case.id,
                    diagnostics
                        .iter()
                        .map(format_diagnostic)
                        .collect::<Vec<_>>()
                        .join("\n")
                )
            });
            let wants_execution = executed.contains("sequential_reference_execution");
            assert_eq!(
                wants_execution,
                expected.outcome.is_some(),
                "{}: the reference execution stage and the expected outcome must agree",
                case.id
            );
            if let Some(outcome) = &expected.outcome {
                let observation = module.reference_observation.as_ref().unwrap_or_else(|| {
                    panic!(
                        "{}: expected a reference execution observation (executable module)",
                        case.id
                    )
                });
                match &observation.outcome {
                    ExecutionOutcome::Ok { value, payload } => {
                        assert_eq!(outcome.error, None, "{}: expected error outcome", case.id);
                        assert_eq!(
                            outcome.ok_value,
                            Some(*value),
                            "{}: reference execution returned {value}",
                            case.id
                        );
                        let expected_payload = outcome.payload.as_deref().unwrap_or_else(|| {
                            panic!(
                                "{}: a normal-exit case must pin the completion-record payload",
                                case.id
                            )
                        });
                        assert_eq!(
                            expected_payload,
                            payload.as_slice(),
                            "{}: completion-record payload mismatch",
                            case.id
                        );
                    }
                    ExecutionOutcome::Error { name, .. } => {
                        assert_eq!(
                            outcome.ok_value, None,
                            "{}: expected ok outcome, got error `{name}`",
                            case.id
                        );
                        assert!(
                            outcome.payload.is_none(),
                            "{}: an error exit carries no completion-record payload",
                            case.id
                        );
                        assert_eq!(
                            outcome.error.as_deref(),
                            Some(name.as_str()),
                            "{}: reference execution raised `{name}`",
                            case.id
                        );
                    }
                }
                let actual_trace: Vec<String> = observation.trace.iter().map(trace_token).collect();
                let expected_trace = expected.trace.as_ref().unwrap_or_else(|| {
                    panic!(
                        "{}: an executed accept case must pin its acquisition/release trace",
                        case.id
                    )
                });
                assert_eq!(
                    &actual_trace, expected_trace,
                    "{}: reference execution trace mismatch",
                    case.id
                );
            }
        }
        "reject" | "bitfield_reject" => {
            assert!(
                expected.outcome.is_none() && expected.trace.is_none(),
                "{}: a reject case has no execution post-state",
                case.id
            );
            assert!(
                !expected.diagnostics.is_empty(),
                "{}: a reject case must pin at least one expected diagnostic",
                case.id
            );
            let diagnostics = match result {
                Err(diagnostics) => diagnostics,
                Ok(_) => panic!(
                    "{}: expected rejection but the checker accepted the fixture",
                    case.id
                ),
            };
            let rendered = diagnostics
                .iter()
                .map(format_diagnostic)
                .collect::<Vec<_>>()
                .join("\n");
            if let Some(reject) = &expected.checked_mutex_reject {
                // The whole cascade, in order, not merely its head: a refused
                // release leaves the guard live, which blocks scope-exit
                // destruction and surfaces in the end-of-trace sweep. Pinning
                // every entry is what shows the refusal did not silently
                // release the guard.
                assert_eq!(
                    diagnostics
                        .iter()
                        .map(|diagnostic| diagnostic.code.clone())
                        .collect::<Vec<_>>(),
                    reject.diagnostic_sequence,
                    "{}: exact diagnostic cascade mismatch; actual:\n{rendered}",
                    case.id
                );
            }
            if let Some(reject) = &expected.epoch_reject {
                // Same discipline on the epoch route. CT36's own control shows
                // why the whole vector matters here: one byte below the summed
                // peak refuses on the credits *and* on the epoch, so a
                // head-only assertion would not distinguish a per-task check
                // from a whole-epoch one.
                assert_eq!(
                    diagnostics
                        .iter()
                        .map(|diagnostic| diagnostic.code.clone())
                        .collect::<Vec<_>>(),
                    reject.diagnostic_sequence,
                    "{}: exact diagnostic cascade mismatch; actual:\n{rendered}",
                    case.id
                );
            }
            if let Some(reject) = &expected.sequential_reject {
                // Same discipline on the sequential route. It matters here for
                // a reason of its own: the borrow window and the inherited
                // ownership checker report through one code, so a fixture that
                // pinned only a head could not tell a window refusal from a
                // second, unrelated custody refusal in the same program.
                assert_eq!(
                    diagnostics
                        .iter()
                        .map(|diagnostic| diagnostic.code.clone())
                        .collect::<Vec<_>>(),
                    reject.diagnostic_sequence,
                    "{}: exact diagnostic cascade mismatch; actual:\n{rendered}",
                    case.id
                );
            }
            if let Some(reject) = &expected.source_reject {
                // Same discipline on the task route: the whole vector, in
                // order. A refusal that also silently dropped a custody
                // obligation would still satisfy a head-only assertion.
                assert_eq!(
                    diagnostics
                        .iter()
                        .map(|diagnostic| diagnostic.code.clone())
                        .collect::<Vec<_>>(),
                    reject.diagnostic_sequence,
                    "{}: exact diagnostic cascade mismatch; actual:\n{rendered}",
                    case.id
                );
            }
            if let Some(reject) = &expected.bitfield_reject {
                assert_eq!(
                    diagnostics
                        .iter()
                        .map(|diagnostic| diagnostic.code.clone())
                        .collect::<Vec<_>>(),
                    reject.diagnostic_sequence,
                    "{}: exact bit-field diagnostic vector mismatch; actual:\n{rendered}",
                    case.id
                );
            }
            for want in &expected.diagnostics {
                assert!(
                    diagnostics
                        .iter()
                        .any(|diagnostic| diagnostic.code == want.code
                            && diagnostic.message.contains(&want.message_contains)),
                    "{}: no diagnostic with code `{}` containing `{}`; actual:\n{rendered}",
                    case.id,
                    want.code,
                    want.message_contains
                );
            }
        }
        other => panic!("{}: unknown verdict vocabulary `{other}`", case.id),
    }
}

#[test]
fn ct_manifest_and_fixture_set_agree_and_mapped_cases_execute() {
    let manifest = load_manifest();
    assert_eq!(manifest.schema_version, 2);
    assert_eq!(manifest.cases.len(), TOTAL_CASES);
    // The manifest states its own vocabulary so a reader knows what the words
    // mean; this file is the authority, so data can never widen it.
    assert_eq!(
        manifest.stage_vocabulary.stages, STAGE_NAMES,
        "manifest stage vocabulary drifted from the harness"
    );
    assert_eq!(
        manifest.stage_vocabulary.statuses, STAGE_STATUSES,
        "manifest status vocabulary drifted from the harness"
    );
    assert_eq!(
        manifest
            .stage_vocabulary
            .routes
            .iter()
            .map(|(name, stages)| (
                name.as_str(),
                stages.iter().map(String::as_str).collect::<Vec<_>>()
            ))
            .collect::<Vec<_>>(),
        ROUTE_STAGES
            .iter()
            .map(|(name, stages)| (*name, stages.to_vec()))
            .collect::<Vec<_>>(),
        "manifest route vocabulary drifted from the harness"
    );
    for (_, stages) in ROUTE_STAGES {
        for stage in *stages {
            assert!(
                STAGE_NAMES.contains(stage),
                "route vocabulary names `{stage}`, which is not a stage"
            );
        }
    }

    let mut mapped = Vec::new();
    let mut unmapped = Vec::new();
    for (index, case) in manifest.cases.iter().enumerate() {
        assert_eq!(
            case.id,
            format!("CT{:02}", index + 1),
            "manifest cases must stay CT01..CT50 in order"
        );
        assert_eq!(
            case.spec_section, "55",
            "{}: CT cases pin spec §55",
            case.id
        );
        let mut stage_keys: Vec<&str> = case.stages.keys().map(String::as_str).collect();
        stage_keys.sort_unstable();
        let mut expected_keys: Vec<&str> = STAGE_NAMES.to_vec();
        expected_keys.sort_unstable();
        assert_eq!(stage_keys, expected_keys, "{}: stage schema drift", case.id);

        let directory = fixture_root().join(&case.id);
        match case.mapping_status.as_str() {
            "unmapped" => {
                assert!(
                    !directory.exists(),
                    "{}: fixture directory exists but the manifest still says `unmapped` — \
                     advance the manifest (and MAPPED_CASES) deliberately or remove the fixture",
                    case.id
                );
                assert!(
                    case.source_fixture.is_none()
                        && case.semantic_profile.is_none()
                        && case.evidence_target.is_none()
                        && case.route.is_none()
                        && case.harness_verdict_mode.is_none(),
                    "{}: an unmapped case must not carry fixture metadata",
                    case.id
                );
                // An unmapped case has no route, so no stage of it is
                // structurally absent either: every stage is genuinely
                // unassessed, and carries no evidence field.
                assert!(
                    case.stages.values().all(|record| record
                        == &StageRecord {
                            status: "unassessed".to_owned(),
                            executor: None,
                            artifact: None,
                            observations: None,
                            host_gate: None,
                            tool_gate: None,
                            record: None,
                        }),
                    "{}: an unmapped case must keep all stages `unassessed` with no evidence",
                    case.id
                );
                unmapped.push(case.id.clone());
            }
            "mapped" => {
                run_mapped_case(case);
                mapped.push(case.id.clone());
            }
            other => panic!(
                "{}: unknown mapping_status `{other}` — the harness fails closed on \
                 unrecognized status vocabulary",
                case.id
            ),
        }
    }

    assert_eq!(
        mapped, MAPPED_CASES,
        "mapped case set drifted from the pinned expectation"
    );
    assert_eq!(
        unmapped.len(),
        TOTAL_CASES - MAPPED_CASES.len(),
        "unmapped CT cases ({}): {unmapped:?}",
        unmapped.len()
    );
}

/// The moving count is the W6 genericity metric: 42 of 50 CT cases are mapped
/// as executed tests over their own fixture; 8 remain unmapped. Thirty-nine of
/// the forty-two are source-level; CT40, CT43 and CT45 are C twins executed
/// through
/// the unrestricted-C gateway, which is a different artifact and a different
/// property, so they are counted here and separated in the report.
/// CT13/CT15/CT26 and the
/// CT03/CT17 sanitized heap scenarios and the admitted controls carried by
/// CT04/CT05/CT18/CT27/CT28/CT29/CT30/CT31/CT32/CT33/CT34/CT48 execute on Linux and macOS.
/// The manifest's `evidence_target` still identifies the archived Linux evidence
/// run; for CT28/CT31/CT32 it records only the `Q500` host gate, because those
/// accepted controls are multi-task modules this backend renders in no thread at
/// all (published checked with `c_source: None`, card T03-F §3.5). CT34 records
/// a real compiled and sanitizer-run control (card `CT34-SANITIZED`), CT49
/// sixteen sanitized runs of its own emitted C (card `CT-WIRE-LANE`), and CT07
/// and CT09 one sanitized run each of their own emitted C (card
/// `CT-WIRE-LANE-2`). The sanitized-native and proof numerators are NOT
/// written here: the authority is `PROOF_GATED_CASES` above, the runner
/// registry and the manifest (read through `scripts/ct_stage_summary.py
/// --check`), and the ledger row in `AGENTS.md` carries the current figures.
/// This sentence used to carry them as prose no test asserts, and drifted
/// three times — it read "0 of 42" until card `CT-WIRE-LANE-3` (the staleness
/// the `CT07-WRITE-REMAP` report §6 observed in passing), "2 of 43" until card
/// `W32-RECONCILE-SWEEP` (because `CT03-PROOF-ALPHABET` merged after the lane
/// that had just refreshed it), and then "7 of 43" / "3 of 43" from wave 32
/// until the wave-39 close found it against a ledger at 10 of 47 / 4 of 47
/// (card `CT07-TWO-THREAD-PROOF-LEG` §11 item 6 asked for the decision). The
/// figures were removed by the wave-39 close rather than refreshed a fourth
/// time, which is the repair the drift itself argued for.
/// CT06, CT08, CT11 and CT21 add none: their fixtures are refusals that emit
/// nothing, and CT23's is too. CT22 (card `CT22-WIRE`, wave 29) adds none
/// either, deliberately: its admitted module emits the task route's own C,
/// but no runner of this harness executes that emission today, so its
/// emission and native stages read `unassessed` — open items, not claims —
/// and CT25 (card `CT-WIRE-LANE-3`, wave 31) sits on exactly that footing
/// beside it, its `CT-COMPOSED-NATIVE` follow-on already named.
/// Mapping is not conformance: the wave-29 and wave-31 denominators each grew
/// by one accepted composed-epoch program that contributes no sanitizer
/// evidence.
/// This test exists so the counts are visible and any drift is a deliberate
/// edit, not an accident.
#[test]
fn ct_mapping_progress_is_pinned_and_counted() {
    let manifest = load_manifest();
    let mapped = manifest
        .cases
        .iter()
        .filter(|case| case.mapping_status == "mapped")
        .count();
    let unmapped = manifest
        .cases
        .iter()
        .filter(|case| case.mapping_status == "unmapped")
        .count();
    assert_eq!(mapped, MAPPED_CASES.len(), "mapped CT case count");
    assert_eq!(
        unmapped,
        TOTAL_CASES - MAPPED_CASES.len(),
        "unmapped CT case count"
    );
    assert_eq!(mapped + unmapped, TOTAL_CASES, "every case is counted");
}

fn stage_depth(stage: &str) -> u8 {
    STAGE_DEPTH
        .iter()
        .find(|(name, _)| *name == stage)
        .unwrap_or_else(|| panic!("stage `{stage}` has no depth rank"))
        .1
}

/// The stage-derived CT summary (T00.9). Every number here is computed from the
/// manifest's per-stage evidence, which `run_mapped_case` has independently
/// checked against this harness's own dispatch. Nothing is hand-kept, and the
/// `mapped`/`unmapped` pair is retained beside the derived rows rather than
/// standing alone: on its own it treats a source-rejection case and a
/// five-scenario sanitized case as one identical unit of coverage.
fn ct_stage_summary(manifest: &Manifest) -> serde_json::Value {
    use serde_json::{Map, Value, json};
    let mut histogram: BTreeMap<&str, BTreeMap<&str, usize>> = STAGE_NAMES
        .iter()
        .map(|stage| (*stage, BTreeMap::new()))
        .collect();
    let mut observations: BTreeMap<&str, usize> = BTreeMap::new();
    let mut host_scope: BTreeMap<&str, BTreeSet<String>> = BTreeMap::new();
    let mut tool_scope: BTreeMap<&str, BTreeSet<String>> = BTreeMap::new();
    let mut depth_profile: BTreeMap<&str, usize> = BTreeMap::new();
    let mut case_depth: BTreeMap<&str, &str> = BTreeMap::new();
    let mut mapped = 0usize;
    let mut unmapped = 0usize;
    for case in &manifest.cases {
        if case.mapping_status != "mapped" {
            unmapped += 1;
            continue;
        }
        mapped += 1;
        let mut deepest: Option<&'static str> = None;
        for stage in STAGE_NAMES {
            let record = &case.stages[*stage];
            *histogram
                .get_mut(stage)
                .expect("stage")
                .entry(record.status.as_str())
                .or_insert(0) += 1;
            if record.status != "executed" {
                continue;
            }
            *observations.entry(stage).or_insert(0) += record
                .observations
                .expect("an executed stage counts its runs");
            let hosts = host_scope.entry(stage).or_default();
            match &record.host_gate {
                Some(gate) => hosts.extend(gate.iter().cloned()),
                None => {
                    hosts.insert("any".to_owned());
                }
            }
            // The tool axis is separate from the host axis on purpose: a C-twin
            // stage runs on any target and only where Clang exists, and reading
            // "any" on the host axis alone would say the opposite.
            let tools = tool_scope.entry(stage).or_default();
            match &record.tool_gate {
                Some(gate) => tools.extend(gate.iter().cloned()),
                None => {
                    tools.insert("none".to_owned());
                }
            }
            if deepest.is_none_or(|current| stage_depth(stage) > stage_depth(current)) {
                deepest = Some(stage);
            }
        }
        let deepest = deepest.expect("a mapped case executes at least the parser");
        *depth_profile.entry(deepest).or_insert(0) += 1;
        case_depth.insert(case.id.as_str(), deepest);
    }
    let mut histogram_json = Map::new();
    for stage in STAGE_NAMES {
        histogram_json.insert(
            (*stage).to_owned(),
            json!(
                histogram[stage]
                    .iter()
                    .map(|(status, count)| ((*status).to_owned(), *count))
                    .collect::<BTreeMap<String, usize>>()
            ),
        );
    }
    json!({
        "mapped": mapped,
        "unmapped": unmapped,
        "stage_histogram": Value::Object(histogram_json),
        "observation_counts": observations,
        "host_scope": host_scope,
        "tool_scope": tool_scope,
        "depth_profile": depth_profile,
        "case_depth": case_depth,
        "unmapped_bucket_counts": unmapped_bucket_counts(manifest),
        "unmapped_label_sources": unmapped_label_sources(manifest),
    })
}

/// The per-card CT stage-evidence records, in landing order, mirroring
/// `scripts/ct_stage_summary.py::EVIDENCE_FILES`. The two lists are tied
/// together through a third committed record: `acceptance.json`'s
/// `ct_evidence.derived_from` is asserted against this array below, and the
/// Python generator asserts the same field against its own list, so neither
/// derivation can quietly read a different file set (card CT-LABELS).
const EVIDENCE_FILES: [&str; 29] = [
    "docs/reports/remaining-t00/coverage.json",
    "docs/reports/remaining-t03-ct-integration/ct_stage_evidence.json",
    "docs/reports/remaining-wave3-ct-integration/ct_stage_evidence.json",
    "docs/reports/wave5-ct-integration/ct_stage_evidence.json",
    "docs/reports/wave8-epoch-ct/ct_stage_evidence.json",
    "docs/reports/wave9-ctwin/ct_stage_evidence.json",
    "docs/reports/wave11-ct33/ct_stage_evidence.json",
    "docs/reports/wave13-ct34/ct_stage_evidence.json",
    "docs/reports/remaining-ct34-sanitized/ct_stage_evidence.json",
    "docs/reports/wave17-ct40/ct_stage_evidence.json",
    "docs/reports/ct-wire-lane/ct_stage_evidence.json",
    "docs/reports/ct-wire-lane-2/ct_stage_evidence.json",
    "docs/reports/ct22-wire/ct_stage_evidence.json",
    "docs/reports/ct01-proof-leg/ct_stage_evidence.json",
    "docs/reports/ct-proof-leg-2/ct_stage_evidence.json",
    "docs/reports/ct07-write-remap/ct_stage_evidence.json",
    "docs/reports/ct-wire-lane-3/ct_stage_evidence.json",
    "docs/reports/ct03-proof-alphabet/ct_stage_evidence.json",
    "docs/reports/ct-composed-native/ct_stage_evidence.json",
    "docs/reports/ct-wire-lane-4/ct_stage_evidence.json",
    // Card `CT-CTWIN-LANE` (wave 35): CT46 mapped, and the four cases this lane
    // re-measured and deliberately left unmapped. It is also the record that
    // retires wave 13's CT46 bucket row, by declaration rather than by editing
    // that record (constraint 21).
    "docs/reports/ct-ctwin-lane/ct_stage_evidence.json",
    // Card `MUTEX-SHARE-CROSS-EMISSION` (wave 36): CT46's four unassessed
    // stages move to executed, the backend having learned to render a
    // non-nested fork/join trace. A superseding record per constraint 21 — the
    // ct-ctwin-lane record keeps its bytes.
    "docs/reports/mutex-share-cross-emission/ct_stage_evidence.json",
    // Card `CT-LANE-5` (wave 37): CT07 re-mapped from the sequential write lend
    // onto the two-thread range write lease — the scenario's own word
    // "independently" resting on two threads for the first time. A superseding
    // record per constraint 21; the ct07-write-remap record keeps its bytes.
    // It maps no new case, so the mapped count does not move.
    "docs/reports/ct-lane-5/ct_stage_evidence.json",
    // Card `CT07-TWO-THREAD-PROOF-LEG` (wave 39): CT07's proof stage moves to
    // executed on the ledger-bound leg. A superseding record per constraint
    // 21; the ct-lane-5 record keeps its bytes, and every other stage row is
    // carried from it unchanged.
    "docs/reports/ct07-two-thread-proof-leg/ct_stage_evidence.json",
    // Card `CT42-CELL-ROUTE` (wave 39): CT42 mapped on the sequential route,
    // the socket arm the wave-35 lane measured unreachable from source now
    // routed through `disjoint NAME NAME;`. It retires the ct-ctwin-lane
    // record's CT42 bucket row by declaration; that record keeps its bytes.
    "docs/reports/ct42-cell-route/ct_stage_evidence.json",
    // Card `CT-PROOF-LEG-3` (wave 41): CT50's proof stage moves to executed on
    // the F02-bound leg. It supersedes card T00's frozen archive for CT50 —
    // the record that still held that case's stages — by declaration;
    // `remaining-t00/coverage.json` keeps its bytes (constraint 21). Every
    // other stage row is carried from it unchanged.
    "docs/reports/ct-proof-leg-3/ct_stage_evidence.json",
    // Card `CT24-CLEANUP-TASK-COMPOSITION`: CT24 maps at the composed route's
    // task-ledger refusal. It adds no native or proof stage.
    "docs/reports/ct24-cleanup-task-composition/ct_stage_evidence.json",
    // Card `CT10-SOURCE-LAYOUT`: CT10's actual field-write captures reach the
    // footprint checker on a property-only route, with no task-ledger claim.
    "docs/reports/ct10-source-layout/ct_stage_evidence.json",
    "docs/reports/ct41-publication-mapping/ct_stage_evidence.json",
];

/// The `label_source` of a case no CT lane measured: card T00's frozen archive.
const ARCHIVE_SOURCE: &str = "t00-archive";

/// The declared retirements of earlier lanes' `unmapped_reasons` rows, keyed by
/// case (wave-17 CT lane). See [`unmapped_labels`] for why this exists and what
/// each condition refuses; this function checks them and returns
/// `{case: retiring record}`.
fn lane_row_retirements(
    records: &[(&'static str, serde_json::Value)],
    unmapped: &BTreeSet<&str>,
) -> BTreeMap<String, String> {
    // Which record most recently published a bucket for each case, regardless of
    // whether that case is still unmapped. A retirement must name exactly this
    // record, so it cannot name a row a later supersession already replaced.
    let mut newest_row: BTreeMap<&str, &str> = BTreeMap::new();
    for (relative, record) in records {
        if let Some(rows) = record.get("unmapped_reasons") {
            for case_id in rows
                .as_object()
                .expect("unmapped_reasons is an object")
                .keys()
            {
                newest_row.insert(case_id.as_str(), relative);
            }
        }
    }
    let position = |wanted: &str| {
        records
            .iter()
            .position(|(relative, _)| *relative == wanted)
            .unwrap_or_else(|| panic!("`{wanted}` is not a CT stage-evidence record"))
    };

    let mut retirements: BTreeMap<String, String> = BTreeMap::new();
    for (index, (relative, record)) in records.iter().enumerate() {
        let Some(rows) = record.get("retired_unmapped_reasons") else {
            continue;
        };
        for (case_id, row) in rows
            .as_object()
            .expect("retired_unmapped_reasons is an object")
        {
            assert!(
                !unmapped.contains(case_id.as_str()),
                "{case_id}: {relative} retires this case's bucket, but the manifest still \
                 leaves the case unmapped — its label is live"
            );
            for field in ["retires", "mapped_by"] {
                let value = row
                    .get(field)
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("");
                assert!(
                    !value.trim().is_empty(),
                    "{case_id}: {relative} carries no {field} for this retirement"
                );
            }
            let retires = row["retires"].as_str().expect("retires");
            let held = newest_row
                .get(case_id.as_str())
                .copied()
                .unwrap_or_else(|| {
                    panic!(
                        "{case_id}: {relative} retires a bucket, but no evidence record publishes \
                     one for this case"
                    )
                });
            assert_eq!(
                retires, held,
                "{case_id}: {relative} retires the bucket in {retires}, but the record holding \
                 it is {held}"
            );
            assert!(
                position(retires) < index,
                "{case_id}: {relative} retires {retires}, which is not an earlier record"
            );
            assert!(
                retirements
                    .insert(case_id.clone(), (*relative).to_owned())
                    .is_none(),
                "{case_id}: two evidence records retire this case's bucket"
            );
        }
    }
    retirements
}

/// Each unmapped case's current bucket label beside the record that measured it
/// (card CT-LABELS, finding `D02-BUCKETS-LABELS`).
///
/// Card HYGIENE-14 derived the *set* of counted cases from the manifest and
/// recorded that the *label* stayed card T00's 2026-09-07 analysis, measurably
/// stale for CT11 and CT46. The repair is the same shape as its predecessor: a
/// CT lane that examines a case and leaves it unmapped records the bucket it
/// measured in its own `ct_stage_evidence.json`, under `unmapped_reasons`, and
/// the newest such record wins over the archive. The source is published beside
/// the label, so "this is a current measurement" and "this is still the archive"
/// are distinguishable without reading three reports.
///
/// Total-or-reject, matching `scripts/ct_stage_summary.py` exactly:
///
/// * a lane row naming a case the manifest currently maps is refused — a mapped
///   case has no blocker to publish — unless a later record *retires* it, see
///   below;
/// * two lane records labelling one case differently with no supersession
///   declared is refused, and a supersession that is stale or vacuous is refused
///   with it;
/// * a case with neither a lane row nor an archived row is a hole, not a zero.
///
/// **Retirement** (wave-17 CT lane). Mapping a case whose bucket an *earlier*
/// lane recorded used to leave two bad options: refuse, or edit that lane's
/// archived record — the in-place rewrite of a frozen archive that card
/// `CT-LABELS` built this whole mechanism to replace and card `HYGIENE-16`
/// reverted. The third option is the one `supersedes` already uses for labels:
/// the mapping lane declares the retirement in *its own* record, under
/// `retired_unmapped_reasons`, as `{CASE: {retires, mapped_by}}`, and the
/// earlier record stays byte-identical. Total-or-reject in every direction that
/// could hide something:
///
/// * a retirement naming a case the manifest still leaves **unmapped** is
///   refused — that would silently drop a live label;
/// * `retires` must name a record in [`EVIDENCE_FILES`] that really carries an
///   `unmapped_reasons` row for that case, and must be the **newest** such
///   record, so a retirement cannot go stale past a supersession;
/// * the retiring record must land strictly **later** than the record it
///   retires;
/// * two records retiring one case is refused;
/// * and a row for a mapped case with no retirement at all still fails closed,
///   exactly as before — the mechanism admits a declaration, never silence.
fn unmapped_labels(manifest: &Manifest) -> BTreeMap<String, (String, String)> {
    let unmapped: BTreeSet<&str> = manifest
        .cases
        .iter()
        .filter(|case| case.mapping_status != "mapped")
        .map(|case| case.id.as_str())
        .collect();

    // Card T00's archive, one row per case or refuse. A duplicate row does not
    // disturb the sum, so the sum assertion below cannot catch it — the natural
    // map-building lets the last row silently decide the label.
    let triage_path = repo_root().join(EVIDENCE_FILES[0]);
    let triage: serde_json::Value = read_json(&triage_path);
    let mut archive: BTreeMap<String, String> = BTreeMap::new();
    for case in triage["unmapped_triage"]["cases"]
        .as_array()
        .expect("T00's triage lists its cases")
    {
        let id = case["id"].as_str().expect("a triage row names its case");
        let bucket = case["current_bucket"]
            .as_str()
            .expect("a triage row names its bucket");
        assert!(
            archive.insert(id.to_owned(), bucket.to_owned()).is_none(),
            "{id}: two triage rows classify the same case in {}",
            triage_path.display()
        );
    }

    let records: Vec<(&str, serde_json::Value)> = EVIDENCE_FILES
        .iter()
        .map(|relative| (*relative, read_json(&repo_root().join(relative))))
        .collect();
    let retirements = lane_row_retirements(&records, &unmapped);

    // The lanes' own measured labels, newest record winning.
    let mut lanes: BTreeMap<String, (String, String)> = BTreeMap::new();
    for (relative, record) in &records {
        let Some(rows) = record.get("unmapped_reasons") else {
            continue;
        };
        for (case_id, row) in rows.as_object().expect("unmapped_reasons is an object") {
            if !unmapped.contains(case_id.as_str()) {
                // The case is mapped now. A row for it is admissible only where
                // a later record declares the retirement; the retirement's own
                // conditions were checked above.
                assert!(
                    retirements.contains_key(case_id.as_str()),
                    "{case_id}: {relative} records an unmapped-case bucket, but the manifest \
                     maps this case"
                );
                continue;
            }
            for field in ["bucket", "reason", "measured_by"] {
                let value = row
                    .get(field)
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("");
                assert!(
                    !value.trim().is_empty(),
                    "{case_id}: {relative} carries no {field} for this case"
                );
            }
            let bucket = row["bucket"].as_str().expect("bucket").to_owned();
            let superseded = row.get("supersedes").and_then(serde_json::Value::as_str);
            match lanes.get(case_id) {
                None => assert!(
                    superseded.is_none(),
                    "{case_id}: {relative} declares its bucket supersedes {}, but no earlier \
                     record labels this case",
                    superseded.unwrap_or_default()
                ),
                // Re-verification reaching the same label is the record saying
                // the label still holds, not a second claim on the case.
                Some((held, _)) if *held == bucket => {}
                Some((held, held_source)) => {
                    let superseded = superseded.unwrap_or_else(|| {
                        panic!(
                            "{case_id}: two evidence records label this case differently \
                             ({held_source} says {held:?}, {relative} says {bucket:?}) and \
                             neither declares a supersession"
                        )
                    });
                    assert_eq!(
                        superseded, held_source,
                        "{case_id}: {relative} declares its bucket supersedes {superseded}, but \
                         the record holding the label is {held_source}"
                    );
                }
            }
            lanes.insert(case_id.clone(), (bucket, (*relative).to_owned()));
        }
    }

    let mut resolved: BTreeMap<String, (String, String)> = BTreeMap::new();
    for case_id in &unmapped {
        let entry = lanes.get(*case_id).cloned().or_else(|| {
            archive
                .get(*case_id)
                .map(|bucket| (bucket.clone(), ARCHIVE_SOURCE.to_owned()))
        });
        let entry = entry.unwrap_or_else(|| {
            panic!(
                "{case_id}: unmapped with no measured bucket and no archived triage row in {}",
                triage_path.display()
            )
        });
        resolved.insert((*case_id).to_owned(), entry);
    }
    resolved
}

fn unmapped_label_sources(manifest: &Manifest) -> BTreeMap<String, String> {
    unmapped_labels(manifest)
        .into_iter()
        .map(|(case_id, (_, source))| (case_id, source))
        .collect()
}

fn read_json(path: &std::path::Path) -> serde_json::Value {
    serde_json::from_str(
        &fs::read_to_string(path)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display())),
    )
    .unwrap_or_else(|error| panic!("cannot parse {}: {error}", path.display()))
}

/// The triage tally over the cases the manifest *currently* leaves unmapped
/// (card HYGIENE-14, finding `D02-BUCKETS-STALE`).
///
/// The hand-kept mirror in `acceptance.json` had drifted to a sum of 22 against
/// `ct_unmapped: 18`: a lane that maps a case edits the row its own case left,
/// and the four cases wave 8 mapped were never decremented out of another
/// lane's row. Deriving the *label* is not possible — the manifest carries no
/// blocker field for an unmapped case, and card T00's archived triage is the
/// only per-case record there is — but deriving the *set of counted cases* is,
/// and that is what ends the rot: a case that gets mapped leaves this tally in
/// the same commit that maps it.
///
/// Total-or-reject, matching `scripts/ct_stage_summary.py` exactly: the labels
/// come from [`unmapped_labels`], which refuses a case it cannot classify, and
/// the counts must sum to the number of unmapped cases — the property whose
/// silent violation *is* the finding.
///
/// Card CT-LABELS made the label derived as well, so this counts resolved
/// labels rather than archived ones; `unmapped_label_sources` records which
/// cases are still on the archive.
fn unmapped_bucket_counts(manifest: &Manifest) -> BTreeMap<String, usize> {
    let labels = unmapped_labels(manifest);
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for (bucket, _) in labels.values() {
        *counts.entry(bucket.clone()).or_insert(0) += 1;
    }
    assert_eq!(
        counts.values().sum::<usize>(),
        labels.len(),
        "the bucket tally must account for every unmapped case — a sum that \
         drifts from ct_unmapped is exactly finding D02-BUCKETS-STALE"
    );
    counts
}

/// The consumable acceptance numbers are generated from the manifest, not
/// hand-kept beside it (T00.9 / T00 finding O7). `acceptance.json` names the
/// fields the generator owns; this test recomputes each of them and refuses any
/// drift, so the record cannot claim depth its stage evidence does not carry.
#[test]
fn ct_acceptance_summary_is_derived_from_the_manifest() {
    let manifest = load_manifest();
    let derived = ct_stage_summary(&manifest);
    let path = repo_root().join("docs/reviews/r0_3/acceptance.json");
    let acceptance: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display())),
    )
    .unwrap_or_else(|error| panic!("cannot parse {}: {error}", path.display()));
    let evidence = acceptance
        .get("ct_evidence")
        .expect("acceptance.json carries the stage-derived CT block");
    let generated: Vec<&str> = evidence
        .get("generated_fields")
        .expect("the CT block names the fields its generator owns")
        .as_array()
        .expect("generated_fields is a list")
        .iter()
        .map(|name| name.as_str().expect("field name"))
        .collect();
    assert_eq!(
        generated,
        [
            "ct_mapped",
            "ct_unmapped",
            "stage_histogram",
            "observation_counts",
            "host_scope",
            "tool_scope",
            "depth_profile",
            "case_depth",
            // Card HYGIENE-14: the triage tally is generated too, over the case
            // set this manifest fixes.
            "unmapped_bucket_counts",
            // Card CT-LABELS: and so is each label's source, which is what makes
            // the labels themselves derivable rather than frozen.
            "unmapped_label_sources",
        ],
        "the generated field set is itself pinned, so a field cannot quietly \
         become hand-maintained"
    );
    assert_eq!(
        acceptance.get("ct_mapped"),
        Some(&derived["mapped"]),
        "ct_mapped is generated from the manifest"
    );
    assert_eq!(
        acceptance.get("ct_unmapped"),
        Some(&derived["unmapped"]),
        "ct_unmapped is generated from the manifest"
    );
    for field in [
        "stage_histogram",
        "observation_counts",
        "host_scope",
        "tool_scope",
        "depth_profile",
        "case_depth",
        "unmapped_bucket_counts",
        "unmapped_label_sources",
    ] {
        assert_eq!(
            evidence.get(field),
            Some(&derived[field]),
            "acceptance.json `{field}` is not what the manifest's stage evidence derives"
        );
    }
    // The two derivations must read the same evidence files, and neither can
    // check the other's source directly. `derived_from` is the third record both
    // assert against (card CT-LABELS).
    let expected_sources: Vec<String> = std::iter::once("docs/reevaluation/ct_manifest.json")
        .chain(EVIDENCE_FILES)
        .map(str::to_owned)
        .collect();
    let recorded: Vec<String> = evidence
        .get("derived_from")
        .expect("the CT block names the records it derives from")
        .as_array()
        .expect("derived_from is a list")
        .iter()
        .map(|value| value.as_str().expect("a path").to_owned())
        .collect();
    assert_eq!(
        recorded, expected_sources,
        "acceptance.json `derived_from` must be the manifest plus this derivation's \
         evidence-file list, or the Rust and Python generators are reading different files"
    );
    // `newly_decidable_not_mapped` stays hand-kept — whether a case is decidable
    // *now* is a judgment no record derives — but "not mapped" is a manifest
    // fact, so the half of it that can go stale the way the buckets did is
    // checked here (card HYGIENE-14).
    let mapped_ids: BTreeSet<&str> = manifest
        .cases
        .iter()
        .filter(|case| case.mapping_status == "mapped")
        .map(|case| case.id.as_str())
        .collect();
    for entry in evidence
        .get("newly_decidable_not_mapped")
        .expect("the CT block lists the newly decidable cases it did not map")
        .as_array()
        .expect("newly_decidable_not_mapped is a list")
    {
        let case_id = entry.as_str().expect("case id");
        assert!(
            !mapped_ids.contains(case_id),
            "acceptance.json still calls {case_id} newly decidable and not mapped, but the \
             manifest maps it"
        );
    }
    // Card HYGIENE-16: the companion reasons map is the same claim in prose and
    // goes stale the same way, so it is checked with it. A key beginning with an
    // underscore is the map's own commentary rather than a case id.
    if let Some(reasons) = evidence.get("newly_decidable_not_mapped_reasons") {
        for case_id in reasons
            .as_object()
            .expect("newly_decidable_not_mapped_reasons is an object")
            .keys()
        {
            assert!(
                case_id.starts_with('_') || !mapped_ids.contains(case_id.as_str()),
                "acceptance.json explains why {case_id} is not mapped, but the manifest maps it"
            );
        }
    }
}

fn unsupported_target(diagnostics: &[qcore_compiler::FileDiagnostic]) {
    assert_eq!(
        diagnostics.len(),
        1,
        "unsupported target must follow successful source/core checks"
    );
    assert_eq!(diagnostics[0].code, "Q500");
    assert!(
        diagnostics[0]
            .message
            .contains("requires a Linux, macOS or Windows threads target"),
        "{diagnostics:?}"
    );
}

/// The controls a mutex rejection carries. An `accept` control compiles through
/// the Linux/macOS/Windows emission path, so on any other host it must arrive as the
/// same `Q500` refusal every other route uses — never as a silent skip.
/// Drive one growth control's own bytes through parse, elaborate,
/// `lower_source_mutex` and `check_mutex_invariant_program`, and require the
/// published certificate to record at least one admitted protection growth.
/// Returns how many it recorded.
///
/// This is what makes a growth control attribute anything: a control that
/// parsed, elaborated, lowered and checked while performing no growth would
/// distinguish nothing from the case it stands beside. It runs on every host —
/// nothing here reaches the C backend.
fn growth_control_admission(id: &str, control: &MutexControl, source: &str) -> usize {
    let parsed = qcore_frontend::frontend::parse_source_program(source)
        .unwrap_or_else(|error| panic!("{id} control {}: {error}", control.source));
    assert!(
        qcore_frontend::source_mutex::has_source_mutex(&parsed),
        "{id} control {}: route detection must select the mutex route",
        control.source
    );
    let typed = qcore_frontend::frontend::elaborate_source(&parsed).unwrap_or_else(|diagnostics| {
        panic!(
            "{id} control {} must elaborate: {:?}",
            control.source,
            diagnostics
                .iter()
                .map(|d| d.code.to_owned())
                .collect::<Vec<_>>()
        )
    });
    let lowered = qcore_frontend::source_mutex::lower_source_mutex(&parsed, &typed, "main")
        .unwrap_or_else(|diagnostics| {
            panic!(
                "{id} control {} must lower: {:?}",
                control.source,
                diagnostics
                    .iter()
                    .map(|d| d.code.to_owned())
                    .collect::<Vec<_>>()
            )
        });
    let certificate = qcore_core::mutex_invariant::check_mutex_invariant_program(&lowered.program)
        .unwrap_or_else(|diagnostics| {
            panic!(
                "{id} control {} must check, so the case's refusal is not vacuous: {:?}",
                control.source,
                diagnostics
                    .iter()
                    .map(|d| lowered.locate(d).code.to_owned())
                    .collect::<Vec<_>>()
            )
        });
    assert!(
        !certificate.admitted_protection_growths.is_empty(),
        "{id} control {} must actually record the growth it performs — a control \
         that checked without growing would attribute nothing",
        control.source
    );
    certificate.admitted_protection_growths.len()
}

/// CT46: the declared crossing, admitted through the verified mutex protocol.
///
/// The case's own module is asserted whole — the spawn/join edge, the child's
/// own lock, and the complete ordered guarded-reach vector rebuilt from the
/// checked event list — so a fixture cannot pin a convenient subset of the
/// payload accesses. The emission boundary is asserted in **both** directions:
/// this module renders no C and its event list is refused by
/// `interpret_mutex_program` with the fixture's verbatim refusal, while the
/// `accept_emitting` control — the same route without the crossing clause —
/// does render and does interpret to its pinned observation. Without that pair
/// the case's three `unassessed` stages would be an omission; with it they are
/// a measurement attributable to the crossing.
fn run_mutex_crossing(
    directory: &Path,
    id: &str,
    crossing: &CheckedMutexCrossing,
    result: Result<qcore_compiler::CompiledModule, Vec<qcore_compiler::FileDiagnostic>>,
) {
    // The whole route refuses a host it cannot emit threads for, before the
    // checker is reached, so there is nothing to measure there.
    if !cfg!(any(
        target_os = "linux",
        target_os = "macos",
        target_os = "windows"
    )) {
        unsupported_target(&result.unwrap_err());
        eprintln!("{id}: declared crossing unexecuted on this host");
        return;
    }
    let module = result.unwrap_or_else(|diagnostics| {
        panic!(
            "{id}: the declared crossing must be admitted:\n{}",
            diagnostics
                .iter()
                .map(format_diagnostic)
                .collect::<Vec<_>>()
                .join("\n")
        )
    });
    let checked = module
        .checked
        .as_checked_mutex_guards()
        .unwrap_or_else(|| panic!("{id}: the case must take the checked mutex route"));
    let events = &checked.lowered.program.events;

    // The crossing is a real task boundary in the model, not a syntactic
    // annotation: the child is spawned, it takes the generation itself, and its
    // creator joins it.
    assert!(
        events.iter().any(|event| matches!(
            event,
            MutexInvariantEvent::Spawn { parent, child }
                if *parent == crossing.spawn.parent && *child == crossing.spawn.child
        )),
        "{id}: the declared crossing must lower to a real spawn"
    );
    assert!(
        events.iter().any(|event| matches!(
            event,
            MutexInvariantEvent::Join { parent, child }
                if *parent == crossing.join.parent && *child == crossing.join.child
        )),
        "{id}: the creator must join the child it shared the payload with"
    );
    assert!(
        events.iter().any(|event| matches!(
            event,
            MutexInvariantEvent::Lock { task, .. } if *task == crossing.child_lock_task
        )),
        "{id}: the child acquires the generation itself; nothing is parked with it"
    );
    // Rebuilt whole, so the fixture pins every reach or none.
    let reaches: Vec<GuardedReach> = events
        .iter()
        .filter_map(|event| match event {
            MutexInvariantEvent::Access { task, guard, .. } => Some(GuardedReach {
                task: *task,
                guarded: guard.is_some(),
            }),
            _ => None,
        })
        .collect();
    assert_eq!(
        reaches, crossing.reaches,
        "{id}: every reach of the shared payload, in order, and whether it named \
         a guard — this vector is what `accept only through a verified mutex \
         protocol` means here"
    );

    // Card `MUTEX-SHARE-CROSS-EMISSION` (wave 36) replaced wave 35's
    // `c_source.is_none()` assertion — which carried the instruction to do so in
    // its own failure message — with the positive one, in the same change as
    // `derive_stage_plan`'s arm and this case's four manifest rows.
    let c = module.c_source.as_deref().unwrap_or_else(|| {
        panic!("{id}: the declared crossing renders C since card MUTEX-SHARE-CROSS-EMISSION")
    });
    for wanted in &crossing.emission.emitted_c_contains {
        assert!(
            c.contains(wanted.as_str()),
            "{id}: emitted C is missing `{wanted}`, so the crossing is not the \
             real thread boundary this case claims"
        );
    }
    let creator_body = c
        .split_once("int main(")
        .unwrap_or_else(|| panic!("{id}: the emitted program has an entry point"))
        .1;
    for forbidden in &crossing.emission.main_excludes {
        assert!(
            !creator_body.contains(forbidden.as_str()),
            "{id}: the creator's own body contains `{forbidden}`; the child's \
             critical section must run in the child's own function, or the \
             crossing is a rendering artefact and not a thread boundary"
        );
    }
    // Card `MUTEX-SHARE-CROSS-EMISSION`, and this assertion exists **because a
    // mutation escaped without it**. The native comparison below compares the
    // program's *logged* event sequence, so deleting a real `mtx_lock` while
    // keeping its log line, or moving a real `mtx_unlock` away from its log
    // line, both produced a byte-identical run under the sanitizers. What ties
    // a logged acquisition to a real one is that the emitter writes the
    // operation and its log adjacently, in one statement — so that adjacency is
    // what gets asserted, on the emitted text, rather than assumed.
    let mut logged_sections = 0;
    let lines: Vec<&str> = c.lines().collect();
    for (index, line) in lines.iter().enumerate() {
        if !line.contains("qcore_diagnostic_log(") {
            continue;
        }
        let operation = if line.contains("QCORE_DIAG_MUTEX_LOCK") {
            "mtx_lock(&m"
        } else if line.contains("QCORE_DIAG_MUTEX_UNLOCK") {
            "mtx_unlock(&m"
        } else {
            continue;
        };
        logged_sections += 1;
        let previous = index
            .checked_sub(1)
            .and_then(|previous| lines.get(previous))
            .copied()
            .unwrap_or_default();
        assert!(
            previous.contains(operation),
            "{id}: the logged mutex event on line {} is not immediately \
             preceded by its own `{operation}…` call, so the observation this \
             case compares would not witness the real operation",
            index + 1
        );
    }
    assert!(
        logged_sections >= 4,
        "{id}: both tasks' critical sections are in the emitted text — the \
         child's and the creator's, two acquisitions and two releases — or this \
         adjacency check is passing vacuously"
    );
    let reference = qcore_core::mutex_c_emit::interpret_mutex_program(
        &checked.lowered.program,
        &checked.certificate,
    )
    .unwrap_or_else(|refusal| {
        panic!("{id}: a fork/join crossing is inside the emitted subset: {refusal:?}")
    });
    assert_eq!(
        reference, crossing.emission.reference_observation,
        "{id}: reference observation drifted from the pinned expectation"
    );
    // The crossing is in the observation, not merely in the event list: the
    // child's acquisition sits strictly between the spawn and the join.
    let position = |needle: &str| {
        reference
            .iter()
            .position(|line| line == needle)
            .unwrap_or_else(|| panic!("{id}: `{needle}` is not in the observation"))
    };
    let (spawn, join) = (
        position(&format!(
            "spawn {} {}",
            crossing.spawn.parent, crossing.spawn.child
        )),
        position(&format!(
            "join {} {}",
            crossing.join.parent, crossing.join.child
        )),
    );
    assert!(
        spawn < join,
        "{id}: the creator joins the child it spawned, in that order"
    );
    assert!(
        reference[spawn + 1..join]
            .iter()
            .any(|line| line.starts_with("lock ")),
        "{id}: the child acquires the generation inside its own segment, which \
         is what `only through a verified mutex protocol` means once the \
         crossing really executes"
    );
    if cfg!(any(target_os = "linux", target_os = "macos")) {
        assert_eq!(
            sanitized_c_run(
                &mutex_observation::instrument(c),
                &crossing.emission.sanitizer_flags,
                &crossing.emission.asan_options,
                &crossing.emission.ubsan_options,
            ),
            reference,
            "{id}: the emitted program's standard output is compared byte for \
             byte with the reference interpreter's observation of the same \
             checked trace"
        );
    } else {
        eprintln!(
            "{id}: emitted on this host; Unix sanitizer evidence unexecuted \
             (manifest evidence_target=linux)"
        );
    }

    for control in &crossing.controls {
        let path = directory.join(&control.source);
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
        let outcome = compile_files(
            &[SourceFile::new(path.to_string_lossy(), source)],
            &CompileOptions::default(),
        );
        match control.expectation.as_str() {
            // The other side of the emission boundary: the same route, the same
            // payload and the same generation with the clause removed. It
            // renders, and it interprets to the observation the fixture pins.
            "accept_emitting" => {
                let module = outcome.unwrap_or_else(|diagnostics| {
                    panic!(
                        "{id} control {}: this control must be admitted, or the \
                         case's empty emission attributes nothing:\n{}",
                        control.source,
                        diagnostics
                            .iter()
                            .map(format_diagnostic)
                            .collect::<Vec<_>>()
                            .join("\n")
                    )
                });
                let checked = module
                    .checked
                    .as_checked_mutex_guards()
                    .unwrap_or_else(|| panic!("{id} control {}: same route", control.source));
                assert!(
                    module.c_source.is_some(),
                    "{id} control {}: the route really does emit without the \
                     crossing clause",
                    control.source
                );
                let observation = qcore_core::mutex_c_emit::interpret_mutex_program(
                    &checked.lowered.program,
                    &checked.certificate,
                )
                .unwrap_or_else(|error| {
                    panic!(
                        "{id} control {}: this control is inside the emitted \
                         subset: {error:?}",
                        control.source
                    )
                });
                assert_eq!(
                    observation, control.reference_observation,
                    "{id} control {}: reference observation drifted from the \
                     pinned expectation",
                    control.source
                );
            }
            // Card `MUTEX-SHARE-CROSS-EMISSION` (wave 36): the boundary the
            // backend still does **not** cross, measured on a shape that is
            // reachable from source so the refusal is not vacuous. A
            // guard-local child is joined inside a critical section its parent
            // holds, so the parent is not simply blocked for the whole of the
            // child's segment and the fork/join rendering does not apply. The
            // module checks; it is published without C.
            "accept_unrendered" => {
                let module = outcome.unwrap_or_else(|diagnostics| {
                    panic!(
                        "{id} control {}: this control must be admitted, or the \
                         surviving emission boundary attributes nothing:\n{}",
                        control.source,
                        diagnostics
                            .iter()
                            .map(format_diagnostic)
                            .collect::<Vec<_>>()
                            .join("\n")
                    )
                });
                let checked = module
                    .checked
                    .as_checked_mutex_guards()
                    .unwrap_or_else(|| panic!("{id} control {}: same route", control.source));
                assert!(
                    module.c_source.is_none(),
                    "{id} control {}: a trace outside both rendered shapes is \
                     published without C, never with C the backend did not produce",
                    control.source
                );
                let refusal = qcore_core::mutex_c_emit::interpret_mutex_program(
                    &checked.lowered.program,
                    &checked.certificate,
                )
                .expect_err("this control is outside the emitted subset");
                assert_eq!(
                    Some(format!("{refusal:?}")),
                    control.emission_refusal,
                    "{id} control {}: the surviving emission refusal is part of \
                     the measurement",
                    control.source
                );
            }
            // One of the ways of reaching the payload without the protocol.
            // Both the code vector and a distinguishing fragment of the message
            // are pinned: a control that reached the right code for the wrong
            // reason would pass on the code alone.
            "reject" => {
                let diagnostics = outcome.err().unwrap_or_else(|| {
                    panic!(
                        "{id} control {}: this program must be refused",
                        control.source
                    )
                });
                assert_eq!(
                    diagnostics
                        .iter()
                        .map(|diagnostic| diagnostic.code.clone())
                        .collect::<Vec<_>>(),
                    control.diagnostic_sequence,
                    "{id} control {}: exact ordered code vector",
                    control.source
                );
                let fragment = control
                    .message_fragment
                    .as_deref()
                    .expect("a refused control pins a message fragment");
                assert!(
                    diagnostics
                        .iter()
                        .any(|diagnostic| diagnostic.message.contains(fragment)),
                    "{id} control {}: no diagnostic carries `{fragment}`; the \
                     reported messages were {:?}",
                    control.source,
                    diagnostics
                        .iter()
                        .map(|diagnostic| diagnostic.message.clone())
                        .collect::<Vec<_>>()
                );
            }
            other => panic!("{id}: unknown crossing control `{other}`"),
        }
    }
}

fn run_mutex_controls(directory: &Path, id: &str, reject: &CheckedMutexReject) {
    for control in &reject.controls {
        let path = directory.join(&control.source);
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
        let result = compile_files(
            &[SourceFile::new(path.to_string_lossy(), source.clone())],
            &CompileOptions::default(),
        );
        match control.expectation.as_str() {
            "accept" => {
                if !cfg!(any(
                    target_os = "linux",
                    target_os = "macos",
                    target_os = "windows"
                )) {
                    unsupported_target(&result.unwrap_err());
                    eprintln!(
                        "{id} control {}: source/core checked; threaded emission unexecuted on this host",
                        control.source
                    );
                    continue;
                }
                let module = result.unwrap_or_else(|diagnostics| {
                    panic!(
                        "{id} control {} must check, so the case's refusal is not vacuous:\n{}",
                        control.source,
                        diagnostics
                            .iter()
                            .map(format_diagnostic)
                            .collect::<Vec<_>>()
                            .join("\n")
                    )
                });
                assert!(
                    module.checked.as_checked_mutex_guards().is_some(),
                    "{id} control {} must take the checked mutex route",
                    control.source
                );
            }
            // Card `CT34-SANITIZED`. A protection growth is admitted by
            // `CheckedMutexInvariantV1` and rendered by the C backend as what it
            // is at C level: no `mtx_t` operation at all, plus the bookkeeping
            // line that keeps the two observations comparable. This branch
            // asserts the whole chain and never a convenient part of it: the
            // core admits the control's own bytes and really records a growth
            // (a control that checked without growing would attribute nothing),
            // `compile_files` returns a compiled module on the checked mutex
            // route carrying emitted C, and — where the control carries a
            // sanitized run — that C is compiled under ASan/UBSan (plus LSan
            // where the platform has it) and its stdout equals the reference
            // observation line for line. The reference is computed by
            // `interpret_mutex_program` from the checked event list and never
            // reads the emitted C; the pinned vector in the fixture is checked
            // against it, so neither can drift alone.
            "accept_growth" => {
                let growths = growth_control_admission(id, control, &source);
                if !cfg!(any(
                    target_os = "linux",
                    target_os = "macos",
                    target_os = "windows"
                )) {
                    unsupported_target(&result.unwrap_err());
                    eprintln!(
                        "{id} control {}: source/core checked ({growths} admitted growth(s)); \
                         threaded emission unexecuted on this host",
                        control.source
                    );
                    continue;
                }
                let module = result.unwrap_or_else(|diagnostics| {
                    panic!(
                        "{id} control {} must check and render, so the case's refusal is \
                         not vacuous:\n{}",
                        control.source,
                        diagnostics
                            .iter()
                            .map(format_diagnostic)
                            .collect::<Vec<_>>()
                            .join("\n")
                    )
                });
                let checked = module.checked.as_checked_mutex_guards().unwrap_or_else(|| {
                    panic!(
                        "{id} control {} must take the checked mutex route",
                        control.source
                    )
                });
                let c = module.c_source.as_deref().unwrap_or_else(|| {
                    panic!(
                        "{id} control {}: an admitted growth renders C — a module without \
                         it would leave the growth unexecuted while claiming acceptance",
                        control.source
                    )
                });
                let Some(run) = &control.sanitized_run else {
                    continue;
                };
                for wanted in &run.emitted_c_contains {
                    assert!(
                        c.contains(wanted.as_str()),
                        "{id} control {}: emitted C is missing `{wanted}`",
                        control.source
                    );
                }
                let reference = qcore_core::mutex_c_emit::interpret_mutex_program(
                    &checked.lowered.program,
                    &checked.certificate,
                )
                .expect("an admitted growth is inside the emitted subset");
                assert_eq!(
                    reference, run.reference_observation,
                    "{id} control {}: reference observation drifted from the pinned \
                     expectation",
                    control.source
                );
                if cfg!(target_os = "windows") {
                    eprintln!(
                        "{id} control {}: emitted on Windows; Unix sanitizer evidence \
                         unexecuted on this host",
                        control.source
                    );
                    continue;
                }
                assert_eq!(
                    control_sanitized_run(&mutex_observation::instrument(c), run),
                    reference,
                    "{id} control {}: native and reference observations must agree line \
                     for line",
                    control.source
                );
            }
            // A control the checker admits and the C backend declines to render.
            // Both halves are asserted, so neither can be quietly dropped: a
            // check-only admission is never reported as a full acceptance, and
            // the backend's refusal stays a recorded decision rather than an
            // omission. No case carries one today — card CT34-SANITIZED retired
            // the boundary that produced the only one — and the expectation is
            // retained rather than deleted so a future check-only route states
            // its refusal instead of inventing a weaker spelling.
            "accept_check_only" => {
                growth_control_admission(id, control, &source);
                if !cfg!(any(
                    target_os = "linux",
                    target_os = "macos",
                    target_os = "windows"
                )) {
                    unsupported_target(&result.unwrap_err());
                    eprintln!(
                        "{id} control {}: source/core checked; the backend's check-only \
                         refusal is unexecuted on this host",
                        control.source
                    );
                    continue;
                }
                let diagnostics = match result {
                    Err(diagnostics) => diagnostics,
                    Ok(_) => panic!(
                        "{id} control {} must be refused by the C backend: this route \
                         renders no growth, and a silent acceptance would emit a mutex \
                         protecting less than the model says",
                        control.source
                    ),
                };
                assert_eq!(
                    diagnostics
                        .iter()
                        .map(|diagnostic| diagnostic.code.clone())
                        .collect::<Vec<_>>(),
                    control.backend_refusal,
                    "{id} control {}: exact backend refusal vector mismatch; actual:\n{}",
                    control.source,
                    diagnostics
                        .iter()
                        .map(format_diagnostic)
                        .collect::<Vec<_>>()
                        .join("\n")
                );
            }
            "reject" => {
                let diagnostics = match result {
                    Err(diagnostics) => diagnostics,
                    Ok(_) => panic!("{id} control {} must be refused", control.source),
                };
                assert_eq!(
                    diagnostics
                        .iter()
                        .map(|diagnostic| diagnostic.code.clone())
                        .collect::<Vec<_>>(),
                    control.diagnostic_sequence,
                    "{id} control {}: exact diagnostic vector mismatch; actual:\n{}",
                    control.source,
                    diagnostics
                        .iter()
                        .map(format_diagnostic)
                        .collect::<Vec<_>>()
                        .join("\n")
                );
            }
            other => panic!("{id}: unknown control expectation `{other}`"),
        }
    }
}

/// The declared `SourceExecutionProfile` label a fixture must select. An
/// unknown label fails rather than being ignored, so a route cannot be renamed
/// out from under a case.
/// Shape rules shared by both epoch verdict modes: an accepted control states a
/// whole certificate and no diagnostics, a refused one states codes and no
/// certificate, and every control source joins the fixture inventory.
fn check_epoch_controls_shape(
    case_id: &str,
    controls: &[EpochControl],
    required_files: &mut BTreeSet<String>,
) {
    assert!(
        !controls.is_empty(),
        "{case_id}: an epoch case needs at least one control"
    );
    for control in controls {
        match control.expectation.as_str() {
            "accept" => {
                assert!(
                    control.diagnostic_sequence.is_empty(),
                    "{case_id}: an accepted control has no diagnostics"
                );
                assert!(
                    control.task_count.is_some() && !control.figures.is_empty(),
                    "{case_id}: an accepted epoch control states the certificate it \
                     measured, so `it also checks` is never the whole control"
                );
            }
            "reject" => {
                assert!(
                    !control.diagnostic_sequence.is_empty(),
                    "{case_id}: a refused control pins its codes"
                );
                assert!(
                    control.task_count.is_none() && control.figures.is_empty(),
                    "{case_id}: a refused control publishes no certificate"
                );
            }
            other => panic!("{case_id}: unknown control expectation `{other}`"),
        }
        assert!(required_files.insert(control.source.clone()));
    }
}

/// Every nonzero axis of every quantity on a published epoch certificate, keyed
/// `field.axis`, plus its task count. Rebuilding the whole map here — rather
/// than looking up the keys a fixture happens to name — is what stops a fixture
/// from pinning only the figures that suit it.
fn epoch_certificate_figures(
    certificate: &qcore_core::epoch_accounting::EpochBoundCertificate,
) -> BTreeMap<String, u64> {
    let mut figures = BTreeMap::new();
    for (field, quantity) in [
        ("shared_entry_union", &certificate.shared_entry_union),
        (
            "concurrent_private_peak",
            &certificate.concurrent_private_peak,
        ),
        ("runtime_overhead", &certificate.runtime_overhead),
        ("epoch_peak", &certificate.epoch_peak),
        ("completed_residency", &certificate.completed_residency),
        ("exit_residency", &certificate.exit_residency),
        ("work_total", &certificate.work_total),
        ("granted_credits", &certificate.granted_credits),
        ("allocation", &certificate.allocation),
    ] {
        for (axis, amount) in quantity.iter() {
            figures.insert(format!("{field}.{axis}"), amount);
        }
    }
    figures
}

/// CT35 and CT36: the accepted half of the `source.epoch.bound.v1` route. The
/// case's own fixture must be admitted under the flat-epoch profile with exactly
/// the recorded certificate — every nonzero axis, the task count and the premise
/// set — and each control must land on its own side of the boundary the case is
/// about. The route is check-only, so this runner asserts that the published
/// module carries no emitted C and no reference observation rather than looking
/// for either.
fn run_epoch_bound_figures(
    directory: &Path,
    case_id: &str,
    figures: &EpochBoundFigures,
    result: Result<qcore_compiler::CompiledModule, Vec<qcore_compiler::FileDiagnostic>>,
) {
    let module = result.unwrap_or_else(|diagnostics| {
        panic!(
            "{case_id}: expected an admitted epoch but the checker refused:\n{}",
            diagnostics
                .iter()
                .map(format_diagnostic)
                .collect::<Vec<_>>()
                .join("\n")
        )
    });
    let certificate = epoch_certificate_of(case_id, &module, &figures.execution_profile);
    assert_eq!(
        certificate.task_count, figures.task_count,
        "{case_id}: task count"
    );
    assert_eq!(
        certificate
            .assumption_names()
            .into_iter()
            .map(str::to_owned)
            .collect::<Vec<_>>(),
        figures.assumptions,
        "{case_id}: the certificate's premise set is part of the measurement — a \
         dropped premise is a changed claim"
    );
    assert_eq!(
        epoch_certificate_figures(certificate),
        figures.figures,
        "{case_id}: published certificate figures"
    );
    run_epoch_controls(directory, case_id, &figures.controls);
}

/// The certificate of an accepted epoch module, with the route and the
/// check-only shape asserted first.
fn epoch_certificate_of<'module>(
    case_id: &str,
    module: &'module qcore_compiler::CompiledModule,
    profile: &str,
) -> &'module qcore_core::epoch_accounting::EpochBoundCertificate {
    assert_eq!(
        module.execution_profile,
        execution_profile(profile),
        "{case_id}: the module must take the route it names"
    );
    assert!(
        module.c_source.is_none() && module.reference_observation.is_none(),
        "{case_id}: the flat-epoch route is check-only — it emits no C and runs no \
         reference execution, which is why its manifest row has no such stage"
    );
    let certificate = &module
        .checked
        .as_checked_epoch_bound()
        .unwrap_or_else(|| panic!("{case_id}: the checked artifact variant is explicit"))
        .certificate;
    // Every certificate this route publishes — a case's or a control's — carries
    // all four premises. They are declared assumptions, not facts the route
    // establishes, so a published certificate that had quietly shed one would be
    // a stronger claim than anything measured here.
    assert_eq!(
        certificate
            .assumption_names()
            .into_iter()
            .collect::<Vec<_>>(),
        [
            "DeclaredRuntimeOverhead",
            "DeclaredTaskFootprints",
            "DisjointSharedStorageCells",
            "NoScheduleModel",
        ],
        "{case_id}: the flat-epoch premise set is fixed"
    );
    certificate
}

/// The controls an epoch case carries, on either verdict mode. An accepted
/// control is held to the same standard as its case: the whole certificate, not
/// merely acceptance.
fn run_epoch_controls(directory: &Path, case_id: &str, controls: &[EpochControl]) {
    for control in controls {
        let path = directory.join(&control.source);
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
        let result = compile_files(
            &[SourceFile::new(path.to_string_lossy(), source)],
            &CompileOptions::default(),
        );
        match control.expectation.as_str() {
            "accept" => {
                let module = result.unwrap_or_else(|diagnostics| {
                    panic!(
                        "{case_id} control {} must check, so the case's measurement is \
                         not vacuous:\n{}",
                        control.source,
                        diagnostics
                            .iter()
                            .map(format_diagnostic)
                            .collect::<Vec<_>>()
                            .join("\n")
                    )
                });
                let certificate = epoch_certificate_of(case_id, &module, "FlatEpochBoundV1");
                assert_eq!(
                    Some(certificate.task_count),
                    control.task_count,
                    "{case_id} control {}: task count",
                    control.source
                );
                assert_eq!(
                    epoch_certificate_figures(certificate),
                    control.figures,
                    "{case_id} control {}: published certificate figures",
                    control.source
                );
            }
            "reject" => {
                let diagnostics = match result {
                    Err(diagnostics) => diagnostics,
                    Ok(_) => panic!("{case_id} control {} must be refused", control.source),
                };
                assert_eq!(
                    diagnostics
                        .iter()
                        .map(|diagnostic| diagnostic.code.clone())
                        .collect::<Vec<_>>(),
                    control.diagnostic_sequence,
                    "{case_id} control {}: exact diagnostic vector mismatch; actual:\n{}",
                    control.source,
                    diagnostics
                        .iter()
                        .map(format_diagnostic)
                        .collect::<Vec<_>>()
                        .join("\n")
                );
            }
            other => panic!("{case_id}: unknown control expectation `{other}`"),
        }
    }
}

// ---------------------------------------------------------------------------
// The `legacy_c` route (card CT-CTWIN)
// ---------------------------------------------------------------------------

/// Shape rules for a C twin. The case and every control state a complete
/// outcome — verdict, reason, per-function statuses and the whole diagnostic
/// inventory — in the same vocabulary, and at least one control must *check*,
/// so a recorded `Unknown` is never merely "the projection cannot express this
/// at all".
fn check_legacy_twin_shape(
    case_id: &str,
    twin: &LegacyCTwin,
    required_files: &mut BTreeSet<String>,
) {
    assert!(
        !twin.attributed_to.trim().is_empty(),
        "{case_id}: a C twin names the construct its scenario is about, so the controls \
         have something to attribute the verdict to"
    );
    check_legacy_contracts(case_id, &twin.contracts, required_files);
    check_legacy_outcome_shape(
        case_id,
        "case",
        &twin.verdict,
        twin.reason.as_deref(),
        &twin.diagnostics,
        &twin.function_status,
    );
    assert!(
        twin.controls
            .iter()
            .any(|control| control.expectation == "safe"),
        "{case_id}: a C twin needs a control the gateway checks, or its verdict says only \
         that the projection cannot express this at all"
    );
    for control in &twin.controls {
        check_legacy_contracts(case_id, &control.contracts, required_files);
        check_legacy_outcome_shape(
            case_id,
            control.source.as_deref().unwrap_or("<case bytes>"),
            &control.expectation,
            control.reason.as_deref(),
            &control.diagnostics,
            &control.function_status,
        );
        if let Some(source) = &control.source {
            assert!(
                source.ends_with(".c"),
                "{case_id}: a C-twin source control is C"
            );
            assert!(
                required_files.insert(source.clone()),
                "{case_id}: control source `{source}` is named twice"
            );
        }
    }
}

fn check_legacy_contracts(
    case_id: &str,
    contracts: &[String],
    required_files: &mut BTreeSet<String>,
) {
    assert!(
        !contracts.is_empty(),
        "{case_id}: a C twin is analyzed against declared contracts, and names them"
    );
    for contract in contracts {
        assert!(
            contract.ends_with(".qv") || contract.ends_with(".qcore"),
            "{case_id}: `{contract}` is not a contract source"
        );
        // Contract sources are deliberately *not* asserted unique: the whole
        // point of a contract-varying control is that it reuses the case's own
        // set and adds to it.
        required_files.insert(contract.clone());
    }
}

/// A stated outcome must be internally coherent before it is compared with a
/// measurement, so a fixture cannot record, say, a `safe` verdict beside a
/// function the analysis could not decide.
fn check_legacy_outcome_shape(
    case_id: &str,
    label: &str,
    verdict: &str,
    reason: Option<&str>,
    diagnostics: &[String],
    function_status: &BTreeMap<String, String>,
) {
    assert!(
        !function_status.is_empty(),
        "{case_id} ({label}): the per-function status map is the measurement, not a sample"
    );
    for status in function_status.values() {
        assert!(
            ["safe", "unsafe", "unknown"].contains(&status.as_str()),
            "{case_id} ({label}): unknown function status `{status}`"
        );
    }
    match verdict {
        "safe" => {
            assert!(
                reason.is_none() && diagnostics.is_empty(),
                "{case_id} ({label}): a completed analysis carries no reason and no diagnostic"
            );
            assert!(
                function_status.values().all(|status| status == "safe"),
                "{case_id} ({label}): a `safe` unit has no undecided function"
            );
        }
        "unknown" => {
            assert!(
                reason.is_some_and(|reason| !reason.trim().is_empty()),
                "{case_id} ({label}): an `unknown` unit states the reason it was withdrawn"
            );
            assert!(
                !diagnostics.is_empty(),
                "{case_id} ({label}): an `unknown` unit states what it could not project"
            );
            assert!(
                function_status.values().any(|status| status == "unknown"),
                "{case_id} ({label}): an `unknown` unit has an undecided function"
            );
        }
        "unsafe" => {
            assert!(
                reason.is_none() && !diagnostics.is_empty(),
                "{case_id} ({label}): a provisional `unsafe` states its diagnostics and \
                 carries no withdrawal reason"
            );
            assert!(
                function_status.values().any(|status| status == "unsafe"),
                "{case_id} ({label}): a provisional `unsafe` names the function it accuses"
            );
        }
        other => panic!("{case_id} ({label}): unknown C-twin verdict `{other}`"),
    }
}

/// CT43 and CT45: the C twin runs through the real unrestricted-C gateway.
///
/// `parse` is `validate_unrestricted_c11` — Clang preprocessing plus syntax
/// acceptance of the case's own `.c` bytes — and `legacy_c_twin` is
/// `analyze_unrestricted_c11`'s projection and unit verdict. Both are gated on
/// Clang; with no usable Clang the stages do not run at all, and the harness
/// says so rather than passing quietly on an unmeasured claim (`QCORE_CLANG`,
/// when set, is an explicit configuration whose failure is an error).
fn run_legacy_c_twin(directory: &Path, case_id: &str, expected: &ExpectedCase, twin: &LegacyCTwin) {
    let entry = expected.entry.as_deref().unwrap_or_else(|| {
        panic!("{case_id}: a C twin names the function the gateway analyzes as its entry")
    });
    if !legacy_clang_available(case_id) {
        return;
    }
    let case_path = directory.join("case.c");
    let case_source = fs::read_to_string(&case_path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", case_path.display()));
    // `parse`: the twin must be admissible C11 in its own right. A twin that
    // only "fails to analyze" because Clang refused it would measure nothing.
    let preprocessed = validate_unrestricted_c11(&C11TranslationUnit {
        source: &case_source,
        ..Default::default()
    })
    .unwrap_or_else(|error| panic!("{case_id}: the C twin must be admissible C11: {error:?}"));
    assert!(
        preprocessed.contains(entry),
        "{case_id}: the admitted translation unit must define the analyzed entry `{entry}`"
    );
    // `legacy_c_twin`: the case itself, then each control.
    check_legacy_analysis(
        case_id,
        "case",
        &case_source,
        entry,
        &load_legacy_contracts(directory, case_id, &twin.contracts),
        &twin.verdict,
        twin.reason.as_deref(),
        &twin.function_status,
        &twin.diagnostics,
    );
    for control in &twin.controls {
        let source = match &control.source {
            Some(name) => {
                let path = directory.join(name);
                fs::read_to_string(&path)
                    .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()))
            }
            // A contract-varying control re-analyzes the case's own bytes, which
            // is the only way to show that the *declaration* is what moved the
            // verdict.
            None => case_source.clone(),
        };
        check_legacy_analysis(
            case_id,
            control.source.as_deref().unwrap_or("<case bytes>"),
            &source,
            entry,
            &load_legacy_contracts(directory, case_id, &control.contracts),
            &control.expectation,
            control.reason.as_deref(),
            &control.function_status,
            &control.diagnostics,
        );
    }
}

fn legacy_clang_available(case_id: &str) -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "{case_id}: explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!(
                "warning: {case_id}: the C-twin stages did not run — no usable Clang: {reason}"
            );
            false
        }
    }
}

fn load_legacy_contracts(directory: &Path, case_id: &str, names: &[String]) -> ResourceContracts {
    let mut sources = BTreeMap::new();
    for name in names {
        let path = directory.join(name);
        let text = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
        sources.insert(name.clone(), text);
    }
    qcore_compiler::contract_loader::load_resource_contract_sources(&sources).unwrap_or_else(
        |error| panic!("{case_id}: the fixture's declared contracts must load: {error:?}"),
    )
}

/// One gateway analysis, compared wholesale against what the fixture recorded.
#[allow(clippy::too_many_arguments)]
fn check_legacy_analysis(
    case_id: &str,
    label: &str,
    source: &str,
    entry: &str,
    contracts: &ResourceContracts,
    verdict: &str,
    reason: Option<&str>,
    function_status: &BTreeMap<String, String>,
    diagnostics: &[String],
) {
    let result = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source,
            ..Default::default()
        },
        entry,
        contracts,
    )
    .unwrap_or_else(|error| {
        panic!("{case_id} ({label}): the gateway refused the input: {error:?}")
    });
    let (observed_verdict, observed_reason, summary) = legacy_outcome(&result);
    assert_eq!(
        observed_verdict, verdict,
        "{case_id} ({label}): gateway verdict"
    );
    assert_eq!(
        observed_reason.as_deref(),
        reason,
        "{case_id} ({label}): the withdrawal reason is part of the measurement"
    );
    let summary = summary.unwrap_or_else(|| {
        panic!("{case_id} ({label}): the analysis published no per-function summary")
    });
    // The published summary must be the one this entry was asked for. This is a
    // weak check by construction and is recorded as such: mutation M20 (analyze
    // `on_complete` instead of `subject`) is *admitted*, because the projection
    // covers every main-source definition, so both entries yield the same unit
    // verdict, the same statuses and the same diagnostics. The declared entry
    // therefore selects the summary's label, not what these two cases measure —
    // recorded as CT-CTWIN-ENTRY rather than papered over.
    assert_eq!(
        summary.entry, entry,
        "{case_id} ({label}): the analysis must be the one this case's declared entry asked for"
    );
    // Both maps are rebuilt whole from the summary rather than looked up by the
    // keys the fixture happens to name, so a fixture cannot pin a convenient
    // subset of the functions or of their diagnostics.
    assert_eq!(
        summary
            .functions
            .iter()
            .map(|(name, function)| (
                name.clone(),
                legacy_status_label(&function.status).to_owned()
            ))
            .collect::<BTreeMap<String, String>>(),
        *function_status,
        "{case_id} ({label}): per-function statuses"
    );
    assert_eq!(
        summary
            .functions
            .values()
            .flat_map(|function| &function.diagnostics)
            .map(|diagnostic| format!(
                "{}:{}:{}",
                diagnostic.function,
                diagnostic.line,
                legacy_diagnostic_label(&diagnostic.kind)
            ))
            .collect::<Vec<_>>(),
        diagnostics,
        "{case_id} ({label}): the whole ordered diagnostic inventory"
    );
}

fn legacy_outcome(
    result: &UnrestrictedLegacyResult,
) -> (&'static str, Option<String>, Option<&LegacyProgramSummary>) {
    match result {
        UnrestrictedLegacyResult::Safe(certificate) => ("safe", None, Some(&certificate.summary)),
        UnrestrictedLegacyResult::Unsafe(summary) => ("unsafe", None, Some(summary)),
        UnrestrictedLegacyResult::Unknown {
            reason, partial, ..
        } => ("unknown", Some(reason.clone()), partial.as_ref()),
    }
}

fn legacy_status_label(status: &AnalysisStatus) -> &'static str {
    match status {
        AnalysisStatus::Safe => "safe",
        AnalysisStatus::Unsafe => "unsafe",
        AnalysisStatus::Unknown => "unknown",
        // Card W45-USER-CONTRACT-PLUGINS: no CT fixture holds a call.
        AnalysisStatus::Pending => "pending",
    }
}

/// A diagnostic rendered by an explicit match rather than by `Debug`, so the
/// pinned vocabulary is this harness's and cannot drift with a formatting
/// change in the gateway.
fn legacy_diagnostic_label(kind: &LegacyDiagnosticKind) -> String {
    match kind {
        LegacyDiagnosticKind::Leak { acquisition_line } => format!("leak:{acquisition_line}"),
        LegacyDiagnosticKind::DoubleRelease { acquisition_line } => {
            format!("double-release:{acquisition_line}")
        }
        LegacyDiagnosticKind::ReleaseAfterEscape { acquisition_line } => {
            format!("release-after-escape:{acquisition_line}")
        }
        LegacyDiagnosticKind::CustodyConflict { acquisition_line } => {
            format!("custody-conflict:{acquisition_line}")
        }
        LegacyDiagnosticKind::InvalidRelease { acquisition_line } => {
            format!("invalid-release:{acquisition_line}")
        }
        LegacyDiagnosticKind::UseAfterRelease { acquisition_line } => {
            format!("use-after-release:{acquisition_line}")
        }
        LegacyDiagnosticKind::KindMismatchedRelease {
            acquisition_line, ..
        } => format!("kind-mismatched-release:{acquisition_line}"),
        LegacyDiagnosticKind::UseAfterClose {
            acquisition_line, ..
        } => format!("use-after-close:{acquisition_line}"),
        LegacyDiagnosticKind::CountedUser { description } => format!("counted-user:{description}"),
        LegacyDiagnosticKind::UnknownAlias { variable } => format!("unknown-alias:{variable}"),
        LegacyDiagnosticKind::Unsupported { description } => format!("unsupported:{description}"),
        LegacyDiagnosticKind::SummaryConflict { .. } => "summary-conflict".to_owned(),
    }
}

fn execution_profile(label: &str) -> qcore_compiler::SourceExecutionProfile {
    use qcore_compiler::SourceExecutionProfile as Profile;
    match label {
        "FlatEpochBoundV1" => Profile::FlatEpochBoundV1,
        "SequentialCleanupV1" => Profile::SequentialCleanupV1,
        "ScopedCopyTasksV1" => Profile::ScopedCopyTasksV1,
        "ScopedOwnedTasksV1" => Profile::ScopedOwnedTasksV1,
        "ScopedReadLoansV1" => Profile::ScopedReadLoansV1,
        "ScopedReadSharesV1" => Profile::ScopedReadSharesV1,
        "ScopedErrorPayloadV1" => Profile::ScopedErrorPayloadV1,
        // Card `CT-WIRE-LANE-4` (wave 32): CT47's third accepted control
        // supplies `lend PLACE LOW..HIGH`, the last of the three contracts its
        // refusal message enumerates, and that control is published under this
        // profile (card `WRITE-LEND-TASKS`).
        "ScopedWriteLeasesV1" => Profile::ScopedWriteLeasesV1,
        "CheckedMutexGuardsV1" => Profile::CheckedMutexGuardsV1,
        "StandardBufferV1" => Profile::StandardBufferV1,
        "StandardStringV1" => Profile::StandardStringV1,
        // Card `CT42-CELL-ROUTE` (wave 39): CT42's accepted controls are
        // `source.socket.v1` programs asking an admitted `disjoint` question.
        "StandardSocketV1" => Profile::StandardSocketV1,
        "BitfieldConcurrencyV1" => Profile::BitfieldConcurrencyV1,
        "ComposedStdIoTasksV1" => Profile::ComposedStdIoTasksV1,
        "ComposedEpochTasksV1" => Profile::ComposedEpochTasksV1,
        other => panic!("unknown execution profile `{other}`"),
    }
}

/// The controls a task-route rejection carries. An `accept` control compiles
/// through the Linux/macOS/Windows emission path, so on any other host it must arrive as
/// the same `Q500` refusal every other route uses — never as a silent skip.
fn run_source_controls(directory: &Path, id: &str, reject: &SourceReject) {
    for control in &reject.controls {
        let path = directory.join(&control.source);
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
        let result = compile_files(
            &[SourceFile::new(path.to_string_lossy(), source)],
            &CompileOptions::default(),
        );
        match control.expectation.as_str() {
            "accept" => {
                if !cfg!(any(
                    target_os = "linux",
                    target_os = "macos",
                    target_os = "windows"
                )) {
                    unsupported_target(&result.unwrap_err());
                    eprintln!(
                        "{id} control {}: source/core checked; threaded emission unexecuted on this host",
                        control.source
                    );
                    continue;
                }
                let module = result.unwrap_or_else(|diagnostics| {
                    panic!(
                        "{id} control {} must check, so the case's refusal is not vacuous:\n{}",
                        control.source,
                        diagnostics
                            .iter()
                            .map(format_diagnostic)
                            .collect::<Vec<_>>()
                            .join("\n")
                    )
                });
                assert_eq!(
                    module.execution_profile,
                    execution_profile(control.execution_profile.as_deref().unwrap()),
                    "{id} control {} must take the route it names",
                    control.source
                );
            }
            "reject" => {
                let diagnostics = match result {
                    Err(diagnostics) => diagnostics,
                    Ok(_) => panic!("{id} control {} must be refused", control.source),
                };
                assert_eq!(
                    diagnostics
                        .iter()
                        .map(|diagnostic| diagnostic.code.clone())
                        .collect::<Vec<_>>(),
                    control.diagnostic_sequence,
                    "{id} control {}: exact diagnostic vector mismatch; actual:\n{}",
                    control.source,
                    diagnostics
                        .iter()
                        .map(format_diagnostic)
                        .collect::<Vec<_>>()
                        .join("\n")
                );
            }
            other => panic!("{id}: unknown control expectation `{other}`"),
        }
    }
}

/// Controls for CT10's property-only route. Admission means only that the
/// successful write window's field footprints are independent; it must remain
/// check-only and publish neither C nor a reference execution.
fn run_bitfield_controls(directory: &Path, id: &str, reject: &SourceReject) {
    for control in &reject.controls {
        let path = directory.join(&control.source);
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
        let module = compile_files(
            &[SourceFile::new(path.to_string_lossy(), source)],
            &CompileOptions::default(),
        )
        .unwrap_or_else(|diagnostics| {
            panic!(
                "{id} control {} must admit the footprint property:\n{}",
                control.source,
                diagnostics
                    .iter()
                    .map(format_diagnostic)
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        });
        assert_eq!(
            module.execution_profile,
            execution_profile(control.execution_profile.as_deref().unwrap())
        );
        assert!(module.c_source.is_none(), "{id}: property route emits no C");
        assert!(
            module.reference_observation.is_none(),
            "{id}: property route executes no task schedule"
        );
    }
}

/// The controls a `sequential`-route rejection carries (card `CT-WIRE-LANE`).
///
/// Deliberately **not** `run_source_controls` with a different case set: that
/// function treats an accepted control as host-gated, because the task backend
/// refuses a host outside Linux/macOS/Windows at `Q500` before reaching the
/// backend at all. The sequential string/buffer route emits self-contained C
/// with no platform boundary of any kind, so an accepted control here is
/// admitted on every host, and copying the gate over would have recorded a host
/// scope these cases do not have — which is also why CT06 and CT11 carry no
/// `evidence_target`.
fn run_sequential_controls(directory: &Path, id: &str, reject: &SourceReject) {
    run_sequential_control_list(directory, id, &reject.controls);
}

/// The control loop the two sequential verdict modes share (card
/// `CT-WIRE-LANE-2` factored it out of `run_sequential_controls` unchanged):
/// this route emits self-contained C with no platform boundary, so an accepted
/// control is admitted on every host and must emit on every host.
fn run_sequential_control_list(directory: &Path, id: &str, controls: &[SourceControl]) {
    for control in controls {
        let path = directory.join(&control.source);
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
        let result = compile_files(
            &[SourceFile::new(path.to_string_lossy(), source)],
            &CompileOptions::default(),
        );
        match control.expectation.as_str() {
            "accept" => {
                let module = result.unwrap_or_else(|diagnostics| {
                    panic!(
                        "{id} control {} must check, so the case's refusal is not vacuous:\n{}",
                        control.source,
                        diagnostics
                            .iter()
                            .map(format_diagnostic)
                            .collect::<Vec<_>>()
                            .join("\n")
                    )
                });
                assert_eq!(
                    module.execution_profile,
                    execution_profile(control.execution_profile.as_deref().unwrap()),
                    "{id} control {} must take the route it names",
                    control.source
                );
                assert!(
                    module.c_source.is_some(),
                    "{id} control {}: this route emits on every host, so an \
                     admitted control that emits nothing is a finding",
                    control.source
                );
            }
            "reject" => {
                let diagnostics = match result {
                    Err(diagnostics) => diagnostics,
                    Ok(_) => panic!("{id} control {} must be refused", control.source),
                };
                assert_eq!(
                    diagnostics
                        .iter()
                        .map(|diagnostic| diagnostic.code.clone())
                        .collect::<Vec<_>>(),
                    control.diagnostic_sequence,
                    "{id} control {}: exact diagnostic vector mismatch; actual:\n{}",
                    control.source,
                    diagnostics
                        .iter()
                        .map(format_diagnostic)
                        .collect::<Vec<_>>()
                        .join("\n")
                );
            }
            other => panic!("{id}: unknown control expectation `{other}`"),
        }
    }
}

/// CT09: the whole sequential stage stack on the case's own fixture —
/// parse, elaborate, whole-program check, the route's own per-stream
/// observation compared exactly, C11 emission, then one binary built from that
/// C under ASan/UBSan (and LSan on Linux) run once and compared per stream
/// with the observation (card `CT-WIRE-LANE-2`).
///
/// The observation is computed from the checked model and never reads the
/// emitted C, which is what makes the comparison a comparison.
///
/// **CT07 left this runner at card `CT07-TWO-THREAD-REMAP` (wave 37)** for
/// `run_native_heap_scenarios`, its scenario's word "independently" having
/// gained a two-thread artifact. The sequential write-lend program it ran here
/// is not gone: it is retained as that case's `sequential_write_halves` accept
/// control, so the out-of-bounds-write failure mode this runner used to trap
/// for CT07 — a displaced `const` pointer whose every store is a real write at
/// `owner + low + i` — is still reached, by
/// `compiler/tests/write_lend.rs`, which has run that program as card
/// `SLICE-PERMISSIONS-WRITE-LEND`'s own fixture since wave 29.
///
/// For CT09 the run is the executed layout
/// **observation** (`AGGREGATE-CT09-LAYOUT`): that a C compiler's designated
/// initializer and member reads agree with the model is cross-checked by the
/// byte comparison, not proved, and the `proof` stage stays `unassessed`.
///
/// The Windows arm asserts the emission exists and returns before compiling;
/// it makes **no** sanitizer claim, exactly as the other sanitized runners do.
fn run_sequential_sanitized(
    directory: &Path,
    case_id: &str,
    expected: &ExpectedCase,
    result: Result<qcore_compiler::CompiledModule, Vec<qcore_compiler::FileDiagnostic>>,
) {
    let mode = expected.sequential_sanitized.as_ref().unwrap();
    let module = result.unwrap_or_else(|diagnostics| {
        panic!(
            "{case_id}: expected acceptance but the checker rejected:\n{}",
            diagnostics
                .iter()
                .map(format_diagnostic)
                .collect::<Vec<_>>()
                .join("\n")
        )
    });
    assert_eq!(
        module.execution_profile,
        execution_profile(&mode.execution_profile),
        "{case_id}: the case must take the route it names"
    );

    // The route's own observation, pinned exactly per stream. Every line is
    // one of the two streams, so nothing in the observation escapes the pin.
    let observation = module
        .extern_observation
        .as_deref()
        .unwrap_or_else(|| panic!("{case_id}: this route publishes a per-stream observation"));
    assert!(
        observation
            .iter()
            .all(|line| line.starts_with("out: ") || line.starts_with("err: ")),
        "{case_id}: an observation line outside both streams would escape the pin"
    );
    let declared_out: Vec<&str> = observation
        .iter()
        .filter_map(|line| line.strip_prefix("out: "))
        .collect();
    let declared_err: Vec<&str> = observation
        .iter()
        .filter_map(|line| line.strip_prefix("err: "))
        .collect();
    assert_eq!(
        declared_out, mode.declared_stdout,
        "{case_id}: the observation's standard output drifted from the pinned rendering"
    );
    assert_eq!(
        declared_err, mode.declared_stderr,
        "{case_id}: the observation's standard error drifted from the pinned rendering"
    );

    // The controls run on every host: this route has no platform boundary.
    run_sequential_control_list(directory, case_id, &mode.controls);

    let c = module
        .c_source
        .as_deref()
        .expect("this route emits on every host");
    if !cfg!(any(target_os = "linux", target_os = "macos")) {
        eprintln!(
            "{case_id}: emitted; Unix sanitizer evidence unexecuted on this host \
             (manifest evidence_target=linux)"
        );
        return;
    }

    use std::process::Command;
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let build = std::env::temp_dir().join(format!(
        "qcore-ct-sequential-{case_id}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&build).unwrap();
    let generated = build.join("module.c");
    fs::write(&generated, c).unwrap();
    let compiler = std::env::var("QCORE_CLANG").unwrap_or_else(|_| "cc".to_owned());
    let compiled = Command::new(&compiler)
        .args(&mode.sanitizer_flags)
        .arg(&generated)
        .arg("-o")
        .arg(build.join("module"))
        .output()
        .unwrap();
    assert!(
        compiled.status.success(),
        "{case_id}: C compiler rejected the emitted program:\n{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    // Apple's AddressSanitizer ships no leak detector, so leak checking stays
    // a Linux observation — the standing platform boundary.
    let asan_options = if cfg!(target_os = "macos") {
        "detect_leaks=0:halt_on_error=1".to_owned()
    } else {
        mode.asan_options.clone()
    };
    let run = Command::new(build.join("module"))
        .env("ASAN_OPTIONS", asan_options)
        .env("UBSAN_OPTIONS", &mode.ubsan_options)
        .output()
        .unwrap();
    assert!(
        run.status.success(),
        "{case_id}: the emitted program failed under the platform sanitizers:\n{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let split = |bytes: &[u8]| -> Vec<String> {
        String::from_utf8(bytes.to_vec())
            .unwrap()
            .lines()
            .map(ToOwned::to_owned)
            .collect()
    };
    assert_eq!(
        split(&run.stdout),
        mode.declared_stdout,
        "{case_id}: the native run's standard output is compared byte for byte"
    );
    assert_eq!(
        split(&run.stderr),
        mode.declared_stderr,
        "{case_id}: the native run's standard error is compared byte for byte"
    );
    fs::remove_dir_all(&build).unwrap();
}

/// Card `CT-COMPOSED-NATIVE` (wave 32): the composed-epoch accept runner, and
/// the discharge of the `CT-COMPOSED-NATIVE` follow-on that `CT22-WIRE` §8 and
/// `CT-WIRE-LANE-3` §8 both name. One compile-and-run of **this case's own
/// emitted C**, on the `run_sequential_sanitized` shape.
///
/// The comparison basis is the module's **own** checked task half. A composed
/// module publishes no reference observation of its own (which is why its
/// route vocabulary has no reference-execution stage), but it does carry the
/// `CheckedTaskModule` the unedited `qcore_core::task_execution` interpreter
/// takes, so the reference run here is that interpreter's — no second
/// interpreter, and nothing hand-built. The fixture pins that rendering line
/// by line, so the native bytes are compared against a *stated* transcript.
///
/// **Observed, not proved** (the `STABILITY-REEVAL` discipline). What one
/// successful run establishes: on this host, with this C compiler, under these
/// sanitizers, the emitted program's standard output was byte-identical to the
/// checked task half's reference transcript, it wrote nothing to standard
/// error, and it exited 0. What it does not establish: that the emission is
/// correct on inputs, schedules or hosts this run did not take; that any
/// credit, abandoned remainder or epoch bound is right (the region and the
/// plan erase from emission, so no byte here is evidence about them); or any
/// theorem about either side — the case's `proof` stage stays `unassessed`.
/// A single scheduling of one program is an observation of that scheduling.
///
/// Host gating is recorded exactly, and it is two different gates. The
/// emission is reached on every host this workspace builds for (the runner
/// asserts `c_source` before it looks at the platform). The compile-and-run is
/// Unix-only — `linux` and `macos`, the `deep_gate` every sanitized runner in
/// this file carries. Leak detection is narrower still: Apple's ASan ships no
/// LSan, so `detect_leaks=1` is a **Linux** observation and macOS runs the
/// same binary with leak checking off. A reader must not take the macOS run
/// as evidence that the discharge's destroy reclaimed anything.
fn run_composed_native(
    case_id: &str,
    expected: &ExpectedCase,
    module: &qcore_compiler::CompiledModule,
) {
    use qcore_core::scoped_tasks::TaskOutcome;
    use qcore_core::task_execution::{
        TaskExecutionScript, execute_task_module, render_task_observation,
    };

    let native = expected
        .composed_native
        .as_ref()
        .unwrap_or_else(|| panic!("{case_id}: a table-listed native case pins its own evidence"));
    let composed = module
        .checked
        .as_composed_epoch_tasks()
        .unwrap_or_else(|| panic!("{case_id}: the checked artifact variant is explicit"));

    // The reference side: the unedited interpreter over this module's own
    // checked task half, on the all-success script. Nothing is injected — the
    // scenario's evidence is the ordinary run, and the refusing controls of
    // the credit-split mode are what bound the other directions.
    let reference = execute_task_module(&composed.tasks, &TaskExecutionScript::default())
        .unwrap_or_else(|error| panic!("{case_id}: the checked task half must execute: {error:?}"));
    assert_eq!(
        reference.root_outcome,
        TaskOutcome::I32(native.root_outcome_i32),
        "{case_id}: the reference run's root outcome is the pinned one"
    );
    assert_eq!(
        reference.tasks.len(),
        native.reference_tasks,
        "{case_id}: the reference run observed the pinned number of tasks"
    );
    let rendered = render_task_observation(&reference);
    assert_eq!(
        rendered.lines().collect::<Vec<_>>(),
        native.declared_observation,
        "{case_id}: the reference transcript drifted from the pinned rendering"
    );

    // The emission is reached on every host; only the compile-and-run is gated.
    let c = module
        .c_source
        .as_deref()
        .unwrap_or_else(|| panic!("{case_id}: the composed route emits the task route's own C"));
    if !cfg!(any(target_os = "linux", target_os = "macos")) {
        eprintln!(
            "{case_id}: emitted; Unix sanitizer evidence unexecuted on this host \
             (manifest evidence_target=linux)"
        );
        return;
    }

    use std::process::Command;
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let build = std::env::temp_dir().join(format!(
        "qcore-ct-composed-{case_id}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&build).unwrap();
    let generated = build.join("module.c");
    // The typed diagnostic stream is what carries the task transcript, so the
    // emitted C is opted into the maintained stdout sink exactly as every other
    // native runner in this file does. The sink is the only addition: the
    // module's own bytes are otherwise compiled as emitted.
    fs::write(&generated, diagnostic_sink::instrument(c)).unwrap();
    let compiler = std::env::var("QCORE_CLANG").unwrap_or_else(|_| "cc".to_owned());
    let mut command = Command::new(&compiler);
    command.args(&native.sanitizer_flags);
    if cfg!(target_os = "macos") {
        // The emitted threads backend resolves `<threads.h>` against the same
        // pthread-backed compatibility surface the other native runners use.
        command.arg("-I").arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/apple-threads"
        ));
    }
    let compiled = command
        .arg(&generated)
        .arg("-o")
        .arg(build.join("module"))
        .output()
        .unwrap();
    assert!(
        compiled.status.success(),
        "{case_id}: C compiler rejected the composed module's emitted program:\n{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    // Apple's AddressSanitizer ships no leak detector, so leak checking stays a
    // Linux observation — the standing platform boundary, stated rather than
    // silently inherited.
    let asan_options = if cfg!(target_os = "macos") {
        "detect_leaks=0:halt_on_error=1".to_owned()
    } else {
        native.asan_options.clone()
    };
    let run = Command::new(build.join("module"))
        .env("ASAN_OPTIONS", asan_options)
        .env("UBSAN_OPTIONS", &native.ubsan_options)
        .output()
        .unwrap();
    assert!(
        run.status.success(),
        "{case_id}: the emitted program failed under the platform sanitizers:\n{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(
        String::from_utf8(run.stdout.clone()).expect("the transcript is UTF-8"),
        rendered,
        "{case_id}: the native run's standard output is compared byte for byte \
         with the checked task half's own reference transcript"
    );
    // Nothing else may appear: the sanitizers report on standard error, so an
    // empty stream is part of what the successful run says. On Linux that
    // includes LSan's silence, which is the only sense in which this run
    // observes that what the module allocated was released.
    assert!(
        run.stderr.is_empty(),
        "{case_id}: the emitted program writes nothing to standard error:\n{}",
        String::from_utf8_lossy(&run.stderr)
    );
    fs::remove_dir_all(&build).unwrap();
}

/// Every nonzero axis of a published credit quantity, keyed by axis. Rebuilt
/// wholesale from the certificate, exactly as `epoch_certificate_figures`
/// does, so a fixture cannot pin only the axes that suit it.
fn credit_axes(quantity: &qcore_core::resource_flow::Quantity) -> BTreeMap<String, u64> {
    quantity
        .iter()
        .map(|(axis, amount)| (axis.to_owned(), amount))
        .collect()
}

/// CT23: the composed flat-epoch/scoped-task route's credit-disposition law
/// (cards `CT23-CREDIT-ERROR-PATH` and `CT-WIRE-LANE-2`). The case's own
/// fixture declares `error(close);` — the creator's error edge abandons the
/// grant — and is refused with the exact cascade; each admitting control's
/// published split is compared wholesale against the pinned figures, its
/// conservation asserted, and the mint control pins the other refusal.
fn run_epoch_credit_split(
    directory: &Path,
    case_id: &str,
    expected: &ExpectedCase,
    result: Result<qcore_compiler::CompiledModule, Vec<qcore_compiler::FileDiagnostic>>,
) {
    let split = expected.epoch_credit_split.as_ref().unwrap();
    if split.diagnostic_sequence.is_empty() {
        // Card `CT22-WIRE`: the accepted case. The verdict alone measures
        // nothing, so every published quantity is compared wholesale against
        // the fixture's pins, and the two premises that make the abandoned
        // figure *parent-local* are re-asserted on the published objects
        // rather than trusted to the checker that already refused their
        // negations (the fixture's own `Q600`/`Q302` controls below).
        let module = result.unwrap_or_else(|diagnostics| {
            panic!(
                "{case_id}: expected acceptance but the checker refused the fixture:\n{}",
                diagnostics
                    .iter()
                    .map(format_diagnostic)
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        });
        assert_eq!(
            module.execution_profile,
            execution_profile(&split.execution_profile),
            "{case_id}: the case must take the composed route"
        );
        assert!(
            module.c_source.is_some(),
            "{case_id}: the composed route emits the task route's own C"
        );
        let composed = module
            .checked
            .as_composed_epoch_tasks()
            .unwrap_or_else(|| panic!("{case_id}: the checked artifact variant is explicit"));
        let credit = &composed.credit_recovery;
        assert_eq!(
            credit_axes(&credit.granted_credits),
            split.granted,
            "{case_id}: the grant is the fixture's"
        );
        assert_eq!(
            credit_axes(&credit.restored_credits),
            split.restored,
            "{case_id}: published restored credits"
        );
        assert_eq!(
            credit_axes(&credit.spent_credits),
            split.spent,
            "{case_id}: published spent credits"
        );
        let mut sum = credit_axes(&credit.restored_credits);
        for (axis, amount) in credit.spent_credits.iter() {
            *sum.entry(axis.to_owned()).or_insert(0) += amount;
        }
        sum.retain(|_, amount| *amount != 0);
        assert_eq!(
            sum,
            credit_axes(&credit.granted_credits),
            "{case_id}: restored + spent must equal the grant"
        );
        assert_eq!(
            credit_axes(&credit.error_arm_committed),
            credit_axes(&credit.spent_credits),
            "{case_id}: the error path commits the spent part and nothing else"
        );
        // The region evidence: the published, independently re-derived
        // abandoned credit is the case's own figure.
        let region = composed
            .region
            .as_ref()
            .unwrap_or_else(|| panic!("{case_id}: the entry's region is published"));
        assert_eq!(
            region.abandoned_credit,
            split.abandoned.expect("an accepted case pins its figure"),
            "{case_id}: the abandoned parent-local credit is the pinned figure"
        );
        // The released duty list (card `CT-WIRE-LANE-3`): the owners the
        // discharge destroyed, re-derived by the driver from the model's own
        // cleanup trace, compared wholesale against the pin — empty for the
        // duty-free shape, the adopted owner for CT25's.
        assert_eq!(
            region.released, split.released,
            "{case_id}: the discharge destroys exactly the pinned owners, in \
             destruction order"
        );
        // Premise one, on the published operations. Two accepted shapes (card
        // `CT-WIRE-LANE-3`): a case pinning no released owner keeps CT22's
        // duty-free premise — the body holds quantities and the decided edge
        // only, so the one thing the discharge can discard is the parent-local
        // remainder. A case pinning released owners is the adopting shape:
        // its published body must carry exactly that many `Acquire`
        // operations — the adoptions handed to the unedited model as the pack
        // acquisitions they are — so `cleanup destroys owner` is a fact of the
        // published object, not of the fixture's comment.
        use qcore_core::error_region::RegionOp;
        let [RegionOp::ErrorRegion { body, .. }] = region.operations.as_slice() else {
            panic!("{case_id}: exactly one region operation is published");
        };
        let acquisitions = body
            .iter()
            .filter(|op| matches!(op, RegionOp::Acquire { .. }))
            .count();
        if split.released.is_empty() {
            assert_eq!(
                acquisitions, 0,
                "{case_id}: the composed region body holds no acquisition, so \
                 the sealed pack carries no duty"
            );
        } else {
            assert_eq!(
                acquisitions,
                split.released.len(),
                "{case_id}: every owner the discharge destroys is a duty the \
                 published body sealed into the pack, and nothing else is"
            );
        }
        // The derived capture list (card `CT-WIRE-LANE-3`): pinned where the
        // scenario's words hang on a capture. CT25's `loan expiry` is real
        // only if the capture really is a loan, so the pin is compared against
        // both the checked ledger and the declared rows the route has already
        // reconciled elementwise.
        if let Some(pinned) = &split.captures {
            use qcore_frontend::source_epoch_tasks::ComposedCapture;
            let words = |captures: &[ComposedCapture]| {
                captures
                    .iter()
                    .map(|capture| {
                        match capture {
                            ComposedCapture::Copy => "copy",
                            ComposedCapture::Move => "move",
                            ComposedCapture::Read => "read",
                        }
                        .to_owned()
                    })
                    .collect::<Vec<_>>()
            };
            let ledger: Vec<Vec<String>> = composed
                .composed
                .ledger
                .iter()
                .map(|row| words(&row.captures))
                .collect();
            assert_eq!(
                &ledger, pinned,
                "{case_id}: the checked ledger's derived captures are the pinned \
                 list"
            );
            let rows: Vec<Vec<String>> = composed
                .composed
                .rows
                .iter()
                .map(|row| words(&row.captures))
                .collect();
            assert_eq!(
                &rows, pinned,
                "{case_id}: the declared rows carry the same captures the ledger \
                 derived"
            );
        }
        // Premise two, on the checked task IR: every creation of the entry is
        // joined before the region statement begins, so no child exists and
        // no child holds any grant at the opening — which, for a lent owner,
        // is also the loan's expiry: the share returned at the join the span
        // rule places before the region.
        assert!(
            !composed.composed.ledger.is_empty(),
            "{case_id}: the scenario's `after joins` needs at least one join"
        );
        for row in &composed.composed.ledger {
            assert!(
                row.span.end < region.span.start,
                "{case_id}: every task-ledger span precedes the region statement"
            );
        }
        // Card `CT-COMPOSED-NATIVE` (wave 32). Everything above is a fact about
        // *checked* objects; this is the one step that runs anything.
        if COMPOSED_NATIVE_CASES.contains(&case_id) {
            run_composed_native(case_id, expected, &module);
        }
    } else {
        assert!(
            !COMPOSED_NATIVE_CASES.contains(&case_id),
            "{case_id}: a refused composed-epoch fixture emits nothing, so no \
             runner can execute its own artifact"
        );
        let diagnostics = match result {
            Err(diagnostics) => diagnostics,
            Ok(_) => panic!("{case_id}: expected rejection but the checker accepted the fixture"),
        };
        let rendered = diagnostics
            .iter()
            .map(format_diagnostic)
            .collect::<Vec<_>>()
            .join("\n");
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.code.clone())
                .collect::<Vec<_>>(),
            split.diagnostic_sequence,
            "{case_id}: exact diagnostic cascade mismatch; actual:\n{rendered}"
        );
        for want in &expected.diagnostics {
            assert!(
                diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.code == want.code
                        && diagnostic.message.contains(&want.message_contains)),
                "{case_id}: no diagnostic with code `{}` containing `{}`; actual:\n{rendered}",
                want.code,
                want.message_contains
            );
        }
    }

    for control in &split.controls {
        let path = directory.join(&control.source);
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
        let result = compile_files(
            &[SourceFile::new(path.to_string_lossy(), source)],
            &CompileOptions::default(),
        );
        match control.expectation.as_str() {
            "accept" => {
                if !cfg!(any(
                    target_os = "linux",
                    target_os = "macos",
                    target_os = "windows"
                )) {
                    unsupported_target(&result.unwrap_err());
                    eprintln!(
                        "{case_id} control {}: source/core checked; threaded emission \
                         unexecuted on this host",
                        control.source
                    );
                    continue;
                }
                let module = result.unwrap_or_else(|diagnostics| {
                    panic!(
                        "{case_id} control {} must check, so the case's refusal is not \
                         vacuous:\n{}",
                        control.source,
                        diagnostics
                            .iter()
                            .map(format_diagnostic)
                            .collect::<Vec<_>>()
                            .join("\n")
                    )
                });
                assert_eq!(
                    module.execution_profile,
                    execution_profile(&split.execution_profile),
                    "{case_id} control {} must take the composed route",
                    control.source
                );
                assert!(
                    module.c_source.is_some(),
                    "{case_id} control {}: the composed route emits the task route's own C",
                    control.source
                );
                let composed = module.checked.as_composed_epoch_tasks().unwrap_or_else(|| {
                    panic!("{case_id}: the checked artifact variant is explicit")
                });
                let credit = &composed.credit_recovery;
                assert_eq!(
                    credit_axes(&credit.granted_credits),
                    split.granted,
                    "{case_id} control {}: the grant is the fixture's, not the control's",
                    control.source
                );
                assert_eq!(
                    credit_axes(&credit.restored_credits),
                    control.restored,
                    "{case_id} control {}: published restored credits",
                    control.source
                );
                assert_eq!(
                    credit_axes(&credit.spent_credits),
                    control.spent,
                    "{case_id} control {}: published spent credits",
                    control.source
                );
                // The law itself, on the published object rather than the
                // fixture's declared figures: the split adds back to the
                // grant, and the error path commits exactly the spent part.
                let mut sum = credit_axes(&credit.restored_credits);
                for (axis, amount) in credit.spent_credits.iter() {
                    *sum.entry(axis.to_owned()).or_insert(0) += amount;
                }
                sum.retain(|_, amount| *amount != 0);
                assert_eq!(
                    sum,
                    credit_axes(&credit.granted_credits),
                    "{case_id} control {}: restored + spent must equal the grant",
                    control.source
                );
                assert_eq!(
                    credit_axes(&credit.error_arm_committed),
                    credit_axes(&credit.spent_credits),
                    "{case_id} control {}: the error path commits the spent part and \
                     nothing else",
                    control.source
                );
                // Card `CT22-WIRE`: an accepted control that writes a region
                // pins the abandoned credit it publishes; one that writes
                // none must publish no region evidence at all, which is what
                // keeps the case's figure attributable to the region rather
                // than to the route.
                match control.abandoned {
                    Some(figure) => {
                        let region = composed.region.as_ref().unwrap_or_else(|| {
                            panic!(
                                "{case_id} control {}: the control's region is published",
                                control.source
                            )
                        });
                        assert_eq!(
                            region.abandoned_credit, figure,
                            "{case_id} control {}: published abandoned credit",
                            control.source
                        );
                        // Card `CT-WIRE-LANE-3`: the control's released duty
                        // list is compared wholesale too, so the case's
                        // destroyed owner stays attributable to the adoption
                        // — the frame-rule control's region publishes none.
                        assert_eq!(
                            region.released, control.released,
                            "{case_id} control {}: published released duties",
                            control.source
                        );
                    }
                    None => assert!(
                        composed.region.is_none(),
                        "{case_id} control {}: a control that writes no region \
                         publishes no region evidence",
                        control.source
                    ),
                }
            }
            "reject" => {
                let diagnostics = match result {
                    Err(diagnostics) => diagnostics,
                    Ok(_) => panic!("{case_id} control {} must be refused", control.source),
                };
                assert_eq!(
                    diagnostics
                        .iter()
                        .map(|diagnostic| diagnostic.code.clone())
                        .collect::<Vec<_>>(),
                    control.diagnostic_sequence,
                    "{case_id} control {}: exact diagnostic vector mismatch; actual:\n{}",
                    control.source,
                    diagnostics
                        .iter()
                        .map(format_diagnostic)
                        .collect::<Vec<_>>()
                        .join("\n")
                );
            }
            other => panic!("{case_id}: unknown control expectation `{other}`"),
        }
    }
}

/// CT49: the whole composed task/std-io stage stack on the case's own fixture,
/// with the native observation compared **up to the relation the program
/// declares** (card `CT-WIRE-LANE`; the relation is card `OBS-REORDERING`'s).
///
/// Every other native runner in this corpus pins an exact total transcript.
/// This one may not: the fixture's worker bodies write from their own threads,
/// so the byte order of the emitted program is the scheduler's, while the
/// reference execution runs each worker eagerly at its spawn. That difference
/// is precisely CT49's scenario, and the comparison is the case's expectation:
/// validate the partial order, not the total trace.
///
/// The looser comparison is bounded four ways, all pinned in the fixture:
/// the relation's own **shape** (window sizes, emitter count, which stream is
/// strict); the reference's declared rendering of both streams; `stdout`
/// compared byte for byte on every run; and two deterministic witnesses that
/// the relation is neither vacuous (an admitted stream that is not the declared
/// order) nor total (a reversal inside one emitter, refused).
fn run_reordering_partial_order(
    directory: &Path,
    expected: &ExpectedCase,
    result: Result<qcore_compiler::CompiledModule, Vec<qcore_compiler::FileDiagnostic>>,
) {
    use qcore_core::obs_reordering::Emitter;
    use qcore_core::std_io::Stream;
    let reordering = expected.reordering_partial_order.as_ref().unwrap();

    // The controls first: both are about the relation a program declares, and
    // both are decided without running anything, so they are checked even on a
    // host where the sanitized run below does not happen.
    for control in &reordering.controls {
        let path = directory.join(&control.source);
        let module = compile_files(
            &[SourceFile::new(
                path.to_string_lossy(),
                fs::read_to_string(&path).unwrap(),
            )],
            &CompileOptions::default(),
        )
        .unwrap_or_else(|diagnostics| {
            panic!(
                "{} control {}: this control's point is its relation, so it must check:\n{}",
                expected.id,
                control.source,
                diagnostics
                    .iter()
                    .map(format_diagnostic)
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        });
        let observation = reordering_reference(&module);
        match control.expectation.as_str() {
            "strict_relation" => {
                for stream in [Stream::Out, Stream::Err] {
                    let relation = observation
                        .admitted_reordering(stream)
                        .expect("a parent-only program declares a relation");
                    assert!(
                        relation.is_strict(),
                        "{} control {}: a program with no worker-body line keeps byte \
                         equality on {stream:?}",
                        expected.id,
                        control.source
                    );
                }
            }
            "refuse_relation" => {
                let refusal = observation
                    .admitted_reordering(Stream::Err)
                    .expect_err("this control's relation must be refused");
                assert_eq!(
                    refusal.code(),
                    control.refusal_code.unwrap(),
                    "{} control {}: {refusal}",
                    expected.id,
                    control.source
                );
                assert!(
                    refusal
                        .to_string()
                        .contains(control.refusal_contains.as_deref().unwrap()),
                    "{} control {}: {refusal}",
                    expected.id,
                    control.source
                );
            }
            other => panic!("{}: unknown control expectation `{other}`", expected.id),
        }
    }

    if cfg!(target_os = "windows") {
        assert!(
            result
                .as_ref()
                .expect("Windows source must check and emit")
                .c_source
                .is_some()
        );
        eprintln!(
            "{}: emitted on Windows; Unix sanitizer evidence unexecuted on this host",
            expected.id
        );
        return;
    }
    if !cfg!(any(target_os = "linux", target_os = "macos")) {
        unsupported_target(&result.unwrap_err());
        eprintln!(
            "{}: source/core checked; partial-order native evidence unexecuted on this \
             host (manifest evidence_target=linux)",
            expected.id
        );
        return;
    }

    let module = result.expect("the partial-order fixture must check on this route");
    assert_eq!(
        module.execution_profile,
        execution_profile(&reordering.execution_profile),
        "{}: the case must select the composed task/std-io route",
        expected.id
    );
    let observation = reordering_reference(&module);
    let (declared_out, declared_err) = observation.projections();
    assert_eq!(
        declared_out.lines().collect::<Vec<_>>(),
        reordering.declared_stdout,
        "{}: the reference's standard output drifted from the pinned rendering",
        expected.id
    );
    assert_eq!(
        declared_err.lines().collect::<Vec<_>>(),
        reordering.declared_stderr,
        "{}: the reference's standard error drifted from the pinned rendering",
        expected.id
    );

    // The relation's shape, pinned. A relation that quietly became one big
    // commuting window, or lost an emitter, moves a number here rather than
    // passing as "still admitted".
    let out_relation = observation.admitted_reordering(Stream::Out).unwrap();
    let err_relation = observation.admitted_reordering(Stream::Err).unwrap();
    assert_eq!(
        out_relation.is_strict(),
        reordering.stdout_relation_strict,
        "{}: the standard-output relation's strictness drifted",
        expected.id
    );
    assert_eq!(
        err_relation.is_strict(),
        reordering.stderr_relation_strict,
        "{}: the standard-error relation's strictness drifted",
        expected.id
    );
    assert_eq!(
        err_relation.window_sizes(),
        reordering.stderr_window_sizes,
        "{}: the standard-error window structure drifted",
        expected.id
    );
    assert_eq!(
        err_relation.emitters().len(),
        reordering.stderr_emitters,
        "{}: the standard-error emitter set drifted",
        expected.id
    );

    // Deterministic: the relation admits a stream that is not the declared
    // order, and refuses one that reverses an emitter's own program order.
    // Neither depends on the scheduler, so the two properties that make the
    // comparison meaningful hold whatever the runs below observe.
    let mut regrouped: Vec<&str> = Vec::new();
    let mut window = usize::MAX;
    let mut pending: Vec<(Emitter, &str)> = Vec::new();
    fn flush<'a>(pending: &mut Vec<(Emitter, &'a str)>, out: &mut Vec<&'a str>) {
        let emitters: BTreeSet<Emitter> = pending.iter().map(|(emitter, _)| *emitter).collect();
        for emitter in emitters.into_iter().rev() {
            out.extend(
                pending
                    .iter()
                    .filter(|(candidate, _)| *candidate == emitter)
                    .map(|(_, text)| *text),
            );
        }
        pending.clear();
    }
    for record in err_relation.records() {
        if record.window != window {
            flush(&mut pending, &mut regrouped);
            window = record.window;
        }
        pending.push((record.emitter, record.text.as_str()));
    }
    flush(&mut pending, &mut regrouped);
    let other: String = regrouped.iter().map(|line| format!("{line}\n")).collect();
    assert_ne!(
        other, declared_err,
        "{}: the regrouping must not be the declared order, or it witnesses nothing",
        expected.id
    );
    let witness = observation
        .compare_stream(Stream::Err, &other)
        .unwrap_or_else(|refusal| {
            panic!(
                "{}: an emitter-preserving regrouping must be admitted: {refusal}",
                expected.id
            )
        });
    assert!(
        !witness.is_declared_order(),
        "{}: the regrouping is a different order, and the witness must say so",
        expected.id
    );
    let mut reversed: Vec<&str> = declared_err.lines().collect();
    let first_window = reordering.stderr_window_sizes[0];
    reversed.swap(first_window, first_window + 1);
    let reversed: String = reversed.iter().map(|line| format!("{line}\n")).collect();
    assert!(
        observation.compare_stream(Stream::Err, &reversed).is_err(),
        "{}: program order inside one emitter is preserved by the relation",
        expected.id
    );

    // The runs. One compile, `runs` executions, every one of them admitted.
    let c = module.c_source.as_deref().expect("threaded emission");
    let built = build_reordering_binary(&expected.id, c, reordering);
    let mut distinct: BTreeSet<String> = BTreeSet::new();
    let mut not_declared = 0usize;
    for index in 0..reordering.runs {
        let (out, err) = run_reordering_binary(&built, reordering);
        assert_eq!(
            out, declared_out,
            "{}: run {index}: the parent's standard output is compared byte for byte",
            expected.id
        );
        let witness = observation
            .compare_stream(Stream::Err, &err)
            .unwrap_or_else(|refusal| {
                panic!(
                    "{}: run {index}: standard error is outside the declared relation: \
                     {refusal}\n{err}",
                    expected.id
                )
            });
        if !witness.is_declared_order() {
            not_declared += 1;
        }
        distinct.insert(err);
    }
    // Reported, never asserted. Observing one order is a legal outcome of a
    // legal scheduler, so an assertion on the count would be a flake; the runs
    // buy exercise, not a claim.
    println!(
        "{}: {} sanitized runs, {} distinct standard-error orders, {not_declared} not the \
         declared order",
        expected.id,
        reordering.runs,
        distinct.len()
    );
    fs::remove_dir_all(&built).unwrap();
}

fn reordering_reference(
    module: &qcore_compiler::CompiledModule,
) -> qcore_core::composed_tasks::ComposedTaskObservation {
    use qcore_core::prelude::concurrency::task_execution::{
        TaskExecutionScript, execute_with_source_lines,
    };
    let composed = module
        .checked
        .as_composed_tasks()
        .expect("the composed task/std-io route publishes a composed artifact");
    execute_with_source_lines(
        composed.tasks(),
        &TaskExecutionScript::default(),
        &BTreeMap::new(),
        composed.sites(),
    )
    .expect("the composed reference execution runs")
}

/// Compile the emitted C once under the sanitizers, with the maintained typed
/// stdout sink installed so the program's own lines reach the streams.
fn build_reordering_binary(
    id: &str,
    generated: &str,
    reordering: &ReorderingPartialOrder,
) -> PathBuf {
    use std::process::Command;
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let directory = std::env::temp_dir().join(format!(
        "qcore-ct-reordering-{id}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&directory).unwrap();
    let path = directory.join("module.c");
    let generated = if generated.contains("#define QCORE_DIAGNOSTIC_THRESHOLD QCORE_LOG_TRACE") {
        generated.to_owned()
    } else {
        diagnostic_sink::instrument(generated)
    };
    fs::write(&path, generated).unwrap();
    let compiler = std::env::var("QCORE_CLANG").unwrap_or_else(|_| "cc".to_owned());
    let mut command = Command::new(&compiler);
    command.args(&reordering.sanitizer_flags);
    if cfg!(target_os = "macos") {
        command.arg("-I").arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/apple-threads"
        ));
    }
    let compiled = command
        .arg(&path)
        .arg("-o")
        .arg(directory.join("module"))
        .output()
        .unwrap();
    assert!(
        compiled.status.success(),
        "{id}: C compiler rejected the emitted program:\n{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    directory
}

fn run_reordering_binary(
    directory: &Path,
    reordering: &ReorderingPartialOrder,
) -> (String, String) {
    use std::process::Command;
    // Apple's AddressSanitizer ships no leak detector, so leak checking stays a
    // Linux observation — the standing platform boundary.
    let asan_options = if cfg!(target_os = "macos") {
        "detect_leaks=0:halt_on_error=1".to_owned()
    } else {
        reordering.asan_options.clone()
    };
    let output = Command::new(directory.join("module"))
        .env("ASAN_OPTIONS", asan_options)
        .env("UBSAN_OPTIONS", &reordering.ubsan_options)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "the emitted program failed under the platform sanitizers:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    (
        String::from_utf8(output.stdout).unwrap(),
        String::from_utf8(output.stderr).unwrap(),
    )
}

/// CT03, CT07 and CT17: the whole task-route stage stack on the case's own fixture —
/// parse, elaborate, lower, check the task IR, reference-execute it, emit C11,
/// then compile that C under ASan/UBSan (and LSan on Linux) and run it once per fixed
/// schedule. The reference observation is computed by
/// `execute_task_module_with_worker_allocations` from the checked task IR and
/// never reads the emitted C. The included header records every Buffer
/// allocation and release by address, so "one object, counted once" and
/// "reclaims nothing" are identity claims rather than trace claims.
///
/// Card `CT07-TWO-THREAD-REMAP` (wave 37) adds the mode's accept controls and
/// its ordered-code-vector pin, both `#[serde(default)]` so CT03's and CT17's
/// fixtures are byte-untouched. Both exist because CT07 arrived carrying wave
/// 30's control set, which this mode could not otherwise have kept: an accept
/// control had nowhere to live, and one retained control refuses in *two*
/// layers, which a head-only diagnostic match would not have noticed losing.
fn run_native_heap_scenarios(
    directory: &Path,
    expected: &ExpectedCase,
    result: Result<qcore_compiler::CompiledModule, Vec<qcore_compiler::FileDiagnostic>>,
) {
    use qcore_core::task_execution::{
        TaskExecutionScript, execute_task_module_with_worker_allocations, render_task_observation,
    };
    let native = expected.native_heap_scenarios.as_ref().unwrap();
    for control in &native.rejection_controls {
        let path = directory.join(&control.source);
        let errors = compile_files(
            &[SourceFile::new(
                path.to_string_lossy(),
                fs::read_to_string(&path).unwrap(),
            )],
            &CompileOptions::default(),
        )
        .unwrap_err();
        assert!(
            errors
                .iter()
                .any(|error| error.code == control.diagnostic.code
                    && error.message.contains(&control.diagnostic.message_contains)),
            "{}: control {} must be refused with its pinned diagnostic; actual:\n{errors:?}",
            expected.id,
            control.source
        );
        // Card `CT07-TWO-THREAD-REMAP`: where a control states its whole ordered
        // code vector, the whole vector is checked, so a refusal that loses one
        // of its layers is a failure rather than a still-passing head match.
        if !control.diagnostic_sequence.is_empty() {
            let observed = errors
                .iter()
                .map(|error| error.code.clone())
                .collect::<Vec<_>>();
            assert_eq!(
                observed, control.diagnostic_sequence,
                "{}: control {} must refuse with its whole pinned code vector",
                expected.id, control.source
            );
        }
    }
    // Card `CT07-TWO-THREAD-REMAP`: the programs this case must keep admitting.
    for control in &native.accept_controls {
        let path = directory.join(&control.source);
        let module = compile_files(
            &[SourceFile::new(
                path.to_string_lossy(),
                fs::read_to_string(&path).unwrap(),
            )],
            &CompileOptions::default(),
        )
        .unwrap_or_else(|errors| {
            panic!(
                "{}: accept control {} must still check ({}); actual:\n{errors:?}",
                expected.id, control.source, control.note
            )
        });
        assert_eq!(
            module.execution_profile,
            execution_profile(&control.execution_profile),
            "{}: accept control {} must stay on the route it names",
            expected.id,
            control.source
        );
    }
    if cfg!(target_os = "windows") {
        assert!(
            result
                .as_ref()
                .expect("Windows source must check and emit")
                .c_source
                .is_some()
        );
        eprintln!(
            "{}: emitted on Windows; Unix sanitizer evidence unexecuted on this host",
            expected.id
        );
        return;
    }
    if !cfg!(any(
        target_os = "linux",
        target_os = "macos",
        target_os = "windows"
    )) {
        unsupported_target(&result.unwrap_err());
        eprintln!(
            "{}: source/core checked; reference/sanitized-native execution unexecuted on \
             this host (manifest evidence_target=linux)",
            expected.id
        );
        return;
    }
    let module = result.expect("the accepted task fixture must pass all-path checks");
    assert_eq!(
        module.execution_profile,
        execution_profile(&native.execution_profile),
        "{}: the case must select the route it names, not another one",
        expected.id
    );
    assert!(
        module.reference_observation.is_none(),
        "{}: do not substitute the sequential execution profile",
        expected.id
    );
    let checked = module
        .checked
        .as_scoped_tasks()
        .expect("the checked task artifact is explicit for this route");
    for scenario in &native.scenarios {
        let reference = render_task_observation(
            &execute_task_module_with_worker_allocations(
                checked,
                &TaskExecutionScript {
                    spawn_outcomes: scenario.spawn_outcomes.clone(),
                    allocation_outcomes: scenario.allocation_outcomes.clone(),
                },
                &scenario.worker_allocation_outcomes,
            )
            .unwrap_or_else(|error| {
                panic!(
                    "{}: {} reference execution: {error:?}",
                    expected.id, scenario.name
                )
            }),
        );
        let quiet = sanitized_heap_run(
            module.c_source.as_deref().unwrap(),
            directory,
            native,
            scenario,
        );
        assert_eq!(quiet.status.code(), Some(scenario.exit_code));
        assert!(
            quiet.stdout.is_empty(),
            "{}: quiet emission wrote diagnostics",
            expected.id
        );
        let output = sanitized_heap_run(
            &diagnostic_sink::instrument(module.c_source.as_deref().unwrap()),
            directory,
            native,
            scenario,
        );
        assert_eq!(
            output.status.code(),
            Some(scenario.exit_code),
            "{}: {} exited unexpectedly:\n{}",
            expected.id,
            scenario.name,
            String::from_utf8_lossy(&output.stderr)
        );
        let observed = String::from_utf8(output.stdout.clone()).unwrap();
        if scenario.exit_code == 0 {
            assert_eq!(
                observed, reference,
                "{}: {} native and reference observations must agree line for line",
                expected.id, scenario.name
            );
        }
        for wanted in &scenario.stdout_contains {
            assert!(
                observed.contains(wanted.as_str()),
                "{}: {} is missing `{wanted}` in:\n{observed}",
                expected.id,
                scenario.name
            );
        }
        for forbidden in &scenario.stdout_absent {
            assert!(
                !observed.contains(forbidden.as_str()),
                "{}: {} must not contain `{forbidden}` in:\n{observed}",
                expected.id,
                scenario.name
            );
        }
        let mut previous = None;
        for line in &scenario.stdout_order {
            let position = observed
                .lines()
                .position(|entry| entry == line)
                .unwrap_or_else(|| {
                    panic!(
                        "{}: {} is missing the ordered line `{line}` in:\n{observed}",
                        expected.id, scenario.name
                    )
                });
            if let Some(previous) = previous {
                assert!(
                    previous < position,
                    "{}: {} observed `{line}` out of its pinned order in:\n{observed}",
                    expected.id,
                    scenario.name
                );
            }
            previous = Some(position);
        }
        let stderr = String::from_utf8_lossy(&output.stderr);
        let addresses = |prefix: &str| {
            stderr
                .lines()
                .filter_map(|line| line.strip_prefix(prefix).map(ToOwned::to_owned))
                .collect::<Vec<_>>()
        };
        let allocated = addresses(&native.alloc_prefix);
        let released = addresses(&native.free_prefix);
        assert_eq!(
            allocated.len(),
            scenario.heap_allocations,
            "{}: {} allocated {allocated:?}",
            expected.id,
            scenario.name
        );
        assert_eq!(
            released.len(),
            scenario.heap_releases,
            "{}: {} released {released:?}",
            expected.id,
            scenario.name
        );
        for address in &released {
            assert!(
                allocated.contains(address),
                "{}: {} released an address it never allocated: {address}",
                expected.id,
                scenario.name
            );
            assert_eq!(
                released.iter().filter(|entry| *entry == address).count(),
                1,
                "{}: {} released {address} more than once",
                expected.id,
                scenario.name
            );
        }
    }
}

/// Compiles the emitted C with the recorded sanitizer flags, beside every
/// fixture header so a header may `#include` another, and runs it once.
/// `ASAN_OPTIONS` is computed here from the scenario's `detect_leaks` rather
/// than read from data, so LeakSanitizer cannot be quietly disabled by editing
/// an option string.
fn sanitized_heap_run(
    source: &str,
    fixtures: &Path,
    native: &NativeHeapScenarios,
    scenario: &HeapScenario,
) -> std::process::Output {
    use std::process::Command;
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let directory = std::env::temp_dir().join(format!(
        "qcore-ct-heap-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&directory).unwrap();
    let c_path = directory.join("module.c");
    let exe_path = directory.join("module");
    fs::write(&c_path, source).unwrap();
    for entry in fs::read_dir(fixtures).unwrap() {
        let entry = entry.unwrap().path();
        if entry.extension().is_some_and(|extension| extension == "h") {
            fs::copy(&entry, directory.join(entry.file_name().unwrap())).unwrap();
        }
    }
    let compiler = std::env::var("QCORE_CLANG").unwrap_or_else(|_| "clang".to_owned());
    let mut command = Command::new(compiler);
    command
        .args(&native.sanitizer_flags)
        .args(&scenario.defines);
    if cfg!(target_os = "macos") {
        command.arg("-I").arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/apple-threads"
        ));
    }
    let compiled = command
        .arg("-include")
        .arg(directory.join(&scenario.header))
        .arg(&c_path)
        .arg("-o")
        .arg(&exe_path)
        .output()
        .unwrap();
    assert!(
        compiled.status.success(),
        "the C compiler rejected the emitted program for {}:\n{}",
        scenario.name,
        String::from_utf8_lossy(&compiled.stderr)
    );
    let output = Command::new(&exe_path)
        .env(
            "ASAN_OPTIONS",
            if scenario.detect_leaks && !cfg!(target_os = "macos") {
                "detect_leaks=1:halt_on_error=1"
            } else {
                "detect_leaks=0:halt_on_error=1"
            },
        )
        .env("UBSAN_OPTIONS", &native.ubsan_options)
        .output()
        .unwrap();
    fs::remove_dir_all(directory).unwrap();
    output
}

/// CT26: the whole `source.mutex.guard.v1` stage stack on the case's own
/// fixture — parse, elaborate, lower, check, independently recheck, emit, then
/// compile the emitted C under ASan/UBSan (and LSan on Linux) and run it. The native stdout is
/// compared line for line with `interpret_mutex_program`, which is written
/// against the checked event list and never reads the emitted C.
fn run_checked_mutex_sanitized(
    expected: &ExpectedCase,
    result: Result<qcore_compiler::CompiledModule, Vec<qcore_compiler::FileDiagnostic>>,
) {
    use qcore_compiler::{ConcurrencyAssumption, ConcurrencyProfile, SourceExecutionProfile};
    let mutex = expected.checked_mutex_sanitized.as_ref().unwrap();
    if cfg!(target_os = "windows") {
        assert!(
            result
                .as_ref()
                .expect("Windows source must check and emit")
                .c_source
                .is_some()
        );
        eprintln!(
            "{}: emitted on Windows; Unix sanitizer evidence unexecuted on this host",
            expected.id
        );
        return;
    }
    if !cfg!(any(
        target_os = "linux",
        target_os = "macos",
        target_os = "windows"
    )) {
        unsupported_target(&result.unwrap_err());
        eprintln!(
            "{}: source/core checked; mutex reference/native execution unexecuted on this host \
             (manifest evidence_target=linux)",
            expected.id
        );
        return;
    }
    let module = result.expect("the accepted mutex fixture must check on this route");
    assert_eq!(
        module.execution_profile,
        SourceExecutionProfile::CheckedMutexGuardsV1,
        "{}: the case must select the checked mutex route, not another one",
        expected.id
    );
    assert!(
        module.reference_observation.is_none(),
        "{}: do not substitute the sequential execution profile",
        expected.id
    );
    assert!(module.checked.as_scoped_tasks().is_none());
    assert!(module.checked.as_sequential().is_none());
    let checked = module
        .checked
        .as_checked_mutex_guards()
        .expect("the checked artifact variant is explicit for this route");
    assert_eq!(
        checked.certificate.profile,
        ConcurrencyProfile::CheckedMutexInvariantV1,
        "{}: a trusted-trace certificate can never satisfy this case",
        expected.id
    );
    for forbidden in [
        ConcurrencyAssumption::GuardFootprintAuthority,
        ConcurrencyAssumption::SynchronizedCustodyEdges,
    ] {
        assert!(
            !checked.certificate.assumptions.contains(&forbidden),
            "{}: a checked-profile certificate never carries a trusted-model \
             guard premise",
            expected.id
        );
    }
    let reference = qcore_core::mutex_c_emit::interpret_mutex_program(
        &checked.lowered.program,
        &checked.certificate,
    )
    .expect("the lowered trace is inside the emitted subset");
    assert_eq!(
        reference, mutex.reference_observation,
        "{}: reference observation drifted from the pinned expectation",
        expected.id
    );
    for guard in &mutex.guards {
        assert!(
            reference
                .iter()
                .any(|line| line == &format!("lock 1 {guard}")),
            "{}: guard {guard} is not minted by its own critical section",
            expected.id
        );
    }
    assert_eq!(
        reference
            .iter()
            .filter(|line| line.starts_with("unlock "))
            .count(),
        mutex.sections,
        "{}: every critical section must release",
        expected.id
    );
    let c = module.c_source.as_deref().expect("threaded emission");
    for wanted in &mutex.emitted_c_contains {
        assert!(
            c.contains(wanted.as_str()),
            "{}: emitted C is missing `{wanted}`",
            expected.id
        );
    }
    assert_eq!(
        mutex_sanitized_run(&mutex_observation::instrument(c), mutex),
        reference,
        "{}: native and reference observations must agree line for line",
        expected.id
    );
}

fn mutex_sanitized_run(source: &str, mutex: &CheckedMutexSanitized) -> Vec<String> {
    sanitized_c_run(
        source,
        &mutex.sanitizer_flags,
        &mutex.asan_options,
        &mutex.ubsan_options,
    )
}

/// Card `CT34-SANITIZED`: the same run for a control that renders its own C.
fn control_sanitized_run(source: &str, run: &ControlSanitizedRun) -> Vec<String> {
    sanitized_c_run(
        source,
        &run.sanitizer_flags,
        &run.asan_options,
        &run.ubsan_options,
    )
}

fn sanitized_c_run(
    source: &str,
    sanitizer_flags: &[String],
    asan_options: &str,
    ubsan_options: &str,
) -> Vec<String> {
    use std::process::Command;
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let directory = std::env::temp_dir().join(format!(
        "qcore-ct-mutex-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&directory).unwrap();
    let path = directory.join("module.c");
    let binary = directory.join("module");
    fs::write(&path, source).unwrap();
    let compiler = std::env::var("QCORE_CLANG").unwrap_or_else(|_| "cc".to_owned());
    let compiled = Command::new(&compiler)
        .args(sanitizer_flags)
        .arg(&path)
        .arg("-o")
        .arg(&binary)
        .output()
        .unwrap();
    assert!(
        compiled.status.success(),
        "C compiler rejected the emitted program:\n{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    let asan_options = if cfg!(target_os = "macos") {
        // Apple's AddressSanitizer does not implement LeakSanitizer.
        "detect_leaks=0:halt_on_error=1"
    } else {
        asan_options
    };
    let run = Command::new(&binary)
        .env("ASAN_OPTIONS", asan_options)
        .env("UBSAN_OPTIONS", ubsan_options)
        .output()
        .unwrap();
    assert!(
        run.status.success(),
        "emitted program failed under the platform sanitizers:\n{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let observation = String::from_utf8(run.stdout)
        .unwrap()
        .lines()
        .map(ToOwned::to_owned)
        .collect();
    fs::remove_dir_all(directory).unwrap();
    observation
}

fn run_positive_control(directory: &Path, expected: &PositiveControl) {
    let path = directory.join(&expected.source);
    let source = fs::read_to_string(&path).unwrap();
    let result = compile_files(
        &[SourceFile::new(path.to_string_lossy(), source)],
        &CompileOptions::default(),
    );
    if !cfg!(any(
        target_os = "linux",
        target_os = "macos",
        target_os = "windows"
    )) {
        unsupported_target(&result.unwrap_err());
        eprintln!(
            "CT19 joined-sibling control: source/core checked; threaded reference/native evidence unexecuted on this host"
        );
        return;
    }
    let module =
        result.expect("joined siblings must be accepted, so rejection is not a syntax gate");
    let reference = qcore_core::task_execution::execute_task_module(
        module
            .checked
            .as_scoped_tasks()
            .expect("control must use actual tasks"),
        &qcore_core::task_execution::TaskExecutionScript::default(),
    )
    .unwrap();
    assert_eq!(
        reference.root_outcome,
        qcore_core::scoped_tasks::TaskOutcome::I32(expected.expected_i32)
    );
    let children = reference
        .tasks
        .iter()
        .filter(|task| task.parent.is_some())
        .collect::<Vec<_>>();
    assert_eq!(children.len(), expected.joined_children);
    assert!(
        children.iter().all(|task| task.joined),
        "positive control must explicitly collect both siblings"
    );
}

fn run_native_join_failure(
    directory: &Path,
    expected: &ExpectedCase,
    result: Result<qcore_compiler::CompiledModule, Vec<qcore_compiler::FileDiagnostic>>,
) {
    let native = expected.native_join_failure.as_ref().unwrap();
    assert!(directory.join(&native.header).is_file());
    if !cfg!(any(
        target_os = "linux",
        target_os = "macos",
        target_os = "windows"
    )) {
        unsupported_target(&result.unwrap_err());
        eprintln!(
            "CT15: source/core checked; native/reference execution unexecuted on this host (manifest evidence_target=linux)"
        );
        return;
    }
    let module = result.expect("CT15 source must check before native fault injection");
    let checked = module
        .checked
        .as_scoped_tasks()
        .expect("CT15 must use the task execution artifact");
    assert!(
        module.reference_observation.is_none(),
        "do not substitute the sequential execution profile"
    );
    let reference = qcore_core::task_execution::execute_task_module(
        checked,
        &qcore_core::task_execution::TaskExecutionScript::default(),
    )
    .unwrap();
    assert_eq!(
        reference.root_outcome,
        qcore_core::scoped_tasks::TaskOutcome::I32(native.control_i32)
    );
    let children = reference
        .tasks
        .iter()
        .filter(|task| task.parent.is_some())
        .collect::<Vec<_>>();
    assert_eq!(children.len(), 1);
    assert!(children[0].joined);
    let transcript = qcore_core::task_execution::render_task_observation(&reference);
    let normal = native_join_run(
        module.c_source.as_deref().unwrap(),
        &directory.join(&native.header),
        false,
    );
    assert!(normal.status.success(), "CT15 control failed: {normal:?}");
    assert!(!transcript.is_empty());
    assert!(normal.stdout.is_empty());
    assert_eq!(
        normal.stderr, b"JOIN_OK\nFREE\n",
        "control must really join and reclaim its environment"
    );
    let failed = native_join_run(
        module.c_source.as_deref().unwrap(),
        &directory.join(&native.header),
        true,
    );
    assert_eq!(
        failed.status.code(),
        Some(native.abort_exit_code),
        "CT15 must take process abort policy: {failed:?}"
    );
    assert_eq!(
        failed.stderr, b"JOIN_FAILED\n",
        "failed join must not reclaim its environment"
    );
    assert!(failed.stdout.is_empty());
}

fn native_join_run(source: &str, header: &Path, fail_join: bool) -> std::process::Output {
    native_fault_run(
        source,
        header,
        if fail_join {
            "-DQCORE_CT_FAIL_JOIN=1"
        } else {
            "-DQCORE_CT_FAIL_JOIN=0"
        },
    )
}

fn native_fault_run(source: &str, header: &Path, define: &str) -> std::process::Output {
    native_fault_run_defines(source, header, &[define.to_owned()])
}

fn native_fault_run_defines(
    source: &str,
    header: &Path,
    defines: &[String],
) -> std::process::Output {
    #[cfg(target_os = "windows")]
    return windows_c::run(
        source,
        Some(&format!(
            "#include \"{}\"",
            header.to_string_lossy().replace('\\', "/")
        )),
        defines,
    );
    #[cfg(not(target_os = "windows"))]
    {
        use std::process::Command;
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let directory = std::env::temp_dir().join(format!(
            "qcore-ct-native-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&directory).unwrap();
        let c_path = directory.join("module.c");
        let exe_path = directory.join("module");
        fs::write(&c_path, source).unwrap();
        let compiler = std::env::var("QCORE_CLANG").unwrap_or_else(|_| "clang".to_owned());
        let mut command = Command::new(compiler);
        command
            .args(["-std=c11", "-pthread", "-Wall", "-Wextra", "-Werror"])
            .args(defines);
        if cfg!(target_os = "macos") {
            // Fault-injection headers are force-included before the generated
            // source, so make their `<threads.h>` include resolve to the same
            // pthread-backed compatibility surface used by emitted C.
            command.arg("-I").arg(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/apple-threads"
            ));
        }
        let compiled = command
            .arg("-include")
            .arg(header)
            .arg(&c_path)
            .arg("-o")
            .arg(&exe_path)
            .output()
            .unwrap();
        assert!(
            compiled.status.success(),
            "CT emitted C failed: {} (artifact {})",
            String::from_utf8_lossy(&compiled.stderr),
            c_path.display()
        );
        let output = Command::new(&exe_path).output().unwrap();
        fs::remove_dir_all(directory).unwrap();
        output
    }
}

fn run_native_create_recovery(
    directory: &Path,
    expected: &ExpectedCase,
    result: Result<qcore_compiler::CompiledModule, Vec<qcore_compiler::FileDiagnostic>>,
) {
    use qcore_core::scoped_tasks::TaskOutcome;
    use qcore_core::task_execution::{
        TaskExecutionScript, execute_task_module, render_task_observation,
    };
    let native = expected.native_create_recovery.as_ref().unwrap();
    // The corresponding failure arm that omits its join must reject at the
    // task-state gate, not merely because matching syntax is unsupported.
    let bad_path = directory.join(&native.unjoined_source);
    let errors = compile_files(
        &[SourceFile::new(
            bad_path.to_string_lossy(),
            fs::read_to_string(&bad_path).unwrap(),
        )],
        &CompileOptions::default(),
    )
    .unwrap_err();
    assert!(
        errors
            .iter()
            .any(|error| error.code == "Q600" && error.message.contains("ScopeBusy")),
        "{errors:?}"
    );
    if !cfg!(any(
        target_os = "linux",
        target_os = "macos",
        target_os = "windows"
    )) {
        unsupported_target(&result.unwrap_err());
        eprintln!(
            "CT13: source/core checked; native/reference creation-recovery evidence unexecuted on this host"
        );
        return;
    }
    let module = result.expect("CT13 recovery source must check before fault injection");
    let checked = module.checked.as_scoped_tasks().unwrap();
    for (failed_create, decisions, outcome, children, stderr) in [
        (
            0,
            vec![true, true],
            TaskOutcome::I32(native.success_i32),
            2,
            "CREATE_OK:1\nCREATE_OK:2\nJOIN_OK:2\nFREE\nJOIN_OK:1\nFREE\n",
        ),
        (
            1,
            vec![false],
            TaskOutcome::Error("SpawnError".into()),
            0,
            "CREATE_FAILED:1\nFREE\n",
        ),
        (
            2,
            vec![true, false],
            TaskOutcome::I32(native.recovery_i32),
            1,
            "CREATE_OK:1\nCREATE_FAILED:2\nFREE\nJOIN_OK:1\nFREE\n",
        ),
    ] {
        let reference = execute_task_module(
            checked,
            &TaskExecutionScript {
                spawn_outcomes: decisions,
                allocation_outcomes: vec![],
            },
        )
        .unwrap();
        assert_eq!(reference.root_outcome, outcome);
        let actual_children = reference
            .tasks
            .iter()
            .filter(|task| task.parent.is_some())
            .collect::<Vec<_>>();
        assert_eq!(actual_children.len(), children);
        assert!(actual_children.iter().all(|task| task.joined));
        let run = native_fault_run(
            &diagnostic_sink::instrument(module.c_source.as_deref().unwrap()),
            &directory.join(&native.header),
            &format!("-DQCORE_CT_FAIL_CREATE={failed_create}"),
        );
        assert!(
            run.status.success(),
            "CT13 native mode {failed_create} failed: {run:?}"
        );
        assert_eq!(run.stdout, render_task_observation(&reference).as_bytes());
        assert_eq!(
            run.stderr,
            stderr.as_bytes(),
            "must join the exact earlier native thread after second creation failure"
        );
    }
}

fn run_native_owned_transfer(
    directory: &Path,
    expected: &ExpectedCase,
    result: Result<qcore_compiler::CompiledModule, Vec<qcore_compiler::FileDiagnostic>>,
) {
    use qcore_core::scoped_tasks::TaskOutcome;
    use qcore_core::task_execution::{
        TaskExecutionScript, execute_task_module, render_task_observation,
    };
    let native = expected.native_owned_transfer.as_ref().unwrap();
    if !cfg!(any(
        target_os = "linux",
        target_os = "macos",
        target_os = "windows"
    )) {
        unsupported_target(&result.unwrap_err());
        eprintln!(
            "{}: owned-task reference/native evidence unexecuted on this host",
            expected.id
        );
        return;
    }
    let module = result.expect("owned-task source must pass all-path ownership checks");
    assert!(module.reference_observation.is_none());
    let checked = module.checked.as_scoped_tasks().unwrap();
    let scripts = if native.fail_create {
        vec![false, true]
    } else {
        vec![true]
    };
    for succeeds in scripts {
        let reference = execute_task_module(
            checked,
            &TaskExecutionScript {
                spawn_outcomes: vec![succeeds],
                allocation_outcomes: vec![true],
            },
        )
        .unwrap();
        assert_eq!(
            reference.root_outcome,
            TaskOutcome::I32(native.expected_i32)
        );
        assert_eq!(reference.tasks.len(), if succeeds { 2 } else { 1 });
        if succeeds {
            assert_eq!(reference.tasks[1].outcome, TaskOutcome::OwnedBuffer(1));
            assert!(reference.tasks[1].joined);
        }
        let output = native_fault_run(
            &diagnostic_sink::instrument(module.c_source.as_deref().unwrap()),
            &directory.join(&native.header),
            if succeeds {
                "-DQCORE_CT_FAIL_CREATE=0"
            } else {
                "-DQCORE_CT_FAIL_CREATE=1"
            },
        );
        assert!(output.status.success(), "{output:?}");
        assert_eq!(
            output.stdout,
            render_task_observation(&reference).as_bytes()
        );
        assert_eq!(
            output.stderr,
            if succeeds {
                b"BUFFER_ALLOC\nCREATE_OK\nJOIN_OK\nENV_FREE\nBUFFER_FREE\n".as_slice()
            } else {
                b"BUFFER_ALLOC\nCREATE_FAILED\nENV_FREE\nBUFFER_FREE\n".as_slice()
            }
        );
    }
}

fn run_native_task_scenarios(
    directory: &Path,
    expected: &ExpectedCase,
    result: Result<qcore_compiler::CompiledModule, Vec<qcore_compiler::FileDiagnostic>>,
) {
    use qcore_core::scoped_tasks::TaskOutcome;
    use qcore_core::task_execution::{
        TaskExecutionScript, execute_task_module_with_worker_allocations, render_task_observation,
    };
    let native = expected.native_task_scenarios.as_ref().unwrap();
    for control in &native.rejection_controls {
        let path = directory.join(&control.source);
        let errors = compile_files(
            &[SourceFile::new(
                path.to_string_lossy(),
                fs::read_to_string(&path).unwrap(),
            )],
            &CompileOptions::default(),
        )
        .unwrap_err();
        assert!(
            errors
                .iter()
                .any(|error| error.code == control.diagnostic.code
                    && error.message.contains(&control.diagnostic.message_contains)),
            "{errors:?}"
        );
    }
    if !cfg!(any(
        target_os = "linux",
        target_os = "macos",
        target_os = "windows"
    )) {
        unsupported_target(&result.unwrap_err());
        eprintln!(
            "{}: threaded task scenarios unexecuted on this host",
            expected.id
        );
        return;
    }
    let module = result.expect("task scenario source must pass all-path checks");
    let checked = module.checked.as_scoped_tasks().unwrap();
    for scenario in &native.scenarios {
        let selected_allocations = if scenario.allocation_task == 0 {
            &scenario.allocation_outcomes
        } else {
            scenario
                .worker_allocation_outcomes
                .get(&scenario.allocation_task)
                .expect("selected allocation task needs explicit decisions")
        };
        for (failed, outcomes) in [
            (scenario.failed_allocation, selected_allocations),
            (scenario.failed_create, &scenario.spawn_outcomes),
        ] {
            assert!(
                outcomes
                    .iter()
                    .enumerate()
                    .all(|(index, succeeds)| *succeeds == (index + 1 != failed)),
                "{}: inconsistent failure script",
                scenario.name
            );
            assert!(
                failed == 0 || failed <= outcomes.len(),
                "{}: failure must execute",
                scenario.name
            );
        }
        let reference = execute_task_module_with_worker_allocations(
            checked,
            &TaskExecutionScript {
                spawn_outcomes: scenario.spawn_outcomes.clone(),
                allocation_outcomes: scenario.allocation_outcomes.clone(),
            },
            &scenario.worker_allocation_outcomes,
        )
        .unwrap();
        let outcome = match (&scenario.outcome.error, scenario.outcome.ok_value) {
            (Some(error), None) => {
                assert!(scenario.outcome.payload.is_none());
                TaskOutcome::Error(error.clone())
            }
            (None, Some(value)) => {
                assert_eq!(scenario.outcome.payload.as_deref(), Some([].as_slice()));
                TaskOutcome::I32(value)
            }
            _ => panic!("scenario requires exactly one result"),
        };
        assert_eq!(reference.root_outcome, outcome, "{}", scenario.name);
        assert_eq!(reference.tasks.len(), scenario.joined_children + 1);
        assert!(reference.tasks.iter().skip(1).all(|task| task.joined));
        let owned = reference
            .tasks
            .iter()
            .skip(1)
            .filter_map(|task| match task.outcome {
                TaskOutcome::OwnedBuffer(id) => Some(id),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(owned, scenario.owned_child_results);
        if expected.id == "CT50" && !cfg!(target_os = "linux") {
            // CT50 is not a threading fixture: its native evidence requires
            // Linux MAP_FIXED_NOREPLACE. The source, reference, CFG, and C
            // emission stages still execute on macOS; only that OS-specific
            // mapping-address observation remains Linux-scoped.
            eprintln!("CT50: Linux MAP_FIXED_NOREPLACE native observation unexecuted on this host");
            continue;
        }
        let output = native_fault_run_defines(
            &diagnostic_sink::instrument(module.c_source.as_deref().unwrap()),
            &directory.join(&native.header),
            &[
                format!("-DQCORE_CT_ALLOC_TASK={}", scenario.allocation_task),
                format!("-DQCORE_CT_FAIL_ALLOC={}", scenario.failed_allocation),
                format!("-DQCORE_CT_FAIL_CREATE={}", scenario.failed_create),
            ],
        );
        assert!(output.status.success(), "{}: {output:?}", scenario.name);
        assert_eq!(
            output.stdout,
            render_task_observation(&reference).as_bytes(),
            "{}",
            scenario.name
        );
        assert_eq!(
            output.stderr,
            scenario.native_stderr.as_bytes(),
            "{}",
            scenario.name
        );
    }
}

#[test]
fn ct16_retains_sequential_implicit_cleanup_control() {
    let module = compile_files(
        &[SourceFile::new(
            "sequential-worker-error.qr",
            include_str!("fixtures/task_worker_allocations/sequential_worker_error.qr"),
        )],
        &CompileOptions {
            entry: Some("worker".into()),
            ..CompileOptions::default()
        },
    )
    .unwrap();
    let observation = module.reference_observation.unwrap();
    assert!(
        matches!(&observation.outcome, ExecutionOutcome::Error {name, ..} if name == "WorkerError")
    );
    assert_eq!(
        observation
            .trace
            .iter()
            .map(trace_token)
            .collect::<Vec<_>>(),
        ["acquire:3", "release:3"]
    );
}

#[test]
fn ct37_retains_sequential_transferred_payload_control() {
    let module = compile_files(
        &[SourceFile::new(
            "sequential-completion-payload.qr",
            include_str!("fixtures/task_worker_allocations/sequential_completion_payload.qr"),
        )],
        &CompileOptions {
            entry: Some("worker".into()),
            ..CompileOptions::default()
        },
    )
    .unwrap();
    let observation = module.reference_observation.unwrap();
    assert!(
        matches!(&observation.outcome, ExecutionOutcome::Ok { value, payload } if *value == 5 && payload == &[6])
    );
    assert_eq!(
        observation
            .trace
            .iter()
            .map(trace_token)
            .collect::<Vec<_>>(),
        ["acquire:5", "acquire:6", "transfer:6", "release:5"]
    );
}

/// Card `STABILITY-REEVAL-3` (wave 38). The proof stage is credited to a
/// **string**: `PROOF_GATED_CASES` names `file::test`, and the manifest checks
/// above compare that string with the manifest's `executor` and with
/// `RUNNER_REGISTRY` — nothing reads the file it names. A proof-leg test
/// renamed, `#[ignore]`d, or re-pointed at another case's fixture (CT12's is
/// byte-identical to CT01's) would therefore leave the numerator reading 3
/// with this harness green. This test reads each named file and asserts the
/// binding the string credits: the named `fn` exists, its attribute block
/// carries `#[test]` and no `#[ignore]`, and the file `include_str!`s the
/// case's own `fixtures/ct/<ID>/case.qr`. Shown red by mutation in
/// `docs/reports/stability-reeval-3/`; the residual it closes is
/// `CT-PROOF-LEG-BINDING-BY-NAME`.
#[test]
fn proof_gated_cases_name_a_live_test_that_reads_the_cases_own_fixture() {
    for (id, executor) in PROOF_GATED_CASES {
        let (file, test) = executor
            .split_once("::")
            .unwrap_or_else(|| panic!("{id}: executor `{executor}` is not `file::test`"));
        let path = repo_root().join(file);
        let text = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("{id}: proof-leg file `{file}` is unreadable: {error}"));
        let signature = format!("fn {test}()");
        let at = text
            .find(&signature)
            .unwrap_or_else(|| panic!("{id}: `{file}` defines no `{signature}`"));
        // The attribute block is the run of `#[…]` and `///` lines immediately
        // above the signature; it must carry `#[test]` and nothing ignoring it.
        let attributes: Vec<&str> = text[..at]
            .lines()
            .rev()
            .take_while(|line| {
                let line = line.trim_start();
                line.starts_with("#[") || line.starts_with("///")
            })
            .map(str::trim)
            .collect();
        assert!(
            attributes.contains(&"#[test]"),
            "{id}: `{signature}` in `{file}` is not a `#[test]`: {attributes:?}"
        );
        assert!(
            attributes.iter().all(|line| !line.contains("ignore")),
            "{id}: `{signature}` in `{file}` is ignored, so the proof stage it is credited \
             with runs nowhere: {attributes:?}"
        );
        let fixture = format!("include_str!(\"fixtures/ct/{id}/case.qr\")");
        assert!(
            text.contains(&fixture),
            "{id}: `{file}` does not read `{fixture}`, so the test it names is not lowering \
             this case's own fixture"
        );
    }
}

/// Every tracked file under `root`, recursively, as `(repository-relative
/// path with `/` separators, bytes)`. Nothing tracked is skipped: a committed
/// copy placed anywhere under the scanned tree is a copy. An untracked or
/// ignored file is not part of the tree (card W45-TEST-HYGIENE).
fn fixture_files(root: &Path, relative: &str, out: &mut Vec<(String, Vec<u8>)>) {
    for path in tracked_files::tracked_files(root) {
        let name = path
            .strip_prefix(root)
            .unwrap()
            .to_str()
            .unwrap()
            .replace('\\', "/");
        out.push((format!("{relative}/{name}"), fs::read(&path).unwrap()));
    }
}

/// Card `CT-SHARED-FIXTURE-DISCLOSURE` (wave 43). The CT denominator counts
/// cases, and three relations make a case's artifact something other than
/// its own: several cases sharing one program (`shared_artifact`), one case's
/// program doubling as another case's control (`case_as_control`), and a CT
/// file being a copy of another test's fixture (`shared_with_non_ct_fixture`).
/// Copies are separate files, so nothing couples an edit on one side to the
/// other: the hazard is a *silent divergence* of an identity a report relies
/// on, or a new identity nobody reviewed.
///
/// This test hashes every file under the manifest's `scan_root` and requires
/// the byte-identical groups that contain a CT fixture to be **exactly** the
/// manifest's reviewed `shared_fixture_groups` — same digest, same members,
/// roles derived from the files rather than trusted, relation consistent with
/// those roles — in both directions, so an edit to one side of a disclosed
/// group, a new undisclosed copy, and a stale allow-list entry each fail here.
/// It also recomputes the independent-artifact figure the manifest discloses.
/// Shown red by mutation in `docs/reports/ct-shared-fixture-disclosure/`.
#[test]
fn ct_fixture_byte_identity_is_exactly_the_reviewed_disclosure() {
    fn repository_bytes(bytes: &[u8]) -> Vec<u8> {
        // Git can check text fixtures out with CRLF on Windows. Compare the
        // committed LF spelling in memory without rewriting the checkout.
        if cfg!(windows) {
            bytes
                .split(|byte| *byte == b'\n')
                .enumerate()
                .flat_map(|(index, line)| {
                    let mut part = if index == 0 { Vec::new() } else { vec![b'\n'] };
                    part.extend_from_slice(line.strip_suffix(b"\r").unwrap_or(line));
                    part
                })
                .collect()
        } else {
            bytes.to_vec()
        }
    }
    let manifest = load_manifest();
    let identity = &manifest.artifact_identity;
    assert_eq!(
        identity.report, "docs/reports/ct-shared-fixture-disclosure/README.md",
        "the disclosure names the report that reviewed it"
    );
    assert!(
        repo_root().join(&identity.report).is_file(),
        "the reviewing report `{}` exists",
        identity.report
    );
    assert_eq!(identity.scan_root, "compiler/tests/fixtures");
    let ct_prefix = format!("{}/ct/", identity.scan_root);

    // 1. The independent-artifact figure, recomputed from the files.
    let mapped: Vec<&ManifestCase> = manifest
        .cases
        .iter()
        .filter(|case| case.mapping_status == "mapped")
        .collect();
    let source_fixtures: BTreeSet<String> = mapped
        .iter()
        .map(|case| {
            case.source_fixture
                .clone()
                .unwrap_or_else(|| panic!("{}: a mapped case names its source fixture", case.id))
        })
        .collect();
    let distinct_sources: BTreeSet<String> = source_fixtures
        .iter()
        .map(|path| {
            qcore_schemas::sha256_hex(&repository_bytes(
                &fs::read(repo_root().join(path)).unwrap(),
            ))
        })
        .collect();
    assert_eq!(
        (
            identity.mapped_cases,
            identity.distinct_source_fixture_digests
        ),
        (mapped.len(), distinct_sources.len()),
        "artifact_identity must disclose (mapped cases, distinct source-fixture digests) \
         as measured"
    );

    // 2. The measured byte-identity groups that contain a CT fixture.
    let mut files = Vec::new();
    fixture_files(
        &repo_root().join(&identity.scan_root),
        &identity.scan_root,
        &mut files,
    );
    let mut by_digest: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (path, bytes) in &files {
        by_digest
            .entry(qcore_schemas::sha256_hex(&repository_bytes(bytes)))
            .or_default()
            .insert(path.clone());
    }
    let measured: BTreeMap<String, BTreeSet<String>> = by_digest
        .into_iter()
        .filter(|(_, members)| {
            members.len() > 1 && members.iter().any(|path| path.starts_with(&ct_prefix))
        })
        .collect();

    // 3. The reviewed allow-list, with every role derived rather than trusted.
    let mut declared: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for group in &identity.shared_fixture_groups {
        assert_eq!(
            group.decision, "disclose",
            "{}: a listed group is disclosed; a decoupled pair is no longer a group",
            group.sha256
        );
        let mut roles: BTreeMap<&str, usize> = BTreeMap::new();
        for member in &group.members {
            let derived = if !member.path.starts_with(&ct_prefix) {
                "non_ct_fixture"
            } else if source_fixtures.contains(&member.path) {
                "case"
            } else if member.path.ends_with(".h") {
                "header"
            } else {
                "control"
            };
            assert_eq!(
                member.role, derived,
                "{}: `{}` is recorded as `{}` but is a `{derived}`",
                group.sha256, member.path, member.role
            );
            *roles.entry(derived).or_insert(0) += 1;
        }
        let ct_roles = roles
            .keys()
            .filter(|role| **role != "non_ct_fixture")
            .count();
        let consistent = match group.relation.as_str() {
            "shared_artifact" => {
                !roles.contains_key("non_ct_fixture")
                    && ct_roles == 1
                    && (roles.contains_key("case") || roles.contains_key("header"))
            }
            "case_as_control" => {
                !roles.contains_key("non_ct_fixture")
                    && roles.contains_key("case")
                    && roles.contains_key("control")
            }
            "shared_with_non_ct_fixture" => roles.contains_key("non_ct_fixture"),
            other => panic!("{}: unknown relation `{other}`", group.sha256),
        };
        assert!(
            consistent,
            "{}: relation `{}` does not match its members' roles {roles:?}",
            group.sha256, group.relation
        );
        let members: BTreeSet<String> = group.members.iter().map(|m| m.path.clone()).collect();
        assert_eq!(
            members.len(),
            group.members.len(),
            "{}: a member is listed twice",
            group.sha256
        );
        assert!(
            declared.insert(group.sha256.clone(), members).is_none(),
            "{}: a digest is listed twice",
            group.sha256
        );
    }

    // 4. Total-or-reject in both directions.
    let undisclosed: Vec<_> = measured
        .iter()
        .filter(|(digest, members)| declared.get(*digest) != Some(*members))
        .collect();
    let stale: Vec<_> = declared
        .iter()
        .filter(|(digest, members)| measured.get(*digest) != Some(*members))
        .collect();
    assert!(
        undisclosed.is_empty() && stale.is_empty(),
        "byte-identical CT fixture groups differ from the reviewed disclosure in \
         docs/reevaluation/ct_manifest.json `artifact_identity.shared_fixture_groups`.\n\
         measured but not listed exactly: {undisclosed:#?}\n\
         listed but not measured exactly: {stale:#?}\n\
         A new copy needs review and a relation; an edit to one side of a listed group \
         ends an identity a report relies on and needs a correction beside that report."
    );
}

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

//! Source-file orchestration for the sequential cleanup and scoped task
//! profiles. Each path produces its own checked evidence and rejects unsupported
//! forms before C emission. Task source is never represented by an erased
//! sequential CFG.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::source_imports::{
    ImportLimits, ResolvedSourceBundle, SourceBundleEvidence, SourceLoading, resolve_source_bundle,
};
use qcore_core::cleanup_lowering::{
    BitwiseOp, BranchCondition, CompareOp, IntExpr, SourceOp, SourceProgram as LoweringProgram,
    interpret_source,
};
use qcore_core::error_region::{
    CleanupEvent, QuantOpen as RegionQuantOpen, RegionCatchClause, RegionExitTag, RegionOp,
    analyze_error_regions,
};
use qcore_core::ownership::{Footprint, OwnershipOp, ResourceId};
use qcore_core::prelude::concurrency::{
    epoch_accounting, epoch_credit_recovery, mutex_c_emit, mutex_invariant, task_c_emit,
    task_execution,
};
use qcore_core::resource_flow::{Flow, Quantity, ResourceBounds};
pub use qcore_core::scalar_types::RecordedTarget;
use qcore_core::typed_errors::{Effect, EffectRow, ErrorDeclarations, ErrorLedger, ErrorOp};
use qcore_core::usage::{Usage, UsageExpr};
use qcore_core::{ExecutionObservation, emit_c11_with_log_level, extern_c_emit, extern_call};
use qcore_core::{
    composed_std_io, composed_std_io_c_emit, composed_tasks, resource_calls, resource_calls_c_emit,
    std_buffer, std_buffer_c_emit, std_event, std_event_c_emit, std_http, std_http_c_emit, std_io,
    std_io_c_emit, std_socket, std_socket_c_emit, std_string, std_string_c_emit,
};
use qcore_frontend::frontend::{
    DeclarationKind, ExecutableResultType, Located, SemanticPass, SourceBlock, SourceDeclaration,
    SourceDiagnostic, SourceModuleMode, SourceProgram, StatementKind, TypedSourceDeclaration,
    TypedSourceProgram, elaborate_source, parse_source_program,
};
use qcore_frontend::lexer::{Span, Symbol, Token, TokenKind};
use qcore_frontend::source_annotation::{
    ExecutableLocalType, check_local_annotation, local_binding_head,
};
use qcore_frontend::whole_program::{
    CallSite, CheckedProgram, FunctionSemantics, FunctionSummary, ParameterOwnership,
    check_whole_program,
};
use qcore_verifier_host::VerifierRegistry;

mod source_routes;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceFile {
    pub path: String,
    pub source: String,
}

impl SourceFile {
    pub fn new(path: impl Into<String>, source: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            source: source.into(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CompileOptions {
    pub entry: Option<String>,
    /// Explicit selection of declared extern contract modules, as an immutable
    /// path → source-text map loaded by the total-or-reject resource-contract
    /// v1 loader (card `source.extern-call.v1`).
    ///
    /// `None` selects nothing. A module that calls a declared extern function
    /// with no selection is refused at `Q023`; there is no default environment,
    /// no filesystem search and no fallback. Contracts admitted here are
    /// `ContractTrust::Declared` assumptions about C implementations this
    /// repository never analysed.
    pub extern_contracts: Option<BTreeMap<String, String>>,
    /// Explicit selection of `source.native-library.v1` manifests, as an
    /// immutable path → source-text map loaded by the total-or-reject manifest
    /// loader (card `UNSAFE-LINKAGE-ARTIFACT`,
    /// [`qcore_core::native_library`]).
    ///
    /// `None` selects nothing, and nothing selected is *exactly* the wave-18
    /// world: a waived declaration keeps the bare
    /// `UncheckedLinkage { callee, signature }` premise, and no program that
    /// compiled without a manifest can be refused because of one.
    ///
    /// What a selection can do is **sharpen** a waiver into a premise naming the
    /// artifact it was waived from, and **refuse** a declaration the named
    /// artifact's manifest contradicts. It can never widen the admitted surface:
    /// a manifest cannot buy a pointer for a contract-less entry, because an
    /// artifact identity says which code, not what it does with your pointer.
    ///
    /// Unlike the waiver itself — which card `UNSAFE-LINKAGE` deliberately put
    /// in the *source*, so that two byte-identical files cannot mean different
    /// things — the manifest is a builder-side selection, because it is not the
    /// trust act. The trust act stays the `unsafe extern fn` declaration the
    /// reader can see; the manifest only says which build that declaration was
    /// checked against, and whichever build the builder names, the certificate
    /// prints it.
    pub native_libraries: Option<BTreeMap<String, String>>,
    /// Explicit selection of exported header texts, as an immutable
    /// reference → source-text map (card `NATIVE-LIBRARY-HEADER`,
    /// [`qcore_core::native_library_header`]).
    ///
    /// The key is the reference a manifest's `header` field spells, compared
    /// exactly and never normalised. `None` selects nothing, which matters only
    /// for a manifest that *names* a header: such a manifest then refuses at
    /// `Q063`, because a header binding this driver cannot apply is a binding it
    /// must not silently drop. A manifest naming no header is unaffected by
    /// anything supplied here — pinned in both directions, since a level that
    /// could reach a manifest which never opted into it would be a level that
    /// changes what a builder already had.
    ///
    /// The text is supplied rather than read from a path for the reason every
    /// other selection here is: this driver performs no filesystem search, and
    /// the digest a certificate carries is measured over exactly the bytes the
    /// decision was taken on.
    pub native_library_headers: Option<BTreeMap<String, String>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceLocation {
    pub path: String,
    pub line: usize,
    pub column: usize,
    pub line_source: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileDiagnostic {
    pub code: String,
    pub pass: SemanticPass,
    pub location: SourceLocation,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EmittedSourceMap {
    pub generated_start_line: usize,
    pub generated_end_line: usize,
    pub source: SourceLocation,
}

/// Result typing records the sequential scalar/identity split separately from
/// checked task-owned Buffer and ordinary owned-byte helper results.
/// General owned/error payload types remain
/// outside these bounded profiles.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceResultProfile {
    /// Body definitions only; external contracts retain structural annotations.
    pub scalar_results: BTreeMap<String, ExecutableResultType>,
    /// Functions whose checked task result carries one owned Buffer identity.
    pub owned_buffer_results: std::collections::BTreeSet<String>,
    pub custody_payloads: CustodyPayloadProfile,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CustodyPayloadProfile {
    /// `move` appends a live acquired identity to a separate completion ledger.
    /// The ownership/cleanup model checks that ledger, but the scalar result
    /// annotation does not type its identities as `own<T>` payloads.
    SeparateIdentityLedger,
    /// The scoped copy profile rejects every resource-bearing source form.
    NoResourcePayloads,
    /// Typed owned Buffer capture/result custody, checked against task slots.
    TypedTaskBuffers,
    /// Normal owned-byte helper results; the sealed `as_resource_calls()` graph
    /// retains their types. This does not admit typed error payloads.
    TypedResourceBytes,
}

/// Checked evidence for `source.bitfield-concurrency.v1`.
///
/// The source lowering retains the actual worker/write attribution beside the
/// exact footprint program handed to `DisjointFootprintsV1`; the certificate
/// is independently rechecked before publication.
#[derive(Clone, Debug)]
pub struct CheckedBitfieldTaskModule {
    pub lowered: qcore_frontend::source_bitfield_concurrency::LoweredBitfieldTaskModule,
    pub certificate: qcore_core::disjoint_footprint::FootprintCertificate,
    /// Premises outside the footprint question. This property-only route does
    /// not run the scoped-task lifetime ledger: in particular, it does not
    /// prove recovery if a later spawn or join propagates failure.
    pub assumptions: BTreeSet<BitfieldTaskAssumption>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum BitfieldTaskAssumption {
    /// The modeled write window is the successful path on which every spawn
    /// starts and every join returns. Failure-path custody remains unproved.
    SpawnAndJoinSucceed,
}

/// Checked evidence is explicit about the executed semantic profile. Task
/// modules never carry a sequential CFG with their task operations erased.
#[derive(Clone, Debug)]
pub enum CheckedSourceModule {
    ScalarCalls(Box<qcore_core::scalar_calls::Checked>),
    Sequential(Box<CheckedProgram>),
    ScopedTasks(Box<task_execution::CheckedTaskModule>),
    BitfieldTasks(Box<CheckedBitfieldTaskModule>),
    /// One entry function checked under `CheckedMutexInvariantV1`. The
    /// certificate is the evidence; the lowered program is retained beside it so
    /// a consumer can re-derive rather than believe it.
    CheckedMutexGuards(Box<CheckedMutexGuardModule>),
    /// One entry function checked under `FlatEpochBoundV1`. The certificate is
    /// the evidence; the lowered epoch is retained beside it so a consumer can
    /// re-derive rather than believe it.
    CheckedEpochBound(Box<CheckedEpochModule>),
    /// One entry function checked under `source.epoch.tasks.v1`: a flat-epoch
    /// plan composed with a real checked task region. Both halves are retained
    /// separately, because both were checked separately by their own unedited
    /// checker and only then required to agree.
    ComposedEpochTasks(Box<ComposedEpochModule>),
    /// One entry function checked under `source.extern-call.v1`. Both the
    /// extern-call certificate and the ordinary sequential evidence are
    /// retained, because both were required: the call sites are checked by the
    /// core extern checker and the surrounding custody, flow and effect
    /// obligations by the established sequential passes.
    ExternCalls(Box<CheckedExternModule>),
    /// One entry function checked under `source.std-io.v1` (card
    /// `STD-MODULES`). The certificate is the evidence; the lowered program is
    /// retained beside it so a consumer can re-derive rather than believe it.
    StandardIo(Box<CheckedStdIoModule>),
    /// One entry function checked under `source.std-io-extern.v1` (card
    /// `STD-IO-COMPOSE`): a module that both calls declared extern functions and
    /// prints. All three halves are retained separately, because all three were
    /// required and none was derived from another — the extern call sites by the
    /// core extern checker, the surrounding custody, flow and effect obligations
    /// by the established sequential passes, and the rendered lines by the
    /// standard-IO checker — with the composed certificate carrying both
    /// projections' own certificates verbatim beside the join.
    Composed(Box<CheckedComposedModule>),
    /// One entry function checked under `source.std-io-tasks.v1` (card
    /// `COMPOSE-CONCURRENCY`): a module that both spawns and prints. The task
    /// half was checked by the **unedited** `check_task_module` and is reachable
    /// on its own through
    /// [`qcore_core::composed_tasks::CheckedComposedTaskModule::tasks`], so
    /// every consumer of a checked task module — the reference execution, the C
    /// backend, the profile classification — reads exactly what the task route
    /// would have handed it.
    ComposedTasks(Box<composed_tasks::CheckedComposedTaskModule>),
    /// One entry function checked under `source.buffer.v1` (card `STD-BUFFER`):
    /// a module holding an owned, extent-carrying byte buffer. The certificate
    /// is the evidence and the lowered program is retained beside it, so a
    /// consumer can re-derive rather than believe it.
    StandardBuffer(Box<CheckedBufferModule>),
    /// One entry function checked under `source.event.v1` (card `STD-EVENT`).
    ///
    /// The custody evidence inside the certificate is the **reactor model's
    /// own**: `qcore_core::event_reactor::check_event_program` checked the
    /// projection unmodified, and this route added the determinism decision on
    /// top of it. The lowered program is retained beside the certificate so a
    /// consumer can re-derive both rather than believe either.
    ///
    /// There is no sequential evidence beside it, and that is a stated boundary
    /// rather than an omission: the route's grammar has no value vocabulary at
    /// all, so the entry binds no ordinary identity for the custody, flow and
    /// cleanup passes to check. The custody that *does* exist — endpoints and
    /// registrations — is checked by the reactor model, which is the model that
    /// defines it.
    Event(Box<CheckedEventModule>),
    /// One entry function checked under `source.string.v1` (card
    /// `STRING-SLICE`): a module holding immutable UTF-8 `{ptr, len}` views of
    /// its own static literal storage. The certificate is the evidence and the
    /// lowered program is retained beside it, so a consumer can re-derive rather
    /// than believe it.
    StandardString(Box<CheckedStringModule>),
    /// One entry function checked under `source.socket.v1` (card
    /// `QUIRIN-SOCKET-V1`): a module holding owned TCP and UDP sockets whose
    /// every peer is another socket of the same module. The certificate is the
    /// evidence and the lowered program is retained beside it, so a consumer can
    /// re-derive rather than believe it.
    StandardSocket(Box<CheckedSocketModule>),
    /// One entry function checked under `source.http.v1` (card
    /// `HTTP-QUIRIN-V1`): a module that declares HTTP/1.1 messages, has their
    /// wire bytes constructed and checked against RFC 9112 at compile time, and
    /// moves them over `source.socket.v1`'s own loopback transport. The
    /// certificate carries the transport's certificate inside it, so the
    /// evidence of the composition is the evidence of both halves.
    StandardHttp(Box<CheckedHttpModule>),
    /// One entry function checked under `source.ffi.v1` (card
    /// `W48-QR-C-STRUCT-IMPORT`): a file that imports C structs and calls C
    /// through `#c_ffi`-marked contracts. The certificate lists every imported
    /// struct's recorded layout and every function called; the published
    /// verdict is never plain `Safe`, because the whole file holds under the
    /// unsafe-FFI assumption.
    Ffi(Box<CheckedFfiModule>),
}

/// A checked `source.ffi.v1` module (card `W48-QR-C-STRUCT-IMPORT`).
#[derive(Clone, Debug)]
pub struct CheckedFfiModule {
    pub lowered: qcore_frontend::source_ffi::LoweredFfiModule,
    pub certificate: qcore_core::ffi::FfiCertificate,
    /// The published verdict: `Safe` under the certificate's named
    /// assumptions, the unsafe-FFI one always among them.
    pub verdict: qcore_schemas::verdict::AssumedVerdict,
}

/// A checked `source.buffer.v1` module (card `STD-BUFFER`).
///
/// Like [`CheckedStdIoModule`] this carries no *sequential* evidence, and the
/// reason is different and worth stating: the entry does bind real resource
/// identities, but their custody is discharged by
/// [`qcore_core::ownership::check_ownership`] over the operation list
/// [`qcore_core::std_buffer::custody_operations`] derives — the same cycle-3
/// checker the sequential resource-flow bundle runs, reached directly rather
/// than through a `FunctionSemantics` synthesis this flat route has no
/// statements to fill. The obligations are therefore discharged, not skipped,
/// and the certificate names every acquisition with its extent so a reader can
/// see what was discharged.
#[derive(Clone, Debug)]
pub struct CheckedBufferModule {
    pub lowered: qcore_frontend::source_std_buffer::LoweredBufferModule,
    pub certificate: std_buffer::BufferCertificate,
    /// Present only for `source.resource-calls.v1`: the retained typed call
    /// graph and its ownership expansion beside the ordinary buffer evidence.
    pub resource_calls: Option<Box<resource_calls::Checked>>,
}

/// A checked `source.string.v1` module (card `STRING-SLICE`).
///
/// Unlike [`CheckedBufferModule`] this carries **no custody evidence at all**,
/// and that is a modelled fact rather than a gap. A string view is not a
/// resource: amendment §4 says slicing "is not ownership splitting", §3.6 says
/// creating a view "neither creates nor discharges" custody, and the only
/// storage v1 can name is a string literal with static storage duration. There
/// is therefore no obligation for the cycle-3 checker to discharge, and running
/// it over an empty operation list would be a ceremony rather than a check.
///
/// What the certificate publishes in its place is **storage provenance**: every
/// view names the literal it addresses and its offset and length within it, so
/// §3.6 — a derived view cannot outlive the storage it came from — is a field a
/// reader checks rather than a sentence they believe. The card that introduces a
/// view over storage that *can* end, `STRING-SLICE-BUFFER-BORROW`, owns the
/// lifetime rule that becomes necessary then.
#[derive(Clone, Debug)]
pub struct CheckedStringModule {
    pub lowered: qcore_frontend::source_std_string::LoweredStringModule,
    pub certificate: std_string::StringCertificate,
}

/// A checked `source.socket.v1` module (card `QUIRIN-SOCKET-V1`).
///
/// Like [`CheckedBufferModule`] this carries no *sequential* evidence, and for
/// the same reason: the entry binds real resource identities, but their custody
/// is discharged by [`qcore_core::ownership::check_ownership`] over the
/// operation list [`qcore_core::std_socket::custody_operations`] derives — the
/// same cycle-3 checker the sequential resource-flow bundle runs, reached
/// directly rather than through a `FunctionSemantics` synthesis this flat route
/// has no statements to fill. The obligations are discharged, not skipped, and
/// the certificate names every acquisition and every close so a reader can see
/// what was discharged.
///
/// Beside that it publishes two things no other route has: the exact bytes every
/// receive takes (which the emitted program **verifies**), and what
/// [`qcore_core::os_endpoint::close_poststate`] answers for each close on both
/// of its branches — which is why the emitted shim never retries a failed one.
#[derive(Clone, Debug)]
pub struct CheckedSocketModule {
    pub lowered: qcore_frontend::source_socket::LoweredSocketModule,
    pub certificate: std_socket::SocketCertificate,
    /// The derived protocol cells and every `disjoint NAME NAME;` judgement,
    /// decided by the **unedited** `DisjointFootprintsV1` checker over the
    /// derived-alias projection (card `CT42-CELL-ROUTE`). Published for every
    /// admitted module, question or none.
    pub cells: qcore_core::socket_footprint::SocketCellCertificate,
}

/// A checked `source.http.v1` module (card `HTTP-QUIRIN-V1`).
///
/// The certificate carries the transport's own certificate, derived by the
/// **unedited** `source.socket.v1` checker over the program this route lowered
/// to, so a consumer reading it sees both halves of the composition rather than
/// a summary of one.
#[derive(Clone, Debug)]
pub struct CheckedHttpModule {
    pub lowered: qcore_frontend::source_http::LoweredHttpModule,
    pub certificate: std_http::HttpCertificate,
    /// The derived protocol cells of the transport program this route lowered
    /// to, and every `disjoint NAME NAME;` judgement over them, decided by the
    /// **unedited** `DisjointFootprintsV1` checker (card `CT42-CELL-ROUTE`).
    /// The HTTP reader hands every statement it does not spell to the socket
    /// reader, so the question reaches this route too and is answered here
    /// rather than admitted unanswered — measured, not assumed.
    pub cells: qcore_core::socket_footprint::SocketCellCertificate,
}

/// A checked `source.event.v1` module (card `STD-EVENT`).
#[derive(Clone, Debug)]
pub struct CheckedEventModule {
    pub lowered: qcore_frontend::source_event::LoweredEventModule,
    pub certificate: std_event::StdEventCertificate,
}

/// A checked `source.std-io.v1` module.
///
/// There is no sequential evidence beside the certificate here, and that is a
/// stated boundary rather than an omission: the route's operand grammar admits
/// no resource, so the entry binds no identity for the custody, flow and
/// cleanup passes to check. If that grammar is ever widened — card
/// `STD-FMT-BORROW` — this module must carry their evidence too.
#[derive(Clone, Debug)]
pub struct CheckedStdIoModule {
    pub lowered: qcore_frontend::source_std_io::LoweredStdIoModule,
    pub certificate: std_io::StdIoCertificate,
}

/// A checked `source.std-io-extern.v1` module (card `STD-IO-COMPOSE`).
///
/// Unlike [`CheckedStdIoModule`], this one **does** carry sequential evidence:
/// its extern half binds real identities, so the custody, flow and cleanup
/// passes have something to check and they run exactly as they do on the extern
/// route.
#[derive(Clone, Debug)]
pub struct CheckedComposedModule {
    pub lowered: qcore_frontend::source_composed::LoweredComposedModule,
    pub certificate: composed_std_io::ComposedCertificate,
    /// The same entry re-checked by every established sequential pass, with the
    /// declared extern summaries supplied as call-site assumptions.
    pub sequential: CheckedProgram,
}

/// A checked `source.extern-call.v1` module.
#[derive(Clone, Debug)]
pub struct CheckedExternModule {
    pub lowered: qcore_frontend::source_extern::LoweredExternModule,
    pub certificate: extern_call::ExternCallCertificate,
    /// The same entry re-checked by every established sequential pass, with the
    /// declared extern summaries supplied as call-site assumptions.
    pub sequential: CheckedProgram,
}

/// A checked `source.mutex.guard.v1` module: the lowered trace and the
/// certificate a caller can independently recheck.
#[derive(Clone, Debug)]
pub struct CheckedMutexGuardModule {
    pub lowered: qcore_frontend::source_mutex::LoweredMutexModule,
    pub certificate: mutex_invariant::MutexInvariantCertificate,
}

/// A checked `source.epoch.bound.v1` module: the lowered flat epoch and the
/// quantitative bound a caller can independently recheck. The certificate's
/// premises are `FlatEpochBoundV1`'s own — declared footprints, declared
/// runtime overhead, disjoint shared cells and no schedule model — and this
/// route discharges none of them.
#[derive(Clone, Debug)]
pub struct CheckedEpochModule {
    pub lowered: qcore_frontend::source_epoch::LoweredEpochModule,
    pub certificate: epoch_accounting::EpochBoundCertificate,
    /// The error path over this epoch's credits, published **beside** the bound
    /// rather than inside it (card `CT23-CREDIT-ERROR-PATH`). On this route
    /// every arm is elided — a declared worker is not a creation the creator
    /// could fail to make — so the split is always "everything spent", which is
    /// exactly the accounting this route had before the card.
    pub credit_recovery: epoch_credit_recovery::EpochCreditRecoveryCertificate,
}

/// A checked `source.epoch.tasks.v1` module: one flat-epoch plan and the real
/// checked task region it accounts for.
///
/// Neither half is evidence for the other. `tasks` was checked by the unedited
/// `check_task_module`, `certificate` by the unedited `check_flat_epoch` over a
/// program whose *shape* — task count, captures, retained results, joins,
/// disposals — was derived from `tasks` and then required to equal what the plan
/// declared. `composed.rows` records that reconciliation row by row, including
/// which disposals were declared and which were derived.
#[derive(Clone, Debug)]
pub struct ComposedEpochModule {
    pub tasks: task_execution::CheckedTaskModule,
    pub composed: qcore_frontend::source_epoch_tasks::LoweredComposedEpoch,
    pub certificate: epoch_accounting::EpochBoundCertificate,
    /// The error path over this epoch's credits (card `CT23-CREDIT-ERROR-PATH`):
    /// for every creation, how much of its grant the creator's own error edges
    /// return to the epoch and how much its failure handling spends. Published
    /// beside the bound, never inside it — the bound is unchanged by it.
    pub credit_recovery: epoch_credit_recovery::EpochCreditRecoveryCertificate,
    /// The parent's own handled cleanup over the unedited error-region model
    /// (cards `REGION-CREDIT-JOINS`, `REGION-JOINS-DUTIES` and
    /// `CT24-CLEANUP-TASK-COMPOSITION`). For an epilogue, this describes the
    /// single path after every join. For a cleanup in a spawn-success arm, it
    /// is conditional evidence for that arm: the spawn-failure arm does not
    /// execute this cleanup and keeps its own task-checked recovery. `None`
    /// for every entry that writes no region.
    pub region: Option<ComposedRegionEvidence>,
    /// The scoped-task profile the task half executes under, kept because the
    /// composed label would otherwise hide whether the region moves owned
    /// Buffers or only copies scalars.
    pub task_profile: SourceExecutionProfile,
}

/// The published evidence for one composed region (cards
/// `REGION-CREDIT-JOINS` and `CT24-CLEANUP-TASK-COMPOSITION`).
///
/// Every field is re-derived rather than believed: `operations` is what the
/// unedited model was handed and admitted, and `abandoned_credit` is read out
/// of the model's own cleanup trace **and** cross-checked against the reader's
/// independent fold of the body's `credit`/`effort` quantities before either is
/// published. When the region is in a spawn-success arm these fields describe
/// that reached arm only; they are not a claim that the cleanup ran on the
/// spawn-failure arm.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComposedRegionEvidence {
    /// The statement's extent in the original file — the erased range.
    pub span: Span,
    /// Exactly one `RegionOp::ErrorRegion`, built from the model's own
    /// constructors and admitted by `analyze_error_regions`.
    pub operations: Vec<RegionOp>,
    /// The parent-local unused credit the discharge abandoned — the sealed
    /// pack's remainder, which the opening may discard because a credit is an
    /// accounting quantity and not a live resource.
    pub abandoned_credit: u64,
    /// The owners the discharge destroyed — the pack's duties, by binding
    /// name, in destruction order (reverse acquisition order). Read out of the
    /// model's own cleanup trace and cross-checked against the reader's
    /// independent duty fold before publication, exactly as the credit is
    /// (card `REGION-JOINS-DUTIES`). Empty for a duty-free region, which is
    /// wave 28's evidence unchanged.
    pub released: Vec<String>,
}

impl CheckedSourceModule {
    pub fn as_sequential(&self) -> Option<&CheckedProgram> {
        match self {
            Self::ScalarCalls(_) => None,
            Self::Sequential(program) => Some(program),
            Self::ScopedTasks(_)
            | Self::BitfieldTasks(_)
            | Self::CheckedMutexGuards(_)
            | Self::CheckedEpochBound(_)
            | Self::ComposedEpochTasks(_)
            | Self::ExternCalls(_)
            | Self::StandardIo(_)
            | Self::Composed(_)
            | Self::ComposedTasks(_)
            | Self::StandardBuffer(_)
            | Self::StandardString(_)
            | Self::Event(_)
            | Self::StandardSocket(_)
            | Self::StandardHttp(_)
            | Self::Ffi(_) => None,
        }
    }

    /// The scoped-task evidence of a module whose *whole* checked evidence is
    /// that task module. A composed epoch answers `None` here on purpose: its
    /// task evidence is one half of a pair, and a consumer that wants it must
    /// ask for the composition and see the accounting half beside it.
    pub fn as_scoped_tasks(&self) -> Option<&task_execution::CheckedTaskModule> {
        match self {
            Self::ScalarCalls(_)
            | Self::Sequential(_)
            | Self::BitfieldTasks(_)
            | Self::CheckedMutexGuards(_)
            | Self::CheckedEpochBound(_)
            | Self::ComposedEpochTasks(_)
            | Self::ExternCalls(_)
            | Self::StandardIo(_)
            | Self::Composed(_)
            | Self::ComposedTasks(_)
            | Self::StandardBuffer(_)
            | Self::StandardString(_)
            | Self::Event(_)
            | Self::StandardSocket(_)
            | Self::StandardHttp(_)
            | Self::Ffi(_) => None,
            Self::ScopedTasks(program) => Some(program),
        }
    }

    pub fn as_checked_mutex_guards(&self) -> Option<&CheckedMutexGuardModule> {
        match self {
            Self::ScalarCalls(_)
            | Self::Sequential(_)
            | Self::ScopedTasks(_)
            | Self::BitfieldTasks(_)
            | Self::CheckedEpochBound(_)
            | Self::ComposedEpochTasks(_)
            | Self::ExternCalls(_)
            | Self::StandardIo(_)
            | Self::Composed(_)
            | Self::ComposedTasks(_)
            | Self::StandardBuffer(_)
            | Self::StandardString(_)
            | Self::Event(_)
            | Self::StandardSocket(_)
            | Self::StandardHttp(_)
            | Self::Ffi(_) => None,
            Self::CheckedMutexGuards(module) => Some(module),
        }
    }

    /// The certificate of a module whose whole checked evidence is the flat
    /// epoch. A composed epoch answers `None` here for the same reason
    /// `as_scoped_tasks` does.
    pub fn as_checked_epoch_bound(&self) -> Option<&CheckedEpochModule> {
        match self {
            Self::ScalarCalls(_)
            | Self::Sequential(_)
            | Self::ScopedTasks(_)
            | Self::BitfieldTasks(_)
            | Self::CheckedMutexGuards(_)
            | Self::ComposedEpochTasks(_)
            | Self::ExternCalls(_)
            | Self::StandardIo(_)
            | Self::Composed(_)
            | Self::ComposedTasks(_)
            | Self::StandardBuffer(_)
            | Self::StandardString(_)
            | Self::Event(_)
            | Self::StandardSocket(_)
            | Self::StandardHttp(_)
            | Self::Ffi(_) => None,
            Self::CheckedEpochBound(module) => Some(module),
        }
    }

    pub fn as_composed_epoch_tasks(&self) -> Option<&ComposedEpochModule> {
        match self {
            Self::ScalarCalls(_)
            | Self::Sequential(_)
            | Self::ScopedTasks(_)
            | Self::BitfieldTasks(_)
            | Self::CheckedMutexGuards(_)
            | Self::CheckedEpochBound(_)
            | Self::ExternCalls(_)
            | Self::StandardIo(_)
            | Self::Composed(_)
            | Self::ComposedTasks(_)
            | Self::StandardBuffer(_)
            | Self::StandardString(_)
            | Self::Event(_)
            | Self::StandardSocket(_)
            | Self::StandardHttp(_)
            | Self::Ffi(_) => None,
            Self::ComposedEpochTasks(module) => Some(module),
        }
    }

    pub fn as_extern_calls(&self) -> Option<&CheckedExternModule> {
        match self {
            Self::ScalarCalls(_)
            | Self::Sequential(_)
            | Self::ScopedTasks(_)
            | Self::BitfieldTasks(_)
            | Self::CheckedMutexGuards(_)
            | Self::CheckedEpochBound(_)
            | Self::ComposedEpochTasks(_)
            | Self::StandardIo(_)
            | Self::Composed(_)
            | Self::ComposedTasks(_)
            | Self::StandardBuffer(_)
            | Self::StandardString(_)
            | Self::Event(_)
            | Self::StandardSocket(_)
            | Self::StandardHttp(_)
            | Self::Ffi(_) => None,
            Self::ExternCalls(module) => Some(module),
        }
    }

    pub fn as_standard_io(&self) -> Option<&CheckedStdIoModule> {
        match self {
            Self::ScalarCalls(_)
            | Self::Sequential(_)
            | Self::ScopedTasks(_)
            | Self::BitfieldTasks(_)
            | Self::CheckedMutexGuards(_)
            | Self::CheckedEpochBound(_)
            | Self::ComposedEpochTasks(_)
            | Self::ExternCalls(_)
            | Self::Composed(_)
            | Self::ComposedTasks(_)
            | Self::StandardBuffer(_)
            | Self::StandardString(_)
            | Self::Event(_)
            | Self::StandardSocket(_)
            | Self::StandardHttp(_)
            | Self::Ffi(_) => None,
            Self::StandardIo(module) => Some(module),
        }
    }

    /// The composed evidence of a module whose *whole* checked evidence is the
    /// composition. A composed module answers `None` to `as_extern_calls` and
    /// `as_standard_io` on the `ComposedEpochTasks` precedent: each of its
    /// certificates is one half of a checked pair, and a consumer that wants
    /// one must ask for the composition and see the other beside it.
    pub fn as_composed(&self) -> Option<&CheckedComposedModule> {
        match self {
            Self::ScalarCalls(_)
            | Self::Sequential(_)
            | Self::ScopedTasks(_)
            | Self::BitfieldTasks(_)
            | Self::CheckedMutexGuards(_)
            | Self::CheckedEpochBound(_)
            | Self::ComposedEpochTasks(_)
            | Self::ExternCalls(_)
            | Self::StandardIo(_)
            | Self::ComposedTasks(_)
            | Self::StandardBuffer(_)
            | Self::StandardString(_)
            | Self::Event(_)
            | Self::StandardSocket(_)
            | Self::StandardHttp(_)
            | Self::Ffi(_) => None,
            Self::Composed(module) => Some(module),
        }
    }

    /// The composed task evidence (card `COMPOSE-CONCURRENCY`).
    ///
    /// A composed task module answers `None` to `as_scoped_tasks` on the same
    /// `ComposedEpochTasks` precedent: its task half is one half of a checked
    /// pair, and a consumer that wants it asks for the composition and takes
    /// `tasks()` from it, with the placed lines in view beside it.
    pub fn as_composed_tasks(&self) -> Option<&composed_tasks::CheckedComposedTaskModule> {
        match self {
            Self::ScalarCalls(_)
            | Self::Sequential(_)
            | Self::ScopedTasks(_)
            | Self::BitfieldTasks(_)
            | Self::CheckedMutexGuards(_)
            | Self::CheckedEpochBound(_)
            | Self::ComposedEpochTasks(_)
            | Self::ExternCalls(_)
            | Self::StandardIo(_)
            | Self::Composed(_)
            | Self::StandardBuffer(_)
            | Self::StandardString(_)
            | Self::Event(_)
            | Self::StandardSocket(_)
            | Self::StandardHttp(_)
            | Self::Ffi(_) => None,
            Self::ComposedTasks(module) => Some(module),
        }
    }

    /// The checked byte-buffer evidence (card `STD-BUFFER`).
    pub fn as_standard_buffer(&self) -> Option<&CheckedBufferModule> {
        match self {
            Self::ScalarCalls(_)
            | Self::Sequential(_)
            | Self::ScopedTasks(_)
            | Self::BitfieldTasks(_)
            | Self::CheckedMutexGuards(_)
            | Self::CheckedEpochBound(_)
            | Self::ComposedEpochTasks(_)
            | Self::ExternCalls(_)
            | Self::StandardIo(_)
            | Self::Composed(_)
            | Self::ComposedTasks(_)
            | Self::StandardString(_)
            | Self::Event(_)
            | Self::StandardSocket(_)
            | Self::StandardHttp(_)
            | Self::Ffi(_) => None,
            Self::StandardBuffer(module) => Some(module),
        }
    }

    pub fn as_resource_calls(&self) -> Option<&resource_calls::Checked> {
        self.as_standard_buffer()
            .and_then(|module| module.resource_calls.as_deref())
    }

    /// The checked string-view evidence (card `STRING-SLICE`).
    pub fn as_standard_string(&self) -> Option<&CheckedStringModule> {
        match self {
            Self::ScalarCalls(_)
            | Self::Sequential(_)
            | Self::ScopedTasks(_)
            | Self::BitfieldTasks(_)
            | Self::CheckedMutexGuards(_)
            | Self::CheckedEpochBound(_)
            | Self::ComposedEpochTasks(_)
            | Self::ExternCalls(_)
            | Self::StandardIo(_)
            | Self::Composed(_)
            | Self::ComposedTasks(_)
            | Self::StandardBuffer(_)
            | Self::Event(_)
            | Self::StandardSocket(_)
            | Self::StandardHttp(_)
            | Self::Ffi(_) => None,
            Self::StandardString(module) => Some(module),
        }
    }

    /// The checked socket evidence (card `QUIRIN-SOCKET-V1`).
    pub fn as_standard_socket(&self) -> Option<&CheckedSocketModule> {
        match self {
            Self::ScalarCalls(_)
            | Self::Sequential(_)
            | Self::ScopedTasks(_)
            | Self::BitfieldTasks(_)
            | Self::CheckedMutexGuards(_)
            | Self::CheckedEpochBound(_)
            | Self::ComposedEpochTasks(_)
            | Self::ExternCalls(_)
            | Self::StandardIo(_)
            | Self::Composed(_)
            | Self::ComposedTasks(_)
            | Self::StandardBuffer(_)
            | Self::StandardString(_)
            | Self::Event(_)
            | Self::StandardHttp(_)
            | Self::Ffi(_) => None,
            Self::StandardSocket(module) => Some(module),
        }
    }

    /// The checked HTTP evidence (card `HTTP-QUIRIN-V1`).
    pub fn as_standard_http(&self) -> Option<&CheckedHttpModule> {
        match self {
            Self::ScalarCalls(_)
            | Self::Sequential(_)
            | Self::ScopedTasks(_)
            | Self::BitfieldTasks(_)
            | Self::CheckedMutexGuards(_)
            | Self::CheckedEpochBound(_)
            | Self::ComposedEpochTasks(_)
            | Self::ExternCalls(_)
            | Self::StandardIo(_)
            | Self::Composed(_)
            | Self::ComposedTasks(_)
            | Self::StandardBuffer(_)
            | Self::StandardString(_)
            | Self::StandardSocket(_)
            | Self::Event(_)
            | Self::Ffi(_) => None,
            Self::StandardHttp(module) => Some(module),
        }
    }

    pub fn function_count(&self) -> usize {
        match self {
            Self::ScalarCalls(program) => program.program().functions.len(),
            Self::Sequential(program) => program.functions.len(),
            Self::ScopedTasks(program) => program.module().functions.len(),
            // This route checks one footprint property over retained writes;
            // it does not run the function/task checker, so counting the entry
            // or workers here would turn a property certificate into function
            // coverage.
            Self::BitfieldTasks(_) => 0,
            // One entry function is checked on this route; the profile admits
            // no calls, so the count is not a coverage claim about a module.
            Self::CheckedMutexGuards(_) => 1,
            // Likewise: the extern-call profile lowers exactly one entry, and
            // the declared callees have no checked body to count.
            Self::ExternCalls(_) => 1,
            // And likewise for the flat epoch: one entry declares the whole
            // epoch, and its workers are declared shapes, not checked bodies.
            Self::CheckedEpochBound(_) => 1,
            // A composed epoch's workers *are* checked bodies, so the count is
            // the task module's, exactly as on the scoped-task route.
            Self::ComposedEpochTasks(module) => module.tasks.module().functions.len(),
            // And likewise for standard IO: one entry is lowered, and the
            // profile admits no calls at all.
            Self::StandardIo(_) => 1,
            // And likewise for the composition: one entry is lowered, by two
            // models rather than one.
            Self::Composed(_) => 1,
            // A composed task module's workers *are* checked bodies, so the
            // count is the task module's, exactly as on the scoped-task route.
            Self::ComposedTasks(module) => module.tasks().module().functions.len(),
            // And likewise for the byte buffer: one flat entry is lowered, and
            // the profile admits no calls at all.
            Self::StandardBuffer(module) => module
                .resource_calls
                .as_ref()
                .map_or(1, |checked| checked.function_count()),
            // And likewise for the string view: one flat entry is lowered, and
            // the profile admits no calls at all.
            Self::StandardString(_) => 1,
            // And likewise for the event route: one entry is lowered, and the
            // profile admits no calls at all.
            Self::Event(_) => 1,
            // And likewise for the socket route: one flat entry is lowered, and
            // the profile admits no calls at all.
            Self::StandardSocket(_) => 1,
            // And likewise for the HTTP route, which composes that one with a
            // message vocabulary in the same single entry.
            Self::StandardHttp(_) => 1,
            // And likewise for the FFI route: one flat entry is lowered, and
            // its callees are C with no checked body to count.
            Self::Ffi(_) => 1,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceExecutionProfile {
    /// I32/unit functions with checked arithmetic, direct calls and explicit std.io.
    ScalarCallsV1,
    /// A closed acyclic source call graph over extent-carrying `bytes` values,
    /// with owned moves and whole-buffer read/write loans checked through the
    /// existing ownership IR and byte-buffer checker.
    ResourceCallsV1,
    SequentialCleanupV1,
    ScopedCopyTasksV1,
    ScopedOwnedTasksV1,
    /// Property-only check of field-write independence on the successful task
    /// window. The unedited `DisjointFootprintsV1` checker decides the actual
    /// spawn captures; adjacent bit-fields sharing a storage unit refuse as
    /// `Q661`. This profile proves no spawn/join failure-path custody and emits
    /// no C.
    BitfieldConcurrencyV1,
    /// `ScopedReadLoansV1`: the owned-task profile plus one bounded scoped read
    /// loan of the whole read authority — never copied, moved, re-lent,
    /// returned or dropped, and closed only by the creator's successful join.
    ScopedReadLoansV1,
    /// `ScopedReadSharesV1`: the read-loan profile with counted shares, so one
    /// allocation may be lent to two borrowers with positive complementary
    /// shares, plus the Buffer write that requires reconstructed exclusive
    /// authority. Selected when a capture lends other than the whole authority,
    /// or when the module writes a Buffer.
    ScopedReadSharesV1,
    /// `ScopedErrorPayloadV1`: the owned-task profile plus one explicitly
    /// tagged error payload carrying exactly one owned Buffer out of a child
    /// through its completion and the creator's join.
    ScopedErrorPayloadV1,
    /// `ScopedWriteLeasesV1` (card `WRITE-LEND-TASKS`): the owned-task
    /// profile plus range write leases — `lend PLACE LOW..HIGH` captures
    /// handing a worker write authority over its own byte range of one
    /// payload for spawn..join, ranges pairwise disjoint by the wave-29
    /// overlap decision, the owner holding no whole-payload authority (read
    /// included) while any lease is out. A strictly stronger claim than owned
    /// transfer, so it names its own profile. In this first slice the holder
    /// exercises nothing: the worker-side range write is the named residual
    /// `WRITE-LEND-TASKS-EXERCISE`.
    ScopedWriteLeasesV1,
    /// `source.mutex.guard.v1`: a lexical guard over one fixed-shape
    /// `CheckedMutexInvariantV1` mutex generation, on one Linux thread.
    CheckedMutexGuardsV1,
    /// `source.extern-call.v1`: a flat sequential entry that calls declared
    /// extern functions through explicitly selected, admitted contracts.
    ExternCallsV1,
    /// `source.epoch.bound.v1`: one flat epoch of independent declared workers
    /// over fixed-shape shared storage, checked by `FlatEpochBoundV1`. A
    /// checked accounting statement about a declared shape: this route has no
    /// reference execution and no C emission.
    FlatEpochBoundV1,
    /// `source.epoch.tasks.v1`: a flat-epoch plan composed with a real checked
    /// task region. The task half executes exactly as it does on its own route —
    /// same lowering, same checker, same emission — and the epoch stays
    /// check-only on top of it.
    ComposedEpochTasksV1,
    /// `source.std-io.v1`: a flat sequential entry that renders scalars and
    /// writes them as lines, behind the `std.fmt`, `std.io` and `std.logger`
    /// bundled modules (card `STD-MODULES`). The route's operand grammar admits
    /// no resource, so its derived effect row is exactly `{Io}`.
    StandardIoV1,
    /// `source.buffer.v1`: a flat sequential entry holding an owned,
    /// extent-carrying byte buffer behind the `std.buffer` bundled module (card
    /// `STD-BUFFER`). Unlike `StandardIoV1` this route's grammar **does** admit
    /// a resource, so its derived effect row carries `Alloc` and `Mut`. The
    /// extern route derives those two as well, but from a **declared
    /// contract**'s projections — `Alloc` when the contract says the callee
    /// acquires — whereas here they follow from an acquisition and a store this
    /// route checks and discharges itself. Every index is a
    /// check-time constant, every custody refusal is
    /// `qcore_core::ownership`'s, and the emitted C is self-contained
    /// (`calloc`/`free` plus two bounds-checked accessors) with no boundary of
    /// any kind.
    StandardBufferV1,
    /// `source.string.v1`: a flat sequential entry holding immutable UTF-8
    /// `{ptr, len}` **string views** of its own static literal storage, behind
    /// the `std.string` bundled module (card `STRING-SLICE`, normative statement
    /// `docs/design/STRING_SLICE_V1.md`).
    ///
    /// The opposite of `StandardBufferV1` in exactly the way amendment §5 says:
    /// that route's value is owned, mutable and explicitly released, and this
    /// route's is immutable, unowned and never released. Its derived effect row
    /// therefore carries **neither `Alloc` nor `Mut`** — literal construction,
    /// `len` and a successful slice have no effect at all (§7) — and `{Io}` only
    /// where the entry also writes a line. Every slice bound is a check-time
    /// constant, so `0 <= low <= high <= len` and both UTF-8 code-point
    /// boundaries are decided before emission and the route needs no run-time
    /// failure arm. The emitted C is self-contained, allocates nothing, copies
    /// nothing and calls `strlen` nowhere.
    StandardStringV1,
    /// `source.std-io-extern.v1`: the checked composition of
    /// `source.extern-call.v1` and `source.std-io.v1` in one entry (card
    /// `STD-IO-COMPOSE`). Both models check their own half unmodified and the
    /// composition checks only where they meet, so this profile's guarantees
    /// are exactly the conjunction of the two it composes plus the join.
    ComposedStdIoExternV1,
    /// `source.std-io-tasks.v1`: the checked composition of `source.std-io.v1`
    /// with the checked task route in one module (card `COMPOSE-CONCURRENCY`).
    /// The task half executes exactly as it does on its own route — same
    /// lowering, same checker, same emission — and the source lines stand in the
    /// **parent's** body, where the interleaving with the synthesized trace is
    /// determined because every write of an emitted task program is performed by
    /// the parent's thread. A line in a worker body is refused.
    ComposedStdIoTasksV1,
    /// `source.event.v1`: a flat sequential entry that opens one event context,
    /// registers interest on descriptors it created itself, dispatches declared
    /// turns and tears the context down (card `STD-EVENT`).
    ///
    /// The custody half is the **existing** `event_reactor`/`os_endpoint`
    /// models, which this route made reachable from source for the first time;
    /// the route's own contribution is the determinism decision that gives each
    /// dispatch turn one well-defined observation
    /// (`docs/design/STD_EVENT_V1.md` §3). A module containing a `write`
    /// interest is accepted and checked but **not executed**: it publishes no
    /// C and no reference observation.
    EventV1,
    /// `source.socket.v1`: a flat sequential entry that creates owned TCP and
    /// UDP sockets, binds them to the loopback interface on kernel-chosen ports,
    /// connects and accepts **within the same module**, exchanges declared byte
    /// sequences and closes them (card `QUIRIN-SOCKET-V1`).
    ///
    /// Its derived effect row carries `Alloc` and `Io` — never `Mut`, because the
    /// route mutates no object of the program. Every custody refusal is
    /// `qcore_core::ownership`'s, unedited, exactly as on `StandardBufferV1`.
    /// What is new is a **determinism** decision no other route makes: the model
    /// carries the bytes every send commits, so a receive the program's own
    /// history does not satisfy is refused at check time rather than blocking,
    /// and the bytes each receive takes are published and **verified by the
    /// emitted program**. The emitted C is self-contained on all three platform
    /// arms; it links no library and names Winsock from inside the emitted file.
    StandardSocketV1,
    /// `source.http.v1`: a flat sequential entry that declares HTTP/1.1 messages
    /// and moves them over `source.socket.v1`'s loopback transport (card
    /// `HTTP-QUIRIN-V1`).
    ///
    /// **An executed HTTP/1.1 subset with the boundary named**, never "HTTP/1.1
    /// conformance". Every component of every message is a literal, so a family
    /// of RFC 9112 facts is decided *statically* and by name: the version is
    /// exactly `HTTP/1.1`, a method and a field name are `token`s, a target is
    /// origin-form, a request carries exactly one `Host`, no component may carry
    /// `CR` or `LF` (which is what makes the constructed framing unforgeable),
    /// and a declared `Content-Length` **must equal the literal content's byte
    /// count**. The transport half is the socket route's, unedited, so the
    /// determinism argument, the custody discipline and the byte-compared
    /// observation are all inherited rather than restated. What is *not* here:
    /// no computed text, no parsing of unknown input, no chunked coding, no
    /// persistent connection, no external peer, no TLS.
    StandardHttpV1,
    /// `source.ffi.v1` (card `W48-QR-C-STRUCT-IMPORT`): imported C structs,
    /// construct-then-read over their listed fields, calls through
    /// `#c_ffi`-marked contracts, and rendered lines — every result under the
    /// file-wide unsafe-FFI assumption.
    FfiV1,
}

#[derive(Clone, Debug)]
pub struct CompiledModule {
    /// Original root/dependency snapshots and resolved edges; never staged text.
    pub source_bundle: Option<SourceBundleEvidence>,
    /// Structured naming notices; never safety diagnostics or verdict inputs.
    pub extension_warnings: Vec<qcore_frontend::source_extension::ExtensionWarning>,
    pub checked: CheckedSourceModule,
    pub execution_profile: SourceExecutionProfile,
    pub declaration_count: usize,
    pub mode: SourceModuleMode,
    pub result_profile: SourceResultProfile,
    pub entry: Option<String>,
    pub c_source: Option<String>,
    pub source_map: Vec<EmittedSourceMap>,
    pub reference_observation: Option<ExecutionObservation>,
    /// Reference execution of a `source.extern-call.v1` trace, computed from
    /// the checked event list independently of the emitted C. Kept separate
    /// from `reference_observation`, which carries the sequential cleanup
    /// observation and models no extern call.
    pub extern_observation: Option<extern_c_emit::ExternObservation>,
}

impl CompiledModule {
    /// The declared premises this module's result holds relative to, if its
    /// route publishes any (card `UNSAFE-LINKAGE`).
    ///
    /// A consumer that does not know which route compiled a module still sees
    /// its premises here, so a `Safe` obtained through a trusted declared
    /// contract or through unchecked linkage is never handed on as a bare
    /// `Safe`. Routes with no declared premise return the empty slice, which is
    /// the honest answer for them and not a placeholder.
    #[must_use]
    /// The recorded target a `source.scalar-calls.v1` result depends on (card
    /// `W48-QR-SCALAR-TYPES`): present exactly when the program names `isize`
    /// or `usize`, whose width only the builder's record supplies. It is a
    /// named assumption of the result, printed beside the declared ones.
    pub fn target_assumption(&self) -> Option<&RecordedTarget> {
        match &self.checked {
            CheckedSourceModule::ScalarCalls(checked) => checked.target_assumption(),
            // Card `W48-QR-SCALAR-TYPES-R3`: a buffer-route program naming
            // `isize` or `usize` (a field type or a conversion) depends on it.
            CheckedSourceModule::StandardBuffer(module) => {
                module.certificate.target_assumption.as_ref()
            }
            // The extern route's value statements and declared outcomes, alone
            // or composed with rendered lines.
            CheckedSourceModule::ExternCalls(module) => {
                module.certificate.target_assumption.as_ref()
            }
            CheckedSourceModule::Composed(module) => {
                module.certificate.externs.target_assumption.as_ref()
            }
            // Card `W48-QR-SCALAR-TYPES-R4`: a task module naming `isize` or
            // `usize`, alone or composed with rendered lines.
            CheckedSourceModule::ScopedTasks(checked) => checked.target_assumption(),
            CheckedSourceModule::ComposedTasks(module) => module.tasks().target_assumption(),
            // Card `W48-QR-SCALAR-TYPES-R6`: the composed string/buffer route
            // (`W48-QR-TYPES-COMPOSED-STRING`).
            CheckedSourceModule::StandardString(module) => {
                module.certificate.target_assumption.as_ref()
            }
            // Card `W48-QR-SCALAR-TYPES-R6`: the composed epoch route's task
            // half (`W48-QR-TYPES-EPOCH-TASKS`), and a `protected isize`/`usize`
            // payload.
            CheckedSourceModule::ComposedEpochTasks(module) => module.tasks.target_assumption(),
            CheckedSourceModule::CheckedMutexGuards(module) => {
                module.lowered.target_assumption.as_ref()
            }
            _ => None,
        }
    }

    /// Card `W48-QR-FLOATING-POINT`: the IEEE premise a result holding an
    /// `f32`/`f64` value carries ([`qcore_core::scalar_floats::ASSUMPTION`]),
    /// a named assumption printed beside the declared ones; `None` for a
    /// module holding no float, so no existing output moves.
    #[must_use]
    pub fn float_assumption(&self) -> Option<&'static str> {
        let uses = match &self.checked {
            CheckedSourceModule::ScalarCalls(checked) => checked.uses_floats(),
            CheckedSourceModule::ExternCalls(module) => {
                qcore_core::extern_c_emit::uses_floats(&module.lowered.program)
            }
            CheckedSourceModule::Composed(module) => {
                qcore_core::extern_c_emit::uses_floats(&module.lowered.program.externs)
            }
            CheckedSourceModule::Ffi(module) => {
                qcore_core::ffi_c_emit::uses_floats(&module.lowered.program)
            }
            CheckedSourceModule::ScopedTasks(checked) => {
                qcore_core::task_execution::fixed_types(checked.module())
                    .iter()
                    .any(|ty| ty.is_float())
            }
            CheckedSourceModule::ComposedTasks(module) => {
                qcore_core::task_execution::fixed_types(module.tasks().module())
                    .iter()
                    .any(|ty| ty.is_float())
            }
            CheckedSourceModule::ComposedEpochTasks(module) => {
                qcore_core::task_execution::fixed_types(module.tasks.module())
                    .iter()
                    .any(|ty| ty.is_float())
            }
            // Card `W48-QR-FLOAT-BUFFER-PAYLOADS`: the buffer route, the
            // composed string route and a `protected f32`/`f64` payload.
            CheckedSourceModule::StandardBuffer(module) => {
                std_buffer::uses_floats(&module.lowered.program)
            }
            CheckedSourceModule::StandardString(module) => {
                std_string::uses_floats(&module.lowered.program)
            }
            CheckedSourceModule::CheckedMutexGuards(module) => {
                module.lowered.payloads().values().any(|ty| ty.is_float())
            }
            _ => false,
        };
        uses.then_some(qcore_core::scalar_floats::ASSUMPTION)
    }

    /// Card `W48-QR-C-STRUCT-IMPORT`: the premises a `source.ffi.v1` result
    /// holds relative to — the unsafe-FFI trust mark first — and the empty
    /// slice on every other route.
    #[must_use]
    pub fn ffi_assumptions(&self) -> &[qcore_core::ffi::FfiAssumption] {
        match &self.checked {
            CheckedSourceModule::Ffi(module) => &module.certificate.assumptions,
            _ => &[],
        }
    }

    /// Card `W48-QR-C-STRUCT-IMPORT`: the verdict a route publishes with its
    /// named assumptions, where it publishes one. Only `source.ffi.v1` does
    /// today, and its verdict is never plain `Safe`.
    #[must_use]
    pub fn published_verdict(&self) -> Option<&qcore_schemas::verdict::AssumedVerdict> {
        match &self.checked {
            CheckedSourceModule::Ffi(module) => Some(&module.verdict),
            _ => None,
        }
    }

    pub fn declared_assumptions(&self) -> &[extern_call::ExternAssumption] {
        match &self.checked {
            CheckedSourceModule::ExternCalls(module) => &module.certificate.assumptions,
            // Card `STD-IO-COMPOSE`: composing must not lose the surface at
            // which a reader sees an unchecked-linkage waiver. The composed
            // certificate carries the extern projection's own premise list
            // verbatim, so this is the same list the extern route publishes.
            CheckedSourceModule::Composed(module) => &module.certificate.externs.assumptions,
            _ => &[],
        }
    }
}

#[derive(Debug)]
struct DriverError(String);

impl fmt::Display for DriverError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for DriverError {}

#[derive(Clone, Debug)]
struct FileRange {
    path: String,
    start: usize,
    end: usize,
    source: String,
}

/// Compiles with an explicit verifier registry. Source-level `verify scope`
/// declarations are resolved (`Q700` on failure) and then dispatched: the core
/// derives an obligation inventory, requests witnesses, and rechecks them
/// behind the firewall. A scope whose obligations are not all discharged by
/// core-checked evidence is `Q701`; conflicting provider evidence is `Q702`
/// and an unvalidated provider allegation is `Q703`. Use
/// `VerifierRegistry::resolve_source_scopes` for configuration validation
/// alone.
/// Collect naming notices separately from error diagnostics, including for a
/// source whose body will later be refused. Paths are retained as spelled.
#[must_use]
pub fn input_extension_warnings(
    files: &[SourceFile],
    options: &CompileOptions,
) -> Vec<qcore_frontend::source_extension::ExtensionWarning> {
    use qcore_frontend::source_extension::{
        ExtensionWarning, legacy_manifest_warning, legacy_source_warning,
    };
    let mut warnings = Vec::new();
    for file in files {
        match qcore_frontend::frontend::parse_source_module_mode(&file.source) {
            Ok(mode) => warnings.extend(legacy_source_warning(&file.path, mode)),
            Err(_) if file.path.ends_with(".qcore") => warnings.push(ExtensionWarning {
                kind: "legacy-source-extension",
                path: file.path.clone(),
                mode: "unresolved".into(),
                recommended_extension: ".qv (contract) or .qr (runtime)",
            }),
            Err(_) => {}
        }
    }
    if let Some(manifests) = &options.native_libraries {
        for path in manifests.keys() {
            warnings.extend(legacy_manifest_warning(path));
        }
    }
    warnings
}

pub fn compile_files_with_registry(
    files: &[SourceFile],
    options: &CompileOptions,
    registry: &VerifierRegistry,
) -> Result<CompiledModule, Vec<FileDiagnostic>> {
    compile_files_with_registry_for_target(files, options, registry, None)
}

/// [`compile_files_with_registry`] for a builder's recorded target (card
/// `W48-QR-SCALAR-TYPES`). The target supplies the width of `isize`/`usize`;
/// without one a program naming either refuses, and with one the width is a
/// named assumption of the result ([`CompiledModule::target_assumption`]).
/// A program that names neither is unaffected by the target.
pub fn compile_files_with_registry_for_target(
    files: &[SourceFile],
    options: &CompileOptions,
    registry: &VerifierRegistry,
    target: Option<&RecordedTarget>,
) -> Result<CompiledModule, Vec<FileDiagnostic>> {
    if files.is_empty() {
        return Err(vec![plain_diagnostic(
            "Q400",
            "no input files were provided",
        )]);
    }
    let roots = files
        .iter()
        .map(|file| file.path.clone())
        .collect::<Vec<_>>();
    compile_source_bundle_for_target(
        &roots,
        files,
        SourceLoading::SnapshotsOnly,
        &ImportLimits::default(),
        options,
        registry,
        target,
    )
}

/// Explicit filesystem variant. Supplied snapshots take precedence; imports
/// absent from them are loaded once. The ordinary API never reads disk.
pub fn compile_files_with_filesystem_with_registry(
    files: &[SourceFile],
    options: &CompileOptions,
    registry: &VerifierRegistry,
) -> Result<CompiledModule, Vec<FileDiagnostic>> {
    compile_files_with_filesystem_with_registry_for_target(files, options, registry, None)
}

/// [`compile_files_with_filesystem_with_registry`] for a builder's recorded
/// target; see [`compile_files_with_registry_for_target`].
pub fn compile_files_with_filesystem_with_registry_for_target(
    files: &[SourceFile],
    options: &CompileOptions,
    registry: &VerifierRegistry,
    target: Option<&RecordedTarget>,
) -> Result<CompiledModule, Vec<FileDiagnostic>> {
    if files.is_empty() {
        return Err(vec![plain_diagnostic(
            "Q400",
            "no input files were provided",
        )]);
    }
    let roots = files
        .iter()
        .map(|file| file.path.clone())
        .collect::<Vec<_>>();
    compile_source_bundle_for_target(
        &roots,
        files,
        SourceLoading::FileSystem,
        &ImportLimits::default(),
        options,
        registry,
        target,
    )
}

/// Compile explicit roots from a supplied snapshot pool, optionally loading
/// missing runtime dependencies. Unreachable pool entries are not compilation
/// roots. Existing compile_files APIs continue treating every input as a root.
pub fn compile_source_bundle_with_registry(
    roots: &[String],
    snapshots: &[SourceFile],
    loading: SourceLoading,
    limits: &ImportLimits,
    options: &CompileOptions,
    registry: &VerifierRegistry,
) -> Result<CompiledModule, Vec<FileDiagnostic>> {
    compile_source_bundle_for_target(roots, snapshots, loading, limits, options, registry, None)
}

fn compile_source_bundle_for_target(
    roots: &[String],
    snapshots: &[SourceFile],
    loading: SourceLoading,
    limits: &ImportLimits,
    options: &CompileOptions,
    registry: &VerifierRegistry,
    target: Option<&RecordedTarget>,
) -> Result<CompiledModule, Vec<FileDiagnostic>> {
    // Naming warnings accompany even a parser/resolver refusal. Preserve the
    // extension compatibility route's pre-validation warning behavior.
    let root_files = snapshots
        .iter()
        .filter(|file| roots.contains(&file.path))
        .cloned()
        .collect::<Vec<_>>();
    let root_warnings = input_extension_warnings(&root_files, options);
    for warning in &root_warnings {
        eprintln!("{warning}");
    }
    let bundle = resolve_source_bundle(roots, snapshots, loading, *limits)?;
    let warnings = input_extension_warnings(&bundle.evidence().sources, options);
    for warning in &warnings {
        if !root_warnings.contains(warning) {
            eprintln!("{warning}");
        }
    }
    compile_files_with_registry_inner(&bundle, options, registry, target).map(|mut module| {
        module.extension_warnings = warnings;
        module.source_bundle = Some(bundle.evidence().clone());
        module
    })
}

fn compile_files_with_registry_inner(
    bundle: &ResolvedSourceBundle,
    options: &CompileOptions,
    registry: &VerifierRegistry,
    target: Option<&RecordedTarget>,
) -> Result<CompiledModule, Vec<FileDiagnostic>> {
    let files = &bundle.evidence().sources;
    qcore_frontend::source_extension::reject_alias_collisions(
        files.iter().map(|file| file.path.as_str()),
    )
    .map_err(|message| vec![plain_diagnostic("Q403", message)])?;
    // Original ranges, including original source lines, survive all staging.
    let (_, ranges) = combine_files(files);
    let stripped = bundle.staged();
    let mut individual = Vec::new();
    for (file, range) in stripped.iter().zip(&ranges) {
        let parsed = parse_source_program(&file.source).map_err(|error| {
            vec![map_raw_diagnostic(
                "Q000",
                SemanticPass::NameResolution,
                Span {
                    start: error.span.start + range.start,
                    end: error.span.end + range.start,
                },
                error.message,
                &ranges,
            )]
        })?;
        // Card SUPPLIED-OWNED-RETURN-CODE-HANDOFF: an `alias` belongs to a
        // contract module; the staged text has lost its header, so the
        // file's own declared mode decides.
        if let Some(alias) = parsed.aliases().first()
            && bundle.file_mode(individual.len()) != Some(SourceModuleMode::Contract)
        {
            return Err(vec![map_raw_diagnostic(
                "Q000",
                SemanticPass::NameResolution,
                Span {
                    start: alias.span.start + range.start,
                    end: alias.span.end + range.start,
                },
                "an `alias` declaration belongs to a contract module".into(),
                &ranges,
            )]);
        }
        individual.push(parsed);
    }
    let externs = individual
        .iter()
        .flat_map(|program| program.declarations.iter())
        .filter(|d| d.value.kind == DeclarationKind::ExternFunction)
        .map(|d| d.value.name.clone())
        .collect();
    for (parsed, range) in individual.iter().zip(&ranges) {
        qcore_frontend::module_imports::check_source_imports_in_scope(parsed, &externs).map_err(
            |errors| {
                errors
                    .into_iter()
                    .map(|error| {
                        map_raw_diagnostic(
                            error.code,
                            error.pass,
                            Span {
                                start: error.span.start + range.start,
                                end: error.span.end + range.start,
                            },
                            error.message,
                            &ranges,
                        )
                    })
                    .collect::<Vec<_>>()
            },
        )?;
    }
    // Only roots decide executable intent; an unmarked dependency cannot
    // turn a library into an executable or nominate a process entry point.
    let mode = bundle.mode();
    if mode != SourceModuleMode::Executable && options.entry.is_some() {
        return Err(vec![plain_diagnostic(
            "Q403",
            "--entry is incompatible with library/contract mode; include an executable module to link an entry point",
        )]);
    }
    let (combined, _) = combine_files(&stripped);
    let mut parsed = parse_source_program(&combined).map_err(|error| {
        vec![map_raw_diagnostic(
            "Q000",
            SemanticPass::NameResolution,
            error.span,
            error.message,
            &ranges,
        )]
    })?;
    parsed.mode = mode;
    // Card `W48-QR-C-STRUCT-IMPORT` (user decision 4): the FFI marker belongs
    // to the contract side, the `.qv` files, so an auditor finds every FFI
    // function there. A runtime or library module carrying one is refused
    // rather than ignored. Decided on the bundle's mode, which is the module
    // header's (the per-file staged text no longer carries it).
    if mode != SourceModuleMode::Contract
        && let Some(span) = parsed
            .declarations
            .iter()
            .find_map(|declaration| declaration.value.c_ffi)
    {
        return Err(vec![map_raw_diagnostic(
            "Q033",
            SemanticPass::UncheckedLinkage,
            span,
            "`[| quant #c_ffi |]` marks an FFI function definition in a `.qv` contract module; a runtime or library module declares its callee with a plain `extern fn` and calls it through the marked contract".into(),
            &ranges,
        )]);
    }
    // Card `NATIVE-LIBRARY-SOURCE-BINDING`. A binding is checked **wherever it
    // is written**, not only where a route happens to read it. Without this a
    // module that declares `extern library NAME;` and is routed somewhere with
    // no extern surface would carry a declaration nothing ever looked at, and an
    // unresolvable one would be silently ignorable.
    //
    // Guarded on the file having written one, and deliberately: loading the
    // selection unconditionally would let a broken manifest refuse a module that
    // never mentioned one, which would move programs written before this card.
    // A module that declares a binding is a new program by construction, so
    // nothing existing travels this branch.
    if let Some(first) = parsed.library_bindings().first() {
        let libraries = load_native_libraries(options, first.span, &ranges)?;
        bind_source_libraries(&parsed, &libraries, &ranges)?;
    }
    let scopes = registry.resolve_source_scopes(&parsed).map_err(|errors| {
        errors
            .into_iter()
            .map(|error| {
                let span = parsed
                    .declarations
                    .iter()
                    .find(|declaration| declaration.value.name == error.scope)
                    .map_or(Span { start: 0, end: 0 }, |declaration| declaration.span);
                map_raw_diagnostic(
                    "Q700",
                    SemanticPass::PluginVerification,
                    span,
                    error.to_string(),
                    &ranges,
                )
            })
            .collect::<Vec<_>>()
    })?;
    if !scopes.is_empty() {
        // W3: resolved scopes drive real verification. The core derives the
        // obligation inventory, dispatches requests, and fails closed; a scope
        // is verified only when every core-derived obligation is discharged by
        // core-checked evidence.
        let unit = stripped
            .first()
            .map_or("<unit>", |file| file.path.as_str())
            .to_owned();
        let report = crate::verify_dispatch::dispatch_scopes(&parsed, &unit, &scopes, registry);
        let diagnostics = report.diagnostics();
        if !diagnostics.is_empty() {
            return Err(diagnostics
                .into_iter()
                .map(|diagnostic| {
                    let span = parsed
                        .declarations
                        .iter()
                        .find(|declaration| declaration.value.name == diagnostic.scope)
                        .map_or(Span { start: 0, end: 0 }, |declaration| declaration.span);
                    map_raw_diagnostic(
                        diagnostic.code,
                        SemanticPass::PluginVerification,
                        span,
                        diagnostic.message,
                        &ranges,
                    )
                })
                .collect());
        }
    }
    let typed = elaborate_source(&parsed).map_err(|diagnostics| {
        diagnostics
            .into_iter()
            .map(|diagnostic| map_diagnostic(diagnostic, &ranges))
            .collect::<Vec<_>>()
    })?;
    // Card `W48-QR-SCALAR-TYPES-R3` (`W48-INERT-EXTERN-DECLARATION-TYPES`):
    // every admitted module's `extern fn` declarations are read, called or
    // not. Run after the route's own passes, on an admitted module only, so
    // every refusal given before keeps its bytes and only an admission can
    // become a refusal.
    //
    // A **contract** module is exempt, and not by omission: its `extern fn`
    // declarations are the contract surface itself, written in the resource
    // contract language (whose type vocabulary is wider than §C04.1's — `f64`,
    // `Buffer`, quantity types) and read by the contract loader whenever a
    // program selects them; it declares nothing callable here.
    let module = compile_elaborated_source(&parsed, &typed, mode, options, &ranges, target)?;
    // Card `ROUTE-SELECTION-ORDER-RESTATED`: the completeness check that
    // stood here (a contract module admitted on any checked variant but
    // `Sequential` is refused) is deleted, being unreachable by construction:
    // the contract guard in `compile_elaborated_source` reads the entry that
    // is lowered, so a contract module is lowered only by the one entry that
    // is not source-only, the sequential route, whose lowering builds
    // `CheckedSourceModule::Sequential` on every admission
    // (`source_routes::tests::only_the_last_entry_reads_contract_modules`;
    // Lean `QCore.RouteSelection.contract_lowered_only_on_last`).
    // Card `W48-QR-C-STRUCT-IMPORT`: `source.ffi.v1` reads every extern
    // declaration of its module itself, against the imported struct names and
    // its selected contracts (a `ptr<const, Header>` is its vocabulary), so the
    // §C04.1 read below is not a second, narrower reader of the same text.
    if mode != SourceModuleMode::Contract
        && module.execution_profile != SourceExecutionProfile::FfiV1
    {
        check_extern_declaration_signatures(&parsed, &ranges)?;
    }
    Ok(module)
}

/// Card `W48-QR-SCALAR-TYPES-R3` (`W48-INERT-EXTERN-DECLARATION-TYPES`).
///
/// An `extern fn` declaration that no body calls used to be read by nothing
/// but name resolution on every route that admitted it: its parameter and
/// result annotations were never looked at, so `extern fn f(x: Widget) ->
/// Widget;` beside a sequential entry was admitted (card
/// `W48-QR-SCALAR-TYPES-R2`'s finding). Every declaration of an admitted
/// module is now read here against the C04.1 ABI vocabulary a declaration can
/// state — the §G02t scalars, the three custody pointers and `unit` as a
/// result — and every other spelling refuses at `Q020`, naming the parameter
/// or the result. A called declaration already met the stricter equality with
/// its selected contract; this check adds nothing for it.
fn check_extern_declaration_signatures(
    parsed: &SourceProgram,
    ranges: &[FileRange],
) -> Result<(), Vec<FileDiagnostic>> {
    use qcore_core::extern_call::AbiType;
    let mut refusals = Vec::new();
    for declaration in &parsed.declarations {
        if declaration.value.kind != DeclarationKind::ExternFunction {
            continue;
        }
        let name = &declaration.value.name;
        for parameter in &declaration.value.parameters {
            let written = parameter.type_source.trim();
            if AbiType::parse(written).is_some_and(|abi| abi != AbiType::Unit) {
                continue;
            }
            refusals.push(map_raw_diagnostic(
                "Q020",
                SemanticPass::OrdinaryTypes,
                parameter.type_span,
                format!(
                    "parameter `{}` of `extern fn {name}` is `{written}`, which is not a type an extern declaration can state; a parameter is a scalar (bool, i8, i16, i32, i64, isize, u8, u16, u32, u64, usize, f32, f64) or one of the custody pointers ptr<mut, u8>, ptr<const, u8>, ptr<mut, ptr<mut, u8>> (§C04.1), and every declaration is read whether or not a body calls it",
                    parameter.name.value
                ),
                ranges,
            ));
        }
        if let Some(result) = &declaration.value.result_type {
            let written = result.source.trim();
            if AbiType::parse(written).is_none() {
                refusals.push(map_raw_diagnostic(
                    "Q020",
                    SemanticPass::OrdinaryTypes,
                    result.span,
                    format!(
                        "the result of `extern fn {name}` is `{written}`, which is not a type an extern declaration can state; a result is `unit`, a fixed-width scalar or one of the custody pointers (§C04.1), and every declaration is read whether or not a body calls it"
                    ),
                    ranges,
                ));
            }
        }
    }
    if refusals.is_empty() {
        Ok(())
    } else {
        Err(refusals)
    }
}

/// The route selection and every route, over one elaborated module (split out
/// of [`compile_files_with_registry_inner`] by card `W48-QR-SCALAR-TYPES-R3`
/// so a module-wide check can follow whichever route admitted it; the body is
/// unchanged).
fn compile_elaborated_source(
    parsed: &SourceProgram,
    typed: &TypedSourceProgram,
    mode: SourceModuleMode,
    options: &CompileOptions,
    ranges: &[FileRange],
    target: Option<&RecordedTarget>,
) -> Result<CompiledModule, Vec<FileDiagnostic>> {
    // Card `ROUTE-SELECTION-ORDER-RESTATED`: the route order is written once,
    // in `source_routes::SOURCE_ROUTES`, and the module's route is selected
    // once (first match, in table order). Every check below reads that one
    // entry, in the order they had when each restated the order itself.
    let (route, selection) = source_routes::select(parsed);
    // Card `W48-QR-CLOSURES`: a closure is lifted into a hidden worker that the
    // two scoped-task routes lower; on any other route it is refused here,
    // naming the route's residual (`SourceRoute::closure`).
    if let Some(closure) = parsed.closures().first()
        && let source_routes::ClosureRule::Refuses(message) = route.closure
    {
        return Err(vec![map_raw_diagnostic(
            "Q600",
            SemanticPass::Concurrency,
            closure.span,
            message.to_owned(),
            ranges,
        )]);
    }
    // Card `QR-LEXICAL-SELECTORS-DECLARATION-NAMES`. Every token-stream
    // selector skips a declaration's own name, a parameter's name and a
    // contract clause (`route_selection::SelectorScope`; a record's name stays
    // read by constraint 14's `AGGREGATE-SELECTOR-WORDS`), so such a name
    // never selects a route. A selecting use of a word the module also
    // declares names both and is refused, never routed.
    if let Some(refusal) = qcore_frontend::route_selection::ambiguous_selector_use(parsed) {
        return Err(vec![map_diagnostic(refusal, ranges)]);
    }
    // The contract guard: a `module contract;` module declares contracts, and
    // every source-only route lowers bodies it would run, so it is refused
    // here, naming the route, before that route reads it. The guard reads the
    // entry that is lowered below, so a route added to the table is guarded
    // by construction; the one entry that is not source-only is the
    // sequential route, which reads contract modules.
    if mode == SourceModuleMode::Contract && route.source_only {
        let span = parsed
            .declarations
            .iter()
            .find(|declaration| declaration.value.kind == DeclarationKind::Function)
            .map_or(Span { start: 0, end: 0 }, |declaration| declaration.span);
        return Err(vec![map_raw_diagnostic(
            "Q020",
            SemanticPass::OrdinaryTypes,
            span,
            format!(
                "{}: a `module contract;` module declares contracts and is never compiled on this route; move the function bodies to a library or executable module",
                route.name
            ),
            ranges,
        )]);
    }
    if let source_routes::Selection::Refused(refusal) = selection {
        return Err(vec![map_diagnostic(refusal, ranges)]);
    }
    (route.lower)(&source_routes::RouteInput {
        parsed,
        typed,
        mode,
        options,
        ranges,
        target,
    })
}

/// Card `W48-QR-SCALAR-TYPES-R2`, item 0 (`W48-SEQUENTIAL-UNCHECKED-PARAMETER-TYPES`).
///
/// The sequential route supplies no argument to any parameter — a direct call
/// names a zero-parameter function and the emitted entry has none — so before
/// this card it never read a parameter annotation at all, and admitted
/// `x: Widget`, `f: fn() -> i32` and `x: usize` with no recorded target. It now
/// checks each one against the §G02t scalar table, with `isize`/`usize`
/// requiring the recorded target exactly as everywhere else, and refuses every
/// other type. Run after the route's own passes, so every refusal it gave before
/// keeps its bytes and only an admission can become a refusal.
fn check_sequential_parameters(
    parsed: &SourceProgram,
    target: Option<&RecordedTarget>,
    ranges: &[FileRange],
) -> Result<(), Vec<FileDiagnostic>> {
    use qcore_core::scalar_types::ScalarType;
    let mut refusals = Vec::new();
    for declaration in &parsed.declarations {
        if declaration.value.kind != DeclarationKind::Function {
            continue;
        }
        for parameter in &declaration.value.parameters {
            let written = parameter.type_source.trim();
            let message = match ScalarType::parse(written) {
                Some(ty) if ty.is_target_width() && target.is_none() => format!(
                    "parameter `{}` is `{written}`, and `isize` and `usize` take their width from a recorded target, none of which was supplied; the width is never guessed (supply one with `--recorded-target FILE`)",
                    parameter.name.value
                ),
                Some(_) => continue,
                None => format!(
                    "parameter `{}` is `{written}`: the sequential route supplies no argument to any parameter and checks a parameter annotation against the fixed-width scalar table (bool, i8, i16, i32, i64, isize, u8, u16, u32, u64, usize) only; `{written}` is not a type it supports",
                    parameter.name.value
                ),
            };
            refusals.push(map_raw_diagnostic(
                "Q020",
                SemanticPass::OrdinaryTypes,
                parameter.type_span,
                message,
                ranges,
            ));
        }
    }
    if refusals.is_empty() {
        Ok(())
    } else {
        Err(refusals)
    }
}

/// One entry-only route's name and its own "models the entry only" code.
#[derive(Clone, Copy)]
struct EntryOnlyRoute {
    name: &'static str,
    code: &'static str,
    pass: SemanticPass,
}

/// Card `W48-QR-SCALAR-TYPES-R2`, item 0. Six routes model the entry and
/// nothing else: `source.std-io.v1`, `source.buffer.v1` (with its record and
/// loop forms), `source.string.v1`, `source.socket.v1`, `source.http.v1` and
/// `source.event.v1`. Each already refused a helper function that carried one
/// of its own constructs, and admitted every other helper **unread** — its
/// parameter, result and local annotations and its body — so
/// `fn h(x: Widget) -> Widget { let q: usize = 7; return 0; }` beside an
/// admitted entry was admitted. That route's existing entry-only condition is
/// widened here, with its own code, to every function it does not model.
/// Applied to an admitted module only, so a module refused before keeps its
/// refusal byte for byte.
fn refuse_unmodeled_functions(
    module: CompiledModule,
    route: EntryOnlyRoute,
    parsed: &SourceProgram,
    ranges: &[FileRange],
) -> Result<CompiledModule, Vec<FileDiagnostic>> {
    let Some(entry) = module.entry.as_deref() else {
        return Ok(module);
    };
    let refusals = parsed
        .declarations
        .iter()
        .filter(|declaration| {
            declaration.value.kind == DeclarationKind::Function && declaration.value.name != entry
        })
        .map(|declaration| {
            map_raw_diagnostic(
                route.code,
                route.pass,
                declaration.span,
                format!(
                    "`{}` models the entry only; `{}` is not the entry `{entry}`, and this route does not check its signature or body, so it is refused rather than admitted unread",
                    route.name, declaration.value.name
                ),
                ranges,
            )
        })
        .collect::<Vec<_>>();
    if refusals.is_empty() {
        Ok(module)
    } else {
        Err(refusals)
    }
}

/// Whether an executable module has a function besides its entry: the
/// `source.std-io.v1` route refuses such a module (item 0 above), so one with
/// scalar-call syntax reaches the scalar route, which checks every function.
fn has_unmodeled_function(
    parsed: &SourceProgram,
    typed: &TypedSourceProgram,
    mode: SourceModuleMode,
    options: &CompileOptions,
) -> bool {
    if mode != SourceModuleMode::Executable {
        return false;
    }
    let Ok(entry) = select_entry(typed, options) else {
        return false;
    };
    parsed.declarations.iter().any(|declaration| {
        declaration.value.kind == DeclarationKind::Function && declaration.value.name != entry
    })
}
/// The sequential cleanup route (§G02), extracted unchanged from
/// `compile_files_with_registry_inner` so that card `W48-QR-SCALAR-TYPES-R2`
/// can check its parameter annotations after it.
fn compile_sequential_source(
    parsed: &SourceProgram,
    typed: &TypedSourceProgram,
    mode: SourceModuleMode,
    options: &CompileOptions,
    ranges: &[FileRange],
) -> Result<CompiledModule, Vec<FileDiagnostic>> {
    let error_names = parsed
        .declarations
        .iter()
        .filter(|declaration| declaration.value.kind == DeclarationKind::ErrorType)
        .map(|declaration| declaration.value.name.clone())
        .collect::<BTreeSet<_>>();
    let error_names = if error_names.is_empty() {
        BTreeSet::from(["Failure".to_owned()])
    } else {
        error_names
    };
    let mut semantics = synthesize_semantics(parsed, typed, &error_names)
        .map_err(|diagnostic| vec![map_diagnostic(diagnostic, ranges)])?;
    mark_recursive_contracts(&mut semantics);
    let semantic_calls = semantics
        .iter()
        .map(|function| (function.symbol, function.calls.clone()))
        .collect::<BTreeMap<_, _>>();
    let checked = check_whole_program(
        typed,
        &ErrorDeclarations::new(error_names.iter().cloned()),
        semantics,
    )
    .map_err(|diagnostics| {
        diagnostics
            .into_iter()
            .map(|diagnostic| map_diagnostic(diagnostic, ranges))
            .collect::<Vec<_>>()
    })?;

    let result_profile = SourceResultProfile {
        owned_buffer_results: std::collections::BTreeSet::new(),
        scalar_results: typed
            .declarations
            .iter()
            .filter(|declaration| declaration.kind == DeclarationKind::Function)
            .map(|declaration| {
                (
                    declaration.name.clone(),
                    declaration
                        .result_type
                        .as_ref()
                        .and_then(|annotation| annotation.executable_type())
                        .expect("function synthesis validated every scalar result"),
                )
            })
            .collect(),
        custody_payloads: CustodyPayloadProfile::SeparateIdentityLedger,
    };
    if mode != SourceModuleMode::Executable {
        return Ok(CompiledModule {
            source_bundle: None,
            extension_warnings: Vec::new(),
            checked: CheckedSourceModule::Sequential(Box::new(checked)),
            execution_profile: SourceExecutionProfile::SequentialCleanupV1,
            declaration_count: typed.declarations.len(),
            mode,
            result_profile,
            entry: None,
            c_source: None,
            source_map: Vec::new(),
            reference_observation: None,
            extern_observation: None,
        });
    }
    let entry = select_entry(typed, options).map_err(|diagnostic| vec![diagnostic])?;
    let symbol = typed.globals[&entry];
    let function = &checked.functions[&symbol];
    validate_emittable_calls(symbol, &semantic_calls, &checked)
        .map_err(|diagnostic| vec![map_diagnostic(diagnostic, ranges)])?;
    if !function.source.parameters.is_empty() {
        return Err(vec![map_raw_diagnostic(
            "Q020",
            SemanticPass::OrdinaryTypes,
            function.source.span,
            "the emitted entry function must not have parameters".into(),
            ranges,
        )]);
    }
    let reference_observation = qcore_core::interpret_cfg(&function.cfg).map_err(|error| {
        vec![map_raw_diagnostic(
            "Q202",
            SemanticPass::CleanupLowering,
            function.source.span,
            error.to_string(),
            ranges,
        )]
    })?;
    let log_level = qcore_frontend::source_std_io::entry_log_level(parsed, &entry);
    let raw_c = emit_c11_with_log_level(&function.cfg, log_level).map_err(|error| {
        vec![map_raw_diagnostic(
            "Q500",
            SemanticPass::CEmission,
            function.source.span,
            error.to_string(),
            ranges,
        )]
    })?;
    let source_location = locate(function.source.span.start, ranges);
    let (c_source, source_map) = add_source_mapping(raw_c, &entry, source_location);
    Ok(CompiledModule {
        source_bundle: None,
        extension_warnings: Vec::new(),
        checked: CheckedSourceModule::Sequential(Box::new(checked)),
        execution_profile: SourceExecutionProfile::SequentialCleanupV1,
        declaration_count: typed.declarations.len(),
        mode,
        result_profile,
        entry: Some(entry),
        c_source: Some(c_source),
        source_map,
        reference_observation: Some(reference_observation),
        extern_observation: None,
    })
}

fn combine_files(files: &[SourceFile]) -> (String, Vec<FileRange>) {
    let mut combined = String::new();
    let mut ranges = Vec::new();
    for file in files {
        let start = combined.len();
        combined.push_str(&file.source);
        let end = combined.len();
        ranges.push(FileRange {
            path: file.path.clone(),
            start,
            end,
            source: file.source.clone(),
        });
        if !combined.ends_with('\n') {
            combined.push('\n');
        }
    }
    (combined, ranges)
}

fn synthesize_semantics(
    parsed: &SourceProgram,
    typed: &TypedSourceProgram,
    errors: &BTreeSet<String>,
) -> Result<Vec<FunctionSemantics>, SourceDiagnostic> {
    parsed
        .declarations
        .iter()
        .filter(|declaration| declaration.value.kind == DeclarationKind::Function)
        .map(|declaration| {
            let typed_declaration = typed
                .declarations
                .iter()
                .find(|candidate| candidate.name == declaration.value.name)
                .expect("elaboration preserves declarations");
            synthesize_function(parsed, declaration, typed_declaration, typed, errors)
        })
        .collect()
}

/// The operations one statement sequence contributes to each checked model.
///
/// Card `IF-STATEMENT`. Before it, `synthesize_function` was a single flat loop
/// appending to five parallel vectors; a branch arm is a *sequence* whose
/// contribution must be bundled so the arms can be joined. Each field is the
/// arm's contribution to one existing model, and each of those models already
/// carries its own branch join — `OwnershipOp::Branch`, `ErrorOp::Branch`,
/// `UsageExpr::Branch` and `Flow::join_branches` all predate this card.
#[derive(Clone, Default)]
struct SequenceLowering {
    lowering: Vec<SourceOp>,
    ownership: Vec<OwnershipOp>,
    error_operations: Vec<ErrorOp>,
    demands: Vec<(UsageExpr, bool)>,
    flow: Flow,
}

/// Where a statement sequence sits relative to the branch construct.
///
/// Card `IF-STATEMENT` carried this as a `top_level: bool` that gated both
/// `return` and a call. Card `IF-ARMS` admits both inside an arm, so what is
/// left to distinguish is *mechanical* — a call inside an arm may not split the
/// flat resource segmentation (see `lower_sequence`) — plus one attribution:
/// statements the reader wrote **after** a branch can end up inside an arm
/// through the continuation hoisting an arm `return` requires, and a refusal
/// about them has to say so rather than claim the reader wrote them in an arm.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ArmPosition {
    /// The entry body itself.
    TopLevel,
    /// Lexically inside a branch arm.
    Arm,
    /// Written after the branch and placed inside the one arm that reaches it,
    /// because the other arm returned (card `IF-ARMS`, `IF-ARM-RETURN`).
    HoistedContinuation,
}

impl ArmPosition {
    /// Whether the statements lowered at this position run on one path through
    /// the branch rather than on every path through the function.
    const fn conditional(self) -> bool {
        !matches!(self, Self::TopLevel)
    }
}

/// The quantitative counters a sequence advances. Snapshotted and joined at a
/// branch, componentwise in the same direction `Flow::join_branches` takes.
#[derive(Clone, Copy)]
struct Counters {
    live: u64,
    peak_live: u64,
    total: u64,
    reachable: bool,
}

fn synthesize_function(
    parsed: &SourceProgram,
    declaration: &Located<SourceDeclaration>,
    typed_declaration: &TypedSourceDeclaration,
    typed: &TypedSourceProgram,
    errors: &BTreeSet<String>,
) -> Result<FunctionSemantics, SourceDiagnostic> {
    let annotation = typed_declaration.result_type.as_ref().ok_or_else(|| {
        diagnostic(
            SemanticPass::OrdinaryTypes,
            declaration.span,
            "missing declared result type",
        )
    })?;
    let result_type = annotation.executable_type().ok_or_else(|| {
        diagnostic(SemanticPass::OrdinaryTypes, annotation.span,
            "the executable result profile supports only i32 and unit; other result types require an implemented typed ABI")
    })?;
    let Some(body) = &declaration.value.body else {
        return Err(diagnostic(
            SemanticPass::CleanupLowering,
            declaration.span,
            "source function requires a body",
        ));
    };
    let usage_declarations = typed_declaration
        .parameters
        .iter()
        .map(|parameter| {
            (
                parameter.name.clone(),
                parameter
                    .usage
                    .as_ref()
                    .map_or(Usage::MANY, |grade| grade.value),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut builder = FunctionLowering {
        parsed,
        typed,
        typed_declaration,
        errors,
        result_type,
        resources: Vec::new(),
        owners: BTreeMap::new(),
        constants: BTreeMap::new(),
        diagnostic_log_level: None,
        usage_declarations,
        local_usage_names: BTreeMap::new(),
        calls: Vec::new(),
        resource_segments: Vec::new(),
        counters: Counters {
            live: 0,
            peak_live: 0,
            total: 0,
            reachable: true,
        },
        region_operations: Vec::new(),
        region_span: None,
    };
    let mut sequence = SequenceLowering::default();
    builder.lower_sequence(&body.statements, &mut sequence, ArmPosition::TopLevel)?;

    let FunctionLowering {
        resources,
        usage_declarations,
        calls,
        mut resource_segments,
        counters,
        region_operations,
        region_span,
        ..
    } = builder;
    resource_segments.push(sequence.flow);
    // Card `QUANT-REGION-SOURCE`. The effect row the sequential route declares
    // is `[Alloc]`, and the error-region model requires a raised error to be in
    // it. An entry that writes a region therefore carries the declared error
    // set as its `Throws` row; an entry that writes none carries exactly the
    // row it always carried, which is what keeps every existing program
    // byte-identical. The row is **derived** from the declarations rather than
    // written by the reader — this route has no `effects { … }` syntax — and
    // that is a boundary, not a claim (follow-on `REGION-EFFECT-ROW`).
    let allowed_effects = if region_operations.is_empty() {
        EffectRow::new([Effect::Alloc])
    } else {
        EffectRow::new(
            std::iter::once(Effect::Alloc).chain(errors.iter().cloned().map(Effect::Throws)),
        )
    };
    // Run the unedited model here, before the flattened program is assembled,
    // so a region's own refusal is the one the reader sees. `check_whole_program`
    // runs it again over the same operations and remains the authority; this
    // call only decides *which* refusal is reported first, because the cleanup
    // lowering below would otherwise report a leak at `Q202` for a pack the
    // handler never discharged.
    if !region_operations.is_empty() {
        analyze_error_regions(
            &ErrorDeclarations::new(errors.iter().cloned()),
            &allowed_effects,
            ErrorLedger::default(),
            &region_operations,
        )
        .map_err(|error| SourceDiagnostic {
            code: qcore_frontend::source_region::error_region_code(&error),
            pass: SemanticPass::ErrorRegion,
            span: region_span.unwrap_or(declaration.span),
            message: qcore_frontend::source_region::error_region_message(&error),
        })?;
    }
    let lowering = LoweringProgram::new(
        declaration.value.name.clone(),
        resources,
        errors.iter().cloned(),
        sequence.lowering,
    )
    .map_err(|error| branch_aware_lowering_diagnostic(declaration.span, &error))?;
    // Exercise the source interpreter at the synthesis boundary as well as the
    // whole-program translation validator.
    interpret_source(&lowering)
        .map_err(|error| branch_aware_lowering_diagnostic(declaration.span, &error))?;
    let parameters = typed_declaration
        .parameters
        .iter()
        .filter_map(|parameter| match parameter.type_kind {
            qcore_frontend::frontend::SourceTypeKind::Borrow => Some(ParameterOwnership::Borrowed),
            qcore_frontend::frontend::SourceTypeKind::Owned => Some(ParameterOwnership::Consumed),
            _ => None,
        })
        .collect();
    let usage_expression = unfold_exit_demands(sequence.demands);
    Ok(FunctionSemantics {
        symbol: typed_declaration.symbol,
        usage_declarations,
        usage_expression,
        ownership_operations: sequence.ownership,
        resource_segments,
        resource_bounds: ResourceBounds::new(
            Quantity::single("resources", counters.peak_live),
            Quantity::single("allocations", counters.total),
        ),
        allowed_effects,
        initial_error_ledger: ErrorLedger::default(),
        error_operations: sequence.error_operations,
        region_operations,
        lowering,
        calls,
        identity_entry: Vec::new(),
        identity_exit: Vec::new(),
        parameters,
        recursive_contract: None,
    })
}

/// The statements written after a branch, together with the **single**
/// permission to place them inside the arm that survives to reach them (card
/// `IF-ARMS`).
///
/// A branch level spends the permission when exactly one of its arms returns.
/// Every level shares one of these, so the statements are lowered at most once:
/// a second level asking for them is refused rather than served by duplicating
/// acquisitions, call positions and usage demands. `spent` distinguishes "there
/// were none" from "a nested level already took them", so the refusal can say
/// which.
struct Continuation<'a> {
    statements: &'a [Located<StatementKind>],
    spent: bool,
}

impl<'a> Continuation<'a> {
    fn new(statements: &'a [Located<StatementKind>]) -> Self {
        Self {
            statements,
            spent: false,
        }
    }

    /// Whether the statements are still to be lowered after the join, which is
    /// where they were before this card and where they stay when no arm exits.
    const fn available(&self) -> bool {
        !self.spent
    }

    fn take(&mut self) -> Option<&'a [Located<StatementKind>]> {
        if self.spent || self.statements.is_empty() {
            return None;
        }
        self.spent = true;
        Some(self.statements)
    }
}

/// Turn one sequence's `(demand, may_exit)` pairs into the usage expression the
/// usage checker consumes.
///
/// Callees may exit before their continuation. Without a proved normal-exit
/// summary, zero is a possible demand on every subsequent source use, so a
/// demand flagged `may_exit` makes everything after it optional.
///
/// Card `IF-ARMS` factored this out of `synthesize_function` so a **branch arm**
/// runs the same unfolding: an arm may now contain a call, and collapsing an
/// arm's demands into a flat `Sequence` would have asserted that the statements
/// after that call always run — the opposite of what the flag records.
fn unfold_exit_demands(demands: Vec<(UsageExpr, bool)>) -> UsageExpr {
    let mut reversed_tail = Vec::new();
    for (expression, may_exit) in demands.into_iter().rev() {
        if may_exit && !reversed_tail.is_empty() {
            reversed_tail.reverse();
            let continuation = UsageExpr::Branch {
                condition: Box::new(UsageExpr::Unit),
                then_expression: Box::new(UsageExpr::Sequence(reversed_tail)),
                else_expression: Box::new(UsageExpr::Unit),
            };
            reversed_tail = vec![continuation];
        }
        reversed_tail.push(expression);
    }
    reversed_tail.reverse();
    UsageExpr::Sequence(reversed_tail)
}

struct FunctionLowering<'a> {
    parsed: &'a SourceProgram,
    typed: &'a TypedSourceProgram,
    typed_declaration: &'a TypedSourceDeclaration,
    errors: &'a BTreeSet<String>,
    result_type: ExecutableResultType,
    resources: Vec<ResourceId>,
    owners: BTreeMap<String, ResourceId>,
    /// Check-time constant integer bindings, `let NAME = INTEGER;` (card
    /// `BITWISE-CONDITIONS`). A separate namespace from `owners` by
    /// construction: nothing moves a name between the two, and the tier that
    /// reads this map has no constructor that could denote an identity.
    constants: BTreeMap<String, i32>,
    diagnostic_log_level: Option<qcore_core::std_io::Level>,
    usage_declarations: BTreeMap<String, Usage>,
    local_usage_names: BTreeMap<String, String>,
    calls: Vec<CallSite>,
    resource_segments: Vec<Flow>,
    counters: Counters,
    /// The checked error-region operations this entry contributes (card
    /// `QUANT-REGION-SOURCE`, `source.region.v1`). Empty for every entry that
    /// writes no `error region`, which is what keeps every program that
    /// predates the card byte-identical: an empty list is the value this field
    /// carried unconditionally before it.
    region_operations: Vec<RegionOp>,
    /// The span of the entry's `error region` statement, for attributing the
    /// model's refusals — [`analyze_error_regions`] reports no position of its
    /// own. v1 admits at most one region per entry, so this span is exact
    /// (follow-on `REGION-MULTIPLE`).
    region_span: Option<Span>,
}

impl FunctionLowering<'_> {
    /// Lower one statement sequence — an entry body, or one arm of a branch —
    /// and report whether it ends the function on **every** path through it.
    ///
    /// Card `IF-ARMS` widened the arm subset. `return` and a zero-argument call
    /// are both admitted inside an arm now; what `position` still decides is
    /// mechanical (a conditional call may not split the flat resource
    /// segmentation) plus the attribution of the one shape that stays refused,
    /// an acquisition inside an arm (`Q103`, follow-on `IF-ARM-ACQUIRE`).
    ///
    /// The returned flag is what the branch join needs: an arm that returns does
    /// not reach the join, so the join is taken over the arms that survive.
    fn lower_sequence(
        &mut self,
        statements: &[Located<StatementKind>],
        output: &mut SequenceLowering,
        position: ArmPosition,
    ) -> Result<bool, SourceDiagnostic> {
        let mut terminates = false;
        for (index, statement) in statements.iter().enumerate() {
            // Card `IF-STATEMENT` narrows this gate rather than removing it:
            // `if` is admitted by `source.branch.v1` and every other structured
            // statement keeps the refusal it has always had.
            // Card `QUANT-REGION-SOURCE` narrows this gate a second time, the
            // way card `IF-STATEMENT` narrowed it the first: `error region` is
            // admitted by `source.region.v1` and every other structured
            // statement keeps the refusal it has always had.
            if !matches!(
                statement.value,
                StatementKind::If { .. } | StatementKind::ErrorRegion { .. }
            ) && !statement.value.nested_blocks().is_empty()
            {
                return Err(diagnostic(
                    SemanticPass::CEmission,
                    statement.span,
                    "structured control flow is not in the Cycle 12 executable subset",
                ));
            }
            if let StatementKind::If {
                conditions,
                branches,
            } = &statement.value
            {
                // Card `IF-ARMS`. The statements after this branch are its
                // **continuation**, and an arm that returns needs them: the arm
                // that survives is the only one that reaches them, so they are
                // lowered inside it rather than after the join. They are lowered
                // exactly once either way — see `lower_branch`.
                let mut continuation = Continuation::new(&statements[index + 1..]);
                let terminated = self.lower_branch(
                    statement.span,
                    conditions,
                    branches,
                    0,
                    output,
                    &mut continuation,
                )?;
                if !continuation.available() {
                    // Hoisted: the continuation was lowered inside the surviving
                    // arm, which is why that arm now terminates too.
                    return Ok(terminated);
                }
                let rest = continuation.statements;
                if terminated {
                    if let Some(unreachable) = rest.first() {
                        return Err(branch_diagnostic(
                            "Q044",
                            unreachable.span,
                            "every arm of this branch returns, so the statements after it are \
                             unreachable",
                        ));
                    }
                    return Ok(true);
                }
                // Not hoisted: the continuation follows the join, exactly as it
                // did before card `IF-ARMS`.
                return self.lower_sequence(rest, output, position);
            }
            let tokens = statement_tokens(self.parsed, statement.span)?;
            if matches!(tokens.first().map(|token| &token.kind), Some(TokenKind::Word(word)) if word == "log_level")
            {
                if position != ArmPosition::TopLevel {
                    return Err(diagnostic(
                        SemanticPass::StandardIo,
                        statement.span,
                        "`log_level LEVEL;` is an entry-level diagnostic adapter declaration",
                    ));
                }
                let level = match qcore_frontend::source_std_io::std_io_statement(
                    self.parsed,
                    statement.span,
                ) {
                    Some(Ok(qcore_core::std_io::StdIoEvent::LogLevel { level })) => level,
                    Some(Err(message)) => {
                        return Err(diagnostic(
                            SemanticPass::StandardIo,
                            statement.span,
                            message.to_owned(),
                        ));
                    }
                    _ => unreachable!("log_level is owned by the standard logger grammar"),
                };
                if self.diagnostic_log_level.replace(level).is_some() {
                    return Err(diagnostic(
                        SemanticPass::StandardIo,
                        statement.span,
                        "an entry accepts at most one `log_level LEVEL;` declaration",
                    ));
                }
                continue;
            }
            match &statement.value {
                StatementKind::Let {
                    binding: Some(binding),
                } => {
                    // Local usage annotations are not in this executable slice.
                    // Reject them instead of silently ignoring a contract.
                    if tokens
                        .iter()
                        .take_while(|token| token.kind != TokenKind::Symbol(Symbol::Equal))
                        .any(|token| matches!(&token.kind, TokenKind::Word(word) if word == "use"))
                    {
                        return Err(diagnostic(
                            SemanticPass::Usage,
                            statement.span,
                            "local usage annotations are not supported in the executable subset",
                        ));
                    }
                    let head = local_binding_head(&tokens).map_err(|message| {
                        diagnostic(SemanticPass::OrdinaryTypes, statement.span, message)
                    })?;
                    // Card `BITWISE-CONDITIONS`: a check-time constant integer
                    // binding, the decidable subset of `IF-CONDITION-BINDINGS`.
                    // Tried **after** `qcore_acquire` cannot match, and only at
                    // the top level of an entry body: an arm-local binding has
                    // no position after the join, so an arm (or a continuation
                    // hoisted into one, card `IF-ARMS`) keeps the refusal.
                    if let Some(value) = parse_constant_binding(&tokens, head.equal) {
                        check_local_annotation(head.annotation.as_ref(), &ExecutableLocalType::I32)
                            .map_err(|message| {
                                diagnostic(SemanticPass::OrdinaryTypes, statement.span, message)
                            })?;
                        if position.conditional() {
                            return Err(branch_diagnostic(
                                "Q044",
                                statement.span,
                                "`source.branch.v1` admits no `let` inside a branch arm: an \
                                 arm-local binding has no position after the join",
                            ));
                        }
                        self.constants.insert(binding.clone(), value);
                        let usage_name = format!("$constant:{}:{binding}", statement.span.start);
                        self.usage_declarations
                            .insert(usage_name.clone(), Usage::MANY);
                        self.local_usage_names.insert(binding.clone(), usage_name);
                        continue;
                    }
                    let (resource, succeeds, error) = parse_acquire(&tokens).ok_or_else(|| {
                        diagnostic(SemanticPass::CEmission, statement.span, "`let` must use `qcore_acquire(resource_id, succeeds, ErrorType)` in the executable subset")
                    })?;
                    check_local_annotation(
                        head.annotation.as_ref(),
                        &ExecutableLocalType::Owned { nominal: None },
                    )
                    .map_err(|message| {
                        diagnostic(SemanticPass::OrdinaryTypes, statement.span, message)
                    })?;
                    // Card `IF-ARM-ACQUIRE-ATTRIBUTION`. An acquisition on one
                    // path leaves a resource-identity ledger the other path does
                    // not have, and `OwnershipState::branch` compares ledgers —
                    // so this has always refused, at `Q103`, attributed to the
                    // *function declaration* with a generic ownership message.
                    // The refusal is deliberately **not weakened**: only its
                    // span and its wording move, which §L21 says is not a code
                    // change. Admitting it is follow-on `IF-ARM-ACQUIRE`.
                    if position.conditional() {
                        return Err(diagnostic(
                            SemanticPass::Ownership,
                            statement.span,
                            match position {
                                ArmPosition::Arm => format!(
                                    "`{binding}` acquires resource identity {resource} inside a \
                                     branch arm: the arm that does not run leaves a different \
                                     resource-identity ledger, so the arms cannot join \
                                     (follow-on `IF-ARM-ACQUIRE`)"
                                ),
                                _ => format!(
                                    "`{binding}` acquires resource identity {resource} after a \
                                     branch one arm of which returns, so it is reached on one \
                                     path only and the arms cannot join on their \
                                     resource-identity ledgers (follow-on `IF-ARM-ACQUIRE`)"
                                ),
                            },
                        ));
                    }
                    if !self.errors.contains(&error) {
                        return Err(diagnostic(
                            SemanticPass::TypedErrors,
                            statement.span,
                            format!("unknown error type `{error}`"),
                        ));
                    }
                    self.record_acquisition(
                        statement.span,
                        binding,
                        resource,
                        succeeds,
                        error,
                        output,
                    )?;
                }
                StatementKind::Expression => {
                    if let Some((passes, error)) = parse_check(&tokens) {
                        if !self.errors.contains(&error) {
                            return Err(diagnostic(
                                SemanticPass::TypedErrors,
                                statement.span,
                                format!("unknown error type `{error}`"),
                            ));
                        }
                        output.lowering.push(SourceOp::Check { passes, error });
                        self.counters.reachable &= passes;
                    } else if let Some(callee_name) = parse_zero_argument_call(&tokens) {
                        if self.local_usage_names.contains_key(&callee_name)
                            || self
                                .typed_declaration
                                .parameters
                                .iter()
                                .any(|parameter| parameter.name == callee_name)
                        {
                            return Err(diagnostic(
                                SemanticPass::OrdinaryTypes,
                                statement.span,
                                "indirect calls are not supported in the executable subset",
                            ));
                        }
                        let Some(&callee) = self.typed.globals.get(&callee_name) else {
                            return Err(diagnostic(
                                SemanticPass::NameResolution,
                                statement.span,
                                format!("unknown function `{callee_name}`"),
                            ));
                        };
                        let callee_declaration = self
                            .typed
                            .declarations
                            .iter()
                            .find(|item| item.symbol == callee);
                        if !callee_declaration
                            .is_some_and(|item| item.kind == DeclarationKind::Function)
                        {
                            return Err(diagnostic(
                                SemanticPass::OrdinaryTypes,
                                statement.span,
                                "only source functions may be called in the executable subset",
                            ));
                        }
                        if callee_declaration.is_some_and(|item| !item.parameters.is_empty()) {
                            return Err(diagnostic(
                                SemanticPass::OrdinaryTypes,
                                statement.span,
                                "executable source calls must have zero arguments",
                            ));
                        }
                        let usage_name = format!("$function:{}", callee.0);
                        self.usage_declarations
                            .insert(usage_name.clone(), Usage::MANY);
                        if self.counters.reachable {
                            output.demands.push((
                                UsageExpr::Apply {
                                    function: Box::new(UsageExpr::Variable(usage_name)),
                                    parameter_usages: Vec::new(),
                                    arguments: Vec::new(),
                                },
                                true,
                            ));
                        }
                        // Card `IF-ARM-CALL`. A call splits the caller's flow
                        // into segments the whole-program summary composes
                        // **positionally**: `segment · callee · segment · …`.
                        // Splitting inside an arm would move the arm's own
                        // custody flow out of the branch and into that flat
                        // chain, where it would be applied on both paths — the
                        // conditional release, silently made unconditional. So a
                        // conditional call contributes a neutral segment and
                        // leaves the arm's flow where the four-model join can see
                        // it; the callee's own flow still composes once, which is
                        // faithful exactly because `unfold_summary` refuses a
                        // conditional call whose callee is not neutral.
                        if position.conditional() {
                            self.resource_segments.push(Flow::zero());
                        } else {
                            self.resource_segments
                                .push(std::mem::take(&mut output.flow));
                        }
                        self.calls.push(CallSite {
                            callee,
                            arguments: Vec::new(),
                            result: None,
                            span: statement.span,
                            conditional: position.conditional(),
                        });
                    } else {
                        return Err(diagnostic(
                            SemanticPass::CEmission,
                            statement.span,
                            "unsupported expression statement in the Cycle 12 executable subset",
                        ));
                    }
                }
                StatementKind::Drop { place: Some(place) } => {
                    let resource = *self.owners.get(place).ok_or_else(|| {
                        diagnostic(
                            SemanticPass::Ownership,
                            statement.span,
                            format!("unknown resource binding `{place}`"),
                        )
                    })?;
                    self.record_release(statement.span, place, resource, false, output)?;
                }
                StatementKind::Move { place: Some(place) } => {
                    let resource = *self.owners.get(place).ok_or_else(|| {
                        diagnostic(
                            SemanticPass::Ownership,
                            statement.span,
                            format!("unknown resource binding `{place}`"),
                        )
                    })?;
                    self.record_release(statement.span, place, resource, true, output)?;
                }
                StatementKind::ErrorRegion {
                    name,
                    blocks,
                    catch_bindings,
                } => {
                    self.lower_error_region(
                        statement,
                        name,
                        blocks,
                        catch_bindings,
                        output,
                        position,
                    )?;
                }
                StatementKind::Return => {
                    let value = parse_return(&tokens, self.result_type).ok_or_else(|| {
                        diagnostic(
                            SemanticPass::OrdinaryTypes,
                            statement.span,
                            match self.result_type {
                                ExecutableResultType::I32 => {
                                    "an i32 result requires a signed 32-bit integer literal return"
                                }
                                ExecutableResultType::Unit => {
                                    "a unit result requires a bare return"
                                }
                            },
                        )
                    })?;
                    // Card `IF-ARMS`, attribution. An exit inside a branch is
                    // checked by the same rule as the program's own exit, and
                    // the cleanup lowering reports a violation of it as
                    // `LiveResourcesAtNormalReturn` — but at the *declaration*,
                    // and only after the resource-flow join has already refused
                    // the arms for a net disagreement the exiting arm never had.
                    // Neither model can say "this arm exited", so the honest
                    // report is made here, at the exit itself, with the code the
                    // condition already owns (`SemanticPass::CleanupLowering`).
                    // The top-level exit is deliberately left alone: its
                    // attribution is pinned elsewhere and this card does not
                    // move it.
                    if position.conditional() && self.counters.live != 0 {
                        return Err(diagnostic(
                            SemanticPass::CleanupLowering,
                            statement.span,
                            format!(
                                "{} owned resource(s) are still live at this `return`: an exit \
                                 from a branch arm carries the same cleanup obligation as the \
                                 program's own exit, so every owned resource must be released \
                                 or moved before it",
                                self.counters.live
                            ),
                        ));
                    }
                    output.lowering.push(SourceOp::Return(value));
                    self.counters.reachable = false;
                    // Card `IF-ARMS`: this exit is now reachable from an arm.
                    // It carries exactly the obligations the sequential route
                    // imposes at any exit — `SourceOp::Return` demands an empty
                    // live list wherever it stands, which is the whole cleanup
                    // discharge, and the error-region rules are unchanged.
                    terminates = true;
                }
                _ => {
                    return Err(diagnostic(
                        SemanticPass::CEmission,
                        statement.span,
                        "statement is not in the Cycle 12 executable subset",
                    ));
                }
            }
        }
        Ok(terminates)
    }

    /// Record one acquisition in every model the sequential route checks.
    ///
    /// Factored out of the statement loop by card `QUANT-REGION-SOURCE` so a
    /// region body's acquisition is *the same* acquisition, in all five models,
    /// as one written outside a region. A second copy of this bookkeeping could
    /// drift; one copy cannot.
    fn record_acquisition(
        &mut self,
        span: Span,
        binding: &str,
        resource: ResourceId,
        succeeds: bool,
        error: String,
        output: &mut SequenceLowering,
    ) -> Result<(), SourceDiagnostic> {
        self.resources.push(resource);
        self.owners.insert(binding.to_owned(), resource);
        // Distinguish a local from a shadowed parameter. Ownership and
        // implicit error cleanup remain the ownership pass's concern.
        let usage_name = format!("$local:{}:{binding}", span.start);
        self.usage_declarations
            .insert(usage_name.clone(), Usage::MANY);
        self.local_usage_names
            .insert(binding.to_owned(), usage_name);
        self.counters.reachable &= succeeds;
        output.lowering.push(SourceOp::Acquire {
            resource,
            succeeds,
            error,
        });
        output.ownership.push(OwnershipOp::Acquire {
            id: resource,
            owner: binding.to_owned(),
            resource_type: "Resource".into(),
            footprint: Footprint::single("resources", 1),
        });
        output.error_operations.push(ErrorOp::Acquire(resource));
        output.flow = output
            .flow
            .then(&Flow::acquire(
                Quantity::single("resources", 1),
                Quantity::single("allocations", 1),
            ))
            .map_err(|error| diagnostic(SemanticPass::ResourceFlow, span, error.to_string()))?;
        self.counters.live = self.counters.live.saturating_add(1);
        self.counters.peak_live = self.counters.peak_live.max(self.counters.live);
        self.counters.total = self.counters.total.saturating_add(1);
        Ok(())
    }

    /// Record one release in every model the sequential route checks —
    /// `transfer` distinguishing `move name;` from `drop name;`.
    ///
    /// Factored out by card `QUANT-REGION-SOURCE` for the reason
    /// [`Self::record_acquisition`] states: the releases a region's cleanup
    /// performs are the releases the reader would have written by hand, so they
    /// must be the same operation in every model rather than a second spelling
    /// of one.
    fn record_release(
        &mut self,
        span: Span,
        place: &str,
        resource: ResourceId,
        transfer: bool,
        output: &mut SequenceLowering,
    ) -> Result<(), SourceDiagnostic> {
        output.lowering.push(if transfer {
            SourceOp::TransferToResult(resource)
        } else {
            SourceOp::Drop(resource)
        });
        if self.counters.reachable {
            output.demands.push((
                UsageExpr::Variable(self.local_usage_names[place].clone()),
                false,
            ));
        }
        output.ownership.push(if transfer {
            OwnershipOp::TransferOut(place.to_owned())
        } else {
            OwnershipOp::Drop(place.to_owned())
        });
        output.error_operations.push(ErrorOp::Drop(resource));
        output.flow = output
            .flow
            .then(&Flow::release(
                Quantity::single("resources", 1),
                Quantity::zero(),
            ))
            .map_err(|error| diagnostic(SemanticPass::ResourceFlow, span, error.to_string()))?;
        self.counters.live = self.counters.live.saturating_sub(1);
        Ok(())
    }

    /// Lower one `error region NAME { … } catch (E) [quant open NAME as B] { … }`
    /// statement (card `QUANT-REGION-SOURCE`, `source.region.v1`).
    ///
    /// # What is built, and what decides it
    ///
    /// Two things, from one reading of the statement:
    ///
    /// * the **region operations** — a `RegionOp::ErrorRegion` whose body,
    ///   catch clause and `quant open` are exactly
    ///   [`qcore_core::error_region`]'s own constructors. Nothing about the
    ///   pack, the capability or its discharge is decided here; the unedited
    ///   model decides all of it, and this is the first source path that
    ///   reaches `RegionOp::QuantOpen`'s `QuantOpen` at all.
    /// * the **emitted operations** — the statements the region's *decided*
    ///   path performs, in the existing flat `SourceOp` vocabulary. Every
    ///   outcome on this route is a check-time literal, so the region has one
    ///   path; the region therefore **erases** into the straight-line program
    ///   that path runs, exactly as card `LOOP-UNROLL-BUDGET`'s licence erases.
    ///   No `SourceOp`, no CFG terminator and no C backend rule is added.
    ///
    /// The erasure is not an approximation and is not asserted: the emitted
    /// program's release sequence **is** the pack's actions in reverse, which
    /// is the same sequence [`qcore_core::error_region`]'s `discharge` puts in
    /// its own cleanup trace, and `region_cleanup_matches_emitted_releases`
    /// compares the two on every fixture of this route.
    ///
    /// # The v1 boundary, stated rather than hidden
    ///
    /// The body must end in the decided error edge `qcore_check(false, E);` and
    /// the region must carry exactly one catch clause, naming `E`. That is what
    /// makes *every* clause of an admitted region one the model reaches and
    /// decides — the model is path-sensitive, so a handler on a path the region
    /// does not take is a handler nothing decides, and admitting one would be
    /// silent rather than fail-closed. A region with no error edge
    /// (`REGION-NORMAL-EXIT`), a second clause (`REGION-MULTIPLE-CATCH`), a
    /// second region in one entry (`REGION-MULTIPLE`), a failing acquisition as
    /// the edge (`REGION-FAILING-ACQUIRE`), an uncaught raise
    /// (`REGION-PROPAGATE`) and a region inside a branch arm
    /// (`REGION-IN-ARM`) are each refused by name.
    fn lower_error_region(
        &mut self,
        statement: &Located<StatementKind>,
        name: &str,
        blocks: &[SourceBlock],
        catch_bindings: &[Option<String>],
        output: &mut SequenceLowering,
        position: ArmPosition,
    ) -> Result<(), SourceDiagnostic> {
        let span = statement.span;
        let refuse = |message: &str| region_refusal(span, message.to_owned());
        if position.conditional() {
            return Err(region_refusal(
                span,
                "`source.region.v1` admits an `error region` at the top level of an entry body \
                 only: a region inside a branch arm seals a cleanup pack on one path and not on \
                 the other, and the four models that join a branch cannot join on that \
                 (follow-on `REGION-IN-ARM`)"
                    .to_owned(),
            ));
        }
        if self.region_span.is_some() {
            return Err(refuse(
                "`source.region.v1` admits at most one `error region` per entry in this revision \
                 (follow-on `REGION-MULTIPLE`)",
            ));
        }
        let clauses = qcore_frontend::source_region::read_catch_headers(
            self.parsed,
            blocks,
            catch_bindings,
            span,
        )?;
        let [clause] = clauses.as_slice() else {
            return Err(refuse(
                "`source.region.v1` admits exactly one `catch` clause per region in this \
                 revision: the error-region model is path-sensitive, so a clause the region's \
                 decided path does not reach is one the model does not decide, and admitting an \
                 undecided handler would be silent rather than fail-closed (follow-on \
                 `REGION-MULTIPLE-CATCH`)",
            ));
        };
        if !self.errors.contains(&clause.error) {
            return Err(diagnostic(
                SemanticPass::TypedErrors,
                clause.span,
                format!("unknown error type `{}`", clause.error),
            ));
        }
        let body_block = &blocks[0];
        let handler_block = &blocks[1];

        // ---- the region body ----------------------------------------------
        //
        // `frame` mirrors the model's own `RegionFrame::actions`: one entry per
        // resource this region acquired and has not released, in acquisition
        // order. At the error edge it *is* the sealed pack's action list, which
        // is what the handler's discharge releases in reverse.
        let mut region_body = Vec::new();
        let mut frame: Vec<(String, ResourceId)> = Vec::new();
        let mut raised: Option<String> = None;
        for body_statement in &body_block.statements {
            if raised.is_some() {
                return Err(region_refusal(
                    body_statement.span,
                    "the region's error edge is decided at check time, so nothing after it runs; \
                     `source.region.v1` refuses an unreachable statement rather than emitting one"
                        .to_owned(),
                ));
            }
            let tokens = statement_tokens(self.parsed, body_statement.span)?;
            match &body_statement.value {
                StatementKind::Let {
                    binding: Some(binding),
                } => {
                    if tokens
                        .iter()
                        .take_while(|token| token.kind != TokenKind::Symbol(Symbol::Equal))
                        .any(|token| matches!(&token.kind, TokenKind::Word(word) if word == "use"))
                    {
                        return Err(diagnostic(
                            SemanticPass::Usage,
                            body_statement.span,
                            "local usage annotations are not supported in an executable error region",
                        ));
                    }
                    let head = local_binding_head(&tokens).map_err(|message| {
                        diagnostic(SemanticPass::OrdinaryTypes, body_statement.span, message)
                    })?;
                    let Some((resource, succeeds, error)) = parse_acquire(&tokens) else {
                        return Err(region_refusal(
                            body_statement.span,
                            "the only `let` a `source.region.v1` body admits is an acquisition, \
                             `let NAME = qcore_acquire(ID, true, ERROR);`"
                                .to_owned(),
                        ));
                    };
                    check_local_annotation(
                        head.annotation.as_ref(),
                        &ExecutableLocalType::Owned { nominal: None },
                    )
                    .map_err(|message| {
                        diagnostic(SemanticPass::OrdinaryTypes, body_statement.span, message)
                    })?;
                    if !succeeds {
                        return Err(region_refusal(
                            body_statement.span,
                            "a **failing** acquisition inside a region never makes its identity \
                             live, so the region's frame and the emitted program would disagree \
                             about which slot exists; `source.region.v1` spells the region's \
                             error edge as `qcore_check(false, ERROR);` instead (follow-on \
                             `REGION-FAILING-ACQUIRE`)"
                                .to_owned(),
                        ));
                    }
                    if !self.errors.contains(&error) {
                        return Err(diagnostic(
                            SemanticPass::TypedErrors,
                            body_statement.span,
                            format!("unknown error type `{error}`"),
                        ));
                    }
                    self.record_acquisition(
                        body_statement.span,
                        binding,
                        resource,
                        true,
                        error,
                        output,
                    )?;
                    region_body.push(RegionOp::Acquire {
                        resource,
                        // The sequential route models one destructor and it is
                        // total: `SourceOp::Drop` lowers to a release with no
                        // failure edge. A fallible destructor is `Q308`'s
                        // condition and is not expressible here.
                        destructor_noexcept: true,
                    });
                    frame.push((binding.clone(), resource));
                }
                StatementKind::Drop { place: Some(place) } => {
                    let Some(index) = frame.iter().position(|(owner, _)| owner == place) else {
                        return Err(region_refusal(
                            body_statement.span,
                            format!(
                                "`{place}` names no resource this region acquired; \
                                 `source.region.v1` admits a release of a region-local \
                                 acquisition only, because a release of an outer resource is not \
                                 a cleanup duty this region's frame carries"
                            ),
                        ));
                    };
                    let (_, resource) = frame.remove(index);
                    self.record_release(body_statement.span, place, resource, false, output)?;
                    region_body.push(RegionOp::Drop(resource));
                }
                StatementKind::Expression => {
                    if let Some(amount) = parse_region_quantity(
                        &tokens,
                        qcore_frontend::source_region::CREDIT_KEYWORD,
                    ) {
                        region_body.push(RegionOp::AddStrictCredit(amount));
                    } else if let Some(amount) = parse_region_quantity(
                        &tokens,
                        qcore_frontend::source_region::EFFORT_KEYWORD,
                    ) {
                        region_body.push(RegionOp::SpendStrictCredit(amount));
                    } else if let Some((passes, error)) = parse_check(&tokens) {
                        if passes {
                            return Err(region_refusal(
                                body_statement.span,
                                "`source.region.v1` reads `qcore_check(false, ERROR);` as the \
                                 region's error edge; a check decided to pass has no edge to \
                                 model and is refused rather than emitted as a dead cleanup \
                                 branch that would leave the region rather than reach its handler"
                                    .to_owned(),
                            ));
                        }
                        if !self.errors.contains(&error) {
                            return Err(diagnostic(
                                SemanticPass::TypedErrors,
                                body_statement.span,
                                format!("unknown error type `{error}`"),
                            ));
                        }
                        region_body.push(RegionOp::Raise(error.clone()));
                        raised = Some(error);
                    } else {
                        return Err(region_refusal(
                            body_statement.span,
                            "statement is not in `source.region.v1`'s region body: the body \
                             admits `let NAME = qcore_acquire(ID, true, ERROR);`, `drop NAME;`, \
                             `credit N;`, `effort N;` and the error edge \
                             `qcore_check(false, ERROR);`"
                                .to_owned(),
                        ));
                    }
                }
                _ => {
                    return Err(region_refusal(
                        body_statement.span,
                        "statement is not in `source.region.v1`'s region body: the body admits \
                         `let NAME = qcore_acquire(ID, true, ERROR);`, `drop NAME;`, `credit N;`, \
                         `effort N;` and the error edge `qcore_check(false, ERROR);`"
                            .to_owned(),
                    ));
                }
            }
        }
        let Some(raised) = raised else {
            return Err(refuse(
                "`source.region.v1` requires a region body to end in its decided error edge, \
                 `qcore_check(false, ERROR);`: a region that exits normally reaches no catch \
                 clause, so the model decides none of the clause's obligations and the clause \
                 would stand unchecked (follow-on `REGION-NORMAL-EXIT`)",
            ));
        };
        if clause.error != raised {
            return Err(refuse(&format!(
                "the region raises `{raised}` and its one catch clause names `{}`, so the raise \
                 is uncaught; `source.region.v1` admits no error leaving a region in this \
                 revision, because a propagated pack is a second emission shape (follow-on \
                 `REGION-PROPAGATE`)",
                clause.error
            )));
        }

        // ---- the handler ---------------------------------------------------
        let mut handler = Vec::new();
        let mut discharges = 0_usize;
        for handler_statement in &handler_block.statements {
            match &handler_statement.value {
                StatementKind::Discharge {
                    binding: Some(binding),
                } => {
                    handler.push(RegionOp::Discharge(binding.clone()));
                    discharges += 1;
                }
                _ => {
                    return Err(region_refusal(
                        handler_statement.span,
                        "`source.region.v1`'s handler admits `discharge CAPABILITY;` only in \
                         this revision (follow-on `REGION-HANDLER-BODY`)"
                            .to_owned(),
                    ));
                }
            }
        }

        let quant_open = match (&clause.opened_region, &clause.binding) {
            (Some(region), Some(binding)) => Some(RegionQuantOpen {
                region: region.clone(),
                binding: binding.clone(),
            }),
            _ => None,
        };
        self.region_span = Some(span);
        self.region_operations.push(RegionOp::ErrorRegion {
            region: name.to_owned(),
            body: region_body,
            catches: vec![RegionCatchClause {
                error: clause.error.clone(),
                quant_open,
                handler,
            }],
        });

        // ---- the emitted path ----------------------------------------------
        //
        // The pack's actions are `frame`, in acquisition order; discharging it
        // runs them in **reverse**, which is the same order the existing
        // cleanup lowering gives every other fallible edge. A handler that does
        // not discharge emits nothing here and is refused by the model
        // (`Q305`/`Q306`) before this entry's lowering is assembled.
        if discharges > 0 {
            for (place, resource) in frame.iter().rev().cloned().collect::<Vec<_>>() {
                self.record_release(span, &place, resource, false, output)?;
            }
        }
        Ok(())
    }

    /// Lower the `index`th `if` header of a (possibly `else if`-chained)
    /// branch statement into one `Branch` operation in each checked model, and
    /// report whether the branch ends the function on every path.
    ///
    /// An `else if` chain is a right-nested branch: level `index`'s else arm is
    /// level `index + 1`'s whole branch, so the chain needs no separate model.
    ///
    /// # The continuation, and why it moves (card `IF-ARMS`)
    ///
    /// `SourceOp::Return` is not the only linear IR an arm has to satisfy. Four
    /// further models join a branch — `Flow::join_branches`, `OwnershipOp::Branch`,
    /// `ErrorOp::Branch` and the usage model's `UsageExpr::Branch` — and none of
    /// them can say "this arm exited". So an arm that returns is not *left* as a
    /// one-sided join for them to choke on: the statements written **after** the
    /// branch are lowered inside the arm that survives to reach them, which is
    /// where they run. Both arms then end the function, every model sees a
    /// two-armed branch it already understood, and `qcore-core`'s general
    /// join-over-survivors rule is what the arms are validated against.
    ///
    /// The continuation is lowered **exactly once**: `continuation` is a budget
    /// this function may spend, and a shape that would need it twice — an early
    /// return in one arm of a *nested* branch whose sibling also falls through —
    /// is refused at `Q044` rather than duplicating statements (which would
    /// duplicate acquisitions, calls and demands). Follow-on
    /// `IF-ARM-RETURN-NESTED`.
    #[allow(clippy::too_many_arguments)]
    fn lower_branch(
        &mut self,
        span: Span,
        conditions: &[Span],
        branches: &[SourceBlock],
        index: usize,
        output: &mut SequenceLowering,
        continuation: &mut Continuation<'_>,
    ) -> Result<bool, SourceDiagnostic> {
        // The parser produces either one block per condition, or one more (the
        // trailing `else`). Anything else is a shape this profile cannot read.
        if branches.len() != conditions.len() && branches.len() != conditions.len() + 1 {
            return Err(branch_diagnostic(
                "Q044",
                span,
                "an `if` statement must carry one block per condition, plus at most one \
                 trailing `else` block",
            ));
        }
        let Some(condition_span) = conditions.get(index) else {
            return Err(branch_diagnostic(
                "Q044",
                span,
                "an `if` requires a condition",
            ));
        };
        let condition_tokens = statement_tokens(self.parsed, *condition_span)?;
        let context = ConditionContext {
            constants: &self.constants,
            owners: &self.owners,
        };
        let condition = parse_condition_tree(&condition_tokens, &context)
            .map_err(|message| branch_diagnostic("Q045", *condition_span, message))?;

        let entry = self.counters;
        let owners = self.owners.clone();
        let local_usage_names = self.local_usage_names.clone();

        let mut then_output = SequenceLowering::default();
        let mut then_terminates = self.lower_sequence(
            &branches[index].statements,
            &mut then_output,
            ArmPosition::Arm,
        )?;
        let mut then_counters = self.counters;

        // Both arms start from the same custody state. A binding introduced by
        // an arm is arm-local: it is restored away here, so nothing after the
        // join can name it. `drop`/`move` do not remove an owner entry, so the
        // restore loses no information about the incoming bindings.
        self.counters = entry;
        self.owners.clone_from(&owners);
        self.local_usage_names.clone_from(&local_usage_names);

        let mut else_output = SequenceLowering::default();
        let mut else_terminates = if index + 1 < conditions.len() {
            self.lower_branch(
                span,
                conditions,
                branches,
                index + 1,
                &mut else_output,
                continuation,
            )?
        } else if branches.len() > conditions.len() {
            self.lower_sequence(
                &branches[conditions.len()].statements,
                &mut else_output,
                ArmPosition::Arm,
            )?
        } else {
            false
        };
        let mut else_counters = self.counters;

        // Card `IF-ARMS`. Exactly one arm returns: the other is the only arm
        // that reaches the statements after this branch, so those statements are
        // lowered inside it. Spending the budget is what makes both arms end the
        // function, which is what the four models that cannot express an exiting
        // arm need to see.
        if then_terminates != else_terminates {
            let Some(rest) = continuation.take() else {
                return Err(branch_diagnostic(
                    "Q044",
                    span,
                    if continuation.spent {
                        "one arm of this branch returns and the other falls through, but the \
                         statements after the branch have already been placed inside a nested \
                         branch's surviving arm — placing them here as well would duplicate \
                         them (follow-on `IF-ARM-RETURN-NESTED`)"
                    } else {
                        "one arm of this branch returns and the other falls through, but there \
                         are no statements after the branch for the surviving arm to continue \
                         into — every path must reach a `return` (follow-on \
                         `IF-ARM-RETURN-NESTED`)"
                    },
                ));
            };
            // The hoisted statements were written *after* the branch, so an
            // arm's own bindings are not in scope for them: restore the entry
            // scope, and keep only the surviving arm's quantitative state.
            self.owners.clone_from(&owners);
            self.local_usage_names.clone_from(&local_usage_names);
            let hoisted = if then_terminates {
                self.counters = else_counters;
                let terminates =
                    self.lower_sequence(rest, &mut else_output, ArmPosition::HoistedContinuation)?;
                else_counters = self.counters;
                else_terminates = terminates;
                terminates
            } else {
                self.counters = then_counters;
                let terminates =
                    self.lower_sequence(rest, &mut then_output, ArmPosition::HoistedContinuation)?;
                then_counters = self.counters;
                then_terminates = terminates;
                terminates
            };
            if !hoisted {
                return Err(branch_diagnostic(
                    "Q044",
                    span,
                    "one arm of this branch returns, so the statements after the branch belong \
                     to the arm that does not — and they do not reach a `return` of their own \
                     (follow-on `IF-ARM-RETURN-NESTED`)",
                ));
            }
        }

        // The else arm's own bindings are arm-local too: nothing after the
        // join names a binding either arm introduced.
        self.owners = owners;
        self.local_usage_names = local_usage_names;
        self.counters = Counters {
            // The **surviving** arms' live counts agree — the ownership,
            // typed-error, flow and cleanup-lowering joins below each refuse a
            // disagreement — so taking a survivor's is a choice between equals,
            // and taking a returning arm's would be reading a state that never
            // reaches the join. The peak and the cumulative total are joined
            // componentwise upward, the same direction `Flow::join_branches`
            // takes. `reachable` needs no arm-exit case: a `return` already set
            // it false on the arm that took one, so the existing disjunction is
            // the survivors' disjunction.
            live: if then_terminates {
                else_counters.live
            } else {
                then_counters.live
            },
            peak_live: then_counters.peak_live.max(else_counters.peak_live),
            total: then_counters.total.max(else_counters.total),
            reachable: then_counters.reachable || else_counters.reachable,
        };

        // Card `OR-AND-LOWERING`: a condition carrying `&&` or `||` is a *tree*
        // of decidable leaves, and each connective becomes one more branch over
        // the same two arms. The arms are lowered exactly once above and their
        // operation vectors are duplicated structurally here, so a connective
        // adds no statement to the program and no rule to any checker.
        let node = build_connective_branch(span, &condition, &then_output, &else_output)?;
        output.flow = output
            .flow
            .then(&node.flow)
            .map_err(|error| diagnostic(SemanticPass::ResourceFlow, span, error.to_string()))?;
        output.lowering.extend(node.lowering);
        output.ownership.extend(node.ownership);
        output.error_operations.extend(node.error_operations);
        output.demands.extend(node.demands);
        Ok(then_terminates && else_terminates)
    }
}

/// `source.branch.v1`'s condition after card `OR-AND-LOWERING`: a tree of
/// decidable leaves joined by the two short-circuit connectives.
///
/// The connectives exist **only** in this front-end tree. They are eliminated
/// by [`build_connective_branch`] into nested [`SourceOp::Branch`] nodes before
/// any checked model sees them, so `qcore_core` is byte-unchanged by this card
/// and no checker learned a new rule.
#[derive(Clone, Debug, PartialEq, Eq)]
enum ConditionTree {
    Leaf(BranchCondition),
    /// `!OPERAND` (card `BITWISE-CONDITIONS`). Eliminated by an **arm swap**:
    /// `if (!A) T else E` is `if (A) E else T`, which is the program a reader
    /// would have written by hand. No checked model sees a negation and
    /// `qcore_core` gains no constructor for one.
    Not(Box<ConditionTree>),
    /// `LEFT && RIGHT`, binding tighter than [`Self::Or`].
    And(Box<ConditionTree>, Box<ConditionTree>),
    /// `LEFT || RIGHT`.
    Or(Box<ConditionTree>, Box<ConditionTree>),
}

/// Turn one condition tree into the nested branch a reader would have written
/// by hand, over the two arms `then_output` and `else_output`.
///
/// The two rewrites are the textbook short-circuit ones, and the *reason* they
/// are short-circuit here is structural rather than asserted: the right operand
/// is a condition of a branch that sits **inside** an arm of the left operand's
/// branch, so it is evaluated on exactly one of the two paths.
///
/// ```text
/// if (A || B) T else E   ==>   if (A) { T } else { if (B) { T } else { E } }
/// if (A && B) T else E   ==>   if (A) { if (B) { T } else { E } } else { E }
/// ```
///
/// Every level runs the same four-model join the single-leaf case runs, so a
/// connective cannot smuggle a custody disagreement past the gate: an arm pair
/// that refuses at `Q046` unconnected refuses at every level connected.
fn build_connective_branch(
    span: Span,
    condition: &ConditionTree,
    then_output: &SequenceLowering,
    else_output: &SequenceLowering,
) -> Result<SequenceLowering, SourceDiagnostic> {
    match condition {
        ConditionTree::Leaf(leaf) => join_branch_arms(span, leaf.clone(), then_output, else_output),
        // Card `BITWISE-CONDITIONS`: negation is the **arm swap**, so it is
        // eliminated here at zero cost and `!!A` is literally `A`'s own
        // lowering. Nothing downstream can tell a `!` was written.
        ConditionTree::Not(inner) => build_connective_branch(span, inner, else_output, then_output),
        ConditionTree::And(left, right) => {
            let inner = build_connective_branch(span, right, then_output, else_output)?;
            build_connective_branch(span, left, &inner, else_output)
        }
        ConditionTree::Or(left, right) => {
            let inner = build_connective_branch(span, right, then_output, else_output)?;
            build_connective_branch(span, left, then_output, &inner)
        }
    }
}

/// One branch node: the four-model join card `IF-STATEMENT` landed, factored
/// out so each connective level runs it identically.
fn join_branch_arms(
    span: Span,
    condition: BranchCondition,
    then_output: &SequenceLowering,
    else_output: &SequenceLowering,
) -> Result<SequenceLowering, SourceDiagnostic> {
    // Each model's own branch join. All four predate card `IF-STATEMENT`: only
    // `SourceOp::Branch` is new, because `cleanup_lowering` was the one
    // linear IR with no branch constructor.
    let flow = then_output
        .flow
        .join_branches(&else_output.flow)
        .map_err(|error| {
            branch_diagnostic(
                "Q046",
                span,
                format!("the arms of this branch do not join in the resource-flow model: {error}"),
            )
        })?;
    let mut node = SequenceLowering {
        flow,
        ..SequenceLowering::default()
    };
    node.lowering.push(SourceOp::Branch {
        condition,
        then_arm: then_output.lowering.clone(),
        else_arm: else_output.lowering.clone(),
    });
    node.ownership.push(OwnershipOp::Branch {
        then_ops: then_output.ownership.clone(),
        else_ops: else_output.ownership.clone(),
    });
    node.error_operations.push(ErrorOp::Branch {
        then_ops: then_output.error_operations.clone(),
        else_ops: else_output.error_operations.clone(),
    });
    // Card `IF-STATEMENT` could write a flat `Sequence` here and set the pair's
    // flag `false`, because an arm admitted no call. Card `IF-ARM-CALL` admits
    // one, so each arm runs the same `may_exit` unfolding the entry body runs —
    // otherwise the arm's demands would assert that the statements after its
    // call always run — and the joined node inherits the flag from its arms: the
    // branch may exit early exactly when one of its arms may.
    if !then_output.demands.is_empty() || !else_output.demands.is_empty() {
        let may_exit = then_output
            .demands
            .iter()
            .chain(&else_output.demands)
            .any(|(_, exits)| *exits);
        node.demands.push((
            UsageExpr::Branch {
                condition: Box::new(UsageExpr::Unit),
                then_expression: Box::new(unfold_exit_demands(then_output.demands.clone())),
                else_expression: Box::new(unfold_exit_demands(else_output.demands.clone())),
            },
            may_exit,
        ));
    }
    Ok(node)
}

/// Present a cleanup-lowering refusal, routing the one condition card
/// `IF-STATEMENT` added to the branch pass that owns it.
///
/// The custody join is checked by four models — resource flow, ownership,
/// typed errors and the cleanup lowering — and each of them has carried its own
/// branch join since its own cycle. Only the cleanup lowering's join is new, so
/// only its condition is re-presented here: a disagreement in the **custody
/// state** is `Q046` whichever model notices it first, which is what makes
/// `Q046` one condition rather than a code that fires sometimes.
///
/// Card `IF-STATEMENT` added a second condition here, `ReturnInBranchArm`. Card
/// `IF-ARMS` admits an arm return and that refusal no longer exists; the shapes
/// the widening cannot place are refused in `lower_branch`, where the branch and
/// its continuation are both in scope and a useful span is available.
///
/// The ownership model's `Q103` is deliberately **not** re-presented: it
/// refuses a different condition — two arms leaving different resource-identity
/// ledgers — which is not a custody-state disagreement and keeps its own code.
fn branch_aware_lowering_diagnostic(
    span: Span,
    error: &qcore_core::cleanup_lowering::LoweringError,
) -> SourceDiagnostic {
    use qcore_core::cleanup_lowering::LoweringError;
    match error {
        LoweringError::BranchArmsDisagree {
            then_live,
            else_live,
            then_transferred,
            else_transferred,
        } => branch_diagnostic(
            "Q046",
            span,
            format!(
                "the arms of a branch do not join: the `then` arm reaches it with live \
                 {then_live:?} and transferred {then_transferred:?}, the `else` arm with live \
                 {else_live:?} and transferred {else_transferred:?}. Both arms must reach the \
                 join in the same custody state"
            ),
        ),
        other => diagnostic(SemanticPass::CleanupLowering, span, other.to_string()),
    }
}

/// A branch-pass diagnostic at one of its specific codes. The generic `Q044`
/// is also the pass's presentation code (spec §L21).
fn branch_diagnostic(
    code: &'static str,
    span: Span,
    message: impl Into<String>,
) -> SourceDiagnostic {
    SourceDiagnostic {
        code,
        pass: SemanticPass::Branch,
        span,
        message: message.into(),
    }
}

/// `source.branch.v1`'s decidable condition tier.
///
/// Card `IF-STATEMENT` admitted a single leaf — `true`, `false`, or a
/// comparison of two signed 32-bit integer literals. Card `OR-AND-LOWERING`
/// widens that to a **connective tree** over the same leaves:
///
/// Card `BITWISE-CONDITIONS` widens it again — with unary `!`, the three
/// bitwise operators inside a comparison operand, and names bound to integer
/// literals — and the whole tier is now one **two-sorted** grammar:
///
/// ```ebnf
/// cond     = conj , { "||" , conj } ;                 (* 1, left  *)
/// conj     = neg  , { "&&" , neg  } ;                 (* 2, left  *)
/// neg      = "!" , neg | cmp ;                        (* 3, right *)
/// cmp      = int , [ op , int ] ;                     (* 4, none  *)
/// int      = xor , { "|" , xor } ;                    (* 5, left  *)
/// xor      = and , { "^" , and } ;                    (* 6, left  *)
/// and      = atom , { "&" , atom } ;                  (* 7, left  *)
/// atom     = "(" , cond , ")" | "true" | "false"
///          | [ "-" ] , INTEGER | NAME ;
/// ```
///
/// **Precedence is declared, not inherited from a parser accident**, and the
/// rule for declaring it is stated once: *a precedence is declared only where
/// it agrees with C; where this tier's two sorts would force a different
/// ranking than C's, the unparenthesized spelling is refused rather than
/// silently redefined.* Levels 1, 2, 5, 6 and 7 agree with C and are declared.
/// Levels 3 and 4 do not — C ranks `!` tighter and `|`/`^`/`&` looser than the
/// comparisons, while a tier in which a comparison yields a condition and a
/// condition is never an integer operand must rank them the other way — so
/// `!A < B` and `A | B == C` are **refused** (`Q045`) and `!(A < B)` and
/// `(A | B) == C` are the spellings.
///
/// `|>` is still never given a relative precedence: it is not in the tier at
/// all, and a condition mixing a pipeline with anything is refused.
///
/// The tier stays **decidable**: every atom is a literal or a name the front
/// end substitutes by a literal, so evaluation is state-free and the
/// translation certificate still compares one reference, one CFG and one native
/// observation (`IF-RUNTIME-CONDITION`).
fn parse_condition_tree(
    tokens: &[Token],
    context: &ConditionContext<'_>,
) -> Result<ConditionTree, String> {
    let (parsed, rest) = parse_disjunction(tokens, context)?;
    if !rest.is_empty() {
        return Err(TIER.to_owned());
    }
    match parsed.node {
        Node::Cond(tree) => Ok(tree),
        // The tier has no integer-to-Boolean coercion: `if (mask & 4)` is a
        // C idiom this revision deliberately does not inherit, because
        // "non-zero is true" is a rule the checked models do not have.
        Node::Int(_) => Err(
            "an integer is not a condition in `source.branch.v1`: there is no \
             integer-to-Boolean coercion, so write the comparison you mean \
             (`(mask & 4) != 0`) rather than relying on `non-zero is true`"
                .to_owned(),
        ),
    }
}

/// The generic refusal text card `IF-STATEMENT` shipped, widened once by
/// `OR-AND-LOWERING` and once here. It is the message for a *structural* parse
/// failure; every sort and grouping refusal below carries its own text instead.
const TIER: &str = "`source.branch.v1` admits `true`, `false` and comparisons of two signed \
     32-bit integer literals (`==`, `!=`, `<`, `<=`, `>`, `>=`), combined with \
     the connectives `&&` and `||`, negated with `!` and grouped with \
     parentheses, as a condition. A comparison operand may be a bitwise \
     expression over `|`, `^` and `&` when it is parenthesized, or a name bound \
     to an integer literal earlier in this body. A call, a pipeline or any \
     other operator is outside the decidable tier";

/// What a name written inside a condition may resolve to.
///
/// Card `BITWISE-CONDITIONS` admits exactly one kind: a **check-time constant
/// binding**, `let NAME = INTEGER;`, whose value the front end substitutes here.
/// A resource binding is deliberately *not* admitted — no custody-bearing value
/// gains a position in this tier — and it is told apart from an unbound name so
/// the refusal can say which rule it broke.
struct ConditionContext<'a> {
    constants: &'a BTreeMap<String, i32>,
    owners: &'a BTreeMap<String, ResourceId>,
}

/// One parsed node of the unified condition/integer grammar.
///
/// The tier has **two sorts** and **one** parser: a sort mismatch is a refusal
/// rather than a second parse, which is what keeps the parse linear and the
/// refusal specific.
enum Node {
    Cond(ConditionTree),
    Int(IntExpr),
}

struct Parsed {
    node: Node,
    /// The node's outermost spelling needs no grouping to be read the way this
    /// tier reads it — a parenthesised group, a literal, a name, or a `!`
    /// applied to one of those.
    ///
    /// This flag is the whole mechanism behind the card's headline decision:
    /// **QCore declares a precedence only where it agrees with C, and refuses
    /// the unparenthesized spelling where it would have to diverge.** C ranks
    /// `|`, `^` and `&` *looser* than the comparisons; this tier must rank them
    /// tighter, because a comparison yields a condition and no condition is an
    /// integer operand. Rather than silently redefine `flags | MASK == 0`, an
    /// ungrouped bitwise operand of a comparison is refused. The same holds for
    /// `!`, which C ranks tighter than every comparison.
    grouped: bool,
}

impl Parsed {
    const fn cond(tree: ConditionTree, grouped: bool) -> Self {
        Self {
            node: Node::Cond(tree),
            grouped,
        }
    }

    const fn int(value: IntExpr, grouped: bool) -> Self {
        Self {
            node: Node::Int(value),
            grouped,
        }
    }

    fn expect_cond(self, operator: &str) -> Result<ConditionTree, String> {
        match self.node {
            Node::Cond(tree) => Ok(tree),
            Node::Int(_) => Err(format!(
                "`{operator}` combines conditions, and this operand is an integer. \
                 `source.branch.v1` has no integer-to-Boolean coercion"
            )),
        }
    }

    fn expect_int(self, operator: &str) -> Result<IntExpr, String> {
        match self.node {
            Node::Int(value) => Ok(value),
            Node::Cond(_) => Err(format!(
                "`{operator}` combines integers, and this operand is a condition. \
                 `source.branch.v1` has no Boolean-to-integer coercion"
            )),
        }
    }
}

type Step<'a> = Result<(Parsed, &'a [Token]), String>;

/// Level 1, loosest: `||`, left-associative, over conditions.
fn parse_disjunction<'a>(tokens: &'a [Token], context: &ConditionContext<'_>) -> Step<'a> {
    let (mut left, mut rest) = parse_conjunction(tokens, context)?;
    while let Some(after) = eat_symbol(rest, Symbol::PipePipe) {
        let tree = left.expect_cond("||")?;
        let (right, next) = parse_conjunction(after, context)?;
        left = Parsed::cond(
            ConditionTree::Or(Box::new(tree), Box::new(right.expect_cond("||")?)),
            false,
        );
        rest = next;
    }
    Ok((left, rest))
}

/// Level 2: `&&`, left-associative, binding tighter than `||` exactly as in C.
fn parse_conjunction<'a>(tokens: &'a [Token], context: &ConditionContext<'_>) -> Step<'a> {
    let (mut left, mut rest) = parse_negation(tokens, context)?;
    while let Some(after) = eat_symbol(rest, Symbol::AndAnd) {
        let tree = left.expect_cond("&&")?;
        let (right, next) = parse_negation(after, context)?;
        left = Parsed::cond(
            ConditionTree::And(Box::new(tree), Box::new(right.expect_cond("&&")?)),
            false,
        );
        rest = next;
    }
    Ok((left, rest))
}

/// Level 3: prefix `!`, right-associative, binding tighter than `&&` exactly as
/// in C (card `BITWISE-CONDITIONS`).
///
/// Its operand must be **grouped**: C ranks `!` tighter than every comparison,
/// so `!a < b` is `(!a) < b` there and would have to be `!(a < b)` here. The
/// spelling is refused rather than redefined.
fn parse_negation<'a>(tokens: &'a [Token], context: &ConditionContext<'_>) -> Step<'a> {
    let Some(after) = eat_symbol(tokens, Symbol::Bang) else {
        return parse_comparison(tokens, context);
    };
    let (operand, rest) = parse_negation(after, context)?;
    if !operand.grouped {
        return Err(
            "`!` negates a grouped condition: C binds `!` tighter than every comparison \
             while this tier would have to bind it looser, so write `!(A < B)` rather \
             than `!A < B` and let the parentheses say which you mean"
                .to_owned(),
        );
    }
    Ok((
        Parsed::cond(
            ConditionTree::Not(Box::new(operand.expect_cond("!")?)),
            true,
        ),
        rest,
    ))
}

/// Level 4: the six comparisons, **non-associative**, integer × integer →
/// condition. Card `IF-STATEMENT` shipped this level over bare literals; card
/// `BITWISE-CONDITIONS` widens each operand to the integer tier below.
fn parse_comparison<'a>(tokens: &'a [Token], context: &ConditionContext<'_>) -> Step<'a> {
    let (left, rest) = parse_bit_or(tokens, context)?;
    let (operator, rest) = match rest.first().map(|token| &token.kind) {
        Some(TokenKind::Symbol(Symbol::EqualEqual)) => (CompareOp::Equal, &rest[1..]),
        Some(TokenKind::Symbol(Symbol::NotEqual)) => (CompareOp::NotEqual, &rest[1..]),
        Some(TokenKind::Symbol(Symbol::Less)) => (CompareOp::Less, &rest[1..]),
        Some(TokenKind::Symbol(Symbol::LessEqual)) => (CompareOp::LessEqual, &rest[1..]),
        Some(TokenKind::Symbol(Symbol::Greater)) => (CompareOp::Greater, &rest[1..]),
        Some(TokenKind::Symbol(Symbol::GreaterEqual)) => (CompareOp::GreaterEqual, &rest[1..]),
        // No comparison here: hand the operand up unchanged, whatever its sort.
        _ => return Ok((left, rest)),
    };
    let left = compare_operand(left)?;
    let (right, rest) = parse_bit_or(rest, context)?;
    let right = compare_operand(right)?;
    Ok((
        Parsed::cond(
            ConditionTree::Leaf(BranchCondition::Compare {
                left,
                operator,
                right,
            }),
            false,
        ),
        rest,
    ))
}

/// The card's headline refusal: a comparison operand that is an ungrouped
/// bitwise expression. See [`Parsed::grouped`].
fn compare_operand(operand: Parsed) -> Result<IntExpr, String> {
    if !operand.grouped {
        return Err(
            "a bitwise comparison operand must be parenthesized: C ranks `|`, `^` and `&` \
             looser than the comparisons while this tier must rank them tighter, so \
             `flags | MASK == 0` would mean one thing to a C reader and another here. \
             Write `(flags | MASK) == 0` and let the parentheses say which you mean"
                .to_owned(),
        );
    }
    operand.expect_int("a comparison")
}

/// Levels 5–7: `|`, then `^`, then `&`, each left-associative, in **C's own
/// order** — which is why this ranking is declared rather than refused.
fn parse_bit_or<'a>(tokens: &'a [Token], context: &ConditionContext<'_>) -> Step<'a> {
    parse_bitwise_level(tokens, context, 0)
}

/// `|` is level 0 of this table, `^` level 1, `&` level 2; level 3 is the atom.
const BITWISE_LEVELS: [(Symbol, BitwiseOp, &str); 3] = [
    (Symbol::Pipe, BitwiseOp::Or, "|"),
    (Symbol::Caret, BitwiseOp::Xor, "^"),
    (Symbol::Ampersand, BitwiseOp::And, "&"),
];

fn parse_bitwise_level<'a>(
    tokens: &'a [Token],
    context: &ConditionContext<'_>,
    level: usize,
) -> Step<'a> {
    let Some((symbol, operator, spelling)) = BITWISE_LEVELS.get(level).copied() else {
        return parse_condition_atom(tokens, context);
    };
    let (mut left, mut rest) = parse_bitwise_level(tokens, context, level + 1)?;
    while let Some(after) = eat_symbol(rest, symbol) {
        let value = left.expect_int(spelling)?;
        let (right, next) = parse_bitwise_level(after, context, level + 1)?;
        left = Parsed::int(
            IntExpr::Bitwise {
                operator,
                left: Box::new(value),
                right: Box::new(right.expect_int(spelling)?),
            },
            false,
        );
        rest = next;
    }
    Ok((left, rest))
}

/// The atom level: a parenthesised group of either sort, a Boolean literal, a
/// signed integer literal, or a name bound to one earlier in this body.
fn parse_condition_atom<'a>(tokens: &'a [Token], context: &ConditionContext<'_>) -> Step<'a> {
    if let Some(inner) = eat_symbol(tokens, Symbol::LeftParen) {
        let (parsed, rest) = parse_disjunction(inner, context)?;
        let Some(after) = eat_symbol(rest, Symbol::RightParen) else {
            return Err(TIER.to_owned());
        };
        // A group is grouped whatever its sort: that is the whole point of
        // writing it.
        return Ok((
            Parsed {
                node: parsed.node,
                grouped: true,
            },
            after,
        ));
    }
    match tokens.first().map(|token| &token.kind) {
        Some(TokenKind::Word(word)) => {
            if let Some(value) = parse_bool(word) {
                return Ok((
                    Parsed::cond(ConditionTree::Leaf(BranchCondition::Literal(value)), true),
                    &tokens[1..],
                ));
            }
            if let Some(value) = context.constants.get(word) {
                return Ok((Parsed::int(IntExpr::Literal(*value), true), &tokens[1..]));
            }
            if context.owners.contains_key(word) {
                // The custody rule, stated at the position it protects: an
                // identity is not a number, and admitting one here would put a
                // resource in an expression tier for the first time.
                return Err(format!(
                    "`{word}` names a resource, and a resource identity is not an integer. \
                     A `source.branch.v1` condition reads only integer literals and names \
                     bound to one by `let {word} = INTEGER;`"
                ));
            }
            Err(format!(
                "`{word}` is not bound to an integer literal in this body. A \
                 `source.branch.v1` condition reads `true`, `false`, integer literals and \
                 names introduced by `let NAME = INTEGER;` earlier in this entry"
            ))
        }
        Some(TokenKind::Symbol(Symbol::Minus)) => {
            let Some(TokenKind::Integer(value)) = tokens.get(1).map(|token| &token.kind) else {
                return Err(TIER.to_owned());
            };
            // Parse the final signed spelling: the minimum's magnitude is not an i32.
            let Ok(value) = format!("-{value}").parse::<i32>() else {
                return Err(TIER.to_owned());
            };
            Ok((Parsed::int(IntExpr::Literal(value), true), &tokens[2..]))
        }
        Some(TokenKind::Integer(value)) => {
            let Ok(value) = value.parse::<i32>() else {
                return Err(TIER.to_owned());
            };
            Ok((Parsed::int(IntExpr::Literal(value), true), &tokens[1..]))
        }
        _ => Err(TIER.to_owned()),
    }
}

fn eat_symbol(tokens: &[Token], symbol: Symbol) -> Option<&[Token]> {
    match tokens.first()?.kind {
        TokenKind::Symbol(found) if found == symbol => Some(&tokens[1..]),
        _ => None,
    }
}

/// `let NAME = INTEGER;` — a **check-time constant binding** (card
/// `BITWISE-CONDITIONS`), the decidable subset of follow-on
/// `IF-CONDITION-BINDINGS`.
///
/// The binding is custody-free by construction: its initializer is a signed
/// 32-bit literal, so it opens no obligation, enters no ledger and takes no
/// position in any of the four checked models. It contributes **no operation**
/// — the name is substituted for its value where a condition reads it, exactly
/// as a connective is eliminated before any model sees it — so a program that
/// writes one emits the C it would have emitted with the literal written out.
///
/// Returned as `Some(value)` only for the exact shape `= INTEGER ;` or
/// `= - INTEGER ;`; anything else keeps the executable subset's existing
/// `qcore_acquire` refusal.
fn parse_constant_binding(tokens: &[Token], equal: usize) -> Option<i32> {
    let kinds: Vec<&TokenKind> = tokens[equal..]
        .iter()
        .map(|token| &token.kind)
        .filter(|kind| **kind != TokenKind::Eof)
        .collect();
    match kinds.as_slice() {
        [
            TokenKind::Symbol(Symbol::Equal),
            TokenKind::Integer(value),
            TokenKind::Symbol(Symbol::Semicolon),
        ] => value.parse().ok(),
        [
            TokenKind::Symbol(Symbol::Equal),
            TokenKind::Symbol(Symbol::Minus),
            TokenKind::Integer(value),
            TokenKind::Symbol(Symbol::Semicolon),
        ] => format!("-{value}").parse().ok(),
        _ => None,
    }
}

fn mark_recursive_contracts(functions: &mut [FunctionSemantics]) {
    let graph = functions
        .iter()
        .map(|function| {
            (
                function.symbol,
                function
                    .calls
                    .iter()
                    .map(|call| call.callee)
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    for function in functions {
        if reaches(
            function.symbol,
            function.symbol,
            &graph,
            &mut BTreeSet::new(),
        ) {
            function.recursive_contract = Some(FunctionSummary {
                effects: EffectRow::new([Effect::Alloc]),
                flow: Flow::zero(),
                parameters: function.parameters.clone(),
            });
        }
    }
}

fn reaches(
    start: qcore_frontend::frontend::SymbolId,
    target: qcore_frontend::frontend::SymbolId,
    graph: &BTreeMap<qcore_frontend::frontend::SymbolId, Vec<qcore_frontend::frontend::SymbolId>>,
    visited: &mut BTreeSet<qcore_frontend::frontend::SymbolId>,
) -> bool {
    graph.get(&start).is_some_and(|next| {
        next.iter().any(|symbol| {
            *symbol == target
                || (visited.insert(*symbol) && reaches(*symbol, target, graph, visited))
        })
    })
}

fn validate_emittable_calls(
    entry: qcore_frontend::frontend::SymbolId,
    calls: &BTreeMap<qcore_frontend::frontend::SymbolId, Vec<CallSite>>,
    checked: &CheckedProgram,
) -> Result<(), SourceDiagnostic> {
    fn visit(
        symbol: qcore_frontend::frontend::SymbolId,
        calls: &BTreeMap<qcore_frontend::frontend::SymbolId, Vec<CallSite>>,
        checked: &CheckedProgram,
        visited: &mut BTreeSet<qcore_frontend::frontend::SymbolId>,
    ) -> Result<(), SourceDiagnostic> {
        if !visited.insert(symbol) {
            return Ok(());
        }
        for call in calls.get(&symbol).into_iter().flatten() {
            if checked.recursive_functions.contains(&call.callee) {
                return Err(diagnostic(
                    SemanticPass::CEmission,
                    call.span,
                    "recursive calls are not in the Cycle 12 executable C subset",
                ));
            }
            let callee = &checked.functions[&call.callee];
            let observation = qcore_core::interpret_cfg(&callee.cfg).map_err(|error| {
                diagnostic(SemanticPass::CleanupLowering, call.span, error.to_string())
            })?;
            let observationally_pure = observation.trace.is_empty()
                && matches!(
                    observation.outcome,
                    qcore_core::ExecutionOutcome::Ok { payload, .. } if payload.is_empty()
                );
            if !observationally_pure {
                return Err(diagnostic(
                    SemanticPass::CEmission,
                    call.span,
                    "only observationally pure source calls can be emitted in Cycle 12",
                ));
            }
            visit(call.callee, calls, checked, visited)?;
        }
        Ok(())
    }
    visit(entry, calls, checked, &mut BTreeSet::new())
}

/// The tokens of one statement (or one `if` condition), read from the program's
/// own **post-desugaring** token stream.
///
/// Card `PIPE-INTRINSIC-STAGE`. Until this card the sequential route re-lexed
/// the raw source slice, which is the one place in the compiler where a later
/// phase could still tell a `|>` had been written: `source.pipe.v1` rewrites the
/// *token stream* before parsing, so a re-lex of the original bytes hands the
/// lowering the reader's pipeline instead of the statement it desugared to. The
/// observable consequence was that this route's own two call-shaped intrinsics
/// — `qcore_acquire` and `qcore_check` — refused through a pipe while their
/// hand-written twins were admitted, with a message naming the hand form the
/// reader had in fact written.
///
/// Reading the parsed program's stream instead is **byte-for-byte identical on
/// every pipe-free file**, because `SourceProgram::lexical_tokens` is the lex of
/// that same source and a statement's tokens are exactly those whose spans lie
/// inside it. What it adds is the synthesized delimiters and hygienic
/// temporaries a desugared pipeline contributes, each anchored zero-width inside
/// the statement's own span (card `PIPE-CHAIN`), in the desugared order. No
/// consumer of this function reads a returned token's span — every diagnostic
/// here is attributed to the statement or condition span — so the change from
/// slice-relative offsets to absolute ones is not observable.
fn statement_tokens(program: &SourceProgram, span: Span) -> Result<Vec<Token>, SourceDiagnostic> {
    Ok(program
        .lexical_tokens()
        .iter()
        .filter(|token| span.start <= token.span.start && token.span.end <= span.end)
        .cloned()
        .collect())
}

fn parse_acquire(tokens: &[Token]) -> Option<(ResourceId, bool, String)> {
    let equal = tokens
        .iter()
        .position(|token| token.kind == TokenKind::Symbol(Symbol::Equal))?;
    let args = &tokens[equal + 1..];
    match args {
        [
            Token {
                kind: TokenKind::Word(name),
                ..
            },
            Token {
                kind: TokenKind::Symbol(Symbol::LeftParen),
                ..
            },
            Token {
                kind: TokenKind::Integer(id),
                ..
            },
            Token {
                kind: TokenKind::Symbol(Symbol::Comma),
                ..
            },
            Token {
                kind: TokenKind::Word(succeeds),
                ..
            },
            Token {
                kind: TokenKind::Symbol(Symbol::Comma),
                ..
            },
            Token {
                kind: TokenKind::Word(error),
                ..
            },
            Token {
                kind: TokenKind::Symbol(Symbol::RightParen),
                ..
            },
            Token {
                kind: TokenKind::Symbol(Symbol::Semicolon),
                ..
            },
        ] if name == "qcore_acquire" => {
            Some((id.parse().ok()?, parse_bool(succeeds)?, error.clone()))
        }
        _ => None,
    }
}

fn parse_check(tokens: &[Token]) -> Option<(bool, String)> {
    match tokens {
        [
            Token {
                kind: TokenKind::Word(name),
                ..
            },
            Token {
                kind: TokenKind::Symbol(Symbol::LeftParen),
                ..
            },
            Token {
                kind: TokenKind::Word(passes),
                ..
            },
            Token {
                kind: TokenKind::Symbol(Symbol::Comma),
                ..
            },
            Token {
                kind: TokenKind::Word(error),
                ..
            },
            Token {
                kind: TokenKind::Symbol(Symbol::RightParen),
                ..
            },
            Token {
                kind: TokenKind::Symbol(Symbol::Semicolon),
                ..
            },
        ] if name == "qcore_check" => Some((parse_bool(passes)?, error.clone())),
        _ => None,
    }
}

/// `credit N;` / `effort N;` — one region-body quantity declaration (card
/// `QUANT-REGION-SOURCE`).
///
/// Both words are **already** reserved (card `T08-EPOCH-SRC`), so this form
/// reserves nothing; it is read only inside a `source.region.v1` region body,
/// which is a statement position no program in this repository could reach
/// before this card. The seam with the epoch route is one-way and measured: the
/// epoch route is selected by `epoch {`
/// (`qcore_frontend::source_epoch::has_source_epoch`), which a region body
/// cannot spell, so a region that credits never reaches that route and an
/// epoch's `credit N effort M` never reaches this reader.
///
/// The amount is a non-negative integer **literal**. The model's quantity is a
/// `u64` and its arithmetic is checked
/// (`CreditOverflow` / `InsufficientCredit`), so nothing here has to be.
fn parse_region_quantity(tokens: &[Token], keyword: &str) -> Option<u64> {
    match tokens {
        [
            Token {
                kind: TokenKind::Word(word),
                ..
            },
            Token {
                kind: TokenKind::Integer(amount),
                ..
            },
            Token {
                kind: TokenKind::Symbol(Symbol::Semicolon),
                ..
            },
        ] if word == keyword => amount.parse().ok(),
        _ => None,
    }
}

/// A `source.region.v1` shape refusal. Always `Q302`: every condition this
/// route adds is a condition of the `ErrorRegion` pass's own generic
/// presentation code, allocated under the standing `Q-NAMESPACE-DECISION` ice
/// box rather than given a code of its own.
fn region_refusal(span: Span, message: String) -> SourceDiagnostic {
    SourceDiagnostic {
        code: "Q302",
        pass: SemanticPass::ErrorRegion,
        span,
        message,
    }
}

fn parse_zero_argument_call(tokens: &[Token]) -> Option<String> {
    match tokens {
        [
            Token {
                kind: TokenKind::Word(name),
                ..
            },
            Token {
                kind: TokenKind::Symbol(Symbol::LeftParen),
                ..
            },
            Token {
                kind: TokenKind::Symbol(Symbol::RightParen),
                ..
            },
            Token {
                kind: TokenKind::Symbol(Symbol::Semicolon),
                ..
            },
        ] => Some(name.clone()),
        _ => None,
    }
}

fn parse_return(tokens: &[Token], result_type: ExecutableResultType) -> Option<i32> {
    match tokens {
        [
            Token {
                kind: TokenKind::Word(word),
                ..
            },
            Token {
                kind: TokenKind::Symbol(Symbol::Semicolon),
                ..
            },
        ] if word == "return" && result_type == ExecutableResultType::Unit => Some(0),
        [
            Token {
                kind: TokenKind::Word(word),
                ..
            },
            Token {
                kind: TokenKind::Integer(value),
                ..
            },
            Token {
                kind: TokenKind::Symbol(Symbol::Semicolon),
                ..
            },
        ] if word == "return" && result_type == ExecutableResultType::I32 => value.parse().ok(),
        [
            Token {
                kind: TokenKind::Word(word),
                ..
            },
            Token {
                kind: TokenKind::Symbol(Symbol::Minus),
                ..
            },
            Token {
                kind: TokenKind::Integer(value),
                ..
            },
            Token {
                kind: TokenKind::Symbol(Symbol::Semicolon),
                ..
            },
        ] if word == "return" && result_type == ExecutableResultType::I32 => {
            // Parse the final signed spelling: the minimum's magnitude is not an i32.
            format!("-{value}").parse().ok()
        }
        _ => None,
    }
}

fn parse_bool(value: &str) -> Option<bool> {
    match value {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

fn select_entry(
    typed: &TypedSourceProgram,
    options: &CompileOptions,
) -> Result<String, FileDiagnostic> {
    let functions = typed
        .declarations
        .iter()
        .filter(|declaration| declaration.kind == DeclarationKind::Function)
        .collect::<Vec<_>>();
    let requested = options
        .entry
        .as_deref()
        .or_else(|| typed.globals.contains_key("main").then_some("main"));
    let selected = match requested {
        Some(name) => functions
            .iter()
            .find(|function| function.name == name)
            .copied(),
        None if functions.len() == 1 => functions.first().copied(),
        None => None,
    };
    selected
        .map(|function| function.name.clone())
        .ok_or_else(|| {
            plain_diagnostic(
                "Q401",
                match requested {
                    Some(name) => format!("entry function `{name}` was not found"),
                    None => "module has no unambiguous entry function; pass `--entry NAME`".into(),
                },
            )
        })
}

fn compile_task_source(
    parsed: &SourceProgram,
    typed: &TypedSourceProgram,
    mode: SourceModuleMode,
    options: &CompileOptions,
    ranges: &[FileRange],
    target: Option<&RecordedTarget>,
) -> Result<CompiledModule, Vec<FileDiagnostic>> {
    // The composed-epoch route's task half. Card `W48-QR-SCALAR-TYPES-R6`
    // (`W48-QR-TYPES-EPOCH-TASKS`) threads the builder's recorded target to it,
    // so `isize`/`usize` have their recorded width there as on the task route,
    // and refuse without one exactly as they always did.
    task_route(parsed, typed, mode, options, ranges, false, target)
}

fn compile_bitfield_task_source(
    parsed: &SourceProgram,
    typed: &TypedSourceProgram,
    mode: SourceModuleMode,
    options: &CompileOptions,
    ranges: &[FileRange],
) -> Result<CompiledModule, Vec<FileDiagnostic>> {
    if mode != SourceModuleMode::Executable {
        return Err(vec![plain_diagnostic(
            "Q600",
            "entry-free bit-field task modules require additional source lowering",
        )]);
    }
    let entry = select_entry(typed, options).map_err(|error| vec![error])?;
    let lowered =
        qcore_frontend::source_bitfield_concurrency::lower_source_bitfield_tasks(parsed, &entry)
            .map_err(|errors| {
                errors
                    .into_iter()
                    .map(|error| map_diagnostic(error, ranges))
                    .collect::<Vec<_>>()
            })?;
    let certificate =
        qcore_core::disjoint_footprint::check_disjoint_footprint_program(&lowered.program)
            .map_err(|errors| {
                errors
                    .into_iter()
                    .map(|error| {
                        map_raw_diagnostic(
                            error.code(),
                            SemanticPass::Concurrency,
                            lowered.scope_span,
                            error.to_string(),
                            ranges,
                        )
                    })
                    .collect::<Vec<_>>()
            })?;
    qcore_core::disjoint_footprint::recheck_disjoint_footprint_certificate(
        &lowered.program,
        &certificate,
    )
    .map_err(|failure| {
        vec![map_raw_diagnostic(
            "Q600",
            SemanticPass::Concurrency,
            lowered.scope_span,
            format!("bit-field footprint certificate failed independent recheck: {failure:?}"),
            ranges,
        )]
    })?;
    Ok(CompiledModule {
        source_bundle: None,
        extension_warnings: Vec::new(),
        checked: CheckedSourceModule::BitfieldTasks(Box::new(CheckedBitfieldTaskModule {
            lowered,
            certificate,
            assumptions: BTreeSet::from([BitfieldTaskAssumption::SpawnAndJoinSucceed]),
        })),
        execution_profile: SourceExecutionProfile::BitfieldConcurrencyV1,
        declaration_count: typed.declarations.len(),
        mode,
        result_profile: SourceResultProfile {
            scalar_results: BTreeMap::from([(entry.clone(), ExecutableResultType::Unit)]),
            owned_buffer_results: Default::default(),
            custody_payloads: CustodyPayloadProfile::NoResourcePayloads,
        },
        entry: Some(entry),
        // This card establishes a checked source-to-concurrency route. Native
        // emission of C bit-fields is a separate target-ABI problem.
        c_source: None,
        source_map: Vec::new(),
        reference_observation: None,
        extern_observation: None,
    })
}

/// `source.std-io-tasks.v1` (card `COMPOSE-CONCURRENCY`).
///
/// The same route function with `composing = true`: one lowering, one task
/// checker, one backend. That is what makes "a module using one feature set
/// keeps its route byte for byte" structural rather than tested — there is no
/// second path for it to diverge on.
fn compile_composed_task_source(
    parsed: &SourceProgram,
    typed: &TypedSourceProgram,
    mode: SourceModuleMode,
    options: &CompileOptions,
    ranges: &[FileRange],
    target: Option<&RecordedTarget>,
) -> Result<CompiledModule, Vec<FileDiagnostic>> {
    task_route(parsed, typed, mode, options, ranges, true, target)
}

fn task_route(
    parsed: &SourceProgram,
    typed: &TypedSourceProgram,
    mode: SourceModuleMode,
    options: &CompileOptions,
    ranges: &[FileRange],
    composing: bool,
    target: Option<&RecordedTarget>,
) -> Result<CompiledModule, Vec<FileDiagnostic>> {
    if mode != SourceModuleMode::Executable {
        return Err(vec![plain_diagnostic(
            "Q500",
            "entry-free scoped task modules require additional source lowering",
        )]);
    }
    let entry = select_entry(typed, options).map_err(|error| vec![error])?;
    let lowered = if composing {
        qcore_frontend::source_tasks::lower_source_composed_tasks_for_target(
            parsed, typed, &entry, target,
        )
    } else {
        qcore_frontend::source_tasks::lower_source_tasks_for_target(parsed, typed, &entry, target)
            .map(
                |tasks| qcore_frontend::source_tasks::LoweredComposedTaskModule {
                    tasks,
                    sites: Vec::new(),
                    declares_io: false,
                    worker_io: Default::default(),
                },
            )
    }
    .map_err(|errors| {
        errors
            .into_iter()
            .map(|error| map_diagnostic(error, ranges))
            .collect::<Vec<_>>()
    })?;
    let task_diagnostics = |errors: Vec<task_execution::TaskDiagnostic>| {
        errors
            .into_iter()
            .map(|error| {
                map_raw_diagnostic(
                    "Q600",
                    SemanticPass::Concurrency,
                    Span {
                        start: error.span.start,
                        end: error.span.end,
                    },
                    error.message,
                    ranges,
                )
            })
            .collect::<Vec<_>>()
    };
    // The task half is checked by `check_task_module` on both routes — through
    // the composed checker when composing, which runs it first and unedited, so
    // composing can never admit a task program the task route refuses.
    let composed = if composing {
        let program = composed_tasks::ComposedTaskProgram {
            tasks: lowered.tasks.clone(),
            sites: lowered.sites.clone(),
            declares_io: lowered.declares_io,
            worker_io: lowered.worker_io.clone(),
        };
        Some(
            composed_tasks::check_composed_task_program(&program).map_err(
                |refusal| match refusal {
                    composed_tasks::ComposedTaskRefusal::Tasks(errors) => task_diagnostics(errors),
                    composed_tasks::ComposedTaskRefusal::Join(errors) => errors
                        .into_iter()
                        .map(|error| {
                            map_raw_diagnostic(
                                "Q050",
                                SemanticPass::ComposedTasks,
                                Span {
                                    start: error.span.start,
                                    end: error.span.end,
                                },
                                error.message,
                                ranges,
                            )
                        })
                        .collect::<Vec<_>>(),
                },
            )?,
        )
    } else {
        None
    };
    let checked = match &composed {
        Some(composed) => composed.tasks().clone(),
        None => task_execution::check_task_module(&lowered.tasks).map_err(task_diagnostics)?,
    };
    // A published composed certificate is never one this driver did not
    // recompute: the join is rederived from the program alone and compared.
    if let Some(composed) = &composed {
        let program = composed.program();
        composed_tasks::recheck_composed_task_certificate(program, composed.certificate())
            .map_err(|failure| {
                vec![plain_diagnostic(
                    "Q050",
                    format!("composed task certificate: {failure}"),
                )]
            })?;
    }
    if !cfg!(any(
        target_os = "linux",
        target_os = "macos",
        target_os = "windows"
    )) {
        return Err(vec![plain_diagnostic(
            "Q500",
            "native scoped task emission currently requires a Linux, macOS or Windows threads target",
        )]);
    }
    let emission = task_c_emit::emit_task_c11_with_source_lines(&checked, &lowered.sites);
    use task_execution::{
        FULL_READ_AUTHORITY, ScalarOperand, ScalarType, TaskInstructionKind, TaskTerminatorKind,
    };
    // A read loan is a strictly stronger claim than owned transfer, so it names
    // its own profile rather than being reported as an owned-task run; counted
    // shares and Buffer writes are stronger again and name a third.
    let read_loan_profile = checked.module().functions.values().any(|function| {
        function
            .parameters
            .iter()
            .any(|(_, kind)| matches!(kind, ScalarType::ReadLoan(_)))
    });
    let read_share_profile = checked.module().functions.values().any(|function| {
        function.blocks.values().any(|block| {
            block.instructions.iter().any(|instruction| {
                matches!(instruction.kind, TaskInstructionKind::WriteBuffer { .. })
            }) || match &block.terminator.kind {
                TaskTerminatorKind::TrySpawn { arguments, .. } => {
                    arguments.iter().any(|argument| {
                        matches!(argument, ScalarOperand::Loan { share, .. }
                            if *share != FULL_READ_AUTHORITY)
                    })
                }
                _ => false,
            }
        })
    });
    // A range write lease transfers write authority over a byte range across
    // the boundary, which no earlier profile does, so it names its own
    // profile (card `WRITE-LEND-TASKS`).
    let write_lease_profile = checked.module().functions.values().any(|function| {
        function
            .parameters
            .iter()
            .any(|(_, kind)| matches!(kind, ScalarType::WriteLease(_)))
            || function
                .blocks
                .values()
                .any(|block| match &block.terminator.kind {
                    TaskTerminatorKind::TrySpawn { arguments, .. } => arguments
                        .iter()
                        .any(|argument| matches!(argument, ScalarOperand::Lease { .. })),
                    _ => false,
                })
    });
    // A typed error payload transfers custody along an error edge, which no
    // earlier profile does, so it names its own profile rather than being
    // reported as an ordinary owned-task run.
    let error_payload_profile = checked
        .module()
        .functions
        .values()
        .any(|function| !function.error_payloads.is_empty());
    let owned_profile = checked.module().functions.values().any(|function| {
        matches!(function.result, ScalarType::OwnedBuffer(_))
            // Card `W48-QR-TASK-RECORD-CAPTURES`: a record result that owns a
            // Buffer transfers custody through join as an owned result does.
            || matches!(function.result, ScalarType::Record(record)
                if checked.module().records.get(&record).is_some_and(|declared| declared.owns()))
            || function
                .parameters
                .iter()
                .any(|(_, kind)| matches!(kind, ScalarType::OwnedBuffer(_)))
            || function.blocks.values().any(|block| {
                matches!(
                    block.terminator.kind,
                    TaskTerminatorKind::TryAllocateBuffer { .. }
                )
            })
    });
    let result_profile = SourceResultProfile {
        scalar_results: checked
            .module()
            .functions
            .values()
            .filter_map(|function| {
                let scalar = match function.result {
                    ScalarType::I32 => ExecutableResultType::I32,
                    ScalarType::Unit => ExecutableResultType::Unit,
                    // Card `W48-QR-SCALAR-TYPES-R4`: a fixed-width worker
                    // result is checked but is not one a caller runs, as on
                    // `source.scalar-calls.v1`.
                    ScalarType::Fixed(_) => return None,
                    // None is an ordinary scalar result. `ReadLoan` and
                    // `WriteLease` are additionally refused as results by the
                    // checker, so this arm is a total match, not a silent
                    // reclassification.
                    ScalarType::OwnedBuffer(_)
                    | ScalarType::ReadLoan(_)
                    | ScalarType::WriteLease(_) => {
                        return None;
                    }
                    // Card `W48-QR-TASK-RECORD-CAPTURES`: a record result is a
                    // worker's, collected by join, never one a caller runs.
                    ScalarType::Record(_) => return None,
                };
                Some((function.name.clone(), scalar))
            })
            .collect(),
        owned_buffer_results: checked
            .module()
            .functions
            .values()
            .filter(|function| matches!(function.result, ScalarType::OwnedBuffer(_)))
            .map(|function| function.name.clone())
            .collect(),
        custody_payloads: if owned_profile || error_payload_profile {
            CustodyPayloadProfile::TypedTaskBuffers
        } else {
            CustodyPayloadProfile::NoResourcePayloads
        },
    };
    let source_map = emission
        .source_map
        .into_iter()
        .map(|mapping| EmittedSourceMap {
            generated_start_line: mapping.generated_start_line,
            generated_end_line: mapping.generated_end_line,
            source: locate(mapping.span.start, ranges),
        })
        .collect();
    Ok(CompiledModule {
        source_bundle: None,
        extension_warnings: Vec::new(),
        checked: match composed {
            Some(composed) => CheckedSourceModule::ComposedTasks(Box::new(composed)),
            None => CheckedSourceModule::ScopedTasks(Box::new(checked)),
        },
        execution_profile: if composing {
            // The composition names its own profile rather than being reported
            // as the task profile it composes: its guarantees are that route's
            // plus the join's, and a consumer must be able to tell.
            SourceExecutionProfile::ComposedStdIoTasksV1
        } else if write_lease_profile {
            SourceExecutionProfile::ScopedWriteLeasesV1
        } else if read_share_profile {
            SourceExecutionProfile::ScopedReadSharesV1
        } else if error_payload_profile {
            SourceExecutionProfile::ScopedErrorPayloadV1
        } else if read_loan_profile {
            SourceExecutionProfile::ScopedReadLoansV1
        } else if owned_profile {
            SourceExecutionProfile::ScopedOwnedTasksV1
        } else {
            SourceExecutionProfile::ScopedCopyTasksV1
        },
        declaration_count: typed.declarations.len(),
        mode,
        result_profile,
        entry: Some(entry),
        c_source: Some(emission.source),
        source_map,
        // Creation is dynamic. A single chosen reference execution is not the
        // evidence that all creation and completion alternatives are checked.
        reference_observation: None,
        extern_observation: None,
    })
}

/// Card `UNSAFE-LINKAGE`. Admit one `unsafe extern fn` declaration, or refuse
/// it with a diagnostic in the `UncheckedLinkage` band.
///
/// Three gates, in order. **`Q034` identity**: a waiver may not sit beside a
/// checkable declaration — an inline `contract { … }` block is an unenforced
/// contract next to a waiver, and a name the *selected environment* declares
/// already has a real one, so the contracted route wins whether or not the two
/// signatures agree. This is LOADER-ABI's rule applied to this surface: where a
/// declaration is visible, it is compared. **`Q033` vocabulary**: every declared
/// spelling must be in the closed §C04.1 ABI table — the *contract* is waived,
/// the ABI is not. **`Q033` table**: the signature must carry no custody, which
/// [`extern_call::unchecked_callee`] decides.
fn admit_unchecked_callee(
    declaration: &qcore_frontend::source_extern::SourceExternDeclaration,
    contracts: Option<&qcore_schemas::contract::ResourceContracts>,
    libraries: &qcore_core::native_library::BoundSelection<'_>,
    ranges: &[FileRange],
    diagnostics: &mut Vec<FileDiagnostic>,
) -> Option<extern_call::ExternCallee> {
    let name = &declaration.name;
    let mut refuse = |code: &'static str, message: String| {
        diagnostics.push(map_raw_diagnostic(
            code,
            SemanticPass::UncheckedLinkage,
            declaration.span,
            message,
            ranges,
        ));
    };
    if declaration.has_inline_contract {
        refuse(
            "Q034",
            format!(
                "`{name}` is declared `unsafe extern fn` and also carries an inline `contract {{ … }}` block; a waiver and an unenforced contract cannot stand together"
            ),
        );
        return None;
    }
    if contracts.is_some_and(|contracts| contracts.contract(name).is_some()) {
        refuse(
            "Q034",
            format!(
                "`{name}` is declared `unsafe extern fn` but the selected extern contract environment declares a contract for it; a contract that exists is not waivable"
            ),
        );
        return None;
    }
    // Card `NATIVE-LIBRARY-CONTRACT-SLICE`, the third face of the same rule, and
    // a **measured non-addition**. `Q034` refuses a waiver for a name the
    // contract *environment* declares; `Q042` refuses a contract for a name the
    // manifest declares contract-less; the missing third case would refuse a
    // waiver for a name the manifest binds to a **contract**.
    //
    // It needs no gate and no code, because two gates already compose to close
    // it: `Q047` refuses a manifest whose contract slice entry the selected
    // environment does not declare, so wherever a manifest binds `name` to a
    // contract, the environment declares `name` — and the `Q034` gate directly
    // above has already refused. The card measured this rather than assuming it
    // (`CONTRACT-SLICE-WAIVER-MIRROR`); a gate here would be unreachable, and an
    // unreachable diagnostic code is a claim a reader cannot check.
    let mut parameters = Vec::with_capacity(declaration.parameter_types.len());
    for (index, spelling) in declaration.parameter_types.iter().enumerate() {
        let Some(abi) = extern_call::AbiType::parse(spelling) else {
            refuse(
                "Q033",
                format!(
                    "`{name}` declares parameter {index} as `{spelling}`, which is not a C ABI type this profile projects; the contract is waived, the ABI is not"
                ),
            );
            return None;
        };
        parameters.push(abi);
    }
    let Some(result) = extern_call::AbiType::parse(&declaration.result_type) else {
        refuse(
            "Q033",
            format!(
                "`{name}` declares result `{}`, which is not a C ABI type this profile projects; the contract is waived, the ABI is not",
                declaration.result_type
            ),
        );
        return None;
    };
    // The admitted surface is decided **first and unchanged**, so a manifest
    // demonstrably cannot widen it: a pointer declaration refuses here whether
    // or not a manifest names the symbol (card `UNSAFE-LINKAGE-ARTIFACT`).
    let callee = match extern_call::unchecked_callee(name, &parameters, result) {
        Ok(callee) => callee,
        Err(refusal) => {
            refuse(
                "Q033",
                format!("`{name}` is not callable through unchecked linkage: {refusal}"),
            );
            return None;
        }
    };
    // Card `NATIVE-LIBRARY-SOURCE-BINDING`. A file that bound a library and
    // declares a symbol some *other* selected manifest binds is refused, not
    // quietly handed the bare premise: the fallback would let a file stop its
    // certificate naming a build by binding a different library, which is
    // `Q041`'s shape — a contradiction rewarded with a premise that says less.
    // An unbound file is never owed this refusal, which is what keeps every
    // pre-card program unmoved.
    if let Some(refusal) = libraries.foreign_symbol(name) {
        diagnostics.push(map_raw_diagnostic(
            "Q066",
            SemanticPass::NativeLibrary,
            declaration.span,
            refusal.to_string(),
            ranges,
        ));
        return None;
    }
    let Some(binding) = libraries.unchecked_binding(name) else {
        // No selected manifest names this symbol, so the waiver keeps the bare
        // premise card `UNSAFE-LINKAGE` published, byte for byte. The absence
        // of an artifact field is itself part of the trust statement: no build
        // was named.
        return Some(callee);
    };
    // The manifest's type gate (design §4.2 at the contract-less setting). A
    // disagreement **refuses**; it does not withdraw to the bare waiver, which
    // would reward a contradiction with a premise that says less.
    if let Err(disagreement) =
        qcore_core::native_library::check_manifest_agreement(binding.entry, &parameters, result)
    {
        diagnostics.push(map_raw_diagnostic(
            "Q041",
            SemanticPass::NativeLibrary,
            declaration.span,
            format!(
                "`{name}` is declared `unsafe extern fn` here but `{}` declares it otherwise: {disagreement}",
                binding.module
            ),
            ranges,
        ));
        return None;
    }
    match extern_call::manifest_unchecked_callee(name, &parameters, result, binding.residence()) {
        Ok(callee) => Some(callee),
        Err(refusal) => {
            refuse(
                "Q033",
                format!("`{name}` is not callable through unchecked linkage: {refusal}"),
            );
            None
        }
    }
}

/// Project one declared contract to the two custody facts design §4.4's
/// closed-allocator-custody check reads (card
/// `NATIVE-LIBRARY-CONTRACT-SLICE`).
///
/// A `resource_stored` parameter is deliberately **not** a discharge: it
/// relocates custody beneath a declared owner and discharges nothing, which is
/// exactly what `ExternAssumption::StoredCustodyUnderDeclaredOwner` records as
/// undischarged. Counting it would let a slice claim closed custody on the
/// strength of a premise nothing in this repository discharges.
fn custody_shape(
    contract: &qcore_schemas::contract::ResourceFunctionContract,
) -> qcore_core::native_library::CustodyShape {
    use qcore_schemas::contract::{ResourceParameterMode, ResourceReturnMode};

    let acquires = matches!(
        contract.result,
        ResourceReturnMode::Owned { .. } | ResourceReturnMode::Reallocated { .. }
    ) || contract
        .parameters
        .iter()
        .any(|mode| matches!(mode, ResourceParameterMode::Out { .. }));
    let discharges = contract.parameters.iter().any(|mode| {
        matches!(
            mode,
            ResourceParameterMode::Consumed { .. } | ResourceParameterMode::Released { .. }
        )
    });
    qcore_core::native_library::CustodyShape {
        acquires,
        discharges,
    }
}

/// Card `UNSAFE-LINKAGE-ARTIFACT`. Load the selected `source.native-library.v1`
/// manifests, or refuse the module.
///
/// A selection that does not load is refused rather than ignored, on card
/// `UNSAFE-LINKAGE` §7.1's measured ground applied to this surface: a silently
/// dropped selection would downgrade every manifest-resident premise to a bare
/// waiver, so shipping a deliberately broken manifest would become a way to
/// stop a certificate saying which build it rests on. The refusal therefore
/// applies whenever the extern route runs, not only when some callee resolves.
fn load_native_libraries(
    options: &CompileOptions,
    span: Span,
    ranges: &[FileRange],
) -> Result<qcore_core::native_library::NativeLibraries, Vec<FileDiagnostic>> {
    use qcore_core::native_library::{NativeLibraries, SelectionRefusal};

    let Some(sources) = options.native_libraries.as_ref() else {
        return Ok(NativeLibraries::default());
    };
    qcore_frontend::source_extension::reject_alias_collisions(sources.keys().map(String::as_str))
        .map_err(|message| {
        vec![map_raw_diagnostic(
            "Q039",
            SemanticPass::NativeLibrary,
            span,
            message,
            ranges,
        )]
    })?;
    for path in sources.keys() {
        qcore_frontend::source_extension::validate_manifest_extension(path).map_err(|message| {
            vec![map_raw_diagnostic(
                "Q039",
                SemanticPass::NativeLibrary,
                span,
                message,
                ranges,
            )]
        })?;
    }
    NativeLibraries::load(sources).map_err(|refusal| {
        let code = match refusal {
            SelectionRefusal::Manifest { .. } => "Q039",
            SelectionRefusal::SymbolBoundTwice { .. } => "Q040",
        };
        vec![map_raw_diagnostic(
            code,
            SemanticPass::NativeLibrary,
            span,
            format!("the selected native-library manifests do not load: {refusal}"),
            ranges,
        )]
    })
}

/// Card `NATIVE-LIBRARY-HEADER`. Check every selected manifest's exported-header
/// binding against the header texts the builder supplied, or refuse the module.
///
/// **Where the digest is measured, and why here.** `qcore-core` has no
/// dependencies at all — that is what makes the manifest rederivable in the
/// kernel — so it implements no SHA-256. The digest of each supplied header text
/// is measured here, where the bytes are, and the *rule* (it must equal the
/// digest the manifest declares) is the kernel's. That is the same split card
/// `NATIVE-LIBRARY-CONTRACT-SLICE` used for `CustodyShape`, and it is recorded
/// as `HEADER-DIGEST-DRIVER-MEASURED`.
///
/// Unlike the artifact identity — a **pin**, never opened, never hashed,
/// because whether the linker used it is the builder's act — the header is
/// **read**, and its text decides admission. Verifying its digest is therefore
/// available and honest, and it is what makes the decision rederivable: a
/// recheck whose `header` reference resolved to different bytes would otherwise
/// silently decide differently.
fn check_native_library_headers(
    libraries: &qcore_core::native_library::NativeLibraries,
    options: &CompileOptions,
    span: Span,
    ranges: &[FileRange],
) -> Result<(), Vec<FileDiagnostic>> {
    use qcore_core::native_library_header::{HeaderFailure, HeaderSource, check_header_bindings};

    let empty = BTreeMap::new();
    let supplied = options.native_library_headers.as_ref().unwrap_or(&empty);
    let digests = supplied
        .iter()
        .map(|(reference, text)| {
            (
                reference.clone(),
                format!(
                    "sha256:{}",
                    qcore_schemas::digest::sha256_hex(text.as_bytes())
                ),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let sources = supplied
        .iter()
        .map(|(reference, text)| {
            (
                reference.clone(),
                HeaderSource {
                    reference,
                    text,
                    measured_digest: &digests[reference],
                },
            )
        })
        .collect::<BTreeMap<_, _>>();
    check_header_bindings(libraries, &sources).map_err(|failure| {
        // Two codes, two conditions. `Q063` is "this level could not be
        // applied"; `Q064` is "it was applied and the manifest lost". A
        // disagreement **refuses** — the weaker admissible reading below a
        // withdrawn header binding is the manifest *without* a header, which is
        // accepted, so withdrawing to it would reward a contradiction with a
        // premise that says less about the build. That is `Q041`'s disposition
        // and deliberately the opposite of `Q048`'s, from the one rule both
        // rest on.
        let (code, message) = match &failure {
            HeaderFailure::Unreadable(_) => (
                "Q063",
                format!("the selected manifest header binding does not load: {failure}"),
            ),
            HeaderFailure::Disagreement { .. } => ("Q064", format!("{failure}")),
        };
        vec![map_raw_diagnostic(
            code,
            SemanticPass::NativeLibrary,
            span,
            message,
            ranges,
        )]
    })
}

/// What [`admit_extern_callees`] decided: the admitted callees, and the
/// `extern library` bindings the module's source files resolved (card
/// `NATIVE-LIBRARY-SOURCE-BINDING`).
///
/// The two travel together because they are one decision: a callee's manifest
/// residence was resolved *through* its own file's binding, and the binding list
/// is what the certificate publishes as the statement that resolution rested on.
struct AdmittedExternSurface {
    callees: BTreeMap<String, extern_call::ExternCallee>,
    libraries: Vec<qcore_core::native_library::SourceLibraryBinding>,
}

/// Resolve each input file's `extern library NAME;` declarations against the
/// manifest selection (card `NATIVE-LIBRARY-SOURCE-BINDING`).
///
/// **Per file, by construction.** The driver checks and compiles a *combined*
/// source, so file identity survives only as `ranges`; a binding's own span
/// decides which file made it, and a declaration's span decides which file's
/// view resolves it. That is constraint 13's discipline — importing another
/// compilation input grants this file nothing — applied to a binding: a bound
/// file resolves against what it bound, and a sibling that bound nothing keeps
/// the selection-wide resolution it had before this card, byte for byte.
fn bind_source_libraries<'a>(
    parsed: &SourceProgram,
    libraries: &'a qcore_core::native_library::NativeLibraries,
    ranges: &[FileRange],
) -> Result<Vec<qcore_core::native_library::BoundSelection<'a>>, Vec<FileDiagnostic>> {
    let mut named: Vec<Vec<String>> = vec![Vec::new(); ranges.len().max(1)];
    let mut spans: Vec<Span> = vec![Span { start: 0, end: 0 }; ranges.len().max(1)];
    for binding in parsed.library_bindings() {
        let index = file_index(binding.span, ranges);
        named[index].push(binding.value.clone());
        if spans[index] == (Span { start: 0, end: 0 }) {
            spans[index] = binding.span;
        }
    }
    named
        .iter()
        .zip(&spans)
        .map(|(names, span)| {
            libraries.bind_source(names).map_err(|refusal| {
                vec![map_raw_diagnostic(
                    "Q065",
                    SemanticPass::NativeLibrary,
                    *span,
                    format!("this file's library binding does not resolve: {refusal}"),
                    ranges,
                )]
            })
        })
        .collect()
}

/// Which input file a combined-source span came from.
///
/// The last range whose start is at or before the span's start: `combine_files`
/// lays the inputs out end to end, so that is the file containing it. A span
/// before every range (the zero span of a module that declares nothing) belongs
/// to the first file, which is where a driver-level diagnostic is attributed
/// already.
fn file_index(span: Span, ranges: &[FileRange]) -> usize {
    ranges
        .iter()
        .rposition(|range| range.start <= span.start)
        .unwrap_or(0)
}

/// Admit every declared extern callee this module calls, or refuse by name.
///
/// The whole contract-selection, manifest, signature-agreement and bridging
/// gate of `source.extern-call.v1`, lifted out of [`compile_extern_source`]
/// unchanged so the composed route (card `STD-IO-COMPOSE`) admits callees by
/// exactly the same decision. There is no second admission table and no second
/// `Q023` default: composing a module with a printing model cannot buy it a
/// callee the extern route alone would refuse.
fn admit_extern_callees(
    parsed: &SourceProgram,
    options: &CompileOptions,
    ranges: &[FileRange],
) -> Result<AdmittedExternSurface, Vec<FileDiagnostic>> {
    use qcore_frontend::source_extern;

    let called = source_extern::called_extern_names(parsed);
    let first_span = called
        .values()
        .copied()
        .next()
        .unwrap_or(Span { start: 0, end: 0 });
    let declarations = source_extern::source_extern_declarations(parsed)
        .into_iter()
        .map(|declaration| (declaration.name.clone(), declaration))
        .collect::<BTreeMap<_, _>>();
    // Card `UNSAFE-LINKAGE`. A contract environment is required for exactly the
    // callees that need one. A module every one of whose callees is declared
    // `unsafe extern fn` selects no environment and is not refused for having
    // none; a module with one contracted callee is refused exactly as before.
    let needs_contracts = called.keys().any(|name| {
        declarations
            .get(name)
            .is_none_or(|declaration| !declaration.unchecked)
    });
    let (contracts, signatures) = if needs_contracts {
        let Some(sources) = options.extern_contracts.as_ref() else {
            return Err(vec![map_raw_diagnostic(
                "Q023",
                SemanticPass::ExternCall,
                first_span,
                "calling a declared extern function requires an explicit extern contract selection; there is no default environment".into(),
                ranges,
            )]);
        };
        let contracts = qcore_contract::contract_loader::load_resource_contract_sources(sources)
            .map_err(|error| {
                vec![map_raw_diagnostic(
                    "Q023",
                    SemanticPass::ExternCall,
                    first_span,
                    format!("the selected extern contract environment does not load: {error}"),
                    ranges,
                )]
            })?;
        // The environment is reloaded from its own retained text before use, so
        // a selection that no longer reproduces itself is refused rather than
        // trusted.
        qcore_contract::contract_loader::check_source_assumptions(&contracts).map_err(|error| {
            vec![map_raw_diagnostic(
                "Q027",
                SemanticPass::ExternCall,
                first_span,
                format!(
                    "the selected extern contracts differ from their declared sources: {error}"
                ),
                ranges,
            )]
        })?;
        let signatures =
            qcore_contract::extern_bridge::declared_signatures(&contracts).map_err(|error| {
                vec![map_raw_diagnostic(
                    "Q023",
                    SemanticPass::ExternCall,
                    first_span,
                    error,
                    ranges,
                )]
            })?;
        (Some(contracts), signatures)
    } else {
        // An all-unchecked module needs no environment, but if one is selected
        // it is still consulted: §5's identity gate refuses a waiver for a name
        // the environment declares. A selection that does not load is refused
        // rather than ignored — the gate would otherwise be silently skipped,
        // and a broken contract module would be a way to hide that a callee has
        // a contract.
        let contracts = match options.extern_contracts.as_ref() {
            None => None,
            Some(sources) => Some(
                qcore_contract::contract_loader::load_resource_contract_sources(sources).map_err(
                    |error| {
                        vec![map_raw_diagnostic(
                            "Q023",
                            SemanticPass::ExternCall,
                            first_span,
                            format!(
                                "the selected extern contract environment does not load: {error}"
                            ),
                            ranges,
                        )]
                    },
                )?,
            ),
        };
        (contracts, BTreeMap::new())
    };
    let libraries = load_native_libraries(options, first_span, ranges)?;
    // Card `NATIVE-LIBRARY-HEADER`. A manifest that names its exported header is
    // checked against that header's own text **before** any callee is admitted:
    // both sides of the comparison are load inputs, so there is no per-unit
    // reading to withdraw and nothing that could hold for one unit and not
    // another. A manifest that names no header is untouched, which is every
    // manifest written before this card.
    check_native_library_headers(&libraries, options, first_span, ranges)?;
    // Card `NATIVE-LIBRARY-CONTRACT-SLICE`. A selection that binds contracts is
    // resolved against the selected environment **before** any callee is
    // admitted, in both directions of design §4.1, and a selection that does
    // not resolve refuses the module rather than resolving partially. The
    // signatures are the same ones the contracted route already reads; where
    // the branch above did not need them (an all-waived module that still
    // selects a contract-binding manifest) they are derived here, so the
    // `Q023` default above is reached by exactly the callees that reached it
    // before.
    let signatures = if libraries.binds_contracts() && !needs_contracts {
        match contracts.as_ref() {
            None => signatures,
            Some(contracts) => qcore_contract::extern_bridge::declared_signatures(contracts)
                .map_err(|error| {
                    vec![map_raw_diagnostic(
                        "Q047",
                        SemanticPass::NativeLibrary,
                        first_span,
                        format!(
                            "a selected manifest binds declared contracts, but the selected environment does not present its signatures: {error}"
                        ),
                        ranges,
                    )]
                })?,
        }
    } else {
        signatures
    };
    let declared_modules = signatures
        .iter()
        .map(|(name, signature)| (name.clone(), signature.module.clone()))
        .collect::<BTreeMap<_, _>>();
    qcore_core::native_library::check_slice_totality(
        &libraries,
        contracts.as_ref().map(|_| &declared_modules),
    )
    .map_err(|refusal| {
        vec![map_raw_diagnostic(
            "Q047",
            SemanticPass::NativeLibrary,
            first_span,
            format!("the selected manifest contract slice does not bind: {refusal}"),
            ranges,
        )]
    })?;
    // Design §4.4's closed-allocator-custody check, which becomes real the
    // moment a manifest entry can acquire. The projection is read here — where
    // the contract machinery lives — and the decision is made in `qcore-core`,
    // where it is a function of its inputs.
    if let Some(contracts) = contracts.as_ref() {
        let shapes = libraries
            .contracted_entries()
            .into_iter()
            .filter_map(|(_, entry)| {
                let contract = contracts.contract(&entry.symbol)?;
                Some((entry.symbol.clone(), custody_shape(contract)))
            })
            .collect::<BTreeMap<_, _>>();
        qcore_core::native_library::check_closed_allocator_custody(&libraries, &shapes).map_err(
            |refusal| {
                vec![map_raw_diagnostic(
                    "Q047",
                    SemanticPass::NativeLibrary,
                    first_span,
                    format!("the selected manifest contract slice does not bind: {refusal}"),
                    ranges,
                )]
            },
        )?;
    }
    // Card `NATIVE-LIBRARY-SOURCE-BINDING`. Every input file's `extern library`
    // declarations are resolved against the selection **before** any callee is
    // admitted, and a file whose binding does not resolve refuses the module
    // rather than resolving partially — the same total-or-reject discipline the
    // manifest load, the header binding and the contract slice already follow.
    let bound = bind_source_libraries(parsed, &libraries, ranges)?;
    let mut diagnostics = Vec::new();
    let mut callees = BTreeMap::new();
    for (name, span) in &called {
        let Some(declaration) = declarations.get(name) else {
            continue;
        };
        // The declaration's own file decides which manifests resolve for it.
        let file_libraries = &bound[file_index(declaration.span, ranges)];
        if declaration.unchecked {
            if let Some(callee) = admit_unchecked_callee(
                declaration,
                contracts.as_ref(),
                file_libraries,
                ranges,
                &mut diagnostics,
            ) {
                callees.insert(name.clone(), callee);
            }
            continue;
        }
        // The contracted mirror of the foreign-symbol refusal in
        // `admit_unchecked_callee`, on exactly the same ground and placed before
        // the `Q042` gate it guards: a contracted declaration whose symbol a
        // manifest this file did not bind declares must not resolve *past* that
        // manifest into an unresident contract.
        if let Some(refusal) = file_libraries.foreign_symbol(name) {
            diagnostics.push(map_raw_diagnostic(
                "Q066",
                SemanticPass::NativeLibrary,
                declaration.span,
                refusal.to_string(),
                ranges,
            ));
            continue;
        }
        // Card `UNSAFE-LINKAGE-ARTIFACT`, the mirror of `Q034`. `Q034` refuses a
        // waiver for a name the contract environment declares — you may not
        // waive a contract you have. This refuses a *contract* for a name the
        // selected manifest declares contract-less: the named build says this
        // symbol has no contract, and admitting one anyway would assert modes
        // and effects about an artifact whose own manifest states none. Where
        // two declarations are visible they are compared, and the refusing
        // direction wins.
        if let Some(binding) = file_libraries.unchecked_binding(name) {
            diagnostics.push(map_raw_diagnostic(
                "Q042",
                SemanticPass::NativeLibrary,
                declaration.span,
                format!(
                    "`{name}` is declared `extern fn` here, but `{}` declares it a contract-less symbol of `{}`; a contract may not be asserted about a build whose manifest states none",
                    binding.module, binding.identity.library
                ),
                ranges,
            ));
            continue;
        }
        let contracts = contracts
            .as_ref()
            .expect("a contracted callee forced the environment to load above");
        if declaration.has_inline_contract {
            diagnostics.push(map_raw_diagnostic(
                "Q027",
                SemanticPass::ExternCall,
                declaration.span,
                format!(
                    "`{name}` carries an inline `contract {{ … }}` block, which this profile does not enforce; the contract must come from the explicit selection"
                ),
                ranges,
            ));
            continue;
        }
        let (Some(contract), Some(signature)) = (contracts.contract(name), signatures.get(name))
        else {
            // Card W45-CONTRACT-DEFERRED-REVIEW: a declaration awaiting human
            // review is withheld by the loader, so the callee is exactly as
            // uncontracted as an undeclared one; the refusal names the review.
            if let Some(message) =
                qcore_contract::extern_bridge::pending_review_refusal(name, contracts)
            {
                diagnostics.push(map_raw_diagnostic(
                    "Q023",
                    SemanticPass::ExternCall,
                    *span,
                    message,
                    ranges,
                ));
                continue;
            }
            diagnostics.push(map_raw_diagnostic(
                "Q023",
                SemanticPass::ExternCall,
                *span,
                format!(
                    "`{name}` has no admitted declared contract in the selected extern contract environment"
                ),
                ranges,
            ));
            continue;
        };
        if declaration.parameter_types != signature.parameter_types
            || declaration.result_type != signature.result_type
            || declaration.parameter_names != signature.parameter_names
        {
            diagnostics.push(map_raw_diagnostic(
                "Q027",
                SemanticPass::ExternCall,
                declaration.span,
                format!(
                    "`{name}` is declared `({}) -> {}` here but `({}) -> {}` in {}",
                    declaration.parameter_types.join(", "),
                    declaration.result_type,
                    signature.parameter_types.join(", "),
                    signature.result_type,
                    signature.module
                ),
                ranges,
            ));
            continue;
        }
        let declared =
            match qcore_contract::extern_bridge::declared_contract(name, signature, contract) {
                Ok(declared) => declared,
                Err(error) => {
                    diagnostics.push(map_raw_diagnostic(
                        "Q024",
                        SemanticPass::ExternCall,
                        declaration.span,
                        error,
                        ranges,
                    ));
                    continue;
                }
            };
        // Card `NATIVE-LIBRARY-CONTRACT-SLICE`: design §4.2's agreement gate at
        // the contracted setting, and **withdrawal** on disagreement.
        //
        // Both sides are read from what a certificate already holds — the
        // manifest text and the environment's retained contract text — never
        // from a typed Clang AST, so a rederivation reproduces the decision
        // (constraint 8's reader discipline).
        //
        // The direction is the argument. `LOADER-ABI` withdraws, card
        // `UNSAFE-LINKAGE-ARTIFACT` refuses, and both follow one rule: withdraw
        // only where the weaker admissible reading *refuses*. There the weaker
        // reading was the bare waiver, which is accepted; here it is the
        // uncontracted callee, which refuses at `Q023`. So the contradicted
        // binding is withdrawn for this unit — the name is not seeded into the
        // admitted environment and every call site becomes an uncontracted one
        // — and the withdrawal itself is reported rather than performed
        // silently. Monotone in the refusing direction: this can only turn an
        // admission into a refusal.
        let residence = match file_libraries.contracted_binding(name) {
            None => None,
            Some(binding) => {
                let manifest_view = binding.entry;
                let projected = declared
                    .parameters
                    .iter()
                    .map(|(abi, _)| *abi)
                    .collect::<Vec<_>>();
                if let Err(disagreement) = qcore_core::native_library::check_contracted_agreement(
                    manifest_view,
                    &projected,
                    declared.result,
                ) {
                    diagnostics.push(map_raw_diagnostic(
                        "Q048",
                        SemanticPass::NativeLibrary,
                        declaration.span,
                        format!(
                            "the contract `{name}` selected from `{}` is withdrawn for this unit: `{}` declares the exported symbol otherwise ({disagreement}); every call site becomes an uncontracted callee",
                            signature.module, binding.module
                        ),
                        ranges,
                    ));
                    diagnostics.push(map_raw_diagnostic(
                        "Q023",
                        SemanticPass::ExternCall,
                        *span,
                        format!(
                            "`{name}` has no admitted declared contract in the selected extern contract environment"
                        ),
                        ranges,
                    ));
                    continue;
                }
                Some(binding.residence())
            }
        };
        match extern_call::bridge_contract(name, &declared) {
            Ok(callee) => {
                let callee = match residence {
                    None => callee,
                    Some(residence) => extern_call::manifest_contracted_callee(callee, residence),
                };
                callees.insert(name.clone(), callee);
            }
            Err(refusal) => diagnostics.push(map_raw_diagnostic(
                "Q024",
                SemanticPass::ExternCall,
                declaration.span,
                format!("`{name}` is not callable in `source.extern-call.v1`: {refusal}"),
                ranges,
            )),
        }
    }
    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }
    // The published binding list is the union over the input files, in file
    // order then declaration order. Two files binding the same library
    // contribute two entries that compare equal and collapse in the sorted
    // premise inventory — one statement, told once.
    let mut resolved = Vec::new();
    for selection in &bound {
        for binding in selection.bindings() {
            if !resolved.contains(binding) {
                resolved.push(binding.clone());
            }
        }
    }
    Ok(AdmittedExternSurface {
        callees,
        libraries: resolved,
    })
}
/// Card `source.extern-call.v1`. Every step here fails closed: no contract
/// selection, an absent name, an unadmitted mode, an unbridgeable type, a
/// missing null adapter, a refused context or a declaration that disagrees
/// with its selected contract each produce a diagnostic and no module.
///
/// A module accepted here is checked twice over the same lowering: the core
/// extern checker validates the call sites and the adapter, and every
/// established sequential pass validates the surrounding custody, resource
/// flow and effect obligations with the declared extern summaries supplied as
/// call-site assumptions. Neither check is derived from the other.
fn compile_extern_source(
    parsed: &SourceProgram,
    typed: &TypedSourceProgram,
    mode: SourceModuleMode,
    options: &CompileOptions,
    ranges: &[FileRange],
    target: Option<&RecordedTarget>,
) -> Result<CompiledModule, Vec<FileDiagnostic>> {
    use qcore_frontend::source_extern;

    if mode != SourceModuleMode::Executable {
        return Err(vec![plain_diagnostic(
            "Q500",
            "entry-free extern-calling modules require additional source lowering",
        )]);
    }
    let entry = select_entry(typed, options).map_err(|error| vec![error])?;
    let admitted = admit_extern_callees(parsed, options, ranges)?;
    let callees = admitted.callees;

    // Card `W48-QR-SCALAR-TYPES-R3`: the recorded target gives `isize`/`usize`
    // values their width in the value statements and declared outcomes.
    let lowered = source_extern::lower_source_extern_for_target(
        parsed,
        typed,
        &entry,
        &callees,
        &admitted.libraries,
        target,
    )
    .map_err(|errors| {
        errors
            .into_iter()
            .map(|error| map_diagnostic(error, ranges))
            .collect::<Vec<_>>()
    })?;
    let certificate = extern_call::check_extern_program(&lowered.program).map_err(|errors| {
        errors
            .into_iter()
            .map(|error| {
                let span = lowered
                    .event_spans
                    .get(error.event())
                    .copied()
                    .unwrap_or(lowered.scope_span);
                map_raw_diagnostic(
                    error.code(),
                    SemanticPass::ExternCall,
                    span,
                    error.to_string(),
                    ranges,
                )
            })
            .collect::<Vec<_>>()
    })?;
    // The driver never publishes a certificate it did not independently
    // re-derive from the same lowered program.
    extern_call::recheck_extern_certificate(&lowered.program, &certificate).map_err(|failure| {
        vec![map_raw_diagnostic(
            "Q027",
            SemanticPass::ExternCall,
            lowered.scope_span,
            format!("the extern-call certificate failed its independent recheck: {failure}"),
            ranges,
        )]
    })?;
    // Card `UNSAFE-LINKAGE`: the publication tripwire. A certificate that lost
    // an unchecked-linkage premise between derivation and publication is
    // refused rather than published. It cannot fail by construction — the
    // premise is derived from the same callee inventory this compares against —
    // and asserting it anyway is the point: the day it can fail, it is caught
    // here rather than inside a `Safe` that stopped saying what it rests on.
    if !extern_call::certificate_carries_unchecked_linkage(&lowered.program, &certificate) {
        return Err(vec![map_raw_diagnostic(
            "Q035",
            SemanticPass::UncheckedLinkage,
            lowered.scope_span,
            "the published extern-call certificate does not carry an unchecked-linkage premise for every uncontracted callee this module declares".into(),
            ranges,
        )]);
    }
    // Card `NATIVE-LIBRARY-CONTRACT-SLICE`: the same tripwire for the stacked
    // premises of a manifest-bound contracted callee. It refuses a certificate
    // that lost either half — the artifact premise, or the declared-contract
    // premise the artifact premise stacks beside rather than replaces.
    if !extern_call::certificate_carries_residence_premises(&lowered.program, &certificate) {
        return Err(vec![map_raw_diagnostic(
            "Q049",
            SemanticPass::NativeLibrary,
            lowered.scope_span,
            "the published extern-call certificate does not carry both the declared-contract and the artifact premise of every manifest-bound contracted callee this module declares".into(),
            ranges,
        )]);
    }
    // Card `NATIVE-LIBRARY-SOURCE-BINDING`: the same tripwire for the file's own
    // binding act. A certificate that lost it would be silent about which build
    // the module declared its extern surface to be resolved against — a weaker
    // statement published by accident.
    if !extern_call::certificate_carries_source_bindings(&lowered.program, &certificate) {
        return Err(vec![map_raw_diagnostic(
            "Q067",
            SemanticPass::NativeLibrary,
            lowered.scope_span,
            "the published extern-call certificate does not carry a source-binding premise for every `extern library` this module declares".into(),
            ranges,
        )]);
    }

    let error_names = lowered.program.errors.clone();
    let sequential = qcore_frontend::whole_program::check_whole_program_with_externs(
        typed,
        &ErrorDeclarations::new(error_names.iter().cloned()),
        vec![lowered.semantics.clone()],
        &lowered.extern_summaries,
    )
    .map_err(|errors| {
        errors
            .into_iter()
            .map(|error| map_diagnostic(error, ranges))
            .collect::<Vec<_>>()
    })?;

    let result_profile = SourceResultProfile {
        scalar_results: BTreeMap::from([(entry.clone(), ExecutableResultType::Unit)]),
        owned_buffer_results: std::collections::BTreeSet::new(),
        custody_payloads: CustodyPayloadProfile::SeparateIdentityLedger,
    };
    let reference_observation =
        extern_c_emit::interpret_extern_program(&lowered.program, &certificate).map_err(
            |refusal| {
                vec![plain_diagnostic(
                    "Q500",
                    format!("the extern-call reference execution refused this trace: {refusal:?}"),
                )]
            },
        )?;
    let emission =
        extern_c_emit::emit_extern_c11(&lowered.program, &certificate).map_err(|refusal| {
            vec![plain_diagnostic(
                "Q500",
                format!("the extern-call backend refused this trace: {refusal:?}"),
            )]
        })?;
    Ok(CompiledModule {
        source_bundle: None,
        extension_warnings: Vec::new(),
        checked: CheckedSourceModule::ExternCalls(Box::new(CheckedExternModule {
            lowered,
            certificate,
            sequential,
        })),
        execution_profile: SourceExecutionProfile::ExternCallsV1,
        declaration_count: typed.declarations.len(),
        mode,
        result_profile,
        entry: Some(entry),
        c_source: Some(emission.source),
        source_map: Vec::new(),
        // The reference observation of this route is the extern call trace, not
        // the sequential cleanup observation this field carries.
        reference_observation: None,
        extern_observation: Some(reference_observation),
    })
}

/// Compile one `source.std-io-extern.v1` module: the checked composition of
/// `source.extern-call.v1` and `source.std-io.v1` (card `STD-IO-COMPOSE`).
///
/// Every gate either composed route runs is run here, by that route's own code:
/// [`admit_extern_callees`] admits the callees, the composed lowering delegates
/// each statement to that model's own unedited lowering,
/// [`composed_std_io::check_composed_program`] runs both checkers before its
/// own join, and the established sequential passes re-check the extern half
/// exactly as `compile_extern_source` does. What composing adds is the join and
/// nothing else.
fn compile_composed_source(
    parsed: &SourceProgram,
    typed: &TypedSourceProgram,
    mode: SourceModuleMode,
    options: &CompileOptions,
    ranges: &[FileRange],
    target: Option<&RecordedTarget>,
) -> Result<CompiledModule, Vec<FileDiagnostic>> {
    use qcore_frontend::source_composed;

    if mode != SourceModuleMode::Executable {
        return Err(vec![plain_diagnostic(
            "Q043",
            "entry-free composed modules require additional source lowering",
        )]);
    }
    let entry = select_entry(typed, options).map_err(|error| vec![error])?;
    let admitted = admit_extern_callees(parsed, options, ranges)?;
    let callees = admitted.callees;
    let lowered = source_composed::lower_source_composed_for_target(
        parsed,
        typed,
        &entry,
        &callees,
        &admitted.libraries,
        target,
    )
    .map_err(|errors| {
        errors
            .into_iter()
            .map(|error| map_diagnostic(error, ranges))
            .collect::<Vec<_>>()
    })?;
    // Both projections are checked by their own checkers first, so a composed
    // module receives each route's own diagnostics under each route's own code.
    // The composed checker runs them again inside its own derivation; running
    // them here as well is what makes the *reported* refusal the specific one.
    let extern_certificate =
        extern_call::check_extern_program(&lowered.program.externs).map_err(|errors| {
            errors
                .into_iter()
                .map(|error| {
                    let span = lowered
                        .externs
                        .event_spans
                        .get(error.event())
                        .copied()
                        .unwrap_or(lowered.scope_span);
                    map_raw_diagnostic(
                        error.code(),
                        SemanticPass::ExternCall,
                        span,
                        error.to_string(),
                        ranges,
                    )
                })
                .collect::<Vec<_>>()
        })?;
    std_io::check_std_io_program(&lowered.program.io).map_err(|errors| {
        errors
            .into_iter()
            .map(|error| {
                let span = lowered
                    .io
                    .event_spans
                    .get(error.event())
                    .copied()
                    .unwrap_or(lowered.scope_span);
                map_raw_diagnostic(
                    error.code(),
                    SemanticPass::StandardIo,
                    span,
                    error.to_string(),
                    ranges,
                )
            })
            .collect::<Vec<_>>()
    })?;
    let certificate =
        composed_std_io::check_composed_program(&lowered.program).map_err(|errors| {
            errors
                .into_iter()
                .map(|error| {
                    map_raw_diagnostic(
                        error.code(),
                        SemanticPass::ComposedStdIo,
                        lowered.scope_span,
                        error.to_string(),
                        ranges,
                    )
                })
                .collect::<Vec<_>>()
        })?;
    // The driver never publishes a certificate it did not independently
    // re-derive — here, all three of them.
    composed_std_io::recheck_composed_certificate(&lowered.program, &certificate).map_err(
        |failure| {
            vec![map_raw_diagnostic(
                "Q043",
                SemanticPass::ComposedStdIo,
                lowered.scope_span,
                format!("the composed certificate failed its independent recheck: {failure}"),
                ranges,
            )]
        },
    )?;
    // Card `UNSAFE-LINKAGE`'s publication tripwire, unchanged and unweakened by
    // composing: a composed certificate that lost an unchecked-linkage premise
    // between derivation and publication is refused rather than published.
    if !extern_call::certificate_carries_unchecked_linkage(
        &lowered.program.externs,
        &extern_certificate,
    ) || !extern_call::certificate_carries_unchecked_linkage(
        &lowered.program.externs,
        &certificate.externs,
    ) {
        return Err(vec![map_raw_diagnostic(
            "Q035",
            SemanticPass::UncheckedLinkage,
            lowered.scope_span,
            "the published composed certificate does not carry an unchecked-linkage premise for every uncontracted callee this module declares".into(),
            ranges,
        )]);
    }
    // Card `NATIVE-LIBRARY-CONTRACT-SLICE`'s tripwire, likewise unweakened by
    // composing.
    if !extern_call::certificate_carries_residence_premises(
        &lowered.program.externs,
        &extern_certificate,
    ) || !extern_call::certificate_carries_residence_premises(
        &lowered.program.externs,
        &certificate.externs,
    ) {
        return Err(vec![map_raw_diagnostic(
            "Q049",
            SemanticPass::NativeLibrary,
            lowered.scope_span,
            "the published composed certificate does not carry both the declared-contract and the artifact premise of every manifest-bound contracted callee this module declares".into(),
            ranges,
        )]);
    }
    // Card `NATIVE-LIBRARY-SOURCE-BINDING`'s tripwire, likewise unweakened by
    // composing.
    if !extern_call::certificate_carries_source_bindings(
        &lowered.program.externs,
        &extern_certificate,
    ) || !extern_call::certificate_carries_source_bindings(
        &lowered.program.externs,
        &certificate.externs,
    ) {
        return Err(vec![map_raw_diagnostic(
            "Q067",
            SemanticPass::NativeLibrary,
            lowered.scope_span,
            "the published composed certificate does not carry a source-binding premise for every `extern library` this module declares".into(),
            ranges,
        )]);
    }

    let error_names = lowered.program.externs.errors.clone();
    let sequential = qcore_frontend::whole_program::check_whole_program_with_externs(
        typed,
        &ErrorDeclarations::new(error_names.iter().cloned()),
        vec![lowered.externs.semantics.clone()],
        &lowered.externs.extern_summaries,
    )
    .map_err(|errors| {
        errors
            .into_iter()
            .map(|error| map_diagnostic(error, ranges))
            .collect::<Vec<_>>()
    })?;

    let reference_observation =
        composed_std_io_c_emit::interpret_composed_program(&lowered.program, &certificate)
            .map_err(|refusal| {
                vec![plain_diagnostic(
                    "Q043",
                    format!("the composed reference execution refused this trace: {refusal:?}"),
                )]
            })?;
    let emission = composed_std_io_c_emit::emit_composed_c11(&lowered.program, &certificate)
        .map_err(|refusal| {
            vec![plain_diagnostic(
                "Q043",
                format!("the composed backend refused this trace: {refusal:?}"),
            )]
        })?;
    let result_profile = SourceResultProfile {
        scalar_results: BTreeMap::from([(entry.clone(), ExecutableResultType::Unit)]),
        owned_buffer_results: std::collections::BTreeSet::new(),
        custody_payloads: CustodyPayloadProfile::SeparateIdentityLedger,
    };
    Ok(CompiledModule {
        source_bundle: None,
        extension_warnings: Vec::new(),
        checked: CheckedSourceModule::Composed(Box::new(CheckedComposedModule {
            lowered,
            certificate,
            sequential,
        })),
        execution_profile: SourceExecutionProfile::ComposedStdIoExternV1,
        declaration_count: typed.declarations.len(),
        mode,
        result_profile,
        entry: Some(entry),
        c_source: Some(emission.source),
        source_map: Vec::new(),
        // The reference observation of this route is the per-stream record of
        // both models' lines, carried beside the extern route's own trace
        // rather than on the sequential cleanup field, which models something
        // else entirely.
        reference_observation: None,
        extern_observation: Some(reference_observation),
    })
}

/// Whether `compile_std_io_source` would refuse this module before its checker
/// runs: an entry-free module, an entry it cannot select, or an entry its
/// lowering refuses. Exactly the conditions under which that route returns a
/// lowering-stage refusal, evaluated with the same functions it calls.
fn std_io_lowering_refuses(
    parsed: &SourceProgram,
    typed: &TypedSourceProgram,
    mode: SourceModuleMode,
    options: &CompileOptions,
) -> bool {
    if mode != SourceModuleMode::Executable {
        return true;
    }
    let Ok(entry) = select_entry(typed, options) else {
        return true;
    };
    qcore_frontend::source_std_io::lower_source_std_io(parsed, typed, &entry).is_err()
}

/// Compile one `source.std-io.v1` module (card `STD-MODULES`).
///
/// The route emits real C and has a real reference execution, and the two are
/// compared per stream. It publishes no sequential evidence, because its
/// operand grammar admits no resource — see [`CheckedStdIoModule`].
fn compile_std_io_source(
    parsed: &SourceProgram,
    typed: &TypedSourceProgram,
    mode: SourceModuleMode,
    options: &CompileOptions,
    ranges: &[FileRange],
) -> Result<CompiledModule, Vec<FileDiagnostic>> {
    if mode != SourceModuleMode::Executable {
        return Err(vec![plain_diagnostic(
            "Q030",
            "entry-free `source.std-io.v1` modules require additional source lowering",
        )]);
    }
    let entry = select_entry(typed, options).map_err(|error| vec![error])?;
    let lowered = qcore_frontend::source_std_io::lower_source_std_io(parsed, typed, &entry)
        .map_err(|errors| {
            errors
                .into_iter()
                .map(|error| map_diagnostic(error, ranges))
                .collect::<Vec<_>>()
        })?;
    let certificate = std_io::check_std_io_program(&lowered.program).map_err(|errors| {
        errors
            .into_iter()
            .map(|error| {
                let span = lowered
                    .event_spans
                    .get(error.event())
                    .copied()
                    .unwrap_or(lowered.scope_span);
                map_raw_diagnostic(
                    error.code(),
                    SemanticPass::StandardIo,
                    span,
                    error.to_string(),
                    ranges,
                )
            })
            .collect::<Vec<_>>()
    })?;
    // The driver never publishes a certificate it did not independently
    // re-derive from the same lowered program.
    std_io::recheck_std_io_certificate(&lowered.program, &certificate).map_err(|failure| {
        vec![map_raw_diagnostic(
            "Q031",
            SemanticPass::StandardIo,
            lowered.scope_span,
            format!("the standard-IO certificate failed its independent recheck: {failure}"),
            ranges,
        )]
    })?;

    let reference_observation =
        std_io_c_emit::interpret_std_io_program(&lowered.program, &certificate).map_err(
            |refusal| {
                vec![plain_diagnostic(
                    "Q030",
                    format!("the standard-IO reference execution refused this trace: {refusal:?}"),
                )]
            },
        )?;
    let emission =
        std_io_c_emit::emit_std_io_c11(&lowered.program, &certificate).map_err(|refusal| {
            vec![plain_diagnostic(
                "Q030",
                format!("the standard-IO backend refused this trace: {refusal:?}"),
            )]
        })?;

    let result_profile = SourceResultProfile {
        scalar_results: BTreeMap::from([(entry.clone(), ExecutableResultType::Unit)]),
        owned_buffer_results: std::collections::BTreeSet::new(),
        custody_payloads: CustodyPayloadProfile::NoResourcePayloads,
    };
    Ok(CompiledModule {
        source_bundle: None,
        extension_warnings: Vec::new(),
        checked: CheckedSourceModule::StandardIo(Box::new(CheckedStdIoModule {
            lowered,
            certificate,
        })),
        execution_profile: SourceExecutionProfile::StandardIoV1,
        declaration_count: typed.declarations.len(),
        mode,
        result_profile,
        entry: Some(entry),
        c_source: Some(emission.source),
        source_map: Vec::new(),
        // The reference observation of this route is the per-stream line trace,
        // carried on `extern_observation` beside the extern route's own trace
        // rather than on the sequential cleanup field, which models something
        // else entirely.
        reference_observation: None,
        extern_observation: Some(reference_observation),
    })
}

/// Attribute the helper graph's refusal to the existing pass that owns the
/// condition. The resource-call profile allocates no diagnostic namespace:
/// ordinary signature/graph conditions stay at Q020, extent arithmetic at
/// Q053, inherited custody at Q054, certificate construction at Q056, and
/// transitive effect rows at Q057.
fn resource_call_refusal_attribution(
    category: &resource_calls::RefusalCategory,
) -> (&'static str, SemanticPass) {
    match category {
        resource_calls::RefusalCategory::Ordinary => ("Q020", SemanticPass::OrdinaryTypes),
        resource_calls::RefusalCategory::Extent => ("Q053", SemanticPass::StandardBuffer),
        resource_calls::RefusalCategory::Ownership => ("Q054", SemanticPass::StandardBuffer),
        resource_calls::RefusalCategory::Certificate => ("Q056", SemanticPass::StandardBuffer),
        resource_calls::RefusalCategory::Effects => ("Q057", SemanticPass::StandardBuffer),
    }
}

/// Compile one closed `source.resource-calls.v1` module.
fn compile_resource_call_source(
    parsed: &SourceProgram,
    typed: &TypedSourceProgram,
    mode: SourceModuleMode,
    options: &CompileOptions,
    ranges: &[FileRange],
) -> Result<CompiledModule, Vec<FileDiagnostic>> {
    if mode != SourceModuleMode::Executable {
        return Err(vec![map_raw_diagnostic(
            "Q020",
            SemanticPass::OrdinaryTypes,
            Span { start: 0, end: 0 },
            "entry-free resource helper modules are outside resource-calls.v1".to_owned(),
            ranges,
        )]);
    }
    let entry = select_entry(typed, options).map_err(|error| vec![error])?;
    let lowered =
        qcore_frontend::source_resource_calls::lower(parsed, &entry).map_err(|errors| {
            errors
                .into_iter()
                .map(|error| map_diagnostic(error, ranges))
                .collect::<Vec<_>>()
        })?;
    let checked = resource_calls::check(lowered.program).map_err(|error| {
        let span = lowered
            .statement_spans
            .get(error.function)
            .and_then(|spans| spans.get(error.statement))
            .copied()
            .or_else(|| lowered.function_spans.get(error.function).copied())
            .unwrap_or(Span { start: 0, end: 0 });
        let (code, pass) = resource_call_refusal_attribution(&error.category);
        vec![map_raw_diagnostic(
            code,
            pass,
            span,
            format!("source.resource-calls.v1: {}", error.message),
            ranges,
        )]
    })?;
    let flattened = checked.flattened().clone();
    let certificate = checked.certificate().clone();
    let scope_span = lowered
        .function_spans
        .get(checked.program().entry)
        .copied()
        .unwrap_or(Span { start: 0, end: 0 });
    let event_spans = vec![scope_span; flattened.events.len()];
    let buffer_lowered = qcore_frontend::source_std_buffer::LoweredBufferModule {
        entry: entry.clone(),
        program: flattened,
        event_spans,
        scope_span,
    };
    let reference_observation =
        std_buffer_c_emit::interpret_buffer_program(&buffer_lowered.program, &certificate)
            .map_err(|refusal| {
                vec![map_raw_diagnostic(
                    "Q056",
                    SemanticPass::StandardBuffer,
                    scope_span,
                    format!("resource helper reference execution refused: {refusal:?}"),
                    ranges,
                )]
            })?;
    let c_source = resource_calls_c_emit::emit(&checked);
    let mut scalar_results = BTreeMap::new();
    for function in &checked.program().functions {
        match function.result {
            resource_calls::Type::Unit => {
                scalar_results.insert(function.name.clone(), ExecutableResultType::Unit);
            }
            resource_calls::Type::I32 => {
                scalar_results.insert(function.name.clone(), ExecutableResultType::I32);
            }
            resource_calls::Type::OwnedBytes
            | resource_calls::Type::ReadBytes
            | resource_calls::Type::WriteBytes => {}
        }
    }
    Ok(CompiledModule {
        source_bundle: None,
        extension_warnings: Vec::new(),
        checked: CheckedSourceModule::StandardBuffer(Box::new(CheckedBufferModule {
            lowered: buffer_lowered,
            certificate,
            resource_calls: Some(Box::new(checked)),
        })),
        execution_profile: SourceExecutionProfile::ResourceCallsV1,
        declaration_count: typed.declarations.len(),
        mode,
        result_profile: SourceResultProfile {
            scalar_results,
            owned_buffer_results: BTreeSet::new(),
            custody_payloads: CustodyPayloadProfile::TypedResourceBytes,
        },
        entry: Some(entry),
        c_source: Some(c_source),
        source_map: Vec::new(),
        reference_observation: None,
        extern_observation: Some(reference_observation),
    })
}

/// Compile one `source.buffer.v1` module (card `STD-BUFFER`).
///
/// The route emits real, self-contained C and has a real reference execution,
/// and the two are compared per stream. Its custody obligations are discharged
/// by the **unedited** `qcore_core::ownership` checker inside
/// `check_buffer_program`; see [`CheckedBufferModule`] for why that is a
/// discharge rather than an omission.
fn compile_buffer_source(
    parsed: &SourceProgram,
    typed: &TypedSourceProgram,
    mode: SourceModuleMode,
    options: &CompileOptions,
    ranges: &[FileRange],
    target: Option<&RecordedTarget>,
) -> Result<CompiledModule, Vec<FileDiagnostic>> {
    if mode != SourceModuleMode::Executable {
        return Err(vec![plain_diagnostic(
            "Q058",
            "entry-free `source.buffer.v1` modules require additional source lowering",
        )]);
    }
    let entry = select_entry(typed, options).map_err(|error| vec![error])?;
    let lowered = qcore_frontend::source_std_buffer::lower_source_buffer_for_target(
        parsed, typed, &entry, target,
    )
    .map_err(|errors| {
        errors
            .into_iter()
            .map(|error| map_diagnostic(error, ranges))
            .collect::<Vec<_>>()
    })?;
    // Card `W48-QR-SCALAR-TYPES-R3`: the builder's recorded target gives
    // `isize`/`usize` fields and conversions their width; a program naming
    // neither is checked exactly as before.
    let certificate = std_buffer::check_buffer_program_for_target(&lowered.program, target)
        .map_err(|errors| {
            errors
                .into_iter()
                .map(|error| {
                    let span = lowered
                        .event_spans
                        .get(error.event())
                        .copied()
                        .unwrap_or(lowered.scope_span);
                    map_raw_diagnostic(
                        error.code(),
                        SemanticPass::StandardBuffer,
                        span,
                        error.to_string(),
                        ranges,
                    )
                })
                .collect::<Vec<_>>()
        })?;
    // The driver never publishes a certificate it did not independently
    // re-derive from the same lowered program.
    std_buffer::recheck_buffer_certificate_for_target(&lowered.program, &certificate, target)
        .map_err(|failure| {
            vec![map_raw_diagnostic(
                "Q056",
                SemanticPass::StandardBuffer,
                lowered.scope_span,
                format!("the byte-buffer certificate failed its independent recheck: {failure}"),
                ranges,
            )]
        })?;

    let reference_observation =
        std_buffer_c_emit::interpret_buffer_program(&lowered.program, &certificate).map_err(
            |refusal| {
                vec![plain_diagnostic(
                    "Q058",
                    format!("the byte-buffer reference execution refused this trace: {refusal:?}"),
                )]
            },
        )?;
    let emission =
        std_buffer_c_emit::emit_buffer_c11(&lowered.program, &certificate).map_err(|refusal| {
            vec![plain_diagnostic(
                "Q063",
                format!("the byte-buffer backend refused this trace: {refusal:?}"),
            )]
        })?;

    let result_profile = SourceResultProfile {
        scalar_results: BTreeMap::from([(entry.clone(), ExecutableResultType::Unit)]),
        // The buffers this entry holds are released inside it — the inherited
        // ownership checker refuses a normal exit that leaves any live — so no
        // owned result crosses the entry boundary.
        owned_buffer_results: std::collections::BTreeSet::new(),
        custody_payloads: CustodyPayloadProfile::NoResourcePayloads,
    };
    Ok(CompiledModule {
        source_bundle: None,
        extension_warnings: Vec::new(),
        checked: CheckedSourceModule::StandardBuffer(Box::new(CheckedBufferModule {
            lowered,
            certificate,
            resource_calls: None,
        })),
        execution_profile: SourceExecutionProfile::StandardBufferV1,
        declaration_count: typed.declarations.len(),
        mode,
        result_profile,
        entry: Some(entry),
        c_source: Some(emission.source),
        source_map: Vec::new(),
        // The reference observation of this route is the per-stream line trace,
        // carried on `extern_observation` beside the extern route's own trace
        // rather than on the sequential cleanup field, exactly as
        // `source.std-io.v1` carries its own.
        reference_observation: None,
        extern_observation: Some(reference_observation),
    })
}

/// Compile one `source.socket.v1` module (card `QUIRIN-SOCKET-V1`).
///
/// The shape is deliberately [`compile_buffer_source`]'s — lower, check,
/// **recheck**, interpret, emit — so the two routes' guarantees read the same
/// way. What differs is what the interpretation step produces: a
/// [`std_socket_c_emit::SocketObservation`] whose declared
/// `qcore.obs-reordering.v1` relation is *strict by derivation* (this route has
/// one thread of control and creates none), rather than a bare line list.
fn compile_socket_source(
    parsed: &SourceProgram,
    typed: &TypedSourceProgram,
    mode: SourceModuleMode,
    options: &CompileOptions,
    ranges: &[FileRange],
) -> Result<CompiledModule, Vec<FileDiagnostic>> {
    if mode != SourceModuleMode::Executable {
        return Err(vec![plain_diagnostic(
            "Q080",
            "entry-free `source.socket.v1` modules require additional source lowering",
        )]);
    }
    let entry = select_entry(typed, options).map_err(|error| vec![error])?;
    let lowered = qcore_frontend::source_socket::lower_source_socket(parsed, typed, &entry)
        .map_err(|errors| {
            errors
                .into_iter()
                .map(|error| map_diagnostic(error, ranges))
                .collect::<Vec<_>>()
        })?;
    let certificate = std_socket::check_socket_program(&lowered.program).map_err(|errors| {
        errors
            .into_iter()
            .map(|error| {
                let span = lowered
                    .event_spans
                    .get(error.event())
                    .copied()
                    .unwrap_or(lowered.scope_span);
                map_raw_diagnostic(
                    error.code(),
                    SemanticPass::StandardSocket,
                    span,
                    error.to_string(),
                    ranges,
                )
            })
            .collect::<Vec<_>>()
    })?;
    // The driver never publishes a certificate it did not independently
    // re-derive from the same lowered program.
    std_socket::recheck_socket_certificate(&lowered.program, &certificate).map_err(|failure| {
        vec![map_raw_diagnostic(
            "Q088",
            SemanticPass::StandardSocket,
            lowered.scope_span,
            format!("the socket certificate failed its independent recheck: {failure}"),
            ranges,
        )]
    })?;

    // The protocol cells (card `CT42-CELL-ROUTE`). Derived for every admitted
    // program from its own decided connection structure, and every
    // `disjoint NAME NAME;` question answered by the **unedited**
    // `DisjointFootprintsV1` checker: the two ends of one accepted connection
    // are refused there as `SharedProtocolCell`, reported under that
    // checker's own `Q664` — the `Concurrency` pass's band, reached from this
    // route by condition extension and allocating nothing. The derivation's
    // own two refusals (an unpaired connect or accept, unreachable after the
    // transport checker admitted the program) land on this route's `Q085`,
    // whose condition they already are.
    let cells =
        qcore_core::socket_footprint::check_socket_cells(&lowered.program).map_err(|errors| {
            errors
                .into_iter()
                .map(|error| {
                    let span = lowered
                        .event_spans
                        .get(error.event())
                        .copied()
                        .unwrap_or(lowered.scope_span);
                    let pass = if error.is_footprint_refusal() {
                        SemanticPass::Concurrency
                    } else {
                        SemanticPass::StandardSocket
                    };
                    map_raw_diagnostic(error.code(), pass, span, error.to_string(), ranges)
                })
                .collect::<Vec<_>>()
        })?;
    qcore_core::socket_footprint::recheck_socket_cell_certificate(&lowered.program, &cells)
        .map_err(|failure| {
            vec![map_raw_diagnostic(
                "Q088",
                SemanticPass::StandardSocket,
                lowered.scope_span,
                format!("the protocol-cell certificate failed its independent recheck: {failure}"),
                ranges,
            )]
        })?;

    // The check-only arm (card `SOCKET-EVENT-COMPOSE`, inheriting
    // `source.event.v1`'s own). Both are absent together or present together: a
    // module that emits C without a reference observation, or the reverse, would
    // publish an unchecked comparison. Only a `write` registration reaches the
    // `false` branch; every program without one is executable exactly as before.
    let (c_source, observation) = if certificate.executable {
        let observation =
            std_socket_c_emit::interpret_socket_program(&lowered.program, &certificate).map_err(
                |refusal| {
                    vec![plain_diagnostic(
                        "Q089",
                        format!("the socket reference execution refused this trace: {refusal:?}"),
                    )]
                },
            )?;
        let emission = std_socket_c_emit::emit_socket_c11(&lowered.program, &certificate).map_err(
            |refusal| {
                vec![plain_diagnostic(
                    "Q089",
                    format!("the socket backend refused this trace: {refusal:?}"),
                )]
            },
        )?;
        (Some(emission.source), Some(observation.labelled()))
    } else {
        (None, None)
    };

    let result_profile = SourceResultProfile {
        scalar_results: BTreeMap::from([(entry.clone(), ExecutableResultType::Unit)]),
        // Every socket this entry opens is closed inside it — the inherited
        // ownership checker refuses a normal exit that leaves any live — so no
        // owned descriptor crosses the entry boundary.
        owned_buffer_results: std::collections::BTreeSet::new(),
        custody_payloads: CustodyPayloadProfile::NoResourcePayloads,
    };
    Ok(CompiledModule {
        source_bundle: None,
        extension_warnings: Vec::new(),
        checked: CheckedSourceModule::StandardSocket(Box::new(CheckedSocketModule {
            lowered,
            certificate,
            cells,
        })),
        execution_profile: SourceExecutionProfile::StandardSocketV1,
        declaration_count: typed.declarations.len(),
        mode,
        result_profile,
        entry: Some(entry),
        c_source,
        source_map: Vec::new(),
        // The reference observation of this route is the per-stream line trace,
        // carried on `extern_observation` beside the extern route's own trace
        // exactly as `source.buffer.v1` and `source.std-io.v1` carry theirs.
        reference_observation: None,
        extern_observation: observation,
    })
}

/// Compile one `source.http.v1` module (card `HTTP-QUIRIN-V1`).
///
/// The shape is deliberately [`compile_socket_source`]'s — lower, check,
/// **recheck**, interpret, emit — so the two routes' guarantees read the same
/// way. What differs is that the check and the emission are *compositions*: the
/// checker validates every message against RFC 9112 and then hands the
/// transport program it built to the **unedited** socket checker, and the
/// backend merges this route's fragments with the socket backend's through
/// `c_program::merge` into the one entry `c_program::assemble` writes.
fn compile_http_source(
    parsed: &SourceProgram,
    typed: &TypedSourceProgram,
    mode: SourceModuleMode,
    options: &CompileOptions,
    ranges: &[FileRange],
) -> Result<CompiledModule, Vec<FileDiagnostic>> {
    if mode != SourceModuleMode::Executable {
        return Err(vec![plain_diagnostic(
            "Q090",
            "entry-free `source.http.v1` modules require additional source lowering",
        )]);
    }
    let entry = select_entry(typed, options).map_err(|error| vec![error])?;
    let lowered = qcore_frontend::source_http::lower_source_http(parsed, typed, &entry).map_err(
        |errors| {
            errors
                .into_iter()
                .map(|error| map_diagnostic(error, ranges))
                .collect::<Vec<_>>()
        },
    )?;
    let certificate = std_http::check_http_program(&lowered.program).map_err(|errors| {
        errors
            .into_iter()
            .map(|error| {
                let span = lowered
                    .event_spans
                    .get(error.event())
                    .copied()
                    .unwrap_or(lowered.scope_span);
                map_raw_diagnostic(
                    error.code(),
                    SemanticPass::StandardHttp,
                    span,
                    error.to_string(),
                    ranges,
                )
            })
            .collect::<Vec<_>>()
    })?;
    // The driver never publishes a certificate it did not independently
    // re-derive from the same lowered program.
    std_http::recheck_http_certificate(&lowered.program, &certificate).map_err(|failure| {
        vec![map_raw_diagnostic(
            "Q090",
            SemanticPass::StandardHttp,
            lowered.scope_span,
            format!("the HTTP certificate failed its independent recheck: {failure}"),
            ranges,
        )]
    })?;

    // The protocol cells of the transport program (card `CT42-CELL-ROUTE`).
    // This route's reader hands every statement it does not spell to
    // `source.socket.v1`'s reader, so a `disjoint NAME NAME;` question lowers
    // into the transport program; before this block it was admitted with no
    // judgement, which the card measured with `quirin check` and closed here.
    // The transport is re-lowered from the checked program (`lower_transport`
    // is total on an admitted program) and its attribution map places every
    // refusal on the HTTP statement the reader wrote. A footprint refusal
    // keeps the footprint checker's own code and pass (`Q664`, `Concurrency`)
    // exactly as on the socket route; the derivation's own two refusals,
    // unreachable after the transport checker admitted the program, land on
    // `Q095` — the condition "a socket-side checker this route inherits
    // unedited refused, carrying its own value" — stated as a widening.
    let transport = std_http::lower_transport(&lowered.program).map_err(|errors| {
        errors
            .into_iter()
            .map(|error| {
                let span = lowered
                    .event_spans
                    .get(error.event())
                    .copied()
                    .unwrap_or(lowered.scope_span);
                map_raw_diagnostic(
                    error.code(),
                    SemanticPass::StandardHttp,
                    span,
                    error.to_string(),
                    ranges,
                )
            })
            .collect::<Vec<_>>()
    })?;
    let cells =
        qcore_core::socket_footprint::check_socket_cells(&transport.program).map_err(|errors| {
            errors
                .into_iter()
                .map(|error| {
                    let event = transport
                        .attribution
                        .get(error.event())
                        .copied()
                        .unwrap_or(lowered.program.statements.len());
                    let span = lowered
                        .event_spans
                        .get(event)
                        .copied()
                        .unwrap_or(lowered.scope_span);
                    if error.is_footprint_refusal() {
                        map_raw_diagnostic(
                            error.code(),
                            SemanticPass::Concurrency,
                            span,
                            error.to_string(),
                            ranges,
                        )
                    } else {
                        map_raw_diagnostic(
                            "Q095",
                            SemanticPass::StandardHttp,
                            span,
                            format!("the transport's protocol-cell derivation refused: {error}"),
                            ranges,
                        )
                    }
                })
                .collect::<Vec<_>>()
        })?;
    qcore_core::socket_footprint::recheck_socket_cell_certificate(&transport.program, &cells)
        .map_err(|failure| {
            vec![map_raw_diagnostic(
                "Q090",
                SemanticPass::StandardHttp,
                lowered.scope_span,
                format!("the protocol-cell certificate failed its independent recheck: {failure}"),
                ranges,
            )]
        })?;

    let observation = std_http_c_emit::interpret_http_program(&lowered.program, &certificate)
        .map_err(|refusal| {
            vec![plain_diagnostic(
                "Q090",
                format!("the HTTP reference execution refused this trace: {refusal:?}"),
            )]
        })?;
    let emission =
        std_http_c_emit::emit_http_c11(&lowered.program, &certificate).map_err(|refusal| {
            vec![plain_diagnostic(
                "Q090",
                format!("the HTTP backend refused this trace: {refusal:?}"),
            )]
        })?;

    let result_profile = SourceResultProfile {
        scalar_results: BTreeMap::from([(entry.clone(), ExecutableResultType::Unit)]),
        // Every socket this entry opens is closed inside it — the inherited
        // ownership checker refuses a normal exit that leaves any live — and a
        // message is a value that owns nothing.
        owned_buffer_results: std::collections::BTreeSet::new(),
        custody_payloads: CustodyPayloadProfile::NoResourcePayloads,
    };
    Ok(CompiledModule {
        source_bundle: None,
        extension_warnings: Vec::new(),
        checked: CheckedSourceModule::StandardHttp(Box::new(CheckedHttpModule {
            lowered,
            certificate,
            cells,
        })),
        execution_profile: SourceExecutionProfile::StandardHttpV1,
        declaration_count: typed.declarations.len(),
        mode,
        result_profile,
        entry: Some(entry),
        c_source: Some(emission.source),
        source_map: Vec::new(),
        reference_observation: None,
        extern_observation: Some(observation.labelled()),
    })
}

/// Compile one `source.string.v1` module (card `STRING-SLICE`).
///
/// The shape is deliberately [`compile_buffer_source`]'s — lower, check,
/// **recheck**, interpret, emit — so the two routes' guarantees read the same
/// way. What differs is that there is no custody step to inherit: a view is not
/// a resource, so this route builds no ownership operation list (see
/// [`CheckedStringModule`]), and the evidence it publishes instead is the
/// storage provenance of every view plus the §4 arithmetic of every slice.
fn compile_string_source(
    parsed: &SourceProgram,
    typed: &TypedSourceProgram,
    mode: SourceModuleMode,
    options: &CompileOptions,
    ranges: &[FileRange],
    target: Option<&RecordedTarget>,
) -> Result<CompiledModule, Vec<FileDiagnostic>> {
    if mode != SourceModuleMode::Executable {
        return Err(vec![plain_diagnostic(
            "Q068",
            "entry-free `source.string.v1` modules require additional source lowering",
        )]);
    }
    let entry = select_entry(typed, options).map_err(|error| vec![error])?;
    // Card `W48-QR-SCALAR-TYPES-R6`: the recorded target reaches the composed
    // string/buffer route's typed tier, as it reaches `source.buffer.v1`'s.
    let lowered = qcore_frontend::source_std_string::lower_source_string_for_target(
        parsed, typed, &entry, target,
    )
    .map_err(|errors| {
        errors
            .into_iter()
            .map(|error| map_diagnostic(error, ranges))
            .collect::<Vec<_>>()
    })?;
    let certificate = std_string::check_string_program_for_target(&lowered.program, target)
        .map_err(|errors| {
            errors
                .into_iter()
                .map(|error| {
                    let span = lowered
                        .event_spans
                        .get(error.event())
                        .copied()
                        .unwrap_or(lowered.scope_span);
                    map_raw_diagnostic(
                        error.code(),
                        SemanticPass::StandardString,
                        span,
                        error.to_string(),
                        ranges,
                    )
                })
                .collect::<Vec<_>>()
        })?;
    // The driver never publishes a certificate it did not independently
    // re-derive from the same lowered program.
    std_string::recheck_string_certificate(&lowered.program, &certificate).map_err(|failure| {
        vec![map_raw_diagnostic(
            "Q068",
            SemanticPass::StandardString,
            lowered.scope_span,
            format!("the string-view certificate failed its independent recheck: {failure}"),
            ranges,
        )]
    })?;

    let reference_observation =
        std_string_c_emit::interpret_string_program(&lowered.program, &certificate).map_err(
            |refusal| {
                vec![plain_diagnostic(
                    "Q068",
                    format!("the string-view reference execution refused this trace: {refusal:?}"),
                )]
            },
        )?;
    let emission =
        std_string_c_emit::emit_string_c11(&lowered.program, &certificate).map_err(|refusal| {
            vec![plain_diagnostic(
                "Q068",
                format!("the string-view backend refused this trace: {refusal:?}"),
            )]
        })?;

    let result_profile = SourceResultProfile {
        scalar_results: BTreeMap::from([(entry.clone(), ExecutableResultType::Unit)]),
        // A view owns nothing, so nothing owned crosses the entry boundary —
        // here because there is no custody at all, not because the route
        // discharged some.
        owned_buffer_results: std::collections::BTreeSet::new(),
        custody_payloads: CustodyPayloadProfile::NoResourcePayloads,
    };
    Ok(CompiledModule {
        source_bundle: None,
        extension_warnings: Vec::new(),
        checked: CheckedSourceModule::StandardString(Box::new(CheckedStringModule {
            lowered,
            certificate,
        })),
        execution_profile: SourceExecutionProfile::StandardStringV1,
        declaration_count: typed.declarations.len(),
        mode,
        result_profile,
        entry: Some(entry),
        c_source: Some(emission.source),
        source_map: Vec::new(),
        // The reference observation of this route is the per-stream line trace,
        // carried on `extern_observation` exactly as `source.buffer.v1` and
        // `source.std-io.v1` carry theirs.
        reference_observation: None,
        extern_observation: Some(reference_observation),
    })
}

/// `source.event.v1` (card `STD-EVENT`).
///
/// The shape of this function is deliberately the same as
/// [`compile_std_io_source`]'s — lower, check, **recheck**, interpret, emit —
/// because the route's guarantees are meant to be read the same way. What
/// differs is where the custody answer comes from: `check_event_source_program`
/// hands the projection to the *existing*
/// `qcore_core::event_reactor::check_event_program` unmodified, so a refusal a
/// reader sees at `Q060` is that model's own and not a second opinion about it.
///
/// A module the certificate reports as **not executable** — one containing a
/// `write` interest — is accepted with no `c_source` and no reference
/// observation, on the `accept_check_only` precedent. That is not a downgrade
/// of the check: the checking is complete and identical. It is the honest
/// statement that v1 cannot induce that registration's readiness and therefore
/// has nothing to compare a run against.
fn compile_event_source(
    parsed: &SourceProgram,
    typed: &TypedSourceProgram,
    mode: SourceModuleMode,
    options: &CompileOptions,
    ranges: &[FileRange],
) -> Result<CompiledModule, Vec<FileDiagnostic>> {
    if mode != SourceModuleMode::Executable {
        return Err(vec![plain_diagnostic(
            "Q058",
            "entry-free `source.event.v1` modules require additional source lowering",
        )]);
    }
    let entry = select_entry(typed, options).map_err(|error| vec![error])?;
    let lowered = qcore_frontend::source_event::lower_source_event(parsed, typed, &entry).map_err(
        |errors| {
            errors
                .into_iter()
                .map(|error| map_diagnostic(error, ranges))
                .collect::<Vec<_>>()
        },
    )?;
    let span_of = |statement: usize| {
        lowered
            .statement_spans
            .get(statement)
            .copied()
            .unwrap_or(lowered.scope_span)
    };
    let certificate =
        std_event::check_event_source_program(&lowered.program).map_err(|errors| {
            errors
                .into_iter()
                .map(|error| {
                    map_raw_diagnostic(
                        error.code(),
                        SemanticPass::Event,
                        span_of(error.statement()),
                        error.to_string(),
                        ranges,
                    )
                })
                .collect::<Vec<_>>()
        })?;
    // The driver never publishes a certificate it did not independently
    // re-derive from the same lowered program.
    std_event::recheck_event_certificate(&lowered.program, &certificate).map_err(|failure| {
        vec![map_raw_diagnostic(
            "Q058",
            SemanticPass::Event,
            lowered.scope_span,
            format!("the event certificate failed its independent recheck: {failure}"),
            ranges,
        )]
    })?;

    // The check-only arm. Both are absent together or present together: a
    // module that emits C without a reference observation, or the reverse,
    // would publish an unchecked comparison.
    let (c_source, observation) = if certificate.executable {
        let observation = std_event_c_emit::interpret_event_program(&lowered.program, &certificate)
            .map_err(|refusal| {
                vec![plain_diagnostic(
                    "Q058",
                    format!("the event reference execution refused this trace: {refusal:?}"),
                )]
            })?;
        let emission = std_event_c_emit::emit_event_c11(&lowered.program, &certificate).map_err(
            |refusal| {
                vec![plain_diagnostic(
                    "Q058",
                    format!("the event backend refused this trace: {refusal:?}"),
                )]
            },
        )?;
        (Some(emission.source), Some(observation))
    } else {
        (None, None)
    };

    let result_profile = SourceResultProfile {
        scalar_results: BTreeMap::from([(entry.clone(), ExecutableResultType::I32)]),
        owned_buffer_results: std::collections::BTreeSet::new(),
        custody_payloads: CustodyPayloadProfile::NoResourcePayloads,
    };
    Ok(CompiledModule {
        source_bundle: None,
        extension_warnings: Vec::new(),
        checked: CheckedSourceModule::Event(Box::new(CheckedEventModule {
            lowered,
            certificate,
        })),
        execution_profile: SourceExecutionProfile::EventV1,
        declaration_count: typed.declarations.len(),
        mode,
        result_profile,
        entry: Some(entry),
        c_source,
        source_map: Vec::new(),
        // The reference observation of this route is the event trace, carried
        // on `extern_observation` beside the standard-IO route's own line trace
        // rather than on the sequential cleanup field, which models something
        // else entirely.
        reference_observation: None,
        extern_observation: observation,
    })
}

fn compile_mutex_source(
    parsed: &SourceProgram,
    typed: &TypedSourceProgram,
    mode: SourceModuleMode,
    options: &CompileOptions,
    ranges: &[FileRange],
    target: Option<&RecordedTarget>,
) -> Result<CompiledModule, Vec<FileDiagnostic>> {
    if mode != SourceModuleMode::Executable {
        return Err(vec![plain_diagnostic(
            "Q500",
            "entry-free checked mutex modules require additional source lowering",
        )]);
    }
    let entry = select_entry(typed, options).map_err(|error| vec![error])?;
    // Card `W48-QR-SCALAR-TYPES-R6`: the recorded target sizes a
    // `protected isize`/`protected usize` payload.
    let lowered =
        qcore_frontend::source_mutex::lower_source_mutex_for_target(parsed, typed, &entry, target)
            .map_err(|errors| {
                errors
                    .into_iter()
                    .map(|error| map_diagnostic(error, ranges))
                    .collect::<Vec<_>>()
            })?;
    let certificate =
        mutex_invariant::check_mutex_invariant_program(&lowered.program).map_err(|errors| {
            errors
                .iter()
                .map(|error| map_diagnostic(lowered.locate(error), ranges))
                .collect::<Vec<_>>()
        })?;
    // The driver never publishes a certificate it did not independently
    // re-derive from the same lowered program.
    mutex_invariant::recheck_mutex_invariant_certificate(&lowered.program, &certificate).map_err(
        |errors| {
            vec![plain_diagnostic(
                "Q600",
                format!("checked mutex certificate failed independent recheck: {errors:?}"),
            )]
        },
    )?;
    let result_profile = SourceResultProfile {
        scalar_results: std::collections::BTreeMap::from([(
            entry.clone(),
            ExecutableResultType::Unit,
        )]),
        owned_buffer_results: std::collections::BTreeSet::new(),
        custody_payloads: CustodyPayloadProfile::NoResourcePayloads,
    };
    let checked = CheckedSourceModule::CheckedMutexGuards(Box::new(CheckedMutexGuardModule {
        lowered: lowered.clone(),
        certificate: certificate.clone(),
    }));
    if !cfg!(any(
        target_os = "linux",
        target_os = "macos",
        target_os = "windows"
    )) {
        return Err(vec![plain_diagnostic(
            "Q500",
            "native checked mutex emission currently requires a Linux, macOS or Windows threads target",
        )]);
    }
    // A checked trace is rendered when the backend has a rendering for its task
    // structure and published without C when it does not. Card
    // `MUTEX-SHARE-CROSS-EMISSION` gave the backend the second shape — a
    // non-nested fork/join trace, which is what the declared crossing lowers to
    // — so a trace outside *both* shapes still publishes no C, never C the
    // backend did not produce and never a silent success that dropped events.
    let c_source = match mutex_c_emit::classify_trace(&lowered.program) {
        mutex_c_emit::TraceShape::SingleTask | mutex_c_emit::TraceShape::ForkJoin { .. } => {
            let emission = mutex_c_emit::emit_typed_mutex_c11(
                &lowered.program,
                &certificate,
                &lowered.payloads(),
            )
            .map_err(|refusal| {
                vec![plain_diagnostic(
                    "Q600",
                    format!("the checked mutex backend refused this trace: {refusal:?}"),
                )]
            })?;
            Some(emission.source)
        }
        mutex_c_emit::TraceShape::Outside { .. } => None,
    };
    Ok(CompiledModule {
        source_bundle: None,
        extension_warnings: Vec::new(),
        checked,
        execution_profile: SourceExecutionProfile::CheckedMutexGuardsV1,
        declaration_count: typed.declarations.len(),
        mode,
        result_profile,
        entry: Some(entry),
        c_source,
        source_map: Vec::new(),
        // A reference observation for this route is the mutex trace, not the
        // sequential cleanup observation this field carries.
        reference_observation: None,
        extern_observation: None,
    })
}

/// Compile one `source.epoch.bound.v1` module.
///
/// The route is check-only by construction. The profile's quantities are
/// declared shapes, not resources this backend allocates, so there is nothing
/// to execute and nothing to emit: `c_source` and `reference_observation` stay
/// `None`, and no native or sanitized run is implied by an admitted epoch.
fn compile_epoch_source(
    parsed: &SourceProgram,
    typed: &TypedSourceProgram,
    mode: SourceModuleMode,
    options: &CompileOptions,
    ranges: &[FileRange],
) -> Result<CompiledModule, Vec<FileDiagnostic>> {
    if mode != SourceModuleMode::Executable {
        return Err(vec![plain_diagnostic(
            "Q500",
            "entry-free flat-epoch modules require additional source lowering",
        )]);
    }
    let entry = select_entry(typed, options).map_err(|error| vec![error])?;
    let lowered = qcore_frontend::source_epoch::lower_source_epoch(parsed, typed, &entry).map_err(
        |errors| {
            errors
                .into_iter()
                .map(|error| map_diagnostic(error, ranges))
                .collect::<Vec<_>>()
        },
    )?;
    let certificate =
        epoch_accounting::check_flat_epoch(&lowered.program).map_err(|diagnostics| {
            diagnostics
                .iter()
                .map(|diagnostic| map_diagnostic(lowered.locate(diagnostic), ranges))
                .collect::<Vec<_>>()
        })?;
    // The driver never publishes a certificate it did not independently
    // re-derive from the same lowered program.
    epoch_accounting::recheck_epoch_bound(&lowered.program, &certificate).map_err(|errors| {
        vec![plain_diagnostic(
            "Q600",
            format!("flat-epoch certificate failed independent recheck: {errors:?}"),
        )]
    })?;
    // A module that asked to read the work bound as a completion time gets the
    // profile's own answer, from the profile's own entry point. There is no
    // schedule model here, so the request is refused (`Q639`) after the bound
    // was derived: the epoch may be perfectly well accounted and the time
    // claim still not follow from it.
    // The error path over the epoch's credits, checked by the core's own
    // sibling module and re-derived before it is published, exactly as the bound
    // is (card `CT23-CREDIT-ERROR-PATH`). On this route the arms carry no error
    // edges, because a declared worker is not a creation the creator could fail
    // to make: an elided arm spends its grant and changes nothing, and a spelled
    // `error(…)` is refused by the core at `Q630`.
    let credit_recovery =
        epoch_credit_recovery::check_epoch_credit_recovery(&lowered.program, &lowered.recovery)
            .map_err(|diagnostics| {
                diagnostics
                    .iter()
                    .map(|diagnostic| map_diagnostic(lowered.locate_credit(diagnostic), ranges))
                    .collect::<Vec<_>>()
            })?;
    epoch_credit_recovery::recheck_epoch_credit_recovery(
        &lowered.program,
        &lowered.recovery,
        &credit_recovery,
    )
    .map_err(|errors| {
        vec![plain_diagnostic(
            "Q600",
            format!("flat-epoch credit split failed independent recheck: {errors:?}"),
        )]
    })?;
    if lowered.latency_claim.is_some() {
        let Err(refusal) = epoch_accounting::epoch_wall_clock_bound(&certificate) else {
            unreachable!("a wall-clock bound is never available in this profile")
        };
        return Err(vec![map_diagnostic(lowered.locate(&refusal), ranges)]);
    }
    let result_profile = SourceResultProfile {
        scalar_results: std::collections::BTreeMap::from([(
            entry.clone(),
            ExecutableResultType::Unit,
        )]),
        owned_buffer_results: std::collections::BTreeSet::new(),
        custody_payloads: CustodyPayloadProfile::NoResourcePayloads,
    };
    Ok(CompiledModule {
        source_bundle: None,
        extension_warnings: Vec::new(),
        checked: CheckedSourceModule::CheckedEpochBound(Box::new(CheckedEpochModule {
            lowered,
            certificate,
            credit_recovery,
        })),
        execution_profile: SourceExecutionProfile::FlatEpochBoundV1,
        declaration_count: typed.declarations.len(),
        mode,
        result_profile,
        entry: Some(entry),
        c_source: None,
        source_map: Vec::new(),
        reference_observation: None,
        extern_observation: None,
    })
}

/// Compile one `source.epoch.tasks.v1` module: card T08-COMPOSE.
///
/// The composition is performed here and nowhere else, and every step of it is
/// a check:
///
/// 1. The entry is split syntactically into its epoch plan and its task region.
///    The plan's bytes are blanked, not deleted, so the task half is byte-for-
///    byte the module the scoped-task route would have compiled and every span
///    it reports still points at the original file.
/// 2. The task half goes through the **unedited** scoped-task route —
///    `lower_source_tasks`, `check_task_module`, `emit_task_c11` — by calling
///    `compile_task_source` itself. Reference execution and emission are that
///    route's, unchanged.
/// 3. The epoch's *shape* is derived from the resulting `CheckedTaskModule` and
///    required to equal what the plan declared. Divergence in either direction
///    is a refusal here, at `Q600`.
/// 4. The reconciled program goes through the **unedited** `check_flat_epoch`,
///    and the driver re-derives the certificate before publishing it, exactly as
///    the pure epoch route does.
///
/// Agreement is not a bound and a bound is not agreement: both are required, and
/// neither is inferred from the other.
fn compile_composed_epoch_source(
    parsed: &SourceProgram,
    mode: SourceModuleMode,
    options: &CompileOptions,
    ranges: &[FileRange],
    target: Option<&RecordedTarget>,
) -> Result<CompiledModule, Vec<FileDiagnostic>> {
    use qcore_frontend::source_epoch_tasks;

    if mode != SourceModuleMode::Executable {
        return Err(vec![plain_diagnostic(
            "Q500",
            "entry-free composed epoch modules require additional source lowering",
        )]);
    }
    // The entry is selected from the module as written, before the plan is
    // removed: the two texts declare the same functions.
    let planned_typed = elaborate_source(parsed).map_err(|diagnostics| {
        diagnostics
            .into_iter()
            .map(|diagnostic| map_diagnostic(diagnostic, ranges))
            .collect::<Vec<_>>()
    })?;
    let entry = select_entry(&planned_typed, options).map_err(|error| vec![error])?;
    // Card `REGION-CREDIT-JOINS`: the parent's own quantitative epilogue, if the
    // entry writes one. Read and validated before the split so the statement is
    // erased from the task half exactly as the plan is — into its straight-line
    // twin: a duty-free region blanks completely (constraint 15l), and a
    // duty-carrying one (card `REGION-JOINS-DUTIES`) keeps its acquisitions and
    // releases at their own offsets with the discharge's drops written after
    // them, so the task half is an ordinary scoped-task program the unedited
    // route compiles. A region anywhere it is not admitted refuses about the
    // composition rather than about the task profile.
    let region = source_epoch_tasks::read_composed_region(parsed, &entry).map_err(|errors| {
        errors
            .into_iter()
            .map(|error| map_diagnostic(error, ranges))
            .collect::<Vec<_>>()
    })?;
    let split = source_epoch_tasks::split_composed_epoch(
        parsed,
        &entry,
        region.as_ref().map(|region| &region.erasure),
    )
    .map_err(|errors| {
        errors
            .into_iter()
            .map(|error| map_diagnostic(error, ranges))
            .collect::<Vec<_>>()
    })?;
    let task_parsed = parse_source_program(&split.task_source).map_err(|error| {
        vec![map_raw_diagnostic(
            "Q000",
            SemanticPass::NameResolution,
            error.span,
            error.message,
            ranges,
        )]
    })?;
    let task_typed = elaborate_source(&task_parsed).map_err(|diagnostics| {
        diagnostics
            .into_iter()
            .map(|diagnostic| map_diagnostic(diagnostic, ranges))
            .collect::<Vec<_>>()
    })?;
    // Card `W48-QR-TASK-RECORD-CAPTURES`: the scoped-task route now admits
    // records, but the composed epoch's derived shape reads a capture per
    // argument and a retained result per owned Buffer, and no card has
    // measured what a record's leaves mean against the epoch's allocation
    // axis. So a record on this route refuses by name, at the code and pass
    // the task route's record refusal always had (`W48-QR-TASK-RECORD-EPOCH`).
    if let Some(span) = qcore_frontend::source_record::record_declaration_span(&task_parsed) {
        return Err(vec![map_raw_diagnostic(
            "Q500",
            SemanticPass::CEmission,
            span,
            "a record type in a composed epoch module is not admitted: the epoch's derived \
             shape has no reading of a record's leaves against its allocation axis \
             (W48-QR-TASK-RECORD-EPOCH)"
                .to_owned(),
            ranges,
        )]);
    }
    // The whole scoped-task route, unedited, on the task half alone.
    let task_module =
        compile_task_source(&task_parsed, &task_typed, mode, options, ranges, target)?;
    let CheckedSourceModule::ScopedTasks(tasks) = task_module.checked else {
        unreachable!("the scoped-task route publishes scoped-task evidence")
    };
    let composed =
        source_epoch_tasks::lower_composed_epoch(parsed, &entry, split.plan_span, &tasks).map_err(
            |errors| {
                errors
                    .into_iter()
                    .map(|error| map_diagnostic(error, ranges))
                    .collect::<Vec<_>>()
            },
        )?;
    let certificate =
        epoch_accounting::check_flat_epoch(&composed.plan.program).map_err(|diagnostics| {
            diagnostics
                .iter()
                .map(|diagnostic| map_diagnostic(composed.plan.locate(diagnostic), ranges))
                .collect::<Vec<_>>()
        })?;
    epoch_accounting::recheck_epoch_bound(&composed.plan.program, &certificate).map_err(
        |errors| {
            vec![plain_diagnostic(
                "Q600",
                format!("composed epoch certificate failed independent recheck: {errors:?}"),
            )]
        },
    )?;
    // The error path over the epoch's credits (card `CT23-CREDIT-ERROR-PATH`).
    // The *edges* in this plan were derived from the checked task IR and the
    // *dispositions* were declared, so the refusals below are about a path the
    // checked region really has. Re-derived before publication, as the bound is.
    let credit_recovery = epoch_credit_recovery::check_epoch_credit_recovery(
        &composed.plan.program,
        &composed.plan.recovery,
    )
    .map_err(|diagnostics| {
        diagnostics
            .iter()
            .map(|diagnostic| map_diagnostic(composed.plan.locate_credit(diagnostic), ranges))
            .collect::<Vec<_>>()
    })?;
    epoch_credit_recovery::recheck_epoch_credit_recovery(
        &composed.plan.program,
        &composed.plan.recovery,
        &credit_recovery,
    )
    .map_err(|errors| {
        vec![plain_diagnostic(
            "Q600",
            format!("composed epoch credit split failed independent recheck: {errors:?}"),
        )]
    })?;
    if composed.plan.latency_claim.is_some() {
        let Err(refusal) = epoch_accounting::epoch_wall_clock_bound(&certificate) else {
            unreachable!("a wall-clock bound is never available in this profile")
        };
        return Err(vec![map_diagnostic(composed.plan.locate(&refusal), ranges)]);
    }
    // The region half (cards `REGION-CREDIT-JOINS` and
    // `CT24-CLEANUP-TASK-COMPOSITION`): the unedited error-region model over
    // the parent's own quantities. The straight-line erasure has already
    // placed each cleanup release at the region's source position, where the
    // unedited task checker accepts or refuses it against live child loans.
    let region = match region {
        None => None,
        Some(region) => Some(check_composed_region(&tasks, region, ranges)?),
    };
    Ok(CompiledModule {
        source_bundle: None,
        extension_warnings: Vec::new(),
        checked: CheckedSourceModule::ComposedEpochTasks(Box::new(ComposedEpochModule {
            tasks: *tasks,
            composed,
            certificate,
            credit_recovery,
            region,
            task_profile: task_module.execution_profile,
        })),
        execution_profile: SourceExecutionProfile::ComposedEpochTasksV1,
        declaration_count: task_module.declaration_count,
        mode,
        result_profile: task_module.result_profile,
        entry: task_module.entry,
        // The task half's emission and reference execution, unchanged. The
        // epoch adds a checked accounting statement on top and emits nothing of
        // its own.
        c_source: task_module.c_source,
        source_map: task_module.source_map,
        reference_observation: task_module.reference_observation,
        extern_observation: task_module.extern_observation,
    })
}

/// Check one composed region against the checked task IR and the unedited
/// `qcore_core::error_region` model (card `REGION-CREDIT-JOINS`).
///
/// Three facts are established, in order, and each is a refusal when absent:
///
/// 1. **The task route admits the erased cleanup.** Each cleanup release stands
///    at the region's source position, so a release before a child's join is
///    checked against that child's live loans by the task ledger itself. For a
///    direct task-scope region, this function also retains the prior
///    after-every-creation-and-join span check. The region model carries only
///    creator-owned duties and creator-local strict credit; it neither owns nor
///    settles a child's epoch credit.
/// 2. **The model admits the region.** `analyze_error_regions` — unedited —
///    decides every pack obligation, reported at the codes that pass already
///    allocates (`Q302`–`Q309`, `Q300`), exactly as the sequential route
///    reports them (constraint 15l).
/// 3. **The published quantity is re-derived.** The abandoned credit is read
///    out of the model's own cleanup trace and cross-checked against the
///    reader's independent fold of the body's quantities; a disagreement is
///    `Q600`, exactly as the bound's and the split's recheck failures are.
fn check_composed_region(
    tasks: &task_execution::CheckedTaskModule,
    region: qcore_frontend::source_epoch_tasks::ComposedRegion,
    ranges: &[FileRange],
) -> Result<ComposedRegionEvidence, Vec<FileDiagnostic>> {
    use qcore_core::task_execution::{TaskInstructionKind, TaskTerminatorKind};
    use qcore_frontend::source_epoch_tasks::ComposedRegionPlacement;

    if region.placement == ComposedRegionPlacement::TaskEpilogue {
        let module = tasks.module();
        let creator = &module.functions[&module.entry];
        let mut last_task_fact: Option<usize> = None;
        for block in creator.blocks.values() {
            if matches!(block.terminator.kind, TaskTerminatorKind::TrySpawn { .. }) {
                last_task_fact = last_task_fact.max(Some(block.terminator.span.end));
            }
            for instruction in &block.instructions {
                if matches!(instruction.kind, TaskInstructionKind::Join { .. }) {
                    last_task_fact = last_task_fact.max(Some(instruction.span.end));
                }
            }
        }
        if last_task_fact.is_some_and(|end| end > region.span.start) {
            return Err(vec![map_diagnostic(
                diagnostic(
                    SemanticPass::Concurrency,
                    region.span,
                    "a region written directly in the task scope is the parent's epilogue, so \
                     every creation and join of the checked task region must precede it; only \
                     the exact spawn-success-arm placement may execute cleanup before its later \
                     join (cards `REGION-CREDIT-JOINS` and \
                     `CT24-CLEANUP-TASK-COMPOSITION`)",
                ),
                ranges,
            )]);
        }
    }
    let declarations = ErrorDeclarations::new(region.declared_errors.iter().cloned());
    let allowed = EffectRow::new(
        std::iter::once(Effect::Alloc)
            .chain(region.declared_errors.iter().cloned().map(Effect::Throws)),
    );
    let exits = analyze_error_regions(
        &declarations,
        &allowed,
        ErrorLedger::default(),
        &region.operations,
    )
    .map_err(|error| {
        vec![map_diagnostic(
            SourceDiagnostic {
                code: qcore_frontend::source_region::error_region_code(&error),
                pass: SemanticPass::ErrorRegion,
                span: region.span,
                message: qcore_frontend::source_region::error_region_message(&error),
            },
            ranges,
        )]
    })?;
    let recheck_failure = |message: &str| {
        vec![map_diagnostic(
            diagnostic(SemanticPass::Concurrency, region.span, message),
            ranges,
        )]
    };
    // An admitted composed region has exactly one path: the decided edge is
    // caught and the handler returns normally, so the model publishes exactly
    // one normal exit and no error exit. Anything else is a defect of this
    // wiring, not of the program.
    let mut states = exits.iter();
    let (RegionExitTag::Normal, [state]) = states
        .next()
        .ok_or_else(|| recheck_failure("composed region produced no exit state"))?
    else {
        return Err(recheck_failure(
            "composed region produced an exit the composition does not admit",
        ));
    };
    if states.next().is_some() {
        return Err(recheck_failure(
            "composed region produced an exit the composition does not admit",
        ));
    }
    // The trace an admitted composed region can produce is exactly: one
    // `Destroy` per remaining duty in reverse acquisition order (card
    // `REGION-JOINS-DUTIES`), then the abandoned remainder when it is not
    // zero. Both halves are re-derived: the destroys against the reader's own
    // duty fold, the credit against its quantity fold — a disagreement in
    // either is a defect of this wiring, never of the program.
    let mut trace = state.cleanup_trace();
    let abandoned = match trace.last() {
        Some(CleanupEvent::AbandonCredit(amount)) => {
            trace = &trace[..trace.len() - 1];
            *amount
        }
        _ => 0,
    };
    let destroyed: Vec<u64> = trace
        .iter()
        .map(|event| match event {
            CleanupEvent::Destroy(resource) => Ok(*resource),
            _ => Err(recheck_failure(
                "composed region's cleanup trace carries an event its body cannot produce",
            )),
        })
        .collect::<Result<_, _>>()?;
    let expected: Vec<u64> = region.duties.iter().rev().map(|(_, id)| *id).collect();
    if destroyed != expected {
        return Err(recheck_failure(
            "composed region's cleanup trace failed independent re-derivation against the \
             reader's duty fold",
        ));
    }
    // The independent re-derivation: the reader's own fold over the written
    // `credit`/`effort` quantities must name the same figure the model's trace
    // does, or nothing is published.
    if region.remainder != Some(abandoned) {
        return Err(recheck_failure(
            "composed region's abandoned credit failed independent re-derivation",
        ));
    }
    Ok(ComposedRegionEvidence {
        span: region.span,
        operations: region.operations,
        abandoned_credit: abandoned,
        released: region
            .duties
            .iter()
            .rev()
            .map(|(owner, _)| owner.clone())
            .collect(),
    })
}

fn add_source_mapping(
    mut c: String,
    entry: &str,
    source: SourceLocation,
) -> (String, Vec<EmittedSourceMap>) {
    let marker = format!("static struct qcore_result qcore_fn_{entry}(void)");
    let marker_offset = c.find(&marker).unwrap_or(0);
    let generated_start_line = c[..marker_offset]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count()
        + 2;
    let function_lines = c[marker_offset..].find("\n}\n\nint main").map_or(1, |end| {
        c[marker_offset..marker_offset + end + 2].lines().count()
    });
    let directive = format!(
        "#line {} \"{}\"\n",
        source.line,
        escape_c_path(&source.path)
    );
    if let Some(offset) = c.find(&marker) {
        c.insert_str(offset, &directive);
    }
    let generated_end_line = generated_start_line + function_lines.saturating_sub(1);
    (
        c,
        vec![EmittedSourceMap {
            generated_start_line,
            generated_end_line,
            source,
        }],
    )
}

/// Card `W48-QR-C-STRUCT-IMPORT`: the verdict a `source.ffi.v1` certificate
/// publishes — `Safe` under every assumption the certificate names — or the
/// `Q035` refusal when it would be a plain `Safe` or has lost the unsafe-FFI
/// trust mark. The publication tripwire: it cannot fail for a certificate the
/// checker derived, and a certificate edited to drop the mark is caught here.
///
/// # Errors
///
/// The refusal message.
pub fn ffi_verdict(
    certificate: &qcore_core::ffi::FfiCertificate,
) -> Result<qcore_schemas::verdict::AssumedVerdict, String> {
    use qcore_core::ffi::{FfiAssumption, UNSAFE_FFI_ASSUMPTION};
    use qcore_schemas::verdict::{AssumedVerdict, Verdict};
    if !qcore_core::ffi::certificate_carries_ffi_assumption(certificate) {
        return Err(
            "the published FFI certificate does not carry the unsafe-FFI assumption naming every imported struct and every `#c_ffi` function it calls; a result from a file that imports C is never plain `Safe`".into(),
        );
    }
    let names = certificate
        .assumptions
        .iter()
        .map(|assumption| match assumption {
            FfiAssumption::UnsafeFfi { .. } => UNSAFE_FFI_ASSUMPTION.to_owned(),
            FfiAssumption::RecordedCompiler { .. } => "recorded-compiler".to_owned(),
            FfiAssumption::DeclaredContract(callee) => format!("declared-contract:{callee}"),
            FfiAssumption::BuilderIncludes { .. } => "builder-includes".to_owned(),
            // Card `W48-QRLIB-LINKAGE`: which code each called function is.
            FfiAssumption::Linked(link) => format!("linked:{}", link.callee),
        });
    let verdict = AssumedVerdict::new(Verdict::Safe, names);
    if verdict.is_plain_safe() || !verdict.holds_under(UNSAFE_FFI_ASSUMPTION) {
        return Err("an FFI result would be published as plain `Safe`".into());
    }
    Ok(verdict)
}

/// Card `W48-QRLIB-LINKAGE` (user decision 5): resolve every admitted callee
/// of an FFI module through a **generated** `.qrlib` its declaring file binds
/// with `extern library NAME;`, or refuse.
///
/// Each refusal widens an existing `NativeLibrary` condition and allocates no
/// code: a file calling C that binds no library, a binding that does not
/// resolve, or one that resolves to a hand-written v1 manifest (`Q065`); a
/// callee no manifest the file bound binds (`Q066`, including a foreign one);
/// an entry naming another contract module or another C signature (`Q041`,
/// refused, not withdrawn: the weaker reading — no manifest — is refused
/// here too); a struct position whose `struct!` header is not the manifest's
/// header, by digest (`Q064`); a slice that is not total (`Q047`).
fn resolve_ffi_links(
    parsed: &SourceProgram,
    options: &CompileOptions,
    callees: &BTreeMap<String, qcore_core::ffi::FfiCallee>,
    structs: &[qcore_core::ffi::CheckedStruct],
    spans: &BTreeMap<String, Span>,
    declared_modules: &BTreeMap<String, String>,
    ranges: &[FileRange],
) -> Result<BTreeMap<String, qcore_core::ffi::FfiLink>, Vec<FileDiagnostic>> {
    use qcore_core::ffi::{FfiLink, FfiType};
    use qcore_core::native_library::{EntryType, check_ffi_agreement};

    let mut links = BTreeMap::new();
    if callees.is_empty() {
        return Ok(links);
    }
    let first = spans
        .values()
        .next()
        .copied()
        .unwrap_or(Span { start: 0, end: 0 });
    let libraries = load_native_libraries(options, first, ranges)?;
    let views = bind_source_libraries(parsed, &libraries, ranges)?;
    qcore_core::native_library::check_slice_totality(&libraries, Some(declared_modules)).map_err(
        |refusal| {
            vec![map_raw_diagnostic(
                "Q047",
                SemanticPass::NativeLibrary,
                first,
                format!("the selected generated manifests do not bind the contract selection totally: {refusal}"),
                ranges,
            )]
        },
    )?;
    let tag_of = |name: &str| {
        structs
            .iter()
            .find(|record| record.name == name)
            .map(|record| record.tag.clone())
    };
    let mut errors = Vec::new();
    for (name, callee) in callees {
        let span = spans.get(name).copied().unwrap_or(first);
        let mut refuse = |code: &'static str, message: String| {
            errors.push(map_raw_diagnostic(
                code,
                SemanticPass::NativeLibrary,
                span,
                message,
                ranges,
            ));
        };
        let view = &views[file_index(span, ranges)];
        if !view.is_bound() {
            refuse(
                "Q065",
                format!(
                    "`{name}` is C an FFI file calls, and its file binds no library; a `#c_ffi` call resolves through a generated `.qrlib` bound with `extern library NAME;`"
                ),
            );
            continue;
        }
        if let Some(refusal) = view.foreign_symbol(name) {
            refuse("Q066", refusal.to_string());
            continue;
        }
        let Some(binding) = view.contracted_binding(name) else {
            refuse(
                "Q066",
                format!(
                    "`{name}` is bound by no generated manifest this file binds ({}); every C function an FFI file calls is linked from a pinned artifact",
                    view.bindings()
                        .iter()
                        .map(|binding| format!("`{}`", binding.named))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            );
            continue;
        };
        let (Some(linkage), Some(export)) = (binding.linkage, binding.entry.export.as_ref()) else {
            refuse(
                "Q065",
                format!(
                    "`{name}` resolves through `{}`, a hand-written v1 manifest; the FFI route links only through a generated (`native_library manifest v2`) one",
                    binding.module
                ),
            );
            continue;
        };
        if binding.entry.contract_module != callee.module {
            refuse(
                "Q041",
                format!(
                    "`{name}` is admitted through the contract `{}`, and the manifest `{}` binds it to `{}`",
                    callee.module, binding.module, binding.entry.contract_module
                ),
            );
            continue;
        }
        let project = |ty: &FfiType| -> Option<EntryType> {
            Some(match ty {
                FfiType::Unit => EntryType::Abi(qcore_core::extern_call::AbiType::Unit),
                FfiType::Scalar(scalar) => EntryType::Abi(
                    [
                        qcore_core::extern_call::AbiType::Bool,
                        qcore_core::extern_call::AbiType::I8,
                        qcore_core::extern_call::AbiType::I16,
                        qcore_core::extern_call::AbiType::I32,
                        qcore_core::extern_call::AbiType::I64,
                        qcore_core::extern_call::AbiType::Isize,
                        qcore_core::extern_call::AbiType::U8,
                        qcore_core::extern_call::AbiType::U16,
                        qcore_core::extern_call::AbiType::U32,
                        qcore_core::extern_call::AbiType::U64,
                        qcore_core::extern_call::AbiType::Usize,
                        qcore_core::extern_call::AbiType::F32,
                        qcore_core::extern_call::AbiType::F64,
                    ]
                    .into_iter()
                    .find(|abi| abi.scalar() == Some(*scalar))?,
                ),
                FfiType::Record(record) => EntryType::Struct(tag_of(record)?),
                FfiType::ConstRecordPointer(record) => EntryType::StructPointer {
                    tag: tag_of(record)?,
                    constant: true,
                },
            })
        };
        let parameters: Option<Vec<EntryType>> = callee
            .parameters
            .iter()
            .map(|(_, ty)| project(ty))
            .collect();
        let (Some(parameters), Some(result)) = (parameters, project(&callee.result)) else {
            refuse(
                "Q041",
                format!("`{name}`'s signature has no manifest spelling"),
            );
            continue;
        };
        if let Err(disagreement) = check_ffi_agreement(binding.entry, &parameters, &result) {
            refuse(
                "Q041",
                format!(
                    "`{name}` is declared otherwise by the manifest `{}`: {disagreement}",
                    binding.module
                ),
            );
            continue;
        }
        // User decision 5: the manifest's header digest must equal the header
        // the `struct!` read, for every struct the callee's signature carries.
        let tags: BTreeSet<&str> = parameters
            .iter()
            .chain([&result])
            .filter_map(|ty| match ty {
                EntryType::Struct(tag) | EntryType::StructPointer { tag, .. } => Some(tag.as_str()),
                EntryType::Abi(_) => None,
            })
            .collect();
        let mut header_ok = true;
        for tag in tags {
            let Some(record) = structs.iter().find(|record| record.tag == tag) else {
                continue;
            };
            let read = format!("sha256:{}", record.facts.header_sha256);
            match binding.header {
                Some(header) if header.digest == read => {}
                Some(header) => {
                    refuse(
                        "Q064",
                        format!(
                            "`{name}` carries `struct {tag}`, which the `struct!` read from \"{}\" at {read}, and the manifest `{}` binds the header `{}` at {}; the two must be the same bytes",
                            record.header, binding.module, header.reference, header.digest
                        ),
                    );
                    header_ok = false;
                }
                None => {
                    refuse(
                        "Q064",
                        format!(
                            "`{name}` carries `struct {tag}`, and the manifest `{}` binds no header to compare with the `struct!` read of \"{}\"",
                            binding.module, record.header
                        ),
                    );
                    header_ok = false;
                }
            }
        }
        if !header_ok {
            continue;
        }
        let named = view
            .bindings()
            .iter()
            .find(|source| source.manifest == binding.module)
            .map_or_else(String::new, |source| source.named.clone());
        links.insert(
            name.clone(),
            FfiLink {
                callee: name.clone(),
                named,
                manifest: binding.module.to_owned(),
                library: binding.identity.library.clone(),
                version: binding.identity.version.clone(),
                kind: linkage.kind,
                target: linkage.target.clone(),
                artifact: binding.identity.artifact.clone(),
                hash: binding.identity.hash.clone(),
                inputs: linkage
                    .inputs
                    .iter()
                    .map(|input| (input.reference.clone(), input.digest.clone()))
                    .collect(),
                export: export.clone(),
                runtime: linkage.runtime.clone(),
                data_model: binding.premises.data_model,
            },
        );
    }
    if errors.is_empty() {
        Ok(links)
    } else {
        Err(errors)
    }
}

/// Compile one `source.ffi.v1` module (card `W48-QR-C-STRUCT-IMPORT`).
///
/// In order, each step failing closed: the entry; the recorded target (the
/// compiler every layout is read by); every `struct!` read, its header
/// resolved against the declaring file's own directory and its layout read by
/// the recorded compiler, then its listing checked against that layout; every
/// extern declaration admitted against the selected contracts — through a
/// `#c_ffi`-marked contract whenever a position carries an imported struct;
/// the entry lowered and checked; the certificate rechecked; the trust mark's
/// tripwire; and the emission.
fn compile_ffi_source(
    parsed: &SourceProgram,
    typed: &TypedSourceProgram,
    mode: SourceModuleMode,
    options: &CompileOptions,
    ranges: &[FileRange],
    target: Option<&RecordedTarget>,
) -> Result<CompiledModule, Vec<FileDiagnostic>> {
    use qcore_core::ffi::{self, CAbiType, FfiCallee, FfiType};
    use qcore_frontend::source_ffi;

    if mode != SourceModuleMode::Executable {
        return Err(vec![plain_diagnostic(
            "Q026",
            "`source.ffi.v1` checks an executable module with one entry; an entry-free FFI module is not modeled",
        )]);
    }
    let entry = select_entry(typed, options).map_err(|error| vec![error])?;
    // Card `W48-QR-USABLE-RECORDS`: an FFI bundle is checked as a whole under
    // the unsafe-FFI assumption, and the assumption is a mark a **file** takes
    // with `#import std.ffi.unsafe;` (user decision 1). A function in a file
    // of the bundle that does not import it would be checked under a mark
    // its file never took — an imported record reaching a file that is not
    // an FFI file — so it is refused, at the import gate's own `Q032`.
    let marked_files: BTreeSet<usize> = parsed
        .imports()
        .iter()
        .filter(|import| import.value == qcore_core::module_imports::FFI_MODULE)
        .map(|import| file_index(import.span, ranges))
        .collect();
    let unmarked: Vec<FileDiagnostic> = parsed
        .declarations
        .iter()
        .filter(|declaration| declaration.value.kind == DeclarationKind::Function)
        .filter(|declaration| !marked_files.contains(&file_index(declaration.span, ranges)))
        .map(|declaration| {
            map_raw_diagnostic(
                "Q032",
                SemanticPass::UncheckedLinkage,
                declaration.span,
                format!(
                    "`{}` is checked with an FFI file's imported C records under the unsafe-FFI assumption, and its own file does not import `{}`; every file whose code an FFI module checks takes the mark itself with `#import {};`",
                    declaration.value.name,
                    qcore_core::module_imports::FFI_MODULE,
                    qcore_core::module_imports::FFI_MODULE
                ),
                ranges,
            )
        })
        .collect();
    if !unmarked.is_empty() {
        return Err(unmarked);
    }
    let imports = source_ffi::imported_structs(parsed).map_err(|errors| {
        errors
            .into_iter()
            .map(|error| map_diagnostic(error, ranges))
            .collect::<Vec<_>>()
    })?;
    let first_span = imports
        .first()
        .map_or(Span { start: 0, end: 0 }, |(_, span)| *span);
    let Some(target) = target else {
        return Err(vec![map_raw_diagnostic(
            "Q063",
            SemanticPass::NativeLibrary,
            first_span,
            "an FFI file is checked against the recorded compiler, and no recorded target was supplied (`quirin --recorded-target FILE`); a C layout is never guessed".into(),
            ranges,
        )]);
    };
    let mut structs = Vec::new();
    let mut errors = Vec::new();
    for (import, span) in &imports {
        let path = &ranges[file_index(*span, ranges)].path;
        let directory = std::path::Path::new(path)
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| std::path::Path::new("."));
        let request = qcore_c_gateway::ffi_layout::StructLayoutRequest {
            tag: &import.tag,
            header: &import.header,
            source_directory: directory,
            recorded_compiler: &target.compiler,
            recorded_triple: &target.triple,
        };
        let facts = match qcore_c_gateway::ffi_layout::read_struct_layout(&request) {
            Ok(facts) => facts,
            Err(refusal) => {
                errors.push(map_raw_diagnostic(
                    "Q063",
                    SemanticPass::NativeLibrary,
                    *span,
                    format!(
                        "the layout of `{}` (`struct {}`) is not read: {refusal}",
                        import.name, import.tag
                    ),
                    ranges,
                ));
                continue;
            }
        };
        match ffi::check_layout(import, &facts, Some(target)) {
            Ok(checked) => structs.push(checked),
            Err(refusal) => errors.push(map_raw_diagnostic(
                refusal.code(),
                SemanticPass::NativeLibrary,
                *span,
                format!(
                    "the listing of `{}` disagrees with the recorded compiler: {refusal}",
                    import.name
                ),
                ranges,
            )),
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    let names: Vec<String> = structs.iter().map(|record| record.name.clone()).collect();
    let tag_of = |name: &str| {
        structs
            .iter()
            .find(|record| record.name == name)
            .map(|record| record.tag.clone())
    };

    // Every extern declaration is admitted, called or not.
    let declarations: Vec<_> = parsed
        .declarations
        .iter()
        .filter(|declaration| declaration.value.kind == DeclarationKind::ExternFunction)
        .collect();
    let mut callees = BTreeMap::new();
    let mut links = BTreeMap::new();
    if !declarations.is_empty() {
        let first = declarations[0].span;
        let Some(sources) = options.extern_contracts.as_ref() else {
            return Err(vec![map_raw_diagnostic(
                "Q023",
                SemanticPass::ExternCall,
                first,
                "calling C from an FFI file requires an explicit extern contract selection; there is no default environment".into(),
                ranges,
            )]);
        };
        let load_refusal = |error: String| {
            vec![map_raw_diagnostic(
                "Q023",
                SemanticPass::ExternCall,
                first,
                format!("the selected extern contract environment does not load: {error}"),
                ranges,
            )]
        };
        let contracts = qcore_contract::contract_loader::load_resource_contract_sources(sources)
            .map_err(load_refusal)?;
        qcore_contract::contract_loader::check_source_assumptions(&contracts).map_err(|error| {
            vec![map_raw_diagnostic(
                "Q027",
                SemanticPass::ExternCall,
                first,
                format!(
                    "the selected extern contracts differ from their declared sources: {error}"
                ),
                ranges,
            )]
        })?;
        let marked =
            qcore_contract::contract_loader::ffi_declarations(sources).map_err(load_refusal)?;
        let signatures =
            qcore_contract::extern_bridge::declared_signatures(&contracts).map_err(load_refusal)?;
        for declaration in declarations {
            let name = &declaration.value.name;
            let span = declaration.span;
            if declaration.value.tokens.first() == Some(&TokenKind::Word("unsafe".into())) {
                errors.push(map_raw_diagnostic(
                    "Q034",
                    SemanticPass::UncheckedLinkage,
                    span,
                    format!("`{name}` is an `unsafe extern fn`: an FFI file calls C only through a selected contract, never through an unchecked-linkage waiver"),
                    ranges,
                ));
                continue;
            }
            if declaration.value.function_contract.is_some() {
                errors.push(map_raw_diagnostic(
                    "Q027",
                    SemanticPass::ExternCall,
                    span,
                    format!("`{name}` carries an inline `contract {{ … }}` block, which this profile does not enforce; the contract must come from the explicit selection"),
                    ranges,
                ));
                continue;
            }
            let mut parameters = Vec::new();
            let mut unreadable = None;
            for parameter in &declaration.value.parameters {
                match source_ffi::ffi_type(&parameter.type_source, &names) {
                    Some(FfiType::Unit) | None => {
                        unreadable = Some(parameter.type_source.clone());
                        break;
                    }
                    Some(ty) => parameters.push((parameter.name.value.clone(), ty)),
                }
            }
            let result = declaration
                .value
                .result_type
                .as_ref()
                .and_then(|result| source_ffi::ffi_type(&result.source, &names));
            let (Some(result), None) = (result, unreadable.as_ref()) else {
                errors.push(map_raw_diagnostic(
                    "Q024",
                    SemanticPass::ExternCall,
                    span,
                    format!(
                        "`{name}` declares a type `source.ffi.v1` does not call through: a parameter is a §G02t scalar, an imported struct by value or `ptr<const, STRUCT>`; a result is `unit`, a scalar or an imported struct by value{}",
                        unreadable.map(|ty| format!(" (`{ty}`)")).unwrap_or_default()
                    ),
                    ranges,
                ));
                continue;
            };
            let carries_struct = parameters
                .iter()
                .map(|(_, ty)| ty)
                .chain([&result])
                .any(|ty| matches!(ty, FfiType::Record(_) | FfiType::ConstRecordPointer(_)));
            if let Some(contract) = marked.get(name) {
                let expected = |ty: &FfiType| -> Option<CAbiType> {
                    Some(match ty {
                        FfiType::Unit => CAbiType::Unit,
                        FfiType::Scalar(scalar) => CAbiType::Scalar(*scalar),
                        FfiType::Record(record) => CAbiType::Struct(tag_of(record)?),
                        FfiType::ConstRecordPointer(record) => {
                            CAbiType::ConstStructPointer(tag_of(record)?)
                        }
                    })
                };
                let agrees = parameters.len() == contract.parameters.len()
                    && parameters.iter().zip(&contract.parameters).all(
                        |((pname, ty), (cname, cty))| {
                            pname == cname && expected(ty).as_ref() == Some(cty)
                        },
                    )
                    && expected(&result).as_ref() == Some(&contract.result);
                if !agrees {
                    errors.push(map_raw_diagnostic(
                        "Q027",
                        SemanticPass::ExternCall,
                        span,
                        format!(
                            "`{name}` is declared `({}) -> {result}` here but `({}) -> {}` in {}",
                            parameters
                                .iter()
                                .map(|(p, ty)| format!("{p}: {ty}"))
                                .collect::<Vec<_>>()
                                .join(", "),
                            contract
                                .parameters
                                .iter()
                                .map(|(p, ty)| format!("{p}: {ty}"))
                                .collect::<Vec<_>>()
                                .join(", "),
                            contract.result,
                            contract.module
                        ),
                        ranges,
                    ));
                    continue;
                }
                callees.insert(
                    name.clone(),
                    FfiCallee {
                        name: name.clone(),
                        module: contract.module.clone(),
                        parameters,
                        result,
                        marked: true,
                    },
                );
                continue;
            }
            let Some(contract) = contracts.contract(name) else {
                let message = qcore_contract::extern_bridge::pending_review_refusal(name, &contracts)
                    .unwrap_or_else(|| {
                        format!("`{name}` has no admitted declared contract in the selected extern contract environment")
                    });
                errors.push(map_raw_diagnostic(
                    "Q023",
                    SemanticPass::ExternCall,
                    span,
                    message,
                    ranges,
                ));
                continue;
            };
            let module = signatures
                .get(name)
                .map_or_else(String::new, |signature| signature.module.clone());
            if carries_struct {
                errors.push(map_raw_diagnostic(
                    "Q033",
                    SemanticPass::UncheckedLinkage,
                    span,
                    format!(
                        "`{name}` takes or returns an imported struct, and its contract in {module} is not marked `[| quant #c_ffi |]`; a call carrying a C struct is admitted only through a marked contract function, so an auditor finds every FFI use in the `.qv` files"
                    ),
                    ranges,
                ));
                continue;
            }
            // An unmarked, custody-free scalar contract: every parameter
            // borrowed, no result custody, and the declared signature equal.
            let custody_free = contract
                .parameters
                .iter()
                .all(|mode| *mode == qcore_schemas::contract::ResourceParameterMode::Borrowed)
                && contract.result == qcore_schemas::contract::ResourceReturnMode::None;
            let declared = signatures.get(name);
            let agrees = declared.is_some_and(|signature| {
                signature.parameter_names
                    == parameters
                        .iter()
                        .map(|(p, _)| p.clone())
                        .collect::<Vec<_>>()
                    && signature.parameter_types
                        == parameters
                            .iter()
                            .map(|(_, ty)| ty.to_string())
                            .collect::<Vec<_>>()
                    && signature.result_type == result.to_string()
            });
            if !custody_free || !agrees {
                errors.push(map_raw_diagnostic(
                    if agrees { "Q024" } else { "Q027" },
                    SemanticPass::ExternCall,
                    span,
                    if agrees {
                        format!("`{name}`'s contract in {module} carries custody, which `source.ffi.v1` does not model: a callee here borrows scalars and returns data")
                    } else {
                        format!("`{name}` is declared here otherwise than in its selected contract {module}")
                    },
                    ranges,
                ));
                continue;
            }
            callees.insert(
                name.clone(),
                FfiCallee {
                    name: name.clone(),
                    module,
                    parameters,
                    result,
                    marked: false,
                },
            );
        }
        if !errors.is_empty() {
            return Err(errors);
        }
        // Card `W48-QRLIB-LINKAGE`: every admitted callee resolves through a
        // generated manifest its declaring file binds.
        let spans: BTreeMap<String, Span> = parsed
            .declarations
            .iter()
            .filter(|declaration| declaration.value.kind == DeclarationKind::ExternFunction)
            .map(|declaration| (declaration.value.name.clone(), declaration.span))
            .collect();
        let declared_modules: BTreeMap<String, String> = signatures
            .iter()
            .map(|(name, signature)| (name.clone(), signature.module.clone()))
            .collect();
        links = resolve_ffi_links(
            parsed,
            options,
            &callees,
            &structs,
            &spans,
            &declared_modules,
            ranges,
        )?;
    }
    let mut lowered = source_ffi::lower_source_ffi(parsed, typed, &entry, structs, callees, target)
        .map_err(|errors| {
            errors
                .into_iter()
                .map(|error| map_diagnostic(error, ranges))
                .collect::<Vec<_>>()
        })?;
    lowered.program.links = links;
    let certificate = ffi::check_ffi_program(&lowered.program).map_err(|errors| {
        errors
            .into_iter()
            .map(|error| {
                let span = lowered
                    .statement_spans
                    .get(error.statement)
                    .copied()
                    .unwrap_or(lowered.scope_span);
                map_raw_diagnostic(
                    error.code,
                    source_ffi::pass_of(error.code),
                    span,
                    error.message,
                    ranges,
                )
            })
            .collect::<Vec<_>>()
    })?;
    ffi::recheck_ffi_certificate(&lowered.program, &certificate).map_err(|failure| {
        vec![map_raw_diagnostic(
            "Q027",
            SemanticPass::ExternCall,
            lowered.scope_span,
            format!("the FFI certificate failed its independent recheck: {failure}"),
            ranges,
        )]
    })?;
    // Card `W48-QRLIB-LINKAGE`: the linkage tripwire, `Q067`'s condition
    // widened to the FFI route — a published certificate names the pinned
    // artifact of every function it calls.
    if !ffi::certificate_carries_linkage(&certificate) {
        return Err(vec![map_raw_diagnostic(
            "Q067",
            SemanticPass::NativeLibrary,
            lowered.scope_span,
            "the published FFI certificate does not name the generated manifest's pinned artifact for every function it calls".into(),
            ranges,
        )]);
    }
    let verdict = ffi_verdict(&certificate).map_err(|message| {
        vec![map_raw_diagnostic(
            "Q035",
            SemanticPass::UncheckedLinkage,
            lowered.scope_span,
            message,
            ranges,
        )]
    })?;
    let source = qcore_core::ffi_c_emit::emit_ffi_c11(&lowered.program, &certificate).map_err(
        |refusal| {
            vec![plain_diagnostic(
                "Q500",
                format!("the `source.ffi.v1` backend refused this program: {refusal:?}"),
            )]
        },
    )?;
    let result_profile = SourceResultProfile {
        scalar_results: BTreeMap::from([(entry.clone(), ExecutableResultType::Unit)]),
        owned_buffer_results: BTreeSet::new(),
        custody_payloads: CustodyPayloadProfile::SeparateIdentityLedger,
    };
    Ok(CompiledModule {
        source_bundle: None,
        extension_warnings: Vec::new(),
        checked: CheckedSourceModule::Ffi(Box::new(CheckedFfiModule {
            lowered,
            certificate,
            verdict,
        })),
        execution_profile: SourceExecutionProfile::FfiV1,
        declaration_count: typed.declarations.len(),
        mode,
        result_profile,
        entry: Some(entry),
        c_source: Some(source),
        source_map: Vec::new(),
        reference_observation: None,
        extern_observation: None,
    })
}

fn escape_c_path(path: &str) -> String {
    path.replace('\\', "\\\\").replace('"', "\\\"")
}

fn diagnostic(pass: SemanticPass, span: Span, message: impl Into<String>) -> SourceDiagnostic {
    let error = DriverError(message.into());
    qcore_frontend::frontend::source_diagnostic(pass, span, &error)
}

fn map_diagnostic(diagnostic: SourceDiagnostic, ranges: &[FileRange]) -> FileDiagnostic {
    map_raw_diagnostic(
        diagnostic.code,
        diagnostic.pass,
        diagnostic.span,
        diagnostic.message,
        ranges,
    )
}

fn map_raw_diagnostic(
    code: &str,
    pass: SemanticPass,
    span: Span,
    message: String,
    ranges: &[FileRange],
) -> FileDiagnostic {
    FileDiagnostic {
        code: code.into(),
        pass,
        location: locate(span.start, ranges),
        message,
    }
}

fn locate(offset: usize, ranges: &[FileRange]) -> SourceLocation {
    // `FileRange::end` is the *exclusive* end of a file in the combined buffer,
    // and `combine_files` starts the next file at exactly that offset whenever
    // the previous file already ended with a newline. Testing `offset <=
    // range.end` therefore let byte 0 of file n+1 satisfy file n's predicate,
    // so any diagnostic at the very start of a non-first input file was blamed
    // on the previous file at a line one past its end. The owning file is now
    // the one that actually contains the offset. An offset belonging to no
    // file — the separator newline `combine_files` appends after a file that
    // did not end with one, or the very end of the combined buffer — falls
    // back to the last file starting at or before it, so an end-of-input span
    // still resolves.
    let index = ranges
        .iter()
        .position(|range| range.start <= offset && offset < range.end)
        .or_else(|| ranges.iter().rposition(|range| range.start <= offset))
        .unwrap_or(0);
    let range = &ranges[index];
    let local = offset.saturating_sub(range.start).min(range.source.len());
    let before = &range.source[..local];
    let line = before.bytes().filter(|byte| *byte == b'\n').count() + 1;
    let line_start = before.rfind('\n').map_or(0, |index| index + 1);
    let column = before[line_start..].chars().count() + 1;
    let line_source = range.source[local.saturating_sub(local - line_start)..]
        .lines()
        .next()
        .unwrap_or("")
        .to_owned();
    SourceLocation {
        path: range.path.clone(),
        line,
        column,
        line_source,
    }
}

fn plain_diagnostic(code: &str, message: impl Into<String>) -> FileDiagnostic {
    FileDiagnostic {
        code: code.into(),
        pass: SemanticPass::NameResolution,
        location: SourceLocation {
            path: "<command-line>".into(),
            line: 1,
            column: 1,
            line_source: String::new(),
        },
        message: message.into(),
    }
}

pub fn format_diagnostic(diagnostic: &FileDiagnostic) -> String {
    let mut output = format!(
        "{}:{}:{}: {}: {}",
        diagnostic.location.path,
        diagnostic.location.line,
        diagnostic.location.column,
        diagnostic.code,
        diagnostic.message
    );
    if !diagnostic.location.line_source.is_empty() {
        output.push('\n');
        output.push_str(&diagnostic.location.line_source);
        output.push('\n');
        output.push_str(&" ".repeat(diagnostic.location.column.saturating_sub(1)));
        output.push('^');
    }
    output
}

/// Typed pure source calls retain their own evidence rather than an erased CFG.
fn compile_scalar_calls(
    parsed: &SourceProgram,
    typed: &TypedSourceProgram,
    mode: SourceModuleMode,
    options: &CompileOptions,
    ranges: &[FileRange],
    target: Option<&RecordedTarget>,
) -> Result<CompiledModule, Vec<FileDiagnostic>> {
    use qcore_core::scalar_calls::{self, Type};
    // Card `QR-PRINT-ROUTE-MATCHES-DECLARATION`'s secondary guard (a
    // `module contract;` module is never lowered here) is deleted by card
    // `ROUTE-SELECTION-ORDER-RESTATED`: this function is called only by the
    // lowering entries of four source-only rows of `source_routes::SOURCE_ROUTES`,
    // and the contract guard refuses a contract module on every such row
    // before its lowering runs (Lean
    // `QCore.RouteSelection.contract_never_lowered_on_source_only`).
    let lowered = qcore_frontend::source_scalar_calls::lower(parsed)
        .map_err(|e| vec![map_diagnostic(e, ranges)])?;
    let report = |e: scalar_calls::Refusal| {
        vec![map_raw_diagnostic(
            "Q020",
            SemanticPass::OrdinaryTypes,
            lowered
                .spans
                .get(e.function)
                .copied()
                .unwrap_or(Span { start: 0, end: 0 }),
            format!("source.scalar-calls.v1: {}", e.message),
            ranges,
        )]
    };
    let checked = scalar_calls::check_for_target(lowered.program, target).map_err(report)?;
    // `ExecutableResultType` names the i32/unit results a caller can run; a
    // function of another scalar type is checked but is not one of them.
    let scalar_results = lowered
        .names
        .iter()
        .cloned()
        .zip(checked.program().functions.iter())
        .filter_map(|(name, f)| match f.result {
            Type::I32 => Some((name, ExecutableResultType::I32)),
            Type::Unit => Some((name, ExecutableResultType::Unit)),
            // Card `W48-QR-USABLE-RECORDS`: a record result is checked, and is
            // not a result a caller outside the module runs.
            Type::Scalar(_) | Type::Record(_) => None,
        })
        .collect();
    let (entry, c_source, reference_observation, line_observation) =
        if mode == SourceModuleMode::Executable {
            let name = select_entry(typed, options).map_err(|e| vec![e])?;
            let id = lowered
                .names
                .iter()
                .position(|n| n == &name)
                .expect("selected source entry");
            let result = checked.program().functions[id].result;
            if result != Type::I32 && result != Type::Unit {
                return Err(report(scalar_calls::Refusal {
                    function: id,
                    message: format!(
                        "an executable entry returns i32 or unit; `{name}` returns {}",
                        checked.program().type_name(result)
                    ),
                }));
            }
            let observed = checked.observe_values(id, &[]).map_err(report)?;
            let value = observed.value.unwrap_or(0);
            let c = qcore_core::scalar_calls_c_emit::emit(&checked, id).map_err(report)?;
            // The per-stream line trace, published beside the value exactly as
            // `source.std-io.v1` publishes its own; absent when the program
            // renders no line, so a module this route admitted before publishes
            // what it published then.
            let lines = (!observed.lines.is_empty()).then(|| {
                observed
                    .lines
                    .iter()
                    .map(|line| format!("{}: {}", line.stream.label(), line.text))
                    .collect::<Vec<_>>()
            });
            // Card `W48-QR-SCALAR-TYPES-R2`: an entry that propagated
            // `ArithmeticError` publishes it as the outcome, with its tag.
            let outcome = match observed.error {
                Some(error) => qcore_core::cfg::ExecutionOutcome::Error {
                    name: error.to_string(),
                    tag: u32::from(error.tag()),
                },
                None => qcore_core::cfg::ExecutionOutcome::Ok {
                    value,
                    payload: Vec::new(),
                },
            };
            (
                Some(name),
                Some(c),
                Some(ExecutionObservation {
                    outcome,
                    trace: Vec::new(),
                }),
                lines,
            )
        } else {
            (None, None, None, None)
        };
    Ok(CompiledModule {
        source_bundle: None,
        extension_warnings: Vec::new(),
        checked: CheckedSourceModule::ScalarCalls(Box::new(checked)),
        execution_profile: SourceExecutionProfile::ScalarCallsV1,
        declaration_count: typed.declarations.len(),
        mode,
        result_profile: SourceResultProfile {
            scalar_results,
            owned_buffer_results: BTreeSet::new(),
            custody_payloads: CustodyPayloadProfile::NoResourcePayloads,
        },
        entry,
        c_source,
        reference_observation,
        source_map: Vec::new(),
        extern_observation: line_observation,
    })
}

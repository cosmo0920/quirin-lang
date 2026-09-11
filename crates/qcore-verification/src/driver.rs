//! Source-file orchestration for the sequential cleanup and scoped task
//! profiles. Each path produces its own checked evidence and rejects unsupported
//! forms before C emission. Task source is never represented by an erased
//! sequential CFG.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use qcore_core::cleanup_lowering::{
    BitwiseOp, BranchCondition, CompareOp, IntExpr, SourceOp, SourceProgram as LoweringProgram,
    interpret_source,
};
use qcore_core::ownership::{Footprint, OwnershipOp, ResourceId};
use qcore_core::prelude::concurrency::{
    epoch_accounting, mutex_c_emit, mutex_invariant, task_c_emit, task_execution,
};
use qcore_core::resource_flow::{Flow, Quantity, ResourceBounds};
use qcore_core::typed_errors::{Effect, EffectRow, ErrorDeclarations, ErrorLedger, ErrorOp};
use qcore_core::usage::{Usage, UsageExpr};
use qcore_core::{ExecutionObservation, emit_c11_with_log_level, extern_c_emit, extern_call};
use qcore_core::{
    composed_std_io, composed_std_io_c_emit, composed_tasks, std_buffer, std_buffer_c_emit,
    std_event, std_event_c_emit, std_io, std_io_c_emit, std_string, std_string_c_emit,
};
use qcore_frontend::frontend::{
    DeclarationKind, ExecutableResultType, Located, SemanticPass, SourceBlock, SourceDeclaration,
    SourceDiagnostic, SourceModuleMode, SourceProgram, StatementKind, TypedSourceDeclaration,
    TypedSourceProgram, elaborate_source, parse_source_program,
};
use qcore_frontend::lexer::{Span, Symbol, Token, TokenKind};
use qcore_frontend::whole_program::{
    CallSite, CheckedProgram, FunctionSemantics, FunctionSummary, ParameterOwnership,
    check_whole_program,
};
use qcore_verifier_host::VerifierRegistry;

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
/// checked task-owned Buffer results. General owned/error payload types remain
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
}

/// Checked evidence is explicit about the executed semantic profile. Task
/// modules never carry a sequential CFG with their task operations erased.
#[derive(Clone, Debug)]
pub enum CheckedSourceModule {
    Sequential(Box<CheckedProgram>),
    ScopedTasks(Box<task_execution::CheckedTaskModule>),
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
    /// The scoped-task profile the task half executes under, kept because the
    /// composed label would otherwise hide whether the region moves owned
    /// Buffers or only copies scalars.
    pub task_profile: SourceExecutionProfile,
}

impl CheckedSourceModule {
    pub fn as_sequential(&self) -> Option<&CheckedProgram> {
        match self {
            Self::Sequential(program) => Some(program),
            Self::ScopedTasks(_)
            | Self::CheckedMutexGuards(_)
            | Self::CheckedEpochBound(_)
            | Self::ComposedEpochTasks(_)
            | Self::ExternCalls(_)
            | Self::StandardIo(_)
            | Self::Composed(_)
            | Self::ComposedTasks(_)
            | Self::StandardBuffer(_)
            | Self::StandardString(_)
            | Self::Event(_) => None,
        }
    }

    /// The scoped-task evidence of a module whose *whole* checked evidence is
    /// that task module. A composed epoch answers `None` here on purpose: its
    /// task evidence is one half of a pair, and a consumer that wants it must
    /// ask for the composition and see the accounting half beside it.
    pub fn as_scoped_tasks(&self) -> Option<&task_execution::CheckedTaskModule> {
        match self {
            Self::Sequential(_)
            | Self::CheckedMutexGuards(_)
            | Self::CheckedEpochBound(_)
            | Self::ComposedEpochTasks(_)
            | Self::ExternCalls(_)
            | Self::StandardIo(_)
            | Self::Composed(_)
            | Self::ComposedTasks(_)
            | Self::StandardBuffer(_)
            | Self::StandardString(_)
            | Self::Event(_) => None,
            Self::ScopedTasks(program) => Some(program),
        }
    }

    pub fn as_checked_mutex_guards(&self) -> Option<&CheckedMutexGuardModule> {
        match self {
            Self::Sequential(_)
            | Self::ScopedTasks(_)
            | Self::CheckedEpochBound(_)
            | Self::ComposedEpochTasks(_)
            | Self::ExternCalls(_)
            | Self::StandardIo(_)
            | Self::Composed(_)
            | Self::ComposedTasks(_)
            | Self::StandardBuffer(_)
            | Self::StandardString(_)
            | Self::Event(_) => None,
            Self::CheckedMutexGuards(module) => Some(module),
        }
    }

    /// The certificate of a module whose whole checked evidence is the flat
    /// epoch. A composed epoch answers `None` here for the same reason
    /// `as_scoped_tasks` does.
    pub fn as_checked_epoch_bound(&self) -> Option<&CheckedEpochModule> {
        match self {
            Self::Sequential(_)
            | Self::ScopedTasks(_)
            | Self::CheckedMutexGuards(_)
            | Self::ComposedEpochTasks(_)
            | Self::ExternCalls(_)
            | Self::StandardIo(_)
            | Self::Composed(_)
            | Self::ComposedTasks(_)
            | Self::StandardBuffer(_)
            | Self::StandardString(_)
            | Self::Event(_) => None,
            Self::CheckedEpochBound(module) => Some(module),
        }
    }

    pub fn as_composed_epoch_tasks(&self) -> Option<&ComposedEpochModule> {
        match self {
            Self::Sequential(_)
            | Self::ScopedTasks(_)
            | Self::CheckedMutexGuards(_)
            | Self::CheckedEpochBound(_)
            | Self::ExternCalls(_)
            | Self::StandardIo(_)
            | Self::Composed(_)
            | Self::ComposedTasks(_)
            | Self::StandardBuffer(_)
            | Self::StandardString(_)
            | Self::Event(_) => None,
            Self::ComposedEpochTasks(module) => Some(module),
        }
    }

    pub fn as_extern_calls(&self) -> Option<&CheckedExternModule> {
        match self {
            Self::Sequential(_)
            | Self::ScopedTasks(_)
            | Self::CheckedMutexGuards(_)
            | Self::CheckedEpochBound(_)
            | Self::ComposedEpochTasks(_)
            | Self::StandardIo(_)
            | Self::Composed(_)
            | Self::ComposedTasks(_)
            | Self::StandardBuffer(_)
            | Self::StandardString(_)
            | Self::Event(_) => None,
            Self::ExternCalls(module) => Some(module),
        }
    }

    pub fn as_standard_io(&self) -> Option<&CheckedStdIoModule> {
        match self {
            Self::Sequential(_)
            | Self::ScopedTasks(_)
            | Self::CheckedMutexGuards(_)
            | Self::CheckedEpochBound(_)
            | Self::ComposedEpochTasks(_)
            | Self::ExternCalls(_)
            | Self::Composed(_)
            | Self::ComposedTasks(_)
            | Self::StandardBuffer(_)
            | Self::StandardString(_)
            | Self::Event(_) => None,
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
            Self::Sequential(_)
            | Self::ScopedTasks(_)
            | Self::CheckedMutexGuards(_)
            | Self::CheckedEpochBound(_)
            | Self::ComposedEpochTasks(_)
            | Self::ExternCalls(_)
            | Self::StandardIo(_)
            | Self::ComposedTasks(_)
            | Self::StandardBuffer(_)
            | Self::StandardString(_)
            | Self::Event(_) => None,
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
            Self::Sequential(_)
            | Self::ScopedTasks(_)
            | Self::CheckedMutexGuards(_)
            | Self::CheckedEpochBound(_)
            | Self::ComposedEpochTasks(_)
            | Self::ExternCalls(_)
            | Self::StandardIo(_)
            | Self::Composed(_)
            | Self::StandardBuffer(_)
            | Self::StandardString(_)
            | Self::Event(_) => None,
            Self::ComposedTasks(module) => Some(module),
        }
    }

    /// The checked byte-buffer evidence (card `STD-BUFFER`).
    pub fn as_standard_buffer(&self) -> Option<&CheckedBufferModule> {
        match self {
            Self::Sequential(_)
            | Self::ScopedTasks(_)
            | Self::CheckedMutexGuards(_)
            | Self::CheckedEpochBound(_)
            | Self::ComposedEpochTasks(_)
            | Self::ExternCalls(_)
            | Self::StandardIo(_)
            | Self::Composed(_)
            | Self::ComposedTasks(_)
            | Self::StandardString(_)
            | Self::Event(_) => None,
            Self::StandardBuffer(module) => Some(module),
        }
    }

    /// The checked string-view evidence (card `STRING-SLICE`).
    pub fn as_standard_string(&self) -> Option<&CheckedStringModule> {
        match self {
            Self::Sequential(_)
            | Self::ScopedTasks(_)
            | Self::CheckedMutexGuards(_)
            | Self::CheckedEpochBound(_)
            | Self::ComposedEpochTasks(_)
            | Self::ExternCalls(_)
            | Self::StandardIo(_)
            | Self::Composed(_)
            | Self::ComposedTasks(_)
            | Self::StandardBuffer(_)
            | Self::Event(_) => None,
            Self::StandardString(module) => Some(module),
        }
    }

    pub fn function_count(&self) -> usize {
        match self {
            Self::Sequential(program) => program.functions.len(),
            Self::ScopedTasks(program) => program.module().functions.len(),
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
            Self::StandardBuffer(_) => 1,
            // And likewise for the string view: one flat entry is lowered, and
            // the profile admits no calls at all.
            Self::StandardString(_) => 1,
            // And likewise for the event route: one entry is lowered, and the
            // profile admits no calls at all.
            Self::Event(_) => 1,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceExecutionProfile {
    SequentialCleanupV1,
    ScopedCopyTasksV1,
    ScopedOwnedTasksV1,
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
}

#[derive(Clone, Debug)]
pub struct CompiledModule {
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
pub fn compile_files_with_registry(
    files: &[SourceFile],
    options: &CompileOptions,
    registry: &VerifierRegistry,
) -> Result<CompiledModule, Vec<FileDiagnostic>> {
    if files.is_empty() {
        return Err(vec![plain_diagnostic(
            "Q400",
            "no input files were provided",
        )]);
    }
    let (_, ranges) = combine_files(files);
    let mut stripped = Vec::new();
    let mut modes = Vec::new();
    for (file, range) in files.iter().zip(&ranges) {
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
        qcore_frontend::module_imports::check_source_imports(&parsed).map_err(|errors| {
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
        })?;
        modes.push(parsed.mode);
        let mut source = file.source.clone();
        if let Some(span) = parsed.mode_span {
            let blank: String = source.as_bytes()[span.start..span.end]
                .iter()
                .map(|byte| if *byte == b'\n' { '\n' } else { ' ' })
                .collect();
            source.replace_range(span.start..span.end, &blank);
        }
        stripped.push(SourceFile::new(&file.path, source));
    }
    // Contract declarations can accompany either a library or an executable.
    // An unmarked file keeps the existing executable compilation behavior.
    let mode = if modes.contains(&SourceModuleMode::Executable) {
        SourceModuleMode::Executable
    } else if modes.contains(&SourceModuleMode::Library) {
        SourceModuleMode::Library
    } else {
        SourceModuleMode::Contract
    };
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
    if qcore_frontend::source_epoch_tasks::has_composed_epoch(&parsed) {
        // Selected before the pure epoch route: a module carrying both an epoch
        // scope and the scoped-task profile's own marker is accounting for a
        // real task region, and must not be handed to a route whose workers are
        // declarations with no bodies.
        return compile_composed_epoch_source(&parsed, mode, options, &ranges);
    }
    if qcore_frontend::source_epoch::has_source_epoch(&parsed) {
        // Selected before every other concurrent route: an epoch scope is also
        // a `thread scope`, and a module that declares a quantitative epoch
        // must not be handed to a route that models no accounting at all.
        return compile_epoch_source(&parsed, &typed, mode, options, &ranges);
    }
    if qcore_frontend::source_mutex::has_source_mutex(&parsed) {
        // Selected before the task route: a `thread scope` alone also selects
        // scoped tasks, and a module declaring a mutex generation must not be
        // handed to a route that models no mutex at all.
        return compile_mutex_source(&parsed, &typed, mode, options, &ranges);
    }
    // Selected after every *more specific* concurrent route and before the task
    // route it composes (card `COMPOSE-CONCURRENCY`). The order is what keeps
    // every other exclusion intact: a module carrying an epoch scope or a mutex
    // generation never reaches this line, and a module that also extern-calls
    // fails this conjunction and falls through to the task route with its
    // existing `Q600`.
    if qcore_frontend::source_composed_tasks::has_source_composed_tasks(&parsed) {
        return compile_composed_task_source(&parsed, &typed, mode, options, &ranges);
    }
    if qcore_frontend::source_tasks::has_source_tasks(&parsed) {
        return compile_task_source(&parsed, &typed, mode, options, &ranges);
    }
    // Selected after every concurrent route and before both routes it composes
    // (card `STD-IO-COMPOSE`). The order is what makes the composition safe to
    // add: a module that spawns *and* prints, or spawns *and* extern-calls,
    // never reaches here, so both concurrency exclusions keep their existing
    // refusals; and a module using one of the two composed feature sets fails
    // this conjunction and falls through to that route byte for byte.
    if qcore_frontend::source_composed::has_source_composed(&parsed) {
        return compile_composed_source(&parsed, &typed, mode, options, &ranges);
    }
    // Selected after the concurrent routes on purpose: a module that both
    // spawns and calls an extern function stays with the task route, which
    // keeps its existing refusal. Lifting that is a separate card, because a
    // task worker is a separate checked model (design §3.4).
    if qcore_frontend::source_extern::has_source_extern(&parsed) {
        return compile_extern_source(&parsed, &typed, mode, options, &ranges);
    }
    // Card `STD-EVENT`. Selected after every concurrent route and after the
    // extern route, and **before** the standard-IO route, on the same principle
    // those two state: a module that both dispatches and prints stays with the
    // more specific model, which keeps its existing refusal for the statements
    // it does not model. Composing an event context with source lines is a
    // follow-on, not this card — and it is a real one, because a `print` in a
    // dispatch turn is exactly the shape a reordering-aware comparison would
    // have to cover.
    if qcore_frontend::source_event::has_source_event(&parsed) {
        return compile_event_source(&parsed, &typed, mode, options, &ranges);
    }
    // Selected last among the special routes, on the same principle as the
    // extern route above: a module that both prints and spawns stays with the
    // more specific model, which keeps its existing refusal for the statements
    // it does not model. Lifting *that* composition is still a separate card —
    // a worker is a separate checked model. The print-and-extern-call
    // composition is no longer a refusal: card `STD-IO-COMPOSE` routes it above,
    // and reaching this line means the module uses this route's statements and
    // no extern call.
    // Card `STD-BUFFER`, selected after the extern route above and before the
    // standard-IO route below. Both neighbours are deliberate:
    //
    // * **After the extern route**, so a module that both allocates a buffer and
    //   calls a declared extern function stays with the extern route and keeps
    //   that route's existing refusal for the statements it does not model. That
    //   is the answer to "does a buffer cross the extern boundary" — no, and the
    //   refusal is an *existing* route's rather than a new one this card wrote.
    //   A C04 pointer is an extentless `u8` projection, so admitting the
    //   crossing would discard the extent this route carries and replace it with
    //   a premise (`STD-BUFFER-EXTERN`).
    // * **Before the standard-IO route**, so a module that both buffers and
    //   prints reaches this route, which models both — its line-producing events
    //   are `source.std-io.v1`'s own, read by that route's own reader and
    //   rendered by that route's own emitter. A module that only prints fails
    //   this conjunction and falls through byte for byte.
    //
    // Every concurrent route is selected above both, so a module that buffers
    // and spawns never reaches here and keeps its `Q600`.
    if qcore_frontend::source_std_buffer::has_source_buffer(&parsed) {
        return compile_buffer_source(&parsed, &typed, mode, options, &ranges);
    }
    // Card `STRING-SLICE`, selected after the buffer route above and before the
    // standard-IO route below. Both neighbours are deliberate, on exactly the
    // principle its two siblings state about each other:
    //
    // * **After the buffer route**, so a module that both holds an owned buffer
    //   and constructs a string view stays with the buffer route and keeps that
    //   route's existing refusal for the statements it does not model. That is
    //   the v1 answer to amendment §5 — `std.buffer` is not implicitly
    //   convertible to `string` — and the refusal is an *existing* route's
    //   rather than a new one this card wrote. Admitting the composition needs a
    //   checked UTF-8 borrow or freeze with mutation exclusion, which is the
    //   named follow-on `STRING-SLICE-BUFFER-BORROW`.
    // * **Before the standard-IO route**, so a module that both slices and
    //   prints reaches this route, which models both — its line-producing events
    //   are `source.std-io.v1`'s own, read by that route's own reader and
    //   rendered by that route's own emitter, over **scalars only**. A module
    //   that only prints fails this conjunction and falls through byte for byte.
    //
    // The extern route is selected above both, so a module that constructs a
    // string and calls a declared extern function never reaches here and keeps
    // that route's existing refusal. That is amendment §6's crossing boundary,
    // measured rather than asserted: no C04 pointer contract transports this
    // aggregate or its extent, and supplying one is `STRING-SLICE-ABI`. Every
    // concurrent route is selected above all three, so a module that strings and
    // spawns keeps its `Q600`.
    if qcore_frontend::source_std_string::has_source_string(&parsed) {
        return compile_string_source(&parsed, &typed, mode, options, &ranges);
    }
    if qcore_frontend::source_std_io::has_source_std_io(&parsed) {
        return compile_std_io_source(&parsed, &typed, mode, options, &ranges);
    }
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
    let mut semantics = synthesize_semantics(&parsed, &typed, &error_names)
        .map_err(|diagnostic| vec![map_diagnostic(diagnostic, &ranges)])?;
    mark_recursive_contracts(&mut semantics);
    let semantic_calls = semantics
        .iter()
        .map(|function| (function.symbol, function.calls.clone()))
        .collect::<BTreeMap<_, _>>();
    let checked = check_whole_program(
        &typed,
        &ErrorDeclarations::new(error_names.iter().cloned()),
        semantics,
    )
    .map_err(|diagnostics| {
        diagnostics
            .into_iter()
            .map(|diagnostic| map_diagnostic(diagnostic, &ranges))
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
    let entry = select_entry(&typed, options).map_err(|diagnostic| vec![diagnostic])?;
    let symbol = typed.globals[&entry];
    let function = &checked.functions[&symbol];
    validate_emittable_calls(symbol, &semantic_calls, &checked)
        .map_err(|diagnostic| vec![map_diagnostic(diagnostic, &ranges)])?;
    if !function.source.parameters.is_empty() {
        return Err(vec![map_raw_diagnostic(
            "Q020",
            SemanticPass::OrdinaryTypes,
            function.source.span,
            "the emitted entry function must not have parameters".into(),
            &ranges,
        )]);
    }
    let reference_observation = qcore_core::interpret_cfg(&function.cfg).map_err(|error| {
        vec![map_raw_diagnostic(
            "Q202",
            SemanticPass::CleanupLowering,
            function.source.span,
            error.to_string(),
            &ranges,
        )]
    })?;
    let log_level = qcore_frontend::source_std_io::entry_log_level(&parsed, &entry);
    let raw_c = emit_c11_with_log_level(&function.cfg, log_level).map_err(|error| {
        vec![map_raw_diagnostic(
            "Q500",
            SemanticPass::CEmission,
            function.source.span,
            error.to_string(),
            &ranges,
        )]
    })?;
    let source_location = locate(function.source.span.start, &ranges);
    let (c_source, source_map) = add_source_mapping(raw_c, &entry, source_location);
    Ok(CompiledModule {
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
    };
    let mut sequence = SequenceLowering::default();
    builder.lower_sequence(&body.statements, &mut sequence, ArmPosition::TopLevel)?;

    let FunctionLowering {
        resources,
        usage_declarations,
        calls,
        mut resource_segments,
        counters,
        ..
    } = builder;
    resource_segments.push(sequence.flow);
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
        allowed_effects: EffectRow::new([Effect::Alloc]),
        initial_error_ledger: ErrorLedger::default(),
        error_operations: sequence.error_operations,
        region_operations: Vec::new(),
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
            if !matches!(statement.value, StatementKind::If { .. })
                && !statement.value.nested_blocks().is_empty()
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
                    // Card `BITWISE-CONDITIONS`: a check-time constant integer
                    // binding, the decidable subset of `IF-CONDITION-BINDINGS`.
                    // Tried **after** `qcore_acquire` cannot match, and only at
                    // the top level of an entry body: an arm-local binding has
                    // no position after the join, so an arm (or a continuation
                    // hoisted into one, card `IF-ARMS`) keeps the refusal.
                    if let Some(value) = parse_constant_binding(&tokens) {
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
                    self.resources.push(resource);
                    self.owners.insert(binding.clone(), resource);
                    // Distinguish a local from a shadowed parameter. Ownership and
                    // implicit error cleanup remain the ownership pass's concern.
                    let usage_name = format!("$local:{}:{binding}", statement.span.start);
                    self.usage_declarations
                        .insert(usage_name.clone(), Usage::MANY);
                    self.local_usage_names.insert(binding.clone(), usage_name);
                    self.counters.reachable &= succeeds;
                    output.lowering.push(SourceOp::Acquire {
                        resource,
                        succeeds,
                        error,
                    });
                    output.ownership.push(OwnershipOp::Acquire {
                        id: resource,
                        owner: binding.clone(),
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
                        .map_err(|error| {
                            diagnostic(
                                SemanticPass::ResourceFlow,
                                statement.span,
                                error.to_string(),
                            )
                        })?;
                    self.counters.live = self.counters.live.saturating_add(1);
                    self.counters.peak_live = self.counters.peak_live.max(self.counters.live);
                    self.counters.total = self.counters.total.saturating_add(1);
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
                    output.lowering.push(SourceOp::Drop(resource));
                    if self.counters.reachable {
                        output.demands.push((
                            UsageExpr::Variable(self.local_usage_names[place].clone()),
                            false,
                        ));
                    }
                    output.ownership.push(OwnershipOp::Drop(place.clone()));
                    output.error_operations.push(ErrorOp::Drop(resource));
                    output.flow = output
                        .flow
                        .then(&Flow::release(
                            Quantity::single("resources", 1),
                            Quantity::zero(),
                        ))
                        .map_err(|error| {
                            diagnostic(
                                SemanticPass::ResourceFlow,
                                statement.span,
                                error.to_string(),
                            )
                        })?;
                    self.counters.live = self.counters.live.saturating_sub(1);
                }
                StatementKind::Move { place: Some(place) } => {
                    let resource = *self.owners.get(place).ok_or_else(|| {
                        diagnostic(
                            SemanticPass::Ownership,
                            statement.span,
                            format!("unknown resource binding `{place}`"),
                        )
                    })?;
                    output.lowering.push(SourceOp::TransferToResult(resource));
                    if self.counters.reachable {
                        output.demands.push((
                            UsageExpr::Variable(self.local_usage_names[place].clone()),
                            false,
                        ));
                    }
                    output
                        .ownership
                        .push(OwnershipOp::TransferOut(place.clone()));
                    output.error_operations.push(ErrorOp::Drop(resource));
                    output.flow = output
                        .flow
                        .then(&Flow::release(
                            Quantity::single("resources", 1),
                            Quantity::zero(),
                        ))
                        .map_err(|error| {
                            diagnostic(
                                SemanticPass::ResourceFlow,
                                statement.span,
                                error.to_string(),
                            )
                        })?;
                    self.counters.live = self.counters.live.saturating_sub(1);
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
fn parse_constant_binding(tokens: &[Token]) -> Option<i32> {
    let kinds: Vec<&TokenKind> = tokens
        .iter()
        .map(|token| &token.kind)
        .filter(|kind| **kind != TokenKind::Eof)
        .collect();
    match kinds.as_slice() {
        [
            TokenKind::Word(keyword),
            TokenKind::Word(_),
            TokenKind::Symbol(Symbol::Equal),
            TokenKind::Integer(value),
            TokenKind::Symbol(Symbol::Semicolon),
        ] if keyword == "let" => value.parse().ok(),
        [
            TokenKind::Word(keyword),
            TokenKind::Word(_),
            TokenKind::Symbol(Symbol::Equal),
            TokenKind::Symbol(Symbol::Minus),
            TokenKind::Integer(value),
            TokenKind::Symbol(Symbol::Semicolon),
        ] if keyword == "let" => format!("-{value}").parse().ok(),
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
) -> Result<CompiledModule, Vec<FileDiagnostic>> {
    task_route(parsed, typed, mode, options, ranges, false)
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
) -> Result<CompiledModule, Vec<FileDiagnostic>> {
    task_route(parsed, typed, mode, options, ranges, true)
}

fn task_route(
    parsed: &SourceProgram,
    typed: &TypedSourceProgram,
    mode: SourceModuleMode,
    options: &CompileOptions,
    ranges: &[FileRange],
    composing: bool,
) -> Result<CompiledModule, Vec<FileDiagnostic>> {
    if mode != SourceModuleMode::Executable {
        return Err(vec![plain_diagnostic(
            "Q500",
            "entry-free scoped task modules require additional source lowering",
        )]);
    }
    let entry = select_entry(typed, options).map_err(|error| vec![error])?;
    let lowered = if composing {
        qcore_frontend::source_tasks::lower_source_composed_tasks(parsed, typed, &entry)
    } else {
        qcore_frontend::source_tasks::lower_source_tasks(parsed, typed, &entry).map(|tasks| {
            qcore_frontend::source_tasks::LoweredComposedTaskModule {
                tasks,
                sites: Vec::new(),
                declares_io: false,
                worker_io: Default::default(),
            }
        })
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
            .any(|(_, kind)| *kind == ScalarType::ReadLoan)
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
    // A typed error payload transfers custody along an error edge, which no
    // earlier profile does, so it names its own profile rather than being
    // reported as an ordinary owned-task run.
    let error_payload_profile = checked
        .module()
        .functions
        .values()
        .any(|function| !function.error_payloads.is_empty());
    let owned_profile = checked.module().functions.values().any(|function| {
        function.result == ScalarType::OwnedBuffer
            || function
                .parameters
                .iter()
                .any(|(_, kind)| *kind == ScalarType::OwnedBuffer)
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
                    // Neither is an ordinary scalar result. `ReadLoan` is
                    // additionally refused as a result by the checker, so this
                    // arm is a total match, not a silent reclassification.
                    ScalarType::OwnedBuffer | ScalarType::ReadLoan => return None,
                };
                Some((function.name.clone(), scalar))
            })
            .collect(),
        owned_buffer_results: checked
            .module()
            .functions
            .values()
            .filter(|function| function.result == ScalarType::OwnedBuffer)
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
        checked: match composed {
            Some(composed) => CheckedSourceModule::ComposedTasks(Box::new(composed)),
            None => CheckedSourceModule::ScopedTasks(Box::new(checked)),
        },
        execution_profile: if composing {
            // The composition names its own profile rather than being reported
            // as the task profile it composes: its guarantees are that route's
            // plus the join's, and a consumer must be able to tell.
            SourceExecutionProfile::ComposedStdIoTasksV1
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
    libraries: &qcore_core::native_library::NativeLibraries,
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
) -> Result<BTreeMap<String, extern_call::ExternCallee>, Vec<FileDiagnostic>> {
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
    let mut diagnostics = Vec::new();
    let mut callees = BTreeMap::new();
    for (name, span) in &called {
        let Some(declaration) = declarations.get(name) else {
            continue;
        };
        if declaration.unchecked {
            if let Some(callee) = admit_unchecked_callee(
                declaration,
                contracts.as_ref(),
                &libraries,
                ranges,
                &mut diagnostics,
            ) {
                callees.insert(name.clone(), callee);
            }
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
        if let Some(binding) = libraries.unchecked_binding(name) {
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
        let residence = match libraries.contracted_binding(name) {
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
    Ok(callees)
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
) -> Result<CompiledModule, Vec<FileDiagnostic>> {
    use qcore_frontend::source_extern;

    if mode != SourceModuleMode::Executable {
        return Err(vec![plain_diagnostic(
            "Q500",
            "entry-free extern-calling modules require additional source lowering",
        )]);
    }
    let entry = select_entry(typed, options).map_err(|error| vec![error])?;
    let callees = admit_extern_callees(parsed, options, ranges)?;

    let lowered =
        source_extern::lower_source_extern(parsed, typed, &entry, &callees).map_err(|errors| {
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
) -> Result<CompiledModule, Vec<FileDiagnostic>> {
    use qcore_frontend::source_composed;

    if mode != SourceModuleMode::Executable {
        return Err(vec![plain_diagnostic(
            "Q043",
            "entry-free composed modules require additional source lowering",
        )]);
    }
    let entry = select_entry(typed, options).map_err(|error| vec![error])?;
    let callees = admit_extern_callees(parsed, options, ranges)?;
    let lowered = source_composed::lower_source_composed(parsed, typed, &entry, &callees).map_err(
        |errors| {
            errors
                .into_iter()
                .map(|error| map_diagnostic(error, ranges))
                .collect::<Vec<_>>()
        },
    )?;
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
) -> Result<CompiledModule, Vec<FileDiagnostic>> {
    if mode != SourceModuleMode::Executable {
        return Err(vec![plain_diagnostic(
            "Q058",
            "entry-free `source.buffer.v1` modules require additional source lowering",
        )]);
    }
    let entry = select_entry(typed, options).map_err(|error| vec![error])?;
    let lowered = qcore_frontend::source_std_buffer::lower_source_buffer(parsed, typed, &entry)
        .map_err(|errors| {
            errors
                .into_iter()
                .map(|error| map_diagnostic(error, ranges))
                .collect::<Vec<_>>()
        })?;
    let certificate = std_buffer::check_buffer_program(&lowered.program).map_err(|errors| {
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
    std_buffer::recheck_buffer_certificate(&lowered.program, &certificate).map_err(|failure| {
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
        checked: CheckedSourceModule::StandardBuffer(Box::new(CheckedBufferModule {
            lowered,
            certificate,
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
) -> Result<CompiledModule, Vec<FileDiagnostic>> {
    if mode != SourceModuleMode::Executable {
        return Err(vec![plain_diagnostic(
            "Q068",
            "entry-free `source.string.v1` modules require additional source lowering",
        )]);
    }
    let entry = select_entry(typed, options).map_err(|error| vec![error])?;
    let lowered = qcore_frontend::source_std_string::lower_source_string(parsed, typed, &entry)
        .map_err(|errors| {
            errors
                .into_iter()
                .map(|error| map_diagnostic(error, ranges))
                .collect::<Vec<_>>()
        })?;
    let certificate = std_string::check_string_program(&lowered.program).map_err(|errors| {
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
) -> Result<CompiledModule, Vec<FileDiagnostic>> {
    if mode != SourceModuleMode::Executable {
        return Err(vec![plain_diagnostic(
            "Q500",
            "entry-free checked mutex modules require additional source lowering",
        )]);
    }
    let entry = select_entry(typed, options).map_err(|error| vec![error])?;
    let lowered = qcore_frontend::source_mutex::lower_source_mutex(parsed, typed, &entry).map_err(
        |errors| {
            errors
                .into_iter()
                .map(|error| map_diagnostic(error, ranges))
                .collect::<Vec<_>>()
        },
    )?;
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
    // A checked trace carrying task structure is *checked* but has no native
    // rendering: this backend emits one thread. The module is published without
    // C, never with C the backend did not produce, and never as a silent
    // success that dropped the task events.
    let c_source = if mutex_c_emit::is_single_task_trace(&lowered.program) {
        let emission =
            mutex_c_emit::emit_mutex_c11(&lowered.program, &certificate).map_err(|refusal| {
                vec![plain_diagnostic(
                    "Q600",
                    format!("the checked mutex backend refused this trace: {refusal:?}"),
                )]
            })?;
        Some(emission.source)
    } else {
        None
    };
    Ok(CompiledModule {
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
        checked: CheckedSourceModule::CheckedEpochBound(Box::new(CheckedEpochModule {
            lowered,
            certificate,
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
    let split = source_epoch_tasks::split_composed_epoch(parsed, &entry).map_err(|errors| {
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
    // The whole scoped-task route, unedited, on the task half alone.
    let task_module = compile_task_source(&task_parsed, &task_typed, mode, options, ranges)?;
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
    if composed.plan.latency_claim.is_some() {
        let Err(refusal) = epoch_accounting::epoch_wall_clock_bound(&certificate) else {
            unreachable!("a wall-clock bound is never available in this profile")
        };
        return Err(vec![map_diagnostic(composed.plan.locate(&refusal), ranges)]);
    }
    Ok(CompiledModule {
        checked: CheckedSourceModule::ComposedEpochTasks(Box::new(ComposedEpochModule {
            tasks: *tasks,
            composed,
            certificate,
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

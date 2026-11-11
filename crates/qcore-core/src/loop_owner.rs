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

//! Derived thread identity and the loop-owner obligation (card
//! **W47-LOOP-OWNER-CONTRACTS**, gateway rule 10p, contract specification
//! §C04.9).
//!
//! # What this kernel decides
//!
//! A collector or loop operation mutates (or reads) state that only the thread
//! waiting on the loop may touch: Monkey's userspace priority-queue links and
//! the collector's `running` flag are not protected by `epoll_ctl`'s own
//! thread safety. The obligation is **discharged only on a path whose derived
//! thread is exactly `owner(L)`**, where `L` is the loop the collector is
//! attached to: the engine loop (`config->evl`) for a non-threaded instance,
//! the instance's worker loop (`thi->evl`) for a threaded one.
//!
//! The kernel is source-independent. An adapter supplies a [`ThreadProgram`]:
//! a closed inventory of thread roots, the call edges it could read with the
//! hop kind that justifies each, and every function it could **not** close
//! (an address taken outside a recognised hop, a macro that names it, a
//! caller outside the read files). [`derive`] computes each function's derived
//! contexts; [`owner_verdict`] decides the obligation at one operation.
//!
//! # Fail-closed
//!
//! * A function the adapter could not close carries an unresolved reason, and
//!   every function it reaches inherits it. A site whose function carries one
//!   is at best [`OwnerVerdict::Unresolved`], never discharged.
//! * A context whose owner is not derived (the mode is not fixed, or the
//!   owner thread of that mode is not a single derived thread) is unresolved.
//! * A site with no derived context at all is unresolved, not vacuously
//!   discharged.
//! * A context that is resolved and whose thread is not the owner refuses the
//!   site outright ([`OwnerVerdict::NotDischarged`]), whatever else is
//!   unresolved: one derived non-owner path is enough to withhold the
//!   obligation. It is still only a refusal (`Unknown` downstream), never a
//!   validated reachable violation.
//!
//! # Coroutines (hop C)
//!
//! A coroutine runs on the thread that resumes it (card
//! W47-LOOP-OWNER-COROUTINE-HOP). The adapter supplies an edge
//! ([`HopKind::Coroutine`]) from each derived resumer to the coroutine's
//! entry, so the entry's contexts are exactly its resumers'. A body the entry
//! runs ([`ThreadProgram::coroutine_bodies`]) that may reach a yield
//! ([`ThreadProgram::yields`]) continues after the yield on a resumer that is
//! not derived: it is unresolved ([`underived_resumer`]), never discharged.
//!
//! # Completion meanings (C2) and paired collectors (C3)
//!
//! [`control_completion`] separates the two meanings of a public
//! pause/resume: a non-threaded input runs the plugin callback before the call
//! returns, a threaded one only enqueues a request. [`claim_discharged`] never
//! lets an enqueue discharge "applied". [`pairing`] states the backend
//! collectors' joint state only at a callback boundary, only when both
//! collector calls are known to have succeeded, and a resume's running state
//! only under an active ingestion premise.
//!
//! # Reduction to the reactor
//!
//! [`reduce_to_reactor`] writes one loop's accesses as an
//! [`crate::event_reactor`] program whose single dispatcher is the derived
//! owner: the owner's accesses run inside its dispatched callback, a
//! non-owner's run from a caller context and request a cross-thread loop
//! operation on a base without threading. The reactor's L05 (conflicting
//! cross-context access) and L04 (cross-thread use without threading) then
//! refuse exactly the non-owner accesses.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::event_reactor::{
    ActivationReason, ArgumentMode, CallbackContext, CallbackSummary, EventDiagnostic, EventFlags,
    EventKey, EventOperation, EventProgram, LoopControl, PayloadAccessKind, ReactorProfile,
    check_event_program,
};

/// Index of a function in [`ThreadProgram::functions`].
pub type FunctionId = usize;

/// The runtime threaded flag of the instance a path is about (hop F).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Mode {
    /// The instance is not threaded: its collectors attach to the engine loop.
    NonThreaded,
    /// The instance is threaded: its collectors attach to its worker loop.
    Threaded,
    /// Not fixed on this path: both values are attributed.
    Either,
}

/// One derived execution context: the thread a function runs on, named by its
/// thread root, and the mode of the instance the path is about.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Context {
    pub thread: String,
    pub mode: Mode,
}

/// The hop kind that justifies an edge (W47-TAIL-PAUSE-BINDINGS §2.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum HopKind {
    /// A direct call by name.
    Direct,
    /// S: a spawn wrapper hands the callee to a new thread.
    Spawn,
    /// V / V*: a field-keyed callback registration dispatched through that
    /// field (a static initializer, or a runtime field store such as the
    /// revised test's copied vtable).
    Vtable,
    /// R: a collector registered through `flb_input_set_collector_*`,
    /// dispatched through `cb_collect`.
    CollectorRegistry,
    /// C: a derived resumer runs a coroutine's entry. A coroutine runs on the
    /// thread that resumes it, so the edge keeps the thread like a call
    /// (card W47-LOOP-OWNER-COROUTINE-HOP).
    Coroutine,
}

/// The mode an edge requires of its caller's context (hop F), or fixes for its
/// callee (hop L).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ModeEffect {
    /// The callee runs in the caller's mode.
    Keep,
    /// The call stands in the branch that the threaded flag selects: a caller
    /// in the other mode cannot take it, and an `Either` caller takes it in
    /// this mode (hop F).
    Require(Mode),
    /// The edge is a loop dispatch (hop L): the callee runs on the
    /// dispatching thread for an instance of this mode, whatever the caller's.
    Dispatch(Mode),
}

/// One call edge the adapter read.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Edge {
    pub caller: FunctionId,
    pub callee: FunctionId,
    pub hop: HopKind,
    pub effect: ModeEffect,
}

/// One thread root of the closed inventory: a test function reached through
/// the test table (hop A), a program entry, or a spawned thread's entry.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ThreadRoot {
    pub function: FunctionId,
    pub context: Context,
}

/// The adapter's reading of one program.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ThreadProgram {
    pub functions: Vec<String>,
    pub roots: Vec<ThreadRoot>,
    pub edges: Vec<Edge>,
    /// A spawn edge's callee is a new thread named after it, in the spawner's
    /// mode. Kept apart from `edges` because it changes the thread.
    pub spawns: Vec<(FunctionId, FunctionId)>,
    /// Functions the adapter could not close, with why.
    pub unresolved: BTreeMap<FunctionId, BTreeSet<String>>,
    /// `(entry, body)`: `body` runs inside coroutine `entry` after its
    /// hand-off, on the entry's resumer (hop C).
    pub coroutine_bodies: Vec<(FunctionId, FunctionId)>,
    /// Functions that yield their coroutine without ending it, with where.
    pub yields: BTreeMap<FunctionId, String>,
}

/// The reason a coroutine body that may yield carries: its continuation after
/// the yield runs on whichever thread resumes the coroutine next, which is
/// not derived.
#[must_use]
pub fn underived_resumer(entry: &str, body: &str, site: &str) -> String {
    format!(
        "`{body}` runs in coroutine `{entry}` and may yield ({site}): its resumer after the \
         yield is not derived"
    )
}

/// Every function that reaches a yield over call edges (not into another
/// coroutine's entry), with the yield it reaches.
fn reaches_a_yield(program: &ThreadProgram) -> Vec<Option<String>> {
    let count = program.functions.len();
    let mut reach = vec![None; count];
    let mut incoming: Vec<Vec<FunctionId>> = vec![Vec::new(); count];
    for edge in &program.edges {
        if edge.hop != HopKind::Coroutine && edge.caller < count && edge.callee < count {
            incoming[edge.callee].push(edge.caller);
        }
    }
    let mut pending = VecDeque::new();
    for (function, site) in &program.yields {
        if *function < count && reach[*function].is_none() {
            reach[*function] = Some(site.clone());
            pending.push_back(*function);
        }
    }
    while let Some(function) = pending.pop_front() {
        let site = reach[function].clone();
        for caller in &incoming[function] {
            if reach[*caller].is_none() {
                reach[*caller].clone_from(&site);
                pending.push_back(*caller);
            }
        }
    }
    reach
}

/// What [`derive`] computed for one function.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Derived {
    pub contexts: BTreeSet<Context>,
    /// Reasons some path to this function could not be closed.
    pub unresolved: BTreeSet<String>,
    /// A witness path (function ids, root first) for each context.
    pub witness: BTreeMap<Context, Vec<FunctionId>>,
}

fn apply(effect: ModeEffect, mode: Mode) -> Option<Mode> {
    match effect {
        ModeEffect::Keep => Some(mode),
        ModeEffect::Dispatch(fixed) => Some(fixed),
        ModeEffect::Require(required) => match mode {
            Mode::Either => Some(required),
            current if current == required => Some(current),
            _ => None,
        },
    }
}

/// Derive every function's contexts to their fixed point.
///
/// Contexts flow from each root along edges (the thread is kept, the mode is
/// transformed by the edge's effect, an infeasible `Require` prunes) and along
/// spawns (a new thread named after the entry, in the spawner's mode).
/// Unresolved reasons flow along call edges only, independently of contexts:
/// a spawned thread's identity is its entry, whoever spawns it, and an entry
/// whose spawner is unresolved is rooted in every mode instead.
#[must_use]
pub fn derive(program: &ThreadProgram) -> Vec<Derived> {
    let count = program.functions.len();
    let mut derived = vec![Derived::default(); count];
    let mut outgoing: Vec<Vec<&Edge>> = vec![Vec::new(); count];
    for edge in &program.edges {
        if edge.caller < count && edge.callee < count {
            outgoing[edge.caller].push(edge);
        }
    }
    let mut spawned: Vec<Vec<FunctionId>> = vec![Vec::new(); count];
    for (spawner, entry) in &program.spawns {
        if *spawner < count && *entry < count {
            spawned[*spawner].push(*entry);
        }
    }
    // Unresolved reasons first: seed, then propagate along every call edge to
    // a fixed point (monotone over a finite set of reasons). A spawn does not
    // carry them: the spawned thread is named by its own entry whoever spawns
    // it. An unresolved spawner instead roots its entry in every mode, since
    // its own contexts (and so the mode it spawns in) may be incomplete.
    let mut pending: VecDeque<FunctionId> = VecDeque::new();
    for (function, reasons) in &program.unresolved {
        if *function < count {
            derived[*function]
                .unresolved
                .extend(reasons.iter().cloned());
            pending.push_back(*function);
        }
    }
    // Hop C: a coroutine body that may yield continues on an underived
    // resumer, so it is unresolved whatever else reaches it.
    if !program.coroutine_bodies.is_empty() {
        let reach = reaches_a_yield(program);
        for (entry, body) in &program.coroutine_bodies {
            if *entry < count
                && *body < count
                && let Some(site) = &reach[*body]
            {
                derived[*body].unresolved.insert(underived_resumer(
                    &program.functions[*entry],
                    &program.functions[*body],
                    site,
                ));
                pending.push_back(*body);
            }
        }
    }
    while let Some(function) = pending.pop_front() {
        let reasons = derived[function].unresolved.clone();
        for edge in &outgoing[function] {
            let target = edge.callee;
            let before = derived[target].unresolved.len();
            derived[target].unresolved.extend(reasons.iter().cloned());
            if derived[target].unresolved.len() != before {
                pending.push_back(target);
            }
        }
    }
    let mut queue: VecDeque<(FunctionId, Context, Vec<FunctionId>)> = VecDeque::new();
    for root in &program.roots {
        if root.function < count {
            queue.push_back((root.function, root.context.clone(), vec![root.function]));
        }
    }
    for (spawner, entry) in &program.spawns {
        if *spawner < count && *entry < count && !derived[*spawner].unresolved.is_empty() {
            queue.push_back((
                *entry,
                Context {
                    thread: program.functions[*entry].clone(),
                    mode: Mode::Either,
                },
                vec![*spawner, *entry],
            ));
        }
    }
    while let Some((function, context, path)) = queue.pop_front() {
        if !derived[function].contexts.insert(context.clone()) {
            continue;
        }
        derived[function]
            .witness
            .insert(context.clone(), path.clone());
        for edge in &outgoing[function] {
            if let Some(mode) = apply(edge.effect, context.mode) {
                let mut next = path.clone();
                next.push(edge.callee);
                queue.push_back((
                    edge.callee,
                    Context {
                        thread: context.thread.clone(),
                        mode,
                    },
                    next,
                ));
            }
        }
        for entry in &spawned[function] {
            let mut next = path.clone();
            next.push(*entry);
            queue.push_back((
                *entry,
                Context {
                    thread: program.functions[*entry].clone(),
                    mode: context.mode,
                },
                next,
            ));
        }
    }
    derived
}

/// The derived owner thread of each loop kind: the single thread that
/// dispatches collectors on the engine loop, and on a worker loop.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Owners {
    pub non_threaded: Option<String>,
    pub threaded: Option<String>,
}

impl Owners {
    /// Derive the owners from the dispatch functions' own derivations: a loop
    /// has an owner only when its dispatcher runs on exactly one thread and no
    /// path to it is unresolved.
    #[must_use]
    pub fn from_dispatchers(engine: Option<&Derived>, worker: Option<&Derived>) -> Self {
        let single = |derived: Option<&Derived>| {
            let derived = derived?;
            if !derived.unresolved.is_empty() {
                return None;
            }
            let threads = derived
                .contexts
                .iter()
                .map(|context| context.thread.clone())
                .collect::<BTreeSet<_>>();
            (threads.len() == 1)
                .then(|| threads.into_iter().next())
                .flatten()
        };
        Self {
            non_threaded: single(engine),
            threaded: single(worker),
        }
    }

    fn of(&self, mode: Mode) -> Result<Vec<&str>, ()> {
        fn get(owner: &Option<String>) -> Result<&str, ()> {
            owner.as_deref().ok_or(())
        }
        match mode {
            Mode::NonThreaded => Ok(vec![get(&self.non_threaded)?]),
            Mode::Threaded => Ok(vec![get(&self.threaded)?]),
            Mode::Either => Ok(vec![get(&self.non_threaded)?, get(&self.threaded)?]),
        }
    }
}

/// The owner obligation's answer at one operation site.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OwnerVerdict {
    /// Every derived context runs on the owner of its mode's loop, and no
    /// path is unresolved.
    Discharged { contexts: BTreeSet<Context> },
    /// A resolved context runs on a thread that is not its loop's owner.
    NotDischarged {
        context: Context,
        witness: Vec<FunctionId>,
    },
    /// Nothing refuses, but the obligation cannot be discharged.
    Unresolved { reasons: BTreeSet<String> },
}

/// Decide the owner obligation for an operation in a function with
/// derivation `derived`.
#[must_use]
pub fn owner_verdict(derived: &Derived, owners: &Owners) -> OwnerVerdict {
    let mut reasons = derived.unresolved.clone();
    for context in &derived.contexts {
        match owners.of(context.mode) {
            Ok(candidates) => {
                let resolved_single = candidates.len() == 1;
                if !candidates.contains(&context.thread.as_str()) {
                    return OwnerVerdict::NotDischarged {
                        context: context.clone(),
                        witness: derived.witness.get(context).cloned().unwrap_or_default(),
                    };
                }
                if !resolved_single {
                    reasons.insert(format!(
                        "the threaded flag is not fixed on a path on thread `{}`",
                        context.thread
                    ));
                }
            }
            Err(()) => {
                reasons.insert(format!(
                    "the owner of the {} loop is not a single derived thread",
                    match context.mode {
                        Mode::NonThreaded => "engine",
                        Mode::Threaded => "worker",
                        Mode::Either => "engine or worker",
                    }
                ));
            }
        }
    }
    if derived.contexts.is_empty() {
        reasons.insert("no derived caller reaches this operation".to_owned());
    }
    if reasons.is_empty() {
        OwnerVerdict::Discharged {
            contexts: derived.contexts.clone(),
        }
    } else {
        OwnerVerdict::Unresolved { reasons }
    }
}

/// What a successful public pause/resume establishes (C2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlCompletion {
    /// Non-threaded: the plugin callback ran to completion before return.
    CallbackFinished,
    /// Threaded: a request was written to the worker's channel; nothing is
    /// applied yet.
    RequestEnqueued,
}

/// A claim a caller may want to rest on after a successful pause/resume.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlClaim {
    /// A request is available to the owner.
    RequestAvailable,
    /// The plugin callback has finished.
    CallbackFinished,
    /// The collectors' state has changed.
    OperationApplied,
}

/// C2's completion meaning in a mode; `None` when the mode is not fixed.
#[must_use]
pub fn control_completion(mode: Mode) -> Option<ControlCompletion> {
    match mode {
        Mode::NonThreaded => Some(ControlCompletion::CallbackFinished),
        Mode::Threaded => Some(ControlCompletion::RequestEnqueued),
        Mode::Either => None,
    }
}

/// Whether `completion` discharges `claim`. An enqueue discharges only
/// "request available"; a finished callback only "callback finished"; no
/// return code alone discharges "applied" (that needs C3's per-collector
/// results).
#[must_use]
pub fn claim_discharged(claim: ControlClaim, completion: Option<ControlCompletion>) -> bool {
    matches!(
        (claim, completion),
        (
            ControlClaim::RequestAvailable,
            Some(ControlCompletion::RequestEnqueued)
        ) | (
            ControlClaim::CallbackFinished,
            Some(ControlCompletion::CallbackFinished)
        )
    )
}

/// One backend collector call's result as the caller can know it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CollectorResult {
    Succeeded,
    Failed,
    /// The caller discards the result (`tail_fs_inotify.c` returns `void`).
    Ignored,
}

/// Which way the backend collectors move.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PairDirection {
    Pause,
    Resume,
}

/// The ingestion premise a resume's running state needs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ingestion {
    Active,
    Inactive,
    Unknown,
}

/// C3's answer about the two backend collectors.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PairingClaim {
    /// Both collectors are in the direction's state at the callback boundary.
    BothAtBoundary(PairDirection),
    /// Nothing can be concluded; the reason is recorded.
    NoConclusion(&'static str),
    /// A claim of the direction's state is refused.
    Refused(&'static str),
}

/// C3: the paired backend collectors' state after one pause or resume.
#[must_use]
pub fn pairing(
    direction: PairDirection,
    results: [CollectorResult; 2],
    ingestion: Ingestion,
    owner: &OwnerVerdict,
) -> PairingClaim {
    match owner {
        OwnerVerdict::NotDischarged { .. } => {
            return PairingClaim::Refused("the collector operations run off the loop owner");
        }
        OwnerVerdict::Unresolved { .. } => {
            return PairingClaim::NoConclusion("the owner obligation is not discharged");
        }
        OwnerVerdict::Discharged { .. } => {}
    }
    if direction == PairDirection::Resume {
        match ingestion {
            Ingestion::Inactive => {
                return PairingClaim::Refused(
                    "a resume while ingestion is inactive promises no running state",
                );
            }
            Ingestion::Unknown => {
                return PairingClaim::NoConclusion("the ingestion premise is not established");
            }
            Ingestion::Active => {}
        }
    }
    if results
        .iter()
        .any(|result| *result != CollectorResult::Succeeded)
    {
        return PairingClaim::NoConclusion(
            "a collector result is failed or discarded: the transition may be partial",
        );
    }
    PairingClaim::BothAtBoundary(direction)
}

/// The kind of one loop-owned operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LoopOperation {
    /// Reads loop-owned state (`running`).
    Read,
    /// Mutates the loop's links (pause, resume, delete, register).
    Mutate,
}

/// The reactor profile of a Monkey loop: no cross-thread use is permitted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OwnedLoop;

impl ReactorProfile for OwnedLoop {
    fn threading_enabled(self) -> bool {
        false
    }
}

/// The loop's link structure as one reactor payload.
pub const LOOP_LINKS: u32 = 1;

/// Write one loop's accesses as a reactor program whose single dispatcher is
/// `owner`. The dispatcher's own walk of the links is the first access of its
/// dispatched callback; `accesses` then run in order, the owner's inside that
/// callback and every other thread's from a caller context with a
/// cross-thread loop request.
#[must_use]
pub fn reduce_to_reactor(
    owner: &str,
    accesses: &[(String, LoopOperation)],
) -> EventProgram<OwnedLoop> {
    let mut threads: BTreeMap<&str, u32> = BTreeMap::new();
    threads.insert(owner, 1);
    for (thread, _) in accesses {
        let next = u32::try_from(threads.len()).unwrap_or(u32::MAX) + 1;
        threads.entry(thread.as_str()).or_insert(next);
    }
    let base = 1;
    let event = EventKey::new(1, 1);
    let kind = |operation: LoopOperation| match operation {
        LoopOperation::Read => PayloadAccessKind::Read,
        LoopOperation::Mutate => PayloadAccessKind::Write,
    };
    let mut operations = vec![
        EventOperation::CreateBase {
            base,
            profile: OwnedLoop,
        },
        EventOperation::Assign {
            event,
            base,
            descriptor: None,
            flags: EventFlags::new(EventFlags::TIMEOUT),
            priority: 0,
            summary: CallbackSummary {
                argument: 1,
                argument_mode: ArgumentMode::Borrowed,
                trigger_reasons: BTreeSet::from([ActivationReason::Timeout]),
                payload_authority: BTreeMap::from([(LOOP_LINKS, PayloadAccessKind::Write)]),
                may_defer: false,
                may_spawn_tasks: false,
            },
        },
        EventOperation::Add { event },
        EventOperation::BeginDispatch {
            base,
            dispatcher: threads[owner],
        },
        EventOperation::Activate {
            event,
            reason: ActivationReason::Timeout,
            cross_thread: false,
        },
        EventOperation::CallbackEnter {
            event,
            reason: ActivationReason::Timeout,
        },
        EventOperation::Access {
            context: CallbackContext::Event(event),
            payload: LOOP_LINKS,
            kind: PayloadAccessKind::Write,
            synchronized: false,
        },
    ];
    let mut foreign = Vec::new();
    for (thread, operation) in accesses {
        if thread == owner {
            operations.push(EventOperation::Access {
                context: CallbackContext::Event(event),
                payload: LOOP_LINKS,
                kind: kind(*operation),
                synchronized: false,
            });
        } else {
            foreign.push((threads[thread.as_str()], *operation));
        }
    }
    // A foreign access overlaps the dispatcher's callback: it is issued while
    // the owner is inside it, which is what "concurrent" means here.
    for (thread, operation) in foreign {
        operations.push(EventOperation::Access {
            context: CallbackContext::Caller(thread),
            payload: LOOP_LINKS,
            kind: kind(operation),
            synchronized: false,
        });
        if operation == LoopOperation::Mutate {
            operations.push(EventOperation::LoopRequest {
                base,
                request: LoopControl::Continue,
                cross_thread: true,
            });
        }
    }
    operations.push(EventOperation::CallbackExit { event });
    operations.push(EventOperation::EndDispatch { base });
    operations.push(EventOperation::FreeEvent { event });
    operations.push(EventOperation::DropArgument { argument: 1 });
    operations.push(EventOperation::FreeBase { base });
    EventProgram {
        live_arguments: BTreeSet::from([1]),
        operations,
    }
}

/// Check one loop's accesses through the reactor; the diagnostics are the
/// reactor's own.
///
/// # Errors
///
/// The reactor's diagnostics when any access is refused.
pub fn check_through_reactor(
    owner: &str,
    accesses: &[(String, LoopOperation)],
) -> Result<(), Vec<EventDiagnostic>> {
    check_event_program(&reduce_to_reactor(owner, accesses), &[OwnedLoop]).map(|_| ())
}

#[cfg(test)]
#[path = "../tests/unit/loop_owner.rs"]
mod tests;

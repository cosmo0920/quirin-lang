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

//! Bounded aggregate-lifecycle composition (card O05).
//!
//! One **aggregate parent** (a container whose children are heap allocations and
//! OS endpoints), one **wrapper lifecycle** (a separate object that holds one
//! aggregate member and owns allocations and publications of its own), and the
//! **admitted callbacks**, scoped tasks and guards that must be quiescent before
//! either is torn down.
//!
//! This module decides one question and refuses everything else:
//!
//! > Under the supplied per-member dispositions, may the composition of these
//! > lifecycles be published as a single whole-composition claim?
//!
//! The answer is [`AggregateVerdict::Composed`] or [`AggregateVerdict::Unknown`].
//! **There is no violation verdict.** The relation consumes records another pass
//! supplied — O04's per-exit residual ledger, a target-qualified OS endpoint
//! post-state, a declared destructor contract, the reactor callback inventory,
//! the scoped-task ledger and the mutex-guard ledger — so a contradictory record
//! is a reason to say nothing, never a reason to accuse. That is the same
//! fail-closed direction `QCore.ExitResiduals` pins for a single exit and
//! `QCore.VerifierPlugin.Composition` pins for certificate merging.
//!
//! Three properties this relation is built to decide, one per acceptance item:
//!
//! 1. **Identity separation across parent release.** No two members share a
//!    logical identity `(kind, generation)`, and two members that share a *raw*
//!    OS-visible address (a descriptor number, a mapping address) must have
//!    disjoint live intervals. A member whose disposition is `Retained` stays
//!    live past the end of the schedule, so nothing may reuse its address.
//! 2. **Quiescence before teardown.** At the teardown point — the first release
//!    step performed by the parent — every callback with declared authority over
//!    a member payload must be quiescent, every task that captured a member must
//!    be joined, and every guard over a footprint touching a member must be
//!    released. A quiescent callback that spawned a task nobody accounts for is
//!    still a refusal.
//! 3. **Separated durability, and no partial composition.** Synchronization and
//!    durability are carried as [`DurabilityPremise`]s on the certificate and are
//!    *never* discharged: offering one as release evidence is a refusal. Any
//!    declared [`UnsupportedClass`] — capacity, metadata, schedule, callback,
//!    backend or OS custody — refuses the whole-composition claim outright.
//!
//! ## What this is not
//!
//! This is a model of the **composition decisions**, not of C semantics, of any
//! kernel, or of any particular codebase. Every input record is a supplied
//! premise: nothing here proves that a reader extracted a disposition faithfully,
//! that a declared callback authority is complete, that a suppressed-syscall
//! observation generalizes to an arbitrary failure, or that a declared contract
//! holds. A [`AggregateVerdict::Composed`] result is exactly the conjunction of
//! the checks below under [`AggregateCertificate::declared_premises`] and
//! [`AggregateCertificate::durability_premises`], and promotes nothing.
//!
//! The relation is not wired into any production audit route. Doing so is a
//! named follow-on (O05-O1).

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

/// A member of the aggregate or of the wrapper.
pub type MemberId = u32;
/// Lifetime generation of a member identity. Distinct generations are distinct
/// identities even when they occupy the same raw address.
pub type Generation = u32;
/// A raw OS-visible number: a descriptor value, a mapping address.
pub type RawAddress = u64;
pub type CallbackId = u32;
pub type TaskId = u32;
pub type GuardId = u32;
pub type FootprintId = u32;
pub type PayloadId = u32;
pub type PublicationId = u32;
/// A custodian outside the aggregate that a retained endpoint is handed to.
pub type CustodianId = u32;

/// The kind of OS endpoint a member is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum EndpointKind {
    Descriptor,
    Mapping,
    DirectoryEntry,
}

/// What a member is. Exactly one member of a program is the [`MemberKind::Aggregate`] root.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum MemberKind {
    /// The aggregate parent itself.
    Aggregate,
    /// A heap allocation.
    Allocation,
    /// An OS endpoint.
    Endpoint(EndpointKind),
}

/// Which lifecycle a member belongs to. The two sets must stay structurally
/// separate: a wrapper allocation is never a child of the aggregate and an
/// aggregate child is never a child of the wrapper.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Owner {
    Aggregate,
    Wrapper,
}

/// One tracked identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AggregateMember {
    pub id: MemberId,
    pub kind: MemberKind,
    /// Together with `kind`, the logical identity. Two members may not share one.
    pub generation: Generation,
    pub owner: Owner,
    /// Containment. The root has `None`.
    pub parent: Option<MemberId>,
    /// The raw OS-visible number, where the member has one. Reuse is permitted
    /// only across disjoint live intervals.
    pub address: Option<RawAddress>,
    /// Payloads a callback could hold declared authority over.
    pub payloads: BTreeSet<PayloadId>,
}

/// A fact about a data structure that looks like progress and is not custody
/// evidence. Cycle 19 measured each of these directly: down-ring membership and
/// a zero up-counter survive a failed close with the descriptor still open, and
/// O04 measured that a success-looking integer discharges nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum StatusFactKind {
    /// The member was moved between state rings.
    RingMembership,
    /// A liveness counter reached zero.
    ZeroUpCounter,
    /// A callee returned a value the caller reads as success.
    SuccessInteger,
    /// The field that held the member was set to null.
    NullFieldAfterRelease,
}

/// A synchronization or durability property. These are **premises**: this
/// relation never discharges one, and offering one as release evidence refuses.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DurabilityPremise {
    SyncSucceeded,
    DataReachedStableStorage,
    MetadataReachedStableStorage,
    MappingFlushed,
    RenameIsAtomic,
}

/// A premise the composition assumes and does not derive.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DeclaredPremise {
    /// The constructor hands the caller the custody its contract declares.
    ConstructorContract,
    /// The destructor releases what its contract declares.
    DestructorContract,
    /// The selected backend profile is the one that was built.
    BackendProfile,
    /// Each callback's declared payload authority is complete.
    CallbackAuthorityDeclared,
    /// Each guard's declared footprint is complete.
    GuardFootprintDeclared,
    /// The scoped-task ledger reports every task and its terminal state.
    TaskLedgerSupplied,
    /// Per-exit residuals were supplied by a validated projection.
    ExitResidualsSupplied,
    /// OS endpoint post-states were supplied by a target-qualified relation.
    EndpointPostStateSupplied,
    /// Recorded observations are trusted for the runs they record.
    ObservationsTrusted,
}

/// Why a member's disposition could not be resolved.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum UnresolvedReason {
    /// The observation suppressed the syscall rather than executing it.
    SuppressedSyscall,
    /// A callee on the release path has no contract.
    UncontractedCallee,
    /// The release is guarded by a condition outside every typed rule.
    UnmodeledBranch,
    /// The backend that owns the member is not in the accepted profile list.
    UnknownBackend,
    /// The member is reached only through an aggregate store the projection
    /// does not follow.
    UnmodeledAggregateStore,
}

/// What became of a member at its release step.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Disposition {
    /// Released here, exactly once.
    Released,
    /// Still live afterwards, handed to a named custodian outside the aggregate.
    /// `None` is an unattributed residual and refuses.
    Retained { custodian: Option<CustodianId> },
    /// Neither established.
    Unresolved(UnresolvedReason),
}

/// Where a release step's disposition came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CustodyEvidence {
    /// A per-exit residual ledger reports every path reaching this exit modeled
    /// and this identity not live (O04).
    ExitResidualDischarged,
    /// A target-qualified OS post-state relation supplied the endpoint's state.
    ObservedEndpointPostState,
    /// A declared destructor contract was in force at the release site.
    DeclaredDestructorContract,
    /// Refused. A status fact is never custody evidence.
    StatusFact(StatusFactKind),
    /// Refused. A durability premise is never a release.
    DurabilityDischarged(DurabilityPremise),
}

/// Which lifecycle performed a release step.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Releaser {
    Parent,
    Wrapper,
}

/// One release step.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ReleaseStep {
    pub member: MemberId,
    pub by: Releaser,
    pub disposition: Disposition,
    pub evidence: CustodyEvidence,
}

/// The ordered lifecycle. Acquisition and release order decide identity
/// separation; the first parent release is the teardown point.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LifecycleStep {
    /// The member becomes live.
    Acquire { member: MemberId },
    /// The wrapper publishes a reference to itself somewhere the aggregate's
    /// teardown cannot see (a list, a callback closure, a hash table).
    Publish { publication: PublicationId },
    /// The wrapper withdraws that publication.
    Reclaim { publication: PublicationId },
    /// The wrapper gives up a borrowed hold on an aggregate member.
    DropBorrow,
    /// A release step.
    Release(ReleaseStep),
}

/// Declared state of an admitted callback at the teardown point.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CallbackState {
    /// Still registered with a reactor and reachable.
    Registered,
    /// Executing.
    Running,
    /// Deregistered and not executing.
    Quiescent,
}

/// One callback in the declared assignment inventory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CallbackRecord {
    pub id: CallbackId,
    pub state: CallbackState,
    /// The callback's complete declared payload authority.
    pub authority: BTreeSet<PayloadId>,
    /// False when the callback is outside the admitted family. An unadmitted
    /// callback contributes [`UnsupportedClass::Callback`].
    pub admitted: bool,
    /// Tasks this callback started.
    pub spawned_tasks: BTreeSet<TaskId>,
}

/// Declared terminal state of a scoped task.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskState {
    Outstanding,
    /// Finished, but its creator has not joined it: its captures are still in a
    /// completion capsule.
    Completed,
    Joined,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskRecord {
    pub id: TaskId,
    pub state: TaskState,
    pub captures: BTreeSet<MemberId>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum GuardState {
    Held,
    Released,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct GuardRecord {
    pub id: GuardId,
    pub state: GuardState,
    pub footprint: FootprintId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FootprintRecord {
    pub id: FootprintId,
    pub members: BTreeSet<MemberId>,
}

/// How the wrapper holds its aggregate member.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum HoldMode {
    /// The wrapper owns it and must release it; the parent must not.
    Owned,
    /// The wrapper borrows it and must drop the borrow strictly before the
    /// parent releases it.
    Borrowed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WrapperLifecycle {
    /// The aggregate member the wrapper holds.
    pub held_member: MemberId,
    pub hold: HoldMode,
    /// Publications that must be reclaimed strictly before the teardown point.
    pub publications: BTreeSet<PublicationId>,
}

/// A class of behaviour no supplied record covers. Any one of these present
/// refuses the whole-composition claim.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum UnsupportedClass {
    /// Capacity limits, resource-limit arms, partial writes.
    Capacity,
    /// Metadata, checksums, permissions, ownership.
    Metadata,
    /// Dispatch schedules, ordering assumptions, priorities.
    Schedule,
    /// A callback outside the admitted family.
    Callback,
    /// A backend outside the accepted profile list.
    Backend,
    /// OS custody the supplied post-state relation does not decide.
    OsCustody,
}

/// The composition input.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AggregateCompositionProgram {
    pub root: MemberId,
    pub members: Vec<AggregateMember>,
    pub schedule: Vec<LifecycleStep>,
    pub callbacks: Vec<CallbackRecord>,
    pub tasks: Vec<TaskRecord>,
    pub guards: Vec<GuardRecord>,
    pub footprints: Vec<FootprintRecord>,
    pub wrapper: Option<WrapperLifecycle>,
    /// Custodians outside the aggregate that may hold a retained endpoint.
    pub external_custodians: BTreeSet<CustodianId>,
    /// Classes the supplied records do not cover.
    pub unsupported: BTreeSet<UnsupportedClass>,
    pub declared_premises: BTreeSet<DeclaredPremise>,
    pub durability_premises: BTreeSet<DurabilityPremise>,
}

/// Structural defects that make the rest of the program unreadable. When any is
/// present the relation reports only these.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum MalformedKind {
    MissingRoot,
    RootIsNotAnAggregate,
    RootHasAParent,
    DuplicateMemberId(MemberId),
    UnknownMember(MemberId),
    ParentCycle(MemberId),
    UnknownPublication(PublicationId),
    UnknownFootprint(FootprintId),
    DuplicateCallbackId(CallbackId),
    DuplicateTaskId(TaskId),
    DuplicateGuardId(GuardId),
    DuplicateFootprintId(FootprintId),
    /// A wrapper step in a program that declares no wrapper.
    WrapperStepWithoutWrapper,
    PublicationReclaimedTwice(PublicationId),
    PublicationReclaimedBeforePublish(PublicationId),
    PublicationPublishedTwice(PublicationId),
    BorrowDroppedTwice,
}

/// Every way the composition claim can be refused. Each is a typed reason; none
/// alleges a violation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CompositionRefusal {
    Malformed(MalformedKind),
    /// A program with no members is not a vacuous success.
    EmptyAggregate,
    /// No step releases the aggregate root.
    ParentNotReleased,
    DuplicateMemberIdentity {
        first: MemberId,
        second: MemberId,
    },
    MemberNeverAcquired {
        member: MemberId,
    },
    MemberAcquiredTwice {
        member: MemberId,
    },
    MemberNotReleased {
        member: MemberId,
    },
    MemberReleasedTwice {
        member: MemberId,
    },
    ReleaseBeforeAcquire {
        member: MemberId,
    },
    /// Two members share a raw address while both are live.
    AddressReusedWhileLive {
        earlier: MemberId,
        later: MemberId,
        address: RawAddress,
    },
    UnresolvedDisposition {
        member: MemberId,
        reason: UnresolvedReason,
    },
    /// A retained residual nobody is named for.
    RetainedEndpointWithoutCustodian {
        member: MemberId,
    },
    /// A retained residual handed to a custodian the program does not declare
    /// as external.
    RetainedEndpointCustodianNotExternal {
        member: MemberId,
        custodian: CustodianId,
    },
    /// Only an OS endpoint may survive parent release. A retained heap
    /// allocation is a leak, not a residual.
    RetainedAllocation {
        member: MemberId,
    },
    StatusFactIsNotCustodyEvidence {
        member: MemberId,
        fact: StatusFactKind,
    },
    DurabilityAssumedAsDischarged {
        member: MemberId,
        premise: DurabilityPremise,
    },
    CallbackNotQuiescent {
        callback: CallbackId,
        state: CallbackState,
    },
    CallbackSpawnedTaskUnaccounted {
        callback: CallbackId,
        task: TaskId,
    },
    TaskNotJoined {
        task: TaskId,
        state: TaskState,
    },
    GuardOutstanding {
        guard: GuardId,
        footprint: FootprintId,
    },
    UnsupportedClassPresent {
        class: UnsupportedClass,
    },
    /// The two lifecycles' containment trees overlap.
    WrapperOverlapsAggregate {
        member: MemberId,
    },
    /// The wrapper releases something it neither owns nor holds.
    WrapperReleasesForeignMember {
        member: MemberId,
    },
    OwnedHoldAlsoReleasedByParent {
        member: MemberId,
    },
    BorrowedHoldReleasedByWrapper {
        member: MemberId,
    },
    /// The borrow is dropped at or after the parent releases the held member,
    /// or never.
    BorrowedHoldOutlivesParentRelease {
        member: MemberId,
    },
    WrapperPublicationUnreclaimed {
        publication: PublicationId,
    },
    /// Reclaimed, but not before the parent began releasing.
    WrapperPublicationSurvivesTeardown {
        publication: PublicationId,
    },
}

impl fmt::Display for CompositionRefusal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

/// The published claim. There is deliberately no violation verdict.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum AggregateVerdict {
    Composed,
    Unknown,
}

/// What a composed claim rests on and what it covers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AggregateCertificate {
    pub root: MemberId,
    /// Members released exactly once by either lifecycle.
    pub released: BTreeSet<MemberId>,
    /// Endpoints that survive parent release, each with its named external
    /// custodian. Reclamation by that custodian is outside this claim.
    pub retained_endpoints: BTreeMap<MemberId, CustodianId>,
    pub quiescent_callbacks: BTreeSet<CallbackId>,
    pub joined_tasks: BTreeSet<TaskId>,
    pub released_guards: BTreeSet<GuardId>,
    /// Count of distinct `(kind, generation)` identities the claim separates.
    pub distinct_identities: usize,
    /// Index of the first parent release step.
    pub teardown_point: usize,
    /// Premises assumed and not derived.
    pub declared_premises: BTreeSet<DeclaredPremise>,
    /// Stated separately, and never discharged by this relation in either
    /// direction.
    pub durability_premises: BTreeSet<DurabilityPremise>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AggregateCompositionResult {
    pub verdict: AggregateVerdict,
    /// Sorted and deduplicated. Empty exactly when the verdict is `Composed`.
    pub refusals: Vec<CompositionRefusal>,
    pub certificate: Option<AggregateCertificate>,
}

impl AggregateCompositionResult {
    pub fn composed(&self) -> bool {
        self.verdict == AggregateVerdict::Composed
    }

    pub fn refuses(&self, refusal: &CompositionRefusal) -> bool {
        self.refusals.contains(refusal)
    }
}

/// Per-member schedule facts derived once.
struct MemberTrace {
    acquired: Option<usize>,
    acquired_twice: bool,
    release: Option<(usize, ReleaseStep)>,
    released_twice: bool,
}

/// Decide whether the supplied aggregate, wrapper, callbacks, tasks and guards
/// compose into one whole-composition claim.
///
/// Fail-closed: any refusal yields [`AggregateVerdict::Unknown`] with no
/// certificate. A refusal never alleges a violation.
pub fn check_aggregate_composition(
    program: &AggregateCompositionProgram,
) -> AggregateCompositionResult {
    let mut refusals: Vec<CompositionRefusal> = Vec::new();

    if program.members.is_empty() {
        return unknown(vec![CompositionRefusal::EmptyAggregate]);
    }

    // --- structural well-formedness -------------------------------------
    let malformed = check_wellformed(program);
    if !malformed.is_empty() {
        return unknown(
            malformed
                .into_iter()
                .map(CompositionRefusal::Malformed)
                .collect(),
        );
    }

    let members: BTreeMap<MemberId, &AggregateMember> =
        program.members.iter().map(|m| (m.id, m)).collect();

    // --- acceptance 3: any unsupported class refuses the whole claim ------
    let mut unsupported = program.unsupported.clone();
    for callback in &program.callbacks {
        if !callback.admitted {
            unsupported.insert(UnsupportedClass::Callback);
        }
    }
    for class in &unsupported {
        refusals.push(CompositionRefusal::UnsupportedClassPresent { class: *class });
    }

    // --- acceptance 1: logical identity separation ------------------------
    let mut seen_identity: BTreeMap<(MemberKind, Generation), MemberId> = BTreeMap::new();
    for member in &program.members {
        if let Some(first) = seen_identity.insert((member.kind, member.generation), member.id) {
            refusals.push(CompositionRefusal::DuplicateMemberIdentity {
                first: first.min(member.id),
                second: first.max(member.id),
            });
        }
    }

    // --- the schedule -----------------------------------------------------
    let traces = trace_members(program);
    let teardown_point = program.schedule.iter().position(|step| {
        matches!(
            step,
            LifecycleStep::Release(ReleaseStep {
                by: Releaser::Parent,
                ..
            })
        )
    });

    for member in &program.members {
        let trace = &traces[&member.id];
        if trace.acquired.is_none() {
            refusals.push(CompositionRefusal::MemberNeverAcquired { member: member.id });
        }
        if trace.acquired_twice {
            refusals.push(CompositionRefusal::MemberAcquiredTwice { member: member.id });
        }
        if trace.released_twice {
            refusals.push(CompositionRefusal::MemberReleasedTwice { member: member.id });
        }
        match &trace.release {
            None => refusals.push(CompositionRefusal::MemberNotReleased { member: member.id }),
            Some((index, step)) => {
                if trace.acquired.is_some_and(|acquired| acquired > *index) {
                    refusals.push(CompositionRefusal::ReleaseBeforeAcquire { member: member.id });
                }
                refusals.extend(check_release_step(program, member, step));
            }
        }
    }

    // root release
    if !program.schedule.iter().any(|step| {
        matches!(
            step,
            LifecycleStep::Release(ReleaseStep {
                member,
                by: Releaser::Parent,
                ..
            }) if *member == program.root
        )
    }) {
        refusals.push(CompositionRefusal::ParentNotReleased);
    }

    // --- acceptance 1: raw address separation across release --------------
    refusals.extend(check_address_separation(program, &traces));

    // --- acceptance 2: quiescence prerequisites at the teardown point ------
    refusals.extend(check_quiescence(program, &members));

    // --- the wrapper lifecycle -------------------------------------------
    refusals.extend(check_wrapper(program, &members, &traces, teardown_point));

    refusals.sort();
    refusals.dedup();
    if !refusals.is_empty() {
        return unknown(refusals);
    }

    let teardown_point = match teardown_point {
        Some(index) => index,
        // Unreachable: `ParentNotReleased` is pushed when no parent release
        // exists, and a nonempty refusal list has already returned.
        None => return unknown(vec![CompositionRefusal::ParentNotReleased]),
    };

    let mut released = BTreeSet::new();
    let mut retained_endpoints = BTreeMap::new();
    for member in &program.members {
        match traces[&member.id]
            .release
            .as_ref()
            .map(|(_, step)| step.disposition)
        {
            Some(Disposition::Released) => {
                released.insert(member.id);
            }
            Some(Disposition::Retained {
                custodian: Some(custodian),
            }) => {
                retained_endpoints.insert(member.id, custodian);
            }
            _ => {}
        }
    }

    AggregateCompositionResult {
        verdict: AggregateVerdict::Composed,
        refusals: Vec::new(),
        certificate: Some(AggregateCertificate {
            root: program.root,
            released,
            retained_endpoints,
            quiescent_callbacks: program
                .callbacks
                .iter()
                .filter(|c| c.state == CallbackState::Quiescent)
                .map(|c| c.id)
                .collect(),
            joined_tasks: program
                .tasks
                .iter()
                .filter(|t| t.state == TaskState::Joined)
                .map(|t| t.id)
                .collect(),
            released_guards: program
                .guards
                .iter()
                .filter(|g| g.state == GuardState::Released)
                .map(|g| g.id)
                .collect(),
            distinct_identities: seen_identity.len(),
            teardown_point,
            declared_premises: program.declared_premises.clone(),
            durability_premises: program.durability_premises.clone(),
        }),
    }
}

fn unknown(mut refusals: Vec<CompositionRefusal>) -> AggregateCompositionResult {
    refusals.sort();
    refusals.dedup();
    AggregateCompositionResult {
        verdict: AggregateVerdict::Unknown,
        refusals,
        certificate: None,
    }
}

fn check_wellformed(program: &AggregateCompositionProgram) -> Vec<MalformedKind> {
    let mut out = Vec::new();
    let mut ids: BTreeSet<MemberId> = BTreeSet::new();
    for member in &program.members {
        if !ids.insert(member.id) {
            out.push(MalformedKind::DuplicateMemberId(member.id));
        }
    }
    match program.members.iter().find(|m| m.id == program.root) {
        None => out.push(MalformedKind::MissingRoot),
        Some(root) => {
            if root.kind != MemberKind::Aggregate {
                out.push(MalformedKind::RootIsNotAnAggregate);
            }
            if root.parent.is_some() {
                out.push(MalformedKind::RootHasAParent);
            }
        }
    }
    for member in &program.members {
        if let Some(parent) = member.parent
            && !ids.contains(&parent)
        {
            out.push(MalformedKind::UnknownMember(parent));
        }
    }
    // containment must be acyclic
    for member in &program.members {
        let mut seen = BTreeSet::new();
        let mut cursor = member.parent;
        while let Some(current) = cursor {
            if !seen.insert(current) {
                out.push(MalformedKind::ParentCycle(member.id));
                break;
            }
            cursor = program
                .members
                .iter()
                .find(|m| m.id == current)
                .and_then(|m| m.parent);
        }
    }

    let mut callback_ids = BTreeSet::new();
    for callback in &program.callbacks {
        if !callback_ids.insert(callback.id) {
            out.push(MalformedKind::DuplicateCallbackId(callback.id));
        }
    }
    let mut task_ids = BTreeSet::new();
    for task in &program.tasks {
        if !task_ids.insert(task.id) {
            out.push(MalformedKind::DuplicateTaskId(task.id));
        }
        for member in &task.captures {
            if !ids.contains(member) {
                out.push(MalformedKind::UnknownMember(*member));
            }
        }
    }
    let mut footprint_ids = BTreeSet::new();
    for footprint in &program.footprints {
        if !footprint_ids.insert(footprint.id) {
            out.push(MalformedKind::DuplicateFootprintId(footprint.id));
        }
        for member in &footprint.members {
            if !ids.contains(member) {
                out.push(MalformedKind::UnknownMember(*member));
            }
        }
    }
    let mut guard_ids = BTreeSet::new();
    for guard in &program.guards {
        if !guard_ids.insert(guard.id) {
            out.push(MalformedKind::DuplicateGuardId(guard.id));
        }
        if !footprint_ids.contains(&guard.footprint) {
            out.push(MalformedKind::UnknownFootprint(guard.footprint));
        }
    }

    if let Some(wrapper) = &program.wrapper
        && !ids.contains(&wrapper.held_member)
    {
        out.push(MalformedKind::UnknownMember(wrapper.held_member));
    }

    // schedule references and publication discipline
    let mut published: BTreeSet<PublicationId> = BTreeSet::new();
    let mut reclaimed: BTreeSet<PublicationId> = BTreeSet::new();
    let mut borrow_dropped = false;
    for step in &program.schedule {
        match step {
            LifecycleStep::Acquire { member }
            | LifecycleStep::Release(ReleaseStep { member, .. }) => {
                if !ids.contains(member) {
                    out.push(MalformedKind::UnknownMember(*member));
                }
            }
            LifecycleStep::Publish { publication } => {
                let declared = program
                    .wrapper
                    .as_ref()
                    .is_some_and(|w| w.publications.contains(publication));
                if !declared {
                    out.push(MalformedKind::UnknownPublication(*publication));
                }
                if !published.insert(*publication) {
                    out.push(MalformedKind::PublicationPublishedTwice(*publication));
                }
            }
            LifecycleStep::Reclaim { publication } => {
                let declared = program
                    .wrapper
                    .as_ref()
                    .is_some_and(|w| w.publications.contains(publication));
                if !declared {
                    out.push(MalformedKind::UnknownPublication(*publication));
                }
                if !published.contains(publication) {
                    out.push(MalformedKind::PublicationReclaimedBeforePublish(
                        *publication,
                    ));
                }
                if !reclaimed.insert(*publication) {
                    out.push(MalformedKind::PublicationReclaimedTwice(*publication));
                }
            }
            LifecycleStep::DropBorrow => {
                if program.wrapper.is_none() {
                    out.push(MalformedKind::WrapperStepWithoutWrapper);
                }
                if borrow_dropped {
                    out.push(MalformedKind::BorrowDroppedTwice);
                }
                borrow_dropped = true;
            }
        }
    }
    if program.wrapper.is_none()
        && program.schedule.iter().any(|step| {
            matches!(
                step,
                LifecycleStep::Publish { .. }
                    | LifecycleStep::Reclaim { .. }
                    | LifecycleStep::Release(ReleaseStep {
                        by: Releaser::Wrapper,
                        ..
                    })
            )
        })
    {
        out.push(MalformedKind::WrapperStepWithoutWrapper);
    }
    if program.wrapper.is_none() && program.members.iter().any(|m| m.owner == Owner::Wrapper) {
        out.push(MalformedKind::WrapperStepWithoutWrapper);
    }

    out.sort();
    out.dedup();
    out
}

fn trace_members(program: &AggregateCompositionProgram) -> BTreeMap<MemberId, MemberTrace> {
    let mut traces: BTreeMap<MemberId, MemberTrace> = program
        .members
        .iter()
        .map(|member| {
            (
                member.id,
                MemberTrace {
                    acquired: None,
                    acquired_twice: false,
                    release: None,
                    released_twice: false,
                },
            )
        })
        .collect();
    for (index, step) in program.schedule.iter().enumerate() {
        match step {
            LifecycleStep::Acquire { member } => {
                if let Some(trace) = traces.get_mut(member) {
                    if trace.acquired.is_some() {
                        trace.acquired_twice = true;
                    } else {
                        trace.acquired = Some(index);
                    }
                }
            }
            LifecycleStep::Release(release) => {
                if let Some(trace) = traces.get_mut(&release.member) {
                    if trace.release.is_some() {
                        trace.released_twice = true;
                    } else {
                        trace.release = Some((index, *release));
                    }
                }
            }
            _ => {}
        }
    }
    traces
}

fn check_release_step(
    program: &AggregateCompositionProgram,
    member: &AggregateMember,
    step: &ReleaseStep,
) -> Vec<CompositionRefusal> {
    let mut out = Vec::new();
    match step.evidence {
        CustodyEvidence::StatusFact(fact) => {
            out.push(CompositionRefusal::StatusFactIsNotCustodyEvidence {
                member: member.id,
                fact,
            });
        }
        CustodyEvidence::DurabilityDischarged(premise) => {
            out.push(CompositionRefusal::DurabilityAssumedAsDischarged {
                member: member.id,
                premise,
            });
        }
        CustodyEvidence::ExitResidualDischarged
        | CustodyEvidence::ObservedEndpointPostState
        | CustodyEvidence::DeclaredDestructorContract => {}
    }
    match step.disposition {
        Disposition::Released => {}
        Disposition::Unresolved(reason) => {
            out.push(CompositionRefusal::UnresolvedDisposition {
                member: member.id,
                reason,
            });
        }
        Disposition::Retained { custodian } => {
            if !matches!(member.kind, MemberKind::Endpoint(_)) {
                out.push(CompositionRefusal::RetainedAllocation { member: member.id });
            }
            match custodian {
                None => out.push(CompositionRefusal::RetainedEndpointWithoutCustodian {
                    member: member.id,
                }),
                Some(custodian) => {
                    if !program.external_custodians.contains(&custodian) {
                        out.push(CompositionRefusal::RetainedEndpointCustodianNotExternal {
                            member: member.id,
                            custodian,
                        });
                    }
                }
            }
        }
    }
    out
}

/// A member's live interval end. A retained or unresolved member never stops
/// being live for the purposes of address reuse.
fn live_end(trace: &MemberTrace) -> usize {
    match &trace.release {
        Some((index, step)) if step.disposition == Disposition::Released => *index,
        _ => usize::MAX,
    }
}

fn check_address_separation(
    program: &AggregateCompositionProgram,
    traces: &BTreeMap<MemberId, MemberTrace>,
) -> Vec<CompositionRefusal> {
    let mut out = Vec::new();
    for (i, first) in program.members.iter().enumerate() {
        let Some(address) = first.address else {
            continue;
        };
        for second in program.members.iter().skip(i + 1) {
            if second.address != Some(address) {
                continue;
            }
            let first_trace = &traces[&first.id];
            let second_trace = &traces[&second.id];
            let (Some(first_acquired), Some(second_acquired)) =
                (first_trace.acquired, second_trace.acquired)
            else {
                // Never-acquired members are already refused; do not double-report.
                continue;
            };
            let disjoint =
                live_end(first_trace) < second_acquired || live_end(second_trace) < first_acquired;
            if !disjoint {
                let (earlier, later) = if first_acquired <= second_acquired {
                    (first.id, second.id)
                } else {
                    (second.id, first.id)
                };
                out.push(CompositionRefusal::AddressReusedWhileLive {
                    earlier,
                    later,
                    address,
                });
            }
        }
    }
    out
}

fn check_quiescence(
    program: &AggregateCompositionProgram,
    members: &BTreeMap<MemberId, &AggregateMember>,
) -> Vec<CompositionRefusal> {
    let mut out = Vec::new();
    let member_payloads: BTreeSet<PayloadId> = members
        .values()
        .flat_map(|member| member.payloads.iter().copied())
        .collect();
    let declared_tasks: BTreeMap<TaskId, &TaskRecord> =
        program.tasks.iter().map(|task| (task.id, task)).collect();

    for callback in &program.callbacks {
        let touches_member = !callback.authority.is_disjoint(&member_payloads);
        if touches_member && callback.state != CallbackState::Quiescent {
            out.push(CompositionRefusal::CallbackNotQuiescent {
                callback: callback.id,
                state: callback.state,
            });
        }
        for task in &callback.spawned_tasks {
            match declared_tasks.get(task) {
                None => out.push(CompositionRefusal::CallbackSpawnedTaskUnaccounted {
                    callback: callback.id,
                    task: *task,
                }),
                Some(record) if record.state != TaskState::Joined => {
                    out.push(CompositionRefusal::CallbackSpawnedTaskUnaccounted {
                        callback: callback.id,
                        task: *task,
                    });
                }
                Some(_) => {}
            }
        }
    }

    for task in &program.tasks {
        let touches_member = task.captures.iter().any(|id| members.contains_key(id));
        if touches_member && task.state != TaskState::Joined {
            out.push(CompositionRefusal::TaskNotJoined {
                task: task.id,
                state: task.state,
            });
        }
    }

    let footprints: BTreeMap<FootprintId, &FootprintRecord> = program
        .footprints
        .iter()
        .map(|footprint| (footprint.id, footprint))
        .collect();
    for guard in &program.guards {
        let touches_member = footprints
            .get(&guard.footprint)
            .is_some_and(|footprint| footprint.members.iter().any(|id| members.contains_key(id)));
        if touches_member && guard.state != GuardState::Released {
            out.push(CompositionRefusal::GuardOutstanding {
                guard: guard.id,
                footprint: guard.footprint,
            });
        }
    }
    out
}

fn check_wrapper(
    program: &AggregateCompositionProgram,
    members: &BTreeMap<MemberId, &AggregateMember>,
    traces: &BTreeMap<MemberId, MemberTrace>,
    teardown_point: Option<usize>,
) -> Vec<CompositionRefusal> {
    let mut out = Vec::new();
    let Some(wrapper) = &program.wrapper else {
        return out;
    };

    // Structural separation of the two containment trees.
    for member in &program.members {
        let Some(parent) = member.parent.and_then(|id| members.get(&id)) else {
            continue;
        };
        if member.owner != parent.owner {
            out.push(CompositionRefusal::WrapperOverlapsAggregate { member: member.id });
        }
    }
    if members
        .get(&wrapper.held_member)
        .is_some_and(|member| member.owner != Owner::Aggregate)
    {
        out.push(CompositionRefusal::WrapperOverlapsAggregate {
            member: wrapper.held_member,
        });
    }

    // A wrapper release may only name a wrapper-owned member, or the held
    // member when the hold is owning.
    for step in &program.schedule {
        let LifecycleStep::Release(release) = step else {
            continue;
        };
        if release.by != Releaser::Wrapper {
            continue;
        }
        let wrapper_owned = members
            .get(&release.member)
            .is_some_and(|member| member.owner == Owner::Wrapper);
        if wrapper_owned {
            continue;
        }
        if release.member == wrapper.held_member {
            if wrapper.hold == HoldMode::Borrowed {
                out.push(CompositionRefusal::BorrowedHoldReleasedByWrapper {
                    member: release.member,
                });
            }
            continue;
        }
        out.push(CompositionRefusal::WrapperReleasesForeignMember {
            member: release.member,
        });
    }

    let held_release = traces.get(&wrapper.held_member).and_then(|t| t.release);
    match wrapper.hold {
        HoldMode::Owned => {
            if held_release.is_some_and(|(_, step)| step.by == Releaser::Parent) {
                out.push(CompositionRefusal::OwnedHoldAlsoReleasedByParent {
                    member: wrapper.held_member,
                });
            }
        }
        HoldMode::Borrowed => {
            let drop_index = program
                .schedule
                .iter()
                .position(|step| matches!(step, LifecycleStep::DropBorrow));
            let held_index = held_release.map(|(index, _)| index);
            let ordered = match (drop_index, held_index) {
                (Some(drop_at), Some(release_at)) => drop_at < release_at,
                _ => false,
            };
            if !ordered {
                out.push(CompositionRefusal::BorrowedHoldOutlivesParentRelease {
                    member: wrapper.held_member,
                });
            }
        }
    }

    for publication in &wrapper.publications {
        let reclaim = program.schedule.iter().position(
            |step| matches!(step, LifecycleStep::Reclaim { publication: p } if p == publication),
        );
        match (reclaim, teardown_point) {
            (None, _) => out.push(CompositionRefusal::WrapperPublicationUnreclaimed {
                publication: *publication,
            }),
            (Some(reclaim_at), Some(teardown_at)) if reclaim_at >= teardown_at => {
                out.push(CompositionRefusal::WrapperPublicationSurvivesTeardown {
                    publication: *publication,
                });
            }
            _ => {}
        }
    }
    out
}

#[cfg(test)]
#[path = "../tests/unit/aggregate_composition.rs"]
mod tests;

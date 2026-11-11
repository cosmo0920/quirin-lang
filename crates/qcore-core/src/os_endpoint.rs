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

//! Target-qualified OS endpoint failure post-state relation (card O03).
//!
//! This module decides the failure post-state of **one** endpoint operation on
//! **one** target: `int close(int fd)` on Linux. The specification, with its
//! primary sources, is `docs/design/OS_FAILURE_POSTSTATE_PROFILE.md`; the
//! evidence record is `docs/reports/remaining-o03/README.md`.
//!
//! # What this is not
//!
//! It is not a universal close or `munmap` policy, not an operating-system
//! durability result, and not an extension of resource-contract v1. **No
//! production audit route consumes it**: wiring it into the C gateway needs
//! relational branch analysis, recorded as follow-on unit U3. Nothing here
//! promotes a verdict.
//!
//! # Why three identities
//!
//! `close` acts differently on three separable things, and a single liveness
//! bit cannot express "the descriptor is gone **and** nothing was discharged":
//!
//! * the **descriptor slot**, identified by `(number, generation)` — never by a
//!   number alone (contract spec §C10);
//! * the **open file description**, released only when its last reference goes,
//!   where a `dup`, a forked duplicate and an in-flight syscall all count;
//! * the **deferred write-back duty**, which `fsync(2)` may discharge and
//!   `close(2)` never does.
//!
//! # Why the target is part of the relation
//!
//! On Linux the kernel releases the descriptor early, so a failed `close`
//! releases it anyway — except on `EBADF` — and retrying is a documented hazard
//! because the number is immediately reusable. POSIX.1-2024 standardized the
//! opposite behaviour, in which the descriptor stays open and `close` must be
//! called again. The two targets therefore prescribe *opposite* recovery for
//! the same errno, so an unpinned target yields [`DescriptorState::Indeterminate`]
//! with no permitted recovery at all, and no portable contract is published.
//!
//! # Fail-closed
//!
//! Every arm the cited documentation does not determine yields a poisoned
//! descriptor state carrying the full conservative residual duty set, and
//! **no arm of this relation ever permits an automatic retry** of a poisoned
//! or released epoch.

use std::collections::BTreeSet;

/// The platform whose documented semantics the relation is being read against.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Target {
    /// Any Linux ABI target. The behaviour specified is the kernel's.
    Linux,
    /// A strictly POSIX.1-2024 conforming implementation. Linux is documented
    /// as non-conforming here, which is the point of keeping both.
    Posix2024,
    /// No target pinned. Every outcome the two disagree on is indeterminate.
    Unpinned,
}

impl Target {
    /// Every target, for exhaustive domain checks.
    pub const ALL: [Target; 3] = [Target::Linux, Target::Posix2024, Target::Unpinned];
}

/// The kind of OS endpoint an epoch identifies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EndpointKind {
    /// A per-process file-descriptor table slot.
    Descriptor,
}

/// An OS endpoint identity: kind, number and acquisition generation.
///
/// Contract spec §C10: numeric reuse after an epoch has ended is permitted and
/// must not merge the epochs. Two epochs are equal only when all three
/// components agree, so a recycled number is a different identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Epoch {
    pub kind: EndpointKind,
    pub number: u32,
    pub generation: u32,
}

impl Epoch {
    pub fn new(kind: EndpointKind, number: u32, generation: u32) -> Self {
        Self {
            kind,
            number,
            generation,
        }
    }

    /// The next epoch on the same number. Never equal to `self`.
    pub fn successor(self) -> Self {
        Self {
            generation: self.generation + 1,
            ..self
        }
    }
}

/// Post-state of the descriptor slot. All five are reachable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DescriptorState {
    /// No call was issued on this path; the epoch is untouched.
    NotAttempted,
    /// A call was issued and the *analysis* cannot map the outcome to a
    /// terminal state. A better analysis could resolve this.
    Attempted,
    /// The epoch is known live after the call.
    Retained,
    /// The epoch ended. The number may be rebound by a fresh generation.
    Released,
    /// The *documentation* does not determine the fate. No analysis resolves
    /// this; only pinning the target, or an external oracle, can.
    Indeterminate,
}

impl DescriptorState {
    pub const ALL: [DescriptorState; 5] = [
        DescriptorState::NotAttempted,
        DescriptorState::Attempted,
        DescriptorState::Retained,
        DescriptorState::Released,
        DescriptorState::Indeterminate,
    ];

    /// The two states that carry the conservative residual duty set. They are
    /// distinguished by *why* the epoch is unresolved, not by what is owed.
    pub fn is_poisoned(self) -> bool {
        matches!(
            self,
            DescriptorState::Attempted | DescriptorState::Indeterminate
        )
    }
}

/// Post-state of the open file description.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DescriptionState {
    Retained,
    Released,
    Unknown,
}

/// Post-state of the deferred write-back duty.
///
/// [`WriteBackState::Discharged`] exists so that the relation can be shown
/// never to derive it: a successful `close` does not guarantee the data reached
/// the disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WriteBackState {
    Failed,
    Unknown,
    Discharged,
}

/// A residual obligation the post-state carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Duty {
    /// The epoch is live and must still be released.
    ReleaseEpoch,
    /// No read, write, `fcntl` or `dup` through the number.
    NoUse,
    /// No retry of `close` on the number.
    NoRetryClose,
    /// The number is not available for a fresh epoch's identity.
    NoTreatNumberAsFree,
    /// The endpoint stays counted as unresolved.
    KeepInDenominator,
    /// The enclosing result is Unknown for descriptor custody.
    ReportUnresolved,
    /// Data may have been lost and that must be reported, not swallowed.
    ReportPossibleDataLoss,
}

/// An operation the post-state permits on the same number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Recovery {
    Close,
}

/// A call-site premise the relation consumes. None is established here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Premise {
    /// A1 — the target is pinned.
    TargetPinned,
    /// A2 — the analysis holds a live epoch and `fd` names it.
    EpochLive,
    /// A3 — no concurrent close or reuse of the number.
    SoleCustodian,
    /// A4 — the `int` result is bound and comparable against zero.
    ResultObserved,
    /// A5 — `errno` is read on the failure branch and not clobbered.
    ErrnoObserved,
    /// A6 — this epoch is the last reference to its description.
    LastReference,
    /// A7 — the process holds no POSIX record locks on the file.
    NoRecordLocks,
}

/// The call-site pre-state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Premises {
    pub epoch_live: bool,
    pub sole_custodian: bool,
    pub result_observed: bool,
    pub errno_observed: bool,
    pub last_reference: bool,
    pub no_record_locks: bool,
}

impl Default for Premises {
    /// Every premise granted. Consumers must justify each one separately.
    fn default() -> Self {
        Self {
            epoch_live: true,
            sole_custodian: true,
            result_observed: true,
            errno_observed: true,
            last_reference: true,
            no_record_locks: true,
        }
    }
}

impl Premises {
    /// All 64 assignments, for exhaustive domain checks.
    pub fn all() -> Vec<Premises> {
        let mut out = Vec::with_capacity(64);
        for bits in 0u8..64 {
            out.push(Premises {
                epoch_live: bits & 1 != 0,
                sole_custodian: bits & 2 != 0,
                result_observed: bits & 4 != 0,
                errno_observed: bits & 8 != 0,
                last_reference: bits & 16 != 0,
                no_record_locks: bits & 32 != 0,
            });
        }
        out
    }
}

/// The `errno` observed on a `-1` return.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Errno {
    /// The one documented error on which implementations do not close.
    Ebadf,
    Eintr,
    /// The deferred write-back family: the error was caused by an earlier
    /// write and is only reported here.
    Eio,
    Enospc,
    Edquot,
    /// Some other observed errno.
    Other,
    /// The failure branch never read `errno`.
    Unobserved,
}

impl Errno {
    fn is_deferred_io(self) -> bool {
        matches!(self, Errno::Eio | Errno::Enospc | Errno::Edquot)
    }
}

/// One observed outcome of one `close(fd)` call. Total over what a consumer can
/// observe, which is why the unobserved cases are outcomes and not gaps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Outcome {
    /// O0 — not attempted on this path.
    NotAttempted,
    /// O1 — attempted, result discarded or the call site unresolvable.
    ResultUnobserved,
    /// O2 — returned zero. The success test is `ret == 0`, spelled: not
    /// `ret != -1`, and not a nullability test.
    Zero,
    /// O3–O7 — returned `-1` with this errno.
    MinusOne(Errno),
}

impl Outcome {
    /// Every outcome shape, for exhaustive domain checks.
    pub fn all() -> Vec<Outcome> {
        let mut out = vec![
            Outcome::NotAttempted,
            Outcome::ResultUnobserved,
            Outcome::Zero,
        ];
        for errno in [
            Errno::Ebadf,
            Errno::Eintr,
            Errno::Eio,
            Errno::Enospc,
            Errno::Edquot,
            Errno::Other,
            Errno::Unobserved,
        ] {
            out.push(Outcome::MinusOne(errno));
        }
        out
    }
}

/// The derived post-state of one call, per identity, with its duties.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostState {
    pub target: Target,
    pub descriptor: DescriptorState,
    pub description: DescriptionState,
    pub writeback: WriteBackState,
    pub duties: BTreeSet<Duty>,
    pub permitted_recovery: BTreeSet<Recovery>,
    pub premise_violations: Vec<Premise>,
    pub note: &'static str,
}

impl PostState {
    pub fn is_poisoned(&self) -> bool {
        self.descriptor.is_poisoned()
    }
}

/// The conservative residual duty set carried by both poisoned states.
///
/// It forbids both directions at once: the number may neither be used nor
/// retried nor recycled, and the endpoint stays in the live denominator.
pub fn poisoned_duties() -> BTreeSet<Duty> {
    [
        Duty::NoUse,
        Duty::NoRetryClose,
        Duty::NoTreatNumberAsFree,
        Duty::KeepInDenominator,
        Duty::ReportUnresolved,
    ]
    .into_iter()
    .collect()
}

/// The relation was asked about an input outside its declared domain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RelationError {
    /// The not-attempted arm was asked about a non-existent epoch.
    NoEpochToDescribe,
    /// Two post-states of different targets cannot be joined.
    MismatchedTargets,
}

fn poisoned(
    target: Target,
    state: DescriptorState,
    note: &'static str,
    violations: Vec<Premise>,
) -> PostState {
    debug_assert!(state.is_poisoned());
    PostState {
        target,
        descriptor: state,
        description: DescriptionState::Unknown,
        writeback: WriteBackState::Unknown,
        duties: poisoned_duties(),
        permitted_recovery: BTreeSet::new(),
        premise_violations: violations,
        note,
    }
}

/// The open file description is freed only when the closed descriptor was its
/// last reference.
fn description_after_release(premises: Premises) -> DescriptionState {
    if premises.last_reference {
        DescriptionState::Released
    } else {
        DescriptionState::Retained
    }
}

/// `R_close[target] : Premises × Outcome → PostState`.
///
/// # Errors
///
/// [`RelationError::NoEpochToDescribe`] when the not-attempted arm is asked
/// about an epoch the caller has already said is not live.
pub fn close_poststate(
    target: Target,
    premises: Premises,
    outcome: Outcome,
) -> Result<PostState, RelationError> {
    // A7 is consumed by the whole relation rather than a single conclusion:
    // closing removes every POSIX record lock the process holds on the file,
    // whichever descriptor took it. That footprint escapes all three modelled
    // identities, so it is refused rather than silently ignored.
    if !premises.no_record_locks {
        return Ok(poisoned(
            target,
            DescriptorState::Indeterminate,
            "A7 denied: close removes process-wide record locks, an effect \
             outside every modelled identity",
            vec![Premise::NoRecordLocks],
        ));
    }

    if outcome == Outcome::NotAttempted {
        if !premises.epoch_live {
            return Err(RelationError::NoEpochToDescribe);
        }
        return Ok(PostState {
            target,
            descriptor: DescriptorState::NotAttempted,
            description: DescriptionState::Retained,
            writeback: WriteBackState::Unknown,
            duties: [Duty::ReleaseEpoch, Duty::KeepInDenominator]
                .into_iter()
                .collect(),
            permitted_recovery: [Recovery::Close].into_iter().collect(),
            premise_violations: Vec::new(),
            note: "no call was issued on this path; the epoch is untouched",
        });
    }

    if outcome == Outcome::ResultUnobserved || !premises.result_observed {
        // A discarded result cannot be separated from EBADF, the one documented
        // arm that does not release. Analysis gap, not a documentation gap.
        return Ok(poisoned(
            target,
            DescriptorState::Attempted,
            "A4 denied: the result was not observed, so the EBADF arm is not \
             excluded and no release can be concluded",
            Vec::new(),
        ));
    }

    if outcome == Outcome::Zero {
        // A3 is load-bearing, not decoration. A successful close proves that
        // *some* epoch on that number was closed; only sole custody identifies
        // it with the tracked one. The dangerous double close does not report
        // EBADF at all — on a recycled number it closes a live foreign epoch
        // and returns zero.
        if !premises.sole_custodian {
            return Ok(poisoned(
                target,
                DescriptorState::Indeterminate,
                "A3 denied: a successful close identifies the epoch it closed \
                 only under sole custody of the number",
                vec![Premise::SoleCustodian],
            ));
        }
        return Ok(PostState {
            target,
            descriptor: DescriptorState::Released,
            description: description_after_release(premises),
            // A successful close does not guarantee the data reached the disk.
            // Success discharges the descriptor, never the data.
            writeback: WriteBackState::Unknown,
            duties: BTreeSet::new(),
            permitted_recovery: BTreeSet::new(),
            premise_violations: Vec::new(),
            note: "epoch ended; the number may be reused by a fresh generation",
        });
    }

    let errno = match outcome {
        Outcome::MinusOne(errno) => errno,
        _ => unreachable!("every other outcome is handled above"),
    };

    if errno == Errno::Unobserved || !premises.errno_observed {
        return Ok(poisoned(
            target,
            DescriptorState::Attempted,
            "A5 denied: errno was not observed, so the EBADF arm is not \
             excluded and no release can be concluded",
            Vec::new(),
        ));
    }

    if errno == Errno::Ebadf {
        // EBADF is a refutation, not information: under A2 the analysis
        // believed the epoch live, and this says it was not. The honest state
        // is poisoned with the violation recorded — not Retained, because the
        // thing that would be retained is exactly what has been refuted.
        let violations = if premises.epoch_live {
            vec![Premise::EpochLive]
        } else {
            Vec::new()
        };
        return Ok(PostState {
            target,
            descriptor: DescriptorState::Indeterminate,
            description: DescriptionState::Unknown,
            writeback: WriteBackState::Unknown,
            duties: poisoned_duties(),
            permitted_recovery: BTreeSet::new(),
            premise_violations: violations,
            note: "EBADF refutes the tracked epoch; this call ended no epoch",
        });
    }

    if errno == Errno::Eintr {
        return Ok(match target {
            // The Linux kernel releases the descriptor early; the steps that
            // can fail happen later. Retrying may close a number another
            // thread has already reused.
            Target::Linux => PostState {
                target,
                descriptor: DescriptorState::Released,
                description: DescriptionState::Unknown,
                writeback: WriteBackState::Failed,
                duties: [Duty::ReportPossibleDataLoss].into_iter().collect(),
                permitted_recovery: BTreeSet::new(),
                premise_violations: Vec::new(),
                note: "Linux releases the descriptor early; retry is forbidden",
            },
            // POSIX.1-2024 standardized the behaviour in which the descriptor
            // stays open and close must be called again.
            Target::Posix2024 => PostState {
                target,
                descriptor: DescriptorState::Retained,
                description: DescriptionState::Retained,
                writeback: WriteBackState::Failed,
                duties: [
                    Duty::ReleaseEpoch,
                    Duty::KeepInDenominator,
                    Duty::ReportPossibleDataLoss,
                ]
                .into_iter()
                .collect(),
                permitted_recovery: [Recovery::Close].into_iter().collect(),
                premise_violations: Vec::new(),
                note: "POSIX.1-2024 keeps the descriptor open; close must be called again",
            },
            // The two targets do not merely differ in what is known: they
            // prescribe opposite recovery. One requires the retry the other
            // forbids, so without a pinned target nothing is permitted.
            Target::Unpinned => {
                let mut duties = poisoned_duties();
                duties.insert(Duty::ReportPossibleDataLoss);
                PostState {
                    target,
                    descriptor: DescriptorState::Indeterminate,
                    description: DescriptionState::Unknown,
                    writeback: WriteBackState::Failed,
                    duties,
                    permitted_recovery: BTreeSet::new(),
                    premise_violations: Vec::new(),
                    note: "documentation-level divergence: Linux forbids the retry \
                           POSIX.1-2024 requires; the target must be pinned",
                }
            }
        });
    }

    if errno.is_deferred_io() {
        return Ok(if target == Target::Linux {
            PostState {
                target,
                descriptor: DescriptorState::Released,
                description: description_after_release(premises),
                writeback: WriteBackState::Failed,
                duties: [Duty::ReportPossibleDataLoss].into_iter().collect(),
                permitted_recovery: BTreeSet::new(),
                premise_violations: Vec::new(),
                note: "Linux released the descriptor before the failing flush step",
            }
        } else {
            let mut duties = poisoned_duties();
            duties.insert(Duty::ReportPossibleDataLoss);
            PostState {
                target,
                descriptor: DescriptorState::Indeterminate,
                description: DescriptionState::Unknown,
                writeback: WriteBackState::Failed,
                duties,
                permitted_recovery: BTreeSet::new(),
                premise_violations: Vec::new(),
                note: "the standard states no general release rule for this error",
            }
        });
    }

    // Any other observed errno.
    Ok(if target == Target::Linux {
        PostState {
            target,
            descriptor: DescriptorState::Released,
            description: DescriptionState::Unknown,
            writeback: WriteBackState::Unknown,
            duties: BTreeSet::new(),
            permitted_recovery: BTreeSet::new(),
            premise_violations: Vec::new(),
            note: "Linux always releases except on EBADF",
        }
    } else {
        poisoned(
            target,
            DescriptorState::Indeterminate,
            "the standard states no general release rule for this error",
            Vec::new(),
        )
    })
}

/// Join two post-states reaching the same program point.
///
/// Agreement is preserved exactly. Disagreement widens to
/// [`DescriptorState::Indeterminate`] with the poisoned duty set, dropping
/// [`Duty::ReleaseEpoch`] because it contradicts [`Duty::NoRetryClose`]. A join
/// never produces a permitted recovery that both sides did not permit.
///
/// # Errors
///
/// [`RelationError::MismatchedTargets`] when the two sides read different
/// platforms.
pub fn join(left: &PostState, right: &PostState) -> Result<PostState, RelationError> {
    if left.target != right.target {
        return Err(RelationError::MismatchedTargets);
    }

    let (descriptor, duties, permitted_recovery) = if left.descriptor == right.descriptor {
        (
            left.descriptor,
            left.duties.union(&right.duties).copied().collect(),
            left.permitted_recovery
                .intersection(&right.permitted_recovery)
                .copied()
                .collect(),
        )
    } else {
        let mut duties: BTreeSet<Duty> = poisoned_duties();
        duties.extend(left.duties.iter().copied());
        duties.extend(right.duties.iter().copied());
        duties.remove(&Duty::ReleaseEpoch);
        (DescriptorState::Indeterminate, duties, BTreeSet::new())
    };

    let description = if left.description == right.description {
        left.description
    } else {
        DescriptionState::Unknown
    };

    let writeback = if left.writeback == right.writeback {
        left.writeback
    } else if left.writeback == WriteBackState::Failed || right.writeback == WriteBackState::Failed
    {
        WriteBackState::Failed
    } else {
        WriteBackState::Unknown
    };

    let mut premise_violations = left.premise_violations.clone();
    for violation in &right.premise_violations {
        if !premise_violations.contains(violation) {
            premise_violations.push(*violation);
        }
    }

    Ok(PostState {
        target: left.target,
        descriptor,
        description,
        writeback,
        duties,
        permitted_recovery,
        premise_violations,
        note: "join",
    })
}

/// A claim about a post-state. Every field is optional, so a partial claim can
/// be checked, but no claim can widen the relation.
///
/// Unlike an untyped record, a claim naming a field or a state the vocabulary
/// does not have is rejected by the compiler rather than at run time.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PostStateClaim {
    pub descriptor: Option<DescriptorState>,
    pub description: Option<DescriptionState>,
    pub writeback: Option<WriteBackState>,
    pub duties: Option<BTreeSet<Duty>>,
    pub permitted_recovery: Option<BTreeSet<Recovery>>,
}

/// Why a claimed post-state was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    Descriptor {
        claimed: DescriptorState,
        derived: DescriptorState,
    },
    Description {
        claimed: DescriptionState,
        derived: DescriptionState,
    },
    WriteBack {
        claimed: WriteBackState,
        derived: WriteBackState,
    },
    Duties {
        claimed: BTreeSet<Duty>,
        derived: BTreeSet<Duty>,
    },
    PermittedRecovery {
        claimed: BTreeSet<Recovery>,
        derived: BTreeSet<Recovery>,
    },
}

/// Refuse a forged post-state.
///
/// Returns `Ok(None)` when every claimed field agrees with the derived
/// post-state, and `Ok(Some(refusal))` otherwise.
///
/// # Errors
///
/// Propagates [`close_poststate`]'s domain errors.
pub fn check_claim(
    target: Target,
    premises: Premises,
    outcome: Outcome,
    claim: &PostStateClaim,
) -> Result<Option<Refusal>, RelationError> {
    let derived = close_poststate(target, premises, outcome)?;

    if let Some(claimed) = claim.descriptor
        && claimed != derived.descriptor
    {
        return Ok(Some(Refusal::Descriptor {
            claimed,
            derived: derived.descriptor,
        }));
    }
    if let Some(claimed) = claim.description
        && claimed != derived.description
    {
        return Ok(Some(Refusal::Description {
            claimed,
            derived: derived.description,
        }));
    }
    if let Some(claimed) = claim.writeback
        && claimed != derived.writeback
    {
        return Ok(Some(Refusal::WriteBack {
            claimed,
            derived: derived.writeback,
        }));
    }
    if let Some(claimed) = &claim.duties
        && claimed != &derived.duties
    {
        return Ok(Some(Refusal::Duties {
            claimed: claimed.clone(),
            derived: derived.duties.clone(),
        }));
    }
    if let Some(claimed) = &claim.permitted_recovery
        && claimed != &derived.permitted_recovery
    {
        return Ok(Some(Refusal::PermittedRecovery {
            claimed: claimed.clone(),
            derived: derived.permitted_recovery.clone(),
        }));
    }
    Ok(None)
}

/// A submitted **declared close record**: what a producer says happened, and
/// what it says the post-state is.
///
/// This is the record layer `QCore.DescriptorCustody` checks in Lean, mirrored
/// here so the two developments state the same admission rule (card O03-U4).
/// The historical record shape carried one `live_after: bool`, which cannot
/// express "the descriptor is gone **and** nothing was discharged"; this shape
/// carries the target, the outcome, the five-valued descriptor state and the
/// permitted recovery separately, and every field is checked against
/// [`close_poststate`] rather than believed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CloseRecord {
    pub target: Target,
    pub premises: Premises,
    pub outcome: Outcome,
    /// The producer asserts the epoch was live before the call (premise A2).
    pub live_before: bool,
    pub descriptor_after: DescriptorState,
    /// The producer asserts a `close` on this number is permitted afterwards.
    pub may_retry_close: bool,
    /// The producer asserts the descriptor custody obligation ended.
    pub custody_discharged: bool,
    /// The producer asserts the data reached the disk. No arm of the relation
    /// ever derives this, so a record claiming it is always refused.
    pub data_duty_discharged: bool,
}

/// Why a declared close record was refused. Module-local keys; no diagnostic
/// code is claimed for this model-level admission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CloseRecordRefusal {
    /// The record does not assert a live epoch before the call.
    NoLiveEpochBefore,
    /// The record claims durability, which `close` never establishes.
    ClaimsDataDuty,
    /// The relation has nothing to say about this input.
    OutsideRelationDomain(RelationError),
    /// The claimed descriptor state is not the derived one.
    Descriptor {
        claimed: DescriptorState,
        derived: DescriptorState,
    },
    /// The claimed recovery is not the derived one — the direction that
    /// licenses the retry the platform documents as a hazard.
    Recovery { claimed: bool, derived: bool },
    /// The claimed discharge is not the derived one.
    Discharge { claimed: bool, derived: bool },
}

/// The accepted record together with the post-state the relation derived for
/// it. There is no other constructor, so an accepted record cannot exist
/// without the relation having agreed with every field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CloseRecordCertificate {
    pub checked: CloseRecord,
    pub derived: PostState,
}

/// Admit a declared close record only when the target-qualified relation
/// derives exactly what it claims.
///
/// The custody obligation is discharged exactly when the relation derives
/// [`DescriptorState::Released`]; a poisoned or retained epoch discharges
/// nothing. Durability is never discharged.
///
/// # Errors
///
/// [`CloseRecordRefusal`] — the record is refused, never repaired.
pub fn check_close_record(
    record: &CloseRecord,
) -> Result<CloseRecordCertificate, CloseRecordRefusal> {
    if !record.live_before || !record.premises.epoch_live {
        return Err(CloseRecordRefusal::NoLiveEpochBefore);
    }
    if record.data_duty_discharged {
        return Err(CloseRecordRefusal::ClaimsDataDuty);
    }
    let derived = close_poststate(record.target, record.premises, record.outcome)
        .map_err(CloseRecordRefusal::OutsideRelationDomain)?;
    if record.descriptor_after != derived.descriptor {
        return Err(CloseRecordRefusal::Descriptor {
            claimed: record.descriptor_after,
            derived: derived.descriptor,
        });
    }
    let derived_recovery = derived.permitted_recovery.contains(&Recovery::Close);
    if record.may_retry_close != derived_recovery {
        return Err(CloseRecordRefusal::Recovery {
            claimed: record.may_retry_close,
            derived: derived_recovery,
        });
    }
    let derived_discharge = derived.descriptor == DescriptorState::Released;
    if record.custody_discharged != derived_discharge {
        return Err(CloseRecordRefusal::Discharge {
            claimed: record.custody_discharged,
            derived: derived_discharge,
        });
    }
    Ok(CloseRecordCertificate {
        checked: record.clone(),
        derived,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn linux(outcome: Outcome) -> PostState {
        close_poststate(Target::Linux, Premises::default(), outcome).expect("in domain")
    }

    fn with(target: Target, premises: Premises, outcome: Outcome) -> PostState {
        close_poststate(target, premises, outcome).expect("in domain")
    }

    /// Every `(target, premises, outcome)` triple the relation accepts.
    fn domain() -> Vec<(Target, Premises, Outcome)> {
        let mut out = Vec::new();
        for target in Target::ALL {
            for premises in Premises::all() {
                for outcome in Outcome::all() {
                    if outcome == Outcome::NotAttempted && !premises.epoch_live {
                        continue; // the one refused input; covered separately
                    }
                    out.push((target, premises, outcome));
                }
            }
        }
        out
    }

    // ---------------------------------------------------------------
    // Taxonomy: acceptance item 1
    // ---------------------------------------------------------------

    #[test]
    fn all_five_descriptor_states_are_reachable() {
        let mut reached = BTreeSet::new();
        for (target, premises, outcome) in domain() {
            reached.insert(with(target, premises, outcome).descriptor);
        }
        let expected: BTreeSet<DescriptorState> = DescriptorState::ALL.into_iter().collect();
        assert_eq!(
            reached, expected,
            "the five-way taxonomy must not be decorative"
        );
    }

    #[test]
    fn not_attempted_is_distinct_from_attempted() {
        let not_attempted = linux(Outcome::NotAttempted);
        let attempted = linux(Outcome::ResultUnobserved);
        assert_eq!(not_attempted.descriptor, DescriptorState::NotAttempted);
        assert_eq!(attempted.descriptor, DescriptorState::Attempted);
        // Not-attempted still owes a release and may still close; attempted
        // owes the conservative residual and may do nothing.
        assert!(not_attempted.duties.contains(&Duty::ReleaseEpoch));
        assert!(not_attempted.permitted_recovery.contains(&Recovery::Close));
        assert!(!attempted.duties.contains(&Duty::ReleaseEpoch));
        assert!(attempted.permitted_recovery.is_empty());
    }

    #[test]
    fn attempted_and_indeterminate_share_duties_but_not_identity() {
        let attempted = linux(Outcome::ResultUnobserved);
        let indeterminate = with(
            Target::Unpinned,
            Premises::default(),
            Outcome::MinusOne(Errno::Eintr),
        );
        assert_eq!(attempted.descriptor, DescriptorState::Attempted);
        assert_eq!(indeterminate.descriptor, DescriptorState::Indeterminate);
        for duty in poisoned_duties() {
            assert!(attempted.duties.contains(&duty));
            assert!(indeterminate.duties.contains(&duty));
        }
    }

    // ---------------------------------------------------------------
    // The Linux arms
    // ---------------------------------------------------------------

    #[test]
    fn success_releases_the_descriptor() {
        assert_eq!(linux(Outcome::Zero).descriptor, DescriptorState::Released);
    }

    #[test]
    fn eintr_releases_the_descriptor_on_linux() {
        let state = linux(Outcome::MinusOne(Errno::Eintr));
        assert_eq!(state.descriptor, DescriptorState::Released);
        assert_eq!(state.writeback, WriteBackState::Failed);
    }

    #[test]
    fn deferred_io_errors_release_the_descriptor_on_linux() {
        for errno in [Errno::Eio, Errno::Enospc, Errno::Edquot] {
            let state = linux(Outcome::MinusOne(errno));
            assert_eq!(state.descriptor, DescriptorState::Released, "{errno:?}");
            assert_eq!(state.writeback, WriteBackState::Failed, "{errno:?}");
            assert!(state.duties.contains(&Duty::ReportPossibleDataLoss));
        }
    }

    #[test]
    fn ebadf_is_the_one_arm_that_releases_nothing() {
        let state = linux(Outcome::MinusOne(Errno::Ebadf));
        assert_eq!(state.descriptor, DescriptorState::Indeterminate);
        assert!(state.premise_violations.contains(&Premise::EpochLive));
    }

    #[test]
    fn an_unobserved_errno_cannot_conclude_a_release() {
        assert_eq!(
            linux(Outcome::MinusOne(Errno::Unobserved)).descriptor,
            DescriptorState::Attempted
        );
    }

    #[test]
    fn record_locks_deny_the_whole_relation() {
        let premises = Premises {
            no_record_locks: false,
            ..Premises::default()
        };
        let state = with(Target::Linux, premises, Outcome::Zero);
        assert_eq!(state.descriptor, DescriptorState::Indeterminate);
        assert!(state.premise_violations.contains(&Premise::NoRecordLocks));
    }

    // ---------------------------------------------------------------
    // Standing invariants, checked over the whole domain
    // ---------------------------------------------------------------

    #[test]
    fn no_outcome_on_any_target_ever_discharges_the_data_duty() {
        for (target, premises, outcome) in domain() {
            assert_ne!(
                with(target, premises, outcome).writeback,
                WriteBackState::Discharged,
                "{target:?} {outcome:?}"
            );
        }
    }

    #[test]
    fn no_poisoned_state_anywhere_permits_a_retry() {
        for (target, premises, outcome) in domain() {
            let state = with(target, premises, outcome);
            if state.is_poisoned() {
                assert!(
                    state.permitted_recovery.is_empty(),
                    "{target:?} {outcome:?} permitted {:?}",
                    state.permitted_recovery
                );
                for duty in poisoned_duties() {
                    assert!(state.duties.contains(&duty), "{target:?} {outcome:?}");
                }
            }
        }
    }

    #[test]
    fn a_released_epoch_never_permits_an_operation_on_its_own_generation() {
        for (target, premises, outcome) in domain() {
            let state = with(target, premises, outcome);
            if state.descriptor == DescriptorState::Released {
                assert!(
                    state.permitted_recovery.is_empty(),
                    "{target:?} {outcome:?}"
                );
            }
        }
    }

    #[test]
    fn only_a_live_epoch_may_be_closed() {
        for (target, premises, outcome) in domain() {
            let state = with(target, premises, outcome);
            if state.permitted_recovery.contains(&Recovery::Close) {
                assert!(
                    matches!(
                        state.descriptor,
                        DescriptorState::NotAttempted | DescriptorState::Retained
                    ),
                    "{target:?} {outcome:?} permits close from {:?}",
                    state.descriptor
                );
            }
        }
    }

    #[test]
    fn the_description_is_released_only_with_the_last_reference() {
        for (target, premises, outcome) in domain() {
            let state = with(target, premises, outcome);
            if state.description == DescriptionState::Released {
                assert!(premises.last_reference, "{target:?} {outcome:?}");
            }
        }
    }

    #[test]
    fn a_not_attempted_arm_needs_an_epoch_to_describe() {
        let premises = Premises {
            epoch_live: false,
            ..Premises::default()
        };
        assert_eq!(
            close_poststate(Target::Linux, premises, Outcome::NotAttempted),
            Err(RelationError::NoEpochToDescribe)
        );
    }

    // ---------------------------------------------------------------
    // Target divergence
    // ---------------------------------------------------------------

    #[test]
    fn posix2024_retains_the_descriptor_on_eintr() {
        let state = with(
            Target::Posix2024,
            Premises::default(),
            Outcome::MinusOne(Errno::Eintr),
        );
        assert_eq!(state.descriptor, DescriptorState::Retained);
        assert!(state.permitted_recovery.contains(&Recovery::Close));
    }

    #[test]
    fn the_two_targets_prescribe_opposite_recovery_on_eintr() {
        let on_linux = linux(Outcome::MinusOne(Errno::Eintr));
        let on_posix = with(
            Target::Posix2024,
            Premises::default(),
            Outcome::MinusOne(Errno::Eintr),
        );
        assert!(!on_linux.permitted_recovery.contains(&Recovery::Close));
        assert!(on_posix.permitted_recovery.contains(&Recovery::Close));
    }

    #[test]
    fn an_unpinned_target_is_indeterminate_and_permits_no_retry() {
        // Acceptance item 3: no automatic retry is prescribed for an
        // indeterminate descriptor, precisely because one target requires the
        // retry the other forbids.
        let state = with(
            Target::Unpinned,
            Premises::default(),
            Outcome::MinusOne(Errno::Eintr),
        );
        assert_eq!(state.descriptor, DescriptorState::Indeterminate);
        assert!(state.permitted_recovery.is_empty());
        assert!(state.duties.contains(&Duty::NoRetryClose));
        assert!(state.duties.contains(&Duty::NoUse));
    }

    // ---------------------------------------------------------------
    // Forged post-states: the twelve refusals, and one admission
    // ---------------------------------------------------------------

    fn refused(target: Target, premises: Premises, outcome: Outcome, claim: PostStateClaim) {
        let refusal = check_claim(target, premises, outcome, &claim).expect("in domain");
        assert!(
            refusal.is_some(),
            "claim {claim:?} was admitted for {target:?} {outcome:?} and should not be"
        );
    }

    fn descriptor_claim(state: DescriptorState) -> PostStateClaim {
        PostStateClaim {
            descriptor: Some(state),
            ..PostStateClaim::default()
        }
    }

    #[test]
    fn claiming_eintr_retains_on_linux_is_refused() {
        refused(
            Target::Linux,
            Premises::default(),
            Outcome::MinusOne(Errno::Eintr),
            descriptor_claim(DescriptorState::Retained),
        );
    }

    #[test]
    fn claiming_eintr_releases_on_posix2024_is_refused() {
        refused(
            Target::Posix2024,
            Premises::default(),
            Outcome::MinusOne(Errno::Eintr),
            descriptor_claim(DescriptorState::Released),
        );
    }

    #[test]
    fn claiming_a_retry_is_permitted_after_a_failed_linux_close_is_refused() {
        refused(
            Target::Linux,
            Premises::default(),
            Outcome::MinusOne(Errno::Eio),
            PostStateClaim {
                permitted_recovery: Some([Recovery::Close].into_iter().collect()),
                ..PostStateClaim::default()
            },
        );
    }

    #[test]
    fn claiming_a_retry_is_permitted_from_an_indeterminate_state_is_refused() {
        refused(
            Target::Unpinned,
            Premises::default(),
            Outcome::MinusOne(Errno::Eintr),
            PostStateClaim {
                permitted_recovery: Some([Recovery::Close].into_iter().collect()),
                ..PostStateClaim::default()
            },
        );
    }

    #[test]
    fn claiming_ebadf_released_the_epoch_is_refused() {
        refused(
            Target::Linux,
            Premises::default(),
            Outcome::MinusOne(Errno::Ebadf),
            descriptor_claim(DescriptorState::Released),
        );
    }

    #[test]
    fn claiming_an_unobserved_result_released_the_epoch_is_refused() {
        refused(
            Target::Linux,
            Premises {
                result_observed: false,
                ..Premises::default()
            },
            Outcome::ResultUnobserved,
            descriptor_claim(DescriptorState::Released),
        );
    }

    #[test]
    fn claiming_success_discharged_the_data_duty_is_refused() {
        refused(
            Target::Linux,
            Premises::default(),
            Outcome::Zero,
            PostStateClaim {
                writeback: Some(WriteBackState::Discharged),
                ..PostStateClaim::default()
            },
        );
    }

    #[test]
    fn claiming_a_failed_close_discharged_the_data_duty_is_refused() {
        refused(
            Target::Linux,
            Premises::default(),
            Outcome::MinusOne(Errno::Eio),
            PostStateClaim {
                writeback: Some(WriteBackState::Discharged),
                ..PostStateClaim::default()
            },
        );
    }

    #[test]
    fn claiming_the_description_died_without_the_last_reference_is_refused() {
        refused(
            Target::Linux,
            Premises {
                last_reference: false,
                ..Premises::default()
            },
            Outcome::Zero,
            PostStateClaim {
                description: Some(DescriptionState::Released),
                ..PostStateClaim::default()
            },
        );
    }

    #[test]
    fn claiming_success_identifies_the_epoch_without_sole_custody_is_refused() {
        refused(
            Target::Linux,
            Premises {
                sole_custodian: false,
                ..Premises::default()
            },
            Outcome::Zero,
            descriptor_claim(DescriptorState::Released),
        );
    }

    #[test]
    fn claiming_a_poisoned_epoch_leaves_the_denominator_is_refused() {
        refused(
            Target::Unpinned,
            Premises::default(),
            Outcome::MinusOne(Errno::Eintr),
            PostStateClaim {
                duties: Some(BTreeSet::new()),
                ..PostStateClaim::default()
            },
        );
    }

    /// The Python matrix's twelfth refusal was an unknown post-state *field*,
    /// which the Rust vocabulary makes unrepresentable: a claim naming a field
    /// or a state that does not exist fails to compile. The equivalently
    /// motivated run-time confusion is treating the analysis gap as a live
    /// epoch, which is the one that would license a retry.
    #[test]
    fn claiming_an_attempted_epoch_is_retained_is_refused() {
        refused(
            Target::Linux,
            Premises::default(),
            Outcome::ResultUnobserved,
            descriptor_claim(DescriptorState::Retained),
        );
    }

    #[test]
    fn the_true_post_state_is_admitted() {
        // The refusals above are not vacuous: the derived answer is accepted.
        let admitted = check_claim(
            Target::Linux,
            Premises::default(),
            Outcome::MinusOne(Errno::Eio),
            &PostStateClaim {
                descriptor: Some(DescriptorState::Released),
                writeback: Some(WriteBackState::Failed),
                permitted_recovery: Some(BTreeSet::new()),
                ..PostStateClaim::default()
            },
        )
        .expect("in domain");
        assert_eq!(admitted, None);
    }

    #[test]
    fn an_empty_claim_is_admitted_and_proves_nothing() {
        for (target, premises, outcome) in domain() {
            assert_eq!(
                check_claim(target, premises, outcome, &PostStateClaim::default())
                    .expect("in domain"),
                None
            );
        }
    }

    // ---------------------------------------------------------------
    // Epoch identity: acceptance item 2
    // ---------------------------------------------------------------

    #[test]
    fn a_reused_number_is_a_distinct_epoch() {
        let first = Epoch::new(EndpointKind::Descriptor, 7, 1);
        assert_ne!(first, first.successor());
        assert_eq!(first, Epoch::new(EndpointKind::Descriptor, 7, 1));
        assert_eq!(first.successor().number, first.number);
    }

    // ---------------------------------------------------------------
    // Joins
    // ---------------------------------------------------------------

    #[test]
    fn disagreeing_paths_widen_to_indeterminate_with_no_recovery() {
        let joined =
            join(&linux(Outcome::Zero), &linux(Outcome::NotAttempted)).expect("same target");
        assert_eq!(joined.descriptor, DescriptorState::Indeterminate);
        assert!(joined.permitted_recovery.is_empty());
        assert!(joined.duties.contains(&Duty::NoRetryClose));
        assert!(!joined.duties.contains(&Duty::ReleaseEpoch));
    }

    #[test]
    fn agreeing_paths_are_preserved_exactly() {
        let joined = join(&linux(Outcome::Zero), &linux(Outcome::MinusOne(Errno::Eio)))
            .expect("same target");
        assert_eq!(joined.descriptor, DescriptorState::Released);
        assert_eq!(joined.writeback, WriteBackState::Failed);
    }

    #[test]
    fn a_join_never_invents_a_permitted_recovery() {
        let both = join(&linux(Outcome::NotAttempted), &linux(Outcome::NotAttempted))
            .expect("same target");
        assert_eq!(
            both.permitted_recovery,
            [Recovery::Close].into_iter().collect::<BTreeSet<_>>()
        );
        let mixed =
            join(&linux(Outcome::NotAttempted), &linux(Outcome::Zero)).expect("same target");
        assert!(mixed.permitted_recovery.is_empty());
    }

    #[test]
    fn a_join_across_targets_is_refused() {
        let on_linux = linux(Outcome::Zero);
        let on_posix = with(Target::Posix2024, Premises::default(), Outcome::Zero);
        assert_eq!(
            join(&on_linux, &on_posix),
            Err(RelationError::MismatchedTargets)
        );
    }

    #[test]
    fn a_join_never_permits_what_neither_side_permitted() {
        let states: Vec<PostState> = domain()
            .into_iter()
            .filter(|(target, _, _)| *target == Target::Linux)
            .map(|(target, premises, outcome)| with(target, premises, outcome))
            .collect();
        for left in &states {
            for right in &states {
                let joined = join(left, right).expect("same target");
                for recovery in &joined.permitted_recovery {
                    assert!(
                        left.permitted_recovery.contains(recovery)
                            && right.permitted_recovery.contains(recovery)
                    );
                }
            }
        }
    }

    // ---------------------------------------------------------------
    // The distinction the card exists to draw
    // ---------------------------------------------------------------

    /// A wrapper that returns `-1`/EIO without reaching the kernel necessarily
    /// leaves the descriptor open. That is the POSIX.1-2024 post-state, not the
    /// Linux one. Reading the project's suppressed-syscall fixtures as evidence
    /// about real Linux close failures would invert the answer.
    #[test]
    fn the_suppressed_wrapper_post_state_is_not_the_linux_post_state() {
        let suppressed_observation = DescriptorState::Retained;
        assert_eq!(
            linux(Outcome::MinusOne(Errno::Eio)).descriptor,
            DescriptorState::Released
        );
        assert_ne!(
            linux(Outcome::MinusOne(Errno::Eio)).descriptor,
            suppressed_observation
        );
        assert_eq!(
            with(
                Target::Posix2024,
                Premises::default(),
                Outcome::MinusOne(Errno::Eintr)
            )
            .descriptor,
            suppressed_observation
        );
    }

    // ---------------------------------------------------------------
    // The declared close record (card O03-U4)
    //
    // These mirror, test for theorem, the corrected `QCore.DescriptorCustody`
    // admission rule. The historical Lean model hard-coded "a failed close
    // retains the descriptor"; on this target that both rejects a truthful
    // record of a real failure and models the epoch as usable, which licenses
    // the retry the platform documents as a hazard.
    // ---------------------------------------------------------------

    fn record_from(
        target: Target,
        premises: Premises,
        outcome: Outcome,
        derived: &PostState,
    ) -> CloseRecord {
        CloseRecord {
            target,
            premises,
            outcome,
            live_before: true,
            descriptor_after: derived.descriptor,
            may_retry_close: derived.permitted_recovery.contains(&Recovery::Close),
            custody_discharged: derived.descriptor == DescriptorState::Released,
            data_duty_discharged: false,
        }
    }

    fn record(target: Target, outcome: Outcome) -> CloseRecord {
        let premises = Premises::default();
        let derived = with(target, premises, outcome);
        record_from(target, premises, outcome, &derived)
    }

    /// The old model's second disjunct, as a record: a Linux `EIO` failure
    /// claiming the descriptor is still live and nothing was discharged. The
    /// old `Close.Valid` accepted exactly this shape; the relation refuses it.
    #[test]
    fn the_returns_on_failure_record_is_refused() {
        let mut forged = record(Target::Linux, Outcome::MinusOne(Errno::Eio));
        forged.descriptor_after = DescriptorState::Retained;
        forged.custody_discharged = false;
        assert_eq!(
            check_close_record(&forged),
            Err(CloseRecordRefusal::Descriptor {
                claimed: DescriptorState::Retained,
                derived: DescriptorState::Released,
            })
        );
    }

    /// The other direction of the same defect: a truthful record of a real
    /// Linux failure, which the old model rejected outright.
    #[test]
    fn the_truthful_linux_failure_record_is_accepted() {
        for errno in [Errno::Eintr, Errno::Eio, Errno::Enospc, Errno::Edquot] {
            let truthful = record(Target::Linux, Outcome::MinusOne(errno));
            assert_eq!(truthful.descriptor_after, DescriptorState::Released);
            assert!(!truthful.may_retry_close);
            let certificate = check_close_record(&truthful).expect("truthful record");
            assert_eq!(certificate.derived.writeback, WriteBackState::Failed);
            assert!(
                certificate
                    .derived
                    .duties
                    .contains(&Duty::ReportPossibleDataLoss)
            );
        }
    }

    /// The hazard the old model licensed: a retry after a failed Linux close.
    #[test]
    fn no_accepted_record_permits_a_retry_after_a_failed_linux_close() {
        for errno in [
            Errno::Eintr,
            Errno::Eio,
            Errno::Enospc,
            Errno::Edquot,
            Errno::Ebadf,
            Errno::Other,
            Errno::Unobserved,
        ] {
            let mut forged = record(Target::Linux, Outcome::MinusOne(errno));
            assert!(!forged.may_retry_close, "{errno:?}");
            forged.may_retry_close = true;
            assert_eq!(
                check_close_record(&forged),
                Err(CloseRecordRefusal::Recovery {
                    claimed: true,
                    derived: false,
                }),
                "{errno:?}"
            );
        }
    }

    /// Retention on a failure is admitted **only** where the documentation says
    /// it happens: POSIX.1-2024 `EINTR`. That is the corrected content of the
    /// historical `accepted_failed_close_retains_custody`.
    #[test]
    fn a_failed_close_retains_the_descriptor_only_on_posix2024_eintr() {
        let mut retaining = Vec::new();
        for (target, premises, outcome) in domain() {
            if !matches!(outcome, Outcome::MinusOne(_)) {
                continue;
            }
            if with(target, premises, outcome).descriptor == DescriptorState::Retained {
                retaining.push((target, outcome));
            }
        }
        assert!(!retaining.is_empty(), "the arm must not be vacuous");
        for (target, outcome) in retaining {
            assert_eq!(target, Target::Posix2024);
            assert_eq!(outcome, Outcome::MinusOne(Errno::Eintr));
        }
    }

    /// With no target pinned, every failed close is poisoned, discharges
    /// nothing and permits nothing. This is the disposition the Cycle 18
    /// reactor vocabulary — which carries neither target nor errno — must take.
    #[test]
    fn an_unpinned_failed_close_is_poisoned_and_permits_no_recovery() {
        for errno in [
            Errno::Ebadf,
            Errno::Eintr,
            Errno::Eio,
            Errno::Enospc,
            Errno::Edquot,
            Errno::Other,
            Errno::Unobserved,
        ] {
            let accepted = record(Target::Unpinned, Outcome::MinusOne(errno));
            assert!(accepted.descriptor_after.is_poisoned(), "{errno:?}");
            assert!(!accepted.may_retry_close, "{errno:?}");
            assert!(!accepted.custody_discharged, "{errno:?}");
            check_close_record(&accepted).expect("truthful record");
        }
    }

    /// No record may claim durability, on any target, under any outcome; and a
    /// record that does not assert a live epoch before the call is refused
    /// before the relation is consulted.
    #[test]
    fn no_accepted_record_discharges_the_data_duty() {
        for (target, premises, outcome) in domain() {
            let derived = with(target, premises, outcome);
            assert_ne!(derived.writeback, WriteBackState::Discharged);
            let mut forged = record_from(target, premises, outcome, &derived);
            forged.data_duty_discharged = true;
            let expected = if premises.epoch_live {
                CloseRecordRefusal::ClaimsDataDuty
            } else {
                CloseRecordRefusal::NoLiveEpochBefore
            };
            assert_eq!(check_close_record(&forged), Err(expected));
        }
    }

    /// A successful close identifies the epoch it closed only under sole
    /// custody. Without it the record cannot claim a discharge.
    #[test]
    fn a_successful_close_without_sole_custody_is_not_a_discharge() {
        let premises = Premises {
            sole_custodian: false,
            ..Premises::default()
        };
        assert_eq!(
            with(Target::Linux, premises, Outcome::Zero).descriptor,
            DescriptorState::Indeterminate
        );
        let forged = CloseRecord {
            target: Target::Linux,
            premises,
            outcome: Outcome::Zero,
            live_before: true,
            descriptor_after: DescriptorState::Released,
            may_retry_close: false,
            custody_discharged: true,
            data_duty_discharged: false,
        };
        assert_eq!(
            check_close_record(&forged),
            Err(CloseRecordRefusal::Descriptor {
                claimed: DescriptorState::Released,
                derived: DescriptorState::Indeterminate,
            })
        );
    }

    /// Discharge is exactly release: a poisoned or retained epoch still owes
    /// its obligation, on every target and every outcome.
    #[test]
    fn custody_is_discharged_exactly_when_the_epoch_is_released() {
        for (target, premises, outcome) in domain() {
            if !premises.epoch_live {
                continue;
            }
            let derived = with(target, premises, outcome);
            let truthful = record_from(target, premises, outcome, &derived);
            check_close_record(&truthful).expect("truthful record");
            let mut forged = truthful;
            forged.custody_discharged = !forged.custody_discharged;
            assert!(matches!(
                check_close_record(&forged),
                Err(CloseRecordRefusal::Discharge { .. })
            ));
        }
    }
}

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

//! The file-backed **capacity** boundary: one grow family, decided (card O02).
//!
//! This module decides the post-state of **one** boundary family from the
//! Cycle 19 failure matrix: growing a file-backed chunk, and the kernel
//! refusing that growth for want of capacity. It also validates a recorded
//! native trace of that boundary against the relation, so a recorded
//! observation cannot claim more than the calls it made.
//!
//! The evidence record is `docs/reports/remaining-o02/README.md`.
//!
//! # Why capacity, and not metadata
//!
//! Both were open. Capacity was chosen because it is the family in which a
//! *single* success bit is provably not enough:
//!
//! * a grow call may change the **file length** without reserving any
//!   **backing store**, so the bytes it "added" fault at first store rather
//!   than at the call — the `SIGBUS` hazard ChunkIO's own resize path
//!   documents in a comment;
//! * the **mapping** may be longer than the file, so a live mapping epoch is
//!   not a witness that the capacity behind it exists;
//! * one of the three grow calls (`posix_fallocate`) reports its error as a
//!   **return value and does not set `errno`**, so "read `errno`" is a
//!   premise violation there rather than a detail;
//! * a `write` at the boundary may **transfer part** of its request, so the
//!   post-state is neither "granted" nor "refused".
//!
//! Metadata/chmod-class boundaries have two recorded Cycle 19 observations
//! already and carry no epoch structure of their own, so they could not
//! satisfy this card's first acceptance item (epochs, transitions, retries and
//! teardown). Only capacity is treated here; the two families are deliberately
//! not combined, and nothing here is exhaustive over either.
//!
//! # The four separated quantities
//!
//! | Quantity | Changed by | Never established by |
//! |---|---|---|
//! | file **length** `S` | `ftruncate`, `fallocate`, `posix_fallocate`, `write` | a mapping |
//! | reserved **backing** `B` | allocating calls only | `ftruncate` |
//! | **mapping** epoch `(generation, address, length)` | `mmap`/`mremap`/`munmap` | a length change |
//! | **logical** content length `L` | the caller's own writes | any grow call |
//!
//! A grow call adds capacity, never content: every granted grow advances `L`
//! by exactly zero. Only an observed transfer advances it, and then by exactly
//! the transferred amount.
//!
//! # Fail-closed
//!
//! Every arm the cited documentation or the recorded observation does not
//! determine yields [`Backing::Unknown`] or [`FileLength::Indeterminate`] with
//! the conservative duty set, and no arm ever discharges the store duty or
//! admits an automatic retry of an unobserved outcome.
//!
//! # What this is not
//!
//! It is not a durability result, not a filesystem model, not a statement
//! about any pinned Cycle 19 unit, and no production audit route consumes it.
//! `ENOSPC` and `EDQUOT` are *classified* here and were not observed: the
//! recorded native evidence observes the same capacity class through
//! `RLIMIT_FSIZE`/`EFBIG`, which is a kernel refusal rather than a suppressed
//! wrapper. Nothing here is promoted.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

/// The platform whose documented semantics the relation is read against.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Target {
    /// A Linux ABI target, read against Linux man-pages 6.17.
    Linux,
    /// No target pinned. Every arm the targets could disagree on is
    /// indeterminate.
    Unpinned,
}

impl Target {
    /// Every target, for exhaustive domain checks.
    pub const ALL: [Target; 2] = [Target::Linux, Target::Unpinned];
}

/// The call that was used to grow the file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GrowCall {
    /// `ftruncate(2)`. Changes the length; reserves nothing.
    Ftruncate,
    /// `fallocate(2)`. Reserves blocks and reports through `errno`.
    Fallocate,
    /// `posix_fallocate(3)`. Reserves blocks and reports the error **as its
    /// return value**; `errno` is not set.
    PosixFallocate,
    /// A `write`/`pwrite` that extends the file. May transfer part of its
    /// request.
    Write,
}

impl GrowCall {
    /// Every call, for exhaustive domain checks.
    pub const ALL: [GrowCall; 4] = [
        GrowCall::Ftruncate,
        GrowCall::Fallocate,
        GrowCall::PosixFallocate,
        GrowCall::Write,
    ];

    /// Whether a *successful* call reserves backing store for the range it
    /// covers. `ftruncate` never does: the added range is a hole.
    pub fn reserves_backing(self) -> bool {
        !matches!(self, GrowCall::Ftruncate)
    }

    /// Whether a failure of this call reports through `errno`.
    pub fn reports_through_errno(self) -> bool {
        !matches!(self, GrowCall::PosixFallocate)
    }

    /// Whether this call can transfer part of its request.
    pub fn may_transfer_partially(self) -> bool {
        matches!(self, GrowCall::Write)
    }

    /// Whether a *failed* call may nevertheless have changed the file length.
    /// `ftruncate(2)` documents no change; the allocating calls leave a
    /// partial effect unspecified, so a length change there is admitted and
    /// carries [`Backing::Unknown`].
    pub fn failure_may_change_length(self) -> bool {
        !matches!(self, GrowCall::Ftruncate)
    }
}

/// The error observed on a failing grow call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CapacityErrno {
    /// The device is full.
    Enospc,
    /// The quota is exhausted.
    Edquot,
    /// The request exceeds a maximum file size — including the one
    /// `RLIMIT_FSIZE` imposes.
    Efbig,
    /// A signal was caught during the call.
    Eintr,
    /// A deferred write-back error surfaced here.
    Eio,
    /// Some other observed error.
    Other,
    /// The failure branch did not observe the error.
    Unobserved,
}

impl CapacityErrno {
    /// Every error, for exhaustive domain checks.
    pub const ALL: [CapacityErrno; 7] = [
        CapacityErrno::Enospc,
        CapacityErrno::Edquot,
        CapacityErrno::Efbig,
        CapacityErrno::Eintr,
        CapacityErrno::Eio,
        CapacityErrno::Other,
        CapacityErrno::Unobserved,
    ];

    /// The capacity class: the errors that say "the space is not available",
    /// and which a retry cannot resolve on its own.
    pub fn is_capacity_class(self) -> bool {
        matches!(
            self,
            CapacityErrno::Enospc | CapacityErrno::Edquot | CapacityErrno::Efbig
        )
    }

    /// Decode a Linux `errno` number into the classification. Unknown numbers
    /// are [`CapacityErrno::Other`], never silently a capacity error.
    pub fn from_linux_number(number: i64) -> CapacityErrno {
        match number {
            4 => CapacityErrno::Eintr,
            5 => CapacityErrno::Eio,
            27 => CapacityErrno::Efbig,
            28 => CapacityErrno::Enospc,
            122 => CapacityErrno::Edquot,
            _ => CapacityErrno::Other,
        }
    }
}

/// What the call was observed to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GrowOutcome {
    /// No call was issued on this path.
    NotAttempted,
    /// A call was issued and its result was not observed.
    ResultUnobserved,
    /// The call reported success.
    Granted,
    /// The call transferred part of its request.
    Partial {
        /// Bytes actually transferred, as the call reported them.
        transferred: u64,
    },
    /// The call reported failure.
    Refused(CapacityErrno),
}

/// The measurements taken around one call. Every field is an observation; none
/// is inferred by the relation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GrowObservation {
    /// The call issued.
    pub call: GrowCall,
    /// The file length the call was asked to reach.
    pub request_len: u64,
    /// `st_size` before the call.
    pub prior_len: u64,
    /// `st_size` after the call, when it was re-read.
    pub observed_len: u64,
    /// `st_blocks` before the call, in 512-byte units.
    pub prior_blocks: u64,
    /// `st_blocks` after the call, in 512-byte units.
    pub observed_blocks: u64,
    /// The length of the live mapping over the file at the time of the call,
    /// if any.
    pub mapping_len: Option<u64>,
}

/// Pre-state facts the relation consumes and never establishes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Premises {
    /// P1 — the call's result was bound and tested.
    pub result_observed: bool,
    /// P2 — `errno` was read on the failure branch, unclobbered.
    pub errno_observed: bool,
    /// P3 — `st_size` was re-read after the call.
    pub length_observed: bool,
    /// P4 — `st_blocks` was re-read after the call.
    pub blocks_observed: bool,
    /// P5 — no other writer or truncator touched the file across the call.
    pub sole_writer: bool,
    /// P6 — `SIGXFSZ` is not left at its default disposition, which would
    /// terminate the process at the boundary and run no teardown at all.
    pub signal_handled: bool,
}

impl Premises {
    /// Every premise holds and the outcome was fully observed.
    pub fn fully_observed() -> Premises {
        Premises {
            result_observed: true,
            errno_observed: true,
            length_observed: true,
            blocks_observed: true,
            sole_writer: true,
            signal_handled: true,
        }
    }
}

/// The post-state of the file length.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileLength {
    /// The length is known to be what it was.
    Unchanged(u64),
    /// The length is known to be this value.
    GrownTo(u64),
    /// The length is not determined by this observation.
    Indeterminate,
}

/// The post-state of the reserved backing store.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backing {
    /// Backing is reserved for the first this-many bytes.
    Reserved(u64),
    /// No backing was reserved by this call.
    NotReserved,
    /// Whether backing was reserved is not determined.
    Unknown,
}

/// What the caller must do with a live mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MappingDuty {
    /// There was no live mapping.
    None,
    /// The mapping is shorter than the file now is and must be remapped
    /// before the new range is used.
    RemapBeforeUse {
        /// The length the mapping must reach.
        to_len: u64,
    },
    /// The mapping is untouched and stays exactly as it was.
    RetainExisting,
    /// The mapping's relation to the file is not determined.
    Poisoned,
}

/// Residual obligations carried out of the call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Duties {
    /// The mapping must be remapped before the new range is touched.
    pub remap_before_use: bool,
    /// Stores past the reserved range may fault; the range is not writable
    /// authority.
    pub no_store_beyond_reserved: bool,
    /// The logical content length may not advance on this path.
    pub keep_logical_length: bool,
    /// The length must be re-observed before anything is concluded.
    pub reobserve_length: bool,
    /// The acquired epochs are still owed a release.
    pub release_epochs: bool,
    /// The outcome is unresolved and stays in the denominator.
    pub report_unresolved: bool,
}

impl Duties {
    /// Nothing outstanding except the standing release duty.
    pub fn released_only() -> Duties {
        Duties {
            remap_before_use: false,
            no_store_beyond_reserved: false,
            keep_logical_length: false,
            reobserve_length: false,
            release_epochs: true,
            report_unresolved: false,
        }
    }

    /// The conservative set carried by every undetermined arm.
    pub fn poisoned() -> Duties {
        Duties {
            remap_before_use: false,
            no_store_beyond_reserved: true,
            keep_logical_length: true,
            reobserve_length: true,
            release_epochs: true,
            report_unresolved: true,
        }
    }
}

/// What the caller may do next.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Recovery {
    /// Nothing is admitted on this path.
    None,
    /// The identical request may be re-issued.
    RetrySameRequest,
    /// A request re-derived from the *observed* length may be issued.
    RetryFromObservedLength,
    /// The request may be re-issued only after the available capacity has
    /// changed; re-issuing it unchanged decides nothing new.
    RetryAfterCapacityChange,
}

/// A premise the observation contradicted or failed to supply.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PremiseViolation {
    /// P1 — the result was not observed.
    ResultUnobserved,
    /// P2 — the failure branch did not read `errno`.
    ErrnoUnobserved,
    /// `errno` was read from a call that does not set it.
    ErrnoReadFromReturningCall,
    /// P3 — the length was not re-observed.
    LengthUnobserved,
    /// P4 — the blocks were not re-observed.
    BlocksUnobserved,
    /// P5 — another writer could have acted across the call.
    ConcurrentWriter,
    /// P6 — `SIGXFSZ` was left at its default disposition.
    SignalUnhandled,
    /// The measured length contradicts the reported outcome.
    LengthContradictsOutcome,
    /// The measured blocks do not cover the range the grant claims.
    BlocksDoNotCoverGrant,
    /// A partial transfer was reported by a call that cannot transfer
    /// partially, or the transfer exceeds the request.
    ImpossibleTransfer,
}

/// The decided post-state of one grow call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrowPostState {
    /// The file length after the call.
    pub file_length: FileLength,
    /// The reserved backing after the call.
    pub backing: Backing,
    /// What the live mapping now owes.
    pub mapping: MappingDuty,
    /// Residual obligations.
    pub duties: Duties,
    /// What may be attempted next.
    pub recovery: Recovery,
    /// Bytes of *content* this call authorises the caller to count. A grow
    /// call always authorises zero; only a transfer authorises anything.
    pub logical_advance: u64,
    /// Premises that were violated. A non-empty set always accompanies a
    /// poisoned post-state.
    pub violations: BTreeSet<PremiseViolation>,
}

impl GrowPostState {
    /// Whether this post-state determines nothing and carries the conservative
    /// duties.
    pub fn is_poisoned(&self) -> bool {
        self.duties.report_unresolved
    }

    fn poisoned(violations: BTreeSet<PremiseViolation>) -> GrowPostState {
        GrowPostState {
            file_length: FileLength::Indeterminate,
            backing: Backing::Unknown,
            mapping: MappingDuty::Poisoned,
            duties: Duties::poisoned(),
            recovery: Recovery::None,
            logical_advance: 0,
            violations,
        }
    }
}

/// A request the relation refuses to answer at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationError {
    /// The request does not exceed the prior length, so it is not a grow.
    NotAGrowRequest,
    /// The observation's mapping length is zero, which is not a mapping.
    EmptyMapping,
}

impl fmt::Display for RelationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            RelationError::NotAGrowRequest => "the request does not exceed the prior length",
            RelationError::EmptyMapping => "a mapping of zero length is not a mapping",
        };
        formatter.write_str(text)
    }
}

/// Decide the post-state of one grow call.
///
/// Every arm that the observation or the documentation leaves open is
/// poisoned: [`FileLength::Indeterminate`], [`Backing::Unknown`],
/// [`MappingDuty::Poisoned`], the conservative duties and [`Recovery::None`].
pub fn grow_poststate(
    target: Target,
    premises: Premises,
    observation: GrowObservation,
    outcome: GrowOutcome,
) -> Result<GrowPostState, RelationError> {
    if observation.request_len <= observation.prior_len {
        return Err(RelationError::NotAGrowRequest);
    }
    if observation.mapping_len == Some(0) {
        return Err(RelationError::EmptyMapping);
    }

    let mut violations = BTreeSet::new();
    if !premises.sole_writer {
        violations.insert(PremiseViolation::ConcurrentWriter);
    }
    if !premises.signal_handled && observation.call == GrowCall::Write {
        // At the boundary the default disposition terminates the process, so
        // no teardown runs at all and no post-state is observable.
        violations.insert(PremiseViolation::SignalUnhandled);
    }
    if premises.errno_observed && !observation.call.reports_through_errno() {
        violations.insert(PremiseViolation::ErrnoReadFromReturningCall);
    }
    if target == Target::Unpinned {
        // The arms below are read against Linux documentation; without a
        // pinned target nothing is concluded.
        return Ok(GrowPostState::poisoned(violations));
    }
    if !violations.is_empty() {
        return Ok(GrowPostState::poisoned(violations));
    }

    let mapping_duty_on_growth = |to_len: u64| match observation.mapping_len {
        Some(len) if len < to_len => MappingDuty::RemapBeforeUse { to_len },
        Some(_) => MappingDuty::RetainExisting,
        None => MappingDuty::None,
    };

    let post = match outcome {
        GrowOutcome::NotAttempted => GrowPostState {
            file_length: FileLength::Unchanged(observation.prior_len),
            backing: Backing::Unknown,
            mapping: match observation.mapping_len {
                Some(_) => MappingDuty::RetainExisting,
                None => MappingDuty::None,
            },
            duties: Duties {
                remap_before_use: false,
                no_store_beyond_reserved: true,
                keep_logical_length: true,
                reobserve_length: false,
                release_epochs: true,
                report_unresolved: false,
            },
            recovery: Recovery::RetrySameRequest,
            logical_advance: 0,
            violations,
        },
        GrowOutcome::ResultUnobserved => {
            violations.insert(PremiseViolation::ResultUnobserved);
            GrowPostState::poisoned(violations)
        }
        GrowOutcome::Granted => {
            if !premises.result_observed {
                violations.insert(PremiseViolation::ResultUnobserved);
            }
            if !premises.length_observed {
                violations.insert(PremiseViolation::LengthUnobserved);
            } else if observation.observed_len < observation.request_len {
                violations.insert(PremiseViolation::LengthContradictsOutcome);
            }
            let reserves = observation.call.reserves_backing();
            if reserves && !premises.blocks_observed {
                violations.insert(PremiseViolation::BlocksUnobserved);
            }
            if reserves
                && premises.blocks_observed
                && observation.observed_blocks.saturating_mul(512) < observation.request_len
            {
                violations.insert(PremiseViolation::BlocksDoNotCoverGrant);
            }
            if !violations.is_empty() {
                GrowPostState::poisoned(violations)
            } else {
                let length = observation.observed_len;
                GrowPostState {
                    file_length: FileLength::GrownTo(length),
                    backing: if reserves {
                        Backing::Reserved(length)
                    } else {
                        Backing::NotReserved
                    },
                    mapping: mapping_duty_on_growth(length),
                    duties: Duties {
                        remap_before_use: matches!(
                            mapping_duty_on_growth(length),
                            MappingDuty::RemapBeforeUse { .. }
                        ),
                        // A length-only grant leaves a hole: the added range
                        // is not writable authority.
                        no_store_beyond_reserved: !reserves,
                        keep_logical_length: true,
                        reobserve_length: false,
                        release_epochs: true,
                        report_unresolved: false,
                    },
                    recovery: Recovery::None,
                    // Capacity is not content.
                    logical_advance: 0,
                    violations,
                }
            }
        }
        GrowOutcome::Partial { transferred } => {
            if !observation.call.may_transfer_partially()
                || transferred == 0
                || observation.prior_len.saturating_add(transferred) >= observation.request_len
            {
                violations.insert(PremiseViolation::ImpossibleTransfer);
            }
            if !premises.length_observed {
                violations.insert(PremiseViolation::LengthUnobserved);
            } else if observation.observed_len != observation.prior_len.saturating_add(transferred)
            {
                violations.insert(PremiseViolation::LengthContradictsOutcome);
            }
            if !violations.is_empty() {
                GrowPostState::poisoned(violations)
            } else {
                GrowPostState {
                    file_length: FileLength::GrownTo(observation.observed_len),
                    // The bytes that were transferred are backed; the rest of
                    // the request is not.
                    backing: Backing::Reserved(observation.observed_len),
                    mapping: mapping_duty_on_growth(observation.observed_len),
                    duties: Duties {
                        remap_before_use: matches!(
                            mapping_duty_on_growth(observation.observed_len),
                            MappingDuty::RemapBeforeUse { .. }
                        ),
                        no_store_beyond_reserved: true,
                        keep_logical_length: false,
                        reobserve_length: true,
                        release_epochs: true,
                        report_unresolved: false,
                    },
                    recovery: Recovery::RetryFromObservedLength,
                    logical_advance: transferred,
                    violations,
                }
            }
        }
        GrowOutcome::Refused(error) => {
            if !premises.result_observed {
                violations.insert(PremiseViolation::ResultUnobserved);
            }
            if error == CapacityErrno::Unobserved
                || (observation.call.reports_through_errno() && !premises.errno_observed)
            {
                violations.insert(PremiseViolation::ErrnoUnobserved);
            }
            let length = if !premises.length_observed {
                FileLength::Indeterminate
            } else if observation.observed_len == observation.prior_len {
                FileLength::Unchanged(observation.prior_len)
            } else if observation.call.failure_may_change_length() {
                FileLength::GrownTo(observation.observed_len)
            } else {
                violations.insert(PremiseViolation::LengthContradictsOutcome);
                FileLength::Indeterminate
            };
            if !violations.is_empty() {
                GrowPostState::poisoned(violations)
            } else {
                GrowPostState {
                    file_length: length,
                    // `ftruncate` reserves nothing even when it succeeds; the
                    // allocating calls leave a partial effect unspecified.
                    backing: if observation.call.reserves_backing() {
                        Backing::Unknown
                    } else {
                        Backing::NotReserved
                    },
                    mapping: MappingDuty::RetainExisting,
                    duties: Duties {
                        remap_before_use: false,
                        no_store_beyond_reserved: true,
                        keep_logical_length: true,
                        reobserve_length: observation.call.reserves_backing(),
                        release_epochs: true,
                        report_unresolved: false,
                    },
                    recovery: match error {
                        e if e.is_capacity_class() => Recovery::RetryAfterCapacityChange,
                        CapacityErrno::Eintr => Recovery::RetryFromObservedLength,
                        _ => Recovery::None,
                    },
                    logical_advance: 0,
                    violations,
                }
            }
        }
    };
    Ok(post)
}

/// What a retry of the identical request is allowed to conclude.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryVerdict {
    /// The retry is not admitted at all.
    Refused,
    /// The retry is admitted and must land in the same class; a grant here is
    /// not believable without a recorded capacity change.
    SameClassExpected,
    /// The retry is admitted and its outcome is open.
    Undetermined,
}

/// Decide what a repeat of the identical request may conclude, given the
/// previous post-state and whether the available capacity changed in between.
pub fn retry_verdict(previous: &GrowPostState, capacity_changed: bool) -> RetryVerdict {
    match previous.recovery {
        Recovery::None => RetryVerdict::Refused,
        Recovery::RetryAfterCapacityChange => {
            if capacity_changed {
                RetryVerdict::Undetermined
            } else {
                RetryVerdict::SameClassExpected
            }
        }
        Recovery::RetrySameRequest | Recovery::RetryFromObservedLength => {
            RetryVerdict::Undetermined
        }
    }
}

/// What a live mapping actually witnesses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MappingWitness {
    /// Bytes that are both mapped and backed, and therefore usable.
    pub usable: u64,
    /// Mapped bytes past the end of the file. Touching them faults.
    pub beyond_end_of_file: u64,
    /// Mapped bytes inside the file but without reserved backing. Storing
    /// into them may fault at the store.
    pub unbacked_within_file: u64,
}

/// Decide what a mapping of `mapping_len` over a file of `file_len` with the
/// given backing witnesses. A mapping never witnesses capacity it does not
/// have: this is the rule a forged "successful mapping" violates.
pub fn mapping_witness(mapping_len: u64, file_len: u64, backing: Backing) -> MappingWitness {
    let within = mapping_len.min(file_len);
    let backed = match backing {
        Backing::Reserved(bytes) => within.min(bytes),
        Backing::NotReserved | Backing::Unknown => 0,
    };
    MappingWitness {
        usable: backed,
        beyond_end_of_file: mapping_len.saturating_sub(file_len),
        unbacked_within_file: within - backed,
    }
}

/// The kind of endpoint an epoch names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EpochKind {
    /// A descriptor slot.
    Descriptor,
    /// A mapping.
    Mapping,
}

/// One acquisition of an endpoint. The generation, never the number or the
/// address, is the identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Epoch {
    /// Which endpoint kind.
    pub kind: EpochKind,
    /// The generation. Strictly increasing per kind and never reused.
    pub generation: u64,
    /// The length the epoch was acquired with. A mapping must be released at
    /// exactly this length.
    pub length: u64,
    /// The address a mapping epoch was acquired at, recorded so a residual can
    /// be named. A descriptor epoch has none.
    pub address: Option<u64>,
}

/// Why a recorded trace was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RefusalCode {
    /// The trace could not be read as the recorded vocabulary.
    MalformedTrace,
    /// A generation was reused, went backwards, or named a released or
    /// unknown epoch.
    ConfusedGeneration,
    /// An acquired epoch was neither released nor declared as a residual.
    LostResidual,
    /// A mapping was released at a length other than the one it was acquired
    /// with, which leaves part of it mapped.
    EpochLengthMismatch,
    /// A refusal that nevertheless grew the file, or a repeat of a refused
    /// request that was granted with no recorded capacity change.
    ForgedGrant,
    /// A claim of usable capacity past the file length or past the reserved
    /// backing.
    ForgedWitness,
    /// Reserved backing claimed from a call that reserves nothing, or without
    /// the blocks to cover it.
    ForgedReservation,
    /// A conclusion drawn from an unobserved result or error.
    UnobservedOutcome,
    /// A partial transfer counted as though the whole request had landed.
    PartialTreatedAsWhole,
    /// The trace ended with live epochs it did not declare.
    MissingTeardown,
    /// A premise the relation requires was contradicted.
    PremiseViolated,
}

impl RefusalCode {
    /// The stable key used in reports.
    pub fn key(self) -> &'static str {
        match self {
            RefusalCode::MalformedTrace => "capacity.malformed-trace",
            RefusalCode::ConfusedGeneration => "capacity.confused-generation",
            RefusalCode::LostResidual => "capacity.lost-residual",
            RefusalCode::EpochLengthMismatch => "capacity.epoch-length-mismatch",
            RefusalCode::ForgedGrant => "capacity.forged-grant",
            RefusalCode::ForgedWitness => "capacity.forged-witness",
            RefusalCode::ForgedReservation => "capacity.forged-reservation",
            RefusalCode::UnobservedOutcome => "capacity.unobserved-outcome",
            RefusalCode::PartialTreatedAsWhole => "capacity.partial-treated-as-whole",
            RefusalCode::MissingTeardown => "capacity.missing-teardown",
            RefusalCode::PremiseViolated => "capacity.premise-violated",
        }
    }
}

/// One refusal of a recorded trace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal {
    /// Which rule refused.
    pub code: RefusalCode,
    /// The index of the offending event, when one can be named.
    pub event: Option<usize>,
    /// A human-readable detail. Never load-bearing.
    pub detail: String,
}

impl fmt::Display for Refusal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.event {
            Some(index) => write!(
                formatter,
                "{} at event {index}: {}",
                self.code.key(),
                self.detail
            ),
            None => write!(formatter, "{}: {}", self.code.key(), self.detail),
        }
    }
}

/// One recorded event of a native capacity observation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CapacityEvent {
    /// Start of one scenario.
    Fixture {
        /// The scenario number.
        scenario: u64,
    },
    /// A change of the enforced file-size limit.
    Limit {
        /// The soft limit before, `None` for infinity.
        soft_before: Option<u64>,
        /// The soft limit after, `None` for infinity.
        soft_after: Option<u64>,
    },
    /// The recorded disposition of `SIGXFSZ`.
    Signal {
        /// Whether it was taken off its default (terminating) disposition.
        handled: bool,
    },
    /// A descriptor epoch was acquired.
    Open {
        /// The epoch generation.
        generation: u64,
        /// The inode the descriptor names.
        inode: u64,
        /// `st_size` at acquisition.
        file_len: u64,
        /// `st_blocks` at acquisition.
        blocks: u64,
    },
    /// A mapping epoch was acquired.
    Map {
        /// The epoch generation.
        generation: u64,
        /// The mapping address.
        address: u64,
        /// The mapping length.
        len: u64,
        /// `st_size` at acquisition.
        file_len: u64,
    },
    /// A grow call was issued.
    Grow {
        /// The call.
        call: GrowCall,
        /// The length the file was asked to reach.
        request: u64,
        /// The raw return value.
        ret: i64,
        /// The observed error classification.
        error: CapacityErrno,
        /// Whether `errno` was read.
        errno_read: bool,
        /// `st_size` before.
        len_before: u64,
        /// `st_size` after.
        len_after: u64,
        /// `st_blocks` before.
        blocks_before: u64,
        /// `st_blocks` after.
        blocks_after: u64,
        /// Bytes transferred, for calls that transfer.
        transferred: u64,
    },
    /// One mapping epoch replaced another.
    Remap {
        /// The generation released.
        from: u64,
        /// The generation acquired.
        to: u64,
        /// The new address.
        address: u64,
        /// The new length.
        len: u64,
        /// `st_size` at the remap.
        file_len: u64,
    },
    /// The caller claims this much valid content.
    Logical {
        /// The claimed content length.
        len: u64,
    },
    /// A mapping epoch was released.
    Unmap {
        /// The generation released.
        generation: u64,
        /// The length passed to the release.
        len: u64,
        /// The return value.
        ret: i64,
    },
    /// A descriptor epoch was released.
    Close {
        /// The generation released.
        generation: u64,
        /// The return value.
        ret: i64,
    },
    /// End of one scenario, with the residuals the fixture declares.
    End {
        /// The scenario number.
        scenario: u64,
        /// Mapping epochs the fixture declares still live.
        live_maps: u64,
        /// Descriptor epochs the fixture declares still live.
        live_fds: u64,
    },
}

/// One classified transition of the observed boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TransitionClass {
    /// A grant that changed the length and reserved nothing.
    LengthOnlyGrant,
    /// A grant that reserved backing for the range.
    ReservedGrant,
    /// A capacity-class refusal.
    CapacityRefusal,
    /// A repeat of a refused request with no capacity change, refused again.
    RepeatedRefusal,
    /// A grant of a previously refused request after a recorded capacity
    /// change.
    GrantAfterCapacityChange,
    /// A transfer that moved part of its request.
    PartialTransfer,
    /// A failure reported as a return value, with `errno` untouched.
    ReturnedErrorWithoutErrno,
    /// A mapping epoch acquired longer than the file.
    MappingBeyondEndOfFile,
    /// A mapping epoch replaced by a longer one after a grant.
    RemapAfterGrant,
    /// An endpoint released.
    Release,
}

/// One accepted transition, with the post-state the relation decided.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordedTransition {
    /// The event index this transition came from.
    pub event: usize,
    /// The class.
    pub class: TransitionClass,
    /// The decided post-state, for grow calls.
    pub post: Option<GrowPostState>,
}

/// What a validated trace records.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapacityObservation {
    /// The scenario number.
    pub scenario: u64,
    /// Mapping epochs acquired.
    pub mapping_epochs_acquired: u64,
    /// Mapping epochs released.
    pub mapping_epochs_released: u64,
    /// Descriptor epochs acquired.
    pub descriptor_epochs_acquired: u64,
    /// Descriptor epochs released.
    pub descriptor_epochs_released: u64,
    /// The classified transitions, in order.
    pub transitions: Vec<RecordedTransition>,
    /// Epochs still live at the end, exactly as declared.
    pub residuals: Vec<Epoch>,
    /// The last observed file length.
    pub final_file_len: u64,
    /// The backing state at the end.
    pub final_backing: Backing,
    /// The claimed content length at the end.
    pub final_logical_len: u64,
}

impl CapacityObservation {
    /// Whether every acquired epoch was released.
    pub fn fully_released(&self) -> bool {
        self.residuals.is_empty()
            && self.mapping_epochs_acquired == self.mapping_epochs_released
            && self.descriptor_epochs_acquired == self.descriptor_epochs_released
    }
}

/// Parse the recorded native trace vocabulary.
///
/// Lines that do not start with the `QCORE_CAP ` marker are ignored, so the
/// fixture may write anything else to the same stream.
pub fn parse_capacity_trace(text: &str) -> Result<Vec<CapacityEvent>, Refusal> {
    let mut events = Vec::new();
    for (number, line) in text.lines().enumerate() {
        let Some(rest) = line.strip_prefix("QCORE_CAP ") else {
            continue;
        };
        let mut words = rest.split_whitespace();
        let Some(kind) = words.next() else {
            return Err(malformed(number, "empty record"));
        };
        let mut fields: BTreeMap<&str, &str> = BTreeMap::new();
        for word in words {
            let Some((key, value)) = word.split_once('=') else {
                return Err(malformed(number, "field without '='"));
            };
            if fields.insert(key, value).is_some() {
                return Err(malformed(number, "repeated field"));
            }
        }
        let number_field = |key: &str| -> Result<u64, Refusal> {
            let raw = fields
                .get(key)
                .ok_or_else(|| malformed(number, &format!("missing field {key}")))?;
            let parsed = if let Some(hex) = raw.strip_prefix("0x") {
                u64::from_str_radix(hex, 16)
            } else {
                raw.parse::<u64>()
            };
            parsed.map_err(|_| malformed(number, &format!("field {key} is not a number")))
        };
        let signed_field = |key: &str| -> Result<i64, Refusal> {
            fields
                .get(key)
                .ok_or_else(|| malformed(number, &format!("missing field {key}")))?
                .parse::<i64>()
                .map_err(|_| malformed(number, &format!("field {key} is not a number")))
        };
        let optional_limit = |key: &str| -> Result<Option<u64>, Refusal> {
            let raw = fields
                .get(key)
                .ok_or_else(|| malformed(number, &format!("missing field {key}")))?;
            if *raw == "inf" {
                return Ok(None);
            }
            raw.parse::<u64>()
                .map(Some)
                .map_err(|_| malformed(number, &format!("field {key} is not a limit")))
        };
        let flag = |key: &str| -> Result<bool, Refusal> {
            match *fields
                .get(key)
                .ok_or_else(|| malformed(number, &format!("missing field {key}")))?
            {
                "0" => Ok(false),
                "1" => Ok(true),
                _ => Err(malformed(number, &format!("field {key} is not a flag"))),
            }
        };
        let event = match kind {
            "FIXTURE" => CapacityEvent::Fixture {
                scenario: number_field("scenario")?,
            },
            "LIMIT" => CapacityEvent::Limit {
                soft_before: optional_limit("soft_before")?,
                soft_after: optional_limit("soft_after")?,
            },
            "SIGNAL" => CapacityEvent::Signal {
                handled: flag("handled")?,
            },
            "OPEN" => CapacityEvent::Open {
                generation: number_field("gen")?,
                inode: number_field("inode")?,
                file_len: number_field("file_len")?,
                blocks: number_field("blocks")?,
            },
            "MAP" => CapacityEvent::Map {
                generation: number_field("gen")?,
                address: number_field("addr")?,
                len: number_field("len")?,
                file_len: number_field("file_len")?,
            },
            "GROW" => CapacityEvent::Grow {
                call: match *fields
                    .get("call")
                    .ok_or_else(|| malformed(number, "missing field call"))?
                {
                    "ftruncate" => GrowCall::Ftruncate,
                    "fallocate" => GrowCall::Fallocate,
                    "posix_fallocate" => GrowCall::PosixFallocate,
                    "write" => GrowCall::Write,
                    other => return Err(malformed(number, &format!("unknown call {other}"))),
                },
                request: number_field("request")?,
                ret: signed_field("ret")?,
                error: match *fields
                    .get("error")
                    .ok_or_else(|| malformed(number, "missing field error"))?
                {
                    "none" => CapacityErrno::Other,
                    "unobserved" => CapacityErrno::Unobserved,
                    other => CapacityErrno::from_linux_number(
                        other
                            .parse::<i64>()
                            .map_err(|_| malformed(number, "field error is not a number"))?,
                    ),
                },
                errno_read: flag("errno_read")?,
                len_before: number_field("len_before")?,
                len_after: number_field("len_after")?,
                blocks_before: number_field("blocks_before")?,
                blocks_after: number_field("blocks_after")?,
                transferred: number_field("transferred")?,
            },
            "REMAP" => CapacityEvent::Remap {
                from: number_field("from")?,
                to: number_field("to")?,
                address: number_field("addr")?,
                len: number_field("len")?,
                file_len: number_field("file_len")?,
            },
            "LOGICAL" => CapacityEvent::Logical {
                len: number_field("len")?,
            },
            "UNMAP" => CapacityEvent::Unmap {
                generation: number_field("gen")?,
                len: number_field("len")?,
                ret: signed_field("ret")?,
            },
            "CLOSE" => CapacityEvent::Close {
                generation: number_field("gen")?,
                ret: signed_field("ret")?,
            },
            "END" => CapacityEvent::End {
                scenario: number_field("scenario")?,
                live_maps: number_field("live_maps")?,
                live_fds: number_field("live_fds")?,
            },
            other => return Err(malformed(number, &format!("unknown record {other}"))),
        };
        events.push(event);
    }
    if events.is_empty() {
        return Err(Refusal {
            code: RefusalCode::MalformedTrace,
            event: None,
            detail: "no recorded events".to_owned(),
        });
    }
    Ok(events)
}

fn malformed(line: usize, detail: &str) -> Refusal {
    Refusal {
        code: RefusalCode::MalformedTrace,
        event: Some(line),
        detail: detail.to_owned(),
    }
}

struct LiveEpoch {
    length: u64,
    address: u64,
}

/// Validate a recorded native trace of the capacity boundary.
///
/// The relation decides every grow call, and the epoch ledger decides every
/// acquisition and release. Any disagreement is a refusal: the trace is never
/// repaired and never partially accepted.
pub fn validate_capacity_trace(
    events: &[CapacityEvent],
) -> Result<CapacityObservation, Vec<Refusal>> {
    let mut refusals = Vec::new();
    let mut mappings: BTreeMap<u64, LiveEpoch> = BTreeMap::new();
    let mut descriptors: BTreeSet<u64> = BTreeSet::new();
    let mut retired: BTreeSet<(EpochKind, u64)> = BTreeSet::new();
    let mut highest: BTreeMap<EpochKind, u64> = BTreeMap::new();
    let mut acquired = BTreeMap::from([(EpochKind::Mapping, 0u64), (EpochKind::Descriptor, 0u64)]);
    let mut released = BTreeMap::from([(EpochKind::Mapping, 0u64), (EpochKind::Descriptor, 0u64)]);
    let mut transitions = Vec::new();
    let mut scenario = None;
    let mut file_len = 0u64;
    // Leading bytes of the file known to be backed. It is raised only by an
    // observation that established backing, and a length-only grant never
    // raises it: the range that call added is a hole.
    let mut reserved = 0u64;
    let mut logical = 0u64;
    let mut signal_handled = false;
    let mut limit_changed_since_refusal = false;
    let mut last_refusal: Option<(GrowCall, u64, GrowPostState)> = None;
    let mut last_transfer: Option<(usize, u64)> = None;
    let mut ended = false;
    let mut residuals = Vec::new();

    let mut refuse = |code: RefusalCode, index: usize, detail: String| {
        refusals.push(Refusal {
            code,
            event: Some(index),
            detail,
        });
    };

    for (index, event) in events.iter().enumerate() {
        if ended {
            refuse(
                RefusalCode::MalformedTrace,
                index,
                "an event follows the end record".to_owned(),
            );
            break;
        }
        match *event {
            CapacityEvent::Fixture { scenario: number } => {
                if scenario.is_some() {
                    refuse(
                        RefusalCode::MalformedTrace,
                        index,
                        "a second fixture record".to_owned(),
                    );
                }
                scenario = Some(number);
            }
            CapacityEvent::Limit {
                soft_before,
                soft_after,
            } => {
                if soft_before != soft_after {
                    limit_changed_since_refusal = true;
                }
            }
            CapacityEvent::Signal { handled } => signal_handled = handled,
            CapacityEvent::Open {
                generation,
                file_len: length,
                blocks,
                ..
            } => {
                check_fresh(
                    EpochKind::Descriptor,
                    generation,
                    index,
                    &highest,
                    &retired,
                    &mut refuse,
                );
                if descriptors.contains(&generation) {
                    refuse(
                        RefusalCode::ConfusedGeneration,
                        index,
                        format!("descriptor generation {generation} is already live"),
                    );
                }
                descriptors.insert(generation);
                highest.insert(EpochKind::Descriptor, generation);
                *acquired.get_mut(&EpochKind::Descriptor).unwrap() += 1;
                file_len = length;
                reserved = blocks.saturating_mul(512).min(length);
            }
            CapacityEvent::Map {
                generation,
                address,
                len,
                file_len: length,
            } => {
                check_fresh(
                    EpochKind::Mapping,
                    generation,
                    index,
                    &highest,
                    &retired,
                    &mut refuse,
                );
                if mappings.contains_key(&generation) {
                    refuse(
                        RefusalCode::ConfusedGeneration,
                        index,
                        format!("mapping generation {generation} is already live"),
                    );
                }
                if length != file_len {
                    refuse(
                        RefusalCode::ForgedWitness,
                        index,
                        format!(
                            "recorded file length {length} disagrees with the ledger's {file_len}"
                        ),
                    );
                }
                mappings.insert(
                    generation,
                    LiveEpoch {
                        length: len,
                        address,
                    },
                );
                highest.insert(EpochKind::Mapping, generation);
                *acquired.get_mut(&EpochKind::Mapping).unwrap() += 1;
                if len > length {
                    transitions.push(RecordedTransition {
                        event: index,
                        class: TransitionClass::MappingBeyondEndOfFile,
                        post: None,
                    });
                }
            }
            CapacityEvent::Grow {
                call,
                request,
                ret,
                error,
                errno_read,
                len_before,
                len_after,
                blocks_before,
                blocks_after,
                transferred,
            } => {
                if len_before != file_len {
                    refuse(
                        RefusalCode::ForgedWitness,
                        index,
                        format!(
                            "recorded prior length {len_before} disagrees with the ledger's {file_len}"
                        ),
                    );
                }
                let outcome = if ret == 0 && transferred == 0 {
                    GrowOutcome::Granted
                } else if call == GrowCall::Write && ret > 0 {
                    let moved = u64::try_from(ret).unwrap_or(0);
                    if len_before.saturating_add(moved) >= request {
                        GrowOutcome::Granted
                    } else {
                        GrowOutcome::Partial { transferred: moved }
                    }
                } else if error == CapacityErrno::Unobserved {
                    GrowOutcome::ResultUnobserved
                } else {
                    GrowOutcome::Refused(error)
                };
                if matches!(outcome, GrowOutcome::ResultUnobserved) {
                    refuse(
                        RefusalCode::UnobservedOutcome,
                        index,
                        "the outcome of the call was not observed".to_owned(),
                    );
                }
                let observation = GrowObservation {
                    call,
                    request_len: request,
                    prior_len: len_before,
                    observed_len: len_after,
                    prior_blocks: blocks_before,
                    observed_blocks: blocks_after,
                    mapping_len: mappings.values().map(|epoch| epoch.length).max(),
                };
                let premises = Premises {
                    result_observed: true,
                    errno_observed: errno_read,
                    length_observed: true,
                    blocks_observed: true,
                    sole_writer: true,
                    signal_handled,
                };
                match grow_poststate(Target::Linux, premises, observation, outcome) {
                    Err(error) => refuse(
                        RefusalCode::MalformedTrace,
                        index,
                        format!("the relation refuses the request: {error}"),
                    ),
                    Ok(post) => {
                        for violation in &post.violations {
                            refuse(
                                RefusalCode::PremiseViolated,
                                index,
                                format!("{violation:?}"),
                            );
                        }
                        if matches!(outcome, GrowOutcome::Refused(_))
                            && len_after > len_before
                            && !call.failure_may_change_length()
                        {
                            refuse(
                                RefusalCode::ForgedGrant,
                                index,
                                "a refusal that grew the file".to_owned(),
                            );
                        }
                        if matches!(outcome, GrowOutcome::Refused(_))
                            && blocks_before.saturating_mul(512) < request
                            && blocks_after.saturating_mul(512) >= request
                        {
                            refuse(
                                RefusalCode::ForgedReservation,
                                index,
                                "a refused call recorded as having reserved the whole request"
                                    .to_owned(),
                            );
                        }
                        let class = classify(
                            call,
                            &outcome,
                            &last_refusal,
                            request,
                            limit_changed_since_refusal,
                            errno_read,
                        );
                        if class == TransitionClass::GrantAfterCapacityChange
                            && !limit_changed_since_refusal
                        {
                            refuse(
                                RefusalCode::ForgedGrant,
                                index,
                                "a refused request was granted with no recorded capacity change"
                                    .to_owned(),
                            );
                        }
                        if let Some((previous_call, previous_request, previous_post)) =
                            &last_refusal
                            && *previous_call == call
                            && *previous_request == request
                            && retry_verdict(previous_post, limit_changed_since_refusal)
                                == RetryVerdict::SameClassExpected
                            && matches!(outcome, GrowOutcome::Granted | GrowOutcome::Partial { .. })
                        {
                            refuse(
                                RefusalCode::ForgedGrant,
                                index,
                                "the identical request was granted after a capacity refusal with no capacity change"
                                    .to_owned(),
                            );
                        }
                        match post.file_length {
                            FileLength::Unchanged(length) | FileLength::GrownTo(length) => {
                                file_len = length;
                            }
                            FileLength::Indeterminate => {}
                        }
                        // A length-only grant adds a hole, so the backed
                        // prefix never grows on that arm.
                        if let Backing::Reserved(bytes) = post.backing {
                            reserved = reserved.max(bytes.min(file_len));
                        }
                        last_transfer = match outcome {
                            GrowOutcome::Partial { transferred } => Some((index, transferred)),
                            GrowOutcome::Granted if call == GrowCall::Write => {
                                Some((index, len_after.saturating_sub(len_before)))
                            }
                            _ => None,
                        };
                        if matches!(outcome, GrowOutcome::Refused(_)) {
                            last_refusal = Some((call, request, post.clone()));
                            limit_changed_since_refusal = false;
                        } else {
                            last_refusal = None;
                        }
                        transitions.push(RecordedTransition {
                            event: index,
                            class,
                            post: Some(post),
                        });
                    }
                }
            }
            CapacityEvent::Remap {
                from,
                to,
                address,
                len,
                file_len: length,
            } => {
                if length != file_len {
                    refuse(
                        RefusalCode::ForgedWitness,
                        index,
                        format!(
                            "recorded file length {length} disagrees with the ledger's {file_len}"
                        ),
                    );
                }
                if mappings.remove(&from).is_none() {
                    refuse(
                        RefusalCode::ConfusedGeneration,
                        index,
                        format!("remap from mapping generation {from}, which is not live"),
                    );
                } else {
                    retired.insert((EpochKind::Mapping, from));
                    *released.get_mut(&EpochKind::Mapping).unwrap() += 1;
                }
                check_fresh(
                    EpochKind::Mapping,
                    to,
                    index,
                    &highest,
                    &retired,
                    &mut refuse,
                );
                mappings.insert(
                    to,
                    LiveEpoch {
                        length: len,
                        address,
                    },
                );
                highest.insert(EpochKind::Mapping, to);
                *acquired.get_mut(&EpochKind::Mapping).unwrap() += 1;
                transitions.push(RecordedTransition {
                    event: index,
                    class: if len > length {
                        TransitionClass::MappingBeyondEndOfFile
                    } else {
                        TransitionClass::RemapAfterGrant
                    },
                    post: None,
                });
            }
            CapacityEvent::Logical { len } => {
                // Content is bounded by the backed prefix of the file, never
                // by a length a grow call merely reported.
                let usable = file_len.min(reserved);
                if len > usable {
                    refuse(
                        RefusalCode::ForgedWitness,
                        index,
                        format!("content length {len} exceeds the {usable} backed bytes observed"),
                    );
                }
                // A claim recorded immediately after a transferring call may
                // advance by exactly what that call moved.
                if let Some((moved_at, moved)) = last_transfer
                    && moved_at + 1 == index
                    && len.saturating_sub(logical) > moved
                {
                    refuse(
                        RefusalCode::PartialTreatedAsWhole,
                        index,
                        format!(
                            "content advanced by {} where the transfer moved {moved}",
                            len - logical
                        ),
                    );
                }
                logical = len;
            }
            CapacityEvent::Unmap {
                generation,
                len,
                ret,
            } => {
                match mappings.remove(&generation) {
                    None => refuse(
                        RefusalCode::ConfusedGeneration,
                        index,
                        format!("release of mapping generation {generation}, which is not live"),
                    ),
                    Some(epoch) => {
                        if epoch.length != len {
                            refuse(
                                RefusalCode::EpochLengthMismatch,
                                index,
                                format!(
                                    "mapping generation {generation} was acquired at {} and released at {len}",
                                    epoch.length
                                ),
                            );
                        }
                        if ret == 0 {
                            retired.insert((EpochKind::Mapping, generation));
                            *released.get_mut(&EpochKind::Mapping).unwrap() += 1;
                            transitions.push(RecordedTransition {
                                event: index,
                                class: TransitionClass::Release,
                                post: None,
                            });
                        } else {
                            // A failed release retains the epoch: it is a
                            // residual, not a release.
                            mappings.insert(generation, epoch);
                        }
                    }
                }
            }
            CapacityEvent::Close { generation, ret } => {
                if !descriptors.remove(&generation) {
                    refuse(
                        RefusalCode::ConfusedGeneration,
                        index,
                        format!("release of descriptor generation {generation}, which is not live"),
                    );
                } else if ret == 0 {
                    retired.insert((EpochKind::Descriptor, generation));
                    *released.get_mut(&EpochKind::Descriptor).unwrap() += 1;
                    transitions.push(RecordedTransition {
                        event: index,
                        class: TransitionClass::Release,
                        post: None,
                    });
                } else {
                    descriptors.insert(generation);
                }
            }
            CapacityEvent::End {
                scenario: number,
                live_maps,
                live_fds,
            } => {
                ended = true;
                if scenario != Some(number) {
                    refuse(
                        RefusalCode::MalformedTrace,
                        index,
                        "the end record names another scenario".to_owned(),
                    );
                }
                if live_maps != mappings.len() as u64 || live_fds != descriptors.len() as u64 {
                    refuse(
                        RefusalCode::LostResidual,
                        index,
                        format!(
                            "declared {live_maps} live mappings and {live_fds} live descriptors; the ledger holds {} and {}",
                            mappings.len(),
                            descriptors.len()
                        ),
                    );
                }
                residuals = mappings
                    .iter()
                    .map(|(generation, epoch)| Epoch {
                        kind: EpochKind::Mapping,
                        generation: *generation,
                        length: epoch.length,
                        address: Some(epoch.address),
                    })
                    .chain(descriptors.iter().map(|generation| Epoch {
                        kind: EpochKind::Descriptor,
                        generation: *generation,
                        length: 0,
                        address: None,
                    }))
                    .collect();
            }
        }
    }

    if !ended {
        refusals.push(Refusal {
            code: RefusalCode::MissingTeardown,
            event: None,
            detail: "the trace has no end record".to_owned(),
        });
    }
    let Some(scenario) = scenario else {
        refusals.push(Refusal {
            code: RefusalCode::MalformedTrace,
            event: None,
            detail: "the trace has no fixture record".to_owned(),
        });
        return Err(refusals);
    };
    if !refusals.is_empty() {
        return Err(refusals);
    }
    Ok(CapacityObservation {
        scenario,
        mapping_epochs_acquired: acquired[&EpochKind::Mapping],
        mapping_epochs_released: released[&EpochKind::Mapping],
        descriptor_epochs_acquired: acquired[&EpochKind::Descriptor],
        descriptor_epochs_released: released[&EpochKind::Descriptor],
        transitions,
        residuals,
        final_file_len: file_len,
        final_backing: Backing::Reserved(reserved),
        final_logical_len: logical,
    })
}

fn check_fresh(
    kind: EpochKind,
    generation: u64,
    index: usize,
    highest: &BTreeMap<EpochKind, u64>,
    retired: &BTreeSet<(EpochKind, u64)>,
    refuse: &mut impl FnMut(RefusalCode, usize, String),
) {
    if retired.contains(&(kind, generation)) {
        refuse(
            RefusalCode::ConfusedGeneration,
            index,
            format!("{kind:?} generation {generation} was already retired"),
        );
    }
    if let Some(previous) = highest.get(&kind)
        && generation <= *previous
    {
        refuse(
            RefusalCode::ConfusedGeneration,
            index,
            format!("{kind:?} generation {generation} does not exceed the previous {previous}"),
        );
    }
}

fn classify(
    call: GrowCall,
    outcome: &GrowOutcome,
    last_refusal: &Option<(GrowCall, u64, GrowPostState)>,
    request: u64,
    capacity_changed: bool,
    errno_read: bool,
) -> TransitionClass {
    let repeats = last_refusal
        .as_ref()
        .is_some_and(|(previous, previous_request, _)| {
            *previous == call && *previous_request == request
        });
    match outcome {
        GrowOutcome::Granted if repeats && capacity_changed => {
            TransitionClass::GrantAfterCapacityChange
        }
        GrowOutcome::Granted if call.reserves_backing() => TransitionClass::ReservedGrant,
        GrowOutcome::Granted => TransitionClass::LengthOnlyGrant,
        GrowOutcome::Partial { .. } => TransitionClass::PartialTransfer,
        GrowOutcome::Refused(_) if !errno_read && !call.reports_through_errno() => {
            TransitionClass::ReturnedErrorWithoutErrno
        }
        GrowOutcome::Refused(_) if repeats => TransitionClass::RepeatedRefusal,
        GrowOutcome::Refused(_) => TransitionClass::CapacityRefusal,
        GrowOutcome::NotAttempted | GrowOutcome::ResultUnobserved => {
            TransitionClass::CapacityRefusal
        }
    }
}

#[cfg(test)]
#[path = "../tests/unit/capacity_boundary.rs"]
mod tests;

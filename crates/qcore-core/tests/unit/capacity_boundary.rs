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

//! Card O02 — the capacity grow boundary: relation, recorded native evidence
//! and the mutation matrix.
//!
//! The native part runs on Linux only: it depends on `RLIMIT_FSIZE`,
//! `mremap(2)` and `st_blocks`, and it makes real, unsuppressed syscalls in an
//! exclusive temporary directory. Everything it observes is scoped to that
//! platform, those sizes and those paths.

use crate::capacity_boundary::{
    Backing, CapacityErrno, CapacityEvent, Duties, Epoch, EpochKind, FileLength, GrowCall,
    GrowObservation, GrowOutcome, GrowPostState, MappingDuty, Premises, Recovery, RefusalCode,
    RelationError, RetryVerdict, Target, TransitionClass, grow_poststate, mapping_witness,
    parse_capacity_trace, retry_verdict, validate_capacity_trace,
};

/// The seed file every scenario starts from: 4096 written bytes.
const SEED: u64 = 4096;
/// The length every grow scenario asks for.
const GROWN: u64 = 8192;

fn observation(call: GrowCall, prior: u64, observed: u64, blocks: u64) -> GrowObservation {
    GrowObservation {
        call,
        request_len: GROWN,
        prior_len: prior,
        observed_len: observed,
        prior_blocks: SEED / 512,
        observed_blocks: blocks,
        mapping_len: Some(SEED),
    }
}

/// `errno` is read exactly where the call sets it, which is the faithful
/// pre-state for each call.
fn premises_for(call: GrowCall) -> Premises {
    Premises {
        errno_observed: call.reports_through_errno(),
        ..Premises::fully_observed()
    }
}

fn decide(call: GrowCall, outcome: GrowOutcome, observed: u64, blocks: u64) -> GrowPostState {
    grow_poststate(
        Target::Linux,
        premises_for(call),
        observation(call, SEED, observed, blocks),
        outcome,
    )
    .expect("the request grows the file")
}

// ---------------------------------------------------------------------------
// The relation.
// ---------------------------------------------------------------------------

/// A length-only grant moves the length and reserves nothing, so the range it
/// added is not writable authority.
#[test]
fn a_length_only_grant_reserves_nothing() {
    let post = decide(GrowCall::Ftruncate, GrowOutcome::Granted, GROWN, SEED / 512);
    assert_eq!(post.file_length, FileLength::GrownTo(GROWN));
    assert_eq!(post.backing, Backing::NotReserved);
    assert_eq!(post.mapping, MappingDuty::RemapBeforeUse { to_len: GROWN });
    assert!(post.duties.no_store_beyond_reserved);
    assert!(post.duties.remap_before_use);
    assert_eq!(post.recovery, Recovery::None);
    assert_eq!(post.logical_advance, 0);
    assert!(post.violations.is_empty());
}

/// An allocating grant reserves the range it covers.
#[test]
fn an_allocating_grant_reserves_the_range() {
    let post = decide(
        GrowCall::PosixFallocate,
        GrowOutcome::Granted,
        GROWN,
        GROWN / 512,
    );
    assert_eq!(post.backing, Backing::Reserved(GROWN));
    assert!(!post.duties.no_store_beyond_reserved);
}

/// A grant whose blocks do not cover the range it claims is not a grant.
#[test]
fn an_allocating_grant_without_the_blocks_is_poisoned() {
    let post = decide(
        GrowCall::PosixFallocate,
        GrowOutcome::Granted,
        GROWN,
        SEED / 512,
    );
    assert!(post.is_poisoned());
    assert_eq!(post.backing, Backing::Unknown);
}

/// A capacity refusal leaves the length where it was and admits only a retry
/// after the available capacity has changed.
#[test]
fn a_capacity_refusal_keeps_the_length_and_defers_the_retry() {
    for error in [
        CapacityErrno::Efbig,
        CapacityErrno::Enospc,
        CapacityErrno::Edquot,
    ] {
        let post = decide(
            GrowCall::Ftruncate,
            GrowOutcome::Refused(error),
            SEED,
            SEED / 512,
        );
        assert_eq!(post.file_length, FileLength::Unchanged(SEED));
        assert_eq!(post.mapping, MappingDuty::RetainExisting);
        assert!(post.duties.keep_logical_length);
        assert_eq!(post.recovery, Recovery::RetryAfterCapacityChange);
        assert!(!post.is_poisoned());
    }
}

/// `ftruncate(2)` documents no length change on failure, so a failure that
/// reports one contradicts the observation rather than extending it.
#[test]
fn a_length_only_refusal_may_not_have_grown_the_file() {
    let post = decide(
        GrowCall::Ftruncate,
        GrowOutcome::Refused(CapacityErrno::Efbig),
        GROWN,
        SEED / 512,
    );
    assert!(post.is_poisoned());
}

/// The allocating calls leave a partial effect unspecified, so a length change
/// on failure is admitted and carries unknown backing rather than a grant.
#[test]
fn an_allocating_refusal_may_have_changed_the_length_with_unknown_backing() {
    let post = decide(
        GrowCall::Fallocate,
        GrowOutcome::Refused(CapacityErrno::Enospc),
        SEED + 512,
        SEED / 512,
    );
    assert_eq!(post.file_length, FileLength::GrownTo(SEED + 512));
    assert_eq!(post.backing, Backing::Unknown);
    assert!(post.duties.reobserve_length);
}

/// An unobserved result concludes nothing and permits nothing.
#[test]
fn an_unobserved_result_is_poisoned() {
    let post = decide(
        GrowCall::Ftruncate,
        GrowOutcome::ResultUnobserved,
        SEED,
        SEED / 512,
    );
    assert!(post.is_poisoned());
    assert_eq!(post.recovery, Recovery::None);
    assert_eq!(post.file_length, FileLength::Indeterminate);
}

/// A failure branch that did not read `errno` cannot classify the failure.
#[test]
fn a_refusal_without_an_observed_errno_is_poisoned() {
    let premises = Premises {
        errno_observed: false,
        ..Premises::fully_observed()
    };
    let post = grow_poststate(
        Target::Linux,
        premises,
        observation(GrowCall::Ftruncate, SEED, SEED, SEED / 512),
        GrowOutcome::Refused(CapacityErrno::Efbig),
    )
    .unwrap();
    assert!(post.is_poisoned());
}

/// `posix_fallocate(3)` does not set `errno`; reading it there is a premise
/// violation, not a detail.
#[test]
fn reading_errno_from_the_returning_call_is_a_premise_violation() {
    let post = grow_poststate(
        Target::Linux,
        Premises::fully_observed(),
        observation(GrowCall::PosixFallocate, SEED, SEED, SEED / 512),
        GrowOutcome::Refused(CapacityErrno::Efbig),
    )
    .unwrap();
    assert!(post.is_poisoned());
    // ... and the same call with `errno` left alone is decided.
    let premises = Premises {
        errno_observed: false,
        ..Premises::fully_observed()
    };
    let decided = grow_poststate(
        Target::Linux,
        premises,
        observation(GrowCall::PosixFallocate, SEED, SEED, SEED / 512),
        GrowOutcome::Refused(CapacityErrno::Efbig),
    )
    .unwrap();
    assert!(!decided.is_poisoned());
}

/// A partial transfer advances the content by exactly what moved.
#[test]
fn a_partial_transfer_advances_by_exactly_what_moved() {
    let post = grow_poststate(
        Target::Linux,
        Premises::fully_observed(),
        GrowObservation {
            call: GrowCall::Write,
            request_len: SEED + 8,
            prior_len: SEED,
            observed_len: SEED + 4,
            prior_blocks: SEED / 512,
            observed_blocks: (SEED + 4).div_ceil(512),
            mapping_len: Some(SEED),
        },
        GrowOutcome::Partial { transferred: 4 },
    )
    .unwrap();
    assert_eq!(post.logical_advance, 4);
    assert_eq!(post.file_length, FileLength::GrownTo(SEED + 4));
    assert_eq!(post.recovery, Recovery::RetryFromObservedLength);
}

/// Only a transferring call can transfer partially.
#[test]
fn a_partial_outcome_from_a_resizing_call_is_poisoned() {
    let post = decide(
        GrowCall::Ftruncate,
        GrowOutcome::Partial { transferred: 4 },
        SEED + 4,
        SEED / 512,
    );
    assert!(post.is_poisoned());
}

/// A concurrent writer poisons every arm, over the whole call x outcome
/// domain.
#[test]
fn a_concurrent_writer_poisons_every_arm() {
    let premises = Premises {
        sole_writer: false,
        ..Premises::fully_observed()
    };
    for call in GrowCall::ALL {
        for outcome in outcome_domain() {
            let post = grow_poststate(
                Target::Linux,
                premises,
                observation(call, SEED, GROWN, GROWN / 512),
                outcome,
            )
            .unwrap();
            assert!(post.is_poisoned(), "{call:?} {outcome:?}");
        }
    }
}

/// An unpinned target concludes nothing anywhere in the domain.
#[test]
fn an_unpinned_target_concludes_nothing() {
    for call in GrowCall::ALL {
        for outcome in outcome_domain() {
            let post = grow_poststate(
                Target::Unpinned,
                Premises::fully_observed(),
                observation(call, SEED, GROWN, GROWN / 512),
                outcome,
            )
            .unwrap();
            assert!(post.is_poisoned(), "{call:?} {outcome:?}");
            assert_eq!(post.recovery, Recovery::None);
        }
    }
}

fn outcome_domain() -> Vec<GrowOutcome> {
    let mut outcomes = vec![
        GrowOutcome::NotAttempted,
        GrowOutcome::ResultUnobserved,
        GrowOutcome::Granted,
        GrowOutcome::Partial { transferred: 4 },
    ];
    outcomes.extend(CapacityErrno::ALL.map(GrowOutcome::Refused));
    outcomes
}

/// Domain-exhaustive: no arm anywhere ever concludes that a store past the
/// reserved range is authorised, unless the call actually reserved it.
#[test]
fn no_arm_authorises_a_store_it_did_not_reserve() {
    for target in Target::ALL {
        for call in GrowCall::ALL {
            for outcome in outcome_domain() {
                for observed_blocks in [SEED / 512, GROWN / 512] {
                    let post = grow_poststate(
                        Target::Linux,
                        premises_for(call),
                        observation(call, SEED, GROWN, observed_blocks),
                        outcome,
                    )
                    .unwrap();
                    let _ = target;
                    if !post.duties.no_store_beyond_reserved {
                        assert!(
                            matches!(post.backing, Backing::Reserved(_)),
                            "{call:?} {outcome:?} dropped the store duty without reserving"
                        );
                    }
                }
            }
        }
    }
}

/// Domain-exhaustive: a poisoned post-state never permits a recovery, and
/// always carries the conservative duties.
#[test]
fn a_poisoned_arm_permits_nothing() {
    for target in Target::ALL {
        for call in GrowCall::ALL {
            for outcome in outcome_domain() {
                let post = grow_poststate(
                    target,
                    premises_for(call),
                    observation(call, SEED, GROWN, GROWN / 512),
                    outcome,
                )
                .unwrap();
                if post.is_poisoned() {
                    assert_eq!(post.recovery, Recovery::None);
                    assert_eq!(post.duties, Duties::poisoned());
                    assert_eq!(post.backing, Backing::Unknown);
                    assert_eq!(post.logical_advance, 0);
                }
            }
        }
    }
}

/// Domain-exhaustive: capacity is not content. No grow call advances the
/// content length; only an observed transfer does.
#[test]
fn capacity_is_never_content() {
    for call in [
        GrowCall::Ftruncate,
        GrowCall::Fallocate,
        GrowCall::PosixFallocate,
    ] {
        for outcome in outcome_domain() {
            let post = grow_poststate(
                Target::Linux,
                premises_for(call),
                observation(call, SEED, GROWN, GROWN / 512),
                outcome,
            )
            .unwrap();
            assert_eq!(post.logical_advance, 0, "{call:?} {outcome:?}");
        }
    }
}

/// A request that does not grow the file is not this relation's business.
#[test]
fn a_request_that_does_not_grow_is_refused() {
    let error = grow_poststate(
        Target::Linux,
        Premises::fully_observed(),
        GrowObservation {
            request_len: SEED,
            ..observation(GrowCall::Ftruncate, SEED, SEED, SEED / 512)
        },
        GrowOutcome::Granted,
    )
    .unwrap_err();
    assert_eq!(error, RelationError::NotAGrowRequest);
}

/// A retry of the identical request decides nothing new until the capacity
/// changes, and is refused outright after an undetermined outcome.
#[test]
fn the_retry_verdict_follows_the_recovery() {
    let refused = decide(
        GrowCall::Ftruncate,
        GrowOutcome::Refused(CapacityErrno::Efbig),
        SEED,
        SEED / 512,
    );
    assert_eq!(
        retry_verdict(&refused, false),
        RetryVerdict::SameClassExpected
    );
    assert_eq!(retry_verdict(&refused, true), RetryVerdict::Undetermined);
    let poisoned = decide(
        GrowCall::Ftruncate,
        GrowOutcome::ResultUnobserved,
        SEED,
        SEED / 512,
    );
    assert_eq!(retry_verdict(&poisoned, true), RetryVerdict::Refused);
    let interrupted = decide(
        GrowCall::Fallocate,
        GrowOutcome::Refused(CapacityErrno::Eintr),
        SEED,
        SEED / 512,
    );
    assert_eq!(
        retry_verdict(&interrupted, false),
        RetryVerdict::Undetermined
    );
}

/// A mapping never witnesses capacity past the end of the file, and never
/// witnesses backing it does not have.
#[test]
fn a_mapping_witnesses_only_backed_bytes_inside_the_file() {
    let beyond = mapping_witness(GROWN, SEED, Backing::Reserved(SEED));
    assert_eq!(beyond.usable, SEED);
    assert_eq!(beyond.beyond_end_of_file, SEED);
    let hole = mapping_witness(GROWN, GROWN, Backing::NotReserved);
    assert_eq!(hole.usable, 0);
    assert_eq!(hole.beyond_end_of_file, 0);
    assert_eq!(hole.unbacked_within_file, GROWN);
    let unknown = mapping_witness(GROWN, GROWN, Backing::Unknown);
    assert_eq!(unknown.usable, 0);
    for mapped in [0, 512, SEED, GROWN, GROWN * 2] {
        for length in [0, SEED, GROWN] {
            let witness = mapping_witness(mapped, length, Backing::Reserved(GROWN * 4));
            assert!(witness.usable <= length);
            assert!(witness.usable <= mapped);
        }
    }
}

/// Non-vacuity: every post-state the vocabulary can express is reachable.
#[test]
fn every_post_state_is_reachable() {
    assert_eq!(
        decide(GrowCall::Ftruncate, GrowOutcome::NotAttempted, SEED, 8).file_length,
        FileLength::Unchanged(SEED)
    );
    assert_eq!(
        decide(GrowCall::Ftruncate, GrowOutcome::Granted, GROWN, 8).backing,
        Backing::NotReserved
    );
    assert_eq!(
        decide(
            GrowCall::Fallocate,
            GrowOutcome::Granted,
            GROWN,
            GROWN / 512
        )
        .backing,
        Backing::Reserved(GROWN)
    );
    assert_eq!(
        decide(
            GrowCall::Fallocate,
            GrowOutcome::Refused(CapacityErrno::Enospc),
            SEED,
            8
        )
        .backing,
        Backing::Unknown
    );
    assert_eq!(
        decide(GrowCall::Ftruncate, GrowOutcome::ResultUnobserved, SEED, 8).mapping,
        MappingDuty::Poisoned
    );
    let unmapped = grow_poststate(
        Target::Linux,
        Premises::fully_observed(),
        GrowObservation {
            mapping_len: None,
            ..observation(GrowCall::Ftruncate, SEED, GROWN, 8)
        },
        GrowOutcome::Granted,
    )
    .unwrap();
    assert_eq!(unmapped.mapping, MappingDuty::None);
    let empty = grow_poststate(
        Target::Linux,
        Premises::fully_observed(),
        GrowObservation {
            mapping_len: Some(0),
            ..observation(GrowCall::Ftruncate, SEED, GROWN, 8)
        },
        GrowOutcome::Granted,
    );
    assert_eq!(empty.unwrap_err(), RelationError::EmptyMapping);
}

// ---------------------------------------------------------------------------
// The recorded traces, and the mutation matrix over them.
// ---------------------------------------------------------------------------

/// Scenario 0 exactly as the native run recorded it: a length-only grant.
const LENGTH_ONLY: &str = "\
QCORE_CAP FIXTURE scenario=0
QCORE_CAP SIGNAL handled=1
QCORE_CAP OPEN gen=1 inode=2054119 file_len=4096 blocks=8
QCORE_CAP MAP gen=1 addr=0x7f8ca5a69000 len=4096 file_len=4096
QCORE_CAP GROW call=ftruncate request=8192 ret=0 error=none errno_read=0 len_before=4096 len_after=8192 blocks_before=8 blocks_after=8 transferred=0
QCORE_CAP REMAP from=1 to=2 addr=0x7f8ca58bd000 len=8192 file_len=8192
QCORE_CAP LOGICAL len=4096
QCORE_CAP UNMAP gen=2 len=8192 ret=0
QCORE_CAP CLOSE gen=1 ret=0
QCORE_CAP END scenario=0 live_maps=0 live_fds=0
";

/// Scenario 2 exactly as the native run recorded it: a refusal, a retry with
/// nothing changed, and a grant only after the enforced capacity changed.
const REFUSAL_AND_RETRY: &str = "\
QCORE_CAP FIXTURE scenario=2
QCORE_CAP SIGNAL handled=1
QCORE_CAP OPEN gen=1 inode=2054123 file_len=4096 blocks=8
QCORE_CAP MAP gen=1 addr=0x73b9b2a42000 len=4096 file_len=4096
QCORE_CAP LIMIT soft_before=inf soft_after=4096
QCORE_CAP GROW call=ftruncate request=8192 ret=-1 error=27 errno_read=1 len_before=4096 len_after=4096 blocks_before=8 blocks_after=8 transferred=0
QCORE_CAP GROW call=ftruncate request=8192 ret=-1 error=27 errno_read=1 len_before=4096 len_after=4096 blocks_before=8 blocks_after=8 transferred=0
QCORE_CAP LIMIT soft_before=4096 soft_after=32768
QCORE_CAP GROW call=ftruncate request=8192 ret=0 error=none errno_read=1 len_before=4096 len_after=8192 blocks_before=8 blocks_after=8 transferred=0
QCORE_CAP REMAP from=1 to=2 addr=0x73b9b2896000 len=8192 file_len=8192
QCORE_CAP LOGICAL len=4096
QCORE_CAP UNMAP gen=2 len=8192 ret=0
QCORE_CAP CLOSE gen=1 ret=0
QCORE_CAP END scenario=2 live_maps=0 live_fds=0
";

fn accept(trace: &str) -> crate::capacity_boundary::CapacityObservation {
    let events = parse_capacity_trace(trace).expect("the recorded trace parses");
    validate_capacity_trace(&events).expect("the recorded trace validates")
}

fn refuse(trace: &str) -> Vec<RefusalCode> {
    let events = parse_capacity_trace(trace).expect("the mutated trace still parses");
    let mut codes: Vec<RefusalCode> = validate_capacity_trace(&events)
        .expect_err("the mutation must be refused")
        .into_iter()
        .map(|refusal| refusal.code)
        .collect();
    codes.sort_unstable();
    codes.dedup();
    codes
}

fn mutate(trace: &str, from: &str, to: &str) -> String {
    assert!(trace.contains(from), "the mutation target is not present");
    trace.replacen(from, to, 1)
}

/// The recorded length-only grant validates, and the ledger keeps the added
/// range out of the backed prefix.
#[test]
fn the_recorded_length_only_grant_validates() {
    let observed = accept(LENGTH_ONLY);
    assert_eq!(observed.scenario, 0);
    assert_eq!(observed.mapping_epochs_acquired, 2);
    assert_eq!(observed.mapping_epochs_released, 2);
    assert_eq!(observed.descriptor_epochs_acquired, 1);
    assert_eq!(observed.descriptor_epochs_released, 1);
    assert!(observed.fully_released());
    assert_eq!(observed.final_file_len, GROWN);
    // The file is twice as long, and not one byte more is backed.
    assert_eq!(observed.final_backing, Backing::Reserved(SEED));
    assert_eq!(observed.final_logical_len, SEED);
    let classes: Vec<TransitionClass> = observed
        .transitions
        .iter()
        .map(|transition| transition.class)
        .collect();
    assert_eq!(
        classes,
        vec![
            TransitionClass::LengthOnlyGrant,
            TransitionClass::RemapAfterGrant,
            TransitionClass::Release,
            TransitionClass::Release,
        ]
    );
}

/// The recorded refusal, repeat and post-change grant validate in that order.
#[test]
fn the_recorded_refusal_and_retry_validate() {
    let observed = accept(REFUSAL_AND_RETRY);
    let classes: Vec<TransitionClass> = observed
        .transitions
        .iter()
        .map(|transition| transition.class)
        .collect();
    assert_eq!(
        classes,
        vec![
            TransitionClass::CapacityRefusal,
            TransitionClass::RepeatedRefusal,
            TransitionClass::GrantAfterCapacityChange,
            TransitionClass::RemapAfterGrant,
            TransitionClass::Release,
            TransitionClass::Release,
        ]
    );
    // The mapping epoch acquired before the refusals survived both of them and
    // was replaced only by the grant.
    assert_eq!(observed.mapping_epochs_acquired, 2);
    assert!(observed.fully_released());
}

/// Mutation: the release is deleted, leaving an epoch nobody accounts for.
#[test]
fn a_lost_residual_is_refused() {
    let mutated = mutate(LENGTH_ONLY, "QCORE_CAP UNMAP gen=2 len=8192 ret=0\n", "");
    assert_eq!(refuse(&mutated), vec![RefusalCode::LostResidual]);
}

/// Mutation: the residual is declared with the wrong count.
#[test]
fn a_misdeclared_residual_is_refused() {
    let mutated = mutate(
        LENGTH_ONLY,
        "live_maps=0 live_fds=0",
        "live_maps=1 live_fds=0",
    );
    assert_eq!(refuse(&mutated), vec![RefusalCode::LostResidual]);
}

/// Mutation: a mapping is claimed over a length the file never reached.
#[test]
fn a_forged_successful_mapping_is_refused() {
    let mutated = mutate(
        REFUSAL_AND_RETRY,
        "QCORE_CAP GROW call=ftruncate request=8192 ret=0 error=none errno_read=1 len_before=4096 len_after=8192 blocks_before=8 blocks_after=8 transferred=0\n",
        "",
    );
    assert_eq!(refuse(&mutated), vec![RefusalCode::ForgedWitness]);
}

/// Mutation: a mapping records a file length the ledger never observed.
#[test]
fn a_mapping_recording_a_forged_file_length_is_refused() {
    let mutated = mutate(
        LENGTH_ONLY,
        "len=4096 file_len=4096",
        "len=4096 file_len=8192",
    );
    assert_eq!(refuse(&mutated), vec![RefusalCode::ForgedWitness]);
}

/// Mutation: a released generation is acquired again.
#[test]
fn a_reused_generation_is_refused() {
    let mutated = mutate(LENGTH_ONLY, "REMAP from=1 to=2", "REMAP from=1 to=1");
    let codes = refuse(&mutated);
    assert!(
        codes.contains(&RefusalCode::ConfusedGeneration),
        "{codes:?}"
    );
}

/// Mutation: a generation goes backwards, so the second acquisition claims an
/// identity older than one already used.
#[test]
fn a_generation_that_goes_backwards_is_refused() {
    let mutated = LENGTH_ONLY
        .replace(
            "MAP gen=1 addr=0x7f8ca5a69000",
            "MAP gen=9 addr=0x7f8ca5a69000",
        )
        .replace("REMAP from=1 to=2", "REMAP from=9 to=2");
    assert_eq!(refuse(&mutated), vec![RefusalCode::ConfusedGeneration]);
}

/// Mutation: a remap claims to replace an epoch that is not live, which both
/// confuses the identity and abandons the epoch that really was live.
#[test]
fn a_remap_from_a_dead_epoch_is_refused() {
    let mutated = mutate(LENGTH_ONLY, "REMAP from=1 to=2", "REMAP from=7 to=2");
    assert_eq!(
        refuse(&mutated),
        vec![RefusalCode::ConfusedGeneration, RefusalCode::LostResidual]
    );
}

/// Mutation: the release names another length, which would leave part of the
/// mapping in place.
#[test]
fn a_release_at_another_length_is_refused() {
    let mutated = mutate(LENGTH_ONLY, "UNMAP gen=2 len=8192", "UNMAP gen=2 len=4096");
    assert_eq!(refuse(&mutated), vec![RefusalCode::EpochLengthMismatch]);
}

/// Mutation: the refusal is recorded as having grown the file anyway.
#[test]
fn a_refusal_that_grew_the_file_is_refused() {
    let mutated = mutate(
        REFUSAL_AND_RETRY,
        "ret=-1 error=27 errno_read=1 len_before=4096 len_after=4096 blocks_before=8 blocks_after=8",
        "ret=-1 error=27 errno_read=1 len_before=4096 len_after=8192 blocks_before=8 blocks_after=8",
    );
    let codes = refuse(&mutated);
    assert!(codes.contains(&RefusalCode::ForgedGrant), "{codes:?}");
}

/// Mutation: the refusal is recorded as having reserved the whole request.
#[test]
fn a_refusal_that_reserved_the_request_is_refused() {
    let mutated = mutate(
        REFUSAL_AND_RETRY,
        "ret=-1 error=27 errno_read=1 len_before=4096 len_after=4096 blocks_before=8 blocks_after=8",
        "ret=-1 error=27 errno_read=1 len_before=4096 len_after=4096 blocks_before=8 blocks_after=16",
    );
    let codes = refuse(&mutated);
    assert!(codes.contains(&RefusalCode::ForgedReservation), "{codes:?}");
}

/// Mutation: the capacity change is deleted, so the grant repeats a refused
/// request with nothing changed.
#[test]
fn a_grant_without_a_recorded_capacity_change_is_refused() {
    let mutated = mutate(
        REFUSAL_AND_RETRY,
        "QCORE_CAP LIMIT soft_before=4096 soft_after=32768\n",
        "",
    );
    let codes = refuse(&mutated);
    assert!(codes.contains(&RefusalCode::ForgedGrant), "{codes:?}");
}

/// Mutation: content is claimed inside the hole a length-only grant created.
#[test]
fn content_claimed_into_the_hole_is_refused() {
    let mutated = mutate(LENGTH_ONLY, "LOGICAL len=4096", "LOGICAL len=8192");
    assert_eq!(refuse(&mutated), vec![RefusalCode::ForgedWitness]);
}

/// Mutation: the outcome of a call is recorded as unobserved and still used.
#[test]
fn an_unobserved_outcome_is_refused() {
    let mutated = mutate(
        REFUSAL_AND_RETRY,
        "ret=-1 error=27 errno_read=1",
        "ret=-1 error=unobserved errno_read=0",
    );
    let codes = refuse(&mutated);
    assert!(codes.contains(&RefusalCode::UnobservedOutcome), "{codes:?}");
}

/// Mutation: `errno` is recorded as read from the call that does not set it.
#[test]
fn errno_read_from_the_returning_call_is_refused() {
    let trace = mutate(
        LENGTH_ONLY,
        "GROW call=ftruncate request=8192 ret=0 error=none errno_read=0",
        "GROW call=posix_fallocate request=8192 ret=-1 error=27 errno_read=1",
    );
    let codes = refuse(&trace);
    assert!(codes.contains(&RefusalCode::PremiseViolated), "{codes:?}");
}

/// Mutation: the trace stops before its teardown record.
#[test]
fn a_trace_without_an_end_record_is_refused() {
    let mutated = mutate(
        LENGTH_ONLY,
        "QCORE_CAP END scenario=0 live_maps=0 live_fds=0\n",
        "",
    );
    assert_eq!(refuse(&mutated), vec![RefusalCode::MissingTeardown]);
}

/// Mutation: an event is appended after the teardown record.
#[test]
fn an_event_after_the_end_record_is_refused() {
    let mutated = format!("{LENGTH_ONLY}QCORE_CAP CLOSE gen=1 ret=0\n");
    assert_eq!(refuse(&mutated), vec![RefusalCode::MalformedTrace]);
}

/// The parser is total: it refuses what it cannot read rather than guessing.
#[test]
fn the_parser_refuses_what_it_cannot_read() {
    for text in [
        "",
        "nothing here\n",
        "QCORE_CAP\n",
        "QCORE_CAP OPEN gen\n",
        "QCORE_CAP OPEN gen=1 gen=2\n",
        "QCORE_CAP OPEN gen=x inode=1 file_len=1 blocks=1\n",
        "QCORE_CAP WIDGET\n",
        "QCORE_CAP SIGNAL handled=2\n",
        "QCORE_CAP LIMIT soft_before=maybe soft_after=1\n",
    ] {
        let refusal = parse_capacity_trace(text).expect_err("must refuse");
        assert_eq!(refusal.code, RefusalCode::MalformedTrace, "{text:?}");
    }
}

/// A trace with no fixture record names no scenario and is refused.
#[test]
fn a_trace_without_a_fixture_record_is_refused() {
    let events = vec![CapacityEvent::End {
        scenario: 0,
        live_maps: 0,
        live_fds: 0,
    }];
    let codes: Vec<RefusalCode> = validate_capacity_trace(&events)
        .unwrap_err()
        .into_iter()
        .map(|refusal| refusal.code)
        .collect();
    assert!(codes.contains(&RefusalCode::MalformedTrace), "{codes:?}");
}

/// A failed release is a residual, not a release: the epoch stays live and the
/// declaration has to say so.
#[test]
fn a_failed_release_retains_the_epoch() {
    let mutated = mutate(
        LENGTH_ONLY,
        "UNMAP gen=2 len=8192 ret=0",
        "UNMAP gen=2 len=8192 ret=-1",
    );
    assert_eq!(refuse(&mutated), vec![RefusalCode::LostResidual]);
    let declared = mutate(&mutated, "live_maps=0 live_fds=0", "live_maps=1 live_fds=0");
    let observed = accept(&declared);
    assert_eq!(observed.mapping_epochs_released, 1);
    assert_eq!(
        observed.residuals,
        vec![Epoch {
            kind: EpochKind::Mapping,
            generation: 2,
            length: GROWN,
            address: Some(0x7f8c_a58b_d000),
        }]
    );
    assert!(!observed.fully_released());
}

// ---------------------------------------------------------------------------
// The native evidence.
// ---------------------------------------------------------------------------

/// Every scenario the fixture runs, with the transition classes its recorded
/// trace must carry.
#[cfg(target_os = "linux")]
const NATIVE_SCENARIOS: [(u64, &[TransitionClass]); 6] = [
    (
        0,
        &[
            TransitionClass::LengthOnlyGrant,
            TransitionClass::RemapAfterGrant,
            TransitionClass::Release,
            TransitionClass::Release,
        ],
    ),
    (
        1,
        &[
            TransitionClass::ReservedGrant,
            TransitionClass::RemapAfterGrant,
            TransitionClass::Release,
            TransitionClass::Release,
        ],
    ),
    (
        2,
        &[
            TransitionClass::CapacityRefusal,
            TransitionClass::RepeatedRefusal,
            TransitionClass::GrantAfterCapacityChange,
            TransitionClass::RemapAfterGrant,
            TransitionClass::Release,
            TransitionClass::Release,
        ],
    ),
    (
        3,
        &[
            TransitionClass::ReturnedErrorWithoutErrno,
            TransitionClass::Release,
            TransitionClass::Release,
        ],
    ),
    (
        4,
        &[
            TransitionClass::PartialTransfer,
            TransitionClass::CapacityRefusal,
            TransitionClass::Release,
            TransitionClass::Release,
        ],
    ),
    (
        5,
        &[
            TransitionClass::Release,
            TransitionClass::MappingBeyondEndOfFile,
            TransitionClass::Release,
            TransitionClass::Release,
        ],
    ),
];

/// Six real, unsuppressed sequences on this Linux host: every syscall reaches
/// the kernel, the capacity refusal is the kernel's own `EFBIG` under
/// `RLIMIT_FSIZE`, and each recorded trace is validated by the relation.
///
/// The claim is exactly this profile: `x86_64` Linux, a 4096-byte seed file
/// grown to 8192 bytes, one stream directory per process under the system
/// temporary directory, one descriptor epoch and at most two mapping epochs,
/// no concurrency, ASan/UBSan/LSan enabled. Nothing about other sizes, other
/// filesystems, other targets or `ENOSPC` on a full device follows.
#[cfg(target_os = "linux")]
#[test]
fn the_native_capacity_scenarios_validate() {
    use std::fs;
    use std::process::Command;

    let root = std::env::temp_dir().join(format!("qcore-o02-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir(&root).unwrap();
    let source = root.join("file_capacity_observation.c");
    let binary = root.join("file_capacity_observation");
    fs::write(
        &source,
        include_str!("../fixtures/file_capacity_observation.c"),
    )
    .unwrap();

    let compiler = std::env::var("QCORE_CLANG").unwrap_or_else(|_| "cc".to_owned());
    let compiled = Command::new(&compiler)
        .args([
            "-std=c11",
            "-Wall",
            "-Wextra",
            "-Werror",
            "-pedantic",
            "-g",
            "-O0",
            "-fsanitize=address,undefined",
            "-fno-omit-frame-pointer",
        ])
        .arg(&source)
        .arg("-o")
        .arg(&binary)
        .output()
        .unwrap();
    assert!(
        compiled.status.success(),
        "the C compiler rejected the capacity fixture:\n{}",
        String::from_utf8_lossy(&compiled.stderr)
    );

    for (scenario, expected) in NATIVE_SCENARIOS {
        let directory = root.join(format!("scenario-{scenario}"));
        fs::create_dir(&directory).unwrap();
        let run = Command::new(&binary)
            .arg(scenario.to_string())
            .arg(&directory)
            .env("ASAN_OPTIONS", "detect_leaks=1:halt_on_error=1")
            .env("UBSAN_OPTIONS", "halt_on_error=1:print_stacktrace=1")
            .output()
            .unwrap();
        let text = String::from_utf8(run.stdout).unwrap();
        assert!(
            run.status.success(),
            "scenario {scenario} did not complete:\n{}\n{text}",
            String::from_utf8_lossy(&run.stderr)
        );
        let events = parse_capacity_trace(&text)
            .unwrap_or_else(|refusal| panic!("scenario {scenario}: {refusal}\n{text}"));
        let observed = validate_capacity_trace(&events).unwrap_or_else(|refusals| {
            panic!(
                "scenario {scenario} was refused:\n{}\n{text}",
                refusals
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        });
        assert_eq!(observed.scenario, scenario);
        let classes: Vec<TransitionClass> = observed
            .transitions
            .iter()
            .map(|transition| transition.class)
            .collect();
        assert_eq!(classes, expected.to_vec(), "scenario {scenario}\n{text}");
        assert!(
            observed.fully_released(),
            "scenario {scenario} left a residual:\n{text}"
        );
        // The fixture removes its own file; the directory must be empty.
        assert_eq!(fs::read_dir(&directory).unwrap().count(), 0);
        fs::remove_dir(&directory).unwrap();

        match scenario {
            // A length-only grant doubles the length and backs nothing more.
            0 => {
                assert_eq!(observed.final_file_len, GROWN);
                assert_eq!(observed.final_backing, Backing::Reserved(SEED));
            }
            // The allocating call backs everything it grew.
            1 => {
                assert_eq!(observed.final_file_len, GROWN);
                assert_eq!(observed.final_backing, Backing::Reserved(GROWN));
                assert_eq!(observed.final_logical_len, GROWN);
            }
            // Two refusals left the length exactly where it was.
            2 => {
                let refusals: Vec<&GrowPostState> = observed
                    .transitions
                    .iter()
                    .filter_map(|transition| transition.post.as_ref())
                    .filter(|post| post.recovery == Recovery::RetryAfterCapacityChange)
                    .collect();
                assert_eq!(refusals.len(), 2);
                for post in refusals {
                    assert_eq!(post.file_length, FileLength::Unchanged(SEED));
                    assert_eq!(post.mapping, MappingDuty::RetainExisting);
                }
            }
            // The error arrived as a return value; `errno` was never read.
            3 => {
                assert_eq!(observed.final_file_len, SEED);
                assert!(events.iter().any(|event| matches!(
                    event,
                    CapacityEvent::Grow {
                        call: GrowCall::PosixFallocate,
                        errno_read: false,
                        error: CapacityErrno::Efbig,
                        ret: 27,
                        ..
                    }
                )));
            }
            // Four of the eight requested bytes moved, and the content
            // advanced by four.
            4 => {
                assert_eq!(observed.final_file_len, SEED + 4);
                assert_eq!(observed.final_logical_len, SEED + 4);
                let partial = observed
                    .transitions
                    .iter()
                    .find(|transition| transition.class == TransitionClass::PartialTransfer)
                    .and_then(|transition| transition.post.clone())
                    .expect("a partial transfer");
                assert_eq!(partial.logical_advance, 4);
                assert_eq!(partial.recovery, Recovery::RetryFromObservedLength);
            }
            // A live mapping twice the length of the file witnesses nothing
            // past the end of it.
            5 => {
                let mapped = events
                    .iter()
                    .filter_map(|event| match event {
                        CapacityEvent::Map { len, file_len, .. } => Some((*len, *file_len)),
                        _ => None,
                    })
                    .next_back()
                    .unwrap();
                assert_eq!(mapped, (GROWN, SEED));
                let witness = mapping_witness(mapped.0, mapped.1, observed.final_backing);
                assert_eq!(witness.beyond_end_of_file, SEED);
                assert_eq!(witness.usable, SEED);
            }
            _ => unreachable!(),
        }
    }
    fs::remove_dir_all(&root).unwrap();
}

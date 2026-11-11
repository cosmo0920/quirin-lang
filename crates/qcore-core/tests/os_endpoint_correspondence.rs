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

//! Rust/Lean agreement for the target-qualified `close(2)` post-state relation
//! (card O03-U4).
//!
//! O03 shipped two statements of one relation —
//! `crates/qcore-core/src/os_endpoint.rs` and `QCore.OsEndpointPostState` —
//! written to agree and checked separately, with no link between them. U4 makes
//! the declared descriptor-custody model depend on that relation, so a silent
//! divergence between the two developments would now be a divergence between
//! the Lean model and the Rust checker. This test closes that gap the way F01
//! did for `check_concurrent`: both sides enumerate the same domain in the same
//! order, encode every derived post-state with the same arithmetic, and pin the
//! same two numbers.
//!
//! What this establishes, and what it does not:
//!
//! * it establishes that the two implementations of the relation answer
//!   identically on **every** input of the declared `3 x 64 x 10` domain — a
//!   difference in any field of any answer moves the digest;
//! * it is a comparison device, not a proof device. Two different encodings
//!   agreeing on a digest would be a collision, and the encoding below is
//!   transcribed by hand from `QCore.DescriptorCustody`, not extracted from it.
//!   Nothing here proves either side faithful to the kernel; that stays a
//!   premise read from the man page, as `docs/design/OS_FAILURE_POSTSTATE_PROFILE.md`
//!   §14 records.
//!
//! The Lean counterparts are `QCore.DescriptorCustody.close_relation_domain_size`
//! and `QCore.DescriptorCustody.close_relation_digest`.

use qcore_core::os_endpoint::{
    DescriptionState, DescriptorState, Duty, Errno, Outcome, PostState, Premises, Recovery, Target,
    WriteBackState, close_poststate,
};

/// `QCore.DescriptorCustody.mixMul`.
const MIX_MUL: u64 = 1000003;
/// `QCore.DescriptorCustody.stateSeed`.
const STATE_SEED: u64 = 1469598103934665603;
/// `QCore.DescriptorCustody.domainSeed`.
const DOMAIN_SEED: u64 = 14695981039346656037;
/// `QCore.DescriptorCustody.domainMul`.
const DOMAIN_MUL: u64 = 1099511628211;

/// The pinned numbers. Both are recomputed independently by the Lean theorems
/// named above; a change on either side must move both deliberately.
const DOMAIN_SIZE: usize = 1920;
const RELATION_DIGEST: u64 = 11372109764820645253;

fn mix(accumulator: u64, value: u64) -> u64 {
    accumulator
        .wrapping_mul(MIX_MUL)
        .wrapping_add(value)
        .wrapping_add(1)
}

fn encode_target(target: Target) -> u64 {
    match target {
        Target::Linux => 0,
        Target::Posix2024 => 1,
        Target::Unpinned => 2,
    }
}

fn encode_errno(errno: Errno) -> u64 {
    match errno {
        Errno::Ebadf => 0,
        Errno::Eintr => 1,
        Errno::Eio => 2,
        Errno::Enospc => 3,
        Errno::Edquot => 4,
        Errno::Other => 5,
        Errno::Unobserved => 6,
    }
}

fn encode_outcome(outcome: Outcome) -> u64 {
    match outcome {
        Outcome::NotAttempted => 0,
        Outcome::ResultUnobserved => 1,
        Outcome::Zero => 2,
        Outcome::MinusOne(errno) => 3 + encode_errno(errno),
    }
}

fn encode_premises(premises: Premises) -> u64 {
    u64::from(premises.epoch_live)
        + 2 * u64::from(premises.sole_custodian)
        + 4 * u64::from(premises.result_observed)
        + 8 * u64::from(premises.errno_observed)
        + 16 * u64::from(premises.last_reference)
        + 32 * u64::from(premises.no_record_locks)
}

fn encode_input(target: Target, premises: Premises, outcome: Outcome) -> u64 {
    encode_target(target) + 3 * (encode_premises(premises) + 64 * encode_outcome(outcome))
}

fn encode_descriptor(state: DescriptorState) -> u64 {
    match state {
        DescriptorState::NotAttempted => 0,
        DescriptorState::Attempted => 1,
        DescriptorState::Retained => 2,
        DescriptorState::Released => 3,
        DescriptorState::Indeterminate => 4,
    }
}

fn encode_description(state: DescriptionState) -> u64 {
    match state {
        DescriptionState::Retained => 0,
        DescriptionState::Released => 1,
        DescriptionState::Unknown => 2,
    }
}

fn encode_writeback(state: WriteBackState) -> u64 {
    match state {
        WriteBackState::Failed => 0,
        WriteBackState::Unknown => 1,
        WriteBackState::Discharged => 2,
    }
}

fn encode_duties(state: &PostState) -> u64 {
    let bit = |duty: Duty, weight: u64| {
        if state.duties.contains(&duty) {
            weight
        } else {
            0
        }
    };
    bit(Duty::ReleaseEpoch, 1)
        + bit(Duty::NoUse, 2)
        + bit(Duty::NoRetryClose, 4)
        + bit(Duty::NoTreatNumberAsFree, 8)
        + bit(Duty::KeepInDenominator, 16)
        + bit(Duty::ReportUnresolved, 32)
        + bit(Duty::ReportPossibleDataLoss, 64)
}

/// `0` is the out-of-domain answer, matching Lean's `none`.
fn encode_poststate(state: Option<&PostState>) -> u64 {
    let Some(state) = state else {
        return 0;
    };
    let may_close = u64::from(state.permitted_recovery.contains(&Recovery::Close));
    1 + encode_target(state.target)
        + 8 * (encode_descriptor(state.descriptor)
            + 8 * (encode_description(state.description)
                + 4 * (encode_writeback(state.writeback)
                    + 4 * (encode_duties(state) + 128 * may_close))))
}

/// The domain in `QCore.OsEndpointPostState`'s own list order:
/// `allTargets`, then `allPremises` (`epochLive` outermost, `noRecordLocks`
/// innermost, `false` before `true`), then `allOutcomes`.
fn lean_domain() -> Vec<(Target, Premises, Outcome)> {
    let bools = [false, true];
    let mut out = Vec::new();
    for target in [Target::Linux, Target::Posix2024, Target::Unpinned] {
        for epoch_live in bools {
            for sole_custodian in bools {
                for result_observed in bools {
                    for errno_observed in bools {
                        for last_reference in bools {
                            for no_record_locks in bools {
                                let premises = Premises {
                                    epoch_live,
                                    sole_custodian,
                                    result_observed,
                                    errno_observed,
                                    last_reference,
                                    no_record_locks,
                                };
                                let mut outcomes = vec![
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
                                    outcomes.push(Outcome::MinusOne(errno));
                                }
                                for outcome in outcomes {
                                    out.push((target, premises, outcome));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    out
}

fn relation_digest(domain: &[(Target, Premises, Outcome)]) -> u64 {
    domain
        .iter()
        .fold(DOMAIN_SEED, |accumulator, &(target, premises, outcome)| {
            let derived = close_poststate(target, premises, outcome).ok();
            let entry = mix(
                mix(STATE_SEED, encode_input(target, premises, outcome)),
                encode_poststate(derived.as_ref()),
            );
            accumulator
                .wrapping_mul(DOMAIN_MUL)
                .wrapping_add(entry)
                .wrapping_add(1)
        })
}

#[test]
fn the_enumerated_domain_is_the_one_the_lean_model_enumerates() {
    let domain = lean_domain();
    assert_eq!(domain.len(), DOMAIN_SIZE);
    // Every triple appears exactly once: the enumeration is the domain, not a
    // sample of it.
    let mut seen: Vec<_> = domain
        .iter()
        .map(|&(t, p, o)| encode_input(t, p, o))
        .collect();
    seen.sort_unstable();
    seen.dedup();
    assert_eq!(seen.len(), DOMAIN_SIZE);
}

#[test]
fn the_relation_agrees_with_the_lean_model_over_the_whole_domain() {
    assert_eq!(relation_digest(&lean_domain()), RELATION_DIGEST);
}

// ---------------------------------------------------------------------------
// Card O03-REACTOR: the reactor-side arm of the same correspondence
// ---------------------------------------------------------------------------
//
// The reactor now resolves a declared `(target, errno, record-locks)` premise
// and reads its disposition out of the same relation. That classification is a
// second implementation surface, so the digest grows to cover it: the domain is
// the three closed vocabularies plus **one representative unrecognized
// spelling** per component, which is faithful because every unrecognized
// spelling takes the same refusal (pinned separately by
// `an_unrecognized_declared_spelling_refuses_the_event` in
// `crates/qcore-core/tests/unit/event_reactor.rs`).
//
// The relation's own 1920-triple digest above is deliberately **kept**: the
// relation did not change, so its pin must not move, and the extended digest is
// the old fold continued over the new rows.

use qcore_core::event_reactor::{
    ClosePremiseDisposition, DeclaredCloseFailure, classify_declared_close_failure,
};

/// `QCore.DescriptorCustody.reactorDomainOffset`.
const REACTOR_INPUT_OFFSET: u64 = 4096;

/// The combined domain: the relation's 1920 triples, then the reactor arm's
/// `4 x 8 x 3` declared rows.
const ADMISSION_DOMAIN_SIZE: usize = 2016;
/// Recomputed after the reactor arm was added. The pre-card value over the
/// 1920-row domain alone is `RELATION_DIGEST` above, which is unchanged.
const ADMISSION_DIGEST: u64 = 2640189996995932678;

/// The representative spellings, in `QCore.DescriptorCustody`'s list order. The
/// last entry of each list is the unrecognized class.
const DECLARED_TARGETS: [&str; 4] = ["linux", "posix-2024", "unpinned", "unrecognized-target"];
const DECLARED_ERRNOS: [&str; 8] = [
    "ebadf",
    "eintr",
    "eio",
    "enospc",
    "edquot",
    "other",
    "unobserved",
    "unrecognized-errno",
];
const DECLARED_LOCKS: [&str; 3] = ["none-held", "unknown", "unrecognized-locks"];

fn encode_reactor_input(target: usize, errno: usize, locks: usize) -> u64 {
    REACTOR_INPUT_OFFSET + target as u64 + 4 * (errno as u64 + 8 * locks as u64)
}

/// `0` stays reserved for the out-of-domain answer, matching the relation's
/// encoding, so the four dispositions occupy `1..=4`.
fn encode_disposition(answer: Option<ClosePremiseDisposition>) -> u64 {
    match answer {
        // A refused declaration.
        None => 1,
        Some(ClosePremiseDisposition::ReleasedNumberRetired) => 2,
        Some(ClosePremiseDisposition::RetainedRetryPermitted) => 3,
        Some(ClosePremiseDisposition::Poisoned) => 4,
    }
}

fn reactor_domain() -> Vec<(usize, usize, usize)> {
    let mut out = Vec::new();
    for target in 0..DECLARED_TARGETS.len() {
        for errno in 0..DECLARED_ERRNOS.len() {
            for locks in 0..DECLARED_LOCKS.len() {
                out.push((target, errno, locks));
            }
        }
    }
    out
}

/// Continue the relation's fold over the reactor rows.
fn admission_digest() -> u64 {
    reactor_domain().iter().fold(
        relation_digest(&lean_domain()),
        |accumulator, &(target, errno, locks)| {
            let declared = DeclaredCloseFailure {
                target: DECLARED_TARGETS[target].to_string(),
                errno: DECLARED_ERRNOS[errno].to_string(),
                record_locks: DECLARED_LOCKS[locks].to_string(),
            };
            let answer = classify_declared_close_failure(&declared)
                .ok()
                .map(|classified| classified.disposition);
            let entry = mix(
                mix(STATE_SEED, encode_reactor_input(target, errno, locks)),
                encode_disposition(answer),
            );
            accumulator
                .wrapping_mul(DOMAIN_MUL)
                .wrapping_add(entry)
                .wrapping_add(1)
        },
    )
}

#[test]
fn the_extended_domain_is_the_one_the_lean_model_enumerates() {
    let reactor = reactor_domain();
    assert_eq!(lean_domain().len() + reactor.len(), ADMISSION_DOMAIN_SIZE);
    let mut seen: Vec<_> = reactor
        .iter()
        .map(|&(t, e, l)| encode_reactor_input(t, e, l))
        .collect();
    seen.sort_unstable();
    seen.dedup();
    assert_eq!(seen.len(), reactor.len());
    // The reactor rows cannot collide with the relation rows.
    assert!(
        seen[0]
            > lean_domain()
                .iter()
                .map(|&(t, p, o)| encode_input(t, p, o))
                .max()
                .unwrap()
    );
}

#[test]
fn the_reactor_arm_agrees_with_the_lean_model_over_the_whole_domain() {
    assert_eq!(admission_digest(), ADMISSION_DIGEST);
}

/// Print the numbers when they must be re-pinned. Not a gate.
#[test]
#[ignore = "recompute the pinned digest deliberately"]
fn recompute_pins() {
    let domain = lean_domain();
    println!("DOMAIN_SIZE {}", domain.len());
    println!("RELATION_DIGEST {}", relation_digest(&domain));
    println!(
        "ADMISSION_DOMAIN_SIZE {}",
        domain.len() + reactor_domain().len()
    );
    println!("ADMISSION_DIGEST {}", admission_digest());
}

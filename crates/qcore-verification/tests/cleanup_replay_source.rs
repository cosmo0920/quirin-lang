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

//! Core-side admission and refusal matrix for the fourth transported source
//! vocabulary: the symbolic cleanup replay.
//!
//! Every row here drives the real gateway import, the real task derivation and
//! the real trusted checker. Nothing is stubbed, and no row asserts anything
//! about the OS: a recorded `close`, `munmap` or `unlink` call is a recorded
//! call.
#![cfg(not(target_os = "windows"))]

#[path = "fixtures/cleanup_replay_source.rs"]
mod fixture;
use fixture::*;

use qcore_schemas::cleanup_replay_summary::*;

#[test]
fn the_three_selected_bodies_replay_their_whole_sampled_prestate_domains() {
    if !clang_available() {
        return;
    }
    for (source, function, records, cases) in [
        (OUTCOMES_SOURCE, CLOSE, &OUTCOMES_RECORDS[..], 4),
        (OUTCOMES_SOURCE, UNMAP, &OUTCOMES_RECORDS[..], 4),
        (DELETE_SOURCE, DELETE, &DELETE_RECORDS[..], 5),
    ] {
        let (_, task) = route(source, function, records, UNIX_TU);
        let reading = core_reading(&task);
        assert_eq!(reading.record().function, function);
        assert_eq!(reading.record().cases.len(), cases);
        // The case vector answers exactly the family's fixed sampled domain,
        // in order.
        let prestates: Vec<_> = reading
            .record()
            .cases
            .iter()
            .map(CleanupReplayCaseRecord::prestate)
            .collect();
        let expected = match &task.task().semantic_profile {
            _ if cases == 4 => CleanupPrestateRecord::native_cleanup_outcome_domain(),
            _ => CleanupPrestateRecord::guarded_delete_domain(),
        };
        assert_eq!(prestates, expected);
        accept(&task, &witness_for(&task)).unwrap();
    }
}

/// The successful arm's field transition is what makes this a *replay* and not
/// a syntactic fold: the writes and the reset flag are functions of the
/// evolved lattice state.
#[test]
fn the_successful_arm_reports_the_declared_reset_and_the_failure_arm_preserves() {
    if !clang_available() {
        return;
    }
    let (_, task) = route(OUTCOMES_SOURCE, UNMAP, &OUTCOMES_RECORDS, UNIX_TU);
    let CleanupReplayProposalRecord::NativeCleanupOutcome(record) = core_reading(&task) else {
        panic!("wrong family")
    };
    let CleanupReplayCaseRecord::NativeCleanupOutcome {
        field_writes,
        fields_reset,
        cleanup_calls,
        returned,
        initial_fields,
        ..
    } = &record.cases[3]
    else {
        panic!("wrong case variant")
    };
    assert_eq!(field_writes, &["alloc_size", "map", "map_truncated_warned"]);
    assert!(fields_reset);
    assert_eq!(*cleanup_calls, 1);
    assert_eq!(*returned, 0);
    // The prestate's lattice state distinguishes pointer identity from an
    // opaque scalar identity, which is what the syscall argument check needs.
    assert_eq!(
        initial_fields["map"],
        CleanupLatticeValue::Pointer {
            field: "map".into()
        }
    );
    assert_eq!(
        initial_fields["alloc_size"],
        CleanupLatticeValue::Symbol {
            field: "alloc_size".into()
        }
    );
    let CleanupReplayCaseRecord::NativeCleanupOutcome {
        fields_preserved,
        field_writes,
        returned,
        ..
    } = &record.cases[2]
    else {
        panic!("wrong case variant")
    };
    assert!(*fields_preserved && field_writes.is_empty() && *returned == -1);
    // The `Absent` prestate starts from the declared reset value, not from an
    // opaque symbol, which is how the eligibility guard decides.
    let CleanupReplayCaseRecord::NativeCleanupOutcome { initial_fields, .. } = &record.cases[1]
    else {
        panic!("wrong case variant")
    };
    assert_eq!(initial_fields["map"], CleanupLatticeValue::Null);
}

/// Every arm of the guarded delete preserves the record, and only the fully
/// eligible arms reach the syscall at all.
#[test]
fn the_guarded_delete_reaches_its_syscall_only_when_both_fields_are_clear() {
    if !clang_available() {
        return;
    }
    let (_, task) = route(DELETE_SOURCE, DELETE, &DELETE_RECORDS, UNIX_TU);
    let CleanupReplayProposalRecord::GuardedDelete(record) = core_reading(&task) else {
        panic!("wrong family")
    };
    for case in &record.cases {
        let CleanupReplayCaseRecord::GuardedDelete {
            descriptor_field_active,
            mapping_field_active,
            syscall_result,
            syscall_calls,
            diagnostic_calls,
            fields_preserved,
            returned,
            initial_fields,
            ..
        } = case
        else {
            panic!("wrong case variant")
        };
        assert!(fields_preserved);
        let eligible = !descriptor_field_active && !mapping_field_active;
        assert_eq!(*syscall_calls, u32::from(eligible));
        assert_eq!(*diagnostic_calls, u32::from(*syscall_result == Some(-1)));
        assert_eq!(*returned, if *syscall_result == Some(0) { 0 } else { -1 });
        // The eligibility overrides are visible in the prestate lattice.
        assert_eq!(
            initial_fields["fd"],
            if *descriptor_field_active {
                CleanupLatticeValue::Symbol { field: "fd".into() }
            } else {
                CleanupLatticeValue::Integer { value: -1 }
            }
        );
        assert_eq!(
            initial_fields["map"],
            if *mapping_field_active {
                CleanupLatticeValue::Pointer {
                    field: "map".into(),
                }
            } else {
                CleanupLatticeValue::Null
            }
        );
    }
}

/// The request carries the question and nothing else: no case vector, no field
/// state, no verdict, no contract text and no policy name.
#[test]
fn no_derived_reading_and_no_policy_travels_with_the_request() {
    if !clang_available() {
        return;
    }
    let (_, task) = route(OUTCOMES_SOURCE, CLOSE, &OUTCOMES_RECORDS, UNIX_TU);
    let request = task.request(qcore_schemas::PluginId::new("qcore.chunkio").unwrap());
    let input = request.cleanup_replay_source.as_ref().unwrap();
    let text = serde_json::to_string(input).unwrap();
    for derived in [
        "cases",
        "field_writes",
        "fields_preserved",
        "fields_reset",
        "source_path",
        "returned",
        "cleanup_calls",
        "verdict",
        "certificate",
        "ensures",
        "native_close_outcomes",
    ] {
        assert!(!text.contains(derived), "`{derived}` travelled");
    }
    // What does travel: the paths, the declared profile and the prerequisite
    // triple.
    let value: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert!(value["family"]["body"]["paths"].is_array());
    assert_eq!(value["family"]["profile"]["syscall"], "close");
    assert_eq!(
        value["prerequisite"]["semantic_profile"],
        "qcore.c_call_result_use.v1"
    );
}

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

//! Card `P04-COMPOSE-FRAME`: the two teardown capabilities gain the
//! prerequisite surface every dependent needs.
//!
//! `memory_close` consumes four readings. The frame helper was the recorded
//! structural blocker, but a census of the other three measured a second one:
//! `CounterSourceTask` had `prerequisite_identity` / `revalidate_as_prerequisite`
//! and `ListUnlinkSourceTask` / `FieldDestructorSourceTask` did not, so two of
//! the four could not be named either. Both surfaces land here, in the shape
//! the counter and cleanup routes already use.
//!
//! Nothing about what those routes *check* changes: `revalidate_as_prerequisite`
//! confirms the claimed triple is the one this context derives and then calls
//! the same unedited `evaluate`.

#[path = "fixtures/teardown_source.rs"]
mod fixture;
use fixture::*;

use qcore_schemas::call_use_summary::PrerequisiteIdentity;
use qcore_verification::teardown_source::{
    FIELD_DESTRUCTOR_SEMANTIC_PROFILE, LIST_UNLINK_SEMANTIC_PROFILE,
};

#[test]
fn the_unlink_task_can_be_named_and_re_established() {
    if !clang_available() {
        return;
    }
    let import = unlink_import(UNLINK_SOURCE, Vec::new());
    let task = unlink_task(&import, UNLINK_CONTRACT);
    let identity = task.prerequisite_identity();
    assert!(identity.is_complete());
    assert_eq!(identity.semantic_profile, LIST_UNLINK_SEMANTIC_PROFILE);
    assert_eq!(identity.task_digest, task.task().task_digest());
    assert_eq!(
        identity.obligation_id,
        task.task().obligations[0].obligation_id
    );
    // Re-establishment recomputes; it never returns a stored reading.
    assert_eq!(
        task.revalidate_as_prerequisite(&identity).unwrap(),
        task.evaluate().unwrap()
    );
}

#[test]
fn the_destructor_task_can_be_named_and_re_established() {
    if !clang_available() {
        return;
    }
    let import = destructor_import(DESTRUCTOR_SOURCE, Vec::new());
    let task = destructor_task(&import, DESTRUCTOR_CONTRACT);
    let identity = task.prerequisite_identity();
    assert!(identity.is_complete());
    assert_eq!(identity.semantic_profile, FIELD_DESTRUCTOR_SEMANTIC_PROFILE);
    assert_eq!(identity.task_digest, task.task().task_digest());
    assert_eq!(
        identity.obligation_id,
        task.task().obligations[0].obligation_id
    );
    assert_eq!(
        task.revalidate_as_prerequisite(&identity).unwrap(),
        task.evaluate().unwrap()
    );
}

#[test]
fn every_teardown_prerequisite_field_is_load_bearing() {
    if !clang_available() {
        return;
    }
    let unlink_import = unlink_import(UNLINK_SOURCE, Vec::new());
    let unlink = unlink_task(&unlink_import, UNLINK_CONTRACT);
    let destructor_import = destructor_import(DESTRUCTOR_SOURCE, Vec::new());
    let destructor = destructor_task(&destructor_import, DESTRUCTOR_CONTRACT);

    let honest = unlink.prerequisite_identity();
    for (row, mutated) in [
        (
            "semantic_profile",
            PrerequisiteIdentity {
                semantic_profile: FIELD_DESTRUCTOR_SEMANTIC_PROFILE.into(),
                ..honest.clone()
            },
        ),
        (
            "task_digest",
            PrerequisiteIdentity {
                task_digest: "0".repeat(64),
                ..honest.clone()
            },
        ),
        (
            "obligation_id",
            PrerequisiteIdentity {
                obligation_id: "resource_contracts:000000000000000000000000".into(),
                ..honest.clone()
            },
        ),
    ] {
        assert_ne!(mutated, honest, "{row}: the row must really differ");
        assert_eq!(
            unlink.revalidate_as_prerequisite(&mutated).unwrap_err(),
            "list unlink prerequisite identity is not the one the core derives",
            "{row}"
        );
    }
    for row in ["semantic_profile", "task_digest", "obligation_id"] {
        let mut incomplete = honest.clone();
        match row {
            "semantic_profile" => incomplete.semantic_profile.clear(),
            "task_digest" => incomplete.task_digest.clear(),
            _ => incomplete.obligation_id.clear(),
        }
        assert_eq!(
            unlink.revalidate_as_prerequisite(&incomplete).unwrap_err(),
            "list unlink prerequisite identity is incomplete",
            "{row}"
        );
    }

    // Neither capability accepts the other's identity, in either direction.
    assert_eq!(
        unlink
            .revalidate_as_prerequisite(&destructor.prerequisite_identity())
            .unwrap_err(),
        "list unlink prerequisite identity is not the one the core derives"
    );
    assert_eq!(
        destructor
            .revalidate_as_prerequisite(&unlink.prerequisite_identity())
            .unwrap_err(),
        "field destructor prerequisite identity is not the one the core derives"
    );
    let mut incomplete = destructor.prerequisite_identity();
    incomplete.task_digest.clear();
    assert_eq!(
        destructor
            .revalidate_as_prerequisite(&incomplete)
            .unwrap_err(),
        "field destructor prerequisite identity is incomplete"
    );
}

#[test]
fn a_moved_teardown_input_retires_its_prerequisite_identity() {
    if !clang_available() {
        return;
    }
    // A build change the projection cannot see still retires the identity, in
    // both directions, on both routes.
    let honest = unlink_import(UNLINK_SOURCE, Vec::new());
    let honest = unlink_task(&honest, UNLINK_CONTRACT);
    let moved = unlink_import(UNLINK_SOURCE, vec!["QCORE_UNLINK_UNUSED=1".into()]);
    let moved = unlink_task(&moved, UNLINK_CONTRACT);
    assert_ne!(honest.task().task_digest(), moved.task().task_digest());
    honest
        .revalidate_as_prerequisite(&moved.prerequisite_identity())
        .expect_err("a stale identity is refused");
    moved
        .revalidate_as_prerequisite(&honest.prerequisite_identity())
        .expect_err("and in the other direction");

    let honest = destructor_import(DESTRUCTOR_SOURCE, Vec::new());
    let honest = destructor_task(&honest, DESTRUCTOR_CONTRACT);
    let moved = destructor_import(DESTRUCTOR_SOURCE, vec!["QCORE_DESTRUCTOR_UNUSED=1".into()]);
    let moved = destructor_task(&moved, DESTRUCTOR_CONTRACT);
    assert_ne!(honest.task().task_digest(), moved.task().task_digest());
    honest
        .revalidate_as_prerequisite(&moved.prerequisite_identity())
        .expect_err("a stale identity is refused");
    moved
        .revalidate_as_prerequisite(&honest.prerequisite_identity())
        .expect_err("and in the other direction");
}

#[test]
fn all_four_memory_close_dependencies_are_now_nameable() {
    if !clang_available() {
        return;
    }
    // The census this card closes. `memory_close` consumes a frame-helper
    // reading, a nullable-field destructor reading, a list-unlink reading and
    // a counter reading. Every one of the four now has a task whose
    // three-field identity a dependent can name and whose evaluation the core
    // can re-run. The four identities are pairwise distinct, so no slot can
    // absorb another's.
    #[path = "fixtures/frame_source.rs"]
    mod frame;

    let frame_task = frame::chunk_task();
    let unlink_import = unlink_import(UNLINK_SOURCE, Vec::new());
    let unlink = unlink_task(&unlink_import, UNLINK_CONTRACT);
    let destructor_import = destructor_import(DESTRUCTOR_SOURCE, Vec::new());
    let destructor = destructor_task(&destructor_import, DESTRUCTOR_CONTRACT);

    let identities = [
        frame_task.prerequisite_identity(),
        unlink.prerequisite_identity(),
        destructor.prerequisite_identity(),
    ];
    for identity in &identities {
        assert!(identity.is_complete());
    }
    let distinct: std::collections::BTreeSet<_> = identities.iter().collect();
    assert_eq!(
        distinct.len(),
        identities.len(),
        "no two dependency slots share an identity"
    );
    let profiles: std::collections::BTreeSet<&str> = identities
        .iter()
        .map(|i| i.semantic_profile.as_str())
        .collect();
    assert_eq!(profiles.len(), identities.len());
    // The counter route's surface has existed since P04-NEXT-AB; this test
    // covers the three that did not.
}

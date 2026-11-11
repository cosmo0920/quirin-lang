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

use qcore_core::callback_task_handoff::{
    ArgumentLifetime, HandoffDiagnostic, HandoffOperation as Op, check_callback_task_handoff,
};

fn prefix() -> Vec<Op> {
    vec![Op::CallbackEnter {
        callback: 1,
        argument: 9,
        argument_lifetime: ArgumentLifetime::Owned,
    }]
}

#[test]
fn completed_task_and_post_return_deferred_work_are_discharged() {
    let mut operations = prefix();
    operations.extend([
        Op::SpawnTask {
            callback: 1,
            task: 2,
            retains_argument: true,
        },
        Op::StartTask { task: 2 },
        Op::RegisterDeferred {
            callback: 1,
            work: 3,
            retains_argument: true,
        },
        Op::CallbackExit { callback: 1 },
        Op::CompleteTask { task: 2 },
        Op::JoinTask {
            callback: 1,
            task: 2,
        },
        Op::StartDeferred { work: 3 },
        Op::CompleteDeferred { work: 3 },
        Op::DropArgument { argument: 9 },
        Op::Teardown { callback: 1 },
    ]);
    let certificate = check_callback_task_handoff(&operations).unwrap();
    assert_eq!(certificate.tasks_joined, [2]);
    assert_eq!(certificate.deferred_completed, [3]);
    assert!(certificate.remaining_argument_custody.is_empty());
}

#[test]
fn cancellation_requires_acknowledgement_before_join() {
    let mut operations = prefix();
    operations.extend([
        Op::SpawnTask {
            callback: 1,
            task: 2,
            retains_argument: false,
        },
        Op::RequestCancel {
            callback: 1,
            task: 2,
        },
        Op::JoinTask {
            callback: 1,
            task: 2,
        },
    ]);
    assert_eq!(
        check_callback_task_handoff(&operations),
        Err(HandoffDiagnostic::InvalidTaskTransition {
            task: 2,
            from: qcore_core::callback_task_handoff::TaskState::CancelRequested,
        })
    );

    let mut accepted = prefix();
    accepted.extend([
        Op::SpawnTask {
            callback: 1,
            task: 2,
            retains_argument: false,
        },
        Op::RequestCancel {
            callback: 1,
            task: 2,
        },
        Op::AcknowledgeCancel { task: 2 },
        Op::JoinTask {
            callback: 1,
            task: 2,
        },
        Op::CallbackExit { callback: 1 },
        Op::Teardown { callback: 1 },
    ]);
    assert!(check_callback_task_handoff(&accepted).is_ok());
}

#[test]
fn retained_argument_and_teardown_refuse_live_work() {
    let mut retained = prefix();
    retained.extend([
        Op::SpawnTask {
            callback: 1,
            task: 2,
            retains_argument: true,
        },
        Op::CallbackExit { callback: 1 },
        Op::DropArgument { argument: 9 },
    ]);
    assert_eq!(
        check_callback_task_handoff(&retained),
        Err(HandoffDiagnostic::ArgumentStillRetained(9))
    );

    let mut deferred = prefix();
    deferred.extend([
        Op::RegisterDeferred {
            callback: 1,
            work: 3,
            retains_argument: false,
        },
        Op::CallbackExit { callback: 1 },
        Op::Teardown { callback: 1 },
    ]);
    assert_eq!(
        check_callback_task_handoff(&deferred),
        Err(HandoffDiagnostic::OutstandingDeferred {
            callback: 1,
            work: 3
        })
    );
}

#[test]
fn work_cannot_be_created_after_callback_exit() {
    let mut operations = prefix();
    operations.extend([
        Op::CallbackExit { callback: 1 },
        Op::SpawnTask {
            callback: 1,
            task: 2,
            retains_argument: false,
        },
    ]);
    assert_eq!(
        check_callback_task_handoff(&operations),
        Err(HandoffDiagnostic::CallbackAlreadyExited(1))
    );
}

#[test]
fn frame_borrow_cannot_escape_into_post_return_work() {
    let operations = [
        Op::CallbackEnter {
            callback: 1,
            argument: 9,
            argument_lifetime: ArgumentLifetime::BorrowedFrame,
        },
        Op::RegisterDeferred {
            callback: 1,
            work: 3,
            retains_argument: true,
        },
    ];
    assert_eq!(
        check_callback_task_handoff(&operations),
        Err(HandoffDiagnostic::BorrowedArgumentEscapes {
            callback: 1,
            argument: 9
        })
    );
}

#[test]
fn frame_borrow_cannot_be_reported_as_dropped_after_return() {
    let operations = [
        Op::CallbackEnter {
            callback: 1,
            argument: 9,
            argument_lifetime: ArgumentLifetime::BorrowedFrame,
        },
        Op::CallbackExit { callback: 1 },
        Op::DropArgument { argument: 9 },
        Op::Teardown { callback: 1 },
    ];
    assert_eq!(
        check_callback_task_handoff(&operations),
        Err(HandoffDiagnostic::BorrowedArgumentCannotDrop(9))
    );
}

#[test]
fn argument_liveness_and_exclusive_identity_are_enforced() {
    let unknown_drop = [Op::DropArgument { argument: 9 }];
    assert_eq!(
        check_callback_task_handoff(&unknown_drop),
        Err(HandoffDiagnostic::UnknownArgument(9))
    );

    let duplicate_owned = [
        Op::CallbackEnter {
            callback: 1,
            argument: 9,
            argument_lifetime: ArgumentLifetime::Owned,
        },
        Op::CallbackEnter {
            callback: 2,
            argument: 9,
            argument_lifetime: ArgumentLifetime::Owned,
        },
    ];
    assert_eq!(
        check_callback_task_handoff(&duplicate_owned),
        Err(HandoffDiagnostic::ArgumentAlreadyClaimed(9))
    );
}

#[test]
fn identities_and_teardown_are_single_use() {
    let mut operations = prefix();
    operations.extend([
        Op::CallbackExit { callback: 1 },
        Op::Teardown { callback: 1 },
        Op::Teardown { callback: 1 },
    ]);
    assert_eq!(
        check_callback_task_handoff(&operations),
        Err(HandoffDiagnostic::DuplicateTeardown(1))
    );
}

#[test]
fn dropped_arguments_and_joined_task_ids_cannot_be_reused() {
    let operations = [
        Op::CallbackEnter {
            callback: 1,
            argument: 9,
            argument_lifetime: ArgumentLifetime::Owned,
        },
        Op::SpawnTask {
            callback: 1,
            task: 2,
            retains_argument: false,
        },
        Op::StartTask { task: 2 },
        Op::CompleteTask { task: 2 },
        Op::JoinTask {
            callback: 1,
            task: 2,
        },
        Op::SpawnTask {
            callback: 1,
            task: 2,
            retains_argument: false,
        },
    ];
    assert_eq!(
        check_callback_task_handoff(&operations),
        Err(HandoffDiagnostic::DuplicateTask(2))
    );

    let argument_reuse = [
        Op::CallbackEnter {
            callback: 1,
            argument: 9,
            argument_lifetime: ArgumentLifetime::Owned,
        },
        Op::CallbackExit { callback: 1 },
        Op::DropArgument { argument: 9 },
        Op::Teardown { callback: 1 },
        Op::CallbackEnter {
            callback: 2,
            argument: 9,
            argument_lifetime: ArgumentLifetime::Owned,
        },
    ];
    assert_eq!(
        check_callback_task_handoff(&argument_reuse),
        Err(HandoffDiagnostic::ArgumentAlreadyDropped(9))
    );
}

#[test]
fn completion_cannot_skip_start_or_race_cancellation() {
    let mut skipped = prefix();
    skipped.extend([
        Op::SpawnTask {
            callback: 1,
            task: 2,
            retains_argument: false,
        },
        Op::CompleteTask { task: 2 },
    ]);
    assert!(matches!(
        check_callback_task_handoff(&skipped),
        Err(HandoffDiagnostic::InvalidTaskTransition { .. })
    ));

    let mut racing = prefix();
    racing.extend([
        Op::SpawnTask {
            callback: 1,
            task: 2,
            retains_argument: false,
        },
        Op::StartTask { task: 2 },
        Op::RequestCancel {
            callback: 1,
            task: 2,
        },
        Op::CompleteTask { task: 2 },
    ]);
    assert!(matches!(
        check_callback_task_handoff(&racing),
        Err(HandoffDiagnostic::InvalidTaskTransition { .. })
    ));
}

#[test]
fn undropped_owned_custody_is_explicit_in_certificate() {
    let mut operations = prefix();
    operations.extend([
        Op::CallbackExit { callback: 1 },
        Op::Teardown { callback: 1 },
    ]);
    let certificate = check_callback_task_handoff(&operations).unwrap();
    assert_eq!(certificate.remaining_argument_custody, [9]);
}

#[test]
fn remaining_custody_is_canonical_by_argument_identity() {
    let operations = [
        Op::CallbackEnter {
            callback: 1,
            argument: 200,
            argument_lifetime: ArgumentLifetime::Owned,
        },
        Op::CallbackExit { callback: 1 },
        Op::Teardown { callback: 1 },
        Op::CallbackEnter {
            callback: 2,
            argument: 100,
            argument_lifetime: ArgumentLifetime::Owned,
        },
        Op::CallbackExit { callback: 2 },
        Op::Teardown { callback: 2 },
    ];
    let certificate = check_callback_task_handoff(&operations).unwrap();
    assert_eq!(certificate.remaining_argument_custody, [100, 200]);
}

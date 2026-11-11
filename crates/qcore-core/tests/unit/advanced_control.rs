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

use super::*;

fn destructors() -> Vec<DestructorSpec> {
    vec![
        DestructorSpec {
            id: 1,
            outcome: DestructorOutcome::Ok,
        },
        DestructorSpec {
            id: 2,
            outcome: DestructorOutcome::Error("close failed".into()),
        },
    ]
}

#[test]
fn cancellation_is_deferred_to_a_safe_point_and_cleans_lifo() {
    let program = AdvancedProgram {
        destructors: destructors(),
        operations: vec![
            AdvancedOp::Acquire {
                resource: 10,
                destructor: 1,
            },
            AdvancedOp::RequestCancellation,
            AdvancedOp::Acquire {
                resource: 11,
                destructor: 1,
            },
            AdvancedOp::SafePoint,
        ],
        ..AdvancedProgram::default()
    };
    let execution = run_advanced(&program).unwrap();
    assert_eq!(execution.outcome, AdvancedOutcome::Cancelled);
    assert_eq!(
        execution.trace,
        vec![
            AdvancedTraceEvent::Acquire(10),
            AdvancedTraceEvent::CancellationRequested,
            AdvancedTraceEvent::Acquire(11),
            AdvancedTraceEvent::CancellationDelivered,
            AdvancedTraceEvent::Destroy(11),
            AdvancedTraceEvent::Destroy(10),
        ]
    );
}

#[test]
fn fallible_cleanup_on_normal_return_has_a_distinct_outcome() {
    let program = AdvancedProgram {
        destructors: destructors(),
        operations: vec![
            AdvancedOp::Acquire {
                resource: 10,
                destructor: 2,
            },
            AdvancedOp::Return(7),
        ],
        ..AdvancedProgram::default()
    };
    assert_eq!(
        run_advanced(&program).unwrap().outcome,
        AdvancedOutcome::CleanupFailure(vec!["close failed".into()])
    );
}

#[test]
fn double_error_preserves_the_primary_error() {
    let program = AdvancedProgram {
        destructors: destructors(),
        operations: vec![
            AdvancedOp::Acquire {
                resource: 10,
                destructor: 2,
            },
            AdvancedOp::Raise("parse failed".into()),
        ],
        ..AdvancedProgram::default()
    };
    assert_eq!(
        run_advanced(&program).unwrap().outcome,
        AdvancedOutcome::DoubleError {
            primary: PrimaryFailure::Error("parse failed".into()),
            cleanup: vec!["close failed".into()]
        }
    );
}

#[test]
fn owned_capture_is_linear_and_one_shot() {
    let handler = HandlerSpec {
        id: 4,
        kind: HandlerKind::OneShot,
        captures: vec![Capture {
            resource: 10,
            mode: CaptureMode::Owned,
        }],
    };
    let accepted = AdvancedProgram {
        handlers: vec![handler.clone()],
        operations: vec![AdvancedOp::Invoke(4), AdvancedOp::Return(0)],
        ..AdvancedProgram::default()
    };
    assert!(run_advanced(&accepted).is_ok());

    let duplicated = AdvancedProgram {
        handlers: vec![handler],
        operations: vec![
            AdvancedOp::Invoke(4),
            AdvancedOp::Invoke(4),
            AdvancedOp::Return(0),
        ],
        ..AdvancedProgram::default()
    };
    assert_eq!(
        run_advanced(&duplicated),
        Err(AdvancedControlError::OneShotHandlerInvokedTwice(4))
    );
}

#[test]
fn multi_shot_and_resumable_handlers_reject_owned_capture() {
    for kind in [HandlerKind::MultiShot, HandlerKind::Resumable] {
        let program = AdvancedProgram {
            handlers: vec![HandlerSpec {
                id: 4,
                kind,
                captures: vec![Capture {
                    resource: 10,
                    mode: CaptureMode::Owned,
                }],
            }],
            operations: vec![AdvancedOp::Return(0)],
            ..AdvancedProgram::default()
        };
        assert_eq!(
            run_advanced(&program),
            Err(AdvancedControlError::OwnedCaptureRequiresOneShot(4))
        );
    }
}

#[test]
fn multi_shot_shared_capture_can_be_invoked_repeatedly() {
    let program = AdvancedProgram {
        handlers: vec![HandlerSpec {
            id: 4,
            kind: HandlerKind::MultiShot,
            captures: vec![Capture {
                resource: 10,
                mode: CaptureMode::Shared,
            }],
        }],
        operations: vec![
            AdvancedOp::Invoke(4),
            AdvancedOp::Invoke(4),
            AdvancedOp::Return(0),
        ],
        ..AdvancedProgram::default()
    };
    assert!(run_advanced(&program).is_ok());
}

#[test]
fn normal_cleanup_destroys_each_resource_exactly_once() {
    let program = AdvancedProgram {
        destructors: destructors(),
        operations: vec![
            AdvancedOp::Acquire {
                resource: 10,
                destructor: 1,
            },
            AdvancedOp::Acquire {
                resource: 11,
                destructor: 1,
            },
            AdvancedOp::Drop(10),
            AdvancedOp::Return(0),
        ],
        ..AdvancedProgram::default()
    };
    let execution = run_advanced(&program).unwrap();
    let destroyed = execution
        .trace
        .iter()
        .filter_map(|event| match event {
            AdvancedTraceEvent::Destroy(resource) => Some(*resource),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(destroyed, vec![10, 11]);
}

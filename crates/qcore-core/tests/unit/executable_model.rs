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

fn program(body: Vec<Operation>) -> ModelProgram {
    ModelProgram {
        usage: vec![UsageBinding {
            variable: 0,
            grade: UsageGrade::ExactlyOne,
        }],
        declared_errors: vec![7],
        body,
    }
}

#[test]
fn returns_a_natural_index_after_satisfying_usage_and_ownership() {
    let mut machine = ExecutableModel::new(
        program(vec![
            Operation::Use(0),
            Operation::Acquire(11),
            Operation::Drop(11),
            Operation::Return(42),
        ]),
        vec![1],
    )
    .unwrap();

    assert_eq!(machine.run(), &ModelOutcome::Returned(42));
    assert!(machine.state().is_well_formed());
    assert_eq!(machine.state().released, vec![11]);
}

#[test]
fn a_typed_error_cleans_partial_initialization_in_lifo_order() {
    let mut machine = ExecutableModel::new(
        program(vec![
            Operation::Use(0),
            Operation::Acquire(10),
            Operation::Acquire(20),
            Operation::Raise(7),
        ]),
        vec![1],
    )
    .unwrap();

    let outcome = machine.run().clone();
    assert_eq!(
        outcome,
        ModelOutcome::Error {
            error: ModelTypedError::Raised(7),
            pack: ModelCleanupPack {
                baseline: vec![1],
                actions: vec![20, 10],
            },
        }
    );
    assert!(machine.state().locals.is_empty());
    assert_eq!(machine.state().released, vec![10, 20]);
    assert_eq!(
        &machine.state().trace[3..],
        &[ModelTraceEvent::Release(20), ModelTraceEvent::Release(10),]
    );
}

#[test]
fn exact_usage_obligation_fails_as_a_typed_error() {
    let mut machine =
        ExecutableModel::new(program(vec![Operation::Return(0)]), Vec::new()).unwrap();

    assert!(matches!(
        machine.run(),
        ModelOutcome::Error {
            error: ModelTypedError::UsageObligationUnsatisfied {
                variable: 0,
                grade: UsageGrade::ExactlyOne,
                actual: 0,
            },
            ..
        }
    ));
}

#[test]
fn duplicate_acquire_is_typed_and_original_resource_is_cleaned_once() {
    let mut machine = ExecutableModel::new(
        program(vec![Operation::Acquire(10), Operation::Acquire(10)]),
        Vec::new(),
    )
    .unwrap();

    assert!(matches!(
        machine.run(),
        ModelOutcome::Error {
            error: ModelTypedError::DuplicateAcquire(10),
            ..
        }
    ));
    assert_eq!(machine.state().released, vec![10]);
    assert!(machine.state().is_well_formed());
}

#[test]
fn invalid_drop_enters_cleanup_without_double_release() {
    let mut machine = ExecutableModel::new(
        program(vec![Operation::Acquire(10), Operation::Drop(99)]),
        Vec::new(),
    )
    .unwrap();

    assert!(matches!(
        machine.run(),
        ModelOutcome::Error {
            error: ModelTypedError::InvalidDrop(99),
            ..
        }
    ));
    assert_eq!(machine.state().released, vec![10]);
}

#[test]
fn construction_rejects_duplicate_static_bindings() {
    let duplicate_usage = ModelProgram {
        usage: vec![
            UsageBinding {
                variable: 3,
                grade: UsageGrade::Affine,
            },
            UsageBinding {
                variable: 3,
                grade: UsageGrade::Many,
            },
        ],
        declared_errors: Vec::new(),
        body: Vec::new(),
    };
    assert_eq!(
        ExecutableModel::new(duplicate_usage, Vec::new()).unwrap_err(),
        ModelBuildError::DuplicateUsageBinding(3)
    );
    assert_eq!(
        ExecutableModel::new(program(Vec::new()), vec![4, 4]).unwrap_err(),
        ModelBuildError::DuplicateBaseline(4)
    );
}

#[test]
fn all_usage_grades_apply_their_interval_bounds() {
    assert!(UsageGrade::Erased.accepts(0));
    assert!(!UsageGrade::Erased.accepts(1));
    assert!(UsageGrade::ExactlyOne.accepts(1));
    assert!(!UsageGrade::ExactlyOne.accepts(0));
    assert!(UsageGrade::Affine.accepts(0));
    assert!(UsageGrade::Affine.accepts(1));
    assert!(!UsageGrade::Affine.accepts(2));
    assert!(UsageGrade::Many.accepts(100));
}

#[test]
fn every_small_step_preserves_the_identity_partition() {
    let mut machine = ExecutableModel::new(
        program(vec![
            Operation::Use(0),
            Operation::Acquire(10),
            Operation::Acquire(20),
            Operation::Raise(7),
        ]),
        vec![1, 2],
    )
    .unwrap();

    assert!(machine.state().is_well_formed());
    while machine.step() {
        assert!(machine.state().is_well_formed());
    }
    assert_eq!(machine.state().released, vec![10, 20]);
}

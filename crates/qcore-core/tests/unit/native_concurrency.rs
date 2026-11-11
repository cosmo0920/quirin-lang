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

#[test]
fn native_workers_preserve_transfer_cleanup_and_observation() {
    let program = NativeProgram {
        tasks: BTreeSet::from([0, 1]),
        cells: BTreeMap::from([(1, 0)]),
        initial_custody: BTreeMap::from([(7, 0)]),
        schedule: vec![
            NativeScheduleStep {
                task: 0,
                operation: NativeOperation::Store { cell: 1, value: 42 },
            },
            NativeScheduleStep {
                task: 1,
                operation: NativeOperation::Load {
                    cell: 1,
                    observation: 9,
                },
            },
            NativeScheduleStep {
                task: 0,
                operation: NativeOperation::TransferCustody { resource: 7, to: 1 },
            },
            NativeScheduleStep {
                task: 1,
                operation: NativeOperation::Destroy { resource: 7 },
            },
        ],
        expected_observations: BTreeMap::from([(9, 42)]),
    };
    let certificate = run_native_backend(&program).unwrap();
    assert_eq!(certificate.destroyed_resources, BTreeSet::from([7]));
    assert_eq!(certificate.joined_tasks, BTreeSet::from([0, 1]));
}

#[test]
fn wrong_custody_and_incomplete_cleanup_fail() {
    let wrong = NativeProgram {
        tasks: BTreeSet::from([0, 1]),
        initial_custody: BTreeMap::from([(7, 0)]),
        schedule: vec![NativeScheduleStep {
            task: 1,
            operation: NativeOperation::Destroy { resource: 7 },
        }],
        ..NativeProgram::default()
    };
    assert!(matches!(
        run_native_backend(&wrong),
        Err(NativeBackendError::WrongCustodian { .. })
    ));
    let leaked = NativeProgram {
        tasks: BTreeSet::from([0]),
        initial_custody: BTreeMap::from([(7, 0)]),
        ..NativeProgram::default()
    };
    assert_eq!(
        run_native_backend(&leaked),
        Err(NativeBackendError::UndestroyedResource(7))
    );
}

#[test]
fn platform_gaps_are_failures_not_inferences() {
    let linux = PlatformRun {
        platform: RequiredPlatform::Linux,
        passed: true,
        sanitizer: true,
    };
    assert_eq!(
        check_platform_matrix(&[linux]),
        Err(PlatformMatrixError::Missing(RequiredPlatform::Windows))
    );
}

#[test]
fn every_deferred_concurrency_feature_is_classified() {
    assert_eq!(PROMOTED_FEATURE_PROFILE.len(), 6);
    assert_eq!(
        PROMOTED_FEATURE_PROFILE
            .iter()
            .filter(|(_, status)| *status == FeatureDisposition::Accepted)
            .count(),
        1
    );
}

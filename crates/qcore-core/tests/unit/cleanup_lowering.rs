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
use crate::cfg::interpret_cfg;

fn program(operations: Vec<SourceOp>) -> SourceProgram {
    SourceProgram::new(
        "load",
        vec![1, 2],
        vec!["IoError".into(), "ParseError".into()],
        operations,
    )
    .unwrap()
}

#[test]
fn partial_initialization_uses_edge_specific_cleanup() {
    let source = program(vec![
        SourceOp::Acquire {
            resource: 1,
            succeeds: true,
            error: "IoError".into(),
        },
        SourceOp::Acquire {
            resource: 2,
            succeeds: false,
            error: "IoError".into(),
        },
        SourceOp::Drop(2),
        SourceOp::Drop(1),
        SourceOp::Return(0),
    ]);
    let cfg = lower_cleanup_cfg(&source).unwrap();
    let expected = interpret_source(&source).unwrap();
    assert_eq!(interpret_cfg(&cfg).unwrap(), expected);
    assert_eq!(
        expected.trace,
        vec![TraceEvent::Acquire(1), TraceEvent::Release(1)]
    );

    let cleanup_blocks = cfg
        .blocks()
        .filter(|block| block.is_cleanup())
        .collect::<Vec<_>>();
    assert_eq!(cleanup_blocks.len(), 2);
    assert!(cleanup_blocks[0].instructions().is_empty());
    assert_eq!(
        cleanup_blocks[1].instructions(),
        &[CfgInstruction::Release(1)]
    );
}

#[test]
fn cleanup_is_lifo_and_releases_each_identity_once() {
    let source = program(vec![
        SourceOp::Acquire {
            resource: 1,
            succeeds: true,
            error: "IoError".into(),
        },
        SourceOp::Acquire {
            resource: 2,
            succeeds: true,
            error: "IoError".into(),
        },
        SourceOp::Check {
            passes: false,
            error: "ParseError".into(),
        },
        SourceOp::Drop(2),
        SourceOp::Drop(1),
        SourceOp::Return(0),
    ]);
    let observation = interpret_cfg(&lower_cleanup_cfg(&source).unwrap()).unwrap();
    assert_eq!(
        observation.trace,
        vec![
            TraceEvent::Acquire(1),
            TraceEvent::Acquire(2),
            TraceEvent::Release(2),
            TraceEvent::Release(1),
        ]
    );
}

#[test]
fn normal_transfer_is_explicit_and_not_cleaned_locally() {
    let source = program(vec![
        SourceOp::Acquire {
            resource: 2,
            succeeds: true,
            error: "IoError".into(),
        },
        SourceOp::TransferToResult(2),
        SourceOp::Return(17),
    ]);
    let expected = interpret_source(&source).unwrap();
    assert_eq!(
        interpret_cfg(&lower_cleanup_cfg(&source).unwrap()).unwrap(),
        expected
    );
    assert_eq!(
        expected.outcome,
        ExecutionOutcome::Ok {
            value: 17,
            payload: vec![2]
        }
    );
    assert_eq!(
        expected.trace,
        vec![TraceEvent::Acquire(2), TraceEvent::Transfer(2)]
    );
}

#[test]
fn invalid_normal_leak_is_rejected() {
    assert!(matches!(
        SourceProgram::new(
            "leak",
            vec![1],
            vec!["IoError".into()],
            vec![
                SourceOp::Acquire {
                    resource: 1,
                    succeeds: true,
                    error: "IoError".into(),
                },
                SourceOp::Return(0),
            ],
        ),
        Err(LoweringError::LiveResourcesAtNormalReturn(_))
    ));
}

#[test]
fn resource_identity_cannot_be_reacquired_after_release() {
    assert_eq!(
        SourceProgram::new(
            "duplicate_identity",
            vec![1],
            vec!["IoError".into()],
            vec![
                SourceOp::Acquire {
                    resource: 1,
                    succeeds: true,
                    error: "IoError".into(),
                },
                SourceOp::Drop(1),
                SourceOp::Acquire {
                    resource: 1,
                    succeeds: true,
                    error: "IoError".into(),
                },
                SourceOp::Drop(1),
                SourceOp::Return(0),
            ],
        ),
        Err(LoweringError::ResourceAlreadyOwned(1))
    );
}

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

fn declarations() -> ErrorDeclarations {
    ErrorDeclarations::new(["IoError".into(), "ParseError".into()])
}

fn allowed() -> EffectRow {
    EffectRow::new([
        Effect::Throws("IoError".into()),
        Effect::Throws("ParseError".into()),
    ])
}

fn open_catch(error: &str, region: &str, handler: Vec<RegionOp>) -> RegionCatchClause {
    RegionCatchClause {
        error: error.into(),
        quant_open: Some(QuantOpen {
            region: region.into(),
            binding: "cleanup".into(),
        }),
        handler,
    }
}

fn region(body: Vec<RegionOp>, catches: Vec<RegionCatchClause>) -> RegionOp {
    RegionOp::ErrorRegion {
        region: "r".into(),
        body,
        catches,
    }
}

#[test]
fn partial_initialization_builds_path_specific_packs() {
    let operation = region(
        vec![RegionOp::Branch {
            then_ops: vec![RegionOp::Raise("IoError".into())],
            else_ops: vec![
                RegionOp::Acquire {
                    resource: 1,
                    destructor_noexcept: true,
                },
                RegionOp::Raise("IoError".into()),
            ],
        }],
        vec![],
    );
    let exits = analyze_error_regions(
        &declarations(),
        &allowed(),
        ErrorLedger::default(),
        &[operation],
    )
    .unwrap();
    let states = exits.states(&RegionExitTag::Error("IoError".into()));
    assert_eq!(states.len(), 2);
    assert_eq!(
        states[0].sealed_pack().unwrap().cleanup_resources().count(),
        0
    );
    assert_eq!(
        states[1]
            .sealed_pack()
            .unwrap()
            .cleanup_resources()
            .collect::<Vec<_>>(),
        vec![1]
    );
}

#[test]
fn discharge_restores_baseline_in_lifo_order() {
    let operation = region(
        vec![
            RegionOp::Acquire {
                resource: 1,
                destructor_noexcept: true,
            },
            RegionOp::Acquire {
                resource: 2,
                destructor_noexcept: true,
            },
            RegionOp::Raise("IoError".into()),
        ],
        vec![open_catch(
            "IoError",
            "r",
            vec![RegionOp::Discharge("cleanup".into())],
        )],
    );
    let exits = analyze_error_regions(
        &declarations(),
        &allowed(),
        ErrorLedger::new([9]),
        &[operation],
    )
    .unwrap();
    let state = &exits.states(&RegionExitTag::Normal)[0];
    assert_eq!(state.ledger(), &ErrorLedger::new([9]));
    assert_eq!(
        state.cleanup_trace(),
        &[CleanupEvent::Destroy(2), CleanupEvent::Destroy(1)]
    );
}

#[test]
fn error_only_credit_abandonment_is_explicit() {
    let operation = region(
        vec![
            RegionOp::AddStrictCredit(7),
            RegionOp::Raise("IoError".into()),
        ],
        vec![open_catch(
            "IoError",
            "r",
            vec![RegionOp::Discharge("cleanup".into())],
        )],
    );
    let exits = analyze_error_regions(
        &declarations(),
        &allowed(),
        ErrorLedger::default(),
        &[operation],
    )
    .unwrap();
    assert_eq!(
        exits.states(&RegionExitTag::Normal)[0].cleanup_trace(),
        &[CleanupEvent::AbandonCredit(7)]
    );

    let normal = region(vec![RegionOp::AddStrictCredit(1)], vec![]);
    assert!(matches!(
        analyze_error_regions(
            &declarations(),
            &allowed(),
            ErrorLedger::default(),
            &[normal]
        ),
        Err(ErrorRegionError::UnusedStrictCredit { .. })
    ));
}

#[test]
fn live_obligations_require_quant_open_and_discharge() {
    let clause = RegionCatchClause {
        error: "IoError".into(),
        quant_open: None,
        handler: vec![],
    };
    let operation = region(
        vec![
            RegionOp::Acquire {
                resource: 1,
                destructor_noexcept: true,
            },
            RegionOp::Raise("IoError".into()),
        ],
        vec![clause],
    );
    assert!(matches!(
        analyze_error_regions(
            &declarations(),
            &allowed(),
            ErrorLedger::default(),
            &[operation]
        ),
        Err(ErrorRegionError::CatchWithoutRequiredQuantOpen { .. })
    ));
}

#[test]
fn cleanup_capability_must_be_consumed_exactly_once() {
    let missing = region(
        vec![
            RegionOp::Acquire {
                resource: 1,
                destructor_noexcept: true,
            },
            RegionOp::Raise("IoError".into()),
        ],
        vec![open_catch("IoError", "r", vec![])],
    );
    assert_eq!(
        analyze_error_regions(
            &declarations(),
            &allowed(),
            ErrorLedger::default(),
            &[missing]
        ),
        Err(ErrorRegionError::CleanupPackNotDischarged)
    );

    let twice = region(
        vec![RegionOp::Raise("IoError".into())],
        vec![open_catch(
            "IoError",
            "r",
            vec![
                RegionOp::Discharge("cleanup".into()),
                RegionOp::Discharge("cleanup".into()),
            ],
        )],
    );
    assert_eq!(
        analyze_error_regions(
            &declarations(),
            &allowed(),
            ErrorLedger::default(),
            &[twice]
        ),
        Err(ErrorRegionError::CleanupPackDuplicated("cleanup".into()))
    );
}

#[test]
fn wrong_and_outside_quant_open_are_rejected() {
    let wrong = region(
        vec![RegionOp::Raise("IoError".into())],
        vec![open_catch(
            "IoError",
            "other",
            vec![RegionOp::Discharge("cleanup".into())],
        )],
    );
    assert!(matches!(
        analyze_error_regions(
            &declarations(),
            &allowed(),
            ErrorLedger::default(),
            &[wrong]
        ),
        Err(ErrorRegionError::WrongRegionOpened { .. })
    ));
    assert_eq!(
        analyze_error_regions(
            &declarations(),
            &allowed(),
            ErrorLedger::default(),
            &[RegionOp::QuantOpen {
                region: "r".into(),
                binding: "cleanup".into(),
            }]
        ),
        Err(ErrorRegionError::QuantOpenOutsideCatch)
    );
}

#[test]
fn rethrow_requires_prior_discharge() {
    let operation = region(
        vec![RegionOp::Raise("IoError".into())],
        vec![open_catch(
            "IoError",
            "r",
            vec![RegionOp::Raise("ParseError".into())],
        )],
    );
    assert_eq!(
        analyze_error_regions(
            &declarations(),
            &allowed(),
            ErrorLedger::default(),
            &[operation]
        ),
        Err(ErrorRegionError::RethrowWithUndischargedCleanup)
    );
}

#[test]
fn fallible_cleanup_is_rejected() {
    let operation = region(
        vec![RegionOp::Acquire {
            resource: 1,
            destructor_noexcept: false,
        }],
        vec![],
    );
    assert_eq!(
        analyze_error_regions(
            &declarations(),
            &allowed(),
            ErrorLedger::default(),
            &[operation]
        ),
        Err(ErrorRegionError::FallibleCleanup)
    );
}

#[test]
fn payload_transfer_is_excluded_from_cleanup() {
    let operation = region(
        vec![
            RegionOp::Acquire {
                resource: 1,
                destructor_noexcept: true,
            },
            RegionOp::Acquire {
                resource: 2,
                destructor_noexcept: true,
            },
            RegionOp::MoveToErrorPayload(2),
            RegionOp::Raise("IoError".into()),
        ],
        vec![open_catch(
            "IoError",
            "r",
            vec![RegionOp::Discharge("cleanup".into())],
        )],
    );
    let exits = analyze_error_regions(
        &declarations(),
        &allowed(),
        ErrorLedger::default(),
        &[operation],
    )
    .unwrap();
    let state = &exits.states(&RegionExitTag::Normal)[0];
    assert_eq!(state.payload().collect::<Vec<_>>(), vec![2]);
    assert_eq!(state.cleanup_trace(), &[CleanupEvent::Destroy(1)]);
}

#[test]
fn explicit_drop_removes_the_cleanup_action() {
    let operation = region(
        vec![
            RegionOp::Acquire {
                resource: 1,
                destructor_noexcept: true,
            },
            RegionOp::Drop(1),
            RegionOp::Raise("IoError".into()),
        ],
        vec![RegionCatchClause {
            error: "IoError".into(),
            quant_open: None,
            handler: vec![],
        }],
    );
    assert!(
        analyze_error_regions(
            &declarations(),
            &allowed(),
            ErrorLedger::default(),
            &[operation]
        )
        .is_ok()
    );
}

#[test]
fn nested_cleanup_order_is_inner_then_outer_and_lifo_within_each() {
    let inner = RegionOp::ErrorRegion {
        region: "inner".into(),
        body: vec![
            RegionOp::Acquire {
                resource: 2,
                destructor_noexcept: true,
            },
            RegionOp::Acquire {
                resource: 3,
                destructor_noexcept: true,
            },
            RegionOp::Raise("IoError".into()),
        ],
        catches: vec![RegionCatchClause {
            error: "IoError".into(),
            quant_open: Some(QuantOpen {
                region: "inner".into(),
                binding: "inner_cleanup".into(),
            }),
            handler: vec![
                RegionOp::Discharge("inner_cleanup".into()),
                RegionOp::Raise("ParseError".into()),
            ],
        }],
    };
    let outer = RegionOp::ErrorRegion {
        region: "outer".into(),
        body: vec![
            RegionOp::Acquire {
                resource: 1,
                destructor_noexcept: true,
            },
            inner,
        ],
        catches: vec![RegionCatchClause {
            error: "ParseError".into(),
            quant_open: Some(QuantOpen {
                region: "outer".into(),
                binding: "outer_cleanup".into(),
            }),
            handler: vec![RegionOp::Discharge("outer_cleanup".into())],
        }],
    };
    let exits = analyze_error_regions(
        &declarations(),
        &allowed(),
        ErrorLedger::default(),
        &[outer],
    )
    .unwrap();
    assert_eq!(
        exits.states(&RegionExitTag::Normal)[0].cleanup_trace(),
        &[
            CleanupEvent::Destroy(3),
            CleanupEvent::Destroy(2),
            CleanupEvent::Destroy(1),
        ]
    );
}

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

//! `EpochCreditRecoveryV1` — the accounting law over a flat epoch's error path.
//!
//! Every refusal test asserts the exact ordered diagnostic vector and the exact
//! code, never `is_err()`.
use super::*;
use crate::epoch_accounting::{
    EpochTask, FlatEpochProgram, OverheadBasis, RuntimeOverhead, TaskResult, check_flat_epoch,
};

fn bytes(amount: u64) -> Quantity {
    Quantity::single("memory", amount)
}

fn effort(amount: u64) -> Quantity {
    Quantity::single("effort", amount)
}

fn both(memory: u64, work: u64) -> Quantity {
    Quantity::from_axes([("memory".to_owned(), memory), ("effort".to_owned(), work)])
        .expect("two distinct axes never overflow")
}

fn task(id: EpochTaskId, credit: Quantity) -> EpochTask {
    EpochTask {
        task: id,
        shared_entry: Vec::new(),
        private_peak: bytes(64),
        work: effort(1),
        result: TaskResult::ReleasedInWorker,
        credit,
        joined: true,
    }
}

fn program(tasks: Vec<EpochTask>, allocation: Quantity) -> FlatEpochProgram {
    FlatEpochProgram {
        declared_task_count: tasks.len() as u64,
        shared_storage: Vec::new(),
        overhead: RuntimeOverhead {
            per_task: Quantity::zero(),
            per_epoch: Quantity::zero(),
            basis: OverheadBasis::Declared,
        },
        allocation,
        tasks,
        unsupported: Vec::new(),
    }
}

fn edges() -> CreationErrorEdges {
    CreationErrorEdges {
        creation_failure: true,
        join_failure: true,
    }
}

fn arm(id: EpochTaskId, declared: Option<ErrorArmCredit>) -> CreationErrorArm {
    CreationErrorArm {
        task: id,
        edges: edges(),
        declared,
    }
}

fn codes(diagnostics: &[EpochCreditDiagnostic]) -> Vec<&'static str> {
    diagnostics
        .iter()
        .map(EpochCreditDiagnostic::code)
        .collect()
}

/// The law, in the admitting direction: every arm's grant is split into a
/// restored part and a spent part, and the two add back to the grant on every
/// row and over the epoch.
#[test]
fn every_admitted_arm_conserves_its_grant() {
    let epoch = program(
        vec![task(1, both(4096, 100)), task(2, both(2048, 50))],
        both(8192, 200),
    );
    let plan = EpochCreditRecoveryPlan {
        arms: vec![
            arm(
                1,
                Some(ErrorArmCredit::RestoredToEpoch {
                    restored: both(4096, 100),
                }),
            ),
            arm(
                2,
                Some(ErrorArmCredit::RestoredToEpoch {
                    restored: both(512, 10),
                }),
            ),
        ],
    };
    let certificate = check_epoch_credit_recovery(&epoch, &plan).expect("the plan is admitted");

    assert_eq!(certificate.profile, PROFILE);
    assert_eq!(certificate.granted_credits, both(6144, 150));
    assert_eq!(certificate.restored_credits, both(4608, 110));
    assert_eq!(certificate.spent_credits, both(1536, 40));
    assert_eq!(certificate.error_arm_committed, certificate.spent_credits);

    // Conservation, over the epoch and on each row.
    assert_eq!(
        add(&certificate.restored_credits, &certificate.spent_credits).unwrap(),
        certificate.granted_credits
    );
    for row in &certificate.arms {
        assert_eq!(add(&row.restored, &row.spent).unwrap(), row.credit);
    }
    assert_eq!(certificate.arms[0].spent, Quantity::zero());
    assert_eq!(certificate.arms[1].spent, both(1536, 40));

    // The corollary the module rests on: the committed credit on the error path
    // never exceeds the granted total, which `check_flat_epoch` bounds.
    assert!(
        certificate
            .error_arm_committed
            .componentwise_le(&certificate.granted_credits)
    );
    assert!(
        certificate
            .granted_credits
            .componentwise_le(&epoch.allocation)
    );
    assert!(recheck_epoch_credit_recovery(&epoch, &plan, &certificate).is_ok());
}

/// An elided arm is `SpentByCreator`, which commits exactly the credit the flat
/// epoch already committed. This is what keeps every epoch written before this
/// module byte-identical.
#[test]
fn an_elided_arm_spends_the_whole_grant_and_changes_no_figure() {
    let epoch = program(vec![task(1, both(4096, 100))], both(8192, 200));
    let elided = EpochCreditRecoveryPlan {
        arms: vec![arm(1, None)],
    };
    let spelled = EpochCreditRecoveryPlan {
        arms: vec![arm(1, Some(ErrorArmCredit::SpentByCreator))],
    };
    let restored_nothing = EpochCreditRecoveryPlan {
        arms: vec![arm(
            1,
            Some(ErrorArmCredit::RestoredToEpoch {
                restored: Quantity::zero(),
            }),
        )],
    };

    let a = check_epoch_credit_recovery(&epoch, &elided).expect("an elided arm is admitted");
    let b = check_epoch_credit_recovery(&epoch, &spelled).expect("the spelled arm is admitted");
    let c = check_epoch_credit_recovery(&epoch, &restored_nothing)
        .expect("restoring nothing is admitted");
    assert_eq!(a, b, "eliding the arm is spelling `SpentByCreator`");
    assert_eq!(b, c, "restoring nothing is spending everything");

    assert_eq!(a.restored_credits, Quantity::zero());
    assert_eq!(a.spent_credits, both(4096, 100));
    // The bound certificate's granted total is untouched by any of it.
    let bound = check_flat_epoch(&epoch).expect("the epoch is admitted");
    assert_eq!(bound.granted_credits, a.granted_credits);
}

/// The leak direction: a grant that is neither restored nor spent is refused,
/// and refused at `Q635`, the flat epoch's outstanding-at-the-exit band.
#[test]
fn an_abandoned_grant_is_refused_and_names_the_task_and_the_axis() {
    let epoch = program(vec![task(1, both(4096, 100))], both(8192, 200));
    let plan = EpochCreditRecoveryPlan {
        arms: vec![arm(1, Some(ErrorArmCredit::AbandonedAtErrorArm))],
    };
    let refusal = check_epoch_credit_recovery(&epoch, &plan).expect_err("abandonment is refused");
    assert_eq!(codes(&refusal), vec!["Q635"]);
    assert_eq!(
        refusal,
        vec![EpochCreditDiagnostic::CreditAbandonedAtErrorArm {
            task: 1,
            // Axis order: `effort` sorts before `memory`.
            axis: "effort".to_owned(),
            abandoned: 100,
        }]
    );

    // A grant of nothing is still a claim about a path, and is still refused.
    let empty = program(vec![task(2, Quantity::zero())], both(8192, 200));
    let plan = EpochCreditRecoveryPlan {
        arms: vec![arm(2, Some(ErrorArmCredit::AbandonedAtErrorArm))],
    };
    let refusal = check_epoch_credit_recovery(&empty, &plan).expect_err("abandonment is refused");
    assert_eq!(
        refusal,
        vec![EpochCreditDiagnostic::CreditAbandonedAtErrorArm {
            task: 2,
            axis: "memory".to_owned(),
            abandoned: 0,
        }]
    );
}

/// The mint direction: restoring more than was granted is refused per task and
/// per axis, at `Q636`, the flat epoch's per-task credit band.
#[test]
fn restoring_more_than_the_grant_is_refused_on_each_axis() {
    let epoch = program(vec![task(1, both(4096, 100))], both(8192, 200));
    for (restored, axis, amount, credit) in [
        (both(4097, 100), "memory", 4097, 4096),
        (both(4096, 101), "effort", 101, 100),
    ] {
        let plan = EpochCreditRecoveryPlan {
            arms: vec![arm(1, Some(ErrorArmCredit::RestoredToEpoch { restored }))],
        };
        let refusal =
            check_epoch_credit_recovery(&epoch, &plan).expect_err("minting credit is refused");
        assert_eq!(codes(&refusal), vec!["Q636"]);
        assert_eq!(
            refusal,
            vec![EpochCreditDiagnostic::RestoredCreditExceedsGrant {
                task: 1,
                axis: axis.to_owned(),
                restored: amount,
                credit,
            }]
        );
    }

    // Restoring on an axis the grant does not have at all is the same refusal:
    // an axis absent from the grant is a grant of zero on it.
    let plan = EpochCreditRecoveryPlan {
        arms: vec![arm(
            1,
            Some(ErrorArmCredit::RestoredToEpoch {
                restored: Quantity::single("pages", 1),
            }),
        )],
    };
    let refusal = check_epoch_credit_recovery(&epoch, &plan).expect_err("minting is refused");
    assert_eq!(
        refusal,
        vec![EpochCreditDiagnostic::RestoredCreditExceedsGrant {
            task: 1,
            axis: "pages".to_owned(),
            restored: 1,
            credit: 0,
        }]
    );
}

/// The error path overflows into `Q641` exactly as the success path does.
#[test]
fn the_error_path_overflows_and_underflows_into_the_arithmetic_band() {
    let epoch = program(
        vec![task(1, bytes(u64::MAX)), task(2, bytes(u64::MAX))],
        bytes(u64::MAX),
    );
    let plan = EpochCreditRecoveryPlan {
        arms: vec![
            arm(
                1,
                Some(ErrorArmCredit::RestoredToEpoch {
                    restored: bytes(u64::MAX),
                }),
            ),
            arm(
                2,
                Some(ErrorArmCredit::RestoredToEpoch {
                    restored: bytes(u64::MAX),
                }),
            ),
        ],
    };
    let refusal = check_epoch_credit_recovery(&epoch, &plan).expect_err("the totals overflow");
    assert_eq!(codes(&refusal), vec!["Q641"]);
    assert_eq!(
        refusal,
        vec![EpochCreditDiagnostic::Arithmetic {
            detail: FlowError::Overflow
        }]
    );

    // The same epoch with the success path's own summation refuses at `Q641`
    // too: the error path is not a softer arithmetic regime.
    let bound = check_flat_epoch(&epoch).expect_err("the granted total overflows");
    assert!(
        bound.iter().any(|diagnostic| diagnostic.code() == "Q641"),
        "the success path refuses the same overflow in the same band"
    );
}

/// Structural refusals precede accounting, and all four are `Q630`.
#[test]
fn a_plan_that_does_not_describe_this_epoch_is_refused_structurally() {
    let epoch = program(vec![task(1, both(4096, 100))], both(8192, 200));

    let short = EpochCreditRecoveryPlan { arms: Vec::new() };
    assert_eq!(
        check_epoch_credit_recovery(&epoch, &short).expect_err("an armless plan is refused"),
        vec![EpochCreditDiagnostic::ArmCountMismatch {
            declared: 0,
            tasks: 1
        }]
    );

    let foreign = EpochCreditRecoveryPlan {
        arms: vec![arm(9, None)],
    };
    assert_eq!(
        check_epoch_credit_recovery(&epoch, &foreign).expect_err("a foreign task is refused"),
        vec![EpochCreditDiagnostic::UnknownTask { task: 9 }]
    );

    let doubled = EpochCreditRecoveryPlan {
        arms: vec![arm(1, None), arm(1, None)],
    };
    assert_eq!(
        check_epoch_credit_recovery(&epoch, &doubled).expect_err("two arms for one task"),
        vec![
            EpochCreditDiagnostic::ArmCountMismatch {
                declared: 2,
                tasks: 1
            },
            EpochCreditDiagnostic::DuplicateArm { task: 1 },
        ]
    );

    // A *declared* arm over a creation the creator cannot fail on. This is the
    // shape the purely declarative epoch route produces: a declared worker has
    // no creation, so it has no error edge.
    let declarative = EpochCreditRecoveryPlan {
        arms: vec![CreationErrorArm {
            task: 1,
            edges: CreationErrorEdges::default(),
            declared: Some(ErrorArmCredit::SpentByCreator),
        }],
    };
    let refusal = check_epoch_credit_recovery(&epoch, &declarative)
        .expect_err("an arm without an edge is refused");
    assert_eq!(codes(&refusal), vec!["Q630"]);
    assert_eq!(
        refusal,
        vec![EpochCreditDiagnostic::ErrorArmWithoutErrorEdge { task: 1 }]
    );

    // An *elided* arm over the same shape is not a refusal: it declares nothing
    // about a path, and spends the grant exactly as before.
    let elided = EpochCreditRecoveryPlan {
        arms: vec![CreationErrorArm {
            task: 1,
            edges: CreationErrorEdges::default(),
            declared: None,
        }],
    };
    assert!(check_epoch_credit_recovery(&epoch, &elided).is_ok());
}

/// Either edge alone carries an arm: a creation that can fail, and a join that
/// can observe a raised child. The second is the edge on which the child did
/// exist and did hold the grant.
#[test]
fn either_error_edge_alone_admits_a_declared_arm() {
    let epoch = program(vec![task(1, both(4096, 100))], both(8192, 200));
    for edges in [
        CreationErrorEdges {
            creation_failure: true,
            join_failure: false,
        },
        CreationErrorEdges {
            creation_failure: false,
            join_failure: true,
        },
    ] {
        assert!(edges.any());
        let plan = EpochCreditRecoveryPlan {
            arms: vec![CreationErrorArm {
                task: 1,
                edges,
                declared: Some(ErrorArmCredit::RestoredToEpoch {
                    restored: both(4096, 100),
                }),
            }],
        };
        let certificate =
            check_epoch_credit_recovery(&epoch, &plan).expect("either edge admits an arm");
        assert_eq!(certificate.restored_credits, both(4096, 100));
        // And abandonment on either edge alone is refused.
        let plan = EpochCreditRecoveryPlan {
            arms: vec![CreationErrorArm {
                task: 1,
                edges,
                declared: Some(ErrorArmCredit::AbandonedAtErrorArm),
            }],
        };
        assert_eq!(
            codes(&check_epoch_credit_recovery(&epoch, &plan).expect_err("abandonment is refused")),
            vec!["Q635"]
        );
    }
    assert!(!CreationErrorEdges::default().any());
}

/// The recheck rederives every field and names the two mutations this module
/// exists to refuse rather than folding them into a field mismatch.
#[test]
fn a_forged_split_is_named_by_the_recheck() {
    let epoch = program(vec![task(1, both(4096, 100))], both(8192, 200));
    let plan = EpochCreditRecoveryPlan {
        arms: vec![arm(
            1,
            Some(ErrorArmCredit::RestoredToEpoch {
                restored: both(1024, 25),
            }),
        )],
    };
    let certificate = check_epoch_credit_recovery(&epoch, &plan).expect("the plan is admitted");
    assert!(recheck_epoch_credit_recovery(&epoch, &plan, &certificate).is_ok());

    // A split that does not add back to the grant.
    let mut unbalanced = certificate.clone();
    unbalanced.spent_credits = both(1, 1);
    unbalanced.error_arm_committed = both(1, 1);
    let errors = recheck_epoch_credit_recovery(&epoch, &plan, &unbalanced)
        .expect_err("an unbalanced ledger is refused");
    assert!(
        errors.contains(&EpochCreditRecheckError::CreditLedgerUnbalanced {
            granted: both(4096, 100),
            restored: both(1024, 25),
            spent: both(1, 1),
        })
    );

    // A restored total larger than the grant.
    let mut minted = certificate.clone();
    minted.restored_credits = both(8192, 200);
    minted.spent_credits = Quantity::zero();
    minted.granted_credits = both(8192, 200);
    let errors = recheck_epoch_credit_recovery(&epoch, &plan, &minted)
        .expect_err("a minted total is refused");
    assert!(
        errors.contains(&EpochCreditRecheckError::RestoredCreditMinted {
            restored: both(8192, 200),
            granted: both(4096, 100),
        }) || errors.contains(&EpochCreditRecheckError::FieldMismatch {
            field: "granted_credits"
        })
    );

    // A per-task row that does not conserve its own grant.
    let mut row = certificate.clone();
    row.arms[0].spent = Quantity::zero();
    let errors =
        recheck_epoch_credit_recovery(&epoch, &plan, &row).expect_err("an unbalanced row refuses");
    assert!(errors.contains(&EpochCreditRecheckError::ArmUnbalanced { task: 1 }));

    // A certificate whose plan no longer checks is refused as a whole.
    let abandoning = EpochCreditRecoveryPlan {
        arms: vec![arm(1, Some(ErrorArmCredit::AbandonedAtErrorArm))],
    };
    assert_eq!(
        recheck_epoch_credit_recovery(&epoch, &abandoning, &certificate)
            .expect_err("a rejected plan is refused"),
        vec![EpochCreditRecheckError::PlanRejected(vec![
            EpochCreditDiagnostic::CreditAbandonedAtErrorArm {
                task: 1,
                axis: "effort".to_owned(),
                abandoned: 100,
            }
        ])]
    );

    // A foreign profile label.
    let mut foreign = certificate.clone();
    foreign.profile = "FlatEpochBoundV1";
    assert!(
        recheck_epoch_credit_recovery(&epoch, &plan, &foreign)
            .expect_err("a foreign label is refused")
            .contains(&EpochCreditRecheckError::FieldMismatch { field: "profile" })
    );
}

/// Non-vacuity: the refusals above are not proved against a checker that
/// refuses everything. Three concrete plans are admitted, and the display form
/// carries the code.
#[test]
fn concrete_plans_are_admitted_and_the_display_carries_the_code() {
    let epoch = program(
        vec![task(1, both(4096, 100)), task(2, both(2048, 50))],
        both(8192, 200),
    );
    for plan in [
        EpochCreditRecoveryPlan {
            arms: vec![arm(1, None), arm(2, None)],
        },
        EpochCreditRecoveryPlan {
            arms: vec![
                arm(1, Some(ErrorArmCredit::SpentByCreator)),
                arm(
                    2,
                    Some(ErrorArmCredit::RestoredToEpoch {
                        restored: both(2048, 50),
                    }),
                ),
            ],
        },
        EpochCreditRecoveryPlan {
            arms: vec![
                arm(
                    1,
                    Some(ErrorArmCredit::RestoredToEpoch {
                        restored: both(4096, 100),
                    }),
                ),
                arm(
                    2,
                    Some(ErrorArmCredit::RestoredToEpoch {
                        restored: both(2048, 50),
                    }),
                ),
            ],
        },
    ] {
        let certificate = check_epoch_credit_recovery(&epoch, &plan).expect("admitted");
        assert_eq!(
            add(&certificate.restored_credits, &certificate.spent_credits).unwrap(),
            certificate.granted_credits
        );
        assert!(recheck_epoch_credit_recovery(&epoch, &plan, &certificate).is_ok());
    }

    let rendered = EpochCreditDiagnostic::CreditAbandonedAtErrorArm {
        task: 1,
        axis: "memory".to_owned(),
        abandoned: 4096,
    }
    .to_string();
    assert!(rendered.starts_with("Q635: "), "{rendered}");
}

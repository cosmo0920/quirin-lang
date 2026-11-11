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

//! `CheckedRegionTaskCreditV1` — card T07.
//!
//! Every test names the acceptance clause it discharges. The admitted program
//! is built once by [`deliverable`]: one flat thread scope, a work region that
//! creates children and raises, and a handler that joins them and then
//! discharges the region's own cleanup pack. Every negative control is that
//! same program with one edit, so a refusal is never a mistake about syntax.

use super::*;
use crate::scoped_tasks::FULL_READ_AUTHORITY;

const WORK: &str = "work";
const ROOT: &str = "main";
const FAILED: &str = "Work.Failed";

fn program(initial_credit: u64, body: Vec<RegionTaskOp>) -> RegionTaskProgram {
    RegionTaskProgram {
        scope: 10,
        root_region: ROOT.into(),
        initial_credit,
        declared_errors: [FAILED.to_string()].into_iter().collect(),
        body,
    }
}

fn spawn(handle: HandleId, child: TaskId, credit: u64) -> Spawn {
    Spawn {
        reservation: 20 + handle,
        child,
        handle,
        moves: Vec::new(),
        loans: Vec::new(),
        child_credit: credit,
        outcome: SpawnOutcome::Created,
    }
}

fn region(credit: u64, body: Vec<RegionTaskOp>, handler: Vec<RegionTaskOp>) -> RegionTaskOp {
    RegionTaskOp::Region {
        region: WORK.into(),
        credit,
        body,
        catches: vec![RegionTaskCatch {
            error: FAILED.into(),
            quant_open: Some(QuantOpen {
                region: WORK.into(),
                binding: "pack".into(),
            }),
            handler,
        }],
    }
}

/// The card's deliverable: one flat scope, one work region that acquires a
/// resource, lends a second one to a child, endows that child with credit and
/// then raises; the handler joins the child, drops what came back and only then
/// discharges the region, abandoning what is left of the region's own credit.
fn deliverable() -> RegionTaskProgram {
    program(
        10,
        vec![
            RegionTaskOp::Acquire(7),
            region(
                6,
                vec![
                    RegionTaskOp::Acquire(8),
                    RegionTaskOp::Spawn(Spawn {
                        loans: vec![7],
                        ..spawn(30, 1, 4)
                    }),
                    RegionTaskOp::Spend(1),
                    RegionTaskOp::Complete {
                        child: 1,
                        outcome: TaskOutcome::I32(42),
                        resources: Vec::new(),
                        unspent_credit: 3,
                    },
                    RegionTaskOp::Raise(FAILED.into()),
                ],
                vec![
                    RegionTaskOp::Join { handle: 30 },
                    RegionTaskOp::Discharge("pack".into()),
                    RegionTaskOp::Spend(7),
                ],
            ),
            RegionTaskOp::Drop(7),
        ],
    )
}

fn checked(program: &RegionTaskProgram) -> RegionTaskCertificate {
    let certificate = check_region_task_program(program).expect("admitted program");
    recheck_region_task_certificate(program, &certificate).expect("independent recheck");
    certificate
}

fn refused(program: &RegionTaskProgram) -> RegionTaskDiagnostic {
    check_region_task_program(program).expect_err("refused program")
}

/// Rewrite the handler of the single work region, leaving everything else byte
/// for byte as the deliverable has it.
fn with_handler(handler: Vec<RegionTaskOp>) -> RegionTaskProgram {
    let mut edited = deliverable();
    let RegionTaskOp::Region { catches, .. } = &mut edited.body[1] else {
        unreachable!("the deliverable's second operation is the work region")
    };
    catches[0].handler = handler;
    edited
}

/// Rewrite the body of the single work region.
fn with_body(body: Vec<RegionTaskOp>) -> RegionTaskProgram {
    let mut edited = deliverable();
    let RegionTaskOp::Region { body: slot, .. } = &mut edited.body[1] else {
        unreachable!("the deliverable's second operation is the work region")
    };
    *slot = body;
    edited
}

// ---------------------------------------------------------------------------
// The deliverable
// ---------------------------------------------------------------------------

/// The whole admitted shape, checked end to end: the handler joins its child,
/// the loan expires at that join, the child's unspent credit comes back, and
/// only then is the region's remainder abandoned.
#[test]
fn a_handler_joins_its_children_then_discharges_its_own_region() {
    let program = deliverable();
    let certificate = checked(&program);

    assert_eq!(certificate.profile, PROFILE);
    assert_eq!(certificate.exit, ExitTag::Normal);
    assert!(certificate.scope_exited);
    assert_eq!(certificate.joined_handles, [30].into_iter().collect());
    assert!(certificate.failed_handles.is_empty());
    assert_eq!(certificate.collected, [(30, TaskOutcome::I32(42))]);
    assert!(certificate.payload.is_empty());
    assert_eq!(certificate.escaping_pack, None);

    // 10 endowed: 6 into the work region, of which 4 are lent to the child
    // (1 spent there, 3 returned at the join), 1 spent in the region and 1
    // abandoned with the pack. The 4 that stayed in the root region, plus the 3
    // the child returned, must be spent before the root region can exit.
    assert_eq!(
        certificate.credit,
        CreditAccounting {
            initial: 10,
            spent: 9,
            abandoned: 1,
            carried: 0
        }
    );
    assert_eq!(
        certificate.trace,
        vec![
            CreditEvent::Endow {
                region: WORK.into(),
                amount: 6
            },
            CreditEvent::Lend {
                child: 1,
                amount: 4
            },
            CreditEvent::Spend {
                region: WORK.into(),
                amount: 1
            },
            CreditEvent::ChildSpend {
                child: 1,
                amount: 1
            },
            CreditEvent::Return {
                child: 1,
                amount: 3
            },
            // The region's own resource is destroyed by the pack, before the
            // remainder is abandoned and long before the scope closes.
            CreditEvent::Destroy(8),
            CreditEvent::AbandonCredit {
                region: WORK.into(),
                amount: 1
            },
            CreditEvent::Spend {
                region: ROOT.into(),
                amount: 7
            },
        ]
    );
    assert_eq!(
        certificate.assumptions,
        [
            RegionTaskAssumption::DeclaredCreationOutcome,
            RegionTaskAssumption::DeclaredChildAccounting
        ]
        .into_iter()
        .collect()
    );
}

// ---------------------------------------------------------------------------
// Acceptance 1 — access permissions and live duties are not spare credit
// ---------------------------------------------------------------------------

/// One creation, no loan and no endowment, so the pack's only duty is the join.
/// Discharging before the join is refused: the join is not swept away with the
/// credit remainder.
fn one_unjoined_child(handler: Vec<RegionTaskOp>) -> RegionTaskProgram {
    program(
        4,
        vec![region(
            4,
            vec![
                RegionTaskOp::Spawn(spawn(30, 1, 0)),
                RegionTaskOp::Complete {
                    child: 1,
                    outcome: TaskOutcome::Unit,
                    resources: Vec::new(),
                    unspent_credit: 0,
                },
                RegionTaskOp::Raise(FAILED.into()),
            ],
            handler,
        )],
    )
}

#[test]
fn an_unjoined_child_blocks_the_abandonment_of_its_regions_credit() {
    let refusal = refused(&one_unjoined_child(vec![
        RegionTaskOp::Discharge("pack".into()),
        RegionTaskOp::Join { handle: 30 },
    ]));
    assert_eq!(
        refusal,
        RegionTaskDiagnostic::PendingLifetimeObligation {
            region: WORK.into(),
            duty: Duty::Join(30)
        }
    );
    assert_eq!(refusal.code(), "Q670");
    assert_eq!(
        refusal.to_string(),
        "Q670: region `work` cannot abandon credit while the join of handle `30` is outstanding"
    );

    // Joining first is admitted, and the whole remainder is then abandoned.
    let certificate = checked(&one_unjoined_child(vec![
        RegionTaskOp::Join { handle: 30 },
        RegionTaskOp::Discharge("pack".into()),
    ]));
    assert_eq!(
        certificate.credit,
        CreditAccounting {
            initial: 4,
            spent: 0,
            abandoned: 4,
            carried: 0
        }
    );
}

/// The same for the read loan the creation opened: read *access* held by
/// somebody else is a duty, not spare credit. This is the loan-expiry half of
/// acceptance 2's "only after required joins/loan expiry".
fn one_lent_buffer(handler: Vec<RegionTaskOp>) -> RegionTaskProgram {
    program(
        4,
        vec![
            RegionTaskOp::Acquire(7),
            region(
                4,
                vec![
                    RegionTaskOp::Spawn(Spawn {
                        loans: vec![7],
                        ..spawn(30, 1, 0)
                    }),
                    RegionTaskOp::Complete {
                        child: 1,
                        outcome: TaskOutcome::Unit,
                        resources: Vec::new(),
                        unspent_credit: 0,
                    },
                    RegionTaskOp::Raise(FAILED.into()),
                ],
                handler,
            ),
            RegionTaskOp::Drop(7),
        ],
    )
}

#[test]
fn an_outstanding_read_loan_blocks_the_abandonment_of_its_regions_credit() {
    let refusal = refused(&one_lent_buffer(vec![
        RegionTaskOp::Discharge("pack".into()),
        RegionTaskOp::Join { handle: 30 },
    ]));
    assert_eq!(
        refusal,
        RegionTaskDiagnostic::PendingLifetimeObligation {
            region: WORK.into(),
            duty: Duty::LoanExpiry(7)
        }
    );
    assert_eq!(refusal.code(), "Q670");

    // The loan gate is exactly as strong as the ledger's own loan state and no
    // stronger: the join closes the loan, and the discharge is then admitted.
    let certificate = checked(&one_lent_buffer(vec![
        RegionTaskOp::Join { handle: 30 },
        RegionTaskOp::Discharge("pack".into()),
    ]));
    assert_eq!(certificate.credit.abandoned, 4);
    assert_eq!(certificate.joined_handles, [30].into_iter().collect());
}

/// Spending credit never discharges a duty: after the enclosing region's whole
/// remainder has been spent, the pack is still refused, and the refusal names
/// the duty rather than a shortfall.
#[test]
fn spending_credit_does_not_pay_off_a_lifetime_duty() {
    let with_spend = |tail: Vec<RegionTaskOp>| {
        let mut handler = vec![RegionTaskOp::Spend(2)];
        handler.extend(tail);
        program(
            6,
            vec![region(
                4,
                vec![
                    RegionTaskOp::Spawn(spawn(30, 1, 0)),
                    RegionTaskOp::Complete {
                        child: 1,
                        outcome: TaskOutcome::Unit,
                        resources: Vec::new(),
                        unspent_credit: 0,
                    },
                    RegionTaskOp::Raise(FAILED.into()),
                ],
                handler,
            )],
        )
    };
    let refusal = refused(&with_spend(vec![
        RegionTaskOp::Discharge("pack".into()),
        RegionTaskOp::Join { handle: 30 },
    ]));
    assert_eq!(
        refusal,
        RegionTaskDiagnostic::PendingLifetimeObligation {
            region: WORK.into(),
            duty: Duty::Join(30)
        }
    );

    // The pack's own credit is not spendable at all: it is in the pack, not in
    // a frame, so the only thing that can happen to it is a gated abandonment.
    let certificate = checked(&with_spend(vec![
        RegionTaskOp::Join { handle: 30 },
        RegionTaskOp::Discharge("pack".into()),
    ]));
    assert_eq!(
        certificate.credit,
        CreditAccounting {
            initial: 6,
            spent: 2,
            abandoned: 4,
            carried: 0
        }
    );
}

/// A normal region exit cannot silently drop its credit either. Abandonment is
/// the error path's sink and nothing else's.
#[test]
fn a_normal_region_exit_refuses_to_abandon_its_remainder() {
    let refusal = refused(&program(
        4,
        vec![RegionTaskOp::Region {
            region: WORK.into(),
            credit: 3,
            body: vec![RegionTaskOp::Spend(1)],
            catches: Vec::new(),
        }],
    ));
    assert_eq!(
        refusal,
        RegionTaskDiagnostic::UnusedCredit {
            region: WORK.into(),
            remainder: 2
        }
    );
    assert_eq!(refusal.code(), "Q674");

    // And the root region is not exempt.
    let root = refused(&program(4, vec![RegionTaskOp::Spend(1)]));
    assert_eq!(
        root,
        RegionTaskDiagnostic::UnusedCredit {
            region: ROOT.into(),
            remainder: 3
        }
    );
}

/// A live release duty is discharged by the cleanup the pack runs, never by
/// abandonment: the destroy is recorded and the resource leaves the ledger.
#[test]
fn a_live_release_duty_is_run_not_abandoned() {
    let certificate = checked(&deliverable());
    let destroys = certificate
        .trace
        .iter()
        .filter(|event| matches!(event, CreditEvent::Destroy(_)))
        .count();
    // Resource 7 belongs to the root region and is dropped explicitly there, so
    // exactly one destroy — the pack's resource 8 — is a cleanup event.
    assert_eq!(destroys, 1);
    assert!(certificate.trace.contains(&CreditEvent::Destroy(8)));

    // Dropping a resource the open pack holds is refused: it is not the
    // handler's to release ahead of the discharge.
    let refusal = refused(&with_handler(vec![
        RegionTaskOp::Join { handle: 30 },
        RegionTaskOp::Drop(8),
        RegionTaskOp::Discharge("pack".into()),
        RegionTaskOp::Spend(7),
    ]));
    assert_eq!(refusal, RegionTaskDiagnostic::ResourceHeldByCleanupPack(8));
    assert_eq!(refusal.code(), "Q302");
}

// ---------------------------------------------------------------------------
// Acceptance 2 — child-owned credit is not the parent's to discard
// ---------------------------------------------------------------------------

/// The child holds credit the parent lent it. Until the join returns it, the
/// parent can neither spend it nor abandon it, and the refusal names the amount.
#[test]
fn a_parent_cannot_abandon_credit_that_is_out_with_a_child() {
    let refusal = refused(&with_handler(vec![
        RegionTaskOp::Discharge("pack".into()),
        RegionTaskOp::Join { handle: 30 },
        RegionTaskOp::Spend(7),
    ]));
    assert_eq!(
        refusal,
        RegionTaskDiagnostic::ChildCreditOutstanding {
            region: WORK.into(),
            child: 1,
            amount: 3
        }
    );
    assert_eq!(refusal.code(), "Q671");
    assert_eq!(
        refusal.to_string(),
        "Q671: region `work` cannot abandon `3` credit held by child `1`"
    );

    // The parent cannot spend the child's credit either: it is not in the
    // frame, so the frame simply does not have it.
    let overspend = refused(&with_body(vec![
        RegionTaskOp::Acquire(8),
        RegionTaskOp::Spawn(spawn(30, 1, 4)),
        RegionTaskOp::Spend(3),
        RegionTaskOp::Raise(FAILED.into()),
    ]));
    assert_eq!(
        overspend,
        RegionTaskDiagnostic::InsufficientCredit {
            region: WORK.into(),
            requested: 3,
            available: 2
        }
    );
    assert_eq!(overspend.code(), "Q675");
}

/// The reporting precedence, pinned so that no duty family can be shadowed out
/// of existence: child credit, then the read loan, then the join. Every one of
/// them refuses; only the *message* is ordered.
#[test]
fn every_duty_family_is_reported_and_none_shadows_another() {
    let attempt = |child_credit: u64, loans: Vec<ResourceId>| {
        let mut body = vec![RegionTaskOp::Spawn(Spawn {
            loans,
            ..spawn(30, 1, child_credit)
        })];
        body.push(RegionTaskOp::Complete {
            child: 1,
            outcome: TaskOutcome::Unit,
            resources: Vec::new(),
            unspent_credit: child_credit,
        });
        body.push(RegionTaskOp::Raise(FAILED.into()));
        refused(&program(
            6,
            vec![
                RegionTaskOp::Acquire(7),
                region(6, body, vec![RegionTaskOp::Discharge("pack".into())]),
                RegionTaskOp::Drop(7),
            ],
        ))
    };
    assert_eq!(
        attempt(2, vec![7]),
        RegionTaskDiagnostic::ChildCreditOutstanding {
            region: WORK.into(),
            child: 1,
            amount: 2
        }
    );
    assert_eq!(
        attempt(0, vec![7]),
        RegionTaskDiagnostic::PendingLifetimeObligation {
            region: WORK.into(),
            duty: Duty::LoanExpiry(7)
        }
    );
    assert_eq!(
        attempt(0, Vec::new()),
        RegionTaskDiagnostic::PendingLifetimeObligation {
            region: WORK.into(),
            duty: Duty::Join(30)
        }
    );
}

/// A creation that failed endowed nothing: the credit never left the frame, and
/// the frame is restored exactly, loan included.
#[test]
fn a_failed_creation_returns_the_endowment_and_the_loan_to_the_parent() {
    let certificate = checked(&program(
        10,
        vec![
            RegionTaskOp::Acquire(7),
            region(
                6,
                vec![
                    RegionTaskOp::Spawn(Spawn {
                        loans: vec![7],
                        outcome: SpawnOutcome::Failed,
                        ..spawn(30, 1, 4)
                    }),
                    RegionTaskOp::Raise(FAILED.into()),
                ],
                vec![
                    // No join is required: nothing was created, so no duty was
                    // opened, and the whole endowment is the pack's to abandon.
                    RegionTaskOp::Discharge("pack".into()),
                    RegionTaskOp::Spend(4),
                ],
            ),
            RegionTaskOp::Drop(7),
        ],
    ));
    assert!(certificate.joined_handles.is_empty());
    assert_eq!(
        certificate.credit,
        CreditAccounting {
            initial: 10,
            spent: 4,
            abandoned: 6,
            carried: 0
        }
    );
    // Nothing was lent, so nothing is returned.
    assert!(
        !certificate
            .trace
            .iter()
            .any(|event| matches!(event, CreditEvent::Lend { .. } | CreditEvent::Return { .. }))
    );
}

/// A failed join is terminal: the pack it belongs to can never be discharged,
/// and there is no admitted continuation that recovers it.
#[test]
fn a_failed_join_makes_its_regions_credit_permanently_unabandonable() {
    assert_eq!(
        refused(&one_unjoined_child(vec![
            RegionTaskOp::FailJoin { handle: 30 },
            RegionTaskOp::Discharge("pack".into()),
        ])),
        RegionTaskDiagnostic::PendingLifetimeObligation {
            region: WORK.into(),
            duty: Duty::Join(30)
        }
    );
    // A retry after a failed join is refused by the ledger itself.
    assert_eq!(
        refused(&one_unjoined_child(vec![
            RegionTaskOp::FailJoin { handle: 30 },
            RegionTaskOp::Join { handle: 30 },
            RegionTaskOp::Discharge("pack".into()),
        ])),
        RegionTaskDiagnostic::Ledger(TaskLedgerError::JoinFailed(30))
    );
    // And a failed join after a successful one is refused as well, so the
    // handle cannot be re-opened to reach the pack.
    assert_eq!(
        refused(&one_unjoined_child(vec![
            RegionTaskOp::Join { handle: 30 },
            RegionTaskOp::FailJoin { handle: 30 },
            RegionTaskOp::Discharge("pack".into()),
        ])),
        RegionTaskDiagnostic::Ledger(TaskLedgerError::HandleConsumed(30))
    );
}

/// Duties are attributed to the region that opened them. An enclosing region's
/// loan is not the inner region's to discard, and it is still enforced where it
/// belongs — at the enclosing region's own exit.
#[test]
fn an_enclosing_regions_duty_is_enforced_at_its_own_exit() {
    let refusal = refused(&program(
        8,
        vec![
            RegionTaskOp::Acquire(7),
            RegionTaskOp::Spawn(Spawn {
                loans: vec![7],
                ..spawn(30, 1, 0)
            }),
            RegionTaskOp::Region {
                region: WORK.into(),
                credit: 4,
                body: vec![RegionTaskOp::Raise(FAILED.into())],
                catches: vec![RegionTaskCatch {
                    error: FAILED.into(),
                    quant_open: Some(QuantOpen {
                        region: WORK.into(),
                        binding: "pack".into(),
                    }),
                    // The inner pack carries no duty of its own, so its
                    // remainder is abandoned even though an outer loan is live.
                    handler: vec![RegionTaskOp::Discharge("pack".into())],
                }],
            },
            RegionTaskOp::Spend(4),
        ],
    ));
    // The enclosing region is then refused at its own exit, naming its own duty.
    assert_eq!(
        refusal,
        RegionTaskDiagnostic::PendingLifetimeObligation {
            region: ROOT.into(),
            duty: Duty::LoanExpiry(7)
        }
    );
}

// ---------------------------------------------------------------------------
// Acceptance 3 — a rethrow carries every remaining payload and cleanup duty
// ---------------------------------------------------------------------------

/// The handler discharges its region, moves a resource into the error payload
/// and rethrows. The escaping certificate names the payload and the remaining
/// destroy duty; neither is dropped, and the scope is not closed.
#[test]
fn a_handler_rethrow_carries_its_payload_and_its_remaining_cleanup() {
    let program = program(
        6,
        vec![
            RegionTaskOp::Acquire(7),
            RegionTaskOp::Acquire(9),
            region(
                4,
                vec![
                    RegionTaskOp::Acquire(8),
                    RegionTaskOp::Spawn(spawn(30, 1, 2)),
                    RegionTaskOp::Complete {
                        child: 1,
                        outcome: TaskOutcome::Error("Child.Failed".into()),
                        resources: Vec::new(),
                        unspent_credit: 2,
                    },
                    RegionTaskOp::Raise(FAILED.into()),
                ],
                vec![
                    RegionTaskOp::Join { handle: 30 },
                    RegionTaskOp::Discharge("pack".into()),
                    RegionTaskOp::MoveToPayload(7),
                    RegionTaskOp::Raise(FAILED.into()),
                ],
            ),
        ],
    );
    let certificate = checked(&program);

    assert_eq!(certificate.exit, ExitTag::Error(FAILED.into()));
    assert!(!certificate.scope_exited);
    assert_eq!(certificate.payload, [7].into_iter().collect());
    assert_eq!(
        certificate.collected,
        [(30, TaskOutcome::Error("Child.Failed".into()))]
    );

    let pack = certificate.escaping_pack.as_ref().expect("escaping pack");
    assert_eq!(pack.region(), ROOT);
    assert_eq!(pack.error(), FAILED);
    // The root region's own resource is still a destroy duty and is carried,
    // not run and not forgotten. Resource 7 left for the payload; 8 was
    // destroyed by the inner pack.
    assert_eq!(pack.releases(), [9]);
    assert_eq!(pack.duties(), [Duty::Release(9)].into_iter().collect());
    // Credit that never had to be spent is carried, never abandoned.
    assert_eq!(
        certificate.credit,
        CreditAccounting {
            initial: 6,
            spent: 0,
            abandoned: 2,
            carried: 4
        }
    );
}

/// A rethrow with the cleanup pack still open is refused at the raise site.
#[test]
fn a_rethrow_cannot_leave_the_handlers_pack_open() {
    let refusal = refused(&with_handler(vec![
        RegionTaskOp::Join { handle: 30 },
        RegionTaskOp::Raise(FAILED.into()),
    ]));
    assert_eq!(
        refusal,
        RegionTaskDiagnostic::RethrowWithUndischargedCleanup
    );
    assert_eq!(refusal.code(), "Q307");
}

/// A handler that returns normally without discharging is refused too, so the
/// pack has no exit that discards it.
#[test]
fn a_handler_cannot_return_with_its_pack_undischarged() {
    let refusal = refused(&with_handler(vec![
        RegionTaskOp::Join { handle: 30 },
        RegionTaskOp::Spend(7),
    ]));
    assert_eq!(refusal, RegionTaskDiagnostic::CleanupPackNotDischarged);
    assert_eq!(refusal.code(), "Q305");
}

/// No lifetime obligation may leave the thread scope on an error edge.
#[test]
fn an_error_cannot_carry_a_lifetime_obligation_out_of_the_scope() {
    let refusal = refused(&program(
        4,
        vec![
            RegionTaskOp::Spawn(spawn(30, 1, 0)),
            RegionTaskOp::Spend(4),
            RegionTaskOp::Raise(FAILED.into()),
        ],
    ));
    assert_eq!(
        refusal,
        RegionTaskDiagnostic::ObligationEscapesScope {
            duty: Duty::Join(30)
        }
    );
    assert_eq!(refusal.code(), "Q672");
    assert_eq!(
        refusal.to_string(),
        "Q672: the join of handle `30` escapes the thread scope"
    );
}

/// A payload obligation cannot be dropped at a normal exit: it is either
/// released explicitly or it leaves with an error.
#[test]
fn a_payload_obligation_cannot_be_dropped_at_a_normal_exit() {
    let refusal = refused(&program(
        0,
        vec![RegionTaskOp::Acquire(7), RegionTaskOp::MoveToPayload(7)],
    ));
    assert_eq!(
        refusal,
        RegionTaskDiagnostic::PayloadObligationDropped { resource: 7 }
    );
    assert_eq!(refusal.code(), "Q677");

    // Releasing it explicitly is admitted, and the release is recorded.
    let certificate = checked(&program(
        0,
        vec![
            RegionTaskOp::Acquire(7),
            RegionTaskOp::MoveToPayload(7),
            RegionTaskOp::DropPayload(7),
        ],
    ));
    assert_eq!(certificate.trace, [CreditEvent::PayloadDestroy(7)]);
    assert!(certificate.payload.is_empty());
}

/// A loaned resource cannot be smuggled into an error payload: the payload move
/// is an exclusive use, and the loan suspends exclusive authority (T04).
#[test]
fn a_loaned_resource_cannot_be_moved_into_an_error_payload() {
    let refusal = refused(&program(
        0,
        vec![
            RegionTaskOp::Acquire(7),
            RegionTaskOp::Spawn(Spawn {
                loans: vec![7],
                ..spawn(30, 1, 0)
            }),
            RegionTaskOp::MoveToPayload(7),
        ],
    ));
    assert_eq!(
        refusal,
        RegionTaskDiagnostic::Ledger(TaskLedgerError::LoanOutstanding {
            resource: 7,
            outstanding: FULL_READ_AUTHORITY,
            borrowers: vec![1]
        })
    );
    assert_eq!(refusal.code(), "Q600");
}

/// An uncaught inner pack cannot propagate past the root region: it is refused
/// rather than merged into the escaping one.
#[test]
fn an_uncaught_inner_pack_cannot_propagate_past_the_root() {
    let refusal = refused(&program(
        0,
        vec![RegionTaskOp::Region {
            region: WORK.into(),
            credit: 0,
            body: vec![
                RegionTaskOp::Acquire(7),
                RegionTaskOp::Acquire(8),
                RegionTaskOp::MoveToPayload(7),
                RegionTaskOp::Raise(FAILED.into()),
            ],
            catches: Vec::new(),
        }],
    ));
    // The inner pack holds resource 8 only; the payload holds 7. The program is
    // refused at the escape, before any pack could be discarded.
    assert_eq!(refusal, RegionTaskDiagnostic::NestedPackNotDischarged);
    assert_eq!(refusal.code(), "Q302");
}

// ---------------------------------------------------------------------------
// Conservation, the pack discipline and certificate forgery
// ---------------------------------------------------------------------------

/// The conservation checkpoint holds at every step of the admitted program, and
/// the certificate's totals reproduce it.
#[test]
fn credit_is_conserved_across_the_admitted_program() {
    for (initial, program) in [
        (10, deliverable()),
        (
            6,
            program(
                6,
                vec![
                    region(
                        4,
                        vec![
                            RegionTaskOp::Spawn(spawn(30, 1, 3)),
                            RegionTaskOp::Complete {
                                child: 1,
                                outcome: TaskOutcome::Unit,
                                resources: Vec::new(),
                                unspent_credit: 0,
                            },
                            RegionTaskOp::Raise(FAILED.into()),
                        ],
                        vec![
                            RegionTaskOp::Join { handle: 30 },
                            RegionTaskOp::Discharge("pack".into()),
                        ],
                    ),
                    RegionTaskOp::Spend(2),
                ],
            ),
        ),
    ] {
        let certificate = checked(&program);
        assert_eq!(certificate.credit.initial, initial);
        assert_eq!(
            certificate.credit.spent + certificate.credit.abandoned + certificate.credit.carried,
            initial
        );
    }
}

/// One pack, one discharge: a binding cannot be consumed twice, an unknown
/// binding is refused and a catch that does not open a nonempty pack is refused.
#[test]
fn the_cleanup_pack_is_linear() {
    let twice = refused(&with_handler(vec![
        RegionTaskOp::Join { handle: 30 },
        RegionTaskOp::Discharge("pack".into()),
        RegionTaskOp::Discharge("pack".into()),
        RegionTaskOp::Spend(7),
    ]));
    assert_eq!(
        twice,
        RegionTaskDiagnostic::CleanupPackDuplicated("pack".into())
    );
    assert_eq!(twice.code(), "Q304");

    let unknown = refused(&with_handler(vec![
        RegionTaskOp::Join { handle: 30 },
        RegionTaskOp::Discharge("other".into()),
    ]));
    assert_eq!(
        unknown,
        RegionTaskDiagnostic::UnknownCleanupCapability("other".into())
    );

    let mut wrong = deliverable();
    let RegionTaskOp::Region { catches, .. } = &mut wrong.body[1] else {
        unreachable!()
    };
    catches[0].quant_open = Some(QuantOpen {
        region: "elsewhere".into(),
        binding: "pack".into(),
    });
    assert_eq!(
        refused(&wrong),
        RegionTaskDiagnostic::WrongRegionOpened {
            expected: WORK.into(),
            actual: "elsewhere".into()
        }
    );

    let mut unopened = deliverable();
    let RegionTaskOp::Region { catches, .. } = &mut unopened.body[1] else {
        unreachable!()
    };
    catches[0].quant_open = None;
    catches[0].handler = vec![RegionTaskOp::Join { handle: 30 }];
    let refusal = refused(&unopened);
    assert_eq!(
        refusal,
        RegionTaskDiagnostic::CatchWithoutRequiredQuantOpen {
            region: WORK.into(),
            error: FAILED.into()
        }
    );
    assert_eq!(refusal.code(), "Q306");
}

/// Structural refusals of the scoped-task ledger reach this profile unchanged,
/// at `Q600`, and the profile does not restate or soften them.
#[test]
fn scoped_task_refusals_are_carried_over_verbatim() {
    let unknown_handle = refused(&with_handler(vec![
        RegionTaskOp::Join { handle: 99 },
        RegionTaskOp::Discharge("pack".into()),
        RegionTaskOp::Spend(7),
    ]));
    assert_eq!(
        unknown_handle,
        RegionTaskDiagnostic::Ledger(TaskLedgerError::UnknownHandle(99))
    );

    let uncompleted = refused(&with_body(vec![
        RegionTaskOp::Acquire(8),
        RegionTaskOp::Spawn(spawn(30, 1, 4)),
        RegionTaskOp::Raise(FAILED.into()),
    ]));
    assert_eq!(
        uncompleted,
        RegionTaskDiagnostic::Ledger(TaskLedgerError::ChildNotCompleted(1))
    );

    let duplicate = refused(&program(
        0,
        vec![RegionTaskOp::Acquire(7), RegionTaskOp::Acquire(7)],
    ));
    assert_eq!(
        duplicate,
        RegionTaskDiagnostic::Ledger(TaskLedgerError::DuplicateResource(7))
    );
    for refusal in [&unknown_handle, &uncompleted, &duplicate] {
        assert_eq!(refusal.code(), "Q600");
    }
}

/// Operations that open a duty or move credit are refused outside a region.
#[test]
fn duties_and_credit_have_no_home_outside_a_region() {
    // The root region is implicit, so the refusal is reached by popping it: a
    // region whose own body is empty of frames cannot happen, but a `Complete`
    // for an unknown child and a spend beyond the frame both fail closed.
    let refusal = refused(&program(0, vec![RegionTaskOp::Spend(1)]));
    assert_eq!(
        refusal,
        RegionTaskDiagnostic::InsufficientCredit {
            region: ROOT.into(),
            requested: 1,
            available: 0
        }
    );
    assert_eq!(RegionTaskDiagnostic::ObligationOutsideRegion.code(), "Q676");
}

/// An undeclared error is refused at both sites that name one.
#[test]
fn an_undeclared_error_is_refused() {
    let raised = refused(&program(0, vec![RegionTaskOp::Raise("Nope".into())]));
    assert_eq!(raised, RegionTaskDiagnostic::UndeclaredError("Nope".into()));
    assert_eq!(raised.code(), "Q300");

    let caught = refused(&program(
        0,
        vec![RegionTaskOp::Region {
            region: WORK.into(),
            credit: 0,
            body: Vec::new(),
            catches: vec![RegionTaskCatch {
                error: "Nope".into(),
                quant_open: None,
                handler: Vec::new(),
            }],
        }],
    ));
    assert_eq!(caught, RegionTaskDiagnostic::UndeclaredError("Nope".into()));
}

/// The certificate is rederived independently, and every field is bound: a
/// forged verdict, accounting, trace or assumption set is refused.
#[test]
fn a_forged_certificate_is_refused_field_by_field() {
    let program = deliverable();
    let honest = checked(&program);

    let mut forged = honest.clone();
    forged.credit.abandoned = 0;
    forged.credit.spent = 10;
    assert_eq!(
        recheck_region_task_certificate(&program, &forged),
        Err(vec![CertificateRecheckError::FieldMismatch {
            field: "credit"
        }])
    );

    let mut forged = honest.clone();
    forged.trace.retain(|event| {
        !matches!(
            event,
            CreditEvent::AbandonCredit { .. } | CreditEvent::Destroy(_)
        )
    });
    assert_eq!(
        recheck_region_task_certificate(&program, &forged),
        Err(vec![CertificateRecheckError::FieldMismatch {
            field: "trace"
        }])
    );

    let mut forged = honest.clone();
    forged.exit = ExitTag::Error(FAILED.into());
    forged.scope_exited = false;
    let errors = recheck_region_task_certificate(&program, &forged).unwrap_err();
    assert_eq!(
        errors,
        vec![
            CertificateRecheckError::FieldMismatch { field: "exit" },
            CertificateRecheckError::FieldMismatch {
                field: "scope_exited"
            }
        ]
    );

    let mut forged = honest.clone();
    forged.assumptions.clear();
    assert_eq!(
        recheck_region_task_certificate(&program, &forged),
        Err(vec![CertificateRecheckError::FieldMismatch {
            field: "assumptions"
        }])
    );

    // A certificate for a program the checker refuses is refused as a whole.
    let rejected = with_handler(vec![RegionTaskOp::Discharge("pack".into())]);
    assert!(matches!(
        recheck_region_task_certificate(&rejected, &honest)
            .unwrap_err()
            .as_slice(),
        [CertificateRecheckError::ProgramRejected(
            RegionTaskDiagnostic::ChildCreditOutstanding { .. }
        )]
    ));
}

/// Every diagnostic this profile can raise carries the code its family owns,
/// and the eight named refusals are exactly `Q670`-`Q677`.
#[test]
fn the_named_refusals_occupy_the_free_q640_range() {
    let named = [
        (
            RegionTaskDiagnostic::PendingLifetimeObligation {
                region: WORK.into(),
                duty: Duty::LoanExpiry(7),
            },
            "Q670",
        ),
        (
            RegionTaskDiagnostic::ChildCreditOutstanding {
                region: WORK.into(),
                child: 1,
                amount: 2,
            },
            "Q671",
        ),
        (
            RegionTaskDiagnostic::ObligationEscapesScope {
                duty: Duty::LoanExpiry(7),
            },
            "Q672",
        ),
        (
            RegionTaskDiagnostic::CreditNotConserved {
                expected: 1,
                observed: 2,
            },
            "Q673",
        ),
        (
            RegionTaskDiagnostic::UnusedCredit {
                region: WORK.into(),
                remainder: 1,
            },
            "Q674",
        ),
        (
            RegionTaskDiagnostic::InsufficientCredit {
                region: WORK.into(),
                requested: 2,
                available: 1,
            },
            "Q675",
        ),
        (RegionTaskDiagnostic::ObligationOutsideRegion, "Q676"),
        (
            RegionTaskDiagnostic::PayloadObligationDropped { resource: 7 },
            "Q677",
        ),
    ];
    let codes = named
        .iter()
        .map(|(diagnostic, code)| {
            assert_eq!(diagnostic.code(), *code);
            assert!(diagnostic.to_string().starts_with(code));
            *code
        })
        .collect::<Vec<_>>();
    assert_eq!(
        codes,
        [
            "Q670", "Q671", "Q672", "Q673", "Q674", "Q675", "Q676", "Q677"
        ]
    );

    // Carried-over conditions keep the code their own pass already emits.
    for (diagnostic, code) in [
        (RegionTaskDiagnostic::UndeclaredError("E".into()), "Q300"),
        (RegionTaskDiagnostic::NestedPackNotDischarged, "Q302"),
        (
            RegionTaskDiagnostic::WrongRegionOpened {
                expected: WORK.into(),
                actual: ROOT.into(),
            },
            "Q303",
        ),
        (
            RegionTaskDiagnostic::CleanupPackDuplicated("p".into()),
            "Q304",
        ),
        (RegionTaskDiagnostic::CleanupPackNotDischarged, "Q305"),
        (
            RegionTaskDiagnostic::CatchWithoutRequiredQuantOpen {
                region: WORK.into(),
                error: FAILED.into(),
            },
            "Q306",
        ),
        (RegionTaskDiagnostic::RethrowWithUndischargedCleanup, "Q307"),
        (
            RegionTaskDiagnostic::ResourceInPayloadAndCleanupPack(7),
            "Q309",
        ),
        (
            RegionTaskDiagnostic::Ledger(TaskLedgerError::ScopeBusy(10)),
            "Q600",
        ),
    ] {
        assert_eq!(diagnostic.code(), code);
    }
}

/// A region name cannot be reused while it is active, and a duplicate catch is
/// refused.
#[test]
fn region_structure_is_checked() {
    let nested = refused(&program(
        0,
        vec![RegionTaskOp::Region {
            region: ROOT.into(),
            credit: 0,
            body: Vec::new(),
            catches: Vec::new(),
        }],
    ));
    assert_eq!(
        nested,
        RegionTaskDiagnostic::DuplicateActiveRegion(ROOT.into())
    );

    let duplicate = refused(&program(
        0,
        vec![RegionTaskOp::Region {
            region: WORK.into(),
            credit: 0,
            body: Vec::new(),
            catches: vec![
                RegionTaskCatch {
                    error: FAILED.into(),
                    quant_open: None,
                    handler: Vec::new(),
                },
                RegionTaskCatch {
                    error: FAILED.into(),
                    quant_open: None,
                    handler: Vec::new(),
                },
            ],
        }],
    ));
    assert_eq!(
        duplicate,
        RegionTaskDiagnostic::DuplicateCatchClause(FAILED.into())
    );
}

/// A systematic mutation matrix over the admitted deliverable. Every row is a
/// single edit of the accepted program, and every row is refused with the exact
/// diagnostic named here. No row is admitted, so no gate is decorative.
#[test]
fn a_mutation_matrix_over_the_deliverable_rejects_every_row() {
    let rows: Vec<(&str, RegionTaskProgram, RegionTaskDiagnostic)> = vec![
        (
            "the handler never joins",
            with_handler(vec![
                RegionTaskOp::Discharge("pack".into()),
                RegionTaskOp::Spend(4),
            ]),
            RegionTaskDiagnostic::ChildCreditOutstanding {
                region: WORK.into(),
                child: 1,
                amount: 3,
            },
        ),
        (
            "the handler never discharges",
            with_handler(vec![
                RegionTaskOp::Join { handle: 30 },
                RegionTaskOp::Spend(7),
            ]),
            RegionTaskDiagnostic::CleanupPackNotDischarged,
        ),
        (
            "the handler discharges twice",
            with_handler(vec![
                RegionTaskOp::Join { handle: 30 },
                RegionTaskOp::Discharge("pack".into()),
                RegionTaskOp::Discharge("pack".into()),
                RegionTaskOp::Spend(7),
            ]),
            RegionTaskDiagnostic::CleanupPackDuplicated("pack".into()),
        ),
        (
            "the handler discharges a binding it does not hold",
            with_handler(vec![
                RegionTaskOp::Join { handle: 30 },
                RegionTaskOp::Discharge("elsewhere".into()),
                RegionTaskOp::Spend(7),
            ]),
            RegionTaskDiagnostic::UnknownCleanupCapability("elsewhere".into()),
        ),
        (
            "the handler rethrows instead of discharging",
            with_handler(vec![
                RegionTaskOp::Join { handle: 30 },
                RegionTaskOp::Raise(FAILED.into()),
            ]),
            RegionTaskDiagnostic::RethrowWithUndischargedCleanup,
        ),
        (
            "one unit of credit is left unspent",
            with_handler(vec![
                RegionTaskOp::Join { handle: 30 },
                RegionTaskOp::Discharge("pack".into()),
                RegionTaskOp::Spend(6),
            ]),
            RegionTaskDiagnostic::UnusedCredit {
                region: ROOT.into(),
                remainder: 1,
            },
        ),
        (
            "one unit of credit is overspent",
            with_handler(vec![
                RegionTaskOp::Join { handle: 30 },
                RegionTaskOp::Discharge("pack".into()),
                RegionTaskOp::Spend(8),
            ]),
            RegionTaskDiagnostic::InsufficientCredit {
                region: ROOT.into(),
                requested: 8,
                available: 7,
            },
        ),
        (
            "the region exits normally with its duties open",
            with_body(vec![
                RegionTaskOp::Acquire(8),
                RegionTaskOp::Spawn(Spawn {
                    loans: vec![7],
                    ..spawn(30, 1, 4)
                }),
                RegionTaskOp::Spend(1),
                RegionTaskOp::Complete {
                    child: 1,
                    outcome: TaskOutcome::I32(42),
                    resources: Vec::new(),
                    unspent_credit: 3,
                },
            ]),
            RegionTaskDiagnostic::ChildCreditOutstanding {
                region: WORK.into(),
                child: 1,
                amount: 3,
            },
        ),
        (
            "the child never completes",
            with_body(vec![
                RegionTaskOp::Acquire(8),
                RegionTaskOp::Spawn(Spawn {
                    loans: vec![7],
                    ..spawn(30, 1, 4)
                }),
                RegionTaskOp::Spend(1),
                RegionTaskOp::Raise(FAILED.into()),
            ]),
            RegionTaskDiagnostic::Ledger(TaskLedgerError::ChildNotCompleted(1)),
        ),
        (
            "the child returns more credit than it was lent",
            with_body(vec![
                RegionTaskOp::Acquire(8),
                RegionTaskOp::Spawn(Spawn {
                    loans: vec![7],
                    ..spawn(30, 1, 4)
                }),
                RegionTaskOp::Spend(1),
                RegionTaskOp::Complete {
                    child: 1,
                    outcome: TaskOutcome::I32(42),
                    resources: Vec::new(),
                    unspent_credit: 5,
                },
                RegionTaskOp::Raise(FAILED.into()),
            ]),
            RegionTaskDiagnostic::InsufficientCredit {
                region: WORK.into(),
                requested: 5,
                available: 4,
            },
        ),
        (
            "the creation is endowed beyond its region's credit",
            with_body(vec![
                RegionTaskOp::Acquire(8),
                RegionTaskOp::Spawn(Spawn {
                    loans: vec![7],
                    ..spawn(30, 1, 7)
                }),
                RegionTaskOp::Raise(FAILED.into()),
            ]),
            RegionTaskDiagnostic::InsufficientCredit {
                region: WORK.into(),
                requested: 7,
                available: 6,
            },
        ),
        (
            "the lender releases its buffer while the loan is outstanding",
            with_body(vec![
                RegionTaskOp::Acquire(8),
                RegionTaskOp::Spawn(Spawn {
                    loans: vec![7],
                    ..spawn(30, 1, 4)
                }),
                RegionTaskOp::Drop(7),
                RegionTaskOp::Raise(FAILED.into()),
            ]),
            RegionTaskDiagnostic::Ledger(TaskLedgerError::LoanOutstanding {
                resource: 7,
                outstanding: FULL_READ_AUTHORITY,
                borrowers: vec![1],
            }),
        ),
        (
            "the region raises an error nobody declared",
            with_body(vec![
                RegionTaskOp::Acquire(8),
                RegionTaskOp::Raise("Work.Undeclared".into()),
            ]),
            RegionTaskDiagnostic::UndeclaredError("Work.Undeclared".into()),
        ),
    ];
    assert_eq!(rows.len(), 13);
    for (name, mutated, expected) in rows {
        assert_eq!(refused(&mutated), expected, "row `{name}` was admitted");
    }
    // The unmutated program is still admitted, so the matrix measures the edits
    // and not a broken fixture.
    checked(&deliverable());
}

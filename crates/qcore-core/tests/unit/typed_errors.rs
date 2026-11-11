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
        Effect::Alloc,
        Effect::Io,
        Effect::Throws("IoError".into()),
        Effect::Throws("ParseError".into()),
    ])
}

#[test]
fn raise_has_no_normal_exit_and_preserves_the_ledger() {
    let initial = ErrorLedger::new([7]);
    let exits = analyze_typed_errors(
        &declarations(),
        &allowed(),
        initial.clone(),
        &[ErrorOp::Raise("IoError".into())],
    )
    .unwrap();
    assert!(exits.states(&ExitTag::Normal).is_empty());
    assert_eq!(
        exits.states(&ExitTag::Error("IoError".into()))[0].ledger(),
        &initial
    );
}

#[test]
fn raise_must_be_declared_and_present_in_the_effect_row() {
    let unknown = analyze_typed_errors(
        &declarations(),
        &allowed(),
        ErrorLedger::default(),
        &[ErrorOp::Raise("Missing".into())],
    )
    .unwrap_err();
    assert_eq!(unknown, TypedErrorError::UnknownErrorType("Missing".into()));

    let forbidden = analyze_typed_errors(
        &declarations(),
        &EffectRow::default(),
        ErrorLedger::default(),
        &[ErrorOp::Raise("IoError".into())],
    )
    .unwrap_err();
    assert_eq!(
        forbidden,
        TypedErrorError::RaisedErrorNotInEffectRow("IoError".into())
    );
}

#[test]
fn question_mark_preserves_each_declared_post_ledger() {
    let summary = CallSummary::new()
        .add_normal(CallExit::new(
            LedgerTransition::new([1], []).unwrap(),
            EffectRow::new([Effect::Io]),
        ))
        .add_error(
            "IoError",
            CallExit::new(
                LedgerTransition::new([], [2]).unwrap(),
                EffectRow::new([Effect::Io]),
            ),
        );
    let exits = analyze_typed_errors(
        &declarations(),
        &allowed(),
        ErrorLedger::new([1]),
        &[ErrorOp::Try(summary)],
    )
    .unwrap();
    assert_eq!(
        exits.states(&ExitTag::Normal)[0].ledger(),
        &ErrorLedger::default()
    );
    assert_eq!(
        exits.states(&ExitTag::Error("IoError".into()))[0].ledger(),
        &ErrorLedger::new([1, 2])
    );
}

#[test]
fn propagated_error_must_be_in_the_callers_effect_row() {
    let summary = CallSummary::new().add_error(
        "IoError",
        CallExit::new(LedgerTransition::identity(), EffectRow::default()),
    );
    let error = analyze_typed_errors(
        &declarations(),
        &EffectRow::default(),
        ErrorLedger::default(),
        &[ErrorOp::Try(summary)],
    )
    .unwrap_err();
    assert_eq!(
        error,
        TypedErrorError::UnhandledPropagatedError("IoError".into())
    );
}

#[test]
fn qualitative_effects_are_checked_separately_from_resource_bounds() {
    assert_eq!(
        analyze_typed_errors(
            &declarations(),
            &EffectRow::default(),
            ErrorLedger::default(),
            &[ErrorOp::Perform(Effect::Io)]
        ),
        Err(TypedErrorError::EffectNotInRow(Effect::Io))
    );
    let exits = analyze_typed_errors(
        &declarations(),
        &allowed(),
        ErrorLedger::default(),
        &[ErrorOp::Perform(Effect::Io)],
    )
    .unwrap();
    assert!(exits.inferred_effects().contains(&Effect::Io));
}

#[test]
fn error_post_ledgers_remain_path_sensitive() {
    let exits = analyze_typed_errors(
        &declarations(),
        &allowed(),
        ErrorLedger::default(),
        &[ErrorOp::Branch {
            then_ops: vec![ErrorOp::Acquire(1), ErrorOp::Raise("IoError".into())],
            else_ops: vec![ErrorOp::Acquire(2), ErrorOp::Raise("IoError".into())],
        }],
    )
    .unwrap();
    let errors = exits.states(&ExitTag::Error("IoError".into()));
    assert_eq!(errors.len(), 2);
    assert_eq!(errors[0].ledger(), &ErrorLedger::new([1]));
    assert_eq!(errors[1].ledger(), &ErrorLedger::new([2]));
}

#[test]
fn catch_without_cleanup_accepts_only_the_entry_ledger() {
    let caught = ErrorOp::Catch {
        body: vec![ErrorOp::Raise("IoError".into())],
        clauses: vec![CatchClause {
            error: "IoError".into(),
            handler: vec![],
        }],
    };
    let exits = check_no_unhandled_errors(
        &declarations(),
        &allowed(),
        ErrorLedger::new([9]),
        &[caught],
    )
    .unwrap();
    assert_eq!(
        exits.states(&ExitTag::Normal)[0].ledger(),
        &ErrorLedger::new([9])
    );
    assert!(
        !exits
            .inferred_effects()
            .contains(&Effect::Throws("IoError".into()))
    );

    let leaking = ErrorOp::Catch {
        body: vec![ErrorOp::Acquire(1), ErrorOp::Raise("IoError".into())],
        clauses: vec![CatchClause {
            error: "IoError".into(),
            handler: vec![],
        }],
    };
    assert!(matches!(
        analyze_typed_errors(
            &declarations(),
            &allowed(),
            ErrorLedger::default(),
            &[leaking]
        ),
        Err(TypedErrorError::CatchHasLocalObligations { .. })
    ));
}

#[test]
fn branch_and_catch_normal_joins_are_checked() {
    let branch = ErrorOp::Branch {
        then_ops: vec![ErrorOp::Acquire(1)],
        else_ops: vec![],
    };
    assert_eq!(
        analyze_typed_errors(
            &declarations(),
            &allowed(),
            ErrorLedger::default(),
            &[branch]
        ),
        Err(TypedErrorError::BranchNormalLedgerMismatch)
    );

    let catch = ErrorOp::Catch {
        body: vec![ErrorOp::Branch {
            then_ops: vec![],
            else_ops: vec![ErrorOp::Raise("IoError".into())],
        }],
        clauses: vec![CatchClause {
            error: "IoError".into(),
            handler: vec![ErrorOp::Acquire(1)],
        }],
    };
    assert_eq!(
        analyze_typed_errors(
            &declarations(),
            &allowed(),
            ErrorLedger::default(),
            &[catch]
        ),
        Err(TypedErrorError::CatchNormalLedgerMismatch)
    );
}

#[test]
fn error_free_boundary_rejects_a_remaining_error_exit() {
    assert_eq!(
        check_no_unhandled_errors(
            &declarations(),
            &allowed(),
            ErrorLedger::default(),
            &[ErrorOp::Raise("ParseError".into())]
        ),
        Err(TypedErrorError::UnhandledError("ParseError".into()))
    );
}

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

//! Real child-process record exchange. This does not admit a protocol witness.
#[path = "../../../crates/qcore-verification/tests/fixtures/reactor_program.rs"]
mod fixture;
use fixture::*;
use qcore_core::event_reactor::{self as model, EventOperation};
use qcore_schemas::reactor::ReactorCallbackProposal;
use qcore_verification::reactor_callback::*;
use std::process::Command;

fn proposal(mode: &str) -> ReactorCallbackProposal {
    let output = Command::new(env!("CARGO_BIN_EXE_callback_proposal_writer"))
        .arg(mode)
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn schema_only_process_cannot_grant_callback_authority() {
    let mut program = one_shot(1, key(1), 9);
    if let EventOperation::Assign { summary, .. } = &mut program.operations[1] {
        summary
            .payload_authority
            .insert(7, model::PayloadAccessKind::Read);
    }
    if let EventOperation::Access { kind, .. } = &mut program.operations[6] {
        *kind = model::PayloadAccessKind::Read;
    }
    let valid = proposal("valid");
    assert_eq!(valid, declared_callback_proposal(&program));
    assert_eq!(
        check_declared_callback_proposal(&program, &[TestProfile(true)], &valid),
        Ok(model::check_event_program(&program, &[TestProfile(true)]).unwrap())
    );
    for (mode, expected) in [
        ("widen", CallbackProposalError::SummaryMismatch(1)),
        (
            "generation",
            CallbackProposalError::EventIdentityMismatch(1),
        ),
        ("omit", CallbackProposalError::MissingOperation(1)),
        ("duplicate", CallbackProposalError::DuplicateOperation(1)),
        ("version", CallbackProposalError::UnsupportedVersion(2)),
    ] {
        assert_eq!(
            check_declared_callback_proposal(&program, &[TestProfile(true)], &proposal(mode)),
            Err(expected),
            "{mode}"
        );
    }
}

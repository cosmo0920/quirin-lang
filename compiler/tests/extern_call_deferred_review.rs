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

//! Card W45-CONTRACT-DEFERRED-REVIEW on the checked extern-call route
//! (`source.extern-call.v1`): a selected declaration awaiting human review
//! admits no call, and the refusal names the review.
//!
//! The contract text is the extern-call fixture with one marker added in
//! memory, so no fixture file and no corpus pin moves.

use std::collections::BTreeMap;

use qcore_compiler::{CompileOptions, SourceExecutionProfile, SourceFile, compile_files};

const CONTRACTS: &str = include_str!("fixtures/extern_calls/contracts/buffer_api.qv");
const ROUND_TRIP: &str = include_str!("fixtures/extern_calls/round_trip.qr");

fn compile_with(
    contracts: String,
) -> Result<qcore_compiler::CompiledModule, Vec<qcore_compiler::FileDiagnostic>> {
    compile_files(
        &[SourceFile::new("extern.qcore", ROUND_TRIP)],
        &CompileOptions {
            entry: None,
            extern_contracts: Some(BTreeMap::from([("buffer_api.qcore".to_owned(), contracts)])),
            native_libraries: None,
            native_library_headers: None,
        },
    )
}

/// The release declaration with a contract-level marker, or its custody clause
/// wrapped in a clause-level one.
fn deferred(clause_level: bool) -> String {
    let contracts = CONTRACTS.replace("\r\n", "\n");
    const DECLARATION: &str = "extern fn qcore_extern_release(buffer: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_consumed(buffer, nonnull);
    ensures normal resource_return_none();
};";
    assert_eq!(
        contracts.matches(DECLARATION).count(),
        1,
        "the fixture shape moved"
    );
    let marked = if clause_level {
        DECLARATION.replace(
            "ensures normal resource_consumed(buffer, nonnull);",
            "ensures normal resource_review_pending(resource_consumed(buffer, nonnull), \
             \"is the buffer freed on every path?\");",
        )
    } else {
        DECLARATION.replace(
            "requires resource_contract_v1;",
            "requires resource_contract_v1;\n    \
             requires resource_review_pending(\"is the buffer freed on every path?\");",
        )
    };
    contracts.replace(DECLARATION, &marked)
}

#[test]
fn the_control_admits_the_round_trip() {
    let module = compile_with(CONTRACTS.to_owned()).unwrap();
    assert_eq!(
        module.execution_profile,
        SourceExecutionProfile::ExternCallsV1
    );
}

#[test]
fn a_pending_callee_admits_no_call_and_names_the_review() {
    for clause_level in [false, true] {
        let diagnostics = compile_with(deferred(clause_level))
            .err()
            .unwrap_or_else(|| panic!("a pending contract admits no call ({clause_level})"));
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.code.as_str())
                .collect::<Vec<_>>(),
            ["Q023"]
        );
        let message = &diagnostics[0].message;
        assert!(
            message.contains("`qcore_extern_release`'s declared contract")
                && message.contains("awaits human review (awaiting-human-review)")
                && message.contains("is the buffer freed on every path?"),
            "{message}"
        );
    }
}

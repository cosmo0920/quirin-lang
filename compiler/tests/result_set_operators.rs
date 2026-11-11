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

//! Card CELL-GUARD-RESULT-RANGE-CHECKED (grammar G05j): the set operators
//! `|`, `&` and `^` of an `ensures normal result_set(SET);` clause are the
//! set's own, read literally by the bounds reader, so the frontend's operator
//! gate (`Q038`) admits them there and nowhere else in a contract module.

use qcore_compiler::{CompileOptions, SourceFile, compile_files};

fn module(clause: &str) -> String {
    format!(
        "module contract;
extern fn pipe(pipedes: ptr<mut, u8>) -> i32
contract {{
    requires bounds_contract_v1;
{clause}
}};
"
    )
}

fn codes(text: &str) -> Vec<String> {
    match compile_files(
        &[SourceFile::new("result_sets.qv", text)],
        &CompileOptions::default(),
    ) {
        Ok(_) => Vec::new(),
        Err(diagnostics) => diagnostics.into_iter().map(|d| d.code).collect(),
    }
}

#[test]
fn a_result_set_keeps_its_operators() {
    for clause in [
        "    requires bounds_revision(flexible);\n    ensures normal result_set(-1 | 0);",
        "    requires bounds_revision(flexible);\n    ensures normal result_set([-1, 5] & [0, MAX]);",
        "    requires bounds_revision(flexible);\n    ensures normal result_set([MIN, MAX] ^ 0);",
        "    requires bounds_revision(flexible);\n    ensures normal result_set((-1 | [0, 3]) & [2, 9] ^ 3);",
    ] {
        assert_eq!(codes(&module(clause)), Vec::<String>::new(), "{clause}");
    }
}

#[test]
fn an_operator_anywhere_else_in_a_contract_still_refuses() {
    for clause in [
        "    requires bounds_extent(pipedes, 2 | 1, elements);",
        "    ensures normal nonnull_result_range(-1 | 0, 0);",
        "    requires result_set(-1 | 0);",
    ] {
        assert!(
            codes(&module(clause)).contains(&"Q038".to_owned()),
            "{clause}"
        );
    }
    // The logical connectives are no set operators.
    assert!(codes(&module("    ensures normal result_set(-1 || 0);")).contains(&"Q038".to_owned()));
}

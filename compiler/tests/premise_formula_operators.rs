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

//! Card SP4-SYMBOLIC-LIBEVENT-PREMISES (contract specification C04.18.9,
//! grammar G05-formula): the `&&` and `|` of a `requires
//! resource_premise(NAME, FORMULA);` statement are the formula's conjunction
//! and set union, read by the premise reader, so the frontend's operator gate
//! (`Q038`) admits them there and nowhere else in a contract module. A
//! malformed formula is still refused, by the contract format's premise
//! reader.

use qcore_compiler::{CompileOptions, SourceFile, compile_files};

fn module(clause: &str) -> String {
    format!(
        "module contract;
extern fn destroy(mth: ptr<mut, u8>) -> i32
contract {{
    requires resource_contract_v1;
{clause}
    ensures normal resource_borrowed(mth);
    ensures normal resource_return_none();
}};
"
    )
}

fn codes(text: &str) -> Vec<String> {
    match compile_files(
        &[SourceFile::new("premise_formulas.qv", text)],
        &CompileOptions::default(),
    ) {
        Ok(_) => Vec::new(),
        Err(diagnostics) => diagnostics.into_iter().map(|d| d.code).collect(),
    }
}

#[test]
fn a_premise_formula_keeps_its_operators() {
    for clause in [
        "    requires resource_premise(chain,\n        quant all #m in #args: #mk_session(#m) in #live\n        && #mk_parent(#m) in #live \\ (#released | #ended));",
        "    requires resource_premise(writers, #concurrent |-> #no_writes(#singleton(#libco_params_key) | #tracked));",
        "    requires resource_premise(slots, #stated |-> #inert && #slots(#args) \\ #stated |-> #inert);",
    ] {
        assert_eq!(codes(&module(clause)), Vec::<String>::new(), "{clause}");
    }
}

#[test]
fn an_operator_anywhere_else_in_a_contract_still_refuses() {
    for clause in [
        // Not a premise: the same operators keep their refusal.
        "    requires resource_borrowed(mth && mth);",
        "    requires resource_premise_all(a | b);",
    ] {
        assert!(
            codes(&module(clause)).contains(&"Q038".to_owned()),
            "{clause}"
        );
    }
    // The logical-or connective is no formula operator.
    assert!(
        !codes(&module(
            "    requires resource_premise(p, #stated |-> #inert || #stated |-> #inert);"
        ))
        .is_empty()
    );
}

#[test]
fn a_malformed_formula_is_still_refused() {
    for clause in [
        "    requires resource_premise(p, #unknown |-> #inert && #stated |-> #inert);",
        "    requires resource_premise(p, #args | #stated |-> #inert);",
        "    requires resource_premise(p, #concurrent |-> #no_writes(#tracked) && #no_resumes(#tracked));",
    ] {
        assert!(!codes(&module(clause)).is_empty(), "{clause}");
    }
}

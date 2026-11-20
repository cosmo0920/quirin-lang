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

//! Card SP2-SYMBOLIC-LIBRDKAFKA-PREMISES: the set union `|` and the
//! conjunction `&&` of a `requires resource_premise(NAME, FORMULA);` clause
//! (G05-formula, contract specification C04.18.9) are the formula's own,
//! read by the premise reader, so the frontend's operator gate (`Q038`)
//! admits them there and nowhere else in a contract module. Before the card a
//! shipped module stating a formula failed the contract corpus's compiler
//! regression on its `|`; SP1's OpenSSL modules belong to a user crate the
//! corpus does not compile.

use qcore_compiler::{CompileOptions, SourceFile, compile_files};

fn module(clause: &str) -> String {
    format!(
        "module contract;
extern fn rd_kafka_q_purge0(rkq: ptr<mut, u8>, do_lock: i32) -> i32
contract {{
    requires resource_contract_v1;
{clause}
    ensures normal resource_borrowed(rkq);
    ensures normal resource_borrowed(do_lock);
    ensures normal resource_return_none();
}};
"
    )
}

fn codes(text: &str) -> Vec<String> {
    match compile_files(
        &[SourceFile::new("premise.qv", text)],
        &CompileOptions::default(),
    ) {
        Ok(_) => Vec::new(),
        Err(diagnostics) => diagnostics.into_iter().map(|d| d.code).collect(),
    }
}

#[test]
fn a_premise_formula_keeps_its_union_and_conjunction() {
    for clause in [
        "    requires resource_premise(served_queue, #op_callbacks |-> #takes(#held_on(#served)) == {});",
        "    requires resource_premise(closure, (quant all #f in #ran(#slots(#rk_conf)) \\ #loaded_registered: #f in #registered | #defaults && #f in #candidates) && quant all #f in #loaded_registered: #f in #enveloped);",
        "    requires resource_premise(prose, \"an op callback takes over no reference\");",
    ] {
        assert_eq!(codes(&module(clause)), Vec::<String>::new(), "{clause}");
    }
}

#[test]
fn a_union_or_conjunction_anywhere_else_in_a_contract_still_refuses() {
    for clause in [
        "    requires resource_borrowed(rkq) && resource_borrowed(do_lock);",
        "    requires resource_borrowed(rkq) | resource_borrowed(do_lock);",
    ] {
        assert!(
            codes(&module(clause)).contains(&"Q038".to_owned()),
            "{clause}"
        );
    }
    // A single ampersand inside a formula is no operator of the formula: the
    // premise reader refuses the module.
    assert!(
        !codes(&module(
            "    requires resource_premise(served_queue, #op_callbacks & #registered |-> #inert);"
        ))
        .is_empty()
    );
}

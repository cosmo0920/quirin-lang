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

//! Card SP3-SYMBOLIC-FLUENT-BIT-LIBC-PREMISES (contract specification
//! C04.18.9, grammar G05-formula): the conjunction `&&` and the set union `|`
//! of a `requires resource_premise(NAME, FORMULA);` clause are the formula's
//! own, read by the premise reader, so the frontend's operator gate (`Q038`)
//! admits them there: in a formula the reader accepts, and nowhere else in a
//! contract module.

use qcore_compiler::{CompileOptions, SourceFile, compile_files};

fn module(clause: &str) -> String {
    format!(
        "module contract;
extern fn get(ic: ptr<mut, u8>) -> ptr<mut, u8>
contract {{
    requires resource_contract_v1;
{clause}
    ensures normal resource_borrowed(ic);
    ensures normal resource_return_none();
}};
"
    )
}

fn codes(text: &str) -> Vec<String> {
    match compile_files(
        &[SourceFile::new("premises.qv", text)],
        &CompileOptions::default(),
    ) {
        Ok(_) => Vec::new(),
        Err(diagnostics) => diagnostics.into_iter().map(|d| d.code).collect(),
    }
}

#[test]
fn a_premise_formula_keeps_its_operators() {
    for clause in [
        "    requires resource_premise(p, #admitted -> #faithful && #faithful);",
        "    requires resource_premise(p, quant all #r in #result: #r in #result \\ (#bound | #stored));",
        "    requires resource_premise(p,\n        quant all #r in #result: #r in #interior(#args)\n        && #r in #result \\ (#tracked | #claimed));",
    ] {
        assert_eq!(codes(&module(clause)), Vec::<String>::new(), "{clause}");
    }
}

#[test]
fn an_operator_anywhere_else_in_a_contract_still_refuses() {
    // Outside a premise clause.
    assert!(
        codes(&module("    requires resource_borrowed(ic | ic);")).contains(&"Q038".to_owned())
    );
    // The logical-or connective is no formula operator.
    assert!(
        !codes(&module(
            "    requires resource_premise(p, #faithful || #admitted);"
        ))
        .is_empty()
    );
    // A formula the reader refuses rejects the module.
    assert!(
        !codes(&module(
            "    requires resource_premise(p, #unknown && #faithful);"
        ))
        .is_empty()
    );
}

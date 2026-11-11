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

//! Unit tests of card CG26-VARIADICS (gateway rule 11w): the variadic
//! parameter list, the promoted-type premise and the borrowed tail contract.

use super::*;
use qcore_schemas::contract::ResourceReturnMode;
use serde_json::json;

#[test]
fn cg26_the_ellipsis_declares_no_object() {
    assert_eq!(
        named_parameters(&["int", "n", ",", ".", ".", "."]),
        &["int", "n"]
    );
    assert_eq!(
        named_parameters(&[
            "const", "char", "*", "f", ",", "int", "x", ",", ".", ".", "."
        ]),
        &["const", "char", "*", "f", ",", "int", "x"]
    );
    // A lone ellipsis, a non-trailing one, and an ordinary list are unchanged
    // (the reader refuses the first two on their `.` tokens).
    assert_eq!(named_parameters(&[".", ".", "."]), &[".", ".", "."]);
    assert_eq!(
        named_parameters(&[",", ".", ".", "."]),
        &[",", ".", ".", "."]
    );
    assert_eq!(named_parameters(&["int", "a"]), &["int", "a"]);
}

fn typed(qual: &str) -> serde_json::Value {
    json!({"kind": "ImplicitCastExpr", "type": {"qualType": qual}})
}

#[test]
fn cg26_promoted_types_are_scalars_or_pointers() {
    for admitted in [
        "int",
        "unsigned int",
        "long",
        "unsigned long long",
        "double",
        "char *",
        "const char *",
        "void *",
        "struct flb_input_instance *",
        "int (*)(void)",
        "enum kind",
    ] {
        assert!(promoted_scalar_or_pointer(&typed(admitted)), "{admitted}");
    }
    for refused in [
        "struct R",
        "union U",
        "float",
        "char",
        "short",
        "_Complex double",
        "int __attribute__((ext_vector_type(4)))",
        "",
    ] {
        assert!(!promoted_scalar_or_pointer(&typed(refused)), "{refused}");
    }
    // The desugared spelling decides when Clang gives one.
    assert!(!promoted_scalar_or_pointer(&json!({
        "type": {"qualType": "flb_sds_t", "desugaredQualType": "struct cfl_sds"}
    })));
    assert!(promoted_scalar_or_pointer(&json!({
        "type": {"qualType": "flb_sds_t", "desugaredQualType": "char *"}
    })));
}

#[test]
fn cg26_the_borrowed_tail_contract_only_grows_the_parameters() {
    let declared = ResourceFunctionContract::new(
        [
            ResourceParameterMode::Borrowed,
            ResourceParameterMode::Borrowed,
        ],
        ResourceReturnMode::None,
    );
    let extended = borrowed_tail_contract(&declared, 3);
    assert_eq!(extended.parameters.len(), 5);
    assert!(
        extended.parameters[2..]
            .iter()
            .all(|mode| matches!(mode, ResourceParameterMode::Borrowed))
    );
    assert_eq!(extended.parameters[..2], declared.parameters[..]);
    assert_eq!(extended.result, declared.result);
}

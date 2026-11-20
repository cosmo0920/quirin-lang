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

//! Card **CG10-READER-GAPS** (gateway rule 71b): `T *` for a function typedef
//! `T` is a function pointer.
//!
//! Rule 10v classes a stored value by its Clang type spelling
//! ([`crate::typed_access_path::qual`]): a function pointer is spelled
//! `R (*)(A…)`. Clang desugars only a type's top level, so a formal declared
//! `T *f` with `typedef R (T)(A…);` is spelled `T *`, with no `(`, and was read
//! as an object pointer: rule 70 retained it (`rd_kafka_buf_set_maker`'s
//! `make_cb`), an obligation a function pointer does not carry.
//!
//! Rule 71b reads such a value as rule 10v's function-pointer name exactly
//! when its type spelling is the two tokens `NAME *` and every `TypedefDecl`
//! of `NAME` in the unit is directly a function type: its type node, through
//! `ParenType` and `ElaboratedType` only, is a `FunctionProtoType` or a
//! `FunctionNoProtoType` ([`declares_function_type`]). A typedef of another
//! typedef, of a pointer, of an array of function pointers or of a record, a
//! name with two disagreeing declarations, and every other spelling keep the
//! object-pointer reading (more obligation, never less). A pointer to a
//! function type is a function pointer (C11 6.2.5p20), which designates no
//! object (6.2.4, 6.3.2.3p8): it carries no lifetime, so rule 10v's own
//! function-pointer decision (the flow proves the name carries no tracked
//! identity) and rule 70's subclass (an overwritten path, no retention)
//! apply unchanged. `QCore.FunctionTypedefPointer` is the model, bound to
//! [`DECISION_VECTORS`].

use serde_json::Value;

use crate::typed_c_ast::{children, kind};

/// Whether a `TypedefDecl` node is directly a function type.
pub(crate) fn declares_function_type(node: &Value) -> bool {
    let mut cursor = children(node).first();
    while let Some(inner) = cursor {
        match kind(inner) {
            "ParenType" | "ElaboratedType" => cursor = children(inner).first(),
            "FunctionProtoType" | "FunctionNoProtoType" => return true,
            _ => return false,
        }
    }
    false
}

/// Rule 71b: whether the type spelling `ty` is `NAME *` for a function
/// typedef `NAME` (`function_typedef` answers for the unit).
pub(crate) fn names_function_pointer(ty: &str, function_typedef: &dyn Fn(&str) -> bool) -> bool {
    let spaced = ty.replace('*', " * ");
    match spaced.split_whitespace().collect::<Vec<_>>().as_slice() {
        [name, "*"] => crate::typed_field_access::is_name(name) && function_typedef(name),
        _ => false,
    }
}

/// `(type spelling, NAME a function typedef?, read as a function pointer?)`:
/// the vectors `QCore.FunctionTypedefPointer.decisionVectors` restates.
#[cfg(test)]
pub(crate) const DECISION_VECTORS: &[(&str, bool, bool)] = &[
    ("cb_t *", true, true),
    ("cb_t*", true, true),
    ("cb_t *", false, false),
    ("cb_t **", true, false),
    ("const cb_t *", true, false),
    ("cb_t", true, false),
    ("int (*)(void *)", true, false),
    ("struct s *", true, false),
];

#[cfg(test)]
mod tests {
    use super::{DECISION_VECTORS, declares_function_type, names_function_pointer};

    #[test]
    fn rule_71b_decision_vectors_hold() {
        for (ty, typedef, expected) in DECISION_VECTORS {
            let answer = names_function_pointer(ty, &|name| *typedef && name == "cb_t");
            assert_eq!(answer, *expected, "{ty} {typedef}");
        }
    }

    #[test]
    fn only_a_direct_function_type_is_a_function_typedef() {
        let node = |inner: &str| -> serde_json::Value {
            serde_json::from_str(&format!(r#"{{"kind":"TypedefDecl","inner":[{inner}]}}"#))
                .expect("json")
        };
        assert!(declares_function_type(&node(
            r#"{"kind":"ParenType","inner":[{"kind":"FunctionProtoType"}]}"#
        )));
        assert!(declares_function_type(&node(r#"{"kind":"FunctionNoProtoType"}"#)));
        assert!(!declares_function_type(&node(
            r#"{"kind":"PointerType","inner":[{"kind":"ParenType"}]}"#
        )));
        assert!(!declares_function_type(&node(r#"{"kind":"TypedefType"}"#)));
        assert!(!declares_function_type(&node(r#"{"kind":"ConstantArrayType"}"#)));
    }
}

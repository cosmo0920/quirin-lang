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

//! Card CG12-VOID-VIEW-READBACK: the decision tables, bound to
//! `proof/QCore/VoidViewReadback.lean` by their shared vectors.

use super::*;

fn code(result: Result<(), ViewRefusal>) -> &'static str {
    match result {
        Ok(()) => "view",
        Err(refusal) => refusal.code(),
    }
}

#[test]
fn every_view_vector_is_the_class_decision() {
    assert_eq!(VIEW_VECTORS.len(), 11);
    for (operand, cast, target, decision) in VIEW_VECTORS {
        let result = view_of(
            cast_class(cast),
            operand_class(operand),
            target_class(target),
        );
        assert_eq!(code(result), *decision, "{operand} {cast} {target}");
    }
}

/// Each row's spelling, read by Clang: the one explicit cast of
/// `(TARGET) a` with `a` declared `OPERAND`, classified by
/// [`classify_view`], is the row's decision.
#[test]
fn every_view_vector_is_clang_s_reading() {
    use crate::legacy_c::CCompilerDialect;
    use crate::typed_access_path::{StatementIndex, enter_source};
    if !crate::test_support::clang_available() {
        return;
    }
    for (operand, cast, target, decision) in VIEW_VECTORS {
        let source = format!(
            "struct T {{ int x; }}; struct A {{ int y; }}; void f({operand} a) {{ (void) ({target}) a; }}"
        );
        let _scope = enter_source(&source, CCompilerDialect::C11, None);
        let index = StatementIndex::read("f").expect("the fixture parses");
        let mut casts = Vec::new();
        let mut parents = Vec::new();
        walk(&index.function, &mut parents, &mut |node, _| {
            if kind(node) == "CStyleCastExpr" && qual(node) != Some("void") {
                casts.push(node.clone());
            }
        });
        let [node] = casts.as_slice() else {
            panic!("{source}: {} casts", casts.len());
        };
        assert_eq!(node["castKind"].as_str(), Some(*cast), "{source}");
        let got = classify_view(node).map(|_| ());
        assert_eq!(code(got), *decision, "{source}");
    }
}

#[test]
fn only_a_bit_cast_from_void_to_an_object_pointer_is_a_view() {
    for cast in [CastClass::BitCast, CastClass::NoOp, CastClass::Other] {
        for operand in [
            OperandClass::VoidPointer,
            OperandClass::ObjectPointer,
            OperandClass::Other,
        ] {
            for target in [
                TargetClass::Object,
                TargetClass::VoidPointer,
                TargetClass::Function,
                TargetClass::Other,
            ] {
                assert_eq!(
                    view_of(cast, operand, target).is_ok(),
                    cast == CastClass::BitCast
                        && operand == OperandClass::VoidPointer
                        && target == TargetClass::Object,
                    "{cast:?} {operand:?} {target:?}"
                );
            }
        }
    }
}

#[test]
fn every_position_vector_is_the_position_decision() {
    assert_eq!(POSITION_VECTORS.len(), 8);
    for (spelling, position, never_bound, tracked, admitted) in POSITION_VECTORS {
        let position = match *position {
            "root" => ViewPosition::Root,
            "load" => ViewPosition::Load,
            other => panic!("unknown position {other}"),
        };
        assert_eq!(
            view_position(position, *never_bound, *tracked),
            *admitted,
            "{spelling}"
        );
    }
}

#[test]
fn every_decision_vector_is_the_readback_decision() {
    assert_eq!(DECISION_VECTORS.len(), 7);
    for (body, store_base, types_agree, owner_escapes, member_escapes, decision) in DECISION_VECTORS
    {
        assert_eq!(
            decide_readback(*store_base, *types_agree, *owner_escapes, *member_escapes).code(),
            *decision,
            "{body}"
        );
    }
}

#[test]
fn only_every_premise_gives_an_identity_and_a_pun_is_refused_on_a_store_base() {
    for store_base in [false, true] {
        for types_agree in [false, true] {
            for owner in [false, true] {
                for member in [false, true] {
                    let decision = decide_readback(store_base, types_agree, owner, member);
                    assert_eq!(
                        decision == ReadbackDecision::Identity,
                        store_base && types_agree && !owner && !member
                    );
                    assert_eq!(
                        decision == ReadbackDecision::Pun,
                        store_base && !types_agree
                    );
                    for resolves in [false, true] {
                        assert_eq!(
                            flow_binds(decision, resolves),
                            decision == ReadbackDecision::Identity && resolves
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn the_event_vectors_are_the_lean_rows() {
    // Their outcomes are run through the gateway by
    // `tests/cg12_void_view_readback.rs`; the Lean rows are the same list.
    let outcomes = EVENT_VECTORS
        .iter()
        .map(|(_, yields)| *yields)
        .collect::<Vec<_>>();
    assert_eq!(outcomes, [true, true, false, false, false, false]);
}

#[test]
fn pointee_keys_drop_qualifiers_and_refuse_void() {
    assert_eq!(pointee_key("struct T *").as_deref(), Some("struct T"));
    assert_eq!(pointee_key("const struct T *").as_deref(), Some("struct T"));
    assert_eq!(pointee_key("const char **").as_deref(), Some("char *"));
    assert_eq!(pointee_key("char *const").as_deref(), Some("char"));
    assert_eq!(pointee_key("void *"), None);
    assert_eq!(pointee_key("const void *"), None);
    assert_eq!(pointee_key("int"), None);
    assert_eq!(pointee_key("volatile int *"), None);
    assert_eq!(operand_class("void *restrict"), OperandClass::VoidPointer);
    assert_eq!(operand_class("volatile void *"), OperandClass::Other);
    assert_eq!(target_class("int (*)(int)"), TargetClass::Function);
}

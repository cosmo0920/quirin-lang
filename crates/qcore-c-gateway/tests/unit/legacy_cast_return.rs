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

//! Card W44-LEAK-PIN-AUDIT: the shape reader of the cast-return rule. The
//! carrier decisions over real declarations are exercised end to end in
//! `tests/leak_pin_audit.rs`; here, with no declaration read and no descriptor
//! name, only the shape and the pointer-target delegation are decidable.

use super::*;

fn decide(expression: &str) -> CastReturn {
    let texts = expression.split_whitespace().collect::<Vec<_>>();
    classify(
        &texts,
        &descriptor_custody::DescriptorNames::default(),
        &FunctionTypes::default(),
    )
}

#[test]
fn a_pointer_cast_of_one_place_is_decided_as_that_place() {
    assert_eq!(decide("( char * ) buffer"), CastReturn::Operand(4));
    assert_eq!(decide("( const char * * ) arr"), CastReturn::Operand(6));
    assert_eq!(
        decide("( struct k_box * ) ctx - > item"),
        CastReturn::Operand(5)
    );
    assert_eq!(decide("( void * ) ( p )"), CastReturn::Operand(4));
    // A chain of pointer casts is one pointer conversion.
    assert_eq!(
        decide("( struct k_box * ) ( void * ) p"),
        CastReturn::Operand(9)
    );
}

#[test]
fn anything_but_one_cast_of_one_place_is_not_this_rule() {
    for expression in [
        "p",
        "( char * ) p + 1",
        "( char * ) f ( p )",
        "( char * ) * out",
        "( char * ) NULL",
        "( a ) - b",
        "( ) p",
        "( 1 + 2 ) p",
        "( char * ) ( long ) p",
    ] {
        assert_eq!(decide(expression), CastReturn::PreCard, "{expression}");
    }
}

#[test]
fn an_uncommitted_operand_under_a_non_pointer_target_keeps_its_answer() {
    // With no declaration read the reader commits to nothing, so a non-pointer
    // cast keeps the pre-card answer (it escapes nothing).
    assert_eq!(decide("( long ) p"), CastReturn::PreCard);
    assert_eq!(decide("( int ) p"), CastReturn::PreCard);
}

#[test]
fn a_refusal_is_a_node_before_a_placeless_exit() {
    let Statement::Return {
        returned_place,
        returned_evaluation: Some(evaluation),
        returned_evaluation_escapes,
        ..
    } = refusal(7, RefusedWrites::Unbounded)
    else {
        panic!("a refusal answers with a return statement carrying its node");
    };
    assert_eq!(returned_place, None);
    assert!(!returned_evaluation_escapes);
    assert!(matches!(
        *evaluation,
        LegacyInstruction::Unknown { ref description, .. } if description == CAST_RETURN_REFUSAL
    ));
}

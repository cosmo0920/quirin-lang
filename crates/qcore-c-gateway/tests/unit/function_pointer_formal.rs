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

use super::{DECISION_VECTORS, Group, admits, called_in, function_pointer_name, read};

fn words(text: &str) -> Vec<&str> {
    text.split_whitespace().collect()
}

#[test]
fn rule_74b_decision_vectors_hold() {
    for (groups, expected) in DECISION_VECTORS {
        assert_eq!(admits(groups), *expected, "{groups:?}");
    }
}

/// `QCore.FunctionPointerFormal.decisionVectors`, entry for entry.
#[test]
fn rule_74b_lean_vectors_match_the_rust_table() {
    let lean: &[(&[Group], bool)] = &[
        (&[Group::Plain, Group::FunctionPointer], true),
        (&[Group::FunctionPointer], true),
        (&[Group::Plain, Group::Plain], true),
        (
            &[Group::FunctionPointer, Group::CalledFunctionPointer],
            false,
        ),
        (&[Group::Plain, Group::Other], false),
        (&[Group::Other], false),
        (&[Group::CalledFunctionPointer], false),
        (&[], true),
    ];
    assert_eq!(lean, DECISION_VECTORS);
}

#[test]
fn only_the_exact_declarator_names_a_function_pointer() {
    for (group, expected) in [
        ("void ( * free_cb ) ( void * )", Some("free_cb")),
        (
            "int ( * cmp ) ( const void * a , const void * b )",
            Some("cmp"),
        ),
        ("void * ( * make ) ( __SIZE_TYPE__ n )", Some("make")),
        ("int ( * log ) ( const char * fmt , ... )", Some("log")),
        ("void ( * f ) ( void )", Some("f")),
        ("int ( cmp ) ( const void * a )", None),
        ("void ( * log_cb )", None),
        ("void ( * const f ) ( void )", None),
        ("void ( * ( * f ) ( int ) ) ( void )", None),
        ("void ( * f ) ( void ) ( int )", None),
        ("( * f ) ( void )", None),
        ("STACK_OF ( X509 ) * chain", None),
        ("void ( * int ) ( void )", None),
        ("void ( * f ) ( struct s { int a ; } )", None),
    ] {
        assert_eq!(function_pointer_name(&words(group)), expected, "{group}");
    }
}

#[test]
fn a_call_through_the_formal_is_seen_in_either_spelling() {
    assert!(called_in(&words("return cb ( x ) ;"), "cb"));
    assert!(called_in(&words("( * cb ) ( x ) ;"), "cb"));
    assert!(!called_in(&words("s -> cb = cb ;"), "cb"));
    assert!(!called_in(&words("if ( cb ) g ( cb ) ;"), "cb"));
}

#[test]
fn the_list_keeps_every_position() {
    assert_eq!(
        read(
            &words("struct buf * b , void ( * free_cb ) ( void * , int ) , void * opaque"),
            &words("b -> free_cb = free_cb ; b -> opaque = opaque ;"),
        ),
        Some(vec![
            Some("b".to_owned()),
            Some("free_cb".to_owned()),
            Some("opaque".to_owned())
        ])
    );
    assert_eq!(
        read(
            &words("int ( * cmp ) ( const void * , const void * ) , int"),
            &[]
        ),
        Some(vec![Some("cmp".to_owned()), None])
    );
}

#[test]
fn every_other_list_stays_refused() {
    let body = words("return 0 ;");
    for list in [
        // A call through the formal (either spelling).
        "void * p , void ( * free_cb ) ( void * )",
        // Another parenthesised shape.
        "int ( cmp ) ( const void * a ) , int n",
        "void ( * log_cb ) , int n",
        // A variadic list.
        "void ( * f ) ( void ) , ...",
        // A repeated name.
        "void ( * f ) ( void ) , int f",
        // `void` beside a formal.
        "void ( * f ) ( void ) , void",
        // No parenthesised group: the old reader's, not this one's.
        "int a , int b",
    ] {
        let body = if list.starts_with("void * p") {
            words("free_cb ( p ) ;")
        } else {
            body.clone()
        };
        assert_eq!(read(&words(list), &body), None, "{list}");
    }
    assert_eq!(
        read(&words("void ( * f ) ( void )"), &words("( * f ) ( ) ;")),
        None
    );
}

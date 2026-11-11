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

//! Card W45-RETURN-EXPRESSION-FAMILIES: the shape readers of the two return
//! families, over a fixed name oracle. The decisions over real declarations
//! and the flow obligations are exercised end to end in
//! `tests/return_expression_families.rs`.

use super::*;

/// `p`, `q`, `n` and `fd` are this body's names: `p` and the type names are
/// untracked, `n` is a declared scalar, `q` may be bound.
struct Names;

impl NameFacts for Names {
    fn untracked(&self, name: &str) -> bool {
        !matches!(name, "q" | "n")
    }

    fn declared_scalar(&self, name: &str) -> bool {
        name == "n"
    }

    fn type_name(&self, words: &[&str]) -> bool {
        matches!(
            words,
            ["int"] | ["char"] | ["long"] | ["size_t"] | ["struct", "s"] | ["const", "char"]
        )
    }
}

fn texts(expression: &str) -> Vec<&str> {
    expression.split_whitespace().collect()
}

fn read(expression: &str) -> Option<ScalarRead> {
    scalar_read(&texts(expression), &Names)
}

#[test]
fn a_read_over_untracked_and_scalar_names_is_admitted() {
    for expression in [
        "p - > type & 0xFF",
        "( p - > type & 0xFF ) = = ( 1 < < 5 )",
        "p - > a . b",
        "p - > l1 - > l2 - > len > 0",
        "p ? p - > code : 0",
        "p & & p - > fatal ? 1 : 0",
        "* p = = __qcore_literal__ | | * p = = __qcore_literal__",
        "( ( const char * ) p ) - > u16",
        "zeros [ 0 ] [ n & 0xff ] ^ zeros [ 1 ] [ ( n > > 8 ) & 0xff ]",
        "( int ) p - > total",
        "sizeof ( struct s )",
        "( int ) sizeof ( size_t )",
        "n < < ( ( long ) p - > shift )",
        "( int ) ( n + 1 )",
    ] {
        assert!(read(expression).is_some(), "{expression}");
    }
}

#[test]
fn a_directly_dereferenced_untracked_root_is_a_base() {
    let bases = |expression: &str| read(expression).map(|read| read.bases);
    assert_eq!(
        bases("p - > l1 - > l2 - > len > 0"),
        Some(vec!["p".to_owned()])
    );
    assert_eq!(bases("p ? p - > code : 0"), Some(vec!["p".to_owned()]));
    assert_eq!(bases("* p = = 1"), Some(vec!["p".to_owned()]));
    assert_eq!(bases("zeros [ n ]"), Some(vec!["zeros".to_owned()]));
    // A name only compared, or inside a cast group, is not dereferenced here.
    assert_eq!(bases("p ! = 0"), Some(vec![]));
    assert_eq!(bases("( ( const char * ) p ) - > u16"), Some(vec![]));
    assert_eq!(bases("sizeof ( struct s ) + n"), Some(vec![]));
}

#[test]
fn every_arrow_root_carries_its_alias_obligation() {
    assert_eq!(
        read("p - > a + q - > b").map(|read| read.arrow_roots),
        Some(vec!["p".to_owned(), "q".to_owned()])
    );
}

#[test]
fn a_may_bind_name_is_a_base_of_one_unconditional_dereference_only() {
    for (expression, base) in [
        ("q - > len", "q"),
        ("q - > a . b + 1", "q"),
        ("q [ n ] = = 0", "q"),
        ("* q > 3", "q"),
        ("( * q ) . x", "q"),
        ("q - > len ? 1 : 0", "q"),
    ] {
        assert_eq!(
            read(expression).map(|read| read.bases),
            Some(vec![base.to_owned()]),
            "{expression}"
        );
    }
    for expression in [
        // As a value, not a dereference.
        "q = = p",
        "( long ) q",
        // Two dereferences.
        "q - > next - > len",
        "q - > buf [ 0 ]",
        "* q - > len",
        "q [ 0 ] - > len",
        // A position C may skip.
        "p ? q - > len : 0",
        "p & & q - > len",
        "n | | * q",
        // Inside a parenthesised operand.
        "( q ) - > len",
        // Card W45-SR5-SOUNDNESS: a second dereference through parentheses
        // (`SCALAR-READ-PARENTHESISED-DOUBLE-DEREF`).
        "( * q ) - > len",
        "( * q ) [ 0 ]",
        "* * q",
        "* ( * q )",
        "( q - > next ) - > len",
        "( q [ 0 ] ) - > len",
        "* ( q - > next )",
        "* ( q [ 0 ] )",
        // ... and a may-bind name inside a cast a `*` applies to
        // (`*(int32_t *)rl->rl_elems[idx]`, rd_list_get_int32).
        "* ( q * ) p",
    ] {
        assert_eq!(read(expression), None, "{expression}");
    }
}

#[test]
fn every_effect_and_address_refuses() {
    for expression in [
        "f ( p )",
        "p - > fn ( n )",
        "p . fn ( n )",
        "( * p ) ( n )",
        "p [ 0 ] ( n )",
        "__builtin_expect ( n , 1 )",
        "_Generic ( n , int : 1 )",
        "n = 1",
        "n + = 1",
        "n + +",
        "- - n",
        "( n , 0 )",
        "n ? : 1",
        "& p - > x = = 0",
        "( long ) & n",
        "( { n ; } )",
        "sizeof n",
        "sizeof ( p )",
        "sizeof ( int [ n ] )",
        "sizeof ( char * )",
        "",
    ] {
        assert_eq!(read(expression), None, "{expression}");
    }
}

#[test]
fn a_bitwise_and_is_not_an_address() {
    assert!(read("( n > > 8 ) & 0xff").is_some());
    assert!(read("p - > flags & ( 1 < < 0 )").is_some());
    // After a cast group, `&` is the address-of operator.
    assert_eq!(read("( long ) & p"), None);
}

#[test]
fn a_comparison_is_not_an_assignment() {
    for expression in ["n = = 0", "n ! = 0", "n < = 0", "n > = 0"] {
        assert!(read(expression).is_some(), "{expression}");
    }
    for expression in ["n < < = 1", "n > > = 1"] {
        assert_eq!(read(expression), None, "{expression}");
    }
}

#[test]
fn the_conditional_positions_follow_the_connectives_and_their_group() {
    let expression = texts("a & & b | | ( c ? d : e ) + f");
    let conditional = conditional_positions(&expression);
    let named = |name: &str| conditional[expression.iter().position(|t| *t == name).unwrap()];
    assert!(!named("a"));
    assert!(named("b"));
    assert!(named("c"));
    assert!(named("d"));
    assert!(named("f"));
    let expression = texts("( a ? b : c ) + d");
    let conditional = conditional_positions(&expression);
    let named = |name: &str| conditional[expression.iter().position(|t| *t == name).unwrap()];
    assert!(!named("a"));
    assert!(named("b"));
    assert!(named("c"));
    assert!(!named("d"));
}

#[test]
fn a_cast_of_a_whole_call_is_read_with_its_targets() {
    let shape = |expression: &str| cast_call(&texts(expression), &Names);
    assert_eq!(
        shape("( char * ) f ( p , 1 )"),
        Some(CastCall {
            pointer_targets: vec![true],
            call: (4, 10)
        })
    );
    assert_eq!(
        shape("( long ) ( int ) f ( )"),
        Some(CastCall {
            pointer_targets: vec![false, false],
            call: (6, 9)
        })
    );
    assert_eq!(
        shape("( ( char * ) ( f ( p ) ) )"),
        Some(CastCall {
            pointer_targets: vec![true],
            call: (6, 10)
        })
    );
    // Parentheses alone are this family too (the bare call is W4's).
    assert_eq!(
        shape("( f ( p ) )"),
        Some(CastCall {
            pointer_targets: vec![],
            call: (1, 5)
        })
    );
    for expression in [
        "f ( p )",
        // A parenthesised function designator is a call, not a cast.
        "( g ) ( f ( p ) )",
        "( char * ) f ( p ) + 1",
        "( char * ) p",
        "( int ) sizeof ( struct s )",
        "( char * ) ( f ( p ) , 0 )",
        "( ) f ( p )",
    ] {
        assert_eq!(shape(expression), None, "{expression}");
    }
}

#[test]
fn the_definition_result_is_its_prefix_without_a_pointer_level() {
    assert_eq!(
        definition_result(
            &texts("int f ( void ) ; static inline int f ( void ) { return 0 ; }"),
            "f"
        ),
        Some(vec!["static", "inline", "int"])
    );
    assert_eq!(
        definition_result(
            &texts(
                "; __attribute__ ( ( visibility ( \"default\" ) ) ) cJSON_bool f ( const cJSON * const item ) { }"
            ),
            "f"
        ),
        Some(vec!["cJSON_bool"])
    );
    assert_eq!(
        definition_result(&texts("char * f ( void ) { }"), "f"),
        None
    );
    assert_eq!(
        definition_result(&texts("int ( * f ( void ) ) ( int ) { }"), "f"),
        None
    );
    assert_eq!(
        definition_result(&texts("int f ( a ) int a ; { }"), "f"),
        None
    );
    assert_eq!(definition_result(&texts("int f ( void ) ;"), "f"), None);
    assert_eq!(definition_result(&texts("x = y f ( void ) { }"), "f"), None);
}

/// Card CAST-FORMALIZATION (rule 12h, `W45-SR5-SCALAR-READ-STAR-THROUGH-CAST`):
/// `QCore.CastSemantics.sr5Vectors`, each a `return` operand over the
/// may-bind `q` and its dereference count; rule 5j admits it exactly when the
/// count is one (`star_through_cast_is_two`, `sr5_vectors_hold`).
#[test]
fn a_star_through_a_cast_counts_as_a_dereference() {
    const LEAN: &str = include_str!("../../../../proof/QCore/CastSemantics.lean");
    let body = LEAN
        .split("def sr5Vectors : List (String × Expr × Nat) := [")
        .nth(1)
        .expect("sr5Vectors");
    let rows = body[..body.find("\n]").expect("end")]
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with('('))
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 6);
    for row in rows {
        let spelling = row.split('"').nth(1).expect("spelling");
        let count = row
            .trim_end_matches(',')
            .trim_end_matches(')')
            .rsplit(", ")
            .next()
            .and_then(|count| count.parse::<usize>().ok())
            .expect("count");
        assert_eq!(read(spelling).is_some(), count == 1, "{spelling}: {count}");
    }
}

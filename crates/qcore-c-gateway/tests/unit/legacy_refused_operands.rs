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

//! Card W44-LEAK-PIN-AUDIT: the hand-on reader of the refused hand-on rule.

use super::*;

fn places(statement: &str) -> Vec<(String, HandOn)> {
    let tokens = statement.split_whitespace().collect::<Vec<_>>();
    handed_on(&tokens).into_iter().collect()
}

fn tagged(statement: &str, how: HandOn) -> Vec<String> {
    places(statement)
        .into_iter()
        .filter(|(_, tag)| *tag == how)
        .map(|(place, _)| place)
        .collect()
}

#[test]
fn a_call_argument_is_handed_to_the_callee() {
    assert_eq!(
        tagged("destroy_metric ( metric )", HandOn::Call),
        ["metric"]
    );
    assert!(tagged("destroy_metric ( metric )", HandOn::Escape).is_empty());
}

#[test]
fn a_cast_value_stored_through_a_member_escapes() {
    // `map->parent = (void *) counter;`: the destination chain is written, the
    // cast operand is the stored value.
    assert_eq!(
        tagged("map - > parent = ( void * ) counter", HandOn::Escape),
        ["counter", "void"]
    );
    assert!(
        !places("map - > parent = ( void * ) counter")
            .iter()
            .any(|(place, _)| place == "map" || place == "map->parent")
    );
}

#[test]
fn a_returned_value_escapes() {
    assert_eq!(
        tagged("return ( const char * * ) arr", HandOn::Escape),
        ["arr", "char", "const"]
    );
    assert_eq!(tagged("return f ( p )", HandOn::Call), ["p"]);
    assert!(tagged("return f ( p )", HandOn::Escape).is_empty());
}

#[test]
fn an_address_inside_the_object_hands_on_every_prefix() {
    let passed = tagged(
        "cfl_list_add ( & counter - > _head , & cmt - > counters )",
        HandOn::Call,
    );
    for place in ["counter", "counter->_head", "cmt", "cmt->counters"] {
        assert!(passed.contains(&place.to_owned()), "{place} in {passed:?}");
    }
    assert_eq!(
        tagged("x - > h = & p - > head", HandOn::Escape),
        ["p", "p->head"]
    );
}

#[test]
fn a_member_value_read_does_not_hand_on_the_base() {
    // `ret = mk_net_fd_connect(conn->fd, addr, port);`: an integer member is
    // read through `conn`; `conn` itself is not handed on.
    let all = places("ret = mk_net_fd_connect ( conn - > fd , addr , port )");
    assert!(all.contains(&("conn->fd".to_owned(), HandOn::Call)));
    assert!(
        !all.iter()
            .any(|(place, _)| place == "conn" || place == "ret")
    );
    assert!(tagged("x = p - > buf", HandOn::Escape).contains(&"p->buf".to_owned()));
    assert!(
        !places("x = p - > buf")
            .iter()
            .any(|(place, _)| place == "p")
    );
    // A store into a member through the identity writes it and names nothing.
    assert!(places("p - > len = 5").is_empty());
}

#[test]
fn a_subscript_or_call_base_is_not_handed_on() {
    let all = places("x = p - > items [ i ] + q - > fn ( y )");
    assert!(
        !all.iter()
            .any(|(place, _)| place == "p->items" || place == "q->fn")
    );
    assert!(all.contains(&("i".to_owned(), HandOn::Escape)));
    assert!(all.contains(&("y".to_owned(), HandOn::Call)));
}

#[test]
fn a_dereferenced_name_hands_on_its_slot_place() {
    assert_eq!(tagged("x = * out", HandOn::Escape), ["*out"]);
    // A store through the slot writes it and stores the value.
    assert_eq!(places("* out = buf"), [("buf".to_owned(), HandOn::Escape)]);
}

#[test]
fn an_operand_outside_calls_and_values_is_an_operand() {
    assert_eq!(
        places("if ( ! p ) x ( )"),
        [("p".to_owned(), HandOn::Operand)]
    );
    assert_eq!(tagged("n + p", HandOn::Operand), ["n", "p"]);
    assert!(
        places("p - > len ++")
            .iter()
            .all(|(place, _)| place == "p->len")
    );
}

#[test]
fn a_local_aggregate_store_escapes_like_any_store() {
    assert_eq!(tagged("local . slot = h", HandOn::Escape), ["h"]);
}

#[test]
fn an_unevaluated_operand_hands_on_nothing() {
    assert!(places("n = sizeof ( * p )").is_empty());
}

#[test]
fn binary_operators_are_not_unary() {
    assert_eq!(tagged("n = a * b & c", HandOn::Escape), ["a", "b", "c"]);
}

#[test]
fn line_reads_key_every_line_a_statement_spans() {
    // `x = f(a,` on line 10, `b) + c;` on line 11, `y = d;` on line 12.
    let texts = [
        "x", "=", "f", "(", "a", ",", "b", ")", "+", "c", ";", "y", "=", "d", ";",
    ];
    let lines = [10, 10, 10, 10, 10, 10, 11, 11, 11, 11, 11, 12, 12, 12, 12];
    let reads = line_reads(&texts, &lines, &BTreeSet::new());
    assert_eq!(reads[&10].statement.get("c"), Some(&HandOn::Escape));
    assert_eq!(reads[&11].statement.get("a"), Some(&HandOn::Call));
    assert!(!reads[&12].statement.contains_key("a"));
    assert_eq!(reads[&12].statement.get("d"), Some(&HandOn::Escape));
    assert!(reads[&10].condition.is_empty());
}

#[test]
fn a_condition_and_the_statements_of_one_macro_line_are_kept_apart() {
    // `RD_UT_ASSERT(strlen(part) > 0, "…")` expands on one line to
    // `if (!(strlen(part) > 0)) { fprintf(stderr, "…"); return 1; }`.
    let texts = [
        "if", "(", "!", "(", "strlen", "(", "part", ")", ">", "0", ")", ")", "{", "fprintf", "(",
        "stderr", ",", "\"…\"", ")", ";", "return", "1", ";", "}",
    ];
    let lines = [7; 24];
    let reads = line_reads(&texts, &lines, &BTreeSet::new());
    assert_eq!(reads[&7].condition.get("part"), Some(&HandOn::Call));
    assert!(!reads[&7].statement.contains_key("part"));
    assert_eq!(reads[&7].statement.get("stderr"), Some(&HandOn::Call));
}

#[test]
fn a_braceless_body_is_a_statement_not_its_condition() {
    let texts = ["if", "(", "x", ")", "destroy", "(", "p", ")", ";"];
    let reads = line_reads(&texts, &[3; 9], &BTreeSet::new());
    assert_eq!(reads[&3].statement.get("p"), Some(&HandOn::Call));
    assert!(!reads[&3].condition.contains_key("p"));
    assert_eq!(reads[&3].condition.get("x"), Some(&HandOn::Operand));
}

#[test]
fn a_for_header_goes_to_both_tables() {
    let texts = [
        "for", "(", "i", "=", "0", ";", "i", "<", "n", ";", "i", "++", ")", "{", "}",
    ];
    let reads = line_reads(&texts, &[4; 15], &BTreeSet::new());
    assert!(reads[&4].condition.contains_key("n"));
    assert!(reads[&4].statement.contains_key("n"));
}

// -- Card CAST-BEFORE-AMPERSAND-READ-AS-BINARY (rule 74f) --------------------

fn among(statement: &str, objects: &[&str]) -> Vec<String> {
    let tokens = statement.split_whitespace().collect::<Vec<_>>();
    let objects = objects.iter().map(|name| (*name).to_owned()).collect();
    handed_on_among(&tokens, &objects).into_keys().collect()
}

/// The ledger shape: `(T *) &x->f` hands on the address chain, `x` and every
/// prefix, not the value of `x->f` alone. FAILS at the base, which read the
/// `&` after the `)` as a binary AND and handed on `x->f` only.
#[test]
fn a_cast_before_an_address_hands_on_the_chain() {
    for statement in [
        "k_link ( ( T * ) & x - > f )",
        "y = ( T * ) & x - > f",
        "return ( T * ) & x - > f",
    ] {
        let handed = among(statement, &[]);
        for place in ["x", "x->f"] {
            assert!(
                handed.contains(&place.to_owned()),
                "{place} in {handed:?}: {statement}"
            );
        }
    }
}

/// `(struct s *) &v.f`, `(void) &x->f`, `(const char **) &x->f` and the
/// nested type names: every scalar or `void` cast is read as a cast. FAILS
/// at the base.
#[test]
fn every_cast_spelling_hands_on_the_chain() {
    for (statement, root) in [
        ("y = ( struct s * ) & v . f", "v"),
        ("( void ) & x - > f", "x"),
        ("y = ( const char * * ) & x - > f", "x"),
        ("y = ( void ( * ) ( int , . . . ) ) & x - > f", "x"),
        ("y = ( char ( * ) [ 4 ] ) & x - > f", "x"),
    ] {
        assert!(
            among(statement, &[]).contains(&root.to_owned()),
            "{statement}"
        );
    }
    // A plain `&v` hands on `v` under either reading.
    assert!(among("y = ( struct s * ) & v", &[]).contains(&"v".to_owned()));
}

/// A typedef-name cast `(my_t *) &x->f` and `(my_t) &x->f`: a name the
/// function does not declare as an object may be a type, so the group is a
/// cast. FAILS at the base for both.
#[test]
fn a_typedef_name_cast_hands_on_the_chain() {
    for statement in ["y = ( my_t * ) & x - > f", "y = ( my_t ) & x - > f"] {
        assert!(
            among(statement, &["x"]).contains(&"x".to_owned()),
            "{statement}"
        );
    }
}

/// `(a) & b` with `a` a declared object is a parenthesised value and a binary
/// AND: it hands on `a` and the value of `b`'s chain, not `b`. Undecided
/// (`a` not known as an object), it is read as a cast.
#[test]
fn a_parenthesised_object_before_an_ampersand_is_binary() {
    assert_eq!(among("y = ( a ) & b", &["a", "b"]), ["a", "b"]);
    assert_eq!(among("y = ( a ) & b - > f", &["a", "b"]), ["a", "b->f"]);
    assert_eq!(among("y = ( a ) & b - > f", &["b"]), ["a", "b", "b->f"]);
}

/// A call's result before `&` is a value: `f(x) & mask` is binary, and so is
/// a call through an expression or a group holding an operator.
#[test]
fn a_call_result_or_an_expression_before_an_ampersand_is_binary() {
    assert_eq!(among("y = f ( x ) & mask", &[]), ["mask", "x"]);
    assert_eq!(among("y = f ( x ) & m - > g", &[]), ["m->g", "x"]);
    assert_eq!(
        among("y = ( * fp ) ( x ) & m - > g", &[]),
        ["*fp", "m->g", "x"]
    );
    assert_eq!(among("y = ( a + 1 ) & m - > g", &[]), ["a", "m->g"]);
    assert_eq!(among("y = ( s . n ) & m - > g", &[]), ["m->g", "s.n"]);
}

/// The objects the publication facts pass: formals, locals and file-scope
/// objects, less every name a typedef spells.
#[test]
fn the_value_names_exclude_every_typedef_spelling() {
    let unit = "typedef struct s my_t ; typedef int a ; int g ; int a ;"
        .split(' ')
        .collect::<Vec<_>>();
    let declared = BTreeSet::from(["n".to_owned()]);
    let file = crate::publication::file_scope_objects(&unit);
    let formals = BTreeSet::from(["p".to_owned()]);
    let names = crate::publication::value_names(&unit, &declared, &file, &formals);
    // `a` is both an object and typedef-spelled, so it stays undecided.
    assert_eq!(names.into_iter().collect::<Vec<_>>(), ["g", "n", "p"]);
}

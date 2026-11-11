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

//! `CTWIN-PUBLISH`: the destination scan, over token vectors alone.
//!
//! These tests drive the two readers directly, so they measure what the scan
//! recognises rather than what the analyzer around it does with the answer.
//! The Clang-backed acceptance family lives in
//! `crates/qcore-c-gateway/tests/publication_custody.rs`.

use super::*;

fn tokens(text: &str) -> Vec<&str> {
    text.split_whitespace().collect()
}

fn scope(text: &str) -> Vec<String> {
    file_scope_objects(&tokens(text)).into_iter().collect()
}

fn body(text: &str) -> (Vec<String>, Vec<String>) {
    let (declared, static_duration) = body_declarations(&tokens(text));
    (
        declared.into_iter().collect(),
        static_duration.into_iter().collect(),
    )
}

#[test]
fn a_plain_file_scope_pointer_is_recognised() {
    assert_eq!(scope("struct h * g_published ;"), ["g_published"]);
    assert_eq!(scope("static struct h * g_published ;"), ["g_published"]);
    assert_eq!(scope("extern int g_count ;"), ["g_count"]);
}

#[test]
fn every_declarator_of_a_declaration_is_recognised() {
    assert_eq!(
        scope("struct h * g_a , * g_b , g_c ;"),
        ["g_a", "g_b", "g_c"]
    );
}

#[test]
fn an_array_declarator_names_its_object_not_its_extent() {
    assert_eq!(scope("struct h * g_slots [ 4 ] ;"), ["g_slots"]);
    assert_eq!(scope("int g_grid [ 2 ] [ 3 ] ;"), ["g_grid"]);
}

#[test]
fn an_initializer_is_dropped_before_the_declarator_is_read() {
    assert_eq!(scope("int g_x = 1 ;"), ["g_x"]);
    assert_eq!(scope("int g_t [ 2 ] = { 1 , 2 } ;"), ["g_t"]);
    // The declaration *after* a brace initializer must still be read: the run
    // scanner must not treat that closing brace as the end of a definition.
    assert_eq!(
        scope("int g_t [ ] = { 1 , 2 } ; struct h * g_p ;"),
        ["g_p", "g_t"]
    );
}

#[test]
fn a_tag_a_record_definition_and_an_enumerator_list_declare_no_object() {
    assert_eq!(scope("struct h ;"), Vec::<String>::new());
    assert_eq!(scope("struct r { int x ; } ;"), Vec::<String>::new());
    assert_eq!(scope("enum e { A , B } ;"), Vec::<String>::new());
    assert_eq!(scope("typedef struct r r_t ;"), Vec::<String>::new());
    // A record definition that *does* declare an object still names it.
    assert_eq!(scope("struct r { int x ; } g_r ;"), ["g_r"]);
    assert_eq!(scope("enum e { A , B } g_e ;"), ["g_e"]);
}

#[test]
fn a_prototype_declares_no_object_and_no_parameter_name() {
    assert_eq!(
        scope("void b_close ( struct h * h ) ;"),
        Vec::<String>::new()
    );
    assert_eq!(
        scope("struct h * b_open ( struct c * cio , int flags ) ;"),
        Vec::<String>::new()
    );
    // A function-pointer object is a `(`-carrying declarator: not recognised,
    // so it keeps its pre-existing modelling (`CTWIN-PUBLISH-SCAN`).
    assert_eq!(scope("void ( * g_hook ) ( int ) ;"), Vec::<String>::new());
}

#[test]
fn a_function_definition_contributes_nothing_and_leaves_the_scan_synchronised() {
    assert_eq!(
        scope("int helper ( int a ) { int local ; return a ; } struct h * g_p ;"),
        ["g_p"]
    );
    // Two definitions in a row, and a declaration between them.
    assert_eq!(
        scope("int f ( void ) { return 0 ; } int g_x ; int g ( void ) { return 1 ; } int g_y ;"),
        ["g_x", "g_y"]
    );
}

#[test]
fn a_body_declaration_is_recorded_and_an_assignment_is_not() {
    let (declared, static_duration) = body("struct h * l ; l = p ; g_published = l ;");
    assert_eq!(declared, ["l"]);
    assert!(static_duration.is_empty());
}

#[test]
fn a_static_local_is_recorded_as_static_duration() {
    let (declared, static_duration) = body("static struct h * cache ; int n ;");
    assert_eq!(declared, ["cache", "n"]);
    assert_eq!(static_duration, ["cache"]);
    let (_, thread_local) = body("_Thread_local int slot ;");
    assert_eq!(thread_local, ["slot"]);
}

#[test]
fn the_destination_set_excludes_formals_and_body_declarations() {
    let unit = tokens("struct h * g_p ; struct h * g_q ; void w ( struct h * g_q ) { }");
    let body = tokens("struct h * g_p ; g_p = h ;");
    let formals = BTreeSet::from(["g_q".to_owned()]);
    // `g_p` is shadowed by the body's own declaration and `g_q` by the formal,
    // so neither is a publication destination.
    assert!(destinations(&unit, &body, &formals).is_empty());
}

#[test]
fn the_destination_set_keeps_an_unshadowed_file_scope_object() {
    let unit = tokens("struct h * g_p ; void w ( int n ) { }");
    let body = tokens("struct h * l ; l = h ;");
    let names = destinations(&unit, &body, &BTreeSet::new());
    assert_eq!(names.into_iter().collect::<Vec<_>>(), ["g_p"]);
}

#[test]
fn a_static_local_is_a_destination_even_though_the_body_declares_it() {
    let unit = tokens("void w ( int n ) { }");
    let body = tokens("static struct h * cache ; cache = h ;");
    let names = destinations(&unit, &body, &BTreeSet::new());
    assert_eq!(names.into_iter().collect::<Vec<_>>(), ["cache"]);
}

#[test]
fn a_qualifier_does_not_change_the_recognised_name() {
    // `CT-CTWIN-SYNC`: the scan reads declarators, not qualifiers, so a
    // `volatile` or `_Atomic` spelling names the same object.
    for text in [
        "struct h * g_p ;",
        "volatile struct h * g_p ;",
        "struct h * volatile g_p ;",
        "_Atomic ( struct h * ) g_p ;",
    ] {
        assert!(
            scope(text).contains(&"g_p".to_owned()) || text.contains('('),
            "{text}"
        );
    }
    assert_eq!(scope("volatile struct h * g_p ;"), ["g_p"]);
    assert_eq!(scope("struct h * volatile g_p ;"), ["g_p"]);
}

// ---------------------------------------------------------------------------
// Card `CTWIN-RESIDUALS`: the parenthesised specifier, and the indirect reader
// ---------------------------------------------------------------------------

fn indirect(text: &str, destinations: &[&str]) -> Vec<(usize, String, Vec<String>)> {
    let tokens = tokens(text);
    let lines = vec![1usize; tokens.len()];
    let destinations = destinations
        .iter()
        .map(|name| (*name).to_owned())
        .collect::<BTreeSet<_>>();
    indirect_publications(&tokens, &lines, &destinations)
        .into_iter()
        .map(|(line, publication)| {
            (
                line,
                publication.place,
                publication.sources.into_iter().collect(),
            )
        })
        .collect()
}

/// `_Atomic ( type-name )` is a specifier, so the declarator after it is read
/// exactly as the postfix spelling's is. Measured `Safe` with no diagnostic
/// before this card, because the `(`-carrying skip swallowed the whole group.
#[test]
fn a_parenthesised_atomic_specifier_is_dropped_before_the_declarator_is_read() {
    assert_eq!(scope("_Atomic ( void * ) g_published ;"), ["g_published"]);
    assert_eq!(scope("void * _Atomic g_published ;"), ["g_published"]);
    assert_eq!(
        scope("_Atomic ( struct h * ) g_a , * g_b ;"),
        ["g_a", "g_b"]
    );
    assert_eq!(scope("static _Atomic ( int ) g_count ;"), ["g_count"]);
    // A nested parenthesis inside the type name is matched, not counted.
    assert_eq!(scope("_Atomic ( void ( * ) ( int ) ) g_hook ;"), ["g_hook"]);
}

/// The drop must not reach a declarator's own parenthesis: a prototype, a
/// function definition and a function-pointer object still contribute nothing,
/// so no parameter name can enter the set through this door.
#[test]
fn dropping_a_specifier_never_reveals_a_function_declarator() {
    assert_eq!(scope("void f ( struct h * h ) ;"), Vec::<String>::new());
    assert_eq!(
        scope("_Atomic ( int ) f ( struct h * h ) ;"),
        Vec::<String>::new()
    );
    assert_eq!(scope("void ( * g_hook ) ( int ) ;"), Vec::<String>::new());
}

/// Monotonicity, stated as a comparison rather than as prose: for every
/// declaration whose `_Atomic(…)` prefix is simply deleted, the reader's answer
/// on the two spellings is the same — so the extension only ever *adds* names,
/// which only ever adds refusals.
#[test]
fn the_specifier_drop_only_adds_destinations() {
    for (parenthesised, plain) in [
        ("_Atomic ( void * ) g ;", "void * g ;"),
        ("static _Atomic ( int ) g ;", "static int g ;"),
        ("_Atomic ( struct h * ) g [ 4 ] ;", "struct h * g [ 4 ] ;"),
        ("_Atomic ( int ) g = 1 ;", "int g = 1 ;"),
    ] {
        assert_eq!(
            scope(parenthesised),
            scope(plain),
            "spelling must not change the answer: {parenthesised}"
        );
    }
}

/// The continued-access reader recognises exactly the three shapes whose base
/// is a destination, and nothing else.
///
/// Every punctuator is one token in this gateway's lexer, so `->`, `+=` and
/// `==` are spelled as two adjacent tokens here exactly as the analyzer sees
/// them.
#[test]
fn the_indirect_reader_recognises_a_continued_access_on_a_destination() {
    assert_eq!(
        indirect("g . slot = h ;", &["g"]),
        [(1, "g".to_owned(), vec!["h".to_owned()])]
    );
    assert_eq!(
        indirect("g - > slot = h ;", &["g"]),
        [(1, "g".to_owned(), vec!["h".to_owned()])]
    );
    assert_eq!(
        indirect("g [ 0 ] = h ;", &["g"]),
        [(1, "g".to_owned(), vec!["h".to_owned()])]
    );
}

/// Everything the reader must **not** recognise: a bare name (which the
/// flow-sensitive rule decides), a local base, a compound assignment, a
/// comparison, and a destination that only appears on the stored side.
#[test]
fn the_indirect_reader_refuses_every_other_shape() {
    for (text, destinations) in [
        ("g = h ;", &["g"][..]),
        ("local . slot = h ;", &["g"][..]),
        ("g . slot + = h ;", &["g"][..]),
        ("g . slot = = h ;", &["g"][..]),
        ("h = g . slot ;", &["g"][..]),
        ("g . slot ;", &["g"][..]),
    ] {
        assert!(
            indirect(text, destinations).is_empty(),
            "must not be read as a publication: {text}"
        );
    }
    // With no destination at all the reader answers nothing, whatever it sees.
    assert!(indirect("g . slot = h ;", &[]).is_empty());
}

/// A statement inside a block is still a statement: the reader restarts at
/// every brace, so a store in a guarded arm is not swallowed by the `if`
/// header in front of it.
#[test]
fn a_store_inside_a_block_is_read() {
    assert_eq!(
        indirect("if ( ! h ) { return ; } g . slot = h ;", &["g"]),
        [(1, "g".to_owned(), vec!["h".to_owned()])]
    );
    assert_eq!(
        indirect("if ( n ) { g . slot = h ; }", &["g"]),
        [(1, "g".to_owned(), vec!["h".to_owned()])]
    );
    // A `for` header's semicolons are inside parentheses and end nothing.
    assert_eq!(
        indirect("for ( i = 0 ; i < n ; i + + ) { g [ i ] = h ; }", &["g"]),
        [(1, "g".to_owned(), vec!["h".to_owned()])]
    );
}

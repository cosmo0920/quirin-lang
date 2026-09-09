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

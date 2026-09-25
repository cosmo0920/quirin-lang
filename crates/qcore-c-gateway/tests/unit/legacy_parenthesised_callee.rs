use super::super::{Token, tokenize, tokenize_with_conditionals};
use super::{TypeNames, normalise_owned, normalise_tokens, opaque_call};
use crate::typed_field_access::TypedAccessModel;

fn texts(tokens: &[Token]) -> Vec<String> {
    tokens.iter().map(|token| token.text.clone()).collect()
}

/// The body of `f` after rule 1, as one string of tokens.
fn normalised(source: &str) -> String {
    let tokens = tokenize(source);
    let tokens = normalise_tokens(&tokens).unwrap_or(tokens);
    texts(&tokens).join(" ")
}

/// Rule 2 over `statement`, with the type names `unit` declares and the
/// statement itself does not shadow.
fn opaque(unit: &str, statement: &str) -> bool {
    let tokens = tokenize(unit);
    let all = texts(&tokens);
    let all = all.iter().map(String::as_str).collect::<Vec<_>>();
    let model = TypedAccessModel::read(&all);
    let statement = texts(&tokenize(statement));
    let statement = statement.iter().map(String::as_str).collect::<Vec<_>>();
    let types = TypeNames::read(&model, &statement, 0..statement.len());
    opaque_call(&statement, &types)
}

#[test]
fn a_parenthesised_name_callee_is_the_name() {
    assert_eq!(
        normalised("void f(void *p) { (q_free)(p); }"),
        "void f ( void * p ) { q_free ( p ) ; }"
    );
    assert_eq!(
        normalised("void f(void *p) { ((q_free))(p); }"),
        "void f ( void * p ) { q_free ( p ) ; }"
    );
    assert_eq!(
        normalised("void f(void *p) { (void)(q_free)(p); }"),
        "void f ( void * p ) { ( void ) q_free ( p ) ; }"
    );
    assert_eq!(
        normalised("int f(void *p) { return (g)(p); }"),
        "int f ( void * p ) { return g ( p ) ; }"
    );
    assert_eq!(
        normalised("void f(void *p) { if (c) (q_free)(p); }"),
        "void f ( void * p ) { if ( c ) q_free ( p ) ; }"
    );
    assert_eq!(
        normalised("void f(void *p) { h(((g))(p)); }"),
        "void f ( void * p ) { h ( g ( p ) ) ; }"
    );
}

#[test]
fn an_argument_list_a_cast_and_a_keyword_are_left_alone() {
    for source in [
        // `g(f)` is called with `p`.
        "void f(void *p) { g(f)(p); }",
        "void f(void *p) { g((f))(p); }",
        // A cast of a parenthesised operand.
        "void f(int n) { (void)(n); (long)(n); }",
        "typedef long word_t; void f(int n) { (word_t)(n); }",
        // `sizeof` is not followed by an expression the rule rewrites.
        "void f(int n) { n = sizeof (n); }",
        // A declarator.
        "void f(void) { int (x)(int); }",
    ] {
        let tokens = tokenize(source);
        assert!(normalise_tokens(&tokens).is_none(), "{source}");
    }
}

#[test]
fn file_scope_declarations_are_left_alone() {
    let source = "extern int (q_free)(void *); void f(void *p) { (q_free)(p); }";
    assert_eq!(
        normalised(source),
        "extern int ( q_free ) ( void * ) ; void f ( void * p ) { q_free ( p ) ; }"
    );
}

#[test]
fn a_shadowed_typedef_name_is_rewritten_as_a_callee() {
    assert_eq!(
        normalised("typedef long word_t; void f(rel_t word_t) { (word_t)(p); }"),
        "typedef long word_t ; void f ( rel_t word_t ) { word_t ( p ) ; }"
    );
}

#[test]
fn conditional_marks_move_with_their_tokens() {
    let source = "void f(void *p) {\n(q_free)(p);\n#if A\n(q_free)(p);\n#endif\n}\n";
    let (tokens, conditionals) = tokenize_with_conditionals(source);
    let before = conditionals
        .iter()
        .map(|mark| tokens[mark.token_index].text.clone())
        .collect::<Vec<_>>();
    let (after_tokens, after_marks) = normalise_owned(tokens.clone(), conditionals);
    assert!(after_tokens.len() < tokens.len());
    let after = after_marks
        .iter()
        .map(|mark| {
            after_tokens
                .get(mark.token_index)
                .map_or_else(String::new, |token| token.text.clone())
        })
        .collect::<Vec<_>>();
    // A mark before a removed `(` now precedes the callee it opened.
    assert_eq!(before.len(), after.len());
    assert!(after.iter().all(|text| text == "q_free" || text == "}"), "{after:?}");
}

#[test]
fn every_other_callee_expression_is_opaque() {
    let unit = "typedef void (*rel_t)(void *); typedef long word_t;";
    for statement in [
        "(c ? f : g)(p);",
        "((rel_t) f)(p);",
        "(&f)(p);",
        "(c, f)(p);",
        "g()(p);",
        "a[0](p);",
        "(*fp)(p);",
        "x = (c ? f : g)(p);",
        "int v = (&f)(p);",
        "int a[(&f)(p)];",
        "(void)(c ? f : g)(p);",
    ] {
        assert!(opaque(unit, statement), "{statement}");
    }
}

#[test]
fn a_cast_a_header_and_a_declarator_are_not_calls() {
    let unit = "typedef void (*rel_t)(void *); typedef long word_t;";
    for statement in [
        "(void)(n);",
        "(long)(n);",
        "(word_t)(n);",
        "(const char *)(s);",
        "(struct s *)(p);",
        "f(p);",
        "void (*cb)(void *);",
        "int (*table[4])(int);",
        "static void *(*make)(void);",
        "word_t n = (word_t)(m);",
    ] {
        assert!(!opaque(unit, statement), "{statement}");
    }
}

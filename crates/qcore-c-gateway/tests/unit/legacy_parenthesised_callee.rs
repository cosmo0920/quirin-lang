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

use super::super::{Token, tokenize, tokenize_with_conditionals};
use super::{
    TypeNames, normalise_owned, normalise_tokens, opaque_call, stripping_is_faithful,
    typedef_declarators,
};

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
    let statement = texts(&tokenize(statement));
    let statement = statement.iter().map(String::as_str).collect::<Vec<_>>();
    let declared = typedef_declarators(&all);
    let types = TypeNames::read_with(&declared, &statement, 0..statement.len());
    opaque_call(&statement, &types)
}

#[test]
fn typedef_declarators_are_read_by_position() {
    let source = "typedef void (*rel_t)(void *a, void *b); \
                  typedef long long v2 __attribute__ ((__vector_size__ (16))); \
                  typedef int T, *U, V[3]; typedef int fn_t(int x); \
                  typedef struct s { int member; } s_t; typedef struct tag tag_t; \
                  typedef int __attribute__((mode(DI))) wide_t;";
    let tokens = tokenize(source);
    let all = texts(&tokens);
    let all = all.iter().map(String::as_str).collect::<Vec<_>>();
    let names = typedef_declarators(&all);
    let expected = [
        "T", "U", "V", "fn_t", "rel_t", "s_t", "tag_t", "v2", "wide_t",
    ];
    assert_eq!(
        names.iter().map(String::as_str).collect::<Vec<_>>(),
        expected
    );
}

#[test]
fn a_for_clause_strips_faithfully_only_around_single_names() {
    let unit = "typedef long word_t;";
    let tokens = tokenize(unit);
    let all = texts(&tokens);
    let all = all.iter().map(String::as_str).collect::<Vec<_>>();
    let declared = typedef_declarators(&all);
    let faithful = |clause: &str| {
        let clause = texts(&tokenize(clause));
        let clause = clause.iter().map(String::as_str).collect::<Vec<_>>();
        let types = TypeNames::read_with(&declared, &clause, 0..clause.len());
        stripping_is_faithful(&clause, &types)
    };
    for clause in [
        "i ++",
        "(elm) = (tmpelm)",
        "((e)) = (f)",
        "i = *(p)",
        "n = 0",
    ] {
        assert!(faithful(clause), "{clause}");
    }
    for clause in [
        "q_free(p)",
        "p = q_alloc()",
        "rd_map_iter_next(&(elem))",
        "(e) = ((e)->next)",
        "a[i] = 0",
        "(f)(p)",
        "n = (word_t) m",
        "x = (a + b)",
    ] {
        assert!(!faithful(clause), "{clause}");
    }
}

#[test]
fn a_block_scope_typedef_is_not_a_type_elsewhere() {
    // `T` is a type only inside `g`; in `f` it is the function `T`.
    assert_eq!(
        normalised("void g(void) { typedef int T; } void f(void *p) { (T)(p); }"),
        "void g ( void ) { typedef int T ; } void f ( void * p ) { T ( p ) ; }"
    );
}

#[test]
fn a_function_pointer_parameter_does_not_shadow_its_result_type() {
    let source = "typedef long word_t; \
                  long f(word_t (*render)(void), int n) { return (word_t)(n); }";
    let tokens = tokenize(source);
    assert!(normalise_tokens(&tokens).is_none());
}

#[test]
fn a_vector_typedef_cast_is_not_a_call() {
    let source = "typedef long long __v2di __attribute__ ((__vector_size__ (16))); \
                  __v2di f(__v2di a) { return (__v2di)(a); }";
    let tokens = tokenize(source);
    assert!(normalise_tokens(&tokens).is_none());
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
    assert!(
        after.iter().all(|text| text == "q_free" || text == "}"),
        "{after:?}"
    );
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

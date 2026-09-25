use super::super::{function_names, tokenize_with_conditionals};
use super::{derivation_scope, has_internal_linkage};

fn scope(source: &str, selected: &[&str]) -> Vec<String> {
    let (tokens, conditionals) = tokenize_with_conditionals(source);
    let definitions = function_names(&tokens, &conditionals).definitions;
    let selected = selected.iter().map(|name| (*name).to_owned()).collect::<Vec<_>>();
    let mut scope = derivation_scope(&tokens, &selected, &definitions, true);
    scope.sort();
    scope
}

fn linkage(source: &str, function: &str) -> bool {
    let (tokens, _) = tokenize_with_conditionals(source);
    has_internal_linkage(&tokens, function)
}

#[test]
fn a_static_definition_has_internal_linkage() {
    assert!(linkage("static void f(void *p) { }", "f"));
    assert!(linkage("static inline void f(void *p) { }", "f"));
    assert!(linkage(
        "int x; static __attribute__((unused)) inline int f(int a) { return a; }",
        "f"
    ));
    assert!(linkage("void g(void) { } static void *f(void) { return 0; }", "f"));
}

#[test]
fn a_definition_without_static_is_not_established() {
    assert!(!linkage("void f(void *p) { }", "f"));
    assert!(!linkage("inline void f(void *p) { }", "f"));
    assert!(!linkage("extern inline void f(void *p) { }", "f"));
    // `static` on an earlier declaration alone: internal by C11 6.2.2p5, but
    // the reader reads the definition only and answers "not established".
    assert!(!linkage("static void f(void *p); void f(void *p) { }", "f"));
    // The previous definition's `static` does not leak across its `}`.
    assert!(!linkage("static void g(void) { } void f(void) { }", "f"));
    // No definition at all.
    assert!(!linkage("static void f(void *p);", "f"));
}

#[test]
fn the_scope_adds_reached_internal_definitions_only() {
    let source = "static void helper(void *p) { inner(p); }\n\
                  static void inner(void *p) { }\n\
                  static void unreached(void *p) { }\n\
                  inline void external(void *p) { }\n\
                  void selected(void *p) { helper(p); external(p); }\n";
    assert_eq!(
        scope(source, &["selected"]),
        vec!["helper", "inner", "selected"]
    );
}

#[test]
fn without_a_selection_the_scope_is_unchanged() {
    let source = "static void helper(void *p) { }\nvoid selected(void *p) { helper(p); }\n";
    let (tokens, conditionals) = tokenize_with_conditionals(source);
    let definitions = function_names(&tokens, &conditionals).definitions;
    assert_eq!(
        derivation_scope(&tokens, &definitions, &definitions, false),
        definitions
    );
}

#[test]
fn a_selected_static_definition_is_not_duplicated() {
    let source = "static void helper(void *p) { }\nstatic void selected(void *p) { helper(p); }\n";
    assert_eq!(scope(source, &["selected", "helper"]), vec!["helper", "selected"]);
}

#[test]
fn a_reserved_static_definition_is_not_admitted() {
    let source = "static __inline unsigned int __bswap_32(unsigned int x) { return x; }\n\
                  static inline int _Upper(int x) { return x; }\n\
                  static inline int _lower(int x) { return x; }\n\
                  void selected(unsigned int x) { __bswap_32(x); _Upper(1); _lower(2); }\n";
    // `_lower` is reserved at file scope only, which corpus code uses.
    assert_eq!(scope(source, &["selected"]), vec!["_lower", "selected"]);
}

//! Card CG03-CALL-SUMMARY-INSTANTIATION (rule 10u): the acceptance table,
//! the typed substitution against Clang, and their mechanical binding to
//! `proof/QCore/CallInstantiation.lean`.

use super::*;
use crate::legacy_c::{CCompilerDialect, tokenize};
use crate::typed_access_path::{StatementIndex, enter_source};

fn standing(code: &str) -> Standing {
    match code {
        "declared" => Standing::Declared,
        "validated" => Standing::Validated,
        "recursive" => Standing::Recursive,
        other => panic!("unknown standing {other}"),
    }
}

fn result(code: &str) -> ResultClass {
    match code {
        "none" => ResultClass::None,
        "derived" => ResultClass::Derived,
        "owned" => ResultClass::Owned,
        "reallocated" => ResultClass::Reallocated,
        other => panic!("unknown result {other}"),
    }
}

fn use_of(code: &str) -> ResultUse {
    match code {
        "discarded" => ResultUse::Discarded,
        "stored" => ResultUse::Stored,
        "tested" => ResultUse::Tested,
        "returned" => ResultUse::Returned,
        "passed" => ResultUse::Passed,
        other => panic!("unknown use {other}"),
    }
}

fn mode(code: &str) -> ResourceParameterMode {
    match code {
        "borrowed" => ResourceParameterMode::Borrowed,
        "released" => ResourceParameterMode::Released { nullable: false },
        "out" => ResourceParameterMode::Out { nullable: false },
        other => panic!("unknown mode {other}"),
    }
}

#[test]
fn the_acceptance_vectors_hold() {
    for (standing_code, result_code, identity, use_code, decision) in ACCEPTANCE_VECTORS {
        let decided = accept(
            standing(standing_code),
            result(result_code),
            *identity,
            use_of(use_code),
        );
        let code = match decided {
            Ok(()) => "accept",
            Err(decline) => decline.code(),
        };
        assert_eq!(
            code, *decision,
            "{standing_code} {result_code} {identity} {use_code}"
        );
    }
}

/// Position independence, over every row of the table: the consumer is not
/// an argument, so two consumers that use a value the same way decide it the
/// same way; and a tested value is accepted exactly when the condition
/// desugaring's older gate (not recursive, resource-free) takes it.
#[test]
fn a_tested_value_is_accepted_exactly_as_the_condition_gate_takes_it() {
    for standing in [Standing::Declared, Standing::Validated, Standing::Recursive] {
        for result in [
            ResultClass::None,
            ResultClass::Derived,
            ResultClass::Owned,
            ResultClass::Reallocated,
        ] {
            for identity in [false, true] {
                let condition_gate = standing != Standing::Recursive && result.resource_free();
                assert_eq!(
                    accept(standing, result, identity, ResultUse::Tested).is_ok(),
                    condition_gate
                );
                // B01's older return gate is contained in it.
                let b01_gate = standing == Standing::Declared && result == ResultClass::None;
                if b01_gate {
                    assert!(accept(standing, result, identity, ResultUse::Tested).is_ok());
                }
            }
        }
    }
}

/// `f` declares every name a substitution vector spells.
const SUBSTITUTION_FIXTURE: &str = "struct T { int x; }; \
struct S { void *p; int a; }; \
void take(void *x); void take_n(unsigned long n); void take_s(const char *s);";

fn callee_for(spelling: &str) -> &'static str {
    match spelling {
        "n * 2 + sizeof(*p)" | "sizeof(struct T)" | "i++" | "(long)p" | "n" => "take_n",
        "\"text\"" => "take_s",
        _ => "take",
    }
}

/// Every substitution vector, read by Clang as the actual of a one-argument
/// call in a function where `p` is a `void *`, `s` a `struct S *`, `n` an
/// `unsigned long` and `i` an `int`, has the vector's form or refusal.
#[test]
fn every_substitution_vector_reads_as_its_form() {
    if crate::clang_command().is_err() {
        return;
    }
    let mut body = String::new();
    for (spelling, ..) in SUBSTITUTION_VECTORS {
        body.push_str(&format!("\n {}({spelling});", callee_for(spelling)));
    }
    let source = format!(
        "{SUBSTITUTION_FIXTURE} void f(void *p, struct S *s, unsigned long n, int i) {{{body}\n}}"
    );
    let _scope = enter_source(&source, CCompilerDialect::C11, None);
    let index = StatementIndex::read("f").expect("typed function");
    for (line, (spelling, mode_code, expected)) in SUBSTITUTION_VECTORS.iter().enumerate() {
        let call = format!("{}({spelling})", callee_for(spelling));
        let tokens = tokenize(&call);
        let texts = tokens
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        let node = index
            .expression_node(&texts, line + 2)
            .unwrap_or_else(|| panic!("{spelling}: no node"));
        let decided = match substitute(node, &[mode(mode_code)], &index.fields) {
            Ok(substituted) => substituted[0].actual.code().to_owned(),
            Err((_, refusal)) => refusal.code(),
        };
        assert_eq!(decided, *expected, "{spelling} at {mode_code}");
    }
}

#[test]
fn a_call_with_the_wrong_number_of_actuals_is_refused_whole() {
    if crate::clang_command().is_err() {
        return;
    }
    let source = "void take2(void *x, void *y); void f(void *p) {\n take2(p, p);\n}";
    let _scope = enter_source(source, CCompilerDialect::C11, None);
    let index = StatementIndex::read("f").expect("typed function");
    let tokens = tokenize("take2(p, p)");
    let texts = tokens.iter().map(|t| t.text.as_str()).collect::<Vec<_>>();
    let node = index.expression_node(&texts, 2).expect("node");
    assert_eq!(
        substitute(node, &[ResourceParameterMode::Borrowed], &index.fields),
        Err((0, SubstitutionRefusal::Arity))
    );
    // One refused formal refuses the whole call.
    let released = ResourceParameterMode::Released { nullable: false };
    assert!(substitute(node, &[released, released], &index.fields).is_ok());
}

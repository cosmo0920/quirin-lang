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
        "removed" => ResultClass::Removed,
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
        "tested-split" => ResultUse::TestedSplit,
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
            ResultClass::Removed,
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
    let source = "void take2(void *x, void *y); void f(void *p, void *q) {\n take2(p, &q);\n}";
    let _scope = enter_source(source, CCompilerDialect::C11, None);
    let index = StatementIndex::read("f").expect("typed function");
    let tokens = tokenize("take2(p, &q)");
    let texts = tokens.iter().map(|t| t.text.as_str()).collect::<Vec<_>>();
    let node = index.expression_node(&texts, 2).expect("node");
    assert_eq!(
        substitute(node, &[ResourceParameterMode::Borrowed], &index.fields),
        Err((0, SubstitutionRefusal::Arity))
    );
    // One refused formal refuses the whole call: `&q` at a released formal.
    let released = ResourceParameterMode::Released { nullable: false };
    assert_eq!(
        substitute(node, &[released, released], &index.fields),
        Err((1, SubstitutionRefusal::CustodyPosition("address")))
    );
    let substituted = substitute(
        node,
        &[released, ResourceParameterMode::Borrowed],
        &index.fields,
    )
    .expect("both formals substituted");
    assert_eq!(substituted.len(), 2);
    assert_eq!(
        substituted[0].actual,
        Actual::Identity {
            name: "p".into(),
            cast: false
        }
    );
}

/// A Lean constructor name as the Rust code spells it: `.recursiveLocal` is
/// `recursive-local`.
fn lean_code(name: &str) -> String {
    let name = name.trim().trim_start_matches('.');
    let mut code = String::new();
    for character in name.chars() {
        if character.is_ascii_uppercase() {
            code.push('-');
            code.push(character.to_ascii_lowercase());
        } else {
            code.push(character);
        }
    }
    code
}

fn lean_table<'a>(lean: &'a str, header: &str) -> Vec<&'a str> {
    let table = lean.split(header).nth(1).expect("table header");
    let table = &table[..table.find("\n]").expect("end of table")];
    table
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with('('))
        .collect()
}

const LEAN: &str = include_str!("../../../../proof/QCore/CallInstantiation.lean");

/// The mechanical binding of `QCore.CallInstantiation.acceptanceVectors`.
#[test]
fn cg03_lean_acceptance_vectors_match_the_rust_table() {
    let rows = lean_table(
        LEAN,
        "def acceptanceVectors : List (Standing × ResultClass × Bool × Use × Decision) := [",
    );
    assert_eq!(rows.len(), ACCEPTANCE_VECTORS.len());
    for (row, vector) in rows.iter().zip(ACCEPTANCE_VECTORS) {
        let fields = row
            .trim_end_matches(',')
            .trim_start_matches('(')
            .trim_end_matches(')')
            .split(',')
            .map(lean_code)
            .collect::<Vec<_>>();
        assert_eq!(
            fields,
            [
                vector.0.to_owned(),
                vector.1.to_owned(),
                vector.2.to_string(),
                vector.3.to_owned(),
                vector.4.to_owned()
            ],
            "{row}"
        );
    }
}

/// The mechanical binding of `QCore.CallInstantiation.substitutionVectors`:
/// the spelling, the formal's mode, and the Rust reading or refusal as the
/// Lean actual and substitution.
#[test]
fn cg03_lean_substitution_vectors_match_the_rust_table() {
    let rows = lean_table(
        LEAN,
        "def substitutionVectors : List (String × Mode × Actual × Subst) := [",
    );
    assert_eq!(rows.len(), SUBSTITUTION_VECTORS.len());
    for (row, (spelling, mode, expected)) in rows.iter().zip(SUBSTITUTION_VECTORS) {
        let rest = row.strip_prefix("(\"").expect("a spelling");
        let mut literal = String::new();
        let mut chars = rest.char_indices();
        let mut end = 0;
        while let Some((at, character)) = chars.next() {
            match character {
                '\\' => {
                    if let Some((_, escaped)) = chars.next() {
                        literal.push(escaped);
                    }
                }
                '"' => {
                    end = at + 1;
                    break;
                }
                other => literal.push(other),
            }
        }
        assert_eq!(literal, *spelling);
        let fields = rest[end..]
            .trim_end_matches(',')
            .trim_end_matches(')')
            .trim_start_matches(',')
            .split(',')
            .map(str::trim)
            .collect::<Vec<_>>();
        assert_eq!(lean_code(fields[0]), *mode, "{row}");
        let actual = fields[1].split_whitespace().collect::<Vec<_>>();
        let subst = lean_code(fields[2].split_whitespace().next().expect("subst"));
        let lean_reading = match actual.as_slice() {
            [".identity", _, "false"] => "identity".to_owned(),
            [".identity", _, "true"] => "identity-cast".to_owned(),
            [constructor, ..] => lean_code(constructor),
            [] => panic!("{row}: no actual"),
        };
        if expected.starts_with("form:") {
            // A form another card owns: the Lean reading is `other`.
            assert_eq!(lean_reading, "other", "{row}");
            assert_eq!(subst, "reject", "{row}");
        } else if let Some(what) = expected.strip_prefix("custody-position:") {
            let reading = match what {
                "value" => "data",
                other => other,
            };
            assert_eq!(lean_reading, reading, "{row}");
            assert_eq!(subst, "reject", "{row}");
        } else {
            assert_eq!(lean_reading, *expected, "{row}");
            assert_ne!(subst, "reject", "{row}");
        }
    }
}

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

//! The three-layer unrolling budget of `source.loop.v1` — card
//! `LOOP-UNROLL-BUDGET`.
//!
//! Layer one is the **absolute ceiling** no declaration may raise; layer two is
//! the **default budget** a `for` with no attribute is held to; layer three is
//! the **licence** a `[| #unroll, where #bound < N |]` clause list declares
//! immediately before a `for`, checked against the bound the checker computes
//! rather than taken on trust.
//!
//! Every refusal below lands as a **condition of a code `LOOP-V1` already
//! allocated** — `Q097` for a clause list this reader cannot read or one that
//! licenses nothing, `Q098` for every question about a number. This card
//! allocates **no new diagnostic code**, per the standing `Q-NAMESPACE-DECISION`
//! note beside §L21.
#[cfg(target_os = "windows")]
#[path = "../../tests/support/windows_c.rs"]
mod windows_c;

use qcore_compiler::{
    CompileOptions, SourceFile, compile_files, elaborate_source, parse_source_program,
};
use qcore_core::ownership::{self, Footprint, OwnershipOp};
use qcore_core::source_loop::{
    BudgetVerdict, DEFAULT_UNROLL_BUDGET, MAX_DECLARED_LIMIT, MAX_ITERATIONS, MAX_UNROLLED_EVENTS,
    UNROLL_CEILING, UnrollLicense, admit_unroll, peak_candidate_prefixes,
};
use qcore_frontend::lexer::{Symbol, TokenKind, lex};

fn compile(
    source: &str,
) -> Result<qcore_compiler::CompiledModule, Vec<qcore_compiler::FileDiagnostic>> {
    compile_files(
        &[SourceFile::new("loop_unroll_budget.qcore", source)],
        &CompileOptions {
            entry: None,
            extern_contracts: None,
            native_libraries: None,
            native_library_headers: None,
        },
    )
}

fn codes(source: &str) -> Vec<String> {
    compile(source)
        .map(|module| module.execution_profile)
        .expect_err("this fixture must be refused")
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect()
}

fn messages(source: &str) -> String {
    compile(source)
        .map(|module| module.execution_profile)
        .expect_err("this fixture must be refused")
        .into_iter()
        .map(|diagnostic| diagnostic.message)
        .collect::<Vec<_>>()
        .join("\n")
}

/// A one-entry byte-buffer module around `body`, so each rule is measured
/// against the same declarations.
fn module(effects: &str, body: &[&str]) -> String {
    let mut source = vec![
        "#import std.buffer;".to_owned(),
        "#import std.fmt;".to_owned(),
        "#import std.io;".to_owned(),
        String::new(),
        format!("fn main() -> unit effects {{ {effects} }} {{"),
    ];
    source.extend(body.iter().map(|line| format!("    {line}")));
    source.push("}".to_owned());
    source.push(String::new());
    source.join("\n")
}

// --- The census and the lexing pins. ---------------------------------------

/// Constraint 14's census result for the one **character** this card claims.
///
/// `#` was a lexer error (`unexpected character '#'`) and becomes a token, on
/// the precedent `OR-AND-LOWERING` set for `^`. The census found **zero** `#`
/// characters in code position across all 422 `.qcore`/`.qc` files in the tree
/// and none inside a QCore fixture embedded in a Rust literal, so the promotion
/// collides with nothing.
#[test]
fn the_hash_lexes_as_its_own_one_byte_token() {
    let kinds: Vec<TokenKind> = lex("#")
        .expect("`#` is a token")
        .into_iter()
        .map(|token| token.kind)
        .collect();
    assert_eq!(
        kinds,
        vec![TokenKind::Symbol(Symbol::Hash), TokenKind::Eof],
        "one byte, one token"
    );
}

/// **The `[| |]` adjacency pins, reproduced here.**
///
/// The vertical-bar family's longest-match table is card `PIPE-OPERATOR`'s,
/// inherited by `OR-AND-LOWERING` and pinned by
/// `qcore_frontend::lexer::tests::lexes_the_vertical_bar_family_by_longest_match`.
/// This card adds a one-byte spelling with no two-byte member, so **not one row
/// of that table can move** — and rather than assert that in prose, the rows are
/// re-measured here beside the new token.
#[test]
fn no_row_of_the_vertical_bar_family_moves_when_the_hash_becomes_a_token() {
    let kinds = |source: &str| {
        lex(source)
            .unwrap()
            .into_iter()
            .map(|token| token.kind)
            .collect::<Vec<_>>()
    };
    assert_eq!(
        kinds("[|>"),
        vec![
            TokenKind::Symbol(Symbol::QuantOpen),
            TokenKind::Symbol(Symbol::Greater),
            TokenKind::Eof,
        ]
    );
    assert_eq!(
        kinds("[||]"),
        vec![
            TokenKind::Symbol(Symbol::QuantOpen),
            TokenKind::Symbol(Symbol::QuantClose),
            TokenKind::Eof,
        ]
    );
    assert_eq!(
        kinds("a |> b"),
        vec![
            TokenKind::Word("a".into()),
            TokenKind::Symbol(Symbol::PipeInto),
            TokenKind::Word("b".into()),
            TokenKind::Eof,
        ]
    );
    assert_eq!(
        kinds("a || b"),
        vec![
            TokenKind::Word("a".into()),
            TokenKind::Symbol(Symbol::PipePipe),
            TokenKind::Word("b".into()),
            TokenKind::Eof,
        ]
    );
    // And the attribute's own opener is the **same two tokens** the quant-index
    // alias uses: this card adds no bracket spelling, it borrows one.
    assert_eq!(
        kinds("[| #unroll |]"),
        vec![
            TokenKind::Symbol(Symbol::QuantOpen),
            TokenKind::Symbol(Symbol::Hash),
            TokenKind::Word("unroll".into()),
            TokenKind::Symbol(Symbol::QuantClose),
            TokenKind::Eof,
        ]
    );
}

/// The census's one genuine collision, and the disambiguation that answers it.
///
/// `where` occurs **eight** times in code position in the pinned corpus, every
/// one of them inside a `quant<n where n := 5>` clause — and the readable alias
/// of that very type is spelled `quant[| n where n := 5 |]`, the same brackets
/// this attribute borrows. The statement head is recognised as `[|` **followed
/// by `#`**, which the census found nowhere at all, so the type form is
/// untouched. Measured rather than asserted.
#[test]
fn the_quant_index_alias_still_elaborates_identically_to_the_angle_form() {
    let source = |quant: &str| {
        format!(
            "dimension Bytes : Idx + Ordered;\n\
             quant axis memory : credit<Bytes>;\n\
             extern fn f(q: {quant}) -> unit;\n"
        )
    };
    let angle = elaborate_source(&parse_source_program(&source("quant<n where n := 5>")).unwrap())
        .expect("the angle form is unchanged");
    let bracket =
        elaborate_source(&parse_source_program(&source("quant[|n where n := 5|]")).unwrap())
            .expect("the readable alias is unchanged");
    let quant_of = |typed: &qcore_compiler::TypedSourceProgram| {
        typed
            .declarations
            .iter()
            .find(|declaration| declaration.name == "f")
            .unwrap()
            .quant_index_types[0]
            .clone()
    };
    assert_eq!(quant_of(&angle).bindings, quant_of(&bracket).bindings);
}

/// The other half: `[|` at a **statement head** without a `#` is not claimed by
/// this card at all, so whatever it did before it still does.
#[test]
fn a_bracket_pipe_without_a_hash_is_not_an_attribute() {
    let source = module(
        "alloc, mut",
        &[
            "let b = bytes(4);",
            "[| unroll, where bound < 300 |]",
            "for i in 0..2 { b[0] = 1; }",
            "drop b;",
            "return;",
        ],
    );
    let codes = codes(&source);
    assert!(
        !codes.iter().any(|code| code == "Q097" || code == "Q098"),
        "the loop pass does not claim this shape: {codes:?}"
    );
}

// --- Layer two: the default budget. ----------------------------------------

#[test]
fn a_loop_at_the_default_budget_needs_no_attribute() {
    let source = module(
        "alloc, mut",
        &[
            "let b = bytes(4);",
            &format!("for i in 0..{DEFAULT_UNROLL_BUDGET} {{ b[0] = 1; }}"),
            "drop b;",
            "return;",
        ],
    );
    let compiled = compile(&source).unwrap_or_else(|_| panic!("{:?}", codes(&source)));
    let checked = compiled.checked.as_standard_buffer().unwrap();
    assert_eq!(
        checked.certificate.accesses.len(),
        DEFAULT_UNROLL_BUDGET as usize
    );
}

/// **The refusal teaches the spelling.** A bound one above the default is
/// refused at the bound band, and the message carries the exact clause list
/// that would license this loop — not a rule to look up.
#[test]
fn a_loop_above_the_default_refuses_and_the_message_names_the_attribute() {
    let bound = DEFAULT_UNROLL_BUDGET + 1;
    let source = module(
        "alloc, mut",
        &[
            "let b = bytes(4);",
            &format!("for i in 0..{bound} {{ b[0] = 1; }}"),
            "drop b;",
            "return;",
        ],
    );
    assert_eq!(codes(&source), ["Q098"]);
    let message = messages(&source);
    assert!(
        message.contains(&format!("[| #unroll, where #bound < {} |]", bound + 1)),
        "{message}"
    );
    assert!(message.contains(&UNROLL_CEILING.to_string()), "{message}");
}

// --- Layer three: the licence. ---------------------------------------------

/// The card's headline: a bound above the default, licensed and admitted.
#[test]
fn an_attribute_licenses_a_bound_above_the_default() {
    let source = module(
        "alloc, mut",
        &[
            "let b = bytes(4);",
            "[| #unroll, where #bound < 1024 |]",
            "for i in 0..600 { b[0] = 1; }",
            "drop b;",
            "return;",
        ],
    );
    let compiled = compile(&source).unwrap_or_else(|_| panic!("{:?}", codes(&source)));
    let checked = compiled.checked.as_standard_buffer().unwrap();
    assert_eq!(checked.certificate.accesses.len(), 600);
}

/// **The declaration is checked, never trusted**, and the refusal names both
/// numbers so the reader can see which one to change.
#[test]
fn a_bound_the_declared_condition_does_not_satisfy_refuses() {
    let source = module(
        "alloc, mut",
        &[
            "let b = bytes(4);",
            "[| #unroll, where #bound < 300 |]",
            "for i in 0..600 { b[0] = 1; }",
            "drop b;",
            "return;",
        ],
    );
    assert_eq!(codes(&source), ["Q098"]);
    let message = messages(&source);
    assert!(
        message.contains("300") && message.contains("600"),
        "{message}"
    );
}

/// `#bound < N` is **strict**, so `N` itself is not licensed. Stating that as a
/// test is what keeps the spelled condition and the checked condition the same
/// relation.
#[test]
fn the_declared_condition_is_strict() {
    let source = module(
        "alloc, mut",
        &[
            "let b = bytes(4);",
            "[| #unroll, where #bound < 300 |]",
            "for i in 0..300 { b[0] = 1; }",
            "drop b;",
            "return;",
        ],
    );
    assert_eq!(codes(&source), ["Q098"]);
    // One below it is licensed, which makes the refusal a boundary.
    let admitted = module(
        "alloc, mut",
        &[
            "let b = bytes(4);",
            "[| #unroll, where #bound < 300 |]",
            "for i in 0..299 { b[0] = 1; }",
            "drop b;",
            "return;",
        ],
    );
    assert!(compile(&admitted).is_ok());
}

// --- Layer one: the ceiling stays a ceiling. -------------------------------

/// A declaration above the ceiling is refused **before** this loop's own bound
/// is looked at, so a tiny loop cannot carry a licence for a huge one.
#[test]
fn a_declaration_above_the_ceiling_refuses_even_on_a_small_loop() {
    let source = module(
        "alloc, mut",
        &[
            "let b = bytes(4);",
            "[| #unroll, where #bound < 99999 |]",
            "for i in 0..2 { b[0] = 1; }",
            "drop b;",
            "return;",
        ],
    );
    assert_eq!(codes(&source), ["Q098"]);
    let message = messages(&source);
    assert!(message.contains(&UNROLL_CEILING.to_string()), "{message}");
    assert!(
        message.contains(&format!("#bound < {MAX_DECLARED_LIMIT}")),
        "the refusal names the largest condition the construct admits: {message}"
    );
}

/// The exact boundary, from both sides. `#bound < 4097` licenses a traversal of
/// the largest buffer the byte route admits — the property that kept the
/// ceiling at 4096 — and `#bound < 4098` does not exist.
#[test]
fn the_largest_declaration_licenses_the_full_extent_traversal_and_nothing_above_it() {
    let admitted = module(
        "alloc, mut",
        &[
            "let b = bytes(1);",
            &format!("[| #unroll, where #bound < {MAX_DECLARED_LIMIT} |]"),
            &format!("for i in 0..{MAX_ITERATIONS} {{ b[0] = 1; }}"),
            "drop b;",
            "return;",
        ],
    );
    assert!(compile(&admitted).is_ok());
    let refused = module(
        "alloc, mut",
        &[
            "let b = bytes(1);",
            &format!("[| #unroll, where #bound < {} |]", MAX_DECLARED_LIMIT + 1),
            &format!("for i in 0..{MAX_ITERATIONS} {{ b[0] = 1; }}"),
            "drop b;",
            "return;",
        ],
    );
    assert_eq!(codes(&refused), ["Q098"]);
}

/// A licence raises the **per-loop** bound and nothing else: the entry's
/// unrolled-event budget is untouched, so a nest of licensed loops is still
/// refused by the cost the checking route actually pays.
#[test]
fn a_licence_does_not_raise_the_entry_budget() {
    let source = module(
        "alloc, mut",
        &[
            "let b = bytes(4);",
            "[| #unroll, where #bound < 1024 |]",
            "for i in 0..300 {",
            "    [| #unroll, where #bound < 1024 |]",
            "    for j in 0..300 { b[0] = 1; }",
            "}",
            "drop b;",
            "return;",
        ],
    );
    assert_eq!(codes(&source), ["Q098"]);
    assert!(messages(&source).contains(&MAX_UNROLLED_EVENTS.to_string()));
}

/// The attribute reaches an **inner** loop, because the pairing is carried by
/// the reader that walks a statement sequence and a loop body is one.
#[test]
fn an_attribute_licenses_an_inner_loop_too() {
    let source = module(
        "alloc, mut",
        &[
            "let b = bytes(4);",
            "for i in 0..2 {",
            "    [| #unroll, where #bound < 400 |]",
            "    for j in 0..300 { b[0] = 1; }",
            "}",
            "drop b;",
            "return;",
        ],
    );
    let compiled = compile(&source).unwrap_or_else(|_| panic!("{:?}", codes(&source)));
    let checked = compiled.checked.as_standard_buffer().unwrap();
    assert_eq!(checked.certificate.accesses.len(), 600);
}

// --- The clause list's shape: `Q097`. --------------------------------------

#[test]
fn an_unknown_clause_word_refuses_rather_than_being_ignored() {
    let source = module(
        "alloc, mut",
        &[
            "let b = bytes(4);",
            "[| #inline, where #bound < 300 |]",
            "for i in 0..2 { b[0] = 1; }",
            "drop b;",
            "return;",
        ],
    );
    assert_eq!(codes(&source), ["Q097"]);
    assert!(messages(&source).contains("#inline"));
}

/// `[| #unroll |]` licenses nothing, because there is no condition in it to
/// check. The full form is required rather than defaulted to the ceiling.
#[test]
fn an_attribute_without_its_condition_refuses() {
    let source = module(
        "alloc, mut",
        &[
            "let b = bytes(4);",
            "[| #unroll |]",
            "for i in 0..2 { b[0] = 1; }",
            "drop b;",
            "return;",
        ],
    );
    assert_eq!(codes(&source), ["Q097"]);
    assert!(messages(&source).contains("licenses nothing"));
}

#[test]
fn a_comparison_other_than_strictly_less_refuses() {
    for clause in [
        "[| #unroll, where #bound <= 300 |]",
        "[| #unroll, where #bound > 300 |]",
        "[| #unroll, where #bound == 300 |]",
    ] {
        let source = module(
            "alloc, mut",
            &[
                "let b = bytes(4);",
                clause,
                "for i in 0..2 { b[0] = 1; }",
                "drop b;",
                "return;",
            ],
        );
        assert_eq!(codes(&source), ["Q097"], "{clause}");
    }
}

/// **One mistake, one refusal.** A clause list this reader cannot read licenses
/// nothing — but holding the loop to the default budget on top of that would
/// print a second refusal for the same mistake, which is card `LOOP-V1` §9.9's
/// rule applied to the attribute. The budget decision is suppressed and every
/// other rule still applies, which the second half measures.
#[test]
fn a_refused_clause_list_suppresses_the_budget_decision_and_nothing_else() {
    let refused_attribute = module(
        "alloc, mut",
        &[
            "let b = bytes(4);",
            "[| #inline, where #bound < 300 |]",
            "for i in 0..600 { b[0] = 1; }",
            "drop b;",
            "return;",
        ],
    );
    assert_eq!(
        codes(&refused_attribute),
        ["Q097"],
        "the bound is 600, far above the default, and it is not reported twice"
    );
    // Every other rule still applies to the loop the unreadable list preceded:
    // the body is still read, and a statement this route does not model is
    // still refused at the route's own code beside the attribute's.
    let still_checked = module(
        "alloc, mut",
        &[
            "let b = bytes(4);",
            "[| #inline, where #bound < 300 |]",
            "for i in 0..600 { break; }",
            "drop b;",
            "return;",
        ],
    );
    let codes = codes(&still_checked);
    assert_eq!(
        codes.len(),
        2,
        "the unreadable list and the body's early exit, and nothing else: {codes:?}"
    );
    assert!(!codes.contains(&"Q098".to_owned()), "{codes:?}");
    assert!(messages(&still_checked).contains("LOOP-BREAK"));
}

/// A condition no bound can satisfy is refused **from the clause list alone**,
/// before any loop is read — so the message names the list rather than a bound
/// the reader has not read yet. `source.loop.v1` counts at least one iteration,
/// so `#bound < 1` and `#bound < 0` license nothing.
#[test]
fn a_condition_no_bound_can_satisfy_refuses_without_naming_one() {
    for limit in ["0", "1"] {
        let source = module(
            "alloc, mut",
            &[
                "let b = bytes(4);",
                &format!("[| #unroll, where #bound < {limit} |]"),
                "for i in 0..5 { b[0] = 1; }",
                "drop b;",
                "return;",
            ],
        );
        assert_eq!(codes(&source), ["Q098"], "{limit}");
        let message = messages(&source);
        assert!(message.contains("licenses no bound at all"), "{message}");
        // And it does not claim anything about this loop's own count.
        assert!(!message.contains("counts 5"), "{message}");
    }
}

#[test]
fn a_limit_the_reader_would_have_to_compute_refuses() {
    let source = module(
        "alloc, mut",
        &[
            "let b = bytes(4);",
            "let n = 300;",
            "[| #unroll, where #bound < n |]",
            "for i in 0..2 { b[0] = 1; }",
            "drop b;",
            "return;",
        ],
    );
    assert_eq!(codes(&source), ["Q097"]);
}

#[test]
fn an_attribute_before_a_statement_that_is_not_a_for_refuses() {
    let source = module(
        "alloc, mut",
        &[
            "let b = bytes(4);",
            "[| #unroll, where #bound < 300 |]",
            "b[0] = 1;",
            "drop b;",
            "return;",
        ],
    );
    assert_eq!(codes(&source), ["Q097"]);
    assert!(messages(&source).contains("licenses nothing"));
}

#[test]
fn a_dangling_attribute_refuses() {
    let source = module(
        "alloc, mut",
        &[
            "let b = bytes(4);",
            "drop b;",
            "return;",
            "[| #unroll, where #bound < 300 |]",
        ],
    );
    assert_eq!(codes(&source), ["Q097"]);
    assert!(messages(&source).contains("dangling"));
}

#[test]
fn a_second_clause_list_refuses_rather_than_one_winning() {
    let source = module(
        "alloc, mut",
        &[
            "let b = bytes(4);",
            "[| #unroll, where #bound < 300 |]",
            "[| #unroll, where #bound < 1024 |]",
            "for i in 0..280 { b[0] = 1; }",
            "drop b;",
            "return;",
        ],
    );
    let codes = codes(&source);
    assert!(codes.contains(&"Q097".to_owned()), "{codes:?}");
}

/// An **unterminated** clause list is a parse failure, where a **malformed**
/// one is this construct's own refusal — the difference being that there is no
/// statement to attribute a diagnostic to. Recorded rather than smoothed over.
#[test]
fn an_unterminated_clause_list_is_a_parse_failure() {
    let source = module(
        "alloc, mut",
        &[
            "let b = bytes(4);",
            "[| #unroll, where #bound < 300",
            "for i in 0..2 { b[0] = 1; }",
            "drop b;",
            "return;",
        ],
    );
    assert_eq!(codes(&source), ["Q000"]);
}

// --- Route admission, measured. -------------------------------------------

/// The attribute is read by the **one** route that admits the loop. Every other
/// route's outcome is measured and pinned rather than widened, the
/// `LOOP-ROUTE-WIDENING` precedent: a clause list reaches a reader that does not
/// model it and is refused at that route's own code, beside the refusal the
/// `for` after it already had.
///
/// One cosmetic finding, recorded rather than chased into another route's
/// reader: `source.std-io.v1`'s generic message is built from the statement's
/// head *word*, and an attribute's head is a symbol, so the message reads
/// ``` `` is not a statement `source.std-io.v1` models ```. The code and the
/// refusal are right and the empty name is not
/// (`LOOP-ATTRIBUTE-FOREIGN-ROUTE-MESSAGE`).
#[test]
fn a_route_that_does_not_admit_the_loop_refuses_the_attribute_at_its_own_code() {
    let standard_io = "#import std.fmt;\n#import std.io;\n\nfn main() -> unit effects { io } {\n    [| #unroll, where #bound < 1024 |]\n    for i in 0..3 { print dec i; }\n    return;\n}\n";
    assert_eq!(codes(standard_io), ["Q030", "Q030"]);
    let sequential = "fn main() -> i32 {\n    [| #unroll, where #bound < 1024 |]\n    for i in 0..3 { }\n    return 0;\n}\n";
    assert!(codes(sequential).contains(&"Q500".to_owned()));
}

// --- Erasure. --------------------------------------------------------------

/// **The attribute governs the checker's budget and nothing else.** Two modules
/// whose only difference is the clause list emit byte-identical C, which is the
/// emission half of the erasure the Lean model states over the unrolled event
/// list.
#[test]
fn an_attributed_and_an_unattributed_loop_emit_byte_identical_c() {
    let body = |attributed: bool| {
        let mut lines = vec!["let b = bytes(4);".to_owned()];
        if attributed {
            lines.push("[| #unroll, where #bound < 64 |]".to_owned());
        }
        lines.push("for i in 0..8 { b[0] = 1; }".to_owned());
        lines.push("drop b;".to_owned());
        lines.push("return;".to_owned());
        let borrowed: Vec<&str> = lines.iter().map(String::as_str).collect();
        module("alloc, mut", &borrowed)
    };
    let plain = compile(&body(false)).unwrap();
    let attributed = compile(&body(true)).unwrap();
    assert_eq!(
        attributed.c_source.as_deref().unwrap(),
        plain.c_source.as_deref().unwrap(),
        "the attribute erases from emission entirely"
    );
    // And the checked certificate is the same one, so the attribute changes no
    // published value either.
    assert_eq!(
        attributed
            .checked
            .as_standard_buffer()
            .unwrap()
            .certificate
            .accesses
            .len(),
        plain
            .checked
            .as_standard_buffer()
            .unwrap()
            .certificate
            .accesses
            .len()
    );
}

// --- The decision itself, over the two numbers. ----------------------------

/// The three-layer decision as the total function it is, exercised directly so
/// the rule is visible without a route around it. This is the Rust twin of the
/// Lean `refused_shapes_are_exactly_these` convention.
#[test]
fn the_three_layer_decision_is_total_and_ordered() {
    let licence = |limit| Some(UnrollLicense { limit });
    // Layer two, both sides of the boundary.
    assert_eq!(admit_unroll(1, None), BudgetVerdict::Default);
    assert_eq!(
        admit_unroll(DEFAULT_UNROLL_BUDGET, None),
        BudgetVerdict::Default
    );
    assert_eq!(
        admit_unroll(DEFAULT_UNROLL_BUDGET + 1, None),
        BudgetVerdict::AboveDefault
    );
    // Layer three, both sides of the declared condition.
    assert_eq!(admit_unroll(299, licence(300)), BudgetVerdict::Licensed);
    assert_eq!(
        admit_unroll(300, licence(300)),
        BudgetVerdict::LicenseViolated
    );
    // Layer one, and the order that makes it a ceiling: the declaration is
    // judged before the loop's own bound, so a small loop cannot carry a
    // licence for a large one.
    assert_eq!(
        admit_unroll(MAX_ITERATIONS, licence(MAX_DECLARED_LIMIT)),
        BudgetVerdict::Licensed
    );
    assert_eq!(
        admit_unroll(2, licence(MAX_DECLARED_LIMIT + 1)),
        BudgetVerdict::LicenseAboveCeiling
    );
    assert_eq!(
        admit_unroll(2, licence(u32::MAX)),
        BudgetVerdict::LicenseAboveCeiling
    );
    // Exactly two of the five verdicts admit.
    for (verdict, admitted) in [
        (BudgetVerdict::Default, true),
        (BudgetVerdict::Licensed, true),
        (BudgetVerdict::AboveDefault, false),
        (BudgetVerdict::LicenseAboveCeiling, false),
        (BudgetVerdict::LicenseViolated, false),
    ] {
        assert_eq!(verdict.is_admitted(), admitted, "{verdict:?}");
    }
}

/// The peak-live repair: **the same number**, in one pass.
///
/// Card `LOOP-V1` narrowed a scan of every prefix to the prefixes that can be
/// the maximum but still re-executed each one from the start, which a body
/// acquiring once per iteration pays quadratically — 56.4 s at a bound of 1024
/// against 0.20 s for this pass. The number is unchanged and that is what this
/// test says, measured against **both** earlier routes.
#[test]
fn the_one_pass_peak_is_the_number_a_full_prefix_scan_gives() {
    let acquire = |id: u64, owner: &str, bytes: u64| OwnershipOp::Acquire {
        id,
        owner: owner.to_owned(),
        resource_type: "T".to_owned(),
        footprint: Footprint::single("bytes", bytes),
    };
    // An unrolled acquire/release body beside two overlapping outer buffers, so
    // the maximum is attained in the middle rather than at either end.
    let mut operations = vec![acquire(1, "a", 3), acquire(2, "b", 5)];
    for iteration in 0..16u64 {
        operations.push(acquire(10 + iteration, &format!("c#{iteration}"), 7));
        operations.push(OwnershipOp::Read(format!("c#{iteration}")));
        operations.push(OwnershipOp::Drop(format!("c#{iteration}")));
    }
    operations.push(OwnershipOp::Drop("b".to_owned()));
    operations.push(OwnershipOp::Drop("a".to_owned()));

    let full = (0..=operations.len())
        .filter_map(|length| ownership::execute_ownership(&operations[..length]).ok())
        .map(|state| state.live().get("bytes"))
        .max()
        .unwrap();
    let narrowed = peak_candidate_prefixes(&operations)
        .into_iter()
        .filter_map(|length| ownership::execute_ownership(&operations[..length]).ok())
        .map(|state| state.live().get("bytes"))
        .max()
        .unwrap();
    let one_pass = ownership::peak_live_axis(&operations, "bytes");
    assert_eq!(full, 15);
    assert_eq!(narrowed, full);
    assert_eq!(one_pass, full);
}

/// A refused prefix ends the walk, exactly as it ended the scan: the maximum
/// reported is the maximum over the prefixes that executed, and nothing about a
/// refused one is invented.
#[test]
fn the_one_pass_peak_stops_where_the_model_refuses() {
    let operations = vec![
        OwnershipOp::Acquire {
            id: 1,
            owner: "a".to_owned(),
            resource_type: "T".to_owned(),
            footprint: Footprint::single("bytes", 9),
        },
        OwnershipOp::Drop("a".to_owned()),
        // A second release of a released resource: the model refuses here.
        OwnershipOp::Drop("a".to_owned()),
    ];
    assert!(ownership::execute_ownership(&operations).is_err());
    assert_eq!(ownership::peak_live_axis(&operations, "bytes"), 9);
}

// --- The native evidence. --------------------------------------------------

/// **The positive control the card asks for**: a bound above the default,
/// licensed by the attribute, checked by unrolling and executed by a rolled C
/// loop, with both streams compared byte for byte under ASan/UBSan (and LSan
/// where the platform provides it).
///
/// The reference execution ran 600 iterations; the native run computes the
/// index itself. A disagreement would appear as a byte difference rather than
/// as a silent success.
#[test]
fn a_licensed_loop_runs_sanitized_and_matches_the_reference_per_stream() {
    let source = module(
        "io, alloc, mut",
        &[
            "let b = bytes(4);",
            "[| #unroll, where #bound < 1024 |]",
            "for i in 0..600 { b[0] = 1; }",
            "let seen = b[0];",
            "print dec seen;",
            "[| #unroll, where #bound < 400 |]",
            "for k in 0..300 { print dec k; }",
            "drop b;",
            "return;",
        ],
    );
    let compiled = compile(&source).unwrap_or_else(|_| panic!("{:?}", codes(&source)));
    let observation = compiled.extern_observation.as_deref().unwrap();
    let expected_out: Vec<&str> = observation
        .iter()
        .filter_map(|line| line.strip_prefix("out: "))
        .collect();
    let expected_err: Vec<&str> = observation
        .iter()
        .filter_map(|line| line.strip_prefix("err: "))
        .collect();
    assert_eq!(expected_out.len(), 301, "one byte read plus 300 indices");
    let (out, err) = run_native(compiled.c_source.as_deref().unwrap(), "licensed");
    assert_eq!(out, expected_out);
    assert_eq!(err, expected_err);
}

/// Compile and run the emitted C under ASan/UBSan (plus LSan where the platform
/// provides it), returning `(stdout lines, stderr lines)`.
fn run_native(source: &str, _label: &str) -> (Vec<String>, Vec<String>) {
    #[cfg(target_os = "windows")]
    {
        let run = windows_c::run(source, None, &[]);
        assert!(
            run.status.success(),
            "emitted program failed:\n{}",
            String::from_utf8_lossy(&run.stderr)
        );
        let out = String::from_utf8(run.stdout)
            .unwrap()
            .lines()
            .map(ToOwned::to_owned)
            .collect();
        let err = String::from_utf8(run.stderr)
            .unwrap()
            .lines()
            .map(ToOwned::to_owned)
            .collect();
        (out, err)
    }

    #[cfg(not(target_os = "windows"))]
    {
        use std::process::Command;
        let directory = std::env::temp_dir().join(format!(
            "qcore-loop-budget-{_label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&directory).unwrap();
        let generated = directory.join("generated.c");
        let binary = directory.join("generated");
        std::fs::write(&generated, source).unwrap();
        let compiler = std::env::var("QCORE_CLANG").unwrap_or_else(|_| "cc".to_owned());
        let compile = Command::new(&compiler)
            .args([
                "-std=c11",
                "-Wall",
                "-Wextra",
                "-Werror",
                "-pedantic",
                "-g",
                "-O0",
                "-fsanitize=address,undefined",
                "-fno-omit-frame-pointer",
            ])
            .arg(&generated)
            .arg("-o")
            .arg(&binary)
            .output()
            .unwrap();
        assert!(
            compile.status.success(),
            "C compiler rejected the emitted program:\n{}",
            String::from_utf8_lossy(&compile.stderr)
        );
        let run = Command::new(&binary)
            .env(
                "ASAN_OPTIONS",
                if cfg!(target_os = "macos") {
                    "detect_leaks=0:halt_on_error=1"
                } else {
                    "detect_leaks=1:halt_on_error=1"
                },
            )
            .env("UBSAN_OPTIONS", "halt_on_error=1:print_stacktrace=1")
            .output()
            .unwrap();
        assert!(
            run.status.success(),
            "emitted program failed under ASan/UBSan/LSan:\n{}",
            String::from_utf8_lossy(&run.stderr)
        );
        let out = String::from_utf8(run.stdout)
            .unwrap()
            .lines()
            .map(ToOwned::to_owned)
            .collect();
        let err = String::from_utf8(run.stderr)
            .unwrap()
            .lines()
            .map(ToOwned::to_owned)
            .collect();
        std::fs::remove_dir_all(directory).unwrap();
        (out, err)
    }
}

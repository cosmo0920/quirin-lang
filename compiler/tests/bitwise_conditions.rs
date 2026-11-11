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

//! The integer tier, unary negation and the check-time constant bindings of
//! `source.branch.v1` (card `BITWISE-CONDITIONS`).
//!
//! The card unifies three named follow-ons that all live in the
//! condition/expression tier — `BITWISE-OPERATORS`, `IF-CONNECTIVE-NEGATION`
//! and `IF-CONDITION-BINDINGS` — and its headline decision is a rule for
//! declaring precedence at all:
//!
//! > **A precedence is declared only where it agrees with C. Where this tier's
//! > two sorts would force a different ranking than C's, the unparenthesized
//! > spelling is refused rather than silently redefined.**
//!
//! C ranks `|`, `^` and `&` *looser* than the comparisons and `!` *tighter*
//! than them; a tier in which a comparison yields a condition and no condition
//! is an integer operand must rank them the other way. So `flags | MASK == 0`
//! and `!A < B` are **refused** and their parenthesized spellings are the
//! programs. Everything else — `&` over `^` over `|`, `!` over `&&` over `||` —
//! is C's own order and is declared.
//!
//! Two absences are made measurable rather than asserted: a negation emits
//! **byte-identical** C to its hand-swapped twin, and no custody-bearing value
//! gains a position anywhere in the tier.
use std::collections::BTreeMap;

use qcore_compiler::{CompileOptions, SourceFile, compile_files};

#[path = "../../tests/support/sequential_observation.rs"]
mod sequential_observation;

#[cfg(target_os = "windows")]
#[path = "../../tests/support/windows_c.rs"]
mod windows_c;

const NEGATED: &str = include_str!("fixtures/bitwise/negated_condition.qr");
const NEGATED_TWIN: &str = include_str!("fixtures/bitwise/negated_condition_twin.qr");
const MASKED: &str = include_str!("fixtures/bitwise/masked_condition.qr");
const LINES: &str = include_str!("fixtures/std_modules/lines.qr");

fn compile_named(
    path: &str,
    source: &str,
) -> Result<qcore_compiler::CompiledModule, Vec<qcore_compiler::FileDiagnostic>> {
    compile_files(
        &[SourceFile::new(path, source)],
        &CompileOptions {
            entry: None,
            extern_contracts: None,
            native_libraries: None,
            native_library_headers: None,
        },
    )
}

fn compile(
    source: &str,
) -> Result<qcore_compiler::CompiledModule, Vec<qcore_compiler::FileDiagnostic>> {
    compile_named("bitwise.qcore", source)
}

fn codes(source: &str) -> Vec<String> {
    compile(source)
        .map(|module| module.execution_profile)
        .expect_err("this fixture must be refused")
        .into_iter()
        .map(|diagnostic| diagnostic.code.to_owned())
        .collect()
}

fn message(source: &str) -> String {
    compile(source)
        .map(|module| module.execution_profile)
        .expect_err("this fixture must be refused")
        .first()
        .expect("a refusal carries at least one diagnostic")
        .message
        .clone()
}

fn program(body: &str) -> String {
    format!("error Failure {{ Failed; }}\nfn main() -> i32 {{\n{body}\n}}\n")
}

/// One custody-balanced branch on the given condition: the shape every
/// condition test uses, so a refusal is always about the condition rather than
/// about the arms.
fn balanced(condition: &str) -> String {
    program(&format!(
        "  let a = qcore_acquire(1, true, Failure);\n  \
         if ({condition}) {{ drop a; }} else {{ drop a; }}\n  return 0;"
    ))
}

// ---------------------------------------------------------------------------
// 1. The three operators gain a meaning, in exactly one position
// ---------------------------------------------------------------------------

#[test]
fn the_three_bitwise_operators_are_admitted_as_a_grouped_comparison_operand() {
    for condition in [
        "(3 | 4) == 7",
        "(6 & 3) == 2",
        "(6 ^ 3) == 5",
        "(-1 & 255) == 255",
        "((1 | 2) & 3) == 3",
        "(1 | 2 & 3) == 3",
        "(1 ^ 2 ^ 4) == 7",
    ] {
        compile(&balanced(condition))
            .unwrap_or_else(|errors| panic!("`{condition}` must be admitted: {errors:?}"));
    }
}

#[test]
fn a_bitwise_operator_outside_a_condition_keeps_its_refusal() {
    // The gate is positional, exactly as it is for `||` and `&&`: outside an
    // `if` condition there is no integer expression tier to hold the value, so
    // every occurrence still refuses at `Q038`. Each unclaimed position is
    // listed rather than described.
    for body in [
        "  return 1 | 2;",
        "  let n = 1 | 2;",
        "  drop 1 | 2;",
        "  qcore_check(1 | 2, Failure);",
        "  while (1 | 2) { }",
    ] {
        assert_eq!(codes(&program(body)), ["Q038"], "for `{body}`");
    }
}

#[test]
fn the_gate_message_names_the_one_admitted_position() {
    let message = message(&program("  return 1 | 2;"));
    assert!(
        message.contains("`|` is the bitwise-or operator"),
        "{message}"
    );
    assert!(message.contains("`if` condition"), "{message}");
    assert!(
        message.contains("parenthesized comparison operand"),
        "{message}"
    );
}

#[test]
fn a_bitwise_operator_in_a_contract_or_library_module_refuses() {
    // The gate runs per file before route selection, so claiming the three
    // characters in *one* position is only safe if every other module mode
    // still refuses them.
    for module in [
        "contract mod probe { }",
        "extern fn f(x: i32) -> i32;",
        "fn helper() -> i32 { return 1 | 2; }",
    ] {
        let source = format!("{module}\n");
        if module.contains('|') {
            assert_eq!(codes(&source), ["Q038"], "for `{module}`");
        }
    }
    // …and a bitwise operator written in a *library* module, which has no
    // executable body at all, still refuses at the gate.
    assert_eq!(
        codes("fn helper() -> i32 { return 1 & 2; }\n"),
        ["Q038"],
        "a library module is refused by the same gate"
    );
}

#[test]
fn a_bitwise_operator_on_another_route_keeps_that_routes_refusal() {
    // Route composition, measured rather than assumed: the operator gate
    // declines to refuse `|` inside a condition on *every* route, and the
    // std-io route then refuses the `if` itself exactly as it did before.
    let with_bitwise = LINES.replace("    return;", "    if ((3 | 4) == 7) { }\n    return;");
    let without = LINES.replace("    return;", "    if (3 < 5) { }\n    return;");
    assert_ne!(with_bitwise, *LINES, "the substitution must have happened");
    assert_eq!(
        codes(&with_bitwise),
        codes(&without),
        "a bitwise operand must not change which refusal the std-io route gives"
    );
}

// ---------------------------------------------------------------------------
// 2. The precedence decision: declared where it agrees with C, refused where it
//    would not
// ---------------------------------------------------------------------------

#[test]
fn an_ungrouped_bitwise_comparison_operand_is_refused() {
    // **The card's headline refusal.** In C, `3 | 4 == 7` is `3 | (4 == 7)`;
    // here it could only be `(3 | 4) == 7`. Rather than silently redefine it,
    // the spelling is not a program.
    for condition in [
        "3 | 4 == 7",
        "7 == 3 | 4",
        "6 & 3 == 2",
        "6 ^ 3 == 5",
        "3 | 4 < 7",
    ] {
        assert_eq!(codes(&balanced(condition)), ["Q045"], "for `{condition}`");
    }
    let message = message(&balanced("3 | 4 == 7"));
    assert!(message.contains("must be parenthesized"), "{message}");
    assert!(
        message.contains('C'),
        "the refusal must name the divergence: {message}"
    );
}

#[test]
fn an_ungrouped_negation_operand_is_refused() {
    // The same rule in the other direction: C binds `!` **tighter** than every
    // comparison, so `!3 < 5` is `(!3) < 5` there and could only be `!(3 < 5)`
    // here.
    assert_eq!(codes(&balanced("!3 < 5")), ["Q045"]);
    let message = message(&balanced("!3 < 5"));
    assert!(message.contains("grouped condition"), "{message}");
    compile(&balanced("!(3 < 5)")).expect("the parenthesized spelling is the program");
}

#[test]
fn the_bitwise_ranking_is_cs_own_and_is_declared() {
    // `&` binds tighter than `^` binds tighter than `|` — C's order, so it is
    // declared rather than refused. Pinned against the intended grouping
    // (accepted, same emission) and against the other grouping (different
    // emission), so it cannot pass vacuously.
    let bare = compile(&balanced("(4 | 1 & 1) == 5")).expect("the bare spelling checks");
    let intended = compile(&balanced("(4 | (1 & 1)) == 5")).expect("the intended grouping checks");
    let other = compile(&balanced("((4 | 1) & 1) == 5")).expect("the other grouping checks");
    assert_eq!(bare.c_source, intended.c_source, "`|` must bind loosest");
    assert_ne!(
        bare.c_source, other.c_source,
        "the two groupings must be distinguishable, or the pin is vacuous"
    );
    // …and `^` sits between the two, rather than the table collapsing to one
    // level: `4 | 1 ^ 5` is `4 | (1 ^ 5)` = 4, not `(4 | 1) ^ 5` = 0.
    let xor_bare = compile(&balanced("(4 | 1 ^ 5) == 4")).expect("checks");
    let xor_intended = compile(&balanced("(4 | (1 ^ 5)) == 4")).expect("checks");
    let xor_other = compile(&balanced("((4 | 1) ^ 5) == 4")).expect("checks");
    assert_eq!(xor_bare.c_source, xor_intended.c_source);
    assert_ne!(xor_bare.c_source, xor_other.c_source);
}

#[test]
fn the_negation_binds_tighter_than_the_connectives() {
    // `!` over `&&` over `||`, again C's own order.
    let bare = compile(&balanced("!true && false || true")).expect("checks");
    let intended = compile(&balanced("((!true) && false) || true")).expect("checks");
    let other = compile(&balanced("!(true && (false || true))")).expect("checks");
    assert_eq!(bare.c_source, intended.c_source);
    assert_ne!(bare.c_source, other.c_source);
}

#[test]
fn a_comparison_is_not_associative() {
    // Deliberately no ranking: `1 < 2 < 3` compares a condition with an
    // integer, which this tier has no coercion for.
    assert_eq!(codes(&balanced("1 < 2 < 3")), ["Q045"]);
}

#[test]
fn the_two_sorts_do_not_coerce() {
    // An integer is not a condition — `if (mask & 4)` is a C idiom this
    // revision deliberately does not inherit — and a condition is not an
    // integer operand.
    for condition in ["3 | 4", "(3 | 4)", "3", "(1 & 4)"] {
        assert_eq!(codes(&balanced(condition)), ["Q045"], "for `{condition}`");
    }
    assert!(
        message(&balanced("(3 | 4)")).contains("no integer-to-Boolean coercion"),
        "the refusal must name the rule"
    );
    for condition in ["true | false", "true & false", "true ^ false"] {
        assert_eq!(codes(&balanced(condition)), ["Q045"], "for `{condition}`");
        assert!(
            message(&balanced(condition)).contains("no Boolean-to-integer coercion"),
            "for `{condition}`"
        );
    }
}

#[test]
fn a_pipeline_is_still_refused_rather_than_ranked() {
    // The `|>` versus `| >` longest-match constraint this card inherits. `|>`
    // wins by width, so a bitwise or followed by `>` must be spaced — and the
    // adjacency never produces a wrong *acceptance*, only a refusal, because
    // `>` has no left operand inside an integer expression.
    assert_eq!(codes(&balanced("1 |> 2")), ["Q045"]);
    assert_eq!(codes(&balanced("(1 |> 2) == 3")), ["Q045"]);
    // The spaced spelling is the bitwise operator followed by a comparison,
    // and it parses.
    compile(&balanced("(1 | 2) > 0")).expect("`| >` spaced is or-then-greater-than");
    // …and `||` is unmoved: it is still the logical connective, never `|`
    // twice.
    compile(&balanced("(1 | 2) == 3 || false")).expect("both bars coexist in one condition");
}

// ---------------------------------------------------------------------------
// 3. Negation is the arm swap
// ---------------------------------------------------------------------------

#[test]
fn a_negated_condition_emits_byte_identical_c_to_its_hand_swapped_twin() {
    // The card's load-bearing measurement for the negation half. Both members
    // are compiled under **one** file name so their `#line` directives agree,
    // and the fixtures are padded so `fn main` lands on the same line — the
    // constraint card `OR-AND-LOWERING` recorded.
    let negated = compile_named("twin.qcore", NEGATED).expect("the negated fixture checks");
    let twin = compile_named("twin.qcore", NEGATED_TWIN).expect("the swapped twin checks");
    assert_ne!(
        NEGATED, NEGATED_TWIN,
        "the two fixtures must differ, or the equality is a restatement"
    );
    assert_eq!(
        negated.c_source, twin.c_source,
        "a negation must emit exactly its hand-swapped twin"
    );
    assert_eq!(negated.reference_observation, twin.reference_observation);
}

#[test]
fn a_double_negation_is_the_identity_lowering() {
    let once = compile(&balanced("3 < 5")).expect("checks");
    let twice = compile(&balanced("!!(3 < 5)")).expect("checks");
    assert_eq!(
        once.c_source, twice.c_source,
        "`!!A` must be `A`'s own lowering"
    );
}

#[test]
fn a_negation_adds_no_operation_to_the_program() {
    // A negation contributes no statement, no acquisition and no release: the
    // emitted C carries exactly one acquisition however many `!` are written.
    let source = compile(&balanced("!(!(!(3 < 5)))")).expect("checks");
    let c = source.c_source.as_deref().unwrap();
    let body = c
        .split_once("static struct qcore_result qcore_fn_main(void)")
        .expect("the entry function is emitted")
        .1;
    assert_eq!(
        body.matches("q_acquire(").count(),
        1,
        "one acquisition, however many negations are written: {body}"
    );
    assert!(
        !body.contains("(!"),
        "no negation reaches the emitted entry function: {body}"
    );
    // …and an odd number of negations does swap the arms, so the count above is
    // not measuring an unchanged program. Measured with **distinguishable**
    // arms, for the reason the next test records.
    let negated = compile(&distinguishable("!(3 < 5)")).expect("checks");
    let plain = compile(&distinguishable("3 < 5")).expect("checks");
    assert_ne!(negated.c_source, plain.c_source);
}

/// Two arms that agree in custody state and differ in everything else: a check
/// leaves the ledger alone, so this is the smallest program in which an arm
/// swap is observable at all.
fn distinguishable(condition: &str) -> String {
    program(&format!(
        "  let a = qcore_acquire(1, true, Failure);\n  \
         if ({condition}) {{ qcore_check(true, Failure); drop a; }} else {{ drop a; }}\n  \
         return 0;"
    ))
}

#[test]
fn a_negation_over_identical_arms_is_unobservable() {
    // **A finding, measured rather than designed.** The negation is an arm
    // swap, so over two *identical* arms it emits byte-identical C however many
    // times it is written — the same shape as card `OR-AND-LOWERING`'s
    // associativity finding, and recorded for the same reason: it says exactly
    // where the construct is load-bearing. It is load-bearing precisely when
    // the arms differ, which is what the byte-equality twin pin measures.
    let none = compile(&balanced("3 < 5")).expect("checks");
    let once = compile(&balanced("!(3 < 5)")).expect("checks");
    let thrice = compile(&balanced("!(!(!(3 < 5)))")).expect("checks");
    assert_eq!(none.c_source, once.c_source);
    assert_eq!(none.c_source, thrice.c_source);
    // …and with distinguishable arms the swap is visible, so the equality above
    // is about the arms rather than about the negation being ignored.
    assert_ne!(
        compile(&distinguishable("3 < 5")).unwrap().c_source,
        compile(&distinguishable("!(3 < 5)")).unwrap().c_source
    );
}

#[test]
fn the_conditional_leak_refuses_through_a_negation() {
    // The custody join is unmoved: an arm swap is the same pair of arms in the
    // other order, and the join is agreement rather than widening.
    for condition in ["!true", "!(3 < 5)", "!(true && false)", "!(3 | 4) == 7"] {
        let source = program(&format!(
            "  let a = qcore_acquire(1, true, Failure);\n  \
             if ({condition}) {{ drop a; }}\n  return 0;"
        ));
        let codes = codes(&source);
        assert!(
            !codes.is_empty(),
            "`{condition}` must not launder the conditional leak"
        );
    }
    assert_eq!(
        codes(&program(
            "  let a = qcore_acquire(1, true, Failure);\n  \
             if (!true) { drop a; }\n  return 0;"
        )),
        ["Q046"]
    );
}

#[test]
fn moving_in_one_arm_and_releasing_in_the_other_refuses_through_a_negation() {
    assert_eq!(
        codes(&program(
            "  let a = qcore_acquire(1, true, Failure);\n  \
             if (!(3 < 5)) { drop a; } else { move a; }\n  return 0;"
        )),
        ["Q046"]
    );
}

// ---------------------------------------------------------------------------
// 4. Check-time constant bindings
// ---------------------------------------------------------------------------

#[test]
fn a_constant_binding_is_admitted_and_readable_in_a_condition() {
    compile(&program("  let n = 3;\n  return 0;")).expect("a constant binding alone checks");
    compile(&program(
        "  let n = 3;\n  let a = qcore_acquire(1, true, Failure);\n  \
         if (n == 3) { drop a; } else { drop a; }\n  return 0;",
    ))
    .expect("a condition may read a constant binding");
    compile(&program(
        "  let n = -7;\n  let a = qcore_acquire(1, true, Failure);\n  \
         if (n < 0) { drop a; } else { drop a; }\n  return 0;",
    ))
    .expect("a negative constant binding is admitted");
}

#[test]
fn a_constant_binding_is_its_value() {
    // The binding is **substituted**: the program that reads it and the
    // program that writes the literal out are the same emission, which is the
    // whole of the "check-time known" claim.
    let bound = compile(&program(
        "  let n = 3;\n  let a = qcore_acquire(1, true, Failure);\n  \
         if (n == 3) { drop a; } else { drop a; }\n  return 0;",
    ))
    .unwrap();
    let literal = compile(&program(
        "  let n = 3;\n  let a = qcore_acquire(1, true, Failure);\n  \
         if (3 == 3) { drop a; } else { drop a; }\n  return 0;",
    ))
    .unwrap();
    assert_eq!(bound.c_source, literal.c_source);
    assert_eq!(bound.reference_observation, literal.reference_observation);
}

#[test]
fn a_constant_binding_reaches_the_integer_tier() {
    compile(&balanced_with_binding("(mask & 4) == 4"))
        .expect("a binding is an operand of the integer tier");
    compile(&balanced_with_binding("(mask | 3) == 15")).expect("…on either side of a bit operator");
}

fn balanced_with_binding(condition: &str) -> String {
    program(&format!(
        "  let mask = 12;\n  let a = qcore_acquire(1, true, Failure);\n  \
         if ({condition}) {{ drop a; }} else {{ drop a; }}\n  return 0;"
    ))
}

#[test]
fn a_resource_binding_is_not_an_integer() {
    // **The custody rule, at the position it protects.** No custody-bearing
    // value gains a position anywhere in this tier: a resource name resolves
    // fine at name resolution and is refused by the tier, with a message that
    // says which rule it broke.
    let source = program(
        "  let a = qcore_acquire(1, true, Failure);\n  \
         if (a == 3) { drop a; } else { drop a; }\n  return 0;",
    );
    assert_eq!(codes(&source), ["Q045"]);
    let message = message(&source);
    assert!(message.contains("names a resource"), "{message}");
    assert!(message.contains("not an integer"), "{message}");
    // …and the same through the bitwise tier, which is the position that would
    // have put an identity into an expression for the first time.
    assert_eq!(
        codes(&program(
            "  let a = qcore_acquire(1, true, Failure);\n  \
             if ((a | 3) == 3) { drop a; } else { drop a; }\n  return 0;"
        )),
        ["Q045"]
    );
}

#[test]
fn an_unbound_name_in_a_condition_still_refuses_at_name_resolution() {
    // Card `IF-STATEMENT` finding 4, inherited: name resolution collects a
    // condition's names before the branch pass runs, so an unbound one is
    // `Q013` and not `Q045`. Fail-closed and pinned as measured.
    assert_eq!(codes(&balanced("nope == 3")), ["Q013"]);
}

#[test]
fn a_constant_binding_gains_no_other_position() {
    // The binding is readable **only** by the condition tier. Everything else
    // keeps the refusal it already had, so the widening is one position wide.
    for body in [
        "  let n = 3;\n  drop n;\n  return 0;",
        "  let n = 3;\n  move n;\n  return 0;",
        "  let n = 3;\n  qcore_check(n, Failure);\n  return 0;",
        "  let n = 3;\n  return n;",
        "  let n = 3;\n  n = 4;\n  return 0;",
    ] {
        assert!(
            !codes(&program(body)).is_empty(),
            "`{body}` must keep its refusal"
        );
    }
}

#[test]
fn a_constant_binding_inside_an_arm_refuses_on_its_own_ground() {
    // Written when card `IF-ARMS` was the arm subset's pending owner; that card
    // landed (without a `let`), so the refusal now states its ground rather
    // than naming a follow-on. The refusal itself is unchanged and deliberate.
    let source = program("  if (true) { let n = 3; }\n  return 0;");
    assert_eq!(codes(&source), ["Q044"]);
    assert!(
        message(&source).contains("no position after the join"),
        "the ground must be stated"
    );
}

#[test]
fn a_duplicate_constant_binding_refuses_at_name_resolution() {
    assert!(
        codes(&program("  let n = 3;\n  let n = 4;\n  return 0;")).contains(&"Q012".to_owned())
    );
    assert!(
        codes(&program(
            "  let a = qcore_acquire(1, true, Failure);\n  let a = 3;\n  drop a;\n  return 0;"
        ))
        .contains(&"Q012".to_owned()),
        "a constant may not shadow a resource binding either"
    );
}

#[test]
fn a_constant_binding_on_another_route_keeps_that_routes_route() {
    // The constant binding is the *sequential* route's, and route selection is
    // untouched: a printing module still reaches the std-io route, whose own
    // integer bindings are unchanged.
    let module = compile(LINES).expect("the std-io fixture still checks");
    assert_eq!(
        format!("{:?}", module.execution_profile),
        "StandardIoV1",
        "route selection must be unmoved"
    );
}

// ---------------------------------------------------------------------------
// 5. Emission, and what a program that uses none of this emits
// ---------------------------------------------------------------------------

#[test]
fn the_emitted_c_computes_the_bit_operations_itself() {
    // Card `IF-STATEMENT`'s design choice, extended: the emitted C carries the
    // operand tree rather than a folded literal, so the reference / CFG /
    // native agreement is a cross-check of the Rust evaluator against a C
    // compiler instead of a tautology.
    let module = compile(MASKED).expect("the masked fixture checks");
    let c = module.c_source.as_deref().unwrap();
    assert!(
        c.contains("(INT32_C(12) | INT32_C(3))"),
        "the bit operation must reach the C: {c}"
    );
    assert!(c.contains("(INT32_C(12) & INT32_C(4))"), "{c}");
    assert!(c.contains("(INT32_C(12) ^ INT32_C(5))"), "{c}");
    // …fully parenthesized, so the grouping this tier declared does not depend
    // on the C compiler's own precedence table.
    assert!(
        c.contains("if ((INT32_C(12) | INT32_C(3)) == INT32_C(15))"),
        "{c}"
    );
}

#[test]
fn a_program_that_writes_none_of_this_emits_byte_identical_c() {
    // The neutrality pin: a plain literal operand is emitted exactly as card
    // `IF-STATEMENT` emitted it, with no parentheses added.
    let module = compile(&balanced("3 < 5")).expect("checks");
    let c = module.c_source.as_deref().unwrap();
    assert!(
        c.contains("if (INT32_C(3) < INT32_C(5))"),
        "an unmoved comparison must be spelled exactly as before: {c}"
    );
    assert!(
        !c.contains("(INT32_C(3))"),
        "no operand parentheses may appear for a literal: {c}"
    );
}

// ---------------------------------------------------------------------------
// 6. Reference / native agreement
// ---------------------------------------------------------------------------

/// The sanitized cross-check. The emitted C computes three bit operations and
/// three comparisons itself, so agreement between the Rust evaluator and a C
/// compiler is a real comparison rather than a tautology. The leak check is a
/// **positive** control — the program allocates and releases.
///
/// Honest scope, carried from cards `IF-STATEMENT` and `OR-AND-LOWERING`: the
/// decided condition means one path executes. The other arms are compiled,
/// linked and reachable from the emitted `goto` table but are not entered at
/// run time; their evidence is static. `IF-RUNTIME-CONDITION` is the follow-on
/// that would move that boundary.
#[test]
fn the_native_run_of_a_masked_condition_matches_its_reference() {
    let module = compile(MASKED).unwrap();
    let instrumented = sequential_observation::instrument(module.c_source.as_deref().unwrap());
    let source = instrumented.as_str();
    let reference = module.reference_observation.as_ref().unwrap();

    #[cfg(target_os = "windows")]
    let run = windows_c::run(source, None, &[]);

    #[cfg(not(target_os = "windows"))]
    let run = {
        use std::process::Command;
        let directory = std::env::temp_dir().join(format!(
            "qcore-bitwise-{}-{}",
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
        let compile_status = Command::new(&compiler)
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
            compile_status.status.success(),
            "C compiler rejected the emitted bit operations:\n{}",
            String::from_utf8_lossy(&compile_status.stderr)
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
        let _ = std::fs::remove_dir_all(&directory);
        run
    };
    assert!(
        run.status.success(),
        "the emitted bit operations failed under ASan/UBSan/LSan:\n{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let stdout = String::from_utf8(run.stdout).unwrap();
    let abi = qcore_compiler::ErrorAbi::new(["Failure".to_owned()]).unwrap();
    let observed = sequential_observation::parse(&stdout, &abi)
        .unwrap_or_else(|error| panic!("malformed native observation: {error:?}\n{stdout}"));
    assert_eq!(
        &observed, reference,
        "the compiled program must observe what the reference interpreter does"
    );
}

// ---------------------------------------------------------------------------
// 7. The tier's closedness, as one list
// ---------------------------------------------------------------------------

#[test]
fn the_tier_is_closed_exactly_here() {
    // Everything this card admits, and everything it does not, in one place —
    // so the next widening has to move a test rather than slip past one.
    let admitted: BTreeMap<&str, &str> = BTreeMap::from([
        ("literal", "true"),
        ("comparison", "3 < 5"),
        ("connectives", "3 < 5 || 2 == 2"),
        ("negation", "!(3 < 5)"),
        ("bitwise or", "(3 | 4) == 7"),
        ("bitwise xor", "(3 ^ 4) == 7"),
        ("bitwise and", "(3 & 4) == 0"),
        ("grouping", "((3 | 4) == 7) && !false"),
    ]);
    for (name, condition) in &admitted {
        compile(&balanced(condition))
            .unwrap_or_else(|errors| panic!("{name} (`{condition}`) must be admitted: {errors:?}"));
    }
    let refused = [
        "3 | 4",         // an integer is not a condition
        "3 | 4 == 7",    // ungrouped bitwise comparison operand
        "!3 < 5",        // ungrouped negation operand
        "true | false",  // a condition is not an integer
        "1 < 2 < 3",     // a comparison is not associative
        "1 |> 2",        // a pipeline is not in the tier
        "~3 == 0",       // complement: not in this card
        "(3 + 4) == 7",  // arithmetic: not in this card
        "(3 << 1) == 6", // shift: not in this card
        "f() == 3",      // a call is not in the tier
    ];
    for condition in refused {
        assert!(
            !codes(&balanced(condition)).is_empty(),
            "`{condition}` must stay outside the tier"
        );
    }
}

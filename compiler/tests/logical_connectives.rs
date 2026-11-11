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

//! The logical connectives `||` and `&&` (card `OR-AND-LOWERING`).
//!
//! Two things land here and they are independent of each other.
//!
//! **The token retable.** By user direction, logical or is `||` and logical and
//! is `&&` — both double-character, both short-circuit — and the single
//! characters `|`, `&` and `^` are freed and reserved for future bitwise
//! operations. This *inverts* part of card `PIPE-OPERATOR`, which had decided
//! the single `|` was logical or and refused `||` as a misspelling of it. The
//! moved pins are listed in `docs/reports/wave19-or-and-lowering/README.md`;
//! the load-bearing invariant across the move is that **no program's verdict
//! changed** — every byte that refused before refuses now.
//!
//! **The lowering.** A connective in an `if` condition is a desugaring onto
//! card `IF-STATEMENT`'s `source.branch.v1`, with no new semantic rule: the
//! arms are lowered once and the connective duplicates their *operation
//! vectors* into nested branches. `qcore-core` is byte-unchanged by this card,
//! and the load-bearing measurement is that `a || b` emits **byte-identical C**
//! to its hand-lowered `else if` twin.
//!
//! Short-circuit is structural, not asserted: the right operand is the
//! condition of a branch nested inside an arm of the left operand's branch, so
//! it is reached on exactly one path. Because every leaf of the tier is closed
//! over literals, short-circuit is not *observable* as a side-effect
//! difference — that is stated plainly here rather than smoothed over, and the
//! follow-on that would make it observable is `IF-RUNTIME-CONDITION`.
use std::collections::BTreeMap;

use qcore_compiler::{CompileOptions, SourceFile, compile_files};

#[path = "../../tests/support/sequential_observation.rs"]
mod sequential_observation;

#[cfg(target_os = "windows")]
#[path = "../../tests/support/windows_c.rs"]
mod windows_c;

const OR_FIXTURE: &str = include_str!("fixtures/connectives/short_circuit_or.qr");
const OR_TWIN: &str = include_str!("fixtures/connectives/short_circuit_or_twin.qr");
const LINES: &str = include_str!("fixtures/std_modules/lines.qr");

/// Both members of a byte-equality pair are compiled under **one** file name,
/// because the emitted C carries a `#line` naming it. The two fixtures are
/// padded to put `fn main` on the same line for the same reason.
const UNIT: &str = "connectives.qcore";

fn compile_with(
    source: &str,
    contracts: Option<BTreeMap<String, String>>,
) -> Result<qcore_compiler::CompiledModule, Vec<qcore_compiler::FileDiagnostic>> {
    compile_files(
        &[SourceFile::new(UNIT, source)],
        &CompileOptions {
            entry: None,
            extern_contracts: contracts,
            native_libraries: None,
            native_library_headers: None,
        },
    )
}

fn compile(
    source: &str,
) -> Result<qcore_compiler::CompiledModule, Vec<qcore_compiler::FileDiagnostic>> {
    compile_with(source, None)
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

fn emitted(source: &str) -> String {
    compile(source)
        .expect("this fixture must check")
        .c_source
        .expect("an executable module emits C")
}

/// A branch whose two arms agree, so the join is admitted and the *condition*
/// is what the test is about.
fn balanced(condition: &str) -> String {
    format!(
        "error Failure {{ Failed; }}\n\
         fn main() -> i32 {{\n    \
             let a = qcore_acquire(1, true, Failure);\n    \
             if ({condition}) {{ drop a; }} else {{ drop a; }}\n    \
             return 0;\n\
         }}\n"
    )
}

// ---------------------------------------------------------------------------
// 1. The lowering is a desugaring: byte-identical C against a hand twin
// ---------------------------------------------------------------------------

#[test]
fn an_or_condition_emits_byte_identical_c_to_its_hand_lowered_twin() {
    // The card's load-bearing pin. `A || B` is *the same program* as the
    // `else if` chain a reader would have written, not a program that behaves
    // like it: the emitted C11 agrees byte for byte, including the `#line`
    // directives, and so does the reference observation.
    let piped = compile(OR_FIXTURE).expect("the or-condition checks");
    let twin = compile(OR_TWIN).expect("the hand-lowered twin checks");
    assert_eq!(
        piped.c_source, twin.c_source,
        "`A || B` must emit the C its hand-lowered `else if` twin emits"
    );
    assert_eq!(piped.reference_observation, twin.reference_observation);
    // …and the source programs are genuinely different, so the equality above
    // is a statement about the lowering rather than a restatement.
    assert_ne!(OR_FIXTURE, OR_TWIN);
}

#[test]
fn an_and_condition_emits_byte_identical_c_to_its_nested_twin() {
    // `A && B` nests the *other* way: the right operand's branch sits in the
    // left's **then** arm, and the else arm is duplicated.
    let connective = balanced("3 < 5 && 2 == 2");
    let hand = "error Failure { Failed; }\n\
                fn main() -> i32 {\n    \
                    let a = qcore_acquire(1, true, Failure);\n    \
                    if (3 < 5) { if (2 == 2) { drop a; } else { drop a; } } else { drop a; }\n    \
                    return 0;\n\
                }\n";
    assert_eq!(
        emitted(&connective),
        emitted(hand),
        "`A && B` must emit the C its hand-lowered nested twin emits"
    );
}

#[test]
fn a_connective_free_condition_is_byte_unmoved() {
    // The widened tier must be a strict extension: a condition with no
    // connective lowers exactly as card `IF-STATEMENT` lowered it, which is
    // what makes the retable safe for every shipped program.
    let single = emitted(&balanced("3 < 5"));
    assert_eq!(single.matches("if (INT32_C(3) < INT32_C(5))").count(), 1);
    assert_eq!(
        single.matches("q_release(&q_res_1, UINT64_C(1));").count(),
        2,
        "one release per arm and no third: {single}"
    );
}

// ---------------------------------------------------------------------------
// 2. Short-circuit, structurally
// ---------------------------------------------------------------------------

#[test]
fn the_right_operand_is_the_condition_of_a_nested_branch() {
    // What "short-circuit" means here, made checkable: the emitted C contains
    // two separate `if` statements, never one C-level `||`, and the second
    // comparison is inside a block the first branch reaches on one path only.
    let or = emitted(&balanced("3 < 5 || 2 == 2"));
    assert!(
        !or.contains("||") && !or.contains("&&"),
        "the connective must be gone from the emitted C:\n{or}"
    );
    assert!(or.contains("if (INT32_C(3) < INT32_C(5))"));
    assert!(or.contains("if (INT32_C(2) == INT32_C(2))"));
    let and = emitted(&balanced("3 < 5 && 2 == 2"));
    assert!(!and.contains("&&"));
    // The two spellings differ: or nests into the else arm, and into the then
    // arm, so they are not the same program.
    assert_ne!(or, and);
}

#[test]
fn a_connective_adds_no_operation_to_the_program() {
    // A connective duplicates the arms' operations into nested branches; it
    // contributes no acquisition, no release and no check of its own. Measured
    // as: the number of release sites is the number of *leaves*, and the arms
    // are the same two arms.
    for (condition, leaves) in [
        ("3 < 5", 2usize),
        ("3 < 5 || 2 == 2", 3),
        ("3 < 5 && 2 == 2", 3),
        ("3 < 5 || 2 == 2 || 1 < 9", 4),
    ] {
        let c = emitted(&balanced(condition));
        assert_eq!(
            c.matches("q_release(&q_res_1, UINT64_C(1));").count(),
            leaves,
            "`{condition}` must emit one release per arm of the nested branch:\n{c}"
        );
        assert_eq!(
            c.matches("q_acquire(&q_res_1").count(),
            1,
            "a connective must not duplicate the acquisition"
        );
    }
}

// ---------------------------------------------------------------------------
// 3. Precedence, declared and pinned
// ---------------------------------------------------------------------------

#[test]
fn and_binds_tighter_than_or() {
    // `A || B && C` is `A || (B && C)`. Pinned against the fully parenthesized
    // spelling of the intended grouping, and asserted *different* from the
    // other grouping so the test cannot pass vacuously.
    let bare = emitted(&balanced("1 < 2 || 3 < 4 && 5 < 6"));
    let intended = emitted(&balanced("1 < 2 || (3 < 4 && 5 < 6)"));
    let other = emitted(&balanced("(1 < 2 || 3 < 4) && 5 < 6"));
    assert_eq!(bare, intended, "`&&` must bind tighter than `||`");
    assert_ne!(
        bare, other,
        "the two groupings must be distinguishable, or the pin says nothing"
    );
}

#[test]
fn a_homogeneous_chain_lowers_the_same_under_either_grouping() {
    // **A finding, measured rather than designed.** The parser declares both
    // connectives left-associative, and for a *homogeneous* chain that choice
    // turns out to be unobservable: the lowering makes `||` and `&&`
    // associative on the nose, so `(A || B) || C` and `A || (B || C)` are the
    // same nested branch and emit the same C. The declaration is therefore
    // load-bearing only where the two connectives *mix*, which is what
    // `and_binds_tighter_than_or` pins. Recorded here so a future widening —
    // an effectful or a run-time condition — has to re-measure it rather than
    // inherit it, since associativity would stop being free the moment a leaf
    // could do something.
    for (bare, left, right) in [
        (
            "1 < 2 || 3 < 4 || 5 < 6",
            "(1 < 2 || 3 < 4) || 5 < 6",
            "1 < 2 || (3 < 4 || 5 < 6)",
        ),
        (
            "1 < 2 && 3 < 4 && 5 < 6",
            "(1 < 2 && 3 < 4) && 5 < 6",
            "1 < 2 && (3 < 4 && 5 < 6)",
        ),
    ] {
        assert_eq!(emitted(&balanced(bare)), emitted(&balanced(left)));
        assert_eq!(
            emitted(&balanced(bare)),
            emitted(&balanced(right)),
            "`{bare}` lowers associatively, so the grouping is unobservable"
        );
    }
}

#[test]
fn parentheses_group_and_nest() {
    for condition in [
        "(true)",
        "((true))",
        "(3 < 5)",
        "(true || false) && (2 == 2)",
        "((1 < 2 && 3 < 4) || (5 < 6 && 7 < 8))",
    ] {
        compile(&balanced(condition))
            .unwrap_or_else(|errors| panic!("`{condition}` must be admitted: {errors:?}"));
    }
}

#[test]
fn a_pipeline_in_a_condition_is_refused_rather_than_given_a_precedence() {
    // The decision recorded rather than inherited: `|>` and `||` are never
    // given a relative precedence, because `|>` is not in the condition tier at
    // all. The pipe desugarer only records a `|>` at bracket depth 0, so one
    // inside a condition survives into the tokens and the tier refuses it —
    // fail-closed, and pinned so a future widening has to decide it.
    for condition in ["1 |> 2", "true || 1 |> 2", "1 |> 2 || true"] {
        assert_eq!(
            codes(&balanced(condition)),
            ["Q045"],
            "`{condition}` must be refused by the condition tier"
        );
    }
}

// ---------------------------------------------------------------------------
// 4. The custody join survives every connective level
// ---------------------------------------------------------------------------

fn leaking(condition: &str) -> String {
    format!(
        "error Failure {{ Failed; }}\n\
         fn main() -> i32 {{\n    \
             let a = qcore_acquire(1, true, Failure);\n    \
             if ({condition}) {{ drop a; }}\n    \
             return 0;\n\
         }}\n"
    )
}

#[test]
fn the_conditional_leak_refuses_through_every_connective() {
    // The heart of card `IF-STATEMENT`, re-measured through the new tier: a
    // connective must not be able to launder a custody disagreement past the
    // join. Each level runs the same four-model join, so the refusal is the
    // same refusal at the same code.
    for condition in [
        "true",
        "true || false",
        "true && false",
        "true || false || true",
        "(true || false) && (2 == 2)",
    ] {
        assert_eq!(
            codes(&leaking(condition)),
            ["Q046"],
            "`{condition}` must still refuse the conditional leak"
        );
    }
}

#[test]
fn one_arm_moving_what_the_other_releases_still_refuses() {
    let source = "error Failure { Failed; }\n\
                  fn sink(r: Resource use one) -> i32 { return 0; }\n\
                  fn main() -> i32 {\n    \
                      let a = qcore_acquire(1, true, Failure);\n    \
                      if (true || false) { drop a; } else { move a; }\n    \
                      return 0;\n\
                  }\n";
    let codes = codes(source);
    assert!(
        !codes.is_empty(),
        "moving on one arm and releasing on the other must refuse through a connective"
    );
}

#[test]
fn an_acquisition_inside_an_arm_keeps_its_pre_existing_refusal() {
    // Card `IF-STATEMENT` measured this as refused by the *pre-existing*
    // ownership ledger join (`Q103`, follow-on `IF-ARM-ACQUIRE`) and
    // deliberately did not weaken it. A connective must not weaken it either —
    // and this is exactly where duplicating the arms' operation vectors could
    // have gone wrong.
    let source = "error Failure { Failed; }\n\
                  fn main() -> i32 {\n    \
                      if (true || false) { let b = qcore_acquire(2, true, Failure); drop b; }\n    \
                      return 0;\n\
                  }\n";
    assert!(
        !codes(source).is_empty(),
        "an acquisition inside an arm must stay refused through a connective"
    );
}

#[test]
fn an_early_return_inside_an_arm_still_refuses_through_a_connective() {
    assert_eq!(
        codes("fn main() -> i32 { if (true || false) { return 1; } }"),
        ["Q044"]
    );
}

// ---------------------------------------------------------------------------
// 5. Condition position only
// ---------------------------------------------------------------------------

#[test]
fn a_connective_outside_a_condition_refuses() {
    // The admission is positional, and the position is decided by the *parser*
    // — the gate reads the `if` condition spans the parser recorded, not "there
    // is an `if` somewhere to the left". Nothing custody-bearing gains a
    // position: there is still no Boolean expression tier anywhere.
    for body in [
        "let x = true || false;",
        "return 1 || 2;",
        "drop a || b;",
        "while (true || false) { }",
    ] {
        let source = format!(
            "error Failure {{ Failed; }}\nfn main() -> i32 {{\n    {body}\n    return 0;\n}}\n"
        );
        let codes = codes(&source);
        assert!(
            codes.contains(&"Q038".to_owned()),
            "`{body}` must refuse at the operator gate: {codes:?}"
        );
    }
}

#[test]
fn the_gate_message_names_the_admitted_position() {
    let source = "error Failure { Failed; }\nfn main() -> i32 {\n    let x = true || false;\n    return 0;\n}\n";
    let message = message(source);
    assert!(message.contains("`||` is the logical-or operator"));
    assert!(message.contains("`if` condition"));
}

#[test]
fn a_connective_in_a_contract_or_library_module_refuses() {
    // Claiming `||` as an admitted spelling *somewhere* is only safe if the
    // refusal is reached on every route, including the non-executable module
    // modes. Card `PIPE-OPERATOR` pinned this for `|` and it is re-pinned here
    // for the retabled pair.
    for mode in ["module contract;", "module library;", "module executable;"] {
        for operator in ["||", "&&"] {
            let source =
                format!("{mode}\nerror E {{ X; }}\nfn f() -> i32 {{ return 0 {operator} 1; }}\n");
            assert_eq!(codes(&source), ["Q038"], "for `{mode}` and `{operator}`");
        }
    }
}

#[test]
fn a_connective_on_another_route_keeps_that_routes_refusal() {
    // `source.branch.v1` is admitted on the sequential route only, so an `if`
    // on the `std.io` route refuses at that route's own nested-block gate — and
    // a connective inside its condition does not change which refusal answers.
    let with_connective = LINES.replace(
        "    print dec total;",
        "    if (true || false) { print dec total; }",
    );
    let without = LINES.replace("    print dec total;", "    if (true) { print dec total; }");
    assert_ne!(with_connective, LINES, "the fixture edit must have applied");
    assert_ne!(without, LINES, "the control edit must have applied");
    assert_eq!(
        codes(&with_connective),
        codes(&without),
        "a connective must not change which route claims the module"
    );
}

// ---------------------------------------------------------------------------
// 6. The token retable
// ---------------------------------------------------------------------------

#[test]
fn the_three_single_byte_operators_are_spelled_only_in_a_condition() {
    // **Deliberate pin update, card `BITWISE-CONDITIONS`.** This card reserved
    // `|`, `&` and `^` and pinned that they meant nothing anywhere; its named
    // follow-on `BITWISE-OPERATORS` spends the reservation, so the three now
    // have a meaning — in exactly one position, a comparison operand inside an
    // `if` condition. What survives unchanged is the half that was load-bearing
    // here: outside that position all three still refuse, at the same `Q038`,
    // from the same gate, and none of them is a logical operator.
    for (operator, expected) in [
        ("|", "`|` is the bitwise-or operator"),
        ("&", "`&` is the bitwise-and operator"),
        ("^", "`^` is the bitwise-xor operator"),
    ] {
        let source = format!(
            "error Failure {{ Failed; }}\nfn main() -> i32 {{\n    return 1 {operator} 2;\n}}\n"
        );
        assert_eq!(codes(&source), ["Q038"], "for `{operator}`");
        let message = message(&source);
        assert!(message.contains(expected), "for `{operator}`: {message}");
        assert!(
            message.contains("`if` condition"),
            "for `{operator}`: the one admitted position must be named"
        );
    }
}

#[test]
fn a_single_byte_operator_in_a_condition_is_the_branch_pass_to_decide() {
    // **Deliberate pin update, card `BITWISE-CONDITIONS`.** Card
    // `OR-AND-LOWERING` pinned that the reservation was *not* positional —
    // `true | false` refused at `Q038` inside a condition too. Now it is
    // positional: the operator gate declines to refuse the three characters in
    // a condition and the branch pass decides them, exactly as it already did
    // for `||` and `&&`. Both halves are asserted so the move is visible: the
    // code changes from `Q038` to `Q045`, and **the refusal survives** —
    // `true | false` is still not a program, because a condition is not an
    // integer.
    for operator in ["|", "&", "^"] {
        let source = balanced(&format!("true {operator} false"));
        assert_eq!(codes(&source), ["Q045"], "for `{operator}`");
        assert!(
            message(&source).contains("no Boolean-to-integer coercion"),
            "for `{operator}`: the refusal must name the sort rule"
        );
    }
}

#[test]
fn the_quant_index_alias_is_untouched_by_the_retable() {
    // The one place a retable of the vertical-bar family could reach a shipped
    // program: `[|` and `|]` are matched at their own two bytes, so `||`
    // becoming one token cannot split or merge them. Measured on a real
    // quant-index program rather than on tokens alone.
    let source = "dimension Bytes : Idx + Ordered; \
                  quant axis memory : credit<Bytes>; \
                  extern fn f(q: quant[|n where n := 5|] use one, tail: i32) -> unit; \
                  fn main() -> i32 { return 0; }";
    // Card `W48-QR-SCALAR-TYPES-R3` (`W48-INERT-EXTERN-DECLARATION-TYPES`):
    // an uncalled `extern fn` is now read, and a `quant[| … |]` parameter is
    // no type an extern declaration can state, so the module refuses at
    // `Q020` — the same correction as `remaining-v04`'s P08 and P10, whose
    // shape this is. What this test measures is unchanged: the alias lexes,
    // parses and elaborates, so the one refusal is that declaration check,
    // after every earlier pass admitted the program.
    let refused = compile(source).expect_err("an unread quant parameter is refused");
    assert_eq!(
        refused.iter().map(|d| d.code.as_str()).collect::<Vec<_>>(),
        ["Q020"],
        "{refused:?}"
    );
    assert!(
        refused[0]
            .message
            .contains("parameter `q` of `extern fn f`")
    );
    // …and the two coexist in one file: the bracket alias and an or-condition
    // in the same compilation input, so the retable is exercised beside the
    // delimiters it could have collided with rather than only away from them.
    let together = source.replace(
        "fn main() -> i32 { return 0; }",
        "error Failure { Failed; } fn main() -> i32 { \
         let a = qcore_acquire(1, true, Failure); \
         if (3 < 5 || 2 == 2) { drop a; } else { drop a; } return 0; }",
    );
    let refused = compile(&together).expect_err("the same declaration check refuses");
    assert_eq!(
        refused.iter().map(|d| d.code.as_str()).collect::<Vec<_>>(),
        ["Q020"],
        "a bracket alias and an or-condition coexist up to the declaration check: {refused:?}"
    );
}

#[test]
fn the_pipe_operator_is_untouched_by_the_retable() {
    // `|>` still wins by width over `||` and `|`, so a shipped pipeline is
    // unmoved. The pipe suite pins the pipelines themselves; this pins the
    // adjacency that the retable could have broken.
    let source = "error Failure { Failed; }\n\
                  fn main() -> i32 {\n    \
                      let a = qcore_acquire(1, true, Failure);\n    \
                      drop a;\n    \
                      return 0;\n\
                  }\n";
    compile(source).expect("the control program checks");
    // `a ||> b` is the or-operator then `>`, never `|` then `|>`; in a
    // condition that is a malformed comparison, refused by the tier.
    assert_eq!(codes(&balanced("true ||> false")), ["Q045"]);
}

// ---------------------------------------------------------------------------
// 7. Reference / native agreement
// ---------------------------------------------------------------------------

/// The sanitized cross-check, on the repo's byte-compare precedent: the emitted
/// C computes both comparisons itself, so agreement between the Rust evaluator
/// and a C compiler is a real comparison rather than a tautology. The leak
/// check is a **positive** control — the program allocates and releases.
///
/// Honest scope, carried from card `IF-STATEMENT`: the decided condition means
/// one path executes. The other arms are compiled, linked and reachable from
/// the emitted `goto` table but are not entered at run time; their evidence is
/// static. `IF-RUNTIME-CONDITION` is the follow-on that would move that.
#[test]
fn the_native_run_of_a_connective_matches_its_reference() {
    let module = compile(OR_FIXTURE).unwrap();
    let instrumented = sequential_observation::instrument(module.c_source.as_deref().unwrap());
    let source = instrumented.as_str();
    let reference = module.reference_observation.as_ref().unwrap();

    #[cfg(target_os = "windows")]
    let run = windows_c::run(source, None, &[]);

    #[cfg(not(target_os = "windows"))]
    let run = {
        use std::process::Command;
        let directory = std::env::temp_dir().join(format!(
            "qcore-connectives-{}-{}",
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
            "C compiler rejected the emitted connective:\n{}",
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
        "the emitted connective failed under ASan/UBSan/LSan:\n{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let stdout = String::from_utf8(run.stdout).unwrap();
    let abi = qcore_compiler::ErrorAbi::new(["Failure".to_owned()]).unwrap();
    let observed = sequential_observation::parse(&stdout, &abi)
        .unwrap_or_else(|error| panic!("malformed native observation: {error:?}\n{stdout}"));
    assert_eq!(
        &observed, reference,
        "the compiled program must observe what the reference does"
    );
}

// ---------------------------------------------------------------------------
// 8. Neutrality
// ---------------------------------------------------------------------------

#[test]
fn no_shipped_program_changes_meaning() {
    // The retable adds two tokens (`||`, `^`) and re-points one (`|`). None of
    // the three appears in any shipped `.qcore` source file — the census is in
    // the card report — so the only way a shipped program could move is through
    // the lexer's adjacency rules, and those are pinned in
    // `crates/qcore-frontend/tests/unit/lexer.rs`. Compile a shipped fixture
    // through the changed gate as the end-to-end control.
    compile(LINES).expect("a shipped std.io fixture must still check");
}

#[test]
fn an_else_if_chain_composes_with_connectives() {
    for condition in [
        "3 < 5 || 2 == 2",
        "3 < 5 && 2 == 2",
        "(3 < 5 || 2 == 2) && 1 < 9",
    ] {
        let source = format!(
            "error Failure {{ Failed; }}\n\
             fn main() -> i32 {{\n    \
                 let a = qcore_acquire(1, true, Failure);\n    \
                 if (1 > 9) {{ drop a; }} else if ({condition}) {{ drop a; }} else {{ drop a; }}\n    \
                 return 0;\n\
             }}\n"
        );
        compile(&source)
            .unwrap_or_else(|errors| panic!("`else if ({condition})` must check: {errors:?}"));
    }
}

#[test]
fn a_nested_branch_inside_an_arm_composes_with_connectives() {
    let source = "error Failure { Failed; }\n\
                  fn main() -> i32 {\n    \
                      let a = qcore_acquire(1, true, Failure);\n    \
                      if (true || false) {\n        \
                          if (2 == 2 && 3 < 4) { drop a; } else { drop a; }\n    \
                      } else { drop a; }\n    \
                      return 0;\n\
                  }\n";
    compile(source).expect("a connective nests inside a branch arm");
}

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

//! The source branching construct `if (COND) { … } else { … }` (card
//! `IF-STATEMENT`, `source.branch.v1`).
//!
//! The load-bearing property here is the **custody join**: a branch is admitted
//! only when its two arms reach the join in agreeing custody state, and every
//! disagreement — the conditional leak, the conditional double-release, one arm
//! moving what the other releases — refuses. Each of those refusals comes from
//! a checker this card did not edit: the ownership, typed-error and
//! resource-flow models have carried branch joins since their own cycles, and
//! the one thing this card added is `SourceOp::Branch`, the branch constructor
//! the source lowering lacked.
//!
//! Two further absences are made measurable rather than asserted: a program
//! that does not use the construct emits **byte-identical** C, and every route
//! other than the sequential one keeps the refusal it already had.
use std::collections::BTreeMap;

use qcore_compiler::{CompileOptions, SourceFile, compile_files};

#[path = "../../tests/support/sequential_observation.rs"]
mod sequential_observation;

#[cfg(target_os = "windows")]
#[path = "../../tests/support/windows_c.rs"]
mod windows_c;

const CONDITIONAL_RELEASE: &str = include_str!("fixtures/branch/conditional_release.qr");
const UNBRANCHED_TWIN: &str = include_str!("fixtures/branch/unbranched_twin.qr");
const LINES: &str = include_str!("fixtures/std_modules/lines.qr");
const ROUND_TRIP: &str = include_str!("fixtures/extern_calls/round_trip.qr");
const EXTERN_CONTRACTS: &str = include_str!("fixtures/extern_calls/contracts/buffer_api.qv");

fn compile_with(
    source: &str,
    contracts: Option<BTreeMap<String, String>>,
) -> Result<qcore_compiler::CompiledModule, Vec<qcore_compiler::FileDiagnostic>> {
    compile_files(
        &[SourceFile::new("branch.qcore", source)],
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

fn program(body: &str) -> String {
    format!("error Failure {{ Failed; }}\nfn main() -> i32 {{\n{body}\n}}\n")
}

// ---------------------------------------------------------------------------
// 1. The construct itself
// ---------------------------------------------------------------------------

#[test]
fn a_branch_whose_arms_agree_is_admitted() {
    let module = compile(CONDITIONAL_RELEASE).expect("both arms release, so the join agrees");
    assert_eq!(module.entry.as_deref(), Some("main"));
}

#[test]
fn both_arms_are_emitted_as_real_basic_blocks() {
    let module = compile(CONDITIONAL_RELEASE).unwrap();
    let c = module.c_source.expect("an executable module emits C");
    // The decided condition is not folded away: the emitted program computes
    // the comparison itself, which is what makes the reference/CFG/native
    // agreement a cross-check rather than a tautology.
    assert!(
        c.contains("if (INT32_C(3) < INT32_C(5))"),
        "the emitted C must compute the condition:\n{c}"
    );
    // Two release sites, one per arm: the arm the condition does not take is
    // emitted too. (The acquisition's own cleanup edge is empty — nothing is
    // live before the acquisition it guards.)
    assert_eq!(
        c.matches("q_release(&q_res_1, UINT64_C(1));").count(),
        2,
        "one release per arm, both emitted:\n{c}"
    );
}

#[test]
fn a_branch_observes_what_its_hand_written_twin_does() {
    let branched = compile(CONDITIONAL_RELEASE).unwrap();
    let twin = compile(UNBRANCHED_TWIN).unwrap();
    assert_eq!(
        branched.reference_observation, twin.reference_observation,
        "the decided condition makes the observation single-valued"
    );
    // …and the two are genuinely different programs, so the agreement above is
    // a statement about the lowering rather than a restatement.
    assert_ne!(branched.c_source, twin.c_source);
}

#[test]
fn a_literal_condition_is_admitted_in_both_polarities() {
    for condition in ["true", "false"] {
        compile(&program(&format!(
            "  let a = qcore_acquire(1, true, Failure);\n  \
             if ({condition}) {{ drop a; }} else {{ drop a; }}\n  return 0;"
        )))
        .unwrap_or_else(|errors| panic!("`{condition}` must be admitted: {errors:?}"));
    }
}

#[test]
fn every_comparison_in_the_tier_is_admitted() {
    for operator in ["==", "!=", "<", "<=", ">", ">="] {
        let source = program(&format!(
            "  if (3 {operator} 5) {{ }} else {{ }}\n  return 0;"
        ));
        compile(&source)
            .unwrap_or_else(|errors| panic!("`{operator}` must lex and parse: {errors:?}"));
    }
}

#[test]
fn a_negative_literal_operand_is_admitted() {
    compile(&program("  if (-7 < 0) { } else { }\n  return 0;")).unwrap();
}

#[test]
fn an_else_if_chain_is_a_right_nested_branch() {
    let module = compile(&program(
        "  let a = qcore_acquire(1, true, Failure);\n  \
         if (3 > 5) { drop a; } else if (2 == 2) { drop a; } else { drop a; }\n  return 0;",
    ))
    .expect("every arm of the chain releases, so every join agrees");
    let c = module.c_source.unwrap();
    // Two conditions, so two branch terminators.
    assert!(c.contains("if (INT32_C(3) > INT32_C(5))"), "{c}");
    assert!(c.contains("if (INT32_C(2) == INT32_C(2))"), "{c}");
}

#[test]
fn a_branch_nests_inside_a_branch_arm() {
    // Nesting is the shape `||` and `&&` lower onto, so it is admitted rather
    // than refused. (This card implements neither operator; the bare `|` and
    // `&` characters stay reserved for future binary/bitwise operations.)
    compile(&program(
        "  let a = qcore_acquire(1, true, Failure);\n  \
         if (1 < 2) { if (3 >= 3) { drop a; } else { drop a; } } else { drop a; }\n  return 0;",
    ))
    .expect("a nested branch whose every path releases is admitted");
}

#[test]
fn a_branch_is_admitted_in_a_non_entry_function() {
    // A different path through the driver: a non-entry declaration's semantics
    // reach `check_whole_program`'s summary unfolding rather than emission, so
    // the branch has to survive that too.
    let module = compile(
        "error Failure { Failed; }\n\
         fn leaf() -> i32 { let a = qcore_acquire(7, true, Failure); \
           if (1 < 2) { drop a; } else { drop a; } return 0; }\n\
         fn main() -> i32 { if (2 > 1) { } else { } return 0; }",
    )
    .expect("a branch checks in a leaf function as well as in the entry");
    assert_eq!(module.checked.function_count(), 2);
    assert_eq!(module.entry.as_deref(), Some("main"));
}

#[test]
fn an_if_without_an_else_is_admitted_when_the_arm_is_custody_neutral() {
    compile(&program("  if (1 < 2) { }\n  return 0;")).expect("the missing arm is the empty arm");
}

// ---------------------------------------------------------------------------
// 2. The custody join — the heart of the card
// ---------------------------------------------------------------------------

#[test]
fn a_conditional_leak_refuses() {
    // The archetype: released on one arm, leaked on the other.
    assert_eq!(
        codes(&program(
            "  let a = qcore_acquire(1, true, Failure);\n  \
             if (true) { drop a; }\n  return 0;"
        )),
        vec!["Q046"]
    );
}

#[test]
fn a_conditional_leak_refuses_whichever_arm_leaks() {
    // Not an accident of which arm the decided condition takes: the *untaken*
    // arm is validated in full.
    for (then_arm, else_arm) in [("drop a;", ""), ("", "drop a;")] {
        assert_eq!(
            codes(&program(&format!(
                "  let a = qcore_acquire(1, true, Failure);\n  \
                 if (true) {{ {then_arm} }} else {{ {else_arm} }}\n  return 0;"
            ))),
            vec!["Q046"],
            "arms `{then_arm}` / `{else_arm}` must not join"
        );
    }
}

#[test]
fn one_arm_moving_what_the_other_releases_refuses() {
    // Both arms leave the same live set; only the transferred set differs. A
    // join that compared live resources alone would admit this.
    assert_eq!(
        codes(&program(
            "  let a = qcore_acquire(1, true, Failure);\n  \
             if (true) { drop a; } else { move a; }\n  return 0;"
        )),
        vec!["Q046"]
    );
}

#[test]
fn the_join_refusal_names_the_model_that_refused() {
    let message = message(&program(
        "  let a = qcore_acquire(1, true, Failure);\n  \
         if (true) { drop a; }\n  return 0;",
    ));
    assert!(
        message.contains("do not join"),
        "the refusal must say the arms do not join: {message}"
    );
}

#[test]
fn a_binding_introduced_by_an_arm_does_not_survive_the_join() {
    // Arm-local scoping: naming an arm's binding after the join is an unknown
    // name, not a silent capture.
    let codes = codes(&program(
        "  if (true) { let a = qcore_acquire(1, true, Failure); drop a; } \
           else { let b = qcore_acquire(2, true, Failure); drop b; }\n  \
         drop a;\n  return 0;",
    ));
    assert!(
        !codes.is_empty() && codes.iter().all(|code| code != "Q046"),
        "naming an arm-local binding after the join must refuse on the name, not the join: {codes:?}"
    );
}

#[test]
fn an_acquisition_inside_an_arm_refuses_at_the_pre_existing_ownership_join() {
    // Measured, not designed: two arms acquiring *different* identities leave
    // different ownership ledgers, so the pre-existing `OwnershipOp::Branch`
    // join refuses. This card does not weaken that join; the shape is recorded
    // as follow-on `IF-ARM-ACQUIRE`.
    assert_eq!(
        codes(&program(
            "  if (1 < 2) { let a = qcore_acquire(1, true, Failure); drop a; } \
               else { let b = qcore_acquire(2, true, Failure); drop b; }\n  return 0;"
        )),
        vec!["Q103"]
    );
}

// ---------------------------------------------------------------------------
// 3. The condition tier
// ---------------------------------------------------------------------------

#[test]
fn a_chained_comparison_is_outside_the_tier() {
    assert_eq!(
        codes(&program("  if (1 < 2 < 3) { }\n  return 0;")),
        vec!["Q045"]
    );
}

#[test]
fn an_empty_condition_is_outside_the_tier() {
    assert_eq!(codes(&program("  if () { }\n  return 0;")), vec!["Q045"]);
}

#[test]
fn an_arithmetic_condition_is_outside_the_tier() {
    assert_eq!(
        codes(&program("  if (1 + 2) { }\n  return 0;")),
        vec!["Q045"]
    );
}

#[test]
fn a_condition_naming_a_binding_refuses_before_the_branch_pass() {
    // Fail-closed and recorded rather than forced: a name in a condition is
    // collected as a referenced name, so it refuses at `Q013` before the branch
    // pass runs. Lifting that is follow-on `IF-CONDITION-BINDINGS`.
    assert_eq!(
        codes(&program("  if (x < 5) { }\n  return 0;")),
        vec!["Q013"]
    );
}

#[test]
fn the_condition_refusal_is_attributed_to_the_condition() {
    let source = program("  if (1 < 2 < 3) { }\n  return 0;");
    let diagnostics = compile(&source)
        .map(|module| module.execution_profile)
        .expect_err("this fixture must be refused");
    let location = &diagnostics[0].location;
    let line = source.lines().nth(location.line - 1).unwrap();
    assert!(
        line.contains("if (1 < 2 < 3)"),
        "the refusal must land on the condition's own line: {line}"
    );
    // …and on the condition, not on the `if`.
    assert!(
        line[location.column - 1..].starts_with('1'),
        "the refusal must start at the condition's first token: {line}"
    );
}

#[test]
fn the_tier_gained_the_connectives_the_negation_and_the_integer_operands() {
    // Card `IF-STATEMENT` pinned this as `the_tier_has_no_connectives`,
    // asserting `Q038` for both spellings. `OR-AND-LOWERING` lowered the two
    // connectives onto this construct and moved the pin once;
    // `BITWISE-CONDITIONS` adds unary `!` and the bitwise integer operands and
    // moves it a second time. Each move is **deliberate**, and the closedness
    // half below is what makes the next widening move a test rather than slip
    // past one. The full suites are `compiler/tests/logical_connectives.rs` and
    // `compiler/tests/bitwise_conditions.rs`; this is the branch card's own
    // boundary marker.
    for condition in [
        "true && false",
        "true || false",
        "(true) || (false)",
        "!true",
        "!(3 < 5)",
        "(3 | 4) == 7",
    ] {
        compile(&program(&format!(
            "  let a = qcore_acquire(1, true, Failure);\n  \
             if ({condition}) {{ drop a; }} else {{ drop a; }}\n  return 0;"
        )))
        .unwrap_or_else(|errors| panic!("`{condition}` must be admitted: {errors:?}"));
    }
    // Everything else the tier refused, it still refuses: a bitwise operator
    // over conditions, an integer standing in for a condition, and the two
    // spellings whose C precedence this tier would have had to redefine.
    for condition in [
        "true | false",
        "true & false",
        "true ^ false",
        "3 | 4",
        "3 | 4 == 7",
        "!3 < 5",
        "1 |> 2",
    ] {
        let codes = codes(&program(&format!("  if ({condition}) {{ }}\n  return 0;")));
        assert!(
            !codes.is_empty(),
            "`{condition}` must stay outside the tier"
        );
    }
}

// ---------------------------------------------------------------------------
// 4. The arm statement subset
// ---------------------------------------------------------------------------

#[test]
fn an_early_return_from_a_branch_arm_keeps_this_programs_refusal() {
    // Card `IF-STATEMENT` pinned this program — `fn main() -> i32 { if (true)
    // { return 1; } }`, the fixture `reports_unsupported_source_at_the_owning_file`
    // used to carry — as `Q044` because a `return` inside an arm was refused
    // outright. Card `IF-ARMS` admits arm returns, so the **pin is moved
    // deliberately**: the code and the refusal survive, on the narrower ground
    // that this branch's surviving arm has no continuation to reach, so one path
    // reaches no `return` at all. The admitted spellings, and the leaking early
    // return that still refuses, are pinned in `compiler/tests/branch_arms.rs`.
    assert_eq!(
        codes("fn main() -> i32 { if (true) { return 1; } }"),
        vec!["Q044"]
    );
    assert!(message("fn main() -> i32 { if (true) { return 1; } }").contains("`return`"));
}

#[test]
fn a_call_inside_a_branch_arm_is_now_admitted() {
    // Card `IF-STATEMENT` pinned this as `["Q044"]`; card `IF-ARM-CALL` admits
    // it. The pin is **moved deliberately** and its load-bearing half is kept as
    // an assertion rather than dropped: a call inside an arm is admitted only
    // when composing the callee into the flat, positional whole-program call
    // sequence changes nothing, so the callee here is flow-neutral and a callee
    // that is not stays refused (`branch_arms.rs`).
    let source = "fn leaf() -> i32 { return 0; }\n\
                  fn main() -> i32 { if (true) { leaf(); } else { leaf(); } return 0; }";
    compile(source).expect("a conditional call to a flow-neutral callee is admitted");
}

#[test]
fn a_structured_statement_inside_a_branch_arm_keeps_its_own_refusal() {
    assert_eq!(
        codes(&program(
            "  if (true) { while (true) { } } else { }\n  return 0;"
        )),
        vec!["Q500"],
        "`while` is not this card's construct and keeps `Q500`"
    );
}

#[test]
fn a_check_inside_a_branch_arm_is_admitted() {
    compile(&program(
        "  if (true) { qcore_check(true, Failure); } else { qcore_check(true, Failure); }\n  \
         return 0;",
    ))
    .expect("a fallible check is a custody-neutral arm statement");
}

// ---------------------------------------------------------------------------
// 5. Nothing that does not use the construct moves
// ---------------------------------------------------------------------------

#[test]
fn a_branch_free_program_emits_byte_identical_c() {
    // The pin the card's block-numbering scheme exists for: block ids are
    // allocated in a pre-order walk that is the identity numbering on a
    // branch-free program, so nothing already shipped moves a byte.
    const BASELINE: &str = "error Failure { Failed; } \
        fn main() -> i32 { let a = qcore_acquire(3, true, Failure); \
        let b = qcore_acquire(4, true, Failure); qcore_check(true, Failure); \
        drop b; drop a; return 0; }";
    let module = compile(BASELINE).unwrap();
    let c = module.c_source.unwrap();
    // The exact block labels the emitter produced before this card.
    for label in [
        "q_bb_0:",
        "q_bb_1:",
        "q_bb_2:",
        "q_bb_3:",
        "q_bb_4:",
        "q_bb_5:",
        "q_cleanup_6:",
        "q_cleanup_7:",
        "q_cleanup_8:",
    ] {
        assert!(c.contains(label), "block `{label}` moved:\n{c}");
    }
    assert!(!c.contains("q_bb_9"), "no block was added:\n{c}");
}

#[test]
fn shipped_fixtures_are_unmoved_by_this_card() {
    // Two shipped programs on two other routes, compiled to make sure the
    // condition-span recording and the narrowed `Q500` gate changed nothing
    // about a file with no `if` in it.
    compile(LINES).expect("the std.io fixture still compiles");
    compile_with(
        ROUND_TRIP,
        Some(BTreeMap::from([(
            "fixtures/extern_calls/contracts/buffer_api.qcore".to_owned(),
            EXTERN_CONTRACTS.to_owned(),
        )])),
    )
    .expect("the extern fixture still compiles");
}

// ---------------------------------------------------------------------------
// 6. Route admission: sequential only, every other route unmoved
// ---------------------------------------------------------------------------

#[test]
fn the_std_io_route_keeps_its_flat_sequence_refusal() {
    let source = "#import std.fmt;\n#import std.io;\n\
        fn main() -> unit effects { io } { let total = 42; if (true) { print dec total; } return; }";
    let codes = compile(source)
        .map(|module| module.execution_profile)
        .expect_err("`source.std-io.v1` models one flat statement sequence")
        .into_iter()
        .map(|diagnostic| diagnostic.code.to_owned())
        .collect::<Vec<_>>();
    assert_eq!(codes, vec!["Q030"]);
}

#[test]
fn the_extern_route_keeps_its_nested_block_refusal() {
    let source = "#import std.extern;\nerror Failure { Failed; }\n\
        extern fn qcore_extern_alloc(size: usize) -> ptr<mut, u8>;\n\
        extern fn qcore_extern_release(buffer: ptr<mut, u8>) -> unit;\n\
        fn main() -> unit effects { io, alloc, mut } {\n\
        let b = qcore_extern_alloc(8) succeeds true else raise Failure;\n\
        if (true) { qcore_extern_release(move b); } else { qcore_extern_release(move b); }\n\
        return; }";
    let diagnostics = compile_with(
        source,
        Some(BTreeMap::from([(
            "fixtures/extern_calls/contracts/buffer_api.qcore".to_owned(),
            EXTERN_CONTRACTS.to_owned(),
        )])),
    )
    .map(|module| module.execution_profile)
    .expect_err("extern calls are admitted in a flat sequential entry body only");
    assert!(
        diagnostics[0].message.contains("nested block"),
        "the extern route's own refusal must stand: {diagnostics:?}"
    );
}

#[test]
fn the_task_route_keeps_its_profile_refusal() {
    let source = "#import std.concurrency;\nconcurrency checked;\n\
        fn main() -> i32 { thread scope s { } if (true) { } return 0; }";
    let codes = compile(source)
        .map(|module| module.execution_profile)
        .expect_err("the copy-scalar task profile does not model a branch")
        .into_iter()
        .map(|diagnostic| diagnostic.code.to_owned())
        .collect::<Vec<_>>();
    assert_eq!(codes, vec!["Q500"]);
}

// ---------------------------------------------------------------------------
// 7. Sanitized native run — reference against the compiled program
// ---------------------------------------------------------------------------

/// Compile and run the emitted C under ASan/UBSan (plus LSan where the platform
/// provides it) and compare the program's own instrumentation output with the
/// reference observation.
///
/// This is the cross-check the decided condition earns: the C computes
/// `3 < 5` itself, so agreement here is a real comparison of the Rust evaluator
/// against a C compiler rather than a tautology. The leak check is a
/// **positive** control — the program allocates and frees.
#[test]
fn the_native_run_of_a_branch_matches_its_reference() {
    let module = compile(CONDITIONAL_RELEASE).unwrap();
    let instrumented = sequential_observation::instrument(module.c_source.as_deref().unwrap());
    let source = instrumented.as_str();
    let reference = module.reference_observation.as_ref().unwrap();

    #[cfg(target_os = "windows")]
    let run = windows_c::run(source, None, &[]);

    #[cfg(not(target_os = "windows"))]
    let run = {
        use std::process::Command;
        let directory = std::env::temp_dir().join(format!(
            "qcore-branch-{}-{}",
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
            "C compiler rejected the emitted branch:\n{}",
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
        "the emitted branch failed under ASan/UBSan/LSan:\n{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let stdout = String::from_utf8(run.stdout).unwrap();
    // The fixture declares exactly one error type, so its ABI is that one
    // variant; the harness's output is parsed against it.
    let abi = qcore_compiler::ErrorAbi::new(["Failure".to_owned()]).unwrap();
    let observed = sequential_observation::parse(&stdout, &abi)
        .unwrap_or_else(|error| panic!("malformed native observation: {error:?}\n{stdout}"));
    assert_eq!(
        &observed, reference,
        "the compiled program must observe what the reference does"
    );
}

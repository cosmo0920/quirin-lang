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

//! The widened arm statement subset of `source.branch.v1` (card `IF-ARMS`).
//!
//! Card `IF-STATEMENT` landed the branch construct with an arm subset that
//! admitted `qcore_check`, `drop`, `move`, a nested `if` and the empty arm, and
//! refused a `return` and a call. This suite is the widening of that subset and
//! the boundary that stays:
//!
//! * **`IF-ARM-RETURN`** — an exit from an arm carries exactly the obligations
//!   the sequential route imposes at any exit, so an arm that returns discharges
//!   at its own exit and does not reach the join. The join is then taken over
//!   the arms that *survive* it, which is a change of shape rather than a
//!   relaxation: every leak that refused before still refuses, and the leaking
//!   early return refuses at its own exit.
//! * **`IF-ARM-CALL`** — a call inside an arm. A source call emits no C at all
//!   in this executable subset, so the reference/native observation comparison
//!   is untouched by one; what a *conditional* call raises is the flat,
//!   positional whole-program call sequence, and that is answered by refusing a
//!   conditional call whose callee's summary flow is not neutral.
//! * **`IF-ARM-ACQUIRE-ATTRIBUTION`** — an acquisition inside an arm is still
//!   refused, at the same `Q103` it always was. Only its span and its wording
//!   move, which is an attribution fix rather than a code change (§L21).
use std::collections::BTreeMap;

use qcore_compiler::{CompileOptions, SourceFile, compile_files};

#[path = "../../tests/support/sequential_observation.rs"]
mod sequential_observation;

#[cfg(target_os = "windows")]
#[path = "../../tests/support/windows_c.rs"]
mod windows_c;

const EARLY_RETURN: &str = include_str!("fixtures/branch_arms/early_return.qr");
const CALL_THEN_ARM: &str = include_str!("fixtures/branch_arms/call_then_arm.qr");
const CALL_ELSE_ARM: &str = include_str!("fixtures/branch_arms/call_else_arm.qr");
const LINES: &str = include_str!("fixtures/std_modules/lines.qr");
const ROUND_TRIP: &str = include_str!("fixtures/extern_calls/round_trip.qr");
const EXTERN_CONTRACTS: &str = include_str!("fixtures/extern_calls/contracts/buffer_api.qv");

fn compile_with(
    source: &str,
    contracts: Option<BTreeMap<String, String>>,
) -> Result<qcore_compiler::CompiledModule, Vec<qcore_compiler::FileDiagnostic>> {
    compile_files(
        &[SourceFile::new("arms.qcore", source)],
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

fn refusal(source: &str) -> Vec<qcore_compiler::FileDiagnostic> {
    compile(source)
        .map(|module| module.execution_profile)
        .expect_err("this fixture must be refused")
}

fn codes(source: &str) -> Vec<String> {
    refusal(source)
        .into_iter()
        .map(|diagnostic| diagnostic.code.to_owned())
        .collect()
}

fn message(source: &str) -> String {
    refusal(source)
        .first()
        .expect("a refusal carries at least one diagnostic")
        .message
        .clone()
}

/// The source line a refusal is attributed to, so an attribution claim is
/// measured rather than asserted.
fn refusal_line(source: &str) -> String {
    let diagnostics = refusal(source);
    let location = &diagnostics[0].location;
    source
        .lines()
        .nth(location.line - 1)
        .expect("a diagnostic names a line of its own file")
        .to_owned()
}

fn program(body: &str) -> String {
    format!("error Failure {{ Failed; }}\nfn main() -> i32 {{\n{body}\n}}\n")
}

/// A program with a callee whose summary flow is neutral — the only kind a
/// conditional call may name.
fn with_helper(body: &str) -> String {
    format!(
        "error Failure {{ Failed; }}\n\
         fn helper() -> i32 {{ qcore_check(true, Failure); return 0; }}\n\
         fn main() -> i32 {{\n{body}\n}}\n"
    )
}

// ---------------------------------------------------------------------------
// 1. IF-ARM-RETURN: what the arm subset now admits
// ---------------------------------------------------------------------------

#[test]
fn an_early_return_from_an_arm_is_admitted() {
    let module = compile(EARLY_RETURN).expect("the returning arm discharges at its own exit");
    assert_eq!(module.entry.as_deref(), Some("main"));
}

#[test]
fn the_early_return_exits_through_the_arms_own_return() {
    // The taken arm returns `1`, not the `0` that follows the branch: the exit
    // is the arm's, which is the whole point of an early return.
    let module = compile(EARLY_RETURN).unwrap();
    let observation = format!("{:?}", module.reference_observation.unwrap());
    assert!(
        observation.contains("value: 1"),
        "the observed exit must be the arm's own: {observation}"
    );
}

#[test]
fn both_paths_release_before_their_own_exit() {
    // Two release sites, one per path, each before that path's `return`. The
    // arm that exits early is not exempt from cleanup; it discharges earlier.
    let module = compile(EARLY_RETURN).unwrap();
    let c = module.c_source.expect("an executable module emits C");
    assert_eq!(
        c.matches("q_release(&q_res_1, UINT64_C(1));").count(),
        2,
        "one release per path:\n{c}"
    );
    assert!(
        c.contains("if (INT32_C(3) < INT32_C(5))"),
        "the emitted C still computes the condition:\n{c}"
    );
}

#[test]
fn both_arms_may_return() {
    compile(&program("  if (3 < 5) { return 1; } else { return 0; }"))
        .expect("a branch every arm of which returns ends the function");
}

#[test]
fn either_arm_may_be_the_one_that_returns() {
    for (then_arm, else_arm) in [
        ("drop a; return 1;", ""),
        ("", "drop a; return 1;"),
        ("drop a; return 1;", "drop a; return 2;"),
    ] {
        let source = program(&format!(
            "  let a = qcore_acquire(1, true, Failure);\n  \
             if (3 < 5) {{ {then_arm} }} else {{ {else_arm} }}\n  drop a;\n  return 0;"
        ));
        // The two shapes with one survivor hoist the tail into it; the shape
        // with no survivor leaves the tail unreachable, which is §2's refusal.
        let outcome = compile(&source);
        if else_arm.contains("return 2") {
            assert_eq!(
                outcome
                    .map(|module| module.execution_profile)
                    .expect_err("no arm survives, so the tail is unreachable")
                    .into_iter()
                    .map(|diagnostic| diagnostic.code.to_owned())
                    .collect::<Vec<_>>(),
                vec!["Q044"]
            );
        } else {
            outcome.unwrap_or_else(|errors| {
                panic!("arms `{then_arm}` / `{else_arm}` must be admitted: {errors:?}")
            });
        }
    }
}

#[test]
fn every_arm_of_an_else_if_chain_may_return() {
    compile(&program(
        "  let a = qcore_acquire(1, true, Failure);\n  \
         if (3 > 5) { drop a; return 1; } else if (2 == 2) { drop a; return 2; }\n  \
         drop a;\n  return 0;",
    ))
    .expect("the chain's last non-returning arm carries the continuation");
}

#[test]
fn an_early_return_is_admitted_in_a_non_entry_function() {
    // A different path through the driver: a leaf declaration's semantics reach
    // `check_whole_program`'s summary unfolding rather than emission.
    let module = compile(
        "error Failure { Failed; }\n\
         fn leaf() -> i32 { let a = qcore_acquire(7, true, Failure); \
           if (1 < 2) { drop a; return 1; } drop a; return 0; }\n\
         fn main() -> i32 { return 0; }",
    )
    .expect("an early return checks in a leaf function as well as in the entry");
    assert_eq!(module.checked.function_count(), 2);
}

#[test]
fn an_early_return_through_a_connective_is_admitted() {
    // Card `OR-AND-LOWERING` lowers a connective into nested branches over the
    // *same* two arms, so a returning arm has to survive being duplicated into
    // a nested position. Measured rather than assumed.
    for condition in ["3 < 5 || 2 == 2", "3 < 5 && 2 == 2"] {
        compile(&program(&format!(
            "  let a = qcore_acquire(1, true, Failure);\n  \
             if ({condition}) {{ drop a; return 1; }}\n  drop a;\n  return 0;"
        )))
        .unwrap_or_else(|errors| panic!("`{condition}` must be admitted: {errors:?}"));
    }
}

// ---------------------------------------------------------------------------
// 2. IF-ARM-RETURN: what stays refused, and on what ground
// ---------------------------------------------------------------------------

#[test]
fn an_early_return_that_leaves_a_resource_live_refuses() {
    // The obligation the widening rests on. `return` demands an empty live list
    // wherever it stands, so an early exit is not a way out of cleanup.
    let source = program(
        "  let a = qcore_acquire(1, true, Failure);\n  \
         if (3 < 5) { return 1; }\n  drop a;\n  return 0;",
    );
    assert_eq!(codes(&source), vec!["Q202"]);
    assert!(
        message(&source).contains("still live at this `return`"),
        "the refusal must name the obligation: {}",
        message(&source)
    );
    // …and it lands on the arm's own `return`, not on the function declaration.
    assert!(
        refusal_line(&source).contains("if (3 < 5) { return 1; }"),
        "the refusal must land on the exit that leaks: {}",
        refusal_line(&source)
    );
}

#[test]
fn an_early_return_with_nothing_after_the_branch_refuses() {
    // Card `IF-STATEMENT` pinned this program as `Q044` because a `return` in an
    // arm was refused outright. It is still `Q044`, on a narrower ground: the
    // surviving arm has no continuation to reach, so one path reaches no
    // `return` at all.
    let source = "fn main() -> i32 { if (true) { return 1; } }";
    assert_eq!(codes(source), vec!["Q044"]);
    assert!(message(source).contains("`return`"));
}

#[test]
fn statements_after_a_branch_whose_arms_all_return_are_unreachable() {
    let source = program("  if (3 < 5) { return 1; } else { return 2; }\n  return 0;");
    assert_eq!(codes(&source), vec!["Q044"]);
    assert!(
        message(&source).contains("unreachable"),
        "the refusal must say so: {}",
        message(&source)
    );
}

#[test]
fn an_early_return_in_one_arm_of_a_nested_branch_refuses() {
    // The shape that would need the statements after the outer branch in *two*
    // arms. Placing them twice would duplicate every acquisition, call and
    // demand in them, so it is refused rather than duplicated
    // (`IF-ARM-RETURN-NESTED`).
    let source = program(
        "  let a = qcore_acquire(1, true, Failure);\n  \
         if (3 < 5) { if (2 == 2) { drop a; return 1; } drop a; } else { drop a; }\n  return 0;",
    );
    assert_eq!(codes(&source), vec!["Q044"]);
    assert!(
        message(&source).contains("IF-ARM-RETURN-NESTED"),
        "the refusal must name its follow-on: {}",
        message(&source)
    );
}

#[test]
fn a_returning_arm_later_in_an_else_if_chain_refuses() {
    // The other half of the duplication refusal, and the one that shows it is a
    // budget rather than a syntax rule: the chain's *second* level spends the
    // continuation into its own surviving arm, and the first level then needs it
    // too. The message distinguishes this from "there were none".
    let source = program(
        "  let a = qcore_acquire(1, true, Failure);\n  \
         if (3 < 5) { drop a; } else if (2 == 2) { drop a; return 1; }\n  drop a;\n  return 0;",
    );
    assert_eq!(codes(&source), vec!["Q044"]);
    assert!(
        message(&source).contains("already been placed"),
        "the refusal must say the continuation was already spent: {}",
        message(&source)
    );
}

#[test]
fn a_hoisted_continuation_may_itself_branch_and_call() {
    // The continuation moved into the surviving arm is lowered by the same
    // recursion as any other sequence, so a branch and a call inside it are
    // handled — and the inner branch, having no returning arm, is not hoisted
    // at all.
    compile(&with_helper(
        "  let a = qcore_acquire(1, true, Failure);\n  \
         if (3 < 5) { drop a; return 1; }\n  \
         if (2 == 2) { drop a; } else { drop a; }\n  helper();\n  return 0;",
    ))
    .expect("a hoisted continuation is an ordinary statement sequence");
}

#[test]
fn an_early_return_whose_path_disposes_differently_refuses() {
    // Measured, not designed. `qcore-core`'s own rule would admit this: the
    // returning arm transferred `a` to the result and exited, the survivor
    // released it, and the join over survivors is the survivor's state. But the
    // hoist places the continuation in the surviving arm and the four *unedited*
    // models then compare two arms that genuinely differ — `a` is `Moved` on one
    // path and `Dropped` on the other — so the pre-existing ownership ledger
    // join refuses at `Q103`. Fail-closed, recorded rather than repaired:
    // follow-on `IF-ARM-RETURN-DIVERGENT-CUSTODY`.
    assert_eq!(
        codes(&program(
            "  let a = qcore_acquire(1, true, Failure);\n  \
             if (3 < 5) { move a; return 1; }\n  drop a;\n  return 0;"
        )),
        vec!["Q103"]
    );
    // The same program with both paths disposing the same way is admitted, so
    // the pin above is about the divergence rather than about early returns.
    compile(&program(
        "  let a = qcore_acquire(1, true, Failure);\n  \
         if (3 < 5) { move a; return 1; }\n  move a;\n  return 0;",
    ))
    .expect("both paths moving the resource out is admitted");
}

#[test]
fn a_structured_statement_after_an_arm_return_keeps_its_own_refusal() {
    assert_eq!(
        codes(&program(
            "  if (3 < 5) { while (true) { } return 1; }\n  return 0;"
        )),
        vec!["Q500"],
        "`while` is not this card's construct and keeps `Q500`"
    );
}

// ---------------------------------------------------------------------------
// 3. IF-ARM-CALL
// ---------------------------------------------------------------------------

#[test]
fn a_call_inside_an_arm_is_admitted() {
    for fixture in [CALL_THEN_ARM, CALL_ELSE_ARM] {
        compile(fixture).expect("a conditional call to a flow-neutral callee is admitted");
    }
}

#[test]
fn a_source_call_emits_no_c_at_all() {
    // The measurement that makes this widening small, recorded rather than
    // assumed: a source call in the executable subset contributes nothing to the
    // emitted program. It composes the callee's summary flow and effects into
    // the caller's and demands the callee's usage — that is all. So a call in an
    // arm cannot make "the" observation ill-defined, whichever arm it is in.
    let module = compile(CALL_THEN_ARM).unwrap();
    let c = module.c_source.expect("an executable module emits C");
    assert!(
        !c.contains("helper"),
        "a source call emits no C, so the callee's name must not appear:\n{c}"
    );
}

#[test]
fn the_two_polarities_observe_the_arm_the_condition_selects() {
    let taken = compile(CALL_THEN_ARM).unwrap();
    let untaken = compile(CALL_ELSE_ARM).unwrap();
    // Both programs acquire and release exactly once, and the call contributes
    // no event either way, so the observations agree — which is the honest form
    // of "the observation stays well-defined".
    assert_eq!(taken.reference_observation, untaken.reference_observation);
    // …and the two are genuinely different programs.
    assert_ne!(taken.c_source, untaken.c_source);
}

#[test]
fn a_conditional_call_to_a_callee_that_uses_resources_refuses() {
    // Fail-closed, and the whole content of the `IF-ARM-CALL` gate: the flat
    // call sequence would compose the callee's flow on paths that never perform
    // it, so a non-neutral callee is refused rather than approximated.
    let source = "error Failure { Failed; }\n\
        fn leaf() -> i32 { let x = qcore_acquire(9, true, Failure); drop x; return 0; }\n\
        fn main() -> i32 { if (3 < 5) { leaf(); } else { } return 0; }";
    assert_eq!(codes(source), vec!["Q044"]);
    assert!(
        message(source).contains("neutral"),
        "the refusal must name the premise it needs: {}",
        message(source)
    );
}

#[test]
fn a_conditional_call_does_not_launder_the_custody_join() {
    // A call inside an arm must not move the arm's own custody out of the branch
    // and into the flat segment chain, where it would be applied on both paths.
    // The conditional leak still refuses with a call beside it.
    assert_eq!(
        codes(&with_helper(
            "  let a = qcore_acquire(1, true, Failure);\n  \
             if (3 < 5) { helper(); drop a; }\n  return 0;"
        )),
        vec!["Q046"]
    );
    // …and the balanced spelling of the same program is admitted, so the pin
    // above is not a blanket refusal of calls beside custody.
    compile(&with_helper(
        "  let a = qcore_acquire(1, true, Failure);\n  \
         if (3 < 5) { helper(); drop a; } else { drop a; }\n  return 0;",
    ))
    .expect("a conditional call beside a joined release is admitted");
}

#[test]
fn a_call_inside_an_arm_keeps_every_other_call_refusal() {
    // The arm position widens *where* a call may appear, not *what* a call may
    // be. Each of these keeps the code it has at the top level.
    let unknown = with_helper("  if (3 < 5) { missing(); } else { }\n  return 0;");
    assert_eq!(codes(&unknown), vec!["Q013"]);
    let indirect = with_helper(
        "  let a = qcore_acquire(1, true, Failure);\n  \
         if (3 < 5) { a(); drop a; } else { drop a; }\n  return 0;",
    );
    assert!(
        !codes(&indirect).is_empty(),
        "an indirect call inside an arm must stay refused"
    );
}

#[test]
fn a_call_inside_an_arm_composes_through_a_connective() {
    compile(&with_helper(
        "  let a = qcore_acquire(1, true, Failure);\n  \
         if (3 < 5 || 2 == 2) { helper(); drop a; } else { drop a; }\n  return 0;",
    ))
    .expect("a connective duplicates the arm structurally, never the call itself");
}

#[test]
fn a_call_and_a_return_may_share_an_arm() {
    compile(&with_helper(
        "  let a = qcore_acquire(1, true, Failure);\n  \
         if (3 < 5) { helper(); drop a; return 1; }\n  drop a;\n  return 0;",
    ))
    .expect("the two widenings compose");
}

// ---------------------------------------------------------------------------
// 4. IF-ARM-ACQUIRE-ATTRIBUTION: the refusal that stays, better attributed
// ---------------------------------------------------------------------------

#[test]
fn an_arm_acquisition_still_refuses_at_the_code_it_always_did() {
    // Deliberately **not** weakened: two arms acquiring leave different
    // resource-identity ledgers and `OwnershipState::branch` compares ledgers.
    // Admitting this is follow-on `IF-ARM-ACQUIRE`, which is out of this card's
    // scope; only the attribution moves, which §L21 says is not a code change.
    assert_eq!(
        codes(&program(
            "  if (1 < 2) { let a = qcore_acquire(1, true, Failure); drop a; } \
               else { let b = qcore_acquire(2, true, Failure); drop b; }\n  return 0;"
        )),
        vec!["Q103"]
    );
}

#[test]
fn the_arm_acquisition_refusal_names_the_arm_and_the_identity() {
    let source = program(
        "  if (1 < 2) { let a = qcore_acquire(1, true, Failure); drop a; } else { }\n  return 0;",
    );
    let message = message(&source);
    assert!(
        message.contains("`a`"),
        "the refusal must name the binding: {message}"
    );
    assert!(
        message.contains("resource identity 1"),
        "the refusal must name the identity: {message}"
    );
    assert!(
        message.contains("branch arm"),
        "the refusal must say where it is: {message}"
    );
    assert!(
        message.contains("IF-ARM-ACQUIRE"),
        "the refusal must name its follow-on: {message}"
    );
}

#[test]
fn the_arm_acquisition_refusal_lands_on_the_acquisition() {
    // Card `IF-STATEMENT` recorded this refusal landing on the function
    // declaration. It now lands on the `let` that causes it.
    let source = program(
        "  if (1 < 2) { let a = qcore_acquire(1, true, Failure); drop a; } else { }\n  return 0;",
    );
    assert!(
        refusal_line(&source).contains("qcore_acquire(1"),
        "the refusal must land on the acquisition: {}",
        refusal_line(&source)
    );
}

#[test]
fn an_acquisition_hoisted_into_an_arm_says_so() {
    // A statement written *after* the branch can end up inside an arm through
    // the continuation hoisting an early return requires. It refuses at the same
    // `Q103` for the same reason, and the message says which situation it is
    // rather than claiming the reader wrote it in an arm.
    let source = program(
        "  if (3 < 5) { return 1; }\n  \
         let a = qcore_acquire(1, true, Failure);\n  drop a;\n  return 0;",
    );
    assert_eq!(codes(&source), vec!["Q103"]);
    assert!(
        message(&source).contains("after a branch one arm of which returns"),
        "the refusal must name the hoist: {}",
        message(&source)
    );
}

// ---------------------------------------------------------------------------
// 5. Nothing that does not use the widening moves
// ---------------------------------------------------------------------------

#[test]
fn a_branch_free_program_emits_byte_identical_c() {
    // The pre-card block numbering, re-pinned here so the arm widening is shown
    // not to have disturbed it either.
    const BASELINE: &str = "error Failure { Failed; } \
        fn main() -> i32 { let a = qcore_acquire(3, true, Failure); \
        let b = qcore_acquire(4, true, Failure); qcore_check(true, Failure); \
        drop b; drop a; return 0; }";
    let c = compile(BASELINE).unwrap().c_source.unwrap();
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
fn a_branch_with_no_arm_exit_is_numbered_as_before() {
    // The continuation hoisting must be inert for every program card
    // `IF-STATEMENT` already admitted: with no returning arm there is nothing to
    // hoist, so the statements after the branch keep their positions.
    let module = compile(
        "error Failure { Failed; } \
         fn main() -> i32 { let a = qcore_acquire(1, true, Failure); \
         if (3 < 5) { drop a; } else { drop a; } return 0; }",
    )
    .unwrap();
    let c = module.c_source.unwrap();
    for label in ["q_bb_0:", "q_bb_1:", "q_bb_2:", "q_bb_3:", "q_bb_4:"] {
        assert!(c.contains(label), "block `{label}` moved:\n{c}");
    }
    assert!(!c.contains("q_bb_5:"), "no block was added:\n{c}");
}

#[test]
fn shipped_fixtures_are_unmoved_by_this_card() {
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
fn the_other_routes_keep_their_refusals_for_the_widened_arms() {
    // An arm that returns and an arm that calls are still only admitted on the
    // sequential route; the widening is of the arm subset, not of route
    // admission (`IF-ROUTE-WIDENING` is unchanged).
    let std_io = "#import std.fmt;\n#import std.io;\n\
        fn main() -> unit effects { io } { let total = 42; if (true) { print dec total; return; } return; }";
    assert_eq!(codes(std_io), vec!["Q030"]);
    let task = "#import std.concurrency;\nconcurrency checked;\n\
        fn main() -> i32 { thread scope s { } if (true) { return 1; } return 0; }";
    assert_eq!(codes(task), vec!["Q500"]);
}

// ---------------------------------------------------------------------------
// 7. Sanitized native runs — both condition polarities
// ---------------------------------------------------------------------------

/// Compile and run the emitted C with the Windows native harness or, elsewhere,
/// under ASan/UBSan (plus LSan where the platform provides it), then compare
/// the program's own instrumentation output with the reference observation.
fn sanitized_run_matches_reference(source: &str, label: &str) {
    let module = compile(source).unwrap_or_else(|errors| panic!("{label}: {errors:?}"));
    let instrumented = sequential_observation::instrument(module.c_source.as_deref().unwrap());
    let c_source = instrumented.as_str();
    let reference = module.reference_observation.as_ref().unwrap();

    #[cfg(target_os = "windows")]
    let run = windows_c::run(c_source, None, &[]);

    #[cfg(not(target_os = "windows"))]
    let run = {
        use std::process::Command;
        let directory = std::env::temp_dir().join(format!(
            "qcore-branch-arms-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&directory).unwrap();
        let generated = directory.join("generated.c");
        let binary = directory.join("generated");
        std::fs::write(&generated, c_source).unwrap();
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
            "{label}: C compiler rejected the emitted program:\n{}",
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
        "{label}: the emitted program failed its native checks:\n{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let stdout = String::from_utf8(run.stdout).unwrap();
    let abi = qcore_compiler::ErrorAbi::new(["Failure".to_owned()]).unwrap();
    let observed = sequential_observation::parse(&stdout, &abi)
        .unwrap_or_else(|error| panic!("malformed native observation: {error:?}\n{stdout}"));
    assert_eq!(
        &observed, reference,
        "{label}: the compiled program must observe what the reference does"
    );
}

/// The early return, run for real: the program exits through the **arm's** own
/// `return`, having released inside the arm. The leak check is a positive
/// control — the program allocates and frees, and the free it performs is the
/// one before the early exit.
#[test]
fn the_native_run_of_an_early_return_matches_its_reference() {
    sanitized_run_matches_reference(EARLY_RETURN, "early-return");
}

/// A conditional call in the arm the decided condition **takes**.
#[test]
fn the_native_run_of_a_call_in_the_taken_arm_matches_its_reference() {
    sanitized_run_matches_reference(CALL_THEN_ARM, "call-taken");
}

/// …and in the arm it does **not** take: the call is compiled, linked and
/// reachable from the emitted `goto` table but never entered, and the
/// observation still agrees.
#[test]
fn the_native_run_of_a_call_in_the_untaken_arm_matches_its_reference() {
    sanitized_run_matches_reference(CALL_ELSE_ARM, "call-untaken");
}

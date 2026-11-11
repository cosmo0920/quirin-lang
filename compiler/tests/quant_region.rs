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

//! `source.region.v1` — the checked error region and its quantitative
//! cleanup-pack opening (card `QUANT-REGION-SOURCE`).
//!
//! This is the **first source path to `RegionOp::QuantOpen`**. Before this card
//! `crates/qcore-core/src/error_region.rs` defined and interpreted the
//! operation and the workspace's only other mention of it was a no-op arm in a
//! traversal, so no fixture could ask what either CT21 or CT22 asks.
//!
//! What the model says a quantitative region **is**, stated once so every
//! assertion below can be read against it:
//!
//! * an **error region** is a lexical frame recording the ledger it was entered
//!   with (its *baseline*), one cleanup duty per resource acquired inside it,
//!   and a strict credit remainder — the region's quantity;
//! * its close is **lexical**, and the two closes differ: on the normal edge
//!   the duties run in reverse acquisition order and the remainder must be
//!   zero; on an error edge the frame *seals* into a `CleanupPack` carrying
//!   region, error, baseline, duties and remainder;
//! * `quant open R as B` is valid **only** on a catch clause and does not
//!   discard the pack — it binds it as a **linear capability** named `B` which
//!   `discharge B;` must consume **exactly once** before the handler leaves by
//!   any edge. That linearity is the whole content of the word *quant* here.
//!
//! Every refusal below is the unedited `qcore_core::error_region` model's own,
//! reported under the `Q3xx` codes that pass already allocates. This card
//! allocates **no** code and reserves **no** word.
use qcore_compiler::{CompileOptions, SourceFile, compile_files};

#[path = "../../tests/support/sequential_observation.rs"]
mod sequential_observation;

#[cfg(target_os = "windows")]
#[path = "../../tests/support/windows_c.rs"]
mod windows_c;

const DISCHARGED_PACK: &str = include_str!("fixtures/region/discharged_pack.qr");
const HAND_WRITTEN_TWIN: &str = include_str!("fixtures/region/hand_written_twin.qr");

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
    compile_named("region.qcore", source)
}

fn refusal(source: &str) -> (String, String) {
    let diagnostics = compile(source)
        .map(|module| module.execution_profile)
        .expect_err("this fixture must be refused");
    let first = diagnostics
        .first()
        .expect("a refusal carries at least one diagnostic");
    (first.code.to_owned(), first.message.clone())
}

/// One entry, one region, parameterised on the body and the clause.
fn program(body: &str, clause: &str, handler: &str) -> String {
    format!(
        "error Failure {{ Failed; }}\n\
         error Other {{ Bad; }}\n\
         fn main() -> i32 {{\n\
         error region R {{\n{body}\n}} catch {clause} {{\n{handler}\n}}\n\
         return 0;\n}}\n"
    )
}

/// The body every refusal fixture shares: one acquisition and the decided
/// error edge, so the pack is nonempty and the handler is reached.
const ONE_RESOURCE: &str = "let a = qcore_acquire(1, true, Failure);\nqcore_check(false, Failure);";

/// Emitted C with its `#line` directives removed. Two fixtures live in
/// different files at different lines, so the directives differ by
/// construction; everything else is the comparison the erasure claim rests on.
fn without_line_directives(c: &str) -> String {
    c.lines()
        .filter(|line| !line.starts_with("#line "))
        .collect::<Vec<_>>()
        .join("\n")
}

// ---------------------------------------------------------------------------
// 1. The construct, and the operation it reaches
// ---------------------------------------------------------------------------

#[test]
fn a_region_whose_pack_is_opened_and_discharged_is_admitted() {
    let module = compile(DISCHARGED_PACK).expect("the pack is opened once and discharged once");
    assert_eq!(module.entry.as_deref(), Some("main"));
    assert_eq!(
        module.execution_profile,
        qcore_compiler::SourceExecutionProfile::SequentialCleanupV1,
        "the region rides the flat sequential route rather than a route of its own"
    );
}

#[test]
fn the_source_path_reaches_the_kernel_quant_open() {
    // The card's whole content in one assertion: a source module now produces
    // `error_region::QuantOpen`, on a catch clause, naming this region and the
    // capability the handler discharges.
    let module = compile(DISCHARGED_PACK).unwrap();
    let checked = module
        .checked
        .as_sequential()
        .expect("the sequential route");
    let function = checked
        .functions
        .values()
        .next()
        .expect("the entry is checked");
    let operations = &function.region_operations;
    let [
        qcore_compiler::RegionOp::ErrorRegion {
            region,
            body,
            catches,
        },
    ] = operations.as_slice()
    else {
        panic!("the entry contributes exactly one region: {operations:?}");
    };
    assert_eq!(region, "R");
    assert_eq!(
        body.len(),
        4,
        "two acquisitions, one credit and the raise: {body:?}"
    );
    assert!(matches!(
        body[2],
        qcore_compiler::RegionOp::AddStrictCredit(3)
    ));
    assert!(matches!(&body[3], qcore_compiler::RegionOp::Raise(e) if e == "Failure"));
    let [clause] = catches.as_slice() else {
        panic!("one catch clause");
    };
    assert_eq!(clause.error, "Failure");
    let opening = clause
        .quant_open
        .as_ref()
        .expect("`quant open R as pack` reaches the kernel operation");
    assert_eq!(opening.region, "R");
    assert_eq!(opening.binding, "pack");
    assert_eq!(
        clause.handler,
        vec![qcore_compiler::RegionOp::Discharge("pack".to_owned())]
    );
}

// ---------------------------------------------------------------------------
// 2. Faithfulness — the emitted program performs the model's own cleanup
// ---------------------------------------------------------------------------

#[test]
fn the_emitted_releases_are_the_packs_duties_in_reverse() {
    use qcore_compiler::{CleanupEvent, RegionExitTag, TraceEvent};
    let module = compile(DISCHARGED_PACK).unwrap();
    let checked = module.checked.as_sequential().unwrap();
    let function = checked.functions.values().next().unwrap();

    // What the model says happens: the two duties run in reverse acquisition
    // order, and the unspent credit is abandoned.
    let states = function.regions.states(&RegionExitTag::Normal);
    let [state] = states else {
        panic!("the decided path has one normal exit: {states:?}");
    };
    assert_eq!(
        state.cleanup_trace(),
        &[
            CleanupEvent::Destroy(2),
            CleanupEvent::Destroy(1),
            CleanupEvent::AbandonCredit(3),
        ]
    );

    // What the emitted program does: the identical releases, in the identical
    // order. The credit has no runtime object and therefore no trace event —
    // which is precisely why abandoning it is admissible and destroying a
    // resource silently is not.
    let reference = module.reference_observation.as_ref().unwrap();
    let releases = reference
        .trace
        .iter()
        .filter(|event| matches!(event, TraceEvent::Release(_)))
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(
        releases,
        vec![TraceEvent::Release(2), TraceEvent::Release(1)]
    );
    assert_eq!(
        reference.trace,
        vec![
            TraceEvent::Acquire(1),
            TraceEvent::Acquire(2),
            TraceEvent::Release(2),
            TraceEvent::Release(1),
        ]
    );
}

#[test]
fn an_admitted_region_emits_what_its_hand_written_twin_emits() {
    // The erasure claim, measured rather than asserted: a checked region and
    // the straight-line program it *is* produce byte-identical C once the
    // `#line` directives (which name two different files) are removed. A region
    // buys checking, not code — card `LOOP-UNROLL-BUDGET`'s licence erased from
    // emission the same way.
    let region = compile_named("region.qcore", DISCHARGED_PACK).unwrap();
    let twin = compile_named("twin.qcore", HAND_WRITTEN_TWIN).unwrap();
    assert_eq!(
        without_line_directives(region.c_source.as_deref().unwrap()),
        without_line_directives(twin.c_source.as_deref().unwrap())
    );
    assert_eq!(region.reference_observation, twin.reference_observation);
}

#[test]
fn a_program_without_a_region_is_untouched() {
    // The compatibility statement. Before this card `error region R { … }`
    // refused at `Q013` (`R` resolved as a reference to nothing), so no program
    // in this repository could contain the statement — but the card also
    // factored the acquisition and release bookkeeping out of the statement
    // loop, and this is the control for that.
    let module = compile(
        "error Failure { Failed; }\n\
         fn main() -> i32 { let a = qcore_acquire(1, true, Failure); drop a; return 42; }",
    )
    .expect("the ordinary sequential program is unaffected");
    let c = module.c_source.unwrap();
    assert!(c.contains("q_acquire(&q_res_1, true, UINT64_C(1))"), "{c}");
    assert!(c.contains("q_release(&q_res_1, UINT64_C(1));"), "{c}");
}

// ---------------------------------------------------------------------------
// 3. The pairing discipline — every refusal is the model's own
// ---------------------------------------------------------------------------

#[test]
fn an_opened_pack_that_is_never_discharged_refuses() {
    let (code, message) = refusal(&program(ONE_RESOURCE, "(Failure) quant open R as pack", ""));
    assert_eq!(code, "Q305");
    assert_eq!(message, "cleanup pack is not discharged");
}

#[test]
fn a_nonempty_pack_caught_without_opening_it_refuses() {
    let (code, message) = refusal(&program(ONE_RESOURCE, "(Failure)", ""));
    assert_eq!(code, "Q306");
    assert_eq!(
        message,
        "catch for `Failure` requires quant open of region `R`"
    );
}

#[test]
fn opening_another_region_refuses() {
    let (code, message) = refusal(&program(
        ONE_RESOURCE,
        "(Failure) quant open S as pack",
        "discharge pack;",
    ));
    assert_eq!(code, "Q303");
    assert_eq!(message, "quant open refers to `S`, expected region `R`");
}

#[test]
fn discharging_one_pack_twice_refuses() {
    let (code, message) = refusal(&program(
        ONE_RESOURCE,
        "(Failure) quant open R as pack",
        "discharge pack;\ndischarge pack;",
    ));
    assert_eq!(code, "Q304");
    assert_eq!(message, "cleanup pack `pack` is consumed more than once");
}

#[test]
fn an_empty_pack_may_be_caught_without_opening_it() {
    // The boundary of `Q306`: the rule is about a pack with something in it.
    // A region that acquires nothing and credits nothing seals an empty pack,
    // and a clause that does not open one discards nothing.
    compile(&program("qcore_check(false, Failure);", "(Failure)", ""))
        .expect("an empty pack carries no duty to discharge");
}

// ---------------------------------------------------------------------------
// 4. The quantity
// ---------------------------------------------------------------------------

#[test]
fn unspent_credit_rides_into_the_pack_and_is_abandoned_by_the_discharge() {
    use qcore_compiler::{CleanupEvent, RegionExitTag};
    let module = compile(&program(
        "credit 7;\nqcore_check(false, Failure);",
        "(Failure) quant open R as pack",
        "discharge pack;",
    ))
    .expect("a credit remainder is abandoned, not leaked");
    let checked = module.checked.as_sequential().unwrap();
    let function = checked.functions.values().next().unwrap();
    let [state] = function.regions.states(&RegionExitTag::Normal) else {
        panic!("one normal exit");
    };
    assert_eq!(state.cleanup_trace(), &[CleanupEvent::AbandonCredit(7)]);
}

#[test]
fn spending_the_whole_credit_leaves_no_remainder() {
    use qcore_compiler::RegionExitTag;
    let module = compile(&program(
        "credit 4;\neffort 4;\nqcore_check(false, Failure);",
        "(Failure)",
        "",
    ))
    .expect("a fully spent credit leaves an empty pack");
    let checked = module.checked.as_sequential().unwrap();
    let function = checked.functions.values().next().unwrap();
    let [state] = function.regions.states(&RegionExitTag::Normal) else {
        panic!("one normal exit");
    };
    assert!(state.cleanup_trace().is_empty());
}

#[test]
fn spending_more_credit_than_the_region_declared_refuses() {
    let (code, message) = refusal(&program(
        "credit 3;\neffort 5;\nqcore_check(false, Failure);",
        "(Failure) quant open R as pack",
        "discharge pack;",
    ));
    assert_eq!(code, "Q302");
    assert_eq!(message, "insufficient strict credit");
}

#[test]
fn the_quantity_is_total_on_every_literal_the_form_admits() {
    // Totality, measured at the two ends the `u64` model has: zero and the
    // largest literal. Neither overflows and neither is special-cased.
    for amount in ["0", "18446744073709551615"] {
        compile(&program(
            &format!("credit {amount};\neffort {amount};\nqcore_check(false, Failure);"),
            "(Failure)",
            "",
        ))
        .unwrap_or_else(|errors| panic!("`credit {amount}` must be admitted: {errors:?}"));
    }
}

// ---------------------------------------------------------------------------
// 5. The v1 boundary, refused by name rather than left silent
// ---------------------------------------------------------------------------

#[test]
fn a_region_that_exits_normally_is_refused_by_name() {
    // The model is path-sensitive: a region with no error edge reaches no
    // handler, so nothing decides the clause's obligations. Admitting it would
    // be silent rather than fail-closed.
    let (code, message) = refusal(&program(
        "let a = qcore_acquire(1, true, Failure);\ndrop a;",
        "(Failure)",
        "",
    ));
    assert_eq!(code, "Q302");
    assert!(message.contains("REGION-NORMAL-EXIT"), "{message}");
}

#[test]
fn a_second_catch_clause_is_refused_by_name() {
    let source = "error Failure { Failed; }\nerror Other { Bad; }\n\
        fn main() -> i32 {\n\
        error region R { qcore_check(false, Failure); }\n\
        catch (Failure) { }\n\
        catch (Other) { }\n\
        return 0; }\n";
    let (code, message) = refusal(source);
    assert_eq!(code, "Q302");
    assert!(message.contains("REGION-MULTIPLE-CATCH"), "{message}");
}

#[test]
fn a_second_region_in_one_entry_is_refused_by_name() {
    let source = "error Failure { Failed; }\n\
        fn main() -> i32 {\n\
        error region R { qcore_check(false, Failure); } catch (Failure) { }\n\
        error region S { qcore_check(false, Failure); } catch (Failure) { }\n\
        return 0; }\n";
    let (code, message) = refusal(source);
    assert_eq!(code, "Q302");
    assert!(message.contains("REGION-MULTIPLE"), "{message}");
}

#[test]
fn an_uncaught_raise_is_refused_by_name() {
    let (code, message) = refusal(&program("qcore_check(false, Other);", "(Failure)", ""));
    assert_eq!(code, "Q302");
    assert!(message.contains("REGION-PROPAGATE"), "{message}");
}

#[test]
fn a_failing_acquisition_as_the_error_edge_is_refused_by_name() {
    let (code, message) = refusal(&program(
        "let a = qcore_acquire(1, false, Failure);",
        "(Failure)",
        "",
    ));
    assert_eq!(code, "Q302");
    assert!(message.contains("REGION-FAILING-ACQUIRE"), "{message}");
}

#[test]
fn a_statement_after_the_decided_error_edge_is_refused() {
    let (code, message) = refusal(&program(
        "let a = qcore_acquire(1, true, Failure);\nqcore_check(false, Failure);\ndrop a;",
        "(Failure) quant open R as pack",
        "discharge pack;",
    ));
    assert_eq!(code, "Q302");
    assert!(message.contains("unreachable statement"), "{message}");
}

#[test]
fn a_handler_statement_other_than_a_discharge_is_refused_by_name() {
    let (code, message) = refusal(&program(
        ONE_RESOURCE,
        "(Failure) quant open R as pack",
        "discharge pack;\nreturn 0;",
    ));
    assert_eq!(code, "Q302");
    assert!(message.contains("REGION-HANDLER-BODY"), "{message}");
}

#[test]
fn a_region_inside_a_branch_arm_is_refused_by_name() {
    let source = "error Failure { Failed; }\n\
        fn main() -> i32 {\n\
        if (1 < 2) { error region R { qcore_check(false, Failure); } catch (Failure) { } }\n\
        else { }\n\
        return 0; }\n";
    let (code, message) = refusal(source);
    assert_eq!(code, "Q302");
    assert!(message.contains("REGION-IN-ARM"), "{message}");
}

#[test]
fn a_body_statement_outside_the_admitted_set_is_refused() {
    let (code, message) = refusal(&program(
        "let a = qcore_acquire(1, true, Failure);\nmove a;",
        "(Failure)",
        "",
    ));
    assert_eq!(code, "Q302");
    assert!(message.contains("region body"), "{message}");
}

#[test]
fn a_catch_pattern_that_is_not_one_error_type_name_is_refused() {
    let (code, message) = refusal(&program("qcore_check(false, Failure);", "(Failure e)", ""));
    assert_eq!(code, "Q302");
    assert!(
        message.contains("REGION-CATCH-PAYLOAD-BINDING"),
        "{message}"
    );
}

#[test]
fn an_undeclared_caught_error_refuses_in_the_typed_error_band() {
    let (code, message) = refusal(&program("qcore_check(false, Failure);", "(Missing)", ""));
    assert_eq!(code, "Q300", "a name that is not a declared error type");
    assert_eq!(message, "unknown error type `Missing`");
}

// ---------------------------------------------------------------------------
// 6. Other routes keep the refusals they had
// ---------------------------------------------------------------------------

#[test]
fn the_task_route_keeps_its_profile_refusal() {
    let source = "#import std.concurrency;\nconcurrency checked;\n\
        error Failure { Failed; }\n\
        fn main() -> i32 { thread scope s { } \
        error region R { qcore_check(false, Failure); } catch (Failure) { } return 0; }";
    let codes = compile(source)
        .map(|module| module.execution_profile)
        .expect_err("the scoped-task profile models no error region")
        .into_iter()
        .map(|diagnostic| diagnostic.code.to_owned())
        .collect::<Vec<_>>();
    // The task route reaches the statement through its own lowering and keeps
    // the `Q500` structural refusal it already had. `source.region.v1` is
    // selected nowhere but the flat sequential route, so this is unmoved.
    assert_eq!(codes, vec!["Q500"]);
}

// ---------------------------------------------------------------------------
// 7. CT21 and CT22 — the shapes, as workspace tests
// ---------------------------------------------------------------------------
//
// The CT manifest is **not** edited here: this card lands the capability and a
// mapping lane decides the rows. What the two shapes now measure is recorded
// so a lane can read it off a test rather than re-derive it.

/// **CT21**, *"quant opening discards positive access permission"*, expected
/// **Reject**.
///
/// The spec's sentence is that opening a pack "does not discard the pack's
/// contents". Here the pack holds one live resource — positive access
/// permission — and the handler opens it and walks away. The refusal is the
/// model's own `Q305`, and it is attributable to the construct the case names:
/// remove *only* the opening and the refusal becomes `Q306`; discharge the
/// capability and the module is admitted.
#[test]
fn ct21_opening_a_pack_and_discarding_it_refuses() {
    let discarded = program(ONE_RESOURCE, "(Failure) quant open R as pack", "");
    assert_eq!(refusal(&discarded).0, "Q305");

    // Attribution control: the same body, the same clause, one `discharge`.
    compile(&program(
        ONE_RESOURCE,
        "(Failure) quant open R as pack",
        "discharge pack;",
    ))
    .expect("the case's refusal is about discarding, not about the region");
}

/// **CT22**, *"quant opening discards parent-local unused credit"*, expected
/// **Accept**.
///
/// Credit is the one thing a discharge may discard, and it is discarded as an
/// explicit `AbandonCredit` event rather than by omission. The case's own
/// wording says *"after joins"*; this route has no joins, so what the shape
/// below establishes is the credit half. The residual is
/// `REGION-CREDIT-WITH-JOINS` — the same missing composition
/// `docs/reports/ct-wire-lane/README.md` §3.4 records for CT23.
#[test]
fn ct22_a_discharge_abandons_the_regions_unused_credit() {
    use qcore_compiler::{CleanupEvent, RegionExitTag};
    let module = compile(DISCHARGED_PACK).expect("the accepting direction");
    let checked = module.checked.as_sequential().unwrap();
    let function = checked.functions.values().next().unwrap();
    let [state] = function.regions.states(&RegionExitTag::Normal) else {
        panic!("one normal exit");
    };
    assert!(
        state
            .cleanup_trace()
            .contains(&CleanupEvent::AbandonCredit(3)),
        "the unused credit is abandoned explicitly: {:?}",
        state.cleanup_trace()
    );
}

// ---------------------------------------------------------------------------
// 8. Sanitized native run — the reference against the compiled program
// ---------------------------------------------------------------------------

/// Compile and run the flagship's emitted C under ASan/UBSan (plus LSan where
/// the platform provides it) and compare the program's own instrumentation
/// output with the reference observation.
///
/// The leak check is a **positive** control for this card specifically: the two
/// resources the pack holds are freed by the releases the discharge emits, so a
/// discharge that lost a duty would surface here as a leak rather than only as
/// a disagreement.
#[test]
fn the_native_run_of_a_discharged_region_matches_its_reference() {
    let module = compile(DISCHARGED_PACK).unwrap();
    let instrumented = sequential_observation::instrument(module.c_source.as_deref().unwrap());
    let source = instrumented.as_str();
    let reference = module.reference_observation.as_ref().unwrap();

    #[cfg(target_os = "windows")]
    let run = windows_c::run(source, None, &[]);

    #[cfg(not(target_os = "windows"))]
    let run = {
        use std::process::Command;
        let directory = std::env::temp_dir().join(format!(
            "qcore-region-{}-{}",
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
            "C compiler rejected the emitted region:\n{}",
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
        "the emitted region failed under ASan/UBSan/LSan:\n{}",
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

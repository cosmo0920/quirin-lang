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

//! End-to-end route selection, admission and refusal for
//! `source.extern-call.v1` (card `source.extern-call.v1`).
//!
//! Every contract fixture here lives under `tests/fixtures/`, never under
//! `the canonical contract corpus/`, so the pinned corpus counts are untouched by this card.
#[cfg(not(target_os = "windows"))]
#[path = "../../tests/support/diagnostic_sink.rs"]
mod diagnostic_sink;

use std::collections::BTreeMap;

use qcore_compiler::{
    CompileOptions, SourceExecutionProfile, SourceFile, compile_files, extern_call,
};

const CONTRACTS: &str = include_str!("fixtures/extern_calls/contracts/buffer_api.qv");
const ROUND_TRIP: &str = include_str!("fixtures/extern_calls/round_trip.qr");

// Exercise both checkout styles regardless of the host's Git configuration.
fn with_line_endings(source: &str) -> [String; 2] {
    let lf = source.replace("\r\n", "\n");
    let crlf = lf.replace('\n', "\r\n");
    [lf, crlf]
}

fn selection() -> Option<BTreeMap<String, String>> {
    Some(BTreeMap::from([(
        "fixtures/extern_calls/contracts/buffer_api.qcore".to_owned(),
        CONTRACTS.to_owned(),
    )]))
}

fn compile_with(
    source: &str,
    contracts: Option<BTreeMap<String, String>>,
) -> Result<qcore_compiler::CompiledModule, Vec<qcore_compiler::FileDiagnostic>> {
    compile_files(
        &[SourceFile::new("extern.qcore", source)],
        &CompileOptions {
            entry: None,
            extern_contracts: contracts,
            native_libraries: None,
            native_library_headers: None,
        },
    )
}

fn codes(source: &str, contracts: Option<BTreeMap<String, String>>) -> Vec<String> {
    compile_with(source, contracts)
        .map(|module| module.execution_profile)
        .expect_err("this fixture must be refused")
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect()
}

// --- The accepted round trip. ---

#[test]
fn a_declared_round_trip_selects_the_extern_call_profile() {
    let module = compile_with(ROUND_TRIP, selection()).unwrap();
    assert_eq!(
        module.execution_profile,
        SourceExecutionProfile::ExternCallsV1
    );
    assert_eq!(module.entry.as_deref(), Some("main"));
    assert!(module.checked.as_sequential().is_none());
    assert!(module.checked.as_scoped_tasks().is_none());
    assert!(module.checked.as_checked_mutex_guards().is_none());
    let checked = module
        .checked
        .as_extern_calls()
        .expect("the checked artifact variant is explicit for this route");
    assert_eq!(checked.certificate.acquisitions, vec![1]);
    assert_eq!(checked.certificate.consumptions, vec![1]);
    // The declared contract of every callee, and the two structural premises,
    // are named on the certificate rather than left implicit.
    assert!(
        checked
            .certificate
            .assumptions
            .contains(&extern_call::ExternAssumption::NoExtentTransport)
    );
    assert!(
        checked
            .certificate
            .assumptions
            .contains(&extern_call::ExternAssumption::LinkageSuppliedByBuilder)
    );
    assert!(checked.certificate.assumptions.contains(
        &extern_call::ExternAssumption::DeclaredContract("qcore_extern_alloc".to_owned())
    ));
    // The sequential passes ran over the same lowering, with the declared
    // extern summaries as call-site assumptions.
    assert_eq!(checked.sequential.functions.len(), 1);
    assert_eq!(checked.lowered.extern_summaries.len(), 4);
}

#[test]
fn the_published_certificate_is_independently_rederivable() {
    let module = compile_with(ROUND_TRIP, selection()).unwrap();
    let checked = module.checked.as_extern_calls().unwrap();
    extern_call::recheck_extern_certificate(&checked.lowered.program, &checked.certificate)
        .expect("the driver publishes only evidence it rederived");
    let mut tampered = checked.certificate.clone();
    tampered.acquisitions.clear();
    assert!(
        extern_call::recheck_extern_certificate(&checked.lowered.program, &tampered).is_err(),
        "a tampered certificate must not survive an independent recheck"
    );
}

#[test]
fn emission_calls_the_declared_symbols_and_the_reference_run_agrees() {
    let module = compile_with(ROUND_TRIP, selection()).unwrap();
    let c = module.c_source.as_deref().expect("this route emits C11");
    for expected in [
        "uint8_t *qcore_extern_alloc(uintptr_t p0);",
        "void qcore_extern_release(uint8_t *p0);",
        "uint8_t *r1 = qcore_extern_alloc(8);",
        "qcore_extern_release(r1);",
    ] {
        assert!(c.contains(expected), "emitted C is missing `{expected}`");
    }
    assert_eq!(
        module.extern_observation.as_deref().unwrap(),
        [
            "acquire qcore_extern_alloc r1 ok",
            "call qcore_extern_fill b:r1 d:7",
            "call qcore_extern_sum b:r1 d:8",
            "call qcore_extern_release c:r1",
            "return",
        ]
    );
    // The sequential observation field models no extern call and stays empty.
    assert!(module.reference_observation.is_none());
}

#[test]
fn the_failing_arm_of_the_adapter_is_exercisable_from_source() {
    let source = ROUND_TRIP.replace(
        "qcore_extern_alloc(8) succeeds true",
        "qcore_extern_alloc_failing(8) succeeds false",
    );
    let source = source.replace(
        "extern fn qcore_extern_alloc(size: usize) -> ptr<mut, u8>;",
        "extern fn qcore_extern_alloc_failing(size: usize) -> ptr<mut, u8>;",
    );
    let module = compile_with(&source, selection()).unwrap();
    assert_eq!(
        module.extern_observation.as_deref().unwrap(),
        [
            "acquire qcore_extern_alloc_failing r1 null",
            "raise Failure",
        ]
    );
}

/// The full source stage stack for one accepted fixture, on a Unix target:
/// parse, elaborate, lower, check, emit, link against a **real C
/// implementation** of every declared callee, and run under ASan/UBSan/LSan,
/// comparing the native observation with the independently computed reference
/// execution.
///
/// One target, one compiler invocation, one process: this is evidence about
/// these two traces and this implementation, not about the declared contracts
/// in general, about buffer extents, or about any other callee.
#[cfg(not(target_os = "windows"))]
#[test]
fn accepted_fixtures_run_natively_against_a_linked_c_implementation() {
    for (label, source) in [
        ("success", ROUND_TRIP.to_owned()),
        ("null-arm", {
            let source = ROUND_TRIP.replace(
                "qcore_extern_alloc(8) succeeds true",
                "qcore_extern_alloc_failing(8) succeeds false",
            );
            source.replace(
                "extern fn qcore_extern_alloc(size: usize) -> ptr<mut, u8>;",
                "extern fn qcore_extern_alloc_failing(size: usize) -> ptr<mut, u8>;",
            )
        }),
        // Card `EXTERN-ERR-REL`. The store's success path allocates an owner
        // and a child and releases both through the owner; the failure path
        // takes the disposition the checker required, so LSan accounts for
        // both allocations on either path.
        ("store", STORED.to_owned()),
        ("store-failure", STORED_FAILURE.to_owned()),
        // Card `EXTERN-MODES`. The release fixture links a callee whose
        // declaration claims discharge and whose implementation performs it;
        // the two reallocation fixtures execute both arms of contract spec
        // §C04.3 against a real `realloc`, so the preserved-subject arm is a
        // real observation and its checked disposition is a real `free` — a
        // double free there is what ASan would report.
        ("released", RELEASED.to_owned()),
        ("realloc", REALLOC.to_owned()),
        ("realloc-failure", REALLOC_FAILURE.to_owned()),
        // Card `PIPE-SCALAR-RESULT`. The two scalar bindings sit inside the
        // pinned round trip's own custody, so this run is a *negative* control
        // for the widening: if binding a scalar had touched the ledger, LSan
        // would report the buffer this fixture still allocates and releases.
        ("scalar", SCALAR_RESULT.to_owned()),
        // Card `EXTERN-SCALAR-OUTCOME` (trace half). The same fixture with both
        // bindings' outcomes declared, so the two `-> N` fields the reference
        // computes from the declaration are compared against a run that took
        // the numbers from the linked C — and that run exits 70 rather than
        // agreeing, if they differ.
        ("scalar-outcome", declared_outcomes()),
    ] {
        let module = compile_with(&source, selection()).unwrap();
        let quiet = run_native_status(
            module.c_source.as_deref().unwrap(),
            &format!("{label}-quiet"),
        );
        assert_eq!(quiet.0, Some(0), "{quiet:?}");
        assert!(quiet.1.is_empty() && quiet.2.is_empty(), "{quiet:?}");
        let reference = module.extern_observation.clone().unwrap();
        assert_eq!(
            run_native(module.c_source.as_deref().unwrap(), label),
            reference,
            "native and reference observations must agree line for line ({label})"
        );
    }
}

#[cfg(not(target_os = "windows"))]
fn run_native(source: &str, label: &str) -> Vec<String> {
    let instrumented = diagnostic_sink::instrument(source);
    let source = instrumented.as_str();
    use std::process::Command;
    let directory = std::env::temp_dir().join(format!(
        "qcore-extern-call-{label}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&directory).unwrap();
    let generated = directory.join("generated.c");
    let callees = directory.join("buffer_api.c");
    let binary = directory.join("generated");
    std::fs::write(&generated, source).unwrap();
    std::fs::write(&callees, include_str!("fixtures/extern_calls/buffer_api.c")).unwrap();
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
        .arg(&callees)
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
    let observation = String::from_utf8(run.stdout)
        .unwrap()
        .lines()
        .map(ToOwned::to_owned)
        .collect();
    std::fs::remove_dir_all(directory).unwrap();
    observation
}

// --- Q022: the file-local import gate. ---

#[test]
fn calling_an_extern_function_without_the_import_refuses() {
    for source in with_line_endings(ROUND_TRIP) {
        let mutated = source.replace("#import std.extern;", "");
        assert_ne!(mutated, source, "the import must have been removed");
        assert_eq!(codes(&mutated, selection()), ["Q022"]);
    }
}

#[test]
fn a_misspelled_module_refuses_as_an_unknown_module() {
    let source = ROUND_TRIP.replace("#import std.extern;", "#import std.externs;");
    assert_eq!(codes(&source, selection()), ["Q014"]);
}

#[test]
fn declaring_an_extern_function_without_calling_it_needs_no_import() {
    // The gate is about the *call*, so an unchanged declaration-only file keeps
    // its existing behavior and never routes here.
    let source = "extern fn qcore_extern_alloc(size: usize) -> ptr<mut, u8>;\n\
                  fn main() -> unit { return; }\n";
    let module = compile_with(source, None).unwrap();
    assert_eq!(
        module.execution_profile,
        SourceExecutionProfile::SequentialCleanupV1
    );
}

#[test]
fn importing_the_module_in_another_file_does_not_grant_this_one_the_call() {
    for source in with_line_endings(ROUND_TRIP) {
        let granted = SourceFile::new("granted.qcore", "module library;\n#import std.extern;\n");
        let mutated = source.replace("#import std.extern;", "");
        assert_ne!(
            mutated, source,
            "the caller's import must have been removed"
        );
        let caller = SourceFile::new("caller.qcore", mutated);
        let errors = compile_files(
            &[granted, caller],
            &CompileOptions {
                entry: None,
                extern_contracts: selection(),
                native_libraries: None,
                native_library_headers: None,
            },
        )
        .unwrap_err();
        assert_eq!(errors[0].code, "Q022");
        assert_eq!(errors[0].location.path, "caller.qcore");
    }
}

// --- Q023: no admitted declared contract. ---

#[test]
fn calling_with_no_contract_selection_refuses() {
    assert_eq!(codes(ROUND_TRIP, None), ["Q023"]);
}

#[test]
fn a_selection_without_the_callee_refuses() {
    let partial = CONTRACTS
        .split("extern fn qcore_extern_release")
        .next()
        .expect("the fixture contains the release declaration")
        .to_owned();
    assert_eq!(
        codes(
            ROUND_TRIP,
            Some(BTreeMap::from([("partial.qr".to_owned(), partial)]))
        ),
        ["Q023"]
    );
}

#[test]
fn a_selection_that_does_not_load_refuses() {
    let broken = CONTRACTS.replace("requires resource_contract_v1;", "requires something_else;");
    assert_eq!(
        codes(
            ROUND_TRIP,
            Some(BTreeMap::from([("broken.qcore".to_owned(), broken)]))
        ),
        ["Q023"]
    );
}

#[test]
fn a_duplicate_declaration_across_two_selected_modules_refuses() {
    assert_eq!(
        codes(
            ROUND_TRIP,
            Some(BTreeMap::from([
                ("a.qcore".to_owned(), CONTRACTS.to_owned()),
                ("b.qcore".to_owned(), CONTRACTS.to_owned()),
            ]))
        ),
        ["Q023"]
    );
}

// --- Q024: the closed bridging table, at the source boundary. ---

/// Card `EXTERN-MODES` moved this pin deliberately. Its original substitution
/// was `resource_released(buffer, nonnull)`, which is now admitted; the
/// admitting direction is pinned by `a_released_argument_is_recorded_apart_from_
/// a_consumed_one` and its nullable form still refuses here. The negative half
/// this test carries — an excluded C04 mode refuses at the source boundary
/// under `Q024`, not somewhere later — is kept on the nullable consumed mode,
/// which the closed table has always excluded.
#[test]
fn an_excluded_contract_mode_refuses_at_the_bridging_table() {
    let contracts = CONTRACTS.replace(
        "ensures normal resource_consumed(buffer, nonnull);",
        "ensures normal resource_consumed(buffer, nullable);",
    );
    assert_eq!(
        codes(
            ROUND_TRIP,
            Some(BTreeMap::from([("nullable.qcore".to_owned(), contracts)]))
        ),
        ["Q024"]
    );
}

#[test]
fn a_nonnull_owned_return_refuses() {
    let contracts = CONTRACTS.replacen(
        "ensures normal resource_return_owned(nullable);",
        "ensures normal resource_return_owned(nonnull);",
        1,
    );
    assert_eq!(
        codes(
            ROUND_TRIP,
            Some(BTreeMap::from([("nonnull.qcore".to_owned(), contracts)]))
        ),
        ["Q024"]
    );
}

#[test]
fn moving_where_the_contract_declares_a_loan_refuses() {
    let source = ROUND_TRIP.replace("qcore_extern_fill(b, 7);", "qcore_extern_fill(move b, 7);");
    assert_eq!(codes(&source, selection()), ["Q024"]);
}

#[test]
fn a_wrong_argument_count_refuses() {
    let source = ROUND_TRIP.replace("qcore_extern_fill(b, 7);", "qcore_extern_fill(b);");
    assert_eq!(codes(&source, selection()), ["Q024"]);
}

#[test]
fn using_a_consumed_binding_again_refuses() {
    let source = ROUND_TRIP.replace(
        "qcore_extern_release(move b);",
        "qcore_extern_release(move b);\n    qcore_extern_fill(b, 1);",
    );
    assert_eq!(codes(&source, selection()), ["Q024"]);
}

// --- Q025: the mandatory nullable-owned-return adapter. ---

#[test]
fn an_unbranched_owned_return_refuses() {
    let source = "#import std.extern;\n\
         error Failure { Failed; }\n\
         extern fn qcore_extern_alloc(size: usize) -> ptr<mut, u8>;\n\
         fn main() -> unit effects { io, alloc } {\n    \
         qcore_extern_alloc(8);\n    return;\n}\n";
    let codes = codes(source, selection());
    assert!(codes.contains(&"Q025".to_owned()), "{codes:?}");
}

#[test]
fn an_adapter_on_a_callee_that_returns_no_custody_refuses() {
    let source = ROUND_TRIP.replace(
        "qcore_extern_sum(b, 8);",
        "let s = qcore_extern_sum(b, 8) succeeds true else raise Failure;",
    );
    let codes = codes(&source, selection());
    assert!(codes.contains(&"Q025".to_owned()), "{codes:?}");
}

#[test]
fn an_adapter_raising_an_unknown_name_refuses_at_name_resolution() {
    // Name resolution runs first and owns this condition; the extern pass does
    // not duplicate it under a second code.
    let source = ROUND_TRIP.replace("else raise Failure;", "else raise Absent;");
    assert_eq!(codes(&source, selection()), ["Q013"]);
}

#[test]
fn an_adapter_raising_a_declared_name_that_is_not_an_error_type_refuses() {
    let source = ROUND_TRIP.replace("else raise Failure;", "else raise qcore_extern_fill;");
    let codes = codes(&source, selection());
    assert!(codes.contains(&"Q025".to_owned()), "{codes:?}");
}

#[test]
fn an_adapter_that_would_abandon_live_custody_refuses() {
    let source = ROUND_TRIP.replace(
        "qcore_extern_fill(b, 7);",
        "let c = qcore_extern_alloc(4) succeeds true else raise Failure;",
    );
    let codes = codes(&source, selection());
    assert!(codes.contains(&"Q025".to_owned()), "{codes:?}");
}

// --- Q026: refused contexts. ---

#[test]
fn an_extern_call_inside_a_nested_block_refuses() {
    let source = ROUND_TRIP.replace(
        "qcore_extern_fill(b, 7);",
        "if (true) { qcore_extern_fill(b, 7); }",
    );
    let codes = codes(&source, selection());
    assert!(codes.contains(&"Q026".to_owned()), "{codes:?}");
}

#[test]
fn an_extern_call_in_a_second_source_function_refuses() {
    let source = format!(
        "{ROUND_TRIP}\nfn helper() -> unit effects {{ io, alloc, mut }} {{\n    \
         let c = qcore_extern_alloc(1) succeeds true else raise Failure;\n    \
         qcore_extern_release(move c);\n    return;\n}}\n"
    );
    let codes = codes(&source, selection());
    assert!(codes.contains(&"Q026".to_owned()), "{codes:?}");
}

#[test]
fn a_concurrent_module_keeps_its_existing_refusal() {
    // The task route is selected first and refuses the extern call with its own
    // message; this card lifts nothing there.
    let source = "#import std.concurrency;\n#import std.extern;\n\
         extern fn qcore_extern_alloc(size: usize) -> ptr<mut, u8>;\n\
         fn main() -> unit {\n    thread scope s {\n    \
         let b = qcore_extern_alloc(8) succeeds true else raise Failure;\n    }\n}\n";
    let codes = codes(source, selection());
    assert!(
        !codes.contains(&"Q023".to_owned()) && !codes.contains(&"Q025".to_owned()),
        "the extern route must not claim a concurrent module: {codes:?}"
    );
}

// --- Q027: declaration and contract must agree. ---

#[test]
fn a_source_declaration_that_disagrees_with_its_contract_refuses() {
    let source = ROUND_TRIP.replace(
        "extern fn qcore_extern_fill(buffer: ptr<mut, u8>, value: u32) -> unit;",
        "extern fn qcore_extern_fill(buffer: ptr<mut, u8>, value: u64) -> unit;",
    );
    assert_eq!(codes(&source, selection()), ["Q027"]);
}

#[test]
fn a_renamed_parameter_refuses() {
    let source = ROUND_TRIP.replace(
        "extern fn qcore_extern_release(buffer: ptr<mut, u8>) -> unit;",
        "extern fn qcore_extern_release(other: ptr<mut, u8>) -> unit;",
    );
    assert_eq!(codes(&source, selection()), ["Q027"]);
}

#[test]
fn an_inline_contract_on_a_called_extern_declaration_refuses() {
    let source = ROUND_TRIP.replace(
        "extern fn qcore_extern_release(buffer: ptr<mut, u8>) -> unit;",
        "extern fn qcore_extern_release(buffer: ptr<mut, u8>) -> unit contract { requires resource_contract_v1; };",
    );
    assert_eq!(codes(&source, selection()), ["Q027"]);
}

#[test]
fn a_mutated_contract_file_refuses_a_previously_admitted_module() {
    // The same source, the same callee names, a silently widened contract: the
    // module that checked before must not check now.
    // Normalize only the mutation baseline so this multiline match also works
    // when Git checked out the contract with CRLF line endings.
    let contracts = CONTRACTS.replace("\r\n", "\n");
    let mutated = contracts.replace(
        "ensures normal resource_borrowed(buffer);\n    ensures normal resource_borrowed(value);",
        "ensures normal resource_consumed(buffer, nonnull);\n    ensures normal resource_borrowed(value);",
    );
    assert_ne!(mutated, contracts, "the mutation must have applied");
    for mutated in with_line_endings(&mutated) {
        let errors = codes(
            ROUND_TRIP,
            Some(BTreeMap::from([("mutated.qcore".to_owned(), mutated)])),
        );
        // `fill` now consumes `b`, so the later loan and move cannot both stand.
        assert!(
            !errors.is_empty(),
            "a widened contract must not be admitted silently"
        );
    }
}

// --- The effect row the caller declares must admit the derived one. ---

#[test]
fn an_entry_with_no_effect_row_refuses_every_extern_call() {
    let source = ROUND_TRIP.replace("effects { io, alloc, mut } ", "");
    let codes = codes(&source, selection());
    assert!(codes.contains(&"Q300".to_owned()), "{codes:?}");
}

#[test]
fn an_entry_row_missing_alloc_refuses_an_acquiring_call() {
    let source = ROUND_TRIP.replace("effects { io, alloc, mut }", "effects { io, mut }");
    let codes = codes(&source, selection());
    assert!(codes.contains(&"Q300".to_owned()), "{codes:?}");
}

#[test]
fn an_unmodeled_effect_name_refuses() {
    let source = ROUND_TRIP.replace("effects { io, alloc, mut }", "effects { io, alloc, panic }");
    let codes = codes(&source, selection());
    assert!(codes.contains(&"Q020".to_owned()), "{codes:?}");
}

// --- Leaks stay with the passes that own them. ---

#[test]
fn an_unreleased_owned_return_refuses_under_the_established_custody_pass() {
    for source in with_line_endings(ROUND_TRIP) {
        let mutated = source.replace("qcore_extern_release(move b);", "");
        assert_ne!(mutated, source, "the release must have been removed");
        let codes = codes(&mutated, selection());
        assert!(
            codes.iter().any(|code| code == "Q103" || code == "Q202"),
            "a leak is reported by the custody/cleanup passes, not by a second extern code: {codes:?}"
        );
    }
}

// ==================================================================== //
// Card EXTERN-F1: the `resource_out` and `resource_stored` call modes. //
// ==================================================================== //

const OUT_SLOT: &str = include_str!("fixtures/extern_calls/out_slot.qr");
const STORED: &str = include_str!("fixtures/extern_calls/stored.qr");
const STORED_FAILURE: &str = include_str!("fixtures/extern_calls/stored_failure.qr");

/// The out fixture with its acquiring callee swapped for the always-refusing
/// implementation and the declared control flipped, so the null arm is a real
/// observation rather than a claim.
fn out_slot_null_arm() -> String {
    let source = OUT_SLOT.replace(
        "qcore_extern_open(out, 8) succeeds true",
        "qcore_extern_open_failing(out, 8) succeeds false",
    );
    source.replace(
        "extern fn qcore_extern_open(slot: ptr<mut, ptr<mut, u8>>, size: usize) -> i32;",
        "extern fn qcore_extern_open_failing(slot: ptr<mut, ptr<mut, u8>>, size: usize) -> i32;",
    )
}

// --- resource_out, end to end. ---

#[test]
fn an_out_slot_round_trip_selects_the_extern_call_profile() {
    let module = compile_with(OUT_SLOT, selection()).unwrap();
    assert_eq!(
        module.execution_profile,
        SourceExecutionProfile::ExternCallsV1
    );
    let checked = module.checked.as_extern_calls().unwrap();
    // The out write is the acquisition, and the certificate records which
    // source established it rather than only that something did.
    assert_eq!(checked.certificate.acquisitions, vec![1]);
    assert_eq!(checked.certificate.out_acquisitions, vec![1]);
    assert_eq!(checked.certificate.consumptions, vec![1]);
    assert!(checked.certificate.stored.is_empty());
    assert!(checked.certificate.assumptions.contains(
        &extern_call::ExternAssumption::OutSlotWrittenByCallee("qcore_extern_open".to_owned())
    ));
    extern_call::recheck_extern_certificate(&checked.lowered.program, &checked.certificate)
        .expect("the driver publishes only evidence it rederived");
}

#[test]
fn out_slot_emission_takes_the_slots_address_and_the_reference_run_agrees() {
    let module = compile_with(OUT_SLOT, selection()).unwrap();
    let c = module.c_source.as_deref().expect("this route emits C11");
    for expected in [
        "int32_t qcore_extern_open(uint8_t **p0, uintptr_t p1);",
        "uint8_t *r1 = NULL;",
        "(void) qcore_extern_open(&r1, 8);",
        "if ((r1 != NULL) != 1)",
        "QCORE_DIAG_EXTERN_OUTCOME",
        "qcore_extern_release(r1);",
    ] {
        assert!(c.contains(expected), "emitted C is missing `{expected}`");
    }
    assert_eq!(
        module.extern_observation.as_deref().unwrap(),
        [
            "out qcore_extern_open o:r1 d:8 ok",
            "call qcore_extern_fill b:r1 d:7",
            "call qcore_extern_sum b:r1 d:8",
            "call qcore_extern_release c:r1",
            "return",
        ]
    );
}

/// The out slot materializes a binding only under the success control. On the
/// declared failure arm the trace raises immediately, so nothing in the rest of
/// the body — three statements that all read `b` — is ever observed.
#[test]
fn the_failing_arm_of_an_out_slot_binds_nothing_and_reads_nothing() {
    let module = compile_with(&out_slot_null_arm(), selection()).unwrap();
    assert_eq!(
        module.extern_observation.as_deref().unwrap(),
        [
            "out qcore_extern_open_failing o:r1 d:8 null",
            "raise Failure",
        ]
    );
}

/// Both out arms through the full stage stack on a Unix target: parse,
/// elaborate, lower, check, emit, link against a **real C implementation** of
/// every declared callee, and run under ASan/UBSan/LSan with the native
/// observation compared to the independently computed reference execution.
///
/// Scope: one target, one compiler invocation, one process and these two
/// traces. It is not evidence about the declared contracts in general, about
/// buffer extents, or about a callee that writes its slot differently — that
/// last case is exactly what the emitted status-70 check refuses to hide.
#[cfg(not(target_os = "windows"))]
#[test]
fn out_slot_fixtures_run_natively_against_a_linked_c_implementation() {
    for (label, source) in [
        ("out-success", OUT_SLOT.to_owned()),
        ("out-null", out_slot_null_arm()),
    ] {
        let module = compile_with(&source, selection()).unwrap();
        let reference = module.extern_observation.clone().unwrap();
        assert_eq!(
            run_native(module.c_source.as_deref().unwrap(), label),
            reference,
            "native and reference observations must agree line for line ({label})"
        );
    }
}

// --- resource_out: refusals. ---

#[test]
fn a_nonnull_out_slot_refuses_at_the_bridging_table() {
    let contracts = CONTRACTS.replacen(
        "ensures normal resource_out(slot, nullable);",
        "ensures normal resource_out(slot, nonnull);",
        1,
    );
    assert_eq!(
        codes(
            OUT_SLOT,
            Some(BTreeMap::from([(
                "nonnull-out.qcore".to_owned(),
                contracts
            )]))
        ),
        ["Q024"]
    );
}

#[test]
fn an_out_call_without_the_adapter_refuses() {
    let source = "#import std.extern;\n\
         error Failure { Failed; }\n\
         extern fn qcore_extern_open(slot: ptr<mut, ptr<mut, u8>>, size: usize) -> i32;\n\
         fn main() -> unit effects { io, alloc, mut } {\n    \
         qcore_extern_open(out, 8);\n    return;\n}\n";
    let codes = codes(source, selection());
    assert!(codes.contains(&"Q025".to_owned()), "{codes:?}");
}

#[test]
fn passing_a_binding_where_the_contract_declares_the_out_slot_refuses() {
    let source = OUT_SLOT.replace(
        "let b = qcore_extern_open(out, 8) succeeds true else raise Failure;",
        "let a = qcore_extern_open(out, 8) succeeds true else raise Failure;\n    \
         let b = qcore_extern_open(a, 8) succeeds true else raise Failure;",
    );
    let codes = codes(&source, selection());
    assert!(codes.contains(&"Q024".to_owned()), "{codes:?}");
}

/// `out` names a parameter position, never a binding, so it cannot be smuggled
/// into an ordinary argument slot: at a declared loan it is simply not a live
/// resource binding.
#[test]
fn the_out_marker_is_not_an_argument_anywhere_else() {
    let source = OUT_SLOT.replace("qcore_extern_fill(b, 7);", "qcore_extern_fill(out, 7);");
    assert_eq!(codes(&source, selection()), ["Q024"]);
}

#[test]
fn a_mutated_out_contract_refuses_a_previously_admitted_module() {
    // The admitted module stops being admitted when its selected contract is
    // silently widened to hand the slot back as a plain loan.
    compile_with(OUT_SLOT, selection()).expect("the unmutated module is admitted");
    let contracts = CONTRACTS.replacen(
        "ensures normal resource_out(slot, nullable);",
        "ensures normal resource_borrowed(slot);",
        1,
    );
    let codes = codes(
        OUT_SLOT,
        Some(BTreeMap::from([(
            "loaned-slot.qcore".to_owned(),
            contracts,
        )])),
    );
    assert!(codes.contains(&"Q024".to_owned()), "{codes:?}");
}

// --- resource_stored, end to end (card `EXTERN-ERR-REL`). ---

/// The two statements a store needs, as written in the fixture. The store
/// tests below substitute *into* this block, so a change to the fixture's
/// spelling fails them loudly instead of silently matching nothing.
const STORE_BLOCK: &str = "    let child = qcore_extern_alloc(8) succeeds true\n        \
     else qcore_extern_release(move owner) raise Failure;\n    \
     qcore_extern_attach(owner, move child);";

/// EXTERN-F1 measured the store to be admitted by the bridging table but
/// unreachable from a single source module: a store needs two simultaneously
/// live checked identities, and that card's adapter error arm could abandon
/// nothing. With the arm's disposition discipline the second acquisition is
/// admitted, so the store is now reached, checked and executed.
#[test]
fn a_store_is_reachable_once_the_error_arm_disposes_of_the_other_identity() {
    assert!(
        STORED.replace("\r\n", "\n").contains(STORE_BLOCK),
        "the store fixture no longer contains the block the store tests substitute into"
    );
    let module = compile_with(STORED, selection()).unwrap();
    assert_eq!(
        module.execution_profile,
        SourceExecutionProfile::ExternCallsV1
    );
    let checked = module.checked.as_extern_calls().unwrap();
    // Two acquisitions, the child stored beneath the owner, and the owner
    // consumed at the end: the shape §C04.2 requires, actually checked.
    assert_eq!(checked.certificate.acquisitions, vec![1, 2]);
    assert_eq!(checked.certificate.stored, vec![(2, 1)]);
    assert_eq!(checked.certificate.consumptions, vec![1]);
    // The first acquisition abandons nothing because nothing else is live; the
    // second disposes of exactly the owner. The certificate keeps both, so a
    // reader can tell an empty arm from an absent one.
    assert_eq!(
        checked.certificate.error_dispositions,
        vec![(1, Vec::new()), (2, vec![1])]
    );
    assert!(checked.certificate.assumptions.contains(
        &extern_call::ExternAssumption::StoredCustodyUnderDeclaredOwner(
            "qcore_extern_attach".to_owned()
        )
    ));
    extern_call::recheck_extern_certificate(&checked.lowered.program, &checked.certificate)
        .expect("the driver publishes only evidence it rederived");
}

#[test]
fn a_store_emits_its_owner_and_child_and_the_reference_run_agrees() {
    let module = compile_with(STORED, selection()).unwrap();
    let c = module.c_source.as_deref().expect("this route emits C11");
    for expected in [
        "void qcore_extern_attach(uint8_t *p0, uint8_t *p1);",
        "uint8_t *r1 = qcore_extern_alloc(16);",
        "uint8_t *r2 = qcore_extern_alloc(8);",
        "qcore_extern_attach(r1, r2);",
        "qcore_extern_release_owner(r1);",
        // The second acquisition's failure branch renders its cleanup edge.
        "        qcore_extern_release(r1);",
    ] {
        assert!(c.contains(expected), "emitted C is missing `{expected}`");
    }
    assert_eq!(
        module.extern_observation.as_deref().unwrap(),
        [
            "acquire qcore_extern_alloc r1 ok",
            "acquire qcore_extern_alloc r2 ok",
            "call qcore_extern_attach w:r1 s:r2",
            "call qcore_extern_release_owner c:r1",
            "return",
        ]
    );
}

/// The store-then-callee-fails path: after the store only the owner is live, so
/// the failing acquisition's arm disposes of exactly the owner and the trace
/// ends at the raise. Nothing after the raise is observed.
#[test]
fn the_store_failure_path_releases_the_owner_before_raising() {
    let module = compile_with(STORED_FAILURE, selection()).unwrap();
    let checked = module.checked.as_extern_calls().unwrap();
    assert_eq!(
        checked.certificate.error_dispositions,
        vec![(1, Vec::new()), (2, vec![1]), (3, vec![1])]
    );
    assert_eq!(
        module.extern_observation.as_deref().unwrap(),
        [
            "acquire qcore_extern_alloc r1 ok",
            "acquire qcore_extern_alloc r2 ok",
            "call qcore_extern_attach w:r1 s:r2",
            "acquire qcore_extern_alloc_failing r3 null",
            "call qcore_extern_release_owner c:r1",
            "raise Failure",
        ]
    );
}

/// The disposition belongs to the error arm and to no other path: the success
/// path keeps every identity the arm would have released, which is exactly why
/// the store two statements later can still name its owner.
#[test]
fn a_disposition_does_not_consume_the_identity_on_the_success_arm() {
    let module = compile_with(STORED, selection()).unwrap();
    let checked = module.checked.as_extern_calls().unwrap();
    // `owner` is released once, at the end, on the success path. The arm's
    // release of the same identity is not a second consumption.
    assert_eq!(checked.certificate.consumptions, vec![1]);
    assert_eq!(checked.certificate.error_dispositions[1].1, vec![1]);
}

// --- Adversarial: the abandonment refusal is narrowed, not weakened. ---

/// The v1 refusal with its meaning intact. An acquisition whose arm names no
/// disposition still abandons the live owner and is still refused.
#[test]
fn an_acquisition_whose_arm_disposes_of_nothing_still_refuses() {
    let stored = STORED.replace("\r\n", "\n");
    let source = stored.replace(
        "succeeds true\n        else qcore_extern_release(move owner) raise Failure;",
        "succeeds true else raise Failure;",
    );
    assert_ne!(source, stored, "the arm must have been emptied");
    let codes = codes(&source, selection());
    assert!(codes.contains(&"Q025".to_owned()), "{codes:?}");
}

#[test]
fn releasing_the_same_identity_twice_on_one_arm_refuses() {
    let source = STORED.replace(
        "else qcore_extern_release(move owner) raise Failure;",
        "else qcore_extern_release(move owner) qcore_extern_release(move owner) raise Failure;",
    );
    let codes = codes(&source, selection());
    assert!(codes.contains(&"Q025".to_owned()), "{codes:?}");
}

/// The stored child left this body's ledger permanently, so a later arm cannot
/// dispose of it — that would release custody the declaration already handed
/// to the owner.
#[test]
fn releasing_a_stored_identity_on_a_later_arm_refuses() {
    let source = STORED_FAILURE.replace(
        "else qcore_extern_release_owner(move owner) raise Failure;",
        "else qcore_extern_release(move child) raise Failure;",
    );
    let codes = codes(&source, selection());
    assert!(codes.contains(&"Q024".to_owned()), "{codes:?}");
}

/// An arm that acquires opens an obligation on a path taken because something
/// already failed, and has no adapter of its own; refused by name.
#[test]
fn an_arm_that_acquires_refuses() {
    let source = STORED.replace(
        "else qcore_extern_release(move owner) raise Failure;",
        "else qcore_extern_alloc(4) raise Failure;",
    );
    let codes = codes(&source, selection());
    assert!(codes.contains(&"Q025".to_owned()), "{codes:?}");
}

/// An arm that stores converts an obligation into a declared premise on a
/// failure path instead of discharging it.
#[test]
fn an_arm_that_stores_refuses() {
    let source = STORED_FAILURE.replace(
        "else qcore_extern_release_owner(move owner) raise Failure;",
        "else qcore_extern_attach(owner, move owner) raise Failure;",
    );
    let codes = codes(&source, selection());
    assert!(
        codes.contains(&"Q025".to_owned()) || codes.contains(&"Q024".to_owned()),
        "{codes:?}"
    );
}

/// A disposition releases exactly one identity. A borrowing call discharges
/// nothing and is not a disposition, however harmless it looks.
#[test]
fn an_arm_call_that_releases_nothing_refuses() {
    let source = STORED.replace(
        "extern fn qcore_extern_release(buffer",
        "extern fn qcore_extern_fill(buffer: ptr<mut, u8>, value: u32) -> unit;\n\
         extern fn qcore_extern_release(buffer",
    );
    let source = source.replace(
        "else qcore_extern_release(move owner) raise Failure;",
        "else qcore_extern_fill(owner, 7) qcore_extern_release(move owner) raise Failure;",
    );
    let codes = codes(&source, selection());
    assert!(codes.contains(&"Q025".to_owned()), "{codes:?}");
}

/// The established cleanup edge releases in reverse acquisition order, so an
/// arm that states another order describes a different program than the
/// sequential lowering of the same module.
/// The store-free variant of the failure fixture: without the store, both the
/// owner and the child are live at the failing acquisition, so the arm must
/// dispose of two identities and the order is observable.
fn two_live_identities_at_the_failing_acquisition(arm: &str) -> String {
    let source = STORED_FAILURE.replace("\r\n", "\n");
    let source = source.replace("    qcore_extern_attach(owner, move child);\n", "");
    let source = source.replace(
        "    qcore_extern_release(move extra);\n",
        "    qcore_extern_release(move extra);\n    qcore_extern_release(move child);\n",
    );
    source.replace(
        "else qcore_extern_release_owner(move owner) raise Failure;",
        arm,
    )
}

#[test]
fn an_arm_whose_order_disagrees_with_the_cleanup_edge_refuses() {
    let source = two_live_identities_at_the_failing_acquisition(
        "else qcore_extern_release(move owner) qcore_extern_release(move child) raise Failure;",
    );
    let codes = codes(&source, selection());
    assert!(codes.contains(&"Q025".to_owned()), "{codes:?}");
}

/// The reverse-acquisition order the cleanup edge performs is admitted, so the
/// refusal above is about the order and not about two dispositions.
#[test]
fn an_arm_in_reverse_acquisition_order_is_admitted() {
    let source = two_live_identities_at_the_failing_acquisition(
        "else qcore_extern_release(move child) qcore_extern_release(move owner) raise Failure;",
    );
    let module = compile_with(&source, selection()).unwrap();
    let checked = module.checked.as_extern_calls().unwrap();
    assert_eq!(checked.certificate.error_dispositions[2].1, vec![2, 1]);
}

/// A published certificate is rederived, never believed: substituting the
/// recorded dispositions is refused by the independent recheck.
#[test]
fn substituting_the_recorded_dispositions_fails_the_recheck() {
    let module = compile_with(STORED, selection()).unwrap();
    let checked = module.checked.as_extern_calls().unwrap();
    let mut tampered = checked.certificate.clone();
    tampered.error_dispositions[1].1.clear();
    assert!(
        extern_call::recheck_extern_certificate(&checked.lowered.program, &tampered).is_err(),
        "a certificate claiming an empty arm must not survive its rederivation"
    );
}

/// An arm naming a callee this module never declared is refused before any
/// custody question is asked. Name resolution runs first and owns this
/// condition, exactly as it does for an adapter that raises an unknown name;
/// the extern pass does not duplicate it under a second code.
#[test]
fn an_arm_calling_an_undeclared_symbol_refuses_at_name_resolution() {
    let stored = STORED.replace("\r\n", "\n");
    let source = stored.replace(
        "extern fn qcore_extern_release(buffer: ptr<mut, u8>) -> unit;\n",
        "",
    );
    assert_ne!(source, stored, "the declaration must have been removed");
    assert_eq!(codes(&source, selection()), ["Q013"]);
}

/// The arm's calls are calls this module makes, so the entry's declared effect
/// row must admit them; a failure path is not an exemption.
#[test]
fn an_arm_call_outside_the_declared_effect_row_refuses() {
    let source = STORED.replace("effects { io, alloc, mut }", "effects { io, alloc }");
    let codes = codes(&source, selection());
    assert!(codes.contains(&"Q025".to_owned()), "{codes:?}");
}

/// The store's argument bridging is reachable with one live identity too,
/// because a self-store needs only one — and is refused by name.
#[test]
fn storing_an_identity_beneath_itself_refuses() {
    let stored = STORED.replace("\r\n", "\n");
    let source = stored.replace(STORE_BLOCK, "    qcore_extern_attach(owner, move owner);");
    assert_ne!(source, stored, "the store block must have been substituted");
    let codes = codes(&source, selection());
    assert!(codes.contains(&"Q024".to_owned()), "{codes:?}");
}

#[test]
fn moving_where_the_contract_declares_the_store_owner_refuses() {
    let stored = STORED.replace("\r\n", "\n");
    let source = stored.replace(
        STORE_BLOCK,
        "    qcore_extern_attach(move owner, move owner);",
    );
    assert_ne!(source, stored, "the store block must have been substituted");
    let codes = codes(&source, selection());
    assert!(codes.contains(&"Q024".to_owned()), "{codes:?}");
}

#[test]
fn a_store_beneath_a_non_owned_parameter_refuses_at_the_loader() {
    // Contract spec §C04.2's own well-formedness rule, re-pinned at this
    // boundary: the loader refuses the whole selection before bridging runs.
    let contracts = CONTRACTS.replacen(
        "ensures normal resource_owned(owner, nonnull);",
        "ensures normal resource_borrowed(owner);",
        1,
    );
    assert_eq!(
        codes(
            STORED,
            Some(BTreeMap::from([("bad-owner.qcore".to_owned(), contracts)]))
        ),
        ["Q023"]
    );
}

// ========================================================================= //
// Card EXTERN-MODES: `resource_released` and `resource_return_reallocated`. //
// ========================================================================= //

const RELEASED: &str = include_str!("fixtures/extern_calls/released.qr");
const REALLOC: &str = include_str!("fixtures/extern_calls/realloc.qr");
const REALLOC_FAILURE: &str = include_str!("fixtures/extern_calls/realloc_failure.qr");

/// A release is published as a release, not as a consumption. The two have the
/// same caller-side ledger delta, so nothing about leak accounting moves; what
/// moves is which field the identity lands in and which premise the certificate
/// names. Contract spec §C04.2 is explicit that a false `resource_released`
/// declaration can invalidate a safety conclusion while a false
/// `resource_consumed` declaration only relocates an obligation, and a reader
/// that could not tell the two apart could not tell which callees a
/// leak-freedom reading depends on.
#[test]
fn a_released_argument_is_recorded_apart_from_a_consumed_one() {
    let module = compile_with(RELEASED, selection()).unwrap();
    let checked = module.checked.as_extern_calls().unwrap();
    assert_eq!(checked.certificate.releases, vec![1]);
    assert!(checked.certificate.consumptions.is_empty());
    assert!(
        checked
            .certificate
            .assumptions
            .contains(&extern_call::ExternAssumption::DeclaredRelease(
                "qcore_extern_free".to_owned()
            )),
        "{:?}",
        checked.certificate.assumptions
    );
    // The round trip's consuming callee keeps its own answer, in both
    // directions: it is not silently reclassified as a release.
    let consuming = compile_with(ROUND_TRIP, selection()).unwrap();
    let consuming = consuming.checked.as_extern_calls().unwrap();
    assert_eq!(consuming.certificate.consumptions, vec![1]);
    assert!(consuming.certificate.releases.is_empty());
    assert!(!consuming.certificate.assumptions.iter().any(|assumption| {
        matches!(
            assumption,
            extern_call::ExternAssumption::DeclaredRelease(_)
        )
    }));
}

#[test]
fn a_released_binding_cannot_be_used_again() {
    let released = RELEASED.replace("\r\n", "\n");
    let source = released.replace(
        "    qcore_extern_free(move b);",
        "    qcore_extern_free(move b);\n    qcore_extern_fill(b, 1);",
    );
    assert_ne!(source, released, "the release must have been substituted");
    let codes = codes(&source, selection());
    assert!(codes.contains(&"Q024".to_owned()), "{codes:?}");
}

/// The nullable form is refused by name. A nullable release discharges nothing
/// when the modeled value is null (contract spec §C04.2), so admitting it would
/// make the discharge conditional on a value this profile does not model —
/// exactly the reason the consumed mode is nonnull-only. Recorded as reach
/// limit `EXTERN-MODES-NULLABLE-RELEASE`: contract spec §C04's own minimal
/// `release` example declares `nullable` and is therefore not callable here.
#[test]
fn a_nullable_release_declaration_refuses_at_the_bridging_table() {
    let contracts = CONTRACTS.replacen(
        "ensures normal resource_released(buffer, nonnull);",
        "ensures normal resource_released(buffer, nullable);",
        1,
    );
    assert_ne!(contracts, CONTRACTS, "the release clause must be present");
    assert_eq!(
        codes(
            RELEASED,
            Some(BTreeMap::from([(
                "nullable-release.qcore".to_owned(),
                contracts
            )]))
        ),
        ["Q024"]
    );
}

/// A successful reallocation replaces the subject exactly once: the old
/// identity leaves the ledger and the result identity enters it, recorded as
/// one pair rather than as an unrelated consumption and acquisition.
#[test]
fn a_successful_reallocation_records_the_replacement_as_one_pair() {
    let module = compile_with(REALLOC, selection()).unwrap();
    let checked = module.checked.as_extern_calls().unwrap();
    assert_eq!(checked.certificate.reallocations, vec![(1, 2)]);
    assert_eq!(checked.certificate.acquisitions, vec![1, 2]);
    assert_eq!(checked.certificate.releases, vec![2]);
    assert!(checked.certificate.assumptions.contains(
        &extern_call::ExternAssumption::DeclaredReallocation("qcore_extern_grow".to_owned())
    ));
    extern_call::recheck_extern_certificate(&checked.lowered.program, &checked.certificate)
        .expect("the driver publishes only evidence it rederived");
    assert_eq!(
        module.extern_observation.as_deref().unwrap(),
        [
            "acquire qcore_extern_alloc r1 ok",
            "realloc qcore_extern_grow r2 g:r1 d:32 ok",
            "call qcore_extern_fill b:r2 d:3",
            "call qcore_extern_free f:r2",
            "return",
        ]
    );
}

#[test]
fn a_reallocated_subject_cannot_be_used_after_the_replacement() {
    let realloc = REALLOC.replace("\r\n", "\n");
    let source = realloc.replace(
        "    qcore_extern_fill(grown, 3);",
        "    qcore_extern_fill(b, 3);",
    );
    assert_ne!(source, realloc, "the read must have been substituted");
    let codes = codes(&source, selection());
    assert!(codes.contains(&"Q024".to_owned()), "{codes:?}");
}

/// The half of contract spec §C04.3 that makes a reallocation different from
/// every other departing mode: a null failure *preserves* the incoming custody.
/// The subject is therefore still live on the error arm and must be
/// dispositioned there — the same abandonment rule as always, applied to an
/// identity an ordinary consumption would have taken off the arm.
#[test]
fn a_failed_reallocation_preserves_its_subject_for_the_error_arm() {
    let module = compile_with(REALLOC_FAILURE, selection()).unwrap();
    let checked = module.checked.as_extern_calls().unwrap();
    assert!(
        checked.certificate.reallocations.is_empty(),
        "a failed reallocation replaced nothing"
    );
    assert_eq!(
        checked.certificate.error_dispositions,
        vec![(1, vec![]), (2, vec![1])]
    );
    assert_eq!(
        module.extern_observation.as_deref().unwrap(),
        [
            "acquire qcore_extern_alloc r1 ok",
            "realloc qcore_extern_grow_failing r2 g:r1 d:32 null",
            "call qcore_extern_free f:r1",
            "raise Failure",
        ]
    );
}

/// Dropping the disposition abandons the preserved subject. This is the pin
/// that keeps the admission honest: had the checker treated the subject as
/// departed at the call — the way it treats a consumption — this source would
/// have checked, and the emitted program would have leaked.
#[test]
fn a_failed_reallocation_whose_arm_abandons_the_subject_refuses() {
    let failure = REALLOC_FAILURE.replace("\r\n", "\n");
    let source = failure.replace(
        "        else qcore_extern_free(move b) raise Failure;",
        "        else raise Failure;",
    );
    assert_ne!(source, failure, "the arm must have been substituted");
    let codes = codes(&source, selection());
    assert!(codes.contains(&"Q025".to_owned()), "{codes:?}");
}

/// A reallocated result whose subject is not `resource_owned` is refused before
/// any of this runs: contract spec §C04.3's own well-formedness rule, re-pinned
/// at this boundary.
#[test]
fn a_reallocation_subject_that_is_not_owned_refuses() {
    let contracts = CONTRACTS.replacen(
        "    ensures normal resource_owned(buffer, nonnull);",
        "    ensures normal resource_borrowed(buffer);",
        1,
    );
    assert_ne!(contracts, CONTRACTS, "the grow clause must be present");
    let codes = codes(
        REALLOC,
        Some(BTreeMap::from([(
            "bad-subject.qcore".to_owned(),
            contracts,
        )])),
    );
    assert!(
        codes.contains(&"Q023".to_owned()) || codes.contains(&"Q024".to_owned()),
        "{codes:?}"
    );
}

/// `resource_removed` stays refused, and the refusal is this card's measured
/// finding rather than an omission: a removal names a child this profile
/// stopped tracking when it was stored, because `EXTERN-F1` records the
/// residual beneath an owner as a premise and not as a modeled index.
/// Follow-on `EXTERN-MODES-REMOVED-INDEX`.
#[test]
fn a_removed_parameter_is_still_not_callable() {
    let contracts = CONTRACTS.replacen(
        "ensures normal resource_stored(child, owner, nonnull);",
        "ensures normal resource_removed(child, owner, nonnull);",
        1,
    );
    assert_ne!(contracts, CONTRACTS, "the store clause must be present");
    let codes = codes(
        STORED,
        Some(BTreeMap::from([("removed.qcore".to_owned(), contracts)])),
    );
    assert!(
        codes.contains(&"Q023".to_owned()) || codes.contains(&"Q024".to_owned()),
        "{codes:?}"
    );
}

// --- Card `PIPE-SCALAR-RESULT`: binding a non-acquiring call's scalar result. ---

const SCALAR_RESULT: &str = include_str!("fixtures/extern_calls/scalar_result.qr");

/// The admitted form, end to end: a scalar result bound to a local, and that
/// local read at a declared scalar parameter of a later call. The two halves are
/// one widening and are pinned together, because a binding nothing can read
/// would not be the missing `render` stage.
#[test]
fn a_scalar_result_is_bound_and_read_at_a_declared_scalar_parameter() {
    let module = compile_with(SCALAR_RESULT, selection()).unwrap();
    assert_eq!(
        module.execution_profile,
        SourceExecutionProfile::ExternCallsV1
    );
    let checked = module.checked.as_extern_calls().unwrap();
    // The scalar bindings are published by name, and they are the only
    // certificate field they touch: the custody accounting beside them is
    // `round_trip.qr`'s exactly.
    assert_eq!(
        checked.certificate.scalar_bindings,
        vec![
            ("qcore_extern_sum".to_owned(), 1),
            ("qcore_extern_scale".to_owned(), 2),
        ]
    );
    assert_eq!(checked.certificate.acquisitions, vec![1]);
    assert_eq!(checked.certificate.consumptions, vec![1]);
    assert!(checked.certificate.releases.is_empty());
    assert!(checked.certificate.stored.is_empty());
    assert_eq!(
        module.extern_observation.as_deref().unwrap(),
        [
            "acquire qcore_extern_alloc r1 ok",
            "call qcore_extern_fill b:r1 d:7",
            "value qcore_extern_sum v1 b:r1 d:8",
            "value qcore_extern_scale v2 v:v1 d:2",
            "call qcore_extern_release c:r1",
            "return",
        ]
    );
    let c = module.c_source.as_deref().expect("this route emits C11");
    for expected in [
        "int32_t qcore_extern_scale(int32_t p0, int32_t p1);",
        "int32_t v1 = qcore_extern_sum(r1, 8);",
        "int32_t v2 = qcore_extern_scale(v1, 2);",
    ] {
        assert!(c.contains(expected), "emitted C is missing `{expected}`");
    }
    // The value namespace never reaches the identity namespace: no call the
    // custody accounting sees takes a `v` name.
    assert!(!c.contains("qcore_extern_release(v"));
}

/// The published certificate still rederives with the new field included: a
/// tampered scalar-binding list is refused exactly as a tampered acquisition
/// list is.
#[test]
fn a_tampered_scalar_binding_list_fails_the_independent_recheck() {
    let module = compile_with(SCALAR_RESULT, selection()).unwrap();
    let checked = module.checked.as_extern_calls().unwrap();
    extern_call::recheck_extern_certificate(&checked.lowered.program, &checked.certificate)
        .expect("the driver publishes only evidence it rederived");
    let mut tampered = checked.certificate.clone();
    tampered.scalar_bindings.clear();
    assert!(extern_call::recheck_extern_certificate(&checked.lowered.program, &tampered).is_err());
}

/// A `unit` result binds nothing: there is no value. Refused at `Q024` naming
/// the declared result, with the discarding form spelled out.
#[test]
fn binding_a_unit_result_refuses() {
    let source = SCALAR_RESULT.replace("\r\n", "\n").replace(
        "    qcore_extern_fill(b, 7);\n",
        "    let u = qcore_extern_fill(b, 7);\n",
    );
    let diagnostics = compile_with(&source, selection())
        .map(|module| module.execution_profile)
        .expect_err("a unit result binds nothing");
    assert_eq!(
        diagnostics
            .iter()
            .map(|d| d.code.clone())
            .collect::<Vec<_>>(),
        ["Q024"]
    );
    assert!(diagnostics[0].message.contains("not a custody-free scalar"));
    assert!(diagnostics[0].message.contains("void"));
}

/// An acquiring callee's result is an identity, not a scalar. The mandatory
/// adapter is the one spelling of an acquisition and this card adds no second
/// one — so dropping the adapter refuses at `Q025` rather than quietly becoming
/// a scalar binding.
#[test]
fn binding_an_acquiring_result_without_the_adapter_refuses() {
    let source = SCALAR_RESULT.replace("\r\n", "\n").replace(
        "    let b = qcore_extern_alloc(8) succeeds true else raise Failure;\n",
        "    let b = qcore_extern_alloc(8);\n",
    );
    let diagnostics = compile_with(&source, selection())
        .map(|module| module.execution_profile)
        .expect_err("an acquisition still needs its adapter");
    assert_eq!(diagnostics[0].code, "Q025");
    assert!(diagnostics[0].message.contains("establishes fresh custody"));
}

/// The same for the other two acquisition sources: an out slot and a declared
/// reallocation both bind identities, and a scalar binder does not reach them.
#[test]
fn binding_an_out_or_reallocating_result_refuses() {
    for source in [
        OUT_SLOT.replace("\r\n", "\n").replace(
            "let b = qcore_extern_open(out, 8) succeeds true else raise Failure;",
            "let b = qcore_extern_open(out, 8);",
        ),
        REALLOC.replace("\r\n", "\n").replace(
            "    let grown = qcore_extern_grow(move b, 32) succeeds true\n",
            "    let grown = qcore_extern_grow(move b, 32);\n    let unused = qcore_extern_grow(move b, 32) succeeds true\n",
        ),
    ] {
        let codes = codes(&source, selection());
        assert!(codes.contains(&"Q025".to_owned()), "{codes:?}");
    }
}

/// A pointer result outside the two declared acquisition modes is not a scalar.
/// This is the fail-closed half of the bridging rule: `resource_return_none()`
/// admits any §C04.1 ABI result type, and only the custody-free scalars among
/// them may be bound.
#[test]
fn binding_a_pointer_result_declared_without_custody_refuses() {
    let contracts = CONTRACTS.replace("\r\n", "\n").replacen(
        "extern fn qcore_extern_scale(value: i32, factor: i32) -> i32",
        "extern fn qcore_extern_scale(value: i32, factor: i32) -> ptr<mut, u8>",
        1,
    );
    assert_ne!(
        contracts, CONTRACTS,
        "the scale declaration must be present"
    );
    let source = SCALAR_RESULT.replace("\r\n", "\n").replace(
        "extern fn qcore_extern_scale(value: i32, factor: i32) -> i32;",
        "extern fn qcore_extern_scale(value: i32, factor: i32) -> ptr<mut, u8>;",
    );
    let diagnostics = compile_with(
        &source,
        Some(BTreeMap::from([(
            "pointer-result.qcore".to_owned(),
            contracts,
        )])),
    )
    .map(|module| module.execution_profile)
    .expect_err("a pointer result is not a bindable scalar");
    assert_eq!(diagnostics[0].code, "Q024");
    assert!(diagnostics[0].message.contains("not a custody-free scalar"));
}

/// A resource binding is not a scalar and a scalar is not a resource binding:
/// each refuses in the other's position. This is the linearity half of the
/// card — the two namespaces are checked apart and never merged.
#[test]
fn the_two_namespaces_do_not_cross_at_a_call_site() {
    let source = SCALAR_RESULT.replace("\r\n", "\n");

    // A live resource where the declaration writes a scalar.
    let resource_as_scalar =
        source.replace("qcore_extern_scale(total, 2);", "qcore_extern_scale(b, 2);");
    let diagnostics = compile_with(&resource_as_scalar, selection())
        .map(|module| module.execution_profile)
        .expect_err("a resource binding is not scalar data");
    assert_eq!(diagnostics[0].code, "Q024");
    assert!(diagnostics[0].message.contains("never custody"));

    // A bound scalar where the declaration writes a departing mode.
    let scalar_as_resource = source.replace(
        "qcore_extern_release(move b);",
        "qcore_extern_release(move total);",
    );
    let moved_scalar = codes(&scalar_as_resource, selection());
    assert!(
        moved_scalar.contains(&"Q024".to_owned()),
        "{moved_scalar:?}"
    );

    // And a scalar this body never bound. Measured, not designed: name
    // resolution refuses an unknown name (`Q013`) before the lowering reaches
    // the bridging table, so the reachable refusal is that one. The lowering's
    // own "not a bound scalar" arm stays as the fail-closed backstop — it is
    // what refuses a name that *does* resolve but names nothing this body
    // bound, which is the previous case above.
    let unbound = source.replace(
        "qcore_extern_scale(total, 2);",
        "qcore_extern_scale(nope, 2);",
    );
    let unbound = codes(&unbound, selection());
    assert_eq!(unbound, ["Q013"]);
}

/// A `let` whose name already means the other kind of thing is refused rather
/// than resolved by lookup order — and the **measured** refusal is name
/// resolution's own, not this card's.
///
/// `Q012` (duplicate local binding) runs over the parsed body before any
/// lowering, so it catches every cross-namespace shadow in both directions and
/// the lowering's own `Q026` arm turns out to be unreachable from source. That
/// arm is kept anyway, exactly as the pipe desugarer's block-boundary arm is
/// kept: it is what keeps the lowering fail-closed if a future route ever hands
/// it a body name resolution did not see. Both halves are pinned — the
/// reachable code, and the fact that neither direction reaches the lowering.
#[test]
fn a_binding_that_shadows_the_other_namespace_refuses() {
    let source = SCALAR_RESULT.replace("\r\n", "\n");

    let scalar_over_resource = source.replace(
        "    let total = qcore_extern_sum(b, 8);\n",
        "    let b = qcore_extern_sum(b, 8);\n",
    );
    let scalar_first = codes(&scalar_over_resource, selection());
    assert!(
        scalar_first.contains(&"Q012".to_owned()),
        "{scalar_first:?}"
    );
    assert!(
        !scalar_first.contains(&"Q026".to_owned()),
        "{scalar_first:?}"
    );

    let resource_over_scalar = source.replace(
        "    let scaled = qcore_extern_scale(total, 2);\n",
        "    let total = qcore_extern_alloc(8) succeeds true else raise Failure;\n",
    );
    let resource_first = codes(&resource_over_scalar, selection());
    assert!(
        resource_first.contains(&"Q012".to_owned()),
        "{resource_first:?}"
    );
    assert!(
        !resource_first.contains(&"Q026".to_owned()),
        "{resource_first:?}"
    );
}

/// A scalar binding opens no obligation, so an unread one is not a leak — which
/// is the whole difference between the value namespace and the ledger.
#[test]
fn an_unread_scalar_binding_is_not_a_leak() {
    let source = SCALAR_RESULT
        .replace("\r\n", "\n")
        .replace("    let scaled = qcore_extern_scale(total, 2);\n", "");
    let module = compile_with(&source, selection()).expect("an unread scalar is not an obligation");
    let checked = module.checked.as_extern_calls().unwrap();
    assert_eq!(
        checked.certificate.scalar_bindings,
        vec![("qcore_extern_sum".to_owned(), 1)]
    );
}

/// An `unsafe extern fn` scalar result is bindable, and binding it deposits
/// exactly the waiver the discarding call deposits — by name and by declared
/// ABI. The premise is per *callee*, not per call site, so a binding can no more
/// launder it than a pipeline can.
#[test]
fn an_unchecked_scalar_result_binds_and_keeps_its_waiver() {
    let discarding = "#import std.extern;\n#import std.extern.unsafe;\n\n\
                      unsafe extern fn k_unchecked_add(a: i32, b: i32) -> i32;\n\n\
                      fn main() -> unit effects { io, alloc, mut } {\n    \
                      k_unchecked_add(2, 40);\n    return;\n}\n";
    let binding = "#import std.extern;\n#import std.extern.unsafe;\n\n\
                   unsafe extern fn k_unchecked_add(a: i32, b: i32) -> i32;\n\n\
                   fn main() -> unit effects { io, alloc, mut } {\n    \
                   let n = k_unchecked_add(2, 40);\n    return;\n}\n";
    let discarding = compile_with(discarding, selection()).unwrap();
    let binding = compile_with(binding, selection()).unwrap();
    let waiver = extern_call::ExternAssumption::UncheckedLinkage {
        callee: "k_unchecked_add".to_owned(),
        signature: "(int32_t, int32_t) -> int32_t".to_owned(),
    };
    for module in [&discarding, &binding] {
        assert!(
            module
                .checked
                .as_extern_calls()
                .unwrap()
                .certificate
                .assumptions
                .contains(&waiver),
            "the unchecked-linkage premise must be carried in both spellings"
        );
    }
    assert_eq!(
        binding
            .checked
            .as_extern_calls()
            .unwrap()
            .certificate
            .scalar_bindings,
        vec![("k_unchecked_add".to_owned(), 1)]
    );
    assert!(
        discarding
            .checked
            .as_extern_calls()
            .unwrap()
            .certificate
            .scalar_bindings
            .is_empty()
    );
}

// --- Card `EXTERN-SCALAR-OUTCOME` (trace half): the observation names the
// returned number where a declared outcome makes it well-defined. ---

/// `scalar_result.qr` with both of its bindings given the site's declared
/// outcome. `qcore_extern_fill(b, 7)` writes eight bytes of 7, so the real
/// `qcore_extern_sum(b, 8)` is 56 and the real `qcore_extern_scale(56, 2)` is
/// 112 — both numbers are the source's, and the emitted program refuses to go
/// on if the linked implementation disagrees.
fn declared_outcomes() -> String {
    let source = SCALAR_RESULT.replace("\r\n", "\n");
    let source = source.replace(
        "    let total = qcore_extern_sum(b, 8);",
        "    let total = qcore_extern_sum(b, 8) succeeds 56;",
    );
    let source = source.replace(
        "    let scaled = qcore_extern_scale(total, 2);",
        "    let scaled = qcore_extern_scale(total, 2) succeeds 112;",
    );
    assert!(
        source.contains("succeeds 112"),
        "both sites must be declared"
    );
    source
}

/// **The whole change, at the route that owns the format.** The two declared
/// bindings name their numbers; every other line of the trace — the
/// acquisition, the lend, the consumption, the return — is
/// `scalar_result.qr`'s exactly, so the diff against the undeclared pin
/// above is the two `-> N` fields and nothing else.
#[test]
fn a_declared_outcome_is_named_in_the_observation() {
    let module = compile_with(&declared_outcomes(), selection()).unwrap();
    assert_eq!(
        module.execution_profile,
        SourceExecutionProfile::ExternCallsV1
    );
    assert_eq!(
        module.extern_observation.as_deref().unwrap(),
        [
            "acquire qcore_extern_alloc r1 ok",
            "call qcore_extern_fill b:r1 d:7",
            "value qcore_extern_sum v1 b:r1 d:8 -> 56",
            "value qcore_extern_scale v2 v:v1 d:2 -> 112",
            "call qcore_extern_release c:r1",
            "return",
        ]
    );
    // The declaration is carried on the certificate too, and the two agree: a
    // binding named here is a binding named there.
    let checked = module.checked.as_extern_calls().unwrap();
    assert_eq!(
        checked.certificate.declared_outcomes,
        vec![(1, 56), (2, 112)]
    );
    // The custody accounting is untouched by the widening, in both directions.
    assert_eq!(checked.certificate.acquisitions, vec![1]);
    assert_eq!(checked.certificate.consumptions, vec![1]);
}

/// The observation's honesty rests on an ordering inside the emitted C: the
/// comparison against the real result dominates the write of the line. A run
/// that reaches the observation has already proved the declared number is the
/// returned one.
#[test]
fn the_emitted_check_dominates_the_observation_it_licenses() {
    let module = compile_with(&declared_outcomes(), selection()).unwrap();
    let c = module.c_source.as_deref().unwrap();
    let check = c.find("if (v1 != 56)").expect("emitted C compares v1");
    let observed = c
        .rfind("qcore_diagnostic_log(QCORE_LOG_DEBUG, QCORE_DIAG_EXTERN_OUTCOME, 1")
        .expect("emitted C submits the checked value to the logger");
    assert!(check < observed, "emitted C: {c}");
    assert!(c.contains("return 70;"));
}

/// A wrong declared outcome is **still admitted by the checker** — a literal
/// outcome control is a fixture control, and no checker in this repository
/// computes a C function's result — and is then caught by the run. This is the
/// fail-closed direction of the trace half: the native program exits 70, writes
/// the disagreement to standard error, and publishes **no** observation line
/// for that call, so a declared number that is not the returned one never
/// appears as an observation anywhere.
#[cfg(not(target_os = "windows"))]
#[test]
fn a_wrong_declared_outcome_exits_70_and_observes_nothing() {
    // A trace with no live acquisition at the moment of the refusal, so the
    // leak checker has nothing to report and the exit status this test reads is
    // the emitted program's own 70 rather than LSan's.
    let wrong = "#import std.extern;\n\
                 extern fn qcore_extern_scale(value: i32, factor: i32) -> i32;\n\
                 fn main() -> unit effects { io } {\n\
                 \x20   let n = qcore_extern_scale(21, 2) succeeds 41;\n\
                 \x20   return;\n}\n";
    let module = compile_with(wrong, selection()).expect("a literal control is a fixture control");
    // The reference execution believes the declaration, as it always has.
    assert_eq!(
        module.extern_observation.as_deref().unwrap(),
        ["value qcore_extern_scale v1 d:21 d:2 -> 41", "return"]
    );
    let run = run_native_status(module.c_source.as_deref().unwrap(), "wrong-outcome");
    assert_eq!(run.0, Some(70), "stderr was:\n{:?}", run.2);
    assert!(
        !run.1
            .iter()
            .any(|line| line.contains("value qcore_extern_scale")),
        "a disagreeing run must publish no observation for that call: {:?}",
        run.1
    );
    assert!(run.2.is_empty(), "{:?}", run.2);

    // The positive control: the same program with the number the linked
    // implementation actually returns runs to completion and publishes the
    // line, which is what makes the refusal above a discrimination rather than
    // a program that never works.
    let right = wrong.replace("succeeds 41", "succeeds 42");
    let module = compile_with(&right, selection()).unwrap();
    let run = run_native_status(module.c_source.as_deref().unwrap(), "right-outcome");
    assert_eq!(run.0, Some(0), "stderr was:\n{:?}", run.2);
    assert!(run.1.is_empty());
    assert_eq!(
        module.extern_observation.as_deref().unwrap(),
        ["value qcore_extern_scale v1 d:21 d:2 -> 42", "return"]
    );
}

/// Compile and run the emitted C against the real callees, returning
/// `(exit status, stdout lines, stderr lines)` **without** asserting success.
///
/// [`run_native`] deliberately asserts a clean exit, because every fixture it
/// runs must have one; this variant exists for the one case whose whole point
/// is a nonzero status.
#[cfg(not(target_os = "windows"))]
fn run_native_status(source: &str, label: &str) -> (Option<i32>, Vec<String>, Vec<String>) {
    use std::process::Command;
    let directory = std::env::temp_dir().join(format!(
        "qcore-extern-status-{label}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&directory).unwrap();
    let generated = directory.join("generated.c");
    let callees = directory.join("buffer_api.c");
    let binary = directory.join("generated");
    std::fs::write(&generated, source).unwrap();
    std::fs::write(&callees, include_str!("fixtures/extern_calls/buffer_api.c")).unwrap();
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
        .arg(&callees)
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
    (run.status.code(), out, err)
}

//! End-to-end route selection, admission and refusal for
//! `source.extern-call.v1` (card `source.extern-call.v1`).
//!
//! Every contract fixture here lives under `tests/fixtures/`, never under
//! `contract/qcore/`, so the pinned corpus counts are untouched by this card.
use std::collections::BTreeMap;

use qcore_compiler::{
    CompileOptions, SourceExecutionProfile, SourceFile, compile_files, extern_call,
};

const CONTRACTS: &str = include_str!("fixtures/extern_calls/contracts/buffer_api.qcore");
const ROUND_TRIP: &str = include_str!("fixtures/extern_calls/round_trip.qcore");

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

#[cfg(not(target_os = "windows"))]
fn run_native(source: &str, label: &str) -> Vec<String> {
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
        let mutated = source.replace("import std.extern;", "");
        assert_ne!(mutated, source, "the import must have been removed");
        assert_eq!(codes(&mutated, selection()), ["Q022"]);
    }
}

#[test]
fn a_misspelled_module_refuses_as_an_unknown_module() {
    let source = ROUND_TRIP.replace("import std.extern;", "import std.externs;");
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
        let granted = SourceFile::new("granted.qcore", "module library;\nimport std.extern;\n");
        let mutated = source.replace("import std.extern;", "");
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
            Some(BTreeMap::from([("partial.qcore".to_owned(), partial)]))
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

#[test]
fn an_excluded_contract_mode_refuses_at_the_bridging_table() {
    let contracts = CONTRACTS.replace(
        "ensures normal resource_consumed(buffer, nonnull);",
        "ensures normal resource_released(buffer, nonnull);",
    );
    assert_eq!(
        codes(
            ROUND_TRIP,
            Some(BTreeMap::from([("released.qcore".to_owned(), contracts)]))
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
    let source = "import std.extern;\n\
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
    let source = "import std.concurrency;\nimport std.extern;\n\
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

const OUT_SLOT: &str = include_str!("fixtures/extern_calls/out_slot.qcore");
const STORED: &str = include_str!("fixtures/extern_calls/stored.qcore");

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
        "declared outcome violated: qcore_extern_open",
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
    let source = "import std.extern;\n\
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

// --- resource_stored: admitted shape, measured reachability boundary. ---

/// The store's *bridging* is admitted — the refusal is `Q025` at the second
/// acquisition, not `Q024` at the contract. A store needs two simultaneously
/// live checked identities (the child and the distinct `resource_owned` owner
/// contract spec §C04.2 requires); `source.extern-call.v1` admits at most one,
/// because its adapter's error arm may abandon nothing. Recorded, not repaired:
/// lifting that restriction is the already-named EXTERN-CALL-F follow-on.
#[test]
fn a_store_is_admitted_but_needs_a_second_live_identity_the_profile_refuses() {
    let codes = codes(STORED, selection());
    assert_eq!(
        codes,
        ["Q025"],
        "the blocker is the adapter rule, not the store"
    );
}

/// The store's argument bridging *is* reachable from source with one live
/// identity, because a self-store needs only one — and is refused by name.
#[test]
fn storing_an_identity_beneath_itself_refuses() {
    let source = STORED.replace(
        "    let child = qcore_extern_alloc(8) succeeds true else raise Failure;\n    \
         qcore_extern_attach(owner, move child);",
        "    qcore_extern_attach(owner, move owner);",
    );
    let codes = codes(&source, selection());
    assert!(codes.contains(&"Q024".to_owned()), "{codes:?}");
}

#[test]
fn moving_where_the_contract_declares_the_store_owner_refuses() {
    let source = STORED.replace(
        "    let child = qcore_extern_alloc(8) succeeds true else raise Failure;\n    \
         qcore_extern_attach(owner, move child);",
        "    qcore_extern_attach(move owner, move owner);",
    );
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

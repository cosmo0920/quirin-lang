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

//! Unchecked linkage: `unsafe extern fn`, the explicit waiver that lets an
//! external library be linked with no contract at all (card `UNSAFE-LINKAGE`,
//! `docs/design/UNSAFE_LINKAGE_V1.md`).
//!
//! Three things are pinned here and they are not the same thing.
//!
//! 1. **The default does not move.** Without the waiver, an uncontracted callee
//!    refuses exactly as it did (`Q023`), and the contracted route is untouched.
//! 2. **The admitted surface carries no custody.** Scalars by value in and out;
//!    every pointer, in every position and every mode, refuses by name.
//! 3. **Trust is carried.** Every certificate names each uncontracted callee and
//!    its declared ABI, and no route to a published certificate omits it.
#[cfg(not(target_os = "windows"))]
#[path = "../../tests/support/diagnostic_sink.rs"]
mod diagnostic_sink;

use std::collections::BTreeMap;

use qcore_compiler::{
    CompileOptions, SourceExecutionProfile, SourceFile, compile_files, extern_call,
};

const ROUND_TRIP: &str = include_str!("fixtures/unsafe_linkage/round_trip.qr");
const CONTRACTS: &str = include_str!("fixtures/extern_calls/contracts/buffer_api.qv");

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
        &[SourceFile::new("unsafe.qcore", source)],
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

fn without_import(source: &str, module: &str) -> String {
    let import = format!("#import {module};");
    source
        .lines()
        .filter(|line| *line != import)
        .collect::<Vec<_>>()
        .join("\n")
}

fn unchecked_premises(module: &qcore_compiler::CompiledModule) -> Vec<(String, String)> {
    module
        .declared_assumptions()
        .iter()
        .filter_map(|assumption| match assumption {
            extern_call::ExternAssumption::UncheckedLinkage { callee, signature } => {
                Some((callee.clone(), signature.clone()))
            }
            _ => None,
        })
        .collect()
}

// --- The accepted round trip, with no contract environment at all. ---

#[test]
fn an_uncontracted_library_links_with_no_contract_selection() {
    // The point of the card: `extern_contracts: None`, three real C symbols,
    // and a checked module. This is also exactly what `quirin check` can do,
    // since the CLI selects no environment.
    let module = compile_with(ROUND_TRIP, None).unwrap();
    assert_eq!(
        module.execution_profile,
        SourceExecutionProfile::ExternCallsV1
    );
    assert_eq!(module.entry.as_deref(), Some("main"));
    let checked = module.checked.as_extern_calls().unwrap();
    // No custody crossed the boundary, in either direction, at any call.
    assert!(checked.certificate.acquisitions.is_empty());
    assert!(checked.certificate.consumptions.is_empty());
    assert!(checked.certificate.releases.is_empty());
    assert!(checked.certificate.stored.is_empty());
    assert!(checked.certificate.reallocations.is_empty());
    for (_, custody) in &checked.certificate.calls {
        assert!(
            custody.is_empty(),
            "an unchecked call is handed no custody at all"
        );
    }
}

#[test]
fn every_uncontracted_callee_is_named_on_the_certificate_with_its_declared_abi() {
    let module = compile_with(ROUND_TRIP, None).unwrap();
    assert_eq!(
        unchecked_premises(&module),
        vec![
            (
                "k_unchecked_add".to_owned(),
                "(int32_t, int32_t) -> int32_t".to_owned()
            ),
            (
                "k_unchecked_measure".to_owned(),
                "(uint64_t) -> uint64_t".to_owned()
            ),
            (
                "k_unchecked_side_effect".to_owned(),
                "(uint32_t) -> void".to_owned()
            ),
        ]
    );
    // And no uncontracted callee is silently carried as a *declared contract*,
    // which is the one substitution that would look like evidence.
    assert!(
        !module
            .declared_assumptions()
            .iter()
            .any(|assumption| matches!(
                assumption,
                extern_call::ExternAssumption::DeclaredContract(_)
            )),
        "a callee with no contract must never be published as having one"
    );
}

#[test]
fn the_premises_reach_a_consumer_that_does_not_know_the_route() {
    // `declared_assumptions` is the surface a downstream reader uses without
    // matching on the checked-module variant. It is non-empty here and empty on
    // a route that publishes no premise, so it never invents one.
    let module = compile_with(ROUND_TRIP, None).unwrap();
    assert_eq!(module.declared_assumptions().len(), 5);
    let sequential = compile_files(
        &[SourceFile::new(
            "plain.qcore",
            "fn main() -> unit { return; }\n",
        )],
        &CompileOptions {
            entry: None,
            extern_contracts: None,
            native_libraries: None,
            native_library_headers: None,
        },
    )
    .unwrap();
    assert!(sequential.declared_assumptions().is_empty());
}

#[test]
fn a_tampered_assumption_list_does_not_survive_the_independent_recheck() {
    let module = compile_with(ROUND_TRIP, None).unwrap();
    let checked = module.checked.as_extern_calls().unwrap();
    extern_call::recheck_extern_certificate(&checked.lowered.program, &checked.certificate)
        .expect("the driver publishes only evidence it rederived");
    let mut laundered = checked.certificate.clone();
    laundered.assumptions.retain(|assumption| {
        !matches!(
            assumption,
            extern_call::ExternAssumption::UncheckedLinkage { .. }
        )
    });
    assert_ne!(laundered.assumptions, checked.certificate.assumptions);
    assert!(
        extern_call::recheck_extern_certificate(&checked.lowered.program, &laundered).is_err(),
        "a certificate that dropped its unchecked-linkage premises must not recheck"
    );
    // The same laundering is what the publication tripwire refuses, and it is
    // checked independently of the recheck rather than through it.
    assert!(!extern_call::certificate_carries_unchecked_linkage(
        &checked.lowered.program,
        &laundered
    ));
    assert!(extern_call::certificate_carries_unchecked_linkage(
        &checked.lowered.program,
        &checked.certificate
    ));
}

#[test]
fn the_derived_effect_row_is_maximal_and_the_entry_must_declare_it() {
    let module = compile_with(ROUND_TRIP, None).unwrap();
    let checked = module.checked.as_extern_calls().unwrap();
    let effects = checked
        .certificate
        .effects
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(
        effects,
        vec![
            qcore_compiler::typed_errors::Effect::Io,
            qcore_compiler::typed_errors::Effect::Alloc,
            qcore_compiler::typed_errors::Effect::Mut,
        ],
        "nothing is known about an uncontracted callee, so its row is everything this profile can name"
    );
    // A narrower declared row refuses. The opt-in is therefore visible in the
    // entry's own signature, not only at the declaration.
    for narrower in [
        "effects { io }",
        "effects { io, alloc }",
        "effects { io, mut }",
    ] {
        let source = ROUND_TRIP.replace("effects { io, alloc, mut }", narrower);
        assert_ne!(source, ROUND_TRIP);
        assert!(
            !codes(&source, None).is_empty(),
            "`{narrower}` must not admit an unchecked call"
        );
    }
}

// --- Q032: the module gate. ---

#[test]
fn declaring_an_uncontracted_extern_without_its_module_refuses() {
    let source = without_import(ROUND_TRIP, "std.extern.unsafe");
    assert_ne!(
        source, ROUND_TRIP,
        "the waiver import must have been removed"
    );
    assert_eq!(codes(&source, None), ["Q032"]);
}

#[test]
fn the_call_module_alone_does_not_grant_the_waiver() {
    // `std.extern` is the call vocabulary and grants no waiver; the reverse
    // holds too, so an unchecked-linkage file names both modules.
    let source = without_import(ROUND_TRIP, "std.extern.unsafe");
    assert!(source.contains("#import std.extern;"));
    assert_eq!(codes(&source, None), ["Q032"]);

    let source = without_import(ROUND_TRIP, "std.extern");
    assert!(source.contains("#import std.extern.unsafe;"));
    assert_eq!(codes(&source, None), ["Q022"]);
}

#[test]
fn importing_the_waiver_in_another_file_does_not_grant_this_one() {
    let granted = SourceFile::new(
        "granted.qcore",
        "module library;\n#import std.extern.unsafe;\n",
    );
    let caller = SourceFile::new(
        "caller.qcore",
        without_import(ROUND_TRIP, "std.extern.unsafe"),
    );
    let errors = compile_files(
        &[granted, caller],
        &CompileOptions {
            entry: None,
            extern_contracts: None,
            native_libraries: None,
            native_library_headers: None,
        },
    )
    .unwrap_err();
    assert_eq!(errors[0].code, "Q032");
    assert_eq!(errors[0].location.path, "caller.qcore");
}

#[test]
fn the_gate_is_on_the_declaration_not_on_a_call_to_it() {
    // Declaring an uncontracted symbol is itself the trust act: a file that
    // links one and never calls it has still linked it.
    let source = "unsafe extern fn k_unchecked_add(a: i32, b: i32) -> i32;\n\
                  fn main() -> unit { return; }\n";
    assert_eq!(codes(source, None), ["Q032"]);
    let permitted = format!("#import std.extern.unsafe;\n{source}");
    let module = compile_with(&permitted, None).unwrap();
    // With the waiver imported and nothing called, the module is an ordinary
    // sequential one and publishes no premise: no call, no trust consumed.
    assert_eq!(
        module.execution_profile,
        SourceExecutionProfile::SequentialCleanupV1
    );
    assert!(module.declared_assumptions().is_empty());
}

// --- Q033: the closed unchecked table. Custody cannot cross. ---

#[test]
fn a_pointer_parameter_refuses_in_every_admitted_pointer_spelling() {
    for spelling in ["ptr<mut, u8>", "ptr<const, u8>", "ptr<mut, ptr<mut, u8>>"] {
        let source = ROUND_TRIP.replace(
            "unsafe extern fn k_unchecked_add(a: i32, b: i32) -> i32;",
            &format!("unsafe extern fn k_unchecked_add(a: {spelling}, b: i32) -> i32;"),
        );
        assert_eq!(
            codes(&source, None),
            ["Q033"],
            "`{spelling}` must not cross an unchecked boundary"
        );
    }
}

#[test]
fn a_read_only_pointer_is_refused_deliberately_and_not_by_oversight() {
    // The weakest pointer form there is. §3.1 of the design: nothing bounds the
    // read, nothing ends the loan at the return, `const` constrains this
    // declaration rather than the implementation, and nothing stops the callee
    // publishing it. The refusal names the type.
    let source = ROUND_TRIP.replace(
        "unsafe extern fn k_unchecked_add(a: i32, b: i32) -> i32;",
        "unsafe extern fn k_unchecked_add(a: ptr<const, u8>, b: i32) -> i32;",
    );
    let message = compile_with(&source, None)
        .map(|_| String::new())
        .expect_err("a pointer parameter must refuse")
        .into_iter()
        .map(|diagnostic| diagnostic.message)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(message.contains("const uint8_t *"), "{message}");
    assert!(message.contains("scalars by value only"), "{message}");
}

#[test]
fn a_pointer_result_refuses() {
    let source = ROUND_TRIP.replace(
        "unsafe extern fn k_unchecked_add(a: i32, b: i32) -> i32;",
        "unsafe extern fn k_unchecked_add(a: i32, b: i32) -> ptr<mut, u8>;",
    );
    assert_eq!(codes(&source, None), ["Q033"]);
}

#[test]
fn a_unit_parameter_refuses() {
    let source = ROUND_TRIP.replace(
        "unsafe extern fn k_unchecked_add(a: i32, b: i32) -> i32;",
        "unsafe extern fn k_unchecked_add(a: unit, b: i32) -> i32;",
    );
    assert_eq!(codes(&source, None), ["Q033"]);
}

#[test]
fn a_spelling_outside_the_abi_vocabulary_refuses() {
    // The contract is waived; the ABI is not. A declaration this repository
    // cannot project into C is not a linking story, it is a guess.
    for spelling in ["fnptr", "Buffer", "own<Buffer>"] {
        let source = ROUND_TRIP.replace(
            "unsafe extern fn k_unchecked_add(a: i32, b: i32) -> i32;",
            &format!("unsafe extern fn k_unchecked_add(a: {spelling}, b: i32) -> i32;"),
        );
        assert_eq!(
            codes(&source, None),
            ["Q033"],
            "`{spelling}` is not a projected C ABI type"
        );
    }
    // Card `W48-QR-FLOATING-POINT` added `f32`/`f64` to the extern route's ABI
    // table (it listed `f64` here before): the declaration now projects, and
    // the call's integer literal at the float parameter is refused as the
    // value it is — an integer is never implicitly a float.
    for spelling in ["f32", "f64"] {
        let source = ROUND_TRIP.replace(
            "unsafe extern fn k_unchecked_add(a: i32, b: i32) -> i32;",
            &format!("unsafe extern fn k_unchecked_add(a: {spelling}, b: i32) -> i32;"),
        );
        assert_eq!(codes(&source, None), ["Q024"], "`{spelling}` projects");
    }
}

#[test]
fn every_admitted_scalar_crosses_and_the_table_is_not_vacuous() {
    for spelling in [
        "bool", "i8", "i16", "i32", "i64", "isize", "u8", "u16", "u32", "u64", "usize",
    ] {
        let mut source = ROUND_TRIP.replace(
            "unsafe extern fn k_unchecked_add(a: i32, b: i32) -> i32;",
            &format!("unsafe extern fn k_unchecked_add(a: {spelling}, b: i32) -> {spelling};"),
        );
        // Card `W48-QR-SCALAR-TYPES-R3`: a literal at a declared scalar
        // parameter is range-checked against its type, so the fixture's `2`
        // at a `bool` parameter — an implicit integer-to-`bool` conversion the
        // C call performed — is refused; the `bool` row passes `1`.
        if spelling == "bool" {
            source = source.replace("k_unchecked_add(2, 40)", "k_unchecked_add(1, 40)");
        }
        assert!(
            compile_with(&source, None).is_ok(),
            "`{spelling}` is a §C04.1 scalar and must cross"
        );
    }
}

// --- Q034: type honesty. A waiver may not sit beside a real declaration. ---

#[test]
fn waiving_a_name_the_selected_environment_contracts_refuses() {
    // The contracted route wins. You may not launder away a contract you have —
    // this is LOADER-ABI's rule applied to the waiver: where a declaration is
    // visible, it is compared.
    let source = "#import std.extern;\n#import std.extern.unsafe;\n\
                  unsafe extern fn qcore_extern_fill(buffer: ptr<mut, u8>, value: u32) -> unit;\n\
                  fn main() -> unit effects { io, alloc, mut } {\n    \
                  qcore_extern_fill(0, 7);\n    return;\n}\n";
    assert_eq!(codes(source, selection()), ["Q034"]);
}

#[test]
fn the_same_waiver_with_no_such_contract_selected_refuses_on_its_own_merits() {
    // Control for the test above: with the name absent from the environment the
    // identity gate does not fire, and the declaration is refused by the table
    // instead — so `Q034` above was about the contract, not about the pointer.
    let source = "#import std.extern;\n#import std.extern.unsafe;\n\
                  unsafe extern fn k_absent(buffer: ptr<mut, u8>, value: u32) -> unit;\n\
                  fn main() -> unit effects { io, alloc, mut } {\n    \
                  k_absent(0, 7);\n    return;\n}\n";
    assert_eq!(codes(source, selection()), ["Q033"]);
}

#[test]
fn a_name_cannot_be_declared_both_ways_in_one_file() {
    // The remaining laundering vector: declare the same symbol twice, once with
    // a contract and once waived, and hope the driver picks the waiver. Name
    // resolution refuses it before any of this card's gates run, so the
    // ambiguity never reaches them.
    let source = "#import std.extern;\n#import std.extern.unsafe;\n\
                  extern fn k_dup(a: i32) -> i32;\n\
                  unsafe extern fn k_dup(a: i32) -> i32;\n\
                  fn main() -> unit effects { io, alloc, mut } {\n    \
                  k_dup(2);\n    return;\n}\n";
    assert_eq!(codes(source, None), ["Q010"]);
}

#[test]
fn a_name_cannot_be_declared_both_ways_across_two_files() {
    let contracted = SourceFile::new(
        "library.qcore",
        "module library;\nextern fn k_dup(a: i32) -> i32;\n",
    );
    let waived = SourceFile::new(
        "caller.qcore",
        "#import std.extern;\n#import std.extern.unsafe;\n\
         unsafe extern fn k_dup(a: i32) -> i32;\n\
         fn main() -> unit effects { io, alloc, mut } {\n    \
         k_dup(2);\n    return;\n}\n",
    );
    let errors = compile_files(
        &[contracted, waived],
        &CompileOptions {
            entry: None,
            extern_contracts: None,
            native_libraries: None,
            native_library_headers: None,
        },
    )
    .unwrap_err();
    assert_eq!(errors[0].code, "Q010");
}

#[test]
fn a_broken_contract_selection_refuses_even_when_every_callee_is_waived() {
    // A selected environment that does not load is refused rather than ignored:
    // the `Q034` identity gate would otherwise be silently skipped, so shipping
    // a deliberately broken contract module would become a way to hide that a
    // callee has a contract.
    let broken = CONTRACTS.replace("requires resource_contract_v1;", "requires something_else;");
    assert_ne!(broken, CONTRACTS);
    assert_eq!(
        codes(
            ROUND_TRIP,
            Some(BTreeMap::from([("broken.qcore".to_owned(), broken)]))
        ),
        ["Q023"]
    );
    // The control: with no selection at all, the same module checks. An absent
    // environment is the ordinary all-unchecked case, not a broken one.
    assert!(compile_with(ROUND_TRIP, None).is_ok());
}

#[test]
fn an_inline_contract_on_a_waived_declaration_refuses() {
    let source = ROUND_TRIP.replace(
        "unsafe extern fn k_unchecked_add(a: i32, b: i32) -> i32;",
        "unsafe extern fn k_unchecked_add(a: i32, b: i32) -> i32\n\
         contract { requires resource_contract_v1;\n           \
         ensures normal resource_return_none(); };",
    );
    assert_eq!(codes(&source, None), ["Q034"]);
}

// --- The default does not move. ---

#[test]
fn an_uncontracted_callee_without_the_waiver_still_refuses_at_q023() {
    let source = "#import std.extern;\n\
                  extern fn k_unchecked_add(a: i32, b: i32) -> i32;\n\
                  fn main() -> unit effects { io } {\n    \
                  k_unchecked_add(2, 40);\n    return;\n}\n";
    assert_eq!(codes(source, None), ["Q023"]);
    assert_eq!(codes(source, selection()), ["Q023"]);
}

#[test]
fn a_contracted_module_is_unchanged_by_this_card() {
    // The contracted round trip still selects its environment, still refuses
    // without it, and still publishes `DeclaredContract` and no waiver.
    const CONTRACTED: &str = include_str!("fixtures/extern_calls/round_trip.qr");
    assert_eq!(codes(CONTRACTED, None), ["Q023"]);
    let module = compile_with(CONTRACTED, selection()).unwrap();
    assert!(unchecked_premises(&module).is_empty());
    assert!(
        module
            .declared_assumptions()
            .iter()
            .any(|assumption| matches!(
                assumption,
                extern_call::ExternAssumption::DeclaredContract(_)
            ))
    );
}

#[test]
fn an_unsafe_statement_inside_a_body_still_parses_as_it_did() {
    // `is_declaration_start` gained `unsafe` so a waiver may follow a function
    // body. This pins that the statement form the word already introduced is
    // unaffected: the module parses, reaches the emission pass, and is refused
    // there under `Q500` exactly as before this card.
    let source = "fn main() -> unit {\n    unsafe {\n        return;\n    }\n}\n";
    assert_eq!(codes(source, None), ["Q500"]);
}

#[test]
fn a_waiver_may_follow_a_function_body() {
    let source = "#import std.extern;\n#import std.extern.unsafe;\n\
                  fn main() -> unit effects { io, alloc, mut } {\n    \
                  k_unchecked_add(2, 40);\n    return;\n}\n\
                  unsafe extern fn k_unchecked_add(a: i32, b: i32) -> i32;\n";
    // The declaration after a body is seen: the module routes to the extern
    // profile and carries its premise, rather than failing to parse.
    let module = compile_with(source, None).unwrap();
    assert_eq!(unchecked_premises(&module).len(), 1);
}

// --- Mixed modules: one waiver beside one real contract. ---

#[test]
fn a_module_may_mix_a_contracted_callee_with_an_uncontracted_one() {
    let source = "#import std.extern;\n#import std.extern.unsafe;\n\
                  error Failure { Failed; }\n\
                  extern fn qcore_extern_alloc(size: usize) -> ptr<mut, u8>;\n\
                  extern fn qcore_extern_release(buffer: ptr<mut, u8>) -> unit;\n\
                  unsafe extern fn k_unchecked_add(a: i32, b: i32) -> i32;\n\
                  fn main() -> unit effects { io, alloc, mut } {\n    \
                  let b = qcore_extern_alloc(8) succeeds true else raise Failure;\n    \
                  k_unchecked_add(2, 40);\n    \
                  qcore_extern_release(move b);\n    return;\n}\n";
    let module = compile_with(source, selection()).unwrap();
    let premises = unchecked_premises(&module);
    assert_eq!(premises.len(), 1);
    assert_eq!(premises[0].0, "k_unchecked_add");
    // The contracted half keeps its own premises and its own custody, which the
    // unchecked call neither touched nor obscured.
    let checked = module.checked.as_extern_calls().unwrap();
    assert_eq!(checked.certificate.acquisitions, vec![1]);
    assert_eq!(checked.certificate.consumptions, vec![1]);
    assert!(module.declared_assumptions().iter().any(|assumption| {
        *assumption
            == extern_call::ExternAssumption::DeclaredContract("qcore_extern_alloc".to_owned())
    }));
}

#[test]
fn a_mixed_module_still_needs_its_contract_selection() {
    // One contracted callee is enough to require the environment; the waiver
    // does not excuse the module from selecting one.
    let source = "#import std.extern;\n#import std.extern.unsafe;\n\
                  error Failure { Failed; }\n\
                  extern fn qcore_extern_alloc(size: usize) -> ptr<mut, u8>;\n\
                  extern fn qcore_extern_release(buffer: ptr<mut, u8>) -> unit;\n\
                  unsafe extern fn k_unchecked_add(a: i32, b: i32) -> i32;\n\
                  fn main() -> unit effects { io, alloc, mut } {\n    \
                  let b = qcore_extern_alloc(8) succeeds true else raise Failure;\n    \
                  k_unchecked_add(2, 40);\n    \
                  qcore_extern_release(move b);\n    return;\n}\n";
    assert_eq!(codes(source, None), ["Q023"]);
}

// --- Native evidence. ---

#[test]
#[cfg_attr(target_os = "windows", ignore = "the sanitizer fixtures are Unix-only")]
fn the_emitted_program_calls_the_real_uncontracted_library() {
    #[cfg(not(target_os = "windows"))]
    {
        let module = compile_with(ROUND_TRIP, None).unwrap();
        let reference = module.extern_observation.clone().unwrap();
        assert_eq!(
            reference,
            [
                "call k_unchecked_add d:2 d:40",
                "call k_unchecked_side_effect d:7",
                "call k_unchecked_measure d:16",
                "return",
            ]
        );
        assert_eq!(
            run_native(&diagnostic_sink::instrument(
                module.c_source.as_deref().unwrap()
            )),
            reference,
            "native and reference observations must agree line for line"
        );
    }
}

#[cfg(not(target_os = "windows"))]
fn run_native(source: &str) -> Vec<String> {
    use std::process::Command;
    let directory = std::env::temp_dir().join(format!(
        "qcore-unsafe-linkage-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&directory).unwrap();
    let generated = directory.join("generated.c");
    let callees = directory.join("unchecked_api.c");
    let binary = directory.join("generated");
    std::fs::write(&generated, source).unwrap();
    std::fs::write(
        &callees,
        include_str!("fixtures/unsafe_linkage/unchecked_api.c"),
    )
    .unwrap();
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

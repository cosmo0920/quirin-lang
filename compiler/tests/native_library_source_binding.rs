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

//! Source-file-scoped library binding: `extern library NAME;` (card
//! `NATIVE-LIBRARY-SOURCE-BINDING`; `docs/design/NATIVE_LIBRARY_LINKAGE_V1.md`
//! §4.1, and the follow-on row of
//! `docs/reports/wave22-native-library-header/README.md` §11).
//!
//! Before this card a manifest bound by **symbol name across the whole
//! selection**: whichever selected manifest happened to name a symbol supplied
//! that symbol's artifact premise, and no source file said so. This suite pins
//! what saying so must and must not do.
//!
//! 1. **A bound file resolves through its binding, and the certificate records
//!    which build.** The premise names `library@hash`, beside — never instead of
//!    — every callee premise the file already carried.
//! 2. **An unbound file is unchanged.** The binding is opt-in: a file that
//!    declares none resolves selection-wide exactly as it did, down to the
//!    emitted C and the published premise list.
//! 3. **Everything else refuses.** An unknown name, an ambiguous name, a
//!    repeated binding, a binding with nothing selected, and a declared symbol
//!    belonging to a manifest this file did **not** bind each refuse by name.
//!    Nothing resolves implicitly and nothing falls back to the weaker premise.
//! 4. **Bindings do not leak across compilation inputs.** One file's binding
//!    decides that file's declarations and no other file's — constraint 13's
//!    discipline applied to a binding.
use std::collections::BTreeMap;

use qcore_compiler::{
    CompileOptions, SourceExecutionProfile, SourceFile, compile_files, extern_call,
};

const ROUND_TRIP: &str = include_str!("fixtures/extern_calls/round_trip.qr");
const WAIVED_ROUND_TRIP: &str = include_str!("fixtures/unsafe_linkage/round_trip.qr");
const SLICE_MODULE: &str = "fixtures/native_library/contracts/buffer_lib.qv";
const SLICE: &str = include_str!("fixtures/native_library/contracts/buffer_lib.qv");
const MANIFEST_PATH: &str = "fixtures/native_library/buffer_lib.qrlib";
const MANIFEST: &str = include_str!("fixtures/native_library/buffer_lib.qrlib");
const L1_PATH: &str = "fixtures/native_library/unchecked_demo.qrlib";
const L1_MANIFEST: &str = include_str!("fixtures/native_library/unchecked_demo.qrlib");

/// The library name the contract-slice manifest declares, and the hash beside
/// it. Spelled here so a test that asserts the premise names the build fails if
/// either the manifest or the resolution moves.
const LIBRARY: &str = "qcore_buffer_demo";
const HASH: &str = "sha256:4d1f8a03be27c5901e6ab4772d05f3c81a9e6b40d7cc2318f5be0947aa6132ed";
const L1_LIBRARY: &str = "qcore_unchecked_demo";

fn map(entries: &[(&str, &str)]) -> Option<BTreeMap<String, String>> {
    Some(
        entries
            .iter()
            .map(|(key, text)| ((*key).to_owned(), (*text).to_owned()))
            .collect(),
    )
}

fn slice_selection() -> Option<BTreeMap<String, String>> {
    map(&[(SLICE_MODULE, SLICE)])
}

fn manifest_selection() -> Option<BTreeMap<String, String>> {
    map(&[(MANIFEST_PATH, MANIFEST)])
}

fn both_manifests() -> Option<BTreeMap<String, String>> {
    map(&[(MANIFEST_PATH, MANIFEST), (L1_PATH, L1_MANIFEST)])
}

fn compile_many(
    files: &[(&str, String)],
    extern_contracts: Option<BTreeMap<String, String>>,
    native_libraries: Option<BTreeMap<String, String>>,
) -> Result<qcore_compiler::CompiledModule, Vec<qcore_compiler::FileDiagnostic>> {
    let inputs = files
        .iter()
        .map(|(path, text)| SourceFile::new(*path, text.clone()))
        .collect::<Vec<_>>();
    compile_files(
        &inputs,
        &CompileOptions {
            entry: None,
            extern_contracts,
            native_libraries,
            native_library_headers: None,
        },
    )
}

fn compile(
    source: &str,
    extern_contracts: Option<BTreeMap<String, String>>,
    native_libraries: Option<BTreeMap<String, String>>,
) -> Result<qcore_compiler::CompiledModule, Vec<qcore_compiler::FileDiagnostic>> {
    compile_many(
        &[("extern.qcore", source.to_owned())],
        extern_contracts,
        native_libraries,
    )
}

fn codes(
    source: &str,
    extern_contracts: Option<BTreeMap<String, String>>,
    native_libraries: Option<BTreeMap<String, String>>,
) -> Vec<String> {
    compile(source, extern_contracts, native_libraries)
        .map(|module| module.execution_profile)
        .expect_err("this fixture must be refused")
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect()
}

fn premises(module: &qcore_compiler::CompiledModule) -> Vec<extern_call::ExternAssumption> {
    module.declared_assumptions().to_vec()
}

/// Every `extern library` premise a module published, as `(named, manifest,
/// library, hash)`.
fn bindings(module: &qcore_compiler::CompiledModule) -> Vec<(String, String, String, String)> {
    let mut found = premises(module)
        .into_iter()
        .filter_map(|assumption| match assumption {
            extern_call::ExternAssumption::SourceBoundLibrary(binding) => Some((
                binding.named,
                binding.manifest,
                binding.identity.library,
                binding.identity.hash,
            )),
            _ => None,
        })
        .collect::<Vec<_>>();
    found.sort();
    found
}

/// `source` with a binding declaration inserted after its first line, which is
/// the `#import std.extern;` every fixture here opens with. Written this way so
/// the bound and unbound rows compare *the same program*, differing in the
/// binding line and nothing else.
fn bound(source: &str, library: &str) -> String {
    let (first, rest) = source.split_once('\n').expect("the fixture has lines");
    format!("{first}\nextern library {library};\n{rest}")
}

// --- 1. A bound file resolves through its binding, and says which build ---

#[test]
fn a_bound_file_resolves_its_declared_extern_surface_through_the_named_library() {
    let module = compile(
        &bound(ROUND_TRIP, LIBRARY),
        slice_selection(),
        manifest_selection(),
    )
    .expect("a file binding the library its manifest declares compiles");
    assert_eq!(
        module.execution_profile,
        SourceExecutionProfile::ExternCallsV1
    );
    // Every contracted callee still resolves, with its artifact premise intact:
    // the binding narrows *which* manifests resolve, it does not withhold one
    // that the file bound.
    let resident = premises(&module)
        .into_iter()
        .filter(|assumption| {
            matches!(
                assumption,
                extern_call::ExternAssumption::ManifestResidentContract { .. }
            )
        })
        .count();
    assert_eq!(resident, 4, "all four contracted callees stay resident");
}

#[test]
fn the_binding_premise_names_the_artifact_the_file_bound() {
    let module = compile(
        &bound(ROUND_TRIP, LIBRARY),
        slice_selection(),
        manifest_selection(),
    )
    .unwrap();
    assert_eq!(
        bindings(&module),
        [(
            LIBRARY.to_owned(),
            MANIFEST_PATH.to_owned(),
            LIBRARY.to_owned(),
            HASH.to_owned()
        )],
        "the certificate records the name the file wrote and the build it resolved to"
    );
}

#[test]
fn the_binding_premise_stacks_and_replaces_no_callee_premise() {
    let bound_module = compile(
        &bound(ROUND_TRIP, LIBRARY),
        slice_selection(),
        manifest_selection(),
    )
    .unwrap();
    let plain = compile(ROUND_TRIP, slice_selection(), manifest_selection()).unwrap();
    let without_bindings = |module: &qcore_compiler::CompiledModule| {
        premises(module)
            .into_iter()
            .filter(|assumption| {
                !matches!(
                    assumption,
                    extern_call::ExternAssumption::SourceBoundLibrary(_)
                )
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(
        without_bindings(&bound_module),
        without_bindings(&plain),
        "binding adds a premise; it removes and rewrites none"
    );
    assert!(bindings(&plain).is_empty());
    assert_eq!(bindings(&bound_module).len(), 1);
}

#[test]
fn the_bound_route_emits_the_same_c_as_the_symbol_name_route() {
    // Invariant 3's shape for this card, made executable: where both routes
    // apply they must agree. The binding is a statement about *selection*, so it
    // may add a premise and must move no emitted byte.
    let bound_module = compile(
        &bound(ROUND_TRIP, LIBRARY),
        slice_selection(),
        manifest_selection(),
    )
    .unwrap();
    let plain = compile(ROUND_TRIP, slice_selection(), manifest_selection()).unwrap();
    assert_eq!(bound_module.c_source, plain.c_source);
    assert_eq!(bound_module.execution_profile, plain.execution_profile);
}

// --- 2. An unbound file is unchanged ---

#[test]
fn an_unbound_file_publishes_no_binding_premise() {
    let module = compile(ROUND_TRIP, slice_selection(), manifest_selection()).unwrap();
    assert!(
        bindings(&module).is_empty(),
        "a file that made no binding statement publishes none"
    );
}

#[test]
fn an_unbound_file_still_resolves_selection_wide() {
    // The pre-card resolution, pinned: with two manifests selected and no
    // binding anywhere, the waived symbols still resolve against the manifest
    // that names them.
    let module = compile(WAIVED_ROUND_TRIP, slice_selection(), both_manifests())
        .expect("an unbound waived module is what it was");
    let residents = premises(&module)
        .into_iter()
        .filter_map(|assumption| match assumption {
            extern_call::ExternAssumption::ManifestUncheckedLinkage { residence, .. } => {
                Some(residence.identity.library)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(residents.len(), 3);
    assert!(residents.iter().all(|library| library == L1_LIBRARY));
    assert!(bindings(&module).is_empty());
}

// --- 3. Everything else refuses ---

#[test]
fn a_binding_naming_a_library_no_selected_manifest_declares_refuses() {
    assert_eq!(
        codes(
            &bound(ROUND_TRIP, "qcore_absent_demo"),
            slice_selection(),
            manifest_selection()
        ),
        ["Q065"]
    );
}

#[test]
fn a_binding_with_no_manifest_selected_at_all_refuses() {
    // Fail-closed in the direction that matters: an unresolvable binding is
    // never read as "then bind nothing", which would make a typo silent.
    assert_eq!(
        codes(&bound(ROUND_TRIP, LIBRARY), slice_selection(), None),
        ["Q065"]
    );
}

#[test]
fn two_manifests_declaring_one_library_name_make_the_binding_ambiguous() {
    // The same ground `Q040` refuses a symbol bound twice: a binding that is not
    // a function has no defined premise, and choosing one by selection order
    // would make the premise depend on the order the builder listed manifests.
    let twin = MANIFEST.replace(
        "contract fn qcore_extern_alloc",
        "unchecked fn qcore_twin_scalar(int32_t) -> int32_t;\n# contract fn qcore_extern_alloc",
    );
    let twin = twin
        .replace(
            "contract fn qcore_extern_fill",
            "# contract fn qcore_extern_fill",
        )
        .replace(
            "contract fn qcore_extern_sum",
            "# contract fn qcore_extern_sum",
        )
        .replace(
            "contract fn qcore_extern_release",
            "# contract fn qcore_extern_release",
        );
    let selection = map(&[(MANIFEST_PATH, MANIFEST), ("twin.qrlib", &twin)]);
    assert_eq!(
        codes(&bound(ROUND_TRIP, LIBRARY), slice_selection(), selection),
        ["Q065"]
    );
}

#[test]
fn a_repeated_binding_of_one_library_refuses() {
    let source = bound(&bound(ROUND_TRIP, LIBRARY), LIBRARY);
    assert_eq!(
        codes(&source, slice_selection(), manifest_selection()),
        ["Q065"]
    );
}

#[test]
fn a_waived_symbol_of_a_manifest_this_file_did_not_bind_refuses() {
    // The load-bearing refusal. The file bound the buffer library and declares a
    // symbol the *other* selected manifest binds. Falling back to the bare
    // waiver would let a file stop its certificate naming a build by binding
    // something else — `Q041`'s shape, a contradiction rewarded with a premise
    // that says less.
    let source = format!(
        "#import std.extern;\n#import std.extern.unsafe;\nextern library {LIBRARY};\n\nunsafe extern fn k_unchecked_add(a: i32, b: i32) -> i32;\n\nfn main() -> unit effects {{ io, alloc, mut }} {{\n    k_unchecked_add(2, 40);\n    return;\n}}\n"
    );
    assert_eq!(
        codes(&source, slice_selection(), both_manifests()),
        ["Q066"]
    );
}

#[test]
fn a_contracted_symbol_of_a_manifest_this_file_did_not_bind_refuses() {
    // The contracted mirror of the row above, and the reason the refusal is
    // placed before the `Q042` gate: an unresident contract must not resolve
    // past the manifest that claims its symbol.
    assert_eq!(
        codes(
            &bound(ROUND_TRIP, L1_LIBRARY),
            slice_selection(),
            both_manifests()
        ),
        ["Q066", "Q066", "Q066", "Q066"]
    );
}

#[test]
fn a_symbol_no_selected_manifest_binds_is_not_foreign() {
    // Absence of evidence is not evidence, and the two absences are different: a
    // symbol *no* manifest claims stays unresident exactly as before, while one
    // an unbound manifest claims refuses. Only the second is a contradiction.
    let source = format!(
        "#import std.extern;\n#import std.extern.unsafe;\nextern library {LIBRARY};\n\nunsafe extern fn k_unnamed_symbol(a: i32) -> i32;\n\nfn main() -> unit effects {{ io, alloc, mut }} {{\n    k_unnamed_symbol(2);\n    return;\n}}\n"
    );
    let module = compile(&source, slice_selection(), manifest_selection())
        .expect("an unclaimed symbol is admitted");
    let bare = premises(&module)
        .into_iter()
        .filter(|assumption| {
            matches!(
                assumption,
                extern_call::ExternAssumption::UncheckedLinkage { .. }
            )
        })
        .count();
    assert_eq!(bare, 1, "it keeps the bare waiver premise, byte for byte");
    assert_eq!(bindings(&module).len(), 1, "and the file's binding is said");
}

// --- 4. Bindings do not leak across compilation inputs ---

/// A sibling input that declares one waived symbol and binds nothing.
fn unbound_sibling() -> String {
    "module library;\n#import std.extern.unsafe;\n\nunsafe extern fn k_unchecked_add(a: i32, b: i32) -> i32;\n".to_owned()
}

/// An entry input that binds the buffer library and calls both surfaces.
fn bound_entry() -> String {
    format!(
        "#import std.extern;\nextern library {LIBRARY};\n\nerror Failure {{ Failed; }}\n\nextern fn qcore_extern_alloc(size: usize) -> ptr<mut, u8>;\nextern fn qcore_extern_fill(buffer: ptr<mut, u8>, value: u32) -> unit;\nextern fn qcore_extern_sum(buffer: ptr<const, u8>, count: u32) -> i32;\nextern fn qcore_extern_release(buffer: ptr<mut, u8>) -> unit;\n\nfn main() -> unit effects {{ io, alloc, mut }} {{\n    let b = qcore_extern_alloc(8) succeeds true else raise Failure;\n    qcore_extern_fill(b, 7);\n    qcore_extern_sum(b, 8);\n    qcore_extern_release(move b);\n    k_unchecked_add(2, 40);\n    return;\n}}\n"
    )
}

#[test]
fn one_file_s_binding_does_not_reach_a_sibling_file_s_declaration() {
    // The declaration is the trust act (card `UNSAFE-LINKAGE`'s own rule), so
    // the *declaring* file's binding decides it. Here the sibling bound nothing,
    // so its waived declaration resolves selection-wide exactly as it did before
    // this card — a binding in another compilation input granted it nothing, and
    // took nothing away.
    let module = compile_many(
        &[
            ("bound.qcore", bound_entry()),
            ("sibling.qcore", unbound_sibling()),
        ],
        slice_selection(),
        both_manifests(),
    )
    .expect("a bound file and an unbound sibling compile together");
    let waived = premises(&module)
        .into_iter()
        .filter_map(|assumption| match assumption {
            extern_call::ExternAssumption::ManifestUncheckedLinkage {
                callee, residence, ..
            } => Some((callee, residence.identity.library)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        waived,
        [("k_unchecked_add".to_owned(), L1_LIBRARY.to_owned())]
    );
    assert_eq!(bindings(&module).len(), 1, "one file bound, one did not");
}

#[test]
fn moving_the_sibling_declaration_into_the_bound_file_refuses() {
    // The same program, one declaration moved across the file boundary. This is
    // the isolation claim with teeth: the two files differ in their binding, so
    // they must differ in what they may declare.
    let merged = format!(
        "{}\n{}",
        bound_entry().replace(
            "#import std.extern;",
            "#import std.extern;\n#import std.extern.unsafe;"
        ),
        "unsafe extern fn k_unchecked_add(a: i32, b: i32) -> i32;\n"
    );
    let refused = compile_many(
        &[("bound.qcore", merged)],
        slice_selection(),
        both_manifests(),
    )
    .expect_err("the bound file may not declare a foreign manifest's symbol")
    .into_iter()
    .map(|diagnostic| diagnostic.code)
    .collect::<Vec<_>>();
    assert_eq!(refused, ["Q066"]);
}

// --- A binding is checked wherever it is written ---

#[test]
fn a_binding_in_a_module_with_no_extern_surface_is_still_resolved() {
    // Without this the declaration would be one nothing ever looked at: a module
    // routed somewhere with no extern surface would carry an unresolvable
    // binding silently. The check runs before route selection, guarded on the
    // file having written a binding at all.
    let source =
        "extern library qcore_absent_demo;\n\nfn main() -> unit effects { } {\n    return;\n}\n";
    assert_eq!(codes(source, None, manifest_selection()), ["Q065"]);
}

#[test]
fn a_module_that_writes_no_binding_never_loads_the_selection() {
    // The guard's other half. A module with no binding and no extern surface
    // compiles with a deliberately unloadable manifest selected, exactly as it
    // did before this card: loading unconditionally would let a broken manifest
    // refuse a module that never mentioned one.
    let broken = map(&[("broken.qrlib", "not a manifest at all\n")]);
    let source = "fn main() -> unit effects { } {\n    return;\n}\n";
    compile(source, None, broken).expect("a module that mentions no manifest is unmoved");
}

// --- The constraint-14 census outcome ---

#[test]
fn a_library_named_after_a_route_selecting_word_does_not_select_that_route() {
    // Card `NATIVE-LIBRARY-SOURCE-BINDING`'s census found exactly two
    // whole-token-stream route selectors — `concurrency_span` and
    // `source_tasks::has_source_tasks` — and a binding's tokens are a binding,
    // never a use. Without the exclusion this program would be refused for a
    // missing `#import std.concurrency;` on the strength of a library's name.
    let renamed = MANIFEST.replace("library qcore_buffer_demo;", "library spawn;");
    let selection = map(&[(MANIFEST_PATH, &renamed)]);
    let module = compile(&bound(ROUND_TRIP, "spawn"), slice_selection(), selection)
        .expect("a library named `spawn` binds without selecting the concurrency route");
    assert_eq!(
        module.execution_profile,
        SourceExecutionProfile::ExternCallsV1
    );
    assert_eq!(bindings(&module).len(), 1);
}

#[test]
fn a_library_named_after_a_task_selecting_intrinsic_does_not_select_that_route() {
    let renamed = MANIFEST.replace("library qcore_buffer_demo;", "library qcore_buffer_new;");
    let selection = map(&[(MANIFEST_PATH, &renamed)]);
    let module = compile(
        &bound(ROUND_TRIP, "qcore_buffer_new"),
        slice_selection(),
        selection,
    )
    .expect("a library named `qcore_buffer_new` binds without selecting the task route");
    assert_eq!(
        module.execution_profile,
        SourceExecutionProfile::ExternCallsV1
    );
}

#[test]
fn library_remains_an_ordinary_identifier_outside_the_binding_position() {
    // Constraint 14: the spelling mints no reserved word. `library` is still
    // usable as an ordinary source name, and `extern library` is claimed only at
    // declaration position, where it was a parse error before this card.
    let source = "fn library() -> unit effects { } {\n    return;\n}\n\nfn main() -> unit effects { } {\n    library();\n    return;\n}\n";
    compile(source, None, None).expect("`library` is an ordinary function name");
}

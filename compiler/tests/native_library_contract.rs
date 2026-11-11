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

//! Manifest contract slices: **where pointer symbols belong** (card
//! `NATIVE-LIBRARY-CONTRACT-SLICE`, level **L3** of the linkage spectrum in
//! `docs/design/UNSAFE_LINKAGE_V1.md` §1.1;
//! `docs/design/NATIVE_LIBRARY_LINKAGE_V1.md` §4.1 and §4.2).
//!
//! Wave 18's rule was that pointers cross exactly where a contract exists to
//! state their premise; wave 19 built the manifest but refused a `contract fn`
//! entry by name. This suite pins the four things that lifting that refusal must
//! and must not do.
//!
//! 1. **A contracted symbol resolves through the manifest and inherits the
//!    artifact identity.** The premises *stack*: the declared contract says what
//!    the code does, the artifact premise says which code it is, and a
//!    certificate carries both.
//! 2. **The binding is total in both directions.** An entry whose contract does
//!    not resolve, and a contract in the slice that no entry binds, each reject
//!    the whole selection.
//! 3. **A contradicted binding withdraws.** The declared contract is withdrawn
//!    for the unit and its call sites fall to the uncontracted refusal — the
//!    opposite disposition from level L1's, for the stated reason that here the
//!    weaker reading refuses and there it is accepted.
//! 4. **Nothing about the unmanifested world moves.** The `Q023` default, the
//!    contracted route's own refusals and the emitted C are what they were.
use std::collections::BTreeMap;

use qcore_compiler::{
    CompileOptions, SourceExecutionProfile, SourceFile, compile_files, extern_call,
};

const ROUND_TRIP: &str = include_str!("fixtures/extern_calls/round_trip.qr");
const WAIVED_ROUND_TRIP: &str = include_str!("fixtures/unsafe_linkage/round_trip.qr");
const SLICE_MODULE: &str = "fixtures/native_library/contracts/buffer_lib.qv";
const SLICE: &str = include_str!("fixtures/native_library/contracts/buffer_lib.qv");
const SHARED_MODULE: &str = "fixtures/extern_calls/contracts/buffer_api.qv";
const SHARED: &str = include_str!("fixtures/extern_calls/contracts/buffer_api.qv");
const MANIFEST_PATH: &str = "fixtures/native_library/buffer_lib.qrlib";
const MANIFEST: &str = include_str!("fixtures/native_library/buffer_lib.qrlib");
const L1_MANIFEST: &str = include_str!("fixtures/native_library/unchecked_demo.qrlib");

fn contracts(entries: &[(&str, &str)]) -> Option<BTreeMap<String, String>> {
    Some(
        entries
            .iter()
            .map(|(module, text)| ((*module).to_owned(), (*text).to_owned()))
            .collect(),
    )
}

fn slice_selection() -> Option<BTreeMap<String, String>> {
    contracts(&[(SLICE_MODULE, SLICE)])
}

fn manifests(entries: &[(&str, String)]) -> Option<BTreeMap<String, String>> {
    Some(
        entries
            .iter()
            .map(|(path, text)| ((*path).to_owned(), text.clone()))
            .collect(),
    )
}

fn manifest_selection() -> Option<BTreeMap<String, String>> {
    manifests(&[(MANIFEST_PATH, MANIFEST.to_owned())])
}

fn compile(
    source: &str,
    extern_contracts: Option<BTreeMap<String, String>>,
    native_libraries: Option<BTreeMap<String, String>>,
) -> Result<qcore_compiler::CompiledModule, Vec<qcore_compiler::FileDiagnostic>> {
    compile_files(
        &[SourceFile::new("extern.qcore", source)],
        &CompileOptions {
            entry: None,
            extern_contracts,
            native_libraries,
            native_library_headers: None,
        },
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

/// The manifest with one entry rewritten, for the disagreement and
/// binding-failure rows.
fn manifest_with(original: &str, replacement: &str) -> String {
    let edited = MANIFEST.replace(original, replacement);
    assert_ne!(edited, MANIFEST, "the manifest edit must have applied");
    edited
}

// --- 1. The contracted symbol resolves through the manifest ---

#[test]
fn a_contracted_pointer_symbol_resolves_through_the_manifest() {
    // The whole point of the level: every callee here takes or returns a
    // pointer, which a contract-less manifest entry may not even declare.
    let module = compile(ROUND_TRIP, slice_selection(), manifest_selection())
        .expect("a manifest whose slice binds the selected contracts admits them");
    assert_eq!(
        module.execution_profile,
        SourceExecutionProfile::ExternCallsV1
    );
    let mut named = premises(&module)
        .into_iter()
        .filter_map(|assumption| match assumption {
            extern_call::ExternAssumption::ManifestResidentContract {
                callee,
                contract_module,
                residence,
            } => Some((
                callee,
                contract_module,
                residence.identity.library.clone(),
                residence.identity.hash.clone(),
            )),
            _ => None,
        })
        .collect::<Vec<_>>();
    named.sort();
    let hash = "sha256:4d1f8a03be27c5901e6ab4772d05f3c81a9e6b40d7cc2318f5be0947aa6132ed";
    assert_eq!(
        named,
        [
            (
                "qcore_extern_alloc".to_owned(),
                SLICE_MODULE.to_owned(),
                "qcore_buffer_demo".to_owned(),
                hash.to_owned()
            ),
            (
                "qcore_extern_fill".to_owned(),
                SLICE_MODULE.to_owned(),
                "qcore_buffer_demo".to_owned(),
                hash.to_owned()
            ),
            (
                "qcore_extern_release".to_owned(),
                SLICE_MODULE.to_owned(),
                "qcore_buffer_demo".to_owned(),
                hash.to_owned()
            ),
            (
                "qcore_extern_sum".to_owned(),
                SLICE_MODULE.to_owned(),
                "qcore_buffer_demo".to_owned(),
                hash.to_owned()
            ),
        ]
    );
}

#[test]
fn the_artifact_premise_stacks_beside_the_contract_premise_and_replaces_nothing() {
    let bound = compile(ROUND_TRIP, slice_selection(), manifest_selection()).unwrap();
    let plain = compile(ROUND_TRIP, slice_selection(), None).unwrap();
    let contract_premises = |module: &qcore_compiler::CompiledModule| {
        let mut names = premises(module)
            .into_iter()
            .filter_map(|assumption| match assumption {
                extern_call::ExternAssumption::DeclaredContract(callee) => Some(callee),
                _ => None,
            })
            .collect::<Vec<_>>();
        names.sort();
        names
    };
    // Every declared-contract premise the unmanifested module carried is still
    // carried. The manifest adds; it never substitutes.
    assert_eq!(contract_premises(&bound), contract_premises(&plain));
    assert_eq!(contract_premises(&bound).len(), 4);
    assert_eq!(
        premises(&bound).len(),
        premises(&plain).len() + 4,
        "exactly one artifact premise per bound callee, and nothing removed"
    );
    for assumption in premises(&plain) {
        assert!(
            premises(&bound).contains(&assumption),
            "the manifest removed the premise {assumption:?}"
        );
    }
}

#[test]
fn the_manifest_changes_the_premises_and_nothing_else() {
    let bound = compile(ROUND_TRIP, slice_selection(), manifest_selection()).unwrap();
    let plain = compile(ROUND_TRIP, slice_selection(), None).unwrap();
    assert_eq!(bound.c_source, plain.c_source);
    assert_eq!(bound.execution_profile, plain.execution_profile);
    assert_eq!(bound.declaration_count, plain.declaration_count);
    assert_eq!(bound.entry, plain.entry);
    assert_ne!(
        bound.declared_assumptions(),
        plain.declared_assumptions(),
        "the premise list is the one field that must differ"
    );
}

// --- 2. Binding totality, in both directions ---

#[test]
fn an_entry_whose_contract_the_environment_does_not_declare_rejects_the_selection() {
    let edited = manifest_with("qcore_extern_sum", "qcore_extern_absent");
    assert_eq!(
        codes(
            ROUND_TRIP,
            slice_selection(),
            manifests(&[(MANIFEST_PATH, edited)])
        ),
        ["Q047"]
    );
}

#[test]
fn an_entry_naming_another_module_rejects_rather_than_resolving_by_symbol_name() {
    let edited = manifest_with(SLICE_MODULE, "fixtures/somewhere/else.qv");
    assert_eq!(
        codes(
            ROUND_TRIP,
            slice_selection(),
            manifests(&[(MANIFEST_PATH, edited)])
        ),
        ["Q047"]
    );
}

#[test]
fn a_contract_in_the_slice_that_no_entry_binds_rejects_the_selection() {
    // The other direction of design §4.1: the shared fixture module declares
    // fifteen contracts and this manifest binds four of them, so binding it
    // makes the slice partial.
    let edited = manifest_with(SLICE_MODULE, SHARED_MODULE);
    assert_eq!(
        codes(
            ROUND_TRIP,
            contracts(&[(SHARED_MODULE, SHARED)]),
            manifests(&[(MANIFEST_PATH, edited)])
        ),
        ["Q047"]
    );
}

#[test]
fn a_manifest_that_binds_contracts_with_no_environment_selected_rejects() {
    // Fail-closed rather than ignored: a selection whose contract slice cannot
    // resolve at all is refused, so a broken slice never becomes a quiet way to
    // stop a certificate saying which build it rests on.
    assert_eq!(
        codes(WAIVED_ROUND_TRIP, None, manifest_selection()),
        ["Q047"]
    );
}

#[test]
fn a_slice_that_can_acquire_and_never_discharge_rejects() {
    // Design §4.4's closed-allocator-custody check, which was vacuous while
    // every entry was contract-less — a waiver cannot acquire — and becomes real
    // exactly here. A library that hands out custody with no declared release
    // entry point would be stating `allocator_custody closed` against its own
    // slice.
    let acquiring_only = "module contract;\n\
                          \n\
                          extern fn k_alloc(size: usize) -> ptr<mut, u8>\n\
                          contract {\n\
                              requires resource_contract_v1;\n\
                              ensures normal resource_borrowed(size);\n\
                              ensures normal resource_return_owned(nullable);\n\
                          };\n";
    let source = "#import std.extern;\n\
                  \n\
                  error Failure { Failed; }\n\
                  \n\
                  extern fn k_alloc(size: usize) -> ptr<mut, u8>;\n\
                  \n\
                  fn main() -> unit effects { io, alloc, mut } {\n\
                      let b = k_alloc(8) succeeds true else raise Failure;\n\
                      return;\n\
                  }\n";
    let one_entry = MANIFEST
        .lines()
        .filter(|line| !line.starts_with("contract fn "))
        .collect::<Vec<_>>()
        .join("\n")
        + "\ncontract fn k_alloc(uintptr_t) -> uint8_t * from lib.qcore;\n";
    assert_eq!(
        codes(
            source,
            contracts(&[("lib.qcore", acquiring_only)]),
            manifests(&[(MANIFEST_PATH, one_entry)])
        ),
        ["Q047"]
    );
}

// --- 3. A contradicted binding withdraws ---

#[test]
fn a_manifest_contradicting_the_declared_contract_withdraws_it_for_this_unit() {
    // `qcore_extern_alloc` is declared `(usize) -> ptr<mut, u8>`, which projects
    // to `(uintptr_t) -> uint8_t *`. The manifest is edited to declare a
    // `uint32_t` parameter instead.
    let edited = manifest_with(
        "contract fn qcore_extern_alloc(uintptr_t)",
        "contract fn qcore_extern_alloc(uint32_t)",
    );
    let codes = codes(
        ROUND_TRIP,
        slice_selection(),
        manifests(&[(MANIFEST_PATH, edited)]),
    );
    // Both halves of the withdrawal are reported: the withdrawal itself, and
    // the uncontracted refusal every call site then reaches. A withdrawal that
    // only refused would be a refusal; one that only refused *silently* would be
    // an override.
    assert_eq!(codes, ["Q048", "Q023"]);
}

#[test]
fn a_withdrawal_is_monotone_in_the_refusing_direction() {
    // The same program, the same environment, the same manifest but for one
    // edited projection: accepted becomes refused, and there is no third
    // outcome in which the contract survives in a weakened form.
    assert!(compile(ROUND_TRIP, slice_selection(), manifest_selection()).is_ok());
    for (original, replacement) in [
        (
            "contract fn qcore_extern_alloc(uintptr_t) -> uint8_t *",
            "contract fn qcore_extern_alloc(uintptr_t) -> const uint8_t *",
        ),
        (
            "contract fn qcore_extern_fill(uint8_t *, uint32_t)",
            "contract fn qcore_extern_fill(uint8_t *)",
        ),
        (
            "contract fn qcore_extern_sum(const uint8_t *, uint32_t) -> int32_t",
            "contract fn qcore_extern_sum(const uint8_t *, uint32_t) -> int64_t",
        ),
    ] {
        let edited = manifest_with(original, replacement);
        let refused = compile(
            ROUND_TRIP,
            slice_selection(),
            manifests(&[(MANIFEST_PATH, edited)]),
        )
        .expect_err("a contradicted binding must withdraw, and the unit must then refuse");
        let codes = refused
            .iter()
            .map(|diagnostic| diagnostic.code.clone())
            .collect::<Vec<_>>();
        assert!(
            codes.contains(&"Q048".to_owned()) && codes.contains(&"Q023".to_owned()),
            "`{replacement}` must withdraw and then refuse, not {codes:?}"
        );
    }
}

#[test]
fn a_withdrawal_never_leaves_a_partially_admitted_callee() {
    // The withdrawn name is removed from the environment the derivation seeds,
    // so no certificate is published at all — in particular none carrying a
    // declared-contract premise for the withdrawn callee.
    let edited = manifest_with(
        "contract fn qcore_extern_alloc(uintptr_t)",
        "contract fn qcore_extern_alloc(uint32_t)",
    );
    assert!(
        compile(
            ROUND_TRIP,
            slice_selection(),
            manifests(&[(MANIFEST_PATH, edited)])
        )
        .is_err()
    );
}

// --- The waiver mirror ---

#[test]
fn a_symbol_the_manifest_binds_to_a_contract_may_not_be_waived() {
    // The third face of one rule, and this card's **measured non-addition**
    // (`CONTRACT-SLICE-WAIVER-MIRROR`). `Q034` refuses a waiver for a name the
    // environment declares; `Q042` refuses a contract for a name the manifest
    // waives; the third case — a waiver for a name the manifest *contracts* —
    // needs no gate of its own, because `Q047` guarantees that a
    // manifest-contracted name is declared by the environment and `Q034` then
    // refuses. A gate here would be unreachable, and an unreachable code is a
    // claim a reader cannot check, so this row pins the composition instead of
    // a fourth code.
    let waiver = "#import std.extern;\n\
                  #import std.extern.unsafe;\n\
                  \n\
                  unsafe extern fn k_scale(value: i32, factor: i32) -> i32;\n\
                  \n\
                  fn main() -> unit effects { io } {\n\
                      k_scale(6, 7);\n\
                      return;\n\
                  }\n";
    let one_contract = "module contract;\n\
                        \n\
                        extern fn k_scale(value: i32, factor: i32) -> i32\n\
                        contract {\n\
                            requires resource_contract_v1;\n\
                            ensures normal resource_borrowed(value);\n\
                            ensures normal resource_borrowed(factor);\n\
                            ensures normal resource_return_none();\n\
                        };\n";
    let one_entry = MANIFEST
        .lines()
        .filter(|line| !line.starts_with("contract fn "))
        .collect::<Vec<_>>()
        .join("\n")
        + "\ncontract fn k_scale(int32_t, int32_t) -> int32_t from lib.qcore;\n";
    assert_eq!(
        codes(
            waiver,
            contracts(&[("lib.qcore", one_contract)]),
            manifests(&[(MANIFEST_PATH, one_entry)])
        ),
        ["Q034"]
    );
}

// --- 4. The unmanifested world does not move ---

#[test]
fn the_q023_default_is_byte_unchanged_in_both_directions() {
    // No environment and no manifest: exactly the refusal card
    // `source.extern-call.v1` shipped.
    assert_eq!(codes(ROUND_TRIP, None, None), ["Q023"]);
    // A manifest that binds nothing relevant does not change it either.
    assert_eq!(
        codes(
            ROUND_TRIP,
            None,
            manifests(&[("l1.qrlib", L1_MANIFEST.to_owned())])
        ),
        ["Q023"]
    );
}

#[test]
fn a_manifest_naming_other_symbols_leaves_the_contracted_route_untouched() {
    let bound = compile(
        ROUND_TRIP,
        slice_selection(),
        manifests(&[("l1.qrlib", L1_MANIFEST.to_owned())]),
    )
    .unwrap();
    let plain = compile(ROUND_TRIP, slice_selection(), None).unwrap();
    assert_eq!(bound.declared_assumptions(), plain.declared_assumptions());
    assert_eq!(bound.c_source, plain.c_source);
}

#[test]
fn the_level_one_route_is_unchanged_beside_the_new_one() {
    // The two levels coexist in one selection: a waiver keeps its L1 premise
    // and a contracted symbol gets the L2 pair, in the same certificate.
    let combined = format!(
        "{}\n{}",
        MANIFEST.trim_end(),
        "\n# nothing further; the L1 manifest is selected separately"
    );
    let module = compile(
        ROUND_TRIP,
        slice_selection(),
        manifests(&[
            (MANIFEST_PATH, combined),
            ("l1.qrlib", L1_MANIFEST.to_owned()),
        ]),
    )
    .expect("two manifests binding disjoint symbols compose");
    let kinds = premises(&module)
        .into_iter()
        .filter(|assumption| {
            matches!(
                assumption,
                extern_call::ExternAssumption::ManifestResidentContract { .. }
                    | extern_call::ExternAssumption::ManifestUncheckedLinkage { .. }
            )
        })
        .count();
    assert_eq!(
        kinds, 4,
        "only the four contracted symbols this program calls are named"
    );
}

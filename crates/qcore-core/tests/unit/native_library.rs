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

//! The `source.native-library.v1` manifest loader (card
//! `UNSAFE-LINKAGE-ARTIFACT`).
//!
//! Every rejection below is a *named* refusal, and each has its own row, so the
//! total-or-reject discipline cannot decay into a lenient reading one condition
//! at a time.
use super::*;

const DIGEST: &str = "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn manifest_text(body: &str) -> String {
    format!(
        "native_library manifest v1;\n\
         library demo;\n\
         version 0.1.0;\n\
         artifact libdemo.a;\n\
         hash {DIGEST};\n\
         data_model lp64;\n\
         unwinding abort;\n\
         allocator_custody closed;\n\
         thread_posture single_caller;\n\
         {body}\n"
    )
}

fn load(body: &str) -> Result<NativeLibraryManifest, ManifestRefusal> {
    load_native_library_manifest(&manifest_text(body))
}

#[test]
fn a_well_formed_manifest_binds_its_symbols_to_its_identity() {
    let manifest = load(
        "unchecked fn k_add(int32_t, int32_t) -> int32_t;\n\
         unchecked fn k_tick(uint32_t) -> void;",
    )
    .unwrap();
    assert_eq!(manifest.identity.library, "demo");
    assert_eq!(manifest.identity.version, "0.1.0");
    assert_eq!(manifest.identity.artifact, "libdemo.a");
    assert_eq!(manifest.identity.hash, DIGEST);
    assert_eq!(manifest.premises.data_model, DataModel::Lp64);
    assert_eq!(manifest.premises.unwinding, UnwindingPosture::Abort);
    assert_eq!(
        manifest.premises.allocator_custody,
        AllocatorCustody::Closed
    );
    assert_eq!(
        manifest.premises.thread_posture,
        ThreadPosture::SingleCaller
    );
    assert_eq!(manifest.unchecked.len(), 2);
    let add = &manifest.unchecked["k_add"];
    assert_eq!(add.parameters, vec![AbiType::I32, AbiType::I32]);
    assert_eq!(add.result, AbiType::I32);
    let tick = &manifest.unchecked["k_tick"];
    assert_eq!(tick.parameters, vec![AbiType::U32]);
    assert_eq!(tick.result, AbiType::Unit);
}

#[test]
fn comments_blank_lines_and_a_parameterless_entry_are_admitted() {
    let manifest = load(
        "# a comment line\n\
         \n\
         unchecked fn k_now() -> uint64_t; # trailing comment",
    )
    .unwrap();
    let entry = &manifest.unchecked["k_now"];
    assert!(entry.parameters.is_empty());
    assert_eq!(entry.result, AbiType::U64);
}

#[test]
fn the_load_is_a_function_of_the_text() {
    // A certificate consumer repeats the load and reproduces the binding; that
    // is what makes a manifest-resident premise rederivable.
    let body = "unchecked fn k_add(int32_t, int32_t) -> int32_t;";
    assert_eq!(load(body).unwrap(), load(body).unwrap());
}

#[test]
fn a_file_without_the_marker_is_rejected_rather_than_guessed_at() {
    assert_eq!(
        load_native_library_manifest("library demo;\n"),
        Err(ManifestRefusal::MissingMarker)
    );
}

#[test]
fn an_unterminated_statement_rejects() {
    let text = format!("{}\nunchecked fn k(int32_t) -> int32_t", manifest_text(""));
    assert!(matches!(
        load_native_library_manifest(&text),
        Err(ManifestRefusal::UnterminatedStatement { .. })
    ));
}

#[test]
fn every_mandatory_field_is_mandatory() {
    for field in MANDATORY_FIELDS {
        let text = manifest_text("unchecked fn k(int32_t) -> int32_t;")
            .lines()
            .filter(|line| !line.starts_with(&format!("{field} ")))
            .collect::<Vec<_>>()
            .join("\n");
        assert_eq!(
            load_native_library_manifest(&text),
            Err(ManifestRefusal::MissingField { field }),
            "dropping `{field}` must reject the whole manifest"
        );
    }
}

#[test]
fn an_unknown_field_rejects_the_whole_manifest() {
    let text = format!(
        "{}\nprovenance somewhere;\n",
        manifest_text("unchecked fn k(int32_t) -> int32_t;")
    );
    assert_eq!(
        load_native_library_manifest(&text),
        Err(ManifestRefusal::UnknownField {
            field: "provenance".to_owned()
        })
    );
}

#[test]
fn a_repeated_field_rejects() {
    let text = format!(
        "{}\nlibrary other;\n",
        manifest_text("unchecked fn k(int32_t) -> int32_t;")
    );
    assert_eq!(
        load_native_library_manifest(&text),
        Err(ManifestRefusal::RepeatedField {
            field: "library".to_owned()
        })
    );
}

#[test]
fn each_abi_premise_admits_exactly_its_v1_value() {
    for (field, admitted, wrong) in [
        ("data_model", "lp64", "ilp32"),
        ("unwinding", "abort", "propagate"),
        ("allocator_custody", "closed", "open"),
        ("thread_posture", "single_caller", "any_thread"),
    ] {
        let text = manifest_text("unchecked fn k(int32_t) -> int32_t;").replace(
            &format!("{field} {admitted};"),
            &format!("{field} {wrong};"),
        );
        assert!(
            matches!(
                load_native_library_manifest(&text),
                Err(ManifestRefusal::UnknownValue { field: f, .. }) if f == field
            ),
            "`{field} {wrong}` must reject"
        );
    }
}

#[test]
fn the_stated_hash_must_be_well_formed() {
    for hash in [
        "sha256:short",
        "md5:0123456789abcdef0123456789abcdef",
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        "sha256:0123456789ABCDEF0123456789abcdef0123456789abcdef0123456789abcdef",
    ] {
        let text = manifest_text("unchecked fn k(int32_t) -> int32_t;").replace(DIGEST, hash);
        assert!(
            matches!(
                load_native_library_manifest(&text),
                Err(ManifestRefusal::MalformedHash { .. })
            ),
            "`{hash}` must reject: an identity a reader cannot interpret is not a premise"
        );
    }
}

#[test]
fn a_contracted_entry_without_its_contract_reference_is_refused() {
    // Design §4.2: a contract-less entry must be *explicitly marked*, so a
    // dropped contract can never be silently read as a waiver. Card
    // `UNSAFE-LINKAGE-ARTIFACT` met that by refusing every contracted entry;
    // card `NATIVE-LIBRARY-CONTRACT-SLICE` implements the entry and keeps the
    // requirement, because the two kinds are told apart by their **keyword**
    // and a `contract fn` that names no contract module is refused rather than
    // demoted to a waiver.
    assert!(matches!(
        load("contract fn k_add(int32_t, int32_t) -> int32_t;"),
        Err(ManifestRefusal::MalformedContractedEntry { .. })
    ));
    // And the converse: an `unchecked fn` carrying a `from MODULE` tail is a
    // malformed waiver, not a contract entry inferred from its shape.
    assert!(matches!(
        load("unchecked fn k_add(int32_t) -> int32_t from lib.qcore;"),
        Err(ManifestRefusal::MalformedEntry { .. })
    ));
}

#[test]
fn a_pointer_entry_refuses_in_either_direction() {
    for body in [
        "unchecked fn k(uint8_t *) -> int32_t;",
        "unchecked fn k(const uint8_t *) -> int32_t;",
        "unchecked fn k(uint8_t **) -> int32_t;",
        "unchecked fn k(int32_t) -> uint8_t *;",
    ] {
        assert!(
            matches!(
                load(body),
                Err(ManifestRefusal::PointerInUncheckedEntry { .. })
            ),
            "`{body}` must refuse: an artifact identity says which code, not what it does with a pointer"
        );
    }
}

#[test]
fn a_unit_parameter_and_an_unknown_spelling_refuse_separately() {
    assert!(matches!(
        load("unchecked fn k(void) -> int32_t;"),
        Err(ManifestRefusal::UnitParameterInEntry { parameter: 0, .. })
    ));
    assert!(matches!(
        load("unchecked fn k(struct point) -> int32_t;"),
        Err(ManifestRefusal::UnknownParameterSpelling { .. })
    ));
    assert!(matches!(
        load("unchecked fn k(int32_t) -> struct point;"),
        Err(ManifestRefusal::UnknownResultSpelling { .. })
    ));
}

#[test]
fn a_malformed_entry_refuses() {
    for body in [
        "unchecked fn k(int32_t);",
        "unchecked fn k -> int32_t;",
        "unchecked fn 9k(int32_t) -> int32_t;",
        "unchecked k(int32_t) -> int32_t;",
        "unchecked fn k(int32_t) ->;",
    ] {
        assert!(
            matches!(load(body), Err(ManifestRefusal::MalformedEntry { .. })),
            "`{body}` must refuse"
        );
    }
}

#[test]
fn a_symbol_bound_twice_rejects_the_manifest() {
    assert_eq!(
        load(
            "unchecked fn k(int32_t) -> int32_t;\n\
             unchecked fn k(int32_t) -> int64_t;"
        ),
        Err(ManifestRefusal::SymbolBoundTwice {
            symbol: "k".to_owned()
        })
    );
}

#[test]
fn a_manifest_binding_nothing_rejects() {
    assert_eq!(load(""), Err(ManifestRefusal::NoSymbols));
}

#[test]
fn the_pointer_spelling_variants_a_header_may_use_are_read_the_same_way() {
    assert_eq!(
        parse_c_declaration_type("uint8_t *"),
        Some(AbiType::PtrMutU8)
    );
    assert_eq!(
        parse_c_declaration_type("uint8_t*"),
        Some(AbiType::PtrMutU8)
    );
    assert_eq!(
        parse_c_declaration_type("const  uint8_t  *"),
        Some(AbiType::PtrConstU8)
    );
    assert_eq!(parse_c_declaration_type("uint8_t const *"), None);
    assert_eq!(parse_c_declaration_type("int"), None);
}

// --- The selection ---

fn selection(entries: &[(&str, &str)]) -> Result<NativeLibraries, SelectionRefusal> {
    let sources = entries
        .iter()
        .map(|(module, body)| ((*module).to_owned(), manifest_text(body)))
        .collect();
    NativeLibraries::load(&sources)
}

#[test]
fn an_empty_selection_resolves_nothing() {
    let libraries = NativeLibraries::default();
    assert!(libraries.is_empty());
    assert!(libraries.unchecked_binding("k_add").is_none());
}

#[test]
fn a_selection_resolves_a_symbol_to_its_own_manifests_identity() {
    let libraries = selection(&[
        ("first.qrlib", "unchecked fn k_add(int32_t) -> int32_t;"),
        ("second.qrlib", "unchecked fn k_other(uint32_t) -> void;"),
    ])
    .unwrap();
    let binding = libraries.unchecked_binding("k_other").unwrap();
    assert_eq!(binding.module, "second.qrlib");
    assert_eq!(binding.entry.result, AbiType::Unit);
    assert!(libraries.unchecked_binding("k_absent").is_none());
    assert_eq!(
        libraries.symbols(),
        ["k_add", "k_other"].into_iter().collect()
    );
}

#[test]
fn one_unloadable_manifest_rejects_the_whole_selection() {
    // A partially loaded selection would silently downgrade a manifest-resident
    // premise to a bare one — a weaker published statement, obtained by mistake.
    assert!(matches!(
        selection(&[
            ("good.qrlib", "unchecked fn k_add(int32_t) -> int32_t;"),
            ("bad.qrlib", "contract fn k_other(uint32_t) -> void;"),
        ]),
        Err(SelectionRefusal::Manifest { module, .. }) if module == "bad.qrlib"
    ));
}

#[test]
fn two_manifests_may_not_bind_the_same_symbol() {
    assert!(matches!(
        selection(&[
            ("first.qrlib", "unchecked fn k_add(int32_t) -> int32_t;"),
            ("second.qrlib", "unchecked fn k_add(int32_t) -> int32_t;"),
        ]),
        Err(SelectionRefusal::SymbolBoundTwice { symbol, .. }) if symbol == "k_add"
    ));
}

// --- The type gate ---

#[test]
fn an_agreeing_declaration_passes_the_gate() {
    let manifest = load("unchecked fn k_add(int32_t, int32_t) -> int32_t;").unwrap();
    let entry = &manifest.unchecked["k_add"];
    assert_eq!(
        check_manifest_agreement(entry, &[AbiType::I32, AbiType::I32], AbiType::I32),
        Ok(())
    );
}

#[test]
fn every_way_of_disagreeing_with_the_manifest_refuses() {
    let manifest = load("unchecked fn k_add(int32_t, int32_t) -> int32_t;").unwrap();
    let entry = &manifest.unchecked["k_add"];
    assert_eq!(
        check_manifest_agreement(entry, &[AbiType::I32], AbiType::I32),
        Err(ManifestDisagreement::ParameterCount {
            declared: 1,
            manifest: 2
        })
    );
    assert_eq!(
        check_manifest_agreement(entry, &[AbiType::I32, AbiType::I64], AbiType::I32),
        Err(ManifestDisagreement::Parameter {
            parameter: 1,
            declared: AbiType::I64,
            manifest: AbiType::I32
        })
    );
    assert_eq!(
        check_manifest_agreement(entry, &[AbiType::I32, AbiType::I32], AbiType::U32),
        Err(ManifestDisagreement::Result {
            declared: AbiType::U32,
            manifest: AbiType::I32
        })
    );
}

// --- The premise a manifest-resident callee contributes ---

fn residence() -> ManifestResidence {
    let manifest = load("unchecked fn k_add(int32_t, int32_t) -> int32_t;").unwrap();
    ManifestResidence {
        identity: manifest.identity,
        premises: manifest.premises,
        contract: None,
        header: None,
    }
}

#[test]
fn a_manifest_does_not_move_the_admitted_surface() {
    use crate::extern_call::{manifest_unchecked_callee, unchecked_callee};

    // Identical decisions, in both directions, for every position the closed
    // table decides. An artifact identity says which code, not what it does
    // with your pointer.
    for (parameters, result) in [
        (vec![AbiType::I32, AbiType::I32], AbiType::I32),
        (vec![], AbiType::Unit),
        (vec![AbiType::PtrConstU8], AbiType::I32),
        (vec![AbiType::Unit], AbiType::I32),
        (vec![AbiType::U64], AbiType::PtrMutU8),
    ] {
        let bare = unchecked_callee("k", &parameters, result);
        let resident = manifest_unchecked_callee("k", &parameters, result, residence());
        assert_eq!(
            bare.is_ok(),
            resident.is_ok(),
            "the manifest must not change what is admitted"
        );
        if let (Err(bare), Err(resident)) = (&bare, &resident) {
            assert_eq!(bare, resident, "and it must not change the refusal either");
        }
    }
}

#[test]
fn the_premise_a_callee_contributes_is_a_total_match_with_three_arms() {
    use crate::extern_call::{
        ExternAssumption, ExternTrust, assumption_for_callee, manifest_unchecked_callee,
        unchecked_callee,
    };

    let bare = unchecked_callee("k_add", &[AbiType::I32], AbiType::I32).unwrap();
    assert_eq!(bare.trust, ExternTrust::UncheckedLinkage);
    assert!(bare.residence.is_none());
    assert!(matches!(
        assumption_for_callee(&bare),
        ExternAssumption::UncheckedLinkage { .. }
    ));

    let resident =
        manifest_unchecked_callee("k_add", &[AbiType::I32], AbiType::I32, residence()).unwrap();
    assert_eq!(resident.trust, ExternTrust::UncheckedLinkage);
    match assumption_for_callee(&resident) {
        ExternAssumption::ManifestUncheckedLinkage {
            callee,
            signature,
            residence,
        } => {
            assert_eq!(callee, "k_add");
            assert_eq!(signature, "(int32_t) -> int32_t");
            assert_eq!(residence.identity.library, "demo");
        }
        other => panic!("a manifest-resident waiver must name its artifact: {other:?}"),
    }
    // The two callees differ only in the premise they contribute.
    assert_eq!(bare.parameters, resident.parameters);
    assert_eq!(bare.result, resident.result);
    assert_ne!(
        assumption_for_callee(&bare),
        assumption_for_callee(&resident)
    );
}

// --- The contract slice (card `NATIVE-LIBRARY-CONTRACT-SLICE`) ---
//
// Level L2 of the linkage spectrum: where a library's **pointer** symbols
// belong. The rows below cover the grammar of a `contract fn` entry, the
// may-not-bind-twice rule across both entry kinds, and design §4.1's binding
// totality in both directions against a declared environment.

const SLICE: &str = "fixtures/lib.qcore";

fn declared(names: &[(&str, &str)]) -> BTreeMap<String, String> {
    names
        .iter()
        .map(|(name, module)| ((*name).to_owned(), (*module).to_owned()))
        .collect()
}

#[test]
fn a_contract_slice_entry_binds_its_symbol_to_a_declared_contract_module() {
    let manifest = load(
        "contract fn k_alloc(uintptr_t) -> uint8_t * from fixtures/lib.qcore;\n\
         contract fn k_free(uint8_t *) -> void from fixtures/lib.qcore;",
    )
    .unwrap();
    assert!(manifest.unchecked.is_empty());
    assert_eq!(manifest.contracted.len(), 2);
    let alloc = &manifest.contracted["k_alloc"];
    assert_eq!(alloc.parameters, vec![EntryType::Abi(AbiType::Usize)]);
    assert_eq!(alloc.result, EntryType::Abi(AbiType::PtrMutU8));
    // Card `W48-QRLIB-LINKAGE`: a v1 entry states no export.
    assert_eq!(alloc.export, None);
    assert_eq!(alloc.contract_module, SLICE);
    assert_eq!(manifest.slice(), BTreeSet::from([SLICE]));
}

#[test]
fn the_pointer_a_contract_less_entry_may_not_declare_is_admitted_here() {
    // The seam card `UNSAFE-LINKAGE-ARTIFACT` left visible, closed from the
    // other side: `PointerInUncheckedEntry` was not weakened, it acquired a
    // destination. The *same* spelling refuses as a waiver and binds as a
    // contract slice entry.
    for spelling in ["uint8_t *", "const uint8_t *", "uint8_t **"] {
        assert!(
            matches!(
                load(&format!("unchecked fn k({spelling}) -> int32_t;")),
                Err(ManifestRefusal::PointerInUncheckedEntry { .. })
            ),
            "a waiver must still refuse `{spelling}`"
        );
        let manifest = load(&format!(
            "contract fn k({spelling}) -> int32_t from fixtures/lib.qcore;"
        ))
        .unwrap_or_else(|refusal| panic!("`{spelling}` must bind as a contract entry: {refusal}"));
        assert_eq!(manifest.contracted.len(), 1);
    }
    // A pointer **result** likewise.
    assert!(matches!(
        load("unchecked fn k(int32_t) -> uint8_t *;"),
        Err(ManifestRefusal::PointerInUncheckedEntry { .. })
    ));
    assert!(load("contract fn k(int32_t) -> uint8_t * from fixtures/lib.qcore;").is_ok());
}

#[test]
fn a_unit_parameter_is_refused_in_a_contract_entry_too() {
    // The ABI vocabulary admits `void` as a result type only. A contract has
    // modes for a pointer; it has no value for a `void` argument.
    assert!(matches!(
        load("contract fn k(void) -> int32_t from fixtures/lib.qcore;"),
        Err(ManifestRefusal::UnitParameterInEntry { .. })
    ));
}

#[test]
fn an_unknown_spelling_in_a_contract_entry_refuses_by_position() {
    assert!(matches!(
        load("contract fn k(int) -> int32_t from fixtures/lib.qcore;"),
        Err(ManifestRefusal::UnknownParameterSpelling { .. })
    ));
    assert!(matches!(
        load("contract fn k(int32_t) -> struct foo from fixtures/lib.qcore;"),
        Err(ManifestRefusal::UnknownResultSpelling { .. })
    ));
}

#[test]
fn a_contract_reference_that_could_not_be_a_selection_key_refuses() {
    assert!(matches!(
        load("contract fn k(int32_t) -> int32_t from a b;"),
        Err(ManifestRefusal::MalformedContractReference { .. })
    ));
}

#[test]
fn a_symbol_may_not_be_bound_twice_across_the_two_entry_kinds() {
    // Both orders, because an entry that is at once waived and contracted has
    // no defined premise and choosing one by line order would make the premise
    // depend on the order the author wrote.
    assert!(matches!(
        load(
            "unchecked fn k(int32_t) -> int32_t;\n\
             contract fn k(int32_t) -> int32_t from fixtures/lib.qcore;"
        ),
        Err(ManifestRefusal::SymbolBoundTwice { .. })
    ));
    assert!(matches!(
        load(
            "contract fn k(int32_t) -> int32_t from fixtures/lib.qcore;\n\
             unchecked fn k(int32_t) -> int32_t;"
        ),
        Err(ManifestRefusal::SymbolBoundTwice { .. })
    ));
}

#[test]
fn a_manifest_binding_only_contracts_is_not_a_manifest_binding_nothing() {
    // `NoSymbols` counts both kinds: a contract slice is a binding.
    assert!(load("contract fn k(int32_t) -> int32_t from fixtures/lib.qcore;").is_ok());
    assert!(matches!(load("# nothing"), Err(ManifestRefusal::NoSymbols)));
}

#[test]
fn a_contracted_binding_names_its_module_and_a_waiver_never_does() {
    let libraries = selection(&[(
        "lib.qrlib",
        "unchecked fn k_tick(uint32_t) -> void;\n\
         contract fn k_alloc(uintptr_t) -> uint8_t * from fixtures/lib.qcore;",
    )])
    .unwrap();
    let waived = libraries.unchecked_binding("k_tick").unwrap();
    assert_eq!(waived.residence().contract, None);
    assert!(libraries.contracted_binding("k_tick").is_none());
    let contracted = libraries.contracted_binding("k_alloc").unwrap();
    assert_eq!(contracted.residence().contract.as_deref(), Some(SLICE));
    assert!(libraries.unchecked_binding("k_alloc").is_none());
    assert!(libraries.binds_contracts());
    assert_eq!(libraries.slice(), BTreeSet::from([SLICE]));
}

#[test]
fn two_manifests_may_not_bind_one_symbol_whatever_the_entry_kinds() {
    assert!(matches!(
        selection(&[
            ("first.qrlib", "unchecked fn k(int32_t) -> int32_t;"),
            (
                "second.qrlib",
                "contract fn k(int32_t) -> int32_t from fixtures/lib.qcore;"
            ),
        ]),
        Err(SelectionRefusal::SymbolBoundTwice { .. })
    ));
}

// --- Design §4.1's binding totality, in both directions ---

#[test]
fn a_slice_with_no_contract_environment_at_all_refuses() {
    let libraries = selection(&[(
        "lib.qrlib",
        "contract fn k_alloc(uintptr_t) -> uint8_t * from fixtures/lib.qcore;",
    )])
    .unwrap();
    assert!(matches!(
        check_slice_totality(&libraries, None),
        Err(SliceRefusal::NoContractEnvironment { .. })
    ));
}

#[test]
fn an_entry_whose_contract_the_environment_does_not_declare_refuses() {
    let libraries = selection(&[(
        "lib.qrlib",
        "contract fn k_alloc(uintptr_t) -> uint8_t * from fixtures/lib.qcore;",
    )])
    .unwrap();
    assert!(matches!(
        check_slice_totality(&libraries, Some(&declared(&[("k_other", SLICE)]))),
        Err(SliceRefusal::UnresolvedContract { .. })
    ));
}

#[test]
fn an_entry_naming_the_wrong_module_refuses_rather_than_resolving_by_name() {
    // The reference is compared exactly. A contract in another module is
    // another contract, and resolving by bare symbol name would let a manifest
    // attribute a build to a declaration it never named.
    let libraries = selection(&[(
        "lib.qrlib",
        "contract fn k_alloc(uintptr_t) -> uint8_t * from fixtures/lib.qcore;",
    )])
    .unwrap();
    assert!(matches!(
        check_slice_totality(
            &libraries,
            Some(&declared(&[("k_alloc", "fixtures/other.qcore")]))
        ),
        Err(SliceRefusal::ContractInAnotherModule { .. })
    ));
}

#[test]
fn a_contract_in_the_slice_that_no_entry_binds_refuses() {
    // The other direction of §4.1. The slice is the library's exported surface,
    // so a declaration inside it that names no entry makes the binding partial.
    let libraries = selection(&[(
        "lib.qrlib",
        "contract fn k_alloc(uintptr_t) -> uint8_t * from fixtures/lib.qcore;",
    )])
    .unwrap();
    assert!(matches!(
        check_slice_totality(
            &libraries,
            Some(&declared(&[("k_alloc", SLICE), ("k_free", SLICE)]))
        ),
        Err(SliceRefusal::UnboundContract { .. })
    ));
    // A module *outside* the slice is not the manifest's business, so its
    // declarations are not demanded.
    assert!(
        check_slice_totality(
            &libraries,
            Some(&declared(&[
                ("k_alloc", SLICE),
                ("k_free", "fixtures/elsewhere.qcore")
            ]))
        )
        .is_ok()
    );
}

#[test]
fn a_selection_binding_no_contract_is_total_vacuously() {
    let libraries = selection(&[("lib.qrlib", "unchecked fn k(int32_t) -> int32_t;")]).unwrap();
    assert!(!libraries.binds_contracts());
    assert!(check_slice_totality(&libraries, None).is_ok());
    assert!(check_slice_totality(&libraries, Some(&declared(&[("k_other", SLICE)]))).is_ok());
}

// --- The stacked premise ---

#[test]
fn a_manifest_bound_contract_stacks_its_artifact_premise_and_replaces_nothing() {
    use crate::extern_call::{
        CallParameter, CallResult, ExternAssumption, ExternCallee, ExternTrust,
        assumption_for_callee, assumptions_for_callee, manifest_contracted_callee,
    };

    let contracted = ExternCallee {
        name: "k_alloc".to_owned(),
        trust: ExternTrust::Contracted,
        parameters: vec![CallParameter::Data(AbiType::Usize)],
        result: CallResult::OwnedNullable,
        result_abi: AbiType::PtrMutU8,
        out_parameter: None,
        realloc_parameter: None,
        residence: None,
    };
    // Unmanifested: exactly one premise, the declared contract, unmoved.
    assert_eq!(
        assumptions_for_callee(&contracted),
        vec![ExternAssumption::DeclaredContract("k_alloc".to_owned())]
    );

    let mut resident = residence();
    resident.contract = Some(SLICE.to_owned());
    let bound = manifest_contracted_callee(contracted.clone(), resident);
    let premises = assumptions_for_callee(&bound);
    assert_eq!(premises.len(), 2, "the premises stack; neither replaces");
    assert_eq!(
        premises[0],
        ExternAssumption::DeclaredContract("k_alloc".to_owned()),
        "the contract premise is still there"
    );
    match &premises[1] {
        ExternAssumption::ManifestResidentContract {
            callee,
            contract_module,
            residence,
        } => {
            assert_eq!(callee, "k_alloc");
            assert_eq!(contract_module, SLICE);
            assert_eq!(residence.identity.library, "demo");
        }
        other => panic!("a manifest-bound contract must name its artifact: {other:?}"),
    }
    // The trust premise is unchanged by the binding, which is what makes the
    // second one an addition rather than a substitution.
    assert_eq!(
        assumption_for_callee(&bound),
        assumption_for_callee(&contracted)
    );
    // And nothing about the callee itself moved.
    assert_eq!(bound.parameters, contracted.parameters);
    assert_eq!(bound.result, contracted.result);
    assert_eq!(bound.trust, contracted.trust);
}

#[test]
fn the_contracted_agreement_gate_is_the_same_comparison_as_the_waived_one() {
    let manifest =
        load("contract fn k_alloc(uintptr_t) -> uint8_t * from fixtures/lib.qcore;").unwrap();
    let entry = &manifest.contracted["k_alloc"];
    assert!(check_contracted_agreement(entry, &[AbiType::Usize], AbiType::PtrMutU8).is_ok());
    assert_eq!(
        check_contracted_agreement(entry, &[], AbiType::PtrMutU8),
        Err(ManifestDisagreement::ParameterCount {
            declared: 0,
            manifest: 1
        })
    );
    assert_eq!(
        check_contracted_agreement(entry, &[AbiType::U32], AbiType::PtrMutU8),
        Err(ManifestDisagreement::Parameter {
            parameter: 0,
            declared: AbiType::U32,
            manifest: AbiType::Usize
        })
    );
    assert_eq!(
        check_contracted_agreement(entry, &[AbiType::Usize], AbiType::PtrConstU8),
        Err(ManifestDisagreement::Result {
            declared: AbiType::PtrConstU8,
            manifest: AbiType::PtrMutU8
        })
    );
}

// --- Design §4.4's closed allocator custody, which becomes real here ---

fn shapes(entries: &[(&str, bool, bool)]) -> BTreeMap<String, CustodyShape> {
    entries
        .iter()
        .map(|(name, acquires, discharges)| {
            (
                (*name).to_owned(),
                CustodyShape {
                    acquires: *acquires,
                    discharges: *discharges,
                },
            )
        })
        .collect()
}

fn acquiring_slice() -> NativeLibraries {
    selection(&[(
        "lib.qrlib",
        "contract fn k_alloc(uintptr_t) -> uint8_t * from fixtures/lib.qcore;\n\
         contract fn k_free(uint8_t *) -> void from fixtures/lib.qcore;",
    )])
    .unwrap()
}

#[test]
fn a_slice_that_can_acquire_and_never_discharge_refuses() {
    // A library handing out custody with no declared release entry point could
    // only leak or lie, and `allocator_custody closed` would be stating what its
    // own slice contradicts.
    assert!(matches!(
        check_closed_allocator_custody(
            &acquiring_slice(),
            &shapes(&[("k_alloc", true, false), ("k_free", false, false)])
        ),
        Err(SliceRefusal::OpenAllocatorCustody { .. })
    ));
}

#[test]
fn a_slice_with_a_discharge_beside_its_acquisition_is_closed() {
    assert!(
        check_closed_allocator_custody(
            &acquiring_slice(),
            &shapes(&[("k_alloc", true, false), ("k_free", false, true)])
        )
        .is_ok()
    );
}

#[test]
fn a_slice_that_cannot_acquire_is_closed_vacuously() {
    assert!(
        check_closed_allocator_custody(
            &acquiring_slice(),
            &shapes(&[("k_alloc", false, false), ("k_free", false, false)])
        )
        .is_ok()
    );
    // And a contract-less selection has nothing to check at all, which is the
    // sense in which card `UNSAFE-LINKAGE-ARTIFACT` satisfied this vacuously.
    let waived = selection(&[("lib.qrlib", "unchecked fn k(int32_t) -> int32_t;")]).unwrap();
    assert!(check_closed_allocator_custody(&waived, &BTreeMap::new()).is_ok());
}

#[test]
fn a_shape_this_check_cannot_read_supplies_no_discharge() {
    // Fail-closed in the direction that matters: an absent projection cannot be
    // the discharge a manifest is missing.
    assert!(matches!(
        check_closed_allocator_custody(&acquiring_slice(), &shapes(&[("k_alloc", true, false)])),
        Err(SliceRefusal::OpenAllocatorCustody { .. })
    ));
}

// --- The source file's own library binding (card
// `NATIVE-LIBRARY-SOURCE-BINDING`) ---

/// A manifest whose `library` field is the given name rather than the fixed
/// `demo` of [`manifest_text`], so a selection can carry two distinguishable
/// names.
fn named_manifest_text(library: &str, body: &str) -> String {
    manifest_text(body).replace("library demo;", &format!("library {library};"))
}

fn named_selection(entries: &[(&str, &str, &str)]) -> NativeLibraries {
    let sources = entries
        .iter()
        .map(|(module, library, body)| ((*module).to_owned(), named_manifest_text(library, body)))
        .collect();
    NativeLibraries::load(&sources).expect("this fixture selection loads")
}

fn two_libraries() -> NativeLibraries {
    named_selection(&[
        ("a.qrlib", "libA", "unchecked fn k_a(int32_t) -> int32_t;"),
        ("b.qrlib", "libB", "unchecked fn k_b(int32_t) -> int32_t;"),
    ])
}

#[test]
fn an_empty_binding_list_is_the_unbound_selection_wide_view() {
    // The opt-in property every pre-card program rests on.
    let libraries = two_libraries();
    let view = libraries.bind_source(&[]).unwrap();
    assert!(!view.is_bound());
    assert!(view.bindings().is_empty());
    assert!(view.unchecked_binding("k_a").is_some());
    assert!(view.unchecked_binding("k_b").is_some());
    assert!(view.foreign_symbol("k_a").is_none());
    assert!(view.foreign_symbol("k_b").is_none());
}

#[test]
fn a_binding_resolves_its_own_manifest_and_narrows_away_the_other() {
    let libraries = two_libraries();
    let view = libraries.bind_source(&["libA".to_owned()]).unwrap();
    assert!(view.is_bound());
    assert_eq!(view.bindings().len(), 1);
    assert_eq!(view.bindings()[0].named, "libA");
    assert_eq!(view.bindings()[0].manifest, "a.qrlib");
    assert_eq!(view.bindings()[0].identity.library, "libA");
    assert!(view.unchecked_binding("k_a").is_some());
    assert!(view.unchecked_binding("k_b").is_none());
}

#[test]
fn a_symbol_narrowed_away_is_refused_rather_than_silently_unresident() {
    // The load-bearing direction: where a binding removes a resolution, the file
    // is owed a refusal. Falling through to the weaker premise would let a file
    // stop its certificate naming a build by binding something else.
    let libraries = two_libraries();
    let view = libraries.bind_source(&["libA".to_owned()]).unwrap();
    assert!(matches!(
        view.foreign_symbol("k_b"),
        Some(BindingRefusal::ForeignSymbol { .. })
    ));
    assert!(view.foreign_symbol("k_a").is_none());
}

#[test]
fn a_symbol_no_selected_manifest_binds_is_foreign_to_nobody() {
    // Absence of evidence is not evidence, and the two absences differ.
    let libraries = two_libraries();
    let view = libraries.bind_source(&["libA".to_owned()]).unwrap();
    assert!(view.foreign_symbol("k_unclaimed").is_none());
    assert!(view.unchecked_binding("k_unclaimed").is_none());
}

#[test]
fn a_library_name_no_selected_manifest_declares_refuses_and_names_the_selection() {
    let libraries = two_libraries();
    match libraries.bind_source(&["libC".to_owned()]) {
        Err(BindingRefusal::UnknownLibrary { named, available }) => {
            assert_eq!(named, "libC");
            assert_eq!(available, vec!["libA".to_owned(), "libB".to_owned()]);
        }
        other => panic!("an unknown library must refuse by name: {other:?}"),
    }
}

#[test]
fn a_binding_against_an_empty_selection_refuses() {
    let libraries = NativeLibraries::default();
    assert!(matches!(
        libraries.bind_source(&["libA".to_owned()]),
        Err(BindingRefusal::UnknownLibrary { .. })
    ));
}

#[test]
fn two_manifests_declaring_one_library_name_refuse_as_ambiguous() {
    // The ground `SelectionRefusal::SymbolBoundTwice` refuses its own ambiguity
    // on: a binding that is not a function has no defined premise.
    let libraries = named_selection(&[
        ("a.qrlib", "libA", "unchecked fn k_a(int32_t) -> int32_t;"),
        ("b.qrlib", "libA", "unchecked fn k_b(int32_t) -> int32_t;"),
    ]);
    match libraries.bind_source(&["libA".to_owned()]) {
        Err(BindingRefusal::AmbiguousLibrary { named, manifests }) => {
            assert_eq!(named, "libA");
            assert_eq!(manifests, vec!["a.qrlib".to_owned(), "b.qrlib".to_owned()]);
        }
        other => panic!("an ambiguous library must refuse: {other:?}"),
    }
}

#[test]
fn a_repeated_binding_of_one_name_refuses() {
    let libraries = two_libraries();
    assert!(matches!(
        libraries.bind_source(&["libA".to_owned(), "libA".to_owned()]),
        Err(BindingRefusal::RepeatedBinding { .. })
    ));
}

#[test]
fn two_bindings_resolve_both_manifests_and_leave_nothing_foreign() {
    // Binding is not one-per-file: a file may bind two libraries, and then
    // neither is foreign to it.
    let libraries = two_libraries();
    let view = libraries
        .bind_source(&["libA".to_owned(), "libB".to_owned()])
        .unwrap();
    assert_eq!(view.bindings().len(), 2);
    assert!(view.unchecked_binding("k_a").is_some());
    assert!(view.unchecked_binding("k_b").is_some());
    assert!(view.foreign_symbol("k_b").is_none());
}

#[test]
fn a_binding_only_ever_narrows() {
    // Hard invariant 1's direction, over the whole selection: every symbol a
    // bound view resolves, the unbound view resolved too.
    let libraries = two_libraries();
    let unbound = libraries.bind_source(&[]).unwrap();
    let view = libraries.bind_source(&["libA".to_owned()]).unwrap();
    for symbol in ["k_a", "k_b", "k_unclaimed"] {
        if view.unchecked_binding(symbol).is_some() {
            assert!(
                unbound.unchecked_binding(symbol).is_some(),
                "a binding may not add a resolution"
            );
        }
    }
}

#[test]
fn the_binding_carries_the_manifests_premises_and_header() {
    let libraries = two_libraries();
    let view = libraries.bind_source(&["libA".to_owned()]).unwrap();
    let binding = &view.bindings()[0];
    assert_eq!(binding.premises.data_model, DataModel::Lp64);
    assert_eq!(binding.premises.unwinding, UnwindingPosture::Abort);
    assert_eq!(binding.premises.allocator_custody, AllocatorCustody::Closed);
    assert_eq!(binding.premises.thread_posture, ThreadPosture::SingleCaller);
    assert_eq!(binding.identity.hash, DIGEST);
    // The fixture manifests name no header, which is every manifest written
    // before card `NATIVE-LIBRARY-HEADER`.
    assert!(binding.header.is_none());
}

#[test]
fn the_custody_check_is_per_manifest_and_not_across_a_selection() {
    // Two libraries do not close each other's custody: the premise
    // `allocator_custody closed` is stated by one manifest about one artifact.
    let libraries = selection(&[
        (
            "first.qrlib",
            "contract fn k_alloc(uintptr_t) -> uint8_t * from fixtures/lib.qcore;",
        ),
        (
            "second.qrlib",
            "contract fn k_free(uint8_t *) -> void from fixtures/other.qcore;",
        ),
    ])
    .unwrap();
    assert!(matches!(
        check_closed_allocator_custody(
            &libraries,
            &shapes(&[("k_alloc", true, false), ("k_free", false, true)])
        ),
        Err(SliceRefusal::OpenAllocatorCustody { .. })
    ));
}

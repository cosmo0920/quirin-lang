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
    for field in HEADER_FIELDS {
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
fn a_contracted_entry_is_refused_and_never_read_as_a_waiver() {
    // Design §4.2: a contract-less entry must be *explicitly marked*, so a
    // dropped contract can never be silently read as a waiver. This slice
    // implements no contracted entry, so it refuses one by name.
    assert!(matches!(
        load("contract fn k_add(int32_t, int32_t) -> int32_t;"),
        Err(ManifestRefusal::ContractedEntry { .. })
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
    assert!(libraries.binding("k_add").is_none());
}

#[test]
fn a_selection_resolves_a_symbol_to_its_own_manifests_identity() {
    let libraries = selection(&[
        ("first.qcorelib", "unchecked fn k_add(int32_t) -> int32_t;"),
        ("second.qcorelib", "unchecked fn k_other(uint32_t) -> void;"),
    ])
    .unwrap();
    let binding = libraries.binding("k_other").unwrap();
    assert_eq!(binding.module, "second.qcorelib");
    assert_eq!(binding.entry.result, AbiType::Unit);
    assert!(libraries.binding("k_absent").is_none());
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
            ("good.qcorelib", "unchecked fn k_add(int32_t) -> int32_t;"),
            ("bad.qcorelib", "contract fn k_other(uint32_t) -> void;"),
        ]),
        Err(SelectionRefusal::Manifest { module, .. }) if module == "bad.qcorelib"
    ));
}

#[test]
fn two_manifests_may_not_bind_the_same_symbol() {
    assert!(matches!(
        selection(&[
            ("first.qcorelib", "unchecked fn k_add(int32_t) -> int32_t;"),
            ("second.qcorelib", "unchecked fn k_add(int32_t) -> int32_t;"),
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

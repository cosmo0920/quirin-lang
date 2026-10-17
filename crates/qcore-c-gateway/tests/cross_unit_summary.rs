//! The opt-in cross-unit carrier must re-derive both bodies and reject a
//! missing or changed home, even when an imported contract looks valid.

use std::collections::{BTreeMap, BTreeSet};

use qcore_c_gateway::legacy_c::{
    C11TranslationUnit, LegacyFunctionScope, ResourceContracts, UnrestrictedLegacyResult,
    analyze_prepared_translation_unit_with_cross_unit, check_legacy_safety_certificate,
    export_cross_unit_homes, prepare_translation_unit,
};

fn clang_available() -> bool {
    qcore_c_gateway::clang_command().is_ok()
}

#[test]
fn home_and_caller_replay_and_missing_evidence_refuses() {
    if !clang_available() {
        return;
    }
    let home = prepare_translation_unit(&C11TranslationUnit {
        source: "void callee(void) { return; }\n",
        function_scope: LegacyFunctionScope::MainSourceDefinitions,
        ..Default::default()
    });
    let caller = prepare_translation_unit(&C11TranslationUnit {
        source: "void callee(void); void caller(void) { callee(); return; }\n",
        function_scope: LegacyFunctionScope::MainSourceDefinitions,
        ..Default::default()
    });
    let base = ResourceContracts::default();
    let digest = "a".repeat(64);
    let exports = export_cross_unit_homes(
        &home,
        &base,
        &BTreeSet::from(["callee".to_owned()]),
        &digest,
    )
    .expect("home export");
    assert_eq!(exports.len(), 1);
    let mut environment = base;
    environment.set_inferred_imports(BTreeMap::from([(
        "callee".to_owned(),
        exports[0].contract.clone(),
    )]));
    environment.set_cross_unit_import_identities(BTreeMap::from([(
        "callee".to_owned(),
        exports[0].home.import_identity(),
    )]));
    environment.set_link_resolution_sha256(Some(digest));
    assert!(
        analyze_prepared_translation_unit_with_cross_unit(&caller, "caller", &environment, &[],)
            .is_err()
    );
    let result = analyze_prepared_translation_unit_with_cross_unit(
        &caller,
        "caller",
        &environment,
        &[exports[0].home.clone()],
    )
    .expect("two-sided analysis");
    let UnrestrictedLegacyResult::Safe(mut certificate) = result else {
        panic!("expected conditional Safe caller: {result:?}");
    };
    check_legacy_safety_certificate(&certificate).expect("home and caller replay");

    certificate.cross_unit_homes.clear();
    assert!(
        check_legacy_safety_certificate(&certificate)
            .unwrap_err()
            .contains("imported contracts differ")
    );
    certificate.cross_unit_homes.push(exports[0].home.clone());
    certificate.cross_unit_homes[0]
        .preprocessed_source
        .push(' ');
    assert!(check_legacy_safety_certificate(&certificate).is_err());
    certificate.cross_unit_homes[0] = exports[0].home.clone();
    certificate.cross_unit_homes[0].link_resolution_sha256 = "b".repeat(64);
    assert!(
        check_legacy_safety_certificate(&certificate)
            .unwrap_err()
            .contains("link premise differs")
    );
    certificate.cross_unit_homes[0] = exports[0].home.clone();
    certificate.cross_unit_homes[0].environment_sha256 = "b".repeat(64);
    assert!(check_legacy_safety_certificate(&certificate).is_err());
}

#[test]
fn static_and_external_inline_are_not_exported() {
    if !clang_available() {
        return;
    }
    let requested = BTreeSet::from(["callee".to_owned()]);
    for source in [
        "static void callee(void) { return; }\n",
        "inline void callee(void) { return; }\n",
        "static void callee(void); void callee(void) { return; }\n",
        "inline void callee(void); void callee(void) { return; }\n",
    ] {
        let home = prepare_translation_unit(&C11TranslationUnit {
            source,
            function_scope: LegacyFunctionScope::MainSourceDefinitions,
            ..Default::default()
        });
        let exports = export_cross_unit_homes(
            &home,
            &ResourceContracts::default(),
            &requested,
            &"a".repeat(64),
        )
        .expect("home analysis");
        assert!(exports.is_empty(), "exported {source}");
    }
}

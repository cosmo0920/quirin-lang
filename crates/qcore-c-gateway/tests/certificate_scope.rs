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

//! Card E01-TAIL-CERT-SCOPE: a Safe certificate carries a reconstructible
//! target denominator, and a caller can bind replay to an independently held
//! expected audit context.

use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use qcore_c_gateway::legacy_c::{
    C11TranslationUnit, LegacyCError, LegacyCertificateCoverage, LegacyCertificateScopeEvidence,
    LegacyFunctionScope, ResourceContracts, UnrestrictedLegacyResult, analyze_unrestricted_c11,
    check_legacy_safety_certificate, check_legacy_safety_certificate_for,
    legacy_certificate_expected_context,
};

fn clang_available() -> bool {
    match std::env::var("QCORE_CLANG") {
        Ok(path) => !path.trim().is_empty(),
        Err(_) => std::process::Command::new("clang")
            .arg("--version")
            .output()
            .is_ok_and(|output| output.status.success()),
    }
}

fn certificate(
    source: &str,
    scope: LegacyFunctionScope,
) -> Option<qcore_c_gateway::legacy_c::LegacySafetyCertificate> {
    let unit = C11TranslationUnit {
        source,
        function_scope: scope,
        ..C11TranslationUnit::default()
    };
    match analyze_unrestricted_c11(&unit, "alpha", &ResourceContracts::default()) {
        Ok(UnrestrictedLegacyResult::Safe(certificate)) => Some(certificate),
        Err(LegacyCError::ProductionParserUnavailable(_)) => None,
        other => panic!("expected Safe certificate: {other:?}"),
    }
}

#[test]
fn the_rechecker_reconstructs_the_preprocessed_denominator() {
    if !clang_available() {
        return;
    }
    let source = "void alpha(void) { return; }\nvoid beta(void) { return; }\n";
    let mut certificate = certificate(source, LegacyFunctionScope::PreprocessedTranslationUnit)
        .expect("Clang was available");
    assert_eq!(
        certificate
            .target_scope
            .definitions
            .iter()
            .map(|row| row.name.as_str())
            .collect::<Vec<_>>(),
        ["alpha", "beta"]
    );
    check_legacy_safety_certificate(&certificate).expect("complete scope rechecks");

    certificate.target_scope.definitions.pop();
    certificate.checked_functions.pop();
    certificate.summary.functions.remove("beta");
    assert!(
        check_legacy_safety_certificate(&certificate)
            .unwrap_err()
            .contains("target inventory"),
        "a coherently reduced checked set must not redefine the denominator"
    );
}

#[test]
fn duplicates_and_non_analyzed_rows_fail_closed() {
    if !clang_available() {
        return;
    }
    let source = "void alpha(void) { return; }\nvoid beta(void) { return; }\n";
    let original = certificate(source, LegacyFunctionScope::MainSourceDefinitions)
        .expect("Clang was available");

    let mut duplicate = original.clone();
    duplicate
        .target_scope
        .definitions
        .push(duplicate.target_scope.definitions[0].clone());
    assert!(check_legacy_safety_certificate(&duplicate).is_err());

    let mut unknown = original.clone();
    unknown.target_scope.definitions[0].coverage = LegacyCertificateCoverage::Unknown {
        reason: "mutation: analysis omitted".into(),
    };
    assert!(check_legacy_safety_certificate(&unknown).is_err());

    let mut excluded = original;
    excluded.target_scope.definitions[0].coverage = LegacyCertificateCoverage::Excluded {
        reason: "mutation: silently narrowed scope".into(),
    };
    assert!(check_legacy_safety_certificate(&excluded).is_err());
}

#[test]
fn scope_label_tampering_is_rejected() {
    if !clang_available() {
        return;
    }
    let mut certificate = certificate(
        "void alpha(void) { return; }\n",
        LegacyFunctionScope::MainSourceDefinitions,
    )
    .expect("Clang was available");
    certificate.function_scope = LegacyFunctionScope::PreprocessedTranslationUnit;
    assert!(
        check_legacy_safety_certificate(&certificate)
            .unwrap_err()
            .contains("scope evidence")
    );
}

#[test]
fn retained_source_evidence_and_request_mutations_are_rejected() {
    if !clang_available() {
        return;
    }
    let original = certificate(
        "void alpha(void) { return; }\n",
        LegacyFunctionScope::MainSourceDefinitions,
    )
    .expect("Clang was available");

    let mut raw_changed = original.clone();
    let LegacyCertificateScopeEvidence::MainSourceDefinitions { raw_source, .. } =
        &mut raw_changed.target_scope.evidence
    else {
        panic!("main-source certificate retained the wrong evidence kind");
    };
    raw_source.push_str("/* mutation */\n");
    assert!(
        check_legacy_safety_certificate(&raw_changed)
            .unwrap_err()
            .contains("input identity")
    );

    let mut marked_changed = original.clone();
    let LegacyCertificateScopeEvidence::MainSourceDefinitions {
        line_marked_source, ..
    } = &mut marked_changed.target_scope.evidence
    else {
        panic!("main-source certificate retained the wrong evidence kind");
    };
    line_marked_source.push_str("int injected;\n");
    assert!(
        check_legacy_safety_certificate(&marked_changed)
            .unwrap_err()
            .contains("does not match the analyzed input")
    );

    let mut request_changed = original;
    request_changed
        .target_scope
        .request
        .definitions
        .push("UNUSED_MUTATION=1".into());
    assert!(
        check_legacy_safety_certificate(&request_changed)
            .unwrap_err()
            .contains("input identity")
    );
}

#[test]
fn caller_context_rejects_a_coherently_replaced_smaller_certificate() {
    if !clang_available() {
        return;
    }
    let intended = "void alpha(void) { return; }\nvoid beta(void) { return; }\n";
    let intended_unit = C11TranslationUnit {
        source: intended,
        function_scope: LegacyFunctionScope::MainSourceDefinitions,
        ..C11TranslationUnit::default()
    };
    let expected = legacy_certificate_expected_context(&intended_unit).expect("expected context");
    let smaller = certificate(
        "void alpha(void) { return; }\n",
        LegacyFunctionScope::MainSourceDefinitions,
    )
    .expect("Clang was available");

    check_legacy_safety_certificate(&smaller).expect("smaller artifact is self-consistent");
    assert!(
        check_legacy_safety_certificate_for(&smaller, &expected)
            .unwrap_err()
            .contains("expected audit scope")
    );
}

#[test]
fn raw_compiled_out_and_knr_targets_cannot_disappear_into_a_safe_certificate() {
    if !clang_available() {
        return;
    }
    let compiled_out = concat!(
        "#if 0\nvoid hidden(void) { return; }\n#endif\n",
        "void alpha(void) { return; }\n",
    );
    let compiled_out_unit = C11TranslationUnit {
        source: compiled_out,
        function_scope: LegacyFunctionScope::MainSourceDefinitions,
        ..C11TranslationUnit::default()
    };
    // The expected context can name this intended denominator, including the
    // raw-only row. Issuance still refuses because `hidden` has no analysis.
    legacy_certificate_expected_context(&compiled_out_unit)
        .expect("raw-only target belongs to the expected denominator");
    match analyze_unrestricted_c11(&compiled_out_unit, "alpha", &ResourceContracts::default()) {
        Ok(UnrestrictedLegacyResult::Unknown { reason, .. }) => {
            assert!(reason.contains("`hidden`"), "{reason}");
        }
        other => panic!("compiled-out target must withhold Safe: {other:?}"),
    }

    let knr = "int knr(a) int a; { return a; }\nvoid alpha(void) { return; }\n";
    let knr_unit = C11TranslationUnit {
        source: knr,
        function_scope: LegacyFunctionScope::MainSourceDefinitions,
        ..C11TranslationUnit::default()
    };
    assert!(legacy_certificate_expected_context(&knr_unit).is_err());
    assert!(matches!(
        analyze_unrestricted_c11(&knr_unit, "alpha", &ResourceContracts::default()),
        Ok(UnrestrictedLegacyResult::Unknown { .. })
    ));
}

#[test]
fn header_only_change_preserves_partial_scope_label_but_changes_input_identity() {
    if !clang_available() {
        return;
    }
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let directory = std::env::temp_dir().join(format!(
        "qcore-certificate-scope-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir(&directory).expect("temporary include directory");
    let header = directory.join("scope.h");
    fs::write(&header, "enum { HEADER_VALUE = 1 };\n").expect("first header");
    let source = "#include \"scope.h\"\nvoid alpha(void) { return; }\n";
    let unit = C11TranslationUnit {
        source,
        include_paths: vec![directory.display().to_string()],
        function_scope: LegacyFunctionScope::MainSourceDefinitions,
        ..C11TranslationUnit::default()
    };
    let first_context = legacy_certificate_expected_context(&unit).expect("first context");
    let certificate = match analyze_unrestricted_c11(&unit, "alpha", &ResourceContracts::default())
    {
        Ok(UnrestrictedLegacyResult::Safe(certificate)) => certificate,
        other => panic!("expected main-source Safe certificate: {other:?}"),
    };
    assert_eq!(certificate.checked_functions, ["alpha"]);
    check_legacy_safety_certificate_for(&certificate, &first_context).expect("first input matches");

    fs::write(&header, "enum { HEADER_VALUE = 2 };\n").expect("changed header");
    let changed_context = legacy_certificate_expected_context(&unit).expect("changed context");
    assert_eq!(
        first_context.inventory_sha256, changed_context.inventory_sha256,
        "the explicit main-source partial scope still selects only alpha"
    );
    assert_ne!(first_context.input_identity, changed_context.input_identity);
    assert!(check_legacy_safety_certificate_for(&certificate, &changed_context).is_err());
    fs::remove_dir_all(&directory).expect("temporary include directory cleanup");
}

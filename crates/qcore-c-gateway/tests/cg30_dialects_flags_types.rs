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

//! Card CG30-DIALECTS-FLAGS-TYPES (gateway rule 11y): the dialect, target and
//! extension refusals on routes G and T, and the evidence identity they join.
//! Fixture contracts are the CG00 harness's synthetic `q_*` assumptions.

use qcore_c_gateway::c_frontend_identity::{
    C2Y_DIFFERENCES, C23_DIFFERENCES, CExtensionFamily, CTargetIdentity, CTargetTriple,
    dialect_refusal, extension_refusal,
};
use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, CCompilerDialect, CFrontendFlag, CPreprocessorOption,
    LegacyCertificateExpectedContext, LegacyDiagnosticKind, UnrestrictedLegacyResult,
    analyze_unrestricted_c11, check_legacy_safety_certificate, check_legacy_safety_certificate_for,
};

#[path = "support/clang_gap.rs"]
mod clang_gap;

use clang_gap::{DECLARATIONS, clang_available, contracts, t_refusal};

fn clang_supports_c2y() -> bool {
    qcore_c_gateway::clang_version().is_ok_and(|version| version.supports_c2y())
}

fn run(
    source: &str,
    dialect: CCompilerDialect,
    options: Vec<CPreprocessorOption>,
) -> UnrestrictedLegacyResult {
    let text = format!("{DECLARATIONS}\n{source}\n");
    analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &text,
            dialect,
            preprocessor_options: options,
            ..Default::default()
        },
        "subject",
        &contracts(),
    )
    .unwrap_or_else(|error| panic!("{dialect:?} rejected the fixture: {error:?}\n{text}"))
}

/// `(status, unsupported descriptions)` of `subject`, or `None` when the unit
/// projected no summary (its reason is returned instead).
fn row(outcome: &UnrestrictedLegacyResult) -> Result<(AnalysisStatus, Vec<String>), String> {
    let summary = match outcome {
        UnrestrictedLegacyResult::Safe(certificate) => {
            check_legacy_safety_certificate(certificate).expect("a Safe result rechecks");
            &certificate.summary
        }
        UnrestrictedLegacyResult::Unsafe(summary) => summary,
        UnrestrictedLegacyResult::Unknown {
            partial: Some(summary),
            ..
        } => summary,
        UnrestrictedLegacyResult::Unknown { reason, .. } => return Err(reason.clone()),
    };
    let function = &summary.functions["subject"];
    Ok((
        function.status,
        function
            .diagnostics
            .iter()
            .filter_map(|d| match &d.kind {
                LegacyDiagnosticKind::Unsupported { description } => Some(description.clone()),
                _ => None,
            })
            .collect(),
    ))
}

const PAIR: &str = "void subject(void) { void *p = q_alloc(); q_free(p); }";

#[test]
fn a_unit_without_differences_reads_alike_under_every_dialect() {
    if !clang_available() {
        return;
    }
    let mut identities = Vec::new();
    let mut dialects = vec![
        CCompilerDialect::C11,
        CCompilerDialect::Gnu17,
        CCompilerDialect::C17,
        CCompilerDialect::C23,
    ];
    if clang_supports_c2y() {
        dialects.push(CCompilerDialect::C2y);
    }
    for dialect in dialects {
        let outcome = run(PAIR, dialect, Vec::new());
        assert_eq!(
            row(&outcome),
            Ok((AnalysisStatus::Safe, vec![])),
            "{dialect:?}"
        );
        let UnrestrictedLegacyResult::Safe(certificate) = outcome else {
            unreachable!()
        };
        identities.push(
            certificate
                .target_scope
                .input_identity
                .analysis_request_sha256,
        );
    }
    // The verdict is the same; the evidence identity is not: a certificate
    // minted under one dialect never stands for another.
    let mut distinct = identities.clone();
    distinct.sort();
    distinct.dedup();
    assert_eq!(distinct.len(), identities.len(), "{identities:?}");
}

/// I05: `nullptr` is not excluded for its `CXX` node name; it is a registered
/// C23 difference, refused by name, and the withdrawn body cannot publish the
/// `Unsafe` a misread null test would.
#[test]
fn c23_nullptr_is_refused_by_name_and_never_misread() {
    if !clang_available() {
        return;
    }
    let source = "void subject(void) { void *p = q_alloc(); if (p == nullptr) return; q_free(p); }";
    let nullptr = C23_DIFFERENCES
        .iter()
        .find(|d| d.key == "c23-nullptr")
        .unwrap();
    let mut dialects = vec![CCompilerDialect::C23];
    if clang_supports_c2y() {
        dialects.push(CCompilerDialect::C2y);
    }
    for dialect in dialects {
        let (status, unsupported) = row(&run(source, dialect, Vec::new())).expect("projected");
        assert_eq!(status, AnalysisStatus::Unknown, "{dialect:?}");
        assert!(
            unsupported.contains(&dialect_refusal(dialect, nullptr)),
            "{dialect:?}: {unsupported:?}"
        );
    }
    // The same function spelled with C11's null pointer constant is Safe.
    let c11 = "void subject(void) { void *p = q_alloc(); if (p == (void *)0) return; q_free(p); }";
    assert_eq!(
        row(&run(c11, CCompilerDialect::C23, Vec::new())),
        Ok((AnalysisStatus::Safe, vec![]))
    );
}

#[test]
fn c23_keywords_and_attributes_are_refused_by_name() {
    if !clang_available() {
        return;
    }
    let version = qcore_c_gateway::clang_version().expect("Clang version is detected");
    for (source, key) in [
        ("int subject(void) { return true; }", "c23-true"),
        ("int subject(void) { bool b = 0; return b; }", "c23-bool"),
        (
            "int subject(void) { constexpr int k = 1; return k; }",
            "c23-constexpr",
        ),
        (
            "[[nodiscard]] int subject(void) { return 0; }",
            "c23-attribute",
        ),
        (
            "int subject(void) { typeof(1) k = 1; return k; }",
            "c23-typeof",
        ),
    ]
    .into_iter()
    .filter(|(_, key)| *key != "c23-constexpr" || version.supports_c23_constexpr())
    {
        let difference = C23_DIFFERENCES.iter().find(|d| d.key == key).unwrap();
        let (status, unsupported) =
            row(&run(source, CCompilerDialect::C23, Vec::new())).expect("projected");
        assert_eq!(status, AnalysisStatus::Unknown, "{source}");
        assert!(
            unsupported.contains(&dialect_refusal(CCompilerDialect::C23, difference)),
            "{source}: {unsupported:?}"
        );
    }
}

#[test]
fn c2y_countof_is_refused_by_name() {
    if !clang_available() {
        return;
    }
    if !clang_supports_c2y() {
        eprintln!("selected Clang predates the C2y frontend mode; C23 coverage remains active");
        return;
    }
    let difference = C2Y_DIFFERENCES
        .iter()
        .find(|d| d.key == "c2y-countof")
        .unwrap();
    let (status, unsupported) = row(&run(
        "unsigned long subject(void) { int a[3]; return _Countof(a); }",
        CCompilerDialect::C2y,
        Vec::new(),
    ))
    .expect("projected");
    assert_eq!(status, AnalysisStatus::Unknown);
    assert!(unsupported.contains(&dialect_refusal(CCompilerDialect::C2y, difference)));
}

/// The token reader would lex `1'000`'s `'` as a character literal and lose
/// tokens up to the next `'`; the unit is refused whole.
#[test]
fn a_digit_separator_refuses_the_unit() {
    if !clang_available() {
        return;
    }
    let reason = row(&run(
        "int subject(void) { void *p = q_alloc(); int n = 1'000; q_free(p); return n; }",
        CCompilerDialect::C23,
        Vec::new(),
    ))
    .expect_err("the unit is refused");
    assert!(reason.contains("c23-digit-separator"), "{reason}");
}

/// N14 (and the I03/N12/N13 extension families): refused by name on G and T.
#[test]
fn extension_types_are_refused_by_name_on_g_and_t() {
    if !clang_available() {
        return;
    }
    for (source, family) in [
        (
            "void subject(void) { _BitInt(7) x = 3; (void)x; }",
            CExtensionFamily::BitPrecise,
        ),
        (
            "void subject(void) { double _Complex z = 1.0; (void)z; }",
            CExtensionFamily::Complex,
        ),
        (
            "typedef int v4 __attribute__((vector_size(16)));\n\
             void subject(v4 a) { v4 c = a + a; (void)c; }",
            CExtensionFamily::Vector,
        ),
    ] {
        let (status, unsupported) =
            row(&run(source, CCompilerDialect::C11, Vec::new())).expect("projected");
        assert_ne!(status, AnalysisStatus::Safe, "{source}");
        assert!(
            unsupported
                .iter()
                .any(|d| d.starts_with("C extension type `") && d.contains(family.name())),
            "{source}: {unsupported:?}"
        );
    }
    assert_eq!(
        t_refusal("int subject(void) { _BitInt(7) x = 0; return 0; }"),
        extension_refusal(CExtensionFamily::BitPrecise, "_BitInt(7)")
    );
    assert_eq!(
        t_refusal("int subject(void) { double _Complex z = 0; return 0; }"),
        extension_refusal(CExtensionFamily::Complex, "_Complex double")
    );
}

/// A sibling's extension type does not touch a definition that uses none.
#[test]
fn an_extension_elsewhere_in_the_unit_leaves_a_definition_alone() {
    if !clang_available() {
        return;
    }
    let source = format!(
        "typedef int v4 __attribute__((vector_size(16)));\n\
         int other(v4 a) {{ return a[0]; }}\n{PAIR}"
    );
    assert_eq!(
        row(&run(&source, CCompilerDialect::C11, Vec::new())),
        Ok((AnalysisStatus::Safe, vec![]))
    );
}

/// I02: a structured target joins the request digest, so evidence minted for
/// one target is refused as the evidence of another.
#[test]
fn a_target_changes_the_evidence_identity() {
    if !clang_available() {
        return;
    }
    let certificate =
        |options: Vec<CPreprocessorOption>| match run(PAIR, CCompilerDialect::Gnu17, options) {
            UnrestrictedLegacyResult::Safe(certificate) => certificate,
            other => panic!("{other:?}"),
        };
    let target = |triple: &str| {
        vec![CPreprocessorOption::FrontendFlag(CFrontendFlag::Target(
            CTargetTriple::parse(triple).unwrap(),
        ))]
    };
    let default = certificate(Vec::new());
    let x86 = certificate(target("x86_64-pc-linux-gnu"));
    let arm = certificate(target("aarch64-unknown-linux-gnu"));
    let digests = [&default, &x86, &arm].map(|c| {
        c.target_scope
            .input_identity
            .analysis_request_sha256
            .clone()
    });
    assert_ne!(digests[0], digests[1]);
    assert_ne!(digests[1], digests[2]);
    assert_ne!(digests[0], digests[2]);
    let expected =
        |c: &qcore_c_gateway::legacy_c::LegacySafetyCertificate| LegacyCertificateExpectedContext {
            inventory_sha256: c.target_scope.inventory_sha256.clone(),
            input_identity: c.target_scope.input_identity.clone(),
        };
    check_legacy_safety_certificate_for(&x86, &expected(&x86)).expect("own context");
    assert!(check_legacy_safety_certificate_for(&arm, &expected(&x86)).is_err());
    assert!(check_legacy_safety_certificate_for(&default, &expected(&x86)).is_err());
}

/// A unit whose translation-unit JSON nests beyond `serde_json`'s recursion
/// limit (librdkafka's `rdhttp.c` nests 223 deep) has no unit index; each
/// definition is then read from its own filtered AST, so a definition with no
/// extension keeps its verdict and one with an extension is refused by name.
#[test]
fn a_unit_too_deep_to_index_is_read_per_definition() {
    if !clang_available() {
        return;
    }
    let deep = format!(
        "int deep_global = {}1{};\n",
        "(".repeat(150),
        ")".repeat(150)
    );
    assert_eq!(
        row(&run(
            &format!("{deep}{PAIR}"),
            CCompilerDialect::C11,
            Vec::new()
        )),
        Ok((AnalysisStatus::Safe, vec![]))
    );
    let (status, unsupported) = row(&run(
        &format!("{deep}void subject(void) {{ _BitInt(7) x = 3; (void)x; }}"),
        CCompilerDialect::C11,
        Vec::new(),
    ))
    .expect("projected");
    assert_eq!(status, AnalysisStatus::Unknown);
    assert!(
        unsupported.contains(&extension_refusal(
            CExtensionFamily::BitPrecise,
            "_BitInt(7)"
        )),
        "{unsupported:?}"
    );
}

/// CG20-ABI-READER-ARCH-FLAG: a unit whose own selections fix a non-LP64
/// data model is refused by the gateway itself, whoever built it, so the
/// lexical ABI reader's default-target `long` question is never asked for
/// another data model. An LP64 `-arch` is read as before.
#[test]
fn a_unit_selecting_a_non_lp64_target_is_refused_by_the_gateway() {
    if !clang_available() {
        return;
    }
    let flag = |flag: CFrontendFlag| vec![CPreprocessorOption::FrontendFlag(flag)];
    let text = format!("{DECLARATIONS}\n{PAIR}\n");
    for options in [
        flag(CFrontendFlag::Architecture("i386".into())),
        flag(CFrontendFlag::Target(
            CTargetTriple::parse("i686-pc-linux-gnu").unwrap(),
        )),
        flag(CFrontendFlag::Target(
            CTargetTriple::parse("x86_64-pc-windows-msvc").unwrap(),
        )),
    ] {
        let error = analyze_unrestricted_c11(
            &C11TranslationUnit {
                source: &text,
                dialect: CCompilerDialect::Gnu17,
                preprocessor_options: options.clone(),
                ..Default::default()
            },
            "subject",
            &contracts(),
        );
        let reason = match error {
            Ok(UnrestrictedLegacyResult::Unknown { reason, .. }) => reason,
            Err(error) => format!("{error:?}"),
            Ok(other) => panic!("{options:?} was decided: {other:?}"),
        };
        // A Linux Clang refuses `-arch` itself before any reader; every other
        // selection reaches the gateway's own admission.
        assert!(
            reason.contains("not the readers' LP64 premise")
                || reason.contains("unsupported option '-arch'"),
            "{reason}"
        );
    }
    let arch =
        |name: &str| CTargetIdentity::admit_flags([&CFrontendFlag::Architecture(name.into())]);
    assert!(arch("i386").is_err());
    assert!(arch("arm64_32").is_err());
    assert!(arch("arm64").is_ok());
    let mixed = [
        CFrontendFlag::Architecture("arm64".into()),
        CFrontendFlag::Architecture("i386".into()),
    ];
    assert!(CTargetIdentity::admit_flags(mixed.iter()).is_err());
}

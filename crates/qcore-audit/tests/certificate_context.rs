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

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use qcore_audit::certificate_context::{
    AuditContractTrustPolicy, bind_audit_legacy_safety_certificate,
    check_audit_legacy_safety_certificate_for, check_audit_legacy_safety_certificate_in_context,
    expected_audit_certificate_context,
};
use qcore_audit::compile_db::{
    CompileEntry, CompileTaskIdentity, EffectiveInput, compile_task_identity, compiler_options,
};
use qcore_audit::projection::{
    AuditCounts, PreparedAuditCertificateContext, project_prepared_unit,
};
use qcore_audit::report::AuditReport;
use qcore_c_gateway::ClangIdentity;
use qcore_c_gateway::legacy_c::{
    C11TranslationUnit, LegacyFunctionScope, UnrestrictedLegacyResult,
    analyze_prepared_translation_unit, prepare_translation_unit,
};
use qcore_contract::contract_loader::load_resource_contract_sources;
use qcore_schemas::contract::ResourceContracts;

static NEXT: AtomicUsize = AtomicUsize::new(0);
const SOURCE: &str = "void alpha(void) { return; }\nvoid beta(void) { return; }\n";
const CONTRACT: &str = r#"module contract;
extern fn q_alloc() -> ptr<mut, u8>
contract {
    requires resource_contract_v1;
    ensures normal resource_return_owned(nonnull);
};
"#;

struct Fixture {
    root: PathBuf,
    compiler: ClangIdentity,
}

impl Fixture {
    fn new() -> Option<Self> {
        let compiler = match ClangIdentity::capture() {
            Ok(identity) => identity,
            Err(reason) => {
                assert!(
                    std::env::var_os("QCORE_CLANG").is_none(),
                    "explicit QCORE_CLANG is invalid: {reason}"
                );
                return None;
            }
        };
        let root = std::env::temp_dir().join(format!(
            "qcore-certificate-context-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("src/unit.c"), SOURCE).unwrap();
        Some(Self { root, compiler })
    }

    fn contracts(&self, source: &str) -> ResourceContracts {
        load_resource_contract_sources(&BTreeMap::from([(
            "fixture.qcore".to_owned(),
            source.to_owned(),
        )]))
        .unwrap()
    }

    fn entry(&self, extra: &[&str]) -> CompileEntry {
        let mut args = vec![
            format!("\"{}\"", self.compiler.program()),
            "-std=c11".to_owned(),
        ];
        args.extend(extra.iter().map(|arg| (*arg).to_owned()));
        args.extend(["-c".to_owned(), "src/unit.c".to_owned()]);
        CompileEntry {
            directory: self.root.clone(),
            command: args.join(" "),
            file: self.root.join("src/unit.c"),
        }
    }

    fn unit(&self) -> C11TranslationUnit<'static> {
        let options = compiler_options(&self.entry(&[])).unwrap();
        C11TranslationUnit {
            source: SOURCE,
            include_paths: options.include_paths,
            definitions: options.definitions,
            preprocessor_options: options.preprocessor_options,
            dialect: options.dialect,
            function_scope: LegacyFunctionScope::MainSourceDefinitions,
            input: options.input,
        }
    }

    fn identity(
        &self,
        entry: &CompileEntry,
        unit: &C11TranslationUnit<'_>,
        prepared: &qcore_c_gateway::legacy_c::PreparedTranslationUnit,
        dependencies: BTreeMap<String, String>,
    ) -> CompileTaskIdentity {
        self.identity_with_compiler(entry, unit, prepared, dependencies, &self.compiler)
    }

    fn identity_with_compiler(
        &self,
        entry: &CompileEntry,
        unit: &C11TranslationUnit<'_>,
        prepared: &qcore_c_gateway::legacy_c::PreparedTranslationUnit,
        dependencies: BTreeMap<String, String>,
        compiler: &ClangIdentity,
    ) -> CompileTaskIdentity {
        compile_task_identity(
            entry,
            unit,
            EffectiveInput::Accepted(prepared.preprocessed_source().unwrap()),
            compiler,
            dependencies,
        )
        .unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn digest(label: &str) -> String {
    qcore_schemas::sha256_hex(label.as_bytes())
}

fn dependencies(contract: &str, profile: &str) -> BTreeMap<String, String> {
    BTreeMap::from([
        ("compile_database".into(), digest("database-v1")),
        ("resource_contract_manifest".into(), contract.into()),
        ("source_profile".into(), profile.into()),
    ])
}

#[test]
fn audit_acceptance_binds_compile_contract_profile_and_trust_context() {
    let Some(fixture) = Fixture::new() else {
        return;
    };
    let contracts = fixture.contracts(CONTRACT);
    let profile = digest("profile-v1");
    let contract_manifest = digest("contract-manifest-v1");
    let unit = fixture.unit();
    let prepared = prepare_translation_unit(&unit);
    let identity = fixture.identity(
        &fixture.entry(&[]),
        &unit,
        &prepared,
        dependencies(&contract_manifest, &profile),
    );
    let certificate = match analyze_prepared_translation_unit(&prepared, "alpha", &contracts)
        .expect("analysis succeeds")
    {
        UnrestrictedLegacyResult::Safe(certificate) => certificate,
        other => panic!("expected Safe: {other:?}"),
    };
    let policy = AuditContractTrustPolicy::DeclaredSourcesAreTrustedAssumptions;
    let bound =
        bind_audit_legacy_safety_certificate(&certificate, identity.clone(), &profile, policy)
            .unwrap();
    let expected = expected_audit_certificate_context(
        &prepared,
        identity.clone(),
        &contracts,
        &profile,
        policy,
    )
    .unwrap();
    check_audit_legacy_safety_certificate_for(&bound, &expected).unwrap();

    // Compiler program/version/target provenance is part of the full task,
    // even when every gateway artifact and request field remains unchanged.
    let changed_compiler = ClangIdentity::recorded(
        fixture.compiler.program(),
        format!("{} (different build)", fixture.compiler.version()),
        format!("{}-different", fixture.compiler.target()),
    )
    .unwrap();
    let compiler_identity = fixture.identity_with_compiler(
        &fixture.entry(&[]),
        &unit,
        &prepared,
        dependencies(&contract_manifest, &profile),
        &changed_compiler,
    );
    let compiler_expected = expected_audit_certificate_context(
        &prepared,
        compiler_identity,
        &contracts,
        &profile,
        policy,
    )
    .unwrap();
    assert!(
        check_audit_legacy_safety_certificate_for(&bound, &compiler_expected)
            .unwrap_err()
            .contains("compile-task provenance")
    );

    // A build-command change that does not affect the analyzed bytes still
    // changes the task identity.
    let build_entry = fixture.entry(&["-o", "unit-alternate.o"]);
    let build_identity = fixture.identity(
        &build_entry,
        &unit,
        &prepared,
        dependencies(&contract_manifest, &profile),
    );
    let build_expected =
        expected_audit_certificate_context(&prepared, build_identity, &contracts, &profile, policy)
            .unwrap();
    assert!(check_audit_legacy_safety_certificate_for(&bound, &build_expected).is_err());

    let dependency_identity = fixture.identity(
        &fixture.entry(&[]),
        &unit,
        &prepared,
        dependencies(&digest("contract-manifest-v2"), &profile),
    );
    let dependency_expected = expected_audit_certificate_context(
        &prepared,
        dependency_identity,
        &contracts,
        &profile,
        policy,
    )
    .unwrap();
    assert!(check_audit_legacy_safety_certificate_for(&bound, &dependency_expected).is_err());

    let changed_contracts = fixture.contracts(&format!("// changed trusted text\n{CONTRACT}"));
    let contract_expected = expected_audit_certificate_context(
        &prepared,
        identity.clone(),
        &changed_contracts,
        &profile,
        policy,
    )
    .unwrap();
    assert!(check_audit_legacy_safety_certificate_for(&bound, &contract_expected).is_err());

    let changed_profile = digest("profile-v2");
    let changed_profile_identity = fixture.identity(
        &fixture.entry(&[]),
        &unit,
        &prepared,
        dependencies(&contract_manifest, &changed_profile),
    );
    let profile_expected = expected_audit_certificate_context(
        &prepared,
        changed_profile_identity,
        &contracts,
        &changed_profile,
        policy,
    )
    .unwrap();
    assert!(check_audit_legacy_safety_certificate_for(&bound, &profile_expected).is_err());

    // Both routes use the same gate and exact refusal ordering. The borrowed
    // route still independently reanalyzes the bodies under every context.
    for context in [
        &expected,
        &compiler_expected,
        &build_expected,
        &dependency_expected,
        &contract_expected,
        &profile_expected,
    ] {
        assert_eq!(
            check_audit_legacy_safety_certificate_in_context(
                &certificate,
                &identity,
                &profile,
                policy,
                context,
            ),
            check_audit_legacy_safety_certificate_for(&bound, context),
        );
    }
    for mutation in ["source", "body", "inventory"] {
        let mut stale = certificate.clone();
        match mutation {
            "source" => stale.preprocessed_source.push('\n'),
            "body" => stale.summary.functions.get_mut("alpha").unwrap().cfg.nodes[0].line += 1,
            "inventory" => stale.target_scope.inventory_sha256 = digest("stale inventory"),
            _ => unreachable!(),
        }
        let owned =
            bind_audit_legacy_safety_certificate(&stale, identity.clone(), &profile, policy)
                .and_then(|presented| {
                    check_audit_legacy_safety_certificate_for(&presented, &expected)
                });
        let borrowed = check_audit_legacy_safety_certificate_in_context(
            &stale, &identity, &profile, policy, &expected,
        );
        assert!(borrowed.is_err(), "{mutation}");
        assert_eq!(borrowed, owned, "{mutation}");
    }

    // Replacing the certificate with another fully structural certificate
    // while retaining the stale wrapper trust identity must be detected at
    // admission, rather than relying on the binding constructor having run.
    let alternate_contracts = fixture.contracts(&format!(
        "{CONTRACT}\nextern fn q_free(p: ptr<mut, u8>) -> unit\ncontract {{\n    requires resource_contract_v1;\n    ensures normal resource_released(p, nonnull);\n    ensures normal resource_return_none();\n}};\n"
    ));
    let alternate_certificate = match analyze_prepared_translation_unit(
        &prepared,
        "alpha",
        &alternate_contracts,
    )
    .unwrap()
    {
        UnrestrictedLegacyResult::Safe(certificate) => certificate,
        other => panic!("expected alternate Safe: {other:?}"),
    };
    let mut stale_wrapper = bound.clone();
    stale_wrapper.certificate = alternate_certificate;
    assert!(
        check_audit_legacy_safety_certificate_for(&stale_wrapper, &expected)
            .unwrap_err()
            .contains("certificate contracts differ")
    );
}

#[test]
fn coherent_substitution_and_missing_provenance_fail_closed() {
    let Some(fixture) = Fixture::new() else {
        return;
    };
    let contracts = fixture.contracts(CONTRACT);
    let profile = digest("profile-v1");
    let policy = AuditContractTrustPolicy::DeclaredSourcesAreTrustedAssumptions;
    let original_unit = fixture.unit();
    let original_prepared = prepare_translation_unit(&original_unit);
    let original_identity = fixture.identity(
        &fixture.entry(&[]),
        &original_unit,
        &original_prepared,
        dependencies(&digest("contract-v1"), &profile),
    );
    let expected = expected_audit_certificate_context(
        &original_prepared,
        original_identity,
        &contracts,
        &profile,
        policy,
    )
    .unwrap();

    let substituted_unit = C11TranslationUnit {
        source: "void alpha(void) { return; }\n",
        ..fixture.unit()
    };
    fs::write(fixture.root.join("src/unit.c"), substituted_unit.source).unwrap();
    let substituted_prepared = prepare_translation_unit(&substituted_unit);
    let substituted_identity = fixture.identity(
        &fixture.entry(&[]),
        &substituted_unit,
        &substituted_prepared,
        dependencies(&digest("contract-v1"), &profile),
    );
    let substituted_certificate = match analyze_prepared_translation_unit(
        &substituted_prepared,
        "alpha",
        &contracts,
    )
    .unwrap()
    {
        UnrestrictedLegacyResult::Safe(certificate) => certificate,
        other => panic!("expected substituted Safe: {other:?}"),
    };
    let substituted_bound = bind_audit_legacy_safety_certificate(
        &substituted_certificate,
        substituted_identity,
        &profile,
        policy,
    )
    .unwrap();
    assert!(check_audit_legacy_safety_certificate_for(&substituted_bound, &expected).is_err());

    let missing_dependencies = fixture.identity(
        &fixture.entry(&[]),
        &substituted_unit,
        &substituted_prepared,
        BTreeMap::new(),
    );
    assert!(
        expected_audit_certificate_context(
            &substituted_prepared,
            missing_dependencies,
            &contracts,
            &profile,
            policy,
        )
        .unwrap_err()
        .contains("resource_contract_manifest")
    );
}

#[test]
fn configured_projection_retains_a_refused_context_as_unknown() {
    let Some(fixture) = Fixture::new() else {
        return;
    };
    let contracts = fixture.contracts(CONTRACT);
    let recorded_profile = digest("profile-v1");
    let mismatched_profile = digest("profile-v2");
    let unit = fixture.unit();
    let prepared = prepare_translation_unit(&unit);
    let identity = fixture.identity(
        &fixture.entry(&[]),
        &unit,
        &prepared,
        dependencies(&digest("contract-v1"), &recorded_profile),
    );
    let mut report = AuditReport::new("fixture-revision");
    let mut counts = AuditCounts::default();
    project_prepared_unit(
        "fixture",
        "src/unit.c",
        &prepared,
        &contracts,
        PreparedAuditCertificateContext {
            compile_task_identity: &identity,
            source_profile_sha256: &mismatched_profile,
        },
        &mut report,
        &mut counts,
    )
    .expect("a context refusal is a recorded Unknown, not a run abort");
    assert_eq!(counts.units, 1);
    assert_eq!(counts.safe_units, 0);
    assert_eq!(counts.unknown_units, 1);
    assert_eq!(counts.source_functions, 2);
    let report_path = fixture.root.join("context-refusal.json");
    report.write(&report_path).unwrap();
    let report: serde_json::Value =
        serde_json::from_slice(&fs::read(report_path).unwrap()).unwrap();
    let unit = &report["units"][0];
    assert_eq!(unit["status"], "Unknown");
    assert_eq!(unit["source_functions"].as_array().unwrap().len(), 2);
    assert_eq!(unit["not_projected"].as_array().unwrap().len(), 0);
    let functions = unit["functions"].as_array().unwrap();
    assert_eq!(functions.len(), 2);
    assert!(functions.iter().all(|row| row["status"] == "Unknown"));
}

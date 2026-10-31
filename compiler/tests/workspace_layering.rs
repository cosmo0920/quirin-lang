//! Layering guard from `docs/GENERALIZATION_PLAN.md` W0 task 4 and
//! invariant 7: workspace dependencies point strictly downward, and the
//! published schema surface depends on no workspace crate.

use std::path::PathBuf;

#[path = "../../tests/support/tracked_files.rs"]
mod tracked_files;

/// Workspace crates and the workspace dependencies each is allowed
/// (`docs/GENERALIZATION_PLAN.md` §3.1; the grab-bag `qcore-compiler` and
/// the CLI sit above every extracted layer).
const ALLOWED: &[(&str, &[&str])] = &[
    ("crates/qcore-schemas", &[]),
    ("crates/qcore-core", &[]),
    // Formal contract ASTs are portable L0 data; frontend parsing points
    // downward to the schema leaf, which still has no workspace dependencies.
    ("crates/qcore-frontend", &["qcore-core", "qcore-schemas"]),
    (
        "crates/qcore-contract",
        &["qcore-core", "qcore-frontend", "qcore-schemas"],
    ),
    // Card CG13-UNION-AND-BITFIELD-PLACES (rule 10w) adds `qcore-core`
    // deliberately: the gateway asks the kernel's `disjoint_footprint` model
    // about bit-field storage units instead of re-deriving it. The edge points
    // down (L2 -> L1), and the gateway already depended on the kernel through
    // `qcore-contract`.
    // Card GATEWAY-RULE-API (pluggable-cards seam B): the rule-package
    // interface sits between the schema surface and the gateway; a rule
    // package depends on it, never on the gateway or another rule package.
    (
        "crates/qcore-gateway-rule-api",
        &["qcore-schemas", "qcore-contract"],
    ),
    // Card RULE35-PILOT: the first rule package. A package under
    // `crates/gateway-rules/` depends on the rule API only (and `qcore-core`
    // where it reads a kernel model); `rule_packages_depend_on_the_api_only`
    // below holds that for every package, present and future.
    (
        "crates/gateway-rules/rule-35-literal-decay",
        &["qcore-gateway-rule-api"],
    ),
    // Card RULE37-PILOT: a rule package depends on the rule interface only,
    // and the gateway asks it through that interface.
    (
        "crates/gateway-rules/rule-37-result-constructors",
        &["qcore-gateway-rule-api"],
    ),
    // Card RULE36-PILOT: rule 36's package. A package under
    // `crates/gateway-rules/` depends on the rule API only (plan seam B),
    // never on the gateway or another rule package.
    (
        "crates/gateway-rules/rule-36-exact-formal-return",
        &["qcore-gateway-rule-api"],
    ),
    // Card RULE33-PILOT: a rule package depends on the rule interface only,
    // and the gateway asks it through that interface.
    (
        "crates/gateway-rules/rule-33-removed-selection",
        &["qcore-gateway-rule-api"],
    ),
    // Card RULE34-PILOT: a rule package depends on the rule interface only,
    // and the gateway asks it through that interface.
    (
        "crates/gateway-rules/rule-34-element-destroyer",
        &["qcore-gateway-rule-api"],
    ),
    // Card RULE32A-PILOT: a rule package depends on the rule interface only,
    // and the gateway asks it through that interface.
    (
        "crates/gateway-rules/rule-32a-caller-frame-hold",
        &["qcore-gateway-rule-api"],
    ),
    (
        "crates/qcore-c-gateway",
        &[
            "qcore-contract",
            "qcore-core",
            "qcore-gateway-rule-api",
            "qcore-schemas",
            // Card RULE35-PILOT: the gateway hosts rule 35's package.
            "rule-35-literal-decay",
            "rule-37-result-constructors",
            // Card RULE36-PILOT: the gateway hosts rule 36's package.
            "rule-36-exact-formal-return",
            "rule-33-removed-selection",
            "rule-34-element-destroyer",
            // Card RULE32A-PILOT: the gateway hosts rule 32a's hold.
            "rule-32a-caller-frame-hold",
        ],
    ),
    (
        "crates/qcore-verifier-host",
        &["qcore-frontend", "qcore-schemas"],
    ),
    (
        "crates/qcore-verification",
        &[
            "qcore-c-gateway",
            "qcore-core",
            "qcore-frontend",
            "qcore-contract",
            "qcore-schemas",
            "qcore-verifier-host",
        ],
    ),
    (
        "crates/quirin-source-analysis",
        &[
            "qcore-c-gateway",
            "qcore-contract",
            "qcore-core",
            "qcore-frontend",
            "qcore-schemas",
            "qcore-verification",
        ],
    ),
    (
        "compiler",
        &[
            "quirin-source-analysis",
            "qcore-audit", // examples-only dev dependency, not the compiler library
            "qcore-c-gateway",
            "qcore-contract",
            "qcore-core",
            "qcore-frontend",
            "qcore-schemas",
            "qcore-verifier-host",
            "qcore-verification",
        ],
    ),
    (
        "crates/qcore-audit",
        &[
            "quirin-source-analysis",
            "qcore-c-gateway",
            "qcore-contract",
            "qcore-schemas",
            "qcore-verification",
            "qcore-verifier-host",
            // Card W45-AUDIT-SUBCOMMAND-PLUGINS: the external-subcommand
            // protocol, a leaf SDK above the schema crate only.
            "quirin-audit-plugin",
        ],
    ),
    // The subcommand-plugin SDK links no core analysis crate (SHA-256 from the
    // schema crate), and its first plugin only the SDK.
    ("crates/quirin-audit-plugin", &["qcore-schemas"]),
    (
        "crates/quirin-audit-plugin-libevent",
        &["quirin-audit-plugin", "qcore-schemas"],
    ),
    // Card MEASURE-TOOL: the measurement plugin reads components through the
    // schema crate's composition interface and links no analysis crate.
    (
        "crates/quirin-audit-plugin-measure",
        &["quirin-audit-plugin", "qcore-schemas"],
    ),
    ("crates/qcorec", &["qcore-compiler", "qcore-contract"]),
];

fn workspace_dependencies(manifest: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut in_dependency_table = false;
    for line in manifest.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_dependency_table = line
                .trim_matches(['[', ']'])
                .split('.')
                .next_back()
                .is_some_and(|table| table.ends_with("dependencies"));
            continue;
        }
        if !in_dependency_table || line.is_empty() || line.starts_with('#') {
            continue;
        }
        let name = line.split(['=', ' ']).next().unwrap_or_default();
        if line.contains("path =") || line.contains("path=") || name.contains("qcore") {
            found.push(name.to_owned());
        }
    }
    found
}

#[test]
fn workspace_dependencies_point_strictly_downward() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    for (crate_path, allowed) in ALLOWED {
        let manifest_path = root.join(crate_path).join("Cargo.toml");
        let manifest = std::fs::read_to_string(&manifest_path)
            .unwrap_or_else(|error| panic!("{}: {error}", manifest_path.display()));
        for dependency in workspace_dependencies(&manifest) {
            assert!(
                allowed.contains(&dependency.as_str()),
                "{crate_path} must not depend on `{dependency}` \
                 (allowed workspace dependencies: {allowed:?})"
            );
        }
    }
}

/// Plan `docs/plans/pluggable-cards/README.md` seam B (card RULE35-PILOT):
/// every rule package under `crates/gateway-rules/` is a workspace member and
/// depends on the rule API (and `qcore-core` where it reads a kernel model)
/// only: never on `qcore-c-gateway`, never on another rule package.
#[test]
fn rule_packages_depend_on_the_api_only() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let workspace = std::fs::read_to_string(root.join("Cargo.toml")).unwrap();
    let base = root.join("crates/gateway-rules");
    let packages: Vec<PathBuf> = tracked_files::tracked_files(&base)
        .into_iter()
        .filter(|path| {
            path.file_name().is_some_and(|name| name == "Cargo.toml")
                && path.parent().and_then(|dir| dir.parent()) == Some(base.as_path())
        })
        .map(|manifest| manifest.parent().unwrap().to_path_buf())
        .collect();
    assert!(!packages.is_empty(), "rule 35's package exists");
    for path in packages {
        let name = path.file_name().unwrap().to_str().unwrap().to_owned();
        let member = format!("\"crates/gateway-rules/{name}\",");
        assert!(
            workspace.contains(&member),
            "{name} is not a workspace member"
        );
        let manifest = std::fs::read_to_string(path.join("Cargo.toml")).unwrap();
        let dependencies = workspace_dependencies(&manifest);
        assert!(
            dependencies.contains(&"qcore-gateway-rule-api".to_owned()),
            "{name} must implement the rule API"
        );
        for dependency in dependencies {
            assert!(
                ["qcore-gateway-rule-api", "qcore-core"].contains(&dependency.as_str()),
                "rule package {name} must not depend on `{dependency}`"
            );
        }
    }
}

/// W5/W0 provider boundary: the core workspace must build and test without any
/// provider implementation. `providers/` is a separate workspace, and no core
/// member may name a provider crate or reach into that directory.
#[test]
fn core_workspace_does_not_depend_on_provider_implementations() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let workspace = std::fs::read_to_string(root.join("Cargo.toml")).unwrap();
    assert!(
        workspace.contains("exclude = [\"providers\"]"),
        "the core workspace must exclude `providers`"
    );
    let members: Vec<String> = workspace
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with('"') && line.ends_with("\","))
        .map(|line| line.trim_matches(['"', ',']).to_owned())
        .collect();
    assert!(
        members.iter().all(|member| !member.contains("providers")),
        "no core workspace member may live under `providers/`: {members:?}"
    );
    // Every declared member manifest, plus the CLI and compiler grab-bag, is
    // checked; ALLOWED above pins their permitted workspace dependencies.
    assert!(members.len() >= ALLOWED.len(), "{members:?}");
    for member in &members {
        let manifest_path = root.join(member).join("Cargo.toml");
        let manifest = std::fs::read_to_string(&manifest_path)
            .unwrap_or_else(|error| panic!("{}: {error}", manifest_path.display()));
        let mut in_dependency_table = false;
        for line in manifest.lines().map(str::trim) {
            if line.starts_with('[') {
                in_dependency_table = line
                    .trim_matches(['[', ']'])
                    .split('.')
                    .next_back()
                    .is_some_and(|table| table.ends_with("dependencies"));
                continue;
            }
            if !in_dependency_table || line.is_empty() || line.starts_with('#') {
                continue;
            }
            assert!(
                !line.starts_with("qcore-provider") && !line.starts_with("quirin-provider"),
                "{member} must not depend on a provider crate: {line}"
            );
            assert!(
                !line.contains("providers/"),
                "{member} must not reach into the provider workspace: {line}"
            );
        }
    }
}

#[test]
fn compiler_uses_audit_only_for_examples() {
    let manifest = include_str!("../Cargo.toml");
    let mut section = "";
    for line in manifest.lines().map(str::trim) {
        if line.starts_with('[') {
            section = line;
        }
        if line.starts_with("qcore-audit =") {
            assert_eq!(section, "[dev-dependencies]");
        }
    }
}

#[test]
fn sequential_kernel_compatibility_paths_are_core_types() {
    // Compiler APIs must remain the same types as their provider-free owners,
    // rather than a duplicate implementation drifting behind a facade.
    use std::any::TypeId;
    macro_rules! same_type {
        ($module:ident, $name:ident) => {
            assert_eq!(
                TypeId::of::<qcore_compiler::$module::$name>(),
                TypeId::of::<qcore_core::$module::$name>()
            );
        };
    }
    same_type!(ownership, OwnershipState);
    same_type!(resource_flow, Flow);
    same_type!(typed_errors, ErrorLedger);
    same_type!(error_region, CleanupPack);
    same_type!(cleanup_lowering, SourceProgram);
    same_type!(cfg, TypedCfg);
    same_type!(c_emit, CEmitError);
    same_type!(executable_model, ModelProgram);
    same_type!(advanced_control, AdvancedProgram);
    same_type!(inference, InferenceCertificate);
}

#[test]
fn concurrent_kernel_compatibility_paths_are_core_types() {
    use std::any::TypeId;
    macro_rules! same_type {
        ($module:ident, $name:ident) => {
            assert_eq!(
                TypeId::of::<qcore_compiler::$module::$name>(),
                TypeId::of::<qcore_core::$module::$name>()
            );
        };
    }
    same_type!(concurrency, ConcurrencyCertificate);
    same_type!(weak_memory, C11ExecutionCertificate);
    same_type!(native_concurrency, NativeExecutionCertificate);
    same_type!(translation_validation, TranslationCertificate);
    same_type!(research_closure, ResearchClosureCertificate);
}

#[test]
fn whole_program_qualified_apis_are_frontend_owned() {
    use std::any::TypeId;
    assert_eq!(
        TypeId::of::<qcore_compiler::whole_program::CheckedProgram>(),
        TypeId::of::<qcore_frontend::whole_program::CheckedProgram>()
    );
    assert_eq!(
        TypeId::of::<qcore_compiler::WholeFunctionSummary>(),
        TypeId::of::<qcore_frontend::whole_program::FunctionSummary>()
    );
    type Checker = fn(
        &qcore_frontend::frontend::TypedSourceProgram,
        &qcore_core::typed_errors::ErrorDeclarations,
        Vec<qcore_frontend::whole_program::FunctionSemantics>,
    ) -> Result<
        qcore_frontend::whole_program::CheckedProgram,
        Vec<qcore_frontend::frontend::SourceDiagnostic>,
    >;
    let _: Checker = qcore_compiler::whole_program::check_whole_program;
    let _: Checker = qcore_compiler::check_whole_program;
    let _: Checker = qcore_frontend::whole_program::check_whole_program;
    let _: Checker = qcore_frontend::check_whole_program;
}

#[test]
fn loop_and_ledger_certificates_are_core_owned() {
    use std::any::TypeId;
    assert_eq!(
        TypeId::of::<qcore_compiler::research_extensions::LoopSummaryCertificate>(),
        TypeId::of::<qcore_core::resource_accounting::LoopSummaryCertificate>()
    );
    assert_eq!(
        TypeId::of::<qcore_compiler::research_extensions::LedgerMorphism>(),
        TypeId::of::<qcore_core::resource_accounting::LedgerMorphism>()
    );
    let _: fn(
        &qcore_core::resource_accounting::LoopSummaryCertificate,
    ) -> Result<(), qcore_core::resource_accounting::LoopSummaryError> =
        qcore_compiler::research_extensions::check_loop_summary;
}

#[test]
fn trusted_replay_and_derivation_are_verification_owned() {
    use std::any::TypeId;
    assert_eq!(
        TypeId::of::<qcore_compiler::obligation_derivation::DerivedTask>(),
        TypeId::of::<qcore_verification::obligation_derivation::DerivedTask>()
    );
    assert_eq!(
        TypeId::of::<qcore_compiler::verify_dispatch::ScopeDispatch>(),
        TypeId::of::<qcore_verification::verify_dispatch::ScopeDispatch>()
    );
    assert_eq!(
        TypeId::of::<qcore_compiler::witness_replay::ChunkLifecycleReplay<'static>>(),
        TypeId::of::<qcore_verification::witness_replay::ChunkLifecycleReplay<'static>>()
    );
}

#[test]
fn reactor_models_are_core_owned_and_adapter_api_is_preserved() {
    use std::any::TypeId;
    assert_eq!(
        TypeId::of::<qcore_compiler::EventKey>(),
        TypeId::of::<qcore_core::event_reactor::EventKey>()
    );
    let model: qcore_core::event_reactor::EventProgram<qcore_compiler::LibeventBuildProfile> =
        qcore_compiler::EventProgram::default().into();
    assert!(model.operations.is_empty());
    assert!(model.live_arguments.is_empty());
    type Checker =
        fn(
            &qcore_compiler::EventProgram,
            &[qcore_schemas::profile::BuildProfileSelector],
        )
            -> Result<qcore_compiler::EventCertificate, Vec<qcore_compiler::EventDiagnostic>>;
    let _: Checker = qcore_compiler::check_event_program;
    assert_eq!(qcore_compiler::EventProgram::default().operations.len(), 0);
}

#[test]
fn reactor_adapter_enum_supports_external_variant_imports() {
    // This integration test is an external consumer of qcore_compiler.
    // A type alias to a generic enum cannot satisfy either import form.
    use qcore_compiler::EventOperation::*;
    let pair = CreateSocketPair {
        descriptors: Some((3, 4)),
    };
    let close = {
        use qcore_compiler::EventOperation::CloseSocket;
        CloseSocket {
            descriptor: 3,
            succeeds: true,
            declared_failure: None,
        }
    };
    let program = qcore_compiler::EventProgram {
        live_arguments: std::collections::BTreeSet::from([9]),
        operations: vec![pair, close, DropArgument { argument: 9 }],
    };
    let model: qcore_core::event_reactor::EventProgram<qcore_compiler::LibeventBuildProfile> =
        program.into();
    assert_eq!(model.live_arguments, std::collections::BTreeSet::from([9]));
    assert_eq!(
        model.operations,
        vec![
            qcore_core::event_reactor::EventOperation::CreateSocketPair {
                descriptors: Some((3, 4))
            },
            qcore_core::event_reactor::EventOperation::CloseSocket {
                descriptor: 3,
                succeeds: true,
                declared_failure: None
            },
            qcore_core::event_reactor::EventOperation::DropArgument { argument: 9 },
        ]
    );
}

#[test]
fn generic_driver_and_custody_facades_share_the_extracted_types() {
    use std::any::TypeId;
    assert_eq!(
        TypeId::of::<qcore_compiler::external_integration::LibraryCustodyProgram>(),
        TypeId::of::<qcore_core::library_custody::LibraryCustodyProgram>()
    );
    assert_eq!(
        TypeId::of::<qcore_compiler::external_integration::LibraryImportCertificate>(),
        TypeId::of::<qcore_verification::library_import::LibraryImportCertificate>()
    );
    assert_eq!(
        TypeId::of::<qcore_compiler::driver::CompiledModule>(),
        TypeId::of::<qcore_verification::driver::CompiledModule>()
    );
    assert_eq!(
        TypeId::of::<qcore_compiler::external_source::LibrarySourceCertificate>(),
        TypeId::of::<qcore_verification::external_source::LibrarySourceCertificate>()
    );
    assert_eq!(
        TypeId::of::<qcore_compiler::research_extensions::Cycle17Certificate>(),
        TypeId::of::<qcore_verification::research_extensions::Cycle17Certificate>()
    );
}

//! Layering guard from `docs/GENERALIZATION_PLAN.md` W0 task 4 and
//! invariant 7: workspace dependencies point strictly downward, and the
//! published schema surface depends on no workspace crate.

use std::path::PathBuf;

/// Workspace crates and the workspace dependencies each is allowed
/// (`docs/GENERALIZATION_PLAN.md` §3.1; the grab-bag `qcore-compiler` and
/// the CLI sit above every extracted layer).
const ALLOWED: &[(&str, &[&str])] = &[
    ("crates/qcore-schemas", &[]),
    ("crates/qcore-core", &[]),
    ("crates/qcore-frontend", &["qcore-core"]),
    (
        "crates/qcore-contract",
        &["qcore-core", "qcore-frontend", "qcore-schemas"],
    ),
    (
        "crates/qcore-c-gateway",
        &["qcore-contract", "qcore-schemas"],
    ),
    (
        "crates/qcore-verifier-host",
        &["qcore-frontend", "qcore-schemas"],
    ),
    (
        "compiler",
        &[
            "qcore-audit", // examples-only dev dependency, not the compiler library
            "qcore-c-gateway",
            "qcore-contract",
            "qcore-core",
            "qcore-frontend",
            "qcore-schemas",
            "qcore-verifier-host",
        ],
    ),
    (
        "crates/qcore-audit",
        &["qcore-c-gateway", "qcore-contract", "qcore-schemas"],
    ),
    ("crates/qcorec", &["qcore-compiler"]),
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

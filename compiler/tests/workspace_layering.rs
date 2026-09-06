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
            "qcore-c-gateway",
            "qcore-contract",
            "qcore-core",
            "qcore-frontend",
            "qcore-schemas",
            "qcore-verifier-host",
        ],
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

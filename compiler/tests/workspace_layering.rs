//! Layering guard from `docs/GENERALIZATION_PLAN.md` W0 task 4: dependencies
//! point strictly downward, so the published schema surface must never
//! depend on any other workspace crate.

use std::path::PathBuf;

#[test]
fn qcore_schemas_has_no_workspace_dependencies() {
    let manifest_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../crates/qcore-schemas/Cargo.toml");
    let manifest = std::fs::read_to_string(&manifest_path)
        .unwrap_or_else(|error| panic!("{}: {error}", manifest_path.display()));

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
        assert!(
            !line.contains("path =") && !line.contains("path="),
            "qcore-schemas must not use path dependencies (dependencies point downward only): {line}"
        );
        assert!(
            !line.contains("qcore"),
            "qcore-schemas must not depend on any workspace crate: {line}"
        );
    }
}

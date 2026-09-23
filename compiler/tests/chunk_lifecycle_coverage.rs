//! The published lifecycle result for every selected input-chunk function.
//!
//! Card `W42-CHUNK-LIFECYCLE-CERT`. The inventory is transported by
//! `docs/reports/w42-chunk-lifecycle-cert/scripts/lifecycle_inventory_extract.py`,
//! which refuses to run unless the pinned sidecar's uncompressed digest matches.
//! Every verdict below is decided here, in Rust, and the committed result
//! artifact is regenerated and compared on every gate run.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use qcore_compiler::verifier_plugins::chunkio::lifecycle_coverage::{
    FieldContract, SelectedFunction, derive_lifecycle_evidence,
};
use qcore_verification::chunk_lifecycle_coverage::{
    ChunkLifecycleCoverageReport, ChunkLifecycleStatus, EVIDENCE_VERSION, FunctionIdentity,
    LifecycleRole, REQUIRED_LEGS, SEMANTIC_PROFILE, SourceBinding,
    account_chunk_lifecycle_coverage,
};
use serde::Deserialize;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the compiler crate lives one level below the repository root")
        .to_path_buf()
}

fn report_dir() -> PathBuf {
    repo_root().join("docs/reports/w42-chunk-lifecycle-cert")
}

#[derive(Debug, Deserialize)]
struct Inventory {
    artifact: String,
    artifact_sha256: String,
    source_revision: String,
    source_profile: String,
    backend_teardown_records: usize,
    selected_functions: usize,
    functions: Vec<InventoryRow>,
}

#[derive(Debug, Deserialize)]
struct InventoryRow {
    library: String,
    file: String,
    function: String,
    field_contract: InventoryFieldContract,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum InventoryFieldContract {
    Checked { states: usize },
    Refused { reason: String, reason_code: String },
}

fn load_inventory() -> Inventory {
    let path = report_dir().join("inventory.json");
    let text = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
    serde_json::from_str(&text)
        .unwrap_or_else(|error| panic!("cannot parse {}: {error}", path.display()))
}

fn assumptions() -> BTreeMap<String, String> {
    BTreeMap::from([
        (
            "extraction".into(),
            "faithful C extraction and Rust-model correspondence remain premises maintained by \
             review (evidence rule 28)"
                .into(),
        ),
        (
            "field_contracts".into(),
            "the pinned sidecar's field contracts assume live/exclusive/Boolean domains and \
             source/build/ABI identity; they establish no lifecycle post-state"
                .into(),
        ),
        (
            "unsealed_report_values".into(),
            "the pinned sidecar's recorded certificates are unsealed report data and are never \
             revalidated here as certificate evidence"
                .into(),
        ),
        (
            "composition_binding".into(),
            "the ChunkIO compositions this card re-derives are taken over committed C fixtures, \
             not over the pinned upstream translation units"
                .into(),
        ),
    ])
}

fn decide(inventory: &Inventory) -> ChunkLifecycleCoverageReport {
    let selected: Vec<SelectedFunction> = inventory
        .functions
        .iter()
        .map(|row| SelectedFunction {
            identity: FunctionIdentity::new(&row.library, &row.file, &row.function),
            field_contract: match &row.field_contract {
                InventoryFieldContract::Checked { states } => {
                    FieldContract::Checked { states: *states }
                }
                InventoryFieldContract::Refused {
                    reason,
                    reason_code,
                } => FieldContract::Refused {
                    reason: reason.clone(),
                    reason_code: reason_code.clone(),
                },
            },
        })
        .collect();
    // No classification is supplied: the pinned projection classifies no
    // selected function's lifecycle role, so every row takes this module's
    // fail-closed default and keeps the projection's own refusal.
    let evidence = derive_lifecycle_evidence(&selected, Vec::new())
        .expect("the pinned selection derives its own evidence rows");
    let identities: Vec<FunctionIdentity> = selected
        .iter()
        .map(|entry| entry.identity.clone())
        .collect();
    account_chunk_lifecycle_coverage(
        SourceBinding {
            artifact: inventory.artifact.clone(),
            artifact_sha256: inventory.artifact_sha256.clone(),
            source_revision: inventory.source_revision.clone(),
            source_profile: inventory.source_profile.clone(),
        },
        &identities,
        assumptions(),
        evidence,
    )
    .expect("total accounting over the pinned selection")
}

#[test]
fn the_pinned_inventory_selects_both_chunk_units_including_input_chunk_get() {
    let inventory = load_inventory();
    assert_eq!(inventory.selected_functions, inventory.functions.len());
    assert_eq!(inventory.selected_functions, 96);
    assert_eq!(inventory.backend_teardown_records, 21);
    assert_eq!(
        inventory.source_revision,
        "66910c10a4d7eafa810b84229cf2dcf7b0f26f97"
    );
    let chunkio = inventory
        .functions
        .iter()
        .filter(|row| row.file == "cio_chunk.c")
        .count();
    let wrapper = inventory
        .functions
        .iter()
        .filter(|row| row.file == "flb_input_chunk.c")
        .count();
    assert_eq!((chunkio, wrapper), (30, 66));
    // The card names this residual explicitly; it must be in the denominator.
    assert!(
        inventory
            .functions
            .iter()
            .any(|row| row.function == "input_chunk_get")
    );
    assert!(
        inventory
            .functions
            .iter()
            .any(|row| row.function == "flb_input_chunk_create")
    );
}

#[test]
fn every_selected_function_has_a_checked_lifecycle_result() {
    let inventory = load_inventory();
    let report = decide(&inventory);
    assert_eq!(report.selected_functions, 96);
    assert_eq!(report.accounted_functions, 96);
    assert_eq!(report.semantic_profile, SEMANTIC_PROFILE);
    assert_eq!(report.evidence_version, EVIDENCE_VERSION);
    // Fail-closed: no selected function's lifecycle post-state is established
    // by the pinned evidence, so none is promoted.
    assert_eq!(report.safe_functions, 0);
    assert_eq!(report.unknown_functions, 96);
    assert_eq!(report.role_totals.get("unclassified"), Some(&96));
    for row in &report.functions {
        assert_eq!(row.role, LifecycleRole::Unclassified);
        let ChunkLifecycleStatus::Unknown {
            reason,
            outstanding_legs,
        } = &row.lifecycle
        else {
            panic!(
                "{} was promoted without a composition",
                row.identity.function
            );
        };
        assert!(
            !reason.trim().is_empty(),
            "{} has an Unknown without a reason",
            row.identity.function
        );
        assert_eq!(
            outstanding_legs.len(),
            REQUIRED_LEGS.len(),
            "{} discharged a leg it has no evidence for",
            row.identity.function
        );
        assert!(row.model.is_none());
        assert!(row.delete_only_post_state.is_none());
    }
    for total in &report.outstanding_leg_totals {
        assert_eq!(total.functions, 96, "{:?}", total.leg);
    }
}

#[test]
fn the_published_lifecycle_result_matches_the_committed_artifact() {
    let inventory = load_inventory();
    let report = decide(&inventory);
    let rendered = format!(
        "{}\n",
        serde_json::to_string_pretty(&report).expect("the report serializes")
    );
    let path = report_dir().join("lifecycle-coverage.json");
    let committed = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
    if committed != rendered {
        let actual = Path::new(env!("CARGO_TARGET_TMPDIR")).join("lifecycle-coverage.json");
        fs::write(&actual, &rendered).expect("the regenerated result is writable");
        panic!(
            "{} differs from the regenerated result; the actual bytes are at {}",
            path.display(),
            actual.display()
        );
    }
}

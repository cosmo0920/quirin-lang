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

#![cfg_attr(windows, allow(dead_code, unused_imports))]

//! The published lifecycle result for every selected input-chunk function.
//!
//! Card `W42-CHUNK-LIFECYCLE-CERT` established the accounting; its published
//! result (`docs/reports/w42-chunk-lifecycle-cert/lifecycle-coverage.json`) is
//! frozen evidence and is no longer regenerated here. Card
//! `W43-CHUNK-SOURCE-PROFILE` derives compositions from the selected
//! functions' **own pinned units** and publishes the current result under its
//! own report. The inventory is still the W42 digest-bound extract, transported
//! by `docs/reports/w42-chunk-lifecycle-cert/scripts/lifecycle_inventory_extract.py`.
//! Every verdict below is decided here, in Rust, and the committed result is
//! regenerated and compared on every gate run.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use qcore_compiler::verifier_plugins::chunkio::lifecycle_coverage::{
    ClassifiedFunction, FieldContract, SelectedFunction, derive_lifecycle_evidence,
};
use qcore_compiler::verifier_plugins::chunkio::pinned_lifecycle::{
    CHUNK_UNIT, ERROR_UNIT, MEMFS_UNIT, PinnedChunkUnits, PinnedUnit, derive_pinned_lifecycle,
    load_vendored_units, profile_pinned_digest,
};
use qcore_verification::chunk_lifecycle_coverage::{
    ChunkLifecycleCoverageReport, ChunkLifecycleStatus, EVIDENCE_VERSION, FunctionIdentity,
    LifecycleLeg, LifecycleRole, REQUIRED_LEGS, SEMANTIC_PROFILE, SourceBinding,
    account_chunk_lifecycle_coverage,
};
use serde::Deserialize;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the compiler crate lives one level below the repository root")
        .to_path_buf()
}

fn inventory_dir() -> PathBuf {
    repo_root().join("docs/reports/w42-chunk-lifecycle-cert")
}

fn report_dir() -> PathBuf {
    repo_root().join("docs/reports/w43-chunk-source-profile")
}

fn vendored_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/pinned/fluent-bit")
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
    let path = inventory_dir().join("inventory.json");
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
            "every attached composition is re-derived on each run from verbatim copies of the \
             pinned units under compiler/tests/fixtures/pinned/fluent-bit; cio_chunk.c is bound \
             to the digest contract/profiles/fluent-bit/profile.json pins, the other copied \
             files to the vendored manifest; distilled fixtures are negative and positive \
             controls only and are refused as row evidence"
                .into(),
        ),
        (
            "vendored_build_configuration".into(),
            "the vendored units are preprocessed with the pinned compile database's recorded \
             definitions and the subset of its include roots the measured include closure uses, \
             in the database's order; host system headers are the host's, as in the pinned audit"
                .into(),
        ),
        (
            "composition_premises".into(),
            "the close and ring-close compositions keep every premise of their component \
             contracts: native pointer provenance, ring order and model identity binding, \
             exclusive access and a live context and stream"
                .into(),
        ),
    ])
}

fn selected(inventory: &Inventory) -> Vec<SelectedFunction> {
    inventory
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
        .collect()
}

fn binding(inventory: &Inventory) -> SourceBinding {
    SourceBinding {
        artifact: inventory.artifact.clone(),
        artifact_sha256: inventory.artifact_sha256.clone(),
        source_revision: inventory.source_revision.clone(),
        source_profile: inventory.source_profile.clone(),
        // Only an independently pinned unit can bind a composition. The
        // profile pins cio_chunk.c; flb_input_chunk.c has no such pin, so any
        // composition attached to one of its rows is refused.
        pinned_units: BTreeMap::from([(
            "cio_chunk.c".to_owned(),
            profile_pinned_digest(CHUNK_UNIT).expect("the profile pins cio_chunk.c"),
        )]),
    }
}

fn decide(
    inventory: &Inventory,
    classified: Vec<ClassifiedFunction>,
) -> ChunkLifecycleCoverageReport {
    let selected = selected(inventory);
    let evidence = derive_lifecycle_evidence(&selected, classified)
        .expect("the pinned selection derives its own evidence rows");
    let identities: Vec<FunctionIdentity> = selected
        .iter()
        .map(|entry| entry.identity.clone())
        .collect();
    account_chunk_lifecycle_coverage(binding(inventory), &identities, assumptions(), evidence)
        .expect("total accounting over the pinned selection")
}

fn pinned_rows() -> Option<Vec<ClassifiedFunction>> {
    if let Err(reason) = qcore_c_gateway::clang_command() {
        eprintln!("warning: skipping Clang-dependent lifecycle derivation: {reason}");
        return None;
    }
    let units = load_vendored_units(&vendored_root()).expect("the vendored pinned units load");
    Some(derive_pinned_lifecycle(&units).expect("the pinned units derive their rows"))
}

#[cfg(windows)]
#[test]
fn windows_pinned_linux_unit_refuses_before_lifecycle_publication() {
    let units = load_vendored_units(&vendored_root()).expect("the vendored pinned units load");
    let error = derive_pinned_lifecycle(&units).unwrap_err();
    assert!(error.contains("getpagesize"), "{error}");
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
    for function in [
        "input_chunk_get",
        "flb_input_chunk_create",
        "cio_chunk_close",
    ] {
        assert!(
            inventory
                .functions
                .iter()
                .any(|row| row.function == function)
        );
    }
}

#[cfg(not(windows))]
#[test]
fn every_selected_function_has_a_checked_lifecycle_result() {
    let Some(rows) = pinned_rows() else {
        return;
    };
    let inventory = load_inventory();
    let report = decide(&inventory, rows);
    assert_eq!(report.selected_functions, 96);
    assert_eq!(report.accounted_functions, 96);
    assert_eq!(report.semantic_profile, SEMANTIC_PROFILE);
    assert_eq!(report.evidence_version, EVIDENCE_VERSION);
    // Fail-closed: two rows gain a pinned-source composition, and neither is
    // promoted, because each leaves its container live by premise and leaves
    // the filesystem and unsupported-backend paths uncovered.
    assert_eq!(report.safe_functions, 0);
    assert_eq!(report.unknown_functions, 96);
    assert_eq!(report.role_totals.get("unclassified"), Some(&94));
    assert_eq!(report.role_totals.get("destruction_dependent"), Some(&2));
    for row in &report.functions {
        let ChunkLifecycleStatus::Unknown {
            reason,
            outstanding_legs,
        } = &row.lifecycle
        else {
            panic!("{} was promoted", row.identity.function);
        };
        assert!(!reason.trim().is_empty());
        // A path-incomplete composition discharges no model leg, and no row
        // has source evidence for the error-exit or wrapper legs.
        assert_eq!(
            outstanding_legs.len(),
            REQUIRED_LEGS.len(),
            "{} discharged a leg",
            row.identity.function
        );
        match row.identity.function.as_str() {
            "cio_chunk_close" | "cio_chunk_close_stream" => {
                assert_eq!(row.role, LifecycleRole::DestructionDependent);
                let derivation = row.derivation.as_ref().expect("a bound derivation");
                assert_eq!(derivation.source.function, row.identity.function);
                assert!(!derivation.uncovered_paths.is_empty());
                assert!(
                    reason.contains("discharge nothing for the function"),
                    "{reason}"
                );
                assert!(reason.contains("context 0, stream 0"), "{reason}");
            }
            _ => {
                assert_eq!(row.role, LifecycleRole::Unclassified);
                assert!(row.model.is_none() && row.derivation.is_none());
            }
        }
    }
    let input_chunk_get = report
        .functions
        .iter()
        .find(|row| row.identity.function == "input_chunk_get")
        .unwrap();
    assert_eq!(input_chunk_get.role, LifecycleRole::Unclassified);
    for total in &report.outstanding_leg_totals {
        assert_eq!(total.functions, 96, "{:?}", total.leg);
    }
    assert!(
        report
            .outstanding_leg_totals
            .iter()
            .any(|total| total.leg == LifecycleLeg::ChunkTeardown)
    );
}

#[cfg(not(windows))]
#[test]
fn a_fixture_derived_composition_cannot_be_attached_to_a_pinned_row() {
    let Some(mut rows) = pinned_rows() else {
        return;
    };
    let inventory = load_inventory();
    let selected = selected(&inventory);
    let identities: Vec<FunctionIdentity> = selected
        .iter()
        .map(|entry| entry.identity.clone())
        .collect();
    let account = |rows: Vec<ClassifiedFunction>| {
        let evidence = derive_lifecycle_evidence(&selected, rows).unwrap();
        account_chunk_lifecycle_coverage(binding(&inventory), &identities, assumptions(), evidence)
    };
    // Bytes other than the pinned unit's: the W42 distilled fixture's digest.
    let fixture = include_str!("fixtures/chunkio/memory_close_source.c");
    let mut foreign = rows.clone();
    foreign[0].derivation.as_mut().unwrap().source.unit_sha256 =
        qcore_schemas::sha256_hex(fixture.as_bytes());
    let error = account(foreign).unwrap_err();
    assert!(error.contains("rather than its pinned unit"), "{error}");
    // Another function's composition.
    let mut borrowed = rows.clone();
    borrowed[1].derivation = borrowed[0].derivation.clone();
    let error = account(borrowed).unwrap_err();
    assert!(error.contains("not from its own source"), "{error}");
    // A composition with its derivation stripped.
    rows[0].derivation = None;
    let error = account(rows).unwrap_err();
    assert!(
        error.contains("without the source it was derived from"),
        "{error}"
    );
}

#[cfg(not(windows))]
#[test]
fn the_published_lifecycle_result_matches_the_committed_artifact() {
    let Some(rows) = pinned_rows() else {
        return;
    };
    let inventory = load_inventory();
    let report = decide(&inventory, rows);
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

/// Parity with the pinned build itself: derive the same rows from the pinned
/// checkout through its own compilation database, as the pinned audit does,
/// and require byte-identical results and byte-identical unit copies.
///
/// It needs the out-of-tree pinned checkout, so it is ignored by default and
/// run explicitly with `QCORE_W43_PINNED_SOURCE` (the checkout root) and
/// `QCORE_W43_PINNED_DATABASE` (its `compile_commands.json`).
#[test]
#[ignore = "needs the out-of-tree pinned Fluent Bit checkout and compilation database"]
fn the_vendored_units_derive_what_the_pinned_build_derives() {
    let root = PathBuf::from(
        std::env::var_os("QCORE_W43_PINNED_SOURCE").expect("QCORE_W43_PINNED_SOURCE is set"),
    );
    let database = fs::read_to_string(
        std::env::var_os("QCORE_W43_PINNED_DATABASE").expect("QCORE_W43_PINNED_DATABASE is set"),
    )
    .expect("the pinned compilation database is readable");
    let entries = qcore_audit::compile_db::compile_entries(&database).unwrap();
    let vendored = load_vendored_units(&vendored_root()).unwrap();
    let unit = |relative: &str| -> PinnedUnit {
        let file = root.join(relative);
        // The pinned audit's exact-unit selection takes the first entry.
        let entry = entries
            .iter()
            .find(|entry| entry.file == file)
            .unwrap_or_else(|| panic!("the database names no {relative}"));
        let options = qcore_audit::compile_db::compiler_options_compatibility(entry).unwrap();
        PinnedUnit {
            path: relative.to_owned(),
            source: fs::read_to_string(&file).unwrap(),
            include_paths: options.include_paths,
            definitions: options.definitions,
            dialect: options.dialect,
        }
    };
    let pinned =
        PinnedChunkUnits::from_options(unit(CHUNK_UNIT), unit(MEMFS_UNIT), unit(ERROR_UNIT))
            .expect("the pinned checkout carries the profile-pinned unit");
    assert_eq!(pinned.digests(), vendored.digests());
    let from_build = derive_pinned_lifecycle(&pinned).unwrap();
    let from_vendored = derive_pinned_lifecycle(&vendored).unwrap();
    assert_eq!(format!("{from_build:?}"), format!("{from_vendored:?}"));
    let inventory = load_inventory();
    let rendered = |rows| serde_json::to_string_pretty(&decide(&inventory, rows)).unwrap();
    assert_eq!(rendered(from_build), rendered(from_vendored));
}

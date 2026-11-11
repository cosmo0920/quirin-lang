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

//! Card **CORPUS-GROUP-CHECKPOINTS** (R8 option D): per-group `.qo`
//! checkpoints of the pinned corpus route (`quirin-audit pinned
//! --checkpoint-dir DIR [--resume]`).
//!
//! The route's library groups (`audit.json`'s libraries, then
//! `monkey-libevent`) are its checkpoint groups: the ChunkIO prelude and
//! postlude share typed certificates across a library's units, so a library
//! is the smallest unit whose result stands alone. Every phase the route runs
//! is a **stage** with its own file, `<label>.<stage>.qo`:
//!
//! | stage      | analysis environment                                   |
//! |------------|--------------------------------------------------------|
//! | `first`    | the loaded contracts (phase one)                       |
//! | `refuted`  | the contracts with phase one's refutation table (C06.4) |
//! | `imported` | the cross-unit environment and homes (link map)        |
//!
//! A stage's checkpoint holds the group's whole contribution to the route's
//! fold: the prelude's console lines, each unit's console lines, report
//! fragment, tally, native and teardown results, and the postlude's console
//! lines and teardown results. The postlude runs when the group completes in
//! the stage, so a completed group is sealed as soon as its last unit ends,
//! whatever the other groups are doing; an interrupted run keeps every sealed
//! group.
//!
//! The identity ([`PinnedIdentity`]) is everything a group's stage result
//! depends on: the run inputs (analyzer build, checkout and revision, source
//! profile, source-contract policies and packages, contract manifest, locks
//! and pin evaluation, Clang, the `QCORE_*` environment with output paths
//! reduced to their selection and the link map to its content digest, the
//! run's member facts), the stage's whole-run facts (the exact contract
//! environment every unit is analysed under, which carries the refutation
//! table and the imports; the cross-unit homes; the bounds and null
//! environments) and the group's own inputs (each unit's compile record, raw
//! source and preparation digests). A group is reused only when the whole
//! identity is equal; anything it cannot cover is refused before any work
//! ([`refuse_unsupported`]) or forces the group to be recomputed.
//!
//! The format is specified in `docs/development/audit-checkpoints.md`.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};
use std::time::Instant;

use qcore_c_gateway::legacy_c::{
    C11TranslationUnit, CrossUnitHomeEvidence, LegacyFunctionScope, ResourceContracts,
    prepare_translation_unit,
};
use qcore_contract::profile_loader::LoadedProfile;
use qcore_schemas::sha256_hex;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::{
    ChunkioCaptures, LibraryContext, LibraryPlan, PublishOnDrop, SourceContractConfiguration, Step,
    StepOutcome, UnitProducts, audit_unit, cross_unit_premise_of, finish_library, prepare_library,
    select_homes,
};
use crate::checkpoint::{
    AnalyzerIdentity, BinaryValue, CategoryCount, CheckpointReader, CompilerIdentity, CountsRecord,
    EnvironmentValue, Layout, LockDigest, PreparationFacts, assemble_as, content_sha256_as, encode,
    json_sha256,
};
use crate::checkpoint_run::{status_kib, write_atomically};
use crate::compile_db::{CompileEntry, compiler_options_compatibility as compiler_options};
use crate::current_route::source_weight;
use crate::parallel::{heaviest_first, run_ordered};
use crate::projection::AuditCounts;
use crate::report::{AuditReport, FragmentParts};

/// The pinned group checkpoint's schema version. The container is the
/// current route's (`checkpoint::Layout`); this schema's section list differs
/// from the current route's, so each reader refuses the other's files.
pub const SCHEMA_VERSION: u32 = 1;
/// The sections of a pinned group checkpoint, in file order.
pub const SECTIONS: [&str; 5] = [
    "pinned-identity",
    "pinned-prelude",
    "pinned-units",
    "pinned-finish",
    "provenance",
];
/// The pinned group checkpoint's layout.
pub const LAYOUT: Layout = Layout {
    schema_version: SCHEMA_VERSION,
    sections: &SECTIONS,
};
/// The run record written beside the checkpoints (provenance only).
pub const RUN_RECORD: &str = "pinned-checkpoint-run.json";

/// Where the checkpoints go, and whether matching ones are reused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Options {
    pub directory: PathBuf,
    pub resume: bool,
}

/// Variables whose value is where the run writes an output the fold
/// produces from the rows: the identity records that the output is selected,
/// not its path. Every other `QCORE_*` variable but the job count is recorded
/// exactly.
const OUTPUT_SELECTIONS: [&str; 3] = [
    crate::projection::NULL_SELECTION,
    crate::outer_selection::PROJECTION,
    "QCORE_MEMBER_PREPASS_SECTION",
];

/// Variables a checkpoint cannot stand for: each makes a unit's analysis
/// write a side output (or read an input) a reused group would not reproduce
/// or the identity does not digest.
const REFUSED: [(&str, &str); 5] = [
    (
        crate::projection::BOUNDS_SELECTION,
        "the bounds selection is not supported by pinned checkpoints yet (CORPUS-CHECKPOINT-BOUNDS-SELECTION); run without --checkpoint-dir",
    ),
    (
        "QCORE_TYPED_PATH_PARITY",
        "a parity stream written while units are analysed, which a reused group would not write",
    ),
    (
        "QCORE_AUDIT_IR_DIR",
        "per-unit IR artifacts written while units are analysed, which a reused group would not write",
    ),
    (
        "QCORE_UNIT_TIMING",
        "per-unit timing records written while units are analysed, which a reused group would not write",
    ),
    (
        "QCORE_SOURCE_CONTRACTS",
        "a process source-contract configuration whose packages the identity does not digest; use QCORE_CHUNKIO_PACKAGE or the profile's direct policies",
    ),
];

/// Refuse, before any work, an environment a checkpoint cannot stand for.
pub(super) fn refuse_unsupported() -> Result<(), String> {
    let census = qcore_c_gateway::active_census_sinks();
    if !census.is_empty() {
        return Err(format!(
            "pinned checkpoints refuse a census run: census streams {census:?} are process-global \
             and record in analysis order, so a reused group could not stand for its share of them"
        ));
    }
    for (variable, why) in REFUSED {
        if std::env::var_os(variable).is_some() {
            return Err(format!("pinned checkpoints refuse {variable}: {why}"));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Identity
// ---------------------------------------------------------------------------

/// Every input a group's stage result depends on. A group is reused only
/// when this whole section is equal.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PinnedIdentity {
    pub run: PinnedRunIdentity,
    pub stage: StageIdentity,
    pub group: PinnedGroupIdentity,
}

/// The run inputs every group shares.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PinnedRunIdentity {
    /// `pinned`.
    pub route: String,
    pub analyzer: AnalyzerIdentity,
    /// The canonical Fluent Bit root and its verified pinned revision.
    pub fluent_bit_root: String,
    pub fluent_bit_revision: String,
    /// The process's working directory: the pinned route binds it into
    /// published rows (each function's `hypothesis_context.build_identity`
    /// and the source-effect configuration digests), so it is an input.
    pub working_directory: String,
    /// SHA-256 of every file of the source-set profile's directory
    /// (`profile.json`, `audit.json`, the member-facts scope, the direct
    /// source-contract policies, ...), by relative path.
    pub source_profile_sha256: String,
    /// The source-contract configuration the route loaded and, in package
    /// mode, every file of each package, by relative path.
    pub source_contracts: SourceContractsIdentity,
    /// The resource contract manifest's path and SHA-256.
    pub contract_manifest: String,
    pub contract_manifest_sha256: String,
    /// Every contract lock the pin evaluation read, with its SHA-256.
    pub locks: Vec<LockDigest>,
    /// SHA-256 of the pin evaluation's section template.
    pub pin_evaluation_sha256: String,
    pub compiler: CompilerIdentity,
    /// Every `QCORE_*` variable set for the run except the job count, sorted;
    /// an output selection's value is `selected`, the link map's is its
    /// content digest.
    pub environment: Vec<EnvironmentValue>,
    /// SHA-256 of the run's member facts (rule 12n), when the pre-pass ran:
    /// every unit decides its member invocations over them.
    pub member_facts_sha256: Option<String>,
}

/// The source-contract policies and packages.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceContractsIdentity {
    pub configuration: String,
    pub configuration_sha256: String,
    /// In package mode (`QCORE_CHUNKIO_PACKAGE`), the package's path and the
    /// SHA-256 of its files; absent in direct mode.
    pub package: Option<TreeIdentity>,
}

/// A directory, by path, and the SHA-256 over its files.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TreeIdentity {
    pub path: String,
    pub sha256: String,
}

/// The whole-run facts one stage's analysis reads.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StageIdentity {
    /// `first`, `refuted` or `imported`.
    pub stage: String,
    /// SHA-256 of the exact contract environment every unit is analysed
    /// under (its serde form, as the cross-unit home digest takes it): the
    /// pinned contracts with their pin holds and result ranges, the
    /// refutation table the stage applies (C06.4), and in the import stage
    /// the imported contracts, their identities and the link-map digest.
    pub environment_sha256: String,
    /// SHA-256 of the cross-unit homes supplied to every unit (function,
    /// import identity, line-marked source, dialect, link resolution); the
    /// empty list's digest outside the import stage.
    pub homes_sha256: String,
    /// SHA-256 of the installed bounds and null environments (their digests,
    /// which carry the bounds refutation table, and the selection record).
    pub bounds_environment_sha256: String,
}

/// The group's own inputs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PinnedGroupIdentity {
    pub label: String,
    pub source_root: String,
    pub units: Vec<PinnedUnitFingerprint>,
}

/// One unit's inputs: its compile record, its raw source and what
/// preparation (three Clang runs) read through its headers.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PinnedUnitFingerprint {
    pub file: String,
    /// SHA-256 of the compact JSON `[file, directory, command]`.
    pub record_sha256: String,
    pub raw_source_sha256: String,
    /// SHA-256 of the validated plain preprocessing, when it validated.
    pub preprocessed_sha256: Option<String>,
    pub preparation: PreparationFacts,
}

// ---------------------------------------------------------------------------
// Stored results
// ---------------------------------------------------------------------------

/// One unit's contribution to the fold, in typed binary values.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StoredUnit {
    pub output: String,
    pub rows: Vec<BinaryValue>,
    pub diagnostic_counts: Vec<CategoryCount>,
    pub bounds: Vec<BinaryValue>,
    pub bounds_evidence: Vec<BinaryValue>,
    pub null: Vec<BinaryValue>,
    pub null_evidence: Vec<BinaryValue>,
    pub callback_members: Vec<BinaryValue>,
    pub counts: CountsRecord,
    pub native_results: Vec<BinaryValue>,
    pub teardown_results: Vec<BinaryValue>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreludeSection {
    pub output: String,
}

#[derive(Serialize)]
struct UnitsSectionRef<'a> {
    units: Vec<&'a StoredUnit>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnitsSection {
    pub units: Vec<StoredUnit>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FinishSection {
    pub output: String,
    pub teardown_results: Vec<BinaryValue>,
}

/// Measurements of the run that sealed the file: not identity, not folded.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PinnedProvenance {
    /// From the stage's start to the seal, in milliseconds.
    pub wall_time_ms: u64,
    pub jobs: u64,
    pub process_peak_rss_kib: Option<u64>,
}

fn values(values: &[Value]) -> Result<Vec<BinaryValue>, String> {
    values.iter().map(BinaryValue::from_json).collect()
}

fn jsons(values: &[BinaryValue]) -> Result<Vec<Value>, String> {
    values.iter().map(BinaryValue::to_json).collect()
}

fn counts_record(counts: &AuditCounts) -> CountsRecord {
    let count = |value: usize| value as u64;
    CountsRecord {
        units: count(counts.units),
        data_only_units: count(counts.data_only_units),
        source_functions: count(counts.source_functions),
        projected_functions: count(counts.projected_functions),
        safe_units: count(counts.safe_units),
        pending_units: count(counts.pending_units),
        unsafe_units: count(counts.unsafe_units),
        unknown_units: count(counts.unknown_units),
        gateway_failures: count(counts.gateway_failures),
        absent_from_raw_scan_definitions: count(counts.absent_from_raw_scan_definitions),
        unanalyzable_definitions: count(counts.unanalyzable_definitions),
        enumeration_refused_units: count(counts.enumeration_refused_units),
        withheld_safe_units: count(counts.withheld_safe_units),
        prepared_units: count(counts.prepared_units),
        preparation_clang_invocations: count(counts.preparation_clang_invocations),
    }
}

/// The tally a stored unit restores to. The three preparation timings are
/// measurements the pinned route never prints, so they are not stored.
fn audit_counts(record: &CountsRecord) -> Result<AuditCounts, String> {
    let count = |value: u64| usize::try_from(value).map_err(|error| error.to_string());
    Ok(AuditCounts {
        units: count(record.units)?,
        data_only_units: count(record.data_only_units)?,
        source_functions: count(record.source_functions)?,
        projected_functions: count(record.projected_functions)?,
        safe_units: count(record.safe_units)?,
        unsafe_units: count(record.unsafe_units)?,
        unknown_units: count(record.unknown_units)?,
        pending_units: count(record.pending_units)?,
        gateway_failures: count(record.gateway_failures)?,
        absent_from_raw_scan_definitions: count(record.absent_from_raw_scan_definitions)?,
        unanalyzable_definitions: count(record.unanalyzable_definitions)?,
        enumeration_refused_units: count(record.enumeration_refused_units)?,
        withheld_safe_units: count(record.withheld_safe_units)?,
        prepared_units: count(record.prepared_units)?,
        preparation_clang_invocations: count(record.preparation_clang_invocations)?,
        plain_preprocessing_elapsed_us: 0,
        syntax_validation_elapsed_us: 0,
        line_marked_preprocessing_elapsed_us: 0,
    })
}

/// One analysed unit's products, stored; its ChunkIO captures go to the
/// group's postlude.
fn store_unit(
    output: String,
    products: UnitProducts,
) -> Result<(StoredUnit, ChunkioCaptures), String> {
    let UnitProducts {
        report,
        counts,
        native_results,
        teardown_results,
        captures,
    } = products;
    let parts = report.into_fragment_parts()?;
    let callback_members = parts
        .callback_members
        .iter()
        .map(|facts| {
            BinaryValue::from_json(&serde_json::to_value(facts).map_err(|e| e.to_string())?)
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok((
        StoredUnit {
            output,
            rows: values(&parts.units)?,
            diagnostic_counts: parts
                .diagnostic_counts
                .into_iter()
                .map(|(category, count)| CategoryCount {
                    category,
                    count: count as u64,
                })
                .collect(),
            bounds: values(&parts.bounds)?,
            bounds_evidence: values(&parts.bounds_evidence)?,
            null: values(&parts.null)?,
            null_evidence: values(&parts.null_evidence)?,
            callback_members,
            counts: counts_record(&counts),
            native_results: values(&native_results)?,
            teardown_results: values(&teardown_results)?,
        },
        captures,
    ))
}

/// A stored unit's products, as the fold absorbs them (its ChunkIO captures
/// were consumed by the group's postlude when the group was sealed).
pub(super) fn restore_unit(revision: &str, unit: &StoredUnit) -> Result<UnitProducts, String> {
    let callback_members = unit
        .callback_members
        .iter()
        .map(|value| {
            serde_json::from_value::<crate::callback_member_fold::UnitFacts>(value.to_json()?)
                .map_err(|error| error.to_string())
        })
        .collect::<Result<Vec<_>, String>>()?;
    let mut diagnostic_counts = BTreeMap::new();
    for entry in &unit.diagnostic_counts {
        let count = usize::try_from(entry.count).map_err(|error| error.to_string())?;
        if diagnostic_counts
            .insert(entry.category.clone(), count)
            .is_some()
        {
            return Err(format!(
                "checkpoint repeats diagnostic category `{}`",
                entry.category
            ));
        }
    }
    let report = AuditReport::from_fragment_parts(
        revision,
        FragmentParts {
            units: jsons(&unit.rows)?,
            diagnostic_counts,
            bounds: jsons(&unit.bounds)?,
            bounds_evidence: jsons(&unit.bounds_evidence)?,
            null: jsons(&unit.null)?,
            null_evidence: jsons(&unit.null_evidence)?,
            callback_members,
        },
    );
    Ok(UnitProducts {
        report,
        counts: audit_counts(&unit.counts)?,
        native_results: jsons(&unit.native_results)?,
        teardown_results: jsons(&unit.teardown_results)?,
        captures: ChunkioCaptures::default(),
    })
}

/// One unit's outcome in a stage.
enum UnitRecord {
    Done(Box<StoredUnit>),
    Failed { output: String, error: String },
}

/// One group's outcome in a stage: complete (reused, or every step and the
/// postlude succeeded), or what ran up to a failure or a stop.
struct GroupRecord {
    prelude_output: String,
    prelude: Result<(), String>,
    /// In unit order; `None` for a unit that did not run (the stage stopped).
    units: Vec<Option<UnitRecord>>,
    /// The postlude's console lines and appended teardown results, when every
    /// unit succeeded.
    finish: Option<(String, Result<Vec<BinaryValue>, String>)>,
}

impl GroupRecord {
    /// Every unit analysed without error.
    fn units_complete(&self) -> bool {
        self.prelude.is_ok()
            && self
                .units
                .iter()
                .all(|unit| matches!(unit, Some(UnitRecord::Done(_))))
    }

    /// Sealable: every step and the postlude succeeded.
    fn complete(&self) -> bool {
        self.units_complete() && matches!(&self.finish, Some((_, Ok(_))))
    }

    /// The rows of every unit that completed, in unit order.
    fn rows(&self) -> impl Iterator<Item = &BinaryValue> {
        self.units.iter().flat_map(|unit| match unit {
            Some(UnitRecord::Done(unit)) => unit.rows.as_slice(),
            _ => &[],
        })
    }

    fn bounds_evidence(&self) -> impl Iterator<Item = &BinaryValue> {
        self.units.iter().flat_map(|unit| match unit {
            Some(UnitRecord::Done(unit)) => unit.bounds_evidence.as_slice(),
            _ => &[],
        })
    }
}

// ---------------------------------------------------------------------------
// Files
// ---------------------------------------------------------------------------

/// Seal a complete group record under `identity`.
fn seal(
    path: &Path,
    identity: &PinnedIdentity,
    record: &GroupRecord,
    provenance: &PinnedProvenance,
) -> Result<String, String> {
    let units = UnitsSectionRef {
        units: record
            .units
            .iter()
            .map(|unit| match unit {
                Some(UnitRecord::Done(unit)) => Ok(unit.as_ref()),
                _ => Err("an incomplete group is never sealed".to_owned()),
            })
            .collect::<Result<_, String>>()?,
    };
    let Some((finish_output, Ok(teardown))) = &record.finish else {
        return Err("an incomplete group is never sealed".into());
    };
    if units.units.len() != identity.group.units.len() {
        return Err(format!(
            "{} unit result(s) for {} fingerprinted unit(s)",
            units.units.len(),
            identity.group.units.len()
        ));
    }
    let finish = FinishSection {
        output: finish_output.clone(),
        teardown_results: teardown.clone(),
    };
    let sections = vec![
        ("pinned-identity", encode("pinned-identity", identity)?),
        (
            "pinned-prelude",
            encode(
                "pinned-prelude",
                &PreludeSection {
                    output: record.prelude_output.clone(),
                },
            )?,
        ),
        ("pinned-units", encode("pinned-units", &units)?),
        ("pinned-finish", encode("pinned-finish", &finish)?),
        ("provenance", encode("provenance", provenance)?),
    ];
    let bytes = assemble_as(LAYOUT, sections)?;
    write_atomically(path, &bytes)?;
    let reader = CheckpointReader::open_as(path, LAYOUT)?;
    content_sha256_as(LAYOUT, reader.manifest())
}

/// The stored group at `path` when its identity equals `identity`;
/// otherwise the reason it is recomputed. Every section's digest is checked
/// and every section decoded before anything is used.
fn load(path: &Path, identity: &PinnedIdentity) -> Result<(GroupRecord, String), String> {
    if !path.exists() {
        return Err("no checkpoint".into());
    }
    let refused = |error: String| format!("checkpoint refused: {error}");
    let mut reader = CheckpointReader::open_as(path, LAYOUT).map_err(refused)?;
    let stored = reader
        .section::<PinnedIdentity>("pinned-identity")
        .map_err(refused)?;
    if let Some(field) = identity_difference(&stored, identity)? {
        return Err(format!("{field} differs"));
    }
    let prelude = reader
        .section::<PreludeSection>("pinned-prelude")
        .map_err(refused)?;
    let units = reader
        .section::<UnitsSection>("pinned-units")
        .map_err(refused)?;
    let finish = reader
        .section::<FinishSection>("pinned-finish")
        .map_err(refused)?;
    reader
        .section::<PinnedProvenance>("provenance")
        .map_err(refused)?;
    if units.units.len() != identity.group.units.len() {
        return Err(refused(format!(
            "{} stored unit(s) for {} planned",
            units.units.len(),
            identity.group.units.len()
        )));
    }
    let content = content_sha256_as(LAYOUT, reader.manifest())?;
    Ok((
        GroupRecord {
            prelude_output: prelude.output,
            prelude: Ok(()),
            units: units
                .units
                .into_iter()
                .map(|unit| Some(UnitRecord::Done(Box::new(unit))))
                .collect(),
            finish: Some((finish.output, Ok(finish.teardown_results))),
        },
        content,
    ))
}

/// The first field (as a dotted path) where two identities differ, `None`
/// when they are equal.
pub fn identity_difference(
    stored: &PinnedIdentity,
    current: &PinnedIdentity,
) -> Result<Option<String>, String> {
    if stored == current {
        return Ok(None);
    }
    let stored = serde_json::to_value(stored).map_err(|error| error.to_string())?;
    let current = serde_json::to_value(current).map_err(|error| error.to_string())?;
    fn first(path: String, left: &Value, right: &Value) -> Option<String> {
        if left == right {
            return None;
        }
        match (left, right) {
            (Value::Object(left), Value::Object(right)) => {
                for (key, value) in left {
                    match right.get(key) {
                        Some(other) => {
                            if let Some(found) = first(format!("{path}.{key}"), value, other) {
                                return Some(found);
                            }
                        }
                        None => return Some(format!("{path}.{key}")),
                    }
                }
                right
                    .keys()
                    .find(|key| !left.contains_key(*key))
                    .map(|key| format!("{path}.{key}"))
            }
            (Value::Array(left), Value::Array(right))
                if left.len() == right.len() && path.ends_with(".units") =>
            {
                left.iter()
                    .zip(right)
                    .enumerate()
                    .find(|(_, (a, b))| a != b)
                    .map(|(index, (a, _))| {
                        format!(
                            "{path}[{index}] ({})",
                            a["file"].as_str().unwrap_or_default()
                        )
                    })
            }
            _ => Some(path),
        }
    }
    Ok(first("identity".into(), &stored, &current)
        .map(|path| path.trim_start_matches("identity.").to_owned()))
}

// ---------------------------------------------------------------------------
// Identity computation
// ---------------------------------------------------------------------------

/// SHA-256 over every file under `directory`, by relative path, in sorted
/// order; a file that cannot be read refuses.
pub fn tree_sha256(directory: &Path) -> Result<String, String> {
    fn walk(root: &Path, directory: &Path, out: &mut Vec<(String, String)>) -> Result<(), String> {
        let mut entries = fs::read_dir(directory)
            .map_err(|error| format!("{}: {error}", directory.display()))?
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("{}: {error}", directory.display()))?;
        entries.sort();
        for path in entries {
            let metadata =
                fs::metadata(&path).map_err(|error| format!("{}: {error}", path.display()))?;
            if metadata.is_dir() {
                walk(root, &path, out)?;
            } else {
                let bytes =
                    fs::read(&path).map_err(|error| format!("{}: {error}", path.display()))?;
                let relative = path
                    .strip_prefix(root)
                    .map_err(|error| error.to_string())?
                    .to_string_lossy()
                    .into_owned();
                out.push((relative, sha256_hex(&bytes)));
            }
        }
        Ok(())
    }
    let mut files = Vec::new();
    walk(directory, directory, &mut files)?;
    json_sha256(&json!(files))
}

fn file_sha256(path: &Path) -> Result<String, String> {
    fs::read(path)
        .map(|bytes| sha256_hex(&bytes))
        .map_err(|error| format!("{}: {error}", path.display()))
}

/// The `QCORE_*` environment as the identity records it.
pub fn environment_identity(
    variables: impl IntoIterator<Item = (String, String)>,
    link_map_sha256: Option<&str>,
) -> Result<Vec<EnvironmentValue>, String> {
    let mut environment = Vec::new();
    for (name, value) in variables {
        if !name.starts_with("QCORE_") || name == crate::parallel::JOBS_VARIABLE {
            continue;
        }
        let value = if OUTPUT_SELECTIONS.contains(&name.as_str()) {
            "selected".to_owned()
        } else if name == crate::cross_unit_links::SELECTION {
            format!(
                "sha256:{}",
                link_map_sha256.ok_or("the link map is selected but was not loaded")?
            )
        } else {
            value
        };
        environment.push(EnvironmentValue { name, value });
    }
    environment.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(environment)
}

/// The route's inputs the checkpointed phases read.
pub(super) struct Route<'a> {
    pub plans: &'a [LibraryPlan],
    pub steps: &'a [Step<'a>],
    pub planning_failed: bool,
    pub root: &'a Path,
    pub revision: &'a str,
    pub profile: &'a LoadedProfile,
    pub contract_manifest: &'a Path,
    pub contracts: &'a ResourceContracts,
    pub pins: &'a crate::contract_revision::PinEvaluation,
    pub links: Option<&'a crate::cross_unit_links::LinkMap>,
    pub run_facts: Option<&'a Arc<qcore_c_gateway::member_run_facts::RunMemberFacts>>,
    pub source_contracts: &'a SourceContractConfiguration,
    pub provider: &'a Mutex<()>,
    pub jobs: usize,
}

fn run_identity(route: &Route<'_>) -> Result<PinnedRunIdentity, String> {
    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    let binary_sha256 = file_sha256(&executable)?;
    let mut variables = Vec::new();
    for (name, value) in std::env::vars_os() {
        let Some(name) = name.to_str() else { continue };
        if !name.starts_with("QCORE_") {
            continue;
        }
        let value = value.into_string().map_err(|_| {
            format!("environment variable {name} is not UTF-8; checkpoints need it exactly")
        })?;
        variables.push((name.to_owned(), value));
    }
    let environment =
        environment_identity(variables, route.links.map(|links| links.digest.as_str()))?;
    let configuration = std::env::var_os("QCORE_SOURCE_CONTRACTS")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            quirin_source_analysis::source_profiles::profiles_root()
                .join(quirin_source_analysis::source_profiles::FLUENT_BIT_SOURCE_SET)
                .join("source-contracts.json")
        });
    let package = std::env::var_os("QCORE_CHUNKIO_PACKAGE")
        .map(PathBuf::from)
        .map(|package| {
            let package = if package.is_absolute() {
                package
            } else {
                std::env::current_dir()
                    .map_err(|error| error.to_string())?
                    .join(package)
            };
            Ok::<_, String>(TreeIdentity {
                path: package.display().to_string(),
                sha256: tree_sha256(&package)?,
            })
        })
        .transpose()?;
    let compiler = qcore_c_gateway::ClangIdentity::capture()?;
    let template = crate::checkpoint::exact_json(
        &serde_json::to_value(crate::contract_revision::template(
            route.contracts,
            route.pins,
        ))
        .map_err(|error| error.to_string())?,
    )?;
    Ok(PinnedRunIdentity {
        route: "pinned".into(),
        analyzer: AnalyzerIdentity {
            package: env!("CARGO_PKG_NAME").into(),
            version: env!("CARGO_PKG_VERSION").into(),
            build_profile: if cfg!(debug_assertions) {
                "dev"
            } else {
                "release"
            }
            .into(),
            build_commit: option_env!("QCORE_AUDIT_BUILD_COMMIT").map(str::to_owned),
            binary_sha256,
        },
        fluent_bit_root: route.root.display().to_string(),
        fluent_bit_revision: route.revision.to_owned(),
        working_directory: std::env::current_dir()
            .map_err(|error| error.to_string())?
            .display()
            .to_string(),
        source_profile_sha256: tree_sha256(route.profile.directory())?,
        source_contracts: SourceContractsIdentity {
            configuration: configuration.display().to_string(),
            configuration_sha256: file_sha256(&configuration)?,
            package,
        },
        contract_manifest: route.contract_manifest.display().to_string(),
        contract_manifest_sha256: file_sha256(route.contract_manifest)?,
        locks: route
            .pins
            .lock_digests()
            .into_iter()
            .map(|(path, sha256)| LockDigest { path, sha256 })
            .collect(),
        pin_evaluation_sha256: sha256_hex(template.as_bytes()),
        compiler: CompilerIdentity {
            program: compiler.program().to_owned(),
            version: compiler.version().to_owned(),
            target: compiler.target().to_owned(),
        },
        environment,
        member_facts_sha256: route
            .run_facts
            .map(|facts| crate::member_prepass::facts_sha256(facts))
            .transpose()?,
    })
}

/// SHA-256 of a contract environment, as the cross-unit home digest takes it.
pub fn environment_sha256(environment: &ResourceContracts) -> Result<String, String> {
    serde_json::to_vec(environment)
        .map(|bytes| sha256_hex(&bytes))
        .map_err(|error| error.to_string())
}

/// SHA-256 of the cross-unit homes supplied to every unit.
pub fn homes_sha256(homes: &[CrossUnitHomeEvidence]) -> Result<String, String> {
    json_sha256(&Value::Array(
        homes
            .iter()
            .map(|home| {
                Ok(json!({
                    "function": home.function,
                    "import": serde_json::to_value(home.import_identity())
                        .map_err(|error| error.to_string())?,
                    "line_marked_sha256": sha256_hex(home.line_marked_source.as_bytes()),
                    "dialect": format!("{:?}", home.dialect),
                    "link_resolution_sha256": home.link_resolution_sha256,
                }))
            })
            .collect::<Result<_, String>>()?,
    ))
}

fn bounds_environment_sha256() -> Result<String, String> {
    json_sha256(&json!({
        "bounds": crate::bounds_selection::environment().digest(),
        "null": crate::bounds_selection::null_environment().digest(),
        "record": crate::bounds_selection::installed_record(),
    }))
}

/// The stages, in the order the route runs them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    First,
    Refuted,
    Imported,
}

impl Stage {
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::First => "first",
            Self::Refuted => "refuted",
            Self::Imported => "imported",
        }
    }
}

/// A stage's identity.
pub fn stage_identity(
    stage: Stage,
    environment: &ResourceContracts,
    homes: &[CrossUnitHomeEvidence],
    bounds_environment_sha256: String,
) -> Result<StageIdentity, String> {
    Ok(StageIdentity {
        stage: stage.name().into(),
        environment_sha256: environment_sha256(environment)?,
        homes_sha256: homes_sha256(homes)?,
        bounds_environment_sha256,
    })
}

/// One unit's fingerprint: three Clang runs, no analysis.
fn fingerprint_unit(entry: &CompileEntry) -> Result<PinnedUnitFingerprint, String> {
    let source = fs::read_to_string(&entry.file)
        .map_err(|error| format!("could not read {}: {error}", entry.file.display()))?;
    let options = compiler_options(entry)?;
    let unit = C11TranslationUnit {
        source: &source,
        include_paths: options.include_paths,
        definitions: options.definitions,
        preprocessor_options: options.preprocessor_options,
        dialect: options.dialect,
        function_scope: LegacyFunctionScope::MainSourceDefinitions,
        input: options.input,
    };
    let prepared = prepare_translation_unit(&unit);
    Ok(PinnedUnitFingerprint {
        file: entry.file.display().to_string(),
        record_sha256: json_sha256(&json!([entry.file, entry.directory, entry.command]))?,
        raw_source_sha256: sha256_hex(source.as_bytes()),
        preprocessed_sha256: prepared
            .preprocessed_source()
            .ok()
            .map(|text| sha256_hex(text.as_bytes())),
        preparation: PreparationFacts::of(&prepared),
    })
}

/// Every group's identity, or why a unit of it could not be fingerprinted
/// (the group is then recomputed and never sealed).
fn group_identities(
    plans: &[LibraryPlan],
    jobs: usize,
) -> Vec<Result<PinnedGroupIdentity, String>> {
    let tasks = plans
        .iter()
        .enumerate()
        .flat_map(|(group, plan)| plan.selected.iter().map(move |entry| (group, entry)))
        .collect::<Vec<_>>();
    let weights = tasks
        .iter()
        .map(|(_, entry)| source_weight(entry))
        .collect::<Vec<_>>();
    let mut units = plans
        .iter()
        .map(|_| Ok(Vec::new()))
        .collect::<Vec<Result<Vec<PinnedUnitFingerprint>, String>>>();
    let _ = run_ordered(
        tasks,
        jobs,
        Some(heaviest_first(&weights)),
        |(group, entry)| (group, fingerprint_unit(entry)),
        |(group, fingerprint)| {
            let slot = &mut units[group];
            match (slot.as_mut(), fingerprint) {
                (Ok(list), Ok(fingerprint)) => list.push(fingerprint),
                (Ok(_), Err(error)) => *slot = Err(error),
                (Err(_), _) => {}
            }
            Ok::<(), ()>(())
        },
    );
    plans
        .iter()
        .zip(units)
        .map(|(plan, units)| {
            Ok(PinnedGroupIdentity {
                label: plan.library.clone(),
                source_root: plan.source_root.display().to_string(),
                units: units?,
            })
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Stages
// ---------------------------------------------------------------------------

/// One unit's deposit in its group's collector.
enum Slot {
    Done(Box<StoredUnit>, Box<ChunkioCaptures>),
    Failed { output: String, error: String },
}

/// A computed group's results as its steps complete, in any order; the
/// worker that completes the group runs its postlude and seals it.
struct Collector {
    remaining: usize,
    prelude: Option<(String, Result<(), String>)>,
    units: Vec<Option<Slot>>,
    record: Option<GroupRecord>,
    sealed: Option<Result<String, String>>,
}

/// The group's record from what its collector holds; with `finish`, its
/// postlude runs when every step succeeded.
fn assemble_record(label: &str, collector: &mut Collector, finish: bool) -> GroupRecord {
    let (prelude_output, prelude) = collector.prelude.take().unwrap_or_else(|| {
        (
            String::new(),
            Err(format!("{label}: library prelude did not run")),
        )
    });
    let mut captures = ChunkioCaptures::default();
    let mut every_unit = prelude.is_ok();
    let units = collector
        .units
        .iter_mut()
        .map(|slot| match slot.take() {
            Some(Slot::Done(unit, captured)) => {
                captures.absorb(*captured);
                Some(UnitRecord::Done(unit))
            }
            Some(Slot::Failed { output, error }) => {
                every_unit = false;
                Some(UnitRecord::Failed { output, error })
            }
            None => {
                every_unit = false;
                None
            }
        })
        .collect();
    let finish = (finish && every_unit).then(|| {
        let mut output = String::new();
        let mut teardown = Vec::new();
        let result = finish_library(label, captures, &mut teardown, &mut output)
            .and_then(|()| values(&teardown));
        (output, result)
    });
    GroupRecord {
        prelude_output,
        prelude,
        units,
        finish,
    }
}

enum Task {
    Prelude(usize),
    Unit(usize, usize),
}

/// What the phases share across stages: each library's prelude runs at most
/// once, in the first stage that recomputes the group.
struct Shared<'a> {
    route: &'a Route<'a>,
    options: &'a Options,
    run: PinnedRunIdentity,
    groups: Vec<Result<PinnedGroupIdentity, String>>,
    contexts: Vec<OnceLock<Option<LibraryContext>>>,
    preludes: Vec<OnceLock<(String, Result<(), String>)>>,
    records: Mutex<Vec<Value>>,
}

struct StageResult {
    records: Vec<GroupRecord>,
    /// A prelude or unit failed (the route's stop rule).
    stopped: bool,
}

fn checkpoint_path(directory: &Path, label: &str, stage: Stage) -> PathBuf {
    directory.join(format!("{label}.{}.qo", stage.name()))
}

fn run_stage(
    shared: &Shared<'_>,
    stage: Stage,
    environment: &ResourceContracts,
    homes: &[CrossUnitHomeEvidence],
) -> Result<StageResult, String> {
    let route = shared.route;
    let started = Instant::now();
    let stage_identity = stage_identity(stage, environment, homes, bounds_environment_sha256()?)?;
    let identities = shared
        .groups
        .iter()
        .map(|group| {
            group.as_ref().map(|group| PinnedIdentity {
                run: shared.run.clone(),
                stage: stage_identity.clone(),
                group: group.clone(),
            })
        })
        .collect::<Vec<_>>();
    // Decide every group first: reused from its file, or recomputed.
    let mut reused = Vec::new();
    for (index, plan) in route.plans.iter().enumerate() {
        let path = checkpoint_path(&shared.options.directory, &plan.library, stage);
        let decision = match &identities[index] {
            Err(error) => Err(format!("unit fingerprint unavailable: {error}")),
            Ok(_) if !shared.options.resume => Err("resume not requested".to_owned()),
            Ok(identity) => load(&path, identity),
        };
        let label = format!("{}.{}", plan.library, stage.name());
        match decision {
            Ok((record, content)) => {
                eprintln!("checkpoint {label}: reused (identity equal)");
                lock(&shared.records).push(json!({
                    "group": plan.library, "stage": stage.name(), "decision": "reused",
                    "reason": "identity equal", "content_sha256": content,
                }));
                reused.push(Some(record));
            }
            Err(reason) => {
                eprintln!("checkpoint {label}: recomputed ({reason})");
                lock(&shared.records).push(json!({
                    "group": plan.library, "stage": stage.name(), "decision": "recomputed",
                    "reason": reason,
                }));
                reused.push(None);
            }
        }
    }
    let mut tasks = Vec::new();
    let mut weights = Vec::new();
    let collectors = route
        .plans
        .iter()
        .enumerate()
        .map(|(index, plan)| {
            if reused[index].is_some() {
                return Mutex::new(None);
            }
            let prelude = shared.preludes[index].get().cloned();
            if prelude.is_none() {
                tasks.push(Task::Prelude(index));
                weights.push(u64::MAX);
            }
            for (unit, entry) in plan.selected.iter().enumerate() {
                tasks.push(Task::Unit(index, unit));
                weights.push(source_weight(entry));
            }
            Mutex::new(Some(Collector {
                remaining: plan.selected.len() + usize::from(prelude.is_none()),
                prelude,
                units: plan.selected.iter().map(|_| None).collect(),
                record: None,
                sealed: None,
            }))
        })
        .collect::<Vec<_>>();
    // A completed group's postlude and seal, by the worker that completed it.
    let finalize = |index: usize, collector: &mut Collector| {
        let label = &route.plans[index].library;
        let record = assemble_record(label, collector, true);
        if record.complete()
            && let Ok(identity) = &identities[index]
        {
            let path = checkpoint_path(&shared.options.directory, label, stage);
            let provenance = PinnedProvenance {
                wall_time_ms: started.elapsed().as_millis() as u64,
                jobs: route.jobs as u64,
                process_peak_rss_kib: status_kib("VmHWM:"),
            };
            let sealed = seal(&path, identity, &record, &provenance);
            if sealed.is_ok() {
                eprintln!("checkpoint ready: {}", path.display());
            }
            collector.sealed = Some(sealed);
        }
        collector.record = Some(record);
    };
    let complete = |index: usize, collector: &mut Collector| {
        collector.remaining -= 1;
        if collector.remaining == 0 {
            finalize(index, collector);
        }
    };
    // A recomputed group with no task in this stage (its prelude ran in an
    // earlier stage and it selects no unit) completes now.
    for (index, collector) in collectors.iter().enumerate() {
        if let Some(collector) = lock(collector).as_mut()
            && collector.remaining == 0
        {
            finalize(index, collector);
        }
    }
    let work = |task: Task| -> bool {
        match task {
            Task::Prelude(index) => {
                let plan = &route.plans[index];
                let mut output = String::new();
                let publish = PublishOnDrop(&shared.contexts[index]);
                let result = prepare_library(plan, route.source_contracts, &mut output)
                    .map(|context| publish.publish(context));
                let failed = result.is_err();
                let _ = shared.preludes[index].set((output.clone(), result.clone()));
                let mut slot = lock(&collectors[index]);
                if let Some(collector) = slot.as_mut() {
                    collector.prelude = Some((output, result));
                    complete(index, collector);
                }
                failed
            }
            Task::Unit(index, unit) => {
                #[cfg(all(feature = "unit-timing", target_os = "linux"))]
                let _timing =
                    crate::unit_timing::Span::start(&route.plans[index].selected[unit].file);
                let plan = &route.plans[index];
                let entry = &plan.selected[unit];
                let mut output = String::new();
                let result = match shared.contexts[index].wait() {
                    Some(context) => {
                        qcore_c_gateway::member_run_facts::scoped(route.run_facts.cloned(), || {
                            audit_unit(
                                plan,
                                context,
                                entry,
                                route.revision,
                                environment,
                                homes,
                                route.source_contracts,
                                route.provider,
                                &mut output,
                            )
                        })
                    }
                    None => Err(format!("{}: library prelude failed", plan.library)),
                };
                let deposit = match result {
                    Ok(products) => match store_unit(output.clone(), products) {
                        Ok((stored, captures)) => Slot::Done(Box::new(stored), Box::new(captures)),
                        Err(error) => Slot::Failed { output, error },
                    },
                    Err(error) => Slot::Failed { output, error },
                };
                let failed = matches!(deposit, Slot::Failed { .. });
                let mut slot = lock(&collectors[index]);
                if let Some(collector) = slot.as_mut() {
                    collector.units[unit] = Some(deposit);
                    complete(index, collector);
                }
                failed
            }
        }
    };
    let stopped = run_ordered(
        tasks,
        route.jobs,
        Some(heaviest_first(&weights)),
        work,
        |failed| if failed { Err(()) } else { Ok(()) },
    )
    .is_err();
    let mut records = Vec::with_capacity(route.plans.len());
    for (index, (stored, collector)) in reused.into_iter().zip(collectors).enumerate() {
        if let Some(record) = stored {
            records.push(record);
            continue;
        }
        let mut collector = collector
            .into_inner()
            .unwrap_or_else(PoisonError::into_inner)
            .ok_or("a recomputed group has a collector")?;
        if let Some(Err(error)) = &collector.sealed {
            return Err(format!(
                "could not seal {}: {error}",
                checkpoint_path(
                    &shared.options.directory,
                    &route.plans[index].library,
                    stage
                )
                .display()
            ));
        }
        if let Some(Ok(content)) = &collector.sealed {
            lock(&shared.records).push(json!({
                "group": route.plans[index].library, "stage": stage.name(), "decision": "sealed",
                "content_sha256": content,
            }));
        }
        let record = match collector.record.take() {
            Some(record) => record,
            None => assemble_record(&route.plans[index].library, &mut collector, false),
        };
        records.push(record);
    }
    lock(&shared.records).push(json!({
        "stage": stage.name(), "wall_time_ms": started.elapsed().as_millis() as u64,
        "stopped": stopped,
    }));
    Ok(StageResult { records, stopped })
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The refutation table of every completed unit's rows, one row restored at
/// a time (the table is a duplicate-free union over rows, so the order and
/// grouping of its insertions do not matter).
fn refutation_table_of(
    records: &[GroupRecord],
) -> Result<qcore_schemas::contract_refutation::RefutationTable, String> {
    let mut table = qcore_schemas::contract_refutation::RefutationTable::new();
    for row in records.iter().flat_map(GroupRecord::rows) {
        let row = row.to_json()?;
        for (function, refutations) in crate::refuted_contracts::table_from_rows([&row]) {
            for refutation in refutations {
                qcore_schemas::contract_refutation::insert_refutation(
                    &mut table, &function, refutation,
                );
            }
        }
    }
    Ok(table)
}

/// The bounds refutation table of every completed unit's body evidence, one
/// record restored at a time (a set union).
fn bounds_refutation_table_of(
    records: &[GroupRecord],
) -> Result<std::collections::BTreeSet<String>, String> {
    let mut table = std::collections::BTreeSet::new();
    for evidence in records.iter().flat_map(GroupRecord::bounds_evidence) {
        table.extend(crate::bounds_evidence::refutation_table(&[
            evidence.to_json()?
        ]));
    }
    Ok(table)
}

/// The checkpointed phases: the same stop rule, refutation table and
/// cross-unit import as the whole-run phases, stage by stage, each library
/// group reused or recomputed; then the step outcomes the shared fold
/// consumes, in step order.
pub(super) fn run_phases<'a>(
    route: &'a Route<'a>,
    options: &'a Options,
) -> Result<
    (
        Vec<StepOutcome>,
        qcore_schemas::contract_refutation::RefutationTable,
        Option<Value>,
    ),
    String,
> {
    let started = Instant::now();
    fs::create_dir_all(&options.directory)
        .map_err(|error| format!("{}: {error}", options.directory.display()))?;
    let run = run_identity(route)?;
    let fingerprinting = Instant::now();
    let groups = group_identities(route.plans, route.jobs);
    let fingerprint_ms = fingerprinting.elapsed().as_millis() as u64;
    let shared = Shared {
        route,
        options,
        run,
        groups,
        contexts: route.plans.iter().map(|_| OnceLock::new()).collect(),
        preludes: route.plans.iter().map(|_| OnceLock::new()).collect(),
        records: Mutex::new(Vec::new()),
    };
    let first = run_stage(&shared, Stage::First, route.contracts, &[])?;
    let mut stopped = first.stopped || route.planning_failed;
    let mut records = first.records;
    let found = if stopped {
        qcore_schemas::contract_refutation::RefutationTable::new()
    } else {
        refutation_table_of(&records)?
    };
    let bounds_found = if stopped {
        Default::default()
    } else {
        bounds_refutation_table_of(&records)?
    };
    let applied = if stopped || (found.is_empty() && bounds_found.is_empty()) {
        qcore_schemas::contract_refutation::RefutationTable::new()
    } else {
        crate::bounds_selection::apply_refutation_table(&bounds_found);
        let held = crate::refuted_contracts::with_table(route.contracts, &found);
        let second = run_stage(&shared, Stage::Refuted, &held, &[])?;
        stopped = second.stopped;
        records = second.records;
        found
    };
    let mut premise = None;
    if let Some(links) = route.links
        && !stopped
    {
        let held = crate::refuted_contracts::with_table(route.contracts, &applied);
        let (imported, homes) = select_homes(links, route.plans, &held)?;
        if !homes.is_empty() {
            let third = run_stage(&shared, Stage::Imported, &imported, &homes)?;
            let complete = !third.stopped && third.records.iter().all(GroupRecord::units_complete);
            if complete && refutation_table_of(&third.records)? == applied {
                records = third.records;
                premise = Some(cross_unit_premise_of(links, &homes));
            } else {
                println!(
                    "cross-unit import phase refused: incomplete run or changed refutation table"
                );
            }
        }
    }
    let record = json!({
        "schema_version": 1,
        "scope": "provenance of one checkpointed pinned run: per stage, which library groups were reused or recomputed and why, the content digest of each file sealed or reused, and measured wall times; not an input to any verdict",
        "jobs": route.jobs,
        "fingerprint_wall_time_ms": fingerprint_ms,
        "phases_wall_time_ms": started.elapsed().as_millis() as u64,
        "process_peak_rss_kib": status_kib("VmHWM:"),
        "groups": shared.records.into_inner().unwrap_or_else(PoisonError::into_inner),
    });
    let mut bytes = serde_json::to_vec_pretty(&record).map_err(|error| error.to_string())?;
    bytes.push(b'\n');
    let path = options.directory.join(RUN_RECORD);
    fs::write(&path, bytes).map_err(|error| format!("{}: {error}", path.display()))?;
    Ok((outcomes(route.steps, records), applied, premise))
}

/// The step outcomes of the final records, in step order, up to the first
/// failure (where the whole-run fold stops too).
fn outcomes(steps: &[Step<'_>], records: Vec<GroupRecord>) -> Vec<StepOutcome> {
    let mut groups = records
        .into_iter()
        .map(|record| {
            (
                Some((record.prelude_output, record.prelude)),
                record.units.into_iter(),
                record.finish,
            )
        })
        .collect::<Vec<_>>();
    let mut outcomes = Vec::new();
    for step in steps {
        match step {
            Step::Prelude(index) => {
                let Some((output, result)) = groups[*index].0.take() else {
                    break;
                };
                let failed = result.is_err();
                outcomes.push(StepOutcome::Prelude { output, result });
                if failed {
                    break;
                }
            }
            Step::Unit(index, _) => match groups[*index].1.next().flatten() {
                Some(UnitRecord::Done(unit)) => {
                    outcomes.push(StepOutcome::StoredUnit(unit));
                }
                Some(UnitRecord::Failed { output, error }) => {
                    outcomes.push(StepOutcome::Unit {
                        output,
                        result: Box::new(Err(error)),
                    });
                    break;
                }
                // Unreachable: a stage runs every step before its first
                // failure. Fail closed rather than end the fold silently.
                None => {
                    outcomes.push(StepOutcome::Unit {
                        output: String::new(),
                        result: Box::new(Err(
                            "checkpointed stage holds no result for a planned unit".into(),
                        )),
                    });
                    break;
                }
            },
            Step::Finish(index) => {
                let Some((output, result)) = groups[*index].2.take() else {
                    outcomes.push(StepOutcome::Finished {
                        index: *index,
                        output: String::new(),
                        result: Err("checkpointed stage holds no postlude result".into()),
                    });
                    break;
                };
                outcomes.push(StepOutcome::Finished {
                    index: *index,
                    output,
                    result: result.and_then(|teardown| jsons(&teardown)),
                });
            }
            Step::MonkeyGate => outcomes.push(StepOutcome::MonkeyGate),
            Step::PlanningFailed(error) => {
                outcomes.push(StepOutcome::PlanningFailed(error.clone()));
                break;
            }
        }
    }
    outcomes
}

#[cfg(test)]
mod tests;

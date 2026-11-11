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

//! Card **W45-AUDIT-CHECKPOINTS**: the `.qo` ("Quirin-audit Object") per-configuration
//! audit checkpoint — its binary format, its sections and their text
//! decomposition.
//!
//! A checkpointed `quirin-audit current` run seals each source/compile configuration
//! independently into `<group-label>-<unit-key-digest>.qo`. A file
//! holds ten sections in a fixed order, each encoded separately with
//! [postcard] (a specified, stable serde wire format) and separately digested
//! with SHA-256:
//!
//! | section        | step (pipeline order)                                        | identity | composed |
//! |----------------|--------------------------------------------------------------|----------|----------|
//! | `identity`     | every input of the unit result                            | the key  | checked  |
//! | `plan`         | the run's ordered unit checkpoints                             | no       | checked  |
//! | `global`       | run-wide publication inputs (revision, assumptions, markers) | no       | used     |
//! | `preparation`  | per unit: compile configuration and task identity, enumeration, original coordinates | no | used |
//! | `lowering`     | per unit: refusal reason and code, source functions, not projected | no | used |
//! | `verdicts`     | per unit: verdict, per-function summaries/verdicts/diagnostics, tallies | no | used |
//! | `certificates` | per unit: the Safe certificate's binding outcome (derived)    | no       | no       |
//! | `contracts`    | the contract set in force and the unit's held calls          | no       | used     |
//! | `result`       | the unit's final result: console lines, counts, sources      | no       | used     |
//! | `provenance`   | wall time, peak memory, preparation timings                   | no       | console only |
//!
//! The format is specified in `docs/development/audit-checkpoints.md`. Every
//! section except `provenance` is a deterministic function of the run's
//! inputs, so two runs over the same inputs — at any job count — agree on the
//! [content digest](Checkpoint::content_sha256). A file that is truncated,
//! tampered with, or of another format or schema version is refused whole;
//! no section of it is ever used.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use qcore_schemas::sha256_hex;
use qcore_schemas::verdict::Verdict;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The checkpoint file extension.
pub const EXTENSION: &str = "qo";
/// The binary container's format version, spelled in the magic header.
pub const FORMAT_VERSION: u16 = 1;
/// The version of the section schemas this build writes and reads.
// Card W45-REFUTED-CONTRACT-WITHDRAWAL: schema 2 adds the refutation
// table's digest to the contract set of every unit's identity.
// Card W47-REVIEW-HOLD-CONCRETE-REASON: schema 4 adds each pending-review
// marker's category to the global section.
// Card MEMBER-PREPASS-OTHER-ROUTES: schema 5 carries the run's member facts
// (rule 12n): their digest in the run identity, the fold's environment and the
// run's closure of each invoked member in the global section, and each unit's
// recorded member and state facts in its contracts section.
pub const SCHEMA_VERSION: u32 = 5;
/// The sections of a schema-1 file, in file order.
pub const SECTIONS: [&str; 10] = [
    "identity",
    "plan",
    "global",
    "preparation",
    "lowering",
    "verdicts",
    "certificates",
    "contracts",
    "result",
    "provenance",
];
/// The one section outside the content digest: measurements of the run that
/// produced the file, never an input to identity or to a composed verdict.
pub const PROVENANCE: &str = "provenance";

/// One kind of checkpoint the container carries: its schema version and its
/// sections in file order (card CORPUS-GROUP-CHECKPOINTS). A reader of one
/// layout refuses a file of another: the header's schema version and section
/// list must both be the layout's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Layout {
    pub schema_version: u32,
    pub sections: &'static [&'static str],
}

/// The `quirin-audit current` unit checkpoint (schema [`SCHEMA_VERSION`],
/// [`SECTIONS`]).
pub const CURRENT_LAYOUT: Layout = Layout {
    schema_version: SCHEMA_VERSION,
    sections: &SECTIONS,
};

/// Magic `QO`, then the format version (big-endian `u16`), then a line-ending
/// and end-of-file guard that text-mode transfers would corrupt.
const GUARD: [u8; 4] = *b"\r\n\x1a\n";
const PRELUDE_LEN: usize = 8;
const MAX_HEADER_LEN: u32 = 1 << 20;

fn prelude() -> [u8; PRELUDE_LEN] {
    let version = FORMAT_VERSION.to_be_bytes();
    [
        b'Q', b'O', version[0], version[1], GUARD[0], GUARD[1], GUARD[2], GUARD[3],
    ]
}

/// One manifest entry: a section's name, its byte length and its SHA-256.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SectionEntry {
    pub name: String,
    pub length: u64,
    pub sha256: [u8; 32],
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Header {
    schema_version: u32,
    sections: Vec<SectionEntry>,
}

fn sha256_bytes(bytes: &[u8]) -> [u8; 32] {
    let hex = sha256_hex(bytes);
    let mut digest = [0u8; 32];
    for (index, byte) in digest.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&hex[index * 2..index * 2 + 2], 16)
            .expect("sha256_hex returns lowercase hex");
    }
    digest
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn unhex(text: &str) -> Result<[u8; 32], String> {
    if text.len() != 64 || !text.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(format!("`{text}` is not a SHA-256 hex digest"));
    }
    let mut digest = [0u8; 32];
    for (index, byte) in digest.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&text[index * 2..index * 2 + 2], 16)
            .map_err(|error| error.to_string())?;
    }
    Ok(digest)
}

// ---------------------------------------------------------------------------
// Section schemas (schema version 4): typed binary recursive values and
// closed numeric status/verdict codes, with optional YAML/TOML inspection.
// ---------------------------------------------------------------------------

/// Every input the unit result depends on. A resumed run reuses a unit
/// only when this whole section is equal.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IdentitySection {
    pub run: RunIdentity,
    pub group: GroupIdentity,
}

/// The run-wide inputs every unit shares.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunIdentity {
    /// The route that produced the checkpoint (`current`).
    pub route: String,
    pub analyzer: AnalyzerIdentity,
    /// `configured` or `contracts-only`.
    pub mode: String,
    /// The canonical checkout path.
    pub checkout: String,
    pub source_revision: String,
    pub source_profile_sha256: String,
    pub contracts: ContractSetIdentity,
    pub compiler: CompilerIdentity,
    /// Every `QCORE_*` variable set for the run except the job count, sorted
    /// by name; an unset variable is absent.
    pub environment: Vec<EnvironmentValue>,
    /// Card MEMBER-PREPASS-OTHER-ROUTES (schema 5): SHA-256 of the run's
    /// member facts the pre-pass read (rule 12n), when it ran. Every unit
    /// decided its member invocations over them, so a changed run set
    /// recomputes every unit and never composes with another.
    pub member_facts_sha256: Option<String>,
}

/// The analyzer build.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnalyzerIdentity {
    pub package: String,
    pub version: String,
    /// `dev` or `release`.
    pub build_profile: String,
    /// The source tree's `HEAD` when the binary was built, when known.
    pub build_commit: Option<String>,
    /// SHA-256 of the running executable's bytes.
    pub binary_sha256: String,
}

/// The contract set in force.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContractSetIdentity {
    pub manifest_sha256: String,
    /// SHA-256 of the elaborated sources' assumption record.
    pub sources_sha256: String,
    pub provider_selection_sha256: Option<String>,
    pub evidence_sha256: Option<String>,
    /// Every contract lock (`contracts.lock`, card W45-CONTRACT-REVISION-MODE)
    /// the pin evaluation read, by path, with its file's SHA-256: a re-pin
    /// changes the identity.
    pub locks: Vec<LockDigest>,
    /// SHA-256 of the pin evaluation's section template (which contracts are
    /// pinned, in force or withheld, over which reviewed sites).
    pub pin_evaluation_sha256: String,
    /// SHA-256 of the checker-plugin section (card W45-USER-CONTRACT-PLUGINS:
    /// each binding's package digest, task digest and classified verdict), when
    /// a checker ran. A plugin's verdict enters every function that applies its
    /// contract, so it is an input of every unit.
    pub plugin_checks_sha256: Option<String>,
    /// SHA-256 of the refutation table the unit was analysed under
    /// (card W45-REFUTED-CONTRACT-WITHDRAWAL, contract spec C06.4): the empty
    /// table's digest in a first phase, the whole run's table in a second. A
    /// changed table recomputes every unit, and units analysed under
    /// different tables never compose.
    pub refuted_table_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LockDigest {
    pub path: String,
    pub sha256: String,
}

/// The Clang every unit was prepared with.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompilerIdentity {
    pub program: String,
    pub version: String,
    pub target: String,
}

/// One environment variable.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnvironmentValue {
    pub name: String,
    pub value: String,
}

/// The unit checkpoint's own inputs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GroupIdentity {
    pub label: String,
    /// The group-wide refusal every unit inherits, if any.
    pub unresolved: Option<String>,
    pub units: Vec<UnitFingerprint>,
}

/// One unit's inputs: its published compile configuration (which carries the
/// compile-task identity: command, flags, compiler, raw and preprocessed
/// digests, dependencies) and the preparation facts the configuration does
/// not carry.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnitFingerprint {
    pub file: String,
    /// SHA-256 of the compact JSON of the unit's `compile_configuration`.
    pub configuration_sha256: String,
    pub preparation: PreparationFacts,
}

/// What preparation established beyond the compile-task identity.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreparationFacts {
    /// The unit was refused before Clang ran.
    #[default]
    NotPrepared,
    Prepared {
        validation_error: Option<String>,
        line_marked_sha256: Option<String>,
        line_marked_error: Option<String>,
    },
}

impl PreparationFacts {
    /// The facts of one prepared translation unit.
    #[must_use]
    pub fn of(prepared: &qcore_c_gateway::legacy_c::PreparedTranslationUnit) -> Self {
        Self::Prepared {
            validation_error: prepared.validation_error().map(ToString::to_string),
            line_marked_sha256: prepared
                .line_marked_source()
                .map(|text| sha256_hex(text.as_bytes())),
            line_marked_error: prepared.line_marked_error().map(ToString::to_string),
        }
    }
}

/// The run's plan: every checkpoint unit, in order, with its key.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanSection {
    /// This file's position in `groups`.
    pub group: u64,
    pub groups: Vec<PlannedGroup>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlannedGroup {
    pub label: String,
    /// The library header for its first unit, empty on later units.
    pub header: String,
    pub units: Vec<UnitKey>,
}

/// A unit's identity in the plan: the checkout source it stands for and a
/// digest of its compile record (file, directory, command), or of its absence.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnitKey {
    pub expected_source: String,
    pub record_sha256: String,
}

/// Common row status spellings encoded as one-byte postcard discriminants.
/// Unknown future spellings remain strings and keep their fail-closed meaning.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum StatusCode {
    Safe,
    Pending,
    Unknown,
    Unsafe,
    DataOnly,
    GatewayFailure,
}

impl StatusCode {
    fn of(value: &str) -> Option<Self> {
        Some(match value {
            "Safe" => Self::Safe,
            "Pending" => Self::Pending,
            "Unknown" => Self::Unknown,
            "Unsafe" => Self::Unsafe,
            "data_only" => Self::DataOnly,
            "gateway_failure" => Self::GatewayFailure,
            _ => return None,
        })
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Safe => "Safe",
            Self::Pending => "Pending",
            Self::Unknown => "Unknown",
            Self::Unsafe => "Unsafe",
            Self::DataOnly => "data_only",
            Self::GatewayFailure => "gateway_failure",
        }
    }
}

/// A JSON value encoded as postcard tags and native numeric payloads, never
/// as an embedded JSON document. Float bits retain negative zero and adjacent
/// finite values exactly; names and messages remain UTF-8 strings.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BinaryValue {
    Null,
    Bool(bool),
    I64(i64),
    U64(u64),
    F64Bits(u64),
    Status(StatusCode),
    String(String),
    Array(Vec<Self>),
    Object(Vec<(String, Self)>),
}

impl BinaryValue {
    pub fn from_json(value: &Value) -> Result<Self, String> {
        Ok(match value {
            Value::Null => Self::Null,
            Value::Bool(value) => Self::Bool(*value),
            Value::Number(number) => {
                if number.is_f64() {
                    Self::F64Bits(
                        number
                            .as_f64()
                            .ok_or("JSON number is not representable as a finite f64")?
                            .to_bits(),
                    )
                } else if let Some(value) = number.as_i64() {
                    Self::I64(value)
                } else {
                    Self::U64(number.as_u64().ok_or("JSON integer exceeds u64")?)
                }
            }
            Value::String(value) => StatusCode::of(value)
                .map(Self::Status)
                .unwrap_or_else(|| Self::String(value.clone())),
            Value::Array(values) => Self::Array(
                values
                    .iter()
                    .map(Self::from_json)
                    .collect::<Result<_, _>>()?,
            ),
            Value::Object(values) => Self::Object(
                values
                    .iter()
                    .map(|(key, value)| Ok((key.clone(), Self::from_json(value)?)))
                    .collect::<Result<_, String>>()?,
            ),
        })
    }

    pub fn to_json(&self) -> Result<Value, String> {
        Ok(match self {
            Self::Null => Value::Null,
            Self::Bool(value) => Value::Bool(*value),
            Self::I64(value) => Value::Number((*value).into()),
            Self::U64(value) => Value::Number((*value).into()),
            Self::F64Bits(bits) => Value::Number(
                serde_json::Number::from_f64(f64::from_bits(*bits))
                    .ok_or("checkpoint contains a non-finite float")?,
            ),
            Self::Status(status) => Value::String(status.as_str().to_owned()),
            Self::String(value) => Value::String(value.clone()),
            Self::Array(values) => {
                Value::Array(values.iter().map(Self::to_json).collect::<Result<_, _>>()?)
            }
            Self::Object(values) => {
                let mut object = serde_json::Map::new();
                for (key, value) in values {
                    if object.insert(key.clone(), value.to_json()?).is_some() {
                        return Err(format!("checkpoint object repeats key `{key}`"));
                    }
                }
                Value::Object(object)
            }
        })
    }
}

/// Run-wide publication inputs. Every unit of one run carries the same.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GlobalSection {
    /// The console lines before the first group header.
    pub console_preamble: Vec<String>,
    pub source_revision: String,
    /// The report's revision key (`fluent_bit_revision`).
    pub revision_key: String,
    /// The report's `evidence` record (a contracts-only run), typed binary.
    pub report_evidence: Option<BinaryValue>,
    /// The `.contracts.json` assumptions before run-final keys, typed binary.
    pub assumptions: BinaryValue,
    pub pending_declarations: u64,
    pub pending_markers: Vec<PendingMarkerRecord>,
    /// The report-independent part of the `contract_revision` section
    /// ([`crate::contract_revision::SectionTemplate`]), typed binary.
    pub revision_template: BinaryValue,
    /// The historical reference revision the Unknown sidecars name.
    pub historical_reference_revision: String,
    /// Card MEMBER-PREPASS-OTHER-ROUTES (schema 5): what the run's member
    /// fold (rule 12n) reads beyond the units' own facts, typed binary: the
    /// fold's environment (`callback_member_fold::FoldEnvironment`) and, when
    /// the pre-pass ran, the run's closure of each invoked member with the
    /// pre-pass's evidence section. `None` when no declaration invokes a
    /// member or states a result state.
    pub member_fold: Option<BinaryValue>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PendingMarkerRecord {
    pub function: String,
    pub scope: String,
    pub clause: Option<String>,
    pub reason: String,
    pub owner: Option<String>,
    /// Card W47-REVIEW-HOLD-CONCRETE-REASON (schema 4): the marker's
    /// published category (`qcore_schemas::contract::ReviewDisposition`).
    pub category: String,
    pub module: String,
    pub module_sha256: Option<String>,
}

/// One step's share of every unit row: per unit, in unit order, the row's
/// members this step owns, each as a typed binary value.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RowStepSection {
    pub units: Vec<RowMembers>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RowMembers {
    pub members: Vec<JsonMember>,
}

/// One member of a report row: its key and numerically typed binary value.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JsonMember {
    pub key: String,
    pub value: BinaryValue,
}

/// Closed certificate publication state, encoded as a postcard discriminant.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CertificateOutcome {
    PublishedSafe,
    BindingRefused,
    NotPublished,
}

/// Per unit, what became of a `Safe` certificate. Derived from the rows for
/// inspection; composition does not read it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CertificatesSection {
    pub units: Vec<CertificateRecord>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CertificateRecord {
    pub file: String,
    /// `bound` (a Safe certificate admitted and bound to the unit's
    /// compile-task identity), `binding-refused`, or `none`.
    pub outcome: CertificateOutcome,
    pub compile_task_identity_json_sha256: Option<String>,
    pub reason: Option<String>,
}

/// The contract set in force, and the unit's held calls.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContractsSection {
    pub manifest_sha256: String,
    pub sources_sha256: String,
    pub modules: Vec<ModuleDigest>,
    pub provider_selection: Option<BinaryValue>,
    pub evidence_mode: String,
    pub locks: Vec<LockDigest>,
    pub pending_declarations: u64,
    pub pending_markers: u64,
    /// The group's `awaiting-human-review` held calls, per callee.
    pub held_calls: Vec<HeldCallRecord>,
    /// The functions the group's analysis read, by name (what the
    /// `contract_revision` section reads of the report).
    pub analysed_functions: Vec<String>,
    /// The group's `contract-pin-mismatch` held calls, per callee.
    pub pin_held_calls: Vec<HeldCallRecord>,
    /// Card MEMBER-PREPASS-OTHER-ROUTES (schema 5): the member and state
    /// facts the group's unit recorded for the run's fold
    /// (`callback_member_fold::UnitFacts`), typed binary, in unit order.
    pub callback_members: Vec<BinaryValue>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModuleDigest {
    pub module: String,
    pub sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HeldCallRecord {
    pub callee: String,
    pub call_sites: u64,
    pub functions: Vec<HeldFunction>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HeldFunction {
    pub library: String,
    pub file: String,
    pub function: String,
}

/// The unit's final result.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResultSection {
    pub header: String,
    /// Each unit's console lines, in unit order.
    pub outputs: Vec<String>,
    pub counts: CountsRecord,
    pub diagnostic_counts: Vec<CategoryCount>,
    /// The canonical checkout source this unit stands for.
    pub sources: Vec<String>,
    /// The group's verdict: the join (`qcore_schemas::verdict::Verdict::fold`)
    /// of its units' verdicts ([`unit_verdict`]), spelled as in reports.
    pub verdict: Verdict,
}

/// A unit row's status as a four-value verdict (card W45-USER-CONTRACT-PLUGINS,
/// §L21.2): a verdict spelling is itself; a gateway failure, or any status
/// this build does not know, is `Unknown` (fail-closed); a data-only unit,
/// which defines no function and so carries no obligation, is `Safe`, the
/// join's identity.
#[must_use]
pub fn unit_verdict(status: &str) -> Verdict {
    match status.parse::<Verdict>() {
        Ok(verdict) => verdict,
        Err(_) if status == "data_only" => Verdict::Safe,
        Err(_) => Verdict::Unknown,
    }
}

/// The verdict a group's tallies imply: the join of one representative verdict
/// per nonzero category. A group's recorded verdict must equal it; composition
/// refuses a checkpoint where it does not.
#[must_use]
pub fn tallied_verdict(counts: &CountsRecord) -> Verdict {
    Verdict::fold(
        [
            (counts.safe_units, Verdict::Safe),
            (counts.pending_units, Verdict::Pending),
            (counts.unknown_units, Verdict::Unknown),
            (counts.gateway_failures, Verdict::Unknown),
            (counts.unsafe_units, Verdict::Unsafe),
        ]
        .into_iter()
        .filter(|(count, _)| *count > 0)
        .map(|(_, verdict)| verdict),
    )
}

/// [`crate::projection::AuditCounts`] without its three wall-clock sums,
/// which are measurements and live in [`ProvenanceSection`].
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CountsRecord {
    pub units: u64,
    pub data_only_units: u64,
    pub source_functions: u64,
    pub projected_functions: u64,
    pub safe_units: u64,
    /// Card W45-USER-CONTRACT-PLUGINS: units whose verdict is `Pending`.
    pub pending_units: u64,
    pub unsafe_units: u64,
    pub unknown_units: u64,
    pub gateway_failures: u64,
    pub absent_from_raw_scan_definitions: u64,
    pub unanalyzable_definitions: u64,
    pub enumeration_refused_units: u64,
    pub withheld_safe_units: u64,
    pub prepared_units: u64,
    pub preparation_clang_invocations: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CategoryCount {
    pub category: String,
    pub count: u64,
}

/// Measurements of the run that computed the group. Not identity, not
/// composed; the console `preparation:` line reads the timings.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProvenanceSection {
    /// Wall time from the group's start to its seal, in milliseconds.
    pub wall_time_ms: u64,
    /// The largest resident set size sampled while the group ran (KiB).
    pub sampled_peak_rss_kib: Option<u64>,
    /// The process's high-water resident set size when the group was sealed
    /// (KiB): a running maximum over the whole process so far.
    pub process_peak_rss_kib: Option<u64>,
    pub rss_sample_interval_ms: u64,
    pub jobs: u64,
    pub plain_preprocessing_elapsed_us: u64,
    pub syntax_validation_elapsed_us: u64,
    pub line_marked_preprocessing_elapsed_us: u64,
}

/// One whole checkpoint in memory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Checkpoint {
    pub identity: IdentitySection,
    pub plan: PlanSection,
    pub global: GlobalSection,
    pub preparation: RowStepSection,
    pub lowering: RowStepSection,
    pub verdicts: RowStepSection,
    pub certificates: CertificatesSection,
    pub contracts: ContractsSection,
    pub result: ResultSection,
    pub provenance: ProvenanceSection,
}

pub(crate) fn encode<T: Serialize>(name: &str, value: &T) -> Result<Vec<u8>, String> {
    postcard::to_stdvec(value)
        .map_err(|error| format!("could not encode section `{name}`: {error}"))
}

fn decode<T: DeserializeOwned>(name: &str, bytes: &[u8]) -> Result<T, String> {
    let (value, rest) = postcard::take_from_bytes::<T>(bytes)
        .map_err(|error| format!("section `{name}` does not decode: {error}"))?;
    if !rest.is_empty() {
        return Err(format!(
            "section `{name}` has {} trailing byte(s) after its value",
            rest.len()
        ));
    }
    Ok(value)
}

impl Checkpoint {
    /// The encoded sections, in file order.
    pub fn encode_sections(&self) -> Result<Vec<(&'static str, Vec<u8>)>, String> {
        Ok(vec![
            ("identity", encode("identity", &self.identity)?),
            ("plan", encode("plan", &self.plan)?),
            ("global", encode("global", &self.global)?),
            ("preparation", encode("preparation", &self.preparation)?),
            ("lowering", encode("lowering", &self.lowering)?),
            ("verdicts", encode("verdicts", &self.verdicts)?),
            ("certificates", encode("certificates", &self.certificates)?),
            ("contracts", encode("contracts", &self.contracts)?),
            ("result", encode("result", &self.result)?),
            ("provenance", encode("provenance", &self.provenance)?),
        ])
    }

    fn validate_payloads(&self) -> Result<(), String> {
        validate_payloads(
            &self.global,
            [&self.preparation, &self.lowering, &self.verdicts],
            &self.contracts,
        )
    }

    /// The file's bytes.
    pub fn to_bytes(&self) -> Result<Vec<u8>, String> {
        self.validate_payloads()?;
        assemble(self.encode_sections()?)
    }

    /// Decode a whole file, refusing it unless every check passes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        let (header, body) = parse_header(bytes, Some(bytes.len() as u64))?;
        let mut offset = 0usize;
        let mut sections = BTreeMap::new();
        for entry in &header.sections {
            let length = usize::try_from(entry.length).map_err(|error| error.to_string())?;
            let slice = &body[offset..offset + length];
            if sha256_bytes(slice) != entry.sha256 {
                return Err(format!(
                    "section `{}` does not match its SHA-256 (tampered or corrupted)",
                    entry.name
                ));
            }
            sections.insert(entry.name.as_str(), slice);
            offset += length;
        }
        let get = |name: &str| sections[name];
        let checkpoint = Self {
            identity: decode("identity", get("identity"))?,
            plan: decode("plan", get("plan"))?,
            global: decode("global", get("global"))?,
            preparation: decode("preparation", get("preparation"))?,
            lowering: decode("lowering", get("lowering"))?,
            verdicts: decode("verdicts", get("verdicts"))?,
            certificates: decode("certificates", get("certificates"))?,
            contracts: decode("contracts", get("contracts"))?,
            result: decode("result", get("result"))?,
            provenance: decode("provenance", get("provenance"))?,
        };
        checkpoint.validate_payloads()?;
        Ok(checkpoint)
    }

    /// The manifest this checkpoint encodes to.
    pub fn manifest(&self) -> Result<Vec<SectionEntry>, String> {
        Ok(self
            .encode_sections()?
            .into_iter()
            .map(|(name, bytes)| SectionEntry {
                name: name.to_owned(),
                length: bytes.len() as u64,
                sha256: sha256_bytes(&bytes),
            })
            .collect())
    }

    /// The content digest: every section but `provenance`.
    pub fn content_sha256(&self) -> Result<String, String> {
        content_sha256(&self.manifest()?)
    }
}

fn validate_payloads(
    global: &GlobalSection,
    steps: [&RowStepSection; 3],
    contracts: &ContractsSection,
) -> Result<(), String> {
    if let Some(value) = &global.report_evidence {
        value.to_json()?;
    }
    global.assumptions.to_json()?;
    global.revision_template.to_json()?;
    if let Some(value) = &contracts.provider_selection {
        value.to_json()?;
    }
    for section in steps {
        for unit in &section.units {
            for member in &unit.members {
                member.value.to_json()?;
            }
        }
    }
    Ok(())
}

/// Assemble a file from encoded sections in [`SECTIONS`] order.
pub fn assemble(sections: Vec<(&str, Vec<u8>)>) -> Result<Vec<u8>, String> {
    assemble_as(CURRENT_LAYOUT, sections)
}

/// Assemble a file of `layout` from encoded sections in its order.
pub fn assemble_as(layout: Layout, sections: Vec<(&str, Vec<u8>)>) -> Result<Vec<u8>, String> {
    let names = sections.iter().map(|(name, _)| *name).collect::<Vec<_>>();
    if names != layout.sections {
        return Err(format!(
            "sections {names:?} are not the schema's {:?}",
            layout.sections
        ));
    }
    let header = Header {
        schema_version: layout.schema_version,
        sections: sections
            .iter()
            .map(|(name, bytes)| SectionEntry {
                name: (*name).to_owned(),
                length: bytes.len() as u64,
                sha256: sha256_bytes(bytes),
            })
            .collect(),
    };
    let header_bytes = encode("header", &header)?;
    let length = u32::try_from(header_bytes.len()).map_err(|error| error.to_string())?;
    let mut out = Vec::new();
    out.extend_from_slice(&prelude());
    out.extend_from_slice(&length.to_le_bytes());
    out.extend_from_slice(&header_bytes);
    out.extend_from_slice(&sha256_bytes(&out));
    for (_, bytes) in sections {
        out.extend_from_slice(&bytes);
    }
    Ok(out)
}

/// The content digest of a manifest: SHA-256 over the postcard encoding of
/// the schema version and every entry except `provenance`.
pub fn content_sha256(manifest: &[SectionEntry]) -> Result<String, String> {
    content_sha256_as(CURRENT_LAYOUT, manifest)
}

/// The content digest of a manifest of `layout`.
pub fn content_sha256_as(layout: Layout, manifest: &[SectionEntry]) -> Result<String, String> {
    let content = manifest
        .iter()
        .filter(|entry| entry.name != PROVENANCE)
        .cloned()
        .collect::<Vec<_>>();
    Ok(sha256_hex(&encode(
        "content",
        &(layout.schema_version, content),
    )?))
}

/// Check the magic, the format version and the header length field; return
/// the declared header length.
fn check_prelude(bytes: &[u8]) -> Result<u32, String> {
    if bytes.len() < PRELUDE_LEN + 4 {
        return Err("truncated: shorter than the `.qo` prelude".into());
    }
    if &bytes[..2] != b"QO" || bytes[4..8] != GUARD {
        return Err("not a `.qo` checkpoint (bad magic)".into());
    }
    let version = u16::from_be_bytes([bytes[2], bytes[3]]);
    if version != FORMAT_VERSION {
        return Err(format!(
            "unsupported `.qo` format version {version} (this build reads version {FORMAT_VERSION})"
        ));
    }
    let length = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
    if length > MAX_HEADER_LEN {
        return Err(format!("implausible header length {length}"));
    }
    Ok(length)
}

/// Check the prelude and the header, and that the file is exactly as long as
/// the header declares (when `file_length` is known). Returns the header and
/// the bytes after it that are present in `bytes`.
fn parse_header(bytes: &[u8], file_length: Option<u64>) -> Result<(Header, &[u8]), String> {
    parse_header_as(CURRENT_LAYOUT, bytes, file_length)
}

fn parse_header_as(
    layout: Layout,
    bytes: &[u8],
    file_length: Option<u64>,
) -> Result<(Header, &[u8]), String> {
    let length = check_prelude(bytes)?;
    let header_end = PRELUDE_LEN + 4 + length as usize;
    if bytes.len() < header_end + 32 {
        return Err("truncated: the header is incomplete".into());
    }
    if sha256_bytes(&bytes[..header_end]) != bytes[header_end..header_end + 32] {
        return Err("the header does not match its SHA-256 (tampered or corrupted)".into());
    }
    let header: Header = decode("header", &bytes[PRELUDE_LEN + 4..header_end])?;
    if header.schema_version != layout.schema_version {
        return Err(format!(
            "unsupported checkpoint schema version {} (this build reads version {})",
            header.schema_version, layout.schema_version
        ));
    }
    let names = header
        .sections
        .iter()
        .map(|entry| entry.name.as_str())
        .collect::<Vec<_>>();
    if names != layout.sections {
        return Err(format!(
            "unexpected section list {names:?} (schema {} has {:?})",
            layout.schema_version, layout.sections
        ));
    }
    let body_length = header
        .sections
        .iter()
        .try_fold(0u64, |sum, entry| sum.checked_add(entry.length))
        .ok_or("section lengths overflow")?;
    let expected = (header_end as u64 + 32)
        .checked_add(body_length)
        .ok_or("section lengths overflow")?;
    if let Some(actual) = file_length {
        if actual < expected {
            return Err(format!(
                "truncated: {actual} byte(s), the header declares {expected}"
            ));
        }
        if actual > expected {
            return Err(format!(
                "{} trailing byte(s) after the last section",
                actual - expected
            ));
        }
    }
    Ok((header, &bytes[header_end + 32..]))
}

/// A checkpoint file opened for section-at-a-time reading (streaming
/// composition). Opening checks the prelude, the header and the file length;
/// every section read checks that section's digest.
pub struct CheckpointReader {
    path: String,
    file: File,
    header: Header,
    body_start: u64,
    layout: Layout,
}

impl CheckpointReader {
    pub fn open(path: &Path) -> Result<Self, String> {
        Self::open_as(path, CURRENT_LAYOUT)
    }

    /// Open a file of `layout`; a file of any other layout is refused.
    pub fn open_as(path: &Path, layout: Layout) -> Result<Self, String> {
        let describe = |error: String| format!("{}: {error}", path.display());
        let mut file = File::open(path).map_err(|error| describe(error.to_string()))?;
        let file_length = file
            .metadata()
            .map_err(|error| describe(error.to_string()))?
            .len();
        let mut prefix = vec![0u8; PRELUDE_LEN + 4];
        file.read_exact(&mut prefix)
            .map_err(|_| describe("truncated: shorter than the `.qo` prelude".into()))?;
        // Validate the prelude before trusting the declared header length.
        let length = check_prelude(&prefix).map_err(describe)?;
        let mut head = prefix;
        head.resize(PRELUDE_LEN + 4 + length as usize + 32, 0);
        file.read_exact(&mut head[PRELUDE_LEN + 4..])
            .map_err(|_| describe("truncated: the header is incomplete".into()))?;
        let (header, _) = parse_header_as(layout, &head, Some(file_length)).map_err(describe)?;
        Ok(Self {
            path: path.display().to_string(),
            file,
            header,
            body_start: head.len() as u64,
            layout,
        })
    }

    /// The manifest.
    #[must_use]
    pub fn manifest(&self) -> &[SectionEntry] {
        &self.header.sections
    }

    /// The raw bytes of one section, digest-checked.
    pub fn raw(&mut self, name: &str) -> Result<Vec<u8>, String> {
        let mut offset = self.body_start;
        for entry in &self.header.sections {
            if entry.name == name {
                let length = usize::try_from(entry.length).map_err(|error| error.to_string())?;
                let mut bytes = vec![0u8; length];
                self.file
                    .seek(SeekFrom::Start(offset))
                    .and_then(|_| self.file.read_exact(&mut bytes))
                    .map_err(|error| format!("{}: section `{name}`: {error}", self.path))?;
                if sha256_bytes(&bytes) != entry.sha256 {
                    return Err(format!(
                        "{}: section `{name}` does not match its SHA-256 (tampered or corrupted)",
                        self.path
                    ));
                }
                return Ok(bytes);
            }
            offset += entry.length;
        }
        Err(format!("{}: no section `{name}`", self.path))
    }

    /// One section, digest-checked and decoded.
    pub fn section<T: DeserializeOwned>(&mut self, name: &str) -> Result<T, String> {
        let bytes = self.raw(name)?;
        decode(name, &bytes).map_err(|error| format!("{}: {error}", self.path))
    }

    /// Check every section's digest and decoding, one section at a time.
    pub fn verify(&mut self) -> Result<(), String> {
        if self.layout != CURRENT_LAYOUT {
            return Err(format!(
                "{}: `verify` reads the current-route layout only",
                self.path
            ));
        }
        self.section::<IdentitySection>("identity")?;
        self.section::<PlanSection>("plan")?;
        let global = self.section::<GlobalSection>("global")?;
        let preparation = self.section::<RowStepSection>("preparation")?;
        let lowering = self.section::<RowStepSection>("lowering")?;
        let verdicts = self.section::<RowStepSection>("verdicts")?;
        self.section::<CertificatesSection>("certificates")?;
        let contracts = self.section::<ContractsSection>("contracts")?;
        self.section::<ResultSection>("result")?;
        self.section::<ProvenanceSection>("provenance")?;
        validate_payloads(&global, [&preparation, &lowering, &verdicts], &contracts)
    }

    /// The content digest of this file's manifest.
    pub fn content_sha256(&self) -> Result<String, String> {
        content_sha256_as(self.layout, &self.header.sections)
    }
}

// ---------------------------------------------------------------------------
// Unit rows: split into step members, and rendered back byte for byte.
// ---------------------------------------------------------------------------

/// The step that owns a row member: 0 preparation, 1 lowering, 2 verdicts.
/// A member no step names (a key added by a later card) is kept in the
/// verdicts step, never dropped.
fn step_of(key: &str) -> usize {
    match key {
        "library"
        | "file"
        | "compile_configuration"
        | "enumeration"
        | "origin_coordinates"
        | "assumptions" => 0,
        "reason" | "reason_code" | "source_functions" | "not_projected" => 1,
        _ => 2,
    }
}

/// Split one unit row into its preparation, lowering and verdict members.
pub fn split_row(row: &Value) -> Result<[RowMembers; 3], String> {
    let Value::Object(map) = row else {
        return Err("a unit row is not a JSON object".into());
    };
    let mut steps: [RowMembers; 3] = Default::default();
    for (key, value) in map {
        steps[step_of(key)].members.push(JsonMember {
            key: key.clone(),
            value: BinaryValue::from_json(value)?,
        });
    }
    Ok(steps)
}

/// Re-indent pretty JSON text written at depth zero for nesting at `depth`.
/// A newline only ever appears between tokens (strings escape theirs), so
/// indenting after every newline is exact.
fn reindent(text: &str, depth: usize, out: &mut String) {
    let indent = "  ".repeat(depth);
    for (index, line) in text.split('\n').enumerate() {
        if index > 0 {
            out.push('\n');
            out.push_str(&indent);
        }
        out.push_str(line);
    }
}

/// Render one row from its step members as serde_json's pretty printer writes
/// it at `depth`: members in key order (serde_json's map order), each value
/// re-indented. Two steps naming one key is an error.
pub fn render_row(steps: [&RowMembers; 3], depth: usize, out: &mut String) -> Result<(), String> {
    let mut members = steps
        .iter()
        .flat_map(|step| step.members.iter())
        .collect::<Vec<_>>();
    members.sort_by(|left, right| left.key.cmp(&right.key));
    if members.windows(2).any(|pair| pair[0].key == pair[1].key) {
        return Err("a unit row member is recorded twice".into());
    }
    if members.is_empty() {
        out.push_str("{}");
        return Ok(());
    }
    out.push('{');
    let indent = "  ".repeat(depth + 1);
    for (index, member) in members.iter().enumerate() {
        out.push_str(if index == 0 { "\n" } else { ",\n" });
        out.push_str(&indent);
        out.push_str(&serde_json::to_string(&member.key).map_err(|error| error.to_string())?);
        out.push_str(": ");
        let json = serde_json::to_string_pretty(&member.value.to_json()?)
            .map_err(|error| error.to_string())?;
        reindent(&json, depth + 1, out);
    }
    out.push('\n');
    out.push_str(&"  ".repeat(depth));
    out.push('}');
    Ok(())
}

/// The pretty report text around its `units` array: everything up to and
/// including `"units": `, and the closing text after the array. The report's
/// members are in key order and `units` sorts last; anything else is refused.
pub fn report_frame(document: &Value) -> Result<(String, String), String> {
    let text = serde_json::to_string_pretty(document).map_err(|error| error.to_string())?;
    let suffix = "\"units\": []\n}";
    let Some(head) = text.strip_suffix(suffix) else {
        return Err("the report skeleton does not end with an empty `units` member".into());
    };
    if document
        .as_object()
        .and_then(|map| map.keys().next_back())
        .is_none_or(|key| key != "units")
    {
        return Err("`units` is not the report's last member".into());
    }
    Ok((format!("{head}\"units\": "), "\n}\n".to_owned()))
}

/// The digest of a JSON value's compact text.
pub fn json_sha256(value: &Value) -> Result<String, String> {
    Ok(sha256_hex(
        &serde_json::to_vec(value).map_err(|error| error.to_string())?,
    ))
}

/// Serialize a JSON value to text that parses back to an equal value, or
/// refuse: a float that does not round-trip would make a checkpoint's text
/// differ from what it stands for.
pub fn exact_json(value: &Value) -> Result<String, String> {
    let text = serde_json::to_string(value).map_err(|error| error.to_string())?;
    let back: Value = serde_json::from_str(&text).map_err(|error| error.to_string())?;
    if &back != value {
        return Err("a JSON value does not round-trip exactly through its text".into());
    }
    Ok(text)
}

// ---------------------------------------------------------------------------
// Text decomposition (YAML and TOML).
// ---------------------------------------------------------------------------

/// A text format for [`decompose`] and [`recompose`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextFormat {
    Yaml,
    Toml,
}

impl TextFormat {
    pub fn parse(text: &str) -> Result<Self, String> {
        match text {
            "yaml" => Ok(Self::Yaml),
            "toml" => Ok(Self::Toml),
            other => Err(format!("unknown format `{other}`; use yaml or toml")),
        }
    }
}

/// The complete text form of one checkpoint: the container's facts and every
/// section's typed contents.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TextCheckpoint {
    pub format: String,
    pub format_version: u16,
    pub schema_version: u32,
    pub content_sha256: String,
    pub manifest: Vec<TextSectionEntry>,
    pub identity: IdentitySection,
    pub plan: PlanSection,
    pub global: GlobalSection,
    pub preparation: RowStepSection,
    pub lowering: RowStepSection,
    pub verdicts: RowStepSection,
    pub certificates: CertificatesSection,
    pub contracts: ContractsSection,
    pub result: ResultSection,
    pub provenance: ProvenanceSection,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TextSectionEntry {
    pub name: String,
    pub length: u64,
    pub sha256: String,
}

const TEXT_FORMAT_NAME: &str = "quirin-audit checkpoint (.qo)";

impl TextCheckpoint {
    pub fn of(checkpoint: &Checkpoint) -> Result<Self, String> {
        let manifest = checkpoint.manifest()?;
        Ok(Self {
            format: TEXT_FORMAT_NAME.into(),
            format_version: FORMAT_VERSION,
            schema_version: SCHEMA_VERSION,
            content_sha256: content_sha256(&manifest)?,
            manifest: manifest
                .iter()
                .map(|entry| TextSectionEntry {
                    name: entry.name.clone(),
                    length: entry.length,
                    sha256: hex(&entry.sha256),
                })
                .collect(),
            identity: checkpoint.identity.clone(),
            plan: checkpoint.plan.clone(),
            global: checkpoint.global.clone(),
            preparation: checkpoint.preparation.clone(),
            lowering: checkpoint.lowering.clone(),
            verdicts: checkpoint.verdicts.clone(),
            certificates: checkpoint.certificates.clone(),
            contracts: checkpoint.contracts.clone(),
            result: checkpoint.result.clone(),
            provenance: checkpoint.provenance.clone(),
        })
    }

    /// The checkpoint this text stands for, refusing a text whose recorded
    /// manifest or versions disagree with its contents (an edited text).
    pub fn into_checkpoint(self) -> Result<Checkpoint, String> {
        if self.format != TEXT_FORMAT_NAME {
            return Err(format!(
                "`format` is `{}`, not `{TEXT_FORMAT_NAME}`",
                self.format
            ));
        }
        if self.format_version != FORMAT_VERSION || self.schema_version != SCHEMA_VERSION {
            return Err(format!(
                "text is format {} schema {}; this build reads format {FORMAT_VERSION} schema {SCHEMA_VERSION}",
                self.format_version, self.schema_version
            ));
        }
        let recorded = self
            .manifest
            .iter()
            .map(|entry| {
                Ok(SectionEntry {
                    name: entry.name.clone(),
                    length: entry.length,
                    sha256: unhex(&entry.sha256)?,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        let checkpoint = Checkpoint {
            identity: self.identity,
            plan: self.plan,
            global: self.global,
            preparation: self.preparation,
            lowering: self.lowering,
            verdicts: self.verdicts,
            certificates: self.certificates,
            contracts: self.contracts,
            result: self.result,
            provenance: self.provenance,
        };
        let actual = checkpoint.manifest()?;
        if actual != recorded {
            let differing = actual
                .iter()
                .zip(recorded.iter().chain(std::iter::repeat(&SectionEntry {
                    name: String::new(),
                    length: 0,
                    sha256: [0; 32],
                })))
                .filter(|(left, right)| left != right)
                .map(|(left, _)| left.name.clone())
                .collect::<Vec<_>>();
            return Err(format!(
                "the text's contents do not match its recorded manifest (section(s) {differing:?} differ): it was edited or is incomplete"
            ));
        }
        if content_sha256(&actual)? != self.content_sha256 {
            return Err("the text's `content_sha256` does not match its manifest".into());
        }
        Ok(checkpoint)
    }
}

/// The complete, lossless text form of a checkpoint.
pub fn decompose(checkpoint: &Checkpoint, format: TextFormat) -> Result<String, String> {
    let text = TextCheckpoint::of(checkpoint)?;
    match format {
        TextFormat::Yaml => serde_norway::to_string(&text).map_err(|error| error.to_string()),
        TextFormat::Toml => toml::to_string(&text).map_err(|error| {
            format!("the checkpoint has no TOML form: {error} (TOML integers are signed 64-bit)")
        }),
    }
}

/// Parse a text form back into the checkpoint it stands for.
pub fn parse_text(text: &str, format: TextFormat) -> Result<Checkpoint, String> {
    let parsed: TextCheckpoint = match format {
        TextFormat::Yaml => serde_norway::from_str(text).map_err(|error| error.to_string())?,
        TextFormat::Toml => toml::from_str(text).map_err(|error| error.to_string())?,
    };
    parsed.into_checkpoint()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    pub(crate) fn sample() -> Checkpoint {
        let row = json!({
            "library": "lib", "file": "a.c", "status": "Unknown", "reason": "why \"quoted\"\nline",
            "reason_code": null, "source_functions": ["f", "g"], "functions": [
                {"function": "f", "status": "Safe", "diagnostics": []},
                {"function": "g", "status": "Unknown", "diagnostics": [{"line": 3, "origin": null, "category": "leak", "description": "d"}]},
            ],
            "not_projected": [], "diagnostic_counts": {"leak": 1},
            "compile_configuration": {"command": "cc -c a.c", "nested": {"deep": [1, {"x": {}}]}, "empty": []},
            "enumeration": {"completeness": "established"},
            "zz_later_card": true,
        });
        let [preparation, lowering, verdicts] = split_row(&row).unwrap();
        Checkpoint {
            identity: IdentitySection {
                run: RunIdentity {
                    route: "current".into(),
                    analyzer: AnalyzerIdentity {
                        package: "qcore-audit".into(),
                        version: "0.1.0".into(),
                        build_profile: "dev".into(),
                        build_commit: None,
                        binary_sha256: "0".repeat(64),
                    },
                    mode: "configured".into(),
                    checkout: "/c".into(),
                    source_revision: "r".into(),
                    source_profile_sha256: "1".repeat(64),
                    contracts: ContractSetIdentity {
                        manifest_sha256: "2".repeat(64),
                        sources_sha256: "3".repeat(64),
                        provider_selection_sha256: Some("4".repeat(64)),
                        evidence_sha256: None,
                        locks: vec![LockDigest {
                            path: "/p/contracts.lock".into(),
                            sha256: "a".repeat(64),
                        }],
                        pin_evaluation_sha256: "b".repeat(64),
                        plugin_checks_sha256: None,
                        refuted_table_sha256: "c".repeat(64),
                    },
                    compiler: CompilerIdentity {
                        program: "/usr/bin/clang".into(),
                        version: "v".into(),
                        target: "t".into(),
                    },
                    environment: vec![EnvironmentValue {
                        name: "QCORE_CLANG".into(),
                        value: "/usr/bin/clang".into(),
                    }],
                    member_facts_sha256: None,
                },
                group: GroupIdentity {
                    label: "lib".into(),
                    unresolved: None,
                    units: vec![
                        UnitFingerprint {
                            file: "a.c".into(),
                            configuration_sha256: "5".repeat(64),
                            preparation: PreparationFacts::Prepared {
                                validation_error: None,
                                line_marked_sha256: Some("6".repeat(64)),
                                line_marked_error: None,
                            },
                        },
                        UnitFingerprint {
                            file: "b.c".into(),
                            configuration_sha256: "7".repeat(64),
                            preparation: PreparationFacts::NotPrepared,
                        },
                    ],
                },
            },
            plan: PlanSection {
                group: 0,
                groups: vec![PlannedGroup {
                    label: "lib".into(),
                    header: "lib: 1 selected compilation configuration(s)".into(),
                    units: vec![UnitKey {
                        expected_source: "/c/a.c".into(),
                        record_sha256: "8".repeat(64),
                    }],
                }],
            },
            global: GlobalSection {
                console_preamble: vec!["x r (current checkout)".into()],
                source_revision: "r".into(),
                revision_key: "fluent_bit_revision".into(),
                report_evidence: None,
                assumptions: BinaryValue::from_json(&json!({"a":1})).unwrap(),
                pending_declarations: 0,
                pending_markers: vec![PendingMarkerRecord {
                    function: "k".into(),
                    scope: "contract".into(),
                    clause: None,
                    reason: "r".into(),
                    owner: None,
                    category: "must-be-rewritten-before-acceptance".into(),
                    module: "m".into(),
                    module_sha256: None,
                }],
                revision_template: BinaryValue::from_json(&json!({"pinned":[]})).unwrap(),
                historical_reference_revision: "h".into(),
                member_fold: None,
            },
            preparation: RowStepSection {
                units: vec![preparation],
            },
            lowering: RowStepSection {
                units: vec![lowering],
            },
            verdicts: RowStepSection {
                units: vec![verdicts],
            },
            certificates: CertificatesSection {
                units: vec![CertificateRecord {
                    file: "a.c".into(),
                    outcome: CertificateOutcome::NotPublished,
                    compile_task_identity_json_sha256: None,
                    reason: None,
                }],
            },
            contracts: ContractsSection {
                manifest_sha256: "2".repeat(64),
                sources_sha256: "3".repeat(64),
                modules: vec![ModuleDigest {
                    module: "m".into(),
                    sha256: "9".repeat(64),
                }],
                provider_selection: None,
                evidence_mode: "configured".into(),
                locks: vec![],
                pending_declarations: 0,
                pending_markers: 0,
                held_calls: vec![],
                analysed_functions: vec!["f".into(), "g".into()],
                pin_held_calls: vec![HeldCallRecord {
                    callee: "k".into(),
                    call_sites: 2,
                    functions: vec![HeldFunction {
                        library: "lib".into(),
                        file: "a.c".into(),
                        function: "g".into(),
                    }],
                }],
                callback_members: Vec::new(),
            },
            result: ResultSection {
                header: "lib: 1 selected compilation configuration(s)".into(),
                outputs: vec!["  lib/a.c: Unknown\n".into()],
                counts: CountsRecord {
                    units: 1,
                    unknown_units: 1,
                    ..Default::default()
                },
                diagnostic_counts: vec![CategoryCount {
                    category: "leak".into(),
                    count: 1,
                }],
                sources: vec!["/c/a.c".into()],
                verdict: Verdict::Unknown,
            },
            provenance: ProvenanceSection {
                wall_time_ms: 5,
                sampled_peak_rss_kib: Some(1024),
                ..Default::default()
            },
        }
    }

    #[test]
    fn numeric_binary_values_preserve_json_boundaries_and_float_bits() {
        let floats = [
            -0.0f64,
            1.0,
            1e100,
            1e-100,
            f64::from_bits(1.0f64.to_bits() + 1),
        ];
        let mut values = vec![json!(u64::MAX), json!(i64::MIN)];
        values.extend(
            floats
                .into_iter()
                .map(|value| Value::Number(serde_json::Number::from_f64(value).unwrap())),
        );
        for value in values {
            let binary = BinaryValue::from_json(&value).unwrap();
            let encoded = postcard::to_stdvec(&binary).unwrap();
            let decoded: BinaryValue = postcard::from_bytes(&encoded).unwrap();
            assert_eq!(decoded, binary);
            assert_eq!(decoded.to_json().unwrap(), value);
            assert_eq!(
                serde_json::to_vec(&decoded.to_json().unwrap()).unwrap(),
                serde_json::to_vec(&value).unwrap()
            );
            if let BinaryValue::F64Bits(bits) = decoded {
                assert_eq!(bits, value.as_f64().unwrap().to_bits());
            }
        }
        assert!(
            postcard::to_stdvec(&BinaryValue::U64(u64::MAX))
                .unwrap()
                .len()
                < 20
        );
        assert!(postcard::from_bytes::<BinaryValue>(&[0x7f]).is_err());
        assert!(BinaryValue::F64Bits(f64::NAN.to_bits()).to_json().is_err());
        assert!(
            BinaryValue::Object(vec![
                ("x".into(), BinaryValue::Null),
                ("x".into(), BinaryValue::Null)
            ])
            .to_json()
            .is_err()
        );
    }

    #[test]
    fn a_row_renders_back_exactly_as_serde_json_pretty_prints_it() {
        let row = json!({
            "library": "lib", "file": "a.c", "status": "Unknown", "reason": "a\nb \"c\" \\ d",
            "functions": [{"function": "f", "diagnostics": [], "status": "Safe"}],
            "compile_configuration": {"nested": {"deep": [1, {"x": {}}], "e": []}},
            "diagnostic_counts": {}, "zz": null, "aa": [[]],
        });
        let steps = split_row(&row).unwrap();
        for depth in 0..4 {
            let mut out = String::new();
            render_row([&steps[0], &steps[1], &steps[2]], depth, &mut out).unwrap();
            // At depth d, serde_json nests the object inside d containers.
            let mut wrapped = row.clone();
            for _ in 0..depth {
                wrapped = json!([wrapped]);
            }
            let expected = serde_json::to_string_pretty(&wrapped).unwrap();
            let mut inner = expected.as_str();
            for level in 0..depth {
                let indent = "  ".repeat(level + 1);
                inner = inner
                    .strip_prefix('[')
                    .unwrap()
                    .strip_suffix(']')
                    .unwrap()
                    .strip_prefix(&format!("\n{indent}"))
                    .unwrap();
                inner = inner
                    .strip_suffix(&format!("\n{}", "  ".repeat(level)))
                    .unwrap();
            }
            assert_eq!(out, inner, "depth {depth}");
        }
        let mut empty = String::new();
        render_row(
            [
                &RowMembers::default(),
                &RowMembers::default(),
                &RowMembers::default(),
            ],
            2,
            &mut empty,
        )
        .unwrap();
        assert_eq!(empty, "{}");
    }

    #[test]
    fn a_key_no_step_names_is_kept_in_the_verdict_step() {
        let steps = split_row(&json!({"library": "l", "later": 1, "reason": null})).unwrap();
        assert_eq!(steps[0].members[0].key, "library");
        assert_eq!(steps[1].members[0].key, "reason");
        assert_eq!(steps[2].members[0].key, "later");
    }

    #[test]
    fn the_binary_form_round_trips_and_is_deterministic() {
        let checkpoint = sample();
        let bytes = checkpoint.to_bytes().unwrap();
        assert_eq!(&bytes[..2], b"QO");
        assert_eq!(bytes, checkpoint.to_bytes().unwrap());
        assert_eq!(Checkpoint::from_bytes(&bytes).unwrap(), checkpoint);
        // Provenance is outside the content digest; everything else is in it.
        let mut measured = checkpoint.clone();
        measured.provenance.wall_time_ms = 99;
        assert_ne!(measured.to_bytes().unwrap(), bytes);
        assert_eq!(
            measured.content_sha256().unwrap(),
            checkpoint.content_sha256().unwrap()
        );
        let mut changed = checkpoint.clone();
        changed.result.outputs[0].push('x');
        assert_ne!(
            changed.content_sha256().unwrap(),
            checkpoint.content_sha256().unwrap()
        );
    }

    #[test]
    fn truncation_tampering_and_version_mismatch_are_refused_whole() {
        let bytes = sample().to_bytes().unwrap();
        for cut in [0, 3, 11, 20, bytes.len() / 2, bytes.len() - 1] {
            let error = Checkpoint::from_bytes(&bytes[..cut]).unwrap_err();
            assert!(error.contains("truncated"), "cut {cut}: {error}");
        }
        let mut longer = bytes.clone();
        longer.push(0);
        assert!(
            Checkpoint::from_bytes(&longer)
                .unwrap_err()
                .contains("trailing")
        );
        // Flip one byte anywhere: the header or some section's digest catches it.
        for position in (12..bytes.len()).step_by(7) {
            let mut tampered = bytes.clone();
            tampered[position] ^= 0x40;
            let error = Checkpoint::from_bytes(&tampered).unwrap_err();
            assert!(
                error.contains("SHA-256")
                    || error.contains("truncated")
                    || error.contains("trailing")
                    || error.contains("decode")
                    || error.contains("section list")
                    || error.contains("implausible"),
                "position {position}: {error}"
            );
        }
        let mut version = bytes.clone();
        version[3] = 2;
        assert!(
            Checkpoint::from_bytes(&version)
                .unwrap_err()
                .contains("unsupported `.qo` format version 2")
        );
        let mut magic = bytes.clone();
        magic[0] = b'X';
        assert!(
            Checkpoint::from_bytes(&magic)
                .unwrap_err()
                .contains("bad magic")
        );
    }

    #[test]
    fn old_and_future_schema_versions_are_refused() {
        let sections = sample().encode_sections().unwrap();
        for version in [2, 3, SCHEMA_VERSION + 1] {
            let header = Header {
                schema_version: version,
                sections: sections
                    .iter()
                    .map(|(name, bytes)| SectionEntry {
                        name: (*name).to_owned(),
                        length: bytes.len() as u64,
                        sha256: sha256_bytes(bytes),
                    })
                    .collect(),
            };
            let header_bytes = encode("header", &header).unwrap();
            let mut out = prelude().to_vec();
            out.extend_from_slice(&(header_bytes.len() as u32).to_le_bytes());
            out.extend_from_slice(&header_bytes);
            out.extend_from_slice(&sha256_bytes(&out));
            for (_, bytes) in &sections {
                out.extend_from_slice(bytes);
            }
            assert!(
                Checkpoint::from_bytes(&out)
                    .unwrap_err()
                    .contains(&format!("unsupported checkpoint schema version {version}"))
            );
        }
    }

    #[test]
    fn both_text_forms_round_trip_to_the_same_checkpoint_and_bytes() {
        let checkpoint = sample();
        let bytes = checkpoint.to_bytes().unwrap();
        for format in [TextFormat::Yaml, TextFormat::Toml] {
            let text = decompose(&checkpoint, format).unwrap();
            let back = parse_text(&text, format).unwrap();
            assert_eq!(back, checkpoint, "{format:?}");
            assert_eq!(back.to_bytes().unwrap(), bytes, "{format:?}");
        }
    }

    #[test]
    fn an_edited_text_is_refused() {
        let checkpoint = sample();
        for format in [TextFormat::Yaml, TextFormat::Toml] {
            let text = decompose(&checkpoint, format).unwrap();
            let edited = text.replace("cc -c a.c", "cc -c b.c");
            assert_ne!(edited, text);
            let error = parse_text(&edited, format).unwrap_err();
            assert!(error.contains("preparation"), "{format:?}: {error}");
        }
    }

    #[test]
    fn the_reader_reads_sections_one_at_a_time_and_checks_each() {
        let directory =
            std::env::temp_dir().join(format!("qcore-checkpoint-reader-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join("lib.qo");
        let checkpoint = sample();
        let bytes = checkpoint.to_bytes().unwrap();
        std::fs::write(&path, &bytes).unwrap();
        let mut reader = CheckpointReader::open(&path).unwrap();
        reader.verify().unwrap();
        assert_eq!(
            reader.section::<ResultSection>("result").unwrap(),
            checkpoint.result
        );
        assert_eq!(
            reader.content_sha256().unwrap(),
            checkpoint.content_sha256().unwrap()
        );
        // A section tampered after opening is refused when read.
        let mut tampered = bytes.clone();
        let last = tampered.len() - 1;
        tampered[last] ^= 1;
        std::fs::write(&path, &tampered).unwrap();
        assert!(reader.verify().unwrap_err().contains("SHA-256"));
        std::fs::write(&path, &bytes[..bytes.len() - 3]).unwrap();
        assert!(
            CheckpointReader::open(&path)
                .err()
                .unwrap()
                .contains("truncated")
        );
        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn the_report_frame_splices_rows_exactly() {
        let document = json!({"a": 1, "units": [], "diagnostic_counts": {"x": 2}});
        let (head, tail) = report_frame(&document).unwrap();
        let rows = [json!({"file": "a", "n": [1]}), json!({"file": "b"})];
        let mut out = head.clone();
        out.push_str("[\n");
        for (index, row) in rows.iter().enumerate() {
            if index > 0 {
                out.push_str(",\n");
            }
            out.push_str("    ");
            let steps = split_row(row).unwrap();
            render_row([&steps[0], &steps[1], &steps[2]], 2, &mut out).unwrap();
        }
        out.push_str("\n  ]");
        out.push_str(&tail);
        let mut whole = document.clone();
        whole["units"] = json!(rows);
        let mut expected = serde_json::to_string_pretty(&whole).unwrap();
        expected.push('\n');
        assert_eq!(out, expected);
        assert!(report_frame(&json!({"units": [], "zz": 1})).is_err());
    }
}

//! Proposed Boolean-field and frame paths at the provider/contract boundary.
//!
//! These are untrusted serializable records, not source-import certificates.
//! A receiver must establish source/build/identity binding separately. Policy
//! evaluation checks the supplied paths only; it cannot certify their producer.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Additive record vocabulary version, not a newly admitted protocol witness.
pub const CHUNK_SUMMARY_RECORD_VERSION: u16 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChunkSourceField {
    Busy,
    Locked,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChunkSourcePath {
    pub initial_value: bool,
    pub final_value: bool,
    pub return_value: i32,
    /// Taken/not-taken decisions in source execution order.
    pub branches: Vec<bool>,
    /// Every field assignment, including idempotent writes, in source order.
    pub writes: Vec<bool>,
    /// Proposed helper writes outside the modeled Boolean field.
    pub frame_writes: Vec<(String, i32)>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChunkSourceSummaryRecord {
    pub function: String,
    pub source_revision: String,
    pub field: ChunkSourceField,
    pub paths: [ChunkSourcePath; 2],
    pub dependencies: BTreeMap<String, String>,
    pub profile_assumptions: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChunkFrameSummaryRecord {
    pub function: String,
    pub source: String,
    pub layout: Vec<(String, String)>,
    pub writes: Vec<(String, i32)>,
}

/// Restricted source projection supplied by the trusted importer to a provider.
/// This raw record conveys no import authority by itself. Its digest is pinned
/// in the core task and its paths are independently checked during replay.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BooleanFieldSourceInput {
    pub summary: ChunkSourceSummaryRecord,
}

pub const BOOLEAN_FIELD_SOURCE_INPUT_KEY: &str = "qcore:boolean-field-source-input:v1";

impl BooleanFieldSourceInput {
    pub fn digest(&self) -> String {
        crate::digest::sha256_hex(
            &serde_json::to_vec(self).expect("serializable source projection"),
        )
    }
}

/// Proposed finite-state source results. The trusted checker also evaluates the
/// pinned contract; equality with this proposal alone never discharges it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BooleanFieldContractWitness {
    pub task_digest: String,
    pub summary: ChunkSourceSummaryRecord,
}

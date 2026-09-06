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

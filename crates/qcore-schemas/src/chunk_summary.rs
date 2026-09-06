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

/// Typed, bounded source expression vocabulary. These records are data only;
/// producer evaluation and trusted source import remain separate implementations.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BooleanFieldExpression {
    Integer(i32),
    Field,
    Not(Box<Self>),
    Negate(Box<Self>),
    Equal(Box<Self>, Box<Self>),
    NotEqual(Box<Self>, Box<Self>),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BooleanFieldStatement {
    Block(Vec<Self>),
    If(BooleanFieldExpression, Box<Self>, Option<Box<Self>>),
    Assign(BooleanFieldExpression),
    Return(BooleanFieldExpression),
    FrameWrites(Vec<(String, i32)>),
}

/// Source program supplied by the trusted importer, without expected results.
/// A raw record conveys no import authority. Its complete digest is pinned in
/// the task; the receiver checks proposed paths against its original import.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BooleanFieldSourceInput {
    pub schema_version: u16,
    pub function: String,
    pub source_revision: String,
    pub field: ChunkSourceField,
    pub program: BooleanFieldStatement,
    pub dependencies: BTreeMap<String, String>,
    pub profile_assumptions: BTreeMap<String, String>,
}

pub const BOOLEAN_FIELD_SOURCE_INPUT_VERSION: u16 = 2;
pub const BOOLEAN_FIELD_SOURCE_INPUT_KEY: &str = "qcore:boolean-field-source-input:v2";

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

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

//! Proposed Boolean-field and frame paths at the provider/contract boundary.
//!
//! These are untrusted serializable records, not source-import certificates.
//! A receiver must establish source/build/identity binding separately. Policy
//! evaluation checks the supplied paths only; it cannot certify their producer.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Additive record vocabulary version, not a newly admitted protocol witness.
pub const CHUNK_SUMMARY_RECORD_VERSION: u16 = 1;

/// C result ABI of the imported function. Unit paths carry zero only as an
/// internal path-record placeholder; contracts may not refer to `result`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BooleanFieldResult {
    #[default]
    I32,
    Unit,
}

impl BooleanFieldResult {
    pub fn is_i32(&self) -> bool {
        *self == Self::I32
    }
}

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
    #[serde(default, skip_serializing_if = "BooleanFieldResult::is_i32")]
    pub result_kind: BooleanFieldResult,
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
    ReturnUnit,
    FrameWrites(Vec<(String, i32)>),
}

/// Source program supplied by the trusted importer, without expected results.
/// A raw record conveys no import authority. Its complete digest is pinned in
/// the task; the receiver checks proposed paths against its original import.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BooleanFieldSourceInput {
    #[serde(default, skip_serializing_if = "BooleanFieldResult::is_i32")]
    pub result_kind: BooleanFieldResult,
    pub schema_version: u16,
    pub function: String,
    pub source_revision: String,
    pub field: ChunkSourceField,
    pub program: BooleanFieldStatement,
    pub dependencies: BTreeMap<String, String>,
    pub profile_assumptions: BTreeMap<String, String>,
}

pub const BOOLEAN_FIELD_SOURCE_INPUT_VERSION: u16 = 2;
pub const BOOLEAN_FIELD_UNIT_SOURCE_INPUT_VERSION: u16 = 3;
pub const BOOLEAN_FIELD_UNIT_SOURCE_INPUT_KEY: &str = "qcore:boolean-field-source-input:v3";
pub const BOOLEAN_FIELD_SOURCE_INPUT_KEY: &str = "qcore:boolean-field-source-input:v2";

impl BooleanFieldSourceInput {
    pub fn version_matches_result(&self) -> bool {
        self.schema_version
            == match self.result_kind {
                BooleanFieldResult::I32 => BOOLEAN_FIELD_SOURCE_INPUT_VERSION,
                BooleanFieldResult::Unit => BOOLEAN_FIELD_UNIT_SOURCE_INPUT_VERSION,
            }
    }
    pub fn digest_key(&self) -> &'static str {
        match self.result_kind {
            BooleanFieldResult::I32 => BOOLEAN_FIELD_SOURCE_INPUT_KEY,
            BooleanFieldResult::Unit => BOOLEAN_FIELD_UNIT_SOURCE_INPUT_KEY,
        }
    }

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

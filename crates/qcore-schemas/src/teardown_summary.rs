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

//! Portable source projections and proposed readings for two bounded ChunkIO
//! teardown families: the intrusive-list unlink (an entry function plus the
//! complete body of the one helper it calls) and the nullable-field backend
//! destructor.
//!
//! These are untrusted serializable records, not source-import certificates.
//! A projection carries the *question* — imported typed paths, the declared C
//! types, and for the destructor family the owned-field inventory the pinned
//! contract declares — and never a derived result. A receiver must establish
//! source/build/identity binding separately; a successful comparison proves
//! only that a producer recomputed the same structural reading from the
//! supplied paths. Contract evaluation, ring membership, exclusive access,
//! allocation provenance and the `free`/libc binding stay with the trusted
//! core, which evaluates the pinned contract over its own import.
//!
//! The call vocabulary is deliberately *not* general. Each family names a
//! closed callee table (the co-transported helper for the unlink family,
//! exactly `free` for the destructor family) and a projection carrying any
//! other call keeps its function Unknown at import.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Required schema version of a teardown source projection.
pub const TEARDOWN_SOURCE_INPUT_VERSION: u16 = 1;

/// Complete-input digest key bound into the verification task.
pub const TEARDOWN_SOURCE_INPUT_KEY: &str = "qcore:teardown-source-input:v1";

/// The bounded typed expression vocabulary a teardown path may use. Anything
/// else keeps its function Unknown at import; there is no fallback encoding.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TeardownExpressionRecord {
    Parameter {
        index: u32,
    },
    /// A value produced by an earlier operation of this same path.
    Symbol {
        operation: u32,
    },
    /// Decimal literal text, never a host-evaluated number.
    Integer {
        value: String,
    },
    Field {
        base: Box<TeardownValueRecord>,
        name: String,
    },
    Cast {
        cast: String,
        value: Box<TeardownValueRecord>,
    },
    Unary {
        operator: String,
        value: Box<TeardownValueRecord>,
    },
    Binary {
        operator: String,
        left: Box<TeardownValueRecord>,
        right: Box<TeardownValueRecord>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TeardownValueRecord {
    pub c_type: String,
    pub expression: TeardownExpressionRecord,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TeardownOperationRecord {
    Read {
        place: TeardownValueRecord,
    },
    Write {
        place: TeardownValueRecord,
        value: TeardownValueRecord,
    },
    LocalWrite {
        local: u32,
        value: TeardownValueRecord,
    },
    LocalEnd {
        local: u32,
    },
    Guard {
        condition: TeardownValueRecord,
        taken: bool,
    },
    /// A call the family's closed callee table admits. The projector refuses
    /// every other callee, so no opaque call ever reaches this vocabulary.
    Call {
        function: String,
        signature: String,
        arguments: Vec<TeardownValueRecord>,
        result_type: String,
    },
    Return,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TeardownPathRecord {
    pub operations: Vec<TeardownOperationRecord>,
}

/// One imported function's complete path set, without any derived result.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TeardownFunctionRecord {
    pub function: String,
    pub source_revision: String,
    /// The imported C signature, exactly as the source declared it.
    pub signature: String,
    pub paths: Vec<TeardownPathRecord>,
    pub profile_assumptions: BTreeMap<String, String>,
}

/// Which bounded teardown question a projection asks. Each family fixes its
/// own closed callee table, its own proposal shape and its own obligation
/// property; a projection of one family can never be read as the other.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "family", rename_all = "snake_case")]
pub enum TeardownFamilyInput {
    /// The unlink entry together with the complete body of the single helper
    /// it calls. Both bodies travel; neither summary does.
    ListUnlink {
        entry: TeardownFunctionRecord,
        helper: TeardownFunctionRecord,
        node_c_type: String,
    },
    /// One nullable-field destructor plus the owned-field inventory the pinned
    /// contract declares. The inventory is part of the question — it selects
    /// which nullable states must be enumerated — and the core refuses a task
    /// whose projection does not name exactly the contract's own inventory.
    FieldDestructor {
        destructor: TeardownFunctionRecord,
        root_c_type: String,
        declared_owned_fields: Vec<String>,
    },
}

impl TeardownFamilyInput {
    /// The function the single obligation of this projection is about.
    #[must_use]
    pub fn subject_function(&self) -> &str {
        match self {
            Self::ListUnlink { entry, .. } => &entry.function,
            Self::FieldDestructor { destructor, .. } => &destructor.function,
        }
    }
}

/// One transported teardown question. A raw record conveys no import
/// authority: its complete digest is pinned in the task, and the receiver
/// checks a proposal against its own import.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TeardownSourceInput {
    pub schema_version: u16,
    pub family: TeardownFamilyInput,
}

impl TeardownSourceInput {
    #[must_use]
    pub fn version_matches(&self) -> bool {
        self.schema_version == TEARDOWN_SOURCE_INPUT_VERSION
    }

    #[must_use]
    pub const fn digest_key(&self) -> &'static str {
        TEARDOWN_SOURCE_INPUT_KEY
    }

    #[must_use]
    pub fn subject_function(&self) -> &str {
        self.family.subject_function()
    }

    #[must_use]
    pub fn digest(&self) -> String {
        crate::digest::sha256_hex(
            &serde_json::to_vec(self).expect("serializable teardown source projection"),
        )
    }
}

/// One link-cell write: which node, which link field, and the node the field
/// now names (`None` for a null store).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LinkWriteRecord {
    pub node: u32,
    pub field: String,
    pub value: Option<u32>,
}

/// One neighbor-alias class of the unlink replay.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnlinkCaseRecord {
    pub neighbors_alias: bool,
    pub helper_calls: u32,
    pub writes: Vec<LinkWriteRecord>,
}

/// A proposed structural reading of one unlink entry and its helper.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListUnlinkRecord {
    pub function: String,
    pub helper_function: String,
    pub source_revision: String,
    pub node_c_type: String,
    pub cases: Vec<UnlinkCaseRecord>,
    pub profile_assumptions: BTreeMap<String, String>,
}

/// One realizable nullable state of the destructor replay.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldReleaseCaseRecord {
    pub initially_live: Vec<String>,
    pub released_in_order: Vec<String>,
    pub source_path: u32,
}

/// A proposed structural reading of one nullable-field destructor.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldDestructorRecord {
    pub function: String,
    pub source_revision: String,
    pub root_c_type: String,
    pub declared_owned_fields: Vec<String>,
    pub cases: Vec<FieldReleaseCaseRecord>,
    pub profile_assumptions: BTreeMap<String, String>,
}

/// The proposal a producer returns. Equality with the core's own reading binds
/// the proposal to the import; it never discharges the pinned contract, which
/// the core evaluates separately over its own summaries.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "family", rename_all = "snake_case")]
pub enum TeardownProposalRecord {
    ListUnlink(ListUnlinkRecord),
    FieldDestructor(FieldDestructorRecord),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TeardownContractWitness {
    pub task_digest: String,
    pub summary: TeardownProposalRecord,
}

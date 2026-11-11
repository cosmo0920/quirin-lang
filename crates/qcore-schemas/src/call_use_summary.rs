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

//! Portable call-result def/use projections and proposed readings for one
//! bounded ChunkIO family: the filesystem cleanup *attempt*.
//!
//! This is the third transported source vocabulary, beside the Boolean/counter
//! path projections and the teardown projections. It exists because this
//! capability's evidence is a **def/use** reading — which call results are
//! ignored, which are guarded, and in what operation order — and not a path
//! replay. Reusing either existing vocabulary would either lose the operation
//! ordering a def/use reading needs, or silently widen a family whose call
//! table is deliberately closed; so this module defines its own enums and
//! shares nothing with them.
//!
//! These are untrusted serializable records, not source-import certificates.
//! A projection carries the *question* — one imported function's complete
//! typed path set, exactly as the importer read it — and never a derived
//! result: no call inventory, no use lists, no fact vector and no verdict.
//! A receiver must establish source/build/identity binding separately; a
//! successful comparison proves only that a producer recomputed the same
//! def/use reading from the supplied paths. Contract evaluation, OS cleanup
//! success, descriptor and mapping custody stay with the trusted core, which
//! evaluates the pinned contract over its own import.
//!
//! The vocabulary is closed by construction. Every expression form, operation
//! form, call form, result binding and branch shape it can represent is listed
//! below; the gateway projector refuses everything else by name, so a body
//! outside this vocabulary keeps its existing `Unknown`. In particular, calls
//! are additionally admitted only against an explicit closed callee table
//! (name **and** declared C signature) supplied to the importer and bound into
//! the verification task, so no general call vocabulary is opened here either.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Required schema version of a call-result def/use projection.
pub const CALL_USE_SOURCE_INPUT_VERSION: u16 = 1;

/// Complete-input digest key bound into the verification task.
pub const CALL_USE_SOURCE_INPUT_KEY: &str = "qcore:call-use-source-input:v1";

/// The bounded typed expression vocabulary a projected path may use.
///
/// Deliberately excluded, each refused by name at projection: `LocalStorage`
/// (an address-taken stack slot), `LoopIndex`, `SizeOf`, `Aggregate`,
/// `Index` (a subscript) and `Function` (a callee designator used as a value).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CallUseExpressionRecord {
    Parameter {
        index: u32,
    },
    /// A value produced by an earlier operation of this same path. This is the
    /// only form that makes a call result observable, so it is what the
    /// def/use reading follows.
    Symbol {
        operation: u32,
    },
    /// Decimal literal text, never a host-evaluated number.
    Integer {
        value: String,
    },
    /// String literal text, retained verbatim.
    String {
        value: String,
    },
    Field {
        base: Box<CallUseValueRecord>,
        name: String,
    },
    Cast {
        cast: String,
        value: Box<CallUseValueRecord>,
    },
    Unary {
        operator: String,
        value: Box<CallUseValueRecord>,
    },
    Binary {
        operator: String,
        left: Box<CallUseValueRecord>,
        right: Box<CallUseValueRecord>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CallUseValueRecord {
    pub c_type: String,
    pub expression: CallUseExpressionRecord,
}

/// The bounded operation vocabulary a projected path may use.
///
/// Deliberately excluded, each refused by name at projection: `ScanLoop`
/// (structured loops, whose dependencies the def/use reading cannot resolve —
/// the in-process analyzer refuses them too) and `Terminate` (a declared
/// nonreturning call, which is not a complete path in this family).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CallUseOperationRecord {
    Read {
        place: CallUseValueRecord,
    },
    Write {
        place: CallUseValueRecord,
        value: CallUseValueRecord,
    },
    /// Declares a stack slot; carries no value and produces no symbol.
    LocalStart {
        local: u32,
        c_type: String,
    },
    /// The `int r = f(...)` result binding: the written value is where a call
    /// result becomes a named local.
    LocalWrite {
        local: u32,
        value: CallUseValueRecord,
    },
    LocalEnd {
        local: u32,
    },
    /// The only branch shape this vocabulary represents: one decided condition
    /// on a complete flat path. A use inside a guard condition is additionally
    /// recorded as a guarded use.
    Guard {
        condition: CallUseValueRecord,
        taken: bool,
    },
    /// A call the family's closed callee table admits, with its declared C
    /// signature and result type. The projector refuses every other callee, so
    /// no opaque call ever reaches this vocabulary.
    Call {
        function: String,
        signature: String,
        arguments: Vec<CallUseValueRecord>,
        result_type: String,
    },
    ReturnValue {
        value: CallUseValueRecord,
    },
    Return,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CallUsePathRecord {
    pub operations: Vec<CallUseOperationRecord>,
}

/// One imported function's complete path set, without any derived result.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CallUseFunctionRecord {
    pub function: String,
    pub source_revision: String,
    /// The imported C signature, exactly as the source declared it.
    pub signature: String,
    pub paths: Vec<CallUsePathRecord>,
    pub profile_assumptions: BTreeMap<String, String>,
}

/// Which bounded call-result question a projection asks. The family fixes the
/// proposal shape and the obligation property; a projection can never be read
/// as another family's question.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "family", rename_all = "snake_case")]
pub enum CallUseFamilyInput {
    /// One filesystem cleanup attempt body. Nothing about *which* callee the
    /// pinned contract's policy selects travels: the policy is the trusted
    /// core's own selection, applied when it evaluates the contract over its
    /// own import, so a producer cannot choose a smaller question.
    FilesystemAttempt { body: CallUseFunctionRecord },
}

impl CallUseFamilyInput {
    /// The function the single obligation of this projection is about.
    #[must_use]
    pub fn subject_function(&self) -> &str {
        match self {
            Self::FilesystemAttempt { body } => &body.function,
        }
    }
}

/// One transported call-result question. A raw record conveys no import
/// authority: its complete digest is pinned in the task, and the receiver
/// checks a proposal against its own import.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CallUseSourceInput {
    pub schema_version: u16,
    pub family: CallUseFamilyInput,
}

impl CallUseSourceInput {
    #[must_use]
    pub fn version_matches(&self) -> bool {
        self.schema_version == CALL_USE_SOURCE_INPUT_VERSION
    }

    #[must_use]
    pub const fn digest_key(&self) -> &'static str {
        CALL_USE_SOURCE_INPUT_KEY
    }

    #[must_use]
    pub fn subject_function(&self) -> &str {
        self.family.subject_function()
    }

    #[must_use]
    pub fn digest(&self) -> String {
        crate::digest::sha256_hex(
            &serde_json::to_vec(self).expect("serializable call-use source projection"),
        )
    }
}

/// One call in one path: where it occurs, what it is, and every later
/// operation of the same path that observes its result.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CallUseRecord {
    pub operation: u32,
    pub function: String,
    pub result_type: String,
    /// Operation indices that read the produced value, in path order.
    pub uses: Vec<u32>,
    /// The subset of `uses` that are guard conditions.
    pub guard_uses: Vec<u32>,
}

/// The complete def/use reading of one path.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PathCallUseRecord {
    pub path: u32,
    pub calls: Vec<CallUseRecord>,
}

/// A proposed def/use reading of one imported body. It is policy-free: every
/// call of every path is reported, so a producer cannot answer a narrower
/// question than the one the core evaluates.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CallResultUseRecord {
    pub function: String,
    pub source_revision: String,
    pub signature: String,
    pub paths: Vec<PathCallUseRecord>,
    pub profile_assumptions: BTreeMap<String, String>,
}

/// The proposal a producer returns. Equality with the core's own reading binds
/// the proposal to the import; it never discharges the pinned contract, which
/// the core evaluates separately over its own summary.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "family", rename_all = "snake_case")]
pub enum CallUseProposalRecord {
    FilesystemAttempt(CallResultUseRecord),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CallResultUseWitness {
    pub task_digest: String,
    pub summary: CallUseProposalRecord,
}

/// The identity a *dependent* capability must bind when its own evidence
/// consumes this one's certificate.
///
/// A dependent never receives, and can never be handed, a prerequisite
/// certificate: it receives only this identity triple, and the trusted core
/// re-derives the prerequisite task from its own import and re-runs the
/// prerequisite's own evaluation before the dependent's proposal is looked at.
/// A producer therefore cannot smuggle a prerequisite conclusion, because
/// nothing it sends is ever read as one.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrerequisiteIdentity {
    /// The prerequisite's semantic profile, so a dependent cannot bind a
    /// certificate produced under a different question.
    pub semantic_profile: String,
    /// The prerequisite's own task digest, recomputed by the core.
    pub task_digest: String,
    /// The prerequisite's single obligation identifier.
    pub obligation_id: String,
}

impl PrerequisiteIdentity {
    /// Digest key under which a dependent task binds this identity.
    pub const DIGEST_KEY: &'static str = "qcore:prerequisite-identity:v1";

    #[must_use]
    pub fn is_complete(&self) -> bool {
        !self.semantic_profile.is_empty()
            && !self.task_digest.is_empty()
            && !self.obligation_id.is_empty()
    }

    #[must_use]
    pub fn digest(&self) -> String {
        crate::digest::sha256_hex(
            &serde_json::to_vec(self).expect("serializable prerequisite identity"),
        )
    }
}

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

//! Evidence schema v2: semantic witnesses the trusted core replays.
//!
//! Implements `docs/design/W3_OBLIGATIONS_EVIDENCE_V2.md` §2. A witness is
//! *data a provider proposes*, never a decision. Every acceptance path in the
//! core terminates in a recomputation over core-derived inputs, and the empty
//! case of every quantifier is decided against a core-computed denominator
//! (the anti-vacuousness rule, design §2.2).
//!
//! These types carry no analysis logic. The checkers that replay them live in
//! the trusted core (`qcore-compiler`, `qcore-contract`); the firewall trait
//! they implement lives in the verifier host.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Evidence schema version a v2 witness speaks. The protocol frame version
/// (`PLUGIN_PROTOCOL_VERSION`) is unchanged: only the evidence schema moves.
pub const PLUGIN_EVIDENCE_VERSION_V2: u16 = 2;

/// A call site in the core's own lifecycle inventory. A provider may only
/// refer to sites the core already discovered and numbered.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceSite {
    pub file: String,
    pub span_start: u64,
    pub span_end: u64,
    pub callee: String,
    /// Index into the core's own argument-identity numbering for the unit.
    pub argument_identity: u32,
}

/// Which chunk backend an opened chunk uses. Schema mirror of the compiler's
/// internal `ChunkBackend`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChunkBackendRecord {
    Memory,
    File,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChunkResidencyRecord {
    Up,
    Down,
}

/// Schema mirror of the compiler's `ChunkOperation`. The compiler keeps its
/// internal type and converts; this is the wire form.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case")]
pub enum ChunkOperationRecord {
    CreateContext {
        context: u32,
    },
    CreateStream {
        context: u32,
        stream: u32,
    },
    OpenChunk {
        context: u32,
        stream: u32,
        chunk: u32,
        backend: ChunkBackendRecord,
        residency: ChunkResidencyRecord,
    },
    SetResidency {
        chunk: u32,
        residency: ChunkResidencyRecord,
    },
    Lock {
        chunk: u32,
    },
    Unlock {
        chunk: u32,
    },
    BeginTransaction {
        chunk: u32,
    },
    CommitTransaction {
        chunk: u32,
    },
    RollbackTransaction {
        chunk: u32,
    },
    CloseChunk {
        chunk: u32,
        delete: bool,
    },
    DestroyStream {
        stream: u32,
    },
    DestroyContext {
        context: u32,
    },
}

impl ChunkOperationRecord {
    /// The operation's kind, ignoring provider-chosen parameters. The core
    /// pins the kind and the entity identities from its own API table; only
    /// the remaining parameters are provider freedom (design §2.3 step 3).
    #[must_use]
    pub const fn kind(self) -> &'static str {
        match self {
            Self::CreateContext { .. } => "create_context",
            Self::CreateStream { .. } => "create_stream",
            Self::OpenChunk { .. } => "open_chunk",
            Self::SetResidency { .. } => "set_residency",
            Self::Lock { .. } => "lock",
            Self::Unlock { .. } => "unlock",
            Self::BeginTransaction { .. } => "begin_transaction",
            Self::CommitTransaction { .. } => "commit_transaction",
            Self::RollbackTransaction { .. } => "rollback_transaction",
            Self::CloseChunk { .. } => "close_chunk",
            Self::DestroyStream { .. } => "destroy_stream",
            Self::DestroyContext { .. } => "destroy_context",
        }
    }

    /// The entity identities this operation names, in a canonical order. The
    /// core rejects any witness whose entity identities differ from the ones
    /// it assigned to that site.
    #[must_use]
    pub fn entities(self) -> Vec<(&'static str, u32)> {
        match self {
            Self::CreateContext { context } | Self::DestroyContext { context } => {
                vec![("context", context)]
            }
            Self::CreateStream { context, stream } => {
                vec![("context", context), ("stream", stream)]
            }
            Self::OpenChunk {
                context,
                stream,
                chunk,
                ..
            } => vec![("context", context), ("stream", stream), ("chunk", chunk)],
            Self::SetResidency { chunk, .. }
            | Self::Lock { chunk }
            | Self::Unlock { chunk }
            | Self::BeginTransaction { chunk }
            | Self::CommitTransaction { chunk }
            | Self::RollbackTransaction { chunk }
            | Self::CloseChunk { chunk, .. } => vec![("chunk", chunk)],
            Self::DestroyStream { stream } => vec![("stream", stream)],
        }
    }
}

/// The operation the core's API table pins for a call site, with the entity
/// identities the core assigned.
///
/// Parameters the core deliberately leaves open — a chunk's backend and
/// residency at open, whether a close requests backing deletion — are absent:
/// they are the bounded freedom a provider's witness exercises. Everything
/// present here is checked exactly, so a provider can neither relabel an
/// operation nor retarget it at a different entity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "template", rename_all = "snake_case")]
pub enum ChunkOperationTemplate {
    CreateContext {
        context: u32,
    },
    CreateStream {
        context: u32,
        stream: u32,
    },
    OpenChunk {
        context: u32,
        stream: u32,
        chunk: u32,
    },
    SetResidencyUp {
        chunk: u32,
    },
    SetResidencyDown {
        chunk: u32,
    },
    Lock {
        chunk: u32,
    },
    Unlock {
        chunk: u32,
    },
    BeginTransaction {
        chunk: u32,
    },
    CommitTransaction {
        chunk: u32,
    },
    RollbackTransaction {
        chunk: u32,
    },
    CloseChunk {
        chunk: u32,
    },
    DestroyStream {
        stream: u32,
    },
    DestroyContext {
        context: u32,
    },
}

/// The bounded source fact that carries one core-discovered lifecycle call
/// site to a provider.
///
/// Evidence-v2.0 chunk-lifecycle requests encode one of these per site in a
/// `NormalizedFact`'s `subject` (as compact JSON), which is how a provider
/// learns which sites the core discovered and how the core numbered their
/// entities. It is *information*, not authority: the core re-derives the same
/// inventory when it replays the returned witness, so a provider that alters
/// or invents a site is rejected.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LifecycleSiteFact {
    pub site: SourceSite,
    pub template: ChunkOperationTemplate,
}

impl LifecycleSiteFact {
    /// Encodes the fact for a `NormalizedFact::subject`.
    #[must_use]
    pub fn encode(&self) -> String {
        serde_json::to_string(self).expect("schema types are serializable")
    }

    /// Decodes a `NormalizedFact::subject` produced by [`Self::encode`].
    pub fn decode(subject: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(subject)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoundOperation {
    pub operation: ChunkOperationRecord,
    pub site: SourceSite,
}

/// Witness kind 1 (design §2.3). The provider supplies an ordered schedule of
/// operations bound to core-discovered call sites; the core recomputes site
/// validity, completeness, operation faithfulness, order admissibility, the
/// semantic replay, and obligation discharge.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChunkLifecycleWitness {
    pub task_digest: String,
    pub schedule: Vec<BoundOperation>,
}

/// Identity binding for one recorded native run (reevaluation §6's
/// native-observation set). Recorded observations are a declared trusted
/// assumption, so the binding travels in the certificate.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeRunBinding {
    pub emitted_source_digest: String,
    pub compiler_invocation: String,
    pub executable_digest: String,
    pub exit_status: i64,
    pub seed_or_input: String,
    pub profile: String,
    pub instrumentation_version: String,
}

/// One recorded observation. `values` are data; the predicates evaluated over
/// them come from digest-pinned trusted contract sources, never from here.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContractObservationRecord {
    pub function: String,
    /// The required-hook name this observation claims to satisfy.
    pub hook: String,
    /// Decimal strings on the wire preserve the full signed 128-bit range.
    #[serde(with = "decimal_values")]
    pub values: BTreeMap<String, i128>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservedRun {
    pub run_id: String,
    pub binding: NativeRunBinding,
    pub observations: Vec<ContractObservationRecord>,
}

/// Witness kind 2 (design §2.4). Discharge claims observed agreement for the
/// bound runs only; it is never composed as static source safety.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContractObservationWitness {
    pub task_digest: String,
    pub runs: Vec<ObservedRun>,
}

/// The witness carried by an evidence-v2 proposal. Adding a kind is an
/// additive schema change; changing a kind's meaning needs a version bump.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "witness_kind", rename_all = "snake_case")]
pub enum SemanticWitness {
    ChunkLifecycle(ChunkLifecycleWitness),
    ContractObservation(ContractObservationWitness),
    BooleanFieldContract(Box<crate::chunk_summary::BooleanFieldContractWitness>),
    /// One unsigned unit-counter update recomputed from the core's own typed
    /// path projection. Equality binds the proposal to that import; the core
    /// still evaluates the pinned contract itself.
    CounterContract(Box<crate::counter_summary::CounterContractWitness>),
    /// One teardown reading — an unlink's alias cases or a destructor's
    /// nullable-state release cases — recomputed from the core's own typed
    /// path projection. Equality binds the proposal to that import; the core
    /// still evaluates the pinned contract itself.
    TeardownContract(Box<crate::teardown_summary::TeardownContractWitness>),
    /// One callback assignment inventory recomputed from the core's own
    /// reactor program projection. Equality binds the proposal to that
    /// program; the core still runs its own reactor model checker.
    ReactorCallback(Box<crate::reactor_program::ReactorCallbackWitness>),
    /// One call-result def/use reading recomputed from the core's own typed
    /// path projection. Equality binds the proposal to that import; the core
    /// still evaluates the pinned contract itself.
    CallResultUse(Box<crate::call_use_summary::CallResultUseWitness>),
    /// One symbolic cleanup replay — every sampled prestate's selected path,
    /// initial lattice state, return value, call counts and field transition —
    /// recomputed from the core's own typed path projection and declared
    /// profile. Equality binds the proposal to that import; the core still runs
    /// its own unedited replay and evaluates the pinned contract itself.
    CleanupReplay(Box<crate::cleanup_replay_summary::CleanupReplayWitness>),
    /// One composed reading over separately checked prerequisite capabilities
    /// — every sampled entry state's selected wrapper path, the prerequisite's
    /// own selected path, the return value, the counter decrement and the
    /// wrapper's field writes — recomputed from the core's own typed path
    /// projections. Equality binds the proposal to those imports; the core
    /// still re-establishes every prerequisite and runs its own unedited
    /// composition itself.
    Composition(Box<crate::composition_summary::CompositionWitness>),
}

impl SemanticWitness {
    /// The digest the witness claims to be bound to. A witness bound to a
    /// different task than the one the core computed is rejected before any
    /// capability-specific checking.
    #[must_use]
    pub fn task_digest(&self) -> &str {
        match self {
            Self::ChunkLifecycle(witness) => &witness.task_digest,
            Self::ContractObservation(witness) => &witness.task_digest,
            Self::BooleanFieldContract(witness) => &witness.task_digest,
            Self::CounterContract(witness) => &witness.task_digest,
            Self::TeardownContract(witness) => &witness.task_digest,
            Self::ReactorCallback(witness) => &witness.task_digest,
            Self::CallResultUse(witness) => &witness.task_digest,
            Self::CleanupReplay(witness) => &witness.task_digest,
            Self::Composition(witness) => &witness.task_digest,
        }
    }

    #[must_use]
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::ChunkLifecycle(_) => "chunk_lifecycle",
            Self::ContractObservation(_) => "contract_observation",
            Self::BooleanFieldContract(_) => "boolean_field_contract",
            Self::CounterContract(_) => "counter_contract",
            Self::TeardownContract(_) => "teardown_contract",
            Self::ReactorCallback(_) => "reactor_callback",
            Self::CallResultUse(_) => "call_result_use",
            Self::CleanupReplay(_) => "cleanup_replay",
            Self::Composition(_) => "composition",
        }
    }
}

// Internally tagged witness enums buffer through serde's content model, which
// does not support i128 numbers. Decimal strings also prevent JSON consumers
// from silently rounding values beyond their native integer precision.
mod decimal_values {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::collections::BTreeMap;

    pub fn serialize<S: Serializer>(
        values: &BTreeMap<String, i128>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        values
            .iter()
            .map(|(key, value)| (key, value.to_string()))
            .collect::<BTreeMap<_, _>>()
            .serialize(serializer)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<BTreeMap<String, i128>, D::Error> {
        BTreeMap::<String, String>::deserialize(deserializer)?
            .into_iter()
            .map(|(key, value)| {
                let parsed = value.parse::<i128>().map_err(serde::de::Error::custom)?;
                if parsed.to_string() != value {
                    return Err(serde::de::Error::custom("noncanonical decimal observation"));
                }
                Ok((key, parsed))
            })
            .collect()
    }
}

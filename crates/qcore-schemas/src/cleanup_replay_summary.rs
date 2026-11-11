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

//! Portable symbolic-replay projections and proposed readings for the two
//! bounded ChunkIO **cleanup-outcome** families: the native cleanup outcome
//! (`close`/`munmap`) and the guarded delete (`unlink`).
//!
//! This is the **fourth** transported source vocabulary, beside the
//! Boolean/counter path projections, the teardown projections and the
//! call-result def/use projection. It exists because this capability's evidence
//! is neither a path evaluation nor a def/use fold: the reading a producer must
//! compute (`field_writes`, `fields_preserved`, `fields_reset`, and *which*
//! path each sampled prestate selects) is a function of a symbolic **field
//! state**, evolved by a nine-constructor value lattice over four
//! (respectively five) sampled prestates, with the syscall's argument identity
//! checked against the initial field values. That is a symbolic replay
//! evaluator, and by P04-NEXT's own rule a producer reimplementing it over a
//! new transported vocabulary is a separate capability definition, not a
//! widening of an existing one. So this module defines its own enums and shares
//! nothing with the other three.
//!
//! These are untrusted serializable records, not source-import certificates. A
//! projection carries the *question* — one imported function's complete typed
//! path set plus the profile that fixes the record shape — and never a derived
//! result: no case vector, no field state, no fact vector and no verdict. The
//! contract text and the policy that selects it never leave the trusted core,
//! which re-runs its own unedited replay over its own import and evaluates the
//! pinned contract itself before requiring agreement. Agreement alone
//! discharges nothing.
//!
//! ## Closed by construction
//!
//! Every lattice constructor, prestate form, expression form and step form is
//! an enum variant; the gateway projector refuses everything else **by name**,
//! so a body outside this vocabulary keeps its existing `Unknown`. In
//! particular:
//!
//! * [`CleanupLatticeValue`] has exactly the nine constructors the trusted
//!   replay's own value domain has, and no `Unknown`/`Other` escape hatch;
//! * [`CleanupReplayOperationRecord`] has exactly the eight step forms the
//!   replay executes — a bare `Return`, a scan loop and a nonreturning
//!   termination each have no representation, because the replay requires a
//!   terminal `ReturnValue` on every complete path;
//! * [`CleanupPrestateRecord`] has exactly the two sampled domains, whose
//!   inhabitants are fixed by the family, so a producer cannot answer over a
//!   smaller or a different prestate set.
//!
//! ## What the transported profile is, and is not
//!
//! The profile that travels is an **input** to the replay, not a reading: the
//! record's field inventory, C types, reset values, syscall name and signature,
//! eligibility field, ordered syscall arguments and diagnostic helpers. It is
//! also a **claim**: the trusted core builds its own profile from the pinned
//! contract's `cleanup_vocabulary_v1` declaration (card P04-B1) and refuses
//! unless the transported record is exactly that. A producer therefore cannot
//! choose the record shape it answers over.
//!
//! Nothing here observes or claims OS behaviour. A recorded `close`, `munmap`
//! or `unlink` call is a recorded call; whether a descriptor was released, a
//! mapping torn down or a file deleted is outside every claim this vocabulary
//! can even express.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Required schema version of a cleanup symbolic-replay projection.
pub const CLEANUP_REPLAY_SOURCE_INPUT_VERSION: u16 = 1;

/// Complete-input digest key bound into the verification task.
pub const CLEANUP_REPLAY_SOURCE_INPUT_KEY: &str = "qcore:cleanup-replay-source-input:v1";

/// The semantic profile of both cleanup-replay families.
pub const CLEANUP_REPLAY_SEMANTIC_PROFILE: &str = "qcore.c_cleanup_replay.v1";

/// The closed typed value lattice the symbolic replay evaluates over.
///
/// These are exactly the nine constructors of the trusted checker's own value
/// domain (`qcore_verification::cleanup_outcome`'s private `Value`), in the
/// same order. There is deliberately no catch-all constructor: a value the
/// replay cannot resolve is a refusal, never an opaque token, so a producer
/// cannot report a reading over a domain the core does not share.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "lattice", rename_all = "snake_case")]
pub enum CleanupLatticeValue {
    /// The null pointer, and the only pointer value that is false.
    Null,
    /// The record parameter itself, when the entry state is not null.
    Root,
    /// A live pointer field's opaque identity, named by its field.
    Pointer { field: String },
    /// An exact machine integer.
    Integer { value: i32 },
    /// A non-pointer field's opaque symbolic identity, named by its field.
    Symbol { field: String },
    /// The result of the declared errno accessor.
    ErrnoPointer,
    /// The integer read through the errno pointer.
    Errno,
    /// A string literal, or a pointer decayed from one.
    Text,
    /// The result of the declared diagnostic helper.
    ReportResult,
}

/// What a successful cleanup leaves in one field. The closed reset vocabulary
/// the pinned contract declares.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "reset", rename_all = "snake_case")]
pub enum CleanupResetRecord {
    Null,
    Integer { value: i32 },
}

/// One declared field of the record the replay evolves.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CleanupFieldRecord {
    pub name: String,
    pub c_type: String,
    pub reset: CleanupResetRecord,
}

/// The record shape, syscall and helper identities the replay runs against.
///
/// This is a transported **input**, and simultaneously a claim: the trusted
/// core derives the same shape from its own pinned contract's declared
/// `cleanup_vocabulary_v1` clauses and refuses unless this record is exactly
/// that. The contract's *policy name* is deliberately not a member — which
/// declaration the core reads is the core's own selection and never travels.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CleanupProfileRecord {
    pub function: String,
    pub root_type: String,
    pub syscall: String,
    pub syscall_signature: String,
    pub presence_field: String,
    pub fields: Vec<CleanupFieldRecord>,
    pub argument_fields: Vec<String>,
    pub errno_function: String,
    pub report_function: String,
}

/// The bounded typed expression vocabulary a projected path may use.
///
/// Deliberately excluded, each refused by name at projection: `LocalStorage`
/// (an address-taken stack slot), `LoopIndex`, `SizeOf`, `Aggregate`, `Index`
/// (a subscript) and `Function` (a callee designator used as a value).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CleanupReplayExpressionRecord {
    Parameter {
        index: u32,
    },
    /// A value produced by an earlier operation of this same path — the only
    /// form that makes a call result or a field read observable.
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
        base: Box<CleanupReplayValueRecord>,
        name: String,
    },
    Cast {
        cast: String,
        value: Box<CleanupReplayValueRecord>,
    },
    Unary {
        operator: String,
        value: Box<CleanupReplayValueRecord>,
    },
    Binary {
        operator: String,
        left: Box<CleanupReplayValueRecord>,
        right: Box<CleanupReplayValueRecord>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CleanupReplayValueRecord {
    pub c_type: String,
    pub expression: CleanupReplayExpressionRecord,
}

/// The bounded step vocabulary a projected path may use — exactly the eight
/// forms the trusted replay executes.
///
/// Deliberately excluded, each refused by name at projection: `ScanLoop`
/// (structured loops, which the in-process analyzer refuses too), `Terminate`
/// (a declared nonreturning call) and a bare `Return`, because a cleanup path
/// without a terminal `ReturnValue` is not a complete path in either family.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CleanupReplayOperationRecord {
    Read {
        place: CleanupReplayValueRecord,
    },
    Write {
        place: CleanupReplayValueRecord,
        value: CleanupReplayValueRecord,
    },
    LocalStart {
        local: u32,
        c_type: String,
    },
    LocalWrite {
        local: u32,
        value: CleanupReplayValueRecord,
    },
    LocalEnd {
        local: u32,
    },
    /// The only branch shape: one decided condition on a complete flat path.
    /// A prestate whose lattice values make the condition disagree with
    /// `taken` does not select this path.
    Guard {
        condition: CleanupReplayValueRecord,
        taken: bool,
    },
    Call {
        function: String,
        signature: String,
        arguments: Vec<CleanupReplayValueRecord>,
        result_type: String,
    },
    /// The terminal step of every complete path in both families.
    ReturnValue {
        value: CleanupReplayValueRecord,
    },
    /// A value-free `return` from a `void` function.
    ///
    /// Added by card `P04-COMPOSE-A` for the composition route's `void`
    /// wrapper. **The cleanup-replay route does not admit it in either
    /// direction**: its gateway projector refuses `SourceOperation::Return` by
    /// name and its producer declines this record by name, so widening the
    /// shared enum did not widen that family's vocabulary. That refusal is
    /// pinned by test on both sides (`P04-COMPOSE-SHARED-PATHS`).
    Return,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CleanupReplayPathRecord {
    pub operations: Vec<CleanupReplayOperationRecord>,
}

/// One imported function's complete path set, without any derived result.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CleanupReplayFunctionRecord {
    pub function: String,
    pub source_revision: String,
    /// The imported C signature, exactly as the source declared it.
    pub signature: String,
    pub paths: Vec<CleanupReplayPathRecord>,
    pub profile_assumptions: BTreeMap<String, String>,
}

/// The record's eligibility state at entry, for the native cleanup family.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CleanupEntryRecord {
    /// The record pointer itself is null.
    Null,
    /// The record is live and the eligibility field already holds its reset
    /// value, so no cleanup is due.
    Absent,
    /// The record is live and the eligibility field is set.
    Present,
}

/// One sampled prestate the replay starts from. The inhabitants of each
/// variant are fixed by the family, so a producer cannot answer over a smaller
/// or a differently sampled domain.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "prestate", rename_all = "snake_case")]
pub enum CleanupPrestateRecord {
    /// Native cleanup outcome: the four sampled `(entry, syscall outcome)`
    /// pairs. `syscall_result` is `None` exactly when the path is not expected
    /// to reach the syscall.
    NativeCleanupOutcome {
        entry: CleanupEntryRecord,
        syscall_result: Option<i32>,
    },
    /// Guarded delete: the five sampled
    /// `(descriptor active, mapping active, syscall outcome)` triples.
    GuardedDelete {
        descriptor_field_active: bool,
        mapping_field_active: bool,
        syscall_result: Option<i32>,
    },
}

impl CleanupPrestateRecord {
    /// The complete sampled domain of the native cleanup outcome family, in
    /// the exact order the trusted checker enumerates it.
    #[must_use]
    pub fn native_cleanup_outcome_domain() -> Vec<Self> {
        [
            (CleanupEntryRecord::Null, None),
            (CleanupEntryRecord::Absent, None),
            (CleanupEntryRecord::Present, Some(-1)),
            (CleanupEntryRecord::Present, Some(0)),
        ]
        .into_iter()
        .map(|(entry, syscall_result)| Self::NativeCleanupOutcome {
            entry,
            syscall_result,
        })
        .collect()
    }

    /// The complete sampled domain of the guarded delete family, in the exact
    /// order the trusted checker enumerates it.
    #[must_use]
    pub fn guarded_delete_domain() -> Vec<Self> {
        [
            (true, false, None),
            (true, true, None),
            (false, true, None),
            (false, false, Some(-1)),
            (false, false, Some(0)),
        ]
        .into_iter()
        .map(
            |(descriptor_field_active, mapping_field_active, syscall_result)| Self::GuardedDelete {
                descriptor_field_active,
                mapping_field_active,
                syscall_result,
            },
        )
        .collect()
    }
}

/// Which bounded cleanup-replay question a projection asks. The family fixes
/// the sampled prestate domain, the proposal shape and the obligation
/// property; a projection can never be read as the other family's question.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "family", rename_all = "snake_case")]
pub enum CleanupReplayFamilyInput {
    /// `cio_file_native_close` / `cio_file_native_unmap`: four sampled
    /// prestates over an eligibility field the successful arm resets.
    NativeCleanupOutcome {
        #[serde(deserialize_with = "value_returning_body")]
        body: CleanupReplayFunctionRecord,
        profile: CleanupProfileRecord,
    },
    /// `cio_file_native_delete`: five sampled prestates over two eligibility
    /// fields, where every arm preserves the record.
    GuardedDelete {
        #[serde(deserialize_with = "value_returning_body")]
        body: CleanupReplayFunctionRecord,
        profile: CleanupProfileRecord,
        /// The second eligibility field, which is not a profile member. It is
        /// declared by the delete policy alone, and the core refuses a
        /// projection naming any other field.
        mapping_field: String,
    },
}

/// Refuse a value-free `return` in a cleanup-replay body **at parse time**.
///
/// Card `P04-COMPOSE-A` added
/// [`CleanupReplayOperationRecord::Return`] to the shared operation vocabulary
/// for the composition route's `void` wrapper. Before that this family's bodies
/// refused a bare `return` because the enum had no spelling for it, and that
/// refusal is a fail-closed gate this family still needs: none of its bodies is
/// `void`-returning, and a projection carrying one is malformed, not merely
/// unreplayable.
///
/// Widening the shared enum would otherwise have moved that refusal one layer
/// in — from the deserializer to the producer's decline and the projector's
/// omission. It is restored exactly where it was instead, so
/// `the_closed_vocabulary_refuses_every_unrepresented_form` keeps asserting a
/// parse-time refusal rather than being relaxed to match the new enum.
fn value_returning_body<'de, D>(deserializer: D) -> Result<CleanupReplayFunctionRecord, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error as _;
    let body = CleanupReplayFunctionRecord::deserialize(deserializer)?;
    if body.paths.iter().any(|path| {
        path.operations
            .iter()
            .any(|operation| matches!(operation, CleanupReplayOperationRecord::Return))
    }) {
        return Err(D::Error::custom(
            "a cleanup replay body has no value-free `return`: that operation form belongs to \
             the composition route's `void` wrapper alone",
        ));
    }
    Ok(body)
}

impl CleanupReplayFamilyInput {
    /// The function the single obligation of this projection is about.
    #[must_use]
    pub fn subject_function(&self) -> &str {
        match self {
            Self::NativeCleanupOutcome { body, .. } | Self::GuardedDelete { body, .. } => {
                &body.function
            }
        }
    }

    #[must_use]
    pub fn body(&self) -> &CleanupReplayFunctionRecord {
        match self {
            Self::NativeCleanupOutcome { body, .. } | Self::GuardedDelete { body, .. } => body,
        }
    }

    #[must_use]
    pub fn profile(&self) -> &CleanupProfileRecord {
        match self {
            Self::NativeCleanupOutcome { profile, .. } | Self::GuardedDelete { profile, .. } => {
                profile
            }
        }
    }

    /// The complete sampled prestate domain this family answers over.
    #[must_use]
    pub fn prestates(&self) -> Vec<CleanupPrestateRecord> {
        match self {
            Self::NativeCleanupOutcome { .. } => {
                CleanupPrestateRecord::native_cleanup_outcome_domain()
            }
            Self::GuardedDelete { .. } => CleanupPrestateRecord::guarded_delete_domain(),
        }
    }
}

/// One transported cleanup-replay question.
///
/// A raw record conveys no import authority: its complete digest is pinned in
/// the task, and the receiver checks a proposal against its own import. The
/// `prerequisite` triple is a **lookup key**, never a conclusion — the trusted
/// core re-derives the prerequisite task from its own import, refuses unless
/// the claim is the identity it derives, and re-runs the prerequisite's own
/// contract evaluation before this projection's proposal is looked at.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CleanupReplaySourceInput {
    pub schema_version: u16,
    /// The call-result def/use prerequisite this reading depends on, as an
    /// identity triple carrying no verdict, certificate or case data.
    pub prerequisite: crate::call_use_summary::PrerequisiteIdentity,
    pub family: CleanupReplayFamilyInput,
}

impl CleanupReplaySourceInput {
    #[must_use]
    pub fn version_matches(&self) -> bool {
        self.schema_version == CLEANUP_REPLAY_SOURCE_INPUT_VERSION
    }

    #[must_use]
    pub const fn digest_key(&self) -> &'static str {
        CLEANUP_REPLAY_SOURCE_INPUT_KEY
    }

    #[must_use]
    pub fn subject_function(&self) -> &str {
        self.family.subject_function()
    }

    #[must_use]
    pub fn digest(&self) -> String {
        crate::digest::sha256_hex(
            &serde_json::to_vec(self).expect("serializable cleanup replay projection"),
        )
    }
}

/// The replay outcome of one sampled prestate.
///
/// Each variant mirrors, field for field, the case the trusted checker
/// publishes for its family, plus the lattice-valued initial field state the
/// replay started from. Nothing is invented: a figure the checker does not
/// publish for a family has no place in that family's variant.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "case", rename_all = "snake_case")]
pub enum CleanupReplayCaseRecord {
    NativeCleanupOutcome {
        entry: CleanupEntryRecord,
        syscall_result: Option<i32>,
        /// The lattice state the replay began from, keyed by field name.
        initial_fields: BTreeMap<String, CleanupLatticeValue>,
        /// The single complete path this prestate selects.
        source_path: u32,
        returned: i32,
        cleanup_calls: u32,
        /// Written field names, in the order the path writes them.
        field_writes: Vec<String>,
        fields_preserved: bool,
        fields_reset: bool,
    },
    GuardedDelete {
        descriptor_field_active: bool,
        mapping_field_active: bool,
        syscall_result: Option<i32>,
        initial_fields: BTreeMap<String, CleanupLatticeValue>,
        source_path: u32,
        returned: i32,
        syscall_calls: u32,
        diagnostic_calls: u32,
        fields_preserved: bool,
    },
}

impl CleanupReplayCaseRecord {
    /// The prestate this case answers, so a case vector can be checked against
    /// the family's fixed sampled domain without reading any other figure.
    #[must_use]
    pub fn prestate(&self) -> CleanupPrestateRecord {
        match self {
            Self::NativeCleanupOutcome {
                entry,
                syscall_result,
                ..
            } => CleanupPrestateRecord::NativeCleanupOutcome {
                entry: *entry,
                syscall_result: *syscall_result,
            },
            Self::GuardedDelete {
                descriptor_field_active,
                mapping_field_active,
                syscall_result,
                ..
            } => CleanupPrestateRecord::GuardedDelete {
                descriptor_field_active: *descriptor_field_active,
                mapping_field_active: *mapping_field_active,
                syscall_result: *syscall_result,
            },
        }
    }
}

/// A proposed symbolic replay of one imported body over its family's complete
/// sampled prestate domain. It carries no verdict and no contract fact: the
/// core evaluates the pinned contract itself over its own import.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CleanupReplayRecord {
    pub function: String,
    pub source_revision: String,
    pub signature: String,
    pub cases: Vec<CleanupReplayCaseRecord>,
    pub profile_assumptions: BTreeMap<String, String>,
}

/// The proposal a producer returns. Equality with the core's own replay binds
/// the proposal to the import; it never discharges the pinned contract.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "family", rename_all = "snake_case")]
pub enum CleanupReplayProposalRecord {
    NativeCleanupOutcome(CleanupReplayRecord),
    GuardedDelete(CleanupReplayRecord),
}

impl CleanupReplayProposalRecord {
    #[must_use]
    pub fn record(&self) -> &CleanupReplayRecord {
        match self {
            Self::NativeCleanupOutcome(record) | Self::GuardedDelete(record) => record,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CleanupReplayWitness {
    pub task_digest: String,
    pub summary: CleanupReplayProposalRecord,
}

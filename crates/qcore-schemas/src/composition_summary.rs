//! Portable projections and proposed readings for the bounded ChunkIO
//! **composition** capabilities — the routes whose evidence is a reading over
//! *other* capabilities' certificates rather than over one body alone.
//!
//! This is the **fifth** transported source vocabulary, beside the
//! Boolean/counter path projections, the teardown projections, the call-result
//! def/use projection and the cleanup-replay projection. It exists because a
//! composition's reading is neither a path evaluation, a def/use fold nor a
//! single-body symbolic replay: it is a replay of a *wrapper* body whose call
//! results are the readings of separately checked prerequisite capabilities,
//! and whose published cases name the prerequisite's own selected paths.
//!
//! ## What a composition projection may and may not carry
//!
//! It carries the *question* and never a derived result:
//!
//! * the wrapper's complete typed path set;
//! * the prerequisite bodies' complete typed path sets and the record profile
//!   the prerequisite replay evolves — so the producer must recompute the
//!   prerequisite reading itself rather than being handed it;
//! * one [`PrerequisiteIdentity`] triple per prerequisite, which is a **lookup
//!   key**: it carries no verdict, no certificate, no case vector and no
//!   coverage.
//!
//! It never carries a composed result. No case vector, no selected path, no
//! counter reading, no contract text, no policy name and no verdict travel.
//! **A composed certificate is never transported in either direction**, which
//! is the property that distinguishes this vocabulary from a naive
//! "composition" wire format: a producer that could send the prerequisite's
//! conclusion would be asserting the prerequisite, and this vocabulary cannot
//! express that.
//!
//! ## Where the authority is
//!
//! Nowhere in this module. The trusted core re-derives every prerequisite task
//! from its own import, refuses unless each claimed triple *is* the identity it
//! derives, re-runs each prerequisite's own contract evaluation, and only then
//! runs its own unedited composition over its own certificates and requires the
//! proposal to be that same reading. Agreement alone discharges nothing.
//!
//! ## Shared path records, deliberately
//!
//! The bodies travel as [`CleanupReplayFunctionRecord`]s. The wrapper replay
//! executes exactly the same eight step forms and eight expression forms the
//! cleanup replay does, so a private copy of those enums would be a
//! byte-for-byte duplicate with no distinct refusal surface. The *projector*
//! for this route is separate and enforces its own admission — in particular
//! its own closed callee table — and
//! `composition_vocabulary_is_pinned` in `crates/qcore-schemas/tests` pins the
//! exact variant inventory this route admits, so a future widening of the
//! shared enums fails there instead of silently widening this route. Recorded
//! as boundary `P04-COMPOSE-SHARED-PATHS`.
//!
//! Nothing here observes or claims OS behaviour. A recorded `munmap` call is a
//! recorded call; whether a mapping was torn down is outside every claim this
//! vocabulary can express.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::call_use_summary::PrerequisiteIdentity;
use crate::cleanup_replay_summary::{CleanupProfileRecord, CleanupReplayFunctionRecord};

/// Wire version of the transported composition projection.
pub const COMPOSITION_SOURCE_INPUT_VERSION: u16 = 1;

/// Digest key under which a task binds the complete projection.
pub const COMPOSITION_SOURCE_INPUT_KEY: &str = "qcore:composition-source-input:v1";

/// The semantic profile of the one moved composition family.
pub const UNMAP_WRAPPER_SEMANTIC_PROFILE: &str = "qcore.c_unmap_wrapper_composition.v1";

/// The three prerequisites the unmap-wrapper composition depends on, each named
/// separately because each is load-bearing and each is re-established
/// independently by the core.
///
/// * `wrapper_call_use` — the call-result def/use contract of the wrapper body
///   itself (`munmap_file`), the capability card P04-NEXT-AB moved;
/// * `native_cleanup` — the symbolic cleanup replay of the native syscall
///   wrapper the composition calls, the capability card P04-B2 moved;
/// * `counter` — the unsigned counter contract of the decrement the wrapper
///   performs, the capability card P04 moved.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnmapWrapperPrerequisites {
    pub wrapper_call_use: PrerequisiteIdentity,
    pub native_cleanup: PrerequisiteIdentity,
    pub counter: PrerequisiteIdentity,
}

impl UnmapWrapperPrerequisites {
    /// Digest keys under which a dependent task binds each identity. Three
    /// distinct keys, so moving one prerequisite cannot be absorbed by another.
    pub const WRAPPER_CALL_USE_KEY: &'static str = "qcore:composition-prerequisite-call-use:v1";
    pub const NATIVE_CLEANUP_KEY: &'static str = "qcore:composition-prerequisite-cleanup:v1";
    pub const COUNTER_KEY: &'static str = "qcore:composition-prerequisite-counter:v1";

    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.wrapper_call_use.is_complete()
            && self.native_cleanup.is_complete()
            && self.counter.is_complete()
    }

    /// The three identities must be distinct: one prerequisite cannot stand in
    /// for another, and a triple that repeats one identity names two things it
    /// has not established.
    #[must_use]
    pub fn are_distinct(&self) -> bool {
        self.wrapper_call_use != self.native_cleanup
            && self.wrapper_call_use != self.counter
            && self.native_cleanup != self.counter
    }
}

/// Which bounded composition question a projection asks.
///
/// One inhabitant today. The other six ChunkIO compositions are classified in
/// `docs/reports/remaining-p04-compose/README.md`: five are blocked behind a
/// prerequisite whose capability has not moved, and two additionally sit behind
/// the sealed `SourceMatch` boundary.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "family", rename_all = "snake_case")]
pub enum CompositionFamilyInput {
    /// `munmap_file`: eight sampled entry states over the backend's presence,
    /// mappedness, prior sync state, the sync outcome and the native unmap
    /// outcome.
    UnmapWrapper {
        /// The wrapper body's complete typed path set.
        wrapper: CleanupReplayFunctionRecord,
        /// The native cleanup body's complete typed path set. It travels so the
        /// producer must recompute the native reading itself; the native
        /// *reading* never travels.
        native: CleanupReplayFunctionRecord,
        /// The record shape the native replay evolves. Like every transported
        /// profile on these routes it is a claim: the core builds its own from
        /// the pinned contract's declaration and refuses anything else.
        native_profile: CleanupProfileRecord,
    },
}

impl CompositionFamilyInput {
    /// The function the single obligation of this projection is about.
    #[must_use]
    pub fn subject_function(&self) -> &str {
        match self {
            Self::UnmapWrapper { wrapper, .. } => &wrapper.function,
        }
    }

    #[must_use]
    pub fn wrapper(&self) -> &CleanupReplayFunctionRecord {
        match self {
            Self::UnmapWrapper { wrapper, .. } => wrapper,
        }
    }

    #[must_use]
    pub fn native(&self) -> &CleanupReplayFunctionRecord {
        match self {
            Self::UnmapWrapper { native, .. } => native,
        }
    }

    #[must_use]
    pub fn native_profile(&self) -> &CleanupProfileRecord {
        match self {
            Self::UnmapWrapper { native_profile, .. } => native_profile,
        }
    }

    /// The semantic profile this family answers under.
    #[must_use]
    pub const fn semantic_profile(&self) -> &'static str {
        match self {
            Self::UnmapWrapper { .. } => UNMAP_WRAPPER_SEMANTIC_PROFILE,
        }
    }
}

/// One transported composition question.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompositionSourceInput {
    pub schema_version: u16,
    /// Identity triples only. Nothing a prerequisite concluded reaches here.
    pub prerequisites: UnmapWrapperPrerequisites,
    pub family: CompositionFamilyInput,
}

impl CompositionSourceInput {
    #[must_use]
    pub fn version_matches(&self) -> bool {
        self.schema_version == COMPOSITION_SOURCE_INPUT_VERSION
    }

    #[must_use]
    pub const fn digest_key(&self) -> &'static str {
        COMPOSITION_SOURCE_INPUT_KEY
    }

    #[must_use]
    pub fn subject_function(&self) -> &str {
        self.family.subject_function()
    }

    #[must_use]
    pub fn digest(&self) -> String {
        crate::digest::sha256_hex(
            &serde_json::to_vec(self).expect("serializable composition projection"),
        )
    }
}

/// One sampled entry state's composed reading.
///
/// Every field mirrors, field for field, the `UnmapCase` the trusted composition
/// publishes. Nothing is invented here: a figure the checker does not publish
/// has no place in this record.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnmapCaseRecord {
    /// The single wrapper path this entry state selects.
    pub source_path: u32,
    pub backend_present: bool,
    pub initially_mapped: bool,
    pub initially_synced: bool,
    /// `None` when the wrapper does not call the sync frame on this entry.
    pub sync_result: Option<i32>,
    /// `None` when the wrapper does not reach the native cleanup call.
    pub native_result: Option<i32>,
    /// The **prerequisite's** own selected path for that outcome. This is the
    /// one published figure that is a function of another capability's reading,
    /// which is what makes this a composition rather than a body replay.
    pub native_source_path: Option<u32>,
    pub returned: i32,
    pub counter_decrements: u32,
    pub wrapper_writes: Vec<String>,
}

/// One composed reading, with the wrapper's imported identity.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompositionRecord {
    pub function: String,
    pub source_revision: String,
    pub signature: String,
    pub cases: Vec<UnmapCaseRecord>,
    pub profile_assumptions: BTreeMap<String, String>,
}

/// The proposed reading, tagged by family so one family's proposal can never be
/// read as another's.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "family", rename_all = "snake_case")]
pub enum CompositionProposalRecord {
    UnmapWrapper(CompositionRecord),
}

/// The witness envelope, bound to the task digest the core computed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompositionWitness {
    pub task_digest: String,
    pub summary: CompositionProposalRecord,
}

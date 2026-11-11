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
//! and whose published cases name those prerequisites' own readings.
//!
//! Card `P04-COMPOSE-A` extended it to the `cio_file_close` compositions, whose
//! own prerequisite is **another composition**, so the vocabulary is now
//! recursive: a family input may carry a nested family input, and the
//! prerequisite set is tagged by family because the shape genuinely differs
//! (the leaf family names three prerequisites, none a composition; the
//! `file_close` families name three or four, one of which is the leaf
//! composition). The projection version is `2` for that change.
//!
//! Card `P04-COMPOSE-MEMCLOSE` extended it again, to the null / memory-backed
//! `cio_chunk_close` composition. **No sixth vocabulary was added**: that
//! question is the same *kind* of question — a wrapper replay whose admitted
//! frames are separately checked prerequisite capabilities — and the family
//! tagging `P04-COMPOSE-A` introduced already carries a set of a different
//! size and shape per family, so a four-member set needed no new mechanism.
//! The additions are variants and records only; **every existing family's wire
//! bytes, and therefore every existing task digest, are unchanged**, and the
//! projection version stays `2`.
//!
//! What that family *does* add is a transport question no other family faces:
//! its bodies are imported from **derived** text rather than from the declared
//! file. See [`DerivedUnitRecord`], which carries the derivation as a set of
//! claims the core checks against its own derivation and never believes.
//!
//! Card `P04-COMPOSE-FS` extended it once more, to the filesystem arm of the
//! same `cio_chunk_close` wrapper. Again **no new vocabulary was added**: the
//! question is the same kind, and the family tagging already carries a set of a
//! different size and shape per family. Two things are new to it, and both are
//! structural rather than new mechanisms:
//!
//! * **two derived units in one question.** Its own filesystem probe and the
//!   nested memory question's memory probe are derived from one base text under
//!   two different recipes. Neither derived text travels; the core derives both
//!   itself, and additionally requires the two recipes and the two derived texts
//!   to differ, because a filesystem probe derived under the memory recipe would
//!   read the memory discriminant and the two arms would collapse.
//! * **every prerequisite is a composition, and they live in different
//!   translation units.** The wrapper is in the chunk unit and the backend
//!   composition in the file unit, so this family cannot carry the memory
//!   family's single-unit base tie; what ties the backend prerequisite is its
//!   own identity, revision and declared prototype. Recorded as boundary
//!   `P04-COMPOSE-FS-CROSS-UNIT` and published as a task assumption.
//!
//! The additions are variants and records only; **every existing family's wire
//! bytes, and therefore every existing task digest, are unchanged**, and the
//! projection version stays `2`.
//!
//! Card `P04-COMPOSE-UNSUPPORTED` extended it a final time, to the **symbolic**
//! other-backend arm of the same `cio_chunk_close` wrapper. Again no new
//! vocabulary and no new projection version. Three things are new to it:
//!
//! * **it is the first family that imports no body of its own.** Its whole
//!   question is one nested question — the filesystem composition, verbatim —
//!   plus one prerequisite identity. Both bodies it reads (the wrapper and the
//!   filesystem probe) are the nested question's, so restating either here would
//!   have let one question name two bodies.
//! * **its reading is over a symbolic domain, not a sampled constant.** The
//!   backend discriminant on this arm is *any* `int` except the two the two
//!   probes return. No representative integer stands in for it: a comparison the
//!   class cannot decide is a refusal on both sides, never a sampled answer.
//! * **its published conclusion is `Unknown`.** The wrapper's own releases are
//!   read, and the backend's custody and the constructor's reachability are
//!   explicitly *not* established. The record therefore carries the two excluded
//!   constants and the one selected path and nothing else: a figure the checker
//!   does not decide has no place in it, and this family decides fewer figures
//!   than any other because it deliberately concludes less.
//!
//! ## What a composition projection may and may not carry
//!
//! It carries the *question* and never a derived result:
//!
//! * the wrapper's complete typed path set;
//! * the prerequisite bodies' complete typed path sets and the record profiles
//!   the prerequisite replays evolve — so the producer must recompute each
//!   prerequisite reading itself rather than being handed it;
//! * on a family whose prerequisite is **another composition**, that
//!   composition's whole question, verbatim, so the producer must recompute the
//!   nested composed reading too;
//! * one [`PrerequisiteIdentity`] per prerequisite, which is a **lookup key**:
//!   it carries no verdict, no certificate, no case vector and no coverage.
//!
//! It never carries a composed result, at any level. No case vector, no
//! selected path, no case index, no counter reading, no residual
//! classification, no contract text, no policy name and no verdict travel.
//! **A composed certificate is never transported in either direction**, which
//! is the property that distinguishes this vocabulary from a naive
//! "composition" wire format: a producer that could send the prerequisite's
//! conclusion would be asserting the prerequisite, and this vocabulary cannot
//! express that.
//!
//! ## Where the authority is
//!
//! Nowhere in this module. The trusted core re-derives every prerequisite task
//! from its own import, refuses unless each claimed identity *is* the one it
//! derives, re-runs each prerequisite's own contract evaluation — recursively,
//! where a prerequisite is itself a composition — and only then runs its own
//! unedited composition over its own certificates and requires the proposal to
//! be that same reading. Agreement alone discharges nothing, at any level.
//!
//! ## Shared path records, deliberately
//!
//! The bodies travel as [`CleanupReplayFunctionRecord`]s. The wrapper replays
//! execute the same step and expression forms the cleanup replay does, so a
//! private copy of those enums would be a byte-for-byte duplicate with no
//! distinct refusal surface. The *projector* for this route is separate and
//! enforces its own admission — in particular its own closed callee tables —
//! and `composition_vocabulary_is_pinned` in `crates/qcore-schemas/tests` pins
//! the exact variant inventory this route admits, so a future widening of the
//! shared enums fails there instead of silently widening this route. Recorded
//! as boundary `P04-COMPOSE-SHARED-PATHS`.
//!
//! Card `P04-COMPOSE-A` widened one shared enum by exactly one variant, for the
//! `void`-returning `cio_file_close` wrapper: see
//! [`crate::cleanup_replay_summary::CleanupReplayOperationRecord::Return`],
//! which the cleanup-replay route refuses by name in both directions.
//!
//! Nothing here observes or claims OS behaviour. A recorded `munmap` call is a
//! recorded call; whether a mapping was torn down is outside every claim this
//! vocabulary can express.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::call_use_summary::PrerequisiteIdentity;
use crate::cleanup_replay_summary::{CleanupProfileRecord, CleanupReplayFunctionRecord};

/// Wire version of the transported composition projection.
///
/// Moved 1 → 2 by card `P04-COMPOSE-A`, which made the prerequisite set
/// per-family (a `file_close` composition names four prerequisites, one of them
/// another composition) and made the family input recursive. The change is
/// declared rather than silent: version 1 is refused in both directions and the
/// digest key moves with it, so a version-1 witness cannot be replayed against a
/// version-2 task or the reverse.
pub const COMPOSITION_SOURCE_INPUT_VERSION: u16 = 2;

/// Digest key under which a task binds the complete projection.
pub const COMPOSITION_SOURCE_INPUT_KEY: &str = "qcore:composition-source-input:v2";

/// The semantic profile of the leaf composition family.
pub const UNMAP_WRAPPER_SEMANTIC_PROFILE: &str = "qcore.c_unmap_wrapper_composition.v1";

/// The semantic profile of the non-deleting `cio_file_close` composition, whose
/// prerequisites include the unmap-wrapper composition above.
pub const FILE_CLOSE_SEMANTIC_PROFILE: &str = "qcore.c_file_close_composition.v1";

/// The semantic profile of the null / memory-backed `cio_chunk_close`
/// composition, whose four prerequisites are the frame helper, the
/// nullable-field destructor, the list unlink and the unsigned counter.
///
/// It is the first family whose bodies are imported from **derived** text: the
/// backend discriminant the wrapper branches on is a preprocessor macro, so the
/// route appends a constant probe to the unit and reads the expansion as an
/// integer literal. See [`DerivedUnitRecord`].
pub const MEMORY_CLOSE_SEMANTIC_PROFILE: &str = "qcore.c_memory_close_composition.v1";

/// The semantic profile of the filesystem-backed `cio_chunk_close` composition,
/// whose two prerequisites are **both compositions**: the memory-close
/// composition (whose certificate supplies the wrapper reading and the memory
/// discriminant) and the deleting `cio_file_close` composition (whose case
/// vectors supply the backend outcomes).
///
/// It is the first family carrying **two** derived-unit imports in one question
/// — its own filesystem probe, and the memory-close question's own memory probe
/// — and the first whose two prerequisites are checked over **different**
/// translation units. See [`FilesystemChunkClosePrerequisites`].
pub const FILESYSTEM_CHUNK_CLOSE_SEMANTIC_PROFILE: &str =
    "qcore.c_filesystem_chunk_close_composition.v1";

/// The semantic profile of the **symbolic** other-backend arm of the same
/// `cio_chunk_close` wrapper, whose single prerequisite is the filesystem
/// composition above.
///
/// This is the only family whose backend discriminant is not a value at all: it
/// is the class of every `int` except the two the two probes return. Nothing in
/// this vocabulary can name a representative of that class, and the trusted
/// replay refuses — rather than samples — any comparison the class does not
/// decide. Its conclusion is an explicit `Unknown`: the wrapper's own six
/// releases are read, and backend custody and constructor reachability are not.
pub const UNSUPPORTED_CLOSE_SEMANTIC_PROFILE: &str = "qcore.c_unsupported_close_composition.v1";

/// The semantic profile of the deleting `cio_file_close` composition. It answers
/// a strictly larger question than the non-deleting one — the whole non-deleting
/// case vector *plus* the deleting arm — so it is a separate profile, a separate
/// task and a separate obligation, exactly as the two in-process entry points
/// are separate functions.
pub const FILE_CLOSE_DELETE_SEMANTIC_PROFILE: &str = "qcore.c_file_close_delete_composition.v1";

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

/// The four prerequisites the `cio_file_close` compositions depend on.
///
/// * `wrapper_call_use` — the call-result def/use contract of `cio_file_close`
///   itself (card P04-NEXT-AB);
/// * `unmap_composition` — **another composition**, the moved `munmap_file`
///   unmap wrapper (card P04-COMPOSE). This is what makes the file_close route
///   the first two-level composition: re-establishing it re-establishes its own
///   three prerequisites in turn, and the core does every level itself;
/// * `native_cleanup` — the symbolic cleanup replay of `cio_file_native_close`
///   (card P04-B2);
/// * `native_delete` — the guarded-delete replay of `cio_file_native_delete`
///   (card P04-B2), present **exactly** on the deleting entry point. The
///   non-deleting arm makes no deletion attempt, so naming a delete prerequisite
///   there would claim a prerequisite the question does not have.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileClosePrerequisites {
    pub wrapper_call_use: PrerequisiteIdentity,
    pub unmap_composition: PrerequisiteIdentity,
    pub native_cleanup: PrerequisiteIdentity,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_delete: Option<PrerequisiteIdentity>,
}

impl FileClosePrerequisites {
    /// Digest keys under which a dependent task binds each identity. Four
    /// distinct keys, so moving one prerequisite cannot be absorbed by another.
    pub const WRAPPER_CALL_USE_KEY: &'static str =
        "qcore:composition-prerequisite-close-call-use:v1";
    pub const UNMAP_COMPOSITION_KEY: &'static str =
        "qcore:composition-prerequisite-unmap-composition:v1";
    pub const NATIVE_CLEANUP_KEY: &'static str = "qcore:composition-prerequisite-close-cleanup:v1";
    pub const NATIVE_DELETE_KEY: &'static str = "qcore:composition-prerequisite-close-delete:v1";

    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.wrapper_call_use.is_complete()
            && self.unmap_composition.is_complete()
            && self.native_cleanup.is_complete()
            && self.native_delete.as_ref().is_none_or(|d| d.is_complete())
    }

    /// Every named identity must be distinct: one prerequisite cannot stand in
    /// for another, and a set that repeats one identity names things it has not
    /// established.
    #[must_use]
    pub fn are_distinct(&self) -> bool {
        let mut named = vec![
            &self.wrapper_call_use,
            &self.unmap_composition,
            &self.native_cleanup,
        ];
        if let Some(delete) = &self.native_delete {
            named.push(delete);
        }
        let count = named.len();
        named.sort_by(|a, b| {
            (&a.semantic_profile, &a.task_digest, &a.obligation_id).cmp(&(
                &b.semantic_profile,
                &b.task_digest,
                &b.obligation_id,
            ))
        });
        named.dedup();
        named.len() == count
    }
}

/// The four prerequisites the memory-close composition depends on.
///
/// * `reset` — the **frame helper** whose write footprint and preservation the
///   card `P04-FRAME` task decides (`cio_error_reset`);
/// * `backend` — the nullable-field destructor of the memory backend
///   (card `P04-NEXT`);
/// * `unlink` — the two-summary list-unlink relation (card `P04-NEXT`);
/// * `counter` — the unsigned counter contract of the decrement the wrapper
///   performs (card `P04`).
///
/// All four are named, none is optional, and each is re-established
/// independently by the core. Three of the four had no `PrerequisiteIdentity`
/// at all before wave 14; the frame helper had no verification task at all,
/// which is what blocked this family for two waves.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MemoryClosePrerequisites {
    pub reset: PrerequisiteIdentity,
    pub backend: PrerequisiteIdentity,
    pub unlink: PrerequisiteIdentity,
    pub counter: PrerequisiteIdentity,
}

impl MemoryClosePrerequisites {
    /// Digest keys under which a dependent task binds each identity. Four
    /// distinct keys, so moving one prerequisite cannot be absorbed by another.
    pub const RESET_KEY: &'static str = "qcore:composition-prerequisite-memory-reset:v1";
    pub const BACKEND_KEY: &'static str = "qcore:composition-prerequisite-memory-backend:v1";
    pub const UNLINK_KEY: &'static str = "qcore:composition-prerequisite-memory-unlink:v1";
    pub const COUNTER_KEY: &'static str = "qcore:composition-prerequisite-memory-counter:v1";

    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.reset.is_complete()
            && self.backend.is_complete()
            && self.unlink.is_complete()
            && self.counter.is_complete()
    }

    /// The four identities must be distinct: one prerequisite cannot stand in
    /// for another, and a set that repeats one identity names things it has not
    /// established.
    #[must_use]
    pub fn are_distinct(&self) -> bool {
        let mut named = vec![&self.reset, &self.backend, &self.unlink, &self.counter];
        let count = named.len();
        named.sort_by(|a, b| {
            (&a.semantic_profile, &a.task_digest, &a.obligation_id).cmp(&(
                &b.semantic_profile,
                &b.task_digest,
                &b.obligation_id,
            ))
        });
        named.dedup();
        named.len() == count
    }
}

/// The two prerequisites the filesystem-backed `cio_chunk_close` composition
/// depends on. **Both are compositions**, which makes this the first family
/// whose every prerequisite is itself a composed reading.
///
/// * `memory_close` — the null / memory-backed `cio_chunk_close` composition
///   (card `P04-COMPOSE-MEMCLOSE`). Its certificate carries the wrapper body
///   this family replays on the filesystem arm and the memory discriminant this
///   family must not collide with;
/// * `file_close_delete` — the **deleting** `cio_file_close` composition (card
///   `P04-COMPOSE-A`), whose deleting and non-deleting case vectors are the
///   backend outcomes this family crosses its two entry states with.
///
/// The two are checked over **different translation units** — the wrapper lives
/// in the chunk unit and the backend in the file unit — so this family cannot
/// carry the memory-close family's single-unit base tie. What ties the backend
/// prerequisite is its own identity, its declared revision and its declared
/// prototype, all re-derived by the core; recorded as boundary
/// `P04-COMPOSE-FS-CROSS-UNIT`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FilesystemChunkClosePrerequisites {
    pub memory_close: PrerequisiteIdentity,
    pub file_close_delete: PrerequisiteIdentity,
}

impl FilesystemChunkClosePrerequisites {
    /// Digest keys under which a dependent task binds each identity. Two
    /// distinct keys, so moving one prerequisite cannot be absorbed by the
    /// other.
    pub const MEMORY_CLOSE_KEY: &'static str = "qcore:composition-prerequisite-fs-memory-close:v1";
    pub const FILE_CLOSE_DELETE_KEY: &'static str =
        "qcore:composition-prerequisite-fs-file-close-delete:v1";

    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.memory_close.is_complete() && self.file_close_delete.is_complete()
    }

    /// The two identities must be distinct: one composition cannot stand in for
    /// the other, and a pair that repeats one identity names something it has
    /// not established.
    #[must_use]
    pub fn are_distinct(&self) -> bool {
        self.memory_close != self.file_close_delete
    }
}

/// The single prerequisite the symbolic other-backend `cio_chunk_close`
/// composition depends on: the filesystem composition (card `P04-COMPOSE-FS`),
/// which is itself a three-level composed reading.
///
/// One identity, not two, and the reason is measured rather than convenient.
/// This family reads exactly two things out of that prerequisite — the memory
/// composition's own certificate and the filesystem discriminant probe — and
/// both are inside it. Naming the memory composition separately here would name
/// a prerequisite this question does not have: the filesystem composition
/// already re-establishes it, and a second claim about it could disagree with
/// the first.
///
/// Because the set has one member, [`Self::are_distinct`] is **vacuously true**.
/// That is recorded rather than dressed up: the distinctness rule the other
/// three families carry has no work to do here, and the single identity's own
/// completeness and its equality with the identity the core derives are what
/// carry the weight.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnsupportedClosePrerequisites {
    pub filesystem_chunk_close: PrerequisiteIdentity,
}

impl UnsupportedClosePrerequisites {
    /// The digest key under which a dependent task binds this identity. It is
    /// distinct from every other family's, so no other family's prerequisite can
    /// be absorbed under it.
    pub const FILESYSTEM_CHUNK_CLOSE_KEY: &'static str =
        "qcore:composition-prerequisite-unsupported-filesystem:v1";

    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.filesystem_chunk_close.is_complete()
    }

    /// Vacuously true: a one-member set cannot repeat an identity. Kept so the
    /// tagged enum's rule is total, and pinned as vacuous by test.
    #[must_use]
    pub const fn are_distinct(&self) -> bool {
        true
    }
}

/// The identity of one callee a composition's wrapper replay is allowed to
/// resolve, matched on **both** the name and the declared C prototype.
///
/// On the memory-close family these are the four prerequisite bodies' own
/// identities. They travel as part of the *question* — which names the wrapper
/// may call — and never as a reading: the core reads its own from the
/// prerequisite tasks it derived and refuses a transported pair that is not
/// exactly that.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CalleeIdentityRecord {
    pub function: String,
    pub signature: String,
}

impl CalleeIdentityRecord {
    #[must_use]
    pub fn is_complete(&self) -> bool {
        !self.function.is_empty() && !self.signature.is_empty()
    }
}

/// The four prerequisite callee identities a memory-close wrapper replay
/// resolves against.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MemoryCloseCalleeRecord {
    pub reset: CalleeIdentityRecord,
    pub backend: CalleeIdentityRecord,
    pub unlink: CalleeIdentityRecord,
    pub counter: CalleeIdentityRecord,
}

impl MemoryCloseCalleeRecord {
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.reset.is_complete()
            && self.backend.is_complete()
            && self.unlink.is_complete()
            && self.counter.is_complete()
    }

    /// The four callees must name four different functions: a wrapper cannot
    /// discharge two prerequisites with one frame.
    #[must_use]
    pub fn are_distinct(&self) -> bool {
        let mut named = [
            &self.reset.function,
            &self.backend.function,
            &self.unlink.function,
            &self.counter.function,
        ];
        named.sort_unstable();
        named.windows(2).all(|pair| pair[0] != pair[1])
    }
}

/// How a family's bodies were derived from the unit under analysis, when they
/// were **not** imported from the declared file itself.
///
/// This is the memory-close family's genuinely new transport question. The
/// route appends a constant probe to the base unit and imports from the result,
/// so the bodies come from text no file holds. Every field here is a *claim*:
/// the core builds the derived text itself, from its own base unit and its own
/// recipe constant, and refuses unless each transported value is exactly what
/// its own derivation produced. **Derived text never travels**, and a producer's
/// account of a derivation is never authoritative.
///
/// The `base_input_class` field is separate from `derived_input_class` for a
/// measured reason. `CSourceInput::for_derived_text` collapses a declared file
/// to an anonymous text, so the derived unit's own captured identity records
/// neither the spelled input nor the directory the base unit declared: two base
/// units with equal bytes and different declared identities derive
/// byte-identical derived identities. A route that bound only the derived
/// identity would treat them as one task. The dependent core therefore binds
/// the base import identity under its own key as well, and this field makes the
/// erasure visible on the wire rather than implicit.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DerivedUnitRecord {
    /// The recipe appended to the base unit's text, verbatim.
    pub recipe: String,
    /// The function that recipe defines and the projection carries.
    pub recipe_function: String,
    /// Digest of the base unit's own text.
    pub base_source_digest: String,
    /// Digest of the derived text the bodies were imported from.
    pub derived_source_digest: String,
    /// The input identity class the **base** unit declares.
    pub base_input_class: String,
    /// The input identity class the **derived** unit records, which
    /// `for_derived_text` may have collapsed.
    pub derived_input_class: String,
}

impl DerivedUnitRecord {
    #[must_use]
    pub fn is_complete(&self) -> bool {
        !self.recipe.is_empty()
            && !self.recipe_function.is_empty()
            && !self.base_source_digest.is_empty()
            && !self.derived_source_digest.is_empty()
            && !self.base_input_class.is_empty()
            && !self.derived_input_class.is_empty()
    }

    /// A derivation is a *change*: derived text equal to base text would mean
    /// the recipe did nothing, and the two digests would then not distinguish
    /// a derived import from a plain one.
    #[must_use]
    pub fn is_a_derivation(&self) -> bool {
        self.base_source_digest != self.derived_source_digest
    }
}

/// Which prerequisite set a composition question names, tagged by family so one
/// family's set can never be read as another's.
///
/// The set is per-family because the shape *is* per-family: the leaf composition
/// names three prerequisites, none of them a composition; the `file_close`
/// compositions name three or four, one of which is the leaf composition itself.
/// A single flat triple would have had to mean different things in different
/// families, which is exactly what these vocabularies exist not to do.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompositionPrerequisiteSet {
    UnmapWrapper(UnmapWrapperPrerequisites),
    FileClose(FileClosePrerequisites),
    MemoryClose(MemoryClosePrerequisites),
    FilesystemChunkClose(FilesystemChunkClosePrerequisites),
    UnsupportedClose(UnsupportedClosePrerequisites),
}

impl CompositionPrerequisiteSet {
    #[must_use]
    pub fn is_complete(&self) -> bool {
        match self {
            Self::UnmapWrapper(set) => set.is_complete(),
            Self::FileClose(set) => set.is_complete(),
            Self::MemoryClose(set) => set.is_complete(),
            Self::FilesystemChunkClose(set) => set.is_complete(),
            Self::UnsupportedClose(set) => set.is_complete(),
        }
    }

    #[must_use]
    pub fn are_distinct(&self) -> bool {
        match self {
            Self::UnmapWrapper(set) => set.are_distinct(),
            Self::FileClose(set) => set.are_distinct(),
            Self::MemoryClose(set) => set.are_distinct(),
            Self::FilesystemChunkClose(set) => set.are_distinct(),
            Self::UnsupportedClose(set) => set.are_distinct(),
        }
    }

    #[must_use]
    pub fn unmap_wrapper(&self) -> Option<&UnmapWrapperPrerequisites> {
        match self {
            Self::UnmapWrapper(set) => Some(set),
            _ => None,
        }
    }

    #[must_use]
    pub fn file_close(&self) -> Option<&FileClosePrerequisites> {
        match self {
            Self::FileClose(set) => Some(set),
            _ => None,
        }
    }

    #[must_use]
    pub fn memory_close(&self) -> Option<&MemoryClosePrerequisites> {
        match self {
            Self::MemoryClose(set) => Some(set),
            _ => None,
        }
    }

    #[must_use]
    pub fn filesystem_chunk_close(&self) -> Option<&FilesystemChunkClosePrerequisites> {
        match self {
            Self::FilesystemChunkClose(set) => Some(set),
            _ => None,
        }
    }

    #[must_use]
    pub fn unsupported_close(&self) -> Option<&UnsupportedClosePrerequisites> {
        match self {
            Self::UnsupportedClose(set) => Some(set),
            _ => None,
        }
    }
}

/// The prerequisite body and record shape the deleting arm additionally needs.
/// It travels so the producer must recompute the guarded-delete reading itself;
/// that reading never travels.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileCloseDeleteInput {
    pub body: CleanupReplayFunctionRecord,
    pub profile: CleanupProfileRecord,
    /// The declared eligibility field, which is not a profile member. The core
    /// reads its own from the pinned contract's declaration and refuses a
    /// transported value that is not exactly that.
    pub mapping_field: String,
}

/// Which bounded composition question a projection asks.
///
/// Two inhabitants. The other five ChunkIO compositions are classified in
/// `docs/reports/remaining-p04-compose/README.md`: three are blocked behind the
/// frame helper, which has no verification task at all, and two additionally sit
/// behind the sealed `SourceMatch` boundary.
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
    /// `cio_file_close`: the entry states of the *nested* unmap composition
    /// crossed with the native close outcomes and the path allocation's
    /// presence, and — on the deleting arm — the guarded-delete outcomes.
    FileClose {
        /// The `cio_file_close` body's complete typed path set.
        wrapper: CleanupReplayFunctionRecord,
        /// **The nested composition question, verbatim.** It is the same value
        /// the leaf composition's own projection carries, so a producer must
        /// recompute the whole nested composed reading — not merely a body
        /// replay — before it can answer this one. The core requires it to be
        /// exactly the family input its own nested composition task imported.
        unmap: Box<CompositionFamilyInput>,
        /// The `cio_file_native_close` body and the record shape its replay
        /// evolves.
        close: CleanupReplayFunctionRecord,
        close_profile: CleanupProfileRecord,
        /// Present exactly on the deleting entry point.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        delete: Option<Box<FileCloseDeleteInput>>,
    },
    /// `cio_chunk_close`: the null and memory-backed entry states over a
    /// wrapper imported from **derived** text, with the backend discriminant
    /// carried as a probe body rather than guessed.
    ///
    /// The four prerequisite readings are not published figures of this family:
    /// what a prerequisite contributes here is the identity of a frame the
    /// wrapper is allowed to resolve, and whether its own pinned contract
    /// holds. The identities travel in `callees`, the contracts never travel at
    /// all, and the core re-establishes all four before comparing anything.
    MemoryClose {
        /// The `cio_chunk_close` body's complete typed path set, imported from
        /// the derived unit.
        wrapper: CleanupReplayFunctionRecord,
        /// The probe body's complete typed path set. A producer reads the
        /// backend discriminant out of this; it is never handed the value.
        probe: CleanupReplayFunctionRecord,
        /// How the two bodies' unit was derived. Every field is a claim the
        /// core checks against its own derivation.
        derivation: DerivedUnitRecord,
        /// The four prerequisite bodies' own callee identities.
        callees: MemoryCloseCalleeRecord,
    },
    /// `cio_chunk_close`'s **filesystem** arm: the memory-close question's own
    /// wrapper replayed under the filesystem discriminant and the delete flag,
    /// crossed with the deleting `cio_file_close` composition's own case
    /// vectors.
    ///
    /// This family carries **no body of its own but the probe**. Its wrapper is
    /// the nested memory-close question's wrapper — the same body, replayed on
    /// a different arm under a different profile — so restating it here would
    /// have let the two halves of one question name different bodies.
    ///
    /// It is the first family with two derived-unit imports in one question:
    /// its own filesystem probe is derived under this family's recipe, and the
    /// nested memory-close question's probe under that family's. The two
    /// recipes differ, both derive from the same base unit, and the core
    /// re-derives both texts itself.
    FilesystemChunkClose {
        /// **The nested memory-close question, verbatim.** A producer must
        /// recompute the whole memory-close composed reading — including its
        /// own four prerequisite frames and its own probed discriminant —
        /// before it can answer this one.
        memory: Box<CompositionFamilyInput>,
        /// **The nested deleting `cio_file_close` question, verbatim**, which
        /// is itself a two-level composition. A producer must recompute that
        /// whole reading too, so this question nests three levels deep.
        backend: Box<CompositionFamilyInput>,
        /// The filesystem discriminant probe body's complete typed path set. A
        /// producer reads the value out of this; it is never handed the value.
        probe: CleanupReplayFunctionRecord,
        /// How **this family's** probe unit was derived. Every field is a claim
        /// the core checks against its own derivation, and the core additionally
        /// requires this derivation and the nested memory question's to derive
        /// from one base text under two different recipes.
        derivation: DerivedUnitRecord,
    },
    /// `cio_chunk_close`'s **symbolic** other-backend arm: the same wrapper
    /// replayed with the stream's backend discriminant standing for the class of
    /// every `int` except the two the two probes return.
    ///
    /// This family carries **no body and no derivation of its own at all**. Its
    /// whole question is the nested filesystem question, verbatim: that question
    /// already carries the wrapper (through *its* nested memory question), the
    /// memory probe, the filesystem probe and both derivation records. A
    /// producer must therefore recompute the whole three-level filesystem
    /// reading before it can answer this one, and then run a fourth replay over
    /// a domain no transported value names.
    UnsupportedClose {
        /// **The nested filesystem question, verbatim.** The core requires it to
        /// be exactly the family input its own filesystem composition context
        /// imported.
        filesystem: Box<CompositionFamilyInput>,
    },
}

impl CompositionFamilyInput {
    /// The function the single obligation of this projection is about.
    #[must_use]
    pub fn subject_function(&self) -> &str {
        &self.wrapper().function
    }

    #[must_use]
    pub fn wrapper(&self) -> &CleanupReplayFunctionRecord {
        match self {
            Self::UnmapWrapper { wrapper, .. }
            | Self::FileClose { wrapper, .. }
            | Self::MemoryClose { wrapper, .. } => wrapper,
            // This family has no wrapper of its own: it replays the nested
            // memory-close question's wrapper on the filesystem arm.
            Self::FilesystemChunkClose { memory, .. } => memory.wrapper(),
            // Nor does this one: it replays the same body a third time, on the
            // symbolic arm, reached through the nested filesystem question.
            Self::UnsupportedClose { filesystem } => filesystem.wrapper(),
        }
    }

    /// How this family's **own** bodies were derived from the unit under
    /// analysis, on the families that do not import from the declared file
    /// itself. A family whose nested question is derived does not inherit its
    /// derivation here: the nested record is reached through [`Self::nested`].
    #[must_use]
    pub fn derivation(&self) -> Option<&DerivedUnitRecord> {
        match self {
            Self::MemoryClose { derivation, .. }
            | Self::FilesystemChunkClose { derivation, .. } => Some(derivation),
            // The symbolic family derives no text of its own: both derivations
            // it depends on are inside its nested question, reached through
            // [`Self::nested`], and it must not appear to own either.
            Self::UnmapWrapper { .. } | Self::FileClose { .. } | Self::UnsupportedClose { .. } => {
                None
            }
        }
    }

    /// The probe body this family reads a backend discriminant out of, on the
    /// two families that carry one.
    #[must_use]
    pub fn probe(&self) -> Option<&CleanupReplayFunctionRecord> {
        match self {
            Self::MemoryClose { probe, .. } | Self::FilesystemChunkClose { probe, .. } => {
                Some(probe)
            }
            Self::UnmapWrapper { .. } | Self::FileClose { .. } | Self::UnsupportedClose { .. } => {
                None
            }
        }
    }

    /// The prerequisite body whose replay this family's own record profile
    /// describes: the native unmap for the leaf family, the native close for the
    /// file_close families.
    #[must_use]
    pub fn native(&self) -> Option<&CleanupReplayFunctionRecord> {
        match self {
            Self::UnmapWrapper { native, .. } => Some(native),
            Self::FileClose { close, .. } => Some(close),
            Self::MemoryClose { .. }
            | Self::FilesystemChunkClose { .. }
            | Self::UnsupportedClose { .. } => None,
        }
    }

    #[must_use]
    pub fn native_profile(&self) -> Option<&CleanupProfileRecord> {
        match self {
            Self::UnmapWrapper { native_profile, .. } => Some(native_profile),
            Self::FileClose { close_profile, .. } => Some(close_profile),
            Self::MemoryClose { .. }
            | Self::FilesystemChunkClose { .. }
            | Self::UnsupportedClose { .. } => None,
        }
    }

    /// The nested composition questions, in the order this family names its
    /// composition prerequisites. Empty on a family with none.
    ///
    /// This returns a **vector**, not one optional question: the filesystem
    /// family's every prerequisite is a composition, so a single-slot accessor
    /// would have had to publish one of two and hide the other.
    #[must_use]
    pub fn nested(&self) -> Vec<&Self> {
        match self {
            Self::UnmapWrapper { .. } | Self::MemoryClose { .. } => Vec::new(),
            Self::FileClose { unmap, .. } => vec![unmap],
            Self::FilesystemChunkClose {
                memory, backend, ..
            } => vec![memory, backend],
            Self::UnsupportedClose { filesystem } => vec![filesystem],
        }
    }

    /// How many composition levels this question nests. The leaf family is 1;
    /// a family whose prerequisites include compositions is one more than the
    /// deepest of them. Pinned by test rather than assumed.
    #[must_use]
    pub fn nesting_depth(&self) -> u32 {
        1 + self
            .nested()
            .into_iter()
            .map(Self::nesting_depth)
            .max()
            .unwrap_or(0)
    }

    /// The semantic profile this family answers under.
    #[must_use]
    pub const fn semantic_profile(&self) -> &'static str {
        match self {
            Self::UnmapWrapper { .. } => UNMAP_WRAPPER_SEMANTIC_PROFILE,
            Self::FileClose { delete: None, .. } => FILE_CLOSE_SEMANTIC_PROFILE,
            Self::FileClose {
                delete: Some(_), ..
            } => FILE_CLOSE_DELETE_SEMANTIC_PROFILE,
            Self::MemoryClose { .. } => MEMORY_CLOSE_SEMANTIC_PROFILE,
            Self::FilesystemChunkClose { .. } => FILESYSTEM_CHUNK_CLOSE_SEMANTIC_PROFILE,
            Self::UnsupportedClose { .. } => UNSUPPORTED_CLOSE_SEMANTIC_PROFILE,
        }
    }

    /// A family input is well-formed only when its shape and its prerequisite
    /// set agree: the leaf family names the leaf set, a file_close family names
    /// the file_close set, and the deleting arm's delete body and delete
    /// prerequisite are present or absent **together**. Neither half may claim
    /// what the other does not.
    #[must_use]
    pub fn matches(&self, prerequisites: &CompositionPrerequisiteSet) -> bool {
        match (self, prerequisites) {
            (Self::UnmapWrapper { .. }, CompositionPrerequisiteSet::UnmapWrapper(_)) => true,
            (Self::FileClose { delete, unmap, .. }, CompositionPrerequisiteSet::FileClose(set)) => {
                delete.is_some() == set.native_delete.is_some()
                    && matches!(**unmap, Self::UnmapWrapper { .. })
            }
            (Self::MemoryClose { .. }, CompositionPrerequisiteSet::MemoryClose(_)) => true,
            // Both nested questions must be the families this route's two
            // prerequisites answer, and the backend one must be the *deleting*
            // arm: the filesystem question crosses its entries with both case
            // vectors, which only the deleting entry point publishes.
            (
                Self::FilesystemChunkClose {
                    memory, backend, ..
                },
                CompositionPrerequisiteSet::FilesystemChunkClose(_),
            ) => {
                matches!(**memory, Self::MemoryClose { .. })
                    && matches!(
                        &**backend,
                        Self::FileClose {
                            delete: Some(_),
                            unmap,
                            ..
                        } if matches!(**unmap, Self::UnmapWrapper { .. })
                    )
            }
            // The one nested question must be the family this route's single
            // prerequisite answers — the filesystem composition — and not the
            // memory or file_close question one level below it. Its own two
            // nested halves are checked here too, so a filesystem question with
            // a non-deleting backend arm cannot reach this route's importer by
            // hiding one level down.
            (
                Self::UnsupportedClose { filesystem },
                CompositionPrerequisiteSet::UnsupportedClose(_),
            ) => matches!(
                &**filesystem,
                Self::FilesystemChunkClose { memory, backend, .. }
                    if matches!(**memory, Self::MemoryClose { .. })
                        && matches!(
                            &**backend,
                            Self::FileClose { delete: Some(_), unmap, .. }
                                if matches!(**unmap, Self::UnmapWrapper { .. })
                        )
            ),
            _ => false,
        }
    }
}

/// One transported composition question.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompositionSourceInput {
    pub schema_version: u16,
    /// Identity triples only. Nothing a prerequisite concluded reaches here.
    pub prerequisites: CompositionPrerequisiteSet,
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

/// Whether a residual OS custody obligation is discharged by the source
/// projection or left unresolved. Mirrors the trusted composition's own
/// two-point domain; there is no third, optimistic value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResidualCustodyRecord {
    Absent,
    Unresolved,
}

/// The four disjoint deletion outcomes the deleting arm distinguishes. Freeing
/// the C path allocation is never one of them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeletionOutcomeRecord {
    NoBackend,
    BlockedByFields,
    SyscallFailed,
    SyscallSucceeded,
}

/// One non-deleting `cio_file_close` case. Every field mirrors, field for field,
/// the `FileCloseCase` the trusted composition publishes.
///
/// `unmap_case` is the composed figure of this family: it names the **nested
/// composition's** own case index for this entry, so a producer that did not
/// recompute the nested composed reading cannot fill it in.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileCloseCaseRecord {
    pub source_path: u32,
    pub backend_present: bool,
    pub path_allocation_present: bool,
    /// The nested unmap composition's own selected case, or `None` when this
    /// entry state never reaches it.
    pub unmap_case: Option<u32>,
    /// The native close replay's own selected case.
    pub close_case: Option<u32>,
    pub mapping: ResidualCustodyRecord,
    pub descriptor: ResidualCustodyRecord,
    pub path_released: bool,
    pub backend_released: bool,
    /// Read out of the nested composition's own case, not recomputed here.
    pub up_counter_decrements: u32,
}

/// One deleting `cio_file_close` case.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeletingCloseCaseRecord {
    pub source_path: u32,
    pub unmap_case: Option<u32>,
    pub close_case: Option<u32>,
    /// The guarded-delete replay's own selected case.
    pub delete_case: Option<u32>,
    pub mapping: ResidualCustodyRecord,
    pub descriptor: ResidualCustodyRecord,
    pub deletion: DeletionOutcomeRecord,
    pub backend_released: bool,
    pub path_released: bool,
    pub diagnostic_calls: u32,
    pub up_counter_decrements: u32,
}

/// One composed `cio_file_close` reading, with the wrapper's imported identity.
///
/// The deleting entry point answers a strictly larger question, so it publishes
/// the whole non-deleting case vector *and* the deleting one; the non-deleting
/// entry point publishes only the first and leaves the second absent rather than
/// empty, so "no deleting arm" and "an empty deleting arm" cannot be confused.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileCloseCompositionRecord {
    pub function: String,
    pub source_revision: String,
    pub signature: String,
    pub cases: Vec<FileCloseCaseRecord>,
    pub uncovered_paths: Vec<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deleting_cases: Option<Vec<DeletingCloseCaseRecord>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deleting_uncovered_paths: Option<Vec<u32>>,
    pub profile_assumptions: BTreeMap<String, String>,
}

/// The proposed reading, tagged by family so one family's proposal can never be
/// read as another's.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "family", rename_all = "snake_case")]
pub enum CompositionProposalRecord {
    UnmapWrapper(CompositionRecord),
    FileClose(FileCloseCompositionRecord),
    MemoryClose(MemoryCloseCompositionRecord),
    FilesystemChunkClose(FilesystemChunkCloseCompositionRecord),
    UnsupportedClose(UnsupportedCloseCompositionRecord),
}

/// One composed **symbolic** other-backend reading.
///
/// This is the thinnest of the five proposal records, deliberately. The trusted
/// composition publishes exactly two figures on this arm — the two excluded
/// constants and the one wrapper path the symbolic class selects — and the
/// discipline every record in this module follows is that a figure the checker
/// does not decide has no place in it. The six release facts this arm asserts
/// are *constants of the admission* (the replay refuses unless all six hold and
/// exactly six calls resolve), not decided figures, so publishing them would
/// have invented agreement rather than measured it.
///
/// The two `kind` fields together **are** the excluded class: the symbolic
/// domain is every `int` other than these two. They travel in the proposal
/// because a producer that did not read both probes cannot name them, and the
/// core requires them to differ.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnsupportedCloseCompositionRecord {
    pub function: String,
    pub source_revision: String,
    pub signature: String,
    /// The discriminant the nested memory question's probe returns.
    pub memory_kind: i32,
    /// The discriminant the nested filesystem question's probe returns.
    pub filesystem_kind: i32,
    /// The single wrapper path the symbolic class selects. It is the same under
    /// either delete flag — the trusted composition refuses the reading outright
    /// when it is not — so one field, not two, and the independence is a
    /// refusal rather than a published figure.
    pub source_path: u32,
    pub profile_assumptions: BTreeMap<String, String>,
}

/// One filesystem-backed `cio_chunk_close` entry state's composed reading.
///
/// Every field mirrors, field for field, the `FilesystemChunkCloseCase` the
/// trusted composition publishes. `backend_case` and the five fields read out
/// of it are the composed figures of this family: they name the **deleting
/// `cio_file_close` composition's** own case for this entry, so a producer that
/// did not recompute that whole two-level reading cannot fill them in.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FilesystemChunkCloseCaseRecord {
    /// The single wrapper path this entry state selects.
    pub source_path: u32,
    pub chunk_present: bool,
    pub delete: bool,
    /// The backend prerequisite's own case index, in its deleting or
    /// non-deleting vector as `delete` selects, or `None` on the null entry.
    pub backend_case: Option<u32>,
    pub backend_released: bool,
    pub path_released: bool,
    pub mapping: ResidualCustodyRecord,
    pub descriptor: ResidualCustodyRecord,
    /// Absent on every non-deleting case, and on the null entry.
    pub deletion: Option<DeletionOutcomeRecord>,
    pub primary_unlinked: bool,
    pub state_unlinked: bool,
    pub name_released: bool,
    pub chunk_released: bool,
    pub total_counter_decrements: u32,
    pub up_counter_decrements: u32,
}

/// One composed filesystem-chunk-close reading, with the wrapper's imported
/// identity — the wrapper being the nested memory-close question's body.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FilesystemChunkCloseCompositionRecord {
    pub function: String,
    pub source_revision: String,
    pub signature: String,
    /// The filesystem discriminant the producer read out of **this** family's
    /// probe body. A reading, never an input.
    pub filesystem_kind: i32,
    /// The memory discriminant it read out of the **nested** question's probe.
    /// It travels in the proposal because this family's own admission requires
    /// the two to differ, so a proposal that did not read both cannot be made.
    pub memory_kind: i32,
    pub cases: Vec<FilesystemChunkCloseCaseRecord>,
    pub checked_paths: Vec<u32>,
    /// The wrapper paths this arm leaves uncovered.
    pub uncovered_paths: Vec<u32>,
    /// The paths neither this arm nor the memory arm covers. Published so the
    /// narrowed joint claim is visible rather than inferred.
    pub combined_uncovered_paths: Vec<u32>,
    pub profile_assumptions: BTreeMap<String, String>,
}

/// One memory-close entry state's composed reading.
///
/// Every field mirrors, field for field, the replay state the trusted
/// composition reaches on that entry. Nothing is invented here: a figure the
/// checker does not decide has no place in this record.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MemoryCloseCaseRecord {
    /// Whether the chunk argument was the nonnull entry or the null one.
    pub nonnull: bool,
    /// The single wrapper path this entry state selects.
    pub source_path: u32,
    /// How many calls the selected path resolved.
    pub calls: u32,
    pub backend_discharged: bool,
    pub primary_unlinked: bool,
    pub state_unlinked: bool,
    pub name_released: bool,
    pub chunk_released: bool,
    pub counter_decremented: bool,
}

/// One composed memory-close reading, with the wrapper's imported identity.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MemoryCloseCompositionRecord {
    pub function: String,
    pub source_revision: String,
    pub signature: String,
    /// The backend discriminant the producer read out of the probe body. It is
    /// a reading, not an input: the request carries the probe's paths, never
    /// the value.
    pub memory_kind: i32,
    /// The two entry states, in the order the trusted composition enumerates
    /// them: the null entry first, then the nonnull one.
    pub cases: Vec<MemoryCloseCaseRecord>,
    pub checked_paths: Vec<u32>,
    /// The filesystem and other-backend paths this family leaves explicitly
    /// uncovered. They are published so a narrowed claim is visible.
    pub uncovered_paths: Vec<u32>,
    pub profile_assumptions: BTreeMap<String, String>,
}

/// The witness envelope, bound to the task digest the core computed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompositionWitness {
    pub task_digest: String,
    pub summary: CompositionProposalRecord,
}

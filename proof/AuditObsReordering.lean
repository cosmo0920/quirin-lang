-- SPDX-License-Identifier: Apache-2.0
--
-- Licensed under the Apache License, Version 2.0 (the "License");
-- you may not use this file except in compliance with the License.
-- You may obtain a copy of the License at
--
--     http://www.apache.org/licenses/LICENSE-2.0
--
-- Unless required by applicable law or agreed to in writing, software
-- distributed under the License is distributed on an "AS IS" BASIS,
-- WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
-- See the License for the specific language governing permissions and
-- limitations under the License.

import QCore.ObsReordering

-- Card `OBS-REORDERING`: `qcore.obs-reordering.v1`, the comparison of an
-- observation stream up to a **declared** reordering. The Rust side is
-- `crates/qcore-core/src/obs_reordering.rs`; the normative statement is
-- `docs/design/OBS_REORDERING_V1.md`.

-- The projection a whole emitter's order is read through, and its basic shape.
#print axioms QCore.ObsReordering.proj_nil
#print axioms QCore.ObsReordering.proj_cons_same
#print axioms QCore.ObsReordering.proj_cons_other
#print axioms QCore.ObsReordering.proj_length_le
#print axioms QCore.ObsReordering.proj_append

-- `≡` is an equivalence relation, so "these two runs agree" is a property of
-- the pair rather than of which one was called the reference.
#print axioms QCore.ObsReordering.windowEquiv_refl
#print axioms QCore.ObsReordering.windowEquiv_symm
#print axioms QCore.ObsReordering.windowEquiv_trans
#print axioms QCore.ObsReordering.equiv_refl
#print axioms QCore.ObsReordering.equiv_symm
#print axioms QCore.ObsReordering.equiv_trans

-- The window decomposition is preserved: the same windows, each holding the
-- same number of records. Windows never merge and never split.
#print axioms QCore.ObsReordering.equiv_length
#print axioms QCore.ObsReordering.equiv_window_sizes

-- **Soundness 1.** An accepted run performed each thread's writes in that
-- thread's own program order — whole-stream, and window by window. This is what
-- an accepted native run has actually been shown to satisfy.
#print axioms QCore.ObsReordering.equiv_preserves_emitter_projection
#print axioms QCore.ObsReordering.equiv_preserves_window_content

-- **Soundness 2.** Barrier structure is preserved. A window holding one record
-- is the shape a quiescent point produces — no worker live, the parent the only
-- thread of control — and it is carried across record for record and position
-- for position.
#print axioms QCore.ObsReordering.barrier_is_carried_across
#print axioms QCore.ObsReordering.equiv_preserves_barriers

-- **Soundness 3.** The strict comparison is the empty-relation case: a stream
-- whose windows each have at most one emitter is equivalent to itself alone, in
-- both directions. Every route in this repository before this card declares such
-- a stream, which is why this card replaces no existing comparison.
#print axioms QCore.ObsReordering.length_proj_of_single_emitter
#print axioms QCore.ObsReordering.single_emitter_of_length_proj
#print axioms QCore.ObsReordering.window_of_single_emitter
#print axioms QCore.ObsReordering.strict_window_admits_only_itself
#print axioms QCore.ObsReordering.strict_admits_only_itself
#print axioms QCore.ObsReordering.strict_iff_byte_equality
#print axioms QCore.ObsReordering.barriers_are_strict

-- The generators: two records of **distinct** emitters in one window commute,
-- and two records of **one** emitter never do. The operational reading of the
-- relation and its closed form therefore agree.
#print axioms QCore.ObsReordering.distinct_emitters_commute
#print axioms QCore.ObsReordering.one_emitter_never_commutes

-- The decision procedure is exact: the transcription of the Rust comparator's
-- per-window, per-emitter loop returns `true` on exactly the equivalent streams.
#print axioms QCore.ObsReordering.proj_eq_nil_of_not_mem
#print axioms QCore.ObsReordering.decideWindow_correct
#print axioms QCore.ObsReordering.decideEquiv_correct

-- Not audited individually: the two `private` recursion helpers `zip_forward`
-- and `zip_backward`, on the `QCore.ComposedStdIo` convention — they are covered
-- by `AuditAll.lean`'s whole-project axiom closure and transitively by
-- `decideEquiv_correct`, which depends on both.

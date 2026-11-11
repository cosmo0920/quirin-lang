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

import QCore.AuditComposition

/-! Card **AUDIT-COMPOSITION**: the integration semigroup across results
(`docs/plans/pluggable-cards/README.md` seam G; language spec §L21.2,
"Composition across results"). Model of `crates/qcore-schemas/src/composition.rs`.

Six directions are audited.

**The algebra.** Provenance-keyed union is associative, commutative and
idempotent with the empty contribution as identity, and erases nothing; every
derived quantity reads only the set of components, so the derived rows inherit
the laws and a byte-identical duplicate changes nothing.

**Monotonicity.** Adding a contribution never lowers a row's verdict, every
component is below its row, and adding never removes an assumption.

**Conflict is kept, not chosen.** Two components of one row under one
provenance with different bytes are both kept and the row is at least
`Unknown`, exactly `Unknown` unless a component is `Unsafe`; distinct
provenances never conflict.

**Removal is a re-join.** Removing a disjoint contribution restores the prior
set, derivation, conflict flag and assumptions.

**Absence is refusal.** A row no component speaks about has no derivation;
the empty contribution publishes nothing.

**Shared vectors.** The vector list is decided here and checked against the
Rust implementation by `crates/qcore-schemas/tests/audit_composition.rs`. -/

-- The algebra.
#print axioms QCore.AuditComposition.mem_union
#print axioms QCore.AuditComposition.union_assoc
#print axioms QCore.AuditComposition.union_comm
#print axioms QCore.AuditComposition.empty_union
#print axioms QCore.AuditComposition.union_empty
#print axioms QCore.AuditComposition.union_idem
#print axioms QCore.AuditComposition.union_keeps_left
#print axioms QCore.AuditComposition.union_keeps_right
#print axioms QCore.AuditComposition.union_duplicate
#print axioms QCore.AuditComposition.fold_congr
#print axioms QCore.AuditComposition.conflicted_congr
#print axioms QCore.AuditComposition.verdictOf_congr
#print axioms QCore.AuditComposition.present_congr
#print axioms QCore.AuditComposition.derive_congr
#print axioms QCore.AuditComposition.assumptions_congr
#print axioms QCore.AuditComposition.derive_assoc
#print axioms QCore.AuditComposition.derive_comm
#print axioms QCore.AuditComposition.derive_identity
#print axioms QCore.AuditComposition.derive_idem
#print axioms QCore.AuditComposition.derive_duplicate

-- Monotonicity.
#print axioms QCore.AuditComposition.the_join_never_lowers
#print axioms QCore.AuditComposition.every_component_is_below_its_row
#print axioms QCore.AuditComposition.adding_never_removes_an_assumption
#print axioms QCore.AuditComposition.the_row_carries_every_component_assumption

-- Conflict is kept, not chosen.
#print axioms QCore.AuditComposition.a_conflict_is_at_least_unknown
#print axioms QCore.AuditComposition.a_conflict_without_unsafe_is_unknown
#print axioms QCore.AuditComposition.a_conflict_keeps_both
#print axioms QCore.AuditComposition.distinct_provenances_never_conflict

-- Removal is a re-join.
#print axioms QCore.AuditComposition.removal_restores_the_set
#print axioms QCore.AuditComposition.removal_restores_the_derivation

-- Absence is refusal.
#print axioms QCore.AuditComposition.derive_none_iff
#print axioms QCore.AuditComposition.the_empty_contribution_publishes_nothing
#print axioms QCore.AuditComposition.derive_some_of_mem

-- Shared vectors.
#print axioms QCore.AuditComposition.vectors_hold
#print axioms QCore.AuditComposition.vectors_reach_every_case

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

import QCore.PluginPremiseDecomposition

/-! The registered overlap guard compares proposed metadata only. Its inputs
are independently replayed property views; parser validity and exact legacy
erasure are explicit prerequisites. It identifies neither original graph,
legacy vectors, Contexts, source correspondence nor proposition truth. -/
namespace QCore.PluginNeutralOverlap

structure Erasure where
  name : String
  statement : String
  origin : String
  deriving DecidableEq

structure Refinement where
  children : List Nat
  target : Erasure
  deriving DecidableEq

/-- Only nonlegacy atom IDs and proposed attachments occur in this view. -/
structure Proposed where
  atoms : Nat → Option String
  refinements : Nat → Option Refinement

def pairedMetadata (nullView resourcePeer : Proposed) : Prop :=
  (∀ key statement, nullView.atoms key = some statement →
    resourcePeer.atoms key = some statement) ∧
  (∀ key refinement, nullView.refinements key = some refinement →
    resourcePeer.refinements key = some refinement)

/-- `checker` is the existing null-only clause predicate. A metadata spelling
alone does not satisfy any of the three independent admission prerequisites. -/
def guard (hasMetadata : Bool) (checker : List α → Prop)
    (clauses : List (PluginPremiseDecomposition.Clause α))
    (nullView resourcePeer : Proposed) (parsedBounds neutralErasure : Prop) : Prop :=
  checker (PluginPremiseDecomposition.semanticClauses clauses) ∧
    (hasMetadata = true → parsedBounds ∧ neutralErasure ∧ pairedMetadata nullView resourcePeer)

theorem matched_atoms_retain_exact_statement
    (nullView resourcePeer : Proposed) (paired : pairedMetadata nullView resourcePeer)
    (key : Nat) (statement : String)
    (present : nullView.atoms key = some statement) :
    resourcePeer.atoms key = some statement := paired.1 key statement present

theorem matched_refinements_retain_children_and_target
    (nullView resourcePeer : Proposed) (paired : pairedMetadata nullView resourcePeer)
    (key : Nat) (refinement : Refinement)
    (present : nullView.refinements key = some refinement) :
    resourcePeer.refinements key = some refinement := paired.2 key refinement present

theorem missing_or_changed_atom_refuses
    (checker : List α → Prop) (clauses) (nullView resourcePeer : Proposed)
    (parsedBounds neutralErasure : Prop) (key : Nat) (statement : String)
    (present : nullView.atoms key = some statement)
    (mismatch : resourcePeer.atoms key ≠ some statement) :
    ¬ guard true checker clauses nullView resourcePeer parsedBounds neutralErasure := by
  intro admitted
  exact mismatch ((admitted.2 rfl).2.2.1 key statement present)

theorem missing_or_changed_refinement_refuses
    (checker : List α → Prop) (clauses) (nullView resourcePeer : Proposed)
    (parsedBounds neutralErasure : Prop) (key : Nat) (refinement : Refinement)
    (present : nullView.refinements key = some refinement)
    (mismatch : resourcePeer.refinements key ≠ some refinement) :
    ¬ guard true checker clauses nullView resourcePeer parsedBounds neutralErasure := by
  intro admitted
  exact mismatch ((admitted.2 rfl).2.2.2 key refinement present)

theorem failed_bounds_parse_refuses
    (checker : List α → Prop) (clauses) (nullView resourcePeer : Proposed)
    (parsedBounds neutralErasure : Prop) (failed : ¬ parsedBounds) :
    ¬ guard true checker clauses nullView resourcePeer parsedBounds neutralErasure := by
  intro admitted
  exact failed (admitted.2 rfl).1

theorem absent_metadata_uses_original_predicate
    (checker : List α → Prop) (clauses) (nullView resourcePeer : Proposed)
    (parsedBounds neutralErasure : Prop) :
    guard false checker clauses nullView resourcePeer parsedBounds neutralErasure ↔
      checker (PluginPremiseDecomposition.semanticClauses clauses) := by
  simp [guard]

theorem independently_matched_attachment_preserves_original_predicate
    (checker : List α → Prop) (original) (atoms : List Nat)
    (nullView resourcePeer : Proposed) (parsedBounds neutralErasure : Prop)
    (parsed : parsedBounds) (neutral : neutralErasure)
    (paired : pairedMetadata nullView resourcePeer) :
    guard true checker (PluginPremiseDecomposition.attach original atoms)
      nullView resourcePeer parsedBounds neutralErasure ↔
      checker (PluginPremiseDecomposition.semanticClauses original) := by
  simp [guard, parsed, neutral, paired,
    PluginPremiseDecomposition.attachment_preserves_original_clause_interpretation]

end QCore.PluginNeutralOverlap

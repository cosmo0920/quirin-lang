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

import QCore.FixedScalarQuery
import QCore.RefusedWrites

/-! Abstract rules for the three C gateway refinements. Faithful token/type
extraction and declared release effects remain premises; this is not extraction
of Rust or a proof of arbitrary C. Cyclic CFGs derive no conditional facts. -/
namespace QCore.CGatewayRefinements

def writeScalar := FixedScalarQuery.assignQuery

theorem scalar_write_preserves_custody (s : FixedScalarQuery.State) (n : Nat) :
    (writeScalar s n).custody = s.custody := rfl

theorem scalar_write_keeps_both_branches (s : FixedScalarQuery.State)
    (n : Nat) (branch : Bool) :
    FixedScalarQuery.permits ((writeScalar s n).scalar n) branch = true :=
  FixedScalarQuery.query_assignment_keeps_both_branches s n branch

/-- Structural control-flow failures have no complete expression evidence. -/
def expressionSpares (complete : Bool) (fr : RefusedWrites.Frame)
    (written : RefusedWrites.Written) (p : RefusedWrites.Place) : Bool :=
  complete && RefusedWrites.spares fr written p

theorem incomplete_expression_spares_nothing (fr : RefusedWrites.Frame)
    (w : RefusedWrites.Written) (p : RefusedWrites.Place) :
    expressionSpares false fr w p = false := rfl

theorem expression_frame_has_all_three_premises (complete : Bool)
    (fr : RefusedWrites.Frame) (w : RefusedWrites.Written) (p : RefusedWrites.Place)
    (h : expressionSpares complete fr w p = true) :
    complete = true ∧ ∃ n, p = .plain n ∧ fr.protects n = true ∧ w n = false := by
  simp only [expressionSpares, Bool.and_eq_true] at h
  exact ⟨h.1, RefusedWrites.spared_places_are_unwritable fr w p h.2⟩

/-- Existence differs from liveness: a released object retains its identity. -/
structure World where
  present : Nat → Bool
  value : Nat → Option Nat
  live : Nat → Bool

def guarantees (paths : List World) (site name : Nat) : Prop :=
  ∀ p ∈ paths, p.present site = true → p.value name = some site

theorem join_guarantees (a b : List World) (site name : Nat) :
    guarantees (a ++ b) site name ↔ guarantees a site name ∧ guarantees b site name := by
  simp only [guarantees, List.mem_append, or_imp, forall_and]

theorem absent_predecessor_is_neutral (a b : List World) (site name : Nat)
    (absent : ∀ p ∈ b, p.present site = false) :
    guarantees (a ++ b) site name ↔ guarantees a site name := by
  rw [join_guarantees]
  constructor
  · exact And.left
  · intro h
    refine ⟨h, ?_⟩
    intro p hp he
    have hn := absent p hp
    simp [hn] at he

theorem null_guard_excludes_a_conditionally_named_site (p : World) (site name : Nat)
    (h : p.present site = true → p.value name = some site)
    (null : p.value name = none) : p.present site = false := by
  cases he : p.present site with
  | false => rfl
  | true => have contradiction := h he; simp [null] at contradiction

/-- Binding transfer is separate from resource existence. -/
def assignValue (p : World) (destination : Nat) (v : Option Nat) : World :=
  { p with value := fun name => if name = destination then v else p.value name }

theorem assignment_preserves_other_conditional_names (p : World) (site name destination : Nat)
    (v : Option Nat) (different : name ≠ destination)
    (h : p.present site = true → p.value name = some site) :
    (assignValue p destination v).present site = true →
      (assignValue p destination v).value name = some site := by
  simpa [assignValue, different] using h

theorem copying_a_conditional_name_establishes_the_destination (p : World)
    (site source destination : Nat)
    (h : p.present site = true → p.value source = some site) :
    (assignValue p destination (p.value source)).present site = true →
      (assignValue p destination (p.value source)).value destination = some site := by
  simpa [assignValue] using h

def acquire (p : World) (site destination : Nat) : World :=
  { present := fun s => if s = site then true else p.present s
    live := fun s => if s = site then true else p.live s
    value := fun n => if n = destination then some site else p.value n }

theorem acquisition_establishes_its_own_conditional_name (p : World)
    (site destination : Nat) :
    (acquire p site destination).present site = true ∧
    (acquire p site destination).value destination = some site := by
  simp [acquire]

/-- Retaining fewer proven names cannot invent a relation. -/
theorem forgetting_relations_is_sound (paths : List World) (site : Nat)
    (known retained : Nat → Prop)
    (sound : ∀ name, known name → guarantees paths site name)
    (subset : ∀ name, retained name → known name) :
    ∀ name, retained name → guarantees paths site name := by
  intro name h
  exact sound name (subset name h)

/-- A declared release changes exactly the selected object's live state. -/
def release (p : World) (name : Nat) : World :=
  { p with live := fun site => if p.value name = some site then false else p.live site }

theorem conditional_release_discharges_every_present_path (paths : List World)
    (site name : Nat) (h : guarantees paths site name)
    (wellFormed : ∀ p ∈ paths, p.present site = false → p.live site = false) :
    ∀ p ∈ paths, (release p name).live site = false := by
  intro p hp
  cases he : p.present site with
  | true => simp [release, h p hp he]
  | false => simp [release, wellFormed p hp he]

theorem conditional_identity_does_not_name_an_absent_lookup (p : World) (site name : Nat)
    (absent : p.present site = false) : guarantees [p] site name := by
  intro w hw he
  simp only [List.mem_singleton] at hw
  subst w
  simp [absent] at he

structure OpaquePointer where
  targets : List Nat
  unknown : Bool
  mayBeNull : Bool

def narrowOpaque (p : OpaquePointer) (isNull : Bool) : OpaquePointer :=
  if isNull then ⟨[], false, true⟩ else { p with mayBeNull := false }

theorem nonnull_does_not_resolve_identity (p : OpaquePointer) :
    (narrowOpaque p false).unknown = p.unknown ∧
    (narrowOpaque p false).targets = p.targets := ⟨rfl, rfl⟩

theorem null_alternative_has_no_target (p : OpaquePointer) :
    (narrowOpaque p true).targets = [] := rfl

/-- The unknown-identity diagnostic survives a known site's discharge. -/
def certifies (unknownIdentity : Bool) (_knownSiteReleased : Bool) : Bool := !unknownIdentity

theorem a_conditional_discharge_does_not_certify_the_lookup (released : Bool) :
    certifies true released = false := rfl

end QCore.CGatewayRefinements

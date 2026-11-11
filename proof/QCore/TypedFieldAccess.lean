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

import QCore.LegacyCAnalyzer

/-!
# Typed direct scalar-field access classification

The model of the conjunctive whitelist implemented in
`crates/qcore-c-gateway/src/typed_field_access.rs`: which candidate accesses the
importer may model as a custody-neutral instruction, and which keep the lexical
rejection they have today.

This is a model of the *classification decision*, not of C semantics. The
declared types, the record completeness flags and the base-binding flag are
premises supplied by the reader; nothing here proves that the reader extracts
them faithfully from C source, and nothing here is a claim about arbitrary C
memory safety.
-/

namespace QCore.TypedFieldAccess

open QCore.LegacyCAnalyzer

/-- The declared type of an accessed member, as far as the reader commits. -/
inductive MemberType where
  /-- Non-pointer, non-array arithmetic or enumeration type. -/
  | scalar
  | pointer
  | aggregate
  deriving DecidableEq

inductive TagKind where
  | recordTag
  | unionTag
  deriving DecidableEq

/-- What the reader established about the accessed record type. -/
structure RecordFacts where
  kind : TagKind
  /-- Every member declaration of the definition was read. -/
  complete : Bool
  /-- Exactly one definition of the tag exists in this translation unit. -/
  unique : Bool
  deriving DecidableEq

/-- One candidate access with every premise the classifier decides over. -/
structure Access where
  record : RecordFacts
  member : MemberType
  /-- The member carries `volatile` or `_Atomic`. -/
  qualified : Bool
  /-- The base's declared type matches the access operator: a single-star
      pointer to the record for `->`, the record itself for `.`. -/
  baseMatches : Bool
  /-- The projection may bind the base to a resource identity. -/
  baseBound : Bool
  /-- The resource identity the projection holds for the base, if any. -/
  baseResource : Option ResourceId
  /-- The access continues into another member, a subscript or a call. -/
  chained : Bool
  /-- The statement takes an address, dereferences or subscripts. -/
  escapes : Bool
  /-- The statement contains a call. -/
  hasCall : Bool
  /-- Every remaining identifier is an enumeration constant or a declared
      scalar variable of the same function. -/
  operandsScalar : Bool
  /-- Every declaration of the enclosing function was read. -/
  declarationsRead : Bool
  deriving DecidableEq

/-- The whitelist. Every clause is a conjunct, so failing any one refuses. -/
def admits (a : Access) : Bool :=
  a.declarationsRead
    && a.record.kind == TagKind.recordTag
    && a.record.complete
    && a.record.unique
    && a.member == MemberType.scalar
    && !a.qualified
    && a.baseMatches
    && !a.baseBound
    && !a.chained
    && !a.escapes
    && !a.hasCall
    && a.operandsScalar

/-- The instruction the importer emits for a candidate access. -/
def effect (a : Access) : Instruction :=
  if admits a then .noop else .unsupported

theorem admitted_access_is_neutral (a : Access) (ok : admits a = true) :
    effect a = .noop := by
  simp [effect, ok]

theorem refused_access_keeps_the_unsupported_effect (a : Access)
    (refused : admits a = false) : effect a = .unsupported := by
  simp [effect, refused]

/-- The admitted effect changes no custody state at all. -/
theorem admitted_effect_preserves_custody (a : Access) (state : AbstractState)
    (ok : admits a = true) : transfer state (effect a) = state := by
  rw [admitted_access_is_neutral a ok]
  exact noop_preserves_state state

/-- A refused access still poisons precision, so it can never be `safe`. -/
theorem refused_effect_is_never_safe (a : Access) (state : AbstractState)
    (refused : admits a = false) :
    classify (transfer state (effect a)) ≠ .safe := by
  rw [refused_access_keeps_the_unsupported_effect a refused]
  exact unsupported_is_never_safe state

-- Each premise, refused on its own.

theorem a_union_never_admits (a : Access)
    (isUnion : a.record.kind = TagKind.unionTag) : admits a = false := by
  simp [admits, isUnion]

theorem an_incomplete_record_never_admits (a : Access)
    (incomplete : a.record.complete = false) : admits a = false := by
  simp [admits, incomplete]

theorem an_ambiguous_tag_never_admits (a : Access)
    (ambiguous : a.record.unique = false) : admits a = false := by
  simp [admits, ambiguous]

theorem a_pointer_member_never_admits (a : Access)
    (isPointer : a.member = MemberType.pointer) : admits a = false := by
  simp [admits, isPointer]

theorem an_aggregate_member_never_admits (a : Access)
    (isAggregate : a.member = MemberType.aggregate) : admits a = false := by
  simp [admits, isAggregate]

theorem a_qualified_member_never_admits (a : Access)
    (qualified : a.qualified = true) : admits a = false := by
  simp [admits, qualified]

theorem a_mismatched_base_never_admits (a : Access)
    (mismatched : a.baseMatches = false) : admits a = false := by
  simp [admits, mismatched]

theorem a_bound_base_never_admits (a : Access) (bound : a.baseBound = true) :
    admits a = false := by
  simp [admits, bound]

theorem a_chained_access_never_admits (a : Access) (chained : a.chained = true) :
    admits a = false := by
  simp [admits, chained]

theorem an_escaping_statement_never_admits (a : Access)
    (escapes : a.escapes = true) : admits a = false := by
  simp [admits, escapes]

theorem a_call_bearing_statement_never_admits (a : Access)
    (hasCall : a.hasCall = true) : admits a = false := by
  simp [admits, hasCall]

theorem a_nonscalar_operand_never_admits (a : Access)
    (operands : a.operandsScalar = false) : admits a = false := by
  simp [admits, operands]

theorem unread_declarations_never_admit (a : Access)
    (unread : a.declarationsRead = false) : admits a = false := by
  simp [admits, unread]

/-- Premise 8, stated against the reader's obligation: when the base-binding
    flag over-approximates faithfully, an admitted access reads no identity the
    projection tracks, so neutralizing it can erase no released, escaped or
    reallocated state. The over-approximation itself is the Rust reader's
    obligation and is assumed, not proved, here. -/
theorem an_admitted_access_reads_no_tracked_identity (a : Access)
    (faithful : a.baseBound = false → a.baseResource = none)
    (ok : admits a = true) : a.baseResource = none := by
  refine faithful ?_
  cases bound : a.baseBound with
  | false => rfl
  | true =>
      rw [a_bound_base_never_admits a bound] at ok
      exact Bool.noConfusion ok

/-- The behavior the fail-closed pin protects: a base the projection may bind
    is never classified, so a read after that base's release keeps the
    unsupported effect and the program stays indeterminate. -/
theorem an_access_through_a_released_base_stays_indeterminate (a : Access)
    (resource : ResourceId) (bound : a.baseBound = true) :
    analyze [.acquire resource, .release resource, effect a] = .indeterminate := by
  rw [refused_access_keeps_the_unsupported_effect a (a_bound_base_never_admits a bound)]
  exact released_then_unmodeled_access_is_indeterminate resource

/-- The admitted classification does not hide a leak: the discharge is still
    required after the neutral instruction. -/
theorem an_admitted_access_does_not_hide_a_leak (a : Access)
    (resource : ResourceId) (ok : admits a = true) :
    analyze [.acquire resource, effect a] = .defect := by
  rw [admitted_access_is_neutral a ok]
  exact noop_does_not_hide_a_leak resource

/-- And a discharged lifetime completes across it. -/
theorem an_admitted_access_preserves_a_discharged_lifetime (a : Access)
    (resource : ResourceId) (ok : admits a = true) :
    analyze [.acquire resource, effect a, .release resource] = .safe := by
  rw [admitted_access_is_neutral a ok]
  exact noop_between_acquire_and_release_is_safe resource

/-- Non-vacuity: the whitelist admits something. Without this the theorems
    above would hold for an empty admitted set. -/
theorem the_whitelist_is_not_empty : ∃ a : Access, admits a = true := by
  refine ⟨{
    record := { kind := TagKind.recordTag, complete := true, unique := true }
    member := MemberType.scalar
    qualified := false
    baseMatches := true
    baseBound := false
    baseResource := none
    chained := false
    escapes := false
    hasCall := false
    operandsScalar := true
    declarationsRead := true }, ?_⟩
  decide

end QCore.TypedFieldAccess

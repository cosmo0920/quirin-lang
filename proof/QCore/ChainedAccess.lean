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

import QCore.ExitResiduals

/-!
# The bounded access chain (O04-ARMS)

`QCore.ExitResiduals` models card C02-P's borrowed-actual whitelist, in which
`Access.chained` is a *refusing* premise: `a->b->c` is refused by construction,
and `docs/reports/remaining-o04/README.md` §7 records that as the reason the
caller's logging arm C1 stays `Unknown`.

This module models the replacement decided by
`crates/qcore-c-gateway/src/borrowed_actual.rs::admitted_access`: an access is
a base followed by one to `limit` member selections, where every selection
*before the last* must be declared exactly `struct POINTEE *` over a complete,
unambiguous, non-union record, and the last must satisfy C02-P's own member
premise and must not continue.

Two directions are proved, and both matter.

* **Refusal.** Every link the reader did not fully type refuses the *whole*
  chain, wherever it sits in it, and a chain longer than the bound is refused
  rather than truncated. A refused chain refuses its statement, is never
  `Safe`, and alleges no residual.
* **Extension, not replacement.** At depth one the rule is *exactly* C02-P's,
  so nothing C02-P refused becomes admitted by accident, and the rule is shown
  non-vacuous at depth three.

This is a model of the *decision*, not of C semantics. That the reader extracts
these declaration facts faithfully is its own obligation and is assumed here,
exactly as in `QCore.TypedFieldAccess`. Nothing here relates two distinct
declared names that may denote the same object: the chain inherits C02-P's
recorded may-alias boundary unchanged, and widens nothing about it.
-/

namespace QCore.ChainedAccess

open QCore.LegacyCAnalyzer
open QCore.TypedFieldAccess
open QCore.ExitResiduals

/-- One member selection of a chain: what the reader established about the
    record the selection happens *in*, and how the selected member is declared. -/
structure Link where
  /-- Facts about the record the selection reads a member of. -/
  record : RecordFacts
  /-- How that member is declared. -/
  member : MemberType
  /-- The member carries `volatile` or `_Atomic`. -/
  qualified : Bool
  /-- The selection is written `->` and the member is declared *exactly*
      `struct POINTEE *`, so the walk may re-enter the model at `POINTEE`.
      A `union` pointee, a typedef the reader will not resolve, a member with
      two or more stars and a record-valued member all clear this flag. -/
  continuable : Bool
  deriving DecidableEq

/-- One candidate chained access. -/
structure Chain where
  /-- The base's declared type matches the first access operator. -/
  baseMatches : Bool
  /-- The projection may bind the base to a resource identity. -/
  baseBound : Bool
  /-- Every declaration of the enclosing function was read. -/
  declarationsRead : Bool
  /-- The selections, outermost first. -/
  links : List Link
  /-- The access continues into `->`, `.`, `[` or `(` after the last selection. -/
  continuesAfterLast : Bool
  deriving DecidableEq

/-- The chain-length bound the walker enforces. -/
def limit : Nat := 4

/-- C02-P's member premise, unchanged: a scalar or pointer member of a complete,
    unambiguous `struct`, carrying no rejecting qualifier. -/
def finalOk (l : Link) : Bool :=
  l.record.kind == TagKind.recordTag
    && l.record.complete
    && l.record.unique
    && (l.member == MemberType.scalar || l.member == MemberType.pointer)
    && !l.qualified

/-- An intermediate link must additionally be *continuable*: only a member the
    reader typed as exactly `struct POINTEE *` lets the walk go on. -/
def intermediateOk (l : Link) : Bool :=
  finalOk l && l.member == MemberType.pointer && l.continuable

/-- Every selection but the last must be an intermediate link; the last must
    satisfy `finalOk`. An empty chain is not an access at all. -/
def linksOk : List Link → Bool
  | [] => false
  | l :: rest => if rest.isEmpty then finalOk l else intermediateOk l && linksOk rest

/-- The whitelist. Every clause is a conjunct, so failing any one refuses. -/
def admitsChain (c : Chain) : Bool :=
  c.declarationsRead
    && c.baseMatches
    && !c.baseBound
    && !c.continuesAfterLast
    && c.links.length ≤ limit
    && linksOk c.links

/-! ## Refusal: every untyped link refuses the whole chain -/

theorem an_empty_chain_never_admits (c : Chain) (empty : c.links = []) :
    admitsChain c = false := by
  simp [admitsChain, empty, linksOk]

theorem unread_declarations_never_admit_a_chain (c : Chain)
    (unread : c.declarationsRead = false) : admitsChain c = false := by
  simp [admitsChain, unread]

theorem a_mismatched_base_never_admits_a_chain (c : Chain)
    (mismatch : c.baseMatches = false) : admitsChain c = false := by
  simp [admitsChain, mismatch]

/-- C02-P premise (4) is unchanged at every depth: a base the projection may
    bind refuses the whole chain. -/
theorem a_bound_root_never_admits_a_chain (c : Chain) (bound : c.baseBound = true) :
    admitsChain c = false := by
  simp [admitsChain, bound]

theorem a_continued_access_never_admits (c : Chain)
    (continues : c.continuesAfterLast = true) : admitsChain c = false := by
  simp [admitsChain, continues]

/-- A chain longer than the bound is refused, never silently truncated to an
    admitted prefix. -/
theorem a_chain_beyond_the_bound_never_admits (c : Chain)
    (long : c.links.length > limit) : admitsChain c = false := by
  have : ¬ c.links.length ≤ limit := Nat.not_le.mpr long
  simp [admitsChain, this]

/-- The workhorse: a link that satisfies neither role refuses the list it sits
    in, wherever it sits. -/
theorem linksOk_false_of_bad_link :
    ∀ (ls : List Link) (l : Link), l ∈ ls → finalOk l = false → linksOk ls = false := by
  intro ls
  induction ls with
  | nil => intro l mem _; cases mem
  | cons head rest ih =>
    intro l mem bad
    cases mem with
    | head =>
      cases h : rest.isEmpty with
      | true => simp [linksOk, h, bad]
      | false => simp [linksOk, h, intermediateOk, bad]
    | tail _ inRest =>
      have restNonEmpty : rest.isEmpty = false := by
        cases rest with
        | nil => cases inRest
        | cons _ _ => rfl
      simp [linksOk, restNonEmpty, ih l inRest bad]

theorem a_bad_link_never_admits (c : Chain) (l : Link) (mem : l ∈ c.links)
    (bad : finalOk l = false) : admitsChain c = false := by
  simp [admitsChain, linksOk_false_of_bad_link c.links l mem bad]

theorem a_union_link_never_admits (c : Chain) (l : Link) (mem : l ∈ c.links)
    (union : l.record.kind = TagKind.unionTag) : admitsChain c = false :=
  a_bad_link_never_admits c l mem (by simp [finalOk, union])

theorem an_incomplete_link_never_admits (c : Chain) (l : Link) (mem : l ∈ c.links)
    (incomplete : l.record.complete = false) : admitsChain c = false :=
  a_bad_link_never_admits c l mem (by simp [finalOk, incomplete])

theorem an_ambiguous_link_never_admits (c : Chain) (l : Link) (mem : l ∈ c.links)
    (ambiguous : l.record.unique = false) : admitsChain c = false :=
  a_bad_link_never_admits c l mem (by simp [finalOk, ambiguous])

theorem a_qualified_link_never_admits (c : Chain) (l : Link) (mem : l ∈ c.links)
    (qualified : l.qualified = true) : admitsChain c = false :=
  a_bad_link_never_admits c l mem (by simp [finalOk, qualified])

theorem an_aggregate_link_never_admits (c : Chain) (l : Link) (mem : l ∈ c.links)
    (aggregate : l.member = MemberType.aggregate) : admitsChain c = false :=
  a_bad_link_never_admits c l mem (by simp [finalOk, aggregate])

/-- The clause the chain rule adds on top of C02-P: an intermediate link that is
    not a fully typed `struct POINTEE *` refuses, even when it would have been a
    perfectly good *final* member. This is what refuses `a->b->c` when `b` is a
    `union` pointer, a double pointer or a typedef the reader will not resolve. -/
theorem linksOk_false_of_unContinuable :
    ∀ (ls : List Link) (l : Link) (rest : List Link),
      ls = l :: rest → rest ≠ [] → intermediateOk l = false → linksOk ls = false := by
  intro ls l rest shape nonEmpty bad
  subst shape
  have restNonEmpty : rest.isEmpty = false := by
    cases rest with
    | nil => exact absurd rfl nonEmpty
    | cons _ _ => rfl
  simp [linksOk, restNonEmpty, bad]

theorem a_non_continuable_intermediate_never_admits (c : Chain) (l : Link)
    (rest : List Link) (shape : c.links = l :: rest) (nonEmpty : rest ≠ [])
    (notTyped : l.continuable = false) : admitsChain c = false := by
  have bad : intermediateOk l = false := by simp [intermediateOk, notTyped]
  simp [admitsChain, linksOk_false_of_unContinuable c.links l rest shape nonEmpty bad]

/-- A scalar member ends a chain; it can never carry it on. -/
theorem a_scalar_intermediate_never_admits (c : Chain) (l : Link)
    (rest : List Link) (shape : c.links = l :: rest) (nonEmpty : rest ≠ [])
    (scalar : l.member = MemberType.scalar) : admitsChain c = false := by
  have bad : intermediateOk l = false := by simp [intermediateOk, scalar]
  simp [admitsChain, linksOk_false_of_unContinuable c.links l rest shape nonEmpty bad]

/-! ## Extension, not replacement -/

/-- The `Access` a depth-one chain denotes, in C02-P's own vocabulary. -/
def directAccess (c : Chain) (l : Link) : Access :=
  { record := l.record
    member := l.member
    qualified := l.qualified
    baseMatches := c.baseMatches
    baseBound := c.baseBound
    baseResource := none
    chained := c.continuesAfterLast
    escapes := false
    hasCall := true
    operandsScalar := true
    declarationsRead := c.declarationsRead }

/-- At depth one the chain rule decides *exactly* what C02-P's borrowed-actual
    whitelist decides, so it is a strict extension: no shape C02-P refused at
    depth one becomes admitted, and none it admitted becomes refused. -/
theorem a_depth_one_chain_is_the_direct_admission (c : Chain) (l : Link)
    (single : c.links = [l]) :
    admitsChain c
      = admitsActual { access := directAccess c l
                     , position := Position.declared ParameterMode.borrowed
                     , wholeArgument := true } := by
  simp [admitsChain, admitsActual, directAccess, single, linksOk, finalOk, limit]
  cases c.declarationsRead <;> cases c.baseMatches <;> cases c.baseBound <;>
    cases c.continuesAfterLast <;> cases l.qualified <;>
    cases hk : l.record.kind <;> cases l.record.complete <;> cases l.record.unique <;>
    cases hm : l.member <;> simp_all

/-- Non-vacuity: a three-link chain over fully typed records is admitted, so
    every refusal above is a refusal and not an empty rule. -/
theorem the_chain_rule_is_not_vacuous :
    admitsChain
      { baseMatches := true
        baseBound := false
        declarationsRead := true
        continuesAfterLast := false
        links :=
          [ { record := ⟨TagKind.recordTag, true, true⟩
              member := MemberType.pointer, qualified := false, continuable := true }
          , { record := ⟨TagKind.recordTag, true, true⟩
              member := MemberType.pointer, qualified := false, continuable := true }
          , { record := ⟨TagKind.recordTag, true, true⟩
              member := MemberType.pointer, qualified := false, continuable := false } ] }
      = true := by
  decide

/-- And the same chain with one `union` link is refused: the two theorems above
    and this one together show the rule separates. -/
theorem one_union_link_flips_the_admitted_chain :
    admitsChain
      { baseMatches := true
        baseBound := false
        declarationsRead := true
        continuesAfterLast := false
        links :=
          [ { record := ⟨TagKind.recordTag, true, true⟩
              member := MemberType.pointer, qualified := false, continuable := true }
          , { record := ⟨TagKind.unionTag, true, true⟩
              member := MemberType.pointer, qualified := false, continuable := true }
          , { record := ⟨TagKind.recordTag, true, true⟩
              member := MemberType.pointer, qualified := false, continuable := false } ] }
      = false := by
  decide

/-- A chain of exactly `limit` links is admitted and one link longer is not, so
    the bound is the bound the walker claims. -/
theorem the_bound_is_exactly_four :
    admitsChain
        { baseMatches := true, baseBound := false, declarationsRead := true
          continuesAfterLast := false
          links := List.replicate 3
              { record := ⟨TagKind.recordTag, true, true⟩
                member := MemberType.pointer, qualified := false, continuable := true }
            ++ [ { record := ⟨TagKind.recordTag, true, true⟩
                   member := MemberType.pointer, qualified := false
                   continuable := false } ] } = true
    ∧ admitsChain
        { baseMatches := true, baseBound := false, declarationsRead := true
          continuesAfterLast := false
          links := List.replicate 4
              { record := ⟨TagKind.recordTag, true, true⟩
                member := MemberType.pointer, qualified := false, continuable := true }
            ++ [ { record := ⟨TagKind.recordTag, true, true⟩
                   member := MemberType.pointer, qualified := false
                   continuable := false } ] } = false := by
  constructor <;> decide

/-! ## Bridging into the statement classification -/

/-- The actual a chained access occupies, at a declared borrowed position. -/
def chainedActual (c : Chain) (l : Link) : Actual :=
  { access := directAccess c l
    position := Position.declared ParameterMode.borrowed
    wholeArgument := true }

/-- Fail-closed, first direction: a refused chain leaves its actual unadmitted,
    so the whole statement is refused and keeps its existing diagnostic. -/
theorem a_refused_chain_refuses_the_statement (c : Chain) (l : Link)
    (single : c.links = [l]) (refused : admitsChain c = false)
    (other : Bool) (actuals : List Actual) (mem : chainedActual c l ∈ actuals) :
    classifyStatement actuals other = .refused := by
  refine an_unadmitted_actual_refuses_the_statement actuals other _ mem ?_
  have := a_depth_one_chain_is_the_direct_admission c l single
  simpa [chainedActual, refused] using this.symm.trans refused

/-- Fail-closed, second direction: a refused chain is a reason to say nothing,
    never a reason to accuse. The statement it refuses contributes
    `Instruction.unsupported`, which can never be classified `safe` — and an
    `unsupported` instruction is not a leak claim. -/
theorem a_refused_chain_is_never_safe (actuals : List Actual) (other : Bool)
    (state : AbstractState)
    (refused : classifyStatement actuals other = .refused) :
    classify (transfer state .unsupported) ≠ .safe :=
  a_refused_statement_is_never_safe actuals other state refused

/-- The admission removes only the spans it justified: any *other* indirect
    access in the same statement still refuses it, however many chains were
    admitted. -/
theorem an_admitted_chain_does_not_rescue_a_statement (actuals : List Actual) :
    classifyStatement actuals true = .refused :=
  a_remaining_indirect_access_still_refuses actuals

end QCore.ChainedAccess

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

import QCore.ReturnCallCustody

/-!
# Two refused `return` families, admitted with exact custody (W45-RETURN-EXPRESSION-FAMILIES)

The rule this module models is `crates/qcore-c-gateway/src/legacy_return_family.rs`,
consulted by `Parser::return_expression_instruction` only after every older
route (constraint 5b's strict and pointer-leaf rules, the cast return, the W4
whole call, both B01 routes and W43's refused-whole-call lowering) declined.

## S — the scalar-result read

`return E;` in a function whose definition declares a scalar value result. C
converts `E` to that type, so the returned value carries **no custody** and the
exit escapes nothing. `E` must be a read: no call, assignment, increment,
comma, brace, GNU `?:` or unary `&`, a type operator only over a resolved,
never variably modified type name, and no tracked descriptor mentioned. Every
name it evaluates is untracked (never bound by the body, C02-P premise 4) or a
declared scalar, except that a name the body may bind is admitted only as the
root of exactly one dereference C always evaluates. Every directly
dereferenced non-scalar root is a **base** of the read node and keeps the typed
access route's flow obligation: unbound or live passes, a released identity is
accused, anything else refuses.

Proved: the rule fires only for a scalar result and only after every older
route declined; an admitted read mentions no descriptor; every bound name it
reads is an unconditionally evaluated direct root, and it is a base; the exit
escapes nothing, so it discharges nothing and withdraws no leak the refusal
reported; under premise 4 (an untracked name is never released by the body) an
accusation comes only from a dereference C always evaluates; a pointer-valued
result is never admitted.

## K — a cast of a whole call

`return ( T1 ) … ( Tn ) CALL ( … );`, possibly parenthesised. The call is
classified as W4 classifies it. A refusal is lowered to itself (W43's
re-attribution, still poisoned). A `none` result is admitted under any cast. An
`owned` result is admitted only when every cast targets a pointer type and the
unit declares `CALL` to return an object pointer, so the conversion keeps the
object (C11 6.3.2.3p7) and the exit is W4's `returnCall .owned`, which escapes
only the fresh site (`QCore.ReturnCallCustody`). With no cast the answer is W4's
own. Everything else keeps the return refusal and restores the site counter.

Proved: the admission discharges nothing the body owed before the call; an
integer cast of an owned result, and a reallocated or derived result under a
cast, keep the refusal; a refused call is lowered to its own refusal and is
still poisoned; parentheses alone give W4's answer; the evaluation node
precedes the exit; declined probes restore the counter; both families are
non-vacuous.

## Premises, stated and not proved here

Faithful C token extraction, the declaration readers (the result carrier, a
type name, an object-pointer declaration), `UntrackedBases::never_bound` as an
over-approximation of the names the body may bind, `DescriptorNames` as an
over-approximation of the names that may hold a descriptor, and the
correspondence between this model and the Rust are review premises under
standing constraint 28.
-/

namespace QCore.ReturnExpressionFamilies

/-! ## S — the scalar-result read -/

/-- How one occurrence of a name in `E` is used. -/
inductive Use where
  /-- Read as a value (compared, computed with, converted). -/
  | value
  /-- The root of exactly one dereference: `B->F`, `B->F.G`, `B[I]`, `*B`. -/
  | directDeref
  /-- Under a second dereference, or inside a parenthesised operand. -/
  | otherDeref
  deriving DecidableEq, Repr

/-- One occurrence of a name `E` evaluates, with what the readers say of it. -/
structure Occurrence where
  /-- `UntrackedBases::never_bound`: the body never binds it to an identity. -/
  untracked : Bool
  /-- A declared scalar. -/
  declaredScalar : Bool
  /-- `DescriptorNames`: it may hold a tracked descriptor. -/
  descriptor : Bool
  use : Use
  /-- After a `?`, `&&` or `||` of its group: C may skip it. -/
  conditional : Bool
  deriving DecidableEq, Repr

/-- One `return` the family is asked about. -/
structure ScalarSite where
  /-- Some older route already decided the expression. -/
  earlierRouteAdmitted : Bool
  /-- The definition declares a scalar value result. -/
  scalarResult : Bool
  /-- No call, assignment, increment, comma, brace, `?:` or unary `&`, and
      every type operator is over a resolved, never variably modified type. -/
  aRead : Bool
  occurrences : List Occurrence
  deriving DecidableEq, Repr

/-- A bound name: neither untracked nor a declared scalar. -/
def Occurrence.bound (o : Occurrence) : Bool := !o.untracked && !o.declaredScalar

/-- The per-occurrence premise of `scalar_read`. -/
def occurrenceAdmitted (o : Occurrence) : Bool :=
  !o.descriptor && (!o.bound || (o.use == .directDeref && !o.conditional))

/-- The read node's bases: every directly dereferenced occurrence that is not
    a declared scalar the body may bind (a scalar is never dereferenced). -/
def isBase (o : Occurrence) : Bool :=
  o.use == .directDeref && (o.untracked || !o.declaredScalar)

/-- What the parser emits. -/
inductive Lowering where
  | earlier
  /-- The return-position refusal at the exit. -/
  | refuse
  /-- The read node over these bases, then a `NoOp` exit. -/
  | read (bases : List Occurrence)
  deriving DecidableEq, Repr

def before (s : ScalarSite) : Lowering := if s.earlierRouteAdmitted then .earlier else .refuse

def after (s : ScalarSite) : Lowering :=
  if s.earlierRouteAdmitted then .earlier
  else if s.scalarResult && s.aRead && s.occurrences.all occurrenceAdmitted then
    .read (s.occurrences.filter isBase)
  else .refuse

theorem an_earlier_admission_is_unchanged (s : ScalarSite)
    (h : s.earlierRouteAdmitted = true) : after s = before s := by
  simp [after, before, h]

theorem the_rule_only_replaces_the_return_refusal (s : ScalarSite)
    (moved : after s ≠ before s) :
    before s = .refuse ∧ ∃ bases, after s = .read bases := by
  cases hE : s.earlierRouteAdmitted with
  | true => exact absurd (an_earlier_admission_is_unchanged s hE) moved
  | false =>
    refine ⟨by simp [before, hE], ?_⟩
    by_cases hc : (s.scalarResult && s.aRead && s.occurrences.all occurrenceAdmitted) = true
    · exact ⟨s.occurrences.filter isBase, by simp [after, hE, hc]⟩
    · exact absurd (by simp [after, before, hE, hc]) moved

theorem only_a_scalar_result_is_admitted (s : ScalarSite) (bases : List Occurrence)
    (h : after s = .read bases) : s.scalarResult = true ∧ s.aRead = true := by
  unfold after at h
  split at h
  · cases h
  · split at h
    · rename_i hc
      simp only [Bool.and_eq_true] at hc
      exact ⟨hc.1.1, hc.1.2⟩
    · cases h

/-- A pointer-valued result (an interior address, an array member's decay or a
    load) is never this rule's. -/
theorem a_pointer_result_is_never_admitted (s : ScalarSite)
    (h : s.scalarResult = false) : after s ≠ .read bases := by
  intro hr
  have := (only_a_scalar_result_is_admitted s bases hr).1
  simp [h] at this

theorem every_occurrence_of_an_admitted_read_is_admitted (s : ScalarSite)
    (bases : List Occurrence) (h : after s = .read bases) :
    ∀ o ∈ s.occurrences, occurrenceAdmitted o = true := by
  unfold after at h
  split at h
  · cases h
  · split at h
    · rename_i hc
      simp only [Bool.and_eq_true, List.all_eq_true] at hc
      exact hc.2
    · cases h

/-- A tracked descriptor is custody, never data: no admitted read mentions one. -/
theorem an_admitted_read_mentions_no_descriptor (s : ScalarSite)
    (bases : List Occurrence) (h : after s = .read bases) :
    ∀ o ∈ s.occurrences, o.descriptor = false := by
  intro o ho
  have := every_occurrence_of_an_admitted_read_is_admitted s bases h o ho
  simp only [occurrenceAdmitted, Bool.and_eq_true, Bool.not_eq_true'] at this
  exact this.1

/-- Every name the body may bind is the root of one dereference C always
    evaluates — never a value, never under a second dereference, never in a
    position C may skip. -/
theorem a_bound_name_is_an_unconditional_direct_root (s : ScalarSite)
    (bases : List Occurrence) (h : after s = .read bases)
    (o : Occurrence) (ho : o ∈ s.occurrences) (hb : o.bound = true) :
    o.use = .directDeref ∧ o.conditional = false := by
  have := every_occurrence_of_an_admitted_read_is_admitted s bases h o ho
  simp only [occurrenceAdmitted, hb, Bool.not_true, Bool.false_or, Bool.and_eq_true,
    beq_iff_eq, Bool.not_eq_true'] at this
  exact this.2

/-- …and every such root keeps its flow obligation: it is a base. -/
theorem every_bound_dereference_is_a_base (s : ScalarSite)
    (bases : List Occurrence) (h : after s = .read bases)
    (o : Occurrence) (ho : o ∈ s.occurrences) (hb : o.bound = true) : o ∈ bases := by
  have hu := a_bound_name_is_an_unconditional_direct_root s bases h o ho hb
  unfold after at h
  split at h
  · cases h
  · split at h
    · cases h
      simp only [Occurrence.bound, Bool.and_eq_true, Bool.not_eq_true'] at hb
      simp [isBase, hu.1, hb.2, ho]
    · cases h

/-! ### The flow obligation of a base -/

/-- What the state holds for one base at the read node. -/
inductive BaseState where
  | unbound
  | live
  | released
  /-- Unknown, possibly null, ambiguous or carrying an unresolved effect. -/
  | unresolved
  deriving DecidableEq, Repr

/-- `indirect_scalar_base_resolves` and the `use_after_release` pass. -/
inductive Check where
  | passes
  | accuses
  | refuses
  deriving DecidableEq, Repr

def check : BaseState → Check
  | .unbound => .passes
  | .live => .passes
  | .released => .accuses
  | .unresolved => .refuses

/-- The read node accuses when some base it checks is released. -/
def accuses (bases : List Occurrence) (state : Occurrence → BaseState) : Bool :=
  bases.any fun o => check (state o) == .accuses

/-- **An accusation is about a dereference C performs.** Under premise 4 — the
    body never releases a name it never binds — every accusation of an
    admitted read comes from a base C evaluates on every path through `E`. -/
theorem an_accusation_is_on_an_evaluated_dereference (s : ScalarSite)
    (bases : List Occurrence) (h : after s = .read bases)
    (state : Occurrence → BaseState)
    (premise4 : ∀ o, o.untracked = true → state o ≠ .released)
    (o : Occurrence) (hob : o ∈ bases) (hr : state o = .released) :
    o.conditional = false := by
  have hmem : o ∈ s.occurrences ∧ isBase o = true := by
    unfold after at h
    split at h
    · cases h
    · split at h
      · cases h
        exact List.mem_filter.mp hob
      · cases h
  have hu : o.untracked = false := by
    cases hu : o.untracked
    · rfl
    · exact absurd hr (premise4 o hu)
  have hsc : o.declaredScalar = false := by
    have := hmem.2
    simp only [isBase, hu, Bool.false_or, Bool.and_eq_true, Bool.not_eq_true'] at this
    exact this.2
  have hb : o.bound = true := by simp [Occurrence.bound, hu, hsc]
  exact (a_bound_name_is_an_unconditional_direct_root s bases h o hmem.1 hb).2

/-! ### The exit -/

/-- The custody a `return` hands its caller. -/
inductive Returned where
  | nothing
  | owned
  | derived
  deriving DecidableEq, Repr

/-- The scalar read's exit is a `NoOp`: a number carries nothing. -/
def returned : Lowering → Option Returned
  | .read _ => some .nothing
  | _ => none

theorem a_scalar_read_returns_nothing (bases : List Occurrence) :
    returned (.read bases) = some .nothing := rfl

/-- `report_leaks` at the exit. The return refusal withdrew every identity its
    expression handed on (the refused hand-on rule); the read withdraws
    nothing and escapes nothing, so it reports every live obligation. -/
def reportsLeak (l : Lowering) (live handedOn : Bool) : Bool :=
  match l with
  | .refuse => live && !handedOn
  | .read _ => live
  | .earlier => false

/-- **No leak is withdrawn by the read**: whatever the refusal reported, the
    read reports too — nothing is modelled as escaping. -/
theorem the_read_withdraws_no_leak (bases : List Occurrence) (live handedOn : Bool)
    (h : reportsLeak .refuse live handedOn = true) :
    reportsLeak (.read bases) live handedOn = true := by
  simp only [reportsLeak, Bool.and_eq_true] at h ⊢
  exact h.1

/-! ## K — a cast of a whole call -/

open QCore.ReturnCallCustody (State Wf returnCall admitted_modes_discharge_nothing_prior)

/-- `ResourceReturnMode` as the cast family distinguishes it. -/
inductive Mode where
  | none
  | owned
  | reallocated
  | derived
  deriving DecidableEq, Repr

/-- What the statement classifier answers for `PLACE = CALL(…);`. -/
inductive Classified where
  | refused (description : String)
  /-- A `ContractCall` into the reserved place with this result. -/
  | call (mode : Mode)
  /-- Anything else. -/
  | other
  deriving DecidableEq, Repr

structure CastSite where
  /-- One entry per cast, `true` for a pointer target; `[]` for parentheses. -/
  targets : List Bool
  /-- The unit declares `CALL` to return an object pointer. -/
  objectPointerDecl : Bool
  /-- `admit_derived_returns` (the derived-result validation CFG). -/
  admitDerived : Bool
  /-- The body spells the reserved place. -/
  placeSpelled : Bool
  classified : Classified
  deriving DecidableEq, Repr

inductive CastLowering where
  | refuse
  /-- The classifier's refusal as an evaluation node, then a `NoOp` exit. -/
  | lowered (description : String)
  /-- The admitted call as an evaluation node, then `EscapeIfOwned PLACE`. -/
  | call (mode : Mode)
  deriving DecidableEq, Repr

/-- W4's own admission of an uncast whole call. -/
def w4Admits (admitDerived : Bool) : Mode → Bool
  | .none => true
  | .owned => true
  | .reallocated => false
  | .derived => admitDerived

def castAdmits (c : CastSite) : Mode → Bool
  | .none => true
  | .owned => c.targets.all id && c.objectPointerDecl
  | .reallocated => false
  | .derived => false

/-- The admission the rule asks of a classified call: W4's with no cast. -/
def admitsCall (c : CastSite) (m : Mode) : Bool :=
  if c.targets.isEmpty then w4Admits c.admitDerived m else castAdmits c m

def decideCast (c : CastSite) : CastLowering :=
  if c.placeSpelled then .refuse
  else match c.classified with
    | .refused d => .lowered d
    | .call m => bif admitsCall c m then .call m else .refuse
    | .other => .refuse

/-- What an admitted cast call rests on. -/
theorem an_admitted_cast_call_rests_on_its_admission (c : CastSite) (m : Mode)
    (h : decideCast c = .call m) :
    c.placeSpelled = false ∧ c.classified = .call m ∧ admitsCall c m = true := by
  unfold decideCast at h
  cases hp : c.placeSpelled
  · cases hc : c.classified with
    | refused d => simp [hp, hc] at h
    | other => simp [hp, hc] at h
    | call m' =>
      cases ha : admitsCall c m'
      · simp [hp, hc, ha] at h
      · simp [hp, hc, ha] at h
        subst h
        exact ⟨rfl, rfl, ha⟩
  · simp [hp] at h

theorem a_refused_call_is_lowered_to_its_own_refusal (c : CastSite) (d : String)
    (hp : c.placeSpelled = false) (h : c.classified = .refused d) :
    decideCast c = .lowered d := by
  simp [decideCast, hp, h]

/-- Parentheses alone give W4's answer, derived results under
    `admit_derived_returns` included. -/
theorem parentheses_alone_are_w4 (c : CastSite) (m : Mode)
    (hp : c.placeSpelled = false) (ht : c.targets = []) (h : c.classified = .call m) :
    decideCast c = bif w4Admits c.admitDerived m then .call m else .refuse := by
  simp [decideCast, hp, ht, h, admitsCall]

theorem targets_nonempty (c : CastSite) (hne : c.targets ≠ []) : c.targets.isEmpty = false := by
  cases ht : c.targets with
  | nil => exact absurd ht hne
  | cons _ _ => rfl

/-- An owned result escapes under casts only when every cast is a pointer
    conversion of a declared object pointer. -/
theorem an_owned_result_under_casts_is_a_pointer_conversion (c : CastSite)
    (hne : c.targets ≠ []) (h : decideCast c = .call .owned) :
    c.targets.all id = true ∧ c.objectPointerDecl = true := by
  have ha := (an_admitted_cast_call_rests_on_its_admission c .owned h).2.2
  simp only [admitsCall, targets_nonempty c hne, castAdmits, Bool.false_eq_true,
    ↓reduceIte, Bool.and_eq_true] at ha
  exact ha

/-- An integer cast of an owned result crosses carriers and keeps the refusal. -/
theorem an_integer_cast_of_an_owned_result_is_refused (c : CastSite)
    (hp : c.placeSpelled = false) (h : c.classified = .call .owned)
    (hint : false ∈ c.targets) : decideCast c = .refuse := by
  have hne : c.targets ≠ [] := by
    intro ht
    rw [ht] at hint
    cases hint
  have hall : c.targets.all id = false := by
    cases ha : c.targets.all id
    · rfl
    · rw [List.all_eq_true] at ha
      exact absurd (ha false hint) (by simp)
  simp [decideCast, hp, h, admitsCall, targets_nonempty c hne, castAdmits, hall]

/-- A reallocated or derived result under a cast keeps the refusal: returning
    it would hand the caller the replaced parameter's obligation, or an alias of
    an object whose custody is not fresh. -/
theorem a_reallocated_or_derived_result_under_a_cast_is_refused (c : CastSite) (m : Mode)
    (hp : c.placeSpelled = false) (hm : m = .reallocated ∨ m = .derived)
    (h : c.classified = .call m) (hne : c.targets ≠ []) : decideCast c = .refuse := by
  rcases hm with hm | hm <;> subst hm <;>
    simp [decideCast, hp, h, admitsCall, targets_nonempty c hne, castAdmits]

/-- The custody mode of an admitted cast call, as `QCore.ReturnCallCustody`
    states it: `none` and `owned` are the only modes that reach the exit. -/
def custodyMode : Mode → Option QCore.ReturnCallCustody.ResultMode
  | .none => some .none
  | .owned => some .owned
  | _ => none

/-- **The admission discharges nothing the body owed.** Outside the
    derived-result validation CFG, an admitted cast call's exit is W4's
    `returnCall` of a `none` or `owned` result (the pointer conversion keeps the
    object), which leaves every site of the pre-state as it found it. -/
theorem an_admitted_cast_call_discharges_nothing_prior (c : CastSite) (m : Mode)
    (hd : c.admitDerived = false) (h : decideCast c = .call m)
    (st : State) (s : Nat) (hs : s < st.nextSite) :
    ∃ r, custodyMode m = some r ∧ (returnCall r st).custody s = st.custody s := by
  have ha := (an_admitted_cast_call_rests_on_its_admission c m h).2.2
  have hm : m = .none ∨ m = .owned := by
    cases m with
    | none => exact Or.inl rfl
    | owned => exact Or.inr rfl
    | reallocated =>
      cases hE : c.targets.isEmpty <;> simp [admitsCall, hE, w4Admits, castAdmits] at ha
    | derived =>
      cases hE : c.targets.isEmpty <;> simp [admitsCall, hE, w4Admits, castAdmits, hd] at ha
  rcases hm with hm | hm <;> subst hm
  · exact ⟨_, rfl, admitted_modes_discharge_nothing_prior .none st s rfl hs⟩
  · exact ⟨_, rfl, admitted_modes_discharge_nothing_prior .owned st s rfl hs⟩

/-! ## Evaluation order and site identity -/

/-- The node sequence one lowering emits, evaluation first. -/
def nodes : CastLowering → List String
  | .refuse => ["exit"]
  | .lowered _ => ["evaluate", "exit"]
  | .call _ => ["evaluate", "exit"]

/-- Every call or read of the expression is evaluated before the return. -/
theorem the_evaluation_precedes_the_exit (l : CastLowering) (h : l ≠ .refuse) :
    nodes l = ["evaluate", "exit"] := by
  cases l <;> simp_all [nodes]

/-- The counter across one probe: an admitted call keeps the sites it drew; a
    refusal lowered or kept restores the counter. -/
def counterAfter (l : CastLowering) (nextSite drawn : Nat) : Nat :=
  match l with
  | .call _ => nextSite + drawn
  | _ => nextSite

theorem a_declined_or_lowered_probe_restores_the_counter (l : CastLowering)
    (nextSite drawn : Nat) (h : ∀ m, l ≠ .call m) : counterAfter l nextSite drawn = nextSite := by
  cases l with
  | call m => exact absurd rfl (h m)
  | _ => rfl

/-! ## Non-vacuity -/

/-- `return (item->type & 0xFF) == (1 << 5);` over an untracked formal. -/
def accessorWitness : ScalarSite where
  earlierRouteAdmitted := false
  scalarResult := true
  aRead := true
  occurrences := [⟨true, false, false, .directDeref, false⟩]

/-- `return f ? b->n : 0;` with `b` bound by the body. -/
def skippedBoundWitness : ScalarSite where
  earlierRouteAdmitted := false
  scalarResult := true
  aRead := true
  occurrences := [⟨false, true, false, .value, false⟩, ⟨false, false, false, .directDeref, true⟩]

/-- `return fd >= 0 ? (int) b->n : 0;` with `fd` a tracked descriptor. -/
def descriptorWitness : ScalarSite where
  earlierRouteAdmitted := false
  scalarResult := true
  aRead := true
  occurrences := [⟨false, true, true, .value, false⟩, ⟨true, false, false, .directDeref, true⟩]

theorem the_scalar_read_is_not_vacuous :
    after accessorWitness = .read [⟨true, false, false, .directDeref, false⟩] ∧
      before accessorWitness = .refuse ∧
      after skippedBoundWitness = .refuse ∧
      after descriptorWitness = .refuse ∧
      after { accessorWitness with scalarResult := false } = .refuse := by
  decide

/-- `return (XXH32_state_t *) XXH_malloc(sizeof(XXH32_state_t));` and its
    integer-cast twin. -/
def ownedWitness : CastSite where
  targets := [true]
  objectPointerDecl := true
  admitDerived := false
  placeSpelled := false
  classified := .call .owned

theorem the_cast_call_is_not_vacuous :
    decideCast ownedWitness = .call .owned ∧
      decideCast { ownedWitness with targets := [false] } = .refuse ∧
      decideCast { ownedWitness with classified := .call .derived } = .refuse ∧
      decideCast { ownedWitness with classified := .refused "call to uncontracted function `f`" } =
        .lowered "call to uncontracted function `f`" := by
  decide

end QCore.ReturnExpressionFamilies

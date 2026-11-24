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

/-!
# The hand-off store into a member of the caller's object (card MEMBER-EFFECT-STORES, rule 38k)

The abstract rule implemented by `crates/qcore-c-gateway/src/member_handoff.rs`
(`decide`, `carry_premise`, `enforce_premise`) and
`legacy_member_handoff.rs` (the parser's facts and the flow).

Every condition is a formula of the C04.18.9 typed rendering over the MES
terms (`mesTerms`), stated first and checked by the premise-formula validator
in Rust; `formulaTable` is the Rust table, row for row. The admission is the
conjunction of the conditions; each named refusal is the formula
`refusalOf r`, the negation of one condition (`a_refusal_negates_its_condition`).

The stated premise, P-CG11-CALLER-SLOT:

  `mes_caller_slot: forall (#o: obj) { (#mes_holds(#mes_place,#o) => !#mes_owed(#o)) }`

Theorems:

* `discharged_only_by_the_handoff`: the body's obligation for the stored value
  is discharged at the store exactly when the hand-off is admitted;
* `a_used_local_keeps_the_refusal`, `a_released_local_keeps_the_refusal`,
  `a_second_store_keeps_the_refusal`;
* `the_old_value_is_custody_free_or_premised`: an admitted store's old value
  is unbound in the projection, and a `Safe` row holding it lists the premise;
  `a_row_lacking_the_premise_is_refused`;
* `no_summary_with_a_handoff`, `a_callers_decision_is_unchanged`;
* `decision_vectors_hold`: the Rust vectors.

## What this is not

A model of Clang or of the lexical readers: the facts are their outputs
(trust boundary 28). The premise is stated, not proved.
-/

namespace QCore.MemberHandoff

inductive ValueKind where
  | ownedResult
  | localAcquisition
  | reallocated
  | other
  deriving DecidableEq, Repr

inductive Reason where
  | reallocSelf
  | valueShape
  | localAliasOfFormal
  | placeShape
  | reboundFormal
  | loop
  | placeUsed
  | localUsed
  | borrowedUseAfter
  | owner
  | ownedOldValue
  | staleValue
  | premiseMissing
  deriving DecidableEq, Repr

/-- `member_handoff::Facts`, field for field: `true` is each condition's
admitting answer. -/
structure Facts where
  value : ValueKind
  nonnullClause : Bool
  pointerCasts : Bool
  formalRoot : Bool
  placeShape : Bool
  formalKept : Bool
  notInLoop : Bool
  placeOnlyAtStore : Bool
  localKept : Bool
  nothingReleasedAfter : Bool
  untrackedOwner : Bool
  oldValueUnbound : Bool
  valueLiveOwed : Bool
  deriving DecidableEq, Repr

/-- The decision (`member_handoff::decide`): `none` admits, `some r` names the
first failing condition. -/
def decide (f : Facts) : Option Reason :=
  match f.value with
  | .reallocated => if f.nonnullClause then some .valueShape else some .reallocSelf
  | .other => some .valueShape
  | .ownedResult | .localAcquisition =>
    if !f.pointerCasts then some .valueShape
    else if !f.formalRoot then some .localAliasOfFormal
    else if !f.placeShape then some .placeShape
    else if !f.formalKept then some .reboundFormal
    else if !f.notInLoop then some .loop
    else if !f.placeOnlyAtStore then some .placeUsed
    else if !f.localKept then some .localUsed
    else if !f.nothingReleasedAfter then some .borrowedUseAfter
    else if !f.untrackedOwner then some .owner
    else if !f.oldValueUnbound then some .ownedOldValue
    else if !f.valueLiveOwed then some .staleValue
    else none

/-- Every condition holds. -/
def admits (f : Facts) : Bool :=
  (f.value == .ownedResult || f.value == .localAcquisition) && f.pointerCasts && f.formalRoot &&
    f.placeShape && f.formalKept && f.notInLoop && f.placeOnlyAtStore && f.localKept &&
    f.nothingReleasedAfter && f.untrackedOwner && f.oldValueUnbound && f.valueLiveOwed

theorem admitted_exactly_when (f : Facts) : decide f = none ↔ admits f = true := by
  rcases f with ⟨v, n, c, r, p, k, l, o, lk, a, u, ov, lv⟩
  cases v <;> cases n <;> simp [decide, admits] <;>
    cases c <;> (try simp) <;> cases r <;> (try simp) <;> cases p <;> (try simp) <;>
    cases k <;> (try simp) <;> cases l <;> (try simp) <;> cases o <;> (try simp) <;>
    cases lk <;> (try simp) <;> cases a <;> (try simp) <;> cases u <;> (try simp) <;>
    cases ov <;> (try simp) <;> cases lv <;> (try simp)

/-- The condition each named refusal negates. -/
def conditionOf : Reason → Facts → Bool
  | .reallocSelf, f => !(f.value == .reallocated && !f.nonnullClause)
  | .valueShape, f => (f.value == .ownedResult || f.value == .localAcquisition) && f.pointerCasts
  | .localAliasOfFormal, f => f.formalRoot
  | .placeShape, f => f.placeShape
  | .reboundFormal, f => f.formalKept
  | .loop, f => f.notInLoop
  | .placeUsed, f => f.placeOnlyAtStore
  | .localUsed, f => f.localKept
  | .borrowedUseAfter, f => f.nothingReleasedAfter
  | .owner, f => f.untrackedOwner
  | .ownedOldValue, f => f.oldValueUnbound
  | .staleValue, f => f.valueLiveOwed
  | .premiseMissing, _ => true

/-- A named refusal is the negation of its condition: the refusal's formula
holds exactly where the decision names it. -/
theorem a_refusal_negates_its_condition (f : Facts) (r : Reason) (h : decide f = some r) :
    conditionOf r f = false := by
  rcases f with ⟨v, n, c, rt, p, k, l, o, lk, a, u, ov, lv⟩
  cases v <;> cases n <;> cases c <;> simp [decide] at h <;> (try (subst h; rfl)) <;>
    cases rt <;> simp at h <;> (try (subst h; rfl)) <;>
    cases p <;> simp at h <;> (try (subst h; rfl)) <;>
    cases k <;> simp at h <;> (try (subst h; rfl)) <;>
    cases l <;> simp at h <;> (try (subst h; rfl)) <;>
    cases o <;> simp at h <;> (try (subst h; rfl)) <;>
    cases lk <;> simp at h <;> (try (subst h; rfl)) <;>
    cases a <;> simp at h <;> (try (subst h; rfl)) <;>
    cases u <;> simp at h <;> (try (subst h; rfl)) <;>
    cases ov <;> simp at h <;> (try (subst h; rfl)) <;>
    cases lv <;> simp at h <;> (try (subst h; rfl))

/-! ## The discharge -/

/-- The body's obligation for the stored value after the store's node: the
hand-off consumes it when admitted; otherwise the node is the gate's refusal
and the obligation stands. -/
def obligationAfter (f : Facts) (owed : Bool) : Bool :=
  match decide f with
  | none => false
  | some _ => owed

theorem discharged_only_by_the_handoff (f : Facts) :
    obligationAfter f true = false ↔ decide f = none := by
  unfold obligationAfter
  cases decide f <;> simp

inductive Status where
  | safe
  | unknown
  deriving DecidableEq, Repr

/-- A refused hand-off is the gate's refusal: the node is `Unknown`. -/
def nodeStatus (f : Facts) : Status :=
  if decide f = none then .safe else .unknown

theorem a_refusal_is_the_gates (f : Facts) (r : Reason) (h : decide f = some r) :
    nodeStatus f = .unknown := by
  simp [nodeStatus, h]

theorem a_used_local_keeps_the_refusal (f : Facts) (h : f.localKept = false) :
    decide f ≠ none := by
  intro hd
  have := (admitted_exactly_when f).mp hd
  simp [admits, h] at this

theorem a_released_local_keeps_the_refusal (f : Facts) (h : f.valueLiveOwed = false) :
    decide f ≠ none := by
  intro hd
  have := (admitted_exactly_when f).mp hd
  simp [admits, h] at this

theorem a_second_store_keeps_the_refusal (f : Facts) (h : f.placeOnlyAtStore = false) :
    decide f ≠ none := by
  intro hd
  have := (admitted_exactly_when f).mp hd
  simp [admits, h] at this

theorem a_realloc_self_assignment_is_refused (f : Facts) (h : f.value = .reallocated) :
    decide f ≠ none := by
  simp [decide, h]
  split <;> simp

theorem a_loop_is_refused (f : Facts) (h : f.notInLoop = false) : decide f ≠ none := by
  intro hd
  have := (admitted_exactly_when f).mp hd
  simp [admits, h] at this

theorem a_rebound_formal_is_refused (f : Facts) (h : f.formalKept = false) :
    decide f ≠ none := by
  intro hd
  have := (admitted_exactly_when f).mp hd
  simp [admits, h] at this

theorem a_local_alias_of_the_formal_is_refused (f : Facts) (h : f.formalRoot = false) :
    decide f ≠ none := by
  intro hd
  have := (admitted_exactly_when f).mp hd
  simp [admits, h] at this

theorem a_borrowed_use_after_is_refused (f : Facts) (h : f.nothingReleasedAfter = false) :
    decide f ≠ none := by
  intro hd
  have := (admitted_exactly_when f).mp hd
  simp [admits, h] at this

/-! ## The premise and the row -/

/-- The stated premise as a row lists it (`member_handoff::premise_statement`). -/
def premise : String :=
  "mes_caller_slot: forall (#o: obj) { (#mes_holds(#mes_place,#o) => !#mes_owed(#o)) }"

/-- One published row: its hand-off stores, its assumptions and whether any
other refusal or open obligation keeps it out of `Safe`. -/
structure Row where
  handoffs : List Facts
  assumptions : List String
  otherRefusal : Bool

/-- `carry_premise`: a row holding a hand-off lists the premise. -/
def carry (r : Row) : Row :=
  if r.handoffs.isEmpty then r else { r with assumptions := premise :: r.assumptions }

/-- A row's status: `Safe` only with no other refusal, every hand-off
admitted, and the premise listed when it holds one (`enforce_premise`). -/
def status (r : Row) : Status :=
  if r.otherRefusal || r.handoffs.any (fun f => decide f != none) ||
      (!r.handoffs.isEmpty && !r.assumptions.contains premise) then .unknown
  else .safe

theorem a_row_lacking_the_premise_is_refused (r : Row) (h : r.handoffs ≠ [])
    (hp : premise ∉ r.assumptions) : status r = .unknown := by
  have he : r.handoffs.isEmpty = false := by
    cases hr : r.handoffs with
    | nil => exact absurd hr h
    | cons _ _ => rfl
  have hc : r.assumptions.contains premise = false := by
    simpa using hp
  unfold status
  rw [he, hc]
  simp

theorem a_carried_row_lists_the_premise (r : Row) (h : r.handoffs ≠ []) :
    premise ∈ (carry r).assumptions := by
  have he : r.handoffs.isEmpty = false := by
    cases hr : r.handoffs with
    | nil => exact absurd hr h
    | cons _ _ => rfl
  simp [carry, he]

/-- The old value of an admitted store is custody-free in the projection
(nothing rooted at the formal is bound), and whatever the projection does not
see is covered by the stated premise, which a `Safe` row lists. -/
theorem the_old_value_is_custody_free_or_premised (r : Row) (f : Facts)
    (hf : f ∈ r.handoffs) (hs : status r = .safe) :
    f.oldValueUnbound = true ∧ premise ∈ r.assumptions := by
  have hne : r.handoffs ≠ [] := by
    intro e; rw [e] at hf; cases hf
  have hp : premise ∈ r.assumptions := by
    apply Classical.byContradiction
    intro hn
    have := a_row_lacking_the_premise_is_refused r hne hn
    rw [hs] at this; cases this
  have hd : decide f = none := by
    apply Classical.byContradiction
    intro hn
    have hany : r.handoffs.any (fun f => decide f != none) = true := by
      rw [List.any_eq_true]
      exact ⟨f, hf, by simpa using hn⟩
    unfold status at hs
    rw [hany] at hs
    simp at hs
  have := (admitted_exactly_when f).mp hd
  refine ⟨?_, hp⟩
  cases hov : f.oldValueUnbound
  · simp [admits, hov] at this
  · rfl

/-! ## No summary -/

/-- Rule 10v's gate: a body holding a pointer store, a hand-off among them,
publishes no summary. -/
def publishes (r : Row) : Bool := r.handoffs.isEmpty

inductive CallerDecision where
  | lowered
  | refused
  deriving DecidableEq, Repr

/-- A caller's call: lowered under a published summary, refused as
uncontracted otherwise. -/
def callerDecision (published : Bool) : CallerDecision :=
  if published then .lowered else .refused

theorem no_summary_with_a_handoff (r : Row) (h : r.handoffs ≠ []) : publishes r = false := by
  cases hr : r.handoffs with
  | nil => exact absurd hr h
  | cons _ _ => simp [publishes, hr]

/-- Before the card the body's store was the gate's refusal, so the body
published nothing; after it the body publishes nothing either. A caller's
decision is the same refusal. -/
theorem a_callers_decision_is_unchanged (r : Row) (h : r.handoffs ≠ []) :
    callerDecision (publishes r) = callerDecision false ∧ callerDecision false = .refused := by
  rw [no_summary_with_a_handoff r h]
  exact ⟨rfl, rfl⟩

/-! ## The Rust tables -/

/-- Every fact admitting, for `v` (`member_handoff::admitting`). -/
def admitting (v : ValueKind) : Facts :=
  ⟨v, false, true, true, true, true, true, true, true, true, true, true, true⟩

/-- `member_handoff::vector_facts`. -/
def vectorFacts (differs : String) : Facts :=
  let f := admitting .ownedResult
  if differs == "" then f
  else if differs == "value=local" then { f with value := .localAcquisition }
  else if differs == "value=reallocated" then { f with value := .reallocated }
  else if differs == "value=other" then { f with value := .other }
  else if differs == "pointer_casts" then { f with pointerCasts := false }
  else if differs == "formal_root" then { f with formalRoot := false }
  else if differs == "place_shape" then { f with placeShape := false }
  else if differs == "formal_kept" then { f with formalKept := false }
  else if differs == "not_in_loop" then { f with notInLoop := false }
  else if differs == "place_only_at_store" then { f with placeOnlyAtStore := false }
  else if differs == "local_kept" then { f with localKept := false }
  else if differs == "nothing_released_after" then { f with nothingReleasedAfter := false }
  else if differs == "untracked_owner" then { f with untrackedOwner := false }
  else if differs == "old_value_unbound" then { f with oldValueUnbound := false }
  else if differs == "value_live_owed" then { f with valueLiveOwed := false }
  else f

/-- `Reason::code`, and `admit` for an admission. -/
def code : Option Reason → String
  | none => "admit"
  | some .reallocSelf => "reallocSelf"
  | some .valueShape => "valueShape"
  | some .localAliasOfFormal => "localAliasOfFormal"
  | some .placeShape => "placeShape"
  | some .reboundFormal => "reboundFormal"
  | some .loop => "loop"
  | some .placeUsed => "placeUsed"
  | some .localUsed => "localUsed"
  | some .borrowedUseAfter => "borrowedUseAfter"
  | some .owner => "owner"
  | some .ownedOldValue => "ownedOldValue"
  | some .staleValue => "staleValue"
  | some .premiseMissing => "premiseMissing"

/-- The Rust `DECISION_VECTORS`: a fixture spelling, the differing fact and
the decision. -/
def decisionVectors : List (String × String × String) := [
  ("p->buf = q_alloc()", "", "admit"),
  ("p->buf = l", "value=local", "admit"),
  ("p->in.buf = q_alloc()", "", "admit"),
  ("p->any = (void *) l", "value=local", "admit"),
  ("p->buf = q_realloc(p->buf, n)", "value=reallocated", "reallocSelf"),
  ("p->buf = q_borrow_get()", "value=other", "valueShape"),
  ("p->n = (long) l", "pointer_casts", "valueShape"),
  ("m = p; m->buf = q_alloc()", "formal_root", "localAliasOfFormal"),
  ("p->next->buf = q_alloc()", "place_shape", "placeShape"),
  ("p = r; p->buf = q_alloc()", "formal_kept", "reboundFormal"),
  ("while (c) p->buf = q_alloc()", "not_in_loop", "loop"),
  ("p->buf = q_alloc(); use(p->buf)", "place_only_at_store", "placeUsed"),
  ("p->buf = l; q_free(l)", "local_kept", "localUsed"),
  ("p->buf = q_alloc(); q_take(p)", "nothing_released_after", "borrowedUseAfter"),
  ("o->buf = q_alloc() (o tracked)", "untracked_owner", "owner"),
  ("p->buf = q_alloc() (old value bound)", "old_value_unbound", "ownedOldValue"),
  ("q_free(l); p->buf = l", "value_live_owed", "staleValue")
]

theorem decision_vectors_hold :
    decisionVectors.all (fun (_, d, e) => code (decide (vectorFacts d)) == e) = true := by
  decide

/-- The MES block of the typed rendering (`premise_formula::RENDERED_TERMS`). -/
def mesTerms : List String := ["mes_store", "mes_place", "mes_parent", "mes_in", "mes_formal_root", "mes_formal_written", "mes_occurs", "mes_null_test", "mes_after", "mes_in_loop", "mes_local_mention", "mes_local_copy", "mes_formal_passed", "mes_indirect", "mes_call", "mes_borrowed_only", "mes_stored", "mes_owed", "mes_live", "mes_holds", "mes_bound_old", "mes_owned_result", "mes_local_acquisition", "mes_reallocates", "mes_nonnull_clause", "mes_pointer_casts", "mes_untracked_owner", "mes_published", "mes_premise_carried"]

/-- The Rust formula table (`member_handoff::formulas`): the premise, the
conditions, then the named refusals, each `(id, formula)`. -/
def formulaTable : List (String × String) := [
  ("mes_caller_slot", "forall (#o: obj) { (#mes_holds(#mes_place,#o) => !#mes_owed(#o)) }"),
  ("mes_place_shape", "(#mes_formal_root() && #mes_in(#mes_place,#mes_parent))"),
  ("mes_formal_kept", "forall (#s: mes_site) { !#mes_formal_written(#s) }"),
  ("mes_value_shape", "((#mes_owned_result() || #mes_local_acquisition()) && #mes_pointer_casts())"),
  ("mes_h1_place_only_at_store", "forall (#s: mes_site) { (#mes_occurs(#s,#mes_place) => (#s == #mes_store || #mes_null_test(#s))) }"),
  ("mes_h2_local_kept", "forall (#s: mes_site) { (!#mes_local_copy(#s) && (#mes_local_mention(#s) => !#mes_after(#s,#mes_store))) }"),
  ("mes_h3_no_stale_value", "forall (#o: obj) { (#mes_stored(#o) => (#mes_owed(#o) && #mes_live(#o))) }"),
  ("mes_not_in_loop", "!#mes_in_loop(#mes_store)"),
  ("mes_nothing_released_after", "forall (#s: mes_site) { (#mes_after(#s,#mes_store) => ((!#mes_formal_passed(#s) && !#mes_indirect(#s)) && (#mes_call(#s) => #mes_borrowed_only(#s)))) }"),
  ("mes_owner_untracked", "(#mes_untracked_owner() && !#mes_bound_old())"),
  ("mes_summary_withheld", "!#mes_published()"),
  ("mes_premise_stated", "#mes_premise_carried()"),
  ("mes_refuse_realloc_self", "(#mes_reallocates() && !#mes_nonnull_clause())"),
  ("mes_refuse_value_shape", "!((#mes_owned_result() || #mes_local_acquisition()) && #mes_pointer_casts())"),
  ("mes_refuse_local_alias_of_formal", "!#mes_formal_root()"),
  ("mes_refuse_place_shape", "!#mes_in(#mes_place,#mes_parent)"),
  ("mes_refuse_rebound_formal", "exists (#s: mes_site) { #mes_formal_written(#s) }"),
  ("mes_refuse_loop", "#mes_in_loop(#mes_store)"),
  ("mes_refuse_place_used", "exists (#s: mes_site) { (#mes_occurs(#s,#mes_place) && !(#s == #mes_store || #mes_null_test(#s))) }"),
  ("mes_refuse_local_used", "exists (#s: mes_site) { (#mes_local_copy(#s) || (#mes_local_mention(#s) && #mes_after(#s,#mes_store))) }"),
  ("mes_refuse_borrowed_use_after", "exists (#s: mes_site) { (#mes_after(#s,#mes_store) && ((#mes_formal_passed(#s) || #mes_indirect(#s)) || (#mes_call(#s) && !#mes_borrowed_only(#s)))) }"),
  ("mes_refuse_owner", "!#mes_untracked_owner()"),
  ("mes_refuse_owned_old_value", "#mes_bound_old()"),
  ("mes_refuse_stale_value", "exists (#o: obj) { (#mes_stored(#o) && !(#mes_owed(#o) && #mes_live(#o))) }"),
  ("mes_refuse_premise_missing", "!#mes_premise_carried()")
]

/-- The tables' sizes, and the premise first (the `mes_` prefix of every id and
term is checked by `tests/unit/member_handoff.rs`). -/
theorem the_tables_are_counted :
    formulaTable.length = 25 ∧ mesTerms.length = 29 ∧
      formulaTable.head? = some ("mes_caller_slot",
        "forall (#o: obj) { (#mes_holds(#mes_place,#o) => !#mes_owed(#o)) }") := by
  decide

end QCore.MemberHandoff

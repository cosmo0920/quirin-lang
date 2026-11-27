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

import QCore.ElementLinkPremises
import QCore.CarriedLinkObligation

/-!
# Rule 46's kill policy for scalar-only calls (card LINK-RECORD-KILL-POLICY-SCALAR-CALLS)

The refinement of rule 46's kill policy implemented by
`crates/qcore-c-gateway/src/legacy_link_member.rs` (`decided`, `reach`,
`nested_reach`, `before_nested`), shared by rule 91's carried records
(`legacy_carried_link.rs`), with the scalar positions recorded at
classification (`scalar_actuals`, a hook in `legacy_classify.rs`).

Before this card a contract call kept a link record only when it had no
nested call and every actual was a place (`decidedBefore`); every other
call forgot every record. `flb_errno()`, which is
`flb_errno_print((*__errno_location ()), "…", LINE)`, has a nested call and
no place at all, so the record a body carried to `flb_task_retry_destroy`
was gone before it.

An actual now has one of three kinds: a **place** (the call may write the
objects its root binding denotes), a **scalar** (a literal, a predefined
identifier, or an admitted dereference of a neutral nested call's scalar
result: it denotes no object), or **undecided** (anything else, which may
carry a pointer to any object). A call is decided when no actual is
undecided and every nested call's actuals are places (`decided`); it then
writes only the objects its places, its nested calls' places and its
written place may denote (`reached`). The nested calls run first, so before
the call's own effect the records of what their places may denote are
forgotten (`nestedStmt`).

Proved here: under the frame premise (a contracted call writes only the
objects its actuals may denote; a scalar denotes none, an undecided actual
may denote any), the refined statement over-approximates the call's writes
(`the_refined_policy_is_sound`, `the_nested_kill_is_sound`), so rule 46's
facts and rule 91's ready records it keeps still hold
(`the_refined_policy_keeps_every_fact`, `the_refined_policy_keeps_every_ready_record`).
Every call the old policy decided is decided with the same reach
(`every_old_reach_is_kept`); a call with an undecided actual, or a nested
call with a non-place actual, is still read as writing everything
(`a_pointer_carrying_call_keeps_its_kill`); and a kill is removed only when
the scalar-only condition holds (`a_kill_is_removed_only_for_scalar_calls`).
`vectors_hold` equals the Rust `KILL_VECTORS`.

## What this is not

A proof that the reader classifies actuals correctly. That a literal,
a predefined identifier and an admitted N4-D/N10 dereference denote no
object (premises T6, N10), that a place's root binding over-approximates
what it denotes, and that a contracted callee writes nothing it is not
handed (the frame premise the old policy's contract-call arm already
relied on: it kept the records of objects no actual denotes) are review
premises, trust boundary 28's, pinned by
`crates/qcore-c-gateway/tests/link_member_premises.rs`.
-/

namespace QCore.ScalarCallKillPolicy

open QCore.ElementLinkPremises

/-- One actual as the kill policy reads it. -/
inductive Kind where
  | place
  | scalar
  | undecided
  deriving DecidableEq, Repr

/-- Rule 46's policy before this card (`decided_before`). -/
def decidedBefore (actuals : List Kind) (nested : List (List Kind)) : Bool :=
  nested.isEmpty && actuals.all (fun k => k == .place)

/-- The refined policy (`decided`). -/
def decided (actuals : List Kind) (nested : List (List Kind)) : Bool :=
  actuals.all (fun k => k != .undecided) &&
    nested.all (fun inner => inner.all (fun k => k == .place))

/-- An actual and the objects its root binding denotes (read only for a
place). -/
structure Actual where
  kind : Kind
  sites : List Nat
  deriving DecidableEq, Repr

/-- A contract call: its actuals, its nested calls' actuals, and the objects
its written place may denote. -/
structure Call where
  actuals : List Actual
  nested : List (List Actual)
  written : List Nat
  deriving Repr

def kinds (as : List Actual) : List Kind := as.map (·.kind)

/-- The objects the places among `as` may denote. -/
def placeSites (as : List Actual) : List Nat :=
  (as.filter (fun a => a.kind == .place)).flatMap (·.sites)

def reachedBefore (c : Call) : List Nat := placeSites c.actuals ++ c.written

def reached (c : Call) : List Nat :=
  placeSites c.actuals ++ c.nested.flatMap placeSites ++ c.written

/-- The old reading of a call. -/
def before (c : Call) : Stmt :=
  if decidedBefore (kinds c.actuals) (c.nested.map kinds) then .call (reachedBefore c) else .other

/-- The refined reading of a call (`reach`). -/
def after (c : Call) : Stmt :=
  if decided (kinds c.actuals) (c.nested.map kinds) then .call (reached c) else .other

/-- The reading of a call's nested calls before its own effect
(`nested_reach`). -/
def nestedStmt (c : Call) : Stmt :=
  if c.nested.isEmpty then .inert
  else if c.nested.all (fun inner => (kinds inner).all (fun k => k == .place))
  then .call (c.nested.flatMap placeSites) else .other

/-- What an actual may denote: a place its sites, a scalar nothing, an
undecided actual anything. -/
def denotes : Actual → Nat → Prop
  | ⟨.place, s⟩, x => x ∈ s
  | ⟨.scalar, _⟩, _ => False
  | ⟨.undecided, _⟩, _ => True

/-- The frame premise of a contracted call: it writes only objects one of
its actuals (its nested calls' included) or its written place may denote. -/
def frame (c : Call) (h h' : Heap) : Prop :=
  ∀ x, (∀ a ∈ c.actuals ++ c.nested.flatten, ¬ denotes a x) → x ∉ c.written →
    ∀ m, h' x m = h x m

/-- The frame premise of the nested calls alone. -/
def nestedFrame (c : Call) (h h' : Heap) : Prop :=
  ∀ x, (∀ a ∈ c.nested.flatten, ¬ denotes a x) → ∀ m, h' x m = h x m

-- Helpers.

theorem mem_placeSites {as : List Actual} {a : Actual} {x : Nat}
    (ha : a ∈ as) (hk : a.kind = .place) (hx : x ∈ a.sites) : x ∈ placeSites as := by
  simp only [placeSites, List.mem_flatMap, List.mem_filter]
  exact ⟨a, ⟨ha, by simp [hk]⟩, hx⟩

theorem kinds_all_iff {as : List Actual} {p : Kind → Bool} :
    (kinds as).all p = true ↔ ∀ a ∈ as, p a.kind = true := by
  simp [kinds, List.all_eq_true]

/-- An actual of a place kind denotes only its sites; a non-undecided one
not in `placeSites` denotes nothing there. -/
theorem not_denotes {as : List Actual} {a : Actual} {x : Nat}
    (ha : a ∈ as) (hnu : a.kind ≠ .undecided) (hx : x ∉ placeSites as) : ¬ denotes a x := by
  obtain ⟨k, s⟩ := a
  cases k with
  | place => intro hd; exact hx (mem_placeSites ha rfl hd)
  | scalar => simp [denotes]
  | undecided => exact absurd rfl hnu

-- Soundness.

theorem the_refined_policy_is_sound (c : Call) (h h' : Heap) (w : frame c h h') :
    writesOnly (after c) h h' := by
  unfold after
  split
  · rename_i hd
    simp only [decided, Bool.and_eq_true] at hd
    obtain ⟨hout, hnest⟩ := hd
    rw [kinds_all_iff] at hout
    simp only [List.all_map, List.all_eq_true, Function.comp_apply] at hnest
    intro x hx m
    simp only [reached, List.mem_append, not_or] at hx
    obtain ⟨⟨hxo, hxn⟩, hxw⟩ := hx
    apply w x _ hxw m
    intro a ha
    simp only [List.mem_append, List.mem_flatten] at ha
    rcases ha with ha | ⟨inner, hi, ha⟩
    · exact not_denotes ha (by simpa using hout a ha) hxo
    · have hk : a.kind = .place := by
        have := hnest inner hi a.kind (List.mem_map_of_mem ha)
        simpa using this
      have hxi : x ∉ placeSites inner := by
        intro hm; exact hxn (List.mem_flatMap.mpr ⟨inner, hi, hm⟩)
      exact not_denotes ha (by simp [hk]) hxi
  · exact trivial

theorem the_nested_kill_is_sound (c : Call) (h h' : Heap) (w : nestedFrame c h h') :
    writesOnly (nestedStmt c) h h' := by
  unfold nestedStmt
  split
  · rename_i he
    simp only [List.isEmpty_iff] at he
    simp only [writesOnly]
    funext x m
    apply w x _ m
    simp [he]
  · split
    · rename_i _ hall
      simp only [List.all_eq_true] at hall
      intro x hx m
      apply w x _ m
      intro a ha
      simp only [List.mem_flatten] at ha
      obtain ⟨inner, hi, ha⟩ := ha
      have hk : a.kind = .place := by
        have := hall inner hi a.kind (List.mem_map_of_mem ha)
        simpa using this
      have hxi : x ∉ placeSites inner := by
        intro hm; exact hx (List.mem_flatMap.mpr ⟨inner, hi, hm⟩)
      exact not_denotes ha (by simp [hk]) hxi
    · exact trivial

theorem the_refined_policy_keeps_every_fact (c : Call) (facts : List (Nat × String))
    (h h' : Heap) (hold : holds facts h) (w : frame c h h') :
    holds (retain (after c) facts) h' :=
  retaining_preserves_every_fact (after c) facts h h' hold (the_refined_policy_is_sound c h h' w)

theorem the_refined_policy_keeps_every_ready_record (c : Call)
    (rs : QCore.CarriedLinkObligation.Records) (h h' : Heap)
    (hold : QCore.CarriedLinkObligation.readyHolds rs h) (w : frame c h h') :
    QCore.CarriedLinkObligation.readyHolds (QCore.CarriedLinkObligation.retain (after c) rs) h' :=
  QCore.CarriedLinkObligation.retaining_keeps_every_ready_record (after c) rs h h' hold
    (the_refined_policy_is_sound c h h' w)

-- The refinement against the old policy.

/-- Every call the old policy decided is decided, with the same reach. -/
theorem every_old_reach_is_kept (c : Call) (r : List Nat) (hb : before c = .call r) :
    after c = .call r := by
  unfold before at hb
  split at hb
  · rename_i hd
    simp only [decidedBefore, Bool.and_eq_true, List.isEmpty_iff] at hd
    obtain ⟨hn, ha⟩ := hd
    have hn' : c.nested = [] := by simpa [List.map_eq_nil_iff] using hn
    have hdec : decided (kinds c.actuals) (c.nested.map kinds) = true := by
      simp only [decided, hn', List.map_nil, List.all_nil, Bool.and_true]
      rw [kinds_all_iff] at ha ⊢
      intro a hm
      have := ha a hm
      simp only [beq_iff_eq] at this
      simp [this]
    simp only [Stmt.call.injEq] at hb
    unfold after
    simp only [hdec, ↓reduceIte]
    simp only [reached, hn', List.flatMap_nil, List.append_nil]
    simp only [reachedBefore] at hb
    exact congrArg Stmt.call hb
  · exact absurd hb (by simp)

/-- A call with an undecided actual, or a nested call with an actual that
is not a place, is read as writing everything, as before. -/
theorem a_pointer_carrying_call_keeps_its_kill (c : Call)
    (h : (∃ a ∈ c.actuals, a.kind = .undecided) ∨
      (∃ inner ∈ c.nested, ∃ a ∈ inner, a.kind ≠ .place)) :
    after c = .other ∧ before c = .other := by
  have hnd : decided (kinds c.actuals) (c.nested.map kinds) = false := by
    rcases h with ⟨a, ha, hk⟩ | ⟨inner, hi, a, ha, hk⟩
    · simp only [decided, Bool.and_eq_false_iff]
      left
      rw [Bool.eq_false_iff]
      intro hall
      have := (kinds_all_iff.mp hall) a ha
      simp [hk] at this
    · simp only [decided, Bool.and_eq_false_iff]
      right
      rw [Bool.eq_false_iff]
      intro hall
      simp only [List.all_map, List.all_eq_true, Function.comp_apply] at hall
      have := hall inner hi a.kind (List.mem_map_of_mem ha)
      simp only [beq_iff_eq] at this
      exact hk this
  constructor
  · simp [after, hnd]
  · cases hb : before c with
    | other => rfl
    | inert => simp [before] at hb; split at hb <;> simp at hb
    | call r =>
      have := every_old_reach_is_kept c r hb
      simp [after, hnd] at this

/-- A kill is removed only when the scalar-only condition holds: no actual
is undecided, every nested call's actuals are places, and the call has a
nested call or a scalar actual (otherwise both readings agree). -/
theorem a_kill_is_removed_only_for_scalar_calls (c : Call) (hne : after c ≠ before c) :
    (∀ a ∈ c.actuals, a.kind ≠ .undecided) ∧
      (∀ inner ∈ c.nested, ∀ a ∈ inner, a.kind = .place) ∧
      (c.nested ≠ [] ∨ ∃ a ∈ c.actuals, a.kind = .scalar) := by
  have hdec : decided (kinds c.actuals) (c.nested.map kinds) = true := by
    cases hd : decided (kinds c.actuals) (c.nested.map kinds) with
    | true => rfl
    | false =>
      exfalso
      apply hne
      cases hb : before c with
      | other => simp [after, hd]
      | inert => simp [before] at hb; split at hb <;> simp at hb
      | call r =>
        have := every_old_reach_is_kept c r hb
        simp [after, hd] at this
  simp only [decided, Bool.and_eq_true] at hdec
  obtain ⟨hout, hnest⟩ := hdec
  rw [kinds_all_iff] at hout
  simp only [List.all_map, List.all_eq_true, Function.comp_apply] at hnest
  refine ⟨fun a ha => by simpa using hout a ha, fun inner hi a ha => ?_, ?_⟩
  · have := hnest inner hi a.kind (List.mem_map_of_mem ha)
    simpa using this
  · by_cases hn : c.nested = []
    case neg => exact Or.inl hn
    by_cases hsome : ∃ a ∈ c.actuals, a.kind = .scalar
    case pos => exact Or.inr hsome
    have hs : ∀ a ∈ c.actuals, a.kind ≠ .scalar := fun a ha hk => hsome ⟨a, ha, hk⟩
    exfalso
    apply hne
    have hb : decidedBefore (kinds c.actuals) (c.nested.map kinds) = true := by
      simp only [decidedBefore, hn, List.map_nil, List.isEmpty_nil, Bool.true_and]
      rw [kinds_all_iff]
      intro a ha
      have hu : a.kind ≠ .undecided := by simpa using hout a ha
      have hsc : a.kind ≠ .scalar := hs a ha
      cases hk : a.kind with
      | place => rfl
      | scalar => exact absurd hk hsc
      | undecided => exact absurd hk hu
    have hbefore : before c = .call (reachedBefore c) := by simp [before, hb]
    rw [every_old_reach_is_kept c (reachedBefore c) hbefore, hbefore]

-- The measured shape.

/-- `flb_errno_print((*__errno_location ()), "…", LINE)`: three scalar
actuals and one nested call with no actual. -/
theorem the_errno_report_is_decided :
    decided [.scalar, .scalar, .scalar] [[]] = true ∧
      decidedBefore [.scalar, .scalar, .scalar] [[]] = false := by
  decide

-- The Rust vectors (`KILL_VECTORS`, checked by the unit tests of
-- `crates/qcore-c-gateway/src/legacy_link_member.rs`).

def killVectors : List (List Kind × List (List Kind) × Bool × Bool) :=
  [ ([.place, .place], [], true, true),
    ([], [], true, true),
    ([.scalar, .scalar, .scalar], [[]], false, true),
    ([.place, .scalar], [], false, true),
    ([.scalar], [[.place]], false, true),
    ([.undecided, .scalar], [], false, false),
    ([.place, .undecided], [], false, false),
    ([.scalar], [[.undecided]], false, false),
    ([.place], [[.place], [.undecided]], false, false) ]

theorem vectors_hold :
    killVectors.all (fun (a, n, b, r) => decidedBefore a n == b && decided a n == r) = true := by
  decide

end QCore.ScalarCallKillPolicy

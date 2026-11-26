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

import QCore.CallbackTableMemberPathStates

/-!
# `gone` is read only under an established invocation

Card **MEMBER-PATH-INVOCATION-PREMISE** (gateway rule 30 amended, rule 13v
amended; contract specification C04.19.6). Every callback clause calls its
callback *zero or more* times. Rule 30's table read a `#released` or
`#moved` candidate state under some or no arm as `gone`, which is sound only
when a candidate runs: `QCore.CallbackTableMemberPathStates.the_gone_reading_assumes_an_invocation`
named the premise (`run [] false = 0`). This module discharges it.

* **The reading** (`reading`, Rust `legacy_member_path_route::apply` with
  `member_path_invocation::{established, unestablished}`): where the
  invocation is established the candidates' states are read as stated
  (`an_established_reading_is_the_stated_one`); otherwise each state `V`
  reads `{V, #kept}` (`weaken`), the invocation having run or not.
* **Sound with no invocation.** Unestablished, a `gone` reading needs an arm
  that releases the path on every arm (`an_unestablished_gone_needs_every_arm`)
  and then leaves the caller's custody exactly once for *every* sequence of
  invocations, the empty one included
  (`an_unestablished_gone_leaves_exactly_once`); a `stays` reading never
  leaves (`an_unestablished_stays_never_leaves`). A `#released` or `#moved`
  candidate under some or no arm refuses
  (`a_taking_candidate_without_an_arm_refuses_unestablished`): with no
  invocation its object stays live, and the caller's state is undecided.
* **Only refuses more** (`the_weakening_only_refuses_more`): a decided
  unestablished reading is the stated one; the weakening never decides
  where the stated reading refused.
* **The body check** (`Stmt`, `Exec`, `must`, `established`; Rust
  `member_path_invocation::body_establishes`): a structured model of the
  invoking body (a call of the invoked formal, member or table entry, any
  other statement, sequence, a branch, a loop that runs zero or more times,
  a return), its executions counting the calls, and the must-call reading.
  `an_established_body_calls_on_every_exit`: on every execution that ends
  (falls off the end or returns) the call ran at least once. The shapes the
  pins read are not established (`may_not_call_vectors_hold`), and each
  has an execution with no call (`each_shape_has_an_execution_with_no_call`).
* **The premise discharged** (`the_premise_is_discharged`): under an
  established body, a `gone` reading over the calls that execution made
  leaves the caller's custody exactly once.

Abstract rule theorems, not an extraction of the Rust analyzer (whose graph
is rule 30's `member_path_reading::Graph`, read to a fixpoint with `and` as
the meet; the structured model is its reading of the same constructs).
-/

namespace QCore.MemberPathInvocationPremise

open QCore.MemberPathRelease
open QCore.CallbackTableMemberPathStates

/-- Rust `member_path_invocation::unestablished`: `V` reads `{V, #kept}`. -/
def weaken : Option PathState → Option PathState
  | none => none
  | some ⟨.kept, m⟩ => some ⟨.kept, m⟩
  | some ⟨.released, _⟩ => some ⟨.released, true⟩
  | some ⟨.moved, _⟩ => some ⟨.moved, true⟩
  | some ⟨.stale, _⟩ => some ⟨.stale, true⟩

/-- The composition at one linked path: as stated where the invocation is
established, weakened otherwise. -/
def reading (established : Bool) (cands : List (Option PathState)) (a : ArmJoin) :
    Option Outcome :=
  tableOutcome (if established then cands else cands.map weaken) a

theorem an_established_reading_is_the_stated_one (cands : List (Option PathState))
    (a : ArmJoin) : reading true cands a = tableOutcome cands a := rfl

/-- A weakened state allows every reading the state allows, and `#kept`. -/
theorem weaken_some (s : PathState) :
    ∃ s', weaken (some s) = some s' ∧ (∀ g ∈ members s, g ∈ members s') ∧
      Fate.kept ∈ members s' := by
  obtain ⟨f, m⟩ := s
  cases f <;> cases m <;> simp [weaken, members]

theorem allowed_weaken (cands : List (Option PathState)) (g : Fate)
    (h : allowed cands g) : allowed (cands.map weaken) g := by
  obtain ⟨s, hs, hm⟩ := h
  obtain ⟨s', he, hsub, _⟩ := weaken_some s
  exact ⟨s', List.mem_map.mpr ⟨some s, hs, he⟩, hsub g hm⟩

/-- Unestablished, `gone` needs an arm releasing the path on every arm. -/
theorem an_unestablished_gone_needs_every_arm (cands : List (Option PathState)) (a : ArmJoin)
    (h : reading false cands a = some .gone) : a = .every := by
  simp only [reading] at h
  have every := a_decided_join_is_every_candidate _ .gone (by simp) h
  cases cands with
  | nil => simp [tableOutcome, joinAll] at h
  | cons c rest =>
    have hc := every (compose (weaken c) a)
      (List.mem_map.mpr ⟨weaken c, List.mem_cons_self .., rfl⟩)
    cases c with
    | none => simp [weaken, compose] at hc
    | some s =>
      obtain ⟨s', he, _, hk⟩ := weaken_some s
      rw [he] at hc
      exact (gone_members s' a hc .kept hk).2 rfl

/-- **Sound with no invocation**: an unestablished `gone` leaves the caller's
custody exactly once for every sequence of the candidates' readings, the
empty one (no candidate ran) included, and every arm the join allows. -/
theorem an_unestablished_gone_leaves_exactly_once (cands : List (Option PathState))
    (a : ArmJoin) (h : reading false cands a = some .gone) (seq : List Fate)
    (hseq : ∀ g ∈ seq, allowed cands g) : ∀ b ∈ armReleases a, run seq b = 1 := by
  have ha := an_unestablished_gone_needs_every_arm cands a h
  simp only [reading] at h
  have every := a_decided_join_is_every_candidate _ .gone (by simp) h
  have hns : ∀ g ∈ seq, g ≠ .stale := by
    intro g hg
    obtain ⟨s, hs, hm⟩ := allowed_weaken cands g (hseq g hg)
    exact (gone_members s a (every _ (List.mem_map.mpr ⟨some s, hs, rfl⟩)) g hm).1
  subst ha
  intro b hb
  simp [armReleases] at hb
  subst hb
  rcases foldl_taking seq hns with e | ⟨e, _⟩ <;> simp [run, e]

/-- Unestablished, `stays` never leaves, for every sequence. -/
theorem an_unestablished_stays_never_leaves (cands : List (Option PathState)) (a : ArmJoin)
    (h : reading false cands a = some .stays) (seq : List Fate)
    (hseq : ∀ g ∈ seq, allowed cands g) : ∀ b ∈ armReleases a, run seq b = 0 :=
  a_stays_table_never_leaves _ a h seq (fun g hg => allowed_weaken cands g (hseq g hg))

/-- A taking candidate (`#released`, `#moved`) under some or no arm refuses
where the invocation is not established. -/
theorem a_taking_candidate_without_an_arm_refuses_unestablished :
    reading false [some ⟨.released, false⟩] .none = some (.refused .undecided) ∧
    reading false [some ⟨.moved, false⟩] .none = some (.refused .undecided) ∧
    reading false [some ⟨.released, false⟩] .some = some (.refused .undecided) ∧
    reading false [some ⟨.moved, false⟩] .some = some (.refused .undecided) ∧
    reading false [some ⟨.released, false⟩] .every = some .gone ∧
    reading true [some ⟨.released, false⟩] .none = some .gone := by
  decide

/-- Per candidate, the weakened composition is the stated one or a refusal. -/
theorem compose_weaken (c : Option PathState) (a : ArmJoin) :
    compose (weaken c) a = compose c a ∨ ∃ r, compose (weaken c) a = .refused r := by
  rcases c with _ | ⟨f, m⟩
  · exact Or.inl rfl
  · cases f <;> cases m <;> cases a <;> simp [weaken, compose, composeFate]

/-- **Only refuses more**: a decided unestablished reading is the stated
one. -/
theorem the_weakening_only_refuses_more (cands : List (Option PathState)) (a : ArmJoin)
    (o : Outcome) (ho : ∀ r, o ≠ .refused r) (h : reading false cands a = some o) :
    tableOutcome cands a = some o := by
  simp only [reading] at h
  have every := a_decided_join_is_every_candidate _ o ho h
  have same : ∀ c ∈ cands, compose c a = compose (weaken c) a := by
    intro c hc
    have hx := every (compose (weaken c) a)
      (List.mem_map.mpr ⟨weaken c, List.mem_map.mpr ⟨c, hc, rfl⟩, rfl⟩)
    rcases compose_weaken c a with e | ⟨r, e⟩
    · exact e.symm
    · exact absurd (hx.symm.trans e) (ho r)
  have hm : cands.map (compose · a) = (cands.map weaken).map (compose · a) := by
    rw [List.map_map]
    exact List.map_congr_left same
  simp only [tableOutcome] at h ⊢
  rw [hm]
  exact h

/-! ## The body check -/

/-- The invoking body, structured: a call of the invoked formal, member or
table entry; any other statement; sequence; a branch; a loop that runs its
body zero or more times; a return. -/
inductive Stmt where
  | call
  | skip
  | seq (s t : Stmt)
  | ite (s t : Stmt)
  | loop (s : Stmt)
  | ret
  deriving DecidableEq, Repr

/-- How an execution ends, with the number of calls it made. -/
inductive Res where
  | normal (n : Nat)
  | returned (n : Nat)
  deriving DecidableEq, Repr

def Res.calls : Res → Nat
  | .normal n => n
  | .returned n => n

/-- The executions: from `n` calls made, the statement ends with `r`. -/
inductive Exec : Stmt → Nat → Res → Prop where
  | call (n) : Exec .call n (.normal (n + 1))
  | skip (n) : Exec .skip n (.normal n)
  | ret (n) : Exec .ret n (.returned n)
  | seqNormal {s t n m r} : Exec s n (.normal m) → Exec t m r → Exec (.seq s t) n r
  | seqReturned {s t n m} : Exec s n (.returned m) → Exec (.seq s t) n (.returned m)
  | iteLeft {s t n r} : Exec s n r → Exec (.ite s t) n r
  | iteRight {s t n r} : Exec t n r → Exec (.ite s t) n r
  | loopDone {s} (n) : Exec (.loop s) n (.normal n)
  | loopAgain {s n m r} : Exec s n (.normal m) → Exec (.loop s) m r → Exec (.loop s) n r
  | loopReturned {s n m} : Exec s n (.returned m) → Exec (.loop s) n (.returned m)

/-- The must-call reading from "called on every path so far" `c`: (called
on every path that falls through, called on every path that returns). -/
def must : Stmt → Bool → Bool × Bool
  | .call, _ => (true, true)
  | .skip, c => (c, true)
  | .ret, c => (true, c)
  | .seq s t, c =>
    let p := must s c
    let q := must t p.1
    (q.1, p.2 && q.2)
  | .ite s t, c =>
    let p := must s c
    let q := must t c
    (p.1 && q.1, p.2 && q.2)
  | .loop s, c => (c, (must s c).2)

/-- Established: every path to every normal exit called. -/
def established (s : Stmt) : Bool :=
  let p := must s false
  p.1 && p.2

/-- Calls are never undone. -/
theorem exec_mono {s n r} (h : Exec s n r) : n ≤ r.calls := by
  induction h with
  | call n => simp [Res.calls]
  | skip n => simp [Res.calls]
  | ret n => simp [Res.calls]
  | seqNormal _ _ ih1 ih2 => simp [Res.calls] at ih1 ih2 ⊢; omega
  | seqReturned _ ih => simpa [Res.calls] using ih
  | iteLeft _ ih => exact ih
  | iteRight _ ih => exact ih
  | loopDone n => simp [Res.calls]
  | loopAgain _ _ ih1 ih2 => simp [Res.calls] at ih1 ih2 ⊢; omega
  | loopReturned _ ih => simpa [Res.calls] using ih

/-- **Soundness of the reading**: from an entry where "called" holds only if
a call was made, a fall-through or a return the reading marks called made
a call. -/
theorem must_sound {s n r} (h : Exec s n r) :
    ∀ c : Bool, (c = true → 0 < n) →
      (∀ m, r = .normal m → (must s c).1 = true → 0 < m) ∧
      (∀ m, r = .returned m → (must s c).2 = true → 0 < m) := by
  induction h with
  | call n => intro c _; simp [must]
  | skip n =>
    intro c hc
    refine ⟨fun m e h => ?_, fun m e _ => (by cases e)⟩
    cases e
    exact hc h
  | ret n =>
    intro c hc
    refine ⟨fun m e _ => (by cases e), fun m e h => ?_⟩
    cases e
    exact hc h
  | @seqNormal s t n m r h1 h2 ih1 ih2 =>
    intro c hc
    have hp := ih1 c hc
    have hm : (must s c).1 = true → 0 < m := hp.1 m rfl
    have hq := ih2 (must s c).1 hm
    refine ⟨fun k e hk => hq.1 k e hk, fun k e hk => ?_⟩
    simp only [must, Bool.and_eq_true] at hk
    exact hq.2 k e hk.2
  | @seqReturned s t n m h1 ih1 =>
    intro c hc
    refine ⟨fun k e _ => (by cases e), fun k e hk => ?_⟩
    cases e
    simp only [must, Bool.and_eq_true] at hk
    exact (ih1 c hc).2 _ rfl hk.1
  | iteLeft h1 ih1 =>
    intro c hc
    have hp := ih1 c hc
    refine ⟨fun k e hk => ?_, fun k e hk => ?_⟩
    · simp only [must, Bool.and_eq_true] at hk
      exact hp.1 k e hk.1
    · simp only [must, Bool.and_eq_true] at hk
      exact hp.2 k e hk.1
  | iteRight h1 ih1 =>
    intro c hc
    have hp := ih1 c hc
    refine ⟨fun k e hk => ?_, fun k e hk => ?_⟩
    · simp only [must, Bool.and_eq_true] at hk
      exact hp.1 k e hk.2
    · simp only [must, Bool.and_eq_true] at hk
      exact hp.2 k e hk.2
  | loopDone n =>
    intro c hc
    refine ⟨fun m e hk => ?_, fun m e _ => (by cases e)⟩
    cases e
    exact hc hk
  | @loopAgain s n m r h1 h2 ih1 ih2 =>
    intro c hc
    have hp := ih1 c hc
    have hm : c = true → 0 < m := fun h => by
      have := exec_mono h1
      simp [Res.calls] at this
      have := hc h
      omega
    have hq := ih2 c hm
    exact ⟨fun k e hk => hq.1 k e hk, fun k e hk => hq.2 k e hk⟩
  | @loopReturned s n m h1 ih1 =>
    intro c hc
    refine ⟨fun k e _ => (by cases e), fun k e hk => ?_⟩
    cases e
    exact (ih1 c hc).2 _ rfl hk

/-- **The body check is sound**: on every execution of an established body
that ends, the invocation ran at least once. -/
theorem an_established_body_calls_on_every_exit (s : Stmt) (hs : established s = true)
    {r : Res} (h : Exec s 0 r) : 0 < r.calls := by
  simp only [established, Bool.and_eq_true] at hs
  have hp := must_sound h false (by simp)
  cases r with
  | normal m => exact hp.1 m rfl hs.1
  | returned m => exact hp.2 m rfl hs.2

/-- The pins' shapes: a call on every path is established; an early return,
a call under a condition, a loop that may run zero times, and a call after
a branch that returns are not. -/
theorem may_not_call_vectors_hold :
    established .call = true ∧
    established (.seq .call (.ite .ret .skip)) = true ∧
    established (.seq (.ite .skip .skip) .call) = true ∧
    established (.seq (.ite .ret .skip) .call) = false ∧
    established (.ite .call .skip) = false ∧
    established (.loop .call) = false ∧
    established .skip = false := by
  decide

/-- Each shape that is not established has an execution with no call. -/
theorem each_shape_has_an_execution_with_no_call :
    Exec (.seq (.ite .ret .skip) .call) 0 (.returned 0) ∧
    Exec (.ite .call .skip) 0 (.normal 0) ∧
    Exec (.loop .call) 0 (.normal 0) :=
  ⟨.seqReturned (.iteLeft (.ret 0)), .iteRight (.skip 0), .loopDone 0⟩

/-- **The premise discharged**: under an established body, a `gone` reading
over the readings of the calls an execution made (at least one) leaves the
caller's custody exactly once, for every arm the join allows. -/
theorem the_premise_is_discharged (body : Stmt) (hb : established body = true) {r : Res}
    (hx : Exec body 0 r) (cands : List (Option PathState)) (a : ArmJoin)
    (h : reading true cands a = some .gone) (seq : List Fate) (hlen : seq.length = r.calls)
    (hseq : ∀ g ∈ seq, allowed cands g) : ∀ b ∈ armReleases a, run seq b = 1 := by
  have hpos := an_established_body_calls_on_every_exit body hb hx
  have hne : seq ≠ [] := by
    intro e
    rw [e] at hlen
    simp at hlen
    omega
  exact a_gone_table_leaves_exactly_once cands a h seq hne hseq

end QCore.MemberPathInvocationPremise

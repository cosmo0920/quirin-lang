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

import QCore.MemberPathRelease
import QCore.CallbackReleasedEffectOrder

/-!
# A table dispatch composes its candidates' member-path states

Card **CALLBACK-TABLE-MEMBER-PATH-STATES** (gateway rules 13v and 30
amended, with rule 74p; contract specification C04.19.6). At a table
dispatch (`resource_callback_table(T, A…)`) the gateway now composes the
candidates' member-path states (`legacy_callback_route::callback_operands`,
the `CallbackClause::Table` branch: the same `member_path_route::operands`
call as the synchronous branch), and `legacy_member_path_route::apply`
joins them (`qcore_schemas::member_path::join_all`, here
`QCore.MemberPathRelease.joinAll`). Before this card the states were
dropped.

A table differs from a synchronous invocation in one way: during the call
the callee may call *several different* candidates, in any order, any
number of times, where a synchronous invocation calls one candidate. This
module models the dispatch as any sequence of invocations, each one reading
of one candidate's state, and proves the synchronous composition sound for
it:

* `step`, `run`: one path's concrete effect over a sequence of invocations
  and then the arm. An invocation reading `#released` or `#moved` takes the
  object and clears the member, `#stale` takes it and leaves the member
  designating it, `#kept` does nothing; an invocation finding the member
  cleared does nothing; the arm releases what the member still designates.
  For one invocation `run` is `MemberPathRelease.count`
  (`run_one_is_count`).
* **The join is at least as strong as each candidate's state**
  (`the_join_is_at_least_each_candidate`): where it decides, every
  candidate's composition decides the same; otherwise it refuses
  (`a_decided_join_is_every_candidate`, and a refusal refuses it,
  `MemberPathRelease.a_refusal_refuses_the_join`).
* **No double release, no missed release, over any sequence**
  (`a_gone_table_leaves_exactly_once`): a path the join decides `gone`
  leaves the caller's custody exactly once for every non-empty sequence of
  the candidates' readings, whichever candidates run and in whatever order,
  and every arm the join allows. `a_stays_table_never_leaves`: a path the
  join decides `stays` never leaves, for every sequence, the empty one
  included. Candidates that disagree refuse (`disagreeing_candidates_refuse`).
* **The shared premise.** `gone` with no arm reads the invocation as run;
  for the empty sequence no object leaves (`the_gone_reading_assumes_an_invocation`).
  This is C04.19.6's table, read alike at every invocation form, the
  synchronous one included (user decision 2026-10-15); the table form adds
  no reading beyond it.
* **Through a released object** (rule 74p): the table's composition is the
  same `compose` of `QCore.CallbackReleasedEffectOrder` applied after the
  release, so its late order is the C order (`a_table_composition_after_a_release_is_the_c_order`).
* **Vectors** (`table_vectors_hold`): the fixture's rows.

Abstract rule theorems, not an extraction of the Rust analyzer.
-/

namespace QCore.CallbackTableMemberPathStates

open QCore.MemberPathRelease

/-- The composition at one linked path of a dispatch whose candidates state
`cands` there (`none`: unstated), under the arm join `a`: the synchronous
branch's, which the table branch now calls (Rust `join_all` over
`compose`). -/
def tableOutcome (cands : List (Option PathState)) (a : ArmJoin) : Option Outcome :=
  joinAll (cands.map (compose · a))

/-- One invocation's effect on (the member still designates the object, how
many times it left the caller's custody). -/
def step : Bool × Nat → Fate → Bool × Nat
  | (false, n), _ => (false, n)
  | (true, n), .released => (false, n + 1)
  | (true, n), .moved => (false, n + 1)
  | (true, n), .stale => (true, n + 1)
  | (true, n), .kept => (true, n)

/-- How many times the linked object leaves the caller's custody over the
sequence `seq` of invocations and then the arm (`armRel`). -/
def run (seq : List Fate) (armRel : Bool) : Nat :=
  let r := seq.foldl step (true, 0)
  if armRel && r.1 then r.2 + 1 else r.2

/-- A reading some candidate's state allows. -/
def allowed (cands : List (Option PathState)) (g : Fate) : Prop :=
  ∃ s, some s ∈ cands ∧ g ∈ members s

/-- `j` is at least as strong as `o`: equal, or a refusal. -/
def AtLeast (j o : Outcome) : Prop :=
  j = o ∨ ∃ r, j = .refused r

/-- For one invocation the sequence model is `MemberPathRelease.count`. -/
theorem run_one_is_count (f : Fate) (b : Bool) : run [f] b = count f b := by
  cases f <;> cases b <;> rfl

theorem join_decided {a b o : Outcome} (ho : ∀ r, o ≠ .refused r) (h : join a b = o) :
    a = o ∧ b = o := by
  unfold join at h
  split at h
  · rename_i e
    exact ⟨h, e ▸ h⟩
  · exfalso
    split at h <;> exact ho _ h.symm

theorem foldl_decided (os : List Outcome) (acc o : Outcome) (ho : ∀ r, o ≠ .refused r)
    (h : os.foldl join acc = o) : acc = o ∧ ∀ x ∈ os, x = o := by
  induction os generalizing acc with
  | nil => exact ⟨h, by simp⟩
  | cons y os ih =>
    simp only [List.foldl_cons] at h
    obtain ⟨h1, h2⟩ := ih _ h
    obtain ⟨ha, hy⟩ := join_decided ho h1
    refine ⟨ha, ?_⟩
    intro x hx
    rcases List.mem_cons.mp hx with hx | hx
    · exact hx ▸ hy
    · exact h2 x hx

/-- A decided join (`gone` or `stays`) is every candidate's outcome. -/
theorem a_decided_join_is_every_candidate (os : List Outcome) (o : Outcome)
    (ho : ∀ r, o ≠ .refused r) (h : joinAll os = some o) : ∀ x ∈ os, x = o := by
  cases os with
  | nil => simp [joinAll] at h
  | cons y os =>
    simp only [joinAll, Option.some.injEq] at h
    obtain ⟨h1, h2⟩ := foldl_decided os y o ho h
    intro x hx
    rcases List.mem_cons.mp hx with hx | hx
    · exact hx ▸ h1
    · exact h2 x hx

/-- **The join is at least as strong as each candidate's state**: it is
each candidate's composition, or a refusal. -/
theorem the_join_is_at_least_each_candidate (cands : List (Option PathState)) (a : ArmJoin)
    (j : Outcome) (h : tableOutcome cands a = some j) :
    ∀ c ∈ cands, AtLeast j (compose c a) := by
  intro c hc
  cases j with
  | refused r => exact Or.inr ⟨r, rfl⟩
  | gone =>
    refine Or.inl (a_decided_join_is_every_candidate _ .gone (by simp) h _ ?_).symm
    exact List.mem_map.mpr ⟨c, hc, rfl⟩
  | stays =>
    refine Or.inl (a_decided_join_is_every_candidate _ .stays (by simp) h _ ?_).symm
    exact List.mem_map.mpr ⟨c, hc, rfl⟩

/-- What a `gone` composition allows: no stale reading, and a kept one only
under an arm that always releases. -/
theorem gone_members (s : PathState) (a : ArmJoin) (h : compose (some s) a = .gone) :
    ∀ g ∈ members s, g ≠ .stale ∧ (g = .kept → a = .every) := by
  obtain ⟨f, m⟩ := s
  cases m <;> cases f <;> cases a <;> simp_all [compose, composeFate, members]

/-- What a `stays` composition allows: only kept readings, under no arm. -/
theorem stays_members (s : PathState) (a : ArmJoin) (h : compose (some s) a = .stays) :
    a = .none ∧ ∀ g ∈ members s, g = .kept := by
  obtain ⟨f, m⟩ := s
  cases m <;> cases f <;> cases a <;> simp_all [compose, composeFate, members]

theorem foldl_cleared (seq : List Fate) (n : Nat) : seq.foldl step (false, n) = (false, n) := by
  induction seq with
  | nil => rfl
  | cons g seq ih => simpa [List.foldl_cons, step] using ih

/-- Over taking readings (released, moved) and kept ones, the object has
left once and the member is cleared, or nothing happened. -/
theorem foldl_taking (seq : List Fate) (h : ∀ g ∈ seq, g ≠ .stale) :
    seq.foldl step (true, 0) = (false, 1) ∨
      (seq.foldl step (true, 0) = (true, 0) ∧ ∀ g ∈ seq, g = .kept) := by
  induction seq with
  | nil => exact Or.inr ⟨rfl, by simp⟩
  | cons g seq ih =>
    have hs : ∀ x ∈ seq, x ≠ .stale := fun x hx => h x (List.mem_cons_of_mem _ hx)
    have hg : g ≠ .stale := h g (List.mem_cons_self ..)
    simp only [List.foldl_cons]
    cases g with
    | released => exact Or.inl (foldl_cleared seq 1)
    | moved => exact Or.inl (foldl_cleared seq 1)
    | stale => exact absurd rfl hg
    | kept =>
      rcases ih hs with e | ⟨e, hk⟩
      · exact Or.inl e
      · refine Or.inr ⟨e, ?_⟩
        intro x hx
        rcases List.mem_cons.mp hx with hx | hx
        · exact hx
        · exact hk x hx

/-- Over kept readings only, nothing happens. -/
theorem foldl_kept (seq : List Fate) (h : ∀ g ∈ seq, g = .kept) :
    seq.foldl step (true, 0) = (true, 0) := by
  induction seq with
  | nil => rfl
  | cons g seq ih =>
    have hg : g = .kept := h g (List.mem_cons_self ..)
    subst hg
    exact ih (fun x hx => h x (List.mem_cons_of_mem _ hx))

/-- **No double release, no missed release, over any sequence**: a path the
table's join decides `gone` leaves the caller's custody exactly once for
every non-empty sequence of the candidates' readings and every arm the
join allows. -/
theorem a_gone_table_leaves_exactly_once (cands : List (Option PathState)) (a : ArmJoin)
    (h : tableOutcome cands a = some .gone) (seq : List Fate) (hne : seq ≠ [])
    (hseq : ∀ g ∈ seq, allowed cands g) : ∀ b ∈ armReleases a, run seq b = 1 := by
  have every := a_decided_join_is_every_candidate _ .gone (by simp) h
  have hg : ∀ g ∈ seq, g ≠ .stale ∧ (g = .kept → a = .every) := by
    intro g hgs
    obtain ⟨s, hs, hm⟩ := hseq g hgs
    exact gone_members s a (every _ (List.mem_map.mpr ⟨some s, hs, rfl⟩)) g hm
  intro b hb
  rcases foldl_taking seq (fun g hgs => (hg g hgs).1) with e | ⟨e, hk⟩
  · simp [run, e]
  · obtain ⟨g, hgs⟩ := List.exists_mem_of_ne_nil seq hne
    have ha : a = .every := (hg g hgs).2 (hk g hgs)
    subst ha
    simp [armReleases] at hb
    subst hb
    simp [run, e]

/-- A path the table's join decides `stays` never leaves the caller's
custody, for every sequence (the empty one included). -/
theorem a_stays_table_never_leaves (cands : List (Option PathState)) (a : ArmJoin)
    (h : tableOutcome cands a = some .stays) (seq : List Fate)
    (hseq : ∀ g ∈ seq, allowed cands g) : ∀ b ∈ armReleases a, run seq b = 0 := by
  have every := a_decided_join_is_every_candidate _ .stays (by simp) h
  have hk : ∀ g ∈ seq, g = .kept := by
    intro g hgs
    obtain ⟨s, hs, hm⟩ := hseq g hgs
    exact (stays_members s a (every _ (List.mem_map.mpr ⟨some s, hs, rfl⟩))).2 g hm
  have hnone : cands ≠ [] → a = .none := by
    intro hc
    obtain ⟨c, hc⟩ := List.exists_mem_of_ne_nil cands hc
    cases c with
    | none =>
      have := every _ (List.mem_map.mpr ⟨none, hc, rfl⟩)
      simp [compose] at this
    | some s => exact (stays_members s a (every _ (List.mem_map.mpr ⟨some s, hc, rfl⟩))).1
  have hcands : cands ≠ [] := by
    intro hc
    simp [tableOutcome, hc, joinAll] at h
  have ha := hnone hcands
  subst ha
  intro b hb
  simp [armReleases] at hb
  subst hb
  simp [run, foldl_kept seq hk]

/-- Candidates that disagree at a linked path (one takes it, one keeps it,
under no arm) refuse, in either order. -/
theorem disagreeing_candidates_refuse (f : Fate) (hf : f = .released ∨ f = .moved) :
    tableOutcome [some ⟨f, false⟩, some ⟨.kept, false⟩] .none = some (.refused .undecided) ∧
      tableOutcome [some ⟨.kept, false⟩, some ⟨f, false⟩] .none =
        some (.refused .undecided) := by
  rcases hf with hf | hf <;> subst hf <;> decide

/-- **The shared premise**: `gone` with no arm reads the invocation as run;
with no invocation no object leaves. C04.19.6 reads it so at every
invocation form; the table form adds nothing beyond it. -/
theorem the_gone_reading_assumes_an_invocation :
    tableOutcome [some ⟨.released, false⟩] .none = some .gone ∧ run [] false = 0 :=
  ⟨rfl, rfl⟩

/-- Through a released object (rule 74p): the table's composition is the
same `compose` of the effect-order model, applied after the release, so
where that late order admits, the C order admits with the same state. -/
theorem a_table_composition_after_a_release_is_the_c_order {s : Nat} {D G C : List Nat}
    {st t : QCore.CallbackReleasedEffectOrder.St}
    (h : QCore.CallbackReleasedEffectOrder.late s D G C st = some t) :
    QCore.CallbackReleasedEffectOrder.cOrder s D G C st = some t :=
  QCore.CallbackReleasedEffectOrder.late_is_the_c_order h

/-- The fixture's rows (`tests/callback_table_member_path_states.rs`):
`c_cb_releases` alone and with `c_cb_moves` take the payload; beside
`c_cb_other` or `c_cb_plain` (the payload unnamed: kept) it refuses;
`c_cb_stale` refuses stale; `c_cb_maybe` refuses undecided; `c_cb_plain`
and `c_cb_other` keep it. The sequence `[released, moved]` takes it once,
and so does `[kept, released, kept]`. -/
theorem table_vectors_hold :
    tableOutcome [some ⟨.released, false⟩] .none = some .gone ∧
    tableOutcome [some ⟨.released, false⟩, some ⟨.moved, false⟩] .none = some .gone ∧
    tableOutcome [some ⟨.released, false⟩, some ⟨.kept, false⟩] .none =
      some (.refused .undecided) ∧
    tableOutcome [some ⟨.stale, false⟩] .none = some (.refused .stale) ∧
    tableOutcome [some ⟨.moved, true⟩] .none = some (.refused .undecided) ∧
    tableOutcome [some ⟨.kept, false⟩, some ⟨.kept, false⟩] .none = some .stays ∧
    run [.released, .moved] false = 1 ∧
    run [.kept, .released, .kept] false = 1 ∧
    run [.stale, .stale] false = 2 := by
  decide

end QCore.CallbackTableMemberPathStates

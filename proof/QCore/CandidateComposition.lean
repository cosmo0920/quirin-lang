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
# A candidate's own invocations (card HANDOFF-CANDIDATE-OWN-CALLBACKS)

The model of the amendment of gateway rule 13u (with 12m and 12n) in
`crates/qcore-c-gateway/src/candidate_composition.rs` and its hook in
`legacy_callback_route.rs` (`Parser::classified`), and of rule 12m's exact
copy in `handoff_branch.rs` (`exact_copy`).

A callback candidate whose declaration carries its own member invocations is
decided at an invocation as the composition of its own clauses: each own
invocation is decided over that member's run set, with the candidate as the
invoker, exactly as the candidate's own body would face it; the candidate is
contracted only when every one of them decides.

* **Composition.** A candidate is contracted (one round) exactly when its
  own reading admits and each of its own invocations decides
  (`a_composed_candidate_is_contracted_iff_each_own_invocation_decides`); one
  undecided invocation refuses it (`an_undecided_own_invocation_refuses`).
* **Least fixed point.** An own invocation decides only once every inner
  candidate is contracted, so the contracted set is the least fixed point
  from nothing. A set of candidates each of which needs one of the set is
  never contracted at any round (`a_cycle_never_supports_itself`), in
  particular a self-cycle (`a_self_cycle_never_stands`) and a two-candidate
  cycle (`a_two_candidate_cycle_never_stands`). The Rust decides by a
  depth-first search that refuses a candidate met again on its own stack
  (`dfs`); on the shared vectors it agrees with the least fixed point
  (`compose_vectors_hold`).
* **The exact copy.** A variable written only by exact copies of a member
  (`v = o->m`) holds, wherever it is called, a value the member held, or the
  value it arrived with when it is a formal and no copy ran on that path
  (`an_exact_copy_calls_a_member_value`); a local whose declaration is the
  copy holds a member value (`a_copied_local_calls_a_member_value`); so a
  closed set of the member's values covers the call
  (`an_exact_copy_stays_in_the_closed_set`). Any other write may call a
  value the member never held (`an_inexact_copy_may_call_another_value`).
  `copyVectors` is the Rust classifier's `COPY_VECTORS`.

`composeVectors` is `COMPOSE_VECTORS` of `candidate_composition.rs`, bound by
its unit test `compose_vectors_hold`.
-/

namespace QCore.CandidateComposition

/-! ## The composition and its least fixed point -/

/-- One own invocation of a candidate: `admits` is whether the run's set of
the member is closed and every inner candidate's own reading (rule 13u's
class against the clause's actuals, rule 12m's branch check) admits it;
`inner` the inner candidates, each of which must itself be contracted. -/
structure Invocation where
  admits : Bool
  inner : List Nat
  deriving DecidableEq, Repr

/-- A candidate: `base` its own reading (a declaration in force whose custody
contract, hand-off and member-path states admit the outer invocation), and
its own member invocations (none: a declared candidate as today). -/
structure Candidate where
  base : Bool
  own : List Invocation
  deriving DecidableEq, Repr

/-- The run's candidates by index. -/
abbrev Graph := List Candidate

/-- The candidate at `c` (none: no contract in force). -/
def cand (g : Graph) (c : Nat) : Candidate := g.getD c ⟨false, []⟩

/-- An own invocation decides over the contracted set `S`. -/
def decides (S : Nat → Bool) (i : Invocation) : Bool := i.admits && i.inner.all S

/-- One round: a candidate is contracted when its own reading admits and
every own invocation decides. -/
def step (g : Graph) (S : Nat → Bool) (c : Nat) : Bool :=
  (cand g c).base && (cand g c).own.all (decides S)

/-- `n` rounds from nothing contracted. -/
def rounds (g : Graph) : Nat → Nat → Bool
  | 0 => fun _ => false
  | n + 1 => step g (rounds g n)

/-- The least fixed point, reached within as many rounds as candidates. -/
def contracted (g : Graph) (c : Nat) : Bool := rounds g (g.length + 1) c

/-- **Composition.** A candidate is contracted over `S` exactly when its own
reading admits and each of its own invocations decides. -/
theorem a_composed_candidate_is_contracted_iff_each_own_invocation_decides
    (g : Graph) (S : Nat → Bool) (c : Nat) :
    step g S c = true ↔
      (cand g c).base = true ∧ ∀ i ∈ (cand g c).own, decides S i = true := by
  simp [step, List.all_eq_true]

/-- One undecided own invocation refuses the candidate. -/
theorem an_undecided_own_invocation_refuses (g : Graph) (S : Nat → Bool) (c : Nat)
    (i : Invocation) (hi : i ∈ (cand g c).own) (hd : decides S i = false) :
    step g S c = false := by
  simp only [step, Bool.and_eq_false_iff]
  right
  exact List.all_eq_false.mpr ⟨i, hi, by simp [hd]⟩

/-- **A cycle never supports itself.** If every member of a set `Z` has an
own invocation one of whose inner candidates is in `Z`, no member of `Z` is
contracted at any round. -/
theorem a_cycle_never_supports_itself (g : Graph) (Z : Nat → Prop)
    (hZ : ∀ z, Z z → ∃ i ∈ (cand g z).own, ∃ d ∈ i.inner, Z d) :
    ∀ n z, Z z → rounds g n z = false := by
  intro n
  induction n with
  | zero => intro z _; rfl
  | succ n ih =>
    intro z hz
    obtain ⟨i, hi, d, hd, hZd⟩ := hZ z hz
    have hdec : decides (rounds g n) i = false := by
      simp only [decides, Bool.and_eq_false_iff]
      right
      exact List.all_eq_false.mpr ⟨d, hd, by simp [ih d hZd]⟩
    exact an_undecided_own_invocation_refuses g (rounds g n) z i hi hdec

/-- A candidate one of whose own invocations may invoke itself is never
contracted. -/
theorem a_self_cycle_never_stands (g : Graph) (c : Nat)
    (h : ∃ i ∈ (cand g c).own, c ∈ i.inner) : ∀ n, rounds g n c = false := by
  intro n
  obtain ⟨i, hi, hc⟩ := h
  exact a_cycle_never_supports_itself g (fun z => z = c)
    (fun z hz => by rw [hz]; exact ⟨i, hi, c, hc, rfl⟩) n c rfl

/-- Two candidates each invoking the other are never contracted. -/
theorem a_two_candidate_cycle_never_stands (g : Graph) (a b : Nat)
    (ha : ∃ i ∈ (cand g a).own, b ∈ i.inner) (hb : ∃ i ∈ (cand g b).own, a ∈ i.inner) :
    ∀ n, rounds g n a = false ∧ rounds g n b = false := by
  intro n
  have hZ : ∀ z, (z = a ∨ z = b) → ∃ i ∈ (cand g z).own, ∃ d ∈ i.inner, (d = a ∨ d = b) := by
    intro z hz
    rcases hz with rfl | rfl
    · obtain ⟨i, hi, hbi⟩ := ha
      exact ⟨i, hi, b, hbi, Or.inr rfl⟩
    · obtain ⟨i, hi, hai⟩ := hb
      exact ⟨i, hi, a, hai, Or.inl rfl⟩
  exact ⟨a_cycle_never_supports_itself g _ hZ n a (Or.inl rfl),
    a_cycle_never_supports_itself g _ hZ n b (Or.inr rfl)⟩

/-- The Rust's decision: a depth-first search that refuses a candidate met
again while its own composition is decided (`fuel` bounds the depth). -/
def dfs (g : Graph) : Nat → List Nat → Nat → Bool
  | 0, _, _ => false
  | fuel + 1, stack, c =>
    if c ∈ stack then false
    else (cand g c).base &&
      (cand g c).own.all (fun i => i.admits && i.inner.all (dfs g fuel (c :: stack)))

/-- `COMPOSE_VECTORS` of `candidate_composition.rs`: (graph, candidate,
contracted). Candidate `n` of a graph is its `n`-th entry. -/
def composeVectors : List (Graph × Nat × Bool) :=
  [ -- a candidate with no own invocation: as today
    ([⟨true, []⟩], 0, true),
    -- no contract in force
    ([⟨false, []⟩], 0, false),
    -- one own invocation over two declared identities: decides
    ([⟨true, [⟨true, [1, 2]⟩]⟩, ⟨true, []⟩, ⟨true, []⟩], 0, true),
    -- an own invocation over an open set: refuses
    ([⟨true, [⟨false, [1]⟩]⟩, ⟨true, []⟩], 0, false),
    -- an inner candidate with no contract: refuses
    ([⟨true, [⟨true, [1]⟩]⟩, ⟨false, []⟩], 0, false),
    -- a second own invocation undecided: refuses
    ([⟨true, [⟨true, [1]⟩, ⟨false, []⟩]⟩, ⟨true, []⟩], 0, false),
    -- a self-cycle
    ([⟨true, [⟨true, [0]⟩]⟩], 0, false),
    -- a two-candidate cycle, from either end
    ([⟨true, [⟨true, [1]⟩]⟩, ⟨true, [⟨true, [0]⟩]⟩], 0, false),
    ([⟨true, [⟨true, [1]⟩]⟩, ⟨true, [⟨true, [0]⟩]⟩], 1, false),
    -- a composed inner candidate that decides: two levels
    ([⟨true, [⟨true, [1]⟩]⟩, ⟨true, [⟨true, [2]⟩]⟩, ⟨true, []⟩], 0, true),
    -- a candidate reaching a cycle it is not on
    ([⟨true, [⟨true, [1]⟩]⟩, ⟨true, [⟨true, [2]⟩]⟩, ⟨true, [⟨true, [1]⟩]⟩], 0, false),
    -- a closed empty set invokes nothing
    ([⟨true, [⟨true, []⟩]⟩], 0, true) ]

/-- Every vector holds, and the depth-first search agrees with the least
fixed point on each. -/
theorem compose_vectors_hold :
    composeVectors.all (fun (g, c, expected) =>
      contracted g c == expected && dfs g (g.length + 1) [] c == expected) = true := by
  decide

/-! ## The exact copy -/

/-- One statement of a path, as it bears on a variable `v` and the member
`o->m` it may copy. -/
inductive Stmt where
  /-- `v = o->m`: the exact copy. -/
  | copyMember
  /-- Any other write of `v` (a cast, a call, another member or variable). -/
  | assignOther (f : Nat)
  /-- A store into the member. -/
  | storeMember (f : Nat)
  /-- Anything else. -/
  | skip
  deriving DecidableEq, Repr

/-- Run a path from (member value, variable value). -/
def exec : Nat × Nat → List Stmt → Nat × Nat
  | st, [] => st
  | (m, _), .copyMember :: rest => exec (m, m) rest
  | (m, _), .assignOther f :: rest => exec (m, f) rest
  | (_, v), .storeMember f :: rest => exec (f, v) rest
  | st, .skip :: rest => exec st rest

/-- The values the path stores into the member. -/
def stores : List Stmt → List Nat
  | [] => []
  | .storeMember f :: rest => f :: stores rest
  | _ :: rest => stores rest

/-- Whether a path writes the variable only by exact copies. -/
def onlyCopies : List Stmt → Bool
  | [] => true
  | .assignOther _ :: _ => false
  | _ :: rest => onlyCopies rest

theorem exec_held (H : List Nat) (v0 : Nat) :
    ∀ (path : List Stmt) (m v : Nat),
      onlyCopies path = true → (∀ f ∈ stores path, f ∈ H) → m ∈ H → (v ∈ H ∨ v = v0) →
      (exec (m, v) path).2 ∈ H ∨ (exec (m, v) path).2 = v0 := by
  intro path
  induction path with
  | nil => intro m v _ _ _ hv; simpa [exec] using hv
  | cons s rest ih =>
    intro m v ho hs hm hv
    cases s with
    | copyMember =>
      simp only [exec]
      exact ih m m (by simpa [onlyCopies] using ho) (by simpa [stores] using hs) hm (Or.inl hm)
    | assignOther f => simp [onlyCopies] at ho
    | storeMember f =>
      simp only [exec]
      have hf : f ∈ H := hs f (by simp [stores])
      exact ih f v (by simpa [onlyCopies] using ho)
        (fun g hg => hs g (by simp [stores, hg])) hf hv
    | skip =>
      simp only [exec]
      exact ih m v (by simpa [onlyCopies] using ho) (by simpa [stores] using hs) hm hv

/-- **An exact copy calls a member value.** On a path that writes the
variable only by exact copies, the value called is one the member held (its
value at the start or one stored on the path), or the value the variable
arrived with (a formal on a path that skips the copy). -/
theorem an_exact_copy_calls_a_member_value (path : List Stmt) (m0 v0 : Nat)
    (h : onlyCopies path = true) :
    (exec (m0, v0) path).2 ∈ m0 :: stores path ∨ (exec (m0, v0) path).2 = v0 :=
  exec_held (m0 :: stores path) v0 path m0 v0 h (fun f hf => by simp [hf]) (by simp)
    (Or.inr rfl)

/-- A local whose declaration is the copy calls a member value. -/
theorem a_copied_local_calls_a_member_value (rest : List Stmt) (m0 v0 : Nat)
    (h : onlyCopies rest = true) :
    (exec (m0, v0) (.copyMember :: rest)).2 ∈ m0 :: stores rest := by
  -- After the copy the variable arrives holding the member's value `m0`.
  have h2 := exec_held (m0 :: stores rest) m0 rest m0 m0 h (fun f hf => by simp [hf])
    (by simp) (Or.inr rfl)
  simp only [exec]
  rcases h2 with h2 | h2
  · exact h2
  · rw [h2]; simp

/-- **The closed set covers the call.** When every value the member holds is
in the run's closed set, an exact copy calls a candidate of the set (or the
formal's own value). -/
theorem an_exact_copy_stays_in_the_closed_set (C : Nat → Prop) (path : List Stmt)
    (m0 v0 : Nat) (h : onlyCopies path = true) (hm : C m0) (hs : ∀ f ∈ stores path, C f) :
    C (exec (m0, v0) path).2 ∨ (exec (m0, v0) path).2 = v0 := by
  rcases an_exact_copy_calls_a_member_value path m0 v0 h with hin | heq
  · left
    simp only [List.mem_cons] at hin
    rcases hin with h0 | hst
    · rw [h0]; exact hm
    · exact hs _ hst
  · exact Or.inr heq

/-- **Any other write may call another value.** One write from anything but
the member makes the variable call that value, which the member never held. -/
theorem an_inexact_copy_may_call_another_value (m0 v0 f : Nat) (hf : f ≠ m0) (hv : f ≠ v0) :
    (exec (m0, v0) [.copyMember, .assignOther f]).2 ∉ m0 :: stores [.copyMember, .assignOther f] ∧
      (exec (m0, v0) [.copyMember, .assignOther f]).2 ≠ v0 := by
  simp [exec, stores, hf, hv]

/-- One write of the variable, as the Rust classifier reads it. -/
inductive Write where
  /-- `v = o->m` (or the declaration's initializer), `o` a variable the body
  never assigns. -/
  | fromMember
  /-- Any other write: a cast, a call, another expression, a compound
  assignment or an increment. -/
  | other
  deriving DecidableEq, Repr

/-- The classifier: exactly one write, an exact copy, and the variable's
address never taken. -/
def exactCopy (writes : List Write) (addressTaken : Bool) : Bool :=
  !addressTaken && writes == [.fromMember]

/-- `COPY_VECTORS` of `handoff_branch.rs`: (writes, address taken, exact). -/
def copyVectors : List (List Write × Bool × Bool) :=
  [ ([.fromMember], false, true),
    ([.fromMember], true, false),
    ([.fromMember, .fromMember], false, false),
    ([.fromMember, .other], false, false),
    ([.other], false, false),
    ([], false, false) ]

theorem copy_vectors_hold :
    copyVectors.all (fun (writes, taken, exact) => exactCopy writes taken == exact) = true := by
  decide

/-- What the classifier admits is a single exact copy: every path through
it writes the variable only by that copy. -/
theorem an_admitted_copy_is_the_only_write (writes : List Write) (taken : Bool)
    (h : exactCopy writes taken = true) : writes = [.fromMember] ∧ taken = false := by
  simp only [exactCopy, Bool.and_eq_true, Bool.not_eq_true', beq_iff_eq] at h
  exact ⟨h.2, h.1⟩

end QCore.CandidateComposition

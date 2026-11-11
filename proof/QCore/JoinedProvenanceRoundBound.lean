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

import QCore.JoinedProvenanceCycles

/-! Abstract rules for the **iteration bound** of the conditional-provenance
fixed point (card JOINED-PROVENANCE-ROUND-BOUND, wave 37).

`QCore.JoinedProvenanceCycles` states what the greatest fixed point may *claim*:
a family of claims satisfying the fixed-point inequality is sound on every
reaching path, weakening is always available, and on an acyclic graph the
equation has one solution. It says nothing about **when the analyzer may stop
iterating** — `FIXED_POINT_ROUNDS = 64` was a magic number, and a graph that had
not settled by then derived nothing.

This module supplies the missing half: the round function's state space is
finite and descends, so the iteration *terminates*, and the round at which it
settles is bounded by a number the graph itself determines. That turns the
analyzer's cap from a guess into a measured point on a proved scale.

Two things are deliberately **not** claimed. Nothing here says the bound is
small — it is `nodes * (1 + sites + sites * names)`, which for a large body is
far above 64; what the card's report publishes beside it is the *measurement* of
where the pinned corpus's cyclic functions actually settle. And nothing here
weakens a refusal: `claiming_nothing_is_sound` already says declining is safe,
so a cap below the bound is a **loss of derivation, never a loss of soundness**.

As with every gateway module these are **abstract rule theorems, not extraction
of the Rust analyzer** (`GATEWAY-LEAN-ABSTRACT-NOT-EXTRACTION`). -/
namespace QCore.JoinedProvenanceRoundBound

/-! ## 1. Counting over a finite prefix

The analyzer's per-node value ranges over two finite universes: the sites the
body creates and the `conditional_names` a claim may list. Everything below is
built from one counting function so that the bound is arithmetic rather than
asserted. -/

/-- How many members of `l` satisfy `f`. -/
def countL (l : List Nat) (f : Nat → Bool) : Nat := (l.filter f).length

/-- How many of `0, …, k-1` satisfy `f`. -/
def countB (k : Nat) (f : Nat → Bool) : Nat := countL (List.range k) f

theorem countL_le (f : Nat → Bool) : ∀ l : List Nat, countL l f ≤ l.length := by
  intro l
  induction l with
  | nil => simp [countL]
  | cons a t ih =>
    simp only [countL, List.filter_cons] at *
    cases f a with
    | false => simp; omega
    | true => simp; omega

theorem countB_le (k : Nat) (f : Nat → Bool) : countB k f ≤ k := by
  have := countL_le f (List.range k)
  simpa [countB, List.length_range] using this

/-- A pointwise-smaller predicate counts no more. This is the monotone half of
"each round's intersection is non-increasing". -/
theorem countL_mono (f g : Nat → Bool) (h : ∀ i, f i = true → g i = true) :
    ∀ l : List Nat, countL l f ≤ countL l g := by
  intro l
  induction l with
  | nil => simp [countL]
  | cons a t ih =>
    simp only [countL, List.filter_cons] at *
    cases hfa : f a with
    | false =>
      cases hga : g a with
      | false => simpa using ih
      | true => simp; omega
    | true =>
      have hga : g a = true := h a hfa
      simp [hga]; omega

theorem countB_mono (k : Nat) (f g : Nat → Bool) (h : ∀ i, f i = true → g i = true) :
    countB k f ≤ countB k g := countL_mono f g h (List.range k)

/-- One witness the larger predicate has and the smaller has not makes the
count strictly smaller: the strict half. -/
theorem countL_lt (f g : Nat → Bool) (h : ∀ i, f i = true → g i = true)
    (x : Nat) (hf : f x = false) (hg : g x = true) :
    ∀ l : List Nat, x ∈ l → countL l f < countL l g := by
  intro l
  induction l with
  | nil => intro hx; exact absurd hx (by simp)
  | cons a t ih =>
    intro hx
    simp only [countL, List.filter_cons] at *
    rcases List.mem_cons.mp hx with rfl | hxt
    · simp [hf, hg]
      have := countL_mono f g h t
      simp only [countL] at this
      omega
    · cases hfa : f a with
      | false =>
        cases hga : g a with
        | false => simpa using ih hxt
        | true => simp; have := ih hxt; omega
      | true =>
        have hga : g a = true := h a hfa
        simp [hga]
        have := ih hxt
        omega

theorem countB_lt (k : Nat) (f g : Nat → Bool) (h : ∀ i, f i = true → g i = true)
    (x : Nat) (hx : x < k) (hf : f x = false) (hg : g x = true) :
    countB k f < countB k g :=
  countL_lt f g h x hf hg (List.range k) (List.mem_range.mpr hx)

/-! ## 2. One node's value and its measure -/

/-- One node's abstract value, in the shape the analyzer's `Option Facts` has.
`top` is where a node sits before any predecessor has reached it — the neutral
element of the meet, and the element the whole vector but the entry starts at.
`reached` carries the present sites and, for each site, the names still claimed.

Descending means: **present grows and names shrink**. That is the direction the
meet moves in (`Facts::join` unions `present` and intersects the name sets), and
it is why the iteration can only ever weaken a claim. -/
inductive NodeValue where
  | top
  | reached (present : Nat → Bool) (names : Nat → Nat → Bool)

/-- **An absent site claims everything.** A site not in `present` makes a
vacuous claim — `soundAt` quantifies over `w.present site = true` — so *absence
is the top element of that site's own lattice*, and the meet's `(false, true)`
arm taking the other side's names is exactly "top meets anything to give
anything".

This is the correction the measure turns on, and it is not cosmetic: counting
only the names literally stored would make a site's arrival — absent, with an
empty stored name set, becoming present with a claim — **increase** the count,
and the descent argument would be false as stated. Reading an absent site as
claiming every name makes arrival a decrease, which is what it is. -/
def liveAt (names : Nat) (p : Nat → Bool) (q : Nat → Nat → Bool) (i : Nat) : Bool :=
  !p (i / names) || q (i / names) (i % names)

/-- Everything one node can still lose: the top element itself, the sites it has
yet to admit, and the name memberships it has yet to drop. -/
def nodeBudget (sites names : Nat) : Nat := 1 + sites + sites * names

/-- How far one node still is from the bottom of its own descent. `top` is worth
the whole budget; a reached value is worth the sites still absent plus the name
memberships still live, an absent site counting as claiming all of them. -/
def measure (sites names : Nat) : NodeValue → Nat
  | .top => nodeBudget sites names
  | .reached p q => countB sites (fun s => !p s) + countB (sites * names) (liveAt names p q)

theorem measure_le (sites names : Nat) (v : NodeValue) :
    measure sites names v ≤ nodeBudget sites names := by
  cases v with
  | top => exact Nat.le_refl _
  | reached p q =>
    have h1 := countB_le sites (fun s => !p s)
    have h2 := countB_le (sites * names) (liveAt names p q)
    simp only [measure, nodeBudget]
    omega

/-- A **strict** descent of one node's value, with the witness that makes it
strict. The three constructors are the only ways the round function can move a
node: off the top, by admitting a site, or by dropping a name at a site it
already has. -/
inductive Descends (sites names : Nat) : NodeValue → NodeValue → Prop where
  /-- A predecessor reached this node for the first time. -/
  | fromTop (p : Nat → Bool) (q : Nat → Nat → Bool) :
      Descends sites names (.reached p q) .top
  /-- A site the meet made present here, every other component weakening. -/
  | site (p q p' q') (presentGrew : ∀ s, p' s = true → p s = true)
      (namesShrank : ∀ s n, q s n = true → q' s n = true)
      (s : Nat) (hs : s < sites) (was : p' s = false) (now : p s = true) :
      Descends sites names (.reached p q) (.reached p' q')
  /-- A name the meet's intersection dropped at a site this value has. The
  site-present premise is what makes the drop a real loss: at an absent site
  there is nothing to lose, the claim being vacuous either way. -/
  | name (p q p' q') (presentGrew : ∀ s, p' s = true → p s = true)
      (namesShrank : ∀ s n, q s n = true → q' s n = true)
      (s n : Nat) (hs : s < sites) (hn : n < names) (here : p s = true)
      (was : q s n = false) (now : q' s n = true) :
      Descends sites names (.reached p q) (.reached p' q')

/-- The flattened index of `(s, n)`, and the two facts that make the decoding
exact. `names` is positive wherever a name witness exists. -/
theorem decode_index (names : Nat) (s n : Nat) (hn : n < names) :
    (s * names + n) / names = s ∧ (s * names + n) % names = n := by
  have hpos : 0 < names := Nat.lt_of_le_of_lt (Nat.zero_le n) hn
  constructor
  · rw [Nat.add_comm, Nat.add_mul_div_right _ _ hpos, Nat.div_eq_of_lt hn]
    omega
  · rw [Nat.add_comm, Nat.add_mul_mod_self_right, Nat.mod_eq_of_lt hn]

theorem liveAt_index (names : Nat) (p : Nat → Bool) (q : Nat → Nat → Bool) (s n : Nat)
    (hn : n < names) : liveAt names p q (s * names + n) = (!p s || q s n) := by
  obtain ⟨hdiv, hmod⟩ := decode_index names s n hn
  simp [liveAt, hdiv, hmod]

/-- The absent-site count is monotone along a descent: present only grows. -/
theorem absent_mono (p p' : Nat → Bool) (presentGrew : ∀ s, p' s = true → p s = true) :
    ∀ i, (!p i) = true → (!p' i) = true := by
  intro i hi
  by_cases hp' : p' i = true
  · simp [presentGrew i hp'] at hi
  · simp_all

/-- The live-name count is monotone along a descent, **because absence reads as
the top**: a site the newer value has and the older had not contributes its real
names on the left and every name on the right. -/
theorem liveAt_mono (names : Nat) (p p' : Nat → Bool) (q q' : Nat → Nat → Bool)
    (presentGrew : ∀ s, p' s = true → p s = true)
    (namesShrank : ∀ s n, q s n = true → q' s n = true) :
    ∀ i, liveAt names p q i = true → liveAt names p' q' i = true := by
  intro i hi
  simp only [liveAt, Bool.or_eq_true, Bool.not_eq_true'] at *
  rcases hi with hp | hq
  · left
    by_cases hp' : p' (i / names) = true
    · rw [presentGrew _ hp'] at hp; exact absurd hp (by simp)
    · simp_all
  · right; exact namesShrank _ _ hq

/-- **Every strict descent strictly decreases the measure.** This is the whole
content of "the iteration cannot run forever": the state space is finite, the
round function only ever moves down in it, and a move down is a step on a
natural-number ladder whose top is `nodeBudget`. -/
theorem descent_decreases_measure (sites names : Nat) (a b : NodeValue)
    (d : Descends sites names a b) :
    measure sites names a < measure sites names b := by
  cases d with
  | fromTop p q =>
    have h1 := countB_le sites (fun s => !p s)
    have h2 := countB_le (sites * names) (liveAt names p q)
    simp only [measure, nodeBudget]
    omega
  | site p q p' q' presentGrew namesShrank s hs was now =>
    have hsite : countB sites (fun s => !p s) < countB sites (fun s => !p' s) :=
      countB_lt sites _ _ (absent_mono p p' presentGrew) s hs (by simp [now]) (by simp [was])
    have hname :
        countB (sites * names) (liveAt names p q)
          ≤ countB (sites * names) (liveAt names p' q') :=
      countB_mono _ _ _ (liveAt_mono names p p' q q' presentGrew namesShrank)
    simp only [measure]
    omega
  | name p q p' q' presentGrew namesShrank s n hs hn here was now =>
    have hsite : countB sites (fun s => !p s) ≤ countB sites (fun s => !p' s) :=
      countB_mono sites _ _ (absent_mono p p' presentGrew)
    have hbound : s * names + n < sites * names := by
      calc s * names + n < s * names + names := Nat.add_lt_add_left hn _
        _ = (s + 1) * names := by rw [Nat.succ_mul]
        _ ≤ sites * names := Nat.mul_le_mul_right _ hs
    have hname :
        countB (sites * names) (liveAt names p q)
          < countB (sites * names) (liveAt names p' q') := by
      refine countB_lt (sites * names) _ _
        (liveAt_mono names p p' q q' presentGrew namesShrank) (s * names + n) hbound ?_ ?_
      · rw [liveAt_index names p q s n hn, here, was]; rfl
      · rw [liveAt_index names p' q' s n hn, now]; simp
    simp only [measure]
    omega

/-! ## 3. The whole vector

The analyzer iterates every node at once (a Jacobi sweep), so the quantity that
descends is the **sum** over nodes. -/

theorem sumMap_le (F G : Nat → Nat) (h : ∀ i, F i ≤ G i) :
    ∀ l : List Nat, (l.map F).sum ≤ (l.map G).sum := by
  intro l
  induction l with
  | nil => simp
  | cons a t ih => simp only [List.map_cons, List.sum_cons]; exact Nat.add_le_add (h a) ih

theorem sumMap_lt (F G : Nat → Nat) (h : ∀ i, F i ≤ G i) (x : Nat) (hx : F x < G x) :
    ∀ l : List Nat, x ∈ l → (l.map F).sum < (l.map G).sum := by
  intro l
  induction l with
  | nil => intro hm; exact absurd hm (by simp)
  | cons a t ih =>
    intro hm
    simp only [List.map_cons, List.sum_cons]
    rcases List.mem_cons.mp hm with rfl | hxt
    · exact Nat.add_lt_add_of_lt_of_le hx (sumMap_le F G h t)
    · exact Nat.add_lt_add_of_le_of_lt (h a) (ih hxt)

theorem sumMap_bound (F : Nat → Nat) (c : Nat) (h : ∀ i, F i ≤ c) :
    ∀ l : List Nat, (l.map F).sum ≤ l.length * c := by
  intro l
  induction l with
  | nil => simp
  | cons a t ih =>
    simp only [List.map_cons, List.sum_cons, List.length_cons, Nat.succ_mul]
    have := Nat.add_le_add (h a) ih
    omega

/-- The measure of the whole vector: one node's measure summed over the
reachable nodes. -/
def total (nodes sites names : Nat) (v : Nat → NodeValue) : Nat :=
  ((List.range nodes).map (fun i => measure sites names (v i))).sum

/-- The bound the card states: `nodes * (1 + sites + sites * names)`. -/
def graphBudget (nodes sites names : Nat) : Nat := nodes * nodeBudget sites names

theorem total_le_graphBudget (nodes sites names : Nat) (v : Nat → NodeValue) :
    total nodes sites names v ≤ graphBudget nodes sites names := by
  have := sumMap_bound (fun i => measure sites names (v i)) (nodeBudget sites names)
    (fun i => measure_le sites names (v i)) (List.range nodes)
  simpa [total, graphBudget, List.length_range] using this

/-- **A sweep that moves anything strictly decreases the vector's measure.**
`weaker` is the round function only ever descending — Kleene iteration from the
top with a monotone transfer — and `moved` is the one node that changed. -/
theorem sweep_decreases_total (nodes sites names : Nat) (v w : Nat → NodeValue)
    (weaker : ∀ i, measure sites names (w i) ≤ measure sites names (v i))
    (x : Nat) (hx : x < nodes) (moved : Descends sites names (w x) (v x)) :
    total nodes sites names w < total nodes sites names v :=
  sumMap_lt _ _ weaker x (descent_decreases_measure sites names _ _ moved)
    (List.range nodes) (List.mem_range.mpr hx)

/-! ## 4. Termination, and what a cap buys

The two theorems the analyzer's `FIXED_POINT_ROUNDS` rests on, stated for any
step function with a strictly decreasing measure — which §2 and §3 have just
shown the round function to be. -/

/-- `iter step k s` is `s` swept `k` times: exactly the analyzer's loop, whose
trip count is the cap. Spelled out here rather than taken from a library, so
this module depends on nothing but Lean's core. -/
def iter {S : Type} (step : S → S) : Nat → S → S
  | 0, s => s
  | n + 1, s => step (iter step n s)

theorem iter_succ_left {S : Type} (step : S → S) (s : S) :
    ∀ n, iter step (n + 1) s = iter step n (step s) := by
  intro n
  induction n generalizing s with
  | zero => rfl
  | succ n ih => show step (iter step (n + 1) s) = _; rw [ih]; rfl

theorem iter_add {S : Type} (step : S → S) (s : S) :
    ∀ m n, iter step (m + n) s = iter step m (iter step n s) := by
  intro m n
  induction m with
  | zero => rw [Nat.zero_add]; rfl
  | succ m ih =>
    rw [Nat.succ_add]
    show step (iter step (m + n) s) = step (iter step m (iter step n s))
    rw [ih]

/-- **Kleene iteration from the top reaches a fixed point within the measure.**
Every round either changes nothing — and the iteration has settled — or spends
one unit of a finite budget. -/
theorem kleene_reaches_a_fixed_point {S : Type} (step : S → S) (μ : S → Nat)
    (decreases : ∀ s, step s ≠ s → μ (step s) < μ s) (s₀ : S) :
    ∃ k, k ≤ μ s₀ ∧ iter step k s₀ = iter step (k + 1) s₀ := by
  have key : ∀ b s, μ s ≤ b → ∃ k, k ≤ μ s ∧ iter step k s = iter step (k + 1) s := by
    intro b
    induction b with
    | zero =>
      intro s hs
      refine ⟨0, Nat.zero_le _, ?_⟩
      by_cases h : step s = s
      · show s = step s
        exact h.symm
      · exact absurd (decreases s h) (by omega)
    | succ b ih =>
      intro s hs
      by_cases h : step s = s
      · exact ⟨0, Nat.zero_le _, h.symm⟩
      · have hlt := decreases s h
        obtain ⟨k, hk, hfix⟩ := ih (step s) (by omega)
        refine ⟨k + 1, by omega, ?_⟩
        rw [iter_succ_left step s k, iter_succ_left step s (k + 1)]
        exact hfix
  exact key (μ s₀) s₀ (Nat.le_refl _)

/-- Once the iteration has settled it stays settled, so a cap above the settling
round returns the same answer as the settling round itself. -/
theorem iter_stable {S : Type} (step : S → S) (s : S) (h : step s = s) :
    ∀ k, iter step k s = s := by
  intro k
  induction k with
  | zero => rfl
  | succ k ih => show step (iter step k s) = s; rw [ih, h]

/-- **A cap at or above the measure never declines.** This is the operative
statement: the analyzer's `FIXED_POINT_ROUNDS` is *sufficient* for any graph
whose measure it reaches, and the proved bound §3 gives is
`graphBudget = nodes * (1 + sites + sites * names)`. A cap **below** the measure
may stop early — which by `JoinedProvenanceCycles.claiming_nothing_is_sound`
costs derivation and never soundness. -/
theorem a_cap_at_the_measure_settles {S : Type} (step : S → S) (μ : S → Nat)
    (decreases : ∀ s, step s ≠ s → μ (step s) < μ s) (s₀ : S) (cap : Nat)
    (enough : μ s₀ ≤ cap) :
    iter step cap s₀ = iter step (cap + 1) s₀ := by
  obtain ⟨k, hk, hfix⟩ := kleene_reaches_a_fixed_point step μ decreases s₀
  have hpt : step (iter step k s₀) = iter step k s₀ := hfix.symm
  have hle : k ≤ cap := Nat.le_trans hk enough
  obtain ⟨j, rfl⟩ : ∃ j, cap = j + k := ⟨cap - k, by omega⟩
  rw [iter_add step s₀ j k, iter_stable step _ hpt j]
  rw [Nat.add_right_comm j k 1, iter_add step s₀ (j + 1) k, iter_stable step _ hpt (j + 1)]

/-- The bound, assembled: a Jacobi sweep over a graph of `nodes` nodes, `sites`
sites and `names` names settles within `nodes * (1 + sites + sites * names)`
rounds, and a cap at least that large therefore never declines. -/
theorem rounds_are_bounded_by_the_graph_budget
    (nodes sites names : Nat) (sweep : (Nat → NodeValue) → (Nat → NodeValue))
    (descending : ∀ v, sweep v ≠ v → total nodes sites names (sweep v)
      < total nodes sites names v)
    (v₀ : Nat → NodeValue) (cap : Nat) (enough : graphBudget nodes sites names ≤ cap) :
    iter sweep cap v₀ = iter sweep (cap + 1) v₀ :=
  a_cap_at_the_measure_settles sweep (total nodes sites names) descending v₀ cap
    (Nat.le_trans (total_le_graphBudget nodes sites names v₀) enough)

/-! ## 5. What a bound does not buy

The settled state is a fixed point of the round function, which is exactly the
hypothesis `JoinedProvenanceCycles.fixed_point_is_sound` consumes. A bound is a
statement about **when** the analyzer may stop, never about what it may then
claim: soundness stays that module's theorem, unchanged by anything here. -/
theorem a_settled_state_is_a_fixed_point {S : Type} (step : S → S) (s : S)
    (h : iter step 0 s = iter step (0 + 1) s) : step s = s := h.symm

/-- And the other direction of the same point: stopping early is a refusal, and
a refusal is sound. Restated here so the two halves of the cap decision sit
beside each other. -/
theorem stopping_early_is_declining (site : Nat) (w : QCore.CGatewayRefinements.World) :
    QCore.JoinedProvenanceCycles.soundAt site w (fun _ => False) :=
  QCore.JoinedProvenanceCycles.claiming_nothing_is_sound site w

end QCore.JoinedProvenanceRoundBound

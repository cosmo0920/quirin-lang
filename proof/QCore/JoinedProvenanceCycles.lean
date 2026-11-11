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

import QCore.CGatewayRefinements

/-! Abstract rules for the **cyclic** extension of the conditional-provenance
pre-pass (card JOINED-PROVENANCE-CYCLES). Wave 31 derived `site -> names` over
acyclic custody CFGs only; this module states the rules under which the same
relation, computed as a greatest fixed point, carries over to a graph with back
edges.

As with every gateway module, these are **abstract rule theorems, not
extraction of the Rust analyzer** (`GATEWAY-LEAN-ABSTRACT-NOT-EXTRACTION`).
Faithful token/type extraction, the CFG construction and declared release
effects remain premises. What is proved here is that the fixed-point rule the
analyzer implements cannot derive a fact that fails on a reachable path, that
declining is always available, and that on an acyclic graph the rule has only
one solution — so no acyclic body's verdict can move. -/
namespace QCore.JoinedProvenanceCycles

open QCore.CGatewayRefinements

abbrev Node := Nat

/-- The names a program point claims for one site. -/
abbrev Claim := Nat → Prop

/-- `c` is sound at `w` for `site`: every claimed name denotes the site
wherever the site exists. Absence makes the claim vacuous, which is what lets a
predecessor that does not have the site stay neutral. -/
def soundAt (site : Nat) (w : World) (c : Claim) : Prop :=
  ∀ n, c n → (w.present site = true → w.value n = some site)

/-- A control-flow graph with a concrete per-node effect and a per-**edge**
abstract transfer. The edge index carries the null-branch exclusion: on the
null edge the analyzer drops every site the tested name denotes, which is a
weakening of the claim and never a strengthening. -/
structure Graph where
  succ : Node → List Node
  entry : Node
  transfer : Node → World → World
  abstractOut : Node → Node → Claim → Claim

/-- The abstract transfer along `p → n` never outruns the concrete one. -/
def TransferSound (g : Graph) (site : Nat) (p n : Node) : Prop :=
  ∀ w c, soundAt site w c → soundAt site (g.transfer p w) (g.abstractOut p n c)

/-- The worlds that actually arrive at a node, by any path from the entry.
`Reach` is closed under steps and therefore describes cyclic graphs exactly as
it does acyclic ones: a loop is a path that visits a node again. -/
inductive Reach (g : Graph) (w₀ : World) : Node → World → Prop where
  | start : Reach g w₀ g.entry w₀
  | step {p n : Node} {w : World} :
      Reach g w₀ p w → n ∈ g.succ p → Reach g w₀ n (g.transfer p w)

/-- **Soundness on a cyclic graph.** A family of claims that satisfies the
fixed-point *inequality* — each node claims no more than every predecessor's
transferred claim — and is sound at the entry, is sound at every node on every
path that reaches it. The induction is on the arrival itself, so a back edge is
no different from a forward one: the claim a cycle carries had to survive the
meet with the path that first entered the loop. -/
theorem fixed_point_is_sound
    (g : Graph) (w₀ : World) (site : Nat) (value : Node → Claim)
    (transfers : ∀ p n, TransferSound g site p n)
    (entry_sound : soundAt site w₀ (value g.entry))
    (fp : ∀ p n, n ∈ g.succ p → ∀ m, value n m → g.abstractOut p n (value p) m) :
    ∀ n w, Reach g w₀ n w → soundAt site w (value n) := by
  intro n w reach
  induction reach with
  | start => exact entry_sound
  | step _ hmem ih =>
      intro m hm
      exact transfers _ _ _ _ ih m (fp _ _ hmem m hm)

/-- **Refusing is always available.** A claim weaker than a sound one is sound,
so dropping a site, clearing a name set, or declining a whole function — what
the analyzer does when the iteration bound is exhausted, and for every site
created inside a cycle — can never make a derivation unsound. -/
theorem weakening_stays_sound (site : Nat) (w : World) (c d : Claim)
    (weaker : ∀ n, d n → c n) (h : soundAt site w c) : soundAt site w d :=
  fun n hn => h n (weaker n hn)

/-- Declining outright is the extreme case of the previous theorem. -/
theorem claiming_nothing_is_sound (site : Nat) (w : World) :
    soundAt site w (fun _ => False) := by
  intro _ h
  exact h.elim

/-- The meet of the predecessors' transferred claims: the greatest claim the
inequality permits at `n`. -/
def meetPreds (g : Graph) (value : Node → Claim) (n : Node) : Claim :=
  fun m => ∀ p, n ∈ g.succ p → g.abstractOut p n (value p) m

/-- Taking the meet exactly satisfies the inequality `fixed_point_is_sound`
wants, so the greatest solution is admissible and not merely some solution. -/
theorem meet_satisfies_the_inequality (g : Graph) (value : Node → Claim)
    (p n : Node) (hmem : n ∈ g.succ p) (m : Nat) (h : meetPreds g value n m) :
    g.abstractOut p n (value p) m := h p hmem

/-- **Conservative extension.** On a graph whose edges strictly increase a rank
— an acyclic graph — the fixed-point *equation* has exactly one solution. The
wave-31 topological pass computes a solution, so the fixed point computed here
is that same one: no verdict on an acyclic body can change. -/
theorem acyclic_solution_is_unique
    (g : Graph) (rank : Node → Nat)
    (increasing : ∀ p n, n ∈ g.succ p → rank p < rank n)
    (a b : Node → Claim)
    (root : a g.entry = b g.entry)
    (eqa : ∀ n, n ≠ g.entry → a n = meetPreds g a n)
    (eqb : ∀ n, n ≠ g.entry → b n = meetPreds g b n) :
    ∀ n, a n = b n := by
  -- Strong induction on the rank, carried by an explicit bound so the proof
  -- needs nothing beyond core `Nat` induction.
  have key : ∀ k n, rank n < k → a n = b n := by
    intro k
    induction k with
    | zero => intro n h; exact absurd h (Nat.not_lt_zero _)
    | succ k ih =>
      intro n hn
      by_cases hentry : n = g.entry
      · subst hentry; exact root
      · have preds : ∀ p, n ∈ g.succ p → a p = b p := fun p hmem =>
          ih p (Nat.lt_of_lt_of_le (increasing p n hmem) (Nat.le_of_lt_succ hn))
        rw [eqa n hentry, eqb n hentry]
        funext m
        apply propext
        simp only [meetPreds]
        constructor
        · intro h p hmem
          have hp := h p hmem
          rwa [preds p hmem] at hp
        · intro h p hmem
          have hp := h p hmem
          rwa [← preds p hmem] at hp
  exact fun n => key (rank n + 1) n (Nat.lt_succ_self _)

/-- One or more edges. `Steps g n n` is exactly "`n` lies on a cycle", which is
the test the analyzer runs to decide whether a site is eligible. -/
inductive Steps (g : Graph) : Node → Node → Prop where
  | one {p n : Node} : n ∈ g.succ p → Steps g p n
  | trans {p m n : Node} : Steps g p m → n ∈ g.succ m → Steps g p n

theorem steps_increase_rank (g : Graph) (rank : Node → Nat)
    (increasing : ∀ p n, n ∈ g.succ p → rank p < rank n) :
    ∀ p n, Steps g p n → rank p < rank n := by
  intro p n s
  induction s with
  | one hmem => exact increasing _ _ hmem
  | trans _ hmem ih => exact Nat.lt_trans ih (increasing _ _ hmem)

/-- A node that lies on no cycle runs at most once per call, which is what makes
a per-site identity fact a statement about **one** object rather than about the
site's whole family of dynamic objects. The analyzer admits a site only when its
creating node passes this test; a site created inside a loop derives nothing,
and `weakening_stays_sound` is why declining it is safe. -/
theorem an_off_cycle_node_does_not_return (g : Graph) (rank : Node → Nat)
    (increasing : ∀ p n, n ∈ g.succ p → rank p < rank n) (n : Node) :
    ¬ Steps g n n := by
  intro s
  exact Nat.lt_irrefl _ (steps_increase_rank g rank increasing n n s)

end QCore.JoinedProvenanceCycles

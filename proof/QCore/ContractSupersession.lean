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

import QCore.VerdictSemigroup

/-!
# Contract bundles compose by supersession

Card **CONTRACT-SUPERSESSION** (contract specification §C06.6;
`docs/plans/pluggable-cards/README.md` seam G, user decisions 9 and 9a). This
is the model of `crates/qcore-schemas/src/contract_supersession.rs`'s
`decide` and of the retention `crates/qcore-contract/src/contract_supersession.rs`
performs.

A *candidate* is one module's declaration of a function: its module, its text
(the normalised contract digest) and its pin stamp, `none` when the stamp is
unusable (missing, malformed, stale or unpinned: the Rust side's non-`Stamped`
standings). The candidates of a function are a finite set, kept as a list read
through membership (`Equiv`). `decide` keeps the text of the candidates with
the greatest stamp in force when every candidate is stamped and those newest
candidates agree on their text; otherwise it is a conflict: no text is in
force.

## What is proved

* **Determinism.** `decide` reads only the set of candidates
  (`decide_congr`): neither their order nor their multiplicity changes the
  outcome, so it is associative and commutative over the union of bundles
  (`decide_union_comm`, `decide_union_assoc`) and a byte-identical duplicate
  changes nothing (`decide_duplicate`).
* **Newest wins.** A stamped candidate whose stamp no candidate exceeds, and
  every candidate of equal stamp agreeing on its text, is in force
  (`newest_wins`), in particular a strictly newest one
  (`strictly_newest_wins`).
* **A tie is a conflict.** Two newest candidates with different texts are a
  conflict (`a_tie_is_a_conflict`), and so is any candidate with no usable
  stamp (`an_unusable_stamp_is_a_conflict`), even beside an older stamped one:
  an unpinned declaration never supersedes. No candidate is no text
  (`nothing_is_a_conflict`).
* **Retention.** A text in force is a newest candidate's, never invented
  (`in_force_is_a_newest_candidate`); every candidate is either the text in
  force or retained in the record (`nothing_is_erased`), and the record holds
  only candidates (`the_record_holds_only_candidates`).
* **Never optimistic.** A conflicted function's call is held as `Unknown`, so
  the row it joins is never `Safe` and never `Pending`
  (`a_conflict_is_never_safe`, `a_conflict_is_never_pending`), and never lowers
  a row (`a_conflict_never_lowers`).
* **Shared vectors.** `vectors` is decided here, and the same cases are
  checked against the Rust `decide` by
  `crates/qcore-schemas/tests/contract_supersession.rs`.

## What this is not

That the lock entry's stamp records a human review, and that the Rust
transform cuts exactly the declarations the record names, are premises here;
the correspondence is pinned by the tests named above and by review
(constraint 28).
-/

namespace QCore.ContractSupersession

open QCore.VerdictSemigroup

/-- One module's declaration of a function. -/
structure Cand where
  module : Nat
  text : Nat
  stamp : Option Nat
  deriving DecidableEq, Repr

/-- What the stamps decide. -/
inductive Outcome where
  | inForce (text : Nat)
  | conflict
  deriving DecidableEq, Repr

/-- Two candidate lists with the same members. -/
def Equiv (a b : List Cand) : Prop := ∀ c, c ∈ a ↔ c ∈ b

/-- A candidate's stamp, `0` when unusable (only read when all are usable). -/
def Cand.key (c : Cand) : Nat := c.stamp.getD 0

/-- Every candidate carries a usable stamp. -/
def allStamped (l : List Cand) : Bool := l.all (fun c => c.stamp.isSome)

/-- The greatest stamp. -/
def top (l : List Cand) : Nat := l.foldr (fun c m => max c.key m) 0

/-- The candidates with the greatest stamp. -/
def newest (l : List Cand) : List Cand := l.filter (fun c => c.stamp == some (top l))

/-- The newest candidates agree on their text. -/
def agreed (l : List Cand) : Bool :=
  (newest l).all fun c => (newest l).all fun d => c.text == d.text

/-- The decision. -/
def decide (l : List Cand) : Outcome :=
  if allStamped l then
    match newest l with
    | [] => .conflict
    | c :: _ => if agreed l then .inForce c.text else .conflict
  else .conflict

/-- What is retained beside the decision: every candidate on a conflict, and
every candidate whose text is not the one in force otherwise. -/
def record (l : List Cand) : List Cand :=
  match decide l with
  | .conflict => l
  | .inForce t => l.filter (fun c => c.text != t)

/-- The verdict a call relying on a decided function contributes when no text
is in force: the held call is `Unknown`. -/
def heldVerdict : Outcome → Option V
  | .conflict => some .unknown
  | .inForce _ => none

/-! ## The greatest stamp reads only the set -/

theorem le_top {l : List Cand} {c : Cand} (h : c ∈ l) : c.key ≤ top l := by
  induction l with
  | nil => cases h
  | cons x xs ih =>
    simp only [top, List.foldr_cons] at *
    rcases List.mem_cons.mp h with rfl | m
    · exact Nat.le_max_left _ _
    · exact Nat.le_trans (ih m) (Nat.le_max_right _ _)

theorem top_attained (l : List Cand) : top l = 0 ∨ ∃ c ∈ l, c.key = top l := by
  induction l with
  | nil => exact Or.inl rfl
  | cons x xs ih =>
    simp only [top, List.foldr_cons] at *
    by_cases hx : x.key ≤ xs.foldr (fun c m => max c.key m) 0
    · rw [Nat.max_eq_right hx]
      rcases ih with h | ⟨c, m, e⟩
      · exact Or.inl h
      · exact Or.inr ⟨c, List.mem_cons_of_mem x m, e⟩
    · rw [Nat.max_eq_left (Nat.le_of_lt (Nat.lt_of_not_le hx))]
      exact Or.inr ⟨x, List.mem_cons_self, rfl⟩

theorem top_congr {a b : List Cand} (same : Equiv a b) : top a = top b := by
  apply Nat.le_antisymm
  · rcases top_attained a with h | ⟨c, m, e⟩
    · rw [h]; exact Nat.zero_le _
    · rw [← e]; exact le_top ((same c).mp m)
  · rcases top_attained b with h | ⟨c, m, e⟩
    · rw [h]; exact Nat.zero_le _
    · rw [← e]; exact le_top ((same c).mpr m)

theorem mem_newest {l : List Cand} {c : Cand} :
    c ∈ newest l ↔ c ∈ l ∧ c.stamp = some (top l) := by
  simp [newest]

theorem allStamped_iff {l : List Cand} : allStamped l = true ↔ ∀ c ∈ l, c.stamp.isSome := by
  simp [allStamped]

theorem agreed_iff {l : List Cand} :
    agreed l = true ↔ ∀ c ∈ newest l, ∀ d ∈ newest l, c.text = d.text := by
  simp [agreed]

theorem newest_congr {a b : List Cand} (same : Equiv a b) (c : Cand) :
    c ∈ newest a ↔ c ∈ newest b := by
  simp only [mem_newest, top_congr same]
  exact and_congr_left' (same c)

theorem allStamped_congr {a b : List Cand} (same : Equiv a b) :
    allStamped a = allStamped b := by
  apply Bool.eq_iff_iff.mpr
  simp only [allStamped_iff]
  exact ⟨fun h c m => h c ((same c).mpr m), fun h c m => h c ((same c).mp m)⟩

theorem agreed_congr {a b : List Cand} (same : Equiv a b) : agreed a = agreed b := by
  apply Bool.eq_iff_iff.mpr
  simp only [agreed_iff]
  constructor
  · intro h c mc d md
    exact h c ((newest_congr same c).mpr mc) d ((newest_congr same d).mpr md)
  · intro h c mc d md
    exact h c ((newest_congr same c).mp mc) d ((newest_congr same d).mp md)

/-- With every candidate stamped and at least one, some candidate is newest. -/
theorem newest_nonempty {l : List Cand} (stamped : allStamped l = true) (c : Cand)
    (m : c ∈ l) : newest l ≠ [] := by
  intro empty
  rcases top_attained l with h | ⟨d, md, e⟩
  · have hs := (allStamped_iff.mp stamped) c m
    match hc : c.stamp, hs with
    | some n, _ =>
      have hle : c.key ≤ top l := le_top m
      simp only [Cand.key, hc, Option.getD_some, h] at hle
      have : c ∈ newest l := mem_newest.mpr ⟨m, by rw [hc, h, Nat.le_zero.mp hle]⟩
      rw [empty] at this; cases this
  · have hs := (allStamped_iff.mp stamped) d md
    match hd : d.stamp, hs with
    | some n, _ =>
      have : d ∈ newest l := mem_newest.mpr ⟨md, by
        simp only [Cand.key, hd, Option.getD_some] at e; rw [hd, e]⟩
      rw [empty] at this; cases this

/-! ## Determinism -/

theorem decide_congr {a b : List Cand} (same : Equiv a b) : decide a = decide b := by
  unfold decide
  rw [allStamped_congr same, agreed_congr same]
  split
  · cases ha : newest a with
    | nil =>
      cases hb : newest b with
      | nil => rfl
      | cons d _ =>
        have : d ∈ newest a := (newest_congr same d).mpr (by rw [hb]; exact List.mem_cons_self)
        rw [ha] at this; cases this
    | cons c _ =>
      cases hb : newest b with
      | nil =>
        have : c ∈ newest b := (newest_congr same c).mp (by rw [ha]; exact List.mem_cons_self)
        rw [hb] at this; cases this
      | cons d _ =>
        simp only
        split
        · rename_i ok
          have hc : c ∈ newest b :=
            (newest_congr same c).mp (by rw [ha]; exact List.mem_cons_self)
          have hd : d ∈ newest b := by rw [hb]; exact List.mem_cons_self
          rw [agreed_iff.mp ok c hc d hd]
        · rfl
  · rfl

theorem decide_union_comm (a b : List Cand) : decide (a ++ b) = decide (b ++ a) :=
  decide_congr (fun c => by simp only [List.mem_append]; exact Or.comm)

theorem decide_union_assoc (a b c : List Cand) :
    decide ((a ++ b) ++ c) = decide (a ++ (b ++ c)) := by
  rw [List.append_assoc]

theorem decide_duplicate {l : List Cand} {c : Cand} (h : c ∈ l) : decide (l ++ [c]) = decide l :=
  decide_congr (fun d => by
    simp only [List.mem_append, List.mem_singleton]
    constructor
    · rintro (m | rfl)
      · exact m
      · exact h
    · exact Or.inl)

/-! ## Newest wins; a tie is a conflict -/

theorem newest_wins {l : List Cand} {c : Cand} (m : c ∈ l) (stamped : allStamped l = true)
    (most : ∀ d ∈ l, d.key ≤ c.key)
    (ties : ∀ d ∈ l, d.stamp = c.stamp → d.text = c.text) :
    decide l = .inForce c.text := by
  have hs := (allStamped_iff.mp stamped) c m
  obtain ⟨n, hc⟩ : ∃ n, c.stamp = some n := Option.isSome_iff_exists.mp hs
  have htop : top l = n := by
    apply Nat.le_antisymm
    · rcases top_attained l with h | ⟨d, md, e⟩
      · rw [h]; exact Nat.zero_le _
      · rw [← e]; have := most d md; simpa [Cand.key, hc] using this
    · have := le_top m; simpa [Cand.key, hc] using this
  have newest_text : ∀ d ∈ newest l, d.text = c.text := by
    intro d md
    obtain ⟨dl, ds⟩ := mem_newest.mp md
    exact ties d dl (by rw [ds, hc, htop])
  have ok : agreed l = true :=
    agreed_iff.mpr (fun x mx y my => by rw [newest_text x mx, newest_text y my])
  unfold decide
  simp only [stamped, ↓reduceIte]
  match hn : newest l with
  | [] => exact absurd hn (newest_nonempty stamped c m)
  | d :: _ =>
    simp only [ok, ↓reduceIte]
    rw [newest_text d (by rw [hn]; exact List.mem_cons_self)]

theorem strictly_newest_wins {l : List Cand} {c : Cand} (m : c ∈ l)
    (stamped : allStamped l = true) (strict : ∀ d ∈ l, d ≠ c → d.key < c.key) :
    decide l = .inForce c.text := by
  apply newest_wins m stamped
  · intro d md
    by_cases e : d = c
    · rw [e]; exact Nat.le_refl _
    · exact Nat.le_of_lt (strict d md e)
  · intro d md ds
    by_cases e : d = c
    · rw [e]
    · have := strict d md e
      simp [Cand.key, ds] at this

theorem a_tie_is_a_conflict {l : List Cand} {c d : Cand} (mc : c ∈ l) (md : d ∈ l)
    (same : c.stamp = d.stamp) (most : ∀ e ∈ l, e.key ≤ c.key) (differ : c.text ≠ d.text) :
    decide l = .conflict := by
  unfold decide
  split
  · rename_i stamped
    have hs := (allStamped_iff.mp stamped) c mc
    obtain ⟨n, hc⟩ : ∃ n, c.stamp = some n := Option.isSome_iff_exists.mp hs
    have htop : top l = n := by
      apply Nat.le_antisymm
      · rcases top_attained l with h | ⟨e, me, k⟩
        · rw [h]; exact Nat.zero_le _
        · rw [← k]; have := most e me; simpa [Cand.key, hc] using this
      · have := le_top mc; simpa [Cand.key, hc] using this
    have nc : c ∈ newest l := mem_newest.mpr ⟨mc, by rw [hc, htop]⟩
    have nd : d ∈ newest l := mem_newest.mpr ⟨md, by rw [← same, hc, htop]⟩
    have notok : agreed l = false := by
      cases h : agreed l with
      | false => rfl
      | true => exact absurd ((agreed_iff.mp h) c nc d nd) differ
    match newest l with
    | [] => rfl
    | _ :: _ => simp [notok]
  · rfl

theorem an_unusable_stamp_is_a_conflict {l : List Cand} {c : Cand} (m : c ∈ l)
    (unusable : c.stamp = none) : decide l = .conflict := by
  unfold decide
  have : allStamped l = false := by
    cases h : allStamped l with
    | false => rfl
    | true => have := (allStamped_iff.mp h) c m; rw [unusable] at this; cases this
  rw [this]; rfl

theorem nothing_is_a_conflict : decide [] = .conflict := by
  simp [decide, allStamped, newest]

/-! ## Retention -/

theorem in_force_is_a_newest_candidate {l : List Cand} {t : Nat} (h : decide l = .inForce t) :
    ∃ c ∈ l, c.text = t ∧ c.stamp = some (top l) := by
  unfold decide at h
  split at h
  · match hn : newest l, h with
    | [], h => simp at h
    | c :: _, h =>
      simp only at h
      split at h
      · cases h
        have := mem_newest.mp (by rw [hn]; exact List.mem_cons_self)
        exact ⟨c, this.1, rfl, this.2⟩
      · cases h
  · cases h

theorem nothing_is_erased (l : List Cand) :
    ∀ c ∈ l, c ∈ record l ∨ decide l = .inForce c.text := by
  intro c m
  unfold record
  split
  · exact Or.inl m
  · rename_i t ht
    by_cases e : c.text = t
    · exact Or.inr (by rw [ht, e])
    · exact Or.inl (List.mem_filter.mpr ⟨m, by simp [e]⟩)

theorem the_record_holds_only_candidates (l : List Cand) : ∀ c ∈ record l, c ∈ l := by
  intro c m
  unfold record at m
  split at m
  · exact m
  · exact (List.mem_filter.mp m).1

/-! ## Never optimistic -/

theorem a_conflict_is_never_safe (v : V) : V.join v .unknown ≠ .safe := by
  cases v <;> decide

theorem a_conflict_is_never_pending (v : V) : V.join v .unknown ≠ .pending := by
  cases v <;> decide

theorem a_conflict_never_lowers (v : V) : v.rank ≤ (V.join v .unknown).rank :=
  le_join_left v .unknown

theorem the_held_verdict_of_a_conflict : heldVerdict .conflict = some .unknown := rfl

/-! ## Shared vectors

Each vector is a candidate list and the expected outcome. The same cases, in
the same order, are checked against the Rust `decide` by
`crates/qcore-schemas/tests/contract_supersession.rs`. -/

structure Vector where
  cands : List Cand
  expected : Outcome
  deriving Repr

def Vector.holds (v : Vector) : Bool := decide v.cands == v.expected

def vectors : List Vector := [
  -- the newest stamp wins, in either order
  ⟨[⟨1, 10, some 1⟩, ⟨2, 20, some 2⟩], .inForce 20⟩,
  ⟨[⟨2, 20, some 2⟩, ⟨1, 10, some 1⟩], .inForce 20⟩,
  -- equal stamps on different texts conflict
  ⟨[⟨1, 10, some 1⟩, ⟨2, 20, some 1⟩], .conflict⟩,
  -- byte-identical texts are idempotent
  ⟨[⟨1, 10, some 1⟩, ⟨2, 10, some 1⟩], .inForce 10⟩,
  -- an unusable stamp withholds, even beside an older stamped one
  ⟨[⟨1, 10, some 2⟩, ⟨2, 20, none⟩], .conflict⟩,
  ⟨[⟨1, 10, none⟩, ⟨2, 20, none⟩], .conflict⟩,
  -- three bundles: the newest of three
  ⟨[⟨1, 10, some 3⟩, ⟨2, 20, some 1⟩, ⟨3, 30, some 2⟩], .inForce 10⟩,
  -- a tie at the top beside an older one
  ⟨[⟨1, 10, some 1⟩, ⟨2, 20, some 3⟩, ⟨3, 30, some 3⟩], .conflict⟩
]

theorem vectors_hold : vectors.all Vector.holds = true := by decide

end QCore.ContractSupersession

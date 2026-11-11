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

import QCore.IndexExtent

/-! Quantified index checks of the bounds property (card QUANT-INDEX-CHECKS,
contract specification C09.2, gateway rule 12l), extending `QCore.IndexExtent`.

The decisions of `crates/qcore-core/src/index_quant.rs`, transcribed:

* the **predicate language**: comparisons `==`, `<`, `>`, `<=`, `>=` of the one
  universally bound index against a bound term (a constant plus at most one
  formal), joined by `&&` and `||`, read over the mathematical integers under
  a valuation `ρ` of the formals;
* its **interval reading** (disjunctive normal form, `||` as union, `&&` as
  pairwise intersection), exact: an index is in one of the intervals exactly
  when the predicate holds of it (`mem_reading`). `||` is read literally;
* the **admitted set**: empty, unbounded, or bounded with a hull. Only a
  bounded set gives the caller an obligation and the body a fact;
* the **body-side decision** of an access's index fact against the admitted
  set (a constant union, or the symbolic prefix `[0, s)`): inside only when
  every value is admitted, outside only when none is;
* the **call-side decision**: the element accesses at the hull's two ends,
  joined; covering both covers every admitted index;
* the **escape decision** of a callee's set shifted into the caller's.

What this is not: a model of C, of Clang's extraction or of the declared
bodies; the C index value enters through the source consumer's reading. The
Rust correspondence is bound by `admittedVectors`, `decideVectors` and
`shiftVectors` (`crates/qcore-core/tests/index_quant.rs`). -/
namespace QCore.IndexQuant

open QCore.IndexExtent

inductive Rel where
  | eq
  | lt
  | gt
  | le
  | ge
  deriving DecidableEq, Repr

/-- A bound term: a constant plus, when named, one formal (coefficient one). -/
inductive Term where
  | mk (const : Int) (formal : Option Nat)
  deriving DecidableEq, Repr

def Term.eval (ρ : Nat → Int) : Term → Int
  | .mk c none => c
  | .mk c (some f) => ρ f + c

inductive Pred where
  | cmp (r : Rel) (t : Term)
  | and (p q : Pred)
  | or (p q : Pred)
  deriving DecidableEq, Repr

def Rel.holds : Rel → Int → Int → Bool
  | .eq, i, t => decide (i = t)
  | .lt, i, t => decide (i < t)
  | .gt, i, t => decide (t < i)
  | .le, i, t => decide (i ≤ t)
  | .ge, i, t => decide (t ≤ i)

/-- The predicate, for one index, under the valuation `ρ`. -/
def Pred.holds (ρ : Nat → Int) : Pred → Int → Bool
  | .cmp r t, i => r.holds i (t.eval ρ)
  | .and p q, i => p.holds ρ i && q.holds ρ i
  | .or p q, i => p.holds ρ i || q.holds ρ i

/-- An integer interval, both ends inclusive; `none` is unbounded. -/
structure Ival where
  lo : Option Int
  hi : Option Int
  deriving DecidableEq, Repr

def loOk : Option Int → Int → Bool
  | none, _ => true
  | some l, i => decide (l ≤ i)

def hiOk : Option Int → Int → Bool
  | none, _ => true
  | some h, i => decide (i ≤ h)

def Ival.mem (v : Ival) (i : Int) : Bool := loOk v.lo i && hiOk v.hi i

def Rel.ival : Rel → Int → Ival
  | .eq, t => .mk (some t) (some t)
  | .lt, t => .mk none (some (t - 1))
  | .le, t => .mk none (some t)
  | .gt, t => .mk (some (t + 1)) none
  | .ge, t => .mk (some t) none

def maxLo : Option Int → Option Int → Option Int
  | some a, some b => some (max a b)
  | a, none => a
  | none, b => b

def minHi : Option Int → Option Int → Option Int
  | some a, some b => some (min a b)
  | a, none => a
  | none, b => b

def Ival.inter (a b : Ival) : Ival := .mk (maxLo a.lo b.lo) (minHi a.hi b.hi)

/-- The interval reading: disjunctive normal form, empty intervals kept. -/
def reading (ρ : Nat → Int) : Pred → List Ival
  | .cmp r t => [r.ival (t.eval ρ)]
  | .and p q => (reading ρ p).flatMap (fun a => (reading ρ q).map (fun b => a.inter b))
  | .or p q => reading ρ p ++ reading ρ q

theorem rel_ival_mem (r : Rel) (t i : Int) : (r.ival t).mem i = r.holds i t := by
  apply Bool.eq_iff_iff.mpr
  cases r <;> simp [Rel.ival, Ival.mem, loOk, hiOk, Rel.holds] <;> omega

theorem lo_ok_max (a b : Option Int) (i : Int) :
    loOk (maxLo a b) i = (loOk a i && loOk b i) := by
  apply Bool.eq_iff_iff.mpr
  cases a <;> cases b <;> simp [maxLo, loOk] <;> omega

theorem hi_ok_min (a b : Option Int) (i : Int) :
    hiOk (minHi a b) i = (hiOk a i && hiOk b i) := by
  apply Bool.eq_iff_iff.mpr
  cases a <;> cases b <;> simp [minHi, hiOk] <;> omega

theorem inter_mem (a b : Ival) (i : Int) : (a.inter b).mem i = (a.mem i && b.mem i) := by
  rcases a with ⟨a1, a2⟩
  rcases b with ⟨b1, b2⟩
  simp only [Ival.inter, Ival.mem, lo_ok_max, hi_ok_min]
  cases loOk a1 i <;> cases loOk b1 i <;> cases hiOk a2 i <;> cases hiOk b2 i <;> rfl

theorem any_and_left (ys : List Ival) (x : Bool) (g : Ival → Bool) :
    ys.any (fun b => x && g b) = (x && ys.any g) := by
  induction ys with
  | nil => cases x <;> rfl
  | cons y ys ih =>
      simp only [List.any_cons, ih]
      cases x <;> cases g y <;> simp

theorem any_and_right (xs : List Ival) (y : Bool) (f : Ival → Bool) :
    xs.any (fun a => f a && y) = (xs.any f && y) := by
  induction xs with
  | nil => cases y <;> rfl
  | cons x xs ih =>
      simp only [List.any_cons, ih]
      cases y <;> cases f x <;> simp

/-- The reading is exact: sound (every index in an interval satisfies the
predicate) and complete (every index satisfying it is in an interval). -/
theorem mem_reading (ρ : Nat → Int) (p : Pred) (i : Int) :
    (reading ρ p).any (fun v => v.mem i) = p.holds ρ i := by
  induction p with
  | cmp r t => simp [reading, Pred.holds, rel_ival_mem]
  | and p q ihp ihq =>
      simp only [reading, Pred.holds, List.any_flatMap, List.any_map, Function.comp_def,
        inter_mem, any_and_left]
      rw [← ihp, ← ihq, any_and_right]
  | or p q ihp ihq =>
      simp [reading, Pred.holds, List.any_append, ihp, ihq]

theorem reading_sound (ρ : Nat → Int) (p : Pred) (v : Ival) (i : Int)
    (hv : v ∈ reading ρ p) (hi : v.mem i = true) : p.holds ρ i = true := by
  rw [← mem_reading]
  exact List.any_eq_true.mpr ⟨v, hv, hi⟩

theorem reading_complete (ρ : Nat → Int) (p : Pred) (i : Int) (h : p.holds ρ i = true) :
    ∃ v ∈ reading ρ p, v.mem i = true := by
  rw [← mem_reading] at h
  exact List.any_eq_true.mp h

/-- `||` is read literally: the union of the two readings, never narrowed. -/
theorem or_reading_literal (ρ : Nat → Int) (p q : Pred) :
    reading ρ (.or p q) = reading ρ p ++ reading ρ q := by
  simp [reading]

theorem or_holds_literal (ρ : Nat → Int) (p q : Pred) (i : Int) :
    (Pred.or p q).holds ρ i = (p.holds ρ i || q.holds ρ i) := by
  simp [Pred.holds]

/-- The user's spelling `#i < u || l < #i` with `l < u` admits every index. -/
theorem over_wide_disjunction_admits_every_index (ρ : Nat → Int) (u l : Int) (h : l < u)
    (i : Int) : (Pred.or (.cmp .lt (.mk u none)) (.cmp .gt (.mk l none))).holds ρ i = true := by
  simp only [Pred.holds, Rel.holds, Term.eval]
  by_cases hi : i < u
  · simp [hi]
  · have hl : l < i := by omega
    simp [hl]

/-! The admitted set. -/

def Ival.nonempty (v : Ival) : Bool :=
  match v.lo, v.hi with
  | some l, some h => decide (l ≤ h)
  | _, _ => true

def Ival.closed (v : Ival) : Bool :=
  match v.lo, v.hi with
  | some _, some _ => true
  | _, _ => false

inductive Admitted where
  | empty
  | unbounded
  | bounded (lo hi : Int)
  deriving DecidableEq, Repr

/-- The hull of closed intervals (`lo`, `hi` of the first, widened). -/
def hull : List Ival → Int × Int
  | [] => (0, 0)
  | [v] => ((v.lo).getD 0, (v.hi).getD 0)
  | v :: rest =>
      let (l, h) := hull rest
      (min ((v.lo).getD 0) l, max ((v.hi).getD 0) h)

def admitted (vs : List Ival) : Admitted :=
  let ne := vs.filter Ival.nonempty
  if ne.any (fun v => !v.closed) then .unbounded
  else if ne = [] then .empty
  else
    let (l, h) := hull ne
    .bounded l h

theorem mem_nonempty (v : Ival) (i : Int) (h : v.mem i = true) : v.nonempty = true := by
  rcases v with ⟨lo, hi⟩
  cases lo <;> cases hi <;> simp_all [Ival.mem, loOk, hiOk, Ival.nonempty]
  omega

theorem closed_mem_le (v : Ival) (i : Int) (hc : v.closed = true) (h : v.mem i = true) :
    (v.lo).getD 0 ≤ i ∧ i ≤ (v.hi).getD 0 := by
  rcases v with ⟨lo, hi⟩
  cases lo <;> cases hi <;> simp_all [Ival.closed, Ival.mem, loOk, hiOk]

theorem hull_bounds (vs : List Ival) (v : Ival) (hv : v ∈ vs) :
    (hull vs).1 ≤ (v.lo).getD 0 ∧ (v.hi).getD 0 ≤ (hull vs).2 := by
  induction vs with
  | nil => simp at hv
  | cons w rest ih =>
      cases rest with
      | nil =>
          simp at hv
          subst hv
          simp [hull]
      | cons x xs =>
          simp only [List.mem_cons] at hv
          rcases hv with h | h
          · subst h
            simp only [hull]
            omega
          · have := ih (by simpa using h)
            simp only [hull] at this ⊢
            omega

/-- Every admitted index lies in the hull: a caller covering the hull covers
every index the predicate admits. -/
theorem bounded_hull_sound (ρ : Nat → Int) (p : Pred) (lo hi i : Int)
    (hb : admitted (reading ρ p) = .bounded lo hi) (h : p.holds ρ i = true) :
    lo ≤ i ∧ i ≤ hi := by
  obtain ⟨v, hv, hm⟩ := reading_complete ρ p i h
  unfold admitted at hb
  have hne : v ∈ (reading ρ p).filter Ival.nonempty :=
    List.mem_filter.mpr ⟨hv, mem_nonempty v i hm⟩
  generalize hl : (reading ρ p).filter Ival.nonempty = ne at hb hne
  by_cases hu : ne.any (fun v => !v.closed) = true
  · simp [hu] at hb
  · have hc : v.closed = true := by
      cases hcv : v.closed
      · exact absurd (List.any_eq_true.mpr ⟨v, hne, by simp [hcv]⟩) hu
      · rfl
    by_cases he : ne = []
    · subst he
      simp at hne
    · simp only [hu, he] at hb
      simp at hb
      obtain ⟨h1, h2⟩ := hull_bounds ne v hne
      obtain ⟨h3, h4⟩ := closed_mem_le v i hc hm
      rw [← hb.1, ← hb.2]
      exact ⟨by omega, by omega⟩

/-- An empty admitted set admits no index: it gives no fact (and the loader
rejects a constant one). -/
theorem empty_admits_nothing (ρ : Nat → Int) (p : Pred) (i : Int)
    (he : admitted (reading ρ p) = .empty) : p.holds ρ i = false := by
  cases h : p.holds ρ i
  · rfl
  · obtain ⟨v, hv, hm⟩ := reading_complete ρ p i h
    have hne : v ∈ (reading ρ p).filter Ival.nonempty :=
      List.mem_filter.mpr ⟨hv, mem_nonempty v i hm⟩
    unfold admitted at he
    generalize (reading ρ p).filter Ival.nonempty = ne at he hne
    by_cases hu : ne.any (fun v => !v.closed) = true
    · simp [hu] at he
    · by_cases hn : ne = []
      · subst hn
        simp at hne
      · simp [hu, hn] at he

/-- An unbounded admitted set admits an index beyond every bound: it gives no
fact (and the loader rejects it). -/
theorem unbounded_admits_beyond_every_bound (ρ : Nat → Int) (p : Pred) (N : Int)
    (hu : admitted (reading ρ p) = .unbounded) :
    ∃ i, p.holds ρ i = true ∧ (N < i ∨ i < -N) := by
  unfold admitted at hu
  generalize hl : (reading ρ p).filter Ival.nonempty = ne at hu
  by_cases hany : ne.any (fun v => !v.closed) = true
  · obtain ⟨v, hv, hc⟩ := List.any_eq_true.mp hany
    rw [← hl] at hv
    obtain ⟨hr, hn⟩ := List.mem_filter.mp hv
    rcases v with ⟨lo, hi⟩
    cases hi with
    | none =>
        refine ⟨max (lo.getD 0) (N + 1), ?_, Or.inl (Int.lt_of_lt_of_le (Int.lt_succ N)
          (Int.le_max_right _ _))⟩
        apply reading_sound ρ p _ _ hr
        cases lo with
        | none => simp [Ival.mem, loOk, hiOk]
        | some l =>
            simp only [Ival.mem, loOk, hiOk, Option.getD_some, Bool.and_true]
            exact decide_eq_true (Int.le_max_left _ _)
    | some h =>
        cases lo with
        | none =>
            refine ⟨min h (-N - 1), ?_, Or.inr (by omega)⟩
            apply reading_sound ρ p _ _ hr
            simp only [Ival.mem, loOk, hiOk, Bool.true_and]
            exact decide_eq_true (Int.min_le_left _ _)
        | some l => simp [Ival.closed] at hc
  · by_cases hn : ne = []
    · simp [hn] at hu
    · simp [hn] at hu
      obtain ⟨x, hx, hc⟩ := hu
      exact absurd (List.any_eq_true.mpr ⟨x, hx, by simp [hc]⟩) hany

/-- The caller's obligation exists only for a bounded set. -/
def requirement (a : Admitted) : Option (Int × Int) :=
  match a with
  | .bounded lo hi => some (lo, hi)
  | _ => none

theorem no_obligation_without_a_bounded_set (a : Admitted)
    (h : a = .empty ∨ a = .unbounded) : requirement a = none := by
  rcases h with h | h <;> subst h <;> rfl

/-! The body-side decision. -/

inductive QSet where
  /-- A bounded, nonempty union of constant inclusive intervals. -/
  | constant (intervals : List (Int × Int))
  /-- `[0, s)` for the entry value of a stable formal `s`. -/
  | pre (s : Nat)
  deriving DecidableEq, Repr

inductive QDecision where
  | inside
  | outside
  | undecided (r : Refusal)
  deriving DecidableEq, Repr

def QSet.admits (env : Nat → Nat) : QSet → Int → Bool
  | .constant intervals, v => intervals.any (fun (a, b) => decide (a ≤ v ∧ v ≤ b))
  | .pre s, v => decide (0 ≤ v ∧ v < (env s : Int))

def covered (intervals : List (Int × Int)) (lo hi : Int) : Bool :=
  intervals.any (fun (a, b) => decide (a ≤ lo ∧ hi ≤ b))

def disjoint (intervals : List (Int × Int)) (lo hi : Int) : Bool :=
  intervals.all (fun (a, b) => decide (hi < a ∨ b < lo))

def indexAdmitted : QSet → IndexFact → QDecision
  | _, .unknown => .undecided .unsupportedRange
  | .constant s, .const v => if covered s v v then .inside else .outside
  | .pre _, .const v => if v < 0 then .outside else .undecided .symbolMismatch
  | set, .range lo (.const h) =>
      if (h : Int) ≤ lo then .undecided .emptyRange
      else match set with
        | .constant s =>
            if covered s lo (h - 1) then .inside
            else if disjoint s lo (h - 1) then .outside
            else .undecided .rangeNotContained
        | .pre _ => if (h : Int) - 1 < 0 then .outside else .undecided .symbolMismatch
  | .pre n, .range lo (.sym s) =>
      if s = n then (if lo < 0 then .undecided .rangeNotContained else .inside)
      else .undecided .symbolMismatch
  | .constant _, .range _ (.sym _) => .undecided .symbolMismatch

theorem covered_sound (s : List (Int × Int)) (lo hi v : Int) (h : covered s lo hi = true)
    (h1 : lo ≤ v) (h2 : v ≤ hi) : s.any (fun (a, b) => decide (a ≤ v ∧ v ≤ b)) = true := by
  obtain ⟨⟨a, b⟩, hm, hc⟩ := List.any_eq_true.mp h
  exact List.any_eq_true.mpr ⟨(a, b), hm, by simp at hc ⊢; omega⟩

theorem disjoint_sound (s : List (Int × Int)) (lo hi v : Int) (h : disjoint s lo hi = true)
    (h1 : lo ≤ v) (h2 : v ≤ hi) : s.any (fun (a, b) => decide (a ≤ v ∧ v ≤ b)) = false := by
  rw [List.any_eq_false]
  intro ⟨a, b⟩ hm
  have := List.all_eq_true.mp h (a, b) hm
  simp at this ⊢
  omega

/-- Inside only when every value the index fact describes is admitted. -/
theorem inside_sound (env : Nat → Nat) (set : QSet) (f : IndexFact) (v : Int)
    (h : indexAdmitted set f = .inside) (hv : Describes env f v) : set.admits env v = true := by
  cases f with
  | unknown => cases set <;> simp [indexAdmitted] at h
  | const c =>
      simp [Describes] at hv
      subst hv
      cases set with
      | constant s =>
          by_cases hc : covered s v v = true
          · exact covered_sound s v v v hc (Int.le_refl v) (Int.le_refl v)
          · simp [indexAdmitted, hc] at h
      | pre n =>
          by_cases hn : v < 0 <;> simp [indexAdmitted, hn] at h
  | range lo hi =>
      cases hi with
      | const h' =>
          simp [Describes, Count.eval] at hv
          by_cases he : (h' : Int) ≤ lo
          · cases set <;> simp [indexAdmitted, he] at h
          · cases set with
            | constant s =>
                by_cases hc : covered s lo (h' - 1) = true
                · exact covered_sound s lo (h' - 1) v hc hv.1 (by omega)
                · by_cases hd : disjoint s lo (h' - 1) = true <;>
                    simp [indexAdmitted, he, hc, hd] at h
            | pre n =>
                by_cases hn : (h' : Int) - 1 < 0 <;> simp [indexAdmitted, he, hn] at h
      | sym s =>
          simp [Describes, Count.eval] at hv
          cases set with
          | constant _ => simp [indexAdmitted] at h
          | pre n =>
              by_cases hs : s = n
              · subst hs
                by_cases hl : lo < 0
                · simp [indexAdmitted, hl] at h
                · simp [QSet.admits]
                  omega
              · simp [indexAdmitted, hs] at h

/-- Outside only when no value the index fact describes is admitted (the body
then refutes its declaration). -/
theorem outside_sound (env : Nat → Nat) (set : QSet) (f : IndexFact) (v : Int)
    (h : indexAdmitted set f = .outside) (hv : Describes env f v) : set.admits env v = false := by
  cases f with
  | unknown => cases set <;> simp [indexAdmitted] at h
  | const c =>
      simp [Describes] at hv
      subst hv
      cases set with
      | constant s =>
          by_cases hc : covered s v v = true
          · simp [indexAdmitted, hc] at h
          · simp [QSet.admits]
            intro a b hm h1
            exact Classical.byContradiction fun hlt =>
              hc (List.any_eq_true.mpr ⟨(a, b), hm, by simp; omega⟩)
      | pre n =>
          by_cases hn : v < 0
          · simp [QSet.admits]
            omega
          · simp [indexAdmitted, hn] at h
  | range lo hi =>
      cases hi with
      | const h' =>
          simp [Describes, Count.eval] at hv
          by_cases he : (h' : Int) ≤ lo
          · cases set <;> simp [indexAdmitted, he] at h
          · cases set with
            | constant s =>
                by_cases hc : covered s lo (h' - 1) = true
                · simp [indexAdmitted, he, hc] at h
                · by_cases hd : disjoint s lo (h' - 1) = true
                  · exact disjoint_sound s lo (h' - 1) v hd hv.1 (by omega)
                  · simp [indexAdmitted, he, hc, hd] at h
            | pre n =>
                by_cases hn : (h' : Int) - 1 < 0
                · simp [QSet.admits]
                  omega
                · simp [indexAdmitted, he, hn] at h
      | sym s =>
          cases set with
          | constant _ => simp [indexAdmitted] at h
          | pre n =>
              by_cases hs : s = n
              · subst hs
                by_cases hl : lo < 0 <;> simp [indexAdmitted, hl] at h
              · simp [indexAdmitted, hs] at h

/-- An undecidable index is never inside. -/
theorem an_unknown_index_is_never_inside (set : QSet) :
    indexAdmitted set .unknown = .undecided .unsupportedRange := by
  cases set <;> rfl

/-! Outcomes: a refutation or an undecided index is an unresolved judgement. -/

/-- The judgement a decision records: `none` when inside (the extent decision
follows), otherwise an unresolved reason (`1` refuted, `0` undecided). -/
def quantOutcome : QDecision → Option Outcome
  | .inside => none
  | .outside => some (.unresolved 1)
  | .undecided _ => some (.unresolved 0)

/-- A function with a refuted or undecided quant judgement is never `Safe`. -/
theorem quant_refusal_never_safe (js : List (Outcome × Outcome)) (r : Nat) (d : QDecision)
    (o : Outcome) (hd : quantOutcome d = some o) (hj : (o, o) ∈ js) :
    boundsVerdict js r ≠ .safe := by
  intro h
  have := (safe_only_when_every_obligation_discharges js r h).2 (o, o) hj
  cases d <;> simp [quantOutcome] at hd <;> subst hd <;> simp [Outcome.clear] at this

/-- A quant judgement never accuses: it is never a violation. -/
theorem quant_refusal_never_accuses (d : QDecision) (o : Outcome)
    (hd : quantOutcome d = some o) : o ≠ .violation true := by
  cases d <;> simp [quantOutcome] at hd <;> subst hd <;> simp

/-! The call side: the hull's two ends, joined. -/

def bothInBounds : Decision → Decision → Decision
  | .inBounds, .inBounds => .inBounds
  | .outOfBounds, _ => .outOfBounds
  | _, .outOfBounds => .outOfBounds
  | .undecided r, _ => .undecided r
  | _, .undecided r => .undecided r

theorem both_in_bounds_sound (a b : Decision) (h : bothInBounds a b = .inBounds) :
    a = .inBounds ∧ b = .inBounds := by
  cases a <;> cases b <;> simp [bothInBounds] at h ⊢

/-- Covering the hull's two ends covers every admitted index: the element
accesses at `o + lo` and `o + hi` in bounds put every `o + k`, `lo ≤ k ≤ hi`,
in bounds. -/
theorem covering_the_ends_covers_the_hull (env : Nat → Nat) (n : Count) (o lo hi k : Int)
    (hl : elementAccess (some n) (.const (o + lo)) = .inBounds)
    (hh : elementAccess (some n) (.const (o + hi)) = .inBounds)
    (h1 : lo ≤ k) (h2 : k ≤ hi) : 0 ≤ o + k ∧ o + k < (n.eval env : Int) := by
  have a := element_access_sound env n _ (o + lo) hl (by simp [Describes])
  have b := element_access_sound env n _ (o + hi) hh (by simp [Describes])
  omega

/-! The escape: a callee's constant set shifted into the caller's. -/

def shiftOne (outer : QSet) (k : Int) (ab : Int × Int) : QDecision :=
  if ab.1 ≤ ab.2 then indexAdmitted outer (.range (ab.1 + k) (.const (ab.2 + k + 1).toNat))
  else .undecided .emptyRange

/-- Inside when every interval is, outside when every one is, otherwise the
first undecided interval's refusal (or a mixed `rangeNotContained`). -/
def combine (ds : List QDecision) : QDecision :=
  if ds.all (· == .inside) then .inside
  else if ds.all (· == .outside) then .outside
  else match ds.find? (fun d => d != .inside && d != .outside) with
    | some d => d
    | none => .undecided .rangeNotContained

def shiftedWithin (outer : QSet) (inner : List (Int × Int)) (k : Int) : QDecision :=
  combine (inner.map (shiftOne outer k))

theorem combine_inside (ds : List QDecision) (h : combine ds = .inside) :
    ds.all (· == .inside) = true := by
  unfold combine at h
  by_cases hall : ds.all (· == .inside) = true
  · exact hall
  · simp only [hall] at h
    by_cases hout : ds.all (· == .outside) = true
    · simp [hout] at h
    · simp only [hout] at h
      cases hf : ds.find? (fun d => d != .inside && d != .outside) with
      | none =>
          rw [hf] at h
          cases h
      | some d =>
          rw [hf] at h
          have := List.find?_some hf
          subst h
          simp at this

/-- Inside only when every index the callee may touch, shifted, is admitted
by the caller's own check. -/
theorem shifted_within_sound (env : Nat → Nat) (outer : QSet) (inner : List (Int × Int))
    (k a b v : Int) (h : shiftedWithin outer inner k = .inside) (hm : (a, b) ∈ inner)
    (h1 : a ≤ v) (h2 : v ≤ b) (hk : 0 ≤ b + k + 1) : outer.admits env (v + k) = true := by
  have hall := combine_inside _ h
  have hone := List.all_eq_true.mp hall _ (List.mem_map.mpr ⟨(a, b), hm, rfl⟩)
  have hab : a ≤ b := by omega
  simp [shiftOne, hab] at hone
  exact inside_sound env outer _ (v + k) hone (by simp [Describes, Count.eval]; omega)

/-! Shared vectors, decided here and by the Rust module. -/

def ρ0 : Nat → Int := fun _ => 0

/-- `(predicate, admitted set under ρ0)`. -/
def admittedVectors : List (Pred × Admitted) := [
  (.and (.cmp .ge (.mk 0 none)) (.cmp .lt (.mk 24 none)), .bounded 0 23),
  (.and (.cmp .ge (.mk 22 none)) (.cmp .le (.mk 23 none)), .bounded 22 23),
  (.and (.cmp .gt (.mk 21 none)) (.cmp .lt (.mk 24 none)), .bounded 22 23),
  (.cmp .eq (.mk 7 none), .bounded 7 7),
  (.or (.and (.cmp .ge (.mk 0 none)) (.cmp .lt (.mk 4 none)))
       (.and (.cmp .ge (.mk 8 none)) (.cmp .lt (.mk 12 none))), .bounded 0 11),
  (.and (.cmp .ge (.mk (-2) none)) (.cmp .lt (.mk 0 none)), .bounded (-2) (-1)),
  (.cmp .ge (.mk 0 none), .unbounded),
  (.cmp .lt (.mk 24 none), .unbounded),
  (.or (.cmp .lt (.mk 24 none)) (.cmp .ge (.mk 0 none)), .unbounded),
  (.or (.cmp .lt (.mk 24 none)) (.cmp .gt (.mk 0 none)), .unbounded),
  (.and (.cmp .lt (.mk 0 none)) (.cmp .gt (.mk 5 none)), .empty),
  (.and (.cmp .eq (.mk 3 none)) (.cmp .eq (.mk 4 none)), .empty),
  (.or (.and (.cmp .lt (.mk 0 none)) (.cmp .gt (.mk 5 none)))
       (.and (.cmp .ge (.mk 1 none)) (.cmp .le (.mk 2 none))), .bounded 1 2),
  (.and (.cmp .ge (.mk 0 none)) (.cmp .lt (.mk 0 (some 1))), .empty),
  (.and (.cmp .ge (.mk 0 none)) (.cmp .lt (.mk 4 (some 1))), .bounded 0 3)
]

theorem admitted_vectors_hold :
    admittedVectors.all (fun (p, a) => admitted (reading ρ0 p) == a) = true := by decide

theorem admitted_vectors_count : admittedVectors.length = 15 := by decide

/-- `(set, index fact, decision)`. -/
def decideVectors : List (QSet × IndexFact × QDecision) := [
  (.constant [(0, 23)], .const 22, .inside),
  (.constant [(0, 23)], .const 23, .inside),
  (.constant [(0, 23)], .const 24, .outside),
  (.constant [(0, 23)], .const (-1), .outside),
  (.constant [(0, 23)], .range 0 (.const 24), .inside),
  (.constant [(0, 23)], .range 0 (.const 25), .undecided .rangeNotContained),
  (.constant [(0, 23)], .range 24 (.const 30), .outside),
  (.constant [(0, 23)], .range 5 (.const 5), .undecided .emptyRange),
  (.constant [(0, 3), (8, 11)], .const 5, .outside),
  (.constant [(0, 3), (8, 11)], .range 2 (.const 9), .undecided .rangeNotContained),
  (.constant [(0, 3), (8, 11)], .range 4 (.const 8), .outside),
  (.constant [(0, 3), (8, 11)], .range 8 (.const 12), .inside),
  (.constant [(0, 23)], .unknown, .undecided .unsupportedRange),
  (.constant [(0, 23)], .range 0 (.sym 1), .undecided .symbolMismatch),
  (.pre 1, .range 0 (.sym 1), .inside),
  (.pre 1, .range 2 (.sym 1), .inside),
  (.pre 1, .range (-1) (.sym 1), .undecided .rangeNotContained),
  (.pre 1, .range 0 (.sym 2), .undecided .symbolMismatch),
  (.pre 1, .const 0, .undecided .symbolMismatch),
  (.pre 1, .const (-1), .outside),
  (.pre 1, .range (-3) (.const 0), .outside),
  (.pre 1, .range 0 (.const 4), .undecided .symbolMismatch),
  (.pre 1, .unknown, .undecided .unsupportedRange)
]

theorem decide_vectors_hold :
    decideVectors.all (fun (s, f, d) => indexAdmitted s f == d) = true := by decide

theorem decide_vectors_count : decideVectors.length = 23 := by decide

/-- `(caller set, callee intervals, offset, decision)`. -/
def shiftVectors : List (QSet × List (Int × Int) × Int × QDecision) := [
  (.constant [(0, 23)], [(22, 23)], 0, .inside),
  (.constant [(0, 23)], [(0, 1)], 22, .inside),
  (.constant [(0, 23)], [(0, 1)], 23, .undecided .rangeNotContained),
  (.constant [(0, 23)], [(0, 3)], 30, .outside),
  (.constant [(0, 3), (8, 11)], [(0, 1), (8, 9)], 0, .inside),
  (.constant [(0, 3), (8, 11)], [(0, 1), (20, 21)], 0, .undecided .rangeNotContained),
  (.pre 1, [(0, 3)], 0, .undecided .symbolMismatch)
]

theorem shift_vectors_hold :
    shiftVectors.all (fun (s, i, k, d) => shiftedWithin s i k == d) = true := by decide

theorem shift_vectors_count : shiftVectors.length = 7 := by decide

end QCore.IndexQuant

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
# Return-code dispositions (card CG23-RELATIONAL-POSTCONDITIONS, rule 11r)

The abstract rule implemented by `crates/qcore-schemas/src/relational_post.rs`
(`ReturnRelation::meets`, `RelationalContract::select` / `join`), the
condition reader `read_test` of `crates/qcore-c-gateway/src/relational_post.rs`
and its route-G adapter `legacy_relational_post.rs`.

A relational contract is a list of **exit classes**: a set of returned
integers, written `x OP k`, and the disposition (an ordinary custody contract,
abstract here as `D`) that holds on every return whose value lies in it.

## The caller

A caller tests the returned value against a constant where it stands. The
condition's true edge carries a relation `t`; its false edge the complement.
The lowering applies the call once on each edge, with `select classes edge`:
the one disposition every class the edge can reach agrees on.

* **Reachability is exact.** `meets` decides whether two relations share a
  value by sampling `k - 1`, `k`, `k + 1` of both constants
  (`meets_sound`, `meets_complete`).
* **The edge disposition is the returned class's.** For every returned value
  `x` in a class with disposition `d`, the edge C takes on `x` is the one whose
  relation `x` satisfies, and an accepted selection on it is `d`
  (`the_edge_disposition_is_the_returned_class`).
* **Every value reaches exactly one edge** (`every_value_reaches_one_edge`).
* **Untested, the join.** Without a test the call keeps the common
  disposition, and refuses when two classes disagree (`join_sound`,
  `join_refuses_disagreement`).
* **The test shapes.** `if (f())`, `if (!f())`, `f() OP k` and `k OP f()` are
  true exactly on the relation the reader records (`test_shape_is_exact`).

## The body

A body-derived disposition has one `eq v` class per returned literal `v`, its
disposition validated on every exit returning `v`
(`a_validated_summary_selects_each_exit_disposition`). A declaration whose
body yields such a summary is kept only when every declared class agrees with
the body's disposition on every value they share (`check`, `check_sound`);
otherwise it is withdrawn and selects nothing (`withdrawn_selects_nothing`).

## What this is not

A model of the decisions, over the mathematical integers; C's `int` values are
a subset, so `meets` can only err towards more classes (more refusal). The
flow's own transfer of each disposition is the existing call transfer,
unchanged. Faithful extraction of C, and that the per-class validation runs
the existing summary validation on exactly the exits returning `v`, are review
premises (trust boundary 28); `meetsVectors` and `selectVectors` bind the
Rust tables mechanically (`tests/unit/relational_post.rs`).
-/

namespace QCore.RelationalPost

/-! ## Relations over the returned value -/

inductive Op where
  | eq
  | ne
  | lt
  | le
  | gt
  | ge
  deriving DecidableEq, Repr

/-- `x OP k`. -/
def holds : Op → Int → Int → Bool
  | .eq, k, x => decide (x = k)
  | .ne, k, x => decide (x ≠ k)
  | .lt, k, x => decide (x < k)
  | .le, k, x => decide (x ≤ k)
  | .gt, k, x => decide (x > k)
  | .ge, k, x => decide (x ≥ k)

structure Rel where
  op : Op
  k : Int
  deriving DecidableEq, Repr

def Rel.sat (r : Rel) (x : Int) : Bool := holds r.op r.k x

/-- The breakpoints of both constants. -/
def samples (r s : Rel) : List Int := [r.k - 1, r.k, r.k + 1, s.k - 1, s.k, s.k + 1]

/-- `ReturnRelation::meets`. -/
def meets (r s : Rel) : Bool := (samples r s).any fun p => r.sat p && s.sat p

theorem meets_sound (r s : Rel) (h : meets r s = true) : ∃ x, r.sat x = true ∧ s.sat x = true := by
  simp only [meets, List.any_eq_true, Bool.and_eq_true] at h
  obtain ⟨p, _, hp⟩ := h
  exact ⟨p, hp⟩

/-- **Sampling is complete**: two relations that share any value share a
sampled one. -/
theorem meets_complete (r s : Rel) (x : Int) (hr : r.sat x = true) (hs : s.sat x = true) :
    meets r s = true := by
  obtain ⟨o1, a⟩ := r
  obtain ⟨o2, b⟩ := s
  simp only [meets, samples, Rel.sat] at hr hs ⊢
  cases o1 <;> cases o2 <;> simp [holds] at hr hs ⊢
  all_goals omega

/-- The complement, `!(x OP k)`. -/
def Op.neg : Op → Op
  | .eq => .ne
  | .ne => .eq
  | .lt => .ge
  | .le => .gt
  | .gt => .le
  | .ge => .lt

/-- The mirror, `k OP x` as `x OP' k`. -/
def Op.mirror : Op → Op
  | .eq => .eq
  | .ne => .ne
  | .lt => .gt
  | .le => .ge
  | .gt => .lt
  | .ge => .le

theorem holds_iff (o : Op) (k x : Int) : holds o k x = true ↔
    match o with
    | .eq => x = k
    | .ne => x ≠ k
    | .lt => x < k
    | .le => x ≤ k
    | .gt => x > k
    | .ge => x ≥ k := by
  cases o <;> simp [holds]

theorem neg_sat (o : Op) (k x : Int) : holds o.neg k x = !holds o k x := by
  apply Bool.eq_iff_iff.mpr
  rw [Bool.not_eq_true', ← Bool.not_eq_true, holds_iff, holds_iff]
  cases o <;> simp [Op.neg] <;> omega

/-- **Every returned value reaches exactly one edge.** -/
theorem every_value_reaches_one_edge (t : Rel) (x : Int) :
    (t.sat x != (Rel.mk t.op.neg t.k).sat x) = true := by
  simp only [Rel.sat, neg_sat]
  cases holds t.op t.k x <;> rfl

/-! ## The condition's shapes -/

/-- The four shapes `read_test` admits, around one call `f(…)`. -/
inductive Shape where
  | bare
  | not
  | cmp (o : Op) (k : Int)
  | cmpMirrored (o : Op) (k : Int)
  deriving Repr

/-- C's truth of the condition on returned value `x`. -/
def cTrue : Shape → Int → Bool
  | .bare, x => decide (x ≠ 0)
  | .not, x => decide (x = 0)
  | .cmp o k, x => holds o k x
  | .cmpMirrored o k, x =>
      match o with
      | .eq => decide (k = x)
      | .ne => decide (k ≠ x)
      | .lt => decide (k < x)
      | .le => decide (k ≤ x)
      | .gt => decide (k > x)
      | .ge => decide (k ≥ x)

/-- The relation `read_test` records for the true edge. -/
def trueEdge : Shape → Rel
  | .bare => ⟨.ne, 0⟩
  | .not => ⟨.eq, 0⟩
  | .cmp o k => ⟨o, k⟩
  | .cmpMirrored o k => ⟨o.mirror, k⟩

/-- **The recorded relation is exactly C's condition.** -/
theorem test_shape_is_exact (s : Shape) (x : Int) : (trueEdge s).sat x = cTrue s x := by
  cases s with
  | bare => rfl
  | not => rfl
  | cmp o k => rfl
  | cmpMirrored o k =>
      apply Bool.eq_iff_iff.mpr
      cases o <;> simp [trueEdge, Rel.sat, Op.mirror, holds, cTrue] <;> omega

/-! ## Selection on an edge -/

variable {D : Type} [DecidableEq D]

/-- `RelationalContract::select`. -/
def select (cs : List (Rel × D)) (e : Rel) : Option D :=
  match cs.filter (fun c => meets c.1 e) with
  | [] => none
  | c :: rest => if rest.all (fun c' => c'.2 == c.2) then some c.2 else none

theorem select_sound (cs : List (Rel × D)) (e : Rel) (dv : D)
    (hd : select cs e = some dv) (c : Rel) (d' : D) (hc : (c, d') ∈ cs) (x : Int)
    (hx : c.sat x = true) (he : e.sat x = true) : d' = dv := by
  have hm : meets c e = true := meets_complete c e x hx he
  have hf : (c, d') ∈ cs.filter (fun c => meets c.1 e) := List.mem_filter.mpr ⟨hc, hm⟩
  unfold select at hd
  split at hd
  · rename_i h
    rw [h] at hf
    simp at hf
  · rename_i first rest h
    split at hd
    · rename_i hall
      simp only [Option.some.injEq] at hd
      rw [h] at hf
      rcases List.mem_cons.mp hf with hfirst | hrest
      · rw [← hd, ← hfirst]
      · have := List.all_eq_true.mp hall _ hrest
        simp only [beq_iff_eq] at this
        rw [← hd]
        exact this
    · simp at hd

/-- **The edge disposition is the returned class's.** If the call returns `x`,
which lies in the class `(c, d)`, C takes the edge whose relation `x`
satisfies; when the lowering accepted that edge's selection, it applied `d`. -/
theorem the_edge_disposition_is_the_returned_class (cs : List (Rel × D)) (t : Rel)
    (c : Rel) (d : D) (hc : (c, d) ∈ cs) (x : Int) (hx : c.sat x = true) (dv : D)
    (hd : select cs (if t.sat x then t else ⟨t.op.neg, t.k⟩) = some dv) : d = dv := by
  by_cases ht : t.sat x = true
  · simp only [ht, ↓reduceIte] at hd
    exact select_sound cs t dv hd c d hc x hx ht
  · simp only [ht, Bool.false_eq_true, ↓reduceIte] at hd
    have hn : (Rel.mk t.op.neg t.k).sat x = true := by
      simp only [Rel.sat, neg_sat] at ht ⊢
      simpa using ht
    exact select_sound cs _ dv hd c d hc x hx hn

/-! ## Untested: the join -/

/-- `RelationalContract::join`. -/
def join (cs : List (Rel × D)) : Option D :=
  match cs with
  | [] => none
  | c :: rest => if rest.all (fun c' => c'.2 == c.2) then some c.2 else none

theorem join_sound (cs : List (Rel × D)) (dv : D) (h : join cs = some dv) (c : Rel) (d : D)
    (hc : (c, d) ∈ cs) : d = dv := by
  unfold join at h
  split at h
  · simp at h
  · rename_i first rest
    split at h
    · rename_i hall
      simp only [Option.some.injEq] at h
      rcases List.mem_cons.mp hc with hfirst | hrest
      · rw [← h, ← hfirst]
      · have := List.all_eq_true.mp hall _ hrest
        simp only [beq_iff_eq] at this
        rw [← h]
        exact this
    · simp at h

/-- **An untested call whose classes disagree refuses.** -/
theorem join_refuses_disagreement (cs : List (Rel × D)) (c1 c2 : Rel) (d1 d2 : D)
    (h1 : (c1, d1) ∈ cs) (h2 : (c2, d2) ∈ cs) (hne : d1 ≠ d2) : join cs = none := by
  cases hj : join cs with
  | none => rfl
  | some dv =>
      exact absurd ((join_sound cs dv hj c1 d1 h1).trans (join_sound cs dv hj c2 d2 h2).symm) hne

/-! ## The body: one class per returned literal -/

/-- A body's exits: the literal each returns and the disposition the existing
validation establishes there. -/
abbrev Exits (D : Type) := List (Int × D)

/-- The body-derived classes, one `eq v` per claimed value. -/
def classesOf (claims : List (Int × D)) : List (Rel × D) :=
  claims.map fun (v, d) => (⟨.eq, v⟩, d)

/-- Per-class validation: every exit returning `v` has the disposition claimed
for `v`, and every exit's value is claimed. -/
def validated (exits : Exits D) (claims : List (Int × D)) : Prop :=
  ∀ v d, (v, d) ∈ exits → (v, d) ∈ claims ∧ ∀ d', (v, d') ∈ claims → d' = d

/-- **A validated body-derived summary selects each exit's disposition**: for
an exit returning `v` and any accepted edge containing `v`, the selected
disposition is that exit's. -/
theorem a_validated_summary_selects_each_exit_disposition (exits : Exits D)
    (claims : List (Int × D)) (hv : validated exits claims) (v : Int) (d : D)
    (he : (v, d) ∈ exits) (e : Rel) (hev : e.sat v = true) (dv : D)
    (hs : select (classesOf claims) e = some dv) : d = dv := by
  have hc : ((⟨.eq, v⟩ : Rel), d) ∈ classesOf claims := by
    simp only [classesOf, List.mem_map]
    exact ⟨(v, d), (hv v d he).1, rfl⟩
  exact select_sound _ e dv hs _ d hc v (by simp [Rel.sat, holds]) hev

/-- The body check of a declaration: every declared class agrees with the
body's disposition on every value they share. -/
def check (declared : List (Rel × D)) (body : List (Int × D)) : Bool :=
  body.all fun (v, d) => declared.all fun (c, d') => !c.sat v || d' == d

theorem check_sound (declared : List (Rel × D)) (body : List (Int × D)) (h : check declared body)
    (v : Int) (d : D) (hb : (v, d) ∈ body) (c : Rel) (d' : D) (hc : (c, d') ∈ declared)
    (hv : c.sat v = true) : d' = d := by
  have := List.all_eq_true.mp (List.all_eq_true.mp h _ hb) _ hc
  simp only [hv, Bool.not_true, Bool.false_or, beq_iff_eq] at this
  exact this

/-- A withdrawn declaration is never instantiated. -/
def selectUnless (withdrawn : Bool) (cs : List (Rel × D)) (e : Rel) : Option D :=
  if withdrawn then none else select cs e

theorem withdrawn_selects_nothing (cs : List (Rel × D)) (e : Rel) :
    selectUnless true cs e = none := rfl

/-! ## Shared vectors -/

/-- `MEETS_VECTORS` of `relational_post.rs`, row by row. -/
def meetsVectors : List (Op × Int × Op × Int × Bool) := [
  (.eq, 0, .ne, 0, false),
  (.eq, 0, .eq, 0, true),
  (.eq, 0, .lt, 0, false),
  (.eq, 0, .le, 0, true),
  (.ne, 0, .ne, 1, true),
  (.lt, 0, .gt, -1, false),
  (.lt, 0, .gt, -2, true),
  (.le, 0, .ge, 0, true),
  (.lt, 0, .ge, 0, false),
  (.eq, -1, .lt, 0, true),
  (.gt, 3, .lt, 5, true),
  (.gt, 3, .lt, 4, false)
]

theorem meets_vectors_hold :
    meetsVectors.all (fun (a, k, b, l, m) => meets ⟨a, k⟩ ⟨b, l⟩ == m) = true := by
  decide

inductive Disp where
  | a
  | b
  deriving DecidableEq, Repr

open Disp

inductive Fixture where
  | eqZeroNeZero
  | ltZeroGeZero
  | eqZeroEqMinusOne
  deriving DecidableEq, Repr

def Fixture.classes : Fixture → List (Rel × Disp)
  | .eqZeroNeZero => [(⟨.eq, 0⟩, .a), (⟨.ne, 0⟩, .b)]
  | .ltZeroGeZero => [(⟨.lt, 0⟩, .a), (⟨.ge, 0⟩, .b)]
  | .eqZeroEqMinusOne => classesOf [(0, .a), (-1, .b)]

/-- `SELECT_VECTORS` of `relational_post.rs`, row by row. -/
def selectVectors : List (Fixture × Op × Int × Option Disp) := [
  (.eqZeroNeZero, .eq, 0, some .a),
  (.eqZeroNeZero, .ne, 0, some .b),
  (.eqZeroNeZero, .lt, 0, some .b),
  (.eqZeroNeZero, .ge, 0, none),
  (.ltZeroGeZero, .lt, 0, some .a),
  (.ltZeroGeZero, .ge, 0, some .b),
  (.ltZeroGeZero, .eq, -1, some .a),
  (.ltZeroGeZero, .ne, 0, none),
  (.eqZeroEqMinusOne, .eq, 0, some .a),
  (.eqZeroEqMinusOne, .ne, 0, some .b),
  (.eqZeroEqMinusOne, .lt, 0, some .b),
  (.eqZeroEqMinusOne, .gt, 0, none)
]

theorem select_vectors_hold :
    selectVectors.all (fun (f, o, k, r) => select f.classes ⟨o, k⟩ == r) = true := by
  decide

end QCore.RelationalPost

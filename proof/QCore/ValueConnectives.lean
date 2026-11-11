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

import QCore.ElaborationCore
import QCore.NestedCallEffects

/-! Value connectives and conditionals (card CG06-VALUE-CONNECTIVES-AND-CONDITIONAL,
gateway rule 10z): a model of the branch regions of
`crates/qcore-c-gateway/src/elaboration.rs` and of the route-G branching
lowering of `legacy_value_connective.rs` / `value_connective.rs`.

Bound mechanically: the Rust tests `value_connective::tests::cg06_lean_vectors_match`
(`admissionVectors` = `ADMISSION_VECTORS`, `transparent` = `TRANSPARENT`) and
`elaboration::tests::cg06_region_vectors_match_the_elaborator` (each
`regionVectors` / `exclusiveVectors` C source through Clang and `elaborate`,
`controlVectors` against `commutes`) read this file, and
`tests/cg06_value_connectives.rs` runs each `mergeVectors` source through
route G. The Clang-JSON reading and node matching stay a review premise
(trust boundary 28).

Stated here:
* **Short circuit.** `A && B` / `A || B`: the operand C does not evaluate
  contributes no event; the value is `0` or `1`.
* **Selection.** `C ? A : B` performs `C`'s events, then exactly one arm's.
* **Omitted middle.** `X ?: B` performs `X`'s events once; `B`'s only where
  `X` is zero. It is not `X ? X : B` for an `X` with effects, and it is for
  a reads-only `X` (the route's re-read lowering).
* **Order.** A reads-only deciding operand may run before reads-only
  siblings; a call that preserves an unaliased place leaves a read of it
  unchanged. Control events touch no store.
* **Merge.** The join of two edges' custody carries every obligation of both;
  a release through a binding with two identities is refused, never resolved
  to one.
* **Forms and positions.** The branching forms are elaborated; every other
  position a connective stands in is refused.
* **Siblings with calls** (card CG06-VALUE-CONNECTIVE-RESIDUALS, rule 11m,
  `connective_footprints.rs`). Where `admit` refuses a sibling's effect or a
  deciding call beside a sibling that is not inert, the region is composed
  with its siblings by `QCore.NestedCallEffects`' footprints, one graph per
  arm: an admitted arm runs alike in every order C permits, the lowering's
  (deciding operand first) included; the deciding operand reads one value
  whichever sibling events C runs before it, so C takes the lowering's edge;
  an overlapping, uncontracted or non-call sibling is refused. Bound by
  `composedVectors` = `COMPOSED_VECTORS` and `footprintVectors` =
  `FOOTPRINT_VECTORS` (`tests/unit/value_connective.rs`,
  `tests/unit/connective_footprints.rs`). -/
namespace QCore.ValueConnectives

open QCore.ElaborationCore

/-! ## The value language -/

/-- A value expression over concrete events: an atom performs `events` and
yields `value` of the store it started in. -/
inductive VExpr where
  | atom (events : List CEv) (value : Store → Int)
  | and (a b : VExpr)
  | or (a b : VExpr)
  | cond (c a b : VExpr)
  | elvis (x b : VExpr)

def truth (v : Int) : Int := if v = 0 then 0 else 1

/-- The value and the events C performs, in order, from store `s`. -/
def exec : VExpr → Store → Int × List CEv
  | .atom events value, s => (value s, events)
  | .and a b, s =>
    let (va, ta) := exec a s
    if va = 0 then (0, ta)
    else
      let (vb, tb) := exec b (run ta s)
      (truth vb, ta ++ tb)
  | .or a b, s =>
    let (va, ta) := exec a s
    if va ≠ 0 then (1, ta)
    else
      let (vb, tb) := exec b (run ta s)
      (truth vb, ta ++ tb)
  | .cond c a b, s =>
    let (vc, tc) := exec c s
    if vc ≠ 0 then
      let (va, ta) := exec a (run tc s)
      (va, tc ++ ta)
    else
      let (vb, tb) := exec b (run tc s)
      (vb, tc ++ tb)
  | .elvis x b, s =>
    let (vx, tx) := exec x s
    if vx ≠ 0 then (vx, tx)
    else
      let (vb, tb) := exec b (run tx s)
      (vb, tx ++ tb)

/-! ## Short circuit -/

/-- `A && B` where `A` is zero: `B` contributes no event. -/
theorem and_skips (a b : VExpr) (s : Store) (h : (exec a s).1 = 0) :
    exec (.and a b) s = (0, (exec a s).2) := by
  simp [exec, h]

/-- `A || B` where `A` is non-zero: `B` contributes no event. -/
theorem or_skips (a b : VExpr) (s : Store) (h : (exec a s).1 ≠ 0) :
    exec (.or a b) s = (1, (exec a s).2) := by
  simp [exec, h]

/-- `A && B` where `A` is non-zero: `B` runs after `A`, on `A`'s final store. -/
theorem and_evaluates (a b : VExpr) (s : Store) (h : (exec a s).1 ≠ 0) :
    (exec (.and a b) s).2 = (exec a s).2 ++ (exec b (run (exec a s).2 s)).2 := by
  simp [exec, h]

theorem or_evaluates (a b : VExpr) (s : Store) (h : (exec a s).1 = 0) :
    (exec (.or a b) s).2 = (exec a s).2 ++ (exec b (run (exec a s).2 s)).2 := by
  simp [exec, h]

theorem truth_boolean (v : Int) : truth v = 0 ∨ truth v = 1 := by
  unfold truth; split <;> simp

/-- The value of a connective is the `int` 0 or 1. -/
theorem connective_value_boolean (a b : VExpr) (s : Store) :
    ((exec (.and a b) s).1 = 0 ∨ (exec (.and a b) s).1 = 1) ∧
      ((exec (.or a b) s).1 = 0 ∨ (exec (.or a b) s).1 = 1) := by
  constructor
  · simp only [exec]
    split
    · exact Or.inl rfl
    · exact truth_boolean _
  · simp only [exec]
    split
    · exact Or.inr rfl
    · exact truth_boolean _

/-! ## Selection -/

/-- `C ? A : B` performs `C`'s events, then exactly the selected arm's. -/
theorem cond_selects (c a b : VExpr) (s : Store) :
    (exec (.cond c a b) s).2 = (exec c s).2 ++
      (if (exec c s).1 ≠ 0 then (exec a (run (exec c s).2 s)).2
       else (exec b (run (exec c s).2 s)).2) := by
  simp only [exec]
  split <;> simp_all

/-- The unselected arm contributes nothing: on the non-zero edge the trace is
`C`'s then `A`'s, and `B`'s events are not in it unless `C` or `A` performs
them. -/
theorem cond_yes (c a b : VExpr) (s : Store) (h : (exec c s).1 ≠ 0) :
    exec (.cond c a b) s =
      ((exec a (run (exec c s).2 s)).1, (exec c s).2 ++ (exec a (run (exec c s).2 s)).2) := by
  simp [exec, h]

theorem cond_no (c a b : VExpr) (s : Store) (h : (exec c s).1 = 0) :
    exec (.cond c a b) s =
      ((exec b (run (exec c s).2 s)).1, (exec c s).2 ++ (exec b (run (exec c s).2 s)).2) := by
  simp [exec, h]

/-! ## The omitted middle operand -/

/-- `X ?: B` performs `X`'s events exactly once, then `B`'s only where `X`
is zero. -/
theorem elvis_once (x b : VExpr) (s : Store) :
    (exec (.elvis x b) s).2 = (exec x s).2 ++
      (if (exec x s).1 ≠ 0 then [] else (exec b (run (exec x s).2 s)).2) := by
  simp only [exec]
  split <;> simp_all

/-- `X ?: f()` with `X` non-zero never evaluates `f`. -/
theorem elvis_skips (x b : VExpr) (s : Store) (h : (exec x s).1 ≠ 0) :
    exec (.elvis x b) s = ((exec x s).1, (exec x s).2) := by
  simp [exec, h]

/-- The omitted middle operand is not `X ? X : B` in general: an `X` that
writes performs its write twice in the latter. -/
theorem elvis_not_cond :
    (exec (.elvis (.atom [.write 0 1] (fun _ => 1)) (.atom [] (fun _ => 0))) (fun _ => 0)).2.length ≠
      (exec (.cond (.atom [.write 0 1] (fun _ => 1)) (.atom [.write 0 1] (fun _ => 1))
        (.atom [] (fun _ => 0))) (fun _ => 0)).2.length := by
  decide

/-- Every event of a list is a read. -/
def readsOnly (l : List CEv) : Prop := ∀ e ∈ l, ∃ p, e = .read p

theorem run_reads (l : List CEv) (h : readsOnly l) (s : Store) : run l s = s := by
  induction l generalizing s with
  | nil => rfl
  | cons e l ih =>
    obtain ⟨p, hp⟩ := h e (List.mem_cons_self ..)
    subst hp
    exact ih (fun e' he => h e' (List.mem_cons_of_mem _ he)) s

/-- **The re-read lowering.** For an `X` whose events are reads only, the
route's `X ? X : B` (re-reading `X` on the non-zero edge) reaches the same
value and store as `X ?: B`, and adds only reads. -/
theorem elvis_reread (x b : VExpr) (s : Store) (h : readsOnly (exec x s).2) :
    (exec (.cond x x b) s).1 = (exec (.elvis x b) s).1 ∧
      run (exec (.cond x x b) s).2 s = run (exec (.elvis x b) s).2 s := by
  have hs : run (exec x s).2 s = s := run_reads _ h s
  simp only [exec]
  by_cases hx : (exec x s).1 = 0
  · simp [hx]
  · simp only [hx, ne_eq, not_false_eq_true, ite_true, hs]
    constructor
    · simp
    · simp [run_append, hs]

/-! ## Order: the deciding operand first -/

/-- Two reads-only operands commute in every interleaving: the route may run a
reads-only deciding operand before reads-only siblings. -/
theorem decision_first (a b m : List CEv) (ha : readsOnly a) (hb : readsOnly b)
    (hi : Interleave a b m) (s : Store) : run m s = run (a ++ b) s := by
  apply interleave_run a b m _ hi s
  intro x hx y hy
  obtain ⟨p, rfl⟩ := ha x hx
  obtain ⟨q, rfl⟩ := hb y hy
  rfl

/-- A call that preserves every unaliased place leaves a read of one
unchanged: an inert sibling observes the same value before or after a
deciding operand's call. -/
theorem inert_read_stable (f : Store → Store) (u : Place → Prop)
    (hf : ∀ s p, u p → f s p = s p) (p : Place) (hp : u p) (s : Store) :
    step (.call f) s p = s p := hf s p hp

/-- The control events a region adds (`Branch`, `Merge`) touch no store. -/
inductive Ctl where
  | branch
  | merge
  deriving DecidableEq, Repr

def ctlStep (_ : Ctl) (s : Store) : Store := s

theorem control_commutes (c : Ctl) (e : CEv) (s : Store) :
    ctlStep c (step e s) = step e (ctlStep c s) := rfl

/-! ## Merge of the two edges -/

/-- A binding's possible identities, and the obligations a path carries. -/
abbrev Binding := List Nat
abbrev Obligations := List Nat

/-- The join of two edges: every possible identity and every obligation of
either edge. -/
def join (a b : List Nat) : List Nat := a ++ b

/-- A release through a binding resolves only a single identity. -/
def release : Binding → Option Nat
  | [t] => some t
  | _ => none

theorem merge_carries (a b : Obligations) :
    (∀ o ∈ a, o ∈ join a b) ∧ (∀ o ∈ b, o ∈ join a b) :=
  ⟨fun _ h => List.mem_append_left _ h, fun _ h => List.mem_append_right _ h⟩

/-- Two different owned identities merge to "one of": a release through the
merged binding is refused, never resolved to either. -/
theorem release_merged_refuses (t u : Nat) : release (join [t] [u]) = none := rfl

theorem release_single (t : Nat) : release [t] = some t := rfl

/-- Neither identity is picked: the refusal holds whichever edge ran. -/
theorem release_never_picks (t u : Nat) :
    release (join [t] [u]) ≠ some t ∧ release (join [t] [u]) ≠ some u := by
  simp [release_merged_refuses]

/-- `mergeVectors`: a C source and whether route G publishes it `Safe`
(`tests/cg06_value_connectives.rs` runs each). The first is the brief's
`r = c ? a : b; free(r)` over two owned identities. -/
def mergeVectors : List (String × Bool) := [
  ("void subject(int c) { void *a = q_alloc(); void *b = q_alloc(); void *r = c ? a : b; q_free(r); }",
    false),
  ("void subject(int c) { void *p = q_alloc(); int x = c && r_release_code(p); }", false),
  ("void subject(int c) { void *p = q_alloc(); int x = c ? r_release_code(p) : r_consume_code(p); }",
    true),
  ("void subject(int x) { void *p = q_alloc(); int r = x ?: r_release_code(p); q_free(p); }", false),
  ("void subject(int x) { void *p = q_alloc(); int r = x ?: r_borrow_code(p); q_free(p); }", true)
]

/-! ## Forms, positions and admission -/

/-- The elaborator's branching forms are elaborated (`FORM_TABLE`). -/
theorem branching_forms_elaborated :
    disposition "value &&" = some .branching ∧ disposition "value ||" = some .branching ∧
      disposition "conditional" = some .branching ∧
      disposition "omitted-middle conditional" = some .branching ∧
      read "value &&" = .elaborated ∧ read "conditional" = .elaborated := by
  decide

/-- `TRANSPARENT`: the positions a value region is found through. -/
def transparent : List String := [
  "ParenExpr",
  "ImplicitCastExpr",
  "CStyleCastExpr",
  "UnaryOperator",
  "BinaryOperator",
  "CallExpr actual",
  "MemberExpr",
  "ArraySubscriptExpr",
  "root =",
  "root op=",
  "root declaration",
  "root return"
]

/-- A region under any other evaluated node is refused. -/
def positionAdmits (node : String) : Bool := transparent.contains node

theorem unlisted_position_refused (node : String) (h : node ∉ transparent) :
    positionAdmits node = false := by
  simpa [positionAdmits] using h

theorem sequencing_positions_refused :
    positionAdmits "BinaryOperator ," = false ∧ positionAdmits "StmtExpr" = false ∧
      positionAdmits "InitListExpr" = false ∧ positionAdmits "GenericSelectionExpr" = false ∧
      positionAdmits "indirect callee" = false := by
  decide

/-- `admit`: siblings effect-free; a deciding operand with an effect needs
inert siblings; an omitted-middle operand must be reads only. -/
def admit (siblingsEffectFree decisionEffectFree siblingsInert : Bool)
    (omittedMiddleReadsOnly : Option Bool) : Bool :=
  omittedMiddleReadsOnly != some false && siblingsEffectFree &&
    (decisionEffectFree || siblingsInert)

/-- `admissionVectors` = `ADMISSION_VECTORS`. -/
def admissionVectors : List (Bool × Bool × Bool × Option Bool × Bool) := [
  (true, true, true, none, true),
  (true, true, false, none, true),
  (true, false, true, none, true),
  (true, false, false, none, false),
  (false, true, true, none, false),
  (true, true, true, some true, true),
  (true, true, true, some false, false),
  (false, false, false, some false, false)
]

theorem admission_vectors_hold :
    admissionVectors.all (fun v => admit v.1 v.2.1 v.2.2.1 v.2.2.2.1 == v.2.2.2.2) = true := by
  decide

theorem sibling_effect_refused (d i : Bool) (m : Option Bool) : admit false d i m = false := by
  cases d <;> cases i <;> simp [admit]

theorem effectful_decision_needs_inert (m : Option Bool) :
    admit true false false m = false := by
  simp [admit]

theorem omitted_middle_effect_refused (s d i : Bool) : admit s d i (some false) = false := by
  simp [admit]

/-! ## The elaborator's regions -/

/-- `regionVectors`: a C source, the region's form, and the number of events
of its deciding operand and of its non-zero and zero arms, as `elaborate`
gives them. -/
def regionVectors : List (String × String × Nat × Nat × Nat) := [
  ("int subject(int a, int b) { int x = 0; x = (a && b); return x; }", "value &&", 1, 1, 0),
  ("int subject(int a, int b) { int x = 0; x = (a || b); return x; }", "value ||", 1, 0, 1),
  ("int subject(int a, int b) { int x = 0; x = a ? b : 1; return x; }", "conditional", 1, 1, 0),
  ("int subject(int a, int b) { int x = 0; x = a ?: b; return x; }",
    "omitted-middle conditional", 1, 0, 1),
  ("int subject(int a, int b) { int x = 0; x = a ? b++ : b--; return x; }", "conditional", 1, 2, 2)
]

/-- `exclusiveVectors`: a C source and whether the order check admits it: the
two arms of one region never pair; an arm is still checked against the rest. -/
def exclusiveVectors : List (String × Bool) := [
  ("int subject(int c, int y) { int x = 0; x = c ? y++ : y--; return x; }", true),
  ("int subject(int c, int y) { int x = 0; x = (c ? y++ : 0) + y; return x; }", false),
  ("int subject(int c, int y) { int x = 0; x = (c && y++) + y; return x; }", false),
  ("int subject(int c, int y, int z) { int x = 0; x = (c ? y++ : 0) + z; return x; }", true)
]

/-- `controlVectors`: `Branch`/`Merge` against each event kind, all commuting
(`commutes` in Rust). -/
def controlVectors : List (String × String × Bool) := [
  ("branch", "read", true),
  ("branch", "write", true),
  ("merge", "call", true),
  ("merge", "branch", true)
]

/-! ## Siblings with calls: composition with CG05's footprints (rule 11m)

Card CG06-VALUE-CONNECTIVE-RESIDUALS (sibling-effect part). The events are
`QCore.NestedCallEffects.Ev`s carrying the footprints the Rust reading gives
them; the order check is `QCore.NestedCallEffects.check`, unchanged. -/

/-- `admit_composed`: `admit`, where its two order refusals are asked of the
composition (`c`: `none` when none is asked); an omitted-middle operand with
an effect is never composed. -/
def admitComposed (s d i : Bool) (m : Option Bool) (c : Option Bool) : Bool :=
  admit s d i m || (m != some false && c == some true)

/-- `composedVectors` = `COMPOSED_VECTORS`. -/
def composedVectors : List (Bool × Bool × Bool × Option Bool × Option Bool × Bool) := [
  (true, true, true, none, some false, true),
  (false, true, true, none, none, false),
  (false, true, true, none, some true, true),
  (false, true, true, none, some false, false),
  (true, false, false, none, some true, true),
  (true, false, false, none, some false, false),
  (false, false, false, none, some true, true),
  (true, true, true, some false, some true, false),
  (false, true, true, some false, some true, false),
  (false, true, true, some true, some true, true)
]

theorem composed_vectors_hold :
    composedVectors.all (fun v =>
      admitComposed v.1 v.2.1 v.2.2.1 v.2.2.2.1 v.2.2.2.2.1 == v.2.2.2.2.2) = true := by
  decide

/-- The composition only adds admissions: rule 10z's admissions stand. -/
theorem composition_extends_admit (s d i : Bool) (m c : Option Bool) (h : admit s d i m = true) :
    admitComposed s d i m c = true := by
  simp [admitComposed, h]

/-- With no composition asked, the decision is rule 10z's. -/
theorem uncomposed_is_admit (s d i : Bool) (m : Option Bool) :
    admitComposed s d i m none = admit s d i m := by
  simp [admitComposed]

/-- An omitted-middle operand with an effect is refused whatever the
composition says (C evaluates it once; the lowering re-reads it). -/
theorem composition_never_rereads (s d i : Bool) (c : Option Bool) :
    admitComposed s d i (some false) c = false := by
  simp [admitComposed, admit]

/-- A control event (`Branch`, `Merge`): an empty footprint and no effect. -/
def ctlEv {L V : Type} : NestedCallEffects.Ev L V := ⟨⟨[], []⟩, id⟩

theorem ctl_respects {L V : Type} (ρ : NestedCallEffects.Den L) :
    NestedCallEffects.Respects (V := V) ρ ⟨[], []⟩ id :=
  { frame := fun _ _ _ => rfl
    local_ := by
      rintro s t - l ⟨c, hc, -⟩
      cases hc }

/-- One arm's graph: the region's events in the order C sequences them (the
deciding operand, `Branch`, the arm, `Merge`), unsequenced with the siblings'
tree. -/
def armTree {L V : Type} (d a : List (NestedCallEffects.Ev L V))
    (sibs : NestedCallEffects.Tm (NestedCallEffects.Ev L V)) :
    NestedCallEffects.Tm (NestedCallEffects.Ev L V) :=
  .pair (.leaf (d ++ [ctlEv] ++ a ++ [ctlEv])) sibs

/-- **An admitted arm runs alike.** If the footprint check admits an arm's
graph, every order C permits on that arm reaches the state of the order that
runs the region first (the deciding operand before every sibling), which is
the lowering's. -/
theorem composed_arm_runs_alike {L V : Type} (u : Nat → Bool) (ρ : NestedCallEffects.Den L)
    (hρ : NestedCallEffects.Sound u ρ) (d a : List (NestedCallEffects.Ev L V))
    (sibs : NestedCallEffects.Tm (NestedCallEffects.Ev L V))
    (hr : ∀ x ∈ NestedCallEffects.post (armTree d a sibs), NestedCallEffects.Respects ρ x.fp x.eff)
    (hc : NestedCallEffects.check u (armTree d a sibs) = true)
    (o : List ((L → V) → (L → V)))
    (ho : NestedCallEffects.Permitted ((armTree d a sibs).map NestedCallEffects.Ev.eff) o)
    (s : L → V) :
    NestedCallEffects.run o s =
      NestedCallEffects.run ((d ++ [ctlEv] ++ a ++ [ctlEv] ++
        NestedCallEffects.post sibs).map NestedCallEffects.Ev.eff) s :=
  NestedCallEffects.every_interleaving_runs_alike u ρ hρ _ hr hc o ho s

/-- **On each arm.** Both arms' graphs admitted: each runs alike. The two
arms are never in one graph (they never both occur). -/
theorem composed_region_runs_alike_on_each_arm {L V : Type} (u : Nat → Bool)
    (ρ : NestedCallEffects.Den L) (hρ : NestedCallEffects.Sound u ρ)
    (d a₀ a₁ : List (NestedCallEffects.Ev L V))
    (sibs : NestedCallEffects.Tm (NestedCallEffects.Ev L V))
    (hr₀ : ∀ x ∈ NestedCallEffects.post (armTree d a₀ sibs),
      NestedCallEffects.Respects ρ x.fp x.eff)
    (hr₁ : ∀ x ∈ NestedCallEffects.post (armTree d a₁ sibs),
      NestedCallEffects.Respects ρ x.fp x.eff)
    (hc₀ : NestedCallEffects.check u (armTree d a₀ sibs) = true)
    (hc₁ : NestedCallEffects.check u (armTree d a₁ sibs) = true) (s : L → V) :
    (∀ o, NestedCallEffects.Permitted ((armTree d a₀ sibs).map NestedCallEffects.Ev.eff) o →
      NestedCallEffects.run o s = NestedCallEffects.run
        ((d ++ [ctlEv] ++ a₀ ++ [ctlEv] ++ NestedCallEffects.post sibs).map
          NestedCallEffects.Ev.eff) s) ∧
    (∀ o, NestedCallEffects.Permitted ((armTree d a₁ sibs).map NestedCallEffects.Ev.eff) o →
      NestedCallEffects.run o s = NestedCallEffects.run
        ((d ++ [ctlEv] ++ a₁ ++ [ctlEv] ++ NestedCallEffects.post sibs).map
          NestedCallEffects.Ev.eff) s) :=
  ⟨fun o ho => composed_arm_runs_alike u ρ hρ d a₀ sibs hr₀ hc₀ o ho s,
    fun o ho => composed_arm_runs_alike u ρ hρ d a₁ sibs hr₁ hc₁ o ho s⟩

/-- A value the deciding operand computes through footprint `D`: it depends
only on the locations `D` touches. -/
def ReadsWithin {L V : Type} (ρ : NestedCallEffects.Den L) (D : NestedCallEffects.Footprint)
    (v : (L → V) → Int) : Prop :=
  ∀ s t, (∀ l, NestedCallEffects.inCells ρ (NestedCallEffects.touches D) l → s l = t l) →
    v s = v t

/-- **The deciding operand reads one value.** Whichever sibling events C runs
before the deciding operand, each commuting with it by footprints, the value
it decides on is the one the lowering reads first: C takes the lowering's
edge. -/
theorem decision_value_stable {L V : Type} (u : Nat → Bool) (ρ : NestedCallEffects.Den L)
    (hρ : NestedCallEffects.Sound u ρ) (D : NestedCallEffects.Footprint) (v : (L → V) → Int)
    (hv : ReadsWithin ρ D v) (ys : List (NestedCallEffects.Ev L V))
    (hr : ∀ y ∈ ys, NestedCallEffects.Respects ρ y.fp y.eff)
    (hc : ∀ y ∈ ys, NestedCallEffects.fpCommute u D y.fp = true) (s : L → V) :
    v (NestedCallEffects.run (ys.map NestedCallEffects.Ev.eff) s) = v s := by
  induction ys generalizing s with
  | nil => rfl
  | cons y ys ih =>
    show v (NestedCallEffects.run (ys.map NestedCallEffects.Ev.eff) (y.eff s)) = v s
    rw [ih (fun y' h => hr y' (List.mem_cons_of_mem _ h))
      (fun y' h => hc y' (List.mem_cons_of_mem _ h))]
    have hyc := hc y (List.mem_cons_self ..)
    have hclash : NestedCallEffects.clash u y.fp D = false := by
      unfold NestedCallEffects.fpCommute at hyc
      cases h : NestedCallEffects.clash u y.fp D <;> simp_all
    have hdis := NestedCallEffects.disjoint_of_no_clash hρ hclash
    exact hv _ _ fun l hl =>
      (hr y (List.mem_cons_self ..)).frame s l fun hw => hdis l hw hl

/-- **An overlapping sibling is refused.** A region event (deciding operand
or arm) and a sibling event whose footprints do not commute refuse the arm. -/
theorem an_overlapping_sibling_is_refused {L V : Type} (u : Nat → Bool)
    (d a : List (NestedCallEffects.Ev L V))
    (sibs : NestedCallEffects.Tm (NestedCallEffects.Ev L V)) (x y : NestedCallEffects.Ev L V)
    (hx : x ∈ d ++ a) (hy : y ∈ NestedCallEffects.post sibs)
    (h : NestedCallEffects.fpCommute u x.fp y.fp = false) :
    NestedCallEffects.check u (armTree d a sibs) = false := by
  apply NestedCallEffects.a_non_commuting_pair_is_refused u _ _ x y _ hy h
  show x ∈ d ++ [ctlEv] ++ a ++ [ctlEv]
  rcases List.mem_append.mp hx with hx | hx <;> simp [hx]

/-- The brief's controls, by their footprints: the deciding read of `p`
(its storage `d` and identity `p`) beside a release of `p` is refused ... -/
theorem the_released_identity_is_refused (u : Nat → Bool) (d p : Nat) :
    NestedCallEffects.fpCommute u ⟨[.object d, .identity p], []⟩ ⟨[], [.identity p]⟩ = false :=
  NestedCallEffects.a_read_unordered_with_its_release_is_refused u d p

/-- ... and beside a release of another name too: two identities are never
proved distinct. -/
theorem another_name_is_refused (u : Nat → Bool) (d p q : Nat) :
    NestedCallEffects.fpCommute u ⟨[.object d, .identity p], []⟩ ⟨[], [.identity q]⟩ = false := by
  simp [NestedCallEffects.fpCommute, NestedCallEffects.clash, NestedCallEffects.touches,
    NestedCallEffects.overlap]

/-- A release of a fresh identity (`r_release_code(r_wrap(p))`: `r_wrap`
borrows `p` and yields fresh `k`, released) commutes with the deciding read
of `p` and with an arm that borrows `p`. -/
theorem a_fresh_release_commutes (u : Nat → Bool) (d p k : Nat) :
    NestedCallEffects.fpCommute u ⟨[.object d, .identity p], []⟩ ⟨[.identity p], [.fresh k]⟩ =
        true ∧
      NestedCallEffects.fpCommute u ⟨[.object d, .identity p], []⟩ ⟨[], [.fresh k]⟩ = true ∧
      NestedCallEffects.fpCommute u ⟨[.identity p], []⟩ ⟨[.identity p], [.fresh k]⟩ = true ∧
      NestedCallEffects.fpCommute u ⟨[.identity p], []⟩ ⟨[], [.fresh k]⟩ = true := by
  simp [NestedCallEffects.fpCommute, NestedCallEffects.clash, NestedCallEffects.touches,
    NestedCallEffects.overlap]

/-- A sibling as the composition reads it: its events with their footprints,
or no reading: an uncontracted call has no footprint, and an effect that is
not a call is invisible to the footprints. -/
inductive Sibling (L V : Type) where
  | read (t : NestedCallEffects.Tm (NestedCallEffects.Ev L V))
  | uncontracted
  | nonCall

/-- The siblings' tree, every two siblings unsequenced; `none` when one is
not read. -/
def siblingTree {L V : Type} :
    List (Sibling L V) → Option (NestedCallEffects.Tm (NestedCallEffects.Ev L V))
  | [] => some (.leaf [])
  | .read t :: rest => (siblingTree rest).map (.pair t)
  | .uncontracted :: _ => none
  | .nonCall :: _ => none

/-- `compose`: every sibling read, and both arms' graphs admitted. -/
def compose {L V : Type} (u : Nat → Bool) (d a₀ a₁ : List (NestedCallEffects.Ev L V))
    (sibs : List (Sibling L V)) : Bool :=
  match siblingTree sibs with
  | none => false
  | some t =>
    NestedCallEffects.check u (armTree d a₀ t) && NestedCallEffects.check u (armTree d a₁ t)

theorem unread_sibling_tree {L V : Type} (sibs : List (Sibling L V))
    (h : Sibling.uncontracted ∈ sibs ∨ Sibling.nonCall ∈ sibs) : siblingTree sibs = none := by
  induction sibs with
  | nil => simp at h
  | cons x rest ih =>
    cases x with
    | read t =>
      have h' : Sibling.uncontracted ∈ rest ∨ Sibling.nonCall ∈ rest := by
        rcases h with h | h <;> simp at h <;> simp [h]
      simp [siblingTree, ih h']
    | uncontracted => rfl
    | nonCall => rfl

/-- **An uncontracted sibling is refused.** -/
theorem an_uncontracted_sibling_is_refused {L V : Type} (u : Nat → Bool)
    (d a₀ a₁ : List (NestedCallEffects.Ev L V)) (sibs : List (Sibling L V))
    (h : Sibling.uncontracted ∈ sibs) : compose u d a₀ a₁ sibs = false := by
  simp [compose, unread_sibling_tree sibs (Or.inl h)]

/-- A sibling whose effect is not a call is refused. -/
theorem a_non_call_sibling_is_refused {L V : Type} (u : Nat → Bool)
    (d a₀ a₁ : List (NestedCallEffects.Ev L V)) (sibs : List (Sibling L V))
    (h : Sibling.nonCall ∈ sibs) : compose u d a₀ a₁ sibs = false := by
  simp [compose, unread_sibling_tree sibs (Or.inr h)]

/-- An admitted composition: every sibling read, and each arm's graph
admitted (so `composed_region_runs_alike_on_each_arm` applies). -/
theorem compose_admits {L V : Type} (u : Nat → Bool) (d a₀ a₁ : List (NestedCallEffects.Ev L V))
    (sibs : List (Sibling L V)) (h : compose u d a₀ a₁ sibs = true) :
    ∃ t, siblingTree sibs = some t ∧ NestedCallEffects.check u (armTree d a₀ t) = true ∧
      NestedCallEffects.check u (armTree d a₁ t) = true := by
  unfold compose at h
  split at h
  · cases h
  · rename_i t ht
    simp only [Bool.and_eq_true] at h
    exact ⟨t, ht, h.1, h.2⟩

/-- `footprintVectors` = `FOOTPRINT_VECTORS` (`connective_footprints.rs`): the
region of `subject`'s last statement composed over the fixture contracts. -/
def footprintVectors : List (String × Bool) := [
  ("cg_log(\"%d %d\", p ? r_borrow_code(p) : 0, r_release_code(r_wrap(p)));", true),
  ("cg_log(\"%d %d\", p ? r_borrow_code(p) : 0, r_release_code(p));", false),
  ("void *q = q_alloc(); cg_log(\"%d %d\", p ? r_borrow_code(p) : 0, r_release_code(q));", false),
  ("cg_log(\"%d %d\", p ? r_borrow_code(p) : 0, r_borrow_code(p));", true),
  ("cg_log(\"%d %d\", p ? r_borrow_code(p) : 0, opaque());", false),
  ("int n = 0; cg_log(\"%d %d\", p ? 1 : 0, n++);", false),
  ("cg_log(\"%d %d\", p ? r_release_code(p) : 0, r_borrow_code(p));", false),
  ("int n = 0; cg_log(\"%d %d\", n ? 1 : 0, cg_count(&n));", false)
]

theorem footprint_vectors_count : footprintVectors.length = 8 := by decide

end QCore.ValueConnectives

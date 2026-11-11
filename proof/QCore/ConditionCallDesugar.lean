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

/-! # Desugaring calls in a controlling expression (card W44-BRANCH-CONDITION-CALLS)

This is the abstract rule implemented by `legacy_condition_desugar.rs`. The C
reader and the Rust-to-Lean correspondence remain review premises (standing
constraint 28): this module proves properties of the rule, not extraction of
arbitrary C.

A refused controlling expression carrying calls is read as a Boolean tree over
`&&`, `||`, `!` and a top-level `?:`. Each **leaf** holds the calls C evaluates
for it, in evaluation order, and decides through one branch. The desugaring
(`assemble`) turns the tree into a decision structure whose connectives are
**edges**: the right operand of `&&` is entered only from the left operand's
true edge, that of `||` only from its false edge, the arms of `?:` only from
the matching edge of the condition, and `!` swaps a subtree's exits.

The leaf values are left free (`v : Nat → Bool`, one Boolean per leaf, indexed
in textual order): an admitted leaf's residual is resource-neutral, so the
projection does not decide it, and every assignment of leaf values is one CFG
path. The central theorem, `the_desugaring_is_c_evaluation`, says that for
**every** such assignment the desugared structure performs exactly the calls
C's short-circuit evaluation performs, in the same order, and leaves by the
same exit. Hence no call C skips is applied on any path, and every call C
evaluates is applied on the path where C evaluates it.

The admission conjunction (`admits`) records the premises the Rust checks: an
admitted shape, and per leaf modeled calls with resource-free results, callees
outside the enclosing function's own recursive component, sequenced effects,
a neutral residual and no tracked descriptor read as data. Each premise refuses
on its own. A custody result refuses, which is why `if ((p = malloc(n)) == NULL)`
keeps its refusal. Two calls of one leaf are admitted only when each is the
identity on the abstract state, and then their unspecified order is
unobservable (`identity_calls_commute`).
-/

namespace QCore.ConditionCallDesugar

/-- A controlling expression. A leaf carries its index (textual order) and the
calls C evaluates for it, in order. -/
inductive Cond (α : Type) where
  | leaf : Nat → List α → Cond α
  | and : Cond α → Cond α → Cond α
  | or : Cond α → Cond α → Cond α
  | not : Cond α → Cond α
  | choose : Cond α → Cond α → Cond α → Cond α
  deriving Repr

/-- C's short-circuit evaluation: the calls performed, in order, and the value,
for the leaf values `v`. -/
def eval {α : Type} (v : Nat → Bool) : Cond α → List α × Bool
  | .leaf i calls => (calls, v i)
  | .and a b =>
      let (ta, ra) := eval v a
      if ra then
        let (tb, rb) := eval v b
        (ta ++ tb, rb)
      else (ta, false)
  | .or a b =>
      let (ta, ra) := eval v a
      if ra then (ta, true)
      else
        let (tb, rb) := eval v b
        (ta ++ tb, rb)
  | .not a =>
      let (ta, ra) := eval v a
      (ta, !ra)
  | .choose c a b =>
      let (tc, rc) := eval v c
      if rc then
        let (t, r) := eval v a
        (tc ++ t, r)
      else
        let (t, r) := eval v b
        (tc ++ t, r)

/-- The desugared structure: call nodes, one decision node per leaf, and the
two exits. It is the Rust plan graph with shared targets unfolded, which
changes no path. -/
inductive Node (α : Type) where
  | call : α → Node α → Node α
  | decide : Nat → Node α → Node α → Node α
  | exitTrue : Node α
  | exitFalse : Node α
  deriving Repr

/-- The path through a desugared structure for the leaf values `v`: the calls
applied and the exit taken. -/
def run {α : Type} (v : Nat → Bool) : Node α → List α × Bool
  | .call x k =>
      let (t, r) := run v k
      (x :: t, r)
  | .decide i yes no => if v i then run v yes else run v no
  | .exitTrue => ([], true)
  | .exitFalse => ([], false)

/-- The desugaring (`assemble` in the Rust): a leaf's calls precede its
decision; the connectives become edges. -/
def assemble {α : Type} : Cond α → Node α → Node α → Node α
  | .leaf i calls, t, f => calls.foldr Node.call (Node.decide i t f)
  | .and a b, t, f => assemble a (assemble b t f) f
  | .or a b, t, f => assemble a t (assemble b t f)
  | .not a, t, f => assemble a f t
  | .choose c a b, t, f => assemble c (assemble a t f) (assemble b t f)

/-- The plan of a whole condition. -/
def desugar {α : Type} (c : Cond α) : Node α :=
  assemble c Node.exitTrue Node.exitFalse

theorem run_calls {α : Type} (v : Nat → Bool) (calls : List α) (k : Node α) :
    run v (calls.foldr Node.call k) = (calls ++ (run v k).1, (run v k).2) := by
  induction calls with
  | nil => simp [List.foldr]
  | cons x rest ih => simp [List.foldr, run, ih]

/-- The general form: running the structure for `c` followed by the
continuations `t`/`f` is C's evaluation of `c` followed by the continuation its
value selects. -/
theorem run_assemble {α : Type} (v : Nat → Bool) (c : Cond α) (t f : Node α) :
    run v (assemble c t f) =
      ((eval v c).1 ++ (run v (if (eval v c).2 then t else f)).1,
        (run v (if (eval v c).2 then t else f)).2) := by
  induction c generalizing t f with
  | leaf i calls =>
      simp only [assemble, run_calls, eval]
      by_cases h : v i <;> simp [run, h]
  | and a b iha ihb =>
      simp only [assemble, eval]
      rw [iha]
      by_cases ha : (eval v a).2 <;> simp [ha, ihb, List.append_assoc]
  | or a b iha ihb =>
      simp only [assemble, eval]
      rw [iha]
      by_cases ha : (eval v a).2 <;> simp [ha, ihb, List.append_assoc]
  | not a iha =>
      simp only [assemble, eval]
      rw [iha]
      by_cases ha : (eval v a).2 <;> simp [ha]
  | choose c a b ihc iha ihb =>
      simp only [assemble, eval]
      rw [ihc]
      by_cases hc : (eval v c).2 <;> simp [hc, iha, ihb, List.append_assoc]

/-- **The desugaring is C's evaluation.** For every assignment of leaf values,
the desugared plan applies exactly the calls C evaluates, in C's order, and
leaves by the exit C's value selects. -/
theorem the_desugaring_is_c_evaluation {α : Type} (v : Nat → Bool) (c : Cond α) :
    run v (desugar c) = eval v c := by
  unfold desugar
  rw [run_assemble]
  cases hev : eval v c with
  | mk calls value => cases value <;> simp [run]

/-- The right operand of `&&` is evaluated exactly on the left operand's true
edge. -/
theorem and_evaluates_its_right_operand_only_when_the_left_holds
    {α : Type} (v : Nat → Bool) (a b : Cond α) :
    (eval v (.and a b)).1 =
      (eval v a).1 ++ (if (eval v a).2 then (eval v b).1 else []) := by
  simp only [eval]
  by_cases h : (eval v a).2 <;> simp [h]

/-- The right operand of `||` is evaluated exactly on the left operand's false
edge. -/
theorem or_evaluates_its_right_operand_only_when_the_left_fails
    {α : Type} (v : Nat → Bool) (a b : Cond α) :
    (eval v (.or a b)).1 =
      (eval v a).1 ++ (if (eval v a).2 then [] else (eval v b).1) := by
  simp only [eval]
  by_cases h : (eval v a).2 <;> simp [h]

/-- Exactly one arm of `?:` is evaluated, the one the condition selects. -/
theorem choose_evaluates_exactly_the_selected_arm
    {α : Type} (v : Nat → Bool) (c a b : Cond α) :
    (eval v (.choose c a b)).1 =
      (eval v c).1 ++ (if (eval v c).2 then (eval v a).1 else (eval v b).1) := by
  simp only [eval]
  by_cases h : (eval v c).2 <;> simp [h]

/-- `!` changes no evaluation, only the exit. -/
theorem not_swaps_only_the_exit {α : Type} (v : Nat → Bool) (a : Cond α) :
    eval v (.not a) = ((eval v a).1, !(eval v a).2) := by
  simp [eval]

/-- A skipped operand's calls are applied on no path: when `a` is false, the
plan for `a && b` applies only `a`'s calls, whatever `b` contains. -/
theorem a_skipped_operand_applies_nothing
    {α : Type} (v : Nat → Bool) (a b : Cond α) (h : (eval v a).2 = false) :
    run v (desugar (.and a b)) = ((eval v a).1, false) := by
  rw [the_desugaring_is_c_evaluation]
  simp [eval, h]

/-- The non-vacuity witness: with a first leaf false, the release in the second
operand of `flag && (free(p), 1)` is not applied, so the live resource stays
visible to the leak check. -/
theorem the_short_circuited_release_is_not_applied :
    run (fun i => i != 0) (desugar (.and (.leaf 0 []) (.leaf 1 ["free p"]))) = ([], false) := by
  decide

/-- The same condition applies the release on the path where C evaluates it. -/
theorem the_evaluated_release_is_applied :
    run (fun _ => true) (desugar (.and (.leaf 0 []) (.leaf 1 ["free p"]))) =
      (["free p"], true) := by
  decide

/-! ## Loops

A loop's header is the plan's entry: every back edge and `continue` re-enters it,
so each visit applies C's evaluation of the condition afresh, and the loop is
left exactly on the visits whose value is false. -/

/-- The loop is left exactly when C's value of the condition is false. -/
theorem the_loop_exit_is_the_false_value {α : Type} (v : Nat → Bool) (c : Cond α) :
    (run v (desugar c)).2 = false ↔ (eval v c).2 = false := by
  rw [the_desugaring_is_c_evaluation]

/-- `k` visits to a loop header apply C's evaluation `k` times, in order. -/
def visits {α : Type} (vs : List (Nat → Bool)) (c : Cond α) : List α :=
  vs.flatMap fun v => (run v (desugar c)).1

theorem every_visit_re_evaluates_the_condition {α : Type} (vs : List (Nat → Bool))
    (c : Cond α) : visits vs c = vs.flatMap fun v => (eval v c).1 := by
  simp [visits, the_desugaring_is_c_evaluation]

/-! ## Admission -/

/-- The facts the Rust establishes for one leaf. -/
structure LeafFacts where
  /-- Every call is classified by the ordinary statement classifier to a
  contracted call (a declared contract or a validated W4 summary). -/
  callsModeled : Bool
  /-- Every call's result owns nothing (`none` or derived); an owned,
  reallocated or descriptor result — custody — refuses. -/
  resultsResourceFree : Bool
  /-- No locally defined callee lies in the enclosing function's recursive
  component, whose contract may be the neutral route's provisional `N(m)`. -/
  calleesOutsideOwnComponent : Bool
  /-- At most one call, or every call the identity on the abstract state. -/
  sequenced : Bool
  /-- The residual, with every call replaced by a neutral literal, is a
  custody-neutral expression or (for the one assigned scalar) a null test. -/
  residualNeutral : Bool
  /-- No tracked descriptor is read as data outside a validity test. -/
  noDescriptorData : Bool
  deriving DecidableEq, Repr

def LeafFacts.ok (l : LeafFacts) : Bool :=
  l.callsModeled && l.resultsResourceFree && l.calleesOutsideOwnComponent &&
    l.sequenced && l.residualNeutral && l.noDescriptorData

/-- One refused call-bearing condition. `shapeAdmitted` is the lexical gate: no
braces, type operator, increment, elvis, connective below a leaf or call through
an expression, and a switch expression is a single leaf. -/
structure Candidate where
  shapeAdmitted : Bool
  leaves : List LeafFacts
  deriving Repr

def admits (c : Candidate) : Bool :=
  c.shapeAdmitted && c.leaves.all LeafFacts.ok

theorem an_admitted_condition_has_an_admitted_shape (c : Candidate)
    (h : admits c = true) : c.shapeAdmitted = true := by
  simp [admits] at h
  exact h.1

theorem every_leaf_of_an_admitted_condition_is_ok (c : Candidate)
    (h : admits c = true) (l : LeafFacts) (hl : l ∈ c.leaves) : l.ok = true := by
  simp [admits] at h
  exact h.2 l hl

theorem a_refused_shape_refuses (leaves : List LeafFacts) :
    admits ⟨false, leaves⟩ = false := by
  simp [admits]

/-- One bad leaf refuses the whole condition: no unmodeled call hides beside
admitted ones. -/
theorem one_refused_leaf_refuses_the_condition (shape : Bool) (before after : List LeafFacts)
    (l : LeafFacts) (hl : l.ok = false) :
    admits ⟨shape, before ++ l :: after⟩ = false := by
  cases shape <;> simp [admits]
  intro _ h
  simp [hl] at h

theorem an_unmodeled_call_refuses (l : LeafFacts) (h : l.callsModeled = false) :
    l.ok = false := by
  simp [LeafFacts.ok, h]

/-- A custody result refuses: `if ((p = malloc(n)) == NULL)` keeps its refusal. -/
theorem a_custody_result_refuses (l : LeafFacts) (h : l.resultsResourceFree = false) :
    l.ok = false := by
  simp [LeafFacts.ok, h]

theorem a_callee_in_the_own_component_refuses (l : LeafFacts)
    (h : l.calleesOutsideOwnComponent = false) : l.ok = false := by
  simp [LeafFacts.ok, h]

theorem unsequenced_effects_refuse (l : LeafFacts) (h : l.sequenced = false) :
    l.ok = false := by
  simp [LeafFacts.ok, h]

theorem a_non_neutral_residual_refuses (l : LeafFacts) (h : l.residualNeutral = false) :
    l.ok = false := by
  simp [LeafFacts.ok, h]

theorem a_descriptor_read_as_data_refuses (l : LeafFacts) (h : l.noDescriptorData = false) :
    l.ok = false := by
  simp [LeafFacts.ok, h]

theorem a_leaf_with_every_premise_is_ok :
    LeafFacts.ok ⟨true, true, true, true, true, true⟩ = true := by
  decide

/-- Calls whose transfer is the identity commute: C's unspecified order of two
calls in one operand is unobservable when every one of them is the identity. -/
theorem identity_calls_commute {State α : Type} (transfer : α → State → State)
    (calls : List α) (h : ∀ x ∈ calls, transfer x = id) (state : State) :
    calls.foldl (fun s x => transfer x s) state = state := by
  induction calls generalizing state with
  | nil => rfl
  | cons x rest ih =>
      simp only [List.foldl]
      rw [h x (by simp)]
      exact ih (fun y hy => h y (by simp [hy])) state

/-- A decision node on a resource-neutral residual refines nothing: both of its
successors receive the state after the leaf's calls. -/
def neutralSuccessors {State : Type} (state : State) : List State := [state, state]

theorem a_neutral_decision_keeps_both_edges {State : Type} (state : State) :
    neutralSuccessors state = List.replicate 2 state := by
  simp [neutralSuccessors]

end QCore.ConditionCallDesugar

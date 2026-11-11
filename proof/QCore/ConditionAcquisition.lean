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

import QCore.ConditionCallDesugar
import QCore.CallInstantiation

/-!
# Acquisitions in conditions and complex condition leaves (card CG04-CONDITION-ACQUISITION, rule 10x)

The abstract rule implemented by `crates/qcore-c-gateway/src/condition_acquisition.rs`
(`leaf_use`, `carrier`, `leaf_route`) and its route-G adapter
`legacy_condition_acquisition.rs`, inside the condition desugaring of
`legacy_condition_desugar.rs` (rule 5h, `QCore.ConditionCallDesugar`).

## The NULL split

`if ((p = malloc(n)) != NULL) free(p);` is the leaf `(p = malloc(n)) != NULL`:
the call, bound to `p`, then the leaf's branch, the null test of `p`. The value
of the call is stored and tested against the contract's failure value where it
stands, the use `testedSplit` (`QCore.CallInstantiation.Use`), which the
acceptance takes for every result class. The five parts are lowered together by
the one existing flow; this module states the facts that make that sound:

* **Custody.** An acquisition either succeeds, binding a fresh identity whose
  obligation is live, or fails, binding the failure value (NULL, or a
  descriptor's sentinel) with no obligation. The split's edge is decided by the
  bound value, so the refinement of each edge keeps only its compatible
  outcomes: exactly one edge carries the obligation
  (`exactly_one_edge_carries_the_obligation`), and every outcome reaches exactly
  one edge (`every_outcome_reaches_one_edge`).
* **Reallocation.** The failure arm keeps the old identity live and binds no
  new one; success consumes the old and makes the new live
  (`on_the_failed_edge_the_old_identity_is_live`).
* **Evaluation.** The acquisition is a call of its leaf in the desugaring's
  structure, never hoisted: every path applies it exactly as often as C
  evaluates it (`the_acquisition_is_applied_as_c_evaluates_it`), and the right
  operand of `&&` acquires nothing on a path whose left operand is false
  (`an_unevaluated_operand_acquires_nothing`).
* **Carrier.** A custody result is split only on its own carrier's failure
  value (`carrier`).

## Complex leaves

`leafRoute` replaces the whole-condition refusals of a type operator and an
increment by a per-leaf decision; a type operator's operand is not evaluated,
so its calls are never applied (`a_type_operand_applies_no_call`), and under
admission no call of a leaf is dropped (`an_admitted_leaf_applies_every_call`).
Unlisted shapes refuse (`only_listed_leaves_are_admitted`).

## What this is not

A model of the decision tables and of custody at the split; the flow's own
transfer (`BranchOnNull`, `BranchOnValidity`, the reallocation arm) is the
existing one, unchanged. Faithful extraction of C and the Rust-Lean
correspondence are review premises (trust boundary 28); `useVectors`,
`carrierVectors` and `leafVectors` bind the Rust tables mechanically
(`tests/unit/condition_acquisition.rs`).
-/

namespace QCore.ConditionAcquisition

open QCore.ConditionCallDesugar
open QCore.CallInstantiation (Use ResultClass Standing Decision accept)

/-! ## The use of a leaf's value -/

/-- How a leaf tests the destination of its one assigned call. -/
inductive Test where
  | null
  | sentinel
  | none
  deriving DecidableEq, Repr

/-- `leaf_use`: a bound value the leaf splits on is the NULL split use. -/
def leafUse (bound : Bool) (t : Test) : Use :=
  if bound && t != .none then .testedSplit else .tested

/-- `USE_VECTORS` of `condition_acquisition.rs`, row by row. -/
def useVectors : List (Bool × Test × Use) := [
  (true, .null, .testedSplit),
  (true, .sentinel, .testedSplit),
  (true, .none, .tested),
  (false, .null, .tested),
  (false, .none, .tested)
]

theorem use_vectors_hold :
    useVectors.all (fun (b, t, u) => leafUse b t == u) = true := by
  decide

/-- The widened row: a split value is accepted whatever its result class,
unless the callee is in the caller's recursive component. -/
theorem the_split_use_is_accepted (s : Standing) (hs : s ≠ .recursive) (r : ResultClass)
    (identity : Bool) : accept s r identity .testedSplit = .accept := by
  cases s <;> simp_all [accept]

/-- A custody result the leaf does not split on keeps its refusal: a discarded
acquisition (`if (malloc(n))`) or an unsplit one. -/
theorem an_unsplit_custody_result_keeps_its_refusal (s : Standing) (hs : s ≠ .recursive)
    (bound : Bool) (t : Test) (h : bound = false ∨ t = .none) (identity : Bool) :
    accept s .owned identity (leafUse bound t) = .custodyResult ∧
      accept s .reallocated identity (leafUse bound t) = .custodyResult := by
  have hu : leafUse bound t = .tested := by
    rcases h with h | h <;> simp [leafUse, h]
  rw [hu]
  cases s <;> simp_all [accept, ResultClass.resourceFree]

/-- A recursive-component callee stays refused in the split use. -/
theorem a_recursive_callee_stays_refused (r : ResultClass) (identity : Bool) :
    accept .recursive r identity .testedSplit = .recursiveLocal := by
  simp [accept]

/-! ## The carrier -/

inductive Split where
  | null
  | sentinel
  deriving DecidableEq, Repr

inductive CarrierDecision where
  | accept
  | carrierMismatch
  deriving DecidableEq, Repr

/-- `carrier`: a custody result is split on its own carrier's failure value. -/
def carrier (r : ResultClass) (integer : Bool) (sp : Split) : CarrierDecision :=
  if r.resourceFree then .accept
  else
    match integer, sp with
    | false, .null => .accept
    | true, .sentinel => .accept
    | _, _ => .carrierMismatch

/-- `CARRIER_VECTORS` of `condition_acquisition.rs`, row by row. -/
def carrierVectors : List (ResultClass × Bool × Split × CarrierDecision) := [
  (.owned, false, .null, .accept),
  (.owned, true, .sentinel, .accept),
  (.reallocated, false, .null, .accept),
  (.owned, true, .null, .carrierMismatch),
  (.owned, false, .sentinel, .carrierMismatch),
  (.reallocated, true, .null, .carrierMismatch),
  (.none, false, .null, .accept),
  (.derived, false, .null, .accept)
]

theorem carrier_vectors_hold :
    carrierVectors.all (fun (r, i, sp, d) => carrier r i sp == d) = true := by
  decide

/-- An accepted custody split tests its own carrier: a pointer's NULL or an
integer descriptor's sentinel. -/
theorem a_custody_split_tests_its_own_carrier (r : ResultClass) (integer : Bool) (sp : Split)
    (hr : r.resourceFree = false) (h : carrier r integer sp = .accept) :
    (integer = false ∧ sp = .null) ∨ (integer = true ∧ sp = .sentinel) := by
  cases integer <;> cases sp <;> simp_all [carrier]

/-! ## Custody at the split -/

/-- An acquisition's outcome, which the contract declares: a fresh identity,
or the failure value. -/
inductive Outcome where
  | success
  | failure
  deriving DecidableEq, Repr

/-- The value bound to the destination. -/
inductive Value where
  | fresh
  | failureValue
  deriving DecidableEq, Repr

def bind : Outcome → Value
  | .success => .fresh
  | .failure => .failureValue

/-- The obligation the call creates is live exactly on success. -/
def obligation : Outcome → Bool
  | .success => true
  | .failure => false

/-- The split's edges: the value is not the failure value (`!= NULL`, valid),
or it is. -/
inductive Edge where
  | ok
  | failed
  deriving DecidableEq, Repr

/-- The edge the split takes on the bound value. -/
def edgeOf : Value → Edge
  | .fresh => .ok
  | .failureValue => .failed

/-- The refinement of an edge: the outcomes compatible with arriving on it. -/
def feasible (e : Edge) (o : Outcome) : Bool :=
  edgeOf (bind o) == e

/-- Whether an edge carries the obligation after its refinement. -/
def carries (e : Edge) : Bool :=
  [Outcome.success, Outcome.failure].any fun o => feasible e o && obligation o

/-- **The NULL split preserves custody**: exactly one edge carries the
obligation. -/
theorem exactly_one_edge_carries_the_obligation :
    [Edge.ok, Edge.failed].countP carries = 1 := by
  decide

theorem the_ok_edge_carries_the_obligation : carries .ok = true := by decide

theorem the_failed_edge_carries_nothing : carries .failed = false := by decide

/-- Every outcome reaches exactly one edge: the split drops no path. -/
theorem every_outcome_reaches_one_edge (o : Outcome) :
    [Edge.ok, Edge.failed].countP (fun e => feasible e o) = 1 := by
  cases o <;> decide

/-- On the edge an outcome reaches, the obligation is live exactly when it is
the outcome's own: the merge after the split joins the two refined states. -/
theorem the_obligation_follows_the_outcome (o : Outcome) :
    (feasible (edgeOf (bind o)) o && obligation o) = obligation o := by
  cases o <;> decide

/-! ## Placement: no split acquisition on a cycle -/

inductive PlacementDecision where
  | accept
  | acquisitionOnCycle
  deriving DecidableEq, Repr

/-- `placement`: a custody split on a node a CFG cycle may pass through only
when the flow keeps the site's generations apart (`generations`). Card
CG04-LOOP-SITE-GENERATIONS gives the flow generation-indexed sites
(`QCore.LoopSiteGenerations`), so the production call passes `true` and the
guard is lifted. -/
def placement (custody onCycle generations : Bool) : PlacementDecision :=
  if custody && onCycle && !generations then .acquisitionOnCycle else .accept

/-- `PLACEMENT_VECTORS` of `condition_acquisition.rs`, row by row. -/
def placementVectors : List (Bool × Bool × Bool × PlacementDecision) := [
  (true, false, false, .accept),
  (true, true, false, .acquisitionOnCycle),
  (false, true, false, .accept),
  (false, false, false, .accept),
  (true, false, true, .accept),
  (true, true, true, .accept),
  (false, true, true, .accept),
  (false, false, true, .accept)
]

theorem placement_vectors_hold :
    placementVectors.all (fun (c, o, g, d) => placement c o g == d) = true := by
  decide

/-- With generation-indexed sites every split is placed. -/
theorem with_generations_every_split_is_placed (custody onCycle : Bool) :
    placement custody onCycle true = .accept := by
  cases custody <;> cases onCycle <;> rfl

/-- The flow keeps one abstract site per acquisition node. On a path that
visits no node twice (no cycle passes through it), that site stands for at
most one runtime generation. -/
theorem off_every_cycle_a_site_has_one_generation (path : List Nat) (node : Nat)
    (h : path.Nodup) : path.count node ≤ 1 :=
  List.nodup_iff_count.mp h node

/-- Why the guard: on a cycle, a live earlier generation and the fresh one share
the site. After a failed re-acquisition the refined failed edge carries nothing
(`the_failed_edge_carries_nothing`), while the earlier generation is still live
concretely: the merge would hide a leak. -/
theorem a_merged_site_would_hide_a_live_generation :
    carries .failed = false ∧ (1 + (if obligation .failure then 1 else 0)) = 1 := by
  decide

/-! ## Reallocation -/

/-- Live obligations after a reallocation: the old identity and the new one. -/
structure Live where
  old : Bool
  new : Bool
  deriving DecidableEq, Repr

/-- The contract's two arms: success moves custody to the new identity; the
failure arm keeps the old identity live. -/
def realloc : Outcome → Live
  | .success => ⟨false, true⟩
  | .failure => ⟨true, false⟩

/-- **The realloc failure arm keeps the old identity.** -/
theorem on_the_failed_edge_the_old_identity_is_live (o : Outcome)
    (h : feasible .failed o = true) : realloc o = ⟨true, false⟩ := by
  cases o <;> simp_all [feasible, bind, edgeOf, realloc]

theorem on_the_ok_edge_only_the_new_identity_is_live (o : Outcome)
    (h : feasible .ok o = true) : realloc o = ⟨false, true⟩ := by
  cases o <;> simp_all [feasible, bind, edgeOf, realloc]

/-- Exactly one of the two identities is live on every path. -/
theorem exactly_one_identity_is_live (o : Outcome) :
    ((realloc o).old != (realloc o).new) = true := by
  cases o <;> decide

/-! ## Evaluation: once, in position, never under a skipped operand -/

/-- Every call a condition holds, syntactically. -/
def allCalls {α : Type} : Cond α → List α
  | .leaf _ calls => calls
  | .and a b => allCalls a ++ allCalls b
  | .or a b => allCalls a ++ allCalls b
  | .not a => allCalls a
  | .choose c a b => allCalls c ++ allCalls a ++ allCalls b

/-- C evaluates only calls the condition holds. -/
theorem evaluated_calls_are_held {α : Type} (v : Nat → Bool) (c : Cond α) (x : α)
    (h : x ∈ (eval v c).1) : x ∈ allCalls c := by
  induction c with
  | leaf i calls => simpa [eval, allCalls] using h
  | and a b iha ihb =>
      simp only [eval] at h
      by_cases ha : (eval v a).2
      · simp only [ha, ite_true, List.mem_append] at h
        rcases h with h | h
        · exact List.mem_append.mpr (Or.inl (iha h))
        · exact List.mem_append.mpr (Or.inr (ihb h))
      · simp only [ha, Bool.false_eq_true, ite_false] at h
        exact List.mem_append.mpr (Or.inl (iha h))
  | or a b iha ihb =>
      simp only [eval] at h
      by_cases ha : (eval v a).2
      · simp only [ha, ite_true] at h
        exact List.mem_append.mpr (Or.inl (iha h))
      · simp only [ha, Bool.false_eq_true, ite_false, List.mem_append] at h
        rcases h with h | h
        · exact List.mem_append.mpr (Or.inl (iha h))
        · exact List.mem_append.mpr (Or.inr (ihb h))
  | not a iha =>
      simp only [eval] at h
      exact iha h
  | choose c a b ihc iha ihb =>
      simp only [eval] at h
      by_cases hc : (eval v c).2
      · simp only [hc, ite_true, List.mem_append] at h
        rcases h with h | h
        · simp [allCalls, ihc h]
        · simp [allCalls, iha h]
      · simp only [hc, Bool.false_eq_true, ite_false, List.mem_append] at h
        rcases h with h | h
        · simp [allCalls, ihc h]
        · simp [allCalls, ihb h]

/-- **Evaluated once, in position.** On every path the desugared plan applies
an acquisition exactly as often, and at the same place among the calls, as C
evaluates it: the plan's calls are C's (`the_desugaring_is_c_evaluation`). -/
theorem the_acquisition_is_applied_as_c_evaluates_it {α : Type} [BEq α] (v : Nat → Bool)
    (c : Cond α) (x : α) :
    (run v (desugar c)).1.count x = (eval v c).1.count x ∧
      (run v (desugar c)).1 = (eval v c).1 := by
  rw [the_desugaring_is_c_evaluation]
  exact ⟨rfl, rfl⟩

/-- A leaf holding one acquisition applies it exactly once. -/
theorem an_acquisition_leaf_applies_its_call_once {α : Type} (v : Nat → Bool) (i : Nat) (x : α) :
    (run v (desugar (.leaf i [x]))).1 = [x] := by
  rw [the_desugaring_is_c_evaluation]
  rfl

/-- **Short-circuit.** An acquisition in the right operand of `&&` is applied on
no path whose left operand is false. -/
theorem an_unevaluated_operand_acquires_nothing {α : Type} (v : Nat → Bool) (a b : Cond α)
    (x : α) (hx : x ∉ allCalls a) (ha : (eval v a).2 = false) :
    x ∉ (run v (desugar (.and a b))).1 := by
  rw [a_skipped_operand_applies_nothing v a b ha]
  exact fun h => hx (evaluated_calls_are_held v a x h)

/-- The right operand of `&&` acquires exactly on the paths that evaluate it. -/
theorem the_right_operand_acquires_only_when_the_left_holds {α : Type} (v : Nat → Bool)
    (a b : Cond α) (x : α) (hx : x ∉ allCalls a) :
    x ∈ (run v (desugar (.and a b))).1 ↔ (eval v a).2 = true ∧ x ∈ (eval v b).1 := by
  rw [the_desugaring_is_c_evaluation]
  have hna : x ∉ (eval v a).1 := fun h => hx (evaluated_calls_are_held v a x h)
  simp only [eval]
  by_cases ha : (eval v a).2
  · simp [ha, hna]
  · simp [ha, hna]

/-- The non-vacuity witness: `flag && (p = malloc(n)) != NULL` with `flag`
false acquires nothing; with `flag` true it acquires once. -/
theorem the_short_circuited_acquisition_is_not_applied :
    run (fun i => i != 0) (desugar (.and (.leaf 0 []) (.leaf 1 ["acquire p"]))) = ([], false) ∧
      run (fun _ => true) (desugar (.and (.leaf 0 []) (.leaf 1 ["acquire p"]))) =
        (["acquire p"], true) := by
  decide

/-! ## Complex leaves -/

structure LeafFeatures where
  call : Bool
  callInTypeOperand : Bool
  increment : Bool
  connective : Bool
  deriving DecidableEq, Repr

inductive LeafDecision where
  | calls
  | callFree
  | callInTypeOperand
  | incrementBesideCall
  | connectiveBesideCall
  deriving DecidableEq, Repr

/-- `leaf_route`. -/
def leafRoute (f : LeafFeatures) : LeafDecision :=
  if f.callInTypeOperand then .callInTypeOperand
  else if !f.call then .callFree
  else if f.increment then .incrementBesideCall
  else if f.connective then .connectiveBesideCall
  else .calls

/-- `LEAF_VECTORS` of `condition_acquisition.rs`, row by row
(call, call in a type operand, increment, connective). -/
def leafVectors : List (Bool × Bool × Bool × Bool × LeafDecision) := [
  (true, false, false, false, .calls),
  (false, false, false, false, .callFree),
  (false, false, true, false, .callFree),
  (false, false, false, true, .callFree),
  (true, false, true, false, .incrementBesideCall),
  (true, false, false, true, .connectiveBesideCall),
  (true, true, false, false, .callInTypeOperand),
  (false, true, false, false, .callInTypeOperand),
  (true, false, true, true, .incrementBesideCall)
]

theorem leaf_vectors_hold :
    leafVectors.all (fun (c, t, i, n, d) => leafRoute ⟨c, t, i, n⟩ == d) = true := by
  decide

/-- **Unlisted shapes are refused.** A leaf is admitted to the calls route only
with a call and none of the refused features, and to the call-free route only
without a call anywhere (a call-free leaf's increment is the elaborator's to
decide, rule 10s). Every other combination is a named refusal. -/
theorem only_listed_leaves_are_admitted (f : LeafFeatures) :
    (leafRoute f = .calls ↔
        f.call = true ∧ f.callInTypeOperand = false ∧ f.increment = false ∧
          f.connective = false) ∧
      (leafRoute f = .callFree ↔ f.call = false ∧ f.callInTypeOperand = false) := by
  obtain ⟨c, t, i, n⟩ := f
  cases c <;> cases t <;> cases i <;> cases n <;> decide

theorem a_call_in_a_type_operand_is_refused (f : LeafFeatures) (h : f.callInTypeOperand = true) :
    leafRoute f = .callInTypeOperand := by
  simp [leafRoute, h]

theorem an_increment_beside_a_call_is_refused (f : LeafFeatures)
    (ht : f.callInTypeOperand = false) (hc : f.call = true) (hi : f.increment = true) :
    leafRoute f = .incrementBesideCall := by
  simp [leafRoute, ht, hc, hi]

/-- A leaf expression: a call with its actuals, a type operator's operand, an
operator, or a literal. -/
inductive Expr where
  | call (callee : Nat) (actuals : List Expr)
  | typeOp (operand : Expr)
  | op (left right : Expr)
  | lit
  deriving Repr

mutual
/-- The calls C evaluates: a type operator's operand is not evaluated
(C11 6.5.3.4p2; a variable-length operand holds no call once admitted). -/
def evaluated : Expr → List Nat
  | .call callee actuals => evaluatedAll actuals ++ [callee]
  | .typeOp _ => []
  | .op l r => evaluated l ++ evaluated r
  | .lit => []
def evaluatedAll : List Expr → List Nat
  | [] => []
  | e :: es => evaluated e ++ evaluatedAll es
end

mutual
/-- Every call the leaf holds, syntactically. -/
def held : Expr → List Nat
  | .call callee actuals => heldAll actuals ++ [callee]
  | .typeOp e => held e
  | .op l r => held l ++ held r
  | .lit => []
def heldAll : List Expr → List Nat
  | [] => []
  | e :: es => held e ++ heldAll es
end

mutual
/-- `leaf_route`'s premise: no call inside any type operand. -/
def admitted : Expr → Bool
  | .call _ actuals => admittedAll actuals
  | .typeOp e => held e == []
  | .op l r => admitted l && admitted r
  | .lit => true
def admittedAll : List Expr → Bool
  | [] => true
  | e :: es => admitted e && admittedAll es
end

/-- **A type operand applies no call**, whatever it holds. -/
theorem a_type_operand_applies_no_call (e : Expr) : evaluated (.typeOp e) = [] := rfl

mutual
/-- Under admission no call of a leaf is dropped: every call it holds is one C
evaluates, so the plan applies it. -/
theorem an_admitted_leaf_applies_every_call : ∀ e : Expr, admitted e = true →
    evaluated e = held e
  | .call callee actuals, h => by
      simp only [admitted] at h
      simp [evaluated, held, an_admitted_list_applies_every_call actuals h]
  | .typeOp e, h => by
      simp only [admitted, beq_iff_eq] at h
      simp [evaluated, held, h]
  | .op l r, h => by
      simp only [admitted, Bool.and_eq_true] at h
      simp [evaluated, held, an_admitted_leaf_applies_every_call l h.1,
        an_admitted_leaf_applies_every_call r h.2]
  | .lit, _ => rfl
theorem an_admitted_list_applies_every_call : ∀ es : List Expr, admittedAll es = true →
    evaluatedAll es = heldAll es
  | [], _ => rfl
  | e :: es, h => by
      simp only [admittedAll, Bool.and_eq_true] at h
      simp [evaluatedAll, heldAll, an_admitted_leaf_applies_every_call e h.1,
        an_admitted_list_applies_every_call es h.2]
end

/-- The witness: `n < sizeof(f(p))` holds the call `f` and applies nothing,
which is why such a leaf is refused rather than read with `f` applied. -/
theorem a_call_in_a_type_operand_is_held_but_not_evaluated :
    held (.op .lit (.typeOp (.call 7 [.lit]))) = [7] ∧
      evaluated (.op .lit (.typeOp (.call 7 [.lit]))) = [] ∧
      admitted (.op .lit (.typeOp (.call 7 [.lit]))) = false := by
  decide

end QCore.ConditionAcquisition

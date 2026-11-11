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

import QCore.TypedPlace

/-!
# One call-contract application point (card CG03-CALL-SUMMARY-INSTANTIATION, rule 10u)

The abstract rule implemented by `crates/qcore-c-gateway/src/call_instantiation.rs`
(`accept`, `substitute`) and its route-G adapter `legacy_call_instantiation.rs`
(`Parser::instantiate_call` and the three consumers it adds).

## Acceptance

A call's contract is instantiated once: callee → the contract in force → one
substitution per formal → the effects the flow applies. Whether a consumer
takes the instantiated call is `accept`, a function of the callee's standing,
the contract's result class, whether its effect is the identity on the abstract
state, and the use C makes of the call's value. The consumer is not an
argument (`decideAt` ignores it), so two positions that use a value the same
way decide it the same way. The condition desugaring's older gate (rule 5h) is
exactly the tested acceptance, and B01's older return gate is contained in it:
the return operand now takes a local callee outside the caller's recursive
component, and a derived result, exactly as a condition leaf does.

## Substitution

`read` is the typed reading of one actual (value → identity → formal):
a named pointer object through value-preserving pointer conversions is its
identity, `&x` is `x`'s address, an expression whose evaluated reads are all
scalars is data, a member/subscript/indirection is a place read by
`QCore.TypedPlace.readPlace`, anything else is refused. `substitute` is total
or rejected: one refused formal refuses the whole call.

## What this is not

A model of the decision and the substitution. Faithful Clang extraction, the
value preservation of a bit cast between object pointers (C11 6.3.2.3p7), and
the Rust-Lean correspondence are review premises (trust boundary 28);
`acceptanceVectors` and `substitutionVectors` bind the Rust tables mechanically
(`tests/unit/call_instantiation.rs`). The contract's own clauses are applied by
the existing flow (`apply_contract_call`), unchanged.
-/

namespace QCore.CallInstantiation

/-! ## Acceptance -/

inductive Standing where
  | declared
  | validated
  | recursive
  deriving DecidableEq, Repr

inductive ResultClass where
  | none
  | derived
  | owned
  | reallocated
  | removed
  deriving DecidableEq, Repr

inductive Use where
  | discarded
  | stored
  | tested
  | returned
  | passed
  /-- Card CG04-CONDITION-ACQUISITION (rule 10x): stored in its destination
  and tested there against the contract's failure value (the NULL split). -/
  | testedSplit
  deriving DecidableEq, Repr

inductive Decision where
  | accept
  | recursiveLocal
  | custodyResult
  | returnedAlias
  | notIdentity
  /-- Card CG05-NESTED-CALL-EFFECTS: a passed reallocated result. -/
  | passedReallocation
  deriving DecidableEq, Repr

/-- Owns nothing: no result, or a derived pointer. -/
def ResultClass.resourceFree : ResultClass → Bool
  | .none => true
  | .derived => true
  | _ => false

/-- `call_instantiation::accept`. -/
def accept (s : Standing) (r : ResultClass) (identity : Bool) (u : Use) : Decision :=
  if s = .recursive then .recursiveLocal
  else
    match u with
    | .stored => .accept
    | .discarded => if r.resourceFree then .accept else .custodyResult
    | .tested => if r.resourceFree then .accept else .custodyResult
    | .returned =>
      match r with
      | .none => .accept
      | .owned => .accept
      | _ => .returnedAlias
    | .passed =>
      if identity && r == .none then .accept
      else if r != .reallocated && r != .removed then .accept
      else .passedReallocation
    | .testedSplit => .accept

/-- `ACCEPTANCE_VECTORS` of `call_instantiation.rs`, row by row. -/
def acceptanceVectors : List (Standing × ResultClass × Bool × Use × Decision) := [
  (.declared, .removed, false, .stored, .accept),
  (.declared, .removed, false, .passed, .passedReallocation),
  (.declared, .removed, false, .returned, .returnedAlias),
  (.declared, .removed, false, .discarded, .custodyResult),
  (.declared, .none, true, .tested, .accept),
  (.validated, .none, false, .tested, .accept),
  (.validated, .derived, true, .tested, .accept),
  (.recursive, .none, true, .tested, .recursiveLocal),
  (.declared, .owned, false, .tested, .custodyResult),
  (.validated, .reallocated, false, .discarded, .custodyResult),
  (.declared, .derived, true, .discarded, .accept),
  (.validated, .owned, false, .stored, .accept),
  (.recursive, .owned, false, .stored, .recursiveLocal),
  (.declared, .owned, false, .returned, .accept),
  (.declared, .derived, true, .returned, .returnedAlias),
  (.declared, .reallocated, false, .returned, .returnedAlias),
  (.declared, .none, true, .passed, .accept),
  (.declared, .none, false, .passed, .accept),
  (.declared, .derived, true, .passed, .accept),
  (.declared, .owned, false, .testedSplit, .accept),
  (.validated, .reallocated, false, .testedSplit, .accept),
  (.declared, .derived, true, .testedSplit, .accept),
  (.declared, .none, true, .testedSplit, .accept),
  (.recursive, .owned, false, .testedSplit, .recursiveLocal),
  (.validated, .owned, false, .passed, .accept),
  (.declared, .reallocated, false, .passed, .passedReallocation),
  (.recursive, .owned, false, .passed, .recursiveLocal)
]

theorem acceptance_vectors_hold :
    acceptanceVectors.all (fun (s, r, i, u, d) => accept s r i u == d) = true := by
  decide

/-- The consumers that ask. -/
inductive Consumer where
  | statement
  | condition
  | returnOperand
  | nested
  | elaborated
  deriving DecidableEq, Repr

/-- A consumer's decision on an instantiated call whose value it uses as `u`. -/
def decideAt (_c : Consumer) (s : Standing) (r : ResultClass) (identity : Bool) (u : Use) :
    Decision :=
  accept s r identity u

/-- Position independence: the consumer never decides, the use does. -/
theorem acceptance_is_position_independent (c₁ c₂ : Consumer) (s : Standing) (r : ResultClass)
    (identity : Bool) (u : Use) : decideAt c₁ s r identity u = decideAt c₂ s r identity u := by
  simp [decideAt]

/-- The condition desugaring's older gate (rule 5h): no recursive callee, a
resource-free result. -/
def conditionGate (s : Standing) (r : ResultClass) : Bool :=
  s != .recursive && r.resourceFree

/-- B01's older scalar-return gate: an externally declared callee, no result. -/
def b01Gate (s : Standing) (r : ResultClass) : Bool :=
  s == .declared && r == .none

/-- The return operand takes exactly what the condition leaf took. -/
theorem the_return_operand_takes_what_the_condition_leaf_takes (s : Standing) (r : ResultClass)
    (identity : Bool) :
    (decideAt .returnOperand s r identity .tested == .accept) = conditionGate s r := by
  cases s <;> cases r <;> cases identity <;> decide

/-- Every B01 admission stands. -/
theorem every_b01_admission_stands (s : Standing) (r : ResultClass) (identity : Bool)
    (h : b01Gate s r = true) : decideAt .returnOperand s r identity .tested = .accept := by
  cases s <;> cases r <;> cases identity <;> simp_all [b01Gate, decideAt, accept] <;> decide

/-- What the alignment adds over B01: a validated local callee, or a derived
result; nothing else. -/
theorem the_widening_is_a_validated_callee_or_a_derived_result (s : Standing) (r : ResultClass)
    (identity : Bool) (h₁ : decideAt .returnOperand s r identity .tested = .accept)
    (h₂ : b01Gate s r = false) : s = .validated ∨ r = .derived := by
  cases s <;> cases r <;> cases identity <;> simp_all [b01Gate, decideAt, accept] <;>
    first | decide | (revert h₁; decide)

/-- A callee in the caller's recursive component is refused in every use. -/
theorem a_recursive_callee_is_refused_in_every_use (r : ResultClass) (identity : Bool) (u : Use) :
    accept .recursive r identity u = .recursiveLocal := by
  simp [accept]

/-- A tested or discarded value that owns something is refused. -/
theorem a_custody_result_is_refused_where_it_is_tested (s : Standing) (identity : Bool)
    (hs : s ≠ .recursive) :
    accept s .owned identity .tested = .custodyResult ∧
      accept s .reallocated identity .tested = .custodyResult ∧
      accept s .owned identity .discarded = .custodyResult := by
  cases s <;> simp_all [accept, ResultClass.resourceFree]

/-- Card CG05-NESTED-CALL-EFFECTS widened the passed row: a passed value is
accepted unless its callee is recursive or its result is a reallocation (the
nested-call consumer binds it to a fresh reserved place, or reads it as data,
and applies it inside-out under its footprint order check,
`QCore.NestedCallEffects`). -/
theorem a_passed_value_is_not_a_reallocation (s : Standing) (r : ResultClass) (identity : Bool)
    (h : accept s r identity .passed = .accept) : s ≠ .recursive ∧ r ≠ .reallocated := by
  cases s <;> cases r <;> cases identity <;> simp_all [accept] <;> revert h <;> decide

/-- Every B04 admission (the identity with no result) stands. -/
theorem every_b04_admission_stands (s : Standing) (hs : s ≠ .recursive) :
    accept s .none true .passed = .accept := by
  cases s <;> simp_all [accept]

/-! ## Instantiation: one summary, one substitution, total or rejected -/

inductive Mode where
  | borrowed
  | owned
  | consumed
  | released
  | out
  | stored
  | removed
  deriving DecidableEq, Repr

/-- A custody-taking formal: anything but `Borrowed`. -/
def Mode.custody : Mode → Bool
  | .borrowed => false
  | _ => true

/-- The typed reading of one actual. -/
inductive Actual where
  | identity (x : Nat) (cast : Bool)
  | address (x : Nat)
  | data
  | place
  | other
  deriving DecidableEq, Repr

/-- One formal's substitution. -/
inductive Subst where
  | ident (x : Nat)
  | slot (x : Nat)
  | observed (x : Nat)
  | data
  | place
  | reject
  deriving DecidableEq, Repr

/-- `substitute`'s per-formal rule. -/
def substOne : Mode → Actual → Subst
  | .out, .identity x _ => .slot x
  | _, .identity x _ => .ident x
  | .out, .address x => .slot x
  | .borrowed, .address x => .observed x
  | _, .address _ => .reject
  | .borrowed, .data => .data
  | _, .data => .reject
  | .borrowed, .place => .place
  | _, .place => .reject
  | _, .other => .reject

def Subst.refused : Subst → Bool
  | .reject => true
  | _ => false

/-- Every formal substituted, or the whole call rejected. -/
def substitute (modes : List Mode) (actuals : List Actual) : Option (List Subst) :=
  if modes.length = actuals.length then
    let σ := List.zipWith substOne modes actuals
    if σ.any Subst.refused then none else some σ
  else none

/-- The instantiation: the one contract in force for the callee, then its
substitution. -/
def instantiate (summary : Nat → Option (List Mode)) (callee : Nat) (actuals : List Actual) :
    Option (List Mode × List Subst) :=
  (summary callee).bind fun modes => (substitute modes actuals).map fun σ => (modes, σ)

/-- One summary, one substitution: an instantiation is the callee's contract in
force and, per formal, that formal's rule applied to its own actual. -/
theorem instantiation_is_one_summary_and_one_substitution (summary : Nat → Option (List Mode))
    (callee : Nat) (actuals : List Actual) (modes : List Mode) (σ : List Subst)
    (h : instantiate summary callee actuals = some (modes, σ)) :
    summary callee = some modes ∧ σ = List.zipWith substOne modes actuals ∧
      σ.length = modes.length := by
  unfold instantiate at h
  cases hs : summary callee with
  | none => simp [hs] at h
  | some ms =>
    simp only [hs, Option.bind_some] at h
    unfold substitute at h
    by_cases hl : ms.length = actuals.length
    · by_cases hr : (List.zipWith substOne ms actuals).any Subst.refused = true
      · simp [hl, hr] at h
      · simp only [hl, hr, ite_true, Bool.false_eq_true, ite_false, Option.map_some,
          Option.some.injEq, Prod.mk.injEq] at h
        obtain ⟨rfl, rfl⟩ := h
        refine ⟨rfl, rfl, ?_⟩
        simp [List.length_zipWith, hl]
    · simp [hl] at h

/-- Deterministic: the same call instantiates one way. -/
theorem instantiation_is_deterministic (summary : Nat → Option (List Mode)) (callee : Nat)
    (actuals : List Actual) (a b : List Mode × List Subst)
    (ha : instantiate summary callee actuals = some a)
    (hb : instantiate summary callee actuals = some b) : a = b := by
  rw [ha] at hb
  exact Option.some.inj hb

/-- Total or reject: an admitted substitution refuses no formal. -/
theorem an_admitted_substitution_refuses_no_formal (modes : List Mode) (actuals : List Actual)
    (σ : List Subst) (h : substitute modes actuals = some σ) : σ.all (fun s => !s.refused) = true := by
  unfold substitute at h
  by_cases hl : modes.length = actuals.length
  · by_cases hr : (List.zipWith substOne modes actuals).any Subst.refused = true
    · simp [hl, hr] at h
    · simp only [hl, hr, ite_true, Bool.false_eq_true, ite_false, Option.some.injEq] at h
      subst h
      simpa [List.all_eq_true, List.any_eq_true] using hr
  · simp [hl] at h

/-- Total or reject: one refused formal rejects the whole call. -/
theorem one_refused_formal_rejects_the_call (modes : List Mode) (actuals : List Actual)
    (h : (List.zipWith substOne modes actuals).any Subst.refused = true) :
    substitute modes actuals = none := by
  unfold substitute
  split <;> simp [h]

/-- A place, an address or a scalar value at a custody-taking formal other
than an `Out` slot is refused (W44-CALL-ARGUMENT-CUSTODY-POSITION). -/
theorem a_custody_formal_takes_only_an_identity (m : Mode) (a : Actual) (hm : m.custody = true)
    (hs : (substOne m a).refused = false) :
    (∃ x c, a = .identity x c) ∨ (m = .out ∧ ∃ x, a = .address x) := by
  cases m <;> cases a <;> simp_all [Mode.custody, substOne, Subst.refused]

/-- An unlisted actual is refused at every formal. -/
theorem an_unlisted_actual_is_refused (m : Mode) : substOne m .other = .reject := by
  cases m <;> rfl

/-! ## Substitution soundness over values and places -/

/-- The C forms of an actual the reader distinguishes. `path` carries the
place forms `QCore.TypedPlace.readPlace` reads. -/
inductive Expr where
  | pointerVar (x : Nat)
  | scalarVar (x : Nat)
  | literal
  | castPointer (e : Expr)
  | addrOf (x : Nat)
  | sizeofOp (e : Expr)
  | arith (a b : Expr)
  | path (forms : List TypedPlace.Form)
  | other

/-- Whether an expression reads as data (evaluated reads all scalar). -/
def Expr.isData : Expr → Bool
  | .scalarVar _ => true
  | .literal => true
  | .sizeofOp _ => true
  | .arith a b => a.isData && b.isData
  | _ => false

/-- `read_actual`: value → identity. -/
def read : Expr → Actual
  | .pointerVar x => .identity x false
  | .castPointer e =>
    match read e with
    | .identity x _ => .identity x true
    | _ => .other
  | .addrOf x => .address x
  | .path forms => if (TypedPlace.readPlace forms).isSome then .place else .other
  | e => if e.isData then .data else .other

/-- An environment: each object's value and address. -/
structure Env where
  value : Nat → Nat
  address : Nat → Nat

/-- The value an expression denotes, where the reader relates it to an
object. A bit cast between object pointers keeps the address (the stated
premise); an unevaluated operand has no value. -/
def denote (ρ : Env) : Expr → Option Nat
  | .pointerVar x => some (ρ.value x)
  | .castPointer e => denote ρ e
  | .addrOf x => some (ρ.address x)
  | _ => none

/-- The pointer objects an expression evaluates. -/
def evaluatedPointers : Expr → List Nat
  | .pointerVar x => [x]
  | .castPointer e => evaluatedPointers e
  | .addrOf x => [x]
  | .arith a b => evaluatedPointers a ++ evaluatedPointers b
  | .path _ => [0]
  | .other => [0]
  | _ => []

/-- An identity substitution denotes its object's value. -/
theorem an_identity_substitution_is_the_objects_value (ρ : Env) (e : Expr) (x : Nat) (c : Bool)
    (h : read e = .identity x c) : denote ρ e = some (ρ.value x) := by
  induction e generalizing c with
  | pointerVar y => simp [read] at h; simp [denote, h]
  | castPointer e ih =>
    simp only [read] at h
    split at h
    · rename_i y c' hy
      simp at h
      obtain ⟨rfl, rfl⟩ := h
      simpa [denote] using ih c' hy
    · simp at h
  | addrOf y => simp [read] at h
  | path forms => simp [read] at h; split at h <;> simp at h
  | scalarVar y => simp [read, Expr.isData] at h
  | literal => simp [read, Expr.isData] at h
  | sizeofOp e _ => simp [read, Expr.isData] at h
  | arith a b _ _ => simp [read] at h; split at h <;> simp at h
  | other => simp [read, Expr.isData] at h

/-- An address substitution denotes the slot's address. -/
theorem an_address_substitution_is_the_slot (ρ : Env) (e : Expr) (x : Nat)
    (h : read e = .address x) : denote ρ e = some (ρ.address x) := by
  cases e with
  | addrOf y => simp [read] at h; simp [denote, h]
  | castPointer e => simp only [read] at h; split at h <;> simp at h
  | pointerVar y => simp [read] at h
  | path forms => simp [read] at h; split at h <;> simp at h
  | scalarVar y => simp [read, Expr.isData] at h
  | literal => simp [read, Expr.isData] at h
  | sizeofOp e => simp [read, Expr.isData] at h
  | arith a b => simp [read] at h; split at h <;> simp at h
  | other => simp [read, Expr.isData] at h

/-- Data carries no identity: it evaluates no pointer object. -/
theorem data_evaluates_no_pointer_object (e : Expr) (h : e.isData = true) :
    evaluatedPointers e = [] := by
  induction e with
  | arith a b iha ihb =>
    simp [Expr.isData] at h
    simp [evaluatedPointers, iha h.1, ihb h.2]
  | _ => simp_all [Expr.isData, evaluatedPointers]

/-- A place CG10's reader refuses stays refused at every formal. -/
theorem a_place_cg10_refuses_stays_refused (m : Mode) (forms : List TypedPlace.Form)
    (h : TypedPlace.readPlace forms = none) : substOne m (read (.path forms)) = .reject := by
  simp [read, h, an_unlisted_actual_is_refused]

/-- A place is substituted only at a `Borrowed` formal. -/
theorem a_place_is_only_observed (m : Mode) (forms : List TypedPlace.Form)
    (h : substOne m (read (.path forms)) = .place) : m = .borrowed := by
  simp only [read] at h
  split at h <;> cases m <;> simp_all [substOne]

/-! ## The lowering applies each call once -/

/-- A lowered node: the events before a call, one application, an exit that
escapes nothing. -/
inductive Node where
  | events (n : Nat)
  | apply (callee : Nat)
  | exitNoOp
  deriving DecidableEq, Repr

/-- The return operand: its calls in evaluation order, then an exit that
escapes nothing. -/
def lowerReturnOperand (calls : List Nat) : List Node :=
  calls.map Node.apply ++ [.exitNoOp]

/-- The effectful-actual statement: the actuals' events, which C sequences
before the call, then the call. -/
def lowerEffectfulCall (events : Nat) (callee : Nat) : List Node :=
  [.events events, .apply callee]

theorem applications_count_their_calls (calls : List Nat) (c : Nat) :
    (calls.map Node.apply).count (.apply c) = calls.count c := by
  induction calls with
  | nil => rfl
  | cons a t ih => simp [List.count_cons, ih]

theorem a_distinct_call_occurs_once (calls : List Nat) (c : Nat) (hn : calls.Nodup)
    (hc : c ∈ calls) : calls.count c = 1 := by
  induction calls with
  | nil => simp at hc
  | cons a t ih =>
    rw [List.nodup_cons] at hn
    rcases List.mem_cons.mp hc with rfl | h
    · simp [List.count_eq_zero.mpr hn.1]
    · have hne : a ≠ c := fun e => hn.1 (e ▸ h)
      simp [ih hn.2 h, hne]

theorem each_returned_call_is_applied_once (calls : List Nat) (c : Nat) (hc : c ∈ calls)
    (hn : calls.Nodup) : (lowerReturnOperand calls).count (.apply c) = 1 := by
  unfold lowerReturnOperand
  rw [List.count_append, applications_count_their_calls]
  have hexit : [Node.exitNoOp].count (.apply c) = 0 := by simp
  rw [hexit, Nat.add_zero]
  exact a_distinct_call_occurs_once calls c hn hc

theorem the_return_operand_escapes_nothing (calls : List Nat) :
    (lowerReturnOperand calls).getLast? = some .exitNoOp := by
  simp [lowerReturnOperand]

theorem the_actual_events_precede_the_call (events callee : Nat) :
    (lowerEffectfulCall events callee).head? = some (.events events) ∧
      (lowerEffectfulCall events callee).getLast? = some (.apply callee) ∧
      (lowerEffectfulCall events callee).count (.apply callee) = 1 := by
  simp [lowerEffectfulCall]

/-- The effectful actual's formal: data at a `Borrowed` formal only. -/
def effectfulActualAdmitted (m : Mode) (pointerValue : Bool) : Bool :=
  m == .borrowed && !pointerValue

theorem an_effectful_actual_is_scalar_data (m : Mode) (pointerValue : Bool)
    (h : effectfulActualAdmitted m pointerValue = true) : m = .borrowed ∧ pointerValue = false := by
  cases m <;> cases pointerValue <;> simp_all [effectfulActualAdmitted]

/-! ## The Rust substitution vectors -/

/-- `SUBSTITUTION_VECTORS` of `call_instantiation.rs`: spelling, mode, and the
substitution the rule gives the reading. -/
def substitutionVectors : List (String × Mode × Actual × Subst) := [
  ("p", .released, .identity 0 false, .ident 0),
  ("(struct T *)p", .released, .identity 0 true, .ident 0),
  ("(void *)(char *)p", .released, .identity 0 true, .ident 0),
  ("&p", .out, .address 0, .slot 0),
  ("&n", .borrowed, .address 1, .observed 1),
  ("n * 2 + sizeof(*p)", .borrowed, .data, .data),
  ("sizeof(struct T)", .borrowed, .data, .data),
  ("\"text\"", .borrowed, .data, .data),
  ("s->p", .borrowed, .place, .place),
  ("s->p", .released, .place, .reject),
  ("&n", .released, .address 1, .reject),
  ("n", .released, .data, .reject),
  ("(char *)p + 1", .borrowed, .other, .reject),
  ("i++", .borrowed, .other, .reject),
  ("n ? p : 0", .borrowed, .other, .reject),
  ("(long)p", .borrowed, .other, .reject)
]

theorem substitution_vectors_hold :
    substitutionVectors.all (fun (_, m, a, s) => substOne m a == s) = true := by
  decide

end QCore.CallInstantiation

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

import QCore.TypedAccessPathStatements
import QCore.ConditionCallDesugar

/-!
Clang-typed access paths in more positions (W47-ACCESS-PATH-POSITIONS, gateway
rule 10k). The AST reader and the Rust-to-Lean correspondence remain review
premises (standing constraint 28). This module models the decision the Rust
makes about *where* P2's unchanged path reader may be applied, and proves:

* refusal of every unlisted position and form, and that the fallback is asked
  only after every older route refused (hard invariant 3);
* short-circuit scoping: an operand C does not evaluate on a path contributes
  neither an obligation nor a discharge there, and a `p && p->x` guard reaches
  `p->x` only on the edge where `p`'s null test is true, while `p || p->x`
  reaches it only where the test is false;
* obligation before branch: a condition operand's reads are checked before
  either edge, and the read changes no binding, custody or field link;
* a return read preserves the frame and hands nothing back (a `NoOp` exit);
* a call actual's read leaves the caller's custody frame unchanged before the
  callee's own contract transfer, which is applied as is; a custody-taking or
  uncontracted position refuses;
* the enum arm only adds positively proven enumerations to scalar evidence.
-/

namespace QCore.TypedAccessPathPositions

open QCore.IndirectScalarAccess
open QCore.ConditionCallDesugar

/-! ## Where the typed reader may be applied -/

inductive Position where
  | conditionOperand
  | conditionAssignment
  | returnExpression
  | callActual
  | unlisted
  deriving DecidableEq, Repr

/-- The parameter mode the contract in force gives an actual's index. -/
inductive ActualMode where
  | borrowed
  | dataTail
  | custody
  | beyond
  | uncontracted
  deriving DecidableEq, Repr

def ActualMode.admitsRead : ActualMode → Bool
  | .borrowed => true
  | .dataTail => true
  | _ => false

structure Candidate where
  shape : TypedAccessPathStatements.Shape
  position : Position
  /-- Every older route of the position refused it. -/
  olderRefused : Bool
  /-- `&&`, `||` or `?:` inside the one expression read here. -/
  connective : Bool
  /-- The value handed out (a return or an actual) is a pointer read. -/
  pointerValue : Bool
  /-- Every root satisfies the position's untracked-root premise. -/
  rootsUntracked : Bool
  mode : ActualMode
  /-- For `NAME = PATH`: `NAME` is a declared, non-formal, non-descriptor local. -/
  localDestination : Bool

def positionAdmits (c : Candidate) : Bool :=
  match c.position with
  | .conditionOperand => true
  | .conditionAssignment => c.localDestination
  | .returnExpression => !c.pointerValue || c.rootsUntracked
  | .callActual => c.mode.admitsRead && (!c.pointerValue || c.rootsUntracked)
  | .unlisted => false

/-- The fallback admits only after every older route refused, only a path P2
admits, never a connective inside one read, and only a listed position. -/
def admits (c : Candidate) : Bool :=
  c.olderRefused && TypedAccessPathStatements.admits c.shape && !c.connective &&
    positionAdmits c

theorem an_unlisted_position_refuses (c : Candidate) (h : c.position = .unlisted) :
    admits c = false := by
  simp [admits, positionAdmits, h]

theorem an_older_admission_is_never_replaced (c : Candidate) (h : c.olderRefused = false) :
    admits c = false := by
  simp [admits, h]

theorem every_statement_refusal_holds_in_every_position (c : Candidate)
    (h : TypedAccessPathStatements.admits c.shape = false) : admits c = false := by
  simp [admits, h]

theorem a_union_path_refuses_in_every_position (c : Candidate)
    (h : c.shape.unionField = true) : admits c = false :=
  every_statement_refusal_holds_in_every_position c
    (TypedAccessPathStatements.union_refuses c.shape h)

theorem a_call_in_the_path_refuses_in_every_position (c : Candidate)
    (h : c.shape.call = true) : admits c = false :=
  every_statement_refusal_holds_in_every_position c
    (TypedAccessPathStatements.call_refuses c.shape h)

theorem a_cast_in_the_path_refuses_in_every_position (c : Candidate)
    (h : c.shape.castInPath = true) : admits c = false :=
  every_statement_refusal_holds_in_every_position c
    (TypedAccessPathStatements.cast_refuses c.shape h)

theorem a_connective_inside_one_read_refuses (c : Candidate) (h : c.connective = true) :
    admits c = false := by
  simp [admits, h]

theorem a_custody_position_refuses (c : Candidate) (hp : c.position = .callActual)
    (hm : c.mode = .custody) : admits c = false := by
  simp [admits, positionAdmits, hp, hm, ActualMode.admitsRead]

theorem an_uncontracted_callee_refuses (c : Candidate) (hp : c.position = .callActual)
    (hm : c.mode = .uncontracted) : admits c = false := by
  simp [admits, positionAdmits, hp, hm, ActualMode.admitsRead]

theorem an_actual_beyond_the_declaration_refuses (c : Candidate)
    (hp : c.position = .callActual) (hm : c.mode = .beyond) : admits c = false := by
  simp [admits, positionAdmits, hp, hm, ActualMode.admitsRead]

theorem a_returned_pointer_from_a_tracked_root_refuses (c : Candidate)
    (hp : c.position = .returnExpression) (hv : c.pointerValue = true)
    (hr : c.rootsUntracked = false) : admits c = false := by
  simp [admits, positionAdmits, hp, hv, hr]

theorem a_pointer_actual_from_a_tracked_root_refuses (c : Candidate)
    (hp : c.position = .callActual) (hv : c.pointerValue = true)
    (hr : c.rootsUntracked = false) : admits c = false := by
  simp [admits, positionAdmits, hp, hv, hr]

theorem an_assignment_into_a_non_local_refuses (c : Candidate)
    (hp : c.position = .conditionAssignment) (hl : c.localDestination = false) :
    admits c = false := by
  simp [admits, positionAdmits, hp, hl]

/-! ## Short-circuit scoping -/

/-- An operand not evaluated on a path contributes no obligation there, and
the exit is decided by the evaluated operand alone (no discharge from it). -/
theorem an_unevaluated_and_operand_contributes_nothing {α : Type} (v : Nat → Bool)
    (a b : Cond α) (h : (eval v a).2 = false) :
    run v (desugar (.and a b)) = ((eval v a).1, false) := by
  rw [the_desugaring_is_c_evaluation]
  simp [eval, h]

theorem an_unevaluated_or_operand_contributes_nothing {α : Type} (v : Nat → Bool)
    (a b : Cond α) (h : (eval v a).2 = true) :
    run v (desugar (.or a b)) = ((eval v a).1, true) := by
  rw [the_desugaring_is_c_evaluation]
  simp [eval, h]

theorem an_unselected_arm_contributes_nothing {α : Type} (v : Nat → Bool)
    (c a b : Cond α) (h : (eval v c).2 = true) :
    (run v (desugar (.choose c a b))).1 = (eval v c).1 ++ (eval v a).1 := by
  rw [the_desugaring_is_c_evaluation]
  simp [eval, h]

/-- A per-hop obligation that the pointer the null-test operand `leaf`
decided is non-null. The operand's value on a path is `v leaf`, and the
existing null branch refines exactly that edge. -/
inductive Hop where
  | nonNullBy (leaf : Nat)
  deriving DecidableEq, Repr

def discharged (v : Nat → Bool) : Hop → Bool
  | .nonNullBy leaf => v leaf

/-- `p && p->x`: operand 0 null-tests `p`, operand 1 reads through it. -/
def guardedAnd : Cond Hop := .and (.leaf 0 []) (.leaf 1 [.nonNullBy 0])

/-- `p || p->x`. -/
def guardedOr : Cond Hop := .or (.leaf 0 []) (.leaf 1 [.nonNullBy 0])

/-- Every hop `p && p->x` checks, on every path, is discharged by the guard. -/
theorem an_and_guard_discharges_every_checked_hop (v : Nat → Bool) :
    ∀ hop ∈ (run v (desugar guardedAnd)).1, discharged v hop = true := by
  rw [the_desugaring_is_c_evaluation]
  cases h : v 0 <;> simp [guardedAnd, eval, h, discharged]

/-- Every hop `p || p->x` checks is checked exactly where the guard failed:
it is never discharged by the guard. -/
theorem an_or_guard_discharges_no_checked_hop (v : Nat → Bool) :
    ∀ hop ∈ (run v (desugar guardedOr)).1, discharged v hop = false := by
  rw [the_desugaring_is_c_evaluation]
  cases h : v 0 <;> simp [guardedOr, eval, h, discharged]

/-- Non-vacuity: the `||` hop is reached, on the path where `p` is null. -/
theorem the_or_hop_is_reached_where_the_guard_fails :
    (run (fun _ => false) (desugar guardedOr)).1 = [Hop.nonNullBy 0] := by
  decide

/-- Non-vacuity: the `&&` hop is reached, on the path where `p` is non-null. -/
theorem the_and_hop_is_reached_where_the_guard_holds :
    (run (fun _ => true) (desugar guardedAnd)).1 = [Hop.nonNullBy 0] := by
  decide

/-! ## Obligation before branch, and the frame -/

/-- An operand's reads are applied before either of its edges, whatever the
operand's value: both successors receive the post-read state. -/
theorem an_operand_reads_before_either_edge {α : Type} (v : Nat → Bool) (i : Nat)
    (reads : List α) (t f : Node α) :
    run v (assemble (.leaf i reads) t f) =
      (reads ++ (run v (if v i then t else f)).1, (run v (if v i then t else f)).2) := by
  rw [run_assemble]
  rfl

/-- A condition operand or a return: the typed read with no destination and
no frame changes no binding, custody or field link. -/
theorem a_read_position_preserves_the_frame (s : IndirectChainAccess.State)
    (p : TypedAccessPathStatements.Statement)
    (ha : TypedAccessPathStatements.admits p.shape = true)
    (hd : p.scalar.destination = none) (hf : p.scalar.frame = none) :
    ∃ after diagnostics, TypedAccessPathStatements.lower s p = some (after, diagnostics) ∧
      after.base.rest = s.base.rest ∧ after.fieldLink = s.fieldLink ∧
      ∀ name, after.base.binding name = s.base.binding name := by
  obtain ⟨after, diagnostics, hl, hr, hfl, hb⟩ :=
    TypedAccessPathStatements.nonpointer_store_preserves_custody s p ha hf
  exact ⟨after, diagnostics, hl, hr, hfl, fun name => hb name (by simp [hd])⟩

/-- What a `return` hands back. The typed return never escapes a place. -/
inductive Exit where
  | handsNothingBack
  | escapes (place : Nat)
  deriving DecidableEq, Repr

def lowerReturn (s : IndirectChainAccess.State) (p : TypedAccessPathStatements.Statement) :
    Option (IndirectChainAccess.State × List Diagnostic × Exit) :=
  (TypedAccessPathStatements.lower s p).map fun (after, diagnostics) =>
    (after, diagnostics, Exit.handsNothingBack)

theorem a_typed_return_hands_nothing_back (s : IndirectChainAccess.State)
    (p : TypedAccessPathStatements.Statement) (r : IndirectChainAccess.State × List Diagnostic × Exit)
    (h : lowerReturn s p = some r) : r.2.2 = .handsNothingBack := by
  unfold lowerReturn at h
  cases hl : TypedAccessPathStatements.lower s p with
  | none => simp [hl] at h
  | some pair =>
      simp [hl] at h
      rw [← h]

theorem a_refused_return_stays_refused (s : IndirectChainAccess.State)
    (p : TypedAccessPathStatements.Statement)
    (h : TypedAccessPathStatements.admits p.shape = false) : lowerReturn s p = none := by
  simp [lowerReturn, TypedAccessPathStatements.refused_shape_has_no_lowering s p h]

/-- A call actual: the read, then the callee's own contract transfer `call`,
unchanged. -/
def lowerActual (s : IndirectChainAccess.State) (p : TypedAccessPathStatements.Statement)
    (call : IndirectChainAccess.State → IndirectChainAccess.State × List Diagnostic) :
    Option (IndirectChainAccess.State × List Diagnostic) :=
  (TypedAccessPathStatements.lower s p).map fun (after, diagnostics) =>
    ((call after).1, diagnostics ++ (call after).2)

/-- The callee's contract is applied as is, to a state whose bindings,
custody and field links are the caller's: the read establishes nothing the
callee's custody premise could depend on. -/
theorem the_callee_contract_applies_to_the_callers_frame (s : IndirectChainAccess.State)
    (p : TypedAccessPathStatements.Statement)
    (call : IndirectChainAccess.State → IndirectChainAccess.State × List Diagnostic)
    (ha : TypedAccessPathStatements.admits p.shape = true)
    (hd : p.scalar.destination = none) (hf : p.scalar.frame = none) :
    ∃ mid diagnostics, lowerActual s p call = some ((call mid).1, diagnostics ++ (call mid).2) ∧
      mid.base.rest = s.base.rest ∧ mid.fieldLink = s.fieldLink ∧
      ∀ name, mid.base.binding name = s.base.binding name := by
  obtain ⟨mid, diagnostics, hl, hr, hfl, hb⟩ := a_read_position_preserves_the_frame s p ha hd hf
  exact ⟨mid, diagnostics, by simp [lowerActual, hl], hr, hfl, hb⟩

theorem a_refused_actual_keeps_the_refusal (s : IndirectChainAccess.State)
    (p : TypedAccessPathStatements.Statement)
    (call : IndirectChainAccess.State → IndirectChainAccess.State × List Diagnostic)
    (h : TypedAccessPathStatements.admits p.shape = false) : lowerActual s p call = none := by
  simp [lowerActual, TypedAccessPathStatements.refused_shape_has_no_lowering s p h]

/-! ## The enum arm (step B) -/

/-- The type evidence the extraction finds for a value. -/
inductive ValueEvidence where
  | builtinScalar
  | enumType
  | typedefSpellingOnly
  | aggregate
  | unionType
  deriving DecidableEq, Repr

def provenScalar (enumArm : Bool) : ValueEvidence → Bool
  | .builtinScalar => true
  | .enumType => enumArm
  | _ => false

theorem a_typedef_spelling_alone_is_never_scalar (arm : Bool) :
    provenScalar arm .typedefSpellingOnly = false := rfl

theorem an_aggregate_is_never_scalar (arm : Bool) : provenScalar arm .aggregate = false := rfl

theorem a_union_is_never_scalar (arm : Bool) : provenScalar arm .unionType = false := rfl

theorem without_the_arm_an_enum_refuses : provenScalar false .enumType = false := rfl

/-- The arm only widens: every value scalar without it is scalar with it. -/
theorem the_enum_arm_only_adds_enumerations (arm : Bool) (e : ValueEvidence)
    (h : provenScalar false e = true) : provenScalar arm e = true := by
  cases e <;> simp_all [provenScalar]

/-! ## Non-vacuity -/

def exampleShape : TypedAccessPathStatements.Shape :=
  { exactType := true, knownFieldOwner := true, knownValueType := true,
    borrowedPointerValue := false, unionField := false, bitfield := false,
    incompleteRecord := false, castInPath := false, pointerArithmetic := false,
    volatileAtomic := false, pointerStore := false, call := false, otherForm := false,
    mixedPath := true, untrackedRoot := true }

def exampleActual : Candidate :=
  { shape := exampleShape, position := .callActual, olderRefused := true,
    connective := false, pointerValue := false, rootsUntracked := true,
    mode := .borrowed, localDestination := false }

theorem a_borrowed_scalar_actual_is_admitted : admits exampleActual = true := by decide

theorem the_same_actual_at_a_custody_position_is_refused :
    admits { exampleActual with mode := .custody } = false := by decide

end QCore.TypedAccessPathPositions

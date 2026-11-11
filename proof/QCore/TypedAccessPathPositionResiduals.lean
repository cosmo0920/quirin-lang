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

import QCore.TypedAccessPathPositions

/-!
The P3 position residual shapes (W47-ACCESS-PATH-POSITION-RESIDUALS, gateway
rule 10n). The AST reader, the node matching and the Rust-to-Lean
correspondence remain review premises (standing constraint 28). This module
models the four shapes the Rust admits after every older route of a position
and rule 10k's typed reader refused, and proves that each keeps P3's per-hop
and short-circuit obligations:

* **address values** (`&PATH`): the place's hops are exactly those of its
  read, only the value-type fact of the place is waived (its value is not
  read); admitted as a condition pointer operand and at a `Borrowed` or
  data-tail actual with untracked roots, never in a return (rule 5j);
  `(&X)->f` is the member selection `X.f`;
* **controlling-scope matching**: an ambiguous operand is resolved only to a
  candidate inside a controlling expression or returned value, and a unique
  match is unchanged; the matched node is read by P3's unchanged decision;
* **the anonymous inline enumeration member**: scalar evidence only widens,
  and a union member, a pointer to one, or a spelling alone refuses;
* **connective returns**: a scalar `&&`, `||` or `?:` return is its desugared
  C evaluation with both exits joined into one return that hands nothing
  back, so P3's short-circuit theorems hold for it.

Every other residual shape (a `?:` actual, a nested call actual, a pointer
`?:` return, a call or assignment in a return, an array-decay return) refuses.
-/

namespace QCore.TypedAccessPathPositionResiduals

open QCore.IndirectScalarAccess
open QCore.ConditionCallDesugar
open QCore.TypedAccessPathPositions

/-! ## The listed shapes; everything else refuses -/

inductive ResidualShape where
  | addressValue
  | scopeMatched
  | anonymousEnumMember
  | connectiveReturn
  | conditionalActual
  | nestedCallActual
  | pointerConditionalReturn
  | callInReturn
  | assignmentInReturn
  | arrayDecayReturn
  | unlisted
  deriving DecidableEq, Repr

def listed : ResidualShape → Bool
  | .addressValue | .scopeMatched | .anonymousEnumMember | .connectiveReturn => true
  | _ => false

theorem every_unlisted_residual_refuses (s : ResidualShape) (h : listed s = false) :
    s ≠ .addressValue ∧ s ≠ .scopeMatched ∧ s ≠ .anonymousEnumMember ∧
      s ≠ .connectiveReturn := by
  cases s <;> simp_all [listed]

theorem a_conditional_actual_is_not_listed : listed .conditionalActual = false := rfl
theorem a_nested_call_actual_is_not_listed : listed .nestedCallActual = false := rfl
theorem a_pointer_conditional_return_is_not_listed : listed .pointerConditionalReturn = false := rfl
theorem a_call_in_a_return_is_not_listed : listed .callInReturn = false := rfl
theorem an_assignment_in_a_return_is_not_listed : listed .assignmentInReturn = false := rfl
theorem an_array_decay_return_is_not_listed : listed .arrayDecayReturn = false := rfl

/-! ## Address values -/

/-- The place's own value is not read, so only its value-type fact is waived;
every other fact (field owner, union, bitfield, cast, arithmetic, call,
mixed path under a tracked root) is the read's. -/
def placeShape (s : TypedAccessPathStatements.Shape) : TypedAccessPathStatements.Shape :=
  { s with knownValueType := true }

structure AddressCandidate where
  place : TypedAccessPathStatements.Shape
  position : Position
  olderRefused : Bool
  rootsUntracked : Bool
  mode : ActualMode

def addressAdmits (a : AddressCandidate) : Bool :=
  a.olderRefused && TypedAccessPathStatements.admits (placeShape a.place) &&
    match a.position with
    | .conditionOperand => true
    | .callActual => a.mode.admitsRead && a.rootsUntracked
    | _ => false

theorem an_address_is_never_returned (a : AddressCandidate)
    (h : a.position = .returnExpression) : addressAdmits a = false := by
  simp [addressAdmits, h]

theorem an_address_at_a_custody_position_refuses (a : AddressCandidate)
    (hp : a.position = .callActual) (hm : a.mode = .custody) : addressAdmits a = false := by
  simp [addressAdmits, hp, hm, ActualMode.admitsRead]

theorem an_address_for_an_uncontracted_callee_refuses (a : AddressCandidate)
    (hp : a.position = .callActual) (hm : a.mode = .uncontracted) :
    addressAdmits a = false := by
  simp [addressAdmits, hp, hm, ActualMode.admitsRead]

theorem an_address_actual_from_a_tracked_root_refuses (a : AddressCandidate)
    (hp : a.position = .callActual) (hr : a.rootsUntracked = false) :
    addressAdmits a = false := by
  simp [addressAdmits, hp, hr]

theorem an_address_never_replaces_an_older_admission (a : AddressCandidate)
    (h : a.olderRefused = false) : addressAdmits a = false := by
  simp [addressAdmits, h]

theorem a_union_place_refuses_behind_an_address (a : AddressCandidate)
    (h : a.place.unionField = true) : addressAdmits a = false := by
  simp [addressAdmits, placeShape, TypedAccessPathStatements.admits, h]

theorem a_cast_place_refuses_behind_an_address (a : AddressCandidate)
    (h : a.place.castInPath = true) : addressAdmits a = false := by
  simp [addressAdmits, placeShape, TypedAccessPathStatements.admits, h]

theorem a_mixed_place_under_a_tracked_root_refuses_behind_an_address (a : AddressCandidate)
    (hm : a.place.mixedPath = true) (hr : a.place.untrackedRoot = false) :
    addressAdmits a = false := by
  simp [addressAdmits, placeShape, TypedAccessPathStatements.admits, hm, hr]

/-- The address waives nothing but the value type: a place whose value type
is proven is admitted behind `&` exactly when its read is. -/
theorem an_address_waives_only_the_value_type (s : TypedAccessPathStatements.Shape)
    (h : s.knownValueType = true) :
    TypedAccessPathStatements.admits (placeShape s) = TypedAccessPathStatements.admits s := by
  simp [placeShape, TypedAccessPathStatements.admits, h]

/-- The lowering of `&PATH`: the read of `PATH`'s hops, with no destination
and no frame. -/
def lowerAddress (s : IndirectChainAccess.State) (p : TypedAccessPathStatements.Statement) :
    Option (IndirectChainAccess.State × List Diagnostic) :=
  TypedAccessPathStatements.lower s { p with shape := placeShape p.shape }

/-- Every hop of the place discharged: the address adds no diagnostic
(P3's per-hop obligation, unchanged). -/
theorem an_address_with_every_hop_discharged_adds_no_diagnostic
    (s : IndirectChainAccess.State) (p : TypedAccessPathStatements.Statement)
    (ha : TypedAccessPathStatements.admits (placeShape p.shape) = true)
    (hh : TypedAccessPathStatements.everyHop s p) :
    ∃ after, lowerAddress s p = some (after, []) :=
  TypedAccessPathStatements.every_hop_discharge_adds_no_diagnostic s
    { p with shape := placeShape p.shape } ha hh

/-- An address changes no binding, custody or field link: the callee's own
contract is applied to the caller's frame (P3's actual theorem). -/
theorem an_address_preserves_the_frame (s : IndirectChainAccess.State)
    (p : TypedAccessPathStatements.Statement)
    (ha : TypedAccessPathStatements.admits (placeShape p.shape) = true)
    (hd : p.scalar.destination = none) (hf : p.scalar.frame = none) :
    ∃ after diagnostics, lowerAddress s p = some (after, diagnostics) ∧
      after.base.rest = s.base.rest ∧ after.fieldLink = s.fieldLink ∧
      ∀ name, after.base.binding name = s.base.binding name :=
  a_read_position_preserves_the_frame s { p with shape := placeShape p.shape } ha hd hf

theorem a_refused_place_keeps_the_address_refused (s : IndirectChainAccess.State)
    (p : TypedAccessPathStatements.Statement)
    (h : TypedAccessPathStatements.admits (placeShape p.shape) = false) :
    lowerAddress s p = none :=
  TypedAccessPathStatements.refused_shape_has_no_lowering s _ h

/-- One selection step of a path. `addressArrow` is `(&X)->f`. -/
inductive Step where
  | arrow
  | dot
  | index
  | deref
  | addressArrow
  deriving DecidableEq, Repr

/-- `(&X)->f` is `X.f`: the address of a place is never null and points
into the place's own object, so the arrow adds no dereference of its own. -/
def normalize : List Step → List Step :=
  List.map fun step => if step = .addressArrow then .dot else step

def dereferences (path : List Step) : Nat :=
  (path.filter fun step => step = .arrow || step = .index || step = .deref).length

theorem a_normalized_path_has_no_address_step (path : List Step) :
    .addressArrow ∉ normalize path := by
  induction path with
  | nil => simp [normalize]
  | cons step rest ih =>
      cases step <;> simp_all [normalize]

theorem an_arrow_through_an_address_adds_no_dereference (path : List Step) :
    dereferences (normalize path) = dereferences path := by
  induction path with
  | nil => rfl
  | cons step rest ih =>
      cases step <;> simp_all [normalize, dereferences, List.filter]

theorem an_arrow_through_an_address_is_a_member_selection :
    normalize [.arrow, .addressArrow] = [.arrow, .dot] := by decide

/-! ## Controlling-scope matching -/

/-- A Clang node carrying the operand's exact tokens on its line: whether it
lies inside a controlling expression or returned value, and whether its
nearest non-wrapper parent is a Boolean leaf position. -/
structure Match where
  inScope : Bool
  leaf : Bool
  deriving DecidableEq, Repr

/-- The one element of a singleton list. -/
def single {α : Type} : List α → Option α
  | [only] => some only
  | _ => none

theorem single_is_the_list {α : Type} {l : List α} {x : α} (h : single l = some x) :
    l = [x] := by
  cases l with
  | nil => simp [single] at h
  | cons y t =>
      cases t with
      | nil => simp [single] at h; simp [h]
      | cons z u => simp [single] at h

/-- The Rust `controlling_scope`, after a unique match was not found: the one
candidate in scope, else the one in scope in a Boolean leaf position. -/
def select (candidates : List Match) : Option Match :=
  match single candidates with
  | some only => some only
  | none =>
      match single (candidates.filter (·.inScope)) with
      | some only => some only
      | none => single ((candidates.filter (·.inScope)).filter (·.leaf))

theorem a_unique_match_is_unchanged (m : Match) : select [m] = some m := rfl

theorem no_candidate_refuses : select [] = none := rfl

theorem a_selected_candidate_is_a_candidate (candidates : List Match) (m : Match)
    (h : select candidates = some m) : m ∈ candidates := by
  unfold select at h
  cases h1 : single candidates with
  | some x =>
      simp only [h1, Option.some.injEq] at h
      subst h
      simp [single_is_the_list h1]
  | none =>
      simp only [h1] at h
      cases h2 : single (candidates.filter (·.inScope)) with
      | some y =>
          simp only [h2, Option.some.injEq] at h
          subst h
          have hm : y ∈ candidates.filter (·.inScope) := by simp [single_is_the_list h2]
          exact (List.mem_filter.mp hm).1
      | none =>
          simp only [h2] at h
          have hm : m ∈ (candidates.filter (·.inScope)).filter (·.leaf) := by
            simp [single_is_the_list h]
          exact (List.mem_filter.mp (List.mem_filter.mp hm).1).1

/-- Among several candidates the selected one lies inside a controlling
expression or returned value. -/
theorem an_ambiguous_selection_is_in_scope (candidates : List Match) (m : Match)
    (hl : 2 ≤ candidates.length) (h : select candidates = some m) : m.inScope = true := by
  unfold select at h
  cases h1 : single candidates with
  | some x =>
      have := single_is_the_list h1
      subst this
      simp at hl
  | none =>
      simp only [h1] at h
      cases h2 : single (candidates.filter (·.inScope)) with
      | some y =>
          simp only [h2, Option.some.injEq] at h
          subst h
          have hm : y ∈ candidates.filter (·.inScope) := by simp [single_is_the_list h2]
          simpa using (List.mem_filter.mp hm).2
      | none =>
          simp only [h2] at h
          have hm : m ∈ (candidates.filter (·.inScope)).filter (·.leaf) := by
            simp [single_is_the_list h]
          simpa using (List.mem_filter.mp (List.mem_filter.mp hm).1).2

theorem two_leaves_in_scope_refuse :
    select [⟨true, true⟩, ⟨true, true⟩] = none := by decide

theorem a_leaf_beside_a_path_base_is_selected :
    select [⟨true, true⟩, ⟨true, false⟩] = some ⟨true, true⟩ := by decide

theorem an_operand_repeated_outside_the_condition_is_selected :
    select [⟨true, true⟩, ⟨false, false⟩] = some ⟨true, true⟩ := by decide

/-- The matched node is read by P3's unchanged decision. -/
def scopeAdmits (candidates : List Match) (c : TypedAccessPathPositions.Candidate) : Bool :=
  (select candidates).isSome && TypedAccessPathPositions.admits c

theorem a_scope_match_keeps_every_positions_refusal (candidates : List Match)
    (c : TypedAccessPathPositions.Candidate) (h : TypedAccessPathPositions.admits c = false) :
    scopeAdmits candidates c = false := by
  simp [scopeAdmits, h]

/-! ## The anonymous inline enumeration member -/

inductive Evidence where
  | known (e : ValueEvidence)
  | anonymousInlineEnumMember
  | anonymousEnumInUnion
  | pointerToAnonymousEnum
  | anonymousEnumSpellingOnly
  deriving DecidableEq, Repr

def residualScalar (arm : Bool) : Evidence → Bool
  | .known e => provenScalar true e
  | .anonymousInlineEnumMember => arm
  | _ => false

theorem the_anonymous_arm_only_adds_inline_members (arm : Bool) (e : Evidence)
    (h : residualScalar false e = true) : residualScalar arm e = true := by
  cases e <;> simp_all [residualScalar]

theorem without_the_anonymous_arm_an_inline_member_refuses :
    residualScalar false .anonymousInlineEnumMember = false := rfl

theorem an_anonymous_enum_in_a_union_is_never_scalar (arm : Bool) :
    residualScalar arm .anonymousEnumInUnion = false := rfl

theorem a_pointer_to_an_anonymous_enum_is_never_scalar (arm : Bool) :
    residualScalar arm .pointerToAnonymousEnum = false := rfl

theorem an_anonymous_enum_spelling_alone_is_never_scalar (arm : Bool) :
    residualScalar arm .anonymousEnumSpellingOnly = false := rfl

theorem the_anonymous_arm_keeps_every_earlier_refusal (arm : Bool) :
    residualScalar arm (.known .typedefSpellingOnly) = false ∧
      residualScalar arm (.known .aggregate) = false ∧
      residualScalar arm (.known .unionType) = false := by
  simp [residualScalar, provenScalar]

/-! ## Connective returns -/

structure ReturnConnective where
  olderRefused : Bool
  scalarResult : Bool
  hasCall : Bool
  hasAssignment : Bool
  topConnective : Bool
  everyLeafDecided : Bool

def returnAdmits (r : ReturnConnective) : Bool :=
  r.olderRefused && r.scalarResult && !r.hasCall && !r.hasAssignment && r.topConnective &&
    r.everyLeafDecided

theorem a_pointer_valued_connective_return_refuses (r : ReturnConnective)
    (h : r.scalarResult = false) : returnAdmits r = false := by
  simp [returnAdmits, h]

theorem a_call_in_a_connective_return_refuses (r : ReturnConnective)
    (h : r.hasCall = true) : returnAdmits r = false := by
  simp [returnAdmits, h]

theorem an_assignment_in_a_connective_return_refuses (r : ReturnConnective)
    (h : r.hasAssignment = true) : returnAdmits r = false := by
  simp [returnAdmits, h]

theorem a_connective_return_never_replaces_an_older_route (r : ReturnConnective)
    (h : r.olderRefused = false) : returnAdmits r = false := by
  simp [returnAdmits, h]

theorem an_undecided_leaf_keeps_the_return_refused (r : ReturnConnective)
    (h : r.everyLeafDecided = false) : returnAdmits r = false := by
  simp [returnAdmits, h]

/-- The lowering: the desugared expression with both exits joined into the
one return node `ret`. -/
def lowerConnectiveReturn {α : Type} (c : Cond α) (ret : Node α) : Node α :=
  assemble c ret ret

/-- The returned expression is its C evaluation: exactly the obligations C's
short circuit evaluates, in order, then the one return, whatever the value. -/
theorem a_connective_return_is_its_c_evaluation {α : Type} (v : Nat → Bool) (c : Cond α)
    (ret : Node α) :
    run v (lowerConnectiveReturn c ret) = ((eval v c).1 ++ (run v ret).1, (run v ret).2) := by
  unfold lowerConnectiveReturn
  rw [run_assemble]
  simp

/-- `return p && p->x;`: every hop it checks is discharged by the guard. -/
theorem a_returned_and_guard_discharges_every_checked_hop (v : Nat → Bool) :
    ∀ hop ∈ (run v (lowerConnectiveReturn guardedAnd Node.exitTrue)).1,
      discharged v hop = true := by
  rw [a_connective_return_is_its_c_evaluation]
  cases h : v 0 <;> simp [guardedAnd, eval, h, discharged, run]

/-- `return p || p->x;`: the hop is checked exactly where the guard failed. -/
theorem a_returned_or_guard_discharges_no_checked_hop (v : Nat → Bool) :
    ∀ hop ∈ (run v (lowerConnectiveReturn guardedOr Node.exitTrue)).1,
      discharged v hop = false := by
  rw [a_connective_return_is_its_c_evaluation]
  cases h : v 0 <;> simp [guardedOr, eval, h, discharged, run]

/-- Non-vacuity: the returned `&&` hop is reached where the guard holds. -/
theorem the_returned_and_hop_is_reached_where_the_guard_holds :
    (run (fun _ => true) (lowerConnectiveReturn guardedAnd Node.exitTrue)).1 =
      [Hop.nonNullBy 0] := by
  decide

/-- Non-vacuity: the returned `||` hop is reached where the guard fails. -/
theorem the_returned_or_hop_is_reached_where_the_guard_fails :
    (run (fun _ => false) (lowerConnectiveReturn guardedOr Node.exitTrue)).1 =
      [Hop.nonNullBy 0] := by
  decide

/-- A `?:` return evaluates only the selected arm. -/
theorem a_returned_conditional_evaluates_only_the_selected_arm {α : Type} (v : Nat → Bool)
    (c a b : Cond α) (ret : Node α) (h : (eval v c).2 = true) :
    (run v (lowerConnectiveReturn (.choose c a b) ret)).1 =
      (eval v c).1 ++ (eval v a).1 ++ (run v ret).1 := by
  rw [a_connective_return_is_its_c_evaluation]
  simp [eval, h, List.append_assoc]

/-- Both exits reach the one return, which hands nothing back: the returned
exit is the typed return's (rule 10k). -/
theorem a_connective_return_hands_nothing_back (s : IndirectChainAccess.State)
    (p : TypedAccessPathStatements.Statement) (r : IndirectChainAccess.State × List Diagnostic × Exit)
    (h : lowerReturn s p = some r) : r.2.2 = .handsNothingBack :=
  a_typed_return_hands_nothing_back s p r h

/-! ## Non-vacuity of the admissions -/

def exampleAddress : AddressCandidate :=
  { place := { exampleShape with knownValueType := false }, position := .callActual,
    olderRefused := true, rootsUntracked := true, mode := .borrowed }

theorem a_borrowed_address_of_an_untyped_place_is_admitted :
    addressAdmits exampleAddress = true := by decide

theorem the_same_address_returned_is_refused :
    addressAdmits { exampleAddress with position := .returnExpression } = false := by decide

theorem a_scalar_connective_return_is_admitted :
    returnAdmits ⟨true, true, false, false, true, true⟩ = true := by decide

end QCore.TypedAccessPathPositionResiduals

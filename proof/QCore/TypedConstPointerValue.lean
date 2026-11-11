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
A pointer value whose own type carries a top-level `const`, and an implicit
`void *` bit-cast in a pointer-destination statement, in the Clang-typed
access-path reader (W47-TYPED-CONST-POINTER-VALUE, gateway rule 10m).

The AST extraction (Clang's printed canonical type, its top-level qualifier
list after the last declarator `*`, and the cast kinds) and the Rust-to-Lean
correspondence remain review premises (standing constraint 28). This module
models the two decisions the card adds and proves:

* **qualifier erasure**: a value type whose only top-level qualifier is
  `const` is decided exactly as its unqualified type (C11 6.3.2.1p2: the value
  read from a qualified lvalue has the unqualified type, so it is the same
  pointer); the reader builds the same shape for both, so P2's statement
  lowering, P3's return lowering and position decision, and the borrowed read
  are the same functions of the same arguments; the rule only adds, and an
  unqualified value is decided as before;
* **unlisted qualifiers refuse**: `volatile`, `restrict`, `_Atomic`, a
  repeated or combined list, in every position;
* **the bit-cast arm preserves obligations**: an admitted implicit bit-cast
  from `void *` to an object pointer lowers to the inner path read's own
  borrowed read, with its hops unchanged and no custody bound; an explicit
  cast, any other conversion, a cast to a function or `void` pointer, a cast
  from an object pointer, a non-pointer destination, a value that is not a
  typed path and a cast in the middle of a path refuse;
* non-vacuity witnesses for both arms.
-/

namespace QCore.TypedConstPointerValue

open QCore.IndirectScalarAccess

/-! ## Qualifier erasure (step 1) -/

/-- A top-level qualifier Clang prints after a pointer type's last `*`. -/
inductive Qualifier where
  | const
  | volatile
  | restrict
  | atomic
  deriving DecidableEq, Repr

/-- A pointer-typed value as the extraction reads it: whether its unqualified
type passes the unchanged positive pointer test (`is_pointer`), and its
top-level qualifier list. -/
structure ValueType where
  unqualifiedPointer : Bool
  qualifiers : List Qualifier
  deriving DecidableEq, Repr

/-- The reader before this card: an unqualified positive pointer only. -/
def oldAdmits (t : ValueType) : Bool :=
  t.unqualifiedPointer && t.qualifiers.isEmpty

/-- The qualifier lists this card erases: none, or exactly one `const`. -/
def erasable (l : List Qualifier) : Bool :=
  l.isEmpty || l == [.const]

def admits (t : ValueType) : Bool :=
  t.unqualifiedPointer && erasable t.qualifiers

/-- The value read from the lvalue: its unqualified type (C11 6.3.2.1p2). -/
def erase (t : ValueType) : ValueType :=
  { t with qualifiers := [] }

theorem the_rule_only_adds (t : ValueType) (h : oldAdmits t = true) : admits t = true := by
  cases t with
  | mk p l =>
    simp only [oldAdmits, admits, erasable, Bool.and_eq_true] at h ⊢
    exact ⟨h.1, by simp [h.2]⟩

theorem an_unqualified_value_is_decided_as_before (t : ValueType) (h : t.qualifiers = []) :
    admits t = oldAdmits t := by
  simp [admits, oldAdmits, h, erasable]

theorem a_non_pointer_value_refuses (t : ValueType) (h : t.unqualifiedPointer = false) :
    admits t = false := by
  simp [admits, h]

/-- Every admitted value is, after erasure, a value the old reader admitted:
the read is of the same unqualified pointer. -/
theorem an_admitted_value_erases_to_an_old_admission (t : ValueType) (h : admits t = true) :
    oldAdmits (erase t) = true := by
  simp [admits] at h
  simp [oldAdmits, erase, h.1]

theorem erasure_keeps_the_decision (t : ValueType) (h : admits t = true) :
    admits (erase t) = true :=
  the_rule_only_adds (erase t) (an_admitted_value_erases_to_an_old_admission t h)

/-- An unlisted qualifier anywhere in the list refuses. -/
theorem an_unlisted_qualifier_refuses (t : ValueType) (q : Qualifier) (hq : q ∈ t.qualifiers)
    (hne : q ≠ .const) : admits t = false := by
  cases t with
  | mk p l =>
    have h1 : l.isEmpty = false := by
      cases l with
      | nil => simp at hq
      | cons _ _ => rfl
    have h2 : (l == [Qualifier.const]) = false := by
      cases hl : (l == [Qualifier.const])
      · rfl
      · have hc : l = [Qualifier.const] := by simpa using hl
        subst hc
        simp at hq
        exact absurd hq hne
    simp [admits, erasable, h1, h2]

theorem a_volatile_value_refuses (t : ValueType) (h : Qualifier.volatile ∈ t.qualifiers) :
    admits t = false :=
  an_unlisted_qualifier_refuses t .volatile h (by decide)

theorem a_restrict_value_refuses (t : ValueType) (h : Qualifier.restrict ∈ t.qualifiers) :
    admits t = false :=
  an_unlisted_qualifier_refuses t .restrict h (by decide)

theorem an_atomic_value_refuses (t : ValueType) (h : Qualifier.atomic ∈ t.qualifiers) :
    admits t = false :=
  an_unlisted_qualifier_refuses t .atomic h (by decide)

theorem a_repeated_const_refuses (p : Bool) :
    admits { unqualifiedPointer := p, qualifiers := [.const, .const] } = false := by
  cases p <;> decide

/-! ### The shape the reader builds, and every lowering over it -/

/-- The shape of a pointer read whose value type is `t`, every other
extraction fact `s` unchanged: the value type decides `knownValueType` and
`borrowedPointerValue`, and nothing else. -/
def shapeOf (s : TypedAccessPathStatements.Shape) (t : ValueType) :
    TypedAccessPathStatements.Shape :=
  { s with knownValueType := admits t, borrowedPointerValue := admits t }

def statementOf (p : TypedAccessPathStatements.Statement) (t : ValueType) :
    TypedAccessPathStatements.Statement :=
  { p with shape := shapeOf p.shape t }

/-- The same pointer: an admitted qualified value and its erasure build the
same shape. -/
theorem erasure_preserves_the_shape (s : TypedAccessPathStatements.Shape) (t : ValueType)
    (h : admits t = true) : shapeOf s t = shapeOf s (erase t) := by
  simp [shapeOf, h, erasure_keeps_the_decision t h]

theorem erasure_preserves_the_statement_lowering (st : IndirectChainAccess.State)
    (p : TypedAccessPathStatements.Statement) (t : ValueType) (h : admits t = true) :
    TypedAccessPathStatements.lower st (statementOf p t) =
      TypedAccessPathStatements.lower st (statementOf p (erase t)) := by
  simp [statementOf, erasure_preserves_the_shape p.shape t h]

theorem erasure_preserves_the_borrowed_read (st : IndirectChainAccess.State)
    (p : TypedAccessPathStatements.Statement) (t : ValueType) (d : Nat) (h : admits t = true) :
    TypedAccessPathStatements.lowerBorrowed st (statementOf p t) d =
      TypedAccessPathStatements.lowerBorrowed st (statementOf p (erase t)) d := by
  simp [statementOf, erasure_preserves_the_shape p.shape t h]

theorem erasure_preserves_the_return (st : IndirectChainAccess.State)
    (p : TypedAccessPathStatements.Statement) (t : ValueType) (h : admits t = true) :
    TypedAccessPathPositions.lowerReturn st (statementOf p t) =
      TypedAccessPathPositions.lowerReturn st (statementOf p (erase t)) := by
  simp [statementOf, erasure_preserves_the_shape p.shape t h]

/-- P3's position decision (older route first, no connective, rule 5b's
root premise for a returned pointer) is the same for both. -/
theorem erasure_preserves_the_position_decision (c : TypedAccessPathPositions.Candidate)
    (t : ValueType) (h : admits t = true) :
    TypedAccessPathPositions.admits { c with shape := shapeOf c.shape t } =
      TypedAccessPathPositions.admits { c with shape := shapeOf c.shape (erase t) } := by
  rw [erasure_preserves_the_shape c.shape t h]

theorem a_const_pointer_return_hands_nothing_back (st : IndirectChainAccess.State)
    (p : TypedAccessPathStatements.Statement) (t : ValueType)
    (r : IndirectChainAccess.State × List Diagnostic × TypedAccessPathPositions.Exit)
    (h : TypedAccessPathPositions.lowerReturn st (statementOf p t) = some r) :
    r.2.2 = .handsNothingBack :=
  TypedAccessPathPositions.a_typed_return_hands_nothing_back st (statementOf p t) r h

/-- Rule 5b's premise is untouched: a returned `const` pointer whose root
is not untracked refuses exactly as an unqualified one. -/
theorem a_const_pointer_return_from_a_tracked_root_refuses
    (c : TypedAccessPathPositions.Candidate) (t : ValueType)
    (hp : c.position = .returnExpression) (hv : c.pointerValue = true)
    (hr : c.rootsUntracked = false) :
    TypedAccessPathPositions.admits { c with shape := shapeOf c.shape t } = false :=
  TypedAccessPathPositions.a_returned_pointer_from_a_tracked_root_refuses
    { c with shape := shapeOf c.shape t } hp hv hr

theorem a_const_pointer_read_binds_no_custody (st : IndirectChainAccess.State)
    (p : TypedAccessPathStatements.Statement) (t : ValueType) (d : Nat)
    (ha : TypedAccessPathStatements.admits (shapeOf p.shape t) = true)
    (ht : admits t = true) (hf : p.scalar.frame = none) :
    ∃ after diagnostics,
      TypedAccessPathStatements.lowerBorrowed st (statementOf p t) d = some (after, diagnostics) ∧
      (after.base.binding d).map Binding.targets = some [] ∧
      after.base.rest = st.base.rest ∧ after.fieldLink = st.fieldLink :=
  TypedAccessPathStatements.pointer_read_binds_no_custody st (statementOf p t) d ha
    (by simp [statementOf, shapeOf, ht]) hf

/-- An unlisted qualifier's value type proves no value type: every
statement, return and position refuses. -/
theorem an_unlisted_qualifier_refuses_in_every_position
    (c : TypedAccessPathPositions.Candidate) (t : ValueType) (h : admits t = false) :
    TypedAccessPathPositions.admits { c with shape := shapeOf c.shape t } = false :=
  TypedAccessPathPositions.every_statement_refusal_holds_in_every_position _
    (TypedAccessPathStatements.unproven_value_type_refuses _ (by simp [shapeOf, h]))

theorem an_unlisted_qualifier_has_no_lowering (st : IndirectChainAccess.State)
    (p : TypedAccessPathStatements.Statement) (t : ValueType) (h : admits t = false) :
    TypedAccessPathStatements.lower st (statementOf p t) = none :=
  TypedAccessPathStatements.refused_shape_has_no_lowering st _
    (TypedAccessPathStatements.unproven_value_type_refuses _ (by simp [statementOf, shapeOf, h]))

/-! ## The `void *` bit-cast arm (step 2) -/

/-- The conversion at the top of a pointer-destination statement's value. -/
inductive Conversion where
  /-- Only `strip`'s own wrappers (parentheses, lvalue, no-op, decay). -/
  | stripped
  | implicitBitCast
  | explicitCast
  /-- Any other implicit conversion (integral, pointer-to-integer, …). -/
  | otherImplicit
  deriving DecidableEq, Repr

/-- The pointee class of a pointer type. -/
inductive Pointee where
  | void
  | object
  | function
  deriving DecidableEq, Repr

structure Source where
  conversion : Conversion
  /-- The pointee of the converted value (the type under the cast). -/
  fromPointee : Pointee
  /-- The destination's declared pointee. -/
  toPointee : Pointee
  /-- The destination is a declared pointer name or declarator. -/
  destinationPointer : Bool
  /-- Under the conversion is a typed access path the reader reads. -/
  pathValue : Bool
  deriving DecidableEq, Repr

def castAdmits (c : Source) : Bool :=
  c.destinationPointer && c.pathValue &&
    match c.conversion with
    | .stripped => true
    | .implicitBitCast => c.fromPointee == .void && c.toPointee == .object
    | _ => false

/-- The statement read: the inner path's own plan, as P2 reads it, when the
conversion is admitted. The cast adds no hop and removes none. -/
def lowerCast (st : IndirectChainAccess.State) (c : Source)
    (p : TypedAccessPathStatements.Statement) (d : Nat) :
    Option (IndirectChainAccess.State × List Diagnostic) :=
  if castAdmits c then TypedAccessPathStatements.lowerBorrowed st p d else none

/-- The arm only adds: without a cast the decision is P2's. -/
theorem an_uncast_value_is_decided_as_before (c : Source) (h : c.conversion = .stripped) :
    castAdmits c = (c.destinationPointer && c.pathValue) := by
  simp [castAdmits, h]

theorem the_bit_cast_arm_preserves_every_obligation (st : IndirectChainAccess.State)
    (c : Source) (p : TypedAccessPathStatements.Statement) (d : Nat) (h : castAdmits c = true) :
    lowerCast st c p d = TypedAccessPathStatements.lowerBorrowed st p d := by
  simp [lowerCast, h]

/-- An admitted bit-cast read binds a borrowed, target-free alias and moves
no custody or field link, exactly as the uncast read. -/
theorem a_bit_cast_read_binds_no_custody (st : IndirectChainAccess.State) (c : Source)
    (p : TypedAccessPathStatements.Statement) (d : Nat) (hc : castAdmits c = true)
    (ha : TypedAccessPathStatements.admits p.shape = true)
    (hp : p.shape.borrowedPointerValue = true) (hf : p.scalar.frame = none) :
    ∃ after diagnostics, lowerCast st c p d = some (after, diagnostics) ∧
      (after.base.binding d).map Binding.targets = some [] ∧
      after.base.rest = st.base.rest ∧ after.fieldLink = st.fieldLink := by
  rw [the_bit_cast_arm_preserves_every_obligation st c p d hc]
  exact TypedAccessPathStatements.pointer_read_binds_no_custody st p d ha hp hf

theorem an_explicit_cast_refuses (c : Source) (h : c.conversion = .explicitCast) :
    castAdmits c = false := by
  simp [castAdmits, h]

theorem any_other_implicit_conversion_refuses (c : Source) (h : c.conversion = .otherImplicit) :
    castAdmits c = false := by
  simp [castAdmits, h]

theorem a_cast_to_a_function_pointer_refuses (c : Source) (hc : c.conversion = .implicitBitCast)
    (h : c.toPointee = .function) : castAdmits c = false := by
  simp [castAdmits, hc, h]

theorem a_cast_to_a_void_pointer_refuses (c : Source) (hc : c.conversion = .implicitBitCast)
    (h : c.toPointee = .void) : castAdmits c = false := by
  simp [castAdmits, hc, h]

theorem a_cast_from_an_object_pointer_refuses (c : Source) (hc : c.conversion = .implicitBitCast)
    (h : c.fromPointee = .object) : castAdmits c = false := by
  simp [castAdmits, hc, h]

theorem a_non_pointer_destination_refuses (c : Source) (h : c.destinationPointer = false) :
    castAdmits c = false := by
  simp [castAdmits, h]

theorem a_cast_of_anything_but_a_typed_path_refuses (c : Source) (h : c.pathValue = false) :
    castAdmits c = false := by
  simp [castAdmits, h]

theorem a_refused_conversion_has_no_lowering (st : IndirectChainAccess.State) (c : Source)
    (p : TypedAccessPathStatements.Statement) (d : Nat) (h : castAdmits c = false) :
    lowerCast st c p d = none := by
  simp [lowerCast, h]

/-- A cast inside the path (not at the top of the value) is P2's own
`castInPath` refusal: no lowering, whatever the top conversion. -/
theorem a_cast_in_the_middle_of_a_path_refuses (st : IndirectChainAccess.State) (c : Source)
    (p : TypedAccessPathStatements.Statement) (d : Nat) (h : p.shape.castInPath = true) :
    lowerCast st c p d = none := by
  unfold lowerCast
  split
  · exact TypedAccessPathStatements.refused_pointer_read_stays_unknown st p d
      (TypedAccessPathStatements.cast_refuses p.shape h)
  · rfl

/-! ## Non-vacuity -/

/-- `node->host` through `const rd_kafka_Node_t *`: a `char *const` value. -/
def constCharPointer : ValueType := { unqualifiedPointer := true, qualifiers := [.const] }

theorem a_const_pointer_value_is_admitted : admits constCharPointer = true := by decide

theorem the_old_reader_refused_it : oldAdmits constCharPointer = false := by decide

theorem a_const_volatile_value_is_refused :
    admits { constCharPointer with qualifiers := [.const, .volatile] } = false := by decide

/-- `parpriv = rktpar->_private`: a `void *` member value, implicitly
bit-cast to a `const` object pointer declared local. -/
def leaderEpoch : Source :=
  { conversion := .implicitBitCast, fromPointee := .void, toPointee := .object,
    destinationPointer := true, pathValue := true }

theorem the_void_bit_cast_is_admitted : castAdmits leaderEpoch = true := by decide

theorem the_same_value_under_an_explicit_cast_is_refused :
    castAdmits { leaderEpoch with conversion := .explicitCast } = false := by decide

end QCore.TypedConstPointerValue

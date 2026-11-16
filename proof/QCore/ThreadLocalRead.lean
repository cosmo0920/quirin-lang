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
# A thread-local pointer read is a custody-free value (card THREAD-LOCAL-CUSTODY-FREE-READ, gateway rule 54)

The abstract rule implemented by `crates/qcore-c-gateway/src/thread_local_read.rs`
(`decide`, `admitted`) and its arm in `legacy_transfer::transfer_base`.

A function's references to a thread-local pointer object, and to the automatic
locals that receive a copy of one, are each classified by their use. The
reading is admitted for the whole function only when every reference is a
custody-free use: a copy into a local, a local's initializer, a `return`
operand or a null test (and, for a copy, its own rebinding). A write to the
object, `&` of it, a member selection or dereference, a call actual (a release
or transfer through it) or any other use withholds the reading.

Where admitted, the copy drops the destination's binding: the transfer of a
`pthread_getspecific` result whose contract is `resource_return_none()`. No
obligation is created or discharged; a release or escape of the copy finds no
binding, so the release refuses and the escape escapes nothing. Where withheld,
the old alias transfer runs unchanged.

## What this is not

A model of the decision and of the transfer of one copy, over an abstract
state of name bindings and obligation states. That a whole read of a
thread-local object yields the value some earlier store left there (C11 6.2.4p4,
the object lives for its thread), Clang's extraction of the `tls` field, the
declaration ids, types and parent nodes, and route G's lowering of the copy as
one alias node are premises (trust boundary 28), not proved.
-/

namespace QCore.ThreadLocalRead

/-! ## The decision -/

inductive Subject where
  | object
  | copy
  deriving DecidableEq, Repr

inductive Use where
  | copyToLocal
  | initLocal
  | «return»
  | nullTest
  | rebind
  | write
  | address
  | dereference
  | callArgument
  | notObjectPointer
  | other
  deriving DecidableEq, Repr

inductive Decision where
  | admit
  | withhold
  deriving DecidableEq, Repr

/-- `thread_local_read::decide`. -/
def decide : Subject → Use → Decision
  | _, .copyToLocal => .admit
  | _, .initLocal => .admit
  | _, .return => .admit
  | _, .nullTest => .admit
  | .copy, .rebind => .admit
  | _, _ => .withhold

/-- The decision vectors, spelled as `thread_local_read::DECISION_VECTORS`. -/
def decisionVectors : List (Subject × Use × Decision) :=
  [ (.object, .copyToLocal, .admit)
  , (.object, .initLocal, .admit)
  , (.object, .return, .admit)
  , (.object, .nullTest, .admit)
  , (.object, .rebind, .withhold)
  , (.object, .write, .withhold)
  , (.object, .address, .withhold)
  , (.object, .dereference, .withhold)
  , (.object, .callArgument, .withhold)
  , (.object, .notObjectPointer, .withhold)
  , (.object, .other, .withhold)
  , (.copy, .copyToLocal, .admit)
  , (.copy, .initLocal, .admit)
  , (.copy, .return, .admit)
  , (.copy, .nullTest, .admit)
  , (.copy, .rebind, .admit)
  , (.copy, .write, .withhold)
  , (.copy, .address, .withhold)
  , (.copy, .dereference, .withhold)
  , (.copy, .callArgument, .withhold)
  , (.copy, .notObjectPointer, .withhold)
  , (.copy, .other, .withhold) ]

theorem decision_vectors_hold :
    decisionVectors.all (fun (s, u, d) => decide s u == d) = true := by
  decide

/-- The reading for a whole function: every reference admitted. -/
def admits (refs : List (Subject × Use)) : Bool :=
  refs.all (fun (s, u) => decide s u == .admit)

/-- A use is custody-free exactly when it is a copy, an initializer, a return
or a null test, or a copy's own rebinding. -/
theorem admitted_exactly_on_a_custody_free_use (s : Subject) (u : Use) :
    decide s u = .admit ↔
      (u = .copyToLocal ∨ u = .initLocal ∨ u = .return ∨ u = .nullTest ∨
        (s = .copy ∧ u = .rebind)) := by
  cases s <;> cases u <;> simp [decide]

/-- A write to the thread-local object itself withholds the reading: a
take-from-thread-local would otherwise hide a transfer behind a custody-free
value. -/
theorem a_write_to_the_object_withholds (refs : List (Subject × Use))
    (u : Use) (hu : u = .rebind ∨ u = .write) (h : (Subject.object, u) ∈ refs) :
    admits refs = false := by
  unfold admits
  rw [List.all_eq_false]
  refine ⟨(Subject.object, u), h, ?_⟩
  rcases hu with hu | hu <;> subst hu <;> decide

/-- `&` of the object or of a copy, a member selection or dereference through
either (a store, release or transfer through it) and a call actual each
withhold the reading. -/
theorem a_use_through_it_withholds (refs : List (Subject × Use)) (s : Subject)
    (u : Use) (hu : u = .address ∨ u = .dereference ∨ u = .callArgument ∨ u = .other)
    (h : (s, u) ∈ refs) : admits refs = false := by
  unfold admits
  rw [List.all_eq_false]
  refine ⟨(s, u), h, ?_⟩
  rcases hu with hu | hu | hu | hu <;> subst hu <;> cases s <;> decide

/-- An object that is not of object-pointer type (an array, a record whose
member would be read, a function pointer, `volatile` or `_Atomic`) withholds
the reading. -/
theorem a_non_pointer_object_withholds (refs : List (Subject × Use))
    (h : (Subject.object, Use.notObjectPointer) ∈ refs) : admits refs = false := by
  unfold admits
  rw [List.all_eq_false]
  exact ⟨_, h, by decide⟩

/-! ## The transfer -/

/-- A binding of a name: the obligation sites it may denote. -/
structure Binding where
  targets : List Nat
  unknown : Bool
  mayBeNull : Bool
  deriving DecidableEq, Repr

/-- Bindings by name and obligation states by site; `true` is live. -/
structure State where
  bindings : Nat → Option Binding
  obligations : Nat → Option Bool

/-- Drop one name's binding. -/
def unbind (st : State) (d : Nat) : State :=
  { st with bindings := fun p => if p = d then none else st.bindings p }

/-- The transfer of a call whose contract result is `resource_return_none()`
into `d` (`pthread_getspecific`): the destination holds no binding afterwards. -/
def returnNoneCall (st : State) (d : Nat) : State := unbind st d

/-- The old alias transfer from an unbound source: an unknown, possibly null
binding and an unresolved-identity diagnostic (`true`). -/
def oldAlias (st : State) (d src : Nat) : State × Bool :=
  match st.bindings src with
  | some b => ({ st with bindings := fun p => if p = d then some b else st.bindings p }, false)
  | none =>
      ({ st with bindings := fun p =>
          if p = d then some { targets := [], unknown := true, mayBeNull := true }
          else st.bindings p }, true)

/-- Rule 54's alias transfer: where the reading is admitted and the source is
unbound, the copy drops the destination's binding with no diagnostic;
everywhere else the old transfer. -/
def alias (admitted : Bool) (st : State) (d src : Nat) : State × Bool :=
  if admitted && (st.bindings src).isNone then (unbind st d, false) else oldAlias st d src

/-- The admitted copy is `pthread_getspecific`'s reading. -/
theorem the_copy_is_the_return_none_reading (st : State) (d src : Nat)
    (h : st.bindings src = none) : alias true st d src = (returnNoneCall st d, false) := by
  simp [alias, returnNoneCall, h]

/-- No obligation is created or discharged by the copy. -/
theorem the_copy_changes_no_obligation (st : State) (d src : Nat) :
    (alias true st d src).1.obligations = st.obligations := by
  unfold alias oldAlias unbind
  split
  · rfl
  · cases st.bindings src <;> rfl

/-- The copy holds no binding afterwards. -/
theorem the_copy_is_unbound (st : State) (d src : Nat) (h : st.bindings src = none) :
    (alias true st d src).1.bindings d = none := by
  simp [alias, unbind, h]

/-- No other name's binding changes. -/
theorem no_other_binding_changes (st : State) (d src p : Nat) (h : st.bindings src = none)
    (hp : p ≠ d) : (alias true st d src).1.bindings p = st.bindings p := by
  simp [alias, unbind, h, hp]

/-- Withheld, the old answer is unchanged: the rule only adds. -/
theorem the_withheld_answer_is_unchanged (st : State) (d src : Nat) :
    alias false st d src = oldAlias st d src := by
  simp [alias]

/-- A bound source is never read by the rule. -/
theorem a_bound_source_keeps_the_old_answer (admitted : Bool) (st : State) (d src : Nat)
    (b : Binding) (h : st.bindings src = some b) :
    alias admitted st d src = oldAlias st d src := by
  simp [alias, h]

/-! ## Custody uses of the copy -/

inductive Outcome where
  | refused
  | discharged (site : Nat)
  deriving DecidableEq, Repr

/-- A release (or a transfer) of a name: refused without a binding naming
exactly one live site. -/
def release (st : State) (x : Nat) : Outcome :=
  match st.bindings x with
  | some { targets := [s], unknown := false, .. } =>
      if st.obligations s = some true then .discharged s else .refused
  | _ => .refused

/-- A release of the copy refuses: the caller or body that releases the
value still answers for it, and no obligation is discharged. -/
theorem a_release_of_the_copy_refuses (st : State) (d src : Nat)
    (h : st.bindings src = none) : release (alias true st d src).1 d = .refused := by
  simp [release, alias, unbind, h]

/-- `return` of a name escapes the obligation it names, if any. -/
def escapeIfOwned (st : State) (x : Nat) : List Nat :=
  match st.bindings x with
  | some b => b.targets
  | none => []

/-- Returning the copy escapes nothing: no fresh owner leaves the function. -/
theorem returning_the_copy_escapes_nothing (st : State) (d src : Nat)
    (h : st.bindings src = none) : escapeIfOwned (alias true st d src).1 d = [] := by
  simp [escapeIfOwned, alias, unbind, h]

end QCore.ThreadLocalRead

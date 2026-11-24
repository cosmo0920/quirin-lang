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
# Locals holding only loads from untracked objects (card K5-LOCAL-ORIGIN-ALIASES, gateway rule 77)

The abstract rule implemented by `crates/qcore-c-gateway/src/borrowed_path_local.rs`
(`decide`, `value_admits`, `untracked`, `read`, `guard_reads_untracked`,
`return_reads_untracked`) and its two arms in `legacy_transfer` (a null test's
edge refinement, a `return`'s `EscapeIfOwned`).

An automatic local is admitted when every value written to it is a null
pointer constant, a literal, a copy of an admitted local or a **load** of an
object-pointer member or element whose base reaches an admitted root (a
formal rule 5b's return premise answers for) or an admitted local, and every
use of it is a null test, a dereference base, a call actual, a `return`, its
own rebinding or a copy into an admitted local. A null test of such a local
whose binding has no target (identity derivation, unwidened state) narrows
that binding with no identity refusal; a `return` of it escapes nothing.

## What is proved

* The decision, value and binding tables and their vectors
  (`decision_vectors_hold`, `value_vectors_hold`, `binding_vectors_hold`), and
  that a step, an address, an unread use, a root's write or copy, or a value
  that is not a load, null, literal or admitted copy withholds the local.
* **Closure soundness** (`every_admitted_local_holds_an_untracked_value`): along
  any run of admitted writes, every admitted local holds an untracked value,
  given P2's premise that a load from an untracked object is untracked
  (`LoadsStayUntracked`) and that roots and literals are untracked. A cursor
  that loads from itself (`c = c->next`) is covered: the induction is over the
  run, not over the closure's construction.
* **The null test** (`guard`): admitted on an untracked binding, the state it
  leaves is exactly the old arm's (`the_admitted_guard_leaves_the_old_state`),
  no obligation moves (`the_guard_touches_no_obligation`), no other binding
  moves (`the_guard_narrows_only_its_binding`), and only the report and the
  unmodelled mark disappear (`the_admitted_guard_reports_nothing`). Withheld,
  the old answer stands (`a_withheld_guard_is_the_old_guard`).
* **The return** (`returned`): an untracked binding escapes nothing, which is
  also what the old arm escaped (`the_old_return_escaped_nothing_either`); only
  the report goes. A release of such a binding still refuses
  (`a_release_of_an_untracked_local_still_refuses`).

## What this is not

A model of the reading and of the two decisions over an abstract state.
Clang's extraction of declarations, storage, types, casts and parent nodes,
route G's lowering of the tested and returned names, the never-bound closure
and `may_bind` (rules 5b and 10l), and P2's premise that a never-bound root
reaches no tracked place (here `LoadsStayUntracked`, with the heap fixed over
the run: a store of a tracked value into an untracked place is refused by the
store rules, rule 5i, rule 10v) are premises (trust boundary 28), not proved
here.
-/

namespace QCore.BorrowedPathLocal

/-! ## The decision -/

inductive Subject where
  | root
  | local
  deriving DecidableEq, Repr

inductive Use where
  | copyToLocal
  | initLocal
  | nullTest
  | dereference
  | callArgument
  | rebind
  | «return»
  | write
  | address
  | other
  deriving DecidableEq, Repr

inductive Decision where
  | admit
  | withhold
  deriving DecidableEq, Repr

/-- `borrowed_path_local::decide`. -/
def decide : Subject → Use → Decision
  | .root, .nullTest => .admit
  | .root, .dereference => .admit
  | .root, .callArgument => .admit
  | .root, .return => .admit
  | .root, _ => .withhold
  | .local, .write => .withhold
  | .local, .address => .withhold
  | .local, .other => .withhold
  | .local, _ => .admit

/-- The decision vectors, spelled as `borrowed_path_local::DECISION_VECTORS`. -/
def decisionVectors : List (Subject × Use × Decision) :=
  [ (.root, .copyToLocal, .withhold)
  , (.root, .initLocal, .withhold)
  , (.root, .nullTest, .admit)
  , (.root, .dereference, .admit)
  , (.root, .callArgument, .admit)
  , (.root, .rebind, .withhold)
  , (.root, .return, .admit)
  , (.root, .write, .withhold)
  , (.root, .address, .withhold)
  , (.root, .other, .withhold)
  , (.local, .copyToLocal, .admit)
  , (.local, .initLocal, .admit)
  , (.local, .nullTest, .admit)
  , (.local, .dereference, .admit)
  , (.local, .callArgument, .admit)
  , (.local, .rebind, .admit)
  , (.local, .return, .admit)
  , (.local, .write, .withhold)
  , (.local, .address, .withhold)
  , (.local, .other, .withhold) ]

theorem decision_vectors_hold :
    decisionVectors.all (fun (s, u, d) => decide s u == d) = true := by
  decide

/-- A root is admitted exactly where it is only read. -/
theorem a_root_is_admitted_exactly_on_a_read (u : Use) :
    decide .root u = .admit ↔
      (u = .nullTest ∨ u = .dereference ∨ u = .callArgument ∨ u = .return) := by
  cases u <;> simp [decide]

/-- A step, a compound write, an address or an unread use withholds a local. -/
theorem a_step_or_address_withholds_the_local (u : Use)
    (hu : u = .write ∨ u = .address ∨ u = .other) : decide .local u = .withhold := by
  rcases hu with hu | hu | hu <;> subst hu <;> rfl

/-- The value kinds written to a local. -/
inductive Written where
  | null
  | literal
  | load
  | copy
  | other
  deriving DecidableEq, Repr

/-- `borrowed_path_local::value_admits`. -/
def valueAdmits : Written → Bool
  | .other => false
  | _ => true

/-- The value vectors, spelled as `borrowed_path_local::VALUE_VECTORS`. -/
def valueVectors : List (Written × Bool) :=
  [ (.null, true)
  , (.literal, true)
  , (.load, true)
  , (.copy, true)
  , (.other, false) ]

theorem value_vectors_hold :
    valueVectors.all (fun (w, b) => valueAdmits w == b) = true := by
  decide

/-- `borrowed_path_local::untracked`: no target, identity, not widened. -/
def untracked (targetsEmpty identity widened : Bool) : Bool :=
  targetsEmpty && identity && !widened

/-- The binding vectors, spelled as `borrowed_path_local::BINDING_VECTORS`. -/
def bindingVectors : List (Bool × Bool × Bool × Bool) :=
  [ (true, true, false, true)
  , (false, true, false, false)
  , (true, false, false, false)
  , (true, true, true, false) ]

theorem binding_vectors_hold :
    bindingVectors.all (fun (t, i, w, u) => untracked t i w == u) = true := by
  decide

/-- A widened state is never read as untracked: its targets were forgotten. -/
theorem a_widened_state_is_never_untracked (t i : Bool) : untracked t i true = false := by
  cases t <;> cases i <;> rfl

/-- The reading of one local: every value admissible and every use admitted. -/
def admits (values : List Written) (uses : List Use) : Bool :=
  values.all valueAdmits && uses.all (fun u => decide .local u == .admit)

theorem an_other_value_withholds (values : List Written) (uses : List Use)
    (h : Written.other ∈ values) : admits values uses = false := by
  unfold admits
  have : values.all valueAdmits = false := by
    rw [List.all_eq_false]
    exact ⟨.other, h, by decide⟩
  rw [this]
  rfl

/-! ## Closure soundness -/

/-- Pointer expressions: a root formal, an admitted local, or a load of a
field of the object an expression points to. -/
inductive PExpr where
  | root (r : Nat)
  | loc (l : Nat)
  | load (base : PExpr) (field : Nat)
  deriving DecidableEq, Repr

/-- Values written to a local. A bare root is no admitted value (that is rule
62's copy); `other` stands for every refused shape. -/
inductive WExpr where
  | null
  | lit (a : Nat)
  | val (e : PExpr)
  | other
  deriving DecidableEq, Repr

/-- The memory a run reads: roots' values, literal addresses' tracking, and a
heap from (object, field) to the pointer stored there; `0` is null. -/
structure Memory where
  heap : Nat → Nat → Nat
  rootValue : Nat → Nat

def eval (m : Memory) (env : Nat → Nat) : PExpr → Nat
  | .root r => m.rootValue r
  | .loc l => env l
  | .load e f => m.heap (eval m env e) f

/-- Every name a pointer expression reaches is admitted. -/
def PExpr.admitted (roots locals : Nat → Bool) : PExpr → Bool
  | .root r => roots r
  | .loc l => locals l
  | .load e _ => e.admitted roots locals

/-- `borrowed_path_local`'s value rule, in this model. -/
def WExpr.admissible (roots locals : Nat → Bool) : WExpr → Bool
  | .null => true
  | .lit _ => true
  | .val (.root _) => false
  | .val e => e.admitted roots locals
  | .other => false

def assign (m : Memory) (env : Nat → Nat) (l : Nat) (w : WExpr) : Nat → Nat :=
  fun x => if x = l then
      (match w with
        | .null => 0
        | .lit a => a
        | .val e => eval m env e
        | .other => 0)
    else env x

/-- P2's premise, stated: a load from an untracked object is untracked. -/
def LoadsStayUntracked (tracked : Nat → Bool) (m : Memory) : Prop :=
  ∀ o f, tracked o = false → tracked (m.heap o f) = false

/-- The invariant: every admitted local holds an untracked value. -/
def Untracked (tracked : Nat → Bool) (locals : Nat → Bool) (env : Nat → Nat) : Prop :=
  ∀ l, locals l = true → tracked (env l) = false

theorem eval_untracked (tracked : Nat → Bool) (m : Memory) (roots locals : Nat → Bool)
    (env : Nat → Nat) (hp : LoadsStayUntracked tracked m)
    (hr : ∀ r, roots r = true → tracked (m.rootValue r) = false)
    (hl : Untracked tracked locals env) :
    ∀ e : PExpr, e.admitted roots locals = true → tracked (eval m env e) = false := by
  intro e
  induction e with
  | root r => intro h; exact hr r h
  | loc l => intro h; exact hl l h
  | load e f ih =>
      intro h
      exact hp _ f (ih h)

/-- One admitted write keeps the invariant. -/
theorem an_admitted_write_keeps_every_local_untracked (tracked : Nat → Bool) (m : Memory)
    (roots locals : Nat → Bool) (env : Nat → Nat) (hp : LoadsStayUntracked tracked m)
    (hr : ∀ r, roots r = true → tracked (m.rootValue r) = false)
    (hlit : ∀ a, tracked a = false) (h0 : tracked 0 = false)
    (hl : Untracked tracked locals env) (l : Nat) (w : WExpr)
    (hw : w.admissible roots locals = true) :
    Untracked tracked locals (assign m env l w) := by
  intro x hx
  unfold assign
  by_cases hxl : x = l
  · simp only [hxl, ite_true]
    cases w with
    | null => exact h0
    | lit a => exact hlit a
    | other => exact h0
    | val e =>
        simp only
        cases e with
        | root r => simp [WExpr.admissible] at hw
        | loc k =>
            exact eval_untracked tracked m roots locals env hp hr hl (.loc k)
              (by simpa [WExpr.admissible] using hw)
        | load b f =>
            exact eval_untracked tracked m roots locals env hp hr hl (.load b f)
              (by simpa [WExpr.admissible] using hw)
  · simp only [hxl, ite_false]
    exact hl x hx

/-- A run of writes to admitted locals. -/
def run (m : Memory) (env : Nat → Nat) : List (Nat × WExpr) → Nat → Nat
  | [] => env
  | (l, w) :: rest => run m (assign m env l w) rest

/-- Closure soundness: along any run of admitted writes from a state where
every admitted local is untracked (null at entry, or never read before its
first write: the no-undefined-behaviour reading), every admitted local holds
an untracked value. A self-loading cursor is one more admitted write. -/
theorem every_admitted_local_holds_an_untracked_value (tracked : Nat → Bool) (m : Memory)
    (roots locals : Nat → Bool) (hp : LoadsStayUntracked tracked m)
    (hr : ∀ r, roots r = true → tracked (m.rootValue r) = false)
    (hlit : ∀ a, tracked a = false) (h0 : tracked 0 = false)
    (prog : List (Nat × WExpr))
    (hprog : ∀ p ∈ prog, locals p.1 = true ∧ p.2.admissible roots locals = true)
    (env : Nat → Nat) (hl : Untracked tracked locals env) :
    Untracked tracked locals (run m env prog) := by
  induction prog generalizing env with
  | nil => simpa [run] using hl
  | cons p rest ih =>
      obtain ⟨l, w⟩ := p
      simp only [run]
      apply ih
      · intro q hq
        exact hprog q (List.mem_cons_of_mem _ hq)
      · exact an_admitted_write_keeps_every_local_untracked tracked m roots locals env hp hr
          hlit h0 hl l w (hprog (l, w) List.mem_cons_self).2

/-! ## The null test -/

/-- A binding of a name: its targets, the unknown and possibly-null flags,
and whether it is the identity derivation. -/
structure Binding where
  targets : List Nat
  unknown : Bool
  mayBeNull : Bool
  identity : Bool
  deriving DecidableEq, Repr

structure State where
  bindings : Nat → Option Binding
  obligations : Nat → Option Bool
  widened : Bool

def Binding.untrackedIn (b : Binding) (st : State) : Bool :=
  untracked b.targets.isEmpty b.identity st.widened

/-- Rule 10b's narrowing of a tested binding on one edge. -/
def narrow (b : Binding) (assumeNull : Bool) : Binding :=
  if assumeNull then { b with targets := [], unknown := false, mayBeNull := true }
  else { b with mayBeNull := false }

def setBinding (st : State) (x : Nat) (b : Binding) : State :=
  { st with bindings := fun p => if p = x then some b else st.bindings p }

/-- The old arm for an unknown binding: narrow it, report the identity, mark
the edge unmodelled. Result: state, report, unmodelled. -/
def oldGuard (st : State) (x : Nat) (assumeNull : Bool) : State × Bool × Bool :=
  match st.bindings x with
  | some b => if b.unknown then (setBinding st x (narrow b assumeNull), true, true)
      else (st, false, false)
  | none => (st, false, false)

/-- Rule 77's arm: an admitted local with an untracked binding is decided. -/
def guard (admitted : Bool) (st : State) (x : Nat) (assumeNull : Bool) : State × Bool × Bool :=
  match st.bindings x with
  | some b =>
      if admitted && b.untrackedIn st && b.unknown then
        (setBinding st x (narrow b assumeNull), false, false)
      else oldGuard st x assumeNull
  | none => oldGuard st x assumeNull

theorem the_admitted_guard_leaves_the_old_state (st : State) (x : Nat) (b : Binding)
    (hb : st.bindings x = some b) (assumeNull : Bool) :
    (guard true st x assumeNull).1 = (oldGuard st x assumeNull).1 := by
  unfold guard oldGuard
  rw [hb]
  by_cases hu : b.unknown = true
  · by_cases ht : b.untrackedIn st = true <;> simp [hu, ht]
  · simp [hu]

theorem the_guard_touches_no_obligation (admitted : Bool) (st : State) (x : Nat)
    (assumeNull : Bool) : (guard admitted st x assumeNull).1.obligations = st.obligations := by
  unfold guard oldGuard setBinding
  cases st.bindings x with
  | none => rfl
  | some b =>
      simp only
      split
      · rfl
      · split <;> rfl

theorem the_guard_narrows_only_its_binding (admitted : Bool) (st : State) (x p : Nat)
    (hp : p ≠ x) (assumeNull : Bool) :
    (guard admitted st x assumeNull).1.bindings p = st.bindings p := by
  unfold guard oldGuard setBinding
  cases st.bindings x with
  | none => rfl
  | some b =>
      simp only
      split
      · simp [hp]
      · split
        · simp [hp]
        · rfl

theorem the_admitted_guard_reports_nothing (st : State) (x : Nat) (b : Binding)
    (hb : st.bindings x = some b) (hu : b.untrackedIn st = true) (hk : b.unknown = true)
    (assumeNull : Bool) : (guard true st x assumeNull).2 = (false, false) := by
  simp [guard, hb, hu, hk]

theorem a_withheld_guard_is_the_old_guard (st : State) (x : Nat) (assumeNull : Bool) :
    guard false st x assumeNull = oldGuard st x assumeNull := by
  unfold guard
  cases st.bindings x <;> simp

/-- A tracked binding is never decided by the rule. -/
theorem a_tracked_binding_keeps_the_old_guard (admitted : Bool) (st : State) (x : Nat)
    (b : Binding) (hb : st.bindings x = some b) (ht : b.targets ≠ []) (assumeNull : Bool) :
    guard admitted st x assumeNull = oldGuard st x assumeNull := by
  have : b.untrackedIn st = false := by
    cases h : b.targets with
    | nil => exact absurd h ht
    | cons s rest => simp [Binding.untrackedIn, untracked, h]
  simp [guard, hb, this]

/-! ## The return and custody uses -/

/-- What a `return` escapes: the targets of the returned binding. -/
def escapes (st : State) (x : Nat) : List Nat :=
  match st.bindings x with
  | some b => b.targets
  | none => []

/-- Rule 77's return: an admitted local with an untracked binding escapes
nothing and reports nothing (`false`). -/
def returned (admitted : Bool) (st : State) (x : Nat) : List Nat × Bool :=
  match st.bindings x with
  | some b => if admitted && b.untrackedIn st then ([], false) else (b.targets, b.unknown)
  | none => ([], false)

theorem returning_an_untracked_local_escapes_nothing (st : State) (x : Nat) (b : Binding)
    (hb : st.bindings x = some b) (hu : b.untrackedIn st = true) :
    returned true st x = ([], false) := by
  simp [returned, hb, hu]

theorem the_old_return_escaped_nothing_either (st : State) (x : Nat) (b : Binding)
    (hb : st.bindings x = some b) (hu : b.untrackedIn st = true) : escapes st x = [] := by
  simp only [Binding.untrackedIn, untracked, Bool.and_eq_true, Bool.not_eq_true'] at hu
  simp [escapes, hb, List.isEmpty_iff.mp hu.1.1]

inductive Outcome where
  | refused
  | discharged (site : Nat)
  deriving DecidableEq, Repr

/-- A release: refused without a binding naming exactly one live site. -/
def release (st : State) (x : Nat) : Outcome :=
  match st.bindings x with
  | some { targets := [s], unknown := false, .. } =>
      if st.obligations s = some true then .discharged s else .refused
  | _ => .refused

theorem a_release_of_an_untracked_local_still_refuses (st : State) (x : Nat) (b : Binding)
    (hb : st.bindings x = some b) (hu : b.untrackedIn st = true) : release st x = .refused := by
  simp only [Binding.untrackedIn, untracked, Bool.and_eq_true, Bool.not_eq_true'] at hu
  have ht : b.targets = [] := List.isEmpty_iff.mp hu.1.1
  unfold release
  rw [hb]
  obtain ⟨targets, unknown, mayBeNull, identity⟩ := b
  simp only at ht
  subst ht
  rfl

end QCore.BorrowedPathLocal

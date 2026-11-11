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

import QCore.NullTriage

/-!
# Scoped member invariants (card NULL-MEMBER-INVARIANT-SCOPE, gateway rules 14x–14z)

The abstract rules of `crates/qcore-core/src/null_member_scope.rs`, of the
gateway walk's stable-member reading (`crates/qcore-c-gateway/src/null_deref/`
`walk.rs`, `scope.rs`) and of the run fold's stable and scope sections
(`crates/qcore-audit/src/null_member_scope_fold.rs`), on top of
`QCore.NullTriage`.

A global member declaration (rule 14t) is withdrawn by one NULL store anywhere.
This module models the scoped readings that stay sound when some stores are
NULL:

* **A heap of objects** (`Heap`): each object either does not exist or has a
  member that is NULL or not. A trace (`Ev`) creates fresh objects (an
  initializer list, `fresh`), stores to an existing object's member (`store`,
  proved non-null or not), sets an existing object's member to NULL (`exit`,
  a reset or an unlink), ends an object (`free`), reads the member (`read`),
  or does anything else (`other`: a call, a store elsewhere, a
  synchronisation; on any thread, since the trace is the whole program's).
* **Stable members** (`stable_fact_persists`). When every store to an
  existing object is proved non-null and no exit store occurs, a member seen
  non-null stays non-null for the object's lifetime, whatever events
  intervene. A fresh object's initializer never changes an existing object
  (`a_fresh_store_does_not_reach`), so NULL initializers are harmless.
* **Function scopes** (`a_scoped_use_sees_non_null`). After the entry
  obligation (the caller proved the member non-null at the call) and with the
  member stable, every read in the scope sees it non-null.
* **Exit stores have no such reading** (`an_exit_store_breaks_persistence`,
  `an_unproved_store_breaks_persistence`): a single NULL store to an existing
  object makes a later read see NULL, so a member with one
  (`rd_kafka_buf_s.rkbuf_rkb`, `mk_list.next`, `cfl_list.next`) is not
  declared.
* **Persistence keeps only non-null facts** (`a_kept_fact_is_non_null`,
  `nothing_is_kept_without_stability`); **withdrawal**: a failed store
  withdraws the declaration, never `validated` (`a_stable_declaration_is_never_validated`,
  `a_failed_store_withdraws_it`), and a store whose proof rests on a withdrawn
  declaration fails under every larger withdrawn set
  (`failure_is_monotone`), so the fold's iteration is the least fixed point.
  A row that relied on a withdrawn declaration is never `Safe`
  (`QCore.NullTriage.a_withdrawn_dependency_is_never_safe`).

## What this is not

A model of C or of the gateway's reading of Clang's AST. That the walk keeps
exactly the facts on `V->FIELD` for a tracked `V` (whose value only an
assignment the walk sees changes), that it records every assignment, update,
initializer list, struct assignment and typed fill or copy of a stable member,
and that whole-object writes through an untyped pointer are a listed premise
(`whole-object`), are review premises (trust boundary 28), pinned by the
controls in `crates/qcore-c-gateway/tests/null_member_invariant_scope.rs`. As
for rule 14t, a store made by code the run does not analyse is outside it.
-/

namespace QCore.NullMemberScope

/-! ## The heap and its events -/

/-- An object's member: `none`, no such object; `some true`, NULL;
`some false`, non-null. -/
abbrev Heap := Nat → Option Bool

inductive Ev where
  | fresh (o : Nat) (null : Bool)
  | store (o : Nat) (proved : Bool)
  | exit (o : Nat)
  | free (o : Nat)
  | read (o : Nat)
  | other
  deriving DecidableEq, Repr

/-- One step. An unproved store may store anything (`oracle`); a store to an
object that does not exist changes nothing. -/
def step (oracle : Nat → Bool) (h : Heap) : Ev → Heap
  | .fresh o b => fun x => if x = o then some b else h x
  | .store o p => fun x =>
      if x = o then (match h o with
        | some _ => some (if p then false else oracle o)
        | none => none)
      else h x
  | .exit o => fun x =>
      if x = o then (match h o with
        | some _ => some true
        | none => none)
      else h x
  | .free o => fun x => if x = o then none else h x
  | .read _ => h
  | .other => h

/-- A fresh object is one no live object is: its event names an object that
does not exist. -/
def wf (oracle : Nat → Bool) : Heap → List Ev → Prop
  | _, [] => True
  | h, e :: rest =>
      (match e with
        | .fresh o _ => h o = none
        | _ => True) ∧ wf oracle (step oracle h e) rest

/-- Every store to an existing object is proved non-null and no exit store
occurs (what the stable declaration's store check establishes). -/
def stable : List Ev → Bool
  | [] => true
  | .store _ p :: rest => p && stable rest
  | .exit _ :: _ => false
  | _ :: rest => stable rest

/-- The trace does not end object `o`. -/
def keeps (o : Nat) : List Ev → Bool
  | [] => true
  | .free x :: rest => x != o && keeps o rest
  | _ :: rest => keeps o rest

def run (oracle : Nat → Bool) : Heap → List Ev → Heap
  | h, [] => h
  | h, e :: rest => run oracle (step oracle h e) rest

/-- One step keeps an existing non-null member of `o` non-null. -/
theorem step_keeps (oracle : Nat → Bool) (h : Heap) (e : Ev) (o : Nat)
    (hwf : match e with | .fresh x _ => h x = none | _ => True)
    (hs : stable [e] = true) (hk : keeps o [e] = true) (ho : h o = some false) :
    step oracle h e o = some false := by
  cases e with
  | fresh x b =>
      simp only at hwf
      have hne : o ≠ x := by
        intro hox; subst hox; rw [ho] at hwf; exact absurd hwf (by simp)
      simp [step, hne, ho]
  | store x p =>
      have hp : p = true := by simpa [stable] using hs
      subst hp
      by_cases hox : o = x
      · subst hox; simp [step, ho]
      · simp [step, hox, ho]
  | exit x => simp [stable] at hs
  | free x =>
      have hne : o ≠ x := by
        intro hox; subst hox; simp [keeps] at hk
      simp [step, hne, ho]
  | read x => simpa [step] using ho
  | other => simpa [step] using ho

/-- **A stable member's non-null value persists.** On a well-formed trace
whose every store is proved and which has no exit store and does not end
`o`, a member of `o` that is non-null stays non-null. -/
theorem stable_fact_persists (oracle : Nat → Bool) :
    ∀ (trace : List Ev) (h : Heap) (o : Nat),
      wf oracle h trace → stable trace = true → keeps o trace = true →
      h o = some false → run oracle h trace o = some false := by
  intro trace
  induction trace with
  | nil => intro h o _ _ _ ho; simpa [run] using ho
  | cons e rest ih =>
      intro h o hwf hs hk ho
      have hs1 : stable [e] = true := by
        cases e <;> simp_all [stable]
      have hsr : stable rest = true := by
        cases e <;> simp_all [stable]
      have hk1 : keeps o [e] = true := by
        cases e <;> simp_all [keeps]
      have hkr : keeps o rest = true := by
        cases e <;> simp_all [keeps]
      exact ih (step oracle h e) o hwf.2 hsr hkr (step_keeps oracle h e o hwf.1 hs1 hk1 ho)

/-- **A fresh object's initializer does not reach an existing object.** -/
theorem a_fresh_store_does_not_reach (oracle : Nat → Bool) (h : Heap) (o x : Nat) (b : Bool)
    (hfresh : h o = none) (hx : h x ≠ none) : step oracle h (.fresh o b) x = h x := by
  have hne : x ≠ o := by intro hxo; subst hxo; exact hx hfresh
  simp [step, hne]

/-- Every read of `o` in the trace sees a non-null member. -/
def readsSeeNonNull (oracle : Nat → Bool) (o : Nat) : Heap → List Ev → Prop
  | _, [] => True
  | h, e :: rest =>
      (match e with
        | .read x => x = o → h o = some false
        | _ => True) ∧ readsSeeNonNull oracle o (step oracle h e) rest

/-- **A use inside the scope sees non-null.** The entry obligation (the
caller proved the member non-null at the call: `h o = some false`), with the
member stable along the scope's trace (every store proved, no exit store, the
object not ended, fresh objects fresh), gives every read of `o` a non-null
member. -/
theorem a_scoped_use_sees_non_null (oracle : Nat → Bool) :
    ∀ (trace : List Ev) (h : Heap) (o : Nat),
      wf oracle h trace → stable trace = true → keeps o trace = true →
      h o = some false → readsSeeNonNull oracle o h trace := by
  intro trace
  induction trace with
  | nil => intro _ _ _ _ _ _; trivial
  | cons e rest ih =>
      intro h o hwf hs hk ho
      have hs1 : stable [e] = true := by
        cases e <;> simp_all [stable]
      have hsr : stable rest = true := by
        cases e <;> simp_all [stable]
      have hk1 : keeps o [e] = true := by
        cases e <;> simp_all [keeps]
      have hkr : keeps o rest = true := by
        cases e <;> simp_all [keeps]
      refine ⟨?_, ih (step oracle h e) o hwf.2 hsr hkr (step_keeps oracle h e o hwf.1 hs1 hk1 ho)⟩
      cases e <;> simp_all

/-- **An exit store breaks persistence**: one NULL store to an existing
object (a reset after a response, an unlink), and a later read sees NULL. A
member with an exit store therefore has no stable reading. -/
theorem an_exit_store_breaks_persistence :
    ∃ (h : Heap) (trace : List Ev), h 0 = some false ∧ wf (fun _ => true) h trace ∧
      keeps 0 trace = true ∧ ¬ readsSeeNonNull (fun _ => true) 0 h trace := by
  refine ⟨fun _ => some false, [.exit 0, .read 0], rfl, ?_, rfl, ?_⟩
  · simp [wf]
  · simp [readsSeeNonNull, step]

/-- **So does one unproved store** (the store check withdraws on it). -/
theorem an_unproved_store_breaks_persistence :
    ∃ (h : Heap) (trace : List Ev), h 0 = some false ∧ wf (fun _ => true) h trace ∧
      keeps 0 trace = true ∧ ¬ readsSeeNonNull (fun _ => true) 0 h trace := by
  refine ⟨fun _ => some false, [.store 0 false, .read 0], rfl, ?_, rfl, ?_⟩
  · simp [wf]
  · simp [readsSeeNonNull, step]

/-! ## The walk's persistence of a fact -/

inductive Fact where
  | nonNull | null | maybe
  deriving DecidableEq, Repr

/-- `null_member_scope::persist`: a fact kept across a call or a store. -/
def persist : Fact → Bool → Option Fact
  | .nonNull, true => some .nonNull
  | _, _ => none

theorem a_kept_fact_is_non_null (f g : Fact) (s : Bool) (h : persist f s = some g) :
    f = .nonNull ∧ s = true ∧ g = .nonNull := by
  cases f <;> cases s <;> simp_all [persist]

theorem nothing_is_kept_without_stability (f : Fact) : persist f false = none := by
  cases f <;> rfl

/-! ## Standing and withdrawal -/

/-- `null_member_scope::stable_standing`. -/
def stableStanding (examined failed : Nat) : String :=
  if failed > 0 then "withdrawn"
  else if examined = 0 then "declared-assumption"
  else "partly-validated"

/-- **A stable declaration is never `validated`**: whole-object writes it
cannot attribute stay a premise. -/
theorem a_stable_declaration_is_never_validated (e f : Nat) :
    stableStanding e f ≠ "validated" := by
  unfold stableStanding
  split
  · decide
  · split <;> decide

theorem a_failed_store_withdraws_it (e f : Nat) (h : f > 0) :
    stableStanding e f = "withdrawn" := by
  simp [stableStanding, h]

/-- A recorded store: proved or not, and the declarations its proof rests on. -/
structure Store where
  proved : Bool
  relies : List String

/-- The fold's judgement of a store under a withdrawn set. -/
def failsUnder (withdrawn : List String) (s : Store) : Bool :=
  !s.proved || s.relies.any (fun d => withdrawn.contains d)

/-- **Failure is monotone in the withdrawn set**, so iterating from the
refuted results reaches the least closed set. -/
theorem failure_is_monotone (w w' : List String) (s : Store)
    (hsub : ∀ d, d ∈ w → d ∈ w') (h : failsUnder w s = true) : failsUnder w' s = true := by
  unfold failsUnder at *
  simp only [Bool.or_eq_true, Bool.not_eq_true', List.any_eq_true, List.contains_iff_mem] at *
  rcases h with h | ⟨d, hd, hw⟩
  · exact Or.inl h
  · exact Or.inr ⟨d, hd, hsub d hw⟩

/-! ## Vectors bound to the Rust -/

/-- Examined stores, failed stores, standing: `STABLE_VECTORS`. -/
def stableVectors : List (Nat × Nat × String) := [
  (0, 0, "declared-assumption"),
  (4, 0, "partly-validated"),
  (4, 1, "withdrawn"),
  (0, 2, "withdrawn")
]

theorem stable_vectors_hold :
    stableVectors.all (fun (e, f, s) => stableStanding e f == s) = true := by
  decide

def factOf : String → Fact
  | "nonnull" => .nonNull
  | "null" => .null
  | _ => .maybe

def keptName : Option Fact → String
  | some _ => "kept"
  | none => "forgotten"

/-- Fact, stable member in force, outcome: `PERSIST_VECTORS`. -/
def persistVectors : List (String × Bool × String) := [
  ("nonnull", true, "kept"),
  ("nonnull", false, "forgotten"),
  ("null", true, "forgotten"),
  ("maybe", true, "forgotten"),
  ("maybe", false, "forgotten")
]

theorem persist_vectors_hold :
    persistVectors.all (fun (f, s, o) => keptName (persist (factOf f) s) == o) = true := by
  decide

end QCore.NullMemberScope

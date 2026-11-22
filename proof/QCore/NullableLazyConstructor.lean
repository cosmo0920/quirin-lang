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

import QCore.VerdictSemigroup

/-!
# A lazy constructor decides its name; a possibly-null return is refused as that (card NULLABLE-LAZY-CONSTRUCTOR-IN-LOOP)

Gateway rule 74d. The lazy-initialisation guard `if (!p) p = ctor(…);` is
decided by the existing null refinement (rules 14l, 10b); this module states
why that is sound in and out of a loop, and why the leaf the card renames
(`crates/qcore-c-gateway/src/legacy_nullable_return.rs`) keeps every verdict.

## What is proved

* **The lazy guard.** On a concrete execution, after `if (!p) p = ctor();`
  with a non-null `ctor`, `p` is non-null (`lazy_is_non_null`). On the arm
  that constructs, the fresh site is the obligation `p` denotes
  (`the_constructing_arm_owes_the_fresh_site`); on the arm that does not, the
  prior value and every prior obligation carry through unchanged
  (`the_other_arm_keeps_the_prior_value`).
* **The abstract fact.** The analyser's `non-null` fact on a set of
  executions is "every one has `p` non-null"; its join is conjunction and is
  sound (`the_join_is_sound`); the lazy step establishes it from any set
  (`the_lazy_step_establishes_it`).
* **The loop head.** A loop head's fact is the join of the entry edge and the
  back edges, so it holds only when every arriving edge re-establishes it
  (`a_loop_head_needs_every_edge`). The NULL initialiser on the entry edge
  makes it false at the head and after the loop
  (`the_zero_iteration_path_is_null`): a release after the loop outside any
  guard is refused, as it must be (`p` is NULL when the loop never ran).
* **What kills it.** A reassignment from a possibly-null value
  (`a_reassignment_kills_it`), and a refused statement that may write `p`
  (its address taken, a write by name, an escape of its slot), which stores an
  arbitrary value (`a_refused_write_kills_it`).
* **The renamed leaf.** A possibly-null return under a non-null result mode
  is refused whichever leaf names it, and the refusing step marks the site
  exactly as before, so it claims no leak (`a_refused_return_claims_nothing`)
  and the function is `Unknown` or `Unsafe`, never `Safe` or `Pending`
  (`the_refusal_is_never_below_unknown`); the old pair of leaves and the new
  single leaf fold to the same verdict beside any others
  (`the_renaming_keeps_the_verdict`). A nullable result mode accepts the
  return (`a_nullable_result_mode_accepts_it`): that is the validated summary's
  route, which the card does not change.

## What this is not

A proof that the Rust computes the states it is said to. That
`refine_pointer_null`, `create_resource` and the join implement this fact, and
that `legacy_nullable_return::refuse` fires exactly on a resolved,
possibly-null binding under a non-null result mode, are review premises
(trust boundary 28), pinned by `crates/qcore-c-gateway/tests/nullable_lazy_constructor.rs`.
It is not a statement about librdkafka.
-/

namespace QCore.NullableLazyConstructor

open QCore.VerdictSemigroup (V fold)

/-! ## The lazy guard on one execution -/

/-- One execution's facts about the name `p`: its value (`none` is NULL) and
the live obligations. -/
structure St where
  p : Option Nat
  live : List Nat
  deriving DecidableEq, Repr

/-- `if (!p) p = ctor();` with a non-null constructor returning `fresh`. -/
def lazy (fresh : Nat) (s : St) : St :=
  match s.p with
  | none => ⟨some fresh, fresh :: s.live⟩
  | some _ => s

theorem lazy_is_non_null (fresh : Nat) (s : St) : (lazy fresh s).p.isSome = true := by
  unfold lazy
  cases h : s.p with
  | none => rfl
  | some _ => simp [h]

theorem the_constructing_arm_owes_the_fresh_site (fresh : Nat) (s : St) (h : s.p = none) :
    (lazy fresh s).p = some fresh ∧ fresh ∈ (lazy fresh s).live := by
  simp [lazy, h]

theorem the_other_arm_keeps_the_prior_value (fresh x : Nat) (s : St) (h : s.p = some x) :
    lazy fresh s = s := by
  simp [lazy, h]

/-! ## The abstract fact over a set of executions -/

/-- The analyser's fact: every execution reaching here has `p` non-null. -/
def nonNull (xs : List St) : Bool := xs.all (fun s => s.p.isSome)

/-- Soundness of the fact: each execution it covers has `p` non-null. -/
theorem the_fact_is_sound (xs : List St) (h : nonNull xs = true) (s : St) (m : s ∈ xs) :
    s.p.isSome = true := by
  simp [nonNull, List.all_eq_true] at h
  simpa using h s m

/-- Joining two edges collects their executions; the fact joins by
conjunction. -/
theorem the_join_is_sound (xs ys : List St) :
    nonNull (xs ++ ys) = (nonNull xs && nonNull ys) := by
  simp [nonNull, List.all_append]

theorem the_lazy_step_establishes_it (fresh : Nat) (xs : List St) :
    nonNull (xs.map (lazy fresh)) = true := by
  simp [nonNull, List.all_map, List.all_eq_true, lazy_is_non_null]

/-! ## The loop head -/

/-- A loop head's executions are its entry edge's and its back edges'. -/
def head (entry : List St) (backs : List (List St)) : List St :=
  entry ++ backs.flatten

theorem the_back_edges_join (backs : List (List St)) :
    nonNull backs.flatten = backs.all nonNull := by
  induction backs with
  | nil => simp [nonNull]
  | cons b bs ih => simp only [List.flatten_cons, the_join_is_sound, ih, List.all_cons]

theorem a_loop_head_needs_every_edge (entry : List St) (backs : List (List St)) :
    nonNull (head entry backs) = (nonNull entry && backs.all nonNull) := by
  rw [head, the_join_is_sound, the_back_edges_join]

/-- `void *p = NULL;` on the entry edge: the fact is false at the head
whatever the back edges establish, so it is false on the loop's exit edge (the
zero-iteration path). -/
theorem the_zero_iteration_path_is_null (live : List Nat) (backs : List (List St)) :
    nonNull (head [⟨none, live⟩] backs) = false := by
  rw [a_loop_head_needs_every_edge]
  simp [nonNull]

/-- Inside the loop, after the lazy step, the fact holds again on every back
edge, whatever the head carried. -/
theorem every_back_edge_after_the_guard_has_it (fresh : Nat) (xs : List St) :
    nonNull (xs.map (lazy fresh)) = true :=
  the_lazy_step_establishes_it fresh xs

/-! ## What kills it -/

/-- `p = v;` -/
def assign (v : Option Nat) (s : St) : St := { s with p := v }

theorem a_reassignment_kills_it (live : List Nat) :
    nonNull [assign none ⟨some 0, live⟩] = false := by
  simp [nonNull, assign]

/-- A refused statement that may write `p` (its slot address taken, a write
by name, an escape of the slot) stores any value, NULL among them. -/
theorem a_refused_write_kills_it (s : St) :
    ∃ v : Option Nat, nonNull [assign v s] = false :=
  ⟨none, by simp [nonNull, assign]⟩

/-! ## The renamed leaf: a possibly-null return under a non-null result mode -/

/-- Whether a return of a binding is accepted by the result mode. -/
def returnAccepted (resultNullable mayBeNull : Bool) : Bool := resultNullable || !mayBeNull

theorem a_non_null_mode_refuses_it : returnAccepted false true = false := rfl

theorem a_nullable_result_mode_accepts_it (mayBeNull : Bool) :
    returnAccepted true mayBeNull = true := by
  simp [returnAccepted]

/-- One site's facts at an exit: live, and marked by a refused effect. -/
structure Site where
  live : Bool
  unresolved : Bool
  deriving DecidableEq, Repr

def claimed (s : Site) : Bool := s.live && !s.unresolved

/-- The refusing step marks the returned site, as `resource_action`'s
refusing branch does; the renamed leaf leaves this marking unchanged. -/
def refuseReturn (s : Site) : Site := { s with unresolved := true }

theorem a_refused_return_claims_nothing (s : Site) : claimed (refuseReturn s) = false := by
  simp [claimed, refuseReturn]

/-- A refusal's leaf is `Unknown`, so the function is `Unknown` or `Unsafe`. -/
theorem the_refusal_is_never_below_unknown (xs : List V) (leaf : V.unknown ∈ xs) :
    fold xs = .unknown ∨ fold xs = .provisionalUnsafe := by
  have h := QCore.VerdictSemigroup.every_link_is_below_the_fold leaf
  cases e : fold xs <;> simp_all [QCore.VerdictSemigroup.V.rank]

/-- The two old leaves (`unresolved resource identity`, `used by a non-null
contract`) and the one new leaf fold to the same verdict beside any others. -/
theorem the_renaming_keeps_the_verdict (xs : List V) :
    fold (.unknown :: .unknown :: xs) = fold (.unknown :: xs) := by
  simp only [fold]
  rw [← QCore.VerdictSemigroup.join_assoc, QCore.VerdictSemigroup.join_idem]

end QCore.NullableLazyConstructor

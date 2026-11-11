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
# Checked specifications (card W45-FB-BACKLOG-CUSTODY, contract spec §C06.3)

A declaration marked `requires resource_body_checked;` is a specification of a
body the audit analyses. The gateway (`crates/qcore-c-gateway/src/legacy_checked_spec.rs`)
keeps it in a unit's environment only while the unit defines the body, outside
every recursive component, and the body **establishes** it under the
environment that keeps exactly the specifications still standing. It settles
by descent: each round withdraws every specification whose body does not
establish it under the current set, until a round withdraws nothing.

The model abstracts what "establishes" means: `est S s` is the gateway's
check (analyse `s`'s body under its entry custody, with the specifications in
`S` in force; fully modelled, no diagnostic, every exit matching).

## What is proved

* **Everything left in force is established under what is left in force**
  (`settle_sound`): the settled set is a fixed point of the check, so no
  specification is applied on the strength of one that was withdrawn.
* **Settling only withdraws** (`settle_subset`), and a specification without a
  body, or in a recursive component, is never in force (`never_without_a_body`).
* **Descent terminates within the bound the Rust uses**
  (`settle_is_a_fixed_point_within_its_bound`): `S.length + 1` rounds.
* **A specification is never trusted** (`a_row_is_safe_only_if_established`):
  the defining function's row is `Safe` only when its specification is in
  force, and then only because its body established it.

## What this is not

That the gateway's check is the W4 gate, and that call sites apply exactly the
environment the settled set names, are Rust boundaries pinned by
`crates/qcore-c-gateway/tests/intrusive_list_iteration.rs` (the checked
specification controls).
-/

namespace QCore.CheckedSpecification

/-- One round: keep the specifications whose bodies establish them under the
current set. -/
def round (est : List Nat → Nat → Bool) (S : List Nat) : List Nat :=
  S.filter (est S)

/-- Descent with fuel. Out of fuel, everything is withdrawn (the refusing
direction; unreachable within the bound, `settle_is_a_fixed_point_within_its_bound`). -/
def settle (est : List Nat → Nat → Bool) : Nat → List Nat → List Nat
  | 0, _ => []
  | n + 1, S => if (round est S).length = S.length then S else settle est n (round est S)

theorem filter_length_le (p : Nat → Bool) : ∀ S : List Nat, (S.filter p).length ≤ S.length
  | [] => by simp
  | x :: xs => by
    have := filter_length_le p xs
    by_cases h : p x = true
    · simp [List.filter, h]; omega
    · simp [List.filter, h]; omega

theorem all_of_filter_length (p : Nat → Bool) :
    ∀ S : List Nat, (S.filter p).length = S.length → ∀ x ∈ S, p x = true
  | [], _, x, hx => by simp at hx
  | y :: ys, h, x, hx => by
    by_cases hy : p y = true
    · simp only [List.filter, hy, List.length_cons, Nat.add_right_cancel_iff] at h
      rcases List.mem_cons.mp hx with e | e
      · exact e ▸ hy
      · exact all_of_filter_length p ys h x e
    · simp [List.filter, hy] at h
      have := filter_length_le p ys
      omega

theorem mem_of_mem_round {est : List Nat → Nat → Bool} {S : List Nat} {s : Nat}
    (h : s ∈ round est S) : s ∈ S :=
  (List.mem_filter.mp h).1

/-- Settling only withdraws. -/
theorem settle_subset (est : List Nat → Nat → Bool) :
    ∀ n S, ∀ s ∈ settle est n S, s ∈ S
  | 0, _, s, h => by simp [settle] at h
  | n + 1, S, s, h => by
    unfold settle at h
    split at h
    · exact h
    · exact mem_of_mem_round (settle_subset est n (round est S) s h)

/-- Everything in force is established under what is in force. -/
theorem settle_sound (est : List Nat → Nat → Bool) :
    ∀ n S, ∀ s ∈ settle est n S, est (settle est n S) s = true
  | 0, _, s, h => by simp [settle] at h
  | n + 1, S, s, h => by
    unfold settle at h ⊢
    split at h
    · rename_i hlen
      simp only [hlen, ite_true]
      exact all_of_filter_length (est S) S hlen s h
    · rename_i hlen
      simp only [hlen, ite_false]
      exact settle_sound est n (round est S) s h

/-- A specification without a body in the unit, or in a recursive component,
is withdrawn before the first round and so is never in force. -/
theorem never_without_a_body (est : List Nat → Nat → Bool) (candidates : List Nat)
    (eligible : Nat → Bool) (n : Nat) (s : Nat)
    (h : s ∈ settle est n (candidates.filter eligible)) : eligible s = true :=
  (List.mem_filter.mp (settle_subset est n _ s h)).2

theorem round_length_lt {est : List Nat → Nat → Bool} {S : List Nat}
    (h : (round est S).length ≠ S.length) : (round est S).length < S.length := by
  have := filter_length_le (est S) S
  unfold round at h ⊢
  omega

/-- Within `S.length + 1` rounds the descent reaches a set the next round
keeps whole. -/
theorem settle_is_a_fixed_point_within_its_bound (est : List Nat → Nat → Bool) :
    ∀ n S, S.length < n →
      (round est (settle est n S)).length = (settle est n S).length
  | 0, _, h => by simp at h
  | n + 1, S, h => by
    unfold settle
    split
    · assumption
    · rename_i hlen
      have hlt := round_length_lt hlen
      exact settle_is_a_fixed_point_within_its_bound est n (round est S) (by omega)

/-- The defining function's row: `Safe` only when its specification is in
force and its body is safe under it; otherwise not `Safe` (the gateway adds
`declared specification of … is not established`). -/
def rowSafe (inForce : List Nat) (bodySafe : Nat → Bool) (s : Nat) : Bool :=
  decide (s ∈ inForce) && bodySafe s

/-- A specification is never trusted: a `Safe` row has an established
specification. -/
theorem a_row_is_safe_only_if_established (est : List Nat → Nat → Bool)
    (bodySafe : Nat → Bool) (n : Nat) (S : List Nat) (s : Nat)
    (h : rowSafe (settle est n S) bodySafe s = true) :
    est (settle est n S) s = true := by
  unfold rowSafe at h
  simp only [Bool.and_eq_true, decide_eq_true_eq] at h
  exact settle_sound est n S s h.1

/-- Non-vacuity: a set whose members all fail is withdrawn entirely, and a
set whose members all hold stays whole. -/
theorem a_failing_specification_is_withdrawn :
    settle (fun _ _ => false) 2 [7] = [] := by
  decide

theorem an_established_specification_stays :
    settle (fun _ _ => true) 2 [7] = [7] := by
  decide

end QCore.CheckedSpecification

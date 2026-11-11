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

import Init.Data.List.Lemmas

/-!
The exact read-only child/next frame computes an indexed non-owning result.
The source reader and Rust/Lean correspondence are maintained boundary-28
premises. A coherent finite concrete/list representation is a separate named
premise at callers. This model does not establish that premise, create an
identity, transfer custody, or turn borrowed provenance into membership.
-/
namespace QCore.ListLookup

def lookup : List Nat → Nat → Option Nat
  | [], _ => none
  | x :: _, 0 => some x
  | _ :: xs, n + 1 => lookup xs n

theorem empty_is_null (n : Nat) : lookup [] n = none := rfl

theorem head_is_the_same_identity (x : Nat) (xs : List Nat) :
    lookup (x :: xs) 0 = some x := rfl

theorem nonnull_is_a_member (xs : List Nat) (n item : Nat)
    (h : lookup xs n = some item) : item ∈ xs := by
  induction xs generalizing n with
  | nil => simp [lookup] at h
  | cons x xs ih =>
    cases n with
    | zero => simp [lookup] at h; simp [h]
    | succ n => exact List.mem_cons_of_mem x (ih n h)

theorem out_of_range_is_null (xs : List Nat) (n : Nat)
    (h : xs.length ≤ n) : lookup xs n = none := by
  induction xs generalizing n with
  | nil => rfl
  | cons x xs ih =>
    cases n with
    | zero => simp at h
    | succ n => exact ih n (Nat.le_of_succ_le_succ h)

-- Executable correspondence controls: empty, head, middle, tail, missing.
def vectors : List (List Nat × Nat × Option Nat) := [
  ([], 0, none),
  ([10], 0, some 10),
  ([10], 1, none),
  ([10, 20, 30], 0, some 10),
  ([10, 20, 30], 1, some 20),
  ([10, 20, 30], 2, some 30),
  ([10, 20, 30], 3, none)
]
theorem vectors_hold : vectors.all (fun (xs, n, r) => lookup xs n == r) = true := by decide

/-- Unknown links and checked NULL are distinct; each visited item must have
    independently checked membership/lifetime and no repeated identity. -/
def walk (n : Nat) (head : Option (Option Nat))
    (next : Nat → Option (Option Nat)) (valid : Nat → Bool)
    (seen : List Nat := []) : Option (Option Nat) :=
  match head with
  | none => none
  | some none => some none
  | some (some item) =>
    if valid item && !seen.contains item then
      match n with
      | 0 => some (some item)
      | k + 1 => walk k (next item) next valid (item :: seen)
    else none

theorem refined_nonnull_needs_valid_member (n : Nat)
    (head : Option (Option Nat)) (next : Nat → Option (Option Nat))
    (valid : Nat → Bool) (seen : List Nat) (item : Nat)
    (h : walk n head next valid seen = some (some item)) : valid item = true := by
  induction n generalizing head seen with
  | zero =>
    cases head with
    | none => simp [walk] at h
    | some head =>
      cases head with
      | none => simp [walk] at h
      | some x =>
        simp [walk] at h
        exact h.2 ▸ h.1.1
  | succ n ih =>
    cases head with
    | none => simp [walk] at h
    | some head =>
      cases head with
      | none => simp [walk] at h
      | some x =>
        simp [walk] at h
        exact ih (next x) (x :: seen) h.2

theorem unknown_head_never_creates_identity (n : Nat) (next) (valid) (seen) :
    walk n none next valid seen = none := by cases n <;> rfl

theorem explicit_null_stays_null (n : Nat) (next) (valid) (seen) :
    walk n (some none) next valid seen = some none := by cases n <;> rfl

/-- The concrete/list relation states link values, live membership and no
    repeated object. It states no conclusion about any indexed lookup. -/
def Represents : List Nat → Option (Option Nat) →
    (Nat → Option (Option Nat)) → (Nat → Bool) → List Nat → Prop
  | [], head, _, _, _ => head = some none
  | x :: xs, head, next, valid, seen =>
    head = some (some x) ∧ valid x = true ∧ x ∉ seen ∧
      Represents xs (next x) next valid (x :: seen)

theorem coherent_prefix_selects_the_same_list_identity (xs : List Nat) (n : Nat)
    (head : Option (Option Nat)) (next : Nat → Option (Option Nat))
    (valid : Nat → Bool) (seen : List Nat)
    (rep : Represents xs head next valid seen) :
    walk n head next valid seen = some (lookup xs n) := by
  induction xs generalizing n head seen with
  | nil =>
    simp only [Represents] at rep
    subst head
    cases n <;> simp [walk, lookup]
  | cons x xs ih =>
    rcases rep with ⟨same, live, fresh, tail⟩
    subst head
    cases n with
    | zero => simp [walk, lookup, live, fresh]
    | succ n => simpa [walk, lookup, live, fresh] using ih n (next x) (x :: seen) tail


/-- The read view concerns only name-only inference sites, never a tracked
    object. Source guard extraction and alias checks remain separate. -/
structure ReadViewFacts where
  virtualSite : Bool
  trackedStatus : Bool
  unresolved : Bool
  conflict : Bool
  cursorRelation : Bool
  deriving DecidableEq, Repr

def allowReadView (f : ReadViewFacts) : Bool :=
  f.virtualSite && !f.trackedStatus && !f.unresolved && !f.conflict && f.cursorRelation

theorem a_view_never_hides_a_tracked_site (f : ReadViewFacts)
    (h : allowReadView f = true) : f.trackedStatus = false := by
  rcases f with ⟨v, t, u, c, r⟩
  cases v <;> cases t <;> cases u <;> cases c <;> cases r <;> simp_all [allowReadView]

def readObligation (f : ReadViewFacts) (original borrowed : Bool) : Bool :=
  if allowReadView f then borrowed else original

theorem a_real_site_keeps_its_original_obligation (f : ReadViewFacts)
    (tracked : f.trackedStatus = true) (original borrowed : Bool) :
    readObligation f original borrowed = original := by
  have refused : allowReadView f = false := by
    cases h : allowReadView f
    · rfl
    · have absent := a_view_never_hides_a_tracked_site f h
      simp [tracked] at absent
  simp [readObligation, refused]

def guardedRead (aliasesClear : Bool) (f : ReadViewFacts) (original borrowed : Bool) : Bool :=
  aliasesClear && readObligation f original borrowed

theorem a_released_peer_cannot_be_erased (f : ReadViewFacts) (original borrowed : Bool) :
    guardedRead false f original borrowed = false := by simp [guardedRead]

def readViewVectors : List (ReadViewFacts × Bool) := [
  (⟨true, false, false, false, true⟩, true),
  (⟨false, false, false, false, true⟩, false),
  (⟨true, true, false, false, true⟩, false),
  (⟨true, false, true, false, true⟩, false),
  (⟨true, false, false, true, true⟩, false),
  (⟨true, false, false, false, false⟩, false),
  (⟨false, true, true, true, false⟩, false)
]
theorem read_view_vectors_hold :
    readViewVectors.all (fun (f, result) => allowReadView f == result) = true := by decide


def nextOf : List Nat → Nat → Option (Option Nat)
  | [], _ => none
  | x :: xs, item => if x == item then some xs.head? else nextOf xs item

theorem physical_vectors_match_indexed_lookup :
    vectors.all (fun (xs, n, r) =>
      walk n (some xs.head?) (nextOf xs) xs.contains [] == some r) = true := by decide

/-- Rust's bounded source extractor supplies a complete canonical actual tag
and its equality with the checked callee tag. No C type system is assumed here. -/
def recordEligible (known same : Bool) : Bool := known && same

theorem a_known_matching_record_is_eligible : recordEligible true true = true := rfl

theorem an_unknown_record_refuses (same : Bool) : recordEligible false same = false := by
  simp [recordEligible]

theorem a_mismatched_record_refuses (known : Bool) : recordEligible known false = false := by
  simp [recordEligible]

theorem eligibility_requires_known_matching_tags (known same : Bool)
    (h : recordEligible known same = true) : known = true ∧ same = true := by
  cases known <;> cases same <;> simp_all [recordEligible]

def recordVectors : List (Bool × Bool × Bool) := [
  (false, false, false),
  (false, true, false),
  (true, false, false),
  (true, true, true)
]
theorem record_vectors_hold :
    recordVectors.all (fun (known, same, result) => recordEligible known same == result) = true := by decide

end QCore.ListLookup

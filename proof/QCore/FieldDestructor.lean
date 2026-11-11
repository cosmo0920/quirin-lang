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

namespace QCore.FieldDestructor

/-! Abstract field-custody semantics. Pointer provenance, distinct field
allocations, complete C import and libc binding remain supplied premises. -/

abbrev Live := Nat → Bool

def release (live : Live) (identity : Nat) : Option Live :=
  if live identity then some (fun other => if other = identity then false else live other)
  else none

def freeNullable (live : Live) : Option Nat → Option Live
  | none => some live
  | some identity => release live identity

theorem null_release_preserves_custody (live : Live) :
    freeNullable live none = some live := rfl

theorem accepted_release_was_live {live after : Live} {identity : Nat}
    (accepted : release live identity = some after) : live identity = true := by
  simp only [release] at accepted
  split at accepted
  · assumption
  · contradiction

theorem accepted_release_discharges_identity {live after : Live} {identity : Nat}
    (accepted : release live identity = some after) : after identity = false := by
  simp only [release] at accepted
  split at accepted
  · cases accepted
    simp
  · contradiction

theorem accepted_release_preserves_other {live after : Live} {identity other : Nat}
    (accepted : release live identity = some after) (different : other ≠ identity) :
    after other = live other := by
  simp only [release] at accepted
  split at accepted
  · cases accepted
    simp [different]
  · contradiction

theorem duplicate_release_rejected {live after : Live} {identity : Nat}
    (accepted : release live identity = some after) : release after identity = none := by
  simp [release, accepted_release_discharges_identity accepted]

def releaseParent (live : Live) (parent : Nat) (children : List Nat) : Option Live :=
  if children.all (fun child => !live child) then release live parent else none

theorem parent_release_requires_discharged_children {live after : Live}
    {parent child : Nat} {children : List Nat}
    (accepted : releaseParent live parent children = some after)
    (member : child ∈ children) : live child = false := by
  unfold releaseParent at accepted
  split at accepted
  next all =>
    have cleared := List.all_eq_true.mp all child member
    simpa using cleared
  next => contradiction

def complete (live : Live) (owned : List Nat) : Bool :=
  owned.all (fun identity => !live identity)

theorem accepted_teardown_has_no_owned_field {live : Live} {owned : List Nat}
    (accepted : complete live owned = true) {identity : Nat} (member : identity ∈ owned) :
    live identity = false := by
  have cleared := List.all_eq_true.mp accepted identity member
  simpa using cleared

end QCore.FieldDestructor

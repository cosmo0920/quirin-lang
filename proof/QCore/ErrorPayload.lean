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

/-! Tagged error payload custody (card T06).

One child, one explicitly tagged error, and exactly one owned Buffer inside it.
A completion names precisely the identities the child still holds; between
completion and the creator's join the payload belongs to nobody; the join hands
it to the creator still inside the bound error, and only an explicit take turns
it into an ordinary owned value. Every local the child acquired and did not carry
must already be released before the completion is admitted.

The child's inventory is enumerated over an explicit finite universe of
identities rather than assumed, so a local the child forgot is visible to the
completion rule instead of being assumed away.

These proofs concern the custody discipline of that one transfer. They are not
about source import, native scheduling, the C memory model, several payloads,
recursive owned graphs, or the Rust checker and this model agreeing. -/
namespace QCore.ErrorPayload

/-- What a child's completion reports. A tagged error is the only outcome that
carries a resource on a failure edge. -/
inductive Outcome where
  | scalar (value : Nat)
  | ownedResult (resource : Nat)
  | error (tag : Nat)
  | errorWithBuffer (tag resource : Nat)
  deriving DecidableEq, Repr

/-- The single identity an outcome carries, if any. Exactly one, never two. -/
def carried : Outcome → Option Nat
  | .ownedResult resource => some resource
  | .errorWithBuffer _ resource => some resource
  | _ => none

/-- Where a resource's authority currently sits. `capsule` is the window between
a child's completion and its creator's join; `inError` is the window between the
join and the explicit take. -/
inductive Authority where
  | owned (task : Nat)
  | capsule (child : Nat)
  | inError (task : Nat)
  | dropped
  deriving DecidableEq, Repr

abbrev Custody := Nat → Option Authority

/-- Release, mutation and move all need this. A payload still inside a bound
error does not have it: `inError` is not `owned`. -/
def canUseExclusively (custody : Custody) (task resource : Nat) : Bool :=
  decide (custody resource = some (.owned task))

private def replace (custody : Custody) (resource : Nat) (authority : Authority) : Custody :=
  fun r => if r = resource then some authority else custody r

private theorem replace_here (custody : Custody) (resource : Nat) (authority : Authority) :
    replace custody resource authority resource = some authority := by
  simp [replace]

private theorem replace_elsewhere (custody : Custody) (resource other : Nat)
    (authority : Authority) (distinct : other ≠ resource) :
    replace custody resource authority other = custody other := by
  simp [replace, distinct]

/-- Everything the child still owns, enumerated over the identities in play. -/
def ownedBy (custody : Custody) (child : Nat) (pool : List Nat) : List Nat :=
  pool.filter (fun r => canUseExclusively custody child r)

/-- The child's completion. It is admitted only when the child's whole remaining
inventory is exactly what the outcome names, so it can neither hide a local it
forgot nor claim an identity it does not hold. -/
def complete (custody : Custody) (child : Nat) (outcome : Outcome) (pool : List Nat) :
    Option Custody :=
  if ownedBy custody child pool = (carried outcome).toList then
    some (match carried outcome with
          | none => custody
          | some resource => replace custody resource (.capsule child))
  else none

/-- The creator's join. It moves the carried identity out of the capsule; a
tagged error's payload arrives still inside the bound error, and an owned result
arrives as an ordinary owned value. -/
def join (custody : Custody) (creator child : Nat) (outcome : Outcome) : Option Custody :=
  match carried outcome with
  | none => some custody
  | some resource =>
    if custody resource = some (.capsule child) then
      some (replace custody resource
        (match outcome with
         | .errorWithBuffer _ _ => .inError creator
         | _ => .owned creator))
    else none

/-- The explicit take. The only transition out of `inError`, and it happens at
most once because the state it needs is gone afterwards. -/
def take (custody : Custody) (resource task : Nat) : Option Custody :=
  if custody resource = some (.inError task) then
    some (replace custody resource (.owned task))
  else none

/-- Release needs exclusive authority, exactly like mutation. -/
def release (custody : Custody) (resource task : Nat) : Option Custody :=
  if canUseExclusively custody task resource then
    some (replace custody resource .dropped)
  else none

/-- Re-raising a bound error carries its tag onward and nothing else, so it is
admitted only once the payload has left the error. -/
def reraise (custody : Custody) (resource task : Nat) : Bool :=
  decide (custody resource ≠ some (.inError task))

private theorem take_result {custody after : Custody} {resource task : Nat}
    (checked : take custody resource task = some after) :
    custody resource = some (.inError task) ∧
      after = replace custody resource (.owned task) := by
  unfold take at checked
  split at checked
  · exact ⟨by assumption, by cases checked; rfl⟩
  · contradiction

/-- An outcome names at most one identity. There is no second payload to lose,
and no shape in which two travel together. -/
theorem an_outcome_carries_at_most_one_identity (outcome : Outcome) :
    (carried outcome).toList.length ≤ 1 := by
  cases outcome <;> simp [carried]

/-- A scalar result and a payload-free error carry nothing at all. -/
theorem payload_free_outcomes_carry_nothing (value tag : Nat) :
    carried (.scalar value) = none ∧ carried (.error tag) = none := by
  simp [carried]

/-- T06 acceptance 2. A completion is admitted only when the child's whole
remaining inventory is exactly what the outcome names. -/
theorem completion_holds_exactly_what_it_names {custody after : Custody}
    {child : Nat} {outcome : Outcome} {pool : List Nat}
    (checked : complete custody child outcome pool = some after) :
    ownedBy custody child pool = (carried outcome).toList := by
  unfold complete at checked
  split at checked
  · assumption
  · contradiction

/-- The same statement in its refusing direction: a child that acquired a local
and did not release it cannot complete with a payload, because its inventory is
longer than the one identity the outcome names. Distinctness of the two pool
entries is not needed: any second identity the child still owns refuses the
completion. -/
theorem an_unreleased_local_refuses_the_completion (custody : Custody)
    {child tag payload extra : Nat}
    (holds_payload : custody payload = some (.owned child))
    (holds_extra : custody extra = some (.owned child)) :
    complete custody child (.errorWithBuffer tag payload) [payload, extra] = none := by
  have inventory : ownedBy custody child [payload, extra] = [payload, extra] := by
    simp [ownedBy, canUseExclusively, holds_payload, holds_extra]
  simp [complete, inventory, carried]

/-- And the other refusing direction: a completion cannot name an identity the
child does not hold. -/
theorem a_foreign_payload_refuses_the_completion (custody : Custody)
    {child tag payload : Nat} (not_held : custody payload ≠ some (.owned child)) :
    complete custody child (.errorWithBuffer tag payload) [payload] = none := by
  have inventory : ownedBy custody child [payload] = [] := by
    simp [ownedBy, canUseExclusively, not_held]
  simp [complete, inventory, carried]

/-- T06 acceptance 1. Completion is not custody return: while the payload sits
in the capsule, no task at all — creator or child — has exclusive authority, and
nobody can take it out of an error it is not in yet. -/
theorem a_completed_payload_belongs_to_nobody (custody : Custody)
    {resource child : Nat} (parked : custody resource = some (.capsule child)) (task : Nat) :
    canUseExclusively custody task resource = false ∧
      release custody resource task = none ∧
      take custody resource task = none := by
  refine ⟨by simp [canUseExclusively, parked], by simp [release, canUseExclusively, parked],
    by simp [take, parked]⟩

/-- A successful completion parks exactly the carried identity in the capsule. -/
theorem completion_parks_exactly_the_carried_identity {custody after : Custody}
    {child tag resource : Nat} {pool : List Nat}
    (checked : complete custody child (.errorWithBuffer tag resource) pool = some after) :
    after resource = some (.capsule child) ∧
      ∀ other, other ≠ resource → after other = custody other := by
  unfold complete at checked
  split at checked
  · cases checked
    exact ⟨by simp [carried, replace_here],
      fun other distinct => by simp [carried]; exact replace_elsewhere _ _ _ _ distinct⟩
  · contradiction

/-- T06 acceptance 1. The join transfers the payload, and it delivers it into
the bound error rather than as an ordinary owned value. -/
theorem the_join_delivers_a_tagged_payload_into_the_bound_error
    {custody after : Custody} {creator child tag resource : Nat}
    (checked : join custody creator child (.errorWithBuffer tag resource) = some after) :
    after resource = some (.inError creator) := by
  simp only [join, carried] at checked
  split at checked
  · cases checked
    simp [replace_here]
  · contradiction

/-- A normal owned result arrives as an ordinary owned value; the two edges are
distinguished by the outcome, not by the caller. -/
theorem the_join_delivers_an_owned_result_directly {custody after : Custody}
    {creator child resource : Nat}
    (checked : join custody creator child (.ownedResult resource) = some after) :
    after resource = some (.owned creator) := by
  simp only [join, carried] at checked
  split at checked
  · cases checked
    simp [replace_here]
  · contradiction

/-- The join touches exactly the carried identity and nothing else. -/
theorem the_join_transfers_only_the_carried_identity {custody after : Custody}
    {creator child tag resource : Nat}
    (checked : join custody creator child (.errorWithBuffer tag resource) = some after)
    (other : Nat) (distinct : other ≠ resource) : after other = custody other := by
  simp only [join, carried] at checked
  split at checked
  · cases checked
    exact replace_elsewhere _ _ _ _ distinct
  · contradiction

/-- T06 acceptance 3. A second join transfers nothing: the capsule state the
join consumes is gone, so the payload cannot be collected twice. -/
theorem a_second_join_cannot_transfer_the_payload_again {custody after : Custody}
    {creator child tag resource : Nat}
    (checked : join custody creator child (.errorWithBuffer tag resource) = some after)
    (task : Nat) : join after task child (.errorWithBuffer tag resource) = none := by
  have here := the_join_delivers_a_tagged_payload_into_the_bound_error checked
  simp [join, carried, here]

/-- T06 acceptance 1. A payload still inside a bound error is not a value its
holder may release: forgetting the take cannot become a silent free. -/
theorem a_payload_inside_a_bound_error_cannot_be_released (custody : Custody)
    {resource holder : Nat} (bound : custody resource = some (.inError holder))
    (task : Nat) :
    canUseExclusively custody task resource = false ∧
      release custody resource task = none := by
  refine ⟨by simp [canUseExclusively, bound], by simp [release, canUseExclusively, bound]⟩

/-- Nor may the error be re-raised while it still holds the payload: a tag alone
cannot carry an identity onward. -/
theorem a_loaded_error_cannot_be_re_raised (custody : Custody)
    {resource holder : Nat} (bound : custody resource = some (.inError holder)) :
    reraise custody resource holder = false := by
  simp [reraise, bound]

/-- Only the task the join delivered to may take the payload. -/
theorem only_the_holder_can_take_the_payload (custody : Custody)
    {resource holder : Nat} (bound : custody resource = some (.inError holder))
    (task : Nat) (distinct : task ≠ holder) : take custody resource task = none := by
  simp [take, bound]
  omega

/-- The take yields exclusive authority to the holder, and to nobody else. -/
theorem taking_a_payload_yields_exclusive_authority {custody after : Custody}
    {resource holder : Nat} (checked : take custody resource holder = some after)
    (task : Nat) : canUseExclusively after task resource = true ↔ task = holder := by
  rw [(take_result checked).2]
  simp [canUseExclusively, replace_here]
  exact ⟨fun h => h.symm, fun h => h.symm⟩

/-- T06 acceptance 3. The payload cannot be taken twice: the state the take
consumes no longer exists, so one identity can never become two owned values. -/
theorem a_payload_cannot_be_taken_twice {custody after : Custody} {resource holder : Nat}
    (checked : take custody resource holder = some after) (task : Nat) :
    take after resource task = none := by
  rw [(take_result checked).2]
  simp [take, replace_here]

/-- Once taken, the error no longer holds a payload, so re-raising it is
admitted. This is the intended way out of the previous refusal. -/
theorem a_taken_error_may_be_re_raised {custody after : Custody} {resource holder : Nat}
    (checked : take custody resource holder = some after) :
    reraise after resource holder = true := by
  rw [(take_result checked).2]
  simp [reraise, replace_here]

/-- The take touches exactly the payload and nothing else. -/
theorem taking_preserves_every_other_identity {custody after : Custody}
    {resource holder : Nat} (checked : take custody resource holder = some after)
    (other : Nat) (distinct : other ≠ resource) : after other = custody other := by
  rw [(take_result checked).2, replace_elsewhere _ _ _ _ distinct]

/-- The whole admitted lifetime, composed: a parent's Buffer moves into a child,
comes back inside a tagged error, is taken and is released. It ends released,
with every other identity untouched throughout. -/
theorem the_admitted_payload_round_trip_ends_released
    {start completed joined taken released : Custody}
    {creator child tag resource : Nat} {pool : List Nat}
    (finished : complete start child (.errorWithBuffer tag resource) pool = some completed)
    (collected : join completed creator child (.errorWithBuffer tag resource) = some joined)
    (unwrapped : take joined resource creator = some taken)
    (freed : release taken resource creator = some released) :
    released resource = some .dropped ∧
      ∀ other, other ≠ resource → released other = start other := by
  refine ⟨?_, ?_⟩
  · unfold release at freed
    split at freed
    · cases freed; exact replace_here _ _ _
    · contradiction
  · intro other distinct
    have step₃ : released other = taken other := by
      unfold release at freed
      split at freed
      · cases freed; exact replace_elsewhere _ _ _ _ distinct
      · contradiction
    rw [step₃, taking_preserves_every_other_identity unwrapped other distinct,
      the_join_transfers_only_the_carried_identity collected other distinct,
      (completion_parks_exactly_the_carried_identity finished).2 other distinct]

/-- Decidable scenarios mirroring the Rust and source fixtures. Resource 1 is
the parent's Buffer, moved into child 1; the creator is task 0. -/
private def moved : Custody := fun r => if r = 1 then some (.owned 1) else none

example :
    (do
      let completed ← complete moved 1 (.errorWithBuffer 7 1) [1, 2]
      pure (completed 1, canUseExclusively completed 0 1, take completed 1 0 |>.isSome)) =
      some (some (.capsule 1), false, false) := by decide +kernel

example :
    (do
      let completed ← complete moved 1 (.errorWithBuffer 7 1) [1, 2]
      let joined ← join completed 0 1 (.errorWithBuffer 7 1)
      pure (joined 1, canUseExclusively joined 0 1, release joined 1 0 |>.isSome,
        reraise joined 1 0)) =
      some (some (.inError 0), false, false, false) := by decide +kernel

example :
    (do
      let completed ← complete moved 1 (.errorWithBuffer 7 1) [1, 2]
      let joined ← join completed 0 1 (.errorWithBuffer 7 1)
      let taken ← take joined 1 0
      pure (taken 1, canUseExclusively taken 0 1, take taken 1 0 |>.isSome, reraise taken 1 0)) =
      some (some (.owned 0), true, false, true) := by decide +kernel

example :
    (do
      let completed ← complete moved 1 (.errorWithBuffer 7 1) [1, 2]
      let joined ← join completed 0 1 (.errorWithBuffer 7 1)
      let taken ← take joined 1 0
      let released ← release taken 1 0
      pure (released 1, released 2)) = some (some .dropped, none) := by decide +kernel

/-- A child that acquired a second identity and did not release it cannot
complete, and neither can one that names an identity it does not hold. -/
example : complete (fun r => if r = 1 ∨ r = 2 then some (.owned 1) else none) 1
    (.errorWithBuffer 7 1) [1, 2] = none := by decide +kernel

example : complete moved 1 (.errorWithBuffer 7 2) [1, 2] = none := by decide +kernel

/-- Releasing the local first is exactly what admits the completion. -/
example :
    (do
      let child := fun r => if r = 1 ∨ r = 2 then some (Authority.owned 1) else none
      let tidy ← release child 2 1
      let completed ← complete tidy 1 (.errorWithBuffer 7 1) [1, 2]
      pure (completed 1, completed 2)) = some (some (.capsule 1), some .dropped) := by
  decide +kernel

end QCore.ErrorPayload

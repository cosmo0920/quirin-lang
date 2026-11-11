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

/-! Scoped read-loan custody (card T04).

One owner, at most one borrower, one resource at a time. A loan suspends
*exclusive* authority — mutation, release, move and re-loan — for everyone
including the owner, and grants *shared read* authority to exactly the owner and
the borrower. The only transition out of a loan is the creator's join, and it
requires the exact recorded owner/borrower pair, so a join that observed nothing
closes nothing.

These proofs concern the custody discipline. They are not about source import,
native scheduling, the C memory model, fractional shares, several borrowers, or
the reference/native executions agreeing. -/
namespace QCore.ReadLoan

/-- Where a resource's authority currently sits. `reserved` is the window
between committing to a creation and learning whether it happened. -/
inductive Authority where
  | owned (task : Nat)
  | reserved (reservation : Nat)
  | loaned (owner borrower : Nat)
  | dropped
  deriving DecidableEq, Repr

abbrev Custody := Nat → Option Authority

/-- Mutation, release, move and re-loan all need this. -/
def canUseExclusively (custody : Custody) (task resource : Nat) : Bool :=
  decide (custody resource = some (.owned task))

/-- Reading needs only this, and a loan is exactly what widens it. -/
def canRead (custody : Custody) (task resource : Nat) : Bool :=
  match custody resource with
  | some (.owned owner) => decide (owner = task)
  | some (.loaned owner borrower) => decide (owner = task ∨ borrower = task)
  | _ => false

private def replace (custody : Custody) (resource : Nat) (authority : Authority) : Custody :=
  fun r => if r = resource then some authority else custody r

/-- Park a resource for a creation that has not happened yet. Requires
exclusive authority, so an already loaned resource cannot be lent again. -/
def reserve (custody : Custody) (resource creator reservation : Nat) : Option Custody :=
  if canUseExclusively custody creator resource then
    some (replace custody resource (.reserved reservation))
  else none

/-- The creation failed. Nothing was lent, so the owner gets everything back. -/
def rollback (custody : Custody) (resource creator reservation : Nat) : Option Custody :=
  if custody resource = some (.reserved reservation) then
    some (replace custody resource (.owned creator))
  else none

/-- The creation succeeded: the reservation becomes a loan. -/
def commit (custody : Custody) (resource creator borrower reservation : Nat) : Option Custody :=
  if custody resource = some (.reserved reservation) then
    some (replace custody resource (.loaned creator borrower))
  else none

/-- The creator's join, and the only way out of a loan. -/
def closeLoan (custody : Custody) (resource owner borrower : Nat) : Option Custody :=
  if custody resource = some (.loaned owner borrower) then
    some (replace custody resource (.owned owner))
  else none

/-- Release needs exclusive authority, exactly like mutation. -/
def release (custody : Custody) (resource task : Nat) : Option Custody :=
  if canUseExclusively custody task resource then
    some (replace custody resource .dropped)
  else none

private theorem replace_here (custody : Custody) (resource : Nat) (authority : Authority) :
    replace custody resource authority resource = some authority := by
  simp [replace]

private theorem replace_elsewhere (custody : Custody) (resource other : Nat)
    (authority : Authority) (distinct : other ≠ resource) :
    replace custody resource authority other = custody other := by
  simp [replace, distinct]

private theorem reserve_result {custody after : Custody} {resource creator reservation : Nat}
    (checked : reserve custody resource creator reservation = some after) :
    custody resource = some (.owned creator) ∧
      after = replace custody resource (.reserved reservation) := by
  unfold reserve at checked
  split at checked
  · rename_i exclusive
    simp only [canUseExclusively, decide_eq_true_eq] at exclusive
    exact ⟨exclusive, by cases checked; rfl⟩
  · contradiction

private theorem commit_result {custody after : Custody}
    {resource creator borrower reservation : Nat}
    (checked : commit custody resource creator borrower reservation = some after) :
    custody resource = some (.reserved reservation) ∧
      after = replace custody resource (.loaned creator borrower) := by
  unfold commit at checked
  split at checked
  · exact ⟨by assumption, by cases checked; rfl⟩
  · contradiction

private theorem close_result {custody after : Custody} {resource owner borrower : Nat}
    (checked : closeLoan custody resource owner borrower = some after) :
    custody resource = some (.loaned owner borrower) ∧
      after = replace custody resource (.owned owner) := by
  unfold closeLoan at checked
  split at checked
  · exact ⟨by assumption, by cases checked; rfl⟩
  · contradiction

/-- Lending demands exclusive authority, so nothing that is already lent,
reserved, moved away or dropped can be lent. -/
theorem reserve_demands_exclusive_authority {custody after : Custody}
    {resource creator reservation : Nat}
    (checked : reserve custody resource creator reservation = some after) :
    custody resource = some (.owned creator) :=
  (reserve_result checked).1

/-- The borrower of a loan cannot lend it onward: it never had exclusive
authority to begin with. -/
theorem a_borrower_cannot_lend_onward (custody : Custody)
    {resource owner borrower reservation : Nat}
    (lent : custody resource = some (.loaned owner borrower)) :
    reserve custody resource borrower reservation = none := by
  simp [reserve, canUseExclusively, lent]

/-- Nor can the owner, while the loan is outstanding. -/
theorem an_outstanding_loan_cannot_be_lent_again (custody : Custody)
    {resource owner borrower reservation : Nat}
    (lent : custody resource = some (.loaned owner borrower)) (task : Nat) :
    reserve custody resource task reservation = none := by
  simp [reserve, canUseExclusively, lent]

/-- T04 acceptance 2, first clause: a creation that failed restores the owner's
custody exactly, pointwise, everywhere. -/
theorem failed_creation_restores_the_owner_exactly {custody reserved restored : Custody}
    {resource creator reservation : Nat}
    (parked : reserve custody resource creator reservation = some reserved)
    (undone : rollback reserved resource creator reservation = some restored) :
    restored = custody := by
  have original := reserve_result parked
  funext other
  by_cases here : other = resource
  · subst here
    unfold rollback at undone
    split at undone
    · cases undone
      simp [replace_here, original.1]
    · contradiction
  · unfold rollback at undone
    split at undone
    · cases undone
      rw [replace_elsewhere _ _ _ _ here, original.2, replace_elsewhere _ _ _ _ here]
    · contradiction

/-- The reservation window grants nobody anything, the creator included. -/
theorem a_reserved_resource_grants_no_authority_at_all {custody after : Custody}
    {resource creator reservation : Nat}
    (checked : reserve custody resource creator reservation = some after) (task : Nat) :
    canUseExclusively after task resource = false ∧ canRead after task resource = false := by
  rw [(reserve_result checked).2]
  constructor
  · simp [canUseExclusively, replace_here]
  · simp [canRead, replace_here]

/-- T04 acceptance 1. A loan suspends exclusive authority for *everyone*: the
owner has not transferred custody, and it still cannot mutate, release, move or
re-lend the resource. -/
theorem an_outstanding_loan_suspends_exclusive_authority (custody : Custody)
    {resource owner borrower : Nat}
    (lent : custody resource = some (.loaned owner borrower)) (task : Nat) :
    canUseExclusively custody task resource = false := by
  simp [canUseExclusively, lent]

/-- The same statement as the operation that matters most: release is refused. -/
theorem an_outstanding_loan_cannot_be_released (custody : Custody)
    {resource owner borrower : Nat}
    (lent : custody resource = some (.loaned owner borrower)) (task : Nat) :
    release custody resource task = none := by
  simp [release, canUseExclusively, lent]

/-- A loan grants read authority to exactly two tasks, and to no others. -/
theorem a_loan_admits_exactly_its_owner_and_its_borrower (custody : Custody)
    {resource owner borrower : Nat}
    (lent : custody resource = some (.loaned owner borrower)) (task : Nat) :
    canRead custody task resource = true ↔ (owner = task ∨ borrower = task) := by
  simp [canRead, lent]

/-- Committing a creation lends exactly the reserved resource and touches no
other identity. -/
theorem commit_lends_only_the_reserved_resource {custody after : Custody}
    {resource creator borrower reservation : Nat}
    (checked : commit custody resource creator borrower reservation = some after)
    (other : Nat) (distinct : other ≠ resource) :
    after other = custody other := by
  rw [(commit_result checked).2, replace_elsewhere _ _ _ _ distinct]

/-- A creation can only be committed against its own recorded reservation. -/
theorem commit_requires_its_own_reservation (custody : Custody)
    {resource creator borrower reservation other : Nat}
    (parked : custody resource = some (.reserved other)) (mismatch : other ≠ reservation) :
    commit custody resource creator borrower reservation = none := by
  simp [commit, parked, mismatch]

/-- A rolled-back creation cannot then be committed: the reservation is gone. -/
theorem a_rolled_back_creation_cannot_be_committed {custody reserved restored : Custody}
    {resource creator borrower reservation : Nat}
    (parked : reserve custody resource creator reservation = some reserved)
    (undone : rollback reserved resource creator reservation = some restored) :
    commit restored resource creator borrower reservation = none := by
  rw [failed_creation_restores_the_owner_exactly parked undone]
  simp [commit, (reserve_result parked).1]

/-- T04 acceptance 2, second clause. A join closes a loan only against the exact
recorded owner/borrower pair; a join that observed nothing to close leaves the
resource lent, and the previous theorems then refuse every reclamation. -/
theorem closing_a_loan_requires_the_recorded_pair (custody : Custody)
    {resource owner borrower claimed : Nat}
    (lent : custody resource = some (.loaned owner borrower))
    (mismatch : claimed ≠ owner) (task : Nat) :
    closeLoan custody resource claimed task = none := by
  simp [closeLoan, lent]
  omega

/-- A retained loan — the state a failed join leaves behind — refuses every
exclusive authority and every release, for every task, forever. -/
theorem a_retained_loan_refuses_every_reclamation (custody : Custody)
    {resource owner borrower : Nat}
    (lent : custody resource = some (.loaned owner borrower)) (task : Nat) :
    canUseExclusively custody task resource = false ∧
      release custody resource task = none ∧
      reserve custody resource task 0 = none := by
  exact ⟨an_outstanding_loan_suspends_exclusive_authority custody lent task,
    an_outstanding_loan_cannot_be_released custody lent task,
    an_outstanding_loan_cannot_be_lent_again custody lent task⟩

/-- T04 acceptance 2, third clause. A successful join returns exclusive
authority to the original owner and to nobody else. -/
theorem a_successful_join_restores_exclusive_authority_to_the_owner
    {custody after : Custody} {resource owner borrower : Nat}
    (checked : closeLoan custody resource owner borrower = some after) (task : Nat) :
    canUseExclusively after task resource = true ↔ task = owner := by
  rw [(close_result checked).2]
  simp [canUseExclusively, replace_here]
  exact ⟨fun h => h.symm, fun h => h.symm⟩

/-- Closing a loan revokes the borrower's read authority, unless the borrower
was the owner all along. -/
theorem a_successful_join_revokes_the_borrower_read {custody after : Custody}
    {resource owner borrower : Nat}
    (checked : closeLoan custody resource owner borrower = some after)
    (distinct : borrower ≠ owner) :
    canRead after borrower resource = false := by
  rw [(close_result checked).2]
  simp [canRead, replace_here]
  omega

/-- Closing a loan touches no other identity. -/
theorem closing_a_loan_preserves_every_other_resource {custody after : Custody}
    {resource owner borrower : Nat}
    (checked : closeLoan custody resource owner borrower = some after)
    (other : Nat) (distinct : other ≠ resource) :
    after other = custody other := by
  rw [(close_result checked).2, replace_elsewhere _ _ _ _ distinct]

/-- The whole admitted lifetime, composed: reserve, commit, close returns the
resource to the state it started in. Lending is authority-neutral over a
completed round trip. -/
theorem the_admitted_lending_round_trip_is_authority_neutral {custody reserved lent closed : Custody}
    {resource creator borrower reservation : Nat}
    (parked : reserve custody resource creator reservation = some reserved)
    (created : commit reserved resource creator borrower reservation = some lent)
    (joined : closeLoan lent resource creator borrower = some closed) :
    closed = custody := by
  have original := reserve_result parked
  funext other
  by_cases here : other = resource
  · subst here
    rw [(close_result joined).2, replace_here, original.1]
  · rw [closing_a_loan_preserves_every_other_resource joined other here,
      commit_lends_only_the_reserved_resource created other here, original.2,
      replace_elsewhere _ _ _ _ here]

/-- Decidable scenarios mirroring the Rust and source fixtures. -/
private def start : Custody := fun r => if r = 1 then some (.owned 0) else none

example :
    (do
      let reserved ← reserve start 1 0 5
      pure (reserved 1, reserved 2, canRead reserved 0 1)) =
      some (some (.reserved 5), none, false) := by decide +kernel

example : (do let dropped ← release start 1 0; pure (dropped 1)) = some (some .dropped) := by
  decide +kernel

example :
    (do
      let reserved ← reserve start 1 0 5
      let lent ← commit reserved 1 0 2 5
      pure (canRead lent 0 1, canRead lent 2 1, canRead lent 3 1,
        canUseExclusively lent 0 1, release lent 1 0 |>.isSome)) =
      some (true, true, false, false, false) := by decide +kernel

example :
    (do
      let reserved ← reserve start 1 0 5
      let restored ← rollback reserved 1 0 5
      pure (canUseExclusively restored 0 1, release restored 1 0 |>.isSome)) =
      some (true, true) := by decide +kernel

example :
    (do
      let reserved ← reserve start 1 0 5
      let lent ← commit reserved 1 0 2 5
      let closed ← closeLoan lent 1 0 2
      pure (canUseExclusively closed 0 1, canRead closed 2 1)) = some (true, false) := by
  decide +kernel

end QCore.ReadLoan

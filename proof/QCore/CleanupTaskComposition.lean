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

import QCore.RegionJoinsLoan

/-! `CT24-CLEANUP-TASK-COMPOSITION` — a handled cleanup composed with a live
task.

The source route erases the handled region into the cleanup releases that its
pack discharge performs, at the region's source position. The existing task
ledger then decides the release. This small model states the composition
boundary: a cleanup of the lent owner before join refuses; closing the loan at
join admits that same cleanup; and cleanup of an independent owner is admitted
at either placement. The pack's strict credit and a child's epoch grant are
separate fields, so discharging the former cannot settle or abandon the latter.

This is an abstract rule theorem. Faithful Rust extraction and the Lean/Rust
correspondence remain the review premises recorded by evidence rule 28. -/

namespace QCore.CleanupTaskComposition

open QCore.ErrorRegion

/-- The actual two components the source composition runs together: an
error-region cleanup pack and the scoped-task read-loan custody ledger. The
child's epoch grant is carried beside them; neither cleanup primitive consumes
that accounting field. -/
structure State where
  pack : CleanupPack
  custody : QCore.ReadLoan.Custody
  childGrant : Nat

structure Result where
  cleanup : DischargeResult
  custody : QCore.ReadLoan.Custody
  childGrant : Nat

/-- The source erasure's composition, in order: the task ledger must admit the
real release, then the opened cleanup pack is discharged. A refusal in either
existing component refuses the composition. -/
def executeCleanup (state : State) (resource task : Nat) : Option Result := do
  let afterRelease ← QCore.ReadLoan.release state.custody resource task
  let cleanup ← discharge (.opened state.pack)
  pure { cleanup, custody := afterRelease, childGrant := state.childGrant }

/-- CT24's reject cell through the existing custody transition: a pack cleanup
that tries to release an owner while its child still holds the share refuses. -/
theorem cleanup_of_the_lent_owner_before_join_refuses
    (state : State) {resource owner borrower : Nat}
    (lent : state.custody resource = some (.loaned owner borrower)) :
    executeCleanup state resource owner = none := by
  simp [executeCleanup,
    QCore.ReadLoan.an_outstanding_loan_cannot_be_released state.custody lent owner]

/-- Moving the join before the same cleanup uses `ReadLoan.closeLoan`'s real
transition. The join restores exclusive ownership, the release succeeds, and
the already-open pack discharges. -/
theorem the_same_cleanup_after_join_is_admitted
    (state : State) {closed : QCore.ReadLoan.Custody}
    {resource owner borrower : Nat}
    (sameCleanup : state.pack.actions = [resource])
    (joined : QCore.ReadLoan.closeLoan state.custody resource owner borrower = some closed) :
    ∃ result, executeCleanup { state with custody := closed } resource owner = some result ∧
      result.cleanup.cleanupTrace = [resource] := by
  have released := QCore.RegionJoinsLoan.the_loan_expires_at_the_join joined
  rw [Option.isSome_iff_exists] at released
  obtain ⟨afterRelease, released⟩ := released
  refine ⟨
    { cleanup :=
        { ledger := state.pack.baseline
          cleanupTrace := [resource]
          abandonedCredit := state.pack.creditRemainder
          capability := .consumed }
      custody := afterRelease
      childGrant := state.childGrant }, ?_, rfl⟩
  simp [executeCleanup, released, discharge, sameCleanup]

/-- A concrete two-identity custody state: resource 1 remains loaned to child
2, while independent resource 3 is exclusively owned by the creator. -/
def independentState : State :=
  { pack :=
      { region := 7, error := 8, baseline := [], actions := [3], creditRemainder := 5 }
    custody := fun resource =>
      if resource = 1 then some (.loaned 0 2)
      else if resource = 3 then some (.owned 0)
      else none
    childGrant := 4096 }

/-- The attribution control through the same two real transitions: cleanup of
the independent owner succeeds before the other resource's join, preserves
that outstanding loan identity, and leaves the child's epoch grant untouched.
The last equality is about this abstract product. The claim that Rust/source
extract this product faithfully remains the explicit correspondence premise. -/
theorem independent_cleanup_before_join_preserves_the_live_task :
    ∃ result, executeCleanup independentState 3 0 = some result ∧
      result.custody 1 = some (.loaned 0 2) ∧
      result.childGrant = 4096 := by
  let afterRelease : QCore.ReadLoan.Custody := fun resource =>
    if resource = 3 then some .dropped else independentState.custody resource
  refine ⟨
    { cleanup :=
        { ledger := [], cleanupTrace := [3], abandonedCredit := 5,
          capability := .consumed }
      custody := afterRelease
      childGrant := 4096 }, ?_, ?_, rfl⟩
  · rfl
  · simp [afterRelease, independentState]

end QCore.CleanupTaskComposition

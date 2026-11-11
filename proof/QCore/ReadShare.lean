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

/-! Counted read shares over one generation-indexed allocation (card T05).

Card T04's whole-authority loan becomes a **lending budget**. One owner may
hand out positive shares of `FULL`; the outstanding total may never exceed
`FULL`; exclusive authority — mutation, release, move, and any further lending
beyond the budget — exists exactly when the outstanding total is zero. A join
returns exactly the share its holder recorded, so full authority is
reconstructed only when the last share is back.

What is budgeted is the authority handed *out*. The owner's own read is
retained across lending, exactly as T04 retained it, and is sound only because
a write demands the reconstructed exclusive authority these theorems are about.
This is deliberately **not** the separation-logic reading in which the owner's
own read is itself a consumed fraction; that model is a named open boundary in
`docs/reports/remaining-t05/README.md`, not something proved here.

These proofs concern the share discipline. They are not about source import,
native scheduling, the C memory model, disjoint slicing, write shares, more
than two holders, or the reference/native executions agreeing. -/
namespace QCore.ReadShare

/-- The whole read authority of one resource generation. -/
def FULL : Nat := 1024

/-- The number of simultaneous holders this profile admits. -/
def MAX_READERS : Nat := 2

/-- Who holds a lent share. `pending` is the window between committing to a
creation and learning whether it happened; nobody reads through it. -/
inductive Holder where
  | borrower (task : Nat)
  | pending (reservation : Nat)
  deriving DecidableEq, Repr

/-- Where a resource's authority currently sits. `loaned` carries the owner and
the outstanding share list; `shares = []` never occurs in a reachable state,
because the last reclamation restores `owned`. -/
inductive Authority where
  | owned (task : Nat)
  | loaned (owner : Nat) (shares : List (Holder × Nat))
  | dropped
  deriving DecidableEq, Repr

abbrev Custody := Nat → Option Authority

/-- The outstanding lent total: what must come back before authority is whole. -/
def lentTotal : List (Holder × Nat) → Nat
  | [] => 0
  | (_, share) :: rest => share + lentTotal rest

/-- Mutation, release, move and further lending beyond the budget all need this,
and so does the Buffer write. -/
def canUseExclusively (custody : Custody) (task resource : Nat) : Bool :=
  decide (custody resource = some (.owned task))

/-- A write is exactly an exclusive-authority operation: there is no separate
write share in this profile. -/
abbrev canWrite := canUseExclusively

/-- The share this holder currently has of this resource. -/
def shareOf : List (Holder × Nat) → Holder → Nat
  | [], _ => 0
  | (holder, share) :: rest, wanted =>
      if holder = wanted then share else shareOf rest wanted

/-- Reading needs a positive share, or ownership with no creation in flight. -/
def canRead (custody : Custody) (task resource : Nat) : Bool :=
  match custody resource with
  | some (.owned owner) => decide (owner = task)
  | some (.loaned owner shares) =>
      decide (0 < shareOf shares (.borrower task)) ||
        (decide (owner = task) &&
          !shares.any (fun entry => match entry.1 with | .pending _ => true | _ => false))
  | _ => false

private def replace (custody : Custody) (resource : Nat) (authority : Authority) : Custody :=
  fun r => if r = resource then some authority else custody r

/-- The share list of a resource, empty when nothing is lent. -/
def sharesOf (custody : Custody) (resource : Nat) : List (Holder × Nat) :=
  match custody resource with
  | some (.loaned _ shares) => shares
  | _ => []

/-- Hand out one positive share. Every admission rule is checked before
anything is written: the creator must own the resource, the share must be a
positive fraction of `FULL`, the outstanding total must have room for it, the
holder must be new, and the holder count must stay within `MAX_READERS`. -/
def lend (custody : Custody) (resource creator : Nat) (holder : Holder) (share : Nat) :
    Option Custody :=
  match custody resource with
  | some (.owned owner) =>
      if owner = creator ∧ 0 < share ∧ share ≤ FULL then
        some (replace custody resource (.loaned creator [(holder, share)]))
      else none
  | some (.loaned owner shares) =>
      if owner = creator ∧ 0 < share ∧ share ≤ FULL ∧
          lentTotal shares + share ≤ FULL ∧ shares.length < MAX_READERS ∧
          shareOf shares holder = 0 then
        some (replace custody resource (.loaned creator (shares ++ [(holder, share)])))
      else none
  | _ => none

private def without (shares : List (Holder × Nat)) (holder : Holder) : List (Holder × Nat) :=
  shares.filter (fun entry => entry.1 ≠ holder)

/-- Return one recorded share. The resource becomes exclusively the owner's
again exactly when the last share comes back. -/
def reclaim (custody : Custody) (resource owner : Nat) (holder : Holder) (share : Nat) :
    Option Custody :=
  match custody resource with
  | some (.loaned recorded shares) =>
      if recorded = owner ∧ shares.contains (holder, share) then
        let remaining := without shares holder
        some (replace custody resource
          (if remaining.isEmpty then .owned owner else .loaned owner remaining))
      else none
  | _ => none

/-- Release needs exclusive authority, exactly like a write. -/
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

private theorem lend_owned {custody after : Custody} {resource creator owner share : Nat}
    {holder : Holder}
    (start : custody resource = some (.owned owner))
    (checked : lend custody resource creator holder share = some after) :
    owner = creator ∧ 0 < share ∧ share ≤ FULL ∧
      after = replace custody resource (.loaned creator [(holder, share)]) := by
  simp only [lend, start] at checked
  split at checked
  · rename_i admitted
    exact ⟨admitted.1, admitted.2.1, admitted.2.2, by cases checked; rfl⟩
  · contradiction

private theorem lend_loaned {custody after : Custody} {resource creator owner share : Nat}
    {holder : Holder} {shares : List (Holder × Nat)}
    (start : custody resource = some (.loaned owner shares))
    (checked : lend custody resource creator holder share = some after) :
    owner = creator ∧ 0 < share ∧ share ≤ FULL ∧ lentTotal shares + share ≤ FULL ∧
      shares.length < MAX_READERS ∧ shareOf shares holder = 0 ∧
      after = replace custody resource (.loaned creator (shares ++ [(holder, share)])) := by
  simp only [lend, start] at checked
  split at checked
  · rename_i admitted
    exact ⟨admitted.1, admitted.2.1, admitted.2.2.1, admitted.2.2.2.1,
      admitted.2.2.2.2.1, admitted.2.2.2.2.2, by cases checked; rfl⟩
  · contradiction

private theorem reclaim_result {custody after : Custody} {resource owner share : Nat}
    {holder : Holder} {recorded : Nat} {shares : List (Holder × Nat)}
    (start : custody resource = some (.loaned recorded shares))
    (checked : reclaim custody resource owner holder share = some after) :
    recorded = owner ∧ shares.contains (holder, share) ∧
      after = replace custody resource
        (if (without shares holder).isEmpty then .owned owner
          else .loaned owner (without shares holder)) := by
  simp only [reclaim, start] at checked
  split at checked
  · rename_i admitted
    exact ⟨admitted.1, admitted.2, by cases checked; rfl⟩
  · contradiction

private theorem lentTotal_append (shares : List (Holder × Nat)) (holder : Holder) (share : Nat) :
    lentTotal (shares ++ [(holder, share)]) = lentTotal shares + share := by
  induction shares with
  | nil => simp [lentTotal]
  | cons head rest ih => simp [lentTotal, ih]; omega

/-! ### The budget -/

/-- T05 acceptance 1. Lending is admitted only for a positive share, and the
outstanding total after it is exactly the old total plus that share: nothing is
minted and nothing is rounded. -/
theorem lending_adds_exactly_the_granted_share {custody after : Custody}
    {resource creator share : Nat} {holder : Holder}
    (checked : lend custody resource creator holder share = some after) :
    0 < share ∧ lentTotal (sharesOf after resource)
      = lentTotal (sharesOf custody resource) + share := by
  cases start : custody resource with
  | none => simp [lend, start] at checked
  | some authority =>
    cases authority with
    | dropped => simp [lend, start] at checked
    | owned owner =>
      have result := lend_owned start checked
      refine ⟨result.2.1, ?_⟩
      rw [result.2.2.2]
      simp [sharesOf, replace_here, start, lentTotal]
    | loaned owner shares =>
      have result := lend_loaned start checked
      refine ⟨result.2.1, ?_⟩
      rw [result.2.2.2.2.2.2]
      simp [sharesOf, replace_here, start, lentTotal_append]

/-- T05 acceptance 1. The outstanding total never exceeds the full authority
after an admitted lend, whatever it was before. -/
theorem the_outstanding_total_never_exceeds_full {custody after : Custody}
    {resource creator share : Nat} {holder : Holder}
    (_start : lentTotal (sharesOf custody resource) ≤ FULL)
    (checked : lend custody resource creator holder share = some after) :
    lentTotal (sharesOf after resource) ≤ FULL := by
  cases begin : custody resource with
  | none => simp [lend, begin] at checked
  | some authority =>
    cases authority with
    | dropped => simp [lend, begin] at checked
    | owned owner =>
      have result := lend_owned begin checked
      rw [result.2.2.2]
      simp [sharesOf, replace_here, lentTotal]
      exact result.2.2.1
    | loaned owner shares =>
      have result := lend_loaned begin checked
      rw [result.2.2.2.2.2.2]
      simp [sharesOf, replace_here, lentTotal_append]
      exact result.2.2.2.1

/-- T05 acceptance 1, the excess direction. A request that does not fit in the
remaining authority is refused outright: no partial share is granted. -/
theorem excess_allocation_of_authority_is_refused (custody : Custody)
    {resource creator owner share : Nat} {holder : Holder}
    {shares : List (Holder × Nat)}
    (lent : custody resource = some (.loaned owner shares))
    (excess : FULL < lentTotal shares + share) :
    lend custody resource creator holder share = none := by
  simp [lend, lent]
  omega

/-- A share of zero grants nothing, so it is not a share. -/
theorem a_zero_share_is_not_a_share (custody : Custody)
    (resource creator : Nat) (holder : Holder) :
    lend custody resource creator holder 0 = none := by
  unfold lend
  split <;> simp

/-- A single share above the full authority is refused even against an
untouched resource. -/
theorem an_over_full_share_is_refused (custody : Custody)
    {resource creator owner share : Nat} {holder : Holder}
    (start : custody resource = some (.owned owner)) (excess : FULL < share) :
    lend custody resource creator holder share = none := by
  simp [lend, start]
  omega

/-- Only the owner lends. A borrower cannot lend onward and a stranger cannot
lend at all, whatever authority is left over. -/
theorem only_the_owner_may_lend (custody : Custody)
    {resource owner creator share : Nat} {holder : Holder}
    {shares : List (Holder × Nat)}
    (lent : custody resource = some (.loaned owner shares)) (stranger : creator ≠ owner) :
    lend custody resource creator holder share = none := by
  simp [lend, lent]
  omega

/-- The declared reader bound is enforced, not assumed: a third simultaneous
holder is refused even when the remaining authority would allow it. -/
theorem a_third_simultaneous_holder_is_refused (custody : Custody)
    {resource owner creator share : Nat} {holder : Holder}
    {shares : List (Holder × Nat)}
    (lent : custody resource = some (.loaned owner shares))
    (full : MAX_READERS ≤ shares.length) :
    lend custody resource creator holder share = none := by
  simp [lend, lent]
  omega

/-! ### Reconstruction -/

/-- T05 acceptance 2. Exclusive authority — and therefore the write and the
release — is refused for *everyone* while any share is outstanding. -/
theorem an_outstanding_share_suspends_exclusive_authority (custody : Custody)
    {resource owner : Nat} {shares : List (Holder × Nat)}
    (lent : custody resource = some (.loaned owner shares)) (task : Nat) :
    canUseExclusively custody task resource = false ∧
      canWrite custody task resource = false ∧
      release custody resource task = none := by
  refine ⟨by simp [canUseExclusively, lent], by simp [canUseExclusively, lent], ?_⟩
  simp [release, canUseExclusively, lent]

/-- T05 acceptance 2. Returning one share of two does not reconstruct full
authority: the resource stays loaned to the other holder, who keeps its share
exactly. -/
theorem reconstruction_waits_for_the_last_share {custody after : Custody}
    {resource owner share other : Nat} {holder sibling : Holder}
    (distinct : sibling ≠ holder)
    (lent : custody resource = some (.loaned owner [(holder, share), (sibling, other)]))
    (checked : reclaim custody resource owner holder share = some after) (task : Nat) :
    after resource = some (.loaned owner [(sibling, other)]) ∧
      canUseExclusively after task resource = false := by
  have result := reclaim_result lent checked
  have remaining : without [(holder, share), (sibling, other)] holder = [(sibling, other)] := by
    simp [without, distinct]
  rw [result.2.2, remaining]
  exact ⟨by simp [replace_here], by simp [canUseExclusively, replace_here]⟩

/-- And the returned share is nobody's: the joined holder keeps none of it. -/
theorem a_returned_share_is_held_by_nobody {custody after : Custody}
    {resource owner share other : Nat} {holder sibling : Holder}
    (distinct : sibling ≠ holder)
    (lent : custody resource = some (.loaned owner [(holder, share), (sibling, other)]))
    (checked : reclaim custody resource owner holder share = some after) :
    shareOf (sharesOf after resource) holder = 0 := by
  have result := reclaim_result lent checked
  have remaining : without [(holder, share), (sibling, other)] holder = [(sibling, other)] := by
    simp [without, distinct]
  rw [result.2.2, remaining]
  simp [sharesOf, replace_here, shareOf, distinct]

/-- T05 acceptance 2, the completing direction. Returning the last share
restores exclusive authority to the owner and to nobody else. -/
theorem returning_the_last_share_restores_exclusive_authority {custody after : Custody}
    {resource owner share : Nat} {holder : Holder}
    (lent : custody resource = some (.loaned owner [(holder, share)]))
    (checked : reclaim custody resource owner holder share = some after) (task : Nat) :
    canUseExclusively after task resource = true ↔ task = owner := by
  have result := reclaim_result lent checked
  have remaining : without [(holder, share)] holder = [] := by simp [without]
  rw [result.2.2, remaining]
  simp [canUseExclusively, replace_here]
  exact ⟨fun h => h.symm, fun h => h.symm⟩

/-- A share can only be returned against its own recorded holder and amount, so
a join that observed nothing to return returns nothing. -/
theorem returning_requires_the_recorded_share (custody : Custody)
    {resource owner claimed : Nat} {holder : Holder}
    {shares : List (Holder × Nat)}
    (lent : custody resource = some (.loaned owner shares))
    (absent : (holder, claimed) ∉ shares) :
    reclaim custody resource owner holder claimed = none := by
  simp [reclaim, lent, absent]

/-- A retained share — the state a failed join leaves behind — refuses every
exclusive authority, every release and every further lend that would exceed the
budget, for every task, forever. -/
theorem a_retained_share_refuses_every_reclamation (custody : Custody)
    {resource owner : Nat} {shares : List (Holder × Nat)}
    (lent : custody resource = some (.loaned owner shares))
    (saturated : FULL ≤ lentTotal shares) (task : Nat) (holder : Holder) :
    canUseExclusively custody task resource = false ∧
      release custody resource task = none ∧
      lend custody resource task holder 1 = none := by
  refine ⟨by simp [canUseExclusively, lent], by simp [release, canUseExclusively, lent], ?_⟩
  simp [lend, lent]
  omega

/-! ### Reading -/

/-- T05 acceptance 3. Two holders with positive shares both read one resource,
and a task with no share does not — so two readers are two holders of one
identity, never two identities. -/
theorem two_positive_shares_admit_exactly_two_borrowers (custody : Custody)
    {resource owner first second left right : Nat}
    (distinct : first ≠ second) (positive : 0 < left ∧ 0 < right)
    (lent : custody resource
      = some (.loaned owner [(.borrower first, left), (.borrower second, right)]))
    (task : Nat) (stranger : task ≠ first ∧ task ≠ second ∧ task ≠ owner) :
    canRead custody first resource = true ∧ canRead custody second resource = true ∧
      canRead custody task resource = false := by
  refine ⟨?_, ?_, ?_⟩
  · simp [canRead, lent, shareOf, positive.1]
  · simp [canRead, lent, shareOf, distinct, positive.2]
  · simp [canRead, lent, shareOf, Ne.symm stranger.1, Ne.symm stranger.2.1,
      Ne.symm stranger.2.2]

/-- The owner keeps its own read across lending: the budget is over the
authority handed *out*, and it is only a write that the outstanding shares
suspend. -/
theorem the_owner_keeps_its_read_across_committed_lending (custody : Custody)
    {resource owner first left : Nat}
    (lent : custody resource = some (.loaned owner [(.borrower first, left)])) :
    canRead custody owner resource = true := by
  by_cases here : owner = first
  · subst here
    simp [canRead, lent, shareOf]
  · simp [canRead, lent, shareOf]

/-- A pending share has left the owner and reached nobody, so nobody reads
through it — the creation window grants no authority at all. -/
theorem a_pending_share_is_readable_by_nobody (custody : Custody)
    {resource owner reservation share : Nat}
    (parked : custody resource = some (.loaned owner [(.pending reservation, share)]))
    (task : Nat) :
    canRead custody task resource = false := by
  simp [canRead, parked, shareOf]

/-! ### Frames and round trips -/

/-- Lending touches no other identity. -/
theorem lending_changes_only_the_lent_resource {custody after : Custody}
    {resource creator share : Nat} {holder : Holder}
    (checked : lend custody resource creator holder share = some after)
    (other : Nat) (distinct : other ≠ resource) :
    after other = custody other := by
  cases start : custody resource with
  | none => simp [lend, start] at checked
  | some authority =>
    cases authority with
    | dropped => simp [lend, start] at checked
    | owned owner =>
      rw [(lend_owned start checked).2.2.2, replace_elsewhere _ _ _ _ distinct]
    | loaned owner shares =>
      rw [(lend_loaned start checked).2.2.2.2.2.2, replace_elsewhere _ _ _ _ distinct]

/-- Returning a share touches no other identity. -/
theorem returning_changes_only_the_lent_resource {custody after : Custody}
    {resource owner share : Nat} {holder : Holder}
    (checked : reclaim custody resource owner holder share = some after)
    (other : Nat) (distinct : other ≠ resource) :
    after other = custody other := by
  cases start : custody resource with
  | none => simp [reclaim, start] at checked
  | some authority =>
    cases authority with
    | owned owner => simp [reclaim, start] at checked
    | dropped => simp [reclaim, start] at checked
    | loaned recorded shares =>
      rw [(reclaim_result start checked).2.2, replace_elsewhere _ _ _ _ distinct]

/-- The whole admitted lifetime, composed: lending a share and returning it
leaves the resource exactly as it was. Lending is authority-neutral over a
completed round trip, at any share the budget admits. -/
theorem the_admitted_share_round_trip_is_authority_neutral {custody lent restored : Custody}
    {resource creator owner share : Nat} {holder : Holder}
    (start : custody resource = some (.owned owner))
    (granted : lend custody resource creator holder share = some lent)
    (returned : reclaim lent resource creator holder share = some restored) :
    restored = custody := by
  have granted_result := lend_owned start granted
  have lent_state : lent resource = some (.loaned creator [(holder, share)]) := by
    rw [granted_result.2.2.2, replace_here]
  have returned_result := reclaim_result lent_state returned
  have remaining : without [(holder, share)] holder = [] := by simp [without]
  funext other
  by_cases here : other = resource
  · subst here
    rw [returned_result.2.2, remaining]
    simp [replace_here, start, granted_result.1]
  · rw [returning_changes_only_the_lent_resource returned other here,
      lending_changes_only_the_lent_resource granted other here]

/-! ### Decidable scenarios mirroring the Rust and source fixtures. -/

private def start : Custody := fun r => if r = 1 then some (.owned 0) else none
private def HALF : Nat := FULL / 2

/-- Two complementary halves: both borrowers read, the owner reads, and nobody
may write. -/
example :
    (do
      let first ← lend start 1 0 (.borrower 1) HALF
      let both ← lend first 1 0 (.borrower 2) HALF
      pure (lentTotal (sharesOf both 1), canRead both 1 1, canRead both 2 1,
        canRead both 0 1, canRead both 3 1, canWrite both 0 1)) =
      some (FULL, true, true, true, false, false) := by decide +kernel

/-- One unit past the budget is refused, and the exact remainder is admitted. -/
example :
    (do
      let first ← lend start 1 0 (.borrower 1) (HALF + 1)
      pure (lend first 1 0 (.borrower 2) HALF |>.isSome,
        lend first 1 0 (.borrower 2) (HALF - 1) |>.isSome)) = some (false, true) := by
  decide +kernel

/-- Reconstruction waits for the last share: one join is not enough. -/
example :
    (do
      let first ← lend start 1 0 (.borrower 1) HALF
      let both ← lend first 1 0 (.borrower 2) HALF
      let one ← reclaim both 1 0 (.borrower 1) HALF
      let none ← reclaim one 1 0 (.borrower 2) HALF
      pure (canWrite one 0 1, lentTotal (sharesOf one 1), canWrite none 0 1,
        release none 1 0 |>.isSome)) = some (false, HALF, true, true) := by decide +kernel

/-- A third simultaneous holder is refused even with authority to spare. -/
example :
    (do
      let first ← lend start 1 0 (.borrower 1) 1
      let both ← lend first 1 0 (.borrower 2) 1
      pure (lentTotal (sharesOf both 1), lend both 1 0 (.borrower 3) 1 |>.isSome)) =
      some (2, false) := by decide +kernel

/-- A pending creation grants nobody anything, and rolling it back returns
exactly its own share while a sibling keeps its own. -/
example :
    (do
      let committed ← lend start 1 0 (.borrower 1) HALF
      let parked ← lend committed 1 0 (.pending 9) HALF
      let rolled ← reclaim parked 1 0 (.pending 9) HALF
      pure (canRead parked 0 1, canRead parked 1 1, lentTotal (sharesOf rolled 1),
        canRead rolled 0 1, canRead rolled 1 1)) =
      some (false, true, HALF, true, true) := by decide +kernel

/-- A borrower cannot lend onward, and a stranger cannot lend at all. -/
example :
    (do
      let lent ← lend start 1 0 (.borrower 1) HALF
      pure (lend lent 1 1 (.borrower 2) 1 |>.isSome,
        lend lent 1 5 (.borrower 2) 1 |>.isSome,
        lend lent 1 0 (.borrower 2) 1 |>.isSome)) = some (false, false, true) := by
  decide +kernel

end QCore.ReadShare

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

import QCore.ScopedTasks
import QCore.WriteLendTasks
import QCore.UsableRecords
import QCore.Transferability

/-!
# Task record captures: records cross spawn and join as their leaves

Card **W48-QR-TASK-RECORD-CAPTURES** (card 3 of the `.qr` language track;
implemented grammars §G02a, §G02b, §G02j, §G07.13, language specification §L16
and the `Q6xx` band of §L21). Models of

* the task lowering's representation of a record — its **leaves**, one slot per
  scalar or owned-Buffer leaf (`crates/qcore-frontend/src/source_tasks/records.rs`);
* the scoped-task ledger's custody transfers over a record's owned leaves at
  reservation, rollback, commit, completion and join
  (`crates/qcore-core/src/scoped_tasks.rs`, `task_execution.rs`'s `TrySpawn`,
  `Join` and `UnpackRecord`);
* the per-resource range write leases, read per leaf
  (`task_execution.rs`, the lifted one-leased-resource bound).

## What is proved

* **The aggregate transfer judgment is the conjunction of its leaves** —
  `QCore.Transferability.theAggregateJudgmentIsTheConjunctionOfItsLeaves`,
  restated here over the task route's leaf kinds
  (`a_record_crosses_exactly_when_every_leaf_may_cross`).
* **Custody is conserved across spawn and join for records**: moving a
  record's owned leaves into a reservation, committing them to the child,
  completing them into its capsule and joining them back leaves every leaf
  exactly where it was (`custody_is_conserved_across_spawn_and_join_for_records`);
  a collected record is unpacked with each carried identity in exactly one
  owned leaf, in order, none invented and none dropped
  (`unpacking_places_every_carried_identity_once`).
* **No byte has two write authorities with per-leaf lends**: over any
  interleaving of per-leaf lease opens and joins, every byte of every leaf has
  at most one write authority (`no_byte_has_two_write_authorities_per_leaf`),
  and two leaves leased at once is such a state (`two_fields_leased_at_once_is_reachable`).
* **Spawn failure restores the record exactly**: the rollback returns every
  owned leaf's custody to the parent and leaves the record's leaves as they
  were (`spawn_failure_restores_the_record_exactly`).
* **A refill is admitted exactly into a hole**, card 2's rule, on a leaf slot
  (`a_leaf_refill_is_admitted_exactly_into_a_hole`).

The judgment's own re-decided theorems — `theAggregateBoundaryIsDecidedExactlyOnce`,
`theBoundaryIsDecidedExactlyOnceOverRecords`, `aWriteCaptureOfARecordIsRefusedByName`,
`aWriteCaptureIsRefusedByNameOverRecords` — are in `QCore.Transferability`.

## What is not

The models are transcriptions of the Rust, bound to it by the test rows, not
by extraction. Nothing here is a data-race proof, a native-refinement claim or
a statement about C layout; the native rows are sanitizer and byte-comparison
evidence, separately.
-/

namespace QCore.TaskRecordCaptures

open QCore.ScopedTasks

/-! ## Records as leaves -/

/-- One leaf of a record value on the task route: a copy scalar (its value)
or an owned Buffer (its resource identity). -/
inductive Leaf where
  | scalar (value : Int)
  | owned (resource : Nat)
  deriving DecidableEq, Repr

/-- The owned identities a record holds, in leaf order. -/
def ownedLeaves : List Leaf → List Nat
  | [] => []
  | .scalar _ :: rest => ownedLeaves rest
  | .owned r :: rest => r :: ownedLeaves rest

/-- The flat transferability kind of a leaf. -/
def leafKind : Leaf → Transferability.Kind
  | .scalar _ => .i32
  | .owned _ => .ownedBuffer

/-- The aggregate judgment over a record value's leaves is the conjunction of
the leaves' own crossings: `copy` exactly when no leaf owns, `move` exactly
when some leaf owns (every task leaf crosses in its induced mode). -/
theorem a_record_crosses_exactly_when_every_leaf_may_cross (record : List Leaf) :
    ((Transferability.admitsAggregate (record.map leafKind) .copy).isSome ↔
        ∀ leaf ∈ record, (Transferability.admits (leafKind leaf) .copy).isSome) ∧
      ((Transferability.admitsAggregate (record.map leafKind) .move).isSome ↔
        ∃ leaf ∈ record, leafKind leaf = .ownedBuffer) := by
  have both := Transferability.theAggregateJudgmentIsTheConjunctionOfItsLeaves
    (record.map leafKind)
  constructor
  · rw [both.1]
    simp
  · rw [both.2]
    constructor
    · rintro ⟨_, k, member, moves⟩
      simp only [List.mem_map] at member
      obtain ⟨leaf, inRecord, rfl⟩ := member
      refine ⟨leaf, inRecord, ?_⟩
      cases leaf <;> simp_all [leafKind] <;> contradiction
    · rintro ⟨leaf, inRecord, owns⟩
      refine ⟨?_, leafKind leaf, List.mem_map.mpr ⟨leaf, inRecord, rfl⟩, ?_⟩
      · intro k member
        simp only [List.mem_map] at member
        obtain ⟨l, _, rfl⟩ := member
        cases l <;> rfl
      · rw [owns]; decide

/-! ## Custody across spawn and join -/

/-- The four custody transfers a record that owns makes when it moves to a
worker that hands it back whole: reserve, commit, complete, join. -/
def spawnAndJoin (before : Custody) (resources : List Nat) (parent reservation child : Nat) :
    Option Custody := do
  let reserved ← transfer before resources (.task parent) (.reservation reservation)
  let running ← transfer reserved resources (.reservation reservation) (.task child)
  let completed ← transfer running resources (.task child) (.capsule child)
  transfer completed resources (.capsule child) (.task parent)

/-- A checked transfer out of one custodian into another, then out of that one
into a third, is the transfer from the first into the third. -/
theorem transfer_then_transfer {before middle after : Custody} {resources : List Nat}
    {a b c : Custodian}
    (first : transfer before resources a b = some middle)
    (second : transfer middle resources b c = some after) :
    transfer before resources a c = some after := by
  have had := checked_transfer_had_original_custody first
  unfold transfer
  rw [ite_eq_left had]
  congr 1
  funext resource
  rw [checked_transfer_places_exact_resources second resource]
  by_cases member : resource ∈ resources
  · simp [member]
  · simp [member, transfer_preserves_unselected_resource first resource member]

/-- **Custody is conserved across spawn and join for records.** When a
record's owned leaves are all the parent's, moving them to a worker that
returns the record whole and joining it leaves the custody exactly as it was:
every owned leaf the parent's again, every other identity untouched. -/
theorem custody_is_conserved_across_spawn_and_join_for_records
    {before after : Custody} {record : List Leaf} {parent reservation child : Nat}
    (checked : spawnAndJoin before (ownedLeaves record) parent reservation child = some after) :
    after = before := by
  unfold spawnAndJoin at checked
  simp only [Option.bind_eq_bind, Option.bind_eq_some_iff] at checked
  obtain ⟨reserved, reserve, running, commit, completed, complete, join⟩ := checked
  have out := transfer_then_transfer (transfer_then_transfer (transfer_then_transfer
    reserve commit) complete) join
  have had := checked_transfer_had_original_custody reserve
  funext resource
  rw [checked_transfer_places_exact_resources out resource]
  by_cases member : resource ∈ ownedLeaves record
  · simp [member, had.2 resource member]
  · simp [member]

/-- The scalar leaves carry no custody: the transfers name exactly the owned
leaves, so an identity outside them never moves. -/
theorem a_scalar_leaf_moves_no_custody
    {before reserved : Custody} {record : List Leaf} {parent reservation : Nat}
    (checked : transfer before (ownedLeaves record) (.task parent) (.reservation reservation)
      = some reserved)
    (resource : Nat) (outside : resource ∉ ownedLeaves record) :
    reserved resource = before resource :=
  transfer_preserves_unselected_resource checked resource outside

/-- The shape of a record result's leaf, as the core's `TaskRecord` states it. -/
inductive LeafShape where
  | scalar
  | owned
  deriving DecidableEq, Repr

/-- Unpack a collected record (`UnpackRecord`, `record_value`): each scalar
leaf takes a value, each owned leaf the next carried identity. `none` when
the carried identities do not match the owned leaves one for one. -/
def unpack : List LeafShape → List Int → List Nat → Option (List Leaf)
  | [], _, [] => some []
  | [], _, _ :: _ => none
  | .scalar :: rest, value :: values, carried =>
    (unpack rest values carried).map (Leaf.scalar value :: ·)
  | .scalar :: _, [], _ => none
  | .owned :: rest, values, r :: carried =>
    (unpack rest values carried).map (Leaf.owned r :: ·)
  | .owned :: _, _, [] => none

/-- **Unpacking places every carried identity once**: a record unpacked from a
completion holds exactly the identities the completion carried, in order —
custody moved leaf by leaf, none invented and none dropped. -/
theorem unpacking_places_every_carried_identity_once (shapes : List LeafShape)
    (values : List Int) (carried : List Nat) (record : List Leaf)
    (unpacked : unpack shapes values carried = some record) :
    ownedLeaves record = carried := by
  induction shapes generalizing values carried record with
  | nil =>
    cases carried with
    | nil => simp [unpack] at unpacked; subst unpacked; rfl
    | cons _ _ => simp [unpack] at unpacked
  | cons shape rest ih =>
    cases shape with
    | scalar =>
      cases values with
      | nil => simp [unpack] at unpacked
      | cons value values =>
        simp only [unpack, Option.map_eq_some_iff] at unpacked
        obtain ⟨tail, inner, rfl⟩ := unpacked
        simpa [ownedLeaves] using ih values carried tail inner
    | owned =>
      cases carried with
      | nil => simp [unpack] at unpacked
      | cons r carried =>
        simp only [unpack, Option.map_eq_some_iff] at unpacked
        obtain ⟨tail, inner, rfl⟩ := unpacked
        simp [ownedLeaves, ih values carried tail inner]

/-- Non-vacuity: a record of a scalar and a Buffer unpacks from one carried
identity, and not from none or two. -/
theorem unpacking_a_pair_needs_exactly_its_one_identity :
    unpack [.scalar, .owned] [41] [7] = some [.scalar 41, .owned 7] ∧
      unpack [.scalar, .owned] [41] [] = none ∧
      unpack [.scalar, .owned] [41] [7, 8] = none := by
  decide

/-! ## Spawn failure -/

/-- The failure edge of a record move: the reservation is rolled back and the
parent's record binding — its leaves — is what it was. -/
def spawnFailure (record : List Leaf) (reserved : Custody) (parent reservation : Nat) :
    Option (List Leaf × Custody) :=
  (transfer reserved (ownedLeaves record) (.reservation reservation) (.task parent)).map
    (fun custody => (record, custody))

/-- **Spawn failure restores the record exactly**: every owned leaf's custody
returns to the parent, every other identity is untouched, and the record's
leaves — scalars and identities alike — are the ones it had. -/
theorem spawn_failure_restores_the_record_exactly
    {before reserved : Custody} {record : List Leaf} {parent reservation : Nat}
    (checked : transfer before (ownedLeaves record) (.task parent) (.reservation reservation)
      = some reserved) :
    spawnFailure record reserved parent reservation = some (record, before) := by
  simp [spawnFailure, rollback_restores_original_custody checked]

/-- While the reservation stands, no owned leaf is the parent's to use — the
window the lowering's `match` arms cannot reach. -/
theorem a_reserved_record_leaf_is_unavailable_to_its_parent
    {before reserved : Custody} {record : List Leaf} {parent reservation : Nat}
    (checked : transfer before (ownedLeaves record) (.task parent) (.reservation reservation)
      = some reserved)
    (phases : Nat → Phase) (resource : Nat) (member : resource ∈ ownedLeaves record) :
    canUse phases reserved parent resource = false :=
  reserved_resource_is_unavailable_to_parent checked phases resource member

/-! ## Per-leaf write leases -/

open QCore.WriteLendTasks in
/-- The lease ledgers of a record's owned leaves: one lease list per leaf
resource, each the per-resource ledger `WRITE-LEND-TASKS` models. -/
abbrev LeafLeases := Nat → List Lease

open QCore.WriteLendTasks QCore.SlicePermissions in
/-- Every state a spawn/join interleaving reaches over per-leaf leases: an
admitted open on **one** leaf (the ledger decides admission per resource),
or a join closing one holder's leases on **every** leaf. -/
inductive ReachableLeaves : LeafLeases → Prop
  | start : ReachableLeaves (fun _ => [])
  | opened {s : LeafLeases} {leaf holder : Nat} {r : Range} {t : List Lease} :
      ReachableLeaves s → openLease (s leaf) holder r = some t →
        ReachableLeaves (fun l => if l = leaf then t else s l)
  | closed {s : LeafLeases} (holder : Nat) :
      ReachableLeaves s → ReachableLeaves (fun l => closeLease (s l) holder)

open QCore.WriteLendTasks in
/-- Each leaf's own lease list is a state the per-resource model reaches. -/
theorem every_leaf_is_reachable {s : LeafLeases} (reachable : ReachableLeaves s) (leaf : Nat) :
    Reachable (s leaf) := by
  induction reachable with
  | start => exact .start
  | @opened s l holder r t _ opened ih =>
    by_cases here : leaf = l
    · subst here; simpa using Reachable.opened ih opened
    · simpa [here] using ih
  | closed holder _ ih => exact .closed holder ih

open QCore.WriteLendTasks in
/-- **No byte has two write authorities with per-leaf lends.** A byte is a
leaf and an offset; at every point of every interleaving, at most one party
holds write authority over it — the per-resource theorem, per leaf. -/
theorem no_byte_has_two_write_authorities_per_leaf {s : LeafLeases}
    (reachable : ReachableLeaves s) (leaf byte : Nat) :
    writeAuthorities (s leaf) byte ≤ 1 :=
  no_byte_has_two_write_authorities (every_leaf_is_reachable reachable leaf) byte

open QCore.WriteLendTasks QCore.SlicePermissions in
/-- Non-vacuity, and what the lifted bound admits: two fields leased at once —
leaf 1 to worker 10 and leaf 2 to worker 20, each over the whole payload `[0, 4)`
— is a reachable state, and the same offset of the two leaves has one writer
each, never two. -/
theorem two_fields_leased_at_once_is_reachable :
    ∃ s : LeafLeases, ReachableLeaves s ∧ (s 1).length = 1 ∧ (s 2).length = 1 ∧
      writeAuthorities (s 1) 0 = 1 ∧ writeAuthorities (s 2) 0 = 1 := by
  let r : Range := ⟨0, 4⟩
  have first : openLease [] 10 r = some [⟨10, r⟩] := by decide
  have second : openLease [] 20 r = some [⟨20, r⟩] := by decide
  let s1 : LeafLeases := fun l => if l = 1 then [⟨10, r⟩] else []
  have h1 : ReachableLeaves s1 := ReachableLeaves.opened (leaf := 1) ReachableLeaves.start first
  let s2 : LeafLeases := fun l => if l = 2 then [⟨20, r⟩] else s1 l
  have h2 : ReachableLeaves s2 := ReachableLeaves.opened (leaf := 2) h1 (by simpa [s1] using second)
  exact ⟨s2, h2, by decide, by decide, by decide, by decide⟩

/-! ## The refill, on a leaf slot -/

/-- **A refill is admitted exactly into a hole** — card
`W48-QR-USABLE-RECORDS`' `RefillField` rule, which the task route's
`RefillField` states over a leaf slot: over a live leaf it refuses
(`OverwriteOfLiveField`), and into a hole it moves the Buffer in. -/
theorem a_leaf_refill_is_admitted_exactly_into_a_hole (leaf : QCore.UsableRecords.Held)
    (resource : Nat) :
    (QCore.UsableRecords.refill leaf resource).isSome = true ↔ leaf = none :=
  QCore.UsableRecords.a_refill_is_admitted_exactly_into_a_hole leaf resource

end QCore.TaskRecordCaptures

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
# A refused release is no leak; a guarded release of a spared binding is decided (card NULLABLE-GUARDED-RELEASE)

The abstract rules implemented by `crates/qcore-c-gateway/src/legacy_settled_leak.rs`
(part A) and the sparing frame of `legacy_c.rs` (part B), gateway rule 74c.

Fluent Bit librdkafka's `rd_kafka_mock_handle_Metadata`
(`rdkafka_mock_handlers.c:1362-1494`) starts `requested_topics` NULL, assigns
it a non-null owned list only when `TopicsCnt > 0`, and releases it under
`if (requested_topics)` on both exits. It was a provisional `Unsafe`:
`&requested_topics->elems[i]` dropped the name from rule 10a's sparing frame,
refused statements poisoned its binding, the guard and the `nonnull` release
were refused, and the release's unresolved marker reached `return 0` only
after a work-list iterate had already claimed the list there.

## What is proved

* **A refused release claims nothing.** A refused release marks its site
  unresolved; an accepted one discharges it. Either way no leak is claimed of
  that site (`a_refused_release_claims_nothing`), and the union join keeps the
  marker, so no exit the refused path reaches claims it
  (`the_marker_survives_the_join`).
* **The claim follows the settled state.** An iterate reached before the
  marker claimed the site; the settled exit, which joins the marked path, does
  not (`the_iterate_claim_was_unsupported`). The Rust withdraws a held report
  exactly when the settled exit marks the identity, a refused release is among
  the marks, and the function carries an `Unknown` leaf (`withdraws`); a mark
  from any other refusal withdraws nothing
  (`another_refusal_withdraws_nothing`). Its table is `withdrawVectors`
  (`withdraw_vectors_hold`).
* **Only `Unsafe -> Unknown`.** A withdrawal happens beside an `Unknown` leaf,
  so the function folds to `Unknown` or stays `Unsafe` on another leak, never
  `Safe` or `Pending` (`a_withdrawal_is_never_below_unknown`).
* **The frame (part B).** A refused statement can change a local's value only
  by writing it by name or through an address of its slot. `&NAME->F` is an
  address in `*NAME`, never of `NAME`'s slot, so the frame that spares a name
  whose only `&` uses are member addresses is sound
  (`a_spared_name_keeps_its_value`), spares everything the old frame spared
  (`the_frame_only_widens`) and still poisons a slot address or a write
  (`a_slot_address_still_poisons`, `a_write_by_name_still_poisons`).
* **The guard refinement (rules 14l, 10b), on a binding that survives.** When
  the name is NULL exactly when the non-null constructor did not run, the
  non-null edge denotes the site (the `nonnull` release applies and
  discharges it) and the null edge owes nothing
  (`the_non_null_edge_denotes_the_site`, `the_null_edge_owes_nothing`).
  A reassignment between the guard and the release breaks the premise: the
  released value need not be the site (`a_reassignment_breaks_it`), which is
  why the binding must survive (a write by name poisons it).

## What this is not

A proof that the Rust computes its states, its frame or its schedule
faithfully. That the settled exit state is the work list's fixed point, that
`member_addresses_read` recognises exactly `&NAME->F` and `&(NAME)->F`, and
that `may_write_names` over-approximates the writes by name are review
premises (trust boundary 28), pinned by
`crates/qcore-c-gateway/tests/unit/legacy_settled_leak.rs` (the shared table)
and the fixtures in `crates/qcore-c-gateway/tests/nullable_guarded_release.rs`.
The frame premise is rule 10a's no-undefined-behaviour C object model. It is
not a statement about librdkafka.
-/

namespace QCore.NullableGuardedRelease

open QCore.VerdictSemigroup (V fold)

/-! ## Part A: a refused release claims nothing -/

/-- One site's facts at a program point: live, and marked by a refused
effect. -/
structure Site where
  live : Bool
  unresolved : Bool
  deriving DecidableEq, Repr

/-- `report_leaks`' claim on a site at an exit. -/
def claimed (s : Site) : Bool := s.live && !s.unresolved

/-- A release at a non-null position: an accepted one discharges the site, a
refused one (nullable, unresolved identity, ambiguous cascade, held,
published or taken object, derived pointer) marks it. -/
def release (accepted : Bool) (s : Site) : Site :=
  if accepted then { s with live := false } else { s with unresolved := true }

theorem a_refused_release_claims_nothing (s : Site) : claimed (release false s) = false := by
  simp [claimed, release]

theorem an_accepted_release_claims_nothing (s : Site) : claimed (release true s) = false := by
  simp [claimed, release]

/-- The flow's union join of one site's facts. -/
def join (a b : Site) : Site := ⟨a.live || b.live, a.unresolved || b.unresolved⟩

/-- No exit the refused path reaches claims the site, whatever joins it. -/
theorem the_marker_survives_the_join (a b : Site) :
    claimed (join (release false a) b) = false ∧ claimed (join b (release false a)) = false := by
  constructor <;> simp [claimed, join, release]

/-- The refused guard's null edge keeps the site live and unmarked; the
refused release's edge marks it. An iterate on which only the null edge had
reached the exit claimed the site; the settled exit joins both and claims
nothing. -/
theorem the_iterate_claim_was_unsupported :
    claimed ⟨true, false⟩ = true ∧
      claimed (join ⟨true, false⟩ (release false ⟨true, false⟩)) = false := by
  decide

/-- The Rust decision (`settled_leak_route::withdraws`): withdraw a held report
exactly when the settled exit marks the identity, a refused release is among
the marks' provenance, and the function carries an `Unknown` leaf. -/
def withdraws (marked byRefusedRelease unknownLeaf : Bool) : Bool :=
  marked && byRefusedRelease && unknownLeaf

/-- `(marked, by a refused release, unknown leaf, withdrawn)`, the Rust
`WITHDRAW_VECTORS` row by row. -/
def withdrawVectors : List (Bool × Bool × Bool × Bool) := [
  (false, false, false, false),
  (false, false, true, false),
  (false, true, false, false),
  (false, true, true, false),
  (true, false, false, false),
  (true, false, true, false),
  (true, true, false, false),
  (true, true, true, true)
]

theorem withdraw_vectors_hold :
    withdrawVectors.all (fun v => withdraws v.1 v.2.1 v.2.2.1 == v.2.2.2) = true := by
  decide

/-- A withdrawn report is one the settled exit does not claim. -/
theorem a_withdrawn_report_is_not_claimed (s : Site) (byRelease leaf : Bool)
    (w : withdraws s.unresolved byRelease leaf = true) : claimed s = false := by
  simp [withdraws, claimed] at *
  simp [w.1]

/-- A report the settled exit does not mark, or marks by another refusal
only, is published: the rule withdraws nothing else. -/
theorem an_unmarked_report_is_published (byRelease leaf : Bool) :
    withdraws false byRelease leaf = false := by
  simp [withdraws]

theorem another_refusal_withdraws_nothing (marked leaf : Bool) :
    withdraws marked false leaf = false := by
  simp [withdraws]

/-- A withdrawal happens beside an `Unknown` leaf, so the function's verdict
is `Unknown` or (on another leak) `Unsafe`: never `Safe`, never `Pending`. -/
theorem a_withdrawal_is_never_below_unknown (xs : List V) (leaf : V.unknown ∈ xs) :
    fold xs = .unknown ∨ fold xs = .provisionalUnsafe := by
  have h := QCore.VerdictSemigroup.every_link_is_below_the_fold leaf
  cases e : fold xs <;> simp_all [QCore.VerdictSemigroup.V.rank]

/-- The Metadata shape: before, the refusals and the unsupported claim;
after, the refusals alone. -/
theorem the_metadata_shape :
    fold [.unknown, .unknown, .provisionalUnsafe] = .provisionalUnsafe ∧
      fold [.unknown, .unknown] = .unknown := by
  decide

/-- A leak the settled exit still claims (`ut_testSameSubscriptions`'
assertion-failure returns) keeps the provisional `Unsafe` beside any
refusal. -/
theorem a_settled_leak_stays_unsafe : fold [.unknown, .provisionalUnsafe] = .provisionalUnsafe := by
  decide

/-! ## Part B: the sparing frame -/

/-- An address a statement may form from a body's `&` uses of a name: the
name's own slot (`&NAME`, `&(NAME)`), or a member of the object the name
points to (`&NAME->F`, `&(NAME)->F`). -/
inductive Addr where
  | slot
  | member
  deriving DecidableEq, Repr

/-- Whether a refused statement can change the name's value: a write by name,
or a store through an address of its slot. A member address designates
storage in `*NAME`, never `NAME`'s slot. -/
def canChange (writesByName : Bool) (uses : List Addr) : Bool :=
  writesByName || uses.contains .slot

/-- The value after a refused statement that may store `new`. -/
def after (writesByName : Bool) (uses : List Addr) (old new : Option Nat) : Option Nat :=
  if canChange writesByName uses then new else old

/-- The frame before the card: no `&` use of the name at all. -/
def oldSpared (writesByName : Bool) (uses : List Addr) : Bool :=
  !writesByName && uses.isEmpty

/-- The frame after the card: no `&` use of the name's slot. -/
def spared (writesByName : Bool) (uses : List Addr) : Bool :=
  !writesByName && !uses.contains .slot

/-- Soundness: a spared name keeps its value across the refused statement,
whatever the statement stores. -/
theorem a_spared_name_keeps_its_value (w : Bool) (uses : List Addr) (old new : Option Nat)
    (h : spared w uses = true) : after w uses old new = old := by
  simp [spared, after, canChange] at *
  simp [h.1, h.2]

theorem the_frame_only_widens (w : Bool) (uses : List Addr) (h : oldSpared w uses = true) :
    spared w uses = true := by
  simp [oldSpared, spared] at *
  obtain ⟨hw, hu⟩ := h
  subst hu
  simp [hw]

theorem a_slot_address_still_poisons (uses : List Addr) (w : Bool) :
    spared w (.slot :: uses) = false := by
  simp [spared]

theorem a_write_by_name_still_poisons (uses : List Addr) : spared true uses = false := by
  simp [spared]

/-- `requested_topics`: its only `&` use is `&requested_topics->elems[i]`. -/
theorem the_metadata_name_is_spared :
    oldSpared false [.member] = false ∧ spared false [.member] = true := by
  decide

/-! ## Part B: the guard on a surviving binding -/

/-- One execution reaching the guard, for the name `p` and the site `S` of the
non-null constructor that may have run. -/
structure Run where
  /-- The constructor ran (and returned its non-null result). -/
  acquired : Bool
  /-- `p` is NULL at the guard. -/
  pNull : Bool
  /-- `p` denotes `S` at the guard. -/
  denotes : Bool
  /-- `S` has been released on this execution before the guard. -/
  released : Bool
  deriving DecidableEq, Repr

/-- The premise the surviving binding gives: `p` was NULL-initialised, is
written only by the non-null constructor's assignment, and nothing between it
and the guard writes `p` (a write by name or a slot address poisons it, so the
binding would not survive). Then `p` denotes `S` exactly when the constructor
ran, and is NULL otherwise. -/
def Run.wf (r : Run) : Prop :=
  (r.acquired = true → r.denotes = true ∧ r.pNull = false) ∧
    (r.acquired = false → r.pNull = true ∧ r.denotes = false)

/-- `S`'s obligation on an execution. -/
def Run.owes (r : Run) : Bool := r.acquired && !r.released

/-- On the non-null edge `p` denotes `S`: the `nonnull` release applies to
`S` and discharges it. -/
theorem the_non_null_edge_denotes_the_site (r : Run) (wf : r.wf) (edge : r.pNull = false) :
    r.denotes = true ∧ r.acquired = true := by
  cases a : r.acquired with
  | true => exact ⟨(wf.1 a).1, rfl⟩
  | false =>
    have := (wf.2 a).1
    rw [edge] at this
    exact absurd this (by decide)

/-- On the null edge no execution owes `S`: the constructor never ran. -/
theorem the_null_edge_owes_nothing (r : Run) (wf : r.wf) (edge : r.pNull = true) :
    r.owes = false := by
  cases a : r.acquired with
  | false => simp [Run.owes, a]
  | true =>
    have := (wf.1 a).2
    rw [edge] at this
    exact absurd this (by decide)

/-- A reassignment of `p` between the constructor and the guard (`p =
other;`) breaks the premise: on the non-null edge `p` need not denote `S`,
which is still owed. -/
def reassigned : Run := ⟨true, false, false, false⟩

theorem a_reassignment_breaks_it :
    ¬ reassigned.wf ∧ reassigned.pNull = false ∧ reassigned.denotes = false ∧
      reassigned.owes = true := by
  refine ⟨?_, rfl, rfl, rfl⟩
  intro wf
  have := (wf.1 rfl).1
  exact absurd this (by decide)

end QCore.NullableGuardedRelease

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
# The `Borrowed` liveness precondition (card ALIAS-LOCALS-BORROW-LIVENESS)

`QCore.MayAlias` models the bounded *may*-alias refinement: a modeled read is
refused when some **other** same-tagged name was released on the path. Card
ALIAS-LOCALS recorded, as its first open item, that the refinement reaches the
hazard only along that relation, because a `Borrowed` position has **no
implemented precondition at all** — the transfer function's parameter loop falls
through on `Borrowed`, so a body may hand a callee an identity the projection
itself records `RELEASED` and still be certified `Safe`.

This module models the decision that closes it,
`crates/qcore-c-gateway/src/borrow_liveness.rs` together with
`legacy_c::require_live_borrowed_actuals`. It is a **must** rule over *one*
name's own binding, and it is deliberately not an alias question: nothing here
reads a peer, a declared type or an alias fact. The two relations stay separate,
and the separation is audited (`a_name_the_projection_will_not_settle_is_never_refused`,
which is exactly the shape a formal parameter has).

## What is decided

`liveness` answers `definitelyNotLive` only under a conjunction of facts:

* the name is **bound with a settled denotation** — exactly one target, not
  `unknown`, not `mayBeNull` — the same commitment `QCore.MayAlias.settled`
  reads, shared so the two rules cannot drift apart;
* that identity carries **no unresolved effect**;
* its joined status satisfies `custodyDefinitelyEnded`: **no** bit that entitles
  a read (`live`, `contained`) and **some** `released` bit. Because `Status.join`
  is a pointwise union, an absent bit is a must fact and a present one a may
  fact, so the condition reads "custody ended on every reaching path, and one of
  them destroyed the identity"
  (`a_refusal_implies_no_incoming_path_entitles_a_read`).

Card ALIAS-LOCALS-BORROW-LIVENESS required the status to be **exactly**
released; card HYGIENE-14 (`BORROW-LIVENESS-MIXED`) derived the condition above
from the join semantics, which admits one further union —
`released ∪ escaped`, a release on one branch joined with a consumption on the
other. `the_refusable_statuses_are_exactly_released_and_released_with_escaped`
decides the whole sixteen-status domain, and its Rust twin
(`borrow_liveness::tests::the_refusable_status_sets_are_exactly_…`) enumerates
the same domain by bitmap code, so the two sides pin one table.

Two of that card's theorems were **retired by name** rather than quietly
restated, because the widening made them false:
`a_release_joined_with_an_escape_is_never_refused` (now
`a_release_joined_with_an_escape_is_refused`) and
`a_refusal_implies_every_incoming_path_released` (now
`a_refusal_implies_no_incoming_path_entitles_a_read`, which no longer claims
anything about the escaped bit). `a_refusal_names_a_settled_released_identity`
became `a_refusal_names_a_settled_identity_whose_custody_ended` for the same
reason.

The Rust decision carries one further conjunct with no counterpart here: a
status bit outside the analyzer's four is not decided. This model has exactly
four bits by construction, so the conjunct is unstatable rather than omitted.

Everything else is `notDecided` and preserves the existing answer exactly,
including every scalar actual — §C04.2 admits scalars in `Borrowed` positions
and nowhere else, and a scalar is a name this projection never binds.

## The published class

A refusal is published as the existing `UnknownAlias`, i.e. the `unresolved`
class, never the `violation` class `classify_diagnostics` publishes as
provisional `Unsafe`. `publish` models that decision and
`a_refusal_is_never_the_violation_class` audits it. The grounds are recorded in
`docs/reports/remaining-borrow-liveness/README.md` §3; the model's own
contribution is `the_existing_violation_bar_is_strictly_weaker`, which shows
that the state fact this rule requires is *stronger* than the one the existing
`DoubleRelease` fires on, so publishing the weaker class is not a weaker
measurement.

Everything here models the *decision*. That the Rust reader extracts the
bindings, the release events and the contract modes faithfully from C source is
a premise, not a theorem, exactly as in `QCore.MayAlias` and
`QCore.BorrowedActual`.
-/

namespace QCore.BorrowLiveness

/-- An abstract acquisition identity. -/
abbrev Site := Nat

/-- The joined per-site status bitmap, as four independent bits.

    `legacy_c` stores it as `u8` flags `LIVE`/`RELEASED`/`ESCAPED`/`CONTAINED`;
    the four booleans are the same information with the bit arithmetic removed,
    so that `join` below is visibly the union the analyzer performs. -/
structure Status where
  live : Bool
  released : Bool
  escaped : Bool
  contained : Bool
deriving DecidableEq, Repr

namespace Status

/-- A site no reaching path has recorded at all. -/
def empty : Status := ⟨false, false, false, false⟩

/-- Live on every reaching path. -/
def liveOnly : Status := ⟨true, false, false, false⟩

/-- Released on every reaching path, and in no other state on any of them.
    The **only** status this rule refuses. -/
def releasedOnly : Status := ⟨false, true, false, false⟩

/-- Custody moved to a callee. `resource_consumed` transfers the obligation
    "without claiming that the callee destroys it" (§C04.2), so this is not a
    dead identity and is never refused. -/
def escapedOnly : Status := ⟨false, false, true, false⟩

/-- Held beneath an owner. -/
def containedOnly : Status := ⟨false, false, false, true⟩

/-- `AbstractState::join` unions the status bits of two incoming states. -/
def join (a b : Status) : Status :=
  ⟨a.live || b.live, a.released || b.released, a.escaped || b.escaped,
   a.contained || b.contained⟩

/-- The bits whose reaching path leaves this body a pointer the projection knows
    to be **alive**, so a read of it is entitled there.

    `live` is the body's own retained custody. `contained` is an identity held
    beneath an owner: containment is not destruction, and the analyzer moves a
    contained child to a terminal state only when its *owner* is released or
    escapes. -/
def entitlesARead (st : Status) : Bool := st.live || st.contained

/-- The analyzer's `u8` bitmap for this status: `LIVE = 1`, `RELEASED = 2`,
    `ESCAPED = 4`, `CONTAINED = 8`. Present so this model and the Rust module
    can pin the *same* table over the *same* domain rather than two tables that
    happen to read alike. -/
def code (st : Status) : Nat :=
  (if st.live then 1 else 0) + (if st.released then 2 else 0)
    + (if st.escaped then 4 else 0) + (if st.contained then 8 else 0)

end Status

/-- Every status the four bits can take, enumerated by bitmap code — the same
    `0..=15` domain the Rust test enumerates. -/
def allStatuses : List Status :=
  (List.range 16).map fun n =>
    ⟨n % 2 == 1, (n / 2) % 2 == 1, (n / 4) % 2 == 1, (n / 8) % 2 == 1⟩

/-- The status-set condition of card HYGIENE-14: **no** reaching path leaves the
    body a pointer the projection knows to be alive, and **some** reaching path
    destroyed the identity.

    Both halves are load-bearing and both are audited: dropping the first would
    refuse a live identity (`an_entitling_incoming_path_prevents_the_refusal`),
    and dropping the second would refuse a merely consumed one, closing the
    `C02-P-ALIAS-CONSUME` boundary through the back door
    (`the_released_conjunct_is_load_bearing`). -/
def custodyDefinitelyEnded (st : Status) : Bool :=
  !st.entitlesARead && st.released

/-- What one name denotes here, as `AbstractPointer` records it. -/
structure Denotation where
  targets : List Site
  unknown : Bool
  mayBeNull : Bool
deriving DecidableEq, Repr

/-- The single non-null identity this name denotes, when the projection
    committed to exactly one.

    This is `QCore.MayAlias.settled`'s condition, restated over the same three
    fields. Both rules read the identical commitment on purpose: a definite
    alias and a definite release are two questions about one notion of "the
    projection decided". -/
def Denotation.settled (d : Denotation) : Option Site :=
  if d.unknown || d.mayBeNull then none
  else match d.targets with
    | [s] => some s
    | _ => none

/-- The projection facts one `Borrowed` actual is decided against.

    `denotation = none` is a name this body never binds: every scalar actual,
    and every identifier outside the resource projection. -/
structure Facts where
  denotation : Option Denotation
  status : Site → Status
  unresolved : Site → Bool

/-- The decision. -/
inductive Liveness where
  /-- The projection committed to one identity; no path reaching this call
      leaves that identity in a state where the body holds a pointer the
      projection knows to be alive, and some path destroyed it. -/
  | definitelyNotLive
  /-- Anything else. No claim in either direction. -/
  | notDecided
deriving DecidableEq, Repr

/-- The rule, fail-closed to `notDecided` at every step. -/
def liveness (f : Facts) : Liveness :=
  match f.denotation with
  | none => .notDecided
  | some d =>
    match d.settled with
    | none => .notDecided
    | some s =>
      if f.unresolved s then .notDecided
      else if custodyDefinitelyEnded (f.status s) then .definitelyNotLive
      else .notDecided

/-- The three verdict classes `classify_diagnostics` publishes. -/
inductive Verdict where
  | safe
  | unresolved
  | violation
deriving DecidableEq, Repr

/-- What a refusal does to the function's published verdict: it contributes an
    `UnknownAlias`, which is the `unresolved` class, and `Unsafe` still wins over
    `Unknown` in `classify_diagnostics`. -/
def publish (v : Verdict) (l : Liveness) : Verdict :=
  match l with
  | .notDecided => v
  | .definitelyNotLive =>
    match v with
    | .violation => .violation
    | _ => .unresolved

/-! ## Convenience constructors for the audited scenarios -/

/-- One settled name denoting site `0` with the given joined status, under no
    unresolved effect. -/
def settledAt (st : Status) : Facts :=
  { denotation := some ⟨[0], false, false⟩
    status := fun _ => st
    unresolved := fun _ => false }

/-! ## The positive direction, and its non-vacuity -/

/-- The headline shape: one name, no peer anywhere, released and then handed to
    a `Borrowed` position. The may-alias relation cannot reach it — it relates
    two *distinct* names — and this rule does. -/
theorem a_settled_released_identity_is_refused :
    liveness (settledAt Status.releasedOnly) = .definitelyNotLive := by
  decide

/-- The refusal withdraws a certificate rather than issuing an allegation. -/
theorem a_refusal_withdraws_a_certificate :
    publish .safe .definitelyNotLive = .unresolved := by
  decide

/-- **Card HYGIENE-14's movement** (`BORROW-LIVENESS-MIXED`): released on one
    branch and consumed on the other. Both arms ended this body's custody and
    one of them destroyed the identity, so no reaching path leaves a pointer the
    projection knows to be alive.

    This retires card ALIAS-LOCALS-BORROW-LIVENESS's
    `a_release_joined_with_an_escape_is_never_refused`, which stated the
    opposite; the retirement is deliberate and named in the module header. -/
theorem a_release_joined_with_an_escape_is_refused :
    liveness (settledAt (Status.join Status.releasedOnly Status.escapedOnly))
      = .definitelyNotLive := by
  decide

/-- The whole classification, decided over the entire sixteen-status domain and
    reported by the analyzer's own bitmap codes: `2` is `RELEASED` and `6` is
    `RELEASED | ESCAPED`. The Rust module's
    `the_refusable_status_sets_are_exactly_released_and_released_with_escaped`
    enumerates `0..=15` and pins the same two values, so neither side can widen
    the condition without the other's table moving. -/
theorem the_refusable_statuses_are_exactly_released_and_released_with_escaped :
    (allStatuses.filter custodyDefinitelyEnded).map Status.code = [2, 6] := by
  decide

/-- …and the enumeration really is the whole domain, in the analyzer's own
    numbering. -/
theorem the_enumerated_domain_is_every_bitmap :
    allStatuses.map Status.code = List.range 16 := by
  decide

/-! ## Every way the decision fails closed -/

theorem a_live_identity_is_never_refused :
    liveness (settledAt Status.liveOnly) = .notDecided := by
  decide

/-- Custody moved is not custody destroyed. This is the `C02-P-ALIAS-CONSUME`
    boundary, retained rather than quietly closed. -/
theorem an_escaped_identity_is_never_refused :
    liveness (settledAt Status.escapedOnly) = .notDecided := by
  decide

theorem a_contained_identity_is_never_refused :
    liveness (settledAt Status.containedOnly) = .notDecided := by
  decide

theorem an_identity_absent_from_the_state_is_never_refused :
    liveness (settledAt Status.empty) = .notDecided := by
  decide

/-- Released on one branch and live on another joins to both bits, and that is
    not a definite release. -/
theorem a_release_on_one_branch_only_is_never_refused :
    liveness (settledAt (Status.join Status.releasedOnly Status.liveOnly))
      = .notDecided := by
  decide

/-- Released on one branch and **contained** on another: containment leaves the
    object alive beneath its owner, so a reaching path does entitle the read. -/
theorem a_release_joined_with_a_containment_is_never_refused :
    liveness (settledAt (Status.join Status.releasedOnly Status.containedOnly))
      = .notDecided := by
  decide

/-- **Every scalar `Borrowed` actual takes this path.** §C04.2 admits scalars in
    exactly this position; a rule that fired on an unbound name would refuse
    most of the corpus for no reason at all. -/
theorem an_unbound_name_is_never_refused (st : Site → Status) (u : Site → Bool) :
    liveness { denotation := none, status := st, unresolved := u } = .notDecided := by
  rfl

/-- The separation from the may-alias relation, in the model.

    A name the projection will not settle — an unknown, ambiguous or
    possibly-null binding, which is exactly the shape an uncontracted **formal
    parameter** has — is never refused here, whatever any other name did. The
    may rule keeps that case and this rule never takes it. -/
theorem a_name_the_projection_will_not_settle_is_never_refused
    (d : Denotation) (f : Facts) (hd : f.denotation = some d)
    (h : d.settled = none) : liveness f = .notDecided := by
  simp [liveness, hd, h]

theorem an_unknown_binding_is_never_refused (st : Site → Status) (u : Site → Bool) :
    liveness { denotation := some ⟨[0], true, false⟩, status := st, unresolved := u }
      = .notDecided := by
  rfl

theorem a_possibly_null_binding_is_never_refused (st : Site → Status) (u : Site → Bool) :
    liveness { denotation := some ⟨[0], false, true⟩, status := st, unresolved := u }
      = .notDecided := by
  rfl

theorem an_ambiguous_binding_is_never_refused (st : Site → Status) (u : Site → Bool) :
    liveness { denotation := some ⟨[0, 1], false, false⟩, status := st, unresolved := u }
      = .notDecided := by
  rfl

theorem an_empty_binding_is_never_refused (st : Site → Status) (u : Site → Bool) :
    liveness { denotation := some ⟨[], false, false⟩, status := st, unresolved := u }
      = .notDecided := by
  rfl

/-- A site whose disposition the projection already declined to decide stays
    undecided here: an unresolved effect is precisely that refusal. -/
theorem an_unresolved_effect_site_is_never_refused :
    liveness { denotation := some ⟨[0], false, false⟩
               status := fun _ => Status.releasedOnly
               unresolved := fun _ => true } = .notDecided := by
  decide

/-! ## The converse: a refusal names the facts it claims -/

/-- Every refusal is backed by a settled binding whose identity carries no
    unresolved effect and whose joined status satisfies the condition. The rule
    invents nothing.

    Card HYGIENE-14 retired this theorem's predecessor,
    `a_refusal_names_a_settled_released_identity`, whose final conjunct said the
    status was *exactly* released. -/
theorem a_refusal_names_a_settled_identity_whose_custody_ended (f : Facts)
    (h : liveness f = .definitelyNotLive) :
    ∃ d s, f.denotation = some d ∧ d.settled = some s ∧
      f.unresolved s = false ∧ custodyDefinitelyEnded (f.status s) = true := by
  cases hd : f.denotation with
  | none => simp [liveness, hd] at h
  | some d =>
    cases hs : d.settled with
    | none => simp [liveness, hd, hs] at h
    | some s =>
      by_cases hu : f.unresolved s = true
      · simp [liveness, hd, hs, hu] at h
      · by_cases hr : custodyDefinitelyEnded (f.status s) = true
        · exact ⟨d, s, rfl, hs, by simpa using hu, hr⟩
        · simp [liveness, hd, hs, hu, hr] at h

/-! ## The join, and why the fact is a must fact -/

theorem the_join_is_commutative (a b : Status) : Status.join a b = Status.join b a := by
  simp [Status.join, Bool.or_comm]

theorem the_join_is_idempotent (a : Status) : Status.join a a = a := by
  cases a; simp [Status.join]

/-- A single incoming path on which the identity is still live prevents the
    refusal, however many other paths released it. -/
theorem a_live_incoming_path_prevents_the_refusal (a b : Status) (h : a.live = true) :
    liveness (settledAt (Status.join a b)) = .notDecided := by
  simp [liveness, settledAt, Denotation.settled, custodyDefinitelyEnded,
    Status.entitlesARead, Status.join, h]

/-- The same for a containment: the widened rule keeps *both* entitling bits, so
    a contained incoming path is as protective as a live one. -/
theorem an_entitling_incoming_path_prevents_the_refusal (a b : Status)
    (h : a.live = true ∨ a.contained = true) :
    liveness (settledAt (Status.join a b)) = .notDecided := by
  cases h with
  | inl h =>
    simp [liveness, settledAt, Denotation.settled, custodyDefinitelyEnded,
      Status.entitlesARead, Status.join, h]
  | inr h =>
    simp [liveness, settledAt, Denotation.settled, custodyDefinitelyEnded,
      Status.entitlesARead, Status.join, h]

/-- The must half, stated over the join: a refusal at a merge point implies that
    **no** incoming path left the identity live or contained, and that at least
    one of them released it. It says nothing about the escaped bit — that is
    exactly what card HYGIENE-14 changed, and the retired predecessor
    `a_refusal_implies_every_incoming_path_released` claimed otherwise. -/
theorem a_refusal_implies_no_incoming_path_entitles_a_read (a b : Status)
    (h : liveness (settledAt (Status.join a b)) = .definitelyNotLive) :
    a.live = false ∧ b.live = false ∧
      a.contained = false ∧ b.contained = false ∧
      (a.released = true ∨ b.released = true) := by
  revert h
  obtain ⟨l1, r1, e1, c1⟩ := a
  obtain ⟨l2, r2, e2, c2⟩ := b
  cases l1 <;> cases r1 <;> cases e1 <;> cases c1 <;>
    cases l2 <;> cases r2 <;> cases e2 <;> cases c2 <;> decide

/-! ## The rule against the existing `Unsafe` bar -/

/-- What `apply_resource_action` tests before reporting `DoubleRelease`: the
    joined status merely *carries* the released bit. That is a **may** fact over
    reaching paths. -/
def doubleReleaseBar (st : Status) : Bool := st.released

/-- Every state this rule refuses would also satisfy the existing `Unsafe`
    kinds' state test. -/
theorem a_refusal_satisfies_the_existing_bar (st : Status)
    (h : liveness (settledAt st) = .definitelyNotLive) : doubleReleaseBar st = true := by
  have hc : custodyDefinitelyEnded st = true := by
    by_cases hne : custodyDefinitelyEnded st = true
    · exact hne
    · simp [liveness, settledAt, Denotation.settled, hne] at h
  simp only [custodyDefinitelyEnded, Bool.and_eq_true] at hc
  simpa [doubleReleaseBar] using hc.2

/-- …and the converse fails, so this rule requires **strictly more** of the
    projection state than the kinds published as provisional `Unsafe` do. The
    weaker published class is therefore not a weaker measurement. -/
theorem the_existing_violation_bar_is_strictly_weaker :
    ∃ st, doubleReleaseBar st = true ∧ liveness (settledAt st) = .notDecided := by
  exact ⟨Status.join Status.releasedOnly Status.liveOnly, by decide, by decide⟩

/-! ## The published class -/

/-- The rule can never certify: a published `safe` means it did not fire. -/
theorem the_rule_never_certifies (v : Verdict) (l : Liveness)
    (h : publish v l = .safe) : v = .safe := by
  cases l with
  | notDecided => simpa [publish] using h
  | definitelyNotLive => cases v <;> simp [publish] at h

/-- The rule never manufactures an upstream defect allegation: a function that
    was not already in the `violation` class does not enter it here. -/
theorem a_refusal_is_never_the_violation_class (v : Verdict) (l : Liveness)
    (h : v ≠ .violation) : publish v l ≠ .violation := by
  cases l with
  | notDecided => simpa [publish] using h
  | definitelyNotLive => cases v <;> simp [publish] at h ⊢

/-- An existing violation is not softened by this rule either. -/
theorem an_existing_violation_survives (l : Liveness) :
    publish .violation l = .violation := by
  cases l <;> rfl

/-- Firing is what changes the answer; not firing changes nothing at all. -/
theorem not_firing_changes_nothing (v : Verdict) : publish v .notDecided = v := by
  rfl

/-! ## Monotonicity and non-vacuity of each guard -/

/-- Recording an unresolved effect can only drop a refusal, never create one:
    the fail-closed direction of the second conjunct, over every binding and
    every status whatsoever. -/
theorem recording_an_unresolved_effect_only_drops_refusals
    (d : Option Denotation) (st : Site → Status) :
    liveness { denotation := d, status := st, unresolved := fun _ => true }
      = .notDecided := by
  cases d with
  | none => rfl
  | some d =>
    cases hs : d.settled with
    | none => simp [liveness, hs]
    | some _ => simp [liveness, hs]

/-- The `unresolved` guard is load-bearing: the same facts without it fire. -/
theorem the_unresolved_guard_is_load_bearing :
    liveness (settledAt Status.releasedOnly) = .definitelyNotLive ∧
      liveness { denotation := some ⟨[0], false, false⟩
                 status := fun _ => Status.releasedOnly
                 unresolved := fun _ => true } = .notDecided := by
  exact ⟨by decide, by decide⟩

/-- The settled guard is load-bearing: the identical status with an ambiguous
    binding does not fire. -/
theorem the_settled_guard_is_load_bearing :
    liveness (settledAt Status.releasedOnly) = .definitelyNotLive ∧
      liveness { denotation := some ⟨[0, 1], false, false⟩
                 status := fun _ => Status.releasedOnly
                 unresolved := fun _ => false } = .notDecided := by
  exact ⟨by decide, by decide⟩

/-- The `released` conjunct is load-bearing, which is what keeps
    `C02-P-ALIAS-CONSUME` open rather than closing it through the back door: a
    consumed identity alone is admitted, and it is the *added* release that makes
    the join refusable. -/
theorem the_released_conjunct_is_load_bearing :
    liveness (settledAt Status.escapedOnly) = .notDecided ∧
      liveness (settledAt (Status.join Status.escapedOnly Status.releasedOnly))
        = .definitelyNotLive := by
  exact ⟨by decide, by decide⟩

/-- The entitling conjunct is load-bearing in both of its bits. -/
theorem the_entitling_conjunct_is_load_bearing :
    liveness (settledAt (Status.join Status.releasedOnly Status.liveOnly)) = .notDecided ∧
      liveness (settledAt (Status.join Status.releasedOnly Status.containedOnly))
        = .notDecided := by
  exact ⟨by decide, by decide⟩

/-- The status guard is load-bearing: the identical settled binding over a live
    identity does not fire, so a re-acquisition between the release and the
    borrow restores the admission. -/
theorem a_reacquired_identity_is_admitted_again :
    liveness (settledAt Status.releasedOnly) = .definitelyNotLive ∧
      liveness (settledAt Status.liveOnly) = .notDecided := by
  exact ⟨by decide, by decide⟩

/-- The decision is a function of the actual's own binding and that identity's
    own status alone. Two states that agree there agree on the answer, whatever
    else differs — the model's statement that this rule reads no peer. -/
theorem the_decision_reads_only_its_own_actual (f g : Facts)
    (hd : f.denotation = g.denotation)
    (hs : ∀ s, f.status s = g.status s)
    (hu : ∀ s, f.unresolved s = g.unresolved s) :
    liveness f = liveness g := by
  cases hgd : g.denotation with
  | none => simp [liveness, hd, hgd]
  | some d =>
    cases hds : d.settled with
    | none => simp [liveness, hd, hgd, hds]
    | some s => simp [liveness, hd, hgd, hds, hs s, hu s]

/-! # Destroyed by the call that reads it (card BORROW-RESIDUALS)

`liveness` above reads the state **entering** the call, which is the standard
reading of a precondition and stays that way. Card BORROW-LIVENESS recorded the
consequence as the residual `BORROW-LIVENESS-PRESTATE`: a single call that
declares `resource_released` on one argument and `resource_borrowed` on another
is not refused by it, because at the pre-state the identity is still live.

Card BORROW-RESIDUALS takes that residual. The question it asks lives entirely
inside one call: **this call's own declared effects destroy the identity one of
its own `Borrowed` positions reads.** The fact is a *must* fact of the same kind
as `liveness`'s — the projection settled both actuals to one identity — and the
reason it is refused rather than admitted is that the contract vocabulary
carries **no ordering** between a callee's own declared effects. An unproven
call effect is `Unknown`.

`sameCall` is a separate decision with a separate result type on purpose. The
path fact and the call fact are never conflated, exactly as this module's rule
and `QCore.MayAlias`'s are not. It also reads **no status at all**
(`the_same_call_rule_reads_no_status`), which is why it can refuse a call whose
identity `liveness` admits.

Only `released` destroys, mirroring `custodyDefinitelyEnded`'s treatment of the
four bits: `consumed`, `stored` and `removed` move or detach custody without
destroying — §C04.2 says `resource_consumed` "is not a proof of destruction" —
and `out` writes a slot (`BORROW-LIVENESS-OUT`, decided here rather than
asserted).
-/

/-- A declared parameter mode, as far as this decision needs it. These are
    exactly `qcore_schemas::contract::ResourceParameterMode`'s constructors with
    their payloads dropped: nothing here reads a nullability. -/
inductive Mode where
  | borrowed
  | released
  | consumed
  | stored
  | removed
  | owned
  | out
deriving DecidableEq, Repr

/-- Which declared modes destroy the object at that position.

    `only_released_destroys` decides the whole seven-mode domain, so the table
    cannot be widened without that theorem moving. -/
def destroys : Mode → Bool
  | .released => true
  | _ => false

/-- One modeled call: the declared mode and the projection's denotation for each
    argument position, in order. -/
structure Call where
  positions : List (Mode × Option Denotation)

/-- The decision of the same-call rule. -/
inductive SameCall where
  /-- The projection settled this `Borrowed` actual and one of the same call's
      destroying actuals to **one** identity. -/
  | destroyedByThisCall
  /-- Anything else. No claim in either direction. -/
  | notDecided
deriving DecidableEq, Repr

/-- The identities this call's own declared effects destroy, so far as the
    projection settled them. A position whose mode does not destroy, and one
    whose actual the projection did not settle, contributes nothing. -/
def destroyingIdentities (c : Call) : List Site :=
  c.positions.filterMap fun position =>
    if destroys position.1 then position.2.bind Denotation.settled else none

/-- The rule, fail-closed to `notDecided` at every step. -/
def sameCall (borrowed : Option Denotation) (c : Call) : SameCall :=
  match borrowed.bind Denotation.settled with
  | none => .notDecided
  | some s => if s ∈ destroyingIdentities c then .destroyedByThisCall else .notDecided

/-- What a same-call refusal does to the published verdict: the same
    `UnknownAlias`, i.e. the `unresolved` class, and `Unsafe` still wins. -/
def publishSameCall (v : Verdict) (s : SameCall) : Verdict :=
  match s with
  | .notDecided => v
  | .destroyedByThisCall =>
    match v with
    | .violation => .violation
    | _ => .unresolved

/-- One settled binding of site `s`. -/
def settledOn (s : Site) : Denotation := ⟨[s], false, false⟩

/-! ## The positive direction, and its non-vacuity -/

/-- **The headline of card BORROW-RESIDUALS.** One call declaring
    `resource_released` at one position and `resource_borrowed` at another, over
    one settled identity. Measured `Safe` with no diagnostics at that card's
    base. -/
theorem a_call_that_destroys_the_identity_it_reads_is_refused :
    sameCall (some (settledOn 0))
        ⟨[(.released, some (settledOn 0)), (.borrowed, some (settledOn 0))]⟩
      = .destroyedByThisCall := by
  decide

/-- Two *distinct* declared names the projection settled to one identity answer
    the same way, because the decision is over identities and not over names.
    This is not the may-alias relation: nothing here reads a declared tag. -/
theorem two_positions_settled_to_one_identity_are_refused :
    sameCall (some ⟨[3], false, false⟩) ⟨[(.released, some ⟨[3], false, false⟩)]⟩
      = .destroyedByThisCall := by
  decide

/-- Position order is not consulted, so the answer cannot be an artefact of the
    parameter loop's direction. -/
theorem the_decision_does_not_depend_on_position_order :
    sameCall (some (settledOn 0))
        ⟨[(.borrowed, some (settledOn 1)), (.released, some (settledOn 0))]⟩
      = sameCall (some (settledOn 0))
        ⟨[(.released, some (settledOn 0)), (.borrowed, some (settledOn 1))]⟩ := by
  decide

/-- …and in general the decision depends only on *which* identities the call
    destroys, never on where or how often they occur. -/
theorem the_decision_reads_only_the_destroyed_identities (b : Option Denotation) (c c' : Call)
    (h : ∀ s, s ∈ destroyingIdentities c ↔ s ∈ destroyingIdentities c') :
    sameCall b c = sameCall b c' := by
  cases hb : b.bind Denotation.settled with
  | none => simp [sameCall, hb]
  | some s => simp [sameCall, hb, h s]

/-- A refusal withdraws a certificate rather than issuing an allegation. -/
theorem a_same_call_refusal_withdraws_a_certificate :
    publishSameCall .safe .destroyedByThisCall = .unresolved := by
  decide

/-! ## The mode table, decided over its whole domain -/

/-- Every declared mode, so the table below is decided rather than sampled. -/
def allModes : List Mode :=
  [.borrowed, .released, .consumed, .stored, .removed, .owned, .out]

/-- Exactly one mode destroys, over the whole seven-mode domain. This is the
    model's counterpart of `custodyDefinitelyEnded`'s `released` conjunct. -/
theorem only_released_destroys : allModes.filter destroys = [.released] := by
  decide

/-- `resource_consumed` "is not a proof of destruction" (§C04.2), so a
    consumed-and-borrowed pair keeps its existing answer exactly as `escapedOnly`
    does on the path rule. `C02-P-ALIAS-CONSUME` is not closed through this back
    door. -/
theorem a_consumption_in_the_same_call_is_not_a_destruction :
    sameCall (some (settledOn 0)) ⟨[(.consumed, some (settledOn 0))]⟩ = .notDecided := by
  decide

/-- **`BORROW-LIVENESS-OUT`, in the model.** An `Out` actual is the address of a
    slot the callee writes; it destroys nothing, so it never makes a borrow of
    the same identity refusable. -/
theorem an_out_position_never_destroys :
    sameCall (some (settledOn 0)) ⟨[(.out, some (settledOn 0))]⟩ = .notDecided := by
  decide

/-- Detaching from a container and storing beneath an owner leave the object
    alive, exactly as `containedOnly` does on the path rule. -/
theorem a_removed_or_stored_position_never_destroys :
    sameCall (some (settledOn 0)) ⟨[(.removed, some (settledOn 0))]⟩ = .notDecided ∧
      sameCall (some (settledOn 0)) ⟨[(.stored, some (settledOn 0))]⟩ = .notDecided := by
  exact ⟨by decide, by decide⟩

/-! ## Every way the same-call decision fails closed -/

/-- **Every scalar `Borrowed` actual takes this path**, and that matters
    concretely: the one declaration in the pinned corpus carrying a released and
    a borrowed parameter at once has a scalar flag in the borrowed position. -/
theorem an_unbound_borrowed_actual_is_never_refused (c : Call) :
    sameCall none c = .notDecided := by
  rfl

/-- A borrowed actual the projection will not settle is never refused, whatever
    the call destroys — the same fail-closed conjunct `liveness` has. -/
theorem an_unsettled_borrowed_actual_is_never_refused (d : Denotation) (c : Call)
    (h : d.settled = none) : sameCall (some d) c = .notDecided := by
  simp [sameCall, h]

/-- A destroying position the projection will not settle names no identity, so
    it can refuse nothing. -/
theorem an_unsettled_destroying_actual_refuses_nothing (b : Option Denotation) :
    sameCall b ⟨[(.released, some ⟨[0], true, false⟩), (.released, some ⟨[0], false, true⟩),
                 (.released, some ⟨[0, 1], false, false⟩), (.released, some ⟨[], false, false⟩),
                 (.released, none)]⟩ = .notDecided := by
  cases hb : b.bind Denotation.settled with
  | none => simp [sameCall, hb]
  | some s => simp [sameCall, hb, destroyingIdentities, Denotation.settled]

/-- The ordinary well-formed call: a different identity at the destroying
    position. Nothing may move here, or the rule would refuse every call that
    releases anything at all. -/
theorem a_distinct_identity_in_the_same_call_is_never_refused :
    sameCall (some (settledOn 0))
        ⟨[(.released, some (settledOn 1)), (.borrowed, some (settledOn 0))]⟩
      = .notDecided := by
  decide

/-- A call with no destroying position at all — the overwhelming majority — is
    untouched, over every borrowed actual whatsoever. -/
theorem a_call_that_destroys_nothing_is_never_refused (b : Option Denotation) (c : Call)
    (h : ∀ p ∈ c.positions, destroys p.1 = false) : sameCall b c = .notDecided := by
  have hempty : destroyingIdentities c = [] := by
    have : ∀ l : List (Mode × Option Denotation), (∀ p ∈ l, destroys p.1 = false) →
        (l.filterMap fun p => if destroys p.1 then p.2.bind Denotation.settled else none) = [] := by
      intro l
      induction l with
      | nil => intro _; rfl
      | cons p rest ih =>
        intro hall
        have hp : destroys p.1 = false := hall p (by simp)
        have hrest : ∀ q ∈ rest, destroys q.1 = false := fun q hq => hall q (by simp [hq])
        simp [List.filterMap, hp, ih hrest]
    simpa [destroyingIdentities] using this c.positions h
  cases hb : b.bind Denotation.settled with
  | none => simp [sameCall, hb]
  | some s => simp [sameCall, hb, hempty]

/-! ## The converse, and the separation from the path rule -/

/-- Every refusal is backed by a settled borrowed actual whose identity this
    call's own declared effects destroy. The rule invents nothing. -/
theorem a_same_call_refusal_names_the_identity_it_claims (b : Option Denotation) (c : Call)
    (h : sameCall b c = .destroyedByThisCall) :
    ∃ s, b.bind Denotation.settled = some s ∧ s ∈ destroyingIdentities c := by
  cases hb : b.bind Denotation.settled with
  | none => simp [sameCall, hb] at h
  | some s =>
    by_cases hm : s ∈ destroyingIdentities c
    · exact ⟨s, rfl, hm⟩
    · simp [sameCall, hb, hm] at h

/-- The two decisions are independent, demonstrated rather than asserted: an
    identity `liveness` admits — live on every reaching path — is still refused
    when this call destroys it. The same-call rule reads no status bitmap at
    all, which is why its `Call` carries none. -/
theorem the_same_call_rule_reads_no_status :
    liveness (settledAt Status.liveOnly) = .notDecided ∧
      sameCall (some (settledOn 0)) ⟨[(.released, some (settledOn 0))]⟩
        = .destroyedByThisCall := by
  exact ⟨by decide, by decide⟩

/-! ## The published class -/

/-- The same-call rule can never certify either. -/
theorem the_same_call_rule_never_certifies (v : Verdict) (s : SameCall)
    (h : publishSameCall v s = .safe) : v = .safe := by
  cases s with
  | notDecided => simpa [publishSameCall] using h
  | destroyedByThisCall => cases v <;> simp [publishSameCall] at h

/-- …and never manufactures an upstream defect allegation. -/
theorem a_same_call_refusal_is_never_the_violation_class (v : Verdict) (s : SameCall)
    (h : v ≠ .violation) : publishSameCall v s ≠ .violation := by
  cases s with
  | notDecided => simpa [publishSameCall] using h
  | destroyedByThisCall => cases v <;> simp [publishSameCall] at h ⊢

/-- An existing violation is not softened by it. -/
theorem an_existing_violation_survives_the_same_call_rule (s : SameCall) :
    publishSameCall .violation s = .violation := by
  cases s <;> rfl

/-! ## `BORROW-LIVENESS-ASSUMPTION`: neither rule can discharge a premise

The Boolean-field source-effect route publishes
`assumptions_used.borrowed_parameter_is_live: true` on every certificate it
issues — a premise about the analyzed function's own **entry** parameter,
supplied by a caller this projection never sees. Both rules here decide the
actuals an analyzed body passes *onward*, so the two are related only by name.

The subject mismatch is prose, recorded in the card. What is decidable, and
decided here, is the shape of the publication: neither codomain contains a value
that asserts liveness, and neither transformer touches the assumption set. A
refusal-only rule discharges nothing.
-/

/-- A premise a published certificate carries as an assumption. -/
inductive Premise where
  | borrowedParameterIsLive
deriving DecidableEq, Repr

/-- A published result: a verdict together with the premises it still assumes. -/
structure Published where
  verdict : Verdict
  assumed : List Premise

/-- Publishing either decision. -/
def publishBoth (p : Published) (l : Liveness) (s : SameCall) : Published :=
  { p with verdict := publishSameCall (publish p.verdict l) s }

/-- Neither rule ever removes a premise from the assumption set, on any input.
    `borrowed_parameter_is_live` is exactly as assumed after this card as before
    it. -/
theorem no_decision_discharges_a_premise (p : Published) (l : Liveness) (s : SameCall) :
    (publishBoth p l s).assumed = p.assumed := by
  rfl

/-- The reason, stated over the verdict rather than the assumption set: every
    outcome of either rule either changes nothing or withdraws a certificate.
    There is no input at which either rule *adds* anything to a published
    result, so nothing it produces can serve as evidence for a premise. -/
theorem every_outcome_either_refuses_or_changes_nothing (v : Verdict) (l : Liveness)
    (s : SameCall) :
    publishSameCall (publish v l) s = v ∨ publishSameCall (publish v l) s = .unresolved := by
  cases v <;> cases l <;> cases s <;> simp [publish, publishSameCall]

end QCore.BorrowLiveness

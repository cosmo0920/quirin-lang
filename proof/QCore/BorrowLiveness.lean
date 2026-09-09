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

`liveness` answers `definitelyReleased` only under a conjunction of must facts:

* the name is **bound with a settled denotation** — exactly one target, not
  `unknown`, not `mayBeNull` — the same commitment `QCore.MayAlias.settled`
  reads, shared so the two rules cannot drift apart;
* that identity carries **no unresolved effect**;
* its joined status is **exactly released** — no `live`, no `escaped`, no
  `contained` bit anywhere. Because `Status.join` is a pointwise union, the
  absence of every other bit is the statement that *no* reaching path leaves the
  identity in another state (`a_refusal_implies_every_incoming_path_released`).

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

end Status

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
  /-- The projection committed to one identity and that identity is released on
      every path reaching this call. -/
  | definitelyReleased
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
      else if f.status s = Status.releasedOnly then .definitelyReleased
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
  | .definitelyReleased =>
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
    liveness (settledAt Status.releasedOnly) = .definitelyReleased := by
  decide

/-- The refusal withdraws a certificate rather than issuing an allegation. -/
theorem a_refusal_withdraws_a_certificate :
    publish .safe .definitelyReleased = .unresolved := by
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

/-- Released on one branch and consumed on another is likewise undecided: dead
    on neither path is not the same as dead on both. -/
theorem a_release_joined_with_an_escape_is_never_refused :
    liveness (settledAt (Status.join Status.releasedOnly Status.escapedOnly))
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
    unresolved effect and whose joined status is exactly released. The rule
    invents nothing. -/
theorem a_refusal_names_a_settled_released_identity (f : Facts)
    (h : liveness f = .definitelyReleased) :
    ∃ d s, f.denotation = some d ∧ d.settled = some s ∧
      f.unresolved s = false ∧ f.status s = Status.releasedOnly := by
  cases hd : f.denotation with
  | none => simp [liveness, hd] at h
  | some d =>
    cases hs : d.settled with
    | none => simp [liveness, hd, hs] at h
    | some s =>
      by_cases hu : f.unresolved s = true
      · simp [liveness, hd, hs, hu] at h
      · by_cases hr : f.status s = Status.releasedOnly
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
  simp [liveness, settledAt, Denotation.settled, Status.join, Status.releasedOnly, h]

/-- The must half, stated over the join: a refusal at a merge point implies that
    **neither** incoming path left the identity live, escaped or contained, and
    that at least one of them released it. -/
theorem a_refusal_implies_every_incoming_path_released (a b : Status)
    (h : liveness (settledAt (Status.join a b)) = .definitelyReleased) :
    a.live = false ∧ b.live = false ∧ a.escaped = false ∧ b.escaped = false ∧
      a.contained = false ∧ b.contained = false ∧
      (a.released = true ∨ b.released = true) := by
  have hj : Status.join a b = Status.releasedOnly := by
    by_cases hne : Status.join a b = Status.releasedOnly
    · exact hne
    · simp [liveness, settledAt, Denotation.settled, hne] at h
  have := congrArg Status.live hj
  have h2 := congrArg Status.released hj
  have h3 := congrArg Status.escaped hj
  have h4 := congrArg Status.contained hj
  simp [Status.join, Status.releasedOnly] at this h2 h3 h4
  exact ⟨this.1, this.2, h3.1, h3.2, h4.1, h4.2, h2⟩

/-! ## The rule against the existing `Unsafe` bar -/

/-- What `apply_resource_action` tests before reporting `DoubleRelease`: the
    joined status merely *carries* the released bit. That is a **may** fact over
    reaching paths. -/
def doubleReleaseBar (st : Status) : Bool := st.released

/-- Every state this rule refuses would also satisfy the existing `Unsafe`
    kinds' state test. -/
theorem a_refusal_satisfies_the_existing_bar (st : Status)
    (h : liveness (settledAt st) = .definitelyReleased) : doubleReleaseBar st = true := by
  have hst : st = Status.releasedOnly := by
    by_cases hne : st = Status.releasedOnly
    · exact hne
    · simp [liveness, settledAt, Denotation.settled, hne] at h
  simp [doubleReleaseBar, hst, Status.releasedOnly]

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
  | definitelyReleased => cases v <;> simp [publish] at h

/-- The rule never manufactures an upstream defect allegation: a function that
    was not already in the `violation` class does not enter it here. -/
theorem a_refusal_is_never_the_violation_class (v : Verdict) (l : Liveness)
    (h : v ≠ .violation) : publish v l ≠ .violation := by
  cases l with
  | notDecided => simpa [publish] using h
  | definitelyReleased => cases v <;> simp [publish] at h ⊢

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
    liveness (settledAt Status.releasedOnly) = .definitelyReleased ∧
      liveness { denotation := some ⟨[0], false, false⟩
                 status := fun _ => Status.releasedOnly
                 unresolved := fun _ => true } = .notDecided := by
  exact ⟨by decide, by decide⟩

/-- The settled guard is load-bearing: the identical status with an ambiguous
    binding does not fire. -/
theorem the_settled_guard_is_load_bearing :
    liveness (settledAt Status.releasedOnly) = .definitelyReleased ∧
      liveness { denotation := some ⟨[0, 1], false, false⟩
                 status := fun _ => Status.releasedOnly
                 unresolved := fun _ => false } = .notDecided := by
  exact ⟨by decide, by decide⟩

/-- The status guard is load-bearing: the identical settled binding over a live
    identity does not fire, so a re-acquisition between the release and the
    borrow restores the admission. -/
theorem a_reacquired_identity_is_admitted_again :
    liveness (settledAt Status.releasedOnly) = .definitelyReleased ∧
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

end QCore.BorrowLiveness

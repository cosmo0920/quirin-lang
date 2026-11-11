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
# Publication into an object that outlives the call (card `CTWIN-PUBLISH`)

Card `CT-CTWIN` measured, and recorded as the open finding **`CT-CTWIN-PUBLISH`**
(`docs/reports/wave9-ctwin/README.md` §1.1), that a store of an owned pointer
into a file-scope object is invisible to the unrestricted-C resource
projection. The importer lowered `g_published = h;` to an ordinary alias, so the
file-scope object was modelled as one more local name: custody stayed inside the
function, a following release discharged it, and the body certified **`Safe`
with no diagnostic**.

This module models the decision that closes it —
`crates/qcore-c-gateway/src/publication.rs` together with the
`CTWIN-PUBLISH` block of `legacy_c::transfer`. Two halves:

* **which destinations outlive the call** (`Names.outlivesTheCall`), a
  function of four *name sets* read from the preprocessed token stream, and
* **when naming one is a refusal** (`classify`), which is flow-sensitive: only a
  store that actually publishes an identity *this body* holds refuses.

## Why refusal and not an escape

The card's brief proposed modelling the store as an **escape**. That is
fail-closed in one direction only, and this module proves it. Under
`escapeVerdict` a published-then-released body becomes a violation, but a
published-and-**not**-released body becomes `safe`, because the escape
discharges the residual — a promotion, on an accounting no contract states.
Unlike a return or a declared consuming position, a store into a static-storage
object hands custody to no named party.
`the_escape_model_certifies_a_body_the_refusal_withdraws` and
`the_escape_model_promotes_on_some_input` are that argument; `refuseVerdict`
withdraws **both** directions instead.

## What a refusal publishes

An `Unsupported` diagnostic — the `unresolved` class — never the `violation`
class `classify_diagnostics` publishes as provisional `Unsafe`, and the
published identity is additionally marked as carrying an unresolved effect, so a
residual leak that *depends on* the publication is withheld rather than reported
as a validated violation (the `w4-nullable-removal` rule).

## Premises, not theorems

That the Rust readers extract the file-scope declarators, the body's own
declarations, the formal names and the flow-sensitive bindings faithfully from C
source is a **premise**, exactly as in `QCore.BorrowedActual` and
`QCore.BorrowLiveness`. The one modelling fact recorded here rather than proved
is that a name this reader does not recognise at all keeps its pre-existing
modelling (`an_unrecognised_name_is_not_a_destination`), the residual named
`CTWIN-PUBLISH-SCAN`.
-/

namespace QCore.PublicationCustody

/-- An abstract acquisition identity. -/
abbrev Site := Nat

/-- An abstract name, as the projection spells a place. -/
abbrev Name := Nat

/-- The four name sets the destination decision reads.

    Each is exactly one reader's output: `publication::file_scope_objects` over
    the whole translation unit, and the two halves of
    `publication::body_declarations` over this body, plus the definition's
    formal parameters. No qualifier, storage class or declared type appears
    here, which is what `the_destination_decision_reads_only_the_four_name_sets`
    makes precise — and is the model half of the `CT-CTWIN-SYNC` decision that a
    `volatile` qualifier cannot move a resource-projection verdict. -/
structure Names where
  fileScope : Name → Bool
  bodyDeclared : Name → Bool
  formal : Name → Bool
  staticLocal : Name → Bool

namespace Names

/-- Whether the object `n` names outlives the call.

    A body's own `static` local does, whatever file scope says. A file-scope
    object does **unless** this body shadows the name — as a formal or as a
    declaration of its own. Everything else is not recognised and keeps its
    pre-existing modelling. -/
def outlivesTheCall (ns : Names) (n : Name) : Bool :=
  ns.staticLocal n || (ns.fileScope n && !ns.formal n && !ns.bodyDeclared n)

end Names

/-- What one name denotes here, as `AbstractPointer` records it, together with
    the one fact about the *name* the rule reads.

    `scalar` is `true` when this function's own declarations read the source
    name as a declared scalar. A candidate entry custody binds every declared
    formal to a resource site, scalar parameters included, so without that
    exclusion `g_count = n;` would refuse on a modelling artefact rather than on
    a published identity (card `CTWIN-RESIDUALS`). It fails closed: a function
    whose declarations the type reader could not read yields `false` here, and
    the store is refused. -/
structure Denotation where
  targets : List Site
  unknown : Bool
  scalar : Bool := false
deriving DecidableEq, Repr

/-- The instruction shapes whose destination the analyzer writes. -/
inductive Instruction where
  /-- `DEST = SRC;` where `SRC` is a place the projection may bind. -/
  | alias (source : Denotation)
  /-- An acquisition bound directly to `DEST`. -/
  | acquire
  /-- A contracted call whose result is bound to `DEST`. -/
  | contractCall (yieldsCustody : Bool)
  /-- `DEST = NULL;`. -/
  | clear
  /-- A value this function never tracks (`OpaqueValue`). -/
  | opaque
  /-- Card `CTWIN-RESIDUALS`, shape `CTWIN-PUBLISH-OUT`: a contracted call with
      a declared `Out` slot whose actual *is* the destination. The identity is
      created directly into the object, so this is the `acquire` arm seen from
      the callee's side. -/
  | outWrite
deriving Repr

/-- The decision. -/
inductive Decision where
  /-- A tracked identity this body holds is stored into an object that outlives
      the call. The statement is refused and the path poisoned. -/
  | publication
  /-- Anything else. The pre-existing modelling stands, exactly. -/
  | notDecided
deriving DecidableEq, Repr

/-- Whether the value this instruction stores carries an identity *this body*
    holds.

    Card `CTWIN-PUBLISH` excluded a caller-held site here, leaving the
    publication of a **borrowed** pointer admitted as `CTWIN-PUBLISH-BORROWED`.
    Card `CTWIN-RESIDUALS` **withdrew** that exclusion, on a measured ground:
    a body publishing a borrowed formal certified, so W4 published `Borrowed`
    for that formal, and a caller that acquired, called it and released
    certified too — the same hazard, one call level up
    (`publishing_borrowed_launders_the_callers_release`). The faithful
    alternative summary modes are the escape model this module already proves
    promotes (`an_escaping_summary_promotes_the_unreleased_caller`).

    `callerHeld` is therefore no longer read; it stays in the signature because
    `TransferContext` still carries it for other rules, and
    `the_rule_reads_no_provenance` makes the independence explicit. -/
def storesTrackedIdentity (_callerHeld : Site → Bool) : Instruction → Bool
  | .alias d => !d.targets.isEmpty && !d.scalar
  | .acquire => true
  | .contractCall yields => yields
  | .clear => false
  | .opaque => false
  | .outWrite => true

/-- The rule, fail-closed to `notDecided` at every step. -/
def classify (ns : Names) (callerHeld : Site → Bool) (destination : Name)
    (i : Instruction) : Decision :=
  if ns.outlivesTheCall destination && storesTrackedIdentity callerHeld i then
    .publication
  else
    .notDecided

/-- The three verdict classes `classify_diagnostics` publishes. -/
inductive Verdict where
  | safe
  | unresolved
  | violation
deriving DecidableEq, Repr

/-- What a refusal does to the function's published verdict: it contributes an
    `Unsupported`, which is the `unresolved` class, and `Unsafe` still wins over
    `Unknown` in `classify_diagnostics`. -/
def publish (v : Verdict) (d : Decision) : Verdict :=
  match d with
  | .notDecided => v
  | .publication =>
    match v with
    | .violation => .violation
    | _ => .unresolved

/-! ## The two candidate models, and the promotion the escape model causes

The abstract body under comparison is the minimal one the card reproduced:
acquire an identity, store it into an object that outlives the call, and then
either release it or not. `released` is that choice.
-/

/-- How the projection accounts for the published identity at the exit. -/
inductive Residual where
  /-- The obligation is settled: nothing is reported. -/
  | discharged
  /-- The identity is still live and nothing explains where it went. -/
  | leaked
  /-- The projection can no longer say. -/
  | unresolved
deriving DecidableEq, Repr

/-- The **escape** model the card's brief proposed: the store transfers custody
    out of the body, exactly as a return or a declared consuming position does. -/
def escapeResidual (_released : Bool) : Residual :=
  -- The identity left the body at the store, so the exit sees nothing live —
  -- whether or not the body went on to release it. That is exactly what makes
  -- the escape model certify the publish-only body.
  .discharged

/-- The **implemented** model: the publication is an unresolved effect on the
    published identity, so neither direction is claimed. -/
def refuseResidual (_released : Bool) : Residual := .unresolved

/-- The verdict a residual produces on its own, with no other diagnostic. -/
def residualVerdict : Residual → Verdict
  | .discharged => .safe
  | .leaked => .violation
  | .unresolved => .unresolved

/-- The escape model's verdict for the minimal body. A release after the escape
    would additionally raise a `ReleaseAfterEscape` violation; that half is
    modelled explicitly so the comparison is not rigged in the refusal's
    favour. -/
def escapeVerdict (released : Bool) : Verdict :=
  if released then .violation else residualVerdict (escapeResidual released)

/-- The implemented model's verdict for the minimal body: the publication is
    refused in both directions. -/
def refuseVerdict (released : Bool) : Verdict :=
  publish (residualVerdict (refuseResidual released)) .publication

/-! ## Convenience fixtures -/

/-- A name set in which exactly `n` is a file-scope object this body neither
    declares nor takes as a formal. -/
def publishedName (n : Name) : Names :=
  { fileScope := fun m => m == n
    bodyDeclared := fun _ => false
    formal := fun _ => false
    staticLocal := fun _ => false }

/-- A name set in which `n` is this body's own `static` local, and every name
    is also a body declaration (as a `static` local's own declaration is). -/
def staticLocalName (n : Name) : Names :=
  { fileScope := fun _ => false
    bodyDeclared := fun _ => true
    formal := fun _ => false
    staticLocal := fun m => m == n }

/-- A file-scope name this body also takes as a **formal**. -/
def formalShadowedName : Names :=
  { fileScope := fun _ => true
    bodyDeclared := fun _ => false
    formal := fun _ => true
    staticLocal := fun _ => false }

/-- A file-scope name this body **declares** of its own. -/
def locallyShadowedName : Names :=
  { fileScope := fun _ => true
    bodyDeclared := fun _ => true
    formal := fun _ => false
    staticLocal := fun _ => false }

/-- A name no reader recognised at all. -/
def unrecognisedName : Names :=
  { fileScope := fun _ => false
    bodyDeclared := fun _ => false
    formal := fun _ => false
    staticLocal := fun _ => false }

/-- A body-declared, non-static local: the ordinary alias destination. -/
def ordinaryLocalName : Names :=
  { fileScope := fun _ => false
    bodyDeclared := fun _ => true
    formal := fun _ => false
    staticLocal := fun _ => false }

/-- No site is caller-held. -/
def noCallerSites : Site → Bool := fun _ => false

/-- One settled binding onto the identity `s`. -/
def bound (s : Site) : Denotation := { targets := [s], unknown := false }

/-! ## The decision, positively -/

theorem a_store_of_a_body_acquired_identity_into_a_file_scope_object_is_refused :
    classify (publishedName 7) noCallerSites 7 (.alias (bound 0)) = .publication := by
  decide

theorem an_acquisition_bound_to_such_an_object_is_refused :
    classify (publishedName 7) noCallerSites 7 .acquire = .publication := by
  decide

theorem a_custody_yielding_call_bound_to_such_an_object_is_refused :
    classify (publishedName 7) noCallerSites 7 (.contractCall true) = .publication := by
  decide

theorem a_static_local_is_a_destination :
    classify (staticLocalName 3) noCallerSites 3 (.alias (bound 0)) = .publication := by
  decide

theorem a_refusal_withdraws_a_certificate :
    publish .safe (classify (publishedName 7) noCallerSites 7 (.alias (bound 0)))
      = .unresolved := by
  decide

/-- The measured hazard itself: a release *after* the store no longer leaves the
    body certifiable, whatever the release did. -/
theorem a_publication_followed_by_a_release_never_certifies (released : Bool) :
    refuseVerdict released ≠ .safe := by
  cases released <;> decide

/-! ## Why refusal, and not an escape -/

/-- The counterexample, concretely: the body that publishes and does **not**
    release certifies under the escape model. -/
theorem the_escape_model_certifies_a_body_the_refusal_withdraws :
    escapeVerdict false = .safe ∧ refuseVerdict false ≠ .safe := by
  decide

/-- Stated as the promotion it is: some input the implemented model refuses is
    certified by the escape model. -/
theorem the_escape_model_promotes_on_some_input :
    ∃ released, escapeVerdict released = .safe ∧ refuseVerdict released ≠ .safe :=
  ⟨false, the_escape_model_certifies_a_body_the_refusal_withdraws⟩

/-- The implemented model never certifies on **any** of the compared inputs, so
    the withdrawal is uniform rather than a convenient asymmetry. -/
theorem the_refusal_model_certifies_nothing (released : Bool) :
    refuseVerdict released ≠ .safe := by
  cases released <;> decide

/-- And it never manufactures a violation either: a publication is `unresolved`
    on both sides, which is exactly `CT40`'s "not certified; unsupported
    lifetime/publication protocol". -/
theorem a_publication_is_never_a_validated_violation (released : Bool) :
    refuseVerdict released = .unresolved := by
  cases released <;> decide

/-! ## Every way the decision fails closed -/

theorem a_null_store_is_never_refused (ns : Names) (c : Site → Bool) (n : Name) :
    classify ns c n .clear = .notDecided := by
  simp [classify, storesTrackedIdentity]

theorem an_untracked_value_is_never_refused (ns : Names) (c : Site → Bool) (n : Name) :
    classify ns c n .opaque = .notDecided := by
  simp [classify, storesTrackedIdentity]

theorem a_call_with_no_custody_result_is_never_refused
    (ns : Names) (c : Site → Bool) (n : Name) :
    classify ns c n (.contractCall false) = .notDecided := by
  simp [classify, storesTrackedIdentity]

theorem an_unbound_source_is_never_refused (ns : Names) (c : Site → Bool) (n : Name)
    (unknown scalar : Bool) :
    classify ns c n (.alias { targets := [], unknown, scalar }) = .notDecided := by
  simp [classify, storesTrackedIdentity]

/-- Card `CTWIN-PUBLISH`'s `a_caller_held_identity_is_never_refused` and
    `the_caller_site_exclusion_is_load_bearing` stood here. Card
    `CTWIN-RESIDUALS` retired them **by name**: the admission they pinned was
    measured to launder a caller's release, and the replacements are
    `a_caller_held_identity_is_now_refused`,
    `publishing_borrowed_launders_the_callers_release` and
    `the_scalar_exclusion_is_load_bearing`. -/
theorem a_declared_scalar_source_is_never_refused
    (ns : Names) (c : Site → Bool) (n : Name) (s : Site) :
    classify ns c n (.alias { targets := [s], unknown := false, scalar := true })
      = .notDecided := by
  simp [classify, storesTrackedIdentity]

theorem a_formal_is_not_a_destination (s : Site) :
    classify formalShadowedName noCallerSites 3 (.alias (bound s)) = .notDecided := by
  simp [classify, Names.outlivesTheCall, formalShadowedName]

theorem a_shadowed_name_is_not_a_destination (s : Site) :
    classify locallyShadowedName noCallerSites 3 (.alias (bound s)) = .notDecided := by
  simp [classify, Names.outlivesTheCall, locallyShadowedName]

/-- A name no reader recognised keeps its pre-existing modelling. This is the
    recorded residual `CTWIN-PUBLISH-SCAN`, stated rather than hidden. -/
theorem an_unrecognised_name_is_not_a_destination (s : Site) :
    classify unrecognisedName noCallerSites 3 (.alias (bound s)) = .notDecided := by
  simp [classify, Names.outlivesTheCall, unrecognisedName]

/-! ## What the decision reads, and what it does not -/

/-- The destination half is a function of the four name sets alone. No declared
    type, qualifier or storage class beyond `static` duration participates, so
    no qualifier change — `volatile` included — can move it. This is the model
    half of the `CT-CTWIN-SYNC` decision. -/
theorem the_destination_decision_reads_only_the_four_name_sets
    (a b : Names) (n : Name)
    (hf : a.fileScope n = b.fileScope n) (hd : a.bodyDeclared n = b.bodyDeclared n)
    (hp : a.formal n = b.formal n) (hs : a.staticLocal n = b.staticLocal n) :
    a.outlivesTheCall n = b.outlivesTheCall n := by
  simp [Names.outlivesTheCall, hf, hd, hp, hs]

/-- The value half reads no per-site status at all: whatever the projection
    recorded about the identity, publishing it refuses. A dangling pointer
    (released, then published) is therefore refused exactly as a live one is.

    Card `CTWIN-RESIDUALS` **strengthened** this statement by deleting its
    `c s = false` hypothesis: the rule no longer reads the caller-held
    predicate at all, so the conclusion holds for every `c`. See
    `the_rule_reads_no_provenance`. -/
theorem the_rule_reads_no_status (ns : Names) (c : Site → Bool) (n : Name) (s : Site)
    (h : ns.outlivesTheCall n = true) :
    classify ns c n (.alias (bound s)) = .publication := by
  simp [classify, storesTrackedIdentity, bound, h]

/-! ## Monotonicity and the load-bearing conjuncts -/

/-- Recognising more file-scope objects only adds refusals; it never removes
    one. A better scanner is therefore always at least as refusing. -/
theorem a_wider_destination_set_only_adds_refusals
    (a b : Names) (c : Site → Bool) (n : Name) (i : Instruction)
    (hwider : a.outlivesTheCall n = true → b.outlivesTheCall n = true)
    (h : classify a c n i = .publication) : classify b c n i = .publication := by
  unfold classify at *
  split at h
  · next hcond =>
    rw [Bool.and_eq_true] at hcond
    simp [hwider hcond.1, hcond.2]
  · exact absurd h (by simp)

/-- The destination half is itself monotone in the two sets a better scanner can
    only grow: recognising more file-scope objects, or more `static` locals,
    never removes a destination. -/
theorem a_wider_scan_only_adds_destinations
    (a b : Names) (n : Name)
    (hf : a.fileScope n = true → b.fileScope n = true)
    (hs : a.staticLocal n = true → b.staticLocal n = true)
    (hd : b.bodyDeclared n = a.bodyDeclared n) (hp : b.formal n = a.formal n)
    (h : a.outlivesTheCall n = true) : b.outlivesTheCall n = true := by
  unfold Names.outlivesTheCall at h ⊢
  rw [hd, hp]
  revert h hf hs
  generalize a.fileScope n = fa
  generalize a.staticLocal n = sa
  generalize a.formal n = pa
  generalize a.bodyDeclared n = da
  generalize b.fileScope n = fb
  generalize b.staticLocal n = sb
  revert fa sa pa da fb sb
  decide

/-- Without the static-duration premise the rule would refuse an ordinary local
    alias, which is the answer card `C02`'s controls pin as `Safe`. -/
theorem the_static_duration_conjunct_is_load_bearing :
    classify ordinaryLocalName noCallerSites 3 (.alias (bound 0)) = .notDecided
      ∧ storesTrackedIdentity noCallerSites (.alias (bound 0)) = true := by
  decide

/-- Without the tracked-value premise the rule would refuse `g = NULL;` and
    every scalar store into a global. -/
theorem the_tracked_value_conjunct_is_load_bearing :
    (publishedName 7).outlivesTheCall 7 = true
      ∧ classify (publishedName 7) noCallerSites 7 .clear = .notDecided := by
  decide

/-- Without the scalar exclusion the rule would refuse `g_count = n;`, whose
    scalar formal a candidate entry custody binds to a site like any other. Both
    directions on one pair of sources. -/
theorem the_scalar_exclusion_is_load_bearing :
    classify (publishedName 7) noCallerSites 7
        (.alias { targets := [0], unknown := false, scalar := true }) = .notDecided
      ∧ classify (publishedName 7) noCallerSites 7
        (.alias { targets := [0], unknown := false, scalar := false }) = .publication := by
  decide

/-! ## The published verdict class -/

theorem the_rule_never_certifies (v : Verdict) (ns : Names) (c : Site → Bool)
    (n : Name) (i : Instruction)
    (h : classify ns c n i = .publication) : publish v (classify ns c n i) ≠ .safe := by
  rw [h]; cases v <;> simp [publish]

theorem a_refusal_is_never_the_violation_class (ns : Names) (c : Site → Bool)
    (n : Name) (i : Instruction) (h : classify ns c n i = .publication) :
    publish .safe (classify ns c n i) = .unresolved
      ∧ publish .unresolved (classify ns c n i) = .unresolved := by
  rw [h]; exact ⟨rfl, rfl⟩

theorem an_existing_violation_survives (d : Decision) :
    publish .violation d = .violation := by
  cases d <;> rfl

theorem not_firing_changes_nothing (v : Verdict) : publish v .notDecided = v := by
  rfl

/-! ## Non-vacuity -/

theorem the_rule_both_refuses_and_admits :
    (∃ ns c n i, classify ns c n i = .publication)
      ∧ ∃ ns c n i, classify ns c n i = .notDecided :=
  ⟨⟨publishedName 7, noCallerSites, 7, .alias (bound 0), by decide⟩,
   ⟨publishedName 7, noCallerSites, 7, .clear, by decide⟩⟩

/-! ## Card `CTWIN-RESIDUALS`: the publication shapes wave 16 left open

Three of card `CTWIN-PUBLISH`'s four recorded residuals carried a **live false
`Safe`** on the measured base, and each is withdrawn here. The fourth,
`CTWIN-PUBLISH-INDIRECT`, carried none — every indirect shape was already
refused — and what it carried instead was a residual `violation` claim the
direct rule withdraws.
-/

/-! ### `CTWIN-PUBLISH-BORROWED`: the exclusion that laundered a release

The rule now reads no provenance at all, exactly as it reads no per-site status.
-/

/-- The value half is independent of which sites the caller holds. -/
theorem the_rule_reads_no_provenance
    (ns : Names) (c₁ c₂ : Site → Bool) (n : Name) (i : Instruction) :
    classify ns c₁ n i = classify ns c₂ n i := by
  cases i <;> rfl

/-- The direct half of the withdrawal: a caller-held identity published into an
    object that outlives the call is now refused. Replaces
    `a_caller_held_identity_is_never_refused`. -/
theorem a_caller_held_identity_is_now_refused (s : Site) :
    classify (publishedName 7) (fun _ => true) 7 (.alias (bound s)) = .publication := by
  simp [classify, publishedName, Names.outlivesTheCall, storesTrackedIdentity, bound]

/-! #### Why the exclusion had to go, at the summary boundary

A helper's published summary mode is what a caller reads. The two modes a
publishing body could be *given*, plus the no-summary answer a refused body
yields, are compared on the same two callers — one that releases after the call
and one that does not — against the *inlined* body, which card
`CTWIN-PUBLISH` already refuses in both directions. `Consumed` is not modelled
separately: for this comparison it says exactly what `Escaped` says, that
custody left the body at the call.
-/

/-- The custody mode a helper's summary can assert for its formal. -/
inductive SummaryMode where
  /-- The caller's obligations are unchanged across the call. -/
  | borrowed
  /-- Custody left the body at the call. -/
  | escaped
deriving DecidableEq, Repr

/-- What a caller sees. `none` is a helper that published **no** summary, which
    is what a refused body yields: the call is uncontracted. -/
def callerVerdict : Option SummaryMode → Bool → Verdict
  -- No summary: the call is an unmodeled effect, so the caller is unresolved
  -- whatever it does afterwards.
  | none, _ => .unresolved
  -- `borrowed`: the caller keeps custody, so releasing discharges it and not
  -- releasing leaks. This is exactly the pre-card answer.
  | some .borrowed, true => .safe
  | some .borrowed, false => .violation
  -- `escaped`: custody left at the call, so a later release is a violation and
  -- *not* releasing certifies — the escape model's promotion, one level up.
  | some .escaped, true => .violation
  | some .escaped, false => .safe

/-- The summary a publishing helper yields under each candidate rule. -/
def publishingSummary (admitted : Bool) : Option SummaryMode :=
  if admitted then some .borrowed else none

/-- The measured launder: under card `CTWIN-PUBLISH`'s admission the helper
    published `borrowed`, and the caller that acquired, called it and released
    **certified** — while the same body inlined is refused. -/
theorem publishing_borrowed_launders_the_callers_release :
    callerVerdict (publishingSummary true) true = .safe
      ∧ refuseVerdict true ≠ .safe := by
  decide

/-- And the faithful alternative promotes in the other direction, exactly as the
    escape model does inside one body: the caller that does **not** release
    certifies. So neither mode is available and the body must be refused. -/
theorem an_escaping_summary_promotes_the_unreleased_caller :
    callerVerdict (some .escaped) false = .safe
      ∧ refuseVerdict false ≠ .safe := by
  decide

/-- The implemented answer: a refused helper publishes no summary, so **neither**
    caller certifies — the composed body is refused exactly as the inlined one
    is. -/
theorem a_refused_helper_certifies_no_caller (releases : Bool) :
    callerVerdict (publishingSummary false) releases ≠ .safe := by
  cases releases <;> decide

/-- Non-vacuity for the composition: a helper that does not publish still lets
    its caller certify, so the withdrawal is not a blanket refusal of helpers. -/
theorem a_non_publishing_helper_still_certifies_its_caller :
    callerVerdict (some .borrowed) true = .safe := by
  decide

/-! ### `CTWIN-PUBLISH-OUT`: the slot that acquires into the object -/

theorem an_out_slot_naming_such_an_object_is_refused :
    classify (publishedName 7) noCallerSites 7 .outWrite = .publication := by
  decide

theorem an_out_slot_naming_a_static_local_is_refused :
    classify (staticLocalName 3) noCallerSites 3 .outWrite = .publication := by
  decide

/-- The control: an `Out` slot naming an ordinary local is untouched, so the
    destination half of the out rule is load-bearing too. -/
theorem an_out_slot_naming_an_ordinary_local_is_not_refused :
    classify ordinaryLocalName noCallerSites 3 .outWrite = .notDecided := by
  decide

/-- Refused *before* the call's effects apply, so no site exists to report: the
    verdict is `unresolved` in both directions, never a violation. This is the
    same two-direction withdrawal the direct acquisition arm performs. -/
theorem an_out_publication_is_never_a_validated_violation :
    publish .safe (classify (publishedName 7) noCallerSites 7 .outWrite) = .unresolved
      ∧ publish .unresolved (classify (publishedName 7) noCallerSites 7 .outWrite)
        = .unresolved := by
  decide

/-! ### `CTWIN-PUBLISH-INDIRECT`: the residual claim, withdrawn

A store into a continued access on such an object — `g.f`, `g->f`, `g[i]` — is
already an unmodeled instruction, so its verdict was never `safe` and nothing
here can make one. What it kept was a `violation` residual: the projection
claimed the identity was never released, from a body that handed it to an
unmodeled publication protocol.
-/

/-- What an unmodeled statement does on its own: it poisons the name bindings
    and contributes an `Unsupported`, but leaves the site's residual standing. -/
def unmodeledVerdict (residual : Residual) : Verdict :=
  match residualVerdict residual with
  | .violation => .violation
  | _ => .unresolved

/-- The indirect record's effect: the identities the statement's own operands
    denote carry an unresolved effect from here on. -/
def withdrawIndirect (base : Names) (n : Name) (bound : Bool) (residual : Residual) :
    Residual :=
  if base.outlivesTheCall n && bound then .unresolved else residual

theorem an_indirect_publication_withdraws_its_leak_claim :
    unmodeledVerdict (withdrawIndirect (publishedName 7) 7 true .leaked) = .unresolved := by
  decide

/-- The control: the same statement with a **local** base keeps its claim, so
    the destination half of the indirect rule is load-bearing. -/
theorem an_indirect_store_into_a_local_keeps_its_claim :
    unmodeledVerdict (withdrawIndirect ordinaryLocalName 3 true .leaked) = .violation := by
  decide

/-- And a statement that names no tracked identity withdraws nothing. -/
theorem an_indirect_store_of_an_untracked_value_withdraws_nothing
    (ns : Names) (n : Name) (residual : Residual) :
    withdrawIndirect ns n false residual = residual := by
  simp [withdrawIndirect]

/-- The record can only ever **weaken** a claim: it never turns a non-certifying
    answer into a certifying one, on any input. -/
theorem the_indirect_record_never_certifies
    (ns : Names) (n : Name) (bound : Bool) (residual : Residual) :
    unmodeledVerdict (withdrawIndirect ns n bound residual) ≠ .safe := by
  cases residual <;> cases bound <;>
    simp [unmodeledVerdict, withdrawIndirect, residualVerdict] <;>
    cases ns.outlivesTheCall n <;> simp

/-! ### `CTWIN-PUBLISH-SCAN`: the parenthesised specifier, and monotonicity

Card `CTWIN-PUBLISH` recorded that a declarator group carrying `(` contributes
nothing, so that no prototype's parameter name enters the destination set. That
skip also swallowed `_Atomic ( type-name ) g;` — CT40's literal atomic-pointer
spelling — which therefore published without a word.

That the reader splits a group's *specifier* parenthesis from its *declarator*
parenthesis faithfully is a premise here, exactly as every other reader's
extraction is. What is proved is that dropping the specifier is **monotone**:
the extension can only add names, and adding names only adds refusals.
-/

/-- One declarator group, as the two facts the recognition reads. -/
structure Group where
  /-- The group carries a `_Atomic ( type-name )` **specifier** parenthesis. -/
  specifierParen : Bool
  /-- The group carries a **declarator** parenthesis — a function declarator, a
      function-pointer declarator, or a K&R parameter list. -/
  declaratorParen : Bool
  /-- The trailing identifier the group declares, when there is one. -/
  name : Option Name
deriving Repr

/-- Card `CTWIN-PUBLISH`'s reader: any `(` at all skips the group whole. -/
def recogniseBefore (g : Group) : Option Name :=
  if g.specifierParen || g.declaratorParen then none else g.name

/-- Card `CTWIN-RESIDUALS`' reader: the specifier parenthesis is dropped first,
    so only a **declarator** parenthesis skips the group. -/
def recogniseAfter (g : Group) : Option Name :=
  if g.declaratorParen then none else g.name

/-- Monotone: every name the old reader recognised, the new one recognises.
    A better scanner is strictly more refusing, never less. -/
theorem the_specifier_drop_only_adds_names (g : Group) (n : Name)
    (h : recogniseBefore g = some n) : recogniseAfter g = some n := by
  unfold recogniseBefore recogniseAfter at *
  cases hs : g.specifierParen <;> cases hd : g.declaratorParen <;>
    simp [hs, hd] at h ⊢ <;> exact h

/-- The drop never reaches a **declarator** parenthesis, so no prototype's
    parameter name and no function-pointer object can enter the set through it.
    Card `CTWIN-PUBLISH`'s reason for the skip is preserved exactly. -/
theorem a_declarator_parenthesis_still_skips_the_group (g : Group)
    (h : g.declaratorParen = true) : recogniseAfter g = none := by
  simp [recogniseAfter, h]

/-- Non-vacuity: there is a group the new reader recognises and the old one did
    not — `_Atomic ( void * ) g_published ;`, measured `Safe` with no diagnostic
    before this card. -/
theorem the_drop_recognises_a_name_the_old_reader_hid :
    recogniseBefore { specifierParen := true, declaratorParen := false, name := some 7 }
        = none
      ∧ recogniseAfter { specifierParen := true, declaratorParen := false, name := some 7 }
        = some 7 := by
  decide

/-- And the two spellings of one object now answer alike, which is what makes
    the answer independent of how the source spelled the qualifier. -/
theorem both_spellings_of_one_object_are_recognised_alike (n : Name) :
    recogniseAfter { specifierParen := true, declaratorParen := false, name := some n }
      = recogniseAfter { specifierParen := false, declaratorParen := false, name := some n } := by
  simp [recogniseAfter]

/-! ### The whole card, in one sentence each direction -/

/-- Every shape this card landed refuses, and none of them can certify. -/
theorem no_landed_shape_certifies (v : Verdict) (i : Instruction)
    (h : classify (publishedName 7) noCallerSites 7 i = .publication) :
    publish v (classify (publishedName 7) noCallerSites 7 i) ≠ .safe :=
  the_rule_never_certifies v _ _ _ i h

/-- Non-vacuity for the extensions: the widened rule still admits. -/
theorem the_widened_rule_both_refuses_and_admits :
    classify (publishedName 7) (fun _ => true) 7 (.alias (bound 0)) = .publication
      ∧ classify (publishedName 7) noCallerSites 7 .outWrite = .publication
      ∧ classify (publishedName 7) noCallerSites 7 .clear = .notDecided
      ∧ classify ordinaryLocalName noCallerSites 3 .outWrite = .notDecided := by
  decide

end QCore.PublicationCustody

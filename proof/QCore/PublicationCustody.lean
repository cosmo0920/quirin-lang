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

/-- What one name denotes here, as `AbstractPointer` records it. -/
structure Denotation where
  targets : List Site
  unknown : Bool
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

    A caller-held site is deliberately excluded: publishing a **borrowed**
    pointer is a lifetime question this projection never claimed to decide, and
    refusing it would newly refuse bodies sound in the acquire/release
    accounting the projection does decide (`CTWIN-PUBLISH-BORROWED`). -/
def storesTrackedIdentity (callerHeld : Site → Bool) : Instruction → Bool
  | .alias d => d.targets.any fun s => !callerHeld s
  | .acquire => true
  | .contractCall yields => yields
  | .clear => false
  | .opaque => false

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
    (unknown : Bool) :
    classify ns c n (.alias { targets := [], unknown }) = .notDecided := by
  simp [classify, storesTrackedIdentity]

/-- Publishing a **borrowed** identity — every target caller-held — is not
    refused. `CTWIN-PUBLISH-BORROWED`. -/
theorem a_caller_held_identity_is_never_refused (ns : Names) (n : Name) (s : Site) :
    classify ns (fun _ => true) n (.alias (bound s)) = .notDecided := by
  simp [classify, storesTrackedIdentity, bound]

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
    (released, then published) is therefore refused exactly as a live one is. -/
theorem the_rule_reads_no_status (ns : Names) (c : Site → Bool) (n : Name) (s : Site)
    (h : ns.outlivesTheCall n = true) (hc : c s = false) :
    classify ns c n (.alias (bound s)) = .publication := by
  simp [classify, storesTrackedIdentity, bound, h, hc]

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

/-- Without the caller-site exclusion the rule would refuse the publication of a
    borrowed formal, which is `Safe` and stays `Safe`. -/
theorem the_caller_site_exclusion_is_load_bearing :
    classify (publishedName 7) (fun _ => true) 7 (.alias (bound 0)) = .notDecided
      ∧ classify (publishedName 7) noCallerSites 7 (.alias (bound 0)) = .publication := by
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

end QCore.PublicationCustody

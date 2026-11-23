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

import QCore.OpensslBioReaders
import QCore.OpensslFreeTeardownSlots

/-!
# A slot invocation through a released parameter

Card **OPENSSL-RELEASED-PARAMETER-SLOTS** (contract specification C04.19.3
amended in place, gateway rule 12n; USER DECISION 2026-11-23: "if yes,
EVP_MD_CTX_free drops its premise, and PKCS12_free keeps it only for the CRL
item"). Beside `QCore.OpensslBioReaders` (one opaque slot's decision,
`decideSlot`) and `QCore.OpensslFreeTeardownSlots` (the free audit).

Before this card a slot invocation read only a `resource_borrowed`
parameter, so `PKCS12_free` and `EVP_MD_CTX_free`, which release their
argument, stated none of their teardown slots. Modelled as the Rust code
decides it:

* **Admission** (`contract_callback.rs`, `contract_loader.rs`). A slot
  invocation's object may also be a `resource_released` parameter, and a
  declaration carrying one states nothing beside its custody contract and
  premises but slot invocations (`admitted`). The admission only grows, and
  only by such declarations (`the_admission_only_grows`,
  `the_admission_grows_only_by_released_slot_invocations_alone`); a
  consumed, owned or other object still rejects
  (`another_object_mode_still_rejects`), and so does a released slot object
  beside any other clause (`a_released_slot_beside_another_clause_rejects`).
* **The reading at a call** (`legacy_retained_by.rs`, `legacy_transfer.rs`).
  The transfer applies the custody contract, the release included, before
  the callback route (`transfer`). So what stands is read from the object
  before the release: each slot's decision over the run's closed set, and a
  candidate's requirement that the object be held (folded into the call's
  custody mode, a non-null requirement strengthening a nullable release).
  A candidate effect the callback route would apply after the release (a
  member-path state, a deferred member reading) refuses
  (`a_member_effect_beside_a_release_refuses`), because the route would read
  a released object (`the_route_after_the_release_reads_a_released_object`).
  The outcome reads only the object before the release
  (`the_reading_is_the_object_before_the_release`); a use after the release
  is the custody contract's, unchanged (`a_use_after_the_release_refuses`).
* **The two frees** (`providers/quirin-provider-openssl`). `EVP_MD_CTX_free`
  states twelve teardown slots and leaves nothing; `PKCS12_free` states the
  key's nine and leaves a CRL's method. Each premise covers exactly what
  remains (`the_premises_cover_exactly_what_remains`): the free statement's
  uncovered functions are `PKCS12_free`'s CRL free alone and none of
  `EVP_MD_CTX_free`'s.

## What this is not

Which functions the frees run (read from the OpenSSL 3.5.5 sources: the
card's report), that the stated keys are members of `#teardown` of an object
in the call's `#freed`, and that the Rust code computes these functions are
premises pinned by the crate's checker tests and the gateway tests
(`openssl_undeclared_handshake_callbacks`). The premises are metadata (SP1).
-/

namespace QCore.OpensslReleasedSlots

open QCore.CallbackLiteralPremise (Outcome)
open QCore.OpensslBioReaders (SlotFacts decideSlot combine combine_applied decideSlot_applied)
open QCore.OpensslFreeTeardownSlots (Entry Reading decideReading audited keySlots handshakeSlots
  a_left_item_keeps_the_premise nothing_left_drops_the_premise)

/-! ## Admission -/

/-- A parameter's custody mode, as the slot reading distinguishes it. -/
inductive Mode where
  | borrowed
  | released
  | consumed
  | owned
  | other
  deriving DecidableEq, Repr

/-- A clause beside the custody contract and the premises. -/
inductive Clause where
  | slotInvoked (object : Mode)
  | memberInvoked
  | invoked
  | registered
  | retention
  | literalArgument
  | otherRetaining
  deriving DecidableEq, Repr

/-- Whether a clause is a slot invocation. -/
def Clause.isSlot : Clause → Bool
  | .slotInvoked _ => true
  | _ => false

/-- The object modes a slot invocation reads before this card (borrowed). -/
def objectOld : Clause → Bool
  | .slotInvoked m => m == .borrowed
  | _ => true

/-- Since this card: borrowed, or released. -/
def objectNew : Clause → Bool
  | .slotInvoked m => m == .borrowed || m == .released
  | _ => true

/-- Whether a declaration states a slot invocation through a released
parameter. -/
def releasesASlotObject (cs : List Clause) : Bool := cs.any (· == .slotInvoked .released)

/-- C04.19.3 before this card (as far as the slot object goes). -/
def admittedOld (cs : List Clause) : Bool := cs.all objectOld

/-- C04.19.3 amended: a released slot object, alone with slot invocations. -/
def admitted (cs : List Clause) : Bool :=
  cs.all objectNew && (!releasesASlotObject cs || cs.all Clause.isSlot)

theorem objectOld_released_false : objectOld (.slotInvoked .released) = false := by decide

theorem the_admission_only_grows {cs : List Clause} (h : admittedOld cs = true) :
    admitted cs = true := by
  unfold admittedOld at h
  have hall := List.all_eq_true.mp h
  have hnew : cs.all objectNew = true := by
    apply List.all_eq_true.mpr
    intro c hc
    have := hall c hc
    cases c with
    | slotInvoked m => cases m <;> simp_all [objectOld, objectNew]
    | _ => rfl
  have hrel : releasesASlotObject cs = false := by
    apply Bool.eq_false_iff.mpr
    intro ha
    obtain ⟨c, hc, heq⟩ := List.any_eq_true.mp ha
    have heq' : c = .slotInvoked .released := by simpa using heq
    subst heq'
    have := hall _ hc
    simp [objectOld] at this
  simp [admitted, hnew, hrel]

theorem the_admission_grows_only_by_released_slot_invocations_alone {cs : List Clause}
    (h : admitted cs = true) (hold : admittedOld cs = false) :
    Clause.slotInvoked .released ∈ cs ∧ ∀ c ∈ cs, c.isSlot = true := by
  unfold admitted at h
  simp only [Bool.and_eq_true, Bool.or_eq_true, Bool.not_eq_true'] at h
  obtain ⟨hnew, hrel⟩ := h
  have hmem : Clause.slotInvoked .released ∈ cs := by
    unfold admittedOld at hold
    obtain ⟨c, hc, hbad⟩ := List.all_eq_false.mp hold
    have hn := List.all_eq_true.mp hnew c hc
    cases c with
    | slotInvoked m => cases m <;> simp_all [objectOld, objectNew]
    | _ => simp [objectOld] at hbad
  refine ⟨hmem, ?_⟩
  rcases hrel with hr | hs
  · have : releasesASlotObject cs = true := List.any_eq_true.mpr ⟨_, hmem, by simp⟩
    simp [this] at hr
  · exact List.all_eq_true.mp hs

theorem another_object_mode_still_rejects {cs : List Clause} {m : Mode}
    (hc : Clause.slotInvoked m ∈ cs) (hb : m ≠ .borrowed) (hr : m ≠ .released) :
    admitted cs = false := by
  have : cs.all objectNew = false := by
    apply Bool.eq_false_iff.mpr
    intro ha
    have := List.all_eq_true.mp ha _ hc
    cases m <;> simp_all [objectNew]
  simp [admitted, this]

theorem a_released_slot_beside_another_clause_rejects {cs : List Clause} {c : Clause}
    (hr : Clause.slotInvoked .released ∈ cs) (hc : c ∈ cs) (hk : c.isSlot = false) :
    admitted cs = false := by
  have hrel : releasesASlotObject cs = true := List.any_eq_true.mpr ⟨_, hr, by simp⟩
  have hs : cs.all Clause.isSlot = false := by
    apply Bool.eq_false_iff.mpr
    intro ha
    have := List.all_eq_true.mp ha _ hc
    simp [hk] at this
  simp [admitted, hrel, hs]

/-! ## The reading at a call -/

/-- The released object before the call: whether the caller holds it (or it
is null), whether it may be null, and whether it is freed. -/
structure Object where
  held : Bool
  maybeNull : Bool
  freed : Bool
  deriving DecidableEq, Repr

/-- The release: the caller's custody ends and the object is freed. -/
def release (o : Object) : Object := { o with held := false, freed := true }

/-- What a stated slot's candidates do to the released object beside the
static decision: nothing, require it held (`resource_owned(n)`, `nullable`
whether a null object meets it), or a member effect (a member-path state, a
member invocation deferred to its constructed state). -/
inductive Effect where
  | none
  | requiresHeld (nullable : Bool)
  | member
  deriving DecidableEq, Repr

/-- The release's nullability after the candidates' requirements are folded
into it (`legacy_retained_by.rs`): a non-null requirement strengthens it. -/
def folded (nullable : Bool) (effects : List Effect) : Bool :=
  nullable && effects.all fun e => match e with
    | .requiresHeld n => n
    | _ => true

/-- The custody contract's precondition on the released position, read
before its effects: the object held, and non-null unless the release is
nullable. -/
def precondition (pre : Object) (nullable : Bool) : Bool :=
  pre.held && !pre.freed && (nullable || !pre.maybeNull)

/-- The outcome at a call stating slot invocations through a released
parameter: `pre` the object before the call, `nullable` the declared
release's nullability, `slots` each stated slot's facts, `effects` the
candidates' effects on the object. -/
def releasedOutcome (pre : Object) (nullable : Bool) (slots : List SlotFacts)
    (effects : List Effect) : Outcome :=
  if effects.any (· == .member) then .refused
  else if !precondition pre (folded nullable effects) then .refused
  else combine (slots.map decideSlot)

/-- The transfer: the custody contract (with the release) first, then the
callback route; the outcome is decided on the object before the release and
the object after is the released one. -/
def transfer (pre : Object) (nullable : Bool) (slots : List SlotFacts) (effects : List Effect) :
    Outcome × Object :=
  (releasedOutcome pre nullable slots effects, release pre)

theorem the_reading_is_the_object_before_the_release (pre : Object) (nullable : Bool)
    (slots : List SlotFacts) (effects : List Effect) :
    (transfer pre nullable slots effects).1 = releasedOutcome pre nullable slots effects ∧
      (transfer pre nullable slots effects).2.freed = true :=
  ⟨rfl, rfl⟩

/-- What the callback route reads, after the transfer's release, is a freed
object the caller no longer holds: why a member effect must refuse. -/
theorem the_route_after_the_release_reads_a_released_object (pre : Object) :
    (release pre).freed = true ∧ (release pre).held = false := ⟨rfl, rfl⟩

theorem a_member_effect_beside_a_release_refuses {pre : Object} {nullable : Bool}
    {slots : List SlotFacts} {effects : List Effect} (h : Effect.member ∈ effects) :
    releasedOutcome pre nullable slots effects = .refused := by
  have : effects.any (· == .member) = true := List.any_eq_true.mpr ⟨_, h, by simp⟩
  simp [releasedOutcome, this]

/-- A second call on the released object (a use after the release) fails its
precondition. -/
theorem a_use_after_the_release_refuses (pre : Object) (nullable : Bool)
    (slots : List SlotFacts) (effects : List Effect) :
    releasedOutcome (release pre) nullable slots effects = .refused := by
  unfold releasedOutcome
  split
  · rfl
  · simp [precondition, release]

theorem a_non_null_requirement_strengthens_the_release {pre : Object} {slots : List SlotFacts}
    {effects : List Effect} (hn : pre.maybeNull = true)
    (h : Effect.requiresHeld false ∈ effects) (nullable : Bool) :
    releasedOutcome pre nullable slots effects = .refused := by
  have hf : folded nullable effects = false := by
    have : (effects.all fun e => match e with
        | .requiresHeld n => n
        | _ => true) = false := by
      apply Bool.eq_false_iff.mpr
      intro ha
      have := List.all_eq_true.mp ha _ h
      simp at this
    simp [folded, this]
  unfold releasedOutcome
  split
  · rfl
  · simp [precondition, hf, hn]

theorem applied_only_over_closed_identity_slots {pre : Object} {nullable : Bool}
    {slots : List SlotFacts} {effects : List Effect}
    (h : releasedOutcome pre nullable slots effects = .applied) :
    (∀ e ∈ effects, e ≠ .member) ∧ pre.held = true ∧ pre.freed = false ∧
      ∀ s ∈ slots, ∃ cs, s = .closed cs ∧ ∀ k ∈ cs, k = .identity := by
  unfold releasedOutcome at h
  cases hm : effects.any (· == .member) with
  | true => simp [hm] at h
  | false =>
      cases hp : precondition pre (folded nullable effects) with
      | false => simp [hm, hp] at h
      | true =>
          simp only [hm, hp, Bool.not_true, Bool.false_eq_true, ↓reduceIte] at h
          refine ⟨?_, ?_, ?_, ?_⟩
          · intro e he heq
            subst heq
            have : effects.any (· == .member) = true := List.any_eq_true.mpr ⟨_, he, by simp⟩
            simp [this] at hm
          · simp only [precondition, Bool.and_eq_true] at hp
            exact hp.1.1
          · simp only [precondition, Bool.and_eq_true, Bool.not_eq_true'] at hp
            exact hp.1.2
          · have hall := combine_applied h
            intro s hs
            exact decideSlot_applied (hall (decideSlot s) (List.mem_map.mpr ⟨s, hs, rfl⟩))

/-- Nothing reads the object after the release: an applied call carries no
candidate effect for the callback route, which runs after the release. -/
theorem nothing_reads_the_object_after_the_release {pre : Object} {nullable : Bool}
    {slots : List SlotFacts} {effects : List Effect}
    (h : releasedOutcome pre nullable slots effects = .applied) : Effect.member ∉ effects :=
  fun hm => (applied_only_over_closed_identity_slots h).1 _ hm rfl

/-! ## The two frees -/

/-- A CRL's `X509_CRL_METHOD` free, which no declaration can register
(OPENSSL-SLOT-REGISTRATION-INTO-RESULT). -/
def crlFree : String := "x509_crl_method_st.crl_free"

/-- The digest context's teardown: the key's nine and the digest, cipher
and public key method cleanups (`DIGEST_CONTEXT_TEARDOWN`). -/
def digestTeardown : List String := handshakeSlots

/-- A PKCS#12 object's teardown: its keys' nine and a CRL's free. -/
def pkcs12Teardown : List String := keySlots ++ [crlFree]

/-- The teardown functions the free statement `#teardown(#o) \ (#stated |
#settable)` still covers: those neither stated nor settable. -/
def uncovered (teardown stated settable : List String) : List String :=
  teardown.filter fun f => !stated.contains f && !settable.contains f

/-- The settable functions: every slot a declared setter registers
(`TEARDOWN_SLOTS`); a CRL's free is not among them. -/
def settable : List String := QCore.OpensslFreeTeardownSlots.teardownSlots

theorem the_premises_cover_exactly_what_remains :
    uncovered pkcs12Teardown keySlots settable = [crlFree] ∧
      uncovered digestTeardown handshakeSlots settable = [] ∧
      -- without the stated slots, the free statement would cover nothing
      -- of PKCS12_free's keys either (they are settable): stating them is
      -- what covers them
      uncovered pkcs12Teardown [] settable = [crlFree] := by
  decide

/-- The two frees as this card lands them, and their audit readings. -/
def landed (f : String) (free : Bool) (stated : List String) : Entry :=
  ⟨f, true, free, false, false, true, true, ["openssl_ex_data_callbacks"], stated⟩

def frees : List (String × Reading) :=
  [("EVP_MD_CTX_free", .runs digestTeardown 0), ("PKCS12_free", .runs keySlots 1)]

theorem vectors_as_landed :
    audited (landed "EVP_MD_CTX_free" false digestTeardown) frees = true ∧
      audited (landed "PKCS12_free" true keySlots) frees = true := by
  decide

theorem vectors_drifted_refuse :
    -- EVP_MD_CTX_free keeping the premise, or missing its digest cleanup
    audited (landed "EVP_MD_CTX_free" true digestTeardown) frees = false ∧
      audited (landed "EVP_MD_CTX_free" false keySlots) frees = false ∧
      -- PKCS12_free dropping the premise, or stating no slot
      audited (landed "PKCS12_free" false keySlots) frees = false ∧
      audited (landed "PKCS12_free" true []) frees = false ∧
      -- either naming the released statement
      audited ⟨"PKCS12_free", true, false, true, false, true, true, [], keySlots⟩ frees =
        false := by
  decide

/-- The user decision's two consequences: `EVP_MD_CTX_free` drops the premise
and `PKCS12_free` keeps it, for what its audit leaves. -/
theorem the_decision_holds {e d : Entry}
    (he : decideReading e (.runs keySlots 1) = true) (hh : e.handshake = false)
    (hd : decideReading d (.runs digestTeardown 0) = true) (hdh : d.handshake = false) :
    e.freeStmt = true ∧ d.freeStmt = false :=
  ⟨a_left_item_keeps_the_premise he (by decide) hh, (nothing_left_drops_the_premise hd hdh).1⟩

/-! ## Vectors of the reading -/

def live : Object := { held := true, maybeNull := false, freed := false }
def maybeNull : Object := { held := true, maybeNull := true, freed := false }

theorem vectors_reading :
    releasedOutcome live true [.closed [], .closed [.identity]] [] = .applied ∧
      releasedOutcome maybeNull true [.closed [.identity]] [] = .applied ∧
      releasedOutcome live true [.closed [.identity]] [.requiresHeld false] = .applied ∧
      releasedOutcome maybeNull true [.closed [.identity]] [.requiresHeld false] = .refused ∧
      releasedOutcome live true [.closed [.identity]] [.member] = .refused ∧
      releasedOutcome live true [.closed [.effectful]] [] = .refused ∧
      releasedOutcome live true [.unregistered] [] = .refused ∧
      releasedOutcome live true [.noRun] [] = .refused ∧
      releasedOutcome (release live) true [.closed []] [] = .refused := by
  decide

end QCore.OpensslReleasedSlots

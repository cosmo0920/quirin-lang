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
# Declared literal-argument premises, and the hash-table publication they bound

Card **HASH-CONTRACTS**. Two models, and the sentence that ties them.

## 1. The premise mechanism

A resource-contract v1 declaration names one custody mode per parameter and says
nothing about argument *values*. A **declared literal-argument premise**
restricts a declaration to the call sites whose argument at a named parameter is
exactly a named integer literal; everywhere else the contract is *not applied*,
and the call is exactly as uncontracted as a call to a function nothing
declares. The theorems below are the two directions that make this safe to add:

* it can only ever **withdraw** a declaration (`applied` returns either the
  declared contract unchanged, or nothing — never a different contract), and
* it withdraws on every ground the implementation refuses on: poison, a
  mismatched actual, and a **missing** actual.

## 2. The upstream body the premise bounds

`flb_hash_table_add` reaches its value through `entry_set_value`, which has two
arms. At a positive size the table allocates its own buffer and copies into it;
otherwise it stores the caller's pointer and records `val_size = -1`.
`flb_hash_table_entry_free` frees `entry->val` exactly when the recorded size is
positive, so a referenced value is never freed by the table.

What is proved here is the consequence for the *declaration*: over the whole v1
parameter vocabulary, `borrowed` is the only mode whose caller-side effect is the
measured one, and the `stored` reading is not even well formed against the
measured signature, because §C04.2 requires its owner parameter to be declared
`owned` and every parameter of this signature is borrowed.

## What this is not

This is a model of a **decision** and of a **declared reading** of a body that
was read by hand. That the upstream arms are as transcribed, that a declared
`borrowed` mode really borrows, and C's object model itself are premises
supplied from outside. Retention is modelled and proved *orthogonal* to custody
precisely so that nothing here can be read as a claim that a retained pointer
stays valid.
-/

namespace QCore.LiteralArgumentPremise

/-- A function name. Names are opaque, so they are modelled as indices. -/
abbrev Name := Nat

/-- One actual argument, as the token codes a C lexer produces for it. Code `0`
is the token `0`; every other code is some other token. -/
abbrev Tokens := List Nat

/-- The resource-contract v1 parameter vocabulary (§C04.2). -/
inductive Mode where
  | borrowed
  | owned
  | consumed
  | released
  | out
  | stored (owner : Nat)
  | removed (owner : Nat)
  deriving DecidableEq, Repr

/-- A declared contract: one mode per parameter. -/
structure Contract where
  parameters : List Mode
  deriving DecidableEq, Repr

/-- A declared literal-argument premise. -/
structure Premise where
  index : Nat
  literal : Tokens
  deriving DecidableEq, Repr

/-- One call site. -/
structure Call where
  callee : Name
  arguments : List Tokens
  deriving DecidableEq, Repr

/-- A contract environment together with the premises its declared text carries.
`poisoned` records a malformed directive. -/
structure Environment where
  contracts : Name → Option Contract
  premises : Name → List Premise
  poisoned : Bool

/-- A premise is satisfied only by an actual that is *present* and *exactly* the
declared literal. -/
def satisfies (arguments : List Tokens) (p : Premise) : Bool :=
  match arguments[p.index]? with
  | some actual => actual == p.literal
  | none => false

/-- Whether the declaration is withdrawn at this call site. -/
def unmet (e : Environment) (c : Call) : Bool :=
  e.poisoned || (e.premises c.callee).any (fun p => !satisfies c.arguments p)

/-- The contract actually applied at a call site. -/
def applied (e : Environment) (c : Call) : Option Contract :=
  if unmet e c then none else e.contracts c.callee

/-! ### The premise never invents, alters or widens a declaration -/

theorem an_applied_contract_is_the_declared_one {e : Environment} {c : Call} {k : Contract}
    (h : applied e c = some k) : e.contracts c.callee = some k := by
  unfold applied at h
  split at h
  · exact absurd h (by simp)
  · exact h

theorem an_applied_call_has_a_declaration {e : Environment} {c : Call}
    (h : (applied e c).isSome = true) : (e.contracts c.callee).isSome = true := by
  cases hk : applied e c with
  | none => simp [hk] at h
  | some k => simp [an_applied_contract_is_the_declared_one hk]

theorem an_undeclared_callee_is_never_applied {e : Environment} {c : Call}
    (h : e.contracts c.callee = none) : applied e c = none := by
  unfold applied; split <;> simp [h]

/-! ### Every withdrawal ground -/

theorem a_poisoned_environment_applies_nothing {e : Environment} {c : Call}
    (h : e.poisoned = true) : applied e c = none := by
  unfold applied unmet; simp [h]

theorem an_unmet_premise_applies_nothing {e : Environment} {c : Call}
    (h : unmet e c = true) : applied e c = none := by
  unfold applied; simp [h]

theorem a_missing_actual_never_satisfies {arguments : List Tokens} {p : Premise}
    (h : arguments.length ≤ p.index) : satisfies arguments p = false := by
  unfold satisfies
  rw [List.getElem?_eq_none h]

theorem only_the_exact_literal_satisfies {arguments : List Tokens} {p : Premise}
    (h : satisfies arguments p = true) : arguments[p.index]? = some p.literal := by
  unfold satisfies at h
  cases hg : arguments[p.index]? with
  | none => rw [hg] at h; exact absurd h (by simp)
  | some actual =>
      rw [hg] at h
      simp only [beq_iff_eq] at h
      rw [h]

theorem a_mismatched_actual_withdraws {e : Environment} {c : Call} {p : Premise}
    (hp : p ∈ e.premises c.callee) (hs : satisfies c.arguments p = false) :
    applied e c = none := by
  refine an_unmet_premise_applies_nothing ?_
  unfold unmet
  simp only [Bool.or_eq_true]
  exact Or.inr (List.any_eq_true.mpr ⟨p, hp, by simp [hs]⟩)

/-! ### With no premise the environment is unchanged -/

theorem no_premise_is_transparent {e : Environment} {c : Call}
    (hp : e.premises c.callee = []) (hq : e.poisoned = false) :
    applied e c = e.contracts c.callee := by
  unfold applied unmet
  simp [hp, hq]

/-! ### Adding a premise can only refuse more -/

/-- Extend an environment with additional premises, leaving contracts alone. -/
def withExtraPremises (e : Environment) (extra : Name → List Premise) : Environment :=
  { e with premises := fun n => extra n ++ e.premises n }

theorem extra_premises_preserve_withdrawal {e : Environment} {c : Call}
    {extra : Name → List Premise} (h : unmet e c = true) :
    unmet (withExtraPremises e extra) c = true := by
  unfold unmet withExtraPremises at *
  simp only [Bool.or_eq_true, List.any_append] at *
  cases h with
  | inl hp => exact Or.inl hp
  | inr ha => exact Or.inr (by simp [ha])

theorem extra_premises_never_admit {e : Environment} {c : Call} {extra : Name → List Premise}
    (h : applied e c = none) : applied (withExtraPremises e extra) c = none := by
  by_cases hu : unmet e c = true
  · exact an_unmet_premise_applies_nothing (extra_premises_preserve_withdrawal hu)
  · have hc : e.contracts c.callee = none := by
      unfold applied at h; simpa [hu] using h
    unfold applied withExtraPremises
    split
    · rfl
    · exact hc

/-! ### Non-vacuity: the mechanism decides both ways -/

/-- The premise this card declares: parameter 4 must be exactly the token `0`. -/
def sizePremise : Premise := ⟨4, [0]⟩

/-- The measured signature: five parameters, every one borrowed. -/
def publication : Contract :=
  ⟨[Mode.borrowed, Mode.borrowed, Mode.borrowed, Mode.borrowed, Mode.borrowed]⟩

/-- The environment the card lands: one declaration, one premise. -/
def declared : Environment where
  contracts := fun n => if n = 0 then some publication else none
  premises := fun n => if n = 0 then [sizePremise] else []
  poisoned := false

/-- A call passing the literal `0` at parameter 4. -/
def atZero : Call := ⟨0, [[1], [2], [3], [4], [0]]⟩

/-- A call passing a positive literal there. -/
def atPositive : Call := ⟨0, [[1], [2], [3], [4], [5]]⟩

/-- A call that does not reach parameter 4 at all. -/
def tooShort : Call := ⟨0, [[1], [2]]⟩

theorem the_literal_call_is_applied : applied declared atZero = some publication := by decide

theorem the_positive_call_is_withdrawn : applied declared atPositive = none := by decide

theorem the_short_call_is_withdrawn : applied declared tooShort = none := by decide

theorem poison_withdraws_the_literal_call :
    applied { declared with poisoned := true } atZero = none := by decide

theorem without_the_premise_the_positive_call_is_applied :
    applied { declared with premises := fun _ => [] } atPositive = some publication := by decide

/-! ### The measured corpus census

The `val_size` actual at each of the fifteen `flb_hash_table_add` call sites of
the pinned Fluent Bit corpus (`66910c10…`), as its token vector. `[0]` is the
literal `0`; every other vector stands for the tokens actually spelled there
(`vlen`, `(ssize_t) value_length`, `sizeof(value)`, and so on), which this model
distinguishes only by not being `[0]`. -/
def measuredSizeActuals : List Tokens :=
  [ [0]        -- src/flb_input_chunk.c:2606   ht_log_chunks
  , [0]        -- src/flb_input_chunk.c:2610   ht_metric_chunks
  , [0]        -- src/flb_input_chunk.c:2614   ht_trace_chunks
  , [0]        -- src/flb_input_chunk.c:2617   ht_profile_chunks
  , [0]        -- src/flb_oauth2_jwt.c:910     jwks_key
  , [1]        -- src/flb_upstream_ha.c:312    vlen
  , [2]        -- src/flb_http_common.c:308    value_length
  , [3]        -- src/flb_http_common.c:1171   value_length
  , [4]        -- src/http_server/…http1.c:462 (ssize_t) value_length
  , [5]        -- src/http_server/…http2.c:140 (ssize_t) value_length
  , [6]        -- src/http_server/…http2.c:559 value_length
  , [7]        -- src/flb_input_chunk.c:216    sizeof(value)
  , [8]        -- src/flb_callback.c:68        sizeof(struct flb_callback_entry *)
  , [9]        -- src/flb_env.c:186            vlen
  , [10]       -- src/flb_env.c:283            vlen
  ]

theorem the_census_has_fifteen_call_sites : measuredSizeActuals.length = 15 := by decide

theorem exactly_five_call_sites_satisfy_the_premise :
    (measuredSizeActuals.filter (fun a => a == sizePremise.literal)).length = 5 := by decide

theorem exactly_ten_call_sites_are_withdrawn :
    (measuredSizeActuals.filter (fun a => !(a == sizePremise.literal))).length = 10 := by decide

/-! ## 3. The carrier, and the migration off the comment directive

Card **HASH-CLAUSE**. The premise above was first written as a machine-parsed
`// qcore-premise: literal_argument(...)` comment in the module's declared text,
and is now the first-class clause `requires resource_literal_argument(p, L);`.
The mechanism of §1 is unchanged; what changes is *where the premise is read
from*. Three things are proved here.

**Parity.** The refusal set at a call site is a function of the premise
environment alone, so two carriers that state the same premises decide every
call site identically. That is the general form of the measurement the card
took, not a restatement of it: the Rust side measured that the two carriers
*do* state the same premises, and this says that measuring that is enough.

**Non-vacuity.** Two carriers can disagree, so the parity theorem is not
vacuous.

**Retirement.** A reader that no longer parses the directive reads a
directive-only text as carrying *no* premise, which re-admits exactly the call
sites the premise withdrew. The exhibited hazard is closed by refusing such a
text outright, and refusing is proved to close it. -/

/-- Where a premise is written. -/
inductive Carrier where
  /-- The retired machine-parsed comment directive (card HASH-CONTRACTS). -/
  | directive
  /-- The §C04.5 clause (card HASH-CLAUSE). -/
  | clause
  deriving DecidableEq, Repr

/-- A declared module text, abstracted to what each carrier states about it and
to whether the reader could read the text at all. `readable = false` is the
loader refusing the text — a malformed clause, or, after retirement, a text
still carrying the directive. -/
structure Declared where
  viaDirective : Name → List Premise
  viaClause : Name → List Premise
  readable : Bool

/-- Read premises from one carrier. An unreadable text **poisons**: the premise
set is not emptied, because an empty set applies every declaration everywhere. -/
def readWith (c : Carrier) (d : Declared) (contracts : Name → Option Contract) :
    Environment where
  contracts := contracts
  premises := match c with
    | Carrier.directive => d.viaDirective
    | Carrier.clause => d.viaClause
  poisoned := !d.readable

/-- The fail-open reading the implementation must never perform: an unreadable
text answered with no premises at all. -/
def readOpen (contracts : Name → Option Contract) : Environment where
  contracts := contracts
  premises := fun _ => []
  poisoned := false

/-! ### Parity: equal premises decide every call site identically -/

theorem equal_carriers_decide_every_call_site_alike {d : Declared}
    {contracts : Name → Option Contract} (h : ∀ n, d.viaDirective n = d.viaClause n)
    (c : Call) :
    applied (readWith Carrier.directive d contracts) c
      = applied (readWith Carrier.clause d contracts) c := by
  unfold applied unmet readWith
  simp [h c.callee]

theorem equal_carriers_withdraw_alike {d : Declared}
    {contracts : Name → Option Contract} (h : ∀ n, d.viaDirective n = d.viaClause n)
    (c : Call) :
    unmet (readWith Carrier.directive d contracts) c
      = unmet (readWith Carrier.clause d contracts) c := by
  unfold unmet readWith
  simp [h c.callee]

/-- The migrated declaration: both carriers state the same one premise, and the
text reads. -/
def migrated : Declared where
  viaDirective := fun n => if n = 0 then [sizePremise] else []
  viaClause := fun n => if n = 0 then [sizePremise] else []
  readable := true

theorem the_migrated_carriers_state_the_same_premises (n : Name) :
    migrated.viaDirective n = migrated.viaClause n := by
  unfold migrated; simp

theorem the_migration_preserves_every_refusal
    (contracts : Name → Option Contract) (c : Call) :
    applied (readWith Carrier.directive migrated contracts) c
      = applied (readWith Carrier.clause migrated contracts) c :=
  equal_carriers_decide_every_call_site_alike the_migrated_carriers_state_the_same_premises c

/-- The environment the card lands is exactly what either carrier reads. -/
theorem the_clause_reading_is_the_declared_environment (c : Call) :
    applied (readWith Carrier.clause migrated declared.contracts) c = applied declared c := by
  unfold applied unmet readWith migrated declared
  simp only [Bool.not_true]

/-! ### Non-vacuity: the carriers *can* disagree -/

/-- A text whose clause states the premise and whose directive states nothing. -/
def clauseOnly : Declared where
  viaDirective := fun _ => []
  viaClause := fun n => if n = 0 then [sizePremise] else []
  readable := true

theorem the_carriers_can_disagree :
    applied (readWith Carrier.directive clauseOnly declared.contracts) atPositive
      ≠ applied (readWith Carrier.clause clauseOnly declared.contracts) atPositive := by
  decide

/-! ### Retirement: an unread directive re-admits what the premise withdrew -/

/-- A text that still carries only the retired directive. After retirement no
reader parses it, so its clause carrier states nothing. -/
def directiveOnly : Declared where
  viaDirective := fun n => if n = 0 then [sizePremise] else []
  viaClause := fun _ => []
  readable := true

/-- **The hazard, exhibited.** Silently ignoring a leftover directive applies the
declaration at the very call site the premise withdrew. -/
theorem an_ignored_directive_readmits_the_withdrawn_call :
    applied (readWith Carrier.clause directiveOnly declared.contracts) atPositive
      = some publication := by decide

/-- The same text, refused by the loader. Refusing closes the hazard. -/
def directiveRefused : Declared := { directiveOnly with readable := false }

theorem refusing_the_directive_text_withdraws_every_call (c : Call) :
    applied (readWith Carrier.clause directiveRefused declared.contracts) c = none := by
  refine a_poisoned_environment_applies_nothing ?_
  unfold readWith directiveRefused directiveOnly
  simp

/-- **Why poisoning is not emptying.** On the same unreadable text, the
fail-open reading applies the declaration everywhere the poisoned one refuses. -/
theorem the_fail_open_reading_applies_what_poison_refuses :
    applied (readOpen declared.contracts) atPositive = some publication
      ∧ applied (readWith Carrier.clause directiveRefused declared.contracts) atPositive
          = none := by
  constructor
  · decide
  · exact refusing_the_directive_text_withdraws_every_call atPositive

theorem the_fail_open_reading_even_applies_at_an_unmet_premise :
    applied (readOpen declared.contracts) tooShort = some publication := by decide

/-! ## The upstream body, and which mode is faithful -/

/-- The two arms of `entry_set_value`. -/
inductive Arm where
  | reference
  | copy
  deriving DecidableEq, Repr

def arm (size : Int) : Arm := if 0 < size then Arm.copy else Arm.reference

/-- What `entry->val` points at after the call. -/
inductive Slot where
  | callerPointer
  | calleeAllocation
  deriving DecidableEq, Repr

def storedSlot (size : Int) : Slot :=
  match arm size with
  | Arm.copy => Slot.calleeAllocation
  | Arm.reference => Slot.callerPointer

/-- `entry->val_size` after the call: the size on the copy arm, `-1` otherwise. -/
def recordedSize (size : Int) : Int := if 0 < size then size else -1

/-- `flb_hash_table_entry_free` frees `entry->val` exactly when the *recorded*
size is positive. -/
def freesStoredSlot (size : Int) : Bool := decide (0 < recordedSize size)

theorem the_reference_arm_records_minus_one {size : Int} (h : size ≤ 0) :
    recordedSize size = -1 := by
  unfold recordedSize; simp [Int.not_lt.mpr h]

theorem the_reference_arm_is_never_freed_by_the_table {size : Int} (h : size ≤ 0) :
    freesStoredSlot size = false := by
  unfold freesStoredSlot
  simp [the_reference_arm_records_minus_one h]

theorem the_reference_arm_stores_the_callers_pointer {size : Int} (h : size ≤ 0) :
    storedSlot size = Slot.callerPointer := by
  unfold storedSlot arm; simp [Int.not_lt.mpr h]

theorem the_copy_arm_stores_the_callees_allocation {size : Int} (h : 0 < size) :
    storedSlot size = Slot.calleeAllocation := by
  unfold storedSlot arm; simp [h]

theorem the_copy_arm_is_freed_by_the_table {size : Int} (h : 0 < size) :
    freesStoredSlot size = true := by
  unfold freesStoredSlot recordedSize; simp [h]

/-- The sentence the whole declaration rests on: on no arm does the table free a
pointer the caller handed it. -/
theorem the_callers_pointer_is_never_freed_by_the_table (size : Int) :
    ¬(freesStoredSlot size = true ∧ storedSlot size = Slot.callerPointer) := by
  rintro ⟨hf, hs⟩
  by_cases h : 0 < size
  · rw [the_copy_arm_stores_the_callees_allocation h] at hs; exact absurd hs (by simp)
  · rw [the_reference_arm_is_never_freed_by_the_table (Int.not_lt.mp h)] at hf
    exact absurd hf (by simp)

/-! ### The caller-side effect of each mode, against the measured one -/

/-- What a declared mode does to the caller, at the granularity §C04.2 states. -/
inductive CallerEffect where
  /-- No precondition and no transfer. -/
  | none
  /-- The caller must supply live custody. -/
  | demandsLive
  /-- The obligation moves to the callee. -/
  | transfers
  /-- The obligation is discharged. -/
  | discharges
  /-- Fresh custody appears at the output destination. -/
  | createsAtDestination
  /-- The obligation moves beneath the declared owner. -/
  | transfersUnderOwner
  /-- The resource is detached from the declared owner. -/
  | detachesFromOwner
  deriving DecidableEq, Repr

def effect : Mode → CallerEffect
  | Mode.borrowed => CallerEffect.none
  | Mode.owned => CallerEffect.demandsLive
  | Mode.consumed => CallerEffect.transfers
  | Mode.released => CallerEffect.discharges
  | Mode.out => CallerEffect.createsAtDestination
  | Mode.stored _ => CallerEffect.transfersUnderOwner
  | Mode.removed _ => CallerEffect.detachesFromOwner

/-- The measured effect of the size-zero arm on the caller's value: none. The
table imposes no precondition on the pointer and frees nothing. -/
def measuredEffect : CallerEffect := CallerEffect.none

theorem borrowed_has_the_measured_effect : effect Mode.borrowed = measuredEffect := rfl

theorem borrowed_is_the_only_mode_with_the_measured_effect (m : Mode) :
    effect m = measuredEffect ↔ m = Mode.borrowed := by
  constructor
  · intro h; cases m <;> simp_all [effect, measuredEffect]
  · rintro rfl; rfl

theorem the_stored_reading_would_move_the_obligation (owner : Nat) :
    effect (Mode.stored owner) ≠ measuredEffect := by
  simp [effect, measuredEffect]

theorem the_released_reading_would_discharge_the_obligation :
    effect Mode.released ≠ measuredEffect := by simp [effect, measuredEffect]

theorem the_consumed_reading_would_transfer_the_obligation :
    effect Mode.consumed ≠ measuredEffect := by simp [effect, measuredEffect]

theorem the_owned_reading_would_demand_live_custody :
    effect Mode.owned ≠ measuredEffect := by simp [effect, measuredEffect]

/-! ### The stored reading is not even well formed here

§C04.2: the `owner` of an indexed mode must name a parameter whose own mode is
`Owned`. Every parameter of the measured signature is borrowed. -/

def indexedOwnerIsOwned (c : Contract) : Mode → Bool
  | Mode.stored owner => c.parameters[owner]? == some Mode.owned
  | Mode.removed owner => c.parameters[owner]? == some Mode.owned
  | _ => true

theorem no_stored_reading_of_the_publication_is_well_formed (owner : Nat) :
    indexedOwnerIsOwned publication (Mode.stored owner) = false := by
  unfold indexedOwnerIsOwned publication
  match owner with
  | 0 => decide
  | 1 => decide
  | 2 => decide
  | 3 => decide
  | 4 => decide
  | _ + 5 => simp

theorem the_publication_borrows_every_parameter (m : Mode) (h : m ∈ publication.parameters) :
    m = Mode.borrowed := by
  unfold publication at h
  simp at h
  exact h

/-! ### Retention is real, and is not custody

At the size-zero arm the table keeps the caller's pointer. That is a
*reachability* fact, and the declaration makes no claim about it: the caller's
obligation is unchanged either way. Both halves are proved so that neither can
be read as the other. -/

def retainsCallerPointer (size : Int) : Bool := storedSlot size == Slot.callerPointer

theorem the_admitted_arm_retains {size : Int} (h : size ≤ 0) :
    retainsCallerPointer size = true := by
  unfold retainsCallerPointer; simp [the_reference_arm_stores_the_callers_pointer h]

theorem the_withdrawn_arm_does_not_retain {size : Int} (h : 0 < size) :
    retainsCallerPointer size = false := by
  unfold retainsCallerPointer; simp [the_copy_arm_stores_the_callees_allocation h]

theorem retention_does_not_change_the_caller_effect (size : Int) :
    retainsCallerPointer size = true → effect Mode.borrowed = measuredEffect := by
  intro _; rfl

/-! ### The premise admits exactly the arm the declaration was read from -/

/-- The token vector a call site spells for the size `0`. -/
def zeroTokens : Tokens := [0]

theorem the_premise_literal_is_the_zero_token : sizePremise.literal = zeroTokens := rfl

theorem the_admitted_size_takes_the_reference_arm : arm 0 = Arm.reference := by decide

theorem a_positive_size_takes_the_copy_arm : arm 8 = Arm.copy := by decide

theorem the_two_arms_are_distinct : Arm.reference ≠ Arm.copy := by decide

/-! ## 4. Which call-site spellings are the declared constant

Card **HASH-REFINE**. §1 compares an actual against the declared literal as token
vectors, exactly. That is sound but *narrow*: a call site writing `0L` or `(0)`
passes the very constant the premise names, and refusing it withdraws a
declaration the premise says applies — a silent narrowing whose real cause is a
spelling, reported as if it were a value.

§C04.5.1 fixes which spellings are that constant. The relation modelled here
erases exactly two things and nothing else:

* **matched enclosing parentheses**, which are grammar, and
* **an integer suffix**, which fixes the constant's *type* and not its value.

Everything else keeps its refusal, because deciding it would need a capability
the premise deliberately lacks — arithmetic (`0 + 0`, `-0`), a base (`0x0`, and
the leading zero of `00`, since `010` is 8), reaching into a cast, or symbol
resolution.

Four sentences are proved.

**It is decided, not evaluated.** An admitted spelling's digit run and sign are
returned verbatim from one integer token of the actual's own stream: nothing is
normalised, carried, negated or re-based, and no value is computed in order to
compare one.

**It is exact.** Every admitted spelling *denotes* the declared constant. The
`denote` function below exists only to state this; the implementation never
computes it.

**It is a strict widening.** Every actual that satisfied the exact test still
satisfies this one, so no call site the premise used to admit can have been
withdrawn — which is what makes the corpus re-measurement's byte-identity a
check on this change rather than a coincidence.

**It never crosses a value.** An admitted spelling of one constant satisfies no
declaration of another, and a sign is never erased. -/

/-- A decimal digit run. -/
abbrev Digits := List Nat

/-- Whether a digit run is one this relation decides: nonempty, and without a
leading zero, which in C is an octal prefix rather than a decimal digit. -/
def digitsDecided : Digits → Bool
  | [] => false
  | [_] => true
  | 0 :: _ :: _ => false
  | _ :: _ :: _ => true

/-- What follows an integer token's digit run. The gateway's C tokenizer takes a
digit and every alphanumeric byte after it as one token, so the letters arrive
attached and are classified here rather than assumed away. -/
inductive Suffix where
  /-- Nothing follows the digits: the canonical declared spelling. -/
  | bare
  /-- An admitted C11 6.4.4.1 integer suffix — `L`, `u`, `ULL`. It fixes the
  constant's type and leaves its value alone. -/
  | admitted (code : Nat)
  /-- Letters that are not an admitted suffix: the `x0` of `0x0`, the `b1` of
  `0b1`, the `i32` of `0i32`. Reading them needs a base or a foreign vocabulary. -/
  | foreign (code : Nat)
  deriving DecidableEq, Repr

def suffixAdmitted : Suffix → Bool
  | Suffix.bare => true
  | Suffix.admitted _ => true
  | Suffix.foreign _ => false

/-- One C token, at the granularity this relation reads them. -/
inductive Tok where
  | lparen
  | rparen
  | minus
  /-- An integer-constant token: its leading decimal digit run and what follows. -/
  | integer (digits : Digits) (suffix : Suffix)
  /-- Any other token: an operator, an identifier, a keyword, a punctuator. -/
  | other (code : Nat)
  deriving DecidableEq, Repr

/-- The canonical declared spelling of a constant: an optional minus and one
unsuffixed integer token. This is exactly what the loader admits as `LITERAL`. -/
def declaredTokens (negative : Bool) (d : Digits) : List Tok :=
  if negative then [Tok.minus, Tok.integer d Suffix.bare] else [Tok.integer d Suffix.bare]

/-- The tokens left when the enclosing parentheses are erased, or `none` when the
leading `(` run and the trailing `)` run do not balance.

Both runs are read off the ends, so a parenthesis that belongs to something else
— the cast in `(long) 0`, the second operand of `(0) + (0)` — is never erased:
either the runs disagree in length, or what is left still carries the stray
parenthesis and no literal is read out of it. Both are exhibited below. -/
def coreOpens (t : List Tok) : Nat :=
  (t.takeWhile (fun token => token == Tok.lparen)).length

def coreRest (t : List Tok) : List Tok := t.drop (coreOpens t)

def coreCloses (t : List Tok) : Nat :=
  ((coreRest t).reverse.takeWhile (fun token => token == Tok.rparen)).length

def core (t : List Tok) : Option (List Tok) :=
  if coreCloses t == coreOpens t then
    some ((coreRest t).take ((coreRest t).length - coreCloses t))
  else
    none

/-- Read one integer constant out of an already-unwrapped token vector. The
result is the token's *own* digit run: this function selects, it does not
compute. -/
def readLiteral : List Tok → Option (Bool × Digits)
  | [Tok.integer d s] => if digitsDecided d && suffixAdmitted s then some (false, d) else none
  | [Tok.minus, Tok.integer d s] =>
      -- `-0` is not a second spelling of `0`: equating them is evaluating the
      -- sign, and the loader refuses to declare it either.
      if digitsDecided d && suffixAdmitted s && !(d == [0]) then some (true, d) else none
  | _ => none

/-- The declared constant an actual spells, if this relation decides it. -/
def canonical (t : List Tok) : Option (Bool × Digits) := (core t).bind readLiteral

/-- The value of a decimal digit run. Nothing in the implementation computes
this; it is here so that "the admitted spellings are the same constant" can be
*stated*. -/
def value (d : Digits) : Nat := d.foldl (fun acc digit => acc * 10 + digit) 0

/-- The integer a spelling denotes, where this relation decides it at all. -/
def denote (t : List Tok) : Option Int :=
  (canonical t).map fun (negative, d) =>
    if negative then -(value d : Int) else (value d : Int)

/-- The premise test, refined: an actual satisfies a declared literal either by
being it token for token, or by spelling it. -/
def satisfiesSpelling (actual : List Tok) (negative : Bool) (d : Digits) : Bool :=
  (actual == declaredTokens negative d) || (canonical actual == some (negative, d))

/-! ### Decided, not evaluated: the digits and the sign are read, not computed -/

/-- Every integer token's digit run in a token vector, in order. -/
def digitRuns : List Tok → List Digits
  | [] => []
  | Tok.integer d _ :: rest => d :: digitRuns rest
  | _ :: rest => digitRuns rest

/-- **The literal reader selects; it does not compute.** Whatever it returns, the
unwrapped vector it read *is* one integer token carrying exactly those digits,
optionally preceded by the minus whose presence is exactly the sign returned. -/
theorem the_read_literal_is_the_token_itself {t : List Tok} {negative : Bool} {d : Digits}
    (h : readLiteral t = some (negative, d)) :
    (negative = false ∧ ∃ s, t = [Tok.integer d s])
      ∨ (negative = true ∧ ∃ s, t = [Tok.minus, Tok.integer d s]) := by
  unfold readLiteral at h
  split at h
  · rename_i e s
    split at h
    · simp only [Option.some.injEq, Prod.mk.injEq] at h
      exact Or.inl ⟨h.1.symm, s, by rw [h.2]⟩
    · exact absurd h (by simp)
  · rename_i e s
    split at h
    · simp only [Option.some.injEq, Prod.mk.injEq] at h
      exact Or.inr ⟨h.1.symm, s, by rw [h.2]⟩
    · exact absurd h (by simp)
  · exact absurd h (by simp)

/-- The same fact as a digit-run identity: the unwrapped vector carries exactly
one digit run, and it is the one returned. -/
theorem an_admitted_spelling_returns_its_own_digits {t c : List Tok} {negative : Bool}
    {d : Digits} (hc : core t = some c) (h : canonical t = some (negative, d)) :
    digitRuns c = [d] := by
  have hr : readLiteral c = some (negative, d) := by
    unfold canonical at h; rw [hc] at h; simpa using h
  rcases the_read_literal_is_the_token_itself hr with ⟨_, s, hs⟩ | ⟨_, s, hs⟩ <;>
    simp [hs, digitRuns]

/-- The vector the literal is read out of is a contiguous slice of the actual's
own token stream — a `drop` followed by a `take`, by definition — so nothing is
read from anywhere else. -/
theorem the_core_is_a_contiguous_slice {t c : List Tok} (h : core t = some c) :
    c = (t.drop (coreOpens t)).take ((coreRest t).length - coreCloses t) := by
  unfold core at h
  split at h
  · simpa [coreRest] using h.symm
  · exact absurd h (by simp)

/-! ### Exact: every admitted spelling denotes the declared constant -/

/-- The declared spelling of a decided constant canonicalises to itself, so the
declared form is a fixed point of the relation. -/
theorem a_declared_literal_is_its_own_canonical_form
    {negative : Bool} {d : Digits} (hd : digitsDecided d = true)
    (hz : negative = true → (!(d == [0])) = true) :
    canonical (declaredTokens negative d) = some (negative, d) := by
  cases negative with
  | false =>
      simp [declaredTokens, canonical, core, coreOpens, coreRest, coreCloses, readLiteral,
        suffixAdmitted, hd]
  | true =>
      simp [declaredTokens, canonical, core, coreOpens, coreRest, coreCloses, readLiteral,
        suffixAdmitted, hd, hz rfl]

/-- **The soundness sentence.** Whatever spelling was admitted, it denotes
exactly the constant the declaration names. -/
theorem an_admitted_spelling_denotes_the_declared_constant
    {t : List Tok} {negative : Bool} {d : Digits}
    (h : canonical t = some (negative, d)) (hd : digitsDecided d = true)
    (hz : negative = true → (!(d == [0])) = true) :
    denote t = denote (declaredTokens negative d) := by
  unfold denote
  rw [h, a_declared_literal_is_its_own_canonical_form hd hz]

/-- And `denote` is not constant: it separates the constants the premise names. -/
theorem denote_separates_two_constants :
    denote (declaredTokens false [0]) ≠ denote (declaredTokens false [8]) := by decide

/-! ### A strict widening: nothing that matched before can have been withdrawn -/

theorem exact_token_equality_still_satisfies (negative : Bool) (d : Digits) :
    satisfiesSpelling (declaredTokens negative d) negative d = true := by
  simp [satisfiesSpelling]

theorem the_refinement_only_admits {actual : List Tok} {negative : Bool} {d : Digits}
    (h : actual = declaredTokens negative d) :
    satisfiesSpelling actual negative d = true := by
  rw [h]; exact exact_token_equality_still_satisfies negative d

/-- Every actual the refined test admits either *is* the declared vector or
denotes the declared constant. There is no third way in. -/
theorem admission_is_identity_or_denotation {actual : List Tok} {negative : Bool} {d : Digits}
    (h : satisfiesSpelling actual negative d = true) :
    actual = declaredTokens negative d ∨ canonical actual = some (negative, d) := by
  unfold satisfiesSpelling at h
  simp only [Bool.or_eq_true, beq_iff_eq] at h
  exact h

/-! ### The admitted spellings, decided -/

/-- The suffix code standing for `L`. -/
def suffixL : Suffix := Suffix.admitted 0

/-- `0`, the declared spelling. -/
def spellZero : List Tok := [Tok.integer [0] Suffix.bare]
/-- `(0)`. -/
def spellParenZero : List Tok := [Tok.lparen, Tok.integer [0] Suffix.bare, Tok.rparen]
/-- `((0))`. -/
def spellDoubleParenZero : List Tok :=
  [Tok.lparen, Tok.lparen, Tok.integer [0] Suffix.bare, Tok.rparen, Tok.rparen]
/-- `0L`: the same digit run with an admitted suffix. -/
def spellSuffixedZero : List Tok := [Tok.integer [0] suffixL]
/-- `(0L)`. -/
def spellParenSuffixedZero : List Tok := [Tok.lparen, Tok.integer [0] suffixL, Tok.rparen]

theorem the_declared_spelling_is_admitted :
    satisfiesSpelling spellZero false [0] = true := by decide

theorem a_parenthesised_spelling_is_admitted :
    satisfiesSpelling spellParenZero false [0] = true := by decide

theorem a_doubly_parenthesised_spelling_is_admitted :
    satisfiesSpelling spellDoubleParenZero false [0] = true := by decide

theorem a_suffixed_spelling_is_admitted :
    satisfiesSpelling spellSuffixedZero false [0] = true := by decide

theorem a_parenthesised_suffixed_spelling_is_admitted :
    satisfiesSpelling spellParenSuffixedZero false [0] = true := by decide

/-- **Non-vacuity of the widening.** Both admitted spellings differ from the
declared vector, so the refinement decides strictly more than the exact test
did — and each is admitted by the canonical clause, not by identity. -/
theorem the_widening_admits_more_than_exact_equality :
    spellParenZero ≠ declaredTokens false [0]
      ∧ spellSuffixedZero ≠ declaredTokens false [0]
      ∧ canonical spellParenZero = some (false, [0])
      ∧ canonical spellSuffixedZero = some (false, [0]) := by
  refine ⟨by decide, by decide, by decide, by decide⟩

/-! ### The refused spellings, and the capability each would need -/

/-- `0 + 0` — arithmetic. -/
def spellSum : List Tok :=
  [Tok.integer [0] Suffix.bare, Tok.other 0, Tok.integer [0] Suffix.bare]
/-- `-0` — arithmetic: unary minus. -/
def spellNegativeZero : List Tok := [Tok.minus, Tok.integer [0] Suffix.bare]
/-- `0x0` — a base; the trailing letters are not an admitted suffix. -/
def spellHexZero : List Tok := [Tok.integer [0] (Suffix.foreign 0)]
/-- `00` — a base: a leading zero is octal, and `010` is 8. -/
def spellOctalZero : List Tok := [Tok.integer [0, 0] Suffix.bare]
/-- `(long) 0` — a cast, whose operand this relation does not reach into. -/
def spellCastZero : List Tok :=
  [Tok.lparen, Tok.other 1, Tok.rparen, Tok.integer [0] Suffix.bare]
/-- `ZERO` left as an identifier — symbol resolution. -/
def spellIdentifier : List Tok := [Tok.other 2]
/-- `(0) + (0)` — arithmetic, and the enclosing parentheses do not match. -/
def spellParenSum : List Tok :=
  [Tok.lparen, Tok.integer [0] Suffix.bare, Tok.rparen, Tok.other 0,
   Tok.lparen, Tok.integer [0] Suffix.bare, Tok.rparen]

theorem arithmetic_is_refused : satisfiesSpelling spellSum false [0] = false := by decide

theorem a_negated_zero_is_refused :
    satisfiesSpelling spellNegativeZero false [0] = false := by decide

theorem an_alternate_base_is_refused :
    satisfiesSpelling spellHexZero false [0] = false := by decide

theorem an_octal_spelling_is_refused :
    satisfiesSpelling spellOctalZero false [0] = false := by decide

theorem a_cast_is_refused : satisfiesSpelling spellCastZero false [0] = false := by decide

theorem an_identifier_is_refused :
    satisfiesSpelling spellIdentifier false [0] = false := by decide

theorem an_unmatched_parenthesis_pair_is_refused :
    satisfiesSpelling spellParenSum false [0] = false := by decide

/-- **The leading zero must not be stripped**, and is not: `010` denotes 8, so
admitting it against a declared `10` would be a wrong answer rather than a
missing one. It is refused against every declaration. -/
theorem an_octal_run_matches_no_decimal_declaration {negative : Bool} {d : Digits}
    (hd : digitsDecided d = true) :
    satisfiesSpelling [Tok.integer [0, 1, 0] Suffix.bare] negative d = false := by
  have hne : d ≠ [0, 1, 0] := by rintro rfl; simp [digitsDecided] at hd
  cases negative <;>
    simp [satisfiesSpelling, declaredTokens, canonical, core, coreOpens, coreRest, coreCloses,
      readLiteral, digitsDecided, Ne.symm hne]

/-! ### The widening never crosses a value -/

theorem an_admitted_zero_never_satisfies_another_constant :
    satisfiesSpelling spellParenZero false [8] = false
      ∧ satisfiesSpelling spellSuffixedZero false [8] = false := by
  refine ⟨by decide, by decide⟩

/-- `(-1)` spells `-1` and does not spell `1`: a sign is erased by nothing. -/
theorem a_sign_is_never_erased :
    satisfiesSpelling [Tok.lparen, Tok.minus, Tok.integer [1] Suffix.bare, Tok.rparen] true [1]
        = true
      ∧ satisfiesSpelling [Tok.lparen, Tok.minus, Tok.integer [1] Suffix.bare, Tok.rparen] false
          [1] = false := by
  refine ⟨by decide, by decide⟩

/-! ## 5. The premise on any parameter (card HASH-REFINE, HASH-CLAUSE-TYPE)

The clause is admitted on a parameter of any declared type. The mechanism of §1
reads the parameter *index* and nothing about its type, so the statement to prove
is that admitting the clause there cannot make a consumer more permissive: a
premise whose literal no call site can supply withdraws every call, and the
declaration applied is still the declared one, unchanged. -/

/-- A declaration whose premise names parameter 1 — a pointer parameter in the
gateway's fixture — and the literal `0`, C's null pointer constant. -/
def pointerPremise : Premise := ⟨1, [0]⟩

def pointerDeclared : Environment where
  contracts := fun n => if n = 0 then some ⟨[Mode.borrowed, Mode.borrowed]⟩ else none
  premises := fun n => if n = 0 then [pointerPremise] else []
  poisoned := false

/-- The null-sentinel call: the second actual is the literal `0`. -/
def atNullSentinel : Call := ⟨0, [[1], [0]]⟩
/-- A call passing a real pointer there. -/
def atLivePointer : Call := ⟨0, [[1], [2]]⟩

theorem the_null_sentinel_call_is_applied :
    applied pointerDeclared atNullSentinel = some ⟨[Mode.borrowed, Mode.borrowed]⟩ := by decide

theorem the_live_pointer_call_is_withdrawn :
    applied pointerDeclared atLivePointer = none := by decide

/-- **A premise no call site can satisfy is vacuous in the refusing direction.**
This is the second half of the type decision: admitting the clause on a pointer
parameter cannot admit anything, because the only thing a premise does is
withdraw. -/
theorem an_unsatisfiable_pointer_premise_withdraws_every_call
    (arguments : List Tokens) (h : ∀ a ∈ arguments, a ≠ [7]) :
    applied { pointerDeclared with premises := fun n => if n = 0 then [⟨1, [7]⟩] else [] }
        ⟨0, arguments⟩ = none := by
  refine an_unmet_premise_applies_nothing ?_
  unfold unmet
  simp only [Bool.or_eq_true]
  refine Or.inr (List.any_eq_true.mpr ⟨⟨1, [7]⟩, by simp, ?_⟩)
  simp only [Bool.not_eq_true']
  unfold satisfies
  cases hg : arguments[1]? with
  | none => simp
  | some a =>
      have : a ≠ [7] := h a (List.mem_of_getElem? hg)
      simp [this]

end QCore.LiteralArgumentPremise

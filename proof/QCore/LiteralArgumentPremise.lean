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

end QCore.LiteralArgumentPremise

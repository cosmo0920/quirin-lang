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
# Declared variadic data tails and one level of nested calls

This module models the two decisions
`crates/qcore-c-gateway/src/variadic_nested.rs` makes, and the one property
that makes them compose:

* **`admitsNested`** — the eight conjunctive premises under which a call nested
  inside another call's argument is *modeled* rather than refused;
* **`admitsTail`** — the premises under which the actuals beyond a declaration's
  named parameter vector are admitted, which requires **both** a declared
  `resource_variadic_data_tail()` and, per call site, that every one of those
  actuals is data;
* **`applyCall`** — the declared effects of one modeled call, as an outcome
  carrying the post-state, the identities the call *demands* be live, and
  whether it *produced* a fresh identity that must be bound.

## The composition theorem, and why it is the whole design

C leaves the evaluation order of a call's arguments unspecified (C11
6.5.2.2p10). A sibling nested call with any custody effect would therefore have to be
ordered against the enclosing call's other arguments, and this projection has
no such order to give. The admission avoids the question rather than answering
it: premises **N6** and **N7** — result mode `none`, every parameter mode
`borrowed` — make the nested call's declared effects the **identity**
(`neutralOutcome`), so `composeInnerFirst` and `composeOuterFirst` agree with
`applyCall outer` alone. Both directions are proved, and the *converse* is
proved too: a contract failing either premise is not neutral, exhibited
concretely, so the premises are load-bearing rather than decorative.

## What the tail declaration is, and is not

`declaredTail` is a **trusted assumption** about a callee: it says the callee
takes no custody of, and keeps no pointer to, any actual beyond the ones its
declaration names. Nothing here verifies a callee, and the declaration admits
nothing on its own — `admitsTail` is a conjunction, and `tailActualIsData`
refuses an actual that names anything the enclosing body may bind to a tracked
identity. `anExcessActualWithoutATailIsRefused` states the fail-closed repair:
without the declaration, an actual beyond the named vector is refused, never
dropped.

This is a model of the **decisions**, not of C semantics. That the reader
extracts these declaration facts faithfully, that a declared `borrowed` mode
really borrows, that a declared tail really takes no custody, and the recorded
C02-P may-alias boundary (**C02-P-ALIAS**), are all premises supplied from
outside and assumed here. Nothing here is a claim about arbitrary C memory
safety.
-/

namespace QCore.VariadicNested

/-! ## The declared vocabulary -/

/-- The v1 parameter modes. Exactly one of them discharges nothing, demands
    nothing and produces nothing. -/
inductive ParamMode where
  | borrowed
  | owned
  | consumed
  | released
  | stored
  | removed
  | out
  deriving DecidableEq, Repr

/-- The v1 result modes. `none` is the **only** non-custody result, which is why
    "a data-only result" and "a borrowed handback" are the same premise here. -/
inductive ResultMode where
  | none
  | owned
  | reallocated
  deriving DecidableEq, Repr

/-- One declared contract, plus the whole-declaration variadic-tail fact. The
    tail is a property of the *declaration*, not of any parameter. -/
structure Contract where
  modes : List ParamMode
  result : ResultMode
  /-- `resource_variadic_data_tail()` was declared. A trusted assumption. -/
  declaredTail : Bool
  deriving DecidableEq, Repr

/-! ## The abstract effect of one modeled call -/

/-- The abstract disposition of one tracked identity. -/
inductive Disposition where
  | live
  | released
  | escaped
  | withdrawn
  deriving DecidableEq, Repr

/-- The projection's state: one disposition per tracked identity. -/
abbrev State := List Disposition

/-- Replace the disposition at one index, leaving every other identity alone. -/
def setAt : State → Nat → Disposition → State
  | [], _, _ => []
  | _ :: rest, 0, d => d :: rest
  | h :: rest, n + 1, d => h :: setAt rest n d

/-- What one modeled call does: the post-state, the identities it **demands**
    be live here, and whether it **produced** a fresh identity a destination
    must bind. A call is neutral exactly when all three are trivial. -/
structure Outcome where
  state : State
  demanded : List Nat
  produced : Bool
  deriving DecidableEq, Repr

/-- The declared effect of one parameter position on one argument. -/
def paramEffect (m : ParamMode) (arg : Option Nat) (o : Outcome) : Outcome :=
  match m, arg with
  | ParamMode.borrowed, _ => o
  | ParamMode.owned, none => o
  | ParamMode.owned, some i => { o with demanded := i :: o.demanded }
  | ParamMode.out, _ => { o with produced := true }
  | ParamMode.released, none => o
  | ParamMode.released, some i =>
      { o with state := setAt o.state i Disposition.released,
               demanded := i :: o.demanded }
  | ParamMode.consumed, none => o
  | ParamMode.consumed, some i =>
      { o with state := setAt o.state i Disposition.escaped,
               demanded := i :: o.demanded }
  | ParamMode.stored, none => o
  | ParamMode.stored, some i =>
      { o with state := setAt o.state i Disposition.escaped,
               demanded := i :: o.demanded }
  | ParamMode.removed, none => o
  | ParamMode.removed, some i =>
      { o with state := setAt o.state i Disposition.withdrawn,
               demanded := i :: o.demanded }

/-- The declared effect of the result mode. -/
def resultEffect (r : ResultMode) (o : Outcome) : Outcome :=
  match r with
  | ResultMode.none => o
  | ResultMode.owned => { o with produced := true }
  | ResultMode.reallocated => { o with produced := true }

/-- Every declared effect of one modeled call, parameters before result, which
    is the order `apply_contract_call` uses. -/
def applyCall (c : Contract) (args : List (Option Nat)) (o : Outcome) : Outcome :=
  resultEffect c.result
    (List.foldl (fun acc (p : ParamMode × Option Nat) => paramEffect p.1 p.2 acc) o
      (c.modes.zip args))

/-- A call is **neutral** when its declared effects leave the outcome
    untouched: no state change, no liveness demand, nothing produced. -/
def isNeutral (c : Contract) (args : List (Option Nat)) : Prop :=
  ∀ o : Outcome, applyCall c args o = o

/-! ## The nested-call admission -/

/-- Where the nested call's result lands in the enclosing call. -/
inductive Slot where
  /-- A declared `borrowed` parameter of the enclosing contract. -/
  | borrowedParameter
  /-- A declared parameter that carries custody. -/
  | custodyParameter
  /-- Beyond the enclosing declaration's named vector. -/
  | beyondNamedVector
  deriving DecidableEq, Repr

/-- One candidate nested call, as the statement scan describes it. -/
structure Nested where
  /-- **N0** — every declaration of the enclosing function was read. -/
  declarationsRead : Bool
  /-- **N2** — this is a sibling argument call, not a call inside another
      nested call's arguments. B04 admits any number of such siblings. -/
  oneLevel : Bool
  /-- **N4** — the nested call is a *whole* argument of the enclosing call:
      its callee is the argument's first token, its `)` the last. -/
  wholeArgument : Bool
  /-- **N5** — the nested call's own argument list carries no call. -/
  innerArgumentsCallFree : Bool
  /-- **N3** — the nested callee has a contract in force. -/
  calleeContracted : Bool
  /-- **N1** — the enclosing callee has a contract in force. -/
  outerContracted : Bool
  /-- The nested callee's declared contract. -/
  contract : Contract
  /-- **N8** — the slot the nested result occupies. -/
  slot : Slot
  /-- The enclosing declaration declares a variadic data tail. -/
  outerDeclaredTail : Bool
  /-- The nested call's actual count equals its declared parameter count. -/
  innerArityExact : Bool
  /-- **N9** — every inner actual is either a place this projection can denote
      or data. Without it the admission is fail-*open*: `f (g (& x))` hands the
      callee an address that `canonicalPlace` refuses to denote and that the
      enclosing gate, exempted for the admitted nested argument, never sees. -/
  innerActualsDenotable : Bool
  deriving DecidableEq, Repr

/-- **N6 and N7**, the load-bearing pair: the nested contract's declared
    effects are the identity. -/
def nestedContractNeutral (c : Contract) : Bool :=
  (c.result == ResultMode.none) && c.modes.all (fun m => m == ParamMode.borrowed)

/-- **N8**: the slot the nested result occupies must need no identity. A
    `beyondNamedVector` slot needs the enclosing declaration's tail. -/
def slotIsData (n : Nested) : Bool :=
  match n.slot with
  | Slot.borrowedParameter => true
  | Slot.custodyParameter => false
  | Slot.beyondNamedVector => n.outerDeclaredTail

/-- The whitelist. Every clause is a conjunct, so failing any one refuses and
    the enclosing statement keeps card O04-B09B10's nested refusal. -/
def admitsNested (n : Nested) : Bool :=
  n.declarationsRead
    && n.oneLevel
    && n.wholeArgument
    && n.innerArgumentsCallFree
    && n.calleeContracted
    && n.outerContracted
    && n.innerArityExact
    && n.innerActualsDenotable
    && nestedContractNeutral n.contract
    && slotIsData n

/-! ## Every nested premise refuses on its own -/

theorem unread_declarations_never_admit_a_nested_call (n : Nested)
    (unread : n.declarationsRead = false) : admitsNested n = false := by
  simp [admitsNested, unread]

/-- Nesting inside a sibling's arguments remains refused. -/
theorem a_two_level_nested_call_is_never_admitted (n : Nested)
    (deeper : n.oneLevel = false) : admitsNested n = false := by
  simp [admitsNested, deeper]

theorem a_partial_nested_expression_is_never_admitted (n : Nested)
    (fragment : n.wholeArgument = false) : admitsNested n = false := by
  simp [admitsNested, fragment]

theorem a_call_inside_the_nested_arguments_is_never_admitted (n : Nested)
    (deeper : n.innerArgumentsCallFree = false) : admitsNested n = false := by
  simp [admitsNested, deeper]

/-- An inner call with no contract in force models nothing, which is exactly
    why the enclosing statement was refused in the first place. -/
theorem an_uncontracted_nested_callee_is_never_admitted (n : Nested)
    (uncontracted : n.calleeContracted = false) : admitsNested n = false := by
  simp [admitsNested, uncontracted]

theorem an_uncontracted_outer_callee_never_admits_a_nested_call (n : Nested)
    (uncontracted : n.outerContracted = false) : admitsNested n = false := by
  simp [admitsNested, uncontracted]

theorem a_nested_arity_mismatch_is_never_admitted (n : Nested)
    (mismatch : n.innerArityExact = false) : admitsNested n = false := by
  simp [admitsNested, mismatch]

/-- **N9**, the fail-open this card closed after finding it: an inner actual
    the projection can neither denote nor call data — `& x` above all — is
    refused. The enclosing `&` gate is exempted for the admitted nested
    argument, so without this premise nothing at all would see the escape. -/
theorem an_undenotable_inner_actual_is_never_admitted (n : Nested)
    (undenotable : n.innerActualsDenotable = false) : admitsNested n = false := by
  simp [admitsNested, undenotable]

/-! ## N6 and N7 — the neutrality premises -/

/-- **The rule the whole admission rests on, half one.** A nested call that
    produces custody is never admitted: its result would have to be bound, and
    an argument position binds nothing. -/
theorem an_owned_nested_result_is_never_admitted (n : Nested)
    (owned : n.contract.result = ResultMode.owned) : admitsNested n = false := by
  simp [admitsNested, nestedContractNeutral, owned]

theorem a_reallocating_nested_result_is_never_admitted (n : Nested)
    (realloc : n.contract.result = ResultMode.reallocated) :
    admitsNested n = false := by
  simp [admitsNested, nestedContractNeutral, realloc]

/-- **Half two**, quantified over the whole mode vocabulary so a later widening
    cannot slip a mode past it: any non-`borrowed` parameter refuses. -/
theorem a_custody_bearing_nested_parameter_is_never_admitted
    (n : Nested) (m : ParamMode) (present : m ∈ n.contract.modes)
    (custody : m ≠ ParamMode.borrowed) : admitsNested n = false := by
  have : n.contract.modes.all (fun m => m == ParamMode.borrowed) = false := by
    apply List.all_eq_false.mpr
    exact ⟨m, present, by simp [custody]⟩
  simp [admitsNested, nestedContractNeutral, this]

/-- The contrapositive: an admitted nested contract is neutral. -/
theorem an_admitted_nested_contract_is_neutral (n : Nested)
    (ok : admitsNested n = true) : nestedContractNeutral n.contract = true := by
  cases h : nestedContractNeutral n.contract with
  | false => simp [admitsNested, h] at ok
  | true => rfl

/-! ## N8 — the slot the result occupies -/

/-- A custody-bearing position never receives a nested call. -/
theorem a_custody_slot_never_admits_a_nested_call (n : Nested)
    (custody : n.slot = Slot.custodyParameter) : admitsNested n = false := by
  simp [admitsNested, slotIsData, custody]

/-- The two halves must compose: a nested call beyond the named vector needs
    the enclosing declaration's **tail**, and without it refuses. -/
theorem a_nested_call_beyond_the_named_vector_needs_a_declared_tail (n : Nested)
    (beyond : n.slot = Slot.beyondNamedVector) (noTail : n.outerDeclaredTail = false) :
    admitsNested n = false := by
  simp [admitsNested, slotIsData, beyond, noTail]

theorem an_admitted_nested_slot_is_data (n : Nested) (ok : admitsNested n = true) :
    slotIsData n = true := by
  cases h : slotIsData n with
  | false => simp [admitsNested, h] at ok
  | true => rfl

/-! ## The composition theorem -/

/-- A neutral contract's parameter fold is the identity. -/
theorem borrowed_fold_is_the_identity
    (ms : List ParamMode) (args : List (Option Nat)) (o : Outcome)
    (borrowed : ms.all (fun m => m == ParamMode.borrowed) = true) :
    List.foldl (fun acc (p : ParamMode × Option Nat) => paramEffect p.1 p.2 acc) o
      (ms.zip args) = o := by
  induction ms generalizing args o with
  | nil => simp
  | cons m rest ih =>
    cases args with
    | nil => simp
    | cons a more =>
      rw [List.all_cons, Bool.and_eq_true] at borrowed
      obtain ⟨head, tail⟩ := borrowed
      have hm : m = ParamMode.borrowed := by simpa using head
      simp [List.zip, List.zipWith, hm, paramEffect]
      exact ih more o tail

/-- **The composition premise, proved.** An admitted nested call's declared
    effects are the identity on the outcome. -/
theorem an_admitted_nested_call_is_neutral (c : Contract) (args : List (Option Nat))
    (neutral : nestedContractNeutral c = true) : isNeutral c args := by
  intro o
  rw [nestedContractNeutral, Bool.and_eq_true] at neutral
  obtain ⟨res, modes⟩ := neutral
  have hr : c.result = ResultMode.none := by simpa using res
  simp [applyCall, hr, resultEffect]
  exact borrowed_fold_is_the_identity c.modes args o modes

/-- **Inner-then-outer equals outer alone.** -/
theorem compose_inner_first (inner outer : Contract)
    (innerArgs outerArgs : List (Option Nat)) (o : Outcome)
    (neutral : nestedContractNeutral inner = true) :
    applyCall outer outerArgs (applyCall inner innerArgs o)
      = applyCall outer outerArgs o := by
  rw [an_admitted_nested_call_is_neutral inner innerArgs neutral o]

/-- **Outer-then-inner equals outer alone.** With both directions proved, the
    order C leaves unspecified cannot be observed by this projection — which is
    the entire justification for admitting a nested call at all. -/
theorem compose_outer_first (inner outer : Contract)
    (innerArgs outerArgs : List (Option Nat)) (o : Outcome)
    (neutral : nestedContractNeutral inner = true) :
    applyCall inner innerArgs (applyCall outer outerArgs o)
      = applyCall outer outerArgs o :=
  an_admitted_nested_call_is_neutral inner innerArgs neutral _

/-- The two orders agree. -/
theorem evaluation_order_is_unobservable (inner outer : Contract)
    (innerArgs outerArgs : List (Option Nat)) (o : Outcome)
    (neutral : nestedContractNeutral inner = true) :
    applyCall outer outerArgs (applyCall inner innerArgs o)
      = applyCall inner innerArgs (applyCall outer outerArgs o) := by
  rw [compose_inner_first inner outer innerArgs outerArgs o neutral,
      compose_outer_first inner outer innerArgs outerArgs o neutral]

/-! ## B04: an arbitrary sibling list -/

/-- The modeled calls paired with their written actuals. -/
abbrev ModeledCall := Contract × List (Option Nat)

/-- Apply the sibling calls in one possible C evaluation order. -/
def applyNestedCalls (calls : List ModeledCall) (o : Outcome) : Outcome :=
  calls.foldl (fun state call => applyCall call.1 call.2 state) o

/-- Every admitted sibling is neutral. This predicate deliberately quantifies
    over the complete list rather than a fixed two-call witness. -/
def allNestedCallsNeutral (calls : List ModeledCall) : Prop :=
  ∀ call ∈ calls, nestedContractNeutral call.1 = true

/-- Any finite list of admitted sibling calls is the identity. -/
theorem admitted_sibling_calls_are_the_identity (calls : List ModeledCall) (o : Outcome)
    (neutral : allNestedCallsNeutral calls) : applyNestedCalls calls o = o := by
  induction calls generalizing o with
  | nil => rfl
  | cons call rest ih =>
      simp only [applyNestedCalls, List.foldl_cons]
      rw [an_admitted_nested_call_is_neutral call.1 call.2 (neutral call (by simp)) o]
      apply ih
      intro member present
      exact neutral member (by simp [present])

/-- C may choose any sibling order. Because every admitted sibling list is the
    identity, any two orders agree; no chosen Rust iteration order is semantic. -/
theorem sibling_evaluation_order_is_unobservable
    (left right : List ModeledCall) (o : Outcome)
    (leftNeutral : allNestedCallsNeutral left)
    (rightNeutral : allNestedCallsNeutral right) :
    applyNestedCalls left o = applyNestedCalls right o := by
  rw [admitted_sibling_calls_are_the_identity left o leftNeutral,
      admitted_sibling_calls_are_the_identity right o rightNeutral]

/-! ## The converse — the neutrality premises are load-bearing -/

/-- An owned result is **not** neutral: it produces an identity that a
    destination would have to bind, and an argument position binds nothing. -/
theorem an_owned_result_is_not_neutral :
    ¬ isNeutral { modes := [], result := ResultMode.owned, declaredTail := false } [] := by
  intro h
  have := h { state := [], demanded := [], produced := false }
  simp [applyCall, resultEffect] at this

/-- A releasing parameter is **not** neutral: it changes a disposition, so
    ordering it against the enclosing call's other arguments would matter. -/
theorem a_releasing_parameter_is_not_neutral :
    ¬ isNeutral { modes := [ParamMode.released], result := ResultMode.none,
                  declaredTail := false } [some 0] := by
  intro h
  have := h { state := [Disposition.live], demanded := [], produced := false }
  simp [applyCall, resultEffect, paramEffect, setAt] at this

/-- An owned parameter is **not** neutral either: it demands liveness here, so
    admitting it would silently move an obligation across an unspecified
    evaluation order. This is why premise N7 says `borrowed`, not "changes no
    disposition". -/
theorem an_owned_parameter_is_not_neutral :
    ¬ isNeutral { modes := [ParamMode.owned], result := ResultMode.none,
                  declaredTail := false } [some 0] := by
  intro h
  have := h { state := [Disposition.live], demanded := [], produced := false }
  simp [applyCall, resultEffect, paramEffect] at this

/-! ## The variadic data tail -/

/-- One actual beyond the named parameter vector, as the walker reads it. -/
structure TailActual where
  /-- **T1** — the actual is nonempty. -/
  nonempty : Bool
  /-- **T2** — it carries `&`, `*`, `[`, `]`, `.` or `->`. -/
  carriesIndirection : Bool
  /-- **T3** — it carries a call the nested rule did not admit. -/
  carriesUnadmittedCall : Bool
  /-- **T4** — every identifier is an enumeration constant, a declared scalar,
      or a name C02-P's closure says this body can never bind to a tracked
      resource identity. -/
  everyNameIsData : Bool
  deriving DecidableEq, Repr

def tailActualIsData (a : TailActual) : Bool :=
  a.nonempty && !a.carriesIndirection && !a.carriesUnadmittedCall && a.everyNameIsData

/-- One call site's excess actuals, with the callee's declaration. -/
structure TailSite where
  /-- Every declaration of the enclosing function was read. -/
  declarationsRead : Bool
  /-- `resource_variadic_data_tail()` was declared for this callee. -/
  declaredTail : Bool
  /-- The actuals beyond the declared parameter vector. -/
  excess : List TailActual
  deriving DecidableEq, Repr

/-- The admission. Both halves are required: the declaration, **and** every
    excess actual being data at this call site. -/
def admitsTail (s : TailSite) : Bool :=
  s.declarationsRead && s.declaredTail && s.excess.all tailActualIsData

/-- **The arity repair**, stated as a theorem: with no declared tail, an actual
    beyond the named vector is refused. Before card O04-TRACE it was silently
    dropped, because the modeling zips the declared vector against the actuals. -/
theorem an_excess_actual_without_a_declared_tail_is_refused (s : TailSite)
    (noTail : s.declaredTail = false) : admitsTail s = false := by
  simp [admitsTail, noTail]

theorem unread_declarations_never_admit_a_tail (s : TailSite)
    (unread : s.declarationsRead = false) : admitsTail s = false := by
  simp [admitsTail, unread]

/-- **The declaration covers what it covers.** A tail actual naming something
    this body may bind to a tracked identity is refused *under* the
    declaration, exactly as it would be without one — which is the adversarial
    "a tail smuggling a custody pointer" row, proved. -/
theorem a_tail_actual_that_may_hold_an_identity_is_never_admitted
    (s : TailSite) (a : TailActual) (present : a ∈ s.excess)
    (risky : a.everyNameIsData = false) : admitsTail s = false := by
  have : s.excess.all tailActualIsData = false := by
    apply List.all_eq_false.mpr
    exact ⟨a, present, by simp [tailActualIsData, risky]⟩
  simp [admitsTail, this]

theorem a_tail_actual_taking_an_address_is_never_admitted
    (s : TailSite) (a : TailActual) (present : a ∈ s.excess)
    (indirect : a.carriesIndirection = true) : admitsTail s = false := by
  have : s.excess.all tailActualIsData = false := by
    apply List.all_eq_false.mpr
    exact ⟨a, present, by simp [tailActualIsData, indirect]⟩
  simp [admitsTail, this]

theorem a_tail_actual_carrying_an_unadmitted_call_is_never_admitted
    (s : TailSite) (a : TailActual) (present : a ∈ s.excess)
    (call : a.carriesUnadmittedCall = true) : admitsTail s = false := by
  have : s.excess.all tailActualIsData = false := by
    apply List.all_eq_false.mpr
    exact ⟨a, present, by simp [tailActualIsData, call]⟩
  simp [admitsTail, this]

theorem an_empty_tail_actual_is_never_admitted
    (s : TailSite) (a : TailActual) (present : a ∈ s.excess)
    (empty : a.nonempty = false) : admitsTail s = false := by
  have : s.excess.all tailActualIsData = false := by
    apply List.all_eq_false.mpr
    exact ⟨a, present, by simp [tailActualIsData, empty]⟩
  simp [admitsTail, this]

/-- The contrapositive: every admitted tail actual is data. -/
theorem every_admitted_tail_actual_is_data (s : TailSite) (ok : admitsTail s = true)
    (a : TailActual) (present : a ∈ s.excess) : tailActualIsData a = true := by
  cases refused : tailActualIsData a with
  | false =>
    have : s.excess.all tailActualIsData = false :=
      List.all_eq_false.mpr ⟨a, present, by simp [refused]⟩
    simp [admitsTail, this] at ok
  | true => rfl

/-- A declared tail alone admits nothing. -/
theorem the_declaration_alone_admits_nothing (a : TailActual)
    (risky : a.everyNameIsData = false) :
    admitsTail { declarationsRead := true, declaredTail := true, excess := [a] } = false :=
  a_tail_actual_that_may_hold_an_identity_is_never_admitted _ a (by simp) risky

/-! ## Non-vacuity, in both directions -/

/-- An admitted nested call exists, on both data slots. -/
def witnessNested (slot : Slot) : Nested :=
  { declarationsRead := true, oneLevel := true, wholeArgument := true,
    innerArgumentsCallFree := true, calleeContracted := true, outerContracted := true,
    contract := { modes := [ParamMode.borrowed], result := ResultMode.none,
                  declaredTail := false },
    slot := slot, outerDeclaredTail := true, innerArityExact := true,
    innerActualsDenotable := true }

theorem a_nested_call_in_a_declared_tail_is_admitted :
    admitsNested (witnessNested Slot.beyondNamedVector) = true := by decide

theorem a_nested_call_at_a_borrowed_parameter_is_admitted :
    admitsNested (witnessNested Slot.borrowedParameter) = true := by decide

theorem a_nested_call_at_a_custody_parameter_is_refused :
    admitsNested (witnessNested Slot.custodyParameter) = false := by decide

/-- Each syntactic premise, flipped separately on the same witness. -/
theorem the_nested_premises_are_separately_load_bearing :
    admitsNested { witnessNested Slot.beyondNamedVector with oneLevel := false } = false
      ∧ admitsNested { witnessNested Slot.beyondNamedVector with wholeArgument := false } = false
      ∧ admitsNested { witnessNested Slot.beyondNamedVector with
                       innerArgumentsCallFree := false } = false
      ∧ admitsNested { witnessNested Slot.beyondNamedVector with
                       calleeContracted := false } = false
      ∧ admitsNested { witnessNested Slot.beyondNamedVector with
                       innerArityExact := false } = false
      ∧ admitsNested { witnessNested Slot.beyondNamedVector with
                       innerActualsDenotable := false } = false
      ∧ admitsNested { witnessNested Slot.beyondNamedVector with
                       outerDeclaredTail := false } = false := by decide

/-- The declared modes, flipped separately on the same witness. -/
theorem the_nested_contract_premises_are_separately_load_bearing :
    admitsNested { witnessNested Slot.beyondNamedVector with
                   contract := { modes := [ParamMode.borrowed], result := ResultMode.owned,
                                 declaredTail := false } } = false
      ∧ admitsNested { witnessNested Slot.beyondNamedVector with
                       contract := { modes := [ParamMode.released], result := ResultMode.none,
                                     declaredTail := false } } = false := by decide

/-- An admitted tail exists, and each premise flips it. -/
def witnessTail : TailSite :=
  { declarationsRead := true, declaredTail := true,
    excess := [{ nonempty := true, carriesIndirection := false,
                 carriesUnadmittedCall := false, everyNameIsData := true }] }

theorem a_data_tail_is_admitted : admitsTail witnessTail = true := by decide

theorem the_tail_premises_are_separately_load_bearing :
    admitsTail { witnessTail with declarationsRead := false } = false
      ∧ admitsTail { witnessTail with declaredTail := false } = false
      ∧ admitsTail { witnessTail with
                     excess := [{ nonempty := true, carriesIndirection := true,
                                  carriesUnadmittedCall := false,
                                  everyNameIsData := true }] } = false
      ∧ admitsTail { witnessTail with
                     excess := [{ nonempty := true, carriesIndirection := false,
                                  carriesUnadmittedCall := true,
                                  everyNameIsData := true }] } = false
      ∧ admitsTail { witnessTail with
                     excess := [{ nonempty := true, carriesIndirection := false,
                                  carriesUnadmittedCall := false,
                                  everyNameIsData := false }] } = false
      ∧ admitsTail { witnessTail with
                     excess := [{ nonempty := false, carriesIndirection := false,
                                  carriesUnadmittedCall := false,
                                  everyNameIsData := true }] } = false := by decide

/-- A call site with no excess actuals is admitted vacuously *only* when the
    declaration is there; the empty tail is not a licence. -/
theorem an_empty_excess_still_needs_the_declaration :
    admitsTail { declarationsRead := true, declaredTail := false, excess := [] } = false := by
  decide

/-- The witness contract really is neutral, computed rather than assumed. -/
theorem the_witness_nested_contract_is_neutral :
    nestedContractNeutral (witnessNested Slot.beyondNamedVector).contract = true := by decide

end QCore.VariadicNested

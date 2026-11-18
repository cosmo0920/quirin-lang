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

import QCore.AppRkCallbacks
import QCore.VerdictSemigroup

/-!
# Callback slots of an opaque record (card OPENSSL-SSL-CTX-CALLBACK-SLOTS)

Gateway rule 12n (amended in place) and contract specification C04.19.3:
`resource_callback_slot_registered(P, TAG.FIELD, F)` and
`resource_callback_slot_invoked(OBJ, TAG.FIELD, A…)` state a callback slot of
a record the library keeps opaque (OpenSSL's `SSL_CTX`, whose public header
declares only `typedef struct ssl_ctx_st SSL_CTX;`). Abstract rule theorems,
not an extraction of the Rust analyzer
(`crates/qcore-c-gateway/src/member_run_facts.rs`, `RunMemberFacts::decides`;
`crates/qcore-c-gateway/src/legacy_callback_route.rs`,
`Parser::member_candidates`; the checker
`providers/quirin-provider-openssl/src/check.rs`).

* **When a slot decides.** A slot invocation is decided only over the run's
  facts (`a_slot_without_the_run_facts_never_decides`), only while no unit of
  the run defines the record (`a_slot_whose_record_a_unit_defines_never_decides`:
  a unit that does may store into it directly), only when a declaration
  registers the slot (`a_slot_nothing_registers_never_decides`: its writers
  are then undeclared, so even a closed empty set does not decide), and then
  exactly as a member over the run's set decides
  (`a_decided_slot_is_the_run_decision`): an open set refuses
  (`an_open_slot_never_decides`), a candidate with no contract in force or an
  effectful one refuses (`an_uncontracted_candidate_refuses`,
  `an_effectful_candidate_refuses`), and the identity needs every candidate
  to be the identity (`identity_needs_every_candidate_identity`).
* **The member form is unchanged.** A member invocation of a record no unit
  defines never decides, whatever registers its key
  (`the_member_form_never_decides_an_undefined_record`).
* **Every writer is a candidate.** The run's set of a slot reads every
  declared writer's actuals; when it closes, each writer's read value is a
  candidate (`every_declared_writers_value_is_a_candidate`). The checker
  requires every writer the slot's manual page names to be declared as such
  a setter (premise `P-OPAQUE-SLOT-WRITERS`), so no value the slot can hold
  is missed.
* **Never `Safe` by an undecided slot.** A row joining a refused slot
  invocation is never `Safe` (`a_refused_slot_never_publishes_safe`).
* **The checker's slot shapes.** A setter's accepted clauses state its slot
  registration and, with a retained argument, the retention, and release
  nothing (`a_registration_states_its_slot_and_releases_nothing`); a stated
  invocation is in every accepted declaration
  (`a_stated_invocation_is_in_every_accepted_declaration`), so a declaration
  dropping it is refused.

## What this is not

That the Rust reading of a unit's facts is this record-definition test, that
the declarations' unit carries every declared registration, and that the
manual page names every writer of the slot are premises, pinned by
`crates/qcore-c-gateway/tests/openssl_callback_slots.rs` and the crate's
checker tests.
-/

namespace QCore.OpaqueCallbackSlots

open QCore.CallbackContracts (Decision)
open QCore.OpCallbacks (Closure SuppliedClass decideSupplied)
open QCore.AppRkCallbacks (runDecision registeredClosure)
open QCore.VerdictSemigroup
open QCore.VerdictSemigroup.V

/-! ## When a slot decides -/

/-- What the run's facts say of one slot key: whether a unit defines its
record, whether a declaration registers it, and the run's closure of it. -/
structure SlotFacts where
  defined : Bool
  registered : Bool
  closure : Closure
  deriving DecidableEq, Repr

/-- `Parser::member_candidates` over a slot: no run facts, a defined record or
no registration refuses; otherwise the run's decision. -/
def slotDecision (facts : Option SlotFacts) (classOf : String → SuppliedClass) : Decision :=
  match facts with
  | none => .refused
  | some f => if f.defined || !f.registered then .refused else runDecision f.closure classOf

/-- The member form over the run's facts: decided only when the key
normalises over the records the run defines. -/
def memberFormDecision (normalises : Bool) (c : Closure) (classOf : String → SuppliedClass) :
    Decision :=
  if normalises then runDecision c classOf else .refused

theorem a_slot_without_the_run_facts_never_decides (classOf : String → SuppliedClass) :
    slotDecision none classOf = .refused := rfl

theorem a_slot_whose_record_a_unit_defines_never_decides (r : Bool) (c : Closure)
    (classOf : String → SuppliedClass) :
    slotDecision (some ⟨true, r, c⟩) classOf = .refused := by
  simp [slotDecision]

theorem a_slot_nothing_registers_never_decides (d : Bool) (c : Closure)
    (classOf : String → SuppliedClass) :
    slotDecision (some ⟨d, false, c⟩) classOf = .refused := by
  simp [slotDecision]

/-- Even a closed empty set: an unregistered slot's writers are unknown. -/
theorem an_unregistered_empty_slot_never_decides (d : Bool) (classOf : String → SuppliedClass) :
    slotDecision (some ⟨d, false, .closed []⟩) classOf = .refused :=
  a_slot_nothing_registers_never_decides d _ classOf

theorem a_decided_slot_is_the_run_decision (c : Closure) (classOf : String → SuppliedClass) :
    slotDecision (some ⟨false, true, c⟩) classOf = runDecision c classOf := by
  simp [slotDecision]

theorem an_open_slot_never_decides (d r : Bool) (classOf : String → SuppliedClass) :
    slotDecision (some ⟨d, r, .opened⟩) classOf = .refused := by
  cases d <;> cases r <;> simp [slotDecision, runDecision]

/-- The run's decision over a closed non-empty set refuses a candidate of the
given class. -/
private theorem refuses_class {names : List String} {classOf : String → SuppliedClass}
    {n : String} (hn : n ∈ names) (k : SuppliedClass)
    (hk : k = .missing ∨ k = .effectful) (hc : classOf n = k) :
    runDecision (.closed names) classOf = .refused := by
  cases names with
  | nil => cases hn
  | cons m ms =>
    show decideSupplied false ((m :: ms).map classOf) = .refused
    have mem : k ∈ (m :: ms).map classOf := List.mem_map.mpr ⟨n, hn, hc⟩
    unfold decideSupplied
    have ne : ¬((false || ((m :: ms).map classOf).isEmpty) = true) := by simp
    rcases hk with rfl | rfl
    · have this : ((m :: ms).map classOf).any (· == .missing) = true :=
        List.any_eq_true.mpr ⟨_, mem, by decide⟩
      simp only [ne, this, ↓reduceIte, ite_self]
    · have this : ((m :: ms).map classOf).any (· == .effectful) = true :=
        List.any_eq_true.mpr ⟨_, mem, by decide⟩
      by_cases hm : ((m :: ms).map classOf).any (· == .missing) = true
      · simp only [ne, hm, ↓reduceIte, ite_self]
      · simp only [ne, hm, this, ↓reduceIte, ite_self]

theorem an_uncontracted_candidate_refuses {names : List String}
    {classOf : String → SuppliedClass} {n : String} (hn : n ∈ names)
    (hc : classOf n = .missing) :
    slotDecision (some ⟨false, true, .closed names⟩) classOf = .refused := by
  rw [a_decided_slot_is_the_run_decision]
  exact refuses_class hn .missing (Or.inl rfl) hc

theorem an_effectful_candidate_refuses {names : List String}
    {classOf : String → SuppliedClass} {n : String} (hn : n ∈ names)
    (hc : classOf n = .effectful) :
    slotDecision (some ⟨false, true, .closed names⟩) classOf = .refused := by
  rw [a_decided_slot_is_the_run_decision]
  exact refuses_class hn .effectful (Or.inr rfl) hc

/-- The identity needs the run's facts, an undefined record, a registration,
a closed set, and no candidate that is uncontracted or effectful. -/
theorem identity_needs_every_candidate_identity {facts : Option SlotFacts}
    {classOf : String → SuppliedClass} (h : slotDecision facts classOf = .identity) :
    ∃ names, facts = some ⟨false, true, .closed names⟩ ∧
      ∀ n ∈ names, classOf n ≠ .missing ∧ classOf n ≠ .effectful := by
  rcases facts with _ | ⟨d, r, c⟩
  · cases h
  · cases d <;> cases r <;> simp [slotDecision] at h
    rcases c with names | _
    · refine ⟨names, rfl, fun n hn => ⟨fun hm => ?_, fun he => ?_⟩⟩
      · rw [refuses_class hn .missing (Or.inl rfl) hm] at h; cases h
      · rw [refuses_class hn .effectful (Or.inr rfl) he] at h; cases h
    · simp [runDecision] at h

theorem the_member_form_never_decides_an_undefined_record (c : Closure)
    (classOf : String → SuppliedClass) :
    memberFormDecision false c classOf = .refused := rfl

/-! ## Every writer is a candidate -/

/-- When the slot's run set closes over the units and every declared writer's
reading, each writer's read value is a candidate. -/
theorem every_declared_writers_value_is_a_candidate
    {units : List QCore.OpCallbacks.UnitReading} {writers : List Closure} {rs : List String}
    (h : registeredClosure units writers = .closed rs) {ns : List String}
    (hw : Closure.closed ns ∈ writers) {x : String} (hx : x ∈ ns) : x ∈ rs :=
  (QCore.AppRkCallbacks.the_registered_set_is_exactly_the_read_and_registered_sets h x).mpr
    ⟨ns, Or.inr hw, hx⟩

/-! ## Never `Safe` by an undecided slot -/

/-- The verdict a slot invocation contributes to its row: the identity
nothing, a refusal `Unknown` (a released element is not a slot's). -/
def invocationVerdict : Decision → V
  | .identity => .safe
  | .releasesElements => .unknown
  | .refused => .unknown

theorem a_refused_slot_never_publishes_safe (own : V) :
    join own (invocationVerdict .refused) ≠ .safe := by
  cases own <;> decide

theorem a_row_resting_on_a_slot_is_safe_only_if_it_decides (own : V) (d : Decision)
    (h : join own (invocationVerdict d) = .safe) : d = .identity := by
  cases d <;> cases own <;> simp_all [invocationVerdict, join, rank]

/-! ## The checker's slot shapes -/

/-- The clauses a slot shape adds (the selector, revision and range aside). -/
inductive Clause where
  | borrowed (p : Nat)
  | slotRegistered (holder callback : Nat) (slot : String)
  | slotInvoked (object : Nat) (slot : String) (arguments : List (Option Nat))
  | retainedBy (item holder : Nat)
  | released (p : Nat)
  | returnNone
  | unstatedPremise
  deriving DecidableEq, Repr

/-- `expected_clauses` for `RegistersSlot`: every parameter borrowed, the
registration, the retention when `arg` is kept, no result custody. -/
def registrationExpected (arity holder callback : Nat) (slot : String)
    (retains : Option Nat) : List Clause :=
  (List.range arity).map Clause.borrowed ++
    [.slotRegistered holder callback slot] ++
    (match retains with
     | some i => [.retainedBy i holder]
     | none => []) ++
    [.returnNone]

theorem a_registration_states_its_slot_and_releases_nothing (arity holder callback : Nat)
    (slot : String) (retains : Option Nat) (p : Nat) :
    Clause.slotRegistered holder callback slot ∈
        registrationExpected arity holder callback slot retains ∧
      (∀ i, retains = some i → Clause.retainedBy i holder ∈
        registrationExpected arity holder callback slot retains) ∧
      Clause.released p ∉ registrationExpected arity holder callback slot retains := by
  refine ⟨by simp [registrationExpected], fun i hi => by simp [registrationExpected, hi], ?_⟩
  cases retains <;> simp [registrationExpected]

/-- A plain call's expected clauses under its premises and stated
invocations (`expected_clauses` for `Plain`). -/
def plainExpected (arity : Nat) (premises : List Clause) : List Clause :=
  premises ++ (List.range arity).map Clause.borrowed ++ [.returnNone]

/-- The checker accepts exactly a permutation of the expected clauses. -/
def accepts (expected declaration : List Clause) : Prop := declaration.Perm expected

theorem a_stated_invocation_is_in_every_accepted_declaration {arity : Nat}
    {premises declaration : List Clause} (h : accepts (plainExpected arity premises) declaration)
    {o : Nat} {slot : String} {args : List (Option Nat)}
    (hp : Clause.slotInvoked o slot args ∈ premises) :
    Clause.slotInvoked o slot args ∈ declaration :=
  h.mem_iff.mpr (List.mem_append.mpr (Or.inl (List.mem_append.mpr (Or.inl hp))))

theorem the_unstated_premise_is_in_every_accepted_declaration {arity : Nat}
    {premises declaration : List Clause} (h : accepts (plainExpected arity premises) declaration)
    (hp : Clause.unstatedPremise ∈ premises) : Clause.unstatedPremise ∈ declaration :=
  h.mem_iff.mpr (List.mem_append.mpr (Or.inl (List.mem_append.mpr (Or.inl hp))))

end QCore.OpaqueCallbackSlots

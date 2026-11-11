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

import QCore.AggregateOwnedField

/-! The **copy bind's dichotomy** (card `OWNED-FIELD-ROUTE-REACH`, wave 41).

`QCore.AggregateOwnedField` proved wave 39's copy rule about the *type*: a
record declaring an owned field is not copyable, hollow or not. What that card
could not state — because no source program reached the rule — is the rule the
**route** applies when it decides whether to ask the question at all.

`STABILITY-REEVAL-4` measured that `OwnershipOp::Copy` was constructed at four
places in the tree, every one a unit test (`OWNERSHIP-COPY-OP-UNPRODUCED`).
This card gives it a source spelling, `let NAME = NAME;`, and with it a
decision the checker did not have to make before:

* if the name is one the **ledger holds a binding for**, the route builds
  `Copy` and the ownership checker answers — always with a refusal;
* if it is not, the route builds **no operation** and the copy is admitted.

The second half is the one that needs an argument, because it is the only
thing this card *admits* that was refused before (invariant 1). The argument
is here: a bound place is refused whatever it is bound to
(`copy_refuses_every_bound_place`), and an unbound place — under the
ledger-coherence invariant the Rust checker maintains — owns no resource, so
copying it moves no duty (`an_admitted_copy_owns_no_resource`) and leaves the
ledger exactly as it was (`an_admitted_step_is_the_identity_on_the_ledger`).
Together they say the decision is safe on both sides and total
(`the_decision_is_total`, `nothing_is_both_admitted_and_refused`).

**What is a premise and not a proof.** `LedgerCoherent` is the Rust
`Ledger::validate_invariant` clause "every resource entry's owner has a
binding", maintained by that checker after every step and unit-tested there.
It is assumed here, named, rather than re-derived: this module is about the
route's decision, not about the checker's own maintenance of its state.

**What is not modelled here at all.** The token reading that recognises the
statement (a tested Rust boundary), the emitted C struct assignment (the
sanitized native run), and the value half's environment. -/

namespace QCore.OwnedFieldCopyReach

open QCore.AggregateOwnedField

abbrev Name := String
abbrev ResourceId := Nat

/-- What a name is bound to, mirroring `ownership.rs`'s `Binding`. -/
inductive Binding where
  | owned (r : ResourceId)
  | aggregate (a : Aggregate)
  | borrowed (r : ResourceId)
  | moved
  | dropped
  deriving DecidableEq, Repr

/-- The refusals `copy_owned` can answer with, mirroring the Rust arms. -/
inductive CopyRefusal where
  | copyOfOwned (p : Name)
  | moveOfBorrow (p : Name)
  | useAfterMove (p : Name)
  | useAfterDrop (p : Name)
  deriving DecidableEq, Repr

/-- The bindings of one entry, by name. -/
abbrev Bindings := List (Name × Binding)

def lookup (b : Bindings) (n : Name) : Option Binding :=
  (b.find? (fun e => e.1 = n)).map Prod.snd

/-- **The copy rule as the IR states it**: `ownership.rs`'s `copy_owned`,
which takes a binding and answers. The result type carries a success case
precisely so that "it never succeeds" is a theorem with content rather than a
fact about the type. -/
def copyOwned (n : Name) : Binding → Except CopyRefusal Unit
  | .owned _ => .error (.copyOfOwned n)
  | .aggregate _ => .error (.copyOfOwned n)
  | .borrowed _ => .error (.moveOfBorrow n)
  | .moved => .error (.useAfterMove n)
  | .dropped => .error (.useAfterDrop n)

/-- **Every bound place refuses.** The total statement of the copy rule: the
refusal differs, the verdict never does. This is what makes the route's
"ask the IR about a ledger-held name" safe without any case analysis at the
route — a single arm turned into a success would break this theorem. -/
theorem copy_refuses_every_bound_place (n : Name) (b : Binding) :
    (copyOwned n b).toOption = none := by
  cases b <;> rfl

/-- Stated where it bites: an aggregate is refused whatever the state of its
fields — full, hollow, or declaring none — so the route needs no field walk to
decide a copy. -/
theorem an_aggregate_binding_is_refused_however_hollow (n f : Name) (r : ResourceId) :
    copyOwned n (.aggregate ⟨[⟨f, some r⟩]⟩) = .error (.copyOfOwned n)
      ∧ copyOwned n (.aggregate ⟨[⟨f, none⟩]⟩) = .error (.copyOfOwned n)
      ∧ copyOwned n (.aggregate ⟨[]⟩) = .error (.copyOfOwned n) :=
  ⟨rfl, rfl, rfl⟩

/-- And an owner is refused by the same arm, which is why one route statement
reaches both wave 39's aggregate rule and the older plain-owner one. -/
theorem an_owner_binding_is_refused_by_the_same_arm (n : Name) (r : ResourceId) :
    copyOwned n (.owned r) = copyOwned n (.aggregate ⟨[⟨"f", some r⟩]⟩) := rfl

/-! ## The route's decision -/

abbrev Ledger := List (ResourceId × Name)

/-- **The route's rule, as a step on the state.** Ask the IR exactly when the
bindings hold the source name; otherwise admit and change nothing.
`std_buffer.rs`'s custody walk, where the guard is
`self.names(source) || self.composed.contains_key(source)`, composed with
`check_ownership`'s answer. -/
def step (ledger : Ledger) (b : Bindings) (n : Name) : Except CopyRefusal Ledger :=
  match lookup b n with
  | some binding =>
    match copyOwned n binding with
    | .error refusal => .error refusal
    | .ok _ => .ok ledger
  | none => .ok ledger

/-- The route refuses exactly the names the bindings hold. -/
theorem the_route_refuses_exactly_the_bound_names
    (ledger : Ledger) (b : Bindings) (n : Name) :
    (step ledger b n).toOption = none ↔ (lookup b n).isSome := by
  unfold step
  cases h : lookup b n with
  | none => simp [Except.toOption]
  | some binding => cases binding <;> simp [copyOwned, Except.toOption]

/-- **The decision is total**: every name gets one of the two answers. -/
theorem the_decision_is_total (ledger : Ledger) (b : Bindings) (n : Name) :
    step ledger b n = .ok ledger ∨ (step ledger b n).toOption = none := by
  unfold step
  cases h : lookup b n with
  | none => exact Or.inl rfl
  | some binding => cases binding <;> simp [copyOwned, Except.toOption]

/-- **Nothing is both.** One question, one answer — which is why the value
half says nothing about a ledger-held name: one fault, one report. -/
theorem nothing_is_both_admitted_and_refused (ledger : Ledger) (b : Bindings) (n : Name) :
    ¬(step ledger b n = .ok ledger ∧ (step ledger b n).toOption = none) := by
  intro ⟨left, right⟩
  rw [left] at right
  simp [Except.toOption] at right

/-! ## The admitted side is safe

The ledger and the bindings are two halves of one state, and the Rust checker
maintains the invariant that every resource entry's owner is a name the
bindings hold. That is the premise; the conclusion is that a name the route
admits a copy of owns nothing. -/

/-- `Ledger::validate_invariant`'s clause, as a premise: every resource's
owner has a binding. -/
def LedgerCoherent (ledger : Ledger) (b : Bindings) : Prop :=
  ∀ e ∈ ledger, (lookup b e.2).isSome

/-- **The admitting side's safety.** A name the route admits a copy of owns no
resource, so the second binding receives no duty — which is exactly what
"a place with no binding owns nothing" has to mean for invariant 1 to hold
across this card's one new admission. -/
theorem an_admitted_copy_owns_no_resource
    (ledger : Ledger) (b : Bindings) (n : Name)
    (coherent : LedgerCoherent ledger b) (admitted : lookup b n = none) :
    ∀ e ∈ ledger, e.2 ≠ n := by
  intro e mem eq
  have bound : (lookup b n).isSome := by
    have holds := coherent e mem
    rwa [eq] at holds
  rw [admitted] at bound
  simp at bound

/-- And an admitted copy is the **identity** on the ledger: the route pushes
no operation for it at all, so no duty moves and no quantity changes. -/
theorem an_admitted_step_is_the_identity_on_the_ledger
    (ledger : Ledger) (b : Bindings) (n : Name) (admitted : lookup b n = none) :
    step ledger b n = .ok ledger := by
  unfold step
  rw [admitted]

/-- The two halves together, as the card's claim: for **every** state and
every name, either the copy is refused, or the name owns nothing and the
ledger is untouched. There is no third case, and no case in which a duty is
duplicated. -/
theorem a_copy_either_refuses_or_moves_no_duty
    (ledger : Ledger) (b : Bindings) (n : Name) (coherent : LedgerCoherent ledger b) :
    (step ledger b n).toOption = none
      ∨ (step ledger b n = .ok ledger ∧ ∀ e ∈ ledger, e.2 ≠ n) := by
  cases h : lookup b n with
  | some binding =>
    refine Or.inl ?_
    unfold step
    rw [h]
    cases binding <;> simp [copyOwned, Except.toOption]
  | none =>
    exact Or.inr
      ⟨an_admitted_step_is_the_identity_on_the_ledger ledger b n h,
       an_admitted_copy_owns_no_resource ledger b n coherent h⟩

end QCore.OwnedFieldCopyReach

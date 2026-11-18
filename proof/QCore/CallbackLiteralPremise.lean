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
# A literal-argument premise beside a synchronous invocation

Card **OPENSSL-PEM-CALLBACK-ARM** (gateway rule 57; contract specification
§C04.5 "Beside a synchronous invocation"; user decision 2026-11-18).

Two decisions, modelled as the Rust code makes them:

* **Admission** (`contract_loader.rs`, the retaining branch): a premise
  stands beside retaining clauses only when every one of them is a
  synchronous invocation (`resource_callback_invoked`).
* **Route G's rewrite** (`legacy_literal_premise::restrict_to_declared_literals`):
  a call whose premise is unmet, or any declared call under a poisoned
  premise set, is renamed to an undeclared callee before any reader. Before
  this card only callees in the function map were renamed; a callback
  declaration is withheld from that map, so the rewrite now also reaches a
  retaining declaration. The callback route then decides a call that kept
  its name exactly as rule 13u does (`QCore.CallbackContracts`): applied only
  over a closed candidate set whose every candidate is the identity.

The null default (user decision 2026-11-18): under
`resource_callback_invoked_or_default` a null pointer constant actual invokes
nothing, a non-null actual is decided exactly as before, the default never
crosses the premise, and a declaration carrying it is never forwarded.

Proved: an unmet premise withdraws a callback declaration whatever the
callback is (so nothing outside the premise is applied); a met premise
defers to the invocation decision unchanged (the premise adds no candidate
and no mode); applied implies every candidate is the identity; the new
rewrite refuses at least what the old one did and is unchanged on the
function map; the admission rejects every other retaining clause beside a
premise. `pem_*` are the PEM readers' vectors, including the gap the old
rewrite left.

## What this is not

Candidate resolution over Clang's AST, the identity classification of a
candidate's contract, the structural spelling relation of C04.5.1 and the
OpenSSL documentation are inputs, not modelled here (trust boundary 28).
-/

namespace QCore.CallbackLiteralPremise

/-- A function name. -/
abbrev Name := Nat

/-- One actual argument, as token codes (code `0` is the token `0`). -/
abbrev Tokens := List Nat

/-- A declared literal-argument premise. -/
structure Premise where
  index : Nat
  literal : Tokens
  deriving DecidableEq, Repr

/-- The retaining clause kinds the loader distinguishes here. -/
inductive Retaining where
  | invoked
  | table
  | registered
  | registry
  | member
  | retention
  | other
  deriving DecidableEq, Repr

/-- The loader's admission of a retaining declaration's premises. -/
def admits (premises : List Premise) (clauses : List Retaining) : Bool :=
  premises.isEmpty || clauses.all (fun clause => clause == .invoked)

/-- One call site. -/
structure Call where
  callee : Name
  arguments : List Tokens
  deriving DecidableEq, Repr

/-- A contract environment: the function map, the withheld retaining
declarations, the premises the texts declare and the poison flag. -/
structure Environment where
  functions : Name → Bool
  retaining : Name → Bool
  premises : Name → List Premise
  poisoned : Bool

/-- A premise is met only by a present actual that is exactly the literal. -/
def satisfies (arguments : List Tokens) (p : Premise) : Bool :=
  match arguments[p.index]? with
  | some actual => actual == p.literal
  | none => false

/-- Whether the premise set refuses this call's arguments. -/
def unmet (e : Environment) (c : Call) : Bool :=
  e.poisoned || (e.premises c.callee).any (fun p => !satisfies c.arguments p)

/-- The rewrite before this card: function-map callees only. -/
def renamedOld (e : Environment) (c : Call) : Bool :=
  e.functions c.callee && unmet e c

/-- The rewrite of this card: every declared callee, retaining ones included. -/
def renamed (e : Environment) (c : Call) : Bool :=
  (e.functions c.callee || e.retaining c.callee) && unmet e c

/-- A callback candidate as rule 13u classifies it. -/
inductive Candidate where
  | identity
  | effectful
  | uncontracted
  deriving DecidableEq, Repr

/-- Rule 11p's resolution of the callback actual. -/
inductive Resolution where
  | closed (candidates : List Candidate)
  | opened
  deriving DecidableEq, Repr

/-- What the call reads as. `undeclared` is an uncontracted callee (the
row's `Unknown`); `refused` a callback refusal; `applied` the declaration. -/
inductive Outcome where
  | undeclared
  | refused
  | applied
  deriving DecidableEq, Repr

/-- Rule 13u's decision of a synchronous invocation. -/
def decideInvocation : Resolution → Outcome
  | .opened => .refused
  | .closed candidates =>
      if candidates.all (fun c => c == .identity) then .applied else .refused

/-- The outcome of a call on route G after the rewrite. -/
def outcome (e : Environment) (c : Call) (r : Resolution) : Outcome :=
  if renamed e c then .undeclared
  else if e.retaining c.callee then decideInvocation r
  else if e.functions c.callee then .applied
  else .undeclared

/-! ### Admission -/

theorem a_premise_stands_only_beside_invocations {ps : List Premise} {cs : List Retaining}
    (h : admits ps cs = true) (hp : ps ≠ []) : ∀ c ∈ cs, c = .invoked := by
  intro c hc
  unfold admits at h
  have hne : ps.isEmpty = false := by
    cases ps with
    | nil => exact absurd rfl hp
    | cons _ _ => rfl
  rw [hne, Bool.false_or, List.all_eq_true] at h
  simpa using h c hc

theorem every_other_clause_beside_a_premise_rejects {ps : List Premise} {cs : List Retaining}
    {c : Retaining} (hc : c ∈ cs) (hk : c ≠ .invoked) (hp : ps ≠ []) :
    admits ps cs = false := by
  cases h : admits ps cs with
  | false => rfl
  | true => exact absurd (a_premise_stands_only_beside_invocations h hp c hc) hk

theorem invocations_with_a_premise_are_admitted {ps : List Premise} {n : Nat} :
    admits ps (List.replicate n .invoked) = true := by
  unfold admits
  simp

theorem no_premise_is_always_admitted {cs : List Retaining} : admits [] cs = true := by
  simp [admits]

/-! ### Withdrawal: nothing outside the premise is applied -/

theorem an_unmet_premise_withdraws_a_callback_declaration {e : Environment} {c : Call}
    {r : Resolution} (hr : e.retaining c.callee = true) (hu : unmet e c = true) :
    outcome e c r = .undeclared := by
  unfold outcome renamed
  simp [hr, hu]

theorem a_mismatched_actual_withdraws {e : Environment} {c : Call} {p : Premise}
    {r : Resolution} (hr : e.retaining c.callee = true) (hp : p ∈ e.premises c.callee)
    (hs : satisfies c.arguments p = false) : outcome e c r = .undeclared := by
  refine an_unmet_premise_withdraws_a_callback_declaration hr ?_
  unfold unmet
  simp only [Bool.or_eq_true]
  exact Or.inr (List.any_eq_true.mpr ⟨p, hp, by simp [hs]⟩)

theorem a_missing_actual_never_satisfies {arguments : List Tokens} {p : Premise}
    (h : arguments.length ≤ p.index) : satisfies arguments p = false := by
  unfold satisfies
  rw [List.getElem?_eq_none h]

theorem a_poisoned_set_withdraws_every_declared_callee {e : Environment} {c : Call}
    {r : Resolution} (hp : e.poisoned = true)
    (hd : (e.functions c.callee || e.retaining c.callee) = true) :
    outcome e c r = .undeclared := by
  unfold outcome renamed unmet
  simp [hp, hd]

/-! ### Within the premise the invocation decides, unchanged -/

theorem a_met_premise_defers_to_the_invocation {e : Environment} {c : Call} {r : Resolution}
    (hr : e.retaining c.callee = true) (hu : unmet e c = false) :
    outcome e c r = decideInvocation r := by
  unfold outcome renamed
  simp [hr, hu]

theorem applied_only_over_identities {e : Environment} {c : Call} {r : Resolution}
    (hr : e.retaining c.callee = true) (h : outcome e c r = .applied) :
    ∃ cs, r = .closed cs ∧ ∀ k ∈ cs, k = .identity := by
  cases hu : unmet e c with
  | true => rw [an_unmet_premise_withdraws_a_callback_declaration hr hu] at h; cases h
  | false =>
      rw [a_met_premise_defers_to_the_invocation hr hu] at h
      cases r with
      | opened => simp [decideInvocation] at h
      | closed cs =>
          refine ⟨cs, rfl, ?_⟩
          cases hall : cs.all (fun c => c == .identity) with
          | false => simp [decideInvocation, hall] at h
          | true =>
              intro k hk
              have := List.all_eq_true.mp hall k hk
              simpa using this

theorem an_open_set_refuses {e : Environment} {c : Call}
    (hr : e.retaining c.callee = true) (hu : unmet e c = false) :
    outcome e c .opened = .refused := by
  rw [a_met_premise_defers_to_the_invocation hr hu]; rfl

theorem an_uncontracted_candidate_refuses {e : Environment} {c : Call} {cs : List Candidate}
    (hr : e.retaining c.callee = true) (hu : unmet e c = false)
    (hk : Candidate.uncontracted ∈ cs) : outcome e c (.closed cs) = .refused := by
  rw [a_met_premise_defers_to_the_invocation hr hu]
  unfold decideInvocation
  have : cs.all (fun c => c == .identity) = false := by
    cases h : cs.all (fun c => c == .identity) with
    | false => rfl
    | true =>
        have := List.all_eq_true.mp h _ hk
        simp at this
  simp [this]

theorem an_effectful_candidate_refuses {e : Environment} {c : Call} {cs : List Candidate}
    (hr : e.retaining c.callee = true) (hu : unmet e c = false)
    (hk : Candidate.effectful ∈ cs) : outcome e c (.closed cs) = .refused := by
  rw [a_met_premise_defers_to_the_invocation hr hu]
  unfold decideInvocation
  have : cs.all (fun c => c == .identity) = false := by
    cases h : cs.all (fun c => c == .identity) with
    | false => rfl
    | true =>
        have := List.all_eq_true.mp h _ hk
        simp at this
  simp [this]

/-! ### The rewrite only grows, and only on withheld declarations -/

theorem the_new_rewrite_refuses_at_least_the_old {e : Environment} {c : Call}
    (h : renamedOld e c = true) : renamed e c = true := by
  unfold renamedOld at h
  unfold renamed
  simp only [Bool.and_eq_true, Bool.or_eq_true] at *
  exact ⟨Or.inl h.1, h.2⟩

theorem the_rewrite_is_unchanged_on_the_function_map {e : Environment} {c : Call}
    (hr : e.retaining c.callee = false) : renamed e c = renamedOld e c := by
  unfold renamed renamedOld
  simp [hr]

theorem no_premise_is_transparent {e : Environment} {c : Call}
    (hp : e.premises c.callee = []) (hq : e.poisoned = false) : renamed e c = false := by
  unfold renamed unmet
  simp [hp, hq]

/-! ### The PEM readers' vectors -/

/-- `PEM_read_bio_X509` (name `1`): premise `x = 0` at index 1, withheld. -/
def pemEnv : Environment where
  functions := fun _ => false
  retaining := fun n => n == 1
  premises := fun n => if n == 1 then [{ index := 1, literal := [0] }] else []
  poisoned := false

/-- `PEM_read_bio_X509(bio, NULL, cb, rk)`: token code `0` at `x`. -/
def pemNullX : Call := { callee := 1, arguments := [[7], [0], [8], [9]] }

/-- `PEM_read_bio_X509(in, &ca, cb, opaque)`: the reuse arm. -/
def pemReuse : Call := { callee := 1, arguments := [[7], [5, 6], [8], [9]] }

theorem pem_reuse_arm_is_withdrawn :
    outcome pemEnv pemReuse (.closed [.identity]) = .undeclared := by decide

theorem pem_old_rewrite_missed_the_reuse_arm : renamedOld pemEnv pemReuse = false := by decide

theorem pem_identity_callback_applies :
    outcome pemEnv pemNullX (.closed [.identity]) = .applied := by decide

theorem pem_uncontracted_callback_refuses :
    outcome pemEnv pemNullX (.closed [.uncontracted]) = .refused := by decide

theorem pem_null_callback_refuses : outcome pemEnv pemNullX .opened = .refused := by decide

/-! ### The null default (`resource_callback_invoked_or_default`)

The user decision of 2026-11-18 extends the card: a declaration may state
that a null callback invokes nothing (the callee's own default). The route
reads a callback actual either as a null pointer constant (Clang's
`null_leaf`) or as rule 11p's resolution of anything else. -/

/-- The callback actual at a call: a null pointer constant, or anything else
with its rule 11p resolution. -/
inductive Actual where
  | nullConstant
  | other (r : Resolution)
  deriving DecidableEq, Repr

/-- Without a null default a null constant is an open set (rule 13u). -/
def asResolution : Actual → Resolution
  | .nullConstant => .opened
  | .other r => r

/-- The invocation decision under the declaration's null-default flag. -/
def decideDefaulted (nullDefault : Bool) : Actual → Outcome
  | .nullConstant => if nullDefault then .applied else .refused
  | .other r => decideInvocation r

/-- The outcome of a call whose retaining declaration carries the flag. -/
def outcomeDefaulted (e : Environment) (c : Call) (nullDefault : Bool) (a : Actual) :
    Outcome :=
  if renamed e c then .undeclared
  else if e.retaining c.callee then decideDefaulted nullDefault a
  else if e.functions c.callee then .applied
  else .undeclared

theorem a_null_callback_under_the_default_invokes_nothing {e : Environment} {c : Call}
    (hr : e.retaining c.callee = true) (hu : unmet e c = false) :
    outcomeDefaulted e c true .nullConstant = .applied := by
  unfold outcomeDefaulted renamed
  simp [hr, hu, decideDefaulted]

theorem without_the_default_a_null_callback_refuses {e : Environment} {c : Call}
    (hr : e.retaining c.callee = true) (hu : unmet e c = false) :
    outcomeDefaulted e c false .nullConstant = .refused := by
  unfold outcomeDefaulted renamed
  simp [hr, hu, decideDefaulted]

theorem the_plain_invocation_is_the_flag_off {e : Environment} {c : Call} {a : Actual} :
    outcomeDefaulted e c false a = outcome e c (asResolution a) := by
  cases a <;> rfl

theorem a_non_null_callback_is_decided_as_before {e : Environment} {c : Call} {nd : Bool}
    {r : Resolution} : outcomeDefaulted e c nd (.other r) = outcome e c r := by
  rfl

theorem the_default_never_crosses_the_premise {e : Environment} {c : Call} {nd : Bool}
    {a : Actual} (hr : e.retaining c.callee = true) (hu : unmet e c = true) :
    outcomeDefaulted e c nd a = .undeclared := by
  unfold outcomeDefaulted renamed
  simp [hr, hu]

theorem applied_only_by_the_default_or_over_identities {e : Environment} {c : Call}
    {nd : Bool} {a : Actual} (hr : e.retaining c.callee = true)
    (h : outcomeDefaulted e c nd a = .applied) :
    (nd = true ∧ a = .nullConstant) ∨ ∃ cs, a = .other (.closed cs) ∧ ∀ k ∈ cs, k = .identity := by
  cases a with
  | nullConstant =>
      cases nd with
      | true => exact Or.inl ⟨rfl, rfl⟩
      | false =>
          cases hu : unmet e c with
          | true => rw [the_default_never_crosses_the_premise hr hu] at h; cases h
          | false => rw [without_the_default_a_null_callback_refuses hr hu] at h; cases h
  | other r =>
      rw [a_non_null_callback_is_decided_as_before] at h
      obtain ⟨cs, hcs, hall⟩ := applied_only_over_identities hr h
      exact Or.inr ⟨cs, by rw [hcs], hall⟩

/-- Rule 15w forwards a caller's own callback formal to a callee's stated
invocation only when neither declaration carries a null default. -/
def forwardable (ownDefault calleeDefault : Bool) : Bool := !ownDefault && !calleeDefault

theorem a_null_default_is_never_forwarded {own callee : Bool}
    (h : forwardable own callee = true) : own = false ∧ callee = false := by
  cases own <;> cases callee <;> simp_all [forwardable]

theorem pem_null_callback_with_the_default_applies :
    outcomeDefaulted pemEnv pemNullX true .nullConstant = .applied := by decide

theorem pem_null_callback_reuse_arm_is_withdrawn :
    outcomeDefaulted pemEnv pemReuse true .nullConstant = .undeclared := by decide

theorem pem_formal_callback_with_the_default_refuses :
    outcomeDefaulted pemEnv pemNullX true (.other .opened) = .refused := by decide

end QCore.CallbackLiteralPremise

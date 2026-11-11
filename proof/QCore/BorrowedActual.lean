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

import QCore.ExitResiduals

/-!
# The untracked-base closure of the borrowed pointer-field admission (C02-P)

`QCore.ExitResiduals` models the borrowed-actual whitelist itself and takes the
base-binding flag `Access.baseBound` as a premise supplied by the reader. This
module models the premise's *producer*: the closure
`crates/qcore-c-gateway/src/borrowed_actual.rs::untracked_bases` computes over
one function body, and which names it refuses.

The closure is the premise card C02 had to add after admitting a read through a
released pointer turned an honest `Unknown` into `Safe`
(`docs/reports/remaining-c02/README.md` §2). C02-P inherits it and needs it to
be *stronger*, because a borrowed actual survives into the modeled call instead
of disappearing into a no-op.

Two properties matter and are proved here:

* every syntactic way a name can take part in a custody change refuses it, and
  the refusal closes over assignments, so an admitted base is a name the body
  never binds;
* the refusal depends only on the *set* of facts, never on their order, which is
  the flow-insensitivity the Rust closure claims.

This is a model of the *decision*, not of C semantics. That the Rust reader
extracts these facts faithfully from C source is its own obligation, stated
here as `Faithful` and assumed; nothing here is a claim about arbitrary C
memory safety, and nothing here relates two distinct declared names that may
denote the same object — see the card's recorded may-alias boundary.
-/

namespace QCore.BorrowedActual

open QCore.LegacyCAnalyzer
open QCore.TypedFieldAccess
open QCore.ExitResiduals

/-- A name of one function body. -/
abbrev Name := Nat

/-- How one call argument is declared by the contract in force. `borrowed` is
    the only mode that leaves the argument's custody with the caller. -/
inductive ArgumentUse where
  | borrowed
  | owned
  | consumed
  | released
  | out
  | stored
  deriving DecidableEq

/-- Every mode but `borrowed` may change the argument's custody. An argument of
    a callee with no contract in force is modeled by `consumed`: the closure
    refuses it at every position. -/
def usesCustody : ArgumentUse → Bool
  | .borrowed => false
  | _ => true

/-- One syntactic fact of a function body that the closure reads. -/
inductive Fact where
  /-- `&name`, taking the address of the whole object. -/
  | addressTaken (n : Name)
  /-- `name++`, `--name` or `name op= …`. -/
  | mutated (n : Name)
  /-- `name` appears as a whole argument at a position declared `use`. -/
  | passedAt (n : Name) (use : ArgumentUse)
  /-- `name = …` from anything that is not an admitted field read over names
      that already survive. -/
  | assignedOpaque (dst : Name)
  /-- `dst = …` from an expression mentioning `src`. -/
  | assignedFrom (dst : Name) (src : Name)
  /-- `dst = base -> field`, an admitted direct member read. -/
  | assignedAdmittedRead (dst : Name) (base : Name)
  deriving DecidableEq

/-- The name one fact refuses on its own, if any. An admitted field read and a
    borrowed argument refuse nothing: without this the closure would be the
    everything-set and the admission vacuous. -/
def seeded : Fact → Option Name
  | .addressTaken n => some n
  | .mutated n => some n
  | .passedAt n use => if usesCustody use then some n else none
  | .assignedOpaque n => some n
  | .assignedFrom _ _ => none
  | .assignedAdmittedRead _ _ => none

/-- Which names a fact mentions at all. -/
def Mentions (n : Name) : Fact → Prop
  | .addressTaken m => m = n
  | .mutated m => m = n
  | .passedAt m _ => m = n
  | .assignedOpaque m => m = n
  | .assignedFrom dst src => dst = n ∨ src = n
  | .assignedAdmittedRead dst base => dst = n ∨ base = n

/-- The names of one body the projection may bind to a resource identity: the
    protected formals, every seeded name, and the destination of every
    assignment from a name already refused.

    Declaring it by membership rather than by an ordered fold is deliberate:
    the Rust closure is flow-insensitive, and `refusal_depends_only_on_membership`
    below is the statement of that. -/
inductive Risky (body : List Fact) (protectedNames : List Name) : Name → Prop where
  | ofProtected {n : Name} : n ∈ protectedNames → Risky body protectedNames n
  | ofSeed {f : Fact} {n : Name} :
      f ∈ body → seeded f = some n → Risky body protectedNames n
  | ofAssignment {dst src : Name} :
      Fact.assignedFrom dst src ∈ body →
        Risky body protectedNames src → Risky body protectedNames dst

/-- The Rust reader's obligation, stated rather than proved: the flag it puts on
    an access over-approximates the closure. -/
def Faithful (body : List Fact) (protectedNames : List Name) (base : Name)
    (a : Actual) : Prop :=
  Risky body protectedNames base → a.access.baseBound = true

/-! ## Every custody-taking use refuses its name -/

theorem a_protected_formal_is_refused {body : List Fact} {prot : List Name}
    {n : Name} (mem : n ∈ prot) : Risky body prot n :=
  .ofProtected mem

theorem an_address_taken_name_is_refused {body : List Fact} {prot : List Name}
    {n : Name} (mem : Fact.addressTaken n ∈ body) : Risky body prot n :=
  .ofSeed mem rfl

theorem a_mutated_name_is_refused {body : List Fact} {prot : List Name}
    {n : Name} (mem : Fact.mutated n ∈ body) : Risky body prot n :=
  .ofSeed mem rfl

theorem an_owned_argument_is_refused {body : List Fact} {prot : List Name}
    {n : Name} (mem : Fact.passedAt n ArgumentUse.owned ∈ body) :
    Risky body prot n :=
  .ofSeed mem rfl

theorem a_consumed_argument_is_refused {body : List Fact} {prot : List Name}
    {n : Name} (mem : Fact.passedAt n ArgumentUse.consumed ∈ body) :
    Risky body prot n :=
  .ofSeed mem rfl

theorem a_released_argument_is_refused {body : List Fact} {prot : List Name}
    {n : Name} (mem : Fact.passedAt n ArgumentUse.released ∈ body) :
    Risky body prot n :=
  .ofSeed mem rfl

theorem an_out_argument_is_refused {body : List Fact} {prot : List Name}
    {n : Name} (mem : Fact.passedAt n ArgumentUse.out ∈ body) :
    Risky body prot n :=
  .ofSeed mem rfl

theorem a_stored_argument_is_refused {body : List Fact} {prot : List Name}
    {n : Name} (mem : Fact.passedAt n ArgumentUse.stored ∈ body) :
    Risky body prot n :=
  .ofSeed mem rfl

theorem an_opaque_assignment_refuses_its_destination {body : List Fact}
    {prot : List Name} {n : Name} (mem : Fact.assignedOpaque n ∈ body) :
    Risky body prot n :=
  .ofSeed mem rfl

/-- The closure: refusal follows assignments, so a name that received a refused
    name is refused too. -/
theorem the_refusal_closes_over_assignments {body : List Fact} {prot : List Name}
    {dst src : Name} (mem : Fact.assignedFrom dst src ∈ body)
    (refused : Risky body prot src) : Risky body prot dst :=
  .ofAssignment mem refused

/-- And transitively, over any chain of them. -/
theorem the_refusal_closes_transitively {body : List Fact} {prot : List Name}
    {a b c : Name} (first : Fact.assignedFrom b a ∈ body)
    (second : Fact.assignedFrom c b ∈ body) (refused : Risky body prot a) :
    Risky body prot c :=
  .ofAssignment second (.ofAssignment first refused)

/-! ## The closure is a set, not a sequence -/

/-- The refusal depends only on which facts and protected names are present.
    This is the flow-insensitivity the Rust closure claims: a fact anywhere in
    the body refuses the name everywhere in it. -/
theorem refusal_depends_only_on_membership {body body' : List Fact}
    {prot prot' : List Name} {n : Name}
    (facts : ∀ f, f ∈ body → f ∈ body')
    (names : ∀ m, m ∈ prot → m ∈ prot')
    (refused : Risky body prot n) : Risky body' prot' n := by
  induction refused with
  | ofProtected mem => exact .ofProtected (names _ mem)
  | ofSeed mem seed => exact .ofSeed (facts _ mem) seed
  | ofAssignment mem _ ih => exact .ofAssignment (facts _ mem) ih

/-- Reordering the body changes nothing. -/
theorem the_refusal_ignores_statement_order {body body' : List Fact}
    {prot : List Name} {n : Name} (same : ∀ f, f ∈ body ↔ f ∈ body')
    (refused : Risky body prot n) : Risky body' prot n :=
  refusal_depends_only_on_membership (fun f mem => (same f).mp mem)
    (fun _ mem => mem) refused

/-- Adding a fact only refuses more names; the closure is monotone. -/
theorem adding_a_fact_only_refuses_more {body : List Fact} {prot : List Name}
    {f : Fact} {n : Name} (refused : Risky body prot n) :
    Risky (f :: body) prot n :=
  refusal_depends_only_on_membership (fun _ mem => List.mem_cons_of_mem _ mem)
    (fun _ mem => mem) refused

theorem protecting_more_formals_only_refuses_more {body : List Fact}
    {prot : List Name} {m : Name} {n : Name} (refused : Risky body prot n) :
    Risky body (m :: prot) n :=
  refusal_depends_only_on_membership (fun _ mem => mem)
    (fun _ mem => List.mem_cons_of_mem _ mem) refused

/-! ## The closure refuses nothing it was not given -/

theorem seeded_is_mentioned {f : Fact} {n : Name} (seed : seeded f = some n) :
    Mentions n f := by
  cases f with
  | addressTaken m => simp [seeded] at seed; simp [Mentions, seed]
  | mutated m => simp [seeded] at seed; simp [Mentions, seed]
  | passedAt m use =>
      cases use <;> simp [seeded, usesCustody] at seed <;> simp [Mentions, seed]
  | assignedOpaque m => simp [seeded] at seed; simp [Mentions, seed]
  | assignedFrom _ _ => simp [seeded] at seed
  | assignedAdmittedRead _ _ => simp [seeded] at seed

/-- A name no fact mentions and no formal protects is admissible as a base.
    Without this the closure could be the everything-set and every refusal
    theorem above would hold vacuously. -/
theorem an_unmentioned_name_is_not_refused {body : List Fact} {prot : List Name}
    {n : Name} (unmentioned : ∀ f, f ∈ body → ¬ Mentions n f)
    (unprotected : n ∉ prot) : ¬ Risky body prot n := by
  intro refused
  induction refused with
  | ofProtected mem => exact unprotected mem
  | ofSeed mem seed => exact unmentioned _ mem (seeded_is_mentioned seed)
  | ofAssignment mem _ _ =>
      exact unmentioned _ mem (Or.inl rfl)

theorem an_empty_body_refuses_nothing (n : Name) : ¬ Risky [] [] n :=
  an_unmentioned_name_is_not_refused (fun _ mem => by simp at mem) (by simp)

/-- Precision, first direction: a *borrowed* argument does not refuse its name.
    This is what lets the real constructor-to-caller path keep its base. -/
theorem a_borrowed_argument_alone_does_not_refuse_its_name (n : Name) :
    ¬ Risky [Fact.passedAt n ArgumentUse.borrowed] [] n := by
  intro refused
  cases refused with
  | ofProtected mem => simp at mem
  | ofSeed mem seed =>
      simp at mem
      subst mem
      simp [seeded, usesCustody] at seed
  | ofAssignment mem _ => simp at mem

/-- Precision, second direction: an admitted field read does not refuse the
    local it binds, which is the shape `storage = in->storage;` needs. -/
theorem an_admitted_read_alone_does_not_refuse_its_destination
    (dst base : Name) :
    ¬ Risky [Fact.assignedAdmittedRead dst base] [] dst := by
  intro refused
  cases refused with
  | ofProtected mem => simp at mem
  | ofSeed mem seed =>
      simp at mem
      subst mem
      simp [seeded] at seed
  | ofAssignment mem _ => simp at mem

/-! ## What the closure buys the admission -/

/-- The main consequence: a base the closure refuses is never admitted as an
    actual, so the borrowed pointer-field rule cannot read through a name this
    function may have bound, released or handed away. -/
theorem a_refused_base_is_never_admitted {body : List Fact} {prot : List Name}
    {base : Name} {a : Actual} (faithful : Faithful body prot base a)
    (refused : Risky body prot base) : admitsActual a = false :=
  a_bound_base_never_admits_an_actual a (faithful refused)

/-- Contrapositive: an admitted actual's base is one the closure did not
    refuse. -/
theorem an_admitted_actual_has_an_unrefused_base {body : List Fact}
    {prot : List Name} {base : Name} {a : Actual}
    (faithful : Faithful body prot base a) (ok : admitsActual a = true) :
    ¬ Risky body prot base := by
  intro refused
  rw [a_refused_base_is_never_admitted faithful refused] at ok
  exact Bool.noConfusion ok

/-- A released base keeps the statement refused, so the program that reads
    through it stays indeterminate. This is the behavior the fail-closed pin
    `unmodeled_memory_reads_do_not_become_neutral_effects` protects, restated
    for the actual-argument form. -/
theorem an_actual_through_a_released_base_stays_indeterminate
    {body : List Fact} {prot : List Name} {base : Name} {a : Actual}
    (resource : ResourceId) (faithful : Faithful body prot base a)
    (mem : Fact.passedAt base ArgumentUse.released ∈ body) :
    analyze [.acquire resource, .release resource,
      Instruction.unsupported] = .indeterminate := by
  have _refused : admitsActual a = false :=
    a_refused_base_is_never_admitted faithful (a_released_argument_is_refused mem)
  exact released_then_unmodeled_access_is_indeterminate resource

/-- And the statement carrying that actual is refused, whatever else it
    contains. -/
theorem a_statement_with_a_refused_base_is_refused {body : List Fact}
    {prot : List Name} {base : Name} {a : Actual} (other : Bool)
    (faithful : Faithful body prot base a) (refused : Risky body prot base) :
    classifyStatement [a] other = .refused :=
  an_unadmitted_actual_refuses_the_statement [a] other a (by simp)
    (a_refused_base_is_never_admitted faithful refused)

/-- Non-vacuity of the whole family: there is a body whose base the closure does
    not refuse and an actual the whitelist admits, so none of the refusals above
    is empty. -/
theorem the_admission_survives_the_closure :
    ∃ (body : List Fact) (prot : List Name) (base : Name) (a : Actual),
      ¬ Risky body prot base ∧ admitsActual a = true := by
  refine ⟨[Fact.assignedAdmittedRead 1 0], [], 1, {
    access := {
      record := { kind := TagKind.recordTag, complete := true, unique := true }
      member := MemberType.pointer
      qualified := false
      baseMatches := true
      baseBound := false
      baseResource := none
      chained := false
      escapes := false
      hasCall := true
      operandsScalar := false
      declarationsRead := true }
    position := Position.declared ParameterMode.borrowed
    wholeArgument := true }, ?_, ?_⟩
  · exact an_admitted_read_alone_does_not_refuse_its_destination 1 0
  · decide

end QCore.BorrowedActual

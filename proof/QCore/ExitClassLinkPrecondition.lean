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

import QCore.CarriedLinkObligation

/-!
# The link precondition without the unlink, per exit class (card LERI-HELD-FORMAL-LINK-OBLIGATION)

The abstract rule implemented by
`crates/qcore-c-gateway/src/legacy_carried_link.rs` (gateway rule 91 (5)),
with its hooks in `legacy_transfer.rs` (the caller's check before the call),
`legacy_summary.rs` (the W4 candidate) and `legacy_relational_summary.rs`
(the exit-specific route's agreement).

Rule 91's clause `resource_unpublished(p, M)` states two things at once: the
caller owes `p`'s `M` links valid on entry, and the call unlinks `p`. A body
that destroys its formal on one exit class and stores or keeps it on another
needs the first without the second. `resource_links_valid(p, M)` states the
entry precondition alone. Its body seeds the same `ready` record as rule 91
(`QCore.CarriedLinkObligation.the_seeded_record_holds` applies unchanged),
admits and forgets it the same way, and differs only at the exit and at the
caller:

* **The exit.** A formal the precondition alone carries is observed released
  only when its record is not `spent` (`observedReleasedFor`): a release
  through the carried destroyer unlinked it, which the clause does not state,
  so the contract does not validate (`a_precondition_release_is_not_the_destroyers`);
  rule 91's formals read as before (`a_destroyer_clause_reads_as_rule_91`),
  and every observation only refutes (`the_observation_only_refutes_for_every_entry`).
* **The classes.** An exit-specific summary is published only when every
  class states the same entry obligations (`classesAgree`). The caller's
  state at the call is one state whichever class returns, so a discharge
  checked on any edge establishes every class's seed
  (`one_discharge_seeds_every_class`); classes that disagree publish nothing
  (`disagreeing_classes_publish_nothing`).
* **The caller.** The caller discharges the precondition with an object it
  published, its own initialisation of `M`, or its own formal with an unspent
  `M` record (`preDischarges`); each establishes the precondition
  (`the_precondition_discharge_establishes_it`), a foreign element does not
  discharge it (`a_foreign_element_does_not_discharge_a_precondition`), and
  the check spends nothing (`checking_spends_nothing`).

## What this is not

A model of the decision. That a published object's links are valid
(P-PUBLISHED-LINKED, stated here as a hypothesis), that the candidates are
read off the destroyer calls and the precondition positions, that the Rust
kill policy over-approximates a statement's writes, that every consumer of a
contract with a link precondition checks it or refuses
(`has_container_effect`), that route G applies one class per edge from the
same pre-call state, and the correspondence with the compiled Rust are
review premises (trust boundary 28), pinned by
`crates/qcore-c-gateway/tests/link_member_premises.rs` (module
`carried_link_obligation`, `exit_class_fixtures`). The Rust `Owed m` record
is this model's `ready m` under an entry of kind `owed`.
-/

namespace QCore.ExitClassLinkPrecondition

open QCore.ElementLinkPremises
open QCore.CarriedLinkObligation

/-- Which clause seeded a formal's record at entry. -/
inductive EntryKind where
  /-- Rule 91: `resource_unpublished(p, M)`, the destroyer clause. -/
  | unlinks
  /-- Rule 91 (5): `resource_links_valid(p, M)`, the precondition alone. -/
  | owed
  deriving DecidableEq, Repr

-- The exit.

/-- `observed_for`: whether a disposition reads `Released`, by the entry's
clause (none when no clause carries the formal) and the record at the exit. -/
def observedReleasedFor : Option EntryKind → Option Carried → Bool → Bool
  | some .unlinks, r, released => observedReleased true r released
  | some .owed, r, released => released && !(match r with | some (.spent _) => true | _ => false)
  | none, r, released => observedReleased false r released

theorem a_precondition_release_is_not_the_destroyers (r : Option Carried) (released : Bool)
    (h : observedReleasedFor (some .owed) r released = true) : ∀ m, r ≠ some (.spent m) := by
  intro m hr
  subst hr
  simp [observedReleasedFor] at h

theorem a_destroyer_clause_reads_as_rule_91 (r : Option Carried) (released : Bool) :
    observedReleasedFor (some .unlinks) r released = observedReleased true r released := rfl

theorem an_uncarried_formal_reads_as_before (r : Option Carried) (released : Bool) :
    observedReleasedFor none r released = released := by
  simp [observedReleasedFor, observedReleased]

theorem the_observation_only_refutes_for_every_entry (k : Option EntryKind) (r : Option Carried)
    (released : Bool) (h : observedReleasedFor k r released = true) : released = true := by
  cases k with
  | none => simpa [observedReleasedFor, observedReleased] using h
  | some k =>
    cases k with
    | unlinks => exact the_observation_only_refutes true r released h
    | owed =>
      simp only [observedReleasedFor, Bool.and_eq_true] at h
      exact h.1

-- The classes.

/-- One exit class's entry obligations: the positions and members its
contract's destroyer clauses and preconditions carry. -/
abbrev Obligations := List (Nat × String)

/-- `classes_agree`: every class states the same entry obligations. -/
def classesAgree : List Obligations → Bool
  | [] => true
  | o :: os => os.all (fun p => p == o)

/-- The obligations every class's body run seeds, when the classes agree:
the first class's. -/
def published (classes : List Obligations) : Option Obligations :=
  if classesAgree classes then classes.head? else none

theorem disagreeing_classes_publish_nothing (classes : List Obligations)
    (h : classesAgree classes = false) : published classes = none := by
  simp [published, h]

theorem agreeing_classes_share_one_obligation (o : Obligations) (os : List Obligations)
    (h : classesAgree (o :: os) = true) : ∀ p ∈ os, p = o := by
  intro p hp
  simp only [classesAgree, List.all_eq_true, beq_iff_eq] at h
  exact h p hp

/-- A caller's discharge at the pre-call heap `h`: every obligation of the
class its edge selects holds there. -/
def discharged (h : Heap) (o : Obligations) : Prop :=
  ∀ c ∈ o, precondition h c.1 c.2

theorem one_discharge_seeds_every_class (h : Heap) (o : Obligations) (os : List Obligations)
    (agree : classesAgree (o :: os) = true) (edge : Obligations) (he : edge ∈ o :: os)
    (d : discharged h edge) : ∀ p ∈ o :: os, readyHolds (seed p) h := by
  have same : ∀ q ∈ o :: os, q = o := by
    intro q hq
    cases hq with
    | head => rfl
    | tail _ hq => exact agreeing_classes_share_one_obligation o os agree q hq
  intro p hp
  apply the_seeded_record_holds
  rw [same p hp]
  rw [same edge he] at d
  exact d

-- The caller.

/-- How a caller may pass the actual at a position carrying a precondition. -/
inductive PreDischarge where
  /-- The caller published the object into a container (rule 10d). -/
  | published
  /-- Rule 46 (3): this caller initialised `m` itself (a recorded fact). -/
  | initialised (facts : List (Nat × String))
  /-- Rule 91: the caller's own formal, with its record. -/
  | carried (r : Option Carried)
  /-- An element its list links through `m` (rule 46 (2)): not admitted for
  a precondition alone. -/
  | element (bindings : List String)
  /-- Anything else. -/
  | other

/-- `pre_decides`: the caller's decision at the call. -/
def preDischarges (m : String) (x : Nat) : PreDischarge → Bool
  | .published => true
  | .initialised facts => facts.contains (x, m)
  | .carried r => admits (some m) r
  | .element _ => false
  | .other => false

/-- P-PUBLISHED-LINKED, as this model states it: an object a caller
published into a container has valid links of the member it is linked by. -/
def publishedLinked (h : Heap) (x : Nat) (m : String) : Prop := unlinkDefined (h x m) = true

theorem the_precondition_discharge_establishes_it (h : Heap) (x : Nat) (m : String)
    (d : PreDischarge)
    (published_hold : d = .published → publishedLinked h x m)
    (facts_hold : ∀ facts, d = .initialised facts → holds facts h)
    (carried_hold : ∀ r, d = .carried r → ∀ n, r = some (.ready n) → unlinkDefined (h x n) = true)
    (hd : preDischarges m x d = true) : precondition h x m := by
  cases d with
  | published => exact published_hold rfl
  | initialised facts =>
    simp only [preDischarges, List.contains_iff_mem] at hd
    exact facts_hold facts rfl (x, m) hd
  | carried r =>
    cases r with
    | none => simp [preDischarges, admits] at hd
    | some r =>
      cases r with
      | spent n => simp [preDischarges, admits] at hd
      | ready n =>
        have hn : n = m := by
          have : m = n := by simpa [preDischarges, admits] using hd
          exact this.symm
        subst hn
        exact carried_hold _ rfl n rfl
  | element _ => simp [preDischarges] at hd
  | other => simp [preDischarges] at hd

theorem a_foreign_element_does_not_discharge_a_precondition (m : String) (x : Nat)
    (bindings : List String) : preDischarges m x (.element bindings) = false := rfl

theorem a_spent_record_does_not_discharge_a_precondition (m n : String) (x : Nat) :
    preDischarges m x (.carried (some (.spent n))) = false := rfl

/-- The caller's records after the check: unchanged, whatever it decided. -/
def afterCheck (rs : Records) : Records := rs

theorem checking_spends_nothing (rs : Records) (h : Heap) (hold : readyHolds rs h) :
    readyHolds (afterCheck rs) h := hold

-- The Rust vectors (`PRE_OBSERVE_VECTORS`, `PRE_DISCHARGE_VECTORS`, checked by
-- the unit tests of `crates/qcore-c-gateway/src/legacy_carried_link.rs`).
-- An entry code 0 is none, 1 `unlinks` (`Ready`), 2 `owed` (`Owed`). A
-- discharge record `(k, m)` is `ready m` for k = 0 or 1 (`Ready` or `Owed`)
-- and `spent m` for k = 2.

def entryOf : Nat → Option EntryKind
  | 1 => some .unlinks
  | 2 => some .owed
  | _ => none

def dischargeRecord : Option (Nat × String) → Option Carried
  | some (2, m) => some (.spent m)
  | some (_, m) => some (.ready m)
  | none => none

/-- The Rust decision (`pre_decides`) as the vectors read it: published,
initialised, then the record. -/
def preDecides (m : String) (published initialised : Bool) (r : Option Carried) : Bool :=
  published || initialised || admits (some m) r

theorem pre_decides_is_the_discharge (m : String) (x : Nat) (r : Option Carried) :
    preDecides m true false none = preDischarges m x .published ∧
    preDecides m false false r = preDischarges m x (.carried r) := by
  constructor
  · rfl
  · simp [preDecides, preDischarges]

def preObserveVectors : List (Nat × Option (Bool × String) × Bool × Bool) :=
  [ (2, some (true, "_head"), true, true),
    (2, none, true, true),
    (2, some (false, "_head"), true, false),
    (2, some (false, "_head"), false, false),
    (2, some (true, "_head"), false, false),
    (1, some (false, "_head"), true, true),
    (1, some (true, "_head"), true, false),
    (0, none, true, true) ]

def preDischargeVectors : List (Bool × Bool × Option (Nat × String) × Bool) :=
  [ (true, false, none, true),
    (false, true, none, true),
    (false, false, some (0, "_head"), true),
    (false, false, some (1, "_head"), true),
    (false, false, some (2, "_head"), false),
    (false, false, some (1, "_rotate"), false),
    (false, false, none, false) ]

theorem vectors_hold :
    preObserveVectors.all (fun (e, r, rel, out) =>
      observedReleasedFor (entryOf e) (record r) rel == out) = true ∧
    preDischargeVectors.all (fun (p, i, r, out) =>
      preDecides "_head" p i (dischargeRecord r) == out) = true := by
  decide

end QCore.ExitClassLinkPrecondition

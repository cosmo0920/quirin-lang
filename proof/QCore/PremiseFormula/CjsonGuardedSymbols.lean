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

import QCore.PremiseFormula.CjsonTypedAtoms
import QCore.GuardedContractApplications

/-!
# The cJSON guarded symbols as formulas (card SP6-GUARDED-SYMBOL-PROSE)

Contract specification C04.18.6 (guarded source embedding) and C04.18.9 (the
SP7 block). The 17 `resource_guarded_symbol_v1` symbols of
`cJSON_DetachItemViaPointer` state unquoted typed-rendering formulas. Eight of
them were English sentences.

`render g` interprets the registered terms in the call's **entry**
environment, exactly as `QCore.CjsonTypedAtoms.render (detachModel e)` does,
and interprets the SP7 terms as follows:

* `#item`, `#item_null()`, `#parent_null()`: the call's item and its two NULL
  flags (`Cjson.Call`);
* `#exit_X`: the registered `#X` read in the call's **exit** environment
  (`detachModel c.after`), and `#exit_parent` its parent;
* `#exit_erases(i)`: the exit child sequence is the entry sequence with `i`
  erased (`List.erase`);
* `#result`: the call's result (the registered term);
* `#admits_removed()`: the admission, a free proposition of the call;
* `#source_corresponds()`: the user's decision of 2026-11-23 (environment
  E2). It is a free proposition of the call that no theorem here
  establishes, and every call has a model in which it fails
  (`source_corresponds_is_never_derived`).
* `#caller_refutes_membership(p,i)`: the user's decision of 2026-11-23
  (invalid_membership I2). A warrant term inside a proposition: a free
  relation constrained only to entail non-ownership (`refutes_sound`), so
  `invalid_symbol_entails_invalid_membership` holds. Non-ownership alone does
  not give it (`absence_of_membership_is_not_refutation`).

The proofs establish the following. Each symbol holds exactly when its
definition does: the nine prerequisites are their
`CjsonRepresentationPremises` propositions, `failure_case` is `rawFailure`,
`original_list_unchanged` is `originalListUnchanged`, `remaining_list_valid`
is `remainingListValid`, `membership` is the call's membership conjuncts
(`prerequisites ↔ representation ∧ membership`), and `success_case` is
`prerequisites ∧ result = item`. A string is not a guarded proposition
(`QF003`): `guardedVectors` is bound to the Rust `GUARDED_DECISION_VECTORS`.

Nothing here establishes that a call model describes cJSON. The symbols stay
proposed metadata.
-/

namespace QCore.CjsonGuardedSymbols

open QCore.QuantProposition
open QCore.CjsonRepresentationPremises
open QCore.GuardedContractApplications.Cjson
open QCore.PremiseFormula (detachModel)
open QCore.CjsonTypedAtoms (Carrier chainAtom distinctAtom emptyAtom firstPrevAtom lastNextAtom
  linkTargetsAtom noWriterAtom parentOutsideAtom predecessorAtom)

/-- The SP7 block, in the order of the Rust table `RENDERED_TERMS`. -/
def sp7Terms : List String := ["item", "item_null", "parent_null", "exit_parent", "exit_live", "exit_owned", "exit_children", "exit_child", "exit_first", "exit_last", "exit_pred", "exit_next", "exit_prev", "exit_next_reach", "exit_nodup", "exit_runs", "exit_erases", "admits_removed", "source_corresponds", "caller_refutes_membership"]

/-! ## The call model -/

/-- A selected call: its entry and exit environments, item and NULL flags
(`Cjson.Call`), its result, and three propositions no source fact supplies:
the admission, the source correspondence (E2) and the caller's evidence
against membership (I2), which entails non-ownership and nothing more. -/
structure GuardedCall where
  call : Call
  result : Option Nat
  admitsRemoved : Prop
  sourceCorresponds : Prop
  callerRefutes : Nat → Nat → Prop
  refutes_sound : ∀ p i, callerRefutes p i → ¬ (detachModel call.entry).owned p i

/-- Each predicate parameter's model component: an entry term as in
`CjsonTypedAtoms.relation` over the entry model, an SP7 term as listed in the
module header. -/
def relation (g : GuardedCall) (name : String) :
    List (SemanticValue (Carrier (detachModel g.call.entry))) → Prop
  | [] =>
      (name = "item_null" ∧ g.call.nullItem = true) ∨
      (name = "parent_null" ∧ g.call.nullParent = true) ∨
      (name = "admits_removed" ∧ g.admitsRemoved) ∨
      (name = "source_corresponds" ∧ g.sourceCorresponds)
  | [.opaque (.obj x)] =>
      CjsonTypedAtoms.relation (detachModel g.call.entry) name [.opaque (.obj x)] ∨
      (name = "exit_live" ∧ (detachModel g.call.after).live x) ∨
      (name = "exit_nodup" ∧ (detachModel g.call.after).nodup x) ∨
      (name = "result" ∧ g.result = some x) ∨
      (name = "exit_erases" ∧ g.call.after.children = g.call.entry.children.erase x)
  | [.opaque (.obj o), .opaque (.obj x)] =>
      CjsonTypedAtoms.relation (detachModel g.call.entry) name [.opaque (.obj o), .opaque (.obj x)] ∨
      (name = "exit_owned" ∧ (detachModel g.call.after).owned o x) ∨
      (name = "exit_children" ∧ (detachModel g.call.after).children o x) ∨
      (name = "exit_child" ∧ (detachModel g.call.after).child o x) ∨
      (name = "exit_first" ∧ (detachModel g.call.after).first o x) ∨
      (name = "exit_last" ∧ (detachModel g.call.after).last o x) ∨
      (name = "exit_next" ∧ (detachModel g.call.after).next o x) ∨
      (name = "exit_prev" ∧ (detachModel g.call.after).prev o x) ∨
      (name = "exit_next_reach" ∧ (detachModel g.call.after).nextReach o x) ∨
      (name = "caller_refutes_membership" ∧ g.callerRefutes o x)
  | [.opaque (.obj o), .opaque (.obj b), .opaque (.obj a)] =>
      CjsonTypedAtoms.relation (detachModel g.call.entry) name
        [.opaque (.obj o), .opaque (.obj b), .opaque (.obj a)] ∨
      (name = "exit_pred" ∧ (detachModel g.call.after).pred o b a)
  | [.opaque (.fn f)] =>
      CjsonTypedAtoms.relation (detachModel g.call.entry) name [.opaque (.fn f)]
  | [.opaque (.fn f), .opaque (.run r)] =>
      CjsonTypedAtoms.relation (detachModel g.call.entry) name [.opaque (.fn f), .opaque (.run r)] ∨
      (name = "exit_runs" ∧ (detachModel g.call.after).runs f r)
  | [.opaque (.obj o), .opaque (.run r)] =>
      CjsonTypedAtoms.relation (detachModel g.call.entry) name [.opaque (.obj o), .opaque (.run r)]
  | _ => False

/-- The typed rendering of a call. -/
def render (g : GuardedCall) : Interpretation (Carrier (detachModel g.call.entry)) where
  sortHolds := CjsonTypedAtoms.sortHolds (detachModel g.call.entry)
  relation := relation g
  function := fun _ _ => none

/-- The three value parameters: `#parent`, `#item`, `#exit_parent`. -/
def valuation (g : GuardedCall) : String → SemanticValue (Carrier (detachModel g.call.entry)) :=
  fun name =>
    if name = "parent" then .opaque (.obj (detachModel g.call.entry).parent)
    else if name = "item" then .opaque (.obj g.call.item)
    else if name = "exit_parent" then .opaque (.obj (detachModel g.call.after).parent)
    else .integer 0

/-- A guarded symbol's formula holds of a call. -/
def Holds (g : GuardedCall) (e : Expr) : Prop := holds (render g) (valuation g) e

/-! ## The eight symbols that were English (the Rust reader's `lean` rendering of `cjson.qv`) -/

def detachSymbol : Expr := (.predicate "admits_removed" [])

def environmentSymbol : Expr := (.predicate "source_corresponds" [])

def failureCaseSymbol : Expr := (.or (.predicate "parent_null" []) (.or (.predicate "item_null" []) (.and (.not (.predicate "child" [(.var "parent"), (.var "item")])) (.forall_ "y" "obj" (.not (.predicate "prev" [(.var "item"), (.var "y")]))))))

def invalidMembershipSymbol : Expr := (.and (.and (.not (.predicate "parent_null" [])) (.not (.predicate "item_null" []))) (.predicate "caller_refutes_membership" [(.var "parent"), (.var "item")]))

def membershipSymbol : Expr := (.and (.and (.not (.predicate "parent_null" [])) (.not (.predicate "item_null" []))) (.and (.predicate "live" [(.var "parent")]) (.predicate "owned" [(.var "parent"), (.var "item")])))

def originalListUnchangedSymbol : Expr := (.and (.and (.forall_ "x" "obj" (.implies (.predicate "exit_child" [(.var "exit_parent"), (.var "x")]) (.predicate "child" [(.var "parent"), (.var "x")]))) (.forall_ "x" "obj" (.implies (.predicate "child" [(.var "parent"), (.var "x")]) (.predicate "exit_child" [(.var "exit_parent"), (.var "x")])))) (.and (.forall_ "o" "obj" (.forall_ "x" "obj" (.and (.implies (.predicate "exit_next" [(.var "o"), (.var "x")]) (.predicate "next" [(.var "o"), (.var "x")])) (.implies (.predicate "next" [(.var "o"), (.var "x")]) (.predicate "exit_next" [(.var "o"), (.var "x")]))))) (.forall_ "o" "obj" (.forall_ "x" "obj" (.and (.implies (.predicate "exit_prev" [(.var "o"), (.var "x")]) (.predicate "prev" [(.var "o"), (.var "x")])) (.implies (.predicate "prev" [(.var "o"), (.var "x")]) (.predicate "exit_prev" [(.var "o"), (.var "x")])))))))

def remainingListValidSymbol : Expr := (.and (.and (.and (.and (.and (.and (.and (.and (.and (.and (.and (.and (.and (.and (.and (.and (.predicate "exit_live" [(.var "exit_parent")]) (.forall_ "c" "obj" (.implies (.predicate "exit_owned" [(.var "exit_parent"), (.var "c")]) (.predicate "exit_children" [(.var "exit_parent"), (.var "c")])))) (.forall_ "c" "obj" (.implies (.predicate "exit_children" [(.var "exit_parent"), (.var "c")]) (.and (.predicate "exit_owned" [(.var "exit_parent"), (.var "c")]) (.predicate "exit_live" [(.var "c")]))))) (.forall_ "c" "obj" (.implies (.predicate "exit_child" [(.var "exit_parent"), (.var "c")]) (.predicate "exit_first" [(.var "exit_parent"), (.var "c")])))) (.forall_ "c" "obj" (.implies (.predicate "exit_first" [(.var "exit_parent"), (.var "c")]) (.predicate "exit_child" [(.var "exit_parent"), (.var "c")])))) (.forall_ "b" "obj" (.implies (.predicate "exit_children" [(.var "exit_parent"), (.var "b")]) (.forall_ "a" "obj" (.implies (.predicate "exit_pred" [(.var "exit_parent"), (.var "b"), (.var "a")]) (.predicate "exit_next" [(.var "a"), (.var "b")])))))) (.forall_ "c" "obj" (.implies (.predicate "exit_children" [(.var "exit_parent"), (.var "c")]) (.not (.predicate "exit_next_reach" [(.var "c"), (.var "c")]))))) (.predicate "exit_nodup" [(.var "exit_parent")])) (.and (.implies (.forall_ "c" "obj" (.not (.predicate "exit_child" [(.var "exit_parent"), (.var "c")]))) (.forall_ "c" "obj" (.not (.predicate "exit_children" [(.var "exit_parent"), (.var "c")])))) (.implies (.forall_ "c" "obj" (.not (.predicate "exit_children" [(.var "exit_parent"), (.var "c")]))) (.forall_ "c" "obj" (.not (.predicate "exit_child" [(.var "exit_parent"), (.var "c")])))))) (.forall_ "f" "obj" (.implies (.predicate "exit_first" [(.var "exit_parent"), (.var "f")]) (.forall_ "l" "obj" (.implies (.predicate "exit_last" [(.var "exit_parent"), (.var "l")]) (.predicate "exit_prev" [(.var "f"), (.var "l")])))))) (.forall_ "l" "obj" (.implies (.predicate "exit_last" [(.var "exit_parent"), (.var "l")]) (.forall_ "y" "obj" (.not (.predicate "exit_next" [(.var "l"), (.var "y")])))))) (.forall_ "c" "obj" (.implies (.predicate "exit_children" [(.var "exit_parent"), (.var "c")]) (.forall_ "y" "obj" (.implies (.or (.predicate "exit_next" [(.var "c"), (.var "y")]) (.predicate "exit_prev" [(.var "c"), (.var "y")])) (.and (.predicate "exit_children" [(.var "exit_parent"), (.var "y")]) (.predicate "exit_live" [(.var "y")]))))))) (.forall_ "f" "fn" (.implies (.predicate "concurrent" [(.var "f")]) (.forall_ "r" "run" (.implies (.predicate "exit_runs" [(.var "f"), (.var "r")]) (.predicate "keeps_links" [(.var "exit_parent"), (.var "r")])))))) (.not (.predicate "exit_children" [(.var "exit_parent"), (.var "exit_parent")]))) (.forall_ "b" "obj" (.implies (.predicate "exit_children" [(.var "exit_parent"), (.var "b")]) (.forall_ "a" "obj" (.implies (.predicate "exit_pred" [(.var "exit_parent"), (.var "b"), (.var "a")]) (.predicate "exit_prev" [(.var "b"), (.var "a")])))))) (.predicate "exit_erases" [(.var "item")])) (.and (.forall_ "y" "obj" (.not (.predicate "exit_next" [(.var "item"), (.var "y")]))) (.forall_ "y" "obj" (.not (.predicate "exit_prev" [(.var "item"), (.var "y")])))))

def successCaseSymbol : Expr := (.and (.and (.and (.and (.and (.and (.and (.and (.and (.and (.and (.and (.and (.and (.and (.and (.predicate "live" [(.var "parent")]) (.forall_ "c" "obj" (.implies (.predicate "owned" [(.var "parent"), (.var "c")]) (.predicate "children" [(.var "parent"), (.var "c")])))) (.forall_ "c" "obj" (.implies (.predicate "children" [(.var "parent"), (.var "c")]) (.and (.predicate "owned" [(.var "parent"), (.var "c")]) (.predicate "live" [(.var "c")]))))) (.forall_ "c" "obj" (.implies (.predicate "child" [(.var "parent"), (.var "c")]) (.predicate "first" [(.var "parent"), (.var "c")])))) (.forall_ "c" "obj" (.implies (.predicate "first" [(.var "parent"), (.var "c")]) (.predicate "child" [(.var "parent"), (.var "c")])))) (.forall_ "b" "obj" (.implies (.predicate "children" [(.var "parent"), (.var "b")]) (.forall_ "a" "obj" (.implies (.predicate "pred" [(.var "parent"), (.var "b"), (.var "a")]) (.predicate "next" [(.var "a"), (.var "b")])))))) (.forall_ "c" "obj" (.implies (.predicate "children" [(.var "parent"), (.var "c")]) (.not (.predicate "next_reach" [(.var "c"), (.var "c")]))))) (.predicate "nodup" [(.var "parent")])) (.and (.implies (.forall_ "c" "obj" (.not (.predicate "child" [(.var "parent"), (.var "c")]))) (.forall_ "c" "obj" (.not (.predicate "children" [(.var "parent"), (.var "c")])))) (.implies (.forall_ "c" "obj" (.not (.predicate "children" [(.var "parent"), (.var "c")]))) (.forall_ "c" "obj" (.not (.predicate "child" [(.var "parent"), (.var "c")])))))) (.forall_ "f" "obj" (.implies (.predicate "first" [(.var "parent"), (.var "f")]) (.forall_ "l" "obj" (.implies (.predicate "last" [(.var "parent"), (.var "l")]) (.predicate "prev" [(.var "f"), (.var "l")])))))) (.forall_ "l" "obj" (.implies (.predicate "last" [(.var "parent"), (.var "l")]) (.forall_ "y" "obj" (.not (.predicate "next" [(.var "l"), (.var "y")])))))) (.forall_ "c" "obj" (.implies (.predicate "children" [(.var "parent"), (.var "c")]) (.forall_ "y" "obj" (.implies (.or (.predicate "next" [(.var "c"), (.var "y")]) (.predicate "prev" [(.var "c"), (.var "y")])) (.and (.predicate "children" [(.var "parent"), (.var "y")]) (.predicate "live" [(.var "y")]))))))) (.forall_ "f" "fn" (.implies (.predicate "concurrent" [(.var "f")]) (.forall_ "r" "run" (.implies (.predicate "runs" [(.var "f"), (.var "r")]) (.predicate "keeps_links" [(.var "parent"), (.var "r")])))))) (.not (.predicate "children" [(.var "parent"), (.var "parent")]))) (.forall_ "b" "obj" (.implies (.predicate "children" [(.var "parent"), (.var "b")]) (.forall_ "a" "obj" (.implies (.predicate "pred" [(.var "parent"), (.var "b"), (.var "a")]) (.predicate "prev" [(.var "b"), (.var "a")])))))) (.and (.and (.not (.predicate "parent_null" [])) (.not (.predicate "item_null" []))) (.and (.predicate "live" [(.var "parent")]) (.predicate "owned" [(.var "parent"), (.var "item")])))) (.predicate "result" [(.var "item")]))

/-! ## The nine prerequisite atoms over the exit terms (each `#X` read as `#exit_X`) -/

def exitChainAtom : Expr := (.and (.and (.and (.and (.and (.and (.predicate "exit_live" [(.var "exit_parent")]) (.forall_ "c" "obj" (.implies (.predicate "exit_owned" [(.var "exit_parent"), (.var "c")]) (.predicate "exit_children" [(.var "exit_parent"), (.var "c")])))) (.forall_ "c" "obj" (.implies (.predicate "exit_children" [(.var "exit_parent"), (.var "c")]) (.and (.predicate "exit_owned" [(.var "exit_parent"), (.var "c")]) (.predicate "exit_live" [(.var "c")]))))) (.forall_ "c" "obj" (.implies (.predicate "exit_child" [(.var "exit_parent"), (.var "c")]) (.predicate "exit_first" [(.var "exit_parent"), (.var "c")])))) (.forall_ "c" "obj" (.implies (.predicate "exit_first" [(.var "exit_parent"), (.var "c")]) (.predicate "exit_child" [(.var "exit_parent"), (.var "c")])))) (.forall_ "b" "obj" (.implies (.predicate "exit_children" [(.var "exit_parent"), (.var "b")]) (.forall_ "a" "obj" (.implies (.predicate "exit_pred" [(.var "exit_parent"), (.var "b"), (.var "a")]) (.predicate "exit_next" [(.var "a"), (.var "b")])))))) (.forall_ "c" "obj" (.implies (.predicate "exit_children" [(.var "exit_parent"), (.var "c")]) (.not (.predicate "exit_next_reach" [(.var "c"), (.var "c")])))))

def exitDistinctAtom : Expr := (.predicate "exit_nodup" [(.var "exit_parent")])

def exitEmptyAtom : Expr := (.and (.implies (.forall_ "c" "obj" (.not (.predicate "exit_child" [(.var "exit_parent"), (.var "c")]))) (.forall_ "c" "obj" (.not (.predicate "exit_children" [(.var "exit_parent"), (.var "c")])))) (.implies (.forall_ "c" "obj" (.not (.predicate "exit_children" [(.var "exit_parent"), (.var "c")]))) (.forall_ "c" "obj" (.not (.predicate "exit_child" [(.var "exit_parent"), (.var "c")])))))

def exitFirstPrevAtom : Expr := (.forall_ "f" "obj" (.implies (.predicate "exit_first" [(.var "exit_parent"), (.var "f")]) (.forall_ "l" "obj" (.implies (.predicate "exit_last" [(.var "exit_parent"), (.var "l")]) (.predicate "exit_prev" [(.var "f"), (.var "l")])))))

def exitLastNextAtom : Expr := (.forall_ "l" "obj" (.implies (.predicate "exit_last" [(.var "exit_parent"), (.var "l")]) (.forall_ "y" "obj" (.not (.predicate "exit_next" [(.var "l"), (.var "y")])))))

def exitLinkTargetsAtom : Expr := (.forall_ "c" "obj" (.implies (.predicate "exit_children" [(.var "exit_parent"), (.var "c")]) (.forall_ "y" "obj" (.implies (.or (.predicate "exit_next" [(.var "c"), (.var "y")]) (.predicate "exit_prev" [(.var "c"), (.var "y")])) (.and (.predicate "exit_children" [(.var "exit_parent"), (.var "y")]) (.predicate "exit_live" [(.var "y")]))))))

def exitNoWriterAtom : Expr := (.forall_ "f" "fn" (.implies (.predicate "concurrent" [(.var "f")]) (.forall_ "r" "run" (.implies (.predicate "exit_runs" [(.var "f"), (.var "r")]) (.predicate "keeps_links" [(.var "exit_parent"), (.var "r")])))))

def exitParentOutsideAtom : Expr := (.not (.predicate "exit_children" [(.var "exit_parent"), (.var "exit_parent")]))

def exitPredecessorAtom : Expr := (.forall_ "b" "obj" (.implies (.predicate "exit_children" [(.var "exit_parent"), (.var "b")]) (.forall_ "a" "obj" (.implies (.predicate "exit_pred" [(.var "exit_parent"), (.var "b"), (.var "a")]) (.predicate "exit_prev" [(.var "b"), (.var "a")])))))

/-! ## The parts the proofs compose -/

/-- The nine prerequisite atoms, conjoined in the decomposition's order. -/
def nineEntry : Expr := (.and (.and (.and (.and (.and (.and (.and (.and chainAtom distinctAtom) emptyAtom) firstPrevAtom) lastNextAtom) linkTargetsAtom) noWriterAtom) parentOutsideAtom) predecessorAtom)

/-- The same nine atoms over the exit terms. -/
def nineExit : Expr := (.and (.and (.and (.and (.and (.and (.and (.and exitChainAtom exitDistinctAtom) exitEmptyAtom) exitFirstPrevAtom) exitLastNextAtom) exitLinkTargetsAtom) exitNoWriterAtom) exitParentOutsideAtom) exitPredecessorAtom)

theorem successCaseSymbol_eq :
    successCaseSymbol = .and (.and nineEntry membershipSymbol) (.predicate "result" [(.var "item")]) :=
  rfl

theorem remainingListValidSymbol_eq :
    remainingListValidSymbol =
      .and (.and nineExit (.predicate "exit_erases" [(.var "item")]))
        (.and (.forall_ "y" "obj" (.not (.predicate "exit_next" [(.var "item"), (.var "y")])))
          (.forall_ "y" "obj" (.not (.predicate "exit_prev" [(.var "item"), (.var "y")])))) :=
  rfl

theorem holds_and {α : Type} (m : Interpretation α) (v : String → SemanticValue α) (a b : Expr) :
    holds m v (.and a b) ↔ holds m v a ∧ holds m v b :=
  Iff.rfl

/-! ## What each symbol states of a call -/

section Readings
variable (g : GuardedCall)

@[simp] theorem render_relation : (render g).relation = relation g := rfl

@[simp] theorem sortMember_obj (v : SemanticValue (Carrier (detachModel g.call.entry))) :
    sortMember (render g) "obj" v ↔ ∃ u, v = .opaque (.obj u) := by
  simp [sortMember, render, CjsonTypedAtoms.sortHolds]

@[simp] theorem sortMember_fn (v : SemanticValue (Carrier (detachModel g.call.entry))) :
    sortMember (render g) "fn" v ↔ ∃ u, v = .opaque (.fn u) := by
  simp [sortMember, render, CjsonTypedAtoms.sortHolds]

@[simp] theorem sortMember_run (v : SemanticValue (Carrier (detachModel g.call.entry))) :
    sortMember (render g) "run" v ↔ ∃ r, v = .opaque (.run r) := by
  simp [sortMember, render, CjsonTypedAtoms.sortHolds]

/-! ### The nine prerequisites: their `CjsonRepresentationPremises` propositions at entry -/

theorem chain_holds : Holds g chainAtom ↔ chain g.call.entry := by
  rw [← CjsonTypedAtoms.chain_atom_iff, CjsonTypedAtoms.chain_atom_reads]
  simp [Holds, chainAtom, holds, evaluate, relation, CjsonTypedAtoms.relation, valuation,
    QuantProposition.bind]

theorem distinct_holds : Holds g distinctAtom ↔ distinct g.call.entry := by
  rw [← CjsonTypedAtoms.distinct_atom_iff, CjsonTypedAtoms.distinct_atom_reads]
  simp [Holds, distinctAtom, holds, evaluate, relation, CjsonTypedAtoms.relation, valuation]

theorem empty_holds : Holds g emptyAtom ↔ empty g.call.entry := by
  rw [← CjsonTypedAtoms.empty_atom_iff, CjsonTypedAtoms.empty_atom_reads]
  simp [Holds, emptyAtom, holds, evaluate, relation, CjsonTypedAtoms.relation, valuation,
    QuantProposition.bind]

theorem first_prev_holds : Holds g firstPrevAtom ↔ firstPrev g.call.entry := by
  rw [← CjsonTypedAtoms.first_prev_atom_iff, CjsonTypedAtoms.first_prev_atom_reads]
  simp [Holds, firstPrevAtom, holds, evaluate, relation, CjsonTypedAtoms.relation, valuation,
    QuantProposition.bind]

theorem last_next_holds : Holds g lastNextAtom ↔ lastNext g.call.entry := by
  rw [← CjsonTypedAtoms.last_next_atom_iff, CjsonTypedAtoms.last_next_atom_reads]
  simp [Holds, lastNextAtom, holds, evaluate, relation, CjsonTypedAtoms.relation, valuation,
    QuantProposition.bind]

theorem link_targets_holds : Holds g linkTargetsAtom ↔ linkTargets g.call.entry := by
  rw [← CjsonTypedAtoms.link_targets_atom_iff, CjsonTypedAtoms.link_targets_atom_reads]
  simp [Holds, linkTargetsAtom, holds, evaluate, relation, CjsonTypedAtoms.relation, valuation,
    QuantProposition.bind]

theorem no_writer_holds : Holds g noWriterAtom ↔ noWriter g.call.entry := by
  rw [← CjsonTypedAtoms.no_writer_atom_iff, CjsonTypedAtoms.no_writer_atom_reads]
  simp [Holds, noWriterAtom, holds, evaluate, relation, CjsonTypedAtoms.relation, valuation,
    QuantProposition.bind]

theorem parent_outside_holds : Holds g parentOutsideAtom ↔ parentOutside g.call.entry := by
  rw [← CjsonTypedAtoms.parent_outside_atom_iff, CjsonTypedAtoms.parent_outside_atom_reads]
  simp [Holds, parentOutsideAtom, holds, evaluate, relation, CjsonTypedAtoms.relation, valuation]

theorem predecessor_holds : Holds g predecessorAtom ↔ predecessor g.call.entry := by
  rw [← CjsonTypedAtoms.predecessor_atom_iff, CjsonTypedAtoms.predecessor_atom_reads]
  simp [Holds, predecessorAtom, holds, evaluate, relation, CjsonTypedAtoms.relation, valuation,
    QuantProposition.bind]

/-! ### The same nine over the exit terms: the propositions of the exit environment -/

theorem exit_chain_holds : Holds g exitChainAtom ↔ chain g.call.after := by
  rw [← CjsonTypedAtoms.chain_atom_iff, CjsonTypedAtoms.chain_atom_reads]
  simp [Holds, exitChainAtom, holds, evaluate, relation, CjsonTypedAtoms.relation, valuation,
    QuantProposition.bind]
  all_goals exact Iff.rfl

theorem exit_distinct_holds : Holds g exitDistinctAtom ↔ distinct g.call.after := by
  rw [← CjsonTypedAtoms.distinct_atom_iff, CjsonTypedAtoms.distinct_atom_reads]
  simp [Holds, exitDistinctAtom, holds, evaluate, relation, CjsonTypedAtoms.relation, valuation]
  all_goals exact Iff.rfl

theorem exit_empty_holds : Holds g exitEmptyAtom ↔ empty g.call.after := by
  rw [← CjsonTypedAtoms.empty_atom_iff, CjsonTypedAtoms.empty_atom_reads]
  simp [Holds, exitEmptyAtom, holds, evaluate, relation, CjsonTypedAtoms.relation, valuation,
    QuantProposition.bind]
  all_goals exact Iff.rfl

theorem exit_first_prev_holds : Holds g exitFirstPrevAtom ↔ firstPrev g.call.after := by
  rw [← CjsonTypedAtoms.first_prev_atom_iff, CjsonTypedAtoms.first_prev_atom_reads]
  simp [Holds, exitFirstPrevAtom, holds, evaluate, relation, CjsonTypedAtoms.relation, valuation,
    QuantProposition.bind]
  all_goals exact Iff.rfl

theorem exit_last_next_holds : Holds g exitLastNextAtom ↔ lastNext g.call.after := by
  rw [← CjsonTypedAtoms.last_next_atom_iff, CjsonTypedAtoms.last_next_atom_reads]
  simp [Holds, exitLastNextAtom, holds, evaluate, relation, CjsonTypedAtoms.relation, valuation,
    QuantProposition.bind]
  all_goals exact Iff.rfl

theorem exit_link_targets_holds : Holds g exitLinkTargetsAtom ↔ linkTargets g.call.after := by
  rw [← CjsonTypedAtoms.link_targets_atom_iff, CjsonTypedAtoms.link_targets_atom_reads]
  simp [Holds, exitLinkTargetsAtom, holds, evaluate, relation, CjsonTypedAtoms.relation, valuation,
    QuantProposition.bind]
  all_goals exact Iff.rfl

theorem exit_no_writer_holds : Holds g exitNoWriterAtom ↔ noWriter g.call.after := by
  rw [← CjsonTypedAtoms.no_writer_atom_iff, CjsonTypedAtoms.no_writer_atom_reads]
  simp [Holds, exitNoWriterAtom, holds, evaluate, relation, CjsonTypedAtoms.relation, valuation,
    QuantProposition.bind]
  all_goals exact Iff.rfl

theorem exit_parent_outside_holds : Holds g exitParentOutsideAtom ↔ parentOutside g.call.after := by
  rw [← CjsonTypedAtoms.parent_outside_atom_iff, CjsonTypedAtoms.parent_outside_atom_reads]
  simp [Holds, exitParentOutsideAtom, holds, evaluate, relation, CjsonTypedAtoms.relation, valuation]
  all_goals exact Iff.rfl

theorem exit_predecessor_holds : Holds g exitPredecessorAtom ↔ predecessor g.call.after := by
  rw [← CjsonTypedAtoms.predecessor_atom_iff, CjsonTypedAtoms.predecessor_atom_reads]
  simp [Holds, exitPredecessorAtom, holds, evaluate, relation, CjsonTypedAtoms.relation, valuation,
    QuantProposition.bind]
  all_goals exact Iff.rfl

theorem nine_entry_holds : Holds g nineEntry ↔ representation g.call.entry := by
  have h1 : holds (render g) (valuation g) chainAtom ↔ _ := chain_holds g
  have h2 : holds (render g) (valuation g) distinctAtom ↔ _ := distinct_holds g
  have h3 : holds (render g) (valuation g) emptyAtom ↔ _ := empty_holds g
  have h4 : holds (render g) (valuation g) firstPrevAtom ↔ _ := first_prev_holds g
  have h5 : holds (render g) (valuation g) lastNextAtom ↔ _ := last_next_holds g
  have h6 : holds (render g) (valuation g) linkTargetsAtom ↔ _ := link_targets_holds g
  have h7 : holds (render g) (valuation g) noWriterAtom ↔ _ := no_writer_holds g
  have h8 : holds (render g) (valuation g) parentOutsideAtom ↔ _ := parent_outside_holds g
  have h9 : holds (render g) (valuation g) predecessorAtom ↔ _ := predecessor_holds g
  unfold Holds nineEntry
  simp only [holds_and, h1, h2, h3, h4, h5, h6, h7, h8, h9, representation, and_assoc]

theorem nine_exit_holds : Holds g nineExit ↔ representation g.call.after := by
  have h1 : holds (render g) (valuation g) exitChainAtom ↔ _ := exit_chain_holds g
  have h2 : holds (render g) (valuation g) exitDistinctAtom ↔ _ := exit_distinct_holds g
  have h3 : holds (render g) (valuation g) exitEmptyAtom ↔ _ := exit_empty_holds g
  have h4 : holds (render g) (valuation g) exitFirstPrevAtom ↔ _ := exit_first_prev_holds g
  have h5 : holds (render g) (valuation g) exitLastNextAtom ↔ _ := exit_last_next_holds g
  have h6 : holds (render g) (valuation g) exitLinkTargetsAtom ↔ _ := exit_link_targets_holds g
  have h7 : holds (render g) (valuation g) exitNoWriterAtom ↔ _ := exit_no_writer_holds g
  have h8 : holds (render g) (valuation g) exitParentOutsideAtom ↔ _ := exit_parent_outside_holds g
  have h9 : holds (render g) (valuation g) exitPredecessorAtom ↔ _ := exit_predecessor_holds g
  unfold Holds nineExit
  simp only [holds_and, h1, h2, h3, h4, h5, h6, h7, h8, h9, representation, and_assoc]

/-! ### The eight symbols that were English -/

/-- `detach` (operation): the admission, nothing about the C body's checks. -/
theorem detach_holds : Holds g detachSymbol ↔ g.admitsRemoved := by
  simp [Holds, detachSymbol, holds, relation]

/-- `environment` (E2): the source correspondence, an independent assumption. -/
theorem environment_holds : Holds g environmentSymbol ↔ g.sourceCorresponds := by
  simp [Holds, environmentSymbol, holds, relation]

/-- `failure_case`: the raw body's early-return guard, `rawFailure`. -/
theorem failure_case_holds : Holds g failureCaseSymbol ↔ rawFailure g.call := by
  simp [Holds, failureCaseSymbol, holds, evaluate, relation, CjsonTypedAtoms.relation, valuation,
    QuantProposition.bind, rawFailure, Option.eq_none_iff_forall_ne_some]
  constructor
  · rintro (h | h | ⟨h1, h2⟩)
    · exact Or.inl h
    · exact Or.inr (Or.inl h)
    · exact Or.inr (Or.inr ⟨fun h => h1 ⟨rfl, h⟩, h2⟩)
  · rintro (h | h | ⟨h1, h2⟩)
    · exact Or.inl h
    · exact Or.inr (Or.inl h)
    · exact Or.inr (Or.inr ⟨fun h => h1 h.2, h2⟩)

/-- The call's membership conjuncts: neither argument NULL, the parent live,
the item an owned child of it at entry. -/
def membership (c : Call) : Prop :=
  c.nullParent = false ∧ c.nullItem = false ∧ c.entry.live c.entry.parent ∧
    c.entry.ownedChild c.item

theorem membership_holds : Holds g membershipSymbol ↔ membership g.call := by
  simp [Holds, membershipSymbol, holds, evaluate, relation, CjsonTypedAtoms.relation, valuation,
    membership]
  constructor
  · rintro ⟨⟨hp, hi⟩, hl, ho⟩
    exact ⟨hp, hi, hl, ho.2⟩
  · rintro ⟨hp, hi, hl, ho⟩
    exact ⟨⟨hp, hi⟩, hl, rfl, ho⟩

/-- `invalid_membership` (I2): neither argument NULL, and the caller's
evidence against the item's membership. -/
theorem invalid_membership_holds :
    Holds g invalidMembershipSymbol ↔
      g.call.nullParent = false ∧ g.call.nullItem = false ∧
        g.callerRefutes g.call.entry.parent g.call.item := by
  simp [Holds, invalidMembershipSymbol, holds, evaluate, relation, CjsonTypedAtoms.relation,
    valuation]
  exact ⟨fun ⟨⟨hp, hi⟩, hr⟩ => ⟨hp, hi, hr⟩, fun ⟨hp, hi, hr⟩ => ⟨⟨hp, hi⟩, hr⟩⟩

/-- `original_list_unchanged`: `originalListUnchanged`. -/
theorem original_list_unchanged_holds :
    Holds g originalListUnchangedSymbol ↔ originalListUnchanged g.call := by
  simp only [Holds, originalListUnchangedSymbol, originalListUnchanged]
  simp [holds, evaluate, relation, CjsonTypedAtoms.relation, valuation, QuantProposition.bind]
  constructor
  · rintro ⟨⟨hc1, hc2⟩, hn, hp⟩
    refine ⟨QCore.PremiseFormula.option_eq_of_values (fun u h => (hc1 u ⟨rfl, h⟩).2)
        (fun u h => (hc2 u ⟨rfl, h⟩).2), funext fun o => ?_, funext fun o => ?_⟩
    · exact QCore.PremiseFormula.option_eq_of_values (fun u h => (hn o u).1 h)
        (fun u h => (hn o u).2 h)
    · exact QCore.PremiseFormula.option_eq_of_values (fun u h => (hp o u).1 h)
        (fun u h => (hp o u).2 h)
  · rintro ⟨hc, hn, hp⟩
    refine ⟨⟨fun u h => ⟨rfl, ?_⟩, fun u h => ⟨rfl, ?_⟩⟩, fun o u => ⟨fun h => ?_, fun h => ?_⟩,
      fun o u => ⟨fun h => ?_, fun h => ?_⟩⟩
    · exact (show g.call.entry.child = some u from hc ▸ h.2)
    · exact (show g.call.after.child = some u from hc.symm ▸ h.2)
    · exact (show g.call.entry.next o = some u from hn ▸ h)
    · exact (show g.call.after.next o = some u from hn.symm ▸ h)
    · exact (show g.call.entry.prev o = some u from hp ▸ h)
    · exact (show g.call.after.prev o = some u from hp.symm ▸ h)

/-- `remaining_list_valid`: `remainingListValid`. -/
theorem remaining_list_valid_holds :
    Holds g remainingListValidSymbol ↔ remainingListValid g.call := by
  have hn : holds (render g) (valuation g) nineExit ↔ _ := nine_exit_holds g
  have he : holds (render g) (valuation g) (.predicate "exit_erases" [(.var "item")]) ↔
      g.call.after.children = g.call.entry.children.erase g.call.item := by
    simp [holds, evaluate, relation, CjsonTypedAtoms.relation, valuation]
  have hx : holds (render g) (valuation g)
      (.forall_ "y" "obj" (.not (.predicate "exit_next" [(.var "item"), (.var "y")]))) ↔
      g.call.after.next g.call.item = none := by
    simp [holds, evaluate, relation, CjsonTypedAtoms.relation, valuation, QuantProposition.bind,
      Option.eq_none_iff_forall_ne_some]
    all_goals exact Iff.rfl
  have hp : holds (render g) (valuation g)
      (.forall_ "y" "obj" (.not (.predicate "exit_prev" [(.var "item"), (.var "y")]))) ↔
      g.call.after.prev g.call.item = none := by
    simp [holds, evaluate, relation, CjsonTypedAtoms.relation, valuation, QuantProposition.bind,
      Option.eq_none_iff_forall_ne_some]
    all_goals exact Iff.rfl
  unfold Holds
  rw [remainingListValidSymbol_eq, holds_and, holds_and, holds_and, hn, he, hx, hp]
  simp only [remainingListValid, and_assoc]

/-- `success_case` (R1): every representation and containment prerequisite,
and the call returns the same item. -/
def successCase (g : GuardedCall) : Prop :=
  prerequisites g.call ∧ g.result = some g.call.item

/-- The prerequisites are the representation and the membership conjuncts
(the membership's live parent is the representation's too). -/
theorem prerequisites_iff (c : Call) : prerequisites c ↔ representation c.entry ∧ membership c := by
  constructor
  · rintro ⟨hr, ho, hp, hi⟩
    exact ⟨hr, hp, hi, hr.1.1, ho⟩
  · rintro ⟨hr, hp, hi, _, ho⟩
    exact ⟨hr, ho, hp, hi⟩

theorem success_case_holds : Holds g successCaseSymbol ↔ successCase g := by
  have hn : holds (render g) (valuation g) nineEntry ↔ _ := nine_entry_holds g
  have hm : holds (render g) (valuation g) membershipSymbol ↔ _ := membership_holds g
  have hr : holds (render g) (valuation g) (.predicate "result" [(.var "item")]) ↔
      g.result = some g.call.item := by
    simp [holds, evaluate, relation, CjsonTypedAtoms.relation, valuation]
    all_goals exact Iff.rfl
  unfold Holds
  rw [successCaseSymbol_eq, holds_and, holds_and, hn, hm, hr, successCase, prerequisites_iff]

end Readings

/-! ## Faithfulness, collected -/

/-- **The 17 guarded symbols are their definitions.** -/
theorem guarded_symbols_are_the_definitions (g : GuardedCall) :
    (Holds g detachSymbol ↔ g.admitsRemoved) ∧
    (Holds g environmentSymbol ↔ g.sourceCorresponds) ∧
    (Holds g failureCaseSymbol ↔ rawFailure g.call) ∧
    (Holds g invalidMembershipSymbol ↔
      g.call.nullParent = false ∧ g.call.nullItem = false ∧
        g.callerRefutes g.call.entry.parent g.call.item) ∧
    (Holds g membershipSymbol ↔ membership g.call) ∧
    (Holds g originalListUnchangedSymbol ↔ originalListUnchanged g.call) ∧
    (Holds g remainingListValidSymbol ↔ remainingListValid g.call) ∧
    (Holds g successCaseSymbol ↔ successCase g) ∧
    (Holds g chainAtom ↔ chain g.call.entry) ∧
    (Holds g distinctAtom ↔ distinct g.call.entry) ∧
    (Holds g emptyAtom ↔ empty g.call.entry) ∧
    (Holds g firstPrevAtom ↔ firstPrev g.call.entry) ∧
    (Holds g lastNextAtom ↔ lastNext g.call.entry) ∧
    (Holds g linkTargetsAtom ↔ linkTargets g.call.entry) ∧
    (Holds g noWriterAtom ↔ noWriter g.call.entry) ∧
    (Holds g parentOutsideAtom ↔ parentOutside g.call.entry) ∧
    (Holds g predecessorAtom ↔ predecessor g.call.entry) :=
  ⟨detach_holds g, environment_holds g, failure_case_holds g, invalid_membership_holds g,
    membership_holds g, original_list_unchanged_holds g, remaining_list_valid_holds g,
    success_case_holds g, chain_holds g, distinct_holds g, empty_holds g, first_prev_holds g,
    last_next_holds g, link_targets_holds g, no_writer_holds g, parent_outside_holds g,
    predecessor_holds g⟩

/-! ## The two assumptions no model derives (user decisions 2026-11-23) -/

/-- The caller's evidence against membership entails the existing
`invalidMembership`: the I2 symbol is never weaker than the I1 one. -/
theorem invalid_symbol_entails_invalid_membership (g : GuardedCall)
    (h : Holds g invalidMembershipSymbol) : invalidMembership g.call := by
  rw [invalid_membership_holds] at h
  obtain ⟨hp, hi, hr⟩ := h
  exact ⟨hp, hi, fun howned => g.refutes_sound _ _ hr ⟨rfl, howned⟩⟩

/-- Admitted membership and the caller's refutation exclude each other. -/
theorem membership_excludes_refutation (g : GuardedCall) (h : Holds g membershipSymbol) :
    ¬ Holds g invalidMembershipSymbol := by
  intro hi
  rw [membership_holds] at h
  exact (invalid_symbol_entails_invalid_membership g hi).2.2 h.2.2.2

/-- A call of any shape with neither assumption. -/
def bare (c : Call) : GuardedCall where
  call := c
  result := none
  admitsRemoved := False
  sourceCorresponds := False
  callerRefutes := fun _ _ => False
  refutes_sound := fun _ _ h => h.elim

/-- `#source_corresponds()` is never derived: every call has a model in which
it fails, so no fact of the call (heap, trace, arguments) establishes it. -/
theorem source_corresponds_is_never_derived (c : Call) :
    ∃ g : GuardedCall, g.call = c ∧ ¬ Holds g environmentSymbol :=
  ⟨bare c, rfl, by rw [environment_holds]; exact id⟩

/-- Nor is the caller's refutation: every call has a model without it. -/
theorem caller_refutation_is_never_derived (c : Call) :
    ∃ g : GuardedCall, g.call = c ∧ ¬ Holds g invalidMembershipSymbol :=
  ⟨bare c, rfl, by rw [invalid_membership_holds]; exact fun h => h.2.2⟩

/-- An entry environment with no owned child. -/
def emptyEntry : Environment :=
  ⟨0, 0, [], fun _ => False, fun _ => True, none, fun _ => none, fun _ => none, []⟩

/-- Non-ownership is not refutation: a call whose item is not an owned child
(so `invalidMembership` holds, a failed membership proof) does not satisfy
the I2 symbol without the caller's evidence. -/
theorem absence_of_membership_is_not_refutation :
    invalidMembership (bare ⟨emptyEntry, emptyEntry, 1, false, false⟩).call ∧
      ¬ Holds (bare ⟨emptyEntry, emptyEntry, 1, false, false⟩) invalidMembershipSymbol := by
  refine ⟨?_, ?_⟩
  · simp [invalidMembership, bare, emptyEntry]
  · rw [invalid_membership_holds]
    exact fun h => h.2.2

/-! ## A string is not a guarded proposition (the `QF003` ban) -/

/-- A guarded clause's statement: a string, or a formula (`typed` when it is
a typed rendering over registered terms). -/
inductive GuardedStatement where
  | str
  | formula (typed : Bool)

/-- Admission of a guarded statement (`guarded_decomposition::parse_source_clause`). -/
def admitGuarded : GuardedStatement → Bool
  | .str => false
  | .formula typed => typed

/-- Card SP6-GUARDED-SYMBOL-PROSE: no string is a guarded proposition. -/
theorem string_is_not_a_guarded_proposition : admitGuarded .str = false := rfl

/-- A guarded statement is admitted exactly when it is a typed formula. -/
theorem guarded_admitted_iff_typed (s : GuardedStatement) :
    admitGuarded s = true ↔ s = .formula true := by
  cases s with
  | str => simp [admitGuarded]
  | formula t => cases t <;> simp [admitGuarded]

/-- The decision vectors shared with the Rust table
`qcore_frontend::premise_formula::GUARDED_DECISION_VECTORS`. -/
def guardedVectors : List (GuardedStatement × Bool) := [(.formula true, true), (.formula true, true), (.str, false), (.str, false), (.str, false), (.formula false, false), (.formula false, false)]

theorem guarded_vectors_agree :
    guardedVectors.all (fun (s, expected) => admitGuarded s == expected) = true := by
  rfl

end QCore.CjsonGuardedSymbols

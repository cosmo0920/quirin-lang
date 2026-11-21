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

import QCore.MemberStoreRetention

/-!
# A function-pointer formal in the W4 lexical parameter reader (card CG10-SMALL-BUNDLE, gateway rule 74b)

The shared W4 parameter reader (`function_parameters`) refused every
declarator list holding `(`. Rule 74b (`function_pointer_formal.rs`) splits
such a list at its top-level commas and reads it when every group holding
`(` is exactly `SPEC+ ( * NAME ) ( BALANCED )` and the body never calls
through `NAME`; every other group is the old reader's, alone.

The model proves:

* the decision vectors (bound to the Rust's `DECISION_VECTORS` by
  `rule_74b_lean_vectors_match_the_rust_table`);
* **positions are kept**: a read list has one entry per top-level group, so
  every consumer that zips the names with a contract's parameters or a
  call's actuals sees each formal at its index;
* a called function-pointer formal or any other parenthesised group keeps
  the whole list refused, and a read list has no called formal;
* **a list with no parenthesised group reads exactly as before**;
* a function-pointer formal carries no lifetime: its value adds no
  retention obligation (`QCore.MemberStoreRetention`).

## What this is not

A model of the tokenizer or of the consumers: that the token shape is the
declarator C11 6.7.6.3 gives a function-pointer parameter, and that every
consumer reads a formal by its position, are the Rust's (the consumer census
in the card's report); faithful token extraction is trust boundary 28.
-/

namespace QCore.FunctionPointerFormal

/-- One top-level parameter group. -/
inductive Group where
  /-- No `(`: the old reader's group, naming a formal or not. -/
  | plain (name : Option String)
  /-- `SPEC+ ( * NAME ) ( BALANCED )`, with whether the body calls `NAME`. -/
  | functionPointer (name : String) (called : Bool)
  /-- Any other group holding `(`. -/
  | other
  deriving DecidableEq, Repr

/-- The class the Rust decides on (`Group` in `function_pointer_formal.rs`). -/
inductive GroupClass where
  | plain
  | functionPointer
  | calledFunctionPointer
  | other
  deriving DecidableEq, Repr

def classOf : Group → GroupClass
  | .plain _ => .plain
  | .functionPointer _ false => .functionPointer
  | .functionPointer _ true => .calledFunctionPointer
  | .other => .other

/-- Rule 74b's decision (`admits`). -/
def admits (gs : List GroupClass) : Bool :=
  gs.all fun g => g == .plain || g == .functionPointer

/-- The Rust's `DECISION_VECTORS`. -/
def decisionVectors : List (List GroupClass × Bool) :=
  [ ([.plain, .functionPointer], true), ([.functionPointer], true), ([.plain, .plain], true),
    ([.functionPointer, .calledFunctionPointer], false), ([.plain, .other], false),
    ([.other], false), ([.calledFunctionPointer], false), ([], true) ]

theorem decision_vectors_hold :
    decisionVectors.all (fun (gs, r) => admits gs == r) = true := by
  decide

/-- The name one group gives its position. -/
def nameOf : Group → Option String
  | .plain n => n
  | .functionPointer n _ => some n
  | .other => none

/-- The reading of a list. -/
def read (gs : List Group) : Option (List (Option String)) :=
  if admits (gs.map classOf) then some (gs.map nameOf) else none

/-- **Positions are kept**: one entry per top-level group. -/
theorem positions_are_kept (gs : List Group) (ns : List (Option String))
    (h : read gs = some ns) : ns.length = gs.length := by
  unfold read at h
  split at h
  · cases h; simp
  · cases h

/-- Each position names its own group's formal. -/
theorem each_position_is_its_group (gs : List Group) (ns : List (Option String))
    (h : read gs = some ns) : ns = gs.map nameOf := by
  unfold read at h
  split at h
  · cases h; rfl
  · cases h

theorem admits_of_mem (gs : List Group) (g : Group) (hg : g ∈ gs)
    (h : admits (gs.map classOf) = true) : classOf g = .plain ∨ classOf g = .functionPointer := by
  unfold admits at h
  rw [List.all_eq_true] at h
  have := h (classOf g) (List.mem_map_of_mem hg)
  simp only [Bool.or_eq_true, beq_iff_eq] at this
  exact this

/-- A function-pointer formal the body calls keeps the whole list refused. -/
theorem a_called_formal_refuses (gs : List Group) (n : String)
    (h : .functionPointer n true ∈ gs) : read gs = none := by
  unfold read
  split
  · next ha =>
    rcases admits_of_mem gs _ h ha with h' | h' <;> simp [classOf] at h'
  · rfl

/-- Any other parenthesised group keeps the whole list refused. -/
theorem another_declarator_refuses (gs : List Group) (h : .other ∈ gs) : read gs = none := by
  unfold read
  split
  · next ha =>
    rcases admits_of_mem gs _ h ha with h' | h' <;> simp [classOf] at h'
  · rfl

/-- A read list holds no called formal. -/
theorem a_read_formal_is_never_called (gs : List Group) (ns : List (Option String))
    (h : read gs = some ns) (n : String) (c : Bool) (hm : .functionPointer n c ∈ gs) :
    c = false := by
  cases c with
  | false => rfl
  | true => rw [a_called_formal_refuses gs n hm] at h; cases h

theorem names_of_plain (ns : List (Option String)) : (ns.map Group.plain).map nameOf = ns := by
  induction ns with
  | nil => rfl
  | cons n ns ih => simp only [List.map_cons, nameOf, ih]

theorem classOf_plain (n : Option String) : classOf (.plain n) = .plain := rfl

theorem admits_plain (ns : List (Option String)) :
    admits ((ns.map Group.plain).map classOf) = true := by
  simp [admits, classOf_plain]

/-- **A list with no parenthesised group reads exactly as before**: every
group the old reader's, each position its old name. -/
theorem a_plain_list_reads_as_before (ns : List (Option String)) :
    read (ns.map .plain) = some ns := by
  unfold read
  simp only [admits_plain, ite_true, names_of_plain]

/-- **A function-pointer formal carries no lifetime**: its value is static,
so handing or storing it adds no retention obligation, whatever the holder. -/
theorem a_function_pointer_adds_no_obligation (h : QCore.RetentionPath.Holder)
    (rs : QCore.RetentionPath.Records) :
    QCore.RetentionPath.step rs (.retain (QCore.RetentionPath.decide .static h)) = some rs :=
  QCore.MemberStoreRetention.a_function_pointer_adds_no_obligation h rs

end QCore.FunctionPointerFormal

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

import QCore.ResultConstructors

/-! Card RULE37-PILOT: rule 37 as a switchable package.

Rule 37 is consulted at two host predicates. The aggregate store scan asks it
which checked `sizeof` operand spans to read as unevaluated; switched off, the
scan keeps its pre-rule-37 reading, in which every mention counts. The
checked-specification settlement asks it about every checked specification
whose declaration names the `asserted_allocation` premise; switched off, such a
specification is withdrawn, and admitted, it carries the premise. These
theorems state that the switch on is exactly rule 37's model
(`QCore.ResultConstructors`), that off is the old reading, and that off never
admits what on refuses. The Rust/model correspondence and the fidelity of the
host's checked spans and declared premises remain premises. -/
namespace QCore.ResultConstructorSwitch

open QCore.ResultConstructors

/-- An occurrence's escape reading with rule 37 on or off. -/
def escapesUnder (on : Bool) (occurrence : Occurrence) : Bool :=
  if on then escapesAfter occurrence else escapesBefore occurrence

/-- The base scan with rule 37 on or off. -/
def baseEscapesUnder (on : Bool) (uses : List Occurrence) : Bool :=
  uses.any (escapesUnder on)

theorem the_switch_on_reads_rule_37 (uses : List Occurrence) :
    baseEscapesUnder true uses = baseEscapes uses := by
  have same : escapesUnder true = escapesAfter := by
    funext occurrence
    simp [escapesUnder]
  simp [baseEscapesUnder, baseEscapes, same]

theorem the_switch_off_reads_the_old_scan (uses : List Occurrence) :
    baseEscapesUnder false uses = uses.any escapesBefore := by
  have same : escapesUnder false = escapesBefore := by
    funext occurrence
    simp [escapesUnder]
  simp [baseEscapesUnder, same]

theorem an_escape_with_the_switch_on_escapes_with_it_off (occurrence : Occurrence)
    (h : escapesUnder true occurrence = true) : escapesUnder false occurrence = true := by
  cases occurrence <;> simp_all [escapesUnder, escapesAfter, escapesBefore]

/-- Off never widens the scan: a base that escapes with rule 37 on escapes
with it off. -/
theorem the_switch_off_never_widens_the_scan (uses : List Occurrence)
    (h : baseEscapesUnder true uses = true) : baseEscapesUnder false uses = true := by
  simp only [baseEscapesUnder, List.any_eq_true] at h ⊢
  obtain ⟨occurrence, mem, escapes⟩ := h
  exact ⟨occurrence, mem, an_escape_with_the_switch_on_escapes_with_it_off occurrence escapes⟩

/-- The store step of a constructor with rule 37 on or off. -/
def constructorStepUnder (on : Bool) (s : QCore.AggregateStore.Store)
    (uses : List Occurrence) (o : QCore.AggregateStore.Owner) :=
  QCore.AggregateStore.storeStep {s with baseEscapesPointer := baseEscapesUnder on uses} o

theorem the_switch_on_is_the_constructor_step (s : QCore.AggregateStore.Store)
    (uses : List Occurrence) (o : QCore.AggregateStore.Owner) :
    constructorStepUnder true s uses o = constructorStep s uses o := by
  simp [constructorStepUnder, constructorStep, storeShape, the_switch_on_reads_rule_37]

theorem the_switch_off_refuses_the_checked_sizeof_constructor :
    constructorStepUnder false QCore.AggregateStore.admittedExample [.checkedSizeofDeref]
      QCore.AggregateStore.liveOwner = .refused := by rfl

theorem the_switch_on_links_the_checked_sizeof_constructor :
    constructorStepUnder true QCore.AggregateStore.admittedExample [.checkedSizeofDeref]
      QCore.AggregateStore.liveOwner = .linked := by rfl

/-- A checked specification at the settlement: whether its body establishes
it, and whether its declaration names `asserted_allocation`. -/
structure Specification where
  established : Bool
  namesPremise : Bool
  deriving DecidableEq, Repr

/-- In force: established, and, when it names rule 37's premise, admitted by
rule 37 (which is asked only while the switch is on). -/
def inForce (on : Bool) (s : Specification) : Bool :=
  s.established && (!s.namesPremise || on)

/-- The premises rule 37's admission adds to an in-force specification. -/
def carried (on : Bool) (s : Specification) : List String :=
  if inForce on s && s.namesPremise then ["asserted_allocation"] else []

theorem the_switch_off_withdraws_a_premised_specification (s : Specification)
    (h : s.namesPremise = true) : inForce false s = false := by
  simp [inForce, h]

theorem the_switch_off_never_widens_the_specifications (s : Specification)
    (h : inForce false s = true) : inForce true s = true := by
  cases s with
  | mk established namesPremise =>
    cases established <;> cases namesPremise <;> simp_all [inForce]

theorem an_unpremised_specification_ignores_the_switch (s : Specification)
    (h : s.namesPremise = false) (on : Bool) : inForce on s = s.established := by
  simp [inForce, h]

theorem no_switch_establishes_a_specification (s : Specification)
    (h : s.established = false) (on : Bool) : inForce on s = false := by
  simp [inForce, h]

theorem an_admitted_premised_specification_carries_the_premise (s : Specification)
    (force : inForce true s = true) (named : s.namesPremise = true) :
    "asserted_allocation" ∈ carried true s := by
  simp [carried, force, named]

theorem a_withdrawn_specification_carries_nothing (s : Specification) (on : Bool)
    (h : inForce on s = false) : carried on s = [] := by
  simp [carried, h]

end QCore.ResultConstructorSwitch

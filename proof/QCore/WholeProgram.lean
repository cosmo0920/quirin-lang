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

import QCore.Frontend

namespace QCore.WholeProgram

/-! The reduced Cycle 11 boundary.  Individual semantic proofs establish the
properties represented by `PassInterfaces`; whole-program checking makes
those properties simultaneous, connects calls by symbol, iterates summaries,
and requires an explicit contract at every recursive node. -/

abbrev FunctionId := Nat
abbrev Identity := Nat
abbrev EffectId := Nat
abbrev CfgId := Nat

inductive ParameterOwnership where
  | borrowed
  | consumed
  | returned
  deriving DecidableEq, Repr

structure Summary where
  effects : List EffectId
  parameters : List ParameterOwnership
  deriving DecidableEq, Repr, Inhabited

def Summary.join (left right : Summary) : Summary where
  effects := left.effects ++ right.effects.filter (fun effect => effect ∉ left.effects)
  parameters := left.parameters

structure PassInterfaces where
  usageAccepted : Bool
  ownershipAccepted : Bool
  borrowAccepted : Bool
  flowAccepted : Bool
  errorsAccepted : Bool
  regionsAccepted : Bool
  sourceIdentities : List Identity
  cfgIdentities : List Identity
  cfg : Option CfgId
  deriving DecidableEq, Repr

def PassInterfaces.Accepts (passes : PassInterfaces) : Prop :=
  passes.usageAccepted = true ∧
  passes.ownershipAccepted = true ∧
  passes.borrowAccepted = true ∧
  passes.flowAccepted = true ∧
  passes.errorsAccepted = true ∧
  passes.regionsAccepted = true ∧
  passes.sourceIdentities = passes.cfgIdentities ∧
  passes.cfg.isSome = true

instance (passes : PassInterfaces) : Decidable passes.Accepts := by
  unfold PassInterfaces.Accepts
  infer_instance

def checkPasses (passes : PassInterfaces) : Option PassInterfaces :=
  if passes.Accepts then some passes else none

theorem checkPasses_implies_interfaces {passes checked : PassInterfaces}
    (result : checkPasses passes = some checked) : passes.Accepts := by
  unfold checkPasses at result
  split at result
  · assumption
  · simp at result

theorem checkPasses_preserves_input {passes checked : PassInterfaces}
    (result : checkPasses passes = some checked) : checked = passes := by
  unfold checkPasses at result
  split at result
  · cases result
    rfl
  · simp at result

theorem accepted_passes_have_checked_cfg {passes checked : PassInterfaces}
    (result : checkPasses passes = some checked) : ∃ cfg, checked.cfg = some cfg := by
  rw [checkPasses_preserves_input result]
  have present := (checkPasses_implies_interfaces result).2.2.2.2.2.2.2
  cases value : passes.cfg with
  | none => simp [value] at present
  | some cfg => exact ⟨cfg, rfl⟩

theorem accepted_passes_preserve_cfg_identities {passes checked : PassInterfaces}
    (result : checkPasses passes = some checked) :
    checked.sourceIdentities = checked.cfgIdentities := by
  rw [checkPasses_preserves_input result]
  exact (checkPasses_implies_interfaces result).2.2.2.2.2.2.1

structure Call where
  callee : FunctionId
  arguments : List Identity
  deriving DecidableEq, Repr

structure FunctionSpec where
  symbol : FunctionId
  localSummary : Summary
  calls : List Call
  recursive : Bool
  declaredSummary : Option Summary
  passes : PassInterfaces
  sourceContractClauses : Nat := 0
  deriving DecidableEq, Repr

def FunctionSpec.ValidIn (function : FunctionSpec) (names : List FunctionId) : Prop :=
  function.passes.Accepts ∧
  (function.recursive = true → function.declaredSummary.isSome = true) ∧
  function.calls.all (fun call => decide (call.callee ∈ names)) = true ∧
  function.sourceContractClauses = 0

instance (function : FunctionSpec) (names : List FunctionId) :
    Decidable (function.ValidIn names) := by
  unfold FunctionSpec.ValidIn
  infer_instance

structure Program where
  functions : List FunctionSpec
  deriving DecidableEq, Repr

def Program.names (program : Program) : List FunctionId :=
  program.functions.map FunctionSpec.symbol

def Program.Accepts (program : Program) : Prop :=
  program.names.Nodup ∧
  program.functions.all
    (fun function => decide (function.ValidIn program.names)) = true

instance (program : Program) : Decidable program.Accepts := by
  unfold Program.Accepts
  infer_instance

abbrev SummaryTable := FunctionId → Summary

def lookupFunction (program : Program) (symbol : FunctionId) : Option FunctionSpec :=
  program.functions.find? (fun function => function.symbol = symbol)

def summarizeCalls (calls : List Call) (table : SummaryTable)
    (initial : Summary) : Summary :=
  calls.foldl (fun summary call => summary.join (table call.callee)) initial

def summaryRound (program : Program) (previous : SummaryTable) : SummaryTable :=
  fun symbol =>
    match lookupFunction program symbol with
    | none => default
    | some function =>
        if function.recursive then
          function.declaredSummary.getD default
        else
          summarizeCalls function.calls previous function.localSummary

def iterateSummaries (program : Program) : Nat → SummaryTable
  | 0 => fun _ => default
  | round + 1 => summaryRound program (iterateSummaries program round)

structure CheckedProgram where
  source : Program
  summaries : SummaryTable

def checkProgram (program : Program) : Option CheckedProgram :=
  if program.Accepts then
    some {
      source := program
      summaries := iterateSummaries program program.functions.length
    }
  else
    none

theorem checkProgram_implies_accepts {program : Program} {checked : CheckedProgram}
    (result : checkProgram program = some checked) : program.Accepts := by
  unfold checkProgram at result
  split at result
  · assumption
  · simp at result

theorem checkProgram_preserves_source {program : Program} {checked : CheckedProgram}
    (result : checkProgram program = some checked) : checked.source = program := by
  unfold checkProgram at result
  split at result
  · cases result
    rfl
  · simp at result

theorem accepted_function_interfaces {program : Program} {checked : CheckedProgram}
    (result : checkProgram program = some checked)
    {function : FunctionSpec} (member : function ∈ program.functions) :
    function.passes.Accepts := by
  have decisions := List.all_eq_true.mp (checkProgram_implies_accepts result).2
  exact (of_decide_eq_true (decisions function member) :
    function.ValidIn program.names).1

/-- Until source clauses have an enforcing interpreter, no accepted function
may carry one. Supplied semantic bundles cannot override this source gate. -/
theorem accepted_function_has_no_unchecked_source_contract
    {program : Program} {checked : CheckedProgram}
    (result : checkProgram program = some checked)
    {function : FunctionSpec} (member : function ∈ program.functions) :
    function.sourceContractClauses = 0 := by
  have decisions := List.all_eq_true.mp (checkProgram_implies_accepts result).2
  exact (of_decide_eq_true (decisions function member) :
    function.ValidIn program.names).2.2.2

theorem unchecked_source_contract_rejects {program : Program}
    {function : FunctionSpec} (member : function ∈ program.functions)
    (unchecked : function.sourceContractClauses ≠ 0) : checkProgram program = none := by
  cases result : checkProgram program with
  | none => rfl
  | some checked =>
      exact False.elim (unchecked (accepted_function_has_no_unchecked_source_contract result member))

theorem recursive_function_has_declared_summary
    {program : Program} {checked : CheckedProgram}
    (result : checkProgram program = some checked)
    {function : FunctionSpec} (member : function ∈ program.functions)
    (recursive : function.recursive = true) :
    function.declaredSummary.isSome = true := by
  have decisions := List.all_eq_true.mp (checkProgram_implies_accepts result).2
  exact (of_decide_eq_true (decisions function member) :
    function.ValidIn program.names).2.1 recursive

theorem accepted_program_function_has_checked_cfg
    {program : Program} {checked : CheckedProgram}
    (result : checkProgram program = some checked)
    {function : FunctionSpec} (member : function ∈ program.functions) :
    ∃ cfg, function.passes.cfg = some cfg := by
  have present := (accepted_function_interfaces result member).2.2.2.2.2.2.2
  cases value : function.passes.cfg with
  | none => simp [value] at present
  | some cfg => exact ⟨cfg, rfl⟩

theorem accepted_program_preserves_function_cfg_identities
    {program : Program} {checked : CheckedProgram}
    (result : checkProgram program = some checked)
    {function : FunctionSpec} (member : function ∈ program.functions) :
    function.passes.sourceIdentities = function.passes.cfgIdentities := by
  exact (accepted_function_interfaces result member).2.2.2.2.2.2.1

/-- Cross-function consumption only removes identities from the caller's
ownership set.  In particular it cannot invent a callee-local replacement. -/
def consume (live transferred : List Identity) : List Identity :=
  match transferred with
  | [] => live
  | identity :: rest => consume (live.erase identity) rest

theorem consume_does_not_mint {live transferred : List Identity} {identity : Identity}
    (member : identity ∈ consume live transferred) : identity ∈ live := by
  induction transferred generalizing live with
  | nil => exact member
  | cons head tail induction =>
      exact List.mem_of_mem_erase (induction member)

theorem consume_preserves_unique_identities {live transferred : List Identity}
    (unique : live.Nodup) : (consume live transferred).Nodup := by
  induction transferred generalizing live with
  | nil => exact unique
  | cons head tail induction =>
      exact induction (unique.erase head)

theorem recursive_summary_round_uses_contract
    (program : Program) (previous : SummaryTable)
    (function : FunctionSpec)
    (found : lookupFunction program function.symbol = some function)
    (recursive : function.recursive = true) :
    summaryRound program previous function.symbol =
      function.declaredSummary.getD default := by
  simp [summaryRound, found, recursive]

end QCore.WholeProgram

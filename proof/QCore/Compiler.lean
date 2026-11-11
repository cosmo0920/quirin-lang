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

import QCore.WholeProgram

namespace QCore.Compiler

/-! A reduced Cycle 12 model of the end-to-end boundary.  Cycle 11 supplies
whole-program acceptance and checked CFG identity; this layer selects an entry,
retains its checked CFG, emits C11, and records where the emitted function came
from. -/

abbrev FileId := Nat
abbrev ByteOffset := Nat
abbrev CSource := String
abbrev Observation := Nat

structure SourceLocation where
  file : FileId
  offset : ByteOffset
  deriving DecidableEq, Repr

structure SourceFile where
  file : FileId
  program : WholeProgram.Program
  deriving DecidableEq, Repr

structure Module where
  files : List SourceFile
  deriving DecidableEq, Repr

def Module.program (module : Module) : WholeProgram.Program where
  functions := module.files.flatMap (fun file => file.program.functions)

structure EmittedSourceMap where
  generatedLine : Nat
  source : SourceLocation
  deriving DecidableEq, Repr

structure EmittedProgram where
  c11 : CSource
  sourceMap : List EmittedSourceMap
  deriving DecidableEq, Repr

def emitC11 (cfg : WholeProgram.CfgId) (source : SourceLocation) : EmittedProgram where
  c11 := s!"/* checked cfg {cfg} */"
  sourceMap := [{ generatedLine := 1, source := source }]

structure Compilation where
  checked : WholeProgram.CheckedProgram
  entry : WholeProgram.FunctionSpec
  cfg : WholeProgram.CfgId
  origin : SourceLocation
  emitted : EmittedProgram

def compile (module : Module) (entry : WholeProgram.FunctionId)
    (origin : SourceLocation) : Option Compilation :=
  match _checkedResult : WholeProgram.checkProgram module.program with
  | none => none
  | some checked =>
      match _functionResult : WholeProgram.lookupFunction module.program entry with
      | none => none
      | some function =>
          match _cfgResult : function.passes.cfg with
          | none => none
          | some cfg => some {
              checked := checked
              entry := function
              cfg := cfg
              origin := origin
              emitted := emitC11 cfg origin
            }

private theorem compile_components
    {module : Module} {entry : WholeProgram.FunctionId}
    {origin : SourceLocation} {compiled : Compilation}
    (result : compile module entry origin = some compiled) :
    ∃ checked function cfg,
      WholeProgram.checkProgram module.program = some checked ∧
      WholeProgram.lookupFunction module.program entry = some function ∧
      function.passes.cfg = some cfg ∧
      compiled = {
        checked := checked
        entry := function
        cfg := cfg
        origin := origin
        emitted := emitC11 cfg origin
  } := by
  unfold compile at result
  split at result
  · contradiction
  · rename_i checked checkedResult
    split at result
    · contradiction
    · rename_i function functionResult
      split at result
      · contradiction
      · rename_i cfg cfgResult
        exact ⟨checked, function, cfg, checkedResult, functionResult,
          cfgResult, Option.some.inj result |>.symm⟩

theorem compile_preserves_checked_source
    {module : Module} {entry : WholeProgram.FunctionId}
    {origin : SourceLocation} {compiled : Compilation}
    (result : compile module entry origin = some compiled) :
    compiled.checked.source = module.program := by
  rcases compile_components result with
    ⟨checked, function, cfg, checkedResult, functionResult, cfgResult, rfl⟩
  exact WholeProgram.checkProgram_preserves_source checkedResult

theorem compile_implies_whole_program_accepts
    {module : Module} {entry : WholeProgram.FunctionId}
    {origin : SourceLocation} {compiled : Compilation}
    (result : compile module entry origin = some compiled) :
    module.program.Accepts := by
  rcases compile_components result with
    ⟨checked, function, cfg, checkedResult, functionResult, cfgResult, compiledResult⟩
  exact WholeProgram.checkProgram_implies_accepts checkedResult

theorem compile_selects_named_entry
    {module : Module} {entry : WholeProgram.FunctionId}
    {origin : SourceLocation} {compiled : Compilation}
    (result : compile module entry origin = some compiled) :
    compiled.entry.symbol = entry := by
  rcases compile_components result with
    ⟨checked, function, cfg, checkedResult, functionResult, cfgResult, rfl⟩
  unfold WholeProgram.lookupFunction at functionResult
  have found := List.find?_some functionResult
  exact of_decide_eq_true found

theorem compile_retains_entry_cfg
    {module : Module} {entry : WholeProgram.FunctionId}
    {origin : SourceLocation} {compiled : Compilation}
    (result : compile module entry origin = some compiled) :
    compiled.entry.passes.cfg = some compiled.cfg := by
  rcases compile_components result with
    ⟨checked, function, cfg, checkedResult, functionResult, cfgResult, rfl⟩
  exact cfgResult

theorem compile_emits_origin_mapping
    {module : Module} {entry : WholeProgram.FunctionId}
    {origin : SourceLocation} {compiled : Compilation}
    (result : compile module entry origin = some compiled) :
    compiled.emitted.sourceMap =
      [{ generatedLine := 1, source := origin }] := by
  rcases compile_components result with
    ⟨checked, function, cfg, checkedResult, functionResult, cfgResult, rfl⟩
  rfl

def runReference (compiled : Compilation) : Observation := compiled.cfg

def runC11 (compiled : Compilation) : Observation := compiled.cfg

/-- The reduced target observation is indexed by the very checked CFG retained
by the reference semantics, so successful emission cannot change it. -/
theorem emitted_c11_matches_reference (compiled : Compilation) :
    runC11 compiled = runReference compiled := by
  rfl

end QCore.Compiler

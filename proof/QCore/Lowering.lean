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

namespace QCore.Lowering

abbrev ResourceId := Nat
abbrev ErrorName := Nat

inductive TraceEvent where
  | acquire (resource : ResourceId)
  | release (resource : ResourceId)
  | transfer (resource : ResourceId)
  deriving DecidableEq

inductive Outcome where
  | ok (value : Int)
  | error (name : ErrorName)
  deriving DecidableEq

structure Observation where
  outcome : Outcome
  trace : List TraceEvent
  payload : List ResourceId
  deriving DecidableEq

/-- The already checked result of one source path.  On an error exit `live`
    records precisely the identities that the Cycle 6 cleanup pack seals. -/
inductive ExitPlan where
  | ok (value : Int) (transferred : List ResourceId)
  | error (name : ErrorName) (live : List ResourceId)
  deriving DecidableEq

structure SourcePath where
  bodyEvents : List TraceEvent
  plan : ExitPlan
  deriving DecidableEq

def cleanupTrace (live : List ResourceId) : List TraceEvent :=
  live.reverse.map TraceEvent.release

def sourceObserve (path : SourcePath) : Observation :=
  match path.plan with
  | .ok value transferred => {
      outcome := .ok value
      trace := path.bodyEvents
      payload := transferred }
  | .error name live => {
      outcome := .error name
      trace := path.bodyEvents ++ cleanupTrace live
      payload := [] }

/-- A reduced cleanup-explicit CFG.  `cleanupLabel = some n` represents the
    edge-specific C label selected for an error path. -/
structure TypedCfg where
  bodyTrace : List TraceEvent
  cleanupLabel : Option Nat
  cleanupInstructions : List TraceEvent
  outcome : Outcome
  payload : List ResourceId
  deriving DecidableEq

def lower (path : SourcePath) : TypedCfg :=
  match path.plan with
  | .ok value transferred => {
      bodyTrace := path.bodyEvents
      cleanupLabel := none
      cleanupInstructions := []
      outcome := .ok value
      payload := transferred }
  | .error name live => {
      bodyTrace := path.bodyEvents
      cleanupLabel := some 0
      cleanupInstructions := cleanupTrace live
      outcome := .error name
      payload := [] }

def cfgObserve (cfg : TypedCfg) : Observation := {
  outcome := cfg.outcome
  trace := cfg.bodyTrace ++ cfg.cleanupInstructions
  payload := cfg.payload }

theorem source_cfg_simulation (path : SourcePath) :
    cfgObserve (lower path) = sourceObserve path := by
  cases path with
  | mk bodyEvents plan =>
    cases plan <;> simp [cfgObserve, lower, sourceObserve]

theorem error_edge_has_cleanup_label
    (bodyEvents : List TraceEvent) (name : ErrorName) (live : List ResourceId) :
    (lower { bodyEvents := bodyEvents, plan := .error name live }).cleanupLabel = some 0 := by
  rfl

theorem partial_initialization_gets_exact_cleanup
    (bodyEvents : List TraceEvent) (name : ErrorName) (initialized : List ResourceId) :
    (lower { bodyEvents := bodyEvents, plan := .error name initialized }).cleanupInstructions =
      cleanupTrace initialized := by
  rfl

def releaseId : TraceEvent → Option ResourceId
  | .release resource => some resource
  | _ => none

def releaseIds (trace : List TraceEvent) : List ResourceId :=
  trace.filterMap releaseId

theorem cleanup_release_ids (live : List ResourceId) :
    releaseIds (cleanupTrace live) = live.reverse := by
  simp [releaseIds, cleanupTrace, Function.comp_def, releaseId]

theorem cleanup_releases_exactly_live_resources
    (live : List ResourceId) (resource : ResourceId) :
    resource ∈ releaseIds (cleanupTrace live) ↔ resource ∈ live := by
  simp [cleanup_release_ids]

theorem cleanup_releases_in_lifo_order (live : List ResourceId) :
    releaseIds (cleanupTrace live) = live.reverse := by
  exact cleanup_release_ids live

private theorem nodup_reverse : ∀ live : List ResourceId,
    live.Nodup → live.reverse.Nodup
  | [], _ => by simp
  | head :: tail, unique => by
      rw [List.nodup_cons] at unique
      rw [List.reverse_cons, List.nodup_append]
      refine ⟨nodup_reverse tail unique.2, by simp, ?_⟩
      intro left leftMember right rightMember
      simp only [List.mem_singleton] at rightMember
      subst right
      intro equal
      subst left
      apply unique.1
      simpa using leftMember

theorem cleanup_releases_each_identity_at_most_once (live : List ResourceId)
    (unique : live.Nodup) : (releaseIds (cleanupTrace live)).Nodup := by
  rw [cleanup_release_ids]
  exact nodup_reverse live unique

/-- The emitted-C abstract machine retains the CFG's tagged outcome, payload,
    and resource trace.  Rust translation validation checks this abstraction
    against compiled C11 for the supported Cycle 7 subset. -/
structure CProgram where
  cfg : TypedCfg
  deriving DecidableEq

def emitC (cfg : TypedCfg) : CProgram := { cfg := cfg }

def cObserve (program : CProgram) : Observation :=
  cfgObserve program.cfg

theorem cfg_c_simulation (cfg : TypedCfg) :
    cObserve (emitC cfg) = cfgObserve cfg := by
  rfl

theorem c_lowering_simulation (path : SourcePath) :
    cObserve (emitC (lower path)) = sourceObserve path := by
  exact source_cfg_simulation path

theorem normal_value_and_transfer_are_preserved
    (bodyEvents : List TraceEvent) (value : Int) (transferred : List ResourceId) :
    cObserve (emitC (lower {
      bodyEvents := bodyEvents
      plan := .ok value transferred })) = {
        outcome := .ok value
        trace := bodyEvents
        payload := transferred } := by
  simp [cObserve, emitC, cfgObserve, lower]

theorem error_tag_and_cleanup_trace_are_preserved
    (bodyEvents : List TraceEvent) (name : ErrorName) (live : List ResourceId) :
    cObserve (emitC (lower {
      bodyEvents := bodyEvents
      plan := .error name live })) = {
        outcome := .error name
        trace := bodyEvents ++ cleanupTrace live
        payload := [] } := by
  rfl

end QCore.Lowering

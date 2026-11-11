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

namespace QCore.ExecutableModel

abbrev NaturalIndex := Nat
abbrev Variable := Nat
abbrev ResourceIdentity := Nat
abbrev ErrorIdentity := Nat

inductive UsageGrade where
  | erased
  | exactlyOne
  | affine
  | many
  deriving DecidableEq

namespace UsageGrade

def Accepts : UsageGrade → Nat → Prop
  | .erased, demand => demand = 0
  | .exactlyOne, demand => demand = 1
  | .affine, demand => demand ≤ 1
  | .many, _ => True

def UpperExceeded : UsageGrade → Nat → Prop
  | .erased, demand => 0 < demand
  | .exactlyOne, demand => 1 < demand
  | .affine, demand => 1 < demand
  | .many, _ => False

instance (grade : UsageGrade) (demand : Nat) : Decidable (grade.Accepts demand) := by
  cases grade <;> simp [Accepts] <;> infer_instance

instance (grade : UsageGrade) (demand : Nat) : Decidable (grade.UpperExceeded demand) := by
  cases grade <;> simp [UpperExceeded] <;> infer_instance

end UsageGrade

structure UsageBinding where
  name : Variable
  grade : UsageGrade
  deriving DecidableEq

inductive Operation where
  | use (name : Variable)
  | acquire (resource : ResourceIdentity)
  | drop (resource : ResourceIdentity)
  | raise (error : ErrorIdentity)
  | returnIndex (value : NaturalIndex)
  deriving DecidableEq

inductive TypedError where
  | raised (error : ErrorIdentity)
  | undeclaredError (error : ErrorIdentity)
  | duplicateAcquire (resource : ResourceIdentity)
  | invalidDrop (resource : ResourceIdentity)
  | unknownUsageVariable (name : Variable)
  | usageUpperExceeded (name : Variable) (grade : UsageGrade) (actual : Nat)
  | usageObligationUnsatisfied (name : Variable) (grade : UsageGrade) (actual : Nat)
  | liveResourcesAtReturn
  | missingTerminator
  deriving DecidableEq

/-- A sealed snapshot of the one region. `actions` is newest first, hence is
    already in cleanup execution order. -/
structure CleanupPack where
  baseline : List ResourceIdentity
  actions : List ResourceIdentity
  deriving DecidableEq

inductive Outcome where
  | returned (value : NaturalIndex)
  | error (cause : TypedError) (pack : CleanupPack)
  deriving DecidableEq

inductive TraceEvent where
  | use (name : Variable)
  | acquire (resource : ResourceIdentity)
  | release (resource : ResourceIdentity)
  deriving DecidableEq

inductive Control where
  | running (usage : List UsageBinding) (declaredErrors : List ErrorIdentity)
      (remaining : List Operation)
  | cleaning (error : TypedError) (pack : CleanupPack)
  | done (outcome : Outcome)
  deriving DecidableEq

structure Config where
  control : Control
  baseline : List ResourceIdentity
  /-- Region-local resource stack, newest first. -/
  locals : List ResourceIdentity
  /-- Released identity history, most recent first. -/
  released : List ResourceIdentity
  trace : List TraceEvent
  deriving DecidableEq

def Config.identities (cfg : Config) : List ResourceIdentity :=
  cfg.locals ++ cfg.released ++ cfg.baseline

/-- The reduced machine-state typing invariant is a duplicate-free partition
    of live locals, released history, and the untouched region baseline. -/
def WellFormed (cfg : Config) : Prop := cfg.identities.Nodup

def usageCount (name : Variable) (trace : List TraceEvent) : Nat :=
  trace.countP fun event => event = .use name

def usageBinding? (name : Variable) : List UsageBinding → Option UsageGrade
  | [] => none
  | binding :: rest =>
      if binding.name = name then some binding.grade else usageBinding? name rest

def UsageSatisfied (usage : List UsageBinding) (trace : List TraceEvent) : Prop :=
  ∀ binding ∈ usage, binding.grade.Accepts (usageCount binding.name trace)

def firstUnsatisfied (usage : List UsageBinding) (trace : List TraceEvent) :
    Option (Variable × UsageGrade × Nat) :=
  usage.findSome? fun binding =>
    let actual := usageCount binding.name trace
    if binding.grade.Accepts actual then none else some (binding.name, binding.grade, actual)

def beginCleanup (cfg : Config) (error : TypedError) : Config :=
  { cfg with
    control := .cleaning error { baseline := cfg.baseline, actions := cfg.locals } }

/-- One executable small step. Every non-final configuration takes a step;
    dynamic faults become typed errors and enter the same cleanup protocol. -/
def step (cfg : Config) : Option Config :=
  match cfg.control with
  | .done _ => none
  | .cleaning error pack =>
      match cfg.locals with
      | [] => some { cfg with control := .done (.error error pack) }
      | resource :: rest => some {
          cfg with
          locals := rest
          released := resource :: cfg.released
          trace := cfg.trace ++ [.release resource] }
  | .running usage declaredErrors remaining =>
      match remaining with
      | [] => some (beginCleanup cfg .missingTerminator)
      | operation :: rest =>
          let next := { cfg with control := .running usage declaredErrors rest }
          match operation with
          | .use name =>
              match usageBinding? name usage with
              | none => some (beginCleanup next (.unknownUsageVariable name))
              | some grade =>
                  let used := { next with trace := next.trace ++ [.use name] }
                  let actual := usageCount name used.trace
                  if grade.UpperExceeded actual then
                    some (beginCleanup used (.usageUpperExceeded name grade actual))
                  else some used
          | .acquire resource =>
              if resource ∈ next.identities then
                some (beginCleanup next (.duplicateAcquire resource))
              else some {
                next with
                locals := resource :: next.locals
                trace := next.trace ++ [.acquire resource] }
          | .drop resource =>
              match next.locals with
              | top :: rest =>
                  if resource = top then some {
                    next with
                    locals := rest
                    released := top :: next.released
                    trace := next.trace ++ [.release top] }
                  else some (beginCleanup next (.invalidDrop resource))
              | [] => some (beginCleanup next (.invalidDrop resource))
          | .raise error =>
              if error ∈ declaredErrors then
                some (beginCleanup next (.raised error))
              else some (beginCleanup next (.undeclaredError error))
          | .returnIndex value =>
              if next.locals ≠ [] then
                some (beginCleanup next .liveResourcesAtReturn)
              else
                match firstUnsatisfied usage next.trace with
                | some (name, grade, actual) =>
                    some (beginCleanup next
                      (.usageObligationUnsatisfied name grade actual))
                | none => some { next with control := .done (.returned value) }

inductive Steps : Config → Config → Prop where
  | refl (cfg : Config) : Steps cfg cfg
  | tail {before after final : Config} :
      step before = some after → Steps after final → Steps before final

private theorem releaseHead_perm (resource : ResourceIdentity)
    (locals released baseline : List ResourceIdentity) :
    (resource :: locals ++ released ++ baseline).Perm
      (locals ++ resource :: released ++ baseline) := by
  have moved : ([resource] ++ locals).Perm (locals ++ [resource]) :=
    List.perm_append_comm
  have withTail := List.Perm.append_right (released ++ baseline) moved
  simpa [List.append_assoc] using withTail

private theorem releaseHead_preserves_wellFormed
    (resource : ResourceIdentity) (locals released baseline : List ResourceIdentity)
    (safe : (resource :: locals ++ released ++ baseline).Nodup) :
    (locals ++ resource :: released ++ baseline).Nodup := by
  exact (releaseHead_perm resource locals released baseline).nodup safe

theorem progress_up_to_typed_error (cfg : Config) (typed : WellFormed cfg) :
    (∃ outcome, cfg.control = .done outcome) ∨
      ∃ next, step cfg = some next := by
  cases cfg with
  | mk control baseline locals released trace =>
    cases control with
    | done outcome => exact Or.inl ⟨outcome, rfl⟩
    | cleaning error pack =>
        right
        cases locals <;> simp [step]
    | running usage errors remaining =>
        right
        cases remaining with
        | nil => simp [step]
        | cons operation rest =>
            cases operation with
            | use name =>
                simp only [step]
                cases binding : usageBinding? name usage with
                | none => simp
                | some grade =>
                    by_cases exceeded : grade.UpperExceeded
                      (usageCount name (trace ++ [TraceEvent.use name]))
                    · simp [exceeded]
                    · simp [exceeded]
            | acquire resource =>
                by_cases duplicate :
                    resource ∈ locals ∨ resource ∈ released ∨ resource ∈ baseline
                · simp [step, Config.identities, duplicate]
                · simp [step, Config.identities, duplicate]
            | drop resource =>
                cases locals with
                | nil => simp [step]
                | cons top localRest =>
                    by_cases same : resource = top
                    · simp [step, same]
                    · simp [step, same]
            | raise error =>
                by_cases declared : error ∈ errors
                · simp [step, declared]
                · simp [step, declared]
            | returnIndex value =>
                by_cases live : locals ≠ []
                · simp [step, live]
                · have empty : locals = [] :=
                    Decidable.byContradiction fun notEmpty => live notEmpty
                  subst locals
                  cases obligation : firstUnsatisfied usage trace with
                  | none => simp [step, obligation]
                  | some detail =>
                      rcases detail with ⟨name, grade, actual⟩
                      simp [step, obligation]

theorem preservation {cfg next : Config} (typed : WellFormed cfg)
    (reduces : step cfg = some next) : WellFormed next := by
  rcases cfg with ⟨control, baseline, locals, released, trace⟩
  cases control with
  | done outcome => simp [step] at reduces
  | cleaning error pack =>
      cases locals with
      | nil =>
          simp [step] at reduces
          subst next
          exact typed
      | cons resource rest =>
          simp [step] at reduces
          subst next
          exact releaseHead_preserves_wellFormed resource rest released baseline typed
  | running usage errors remaining =>
      cases remaining with
      | nil =>
          simp [step, beginCleanup] at reduces
          subst next
          exact typed
      | cons operation rest =>
          cases operation with
          | use name =>
              simp only [step] at reduces
              split at reduces
              · simp [beginCleanup] at reduces
                subst next
                exact typed
              · split at reduces <;> simp [beginCleanup] at reduces
                · subst next
                  exact typed
                · subst next
                  exact typed
          | acquire resource =>
              simp only [step] at reduces
              split at reduces
              · simp [beginCleanup] at reduces
                subst next
                exact typed
              · simp at reduces
                subst next
                have fresh : resource ∉ locals ++ released ++ baseline := by
                  change resource ∉ locals ++ released ++ baseline at ‹resource ∉ _›
                  exact ‹resource ∉ locals ++ released ++ baseline›
                exact List.nodup_cons.mpr ⟨fresh, typed⟩
          | drop resource =>
              simp only [step] at reduces
              cases locals with
              | nil =>
                  simp [beginCleanup] at reduces
                  subst next
                  exact typed
              | cons top remainingLocals =>
                  by_cases same : resource = top
                  · subst resource
                    simp at reduces
                    subst next
                    exact releaseHead_preserves_wellFormed top remainingLocals released baseline typed
                  · simp [same, beginCleanup] at reduces
                    subst next
                    exact typed
          | raise error =>
              simp only [step] at reduces
              split at reduces <;> simp [beginCleanup] at reduces
              · subst next
                exact typed
              · subst next
                exact typed
          | returnIndex value =>
              simp only [step] at reduces
              split at reduces
              · simp [beginCleanup] at reduces
                subst next
                exact typed
              · split at reduces <;> simp [beginCleanup] at reduces
                · subst next
                  exact typed
                · subst next
                  exact typed

theorem no_duplication_of_resource_identities (cfg : Config)
    (typed : WellFormed cfg) : cfg.locals.Nodup := by
  exact (List.nodup_append.mp (List.nodup_append.mp typed).1).1

theorem no_double_release (cfg : Config) (typed : WellFormed cfg) :
    cfg.released.Nodup := by
  exact (List.nodup_append.mp (List.nodup_append.mp typed).1).2.1

theorem steps_preservation {cfg final : Config} (typed : WellFormed cfg)
    (reduces : Steps cfg final) : WellFormed final := by
  induction reduces with
  | refl => exact typed
  | tail oneStep _ ih => exact ih (preservation typed oneStep)

private theorem drainCleanup (error : TypedError) (pack : CleanupPack)
    (baseline actions released : List ResourceIdentity) (trace : List TraceEvent) :
    Steps {
      control := .cleaning error pack
      baseline := baseline
      locals := actions
      released := released
      trace := trace
    } {
      control := .done (.error error pack)
      baseline := baseline
      locals := []
      released := actions.reverse ++ released
      trace := trace ++ actions.map TraceEvent.release
    } := by
  induction actions generalizing released trace with
  | nil =>
      apply Steps.tail (by rfl)
      simpa using Steps.refl {
        control := .done (.error error pack)
        baseline := baseline
        locals := []
        released := released
        trace := trace }
  | cons resource rest ih =>
      apply Steps.tail (by rfl)
      have releasedEq :
          rest.reverse ++ resource :: released =
            (resource :: rest).reverse ++ released := by
        simp [List.reverse_cons, List.append_assoc]
      have traceEq :
          (trace ++ [.release resource]) ++ rest.map TraceEvent.release =
            trace ++ (resource :: rest).map TraceEvent.release := by
        rw [List.map_cons, List.append_assoc]
        rfl
      rw [← releasedEq, ← traceEq]
      exact ih (resource :: released) (trace ++ [.release resource])

/-- Every typed error restores the one region's baseline, releases every
    packed identity exactly once in pack order, and ends with that same typed
    error and sealed evidence. -/
theorem region_cleanup_theorem (error : TypedError) (pack : CleanupPack)
    (released : List ResourceIdentity) (trace : List TraceEvent) :
    Steps {
      control := .cleaning error pack
      baseline := pack.baseline
      locals := pack.actions
      released := released
      trace := trace
    } {
      control := .done (.error error pack)
      baseline := pack.baseline
      locals := []
      released := pack.actions.reverse ++ released
      trace := trace ++ pack.actions.map TraceEvent.release
    } := by
  exact drainCleanup error pack pack.baseline pack.actions released trace

end QCore.ExecutableModel

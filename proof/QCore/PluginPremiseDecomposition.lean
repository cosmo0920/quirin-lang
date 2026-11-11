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

import QCore.HypothesisGraph

/-!
Concrete conditional plugin obligations. Event completeness, actual C/source,
ABI/build interpretation and model/Rust correspondence are open prerequisites.
These definitions intentionally establish no implication to old English prose.
-/
namespace QCore.PluginPremiseDecomposition

inductive Family where
  | rk | fluentC | fluentGo
  deriving DecidableEq

inductive Action where
  | invoke | register | intercept | acquire | release | retain | transfer
  | returnLocal | globalCustody | internalWrite | coreWrite | forge | access
  | apiCall | hookInstall | hookReplace | cjsonLinkWrite | cjsonCall | close
  | destroyCallback | copyReference
  deriving DecidableEq

structure Event where
  action : Action
  plugin : Nat
  function : Nat
  member : Nat
  opaqueId : Nat
  handle : Nat
  resource : Nat
  time : Nat
  result : Int
  publicSetter : Bool
  exactCandidate : Bool
  pointerBorrowed : Bool
  scalarBorrowed : Bool
  localResource : Bool
  confLive : Bool
  fromOnNew : Bool
  initialized : Bool
  shareConsumer : Bool
  handedObject : Bool
  exportedApi : Bool
  isRkCall : Bool
  publicRkSymbol : Bool
  forged : Bool
  apiPrecondition : Bool
  properList : Bool
  configRegistration : Bool
  inInterceptor : Bool
  activation : Nat

structure Environment where
  trace : List Event
  loaded : Nat → Family → Prop
  loaderEnabled : Family → Bool
  actualArtifact : Nat → String
  expectedArtifact : Nat → String
  actualBuild : Nat → String
  expectedBuild : Nat → String
  requiredSymbol : Nat → Nat → Prop
  resolvedAddress : Nat → Nat → Nat
  expectedAddress : Nat → Nat → Nat
  actualAbi : Nat → String
  expectedAbi : Nat → String
  memberCandidate : Nat → Nat → Prop
  staticDefault : Nat → Nat → Prop
  codeLive : Nat → Nat → Prop
  contextLive : Nat → Nat → Prop
  handleFor : Nat → Nat
  refCount : Nat → Nat → Nat
  returnBoundary : Nat → Nat
  expectedInventory : String
  actualInventory : String
  preexistingHooksValid : Nat → Prop

abbrev Loaded (e : Environment) (p : Nat) : Prop := ∃ f, e.loaded p f

def loaderEnvironment (e : Environment) (p : Nat) : Prop :=
  e.actualBuild p = e.expectedBuild p ∧
  e.actualInventory = e.expectedInventory ∧
  ∀ family, e.loaded p family → e.loaderEnabled family = true

def artifactSymbolAbi (e : Environment) (p : Nat) : Prop :=
  e.actualArtifact p = e.expectedArtifact p ∧
  ∀ symbol, e.requiredSymbol p symbol →
    e.resolvedAddress p symbol = e.expectedAddress p symbol ∧
    e.resolvedAddress p symbol ≠ 0 ∧
    e.actualAbi (e.resolvedAddress p symbol) =
      e.expectedAbi (e.expectedAddress p symbol)

def callbackCandidates (e : Environment) : Prop :=
  ∀ ev ∈ e.trace, ev.action = .invoke →
    e.memberCandidate ev.member ev.function ∧
    (e.staticDefault ev.member ev.function ∨
      ∃ reg ∈ e.trace, reg.action = .register ∧
        reg.function = ev.function ∧ reg.member = ev.member ∧
        reg.time < ev.time ∧ reg.result = 0 ∧ reg.publicSetter = true)

def publicCallbackRegistration (e : Environment) (p : Nat) : Prop :=
  ∀ reg ∈ e.trace, reg.plugin = p → reg.action = .register →
    reg.publicSetter = true ∧ reg.result = 0 ∧
    e.memberCandidate reg.member reg.function

def interceptorCandidates (e : Environment) (p : Nat) : Prop :=
  ∀ reg ∈ e.trace, reg.plugin = p → reg.action = .intercept →
    reg.publicSetter = true ∧ reg.result = 0 ∧ reg.exactCandidate = true

def registrationLifecycle (e : Environment) (p : Nat) : Prop :=
  ∀ reg ∈ e.trace, reg.plugin = p → reg.action = .intercept →
    ((reg.configRegistration = true ∧ reg.confLive = true) ∨
     (reg.configRegistration = false ∧ reg.fromOnNew = true ∧
      reg.initialized = false ∧ reg.shareConsumer = false))

def interceptorRegistration (e : Environment) (p : Nat) : Prop :=
  interceptorCandidates e p ∧ registrationLifecycle e p

def interceptorCustody (e : Environment) (p : Nat) : Prop :=
  ∀ ev ∈ e.trace, ev.plugin = p → ev.inInterceptor = true →
    ev.pointerBorrowed = true ∧ ev.scalarBorrowed = true ∧
    (ev.action = .release → ev.localResource = true) ∧
    ev.action ≠ .retain ∧ ev.action ≠ .transfer ∧
    ev.action ≠ .globalCustody ∧
    (ev.action = .acquire → ∃ drop ∈ e.trace,
      drop.plugin = p ∧ drop.resource = ev.resource ∧
      drop.activation = ev.activation ∧ drop.action = .returnLocal ∧
      ev.time < drop.time ∧ drop.time < e.returnBoundary ev.activation)

def codeContextLifetime (e : Environment) (p : Nat) : Prop :=
  ∀ ev ∈ e.trace, ev.plugin = p →
    (ev.action = .invoke ∨ ev.action = .destroyCallback) →
    e.codeLive ev.function ev.time ∧ e.contextLive ev.opaqueId ev.time

def unloadOrder (e : Environment) (p : Nat) : Prop :=
  (∀ stop ∈ e.trace, stop.plugin = p → stop.action = .close →
    stop.handle = e.handleFor p → e.refCount stop.handle stop.time = 0 →
    ∀ call ∈ e.trace, call.plugin = p →
      (call.action = .invoke ∨ call.action = .destroyCallback) →
      call.time < stop.time) ∧
  (∀ cp ∈ e.trace, cp.plugin = p → cp.action = .copyReference →
    0 < e.refCount (e.handleFor p) cp.time)

def cjsonHooks (e : Environment) (p : Nat) : Prop :=
  ∀ ev ∈ e.trace, ev.plugin = p →
    ev.action ≠ .hookInstall ∧ ev.action ≠ .hookReplace

def cjsonShape (e : Environment) (p : Nat) : Prop :=
  ∀ ev ∈ e.trace, ev.plugin = p →
    ev.action ≠ .cjsonLinkWrite ∧
    (ev.action = .cjsonCall → ev.properList = true)

def internalFrameApi (e : Environment) (p : Nat) : Prop :=
  ∀ ev ∈ e.trace, ev.plugin = p →
    ev.action ≠ .internalWrite ∧
    (ev.action = .apiCall → ev.isRkCall = true →
      ev.publicRkSymbol = true ∧ ev.apiPrecondition = true)

def coreFrame (e : Environment) (p : Nat) : Prop :=
  ∀ ev ∈ e.trace, ev.plugin = p → ev.action ≠ .coreWrite

def handedAccess (e : Environment) (p : Nat) : Prop :=
  ∀ ev ∈ e.trace, ev.plugin = p →
    ev.action ≠ .forge ∧ ev.forged = false ∧
    (ev.action = .access → ev.handedObject = true ∨ ev.exportedApi = true)

def preexistingAllocatorHooks (e : Environment) (p : Nat) : Prop :=
  e.preexistingHooksValid p

structure RkProposed (e : Environment) (p : Nat) : Prop where
  loader : loaderEnvironment e p
  artifact : artifactSymbolAbi e p
  callback : publicCallbackRegistration e p
  registration : interceptorRegistration e p
  custody : interceptorCustody e p
  lifetime : codeContextLifetime e p
  unload : unloadOrder e p
  hooks : cjsonHooks e p
  shape : cjsonShape e p
  frame : internalFrameApi e p
  previousHooks : preexistingAllocatorHooks e p

structure FluentProposed (e : Environment) (p : Nat) : Prop where
  common : RkProposed e p
  core : coreFrame e p
  access : handedAccess e p

def outerBridge (e : Environment) : Prop :=
  (∀ p, e.loaded p .rk → RkProposed e p) ∧
  (∀ p, (e.loaded p .fluentC ∨ e.loaded p .fluentGo) → FluentProposed e p)

def loadedArtifacts (e : Environment) : Prop :=
  ∀ p, Loaded e p → loaderEnvironment e p ∧ artifactSymbolAbi e p

def loadedRegistration (e : Environment) : Prop :=
  ∀ p, Loaded e p → interceptorRegistration e p

def loadedCustody (e : Environment) : Prop :=
  ∀ p, Loaded e p → interceptorCustody e p

def loadedLifetime (e : Environment) : Prop :=
  ∀ p, Loaded e p → codeContextLifetime e p

def loadedUnload (e : Environment) : Prop :=
  ∀ p, Loaded e p → unloadOrder e p

structure CallbackRefinement (e : Environment) : Prop where
  artifact : loadedArtifacts e
  callback : callbackCandidates e
  registration : loadedRegistration e
  custody : loadedCustody e
  lifetime : loadedLifetime e
  unload : loadedUnload e
  bridge : outerBridge e

theorem resolved_loaded_symbol_has_expected_abi
    (h : CallbackRefinement e) (loaded : Loaded e p)
    (requiredSymbol : e.requiredSymbol p symbol) :
    e.resolvedAddress p symbol ≠ 0 ∧
      e.actualAbi (e.resolvedAddress p symbol) =
        e.expectedAbi (e.expectedAddress p symbol) := by
  exact ⟨((h.artifact p loaded).2.2 symbol requiredSymbol).2.1,
    ((h.artifact p loaded).2.2 symbol requiredSymbol).2.2⟩

theorem loaded_callback_after_close_is_refuted
    (loaded : Loaded e p) (stopIn : stop ∈ e.trace) (callIn : call ∈ e.trace)
    (sameStop : stop.plugin = p) (sameCall : call.plugin = p)
    (closed : stop.action = .close) (handle : stop.handle = e.handleFor p)
    (lastClose : e.refCount stop.handle stop.time = 0)
    (invoked : call.action = .invoke)
    (late : stop.time ≤ call.time) : ¬ CallbackRefinement e := by
  intro h
  have earlier := (h.unload p loaded).1 stop stopIn sameStop closed handle lastClose
    call callIn sameCall (Or.inl invoked)
  exact Nat.not_lt_of_ge late earlier

theorem loaded_fluent_core_store_is_refuted
    (loaded : e.loaded p .fluentC ∨ e.loaded p .fluentGo)
    (eventIn : ev ∈ e.trace) (same : ev.plugin = p)
    (write : ev.action = .coreWrite) : ¬ CallbackRefinement e := by
  intro h
  exact ((h.bridge.2 p loaded).core ev eventIn same) write

theorem callback_without_registration_or_default_is_refuted
    (eventIn : ev ∈ e.trace) (invoked : ev.action = .invoke)
    (noDefault : ¬ e.staticDefault ev.member ev.function)
    (noRegistration : ¬ ∃ reg ∈ e.trace, reg.action = .register ∧
      reg.function = ev.function ∧ reg.member = ev.member ∧
      reg.time < ev.time ∧ reg.result = 0 ∧ reg.publicSetter = true) :
    ¬ CallbackRefinement e := by
  intro h
  rcases (h.callback ev eventIn invoked).2 with default | registered
  · exact noDefault default
  · exact noRegistration registered

/-! Property-specific metadata erasure models the bounds/null parser boundary.
The generic graph checker remains a prerequisite, not proved by this erasure. -/
inductive Clause (α : Type) where
  | semantic (value : α)
  | atom (id : Nat)
  | refinement (children : List Nat)

def semanticClauses : List (Clause α) → List α
  | [] => []
  | .semantic value :: rest => value :: semanticClauses rest
  | .atom _ :: rest => semanticClauses rest
  | .refinement _ :: rest => semanticClauses rest

theorem semanticClauses_append (left right : List (Clause α)) :
    semanticClauses (left ++ right) = semanticClauses left ++ semanticClauses right := by
  induction left with
  | nil => rfl
  | cons head rest ih => cases head <;> simp [semanticClauses, ih]

def attach (original : List (Clause α)) (atoms : List Nat) : List (Clause α) :=
  atoms.map Clause.atom ++ Clause.refinement atoms :: original

theorem attachment_preserves_original_clause_interpretation
    (original : List (Clause α)) (atoms : List Nat) :
    semanticClauses (attach original atoms) = semanticClauses original := by
  have erased : semanticClauses (atoms.map (Clause.atom (α := α))) = [] := by
    induction atoms with
    | nil => rfl
    | cons a rest ih => simpa [semanticClauses] using ih
  simp [attach, semanticClauses_append, erased, semanticClauses]

theorem attachment_preserves_any_existing_null_checker
    (checker : List α → β) (original : List (Clause α)) (atoms : List Nat) :
    checker (semanticClauses (attach original atoms)) =
      checker (semanticClauses original) := by
  rw [attachment_preserves_original_clause_interpretation]

structure Instance where
  context : HypothesisGraph.Context
  dependencies : List Nat

def affected (expected : HypothesisGraph.Context) (atom : Nat) (i : Instance) : Bool :=
  decide (i.context = expected) && i.dependencies.contains atom

def impact (expected : HypothesisGraph.Context) (atom : Nat) (instances : List Instance) :=
  instances.filter (affected expected atom)

theorem impact_requires_exact_context_and_dependency
    (expected : HypothesisGraph.Context) (atom : Nat) (instances : List Instance)
    (i : Instance) :
    i ∈ impact expected atom instances ↔
      i ∈ instances ∧ i.context = expected ∧ atom ∈ i.dependencies := by
  simp [impact, affected]

theorem foreign_property_is_not_selected
    (expected : HypothesisGraph.Context) (atom : Nat) (i : Instance)
    (different : i.context.property ≠ expected.property) : affected expected atom i = false := by
  have contexts : i.context ≠ expected := by
    intro equal
    exact different (congrArg HypothesisGraph.Context.property equal)
  simp [affected, contexts]

end QCore.PluginPremiseDecomposition

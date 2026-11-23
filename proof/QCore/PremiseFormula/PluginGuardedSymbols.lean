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

import QCore.QuantProposition
import QCore.GuardedContractApplications

/-!
# The plugin guarded symbols as formulas (card SP9-PLUGIN-GUARDED-APPLICATION-FORMULAS)

Contract specification C04.18.6 and C04.18.9 (the SP9 block). The frozen
plugin guarded sidecar
(`docs/reports/premise-guarded-decomposition/applications/plugin.json`) bound
14 English symbols to `rd_kafka_dump0`'s seven `plugin_*` typed atoms. Each
is now a typed-rendering formula over the SP9 block, per the user's
decisions of 2026-11-23:

* **D1 A′.** Every SP9 term is prefixed `plg_`, and the environment stays
  implicit. `#plg_X` is the typed atoms' parameter `#X` read in the selected
  `PluginPremiseDecomposition.Environment`, with its environment argument
  dropped.
* **D2 b.** `default_lifetime_valid` states code and context liveness only
  (`defaultLifetime`). This follows from `lifetimePostcondition`
  (`lifetime_entails_default_lifetime`), but the converse does not hold.
* **D3 I2.** `unregistered` is the warrant term
  `#plg_refutes_registration(#plg_ev)`. It entails `unregistered`
  (`unregistered_symbol_entails_unregistered`), and no model derives it.

`render g` interprets the SP9 terms in one `GuardedInvocation`. It holds the
selected environment `g.env` and the reached invocation `g.ev` (`#plg_ev`)
that the cases and postconditions are read at. It also holds three
propositions that no environment fact supplies:

* the admission (`#plg_admits_callback_closure()`);
* the trace correspondence (`#plg_trace_corresponds()`, E2-shaped);
* the refutation of registration (`#plg_refutes_registration`), constrained
  only to entail `unregistered` (`refutes_sound`).

Integer-valued components are read as integers: a trace position `t` is
`Int.ofNat`, and a Boolean flag is `1` or `0`.

Each symbol holds exactly when its existing definition does
(`plugin_guarded_symbols_are_the_definitions`):

* the seven prerequisites are `PluginPremiseDecomposition`'s `loadedArtifacts`,
  `callbackCandidates`, `loadedLifetime`, `loadedCustody`,
  `loadedRegistration`, `loadedUnload` and `outerBridge`;
* the cases are `GuardedContractApplications.Plugin`'s `registeredCase` and
  `defaultCase`;
* `registered_lifetime_valid` is `lifetimePostcondition`.

Nothing here establishes that an environment describes a run. The symbols
stay proposed metadata.
-/

namespace QCore.PluginGuardedSymbols

open QCore.QuantProposition
open QCore.PluginPremiseDecomposition
open QCore.GuardedContractApplications.Plugin

/-- The SP9 value sorts, in the order of the Rust table `RENDERED_VALUE_SORTS`. -/
def sp9Sorts : List String := ["plg_event", "plg_plugin", "plg_family", "plg_action", "plg_callback", "plg_member", "plg_context", "plg_handle", "plg_activation", "plg_resource", "plg_symbol", "plg_digest", "plg_abi"]

/-- The SP9 block, in the order of the Rust table `RENDERED_TERMS`. -/
def sp9Terms : List String := ["plg_in_trace", "plg_action", "plg_plugin", "plg_function", "plg_member", "plg_opaque", "plg_handle", "plg_resource", "plg_activation", "plg_time", "plg_result", "plg_public_setter", "plg_exact_candidate", "plg_pointer_borrowed", "plg_scalar_borrowed", "plg_local_resource", "plg_conf_live", "plg_from_on_new", "plg_initialized", "plg_share_consumer", "plg_config_registration", "plg_in_interceptor", "plg_invoke", "plg_register", "plg_intercept", "plg_acquire", "plg_release", "plg_retain", "plg_transfer", "plg_return_local", "plg_global_custody", "plg_close", "plg_destroy_callback", "plg_copy_reference", "plg_rk", "plg_fluent_c", "plg_fluent_go", "plg_loaded", "plg_loader_enabled", "plg_actual_build", "plg_expected_build", "plg_actual_inventory", "plg_expected_inventory", "plg_actual_artifact", "plg_expected_artifact", "plg_required_symbol", "plg_resolved_address", "plg_expected_address", "plg_actual_abi", "plg_expected_abi", "plg_member_candidate", "plg_static_default", "plg_code_live", "plg_context_live", "plg_handle_for", "plg_ref_count", "plg_return_boundary", "plg_rk_proposed", "plg_fluent_proposed", "plg_ev", "plg_admits_callback_closure", "plg_trace_corresponds", "plg_refutes_registration"]

/-! ## The invocation model -/

/-- The opaque values of the SP9 sorts. -/
inductive Carrier where
  | event (ev : Event)
  | plugin (p : Nat)
  | family (f : Family)
  | action (a : Action)
  | callback (f : Nat)
  | member (m : Nat)
  | context (x : Nat)
  | handle (h : Nat)
  | activation (a : Nat)
  | resource (r : Nat)
  | symbol (s : Nat)
  | digest (d : String)
  | abi (a : String)

/-- The SP9 sort of an opaque value. -/
def sortOf : Carrier → String
  | .event _ => "plg_event"
  | .plugin _ => "plg_plugin"
  | .family _ => "plg_family"
  | .action _ => "plg_action"
  | .callback _ => "plg_callback"
  | .member _ => "plg_member"
  | .context _ => "plg_context"
  | .handle _ => "plg_handle"
  | .activation _ => "plg_activation"
  | .resource _ => "plg_resource"
  | .symbol _ => "plg_symbol"
  | .digest _ => "plg_digest"
  | .abi _ => "plg_abi"

/-- A selected environment, the reached invocation the decomposition is read
at, and three propositions no environment fact supplies: the admission, the
trace correspondence (E2) and the refutation of registration (I2), which
entails `unregistered` and nothing more. -/
structure GuardedInvocation where
  env : Environment
  ev : Event
  admitsCallbackClosure : Prop
  traceCorresponds : Prop
  refutesRegistration : Event → Prop
  refutes_sound : ∀ e, refutesRegistration e → unregistered env e

/-- A Boolean component read as an integer flag. -/
def flag (b : Bool) : Int := if b then 1 else 0

@[simp] theorem flag_eq_one (b : Bool) : flag b = 1 ↔ b = true := by
  cases b <;> simp [flag]

@[simp] theorem flag_eq_zero (b : Bool) : flag b = 0 ↔ b = false := by
  cases b <;> simp [flag]

def sortHolds (s : String) : SemanticValue Carrier → Prop
  | .opaque c => sortOf c = s
  | .integer _ => False

/-- Each predicate term's model component. -/
def relation (g : GuardedInvocation) (name : String) : List (SemanticValue Carrier) → Prop
  | [] =>
      (name = "plg_trace_corresponds" ∧ g.traceCorresponds) ∨
      (name = "plg_admits_callback_closure" ∧ g.admitsCallbackClosure)
  | [.opaque (.event ev)] =>
      (name = "plg_in_trace" ∧ ev ∈ g.env.trace) ∨
      (name = "plg_refutes_registration" ∧ g.refutesRegistration ev)
  | [.opaque (.plugin p)] =>
      (name = "plg_rk_proposed" ∧ RkProposed g.env p) ∨
      (name = "plg_fluent_proposed" ∧ FluentProposed g.env p)
  | [.opaque (.plugin p), .opaque (.family f)] => name = "plg_loaded" ∧ g.env.loaded p f
  | [.opaque (.plugin p), .opaque (.symbol s)] =>
      name = "plg_required_symbol" ∧ g.env.requiredSymbol p s
  | [.opaque (.member m), .opaque (.callback f)] =>
      (name = "plg_member_candidate" ∧ g.env.memberCandidate m f) ∨
      (name = "plg_static_default" ∧ g.env.staticDefault m f)
  | [.opaque (.callback f), .integer t] => name = "plg_code_live" ∧ g.env.codeLive f t.toNat
  | [.opaque (.context x), .integer t] => name = "plg_context_live" ∧ g.env.contextLive x t.toNat
  | _ => False

/-- An event's component. -/
def eventFunction (name : String) (ev : Event) : Option (SemanticValue Carrier) :=
  if name = "plg_action" then some (.opaque (.action ev.action))
  else if name = "plg_plugin" then some (.opaque (.plugin ev.plugin))
  else if name = "plg_function" then some (.opaque (.callback ev.function))
  else if name = "plg_member" then some (.opaque (.member ev.member))
  else if name = "plg_opaque" then some (.opaque (.context ev.opaqueId))
  else if name = "plg_handle" then some (.opaque (.handle ev.handle))
  else if name = "plg_resource" then some (.opaque (.resource ev.resource))
  else if name = "plg_activation" then some (.opaque (.activation ev.activation))
  else if name = "plg_time" then some (.integer ev.time)
  else if name = "plg_result" then some (.integer ev.result)
  else if name = "plg_public_setter" then some (.integer (flag ev.publicSetter))
  else if name = "plg_exact_candidate" then some (.integer (flag ev.exactCandidate))
  else if name = "plg_pointer_borrowed" then some (.integer (flag ev.pointerBorrowed))
  else if name = "plg_scalar_borrowed" then some (.integer (flag ev.scalarBorrowed))
  else if name = "plg_local_resource" then some (.integer (flag ev.localResource))
  else if name = "plg_conf_live" then some (.integer (flag ev.confLive))
  else if name = "plg_from_on_new" then some (.integer (flag ev.fromOnNew))
  else if name = "plg_initialized" then some (.integer (flag ev.initialized))
  else if name = "plg_share_consumer" then some (.integer (flag ev.shareConsumer))
  else if name = "plg_config_registration" then some (.integer (flag ev.configRegistration))
  else if name = "plg_in_interceptor" then some (.integer (flag ev.inInterceptor))
  else none

/-- Each function term's model component. -/
def function (g : GuardedInvocation) (name : String) :
    List (SemanticValue Carrier) → Option (SemanticValue Carrier)
  | [.opaque (.event ev)] => eventFunction name ev
  | [.opaque (.plugin p)] =>
      if name = "plg_actual_build" then some (.opaque (.digest (g.env.actualBuild p)))
      else if name = "plg_expected_build" then some (.opaque (.digest (g.env.expectedBuild p)))
      else if name = "plg_actual_artifact" then some (.opaque (.digest (g.env.actualArtifact p)))
      else if name = "plg_expected_artifact" then
        some (.opaque (.digest (g.env.expectedArtifact p)))
      else if name = "plg_handle_for" then some (.opaque (.handle (g.env.handleFor p)))
      else none
  | [.opaque (.plugin p), .opaque (.symbol s)] =>
      if name = "plg_resolved_address" then some (.integer (g.env.resolvedAddress p s))
      else if name = "plg_expected_address" then some (.integer (g.env.expectedAddress p s))
      else none
  | [.integer a] =>
      if name = "plg_actual_abi" then some (.opaque (.abi (g.env.actualAbi a.toNat)))
      else if name = "plg_expected_abi" then some (.opaque (.abi (g.env.expectedAbi a.toNat)))
      else none
  | [.opaque (.family f)] =>
      if name = "plg_loader_enabled" then some (.integer (flag (g.env.loaderEnabled f)))
      else none
  | [.opaque (.handle h), .integer t] =>
      if name = "plg_ref_count" then some (.integer (g.env.refCount h t.toNat)) else none
  | [.opaque (.activation a)] =>
      if name = "plg_return_boundary" then some (.integer (g.env.returnBoundary a)) else none
  | _ => none

/-- The typed rendering of an invocation. -/
def render (g : GuardedInvocation) : Interpretation Carrier where
  sortHolds := sortHolds
  relation := relation g
  function := function g

/-- The value terms: the invocation, the actions, the families and the two
inventory identities. -/
def valuation (g : GuardedInvocation) : String → SemanticValue Carrier :=
  fun name =>
    if name = "plg_ev" then .opaque (.event g.ev)
    else if name = "plg_invoke" then .opaque (.action .invoke)
    else if name = "plg_register" then .opaque (.action .register)
    else if name = "plg_intercept" then .opaque (.action .intercept)
    else if name = "plg_acquire" then .opaque (.action .acquire)
    else if name = "plg_release" then .opaque (.action .release)
    else if name = "plg_retain" then .opaque (.action .retain)
    else if name = "plg_transfer" then .opaque (.action .transfer)
    else if name = "plg_return_local" then .opaque (.action .returnLocal)
    else if name = "plg_global_custody" then .opaque (.action .globalCustody)
    else if name = "plg_close" then .opaque (.action .close)
    else if name = "plg_destroy_callback" then .opaque (.action .destroyCallback)
    else if name = "plg_copy_reference" then .opaque (.action .copyReference)
    else if name = "plg_rk" then .opaque (.family .rk)
    else if name = "plg_fluent_c" then .opaque (.family .fluentC)
    else if name = "plg_fluent_go" then .opaque (.family .fluentGo)
    else if name = "plg_actual_inventory" then .opaque (.digest g.env.actualInventory)
    else if name = "plg_expected_inventory" then .opaque (.digest g.env.expectedInventory)
    else .integer 0

/-- A guarded symbol's formula holds of an invocation. -/
def Holds (g : GuardedInvocation) (e : Expr) : Prop := holds (render g) (valuation g) e

/-! ## The 14 symbols (the Rust reader's `lean` rendering of the recorded formulas) -/

def defaultCaseSymbol : Expr := (.and (.predicate "plg_member_candidate" [(.apply "plg_member" [(.var "plg_ev")]), (.apply "plg_function" [(.var "plg_ev")])]) (.predicate "plg_static_default" [(.apply "plg_member" [(.var "plg_ev")]), (.apply "plg_function" [(.var "plg_ev")])]))

def defaultLifetimeValidSymbol : Expr := (.and (.predicate "plg_code_live" [(.apply "plg_function" [(.var "plg_ev")]), (.apply "plg_time" [(.var "plg_ev")])]) (.predicate "plg_context_live" [(.apply "plg_opaque" [(.var "plg_ev")]), (.apply "plg_time" [(.var "plg_ev")])]))

def environmentSymbol : Expr := (.predicate "plg_trace_corresponds" [])

def invokeSymbol : Expr := (.predicate "plg_admits_callback_closure" [])

def artifactSymbolAbiSymbol : Expr := (.forall_ "p" "plg_plugin" (.implies (.exists_ "family" "plg_family" (.predicate "plg_loaded" [(.var "p"), (.var "family")])) (.and (.and (.and (.and (.equal (.apply "plg_actual_build" [(.var "p")]) (.apply "plg_expected_build" [(.var "p")])) (.equal (.var "plg_actual_inventory") (.var "plg_expected_inventory"))) (.forall_ "family" "plg_family" (.implies (.predicate "plg_loaded" [(.var "p"), (.var "family")]) (.equal (.apply "plg_loader_enabled" [(.var "family")]) (.integer 1))))) (.equal (.apply "plg_actual_artifact" [(.var "p")]) (.apply "plg_expected_artifact" [(.var "p")]))) (.forall_ "symbol" "plg_symbol" (.implies (.predicate "plg_required_symbol" [(.var "p"), (.var "symbol")]) (.and (.and (.equal (.apply "plg_resolved_address" [(.var "p"), (.var "symbol")]) (.apply "plg_expected_address" [(.var "p"), (.var "symbol")])) (.not (.equal (.apply "plg_resolved_address" [(.var "p"), (.var "symbol")]) (.integer 0)))) (.equal (.apply "plg_actual_abi" [(.apply "plg_resolved_address" [(.var "p"), (.var "symbol")])]) (.apply "plg_expected_abi" [(.apply "plg_expected_address" [(.var "p"), (.var "symbol")])]))))))))

def callbackCandidatesSymbol : Expr := (.forall_ "ev" "plg_event" (.implies (.and (.predicate "plg_in_trace" [(.var "ev")]) (.equal (.apply "plg_action" [(.var "ev")]) (.var "plg_invoke"))) (.and (.predicate "plg_member_candidate" [(.apply "plg_member" [(.var "ev")]), (.apply "plg_function" [(.var "ev")])]) (.or (.predicate "plg_static_default" [(.apply "plg_member" [(.var "ev")]), (.apply "plg_function" [(.var "ev")])]) (.exists_ "reg" "plg_event" (.and (.and (.and (.and (.and (.and (.predicate "plg_in_trace" [(.var "reg")]) (.equal (.apply "plg_action" [(.var "reg")]) (.var "plg_register"))) (.equal (.apply "plg_function" [(.var "reg")]) (.apply "plg_function" [(.var "ev")]))) (.equal (.apply "plg_member" [(.var "reg")]) (.apply "plg_member" [(.var "ev")]))) (.compare .less (.apply "plg_time" [(.var "reg")]) (.apply "plg_time" [(.var "ev")]))) (.equal (.apply "plg_result" [(.var "reg")]) (.integer 0))) (.equal (.apply "plg_public_setter" [(.var "reg")]) (.integer 1))))))))

def codeContextLifetimeSymbol : Expr := (.forall_ "p" "plg_plugin" (.implies (.exists_ "family" "plg_family" (.predicate "plg_loaded" [(.var "p"), (.var "family")])) (.forall_ "ev" "plg_event" (.implies (.and (.predicate "plg_in_trace" [(.var "ev")]) (.equal (.apply "plg_plugin" [(.var "ev")]) (.var "p"))) (.implies (.or (.equal (.apply "plg_action" [(.var "ev")]) (.var "plg_invoke")) (.equal (.apply "plg_action" [(.var "ev")]) (.var "plg_destroy_callback"))) (.and (.predicate "plg_code_live" [(.apply "plg_function" [(.var "ev")]), (.apply "plg_time" [(.var "ev")])]) (.predicate "plg_context_live" [(.apply "plg_opaque" [(.var "ev")]), (.apply "plg_time" [(.var "ev")])])))))))

def interceptorCustodySymbol : Expr := (.forall_ "p" "plg_plugin" (.implies (.exists_ "family" "plg_family" (.predicate "plg_loaded" [(.var "p"), (.var "family")])) (.forall_ "ev" "plg_event" (.implies (.and (.predicate "plg_in_trace" [(.var "ev")]) (.equal (.apply "plg_plugin" [(.var "ev")]) (.var "p"))) (.implies (.equal (.apply "plg_in_interceptor" [(.var "ev")]) (.integer 1)) (.and (.and (.and (.and (.and (.and (.equal (.apply "plg_pointer_borrowed" [(.var "ev")]) (.integer 1)) (.equal (.apply "plg_scalar_borrowed" [(.var "ev")]) (.integer 1))) (.implies (.equal (.apply "plg_action" [(.var "ev")]) (.var "plg_release")) (.equal (.apply "plg_local_resource" [(.var "ev")]) (.integer 1)))) (.not (.equal (.apply "plg_action" [(.var "ev")]) (.var "plg_retain")))) (.not (.equal (.apply "plg_action" [(.var "ev")]) (.var "plg_transfer")))) (.not (.equal (.apply "plg_action" [(.var "ev")]) (.var "plg_global_custody")))) (.implies (.equal (.apply "plg_action" [(.var "ev")]) (.var "plg_acquire")) (.exists_ "drop" "plg_event" (.and (.and (.and (.and (.and (.and (.predicate "plg_in_trace" [(.var "drop")]) (.equal (.apply "plg_plugin" [(.var "drop")]) (.var "p"))) (.equal (.apply "plg_resource" [(.var "drop")]) (.apply "plg_resource" [(.var "ev")]))) (.equal (.apply "plg_activation" [(.var "drop")]) (.apply "plg_activation" [(.var "ev")]))) (.equal (.apply "plg_action" [(.var "drop")]) (.var "plg_return_local"))) (.compare .less (.apply "plg_time" [(.var "ev")]) (.apply "plg_time" [(.var "drop")]))) (.compare .less (.apply "plg_time" [(.var "drop")]) (.apply "plg_return_boundary" [(.apply "plg_activation" [(.var "ev")])])))))))))))

def interceptorRegistrationSymbol : Expr := (.forall_ "p" "plg_plugin" (.implies (.exists_ "family" "plg_family" (.predicate "plg_loaded" [(.var "p"), (.var "family")])) (.forall_ "reg" "plg_event" (.implies (.and (.predicate "plg_in_trace" [(.var "reg")]) (.equal (.apply "plg_plugin" [(.var "reg")]) (.var "p"))) (.implies (.equal (.apply "plg_action" [(.var "reg")]) (.var "plg_intercept")) (.and (.and (.and (.equal (.apply "plg_public_setter" [(.var "reg")]) (.integer 1)) (.equal (.apply "plg_result" [(.var "reg")]) (.integer 0))) (.equal (.apply "plg_exact_candidate" [(.var "reg")]) (.integer 1))) (.or (.and (.equal (.apply "plg_config_registration" [(.var "reg")]) (.integer 1)) (.equal (.apply "plg_conf_live" [(.var "reg")]) (.integer 1))) (.and (.and (.and (.equal (.apply "plg_config_registration" [(.var "reg")]) (.integer 0)) (.equal (.apply "plg_from_on_new" [(.var "reg")]) (.integer 1))) (.equal (.apply "plg_initialized" [(.var "reg")]) (.integer 0))) (.equal (.apply "plg_share_consumer" [(.var "reg")]) (.integer 0))))))))))

def lifecycleUnloadSymbol : Expr := (.forall_ "p" "plg_plugin" (.implies (.exists_ "family" "plg_family" (.predicate "plg_loaded" [(.var "p"), (.var "family")])) (.and (.forall_ "stop" "plg_event" (.implies (.and (.predicate "plg_in_trace" [(.var "stop")]) (.equal (.apply "plg_plugin" [(.var "stop")]) (.var "p"))) (.implies (.and (.and (.equal (.apply "plg_action" [(.var "stop")]) (.var "plg_close")) (.equal (.apply "plg_handle" [(.var "stop")]) (.apply "plg_handle_for" [(.var "p")]))) (.equal (.apply "plg_ref_count" [(.apply "plg_handle" [(.var "stop")]), (.apply "plg_time" [(.var "stop")])]) (.integer 0))) (.forall_ "call" "plg_event" (.implies (.and (.predicate "plg_in_trace" [(.var "call")]) (.equal (.apply "plg_plugin" [(.var "call")]) (.var "p"))) (.implies (.or (.equal (.apply "plg_action" [(.var "call")]) (.var "plg_invoke")) (.equal (.apply "plg_action" [(.var "call")]) (.var "plg_destroy_callback"))) (.compare .less (.apply "plg_time" [(.var "call")]) (.apply "plg_time" [(.var "stop")])))))))) (.forall_ "copy" "plg_event" (.implies (.and (.predicate "plg_in_trace" [(.var "copy")]) (.equal (.apply "plg_plugin" [(.var "copy")]) (.var "p"))) (.implies (.equal (.apply "plg_action" [(.var "copy")]) (.var "plg_copy_reference")) (.compare .less (.integer 0) (.apply "plg_ref_count" [(.apply "plg_handle_for" [(.var "p")]), (.apply "plg_time" [(.var "copy")])]))))))))

def outerBridgeSymbol : Expr := (.and (.forall_ "p" "plg_plugin" (.implies (.predicate "plg_loaded" [(.var "p"), (.var "plg_rk")]) (.predicate "plg_rk_proposed" [(.var "p")]))) (.forall_ "p" "plg_plugin" (.implies (.or (.predicate "plg_loaded" [(.var "p"), (.var "plg_fluent_c")]) (.predicate "plg_loaded" [(.var "p"), (.var "plg_fluent_go")])) (.predicate "plg_fluent_proposed" [(.var "p")]))))

def registeredCaseSymbol : Expr := (.and (.predicate "plg_member_candidate" [(.apply "plg_member" [(.var "plg_ev")]), (.apply "plg_function" [(.var "plg_ev")])]) (.exists_ "reg" "plg_event" (.and (.and (.and (.and (.and (.and (.predicate "plg_in_trace" [(.var "reg")]) (.equal (.apply "plg_action" [(.var "reg")]) (.var "plg_register"))) (.equal (.apply "plg_function" [(.var "reg")]) (.apply "plg_function" [(.var "plg_ev")]))) (.equal (.apply "plg_member" [(.var "reg")]) (.apply "plg_member" [(.var "plg_ev")]))) (.compare .less (.apply "plg_time" [(.var "reg")]) (.apply "plg_time" [(.var "plg_ev")]))) (.equal (.apply "plg_result" [(.var "reg")]) (.integer 0))) (.equal (.apply "plg_public_setter" [(.var "reg")]) (.integer 1)))))

def registeredLifetimeValidSymbol : Expr := (.and (.and (.predicate "plg_code_live" [(.apply "plg_function" [(.var "plg_ev")]), (.apply "plg_time" [(.var "plg_ev")])]) (.predicate "plg_context_live" [(.apply "plg_opaque" [(.var "plg_ev")]), (.apply "plg_time" [(.var "plg_ev")])])) (.and (.forall_ "stop" "plg_event" (.implies (.and (.predicate "plg_in_trace" [(.var "stop")]) (.equal (.apply "plg_plugin" [(.var "stop")]) (.apply "plg_plugin" [(.var "plg_ev")]))) (.implies (.and (.and (.equal (.apply "plg_action" [(.var "stop")]) (.var "plg_close")) (.equal (.apply "plg_handle" [(.var "stop")]) (.apply "plg_handle_for" [(.apply "plg_plugin" [(.var "plg_ev")])]))) (.equal (.apply "plg_ref_count" [(.apply "plg_handle" [(.var "stop")]), (.apply "plg_time" [(.var "stop")])]) (.integer 0))) (.forall_ "call" "plg_event" (.implies (.and (.predicate "plg_in_trace" [(.var "call")]) (.equal (.apply "plg_plugin" [(.var "call")]) (.apply "plg_plugin" [(.var "plg_ev")]))) (.implies (.or (.equal (.apply "plg_action" [(.var "call")]) (.var "plg_invoke")) (.equal (.apply "plg_action" [(.var "call")]) (.var "plg_destroy_callback"))) (.compare .less (.apply "plg_time" [(.var "call")]) (.apply "plg_time" [(.var "stop")])))))))) (.forall_ "copy" "plg_event" (.implies (.and (.predicate "plg_in_trace" [(.var "copy")]) (.equal (.apply "plg_plugin" [(.var "copy")]) (.apply "plg_plugin" [(.var "plg_ev")]))) (.implies (.equal (.apply "plg_action" [(.var "copy")]) (.var "plg_copy_reference")) (.compare .less (.integer 0) (.apply "plg_ref_count" [(.apply "plg_handle_for" [(.apply "plg_plugin" [(.var "plg_ev")])]), (.apply "plg_time" [(.var "copy")])])))))))

def unregisteredSymbol : Expr := (.predicate "plg_refutes_registration" [(.var "plg_ev")])


/-! ## Reading a formula: one lemma per connective and per quantified sort -/

section Connectives
variable {α : Type} (m : Interpretation α) (v : String → SemanticValue α)

theorem holds_and_iff (a b : Expr) : holds m v (.and a b) ↔ holds m v a ∧ holds m v b := Iff.rfl
theorem holds_or_iff (a b : Expr) : holds m v (.or a b) ↔ holds m v a ∨ holds m v b := Iff.rfl
theorem holds_implies_iff (a b : Expr) :
    holds m v (.implies a b) ↔ (holds m v a → holds m v b) := Iff.rfl
theorem holds_not_iff (a : Expr) : holds m v (.not a) ↔ ¬ holds m v a := Iff.rfl
theorem holds_predicate_iff (name : String) (arguments : List Term) :
    holds m v (.predicate name arguments) ↔
      ∃ values, arguments.mapM (evaluate m v) = some values ∧ m.relation name values := Iff.rfl
/-- Stated right side first, so the reading is oriented `left = right`. -/
theorem holds_equal_iff (l r : Term) :
    holds m v (.equal l r) ↔ ∃ value, evaluate m v r = some value ∧ evaluate m v l = some value :=
  ⟨fun ⟨x, hl, hr⟩ => ⟨x, hr, hl⟩, fun ⟨x, hr, hl⟩ => ⟨x, hl, hr⟩⟩
theorem holds_compare_iff (o : Comparison) (l r : Term) :
    holds m v (.compare o l r) ↔
      ∃ a b, evaluate m v l = some a ∧ evaluate m v r = some b ∧ ordered o a b := Iff.rfl

end Connectives

section Readings
variable (g : GuardedInvocation)

@[simp] theorem render_relation : (render g).relation = relation g := rfl
@[simp] theorem render_function : (render g).function = function g := rfl

/-- A quantifier over an SP9 sort ranges over exactly that sort's values. -/
theorem holds_forall_sort {β : Type} (v : String → SemanticValue Carrier) (x sort : String)
    (body : Expr) (wrap : β → Carrier) (hs : sort ≠ "integer")
    (members : ∀ c, sortOf c = sort ↔ ∃ b, wrap b = c) :
    holds (render g) v (.forall_ x sort body) ↔
      ∀ b, holds (render g) (QuantProposition.bind v x (.opaque (wrap b))) body := by
  constructor
  · intro h b
    exact h _ (by simp [sortMember, hs, render, sortHolds, members])
  · intro h value hm
    simp only [sortMember, hs, render] at hm
    cases value
    · exact absurd hm (by simp [sortHolds])
    · rename_i c
      obtain ⟨b, rfl⟩ := (members c).1 hm
      exact h b

theorem holds_exists_sort {β : Type} (v : String → SemanticValue Carrier) (x sort : String)
    (body : Expr) (wrap : β → Carrier) (hs : sort ≠ "integer")
    (members : ∀ c, sortOf c = sort ↔ ∃ b, wrap b = c) :
    holds (render g) v (.exists_ x sort body) ↔
      ∃ b, holds (render g) (QuantProposition.bind v x (.opaque (wrap b))) body := by
  constructor
  · rintro ⟨value, hm, h⟩
    simp only [sortMember, hs, render] at hm
    cases value
    · exact absurd hm (by simp [sortHolds])
    · rename_i c
      obtain ⟨b, rfl⟩ := (members c).1 hm
      exact ⟨b, h⟩
  · rintro ⟨b, h⟩
    exact ⟨_, by simp [sortMember, hs, render, sortHolds, members], h⟩

theorem event_members (c : Carrier) : sortOf c = "plg_event" ↔ ∃ e, Carrier.event e = c := by
  cases c <;> simp [sortOf]
theorem plugin_members (c : Carrier) : sortOf c = "plg_plugin" ↔ ∃ p, Carrier.plugin p = c := by
  cases c <;> simp [sortOf]
theorem family_members (c : Carrier) : sortOf c = "plg_family" ↔ ∃ f, Carrier.family f = c := by
  cases c <;> simp [sortOf]
theorem symbol_members (c : Carrier) : sortOf c = "plg_symbol" ↔ ∃ s, Carrier.symbol s = c := by
  cases c <;> simp [sortOf]

@[simp] theorem holds_forall_event (v x body) :
    holds (render g) v (.forall_ x "plg_event" body) ↔
      ∀ e, holds (render g) (QuantProposition.bind v x (.opaque (.event e))) body :=
  holds_forall_sort g v x _ body Carrier.event (by decide) event_members
@[simp] theorem holds_exists_event (v x body) :
    holds (render g) v (.exists_ x "plg_event" body) ↔
      ∃ e, holds (render g) (QuantProposition.bind v x (.opaque (.event e))) body :=
  holds_exists_sort g v x _ body Carrier.event (by decide) event_members
@[simp] theorem holds_forall_plugin (v x body) :
    holds (render g) v (.forall_ x "plg_plugin" body) ↔
      ∀ p, holds (render g) (QuantProposition.bind v x (.opaque (.plugin p))) body :=
  holds_forall_sort g v x _ body Carrier.plugin (by decide) plugin_members
@[simp] theorem holds_forall_family (v x body) :
    holds (render g) v (.forall_ x "plg_family" body) ↔
      ∀ f, holds (render g) (QuantProposition.bind v x (.opaque (.family f))) body :=
  holds_forall_sort g v x _ body Carrier.family (by decide) family_members
@[simp] theorem holds_exists_family (v x body) :
    holds (render g) v (.exists_ x "plg_family" body) ↔
      ∃ f, holds (render g) (QuantProposition.bind v x (.opaque (.family f))) body :=
  holds_exists_sort g v x _ body Carrier.family (by decide) family_members
@[simp] theorem holds_forall_symbol (v x body) :
    holds (render g) v (.forall_ x "plg_symbol" body) ↔
      ∀ s, holds (render g) (QuantProposition.bind v x (.opaque (.symbol s))) body :=
  holds_forall_sort g v x _ body Carrier.symbol (by decide) symbol_members

/-- `invoke` (operation): the admission, never a sampled callback's. -/
theorem invoke_holds : Holds g invokeSymbol ↔ g.admitsCallbackClosure := by
  simp [Holds, invokeSymbol, holds, relation]

/-- `environment` (E2-shaped): the trace correspondence, an independent assumption. -/
theorem environment_holds : Holds g environmentSymbol ↔ g.traceCorresponds := by
  simp [Holds, environmentSymbol, holds, relation]

/-- `unregistered` (I2): the refutation of the invocation's registration. -/
theorem unregistered_holds : Holds g unregisteredSymbol ↔ g.refutesRegistration g.ev := by
  simp [Holds, unregisteredSymbol, holds, evaluate, relation, valuation]

theorem default_case_holds : Holds g defaultCaseSymbol ↔ defaultCase g.env g.ev := by
  simp [Holds, defaultCaseSymbol, holds, evaluate, relation, function, valuation, eventFunction,
    defaultCase]

theorem callback_candidates_holds :
    Holds g callbackCandidatesSymbol ↔ callbackCandidates g.env := by
  simp [Holds, callbackCandidatesSymbol, holds_and_iff, holds_or_iff, holds_implies_iff,
    holds_predicate_iff, holds_equal_iff, holds_compare_iff, evaluate, relation,
    function, valuation, eventFunction, QuantProposition.bind, callbackCandidates, ordered, and_assoc]

/-! ### The seven prerequisites: their atoms' existing definitions -/

theorem artifact_symbol_abi_holds :
    Holds g artifactSymbolAbiSymbol ↔ loadedArtifacts g.env := by
  simp [Holds, artifactSymbolAbiSymbol, holds_and_iff, holds_implies_iff, holds_not_iff,
    holds_predicate_iff, holds_equal_iff, evaluate, relation, function, valuation, eventFunction,
    QuantProposition.bind, loadedArtifacts, Loaded, loaderEnvironment, artifactSymbolAbi,
    and_assoc, Int.natCast_inj]

theorem code_context_lifetime_holds :
    Holds g codeContextLifetimeSymbol ↔ loadedLifetime g.env := by
  simp [Holds, codeContextLifetimeSymbol, holds_and_iff, holds_or_iff, holds_implies_iff,
    holds_predicate_iff, holds_equal_iff, evaluate, relation, function, valuation, eventFunction,
    QuantProposition.bind, loadedLifetime, Loaded, codeContextLifetime]

theorem interceptor_custody_holds :
    Holds g interceptorCustodySymbol ↔ loadedCustody g.env := by
  simp [Holds, interceptorCustodySymbol, holds_and_iff, holds_implies_iff, holds_not_iff,
    holds_predicate_iff, holds_equal_iff, holds_compare_iff, evaluate, relation, function,
    valuation, eventFunction, QuantProposition.bind, loadedCustody, Loaded, interceptorCustody,
    ordered, and_assoc]

theorem interceptor_registration_holds :
    Holds g interceptorRegistrationSymbol ↔ loadedRegistration g.env := by
  simp [Holds, interceptorRegistrationSymbol, holds_and_iff, holds_or_iff, holds_implies_iff,
    holds_predicate_iff, holds_equal_iff, evaluate, relation, function, valuation, eventFunction,
    QuantProposition.bind, loadedRegistration, Loaded, interceptorRegistration,
    interceptorCandidates, registrationLifecycle, and_assoc, imp_and, forall_and]

theorem lifecycle_unload_holds :
    Holds g lifecycleUnloadSymbol ↔ loadedUnload g.env := by
  simp [Holds, lifecycleUnloadSymbol, holds_and_iff, holds_or_iff, holds_implies_iff,
    holds_predicate_iff, holds_equal_iff, holds_compare_iff, evaluate, relation, function,
    valuation, eventFunction, QuantProposition.bind, loadedUnload, Loaded, unloadOrder, ordered,
    and_assoc]

theorem outer_bridge_holds : Holds g outerBridgeSymbol ↔ outerBridge g.env := by
  simp [Holds, outerBridgeSymbol, holds_and_iff, holds_or_iff, holds_implies_iff,
    holds_predicate_iff, evaluate, relation, valuation, QuantProposition.bind, outerBridge]

/-! ### The cases and postconditions, read at the reached invocation `#plg_ev` -/

theorem registered_case_holds : Holds g registeredCaseSymbol ↔ registeredCase g.env g.ev := by
  simp [Holds, registeredCaseSymbol, holds_and_iff, holds_predicate_iff, holds_equal_iff,
    holds_compare_iff, evaluate, relation, function, valuation, eventFunction,
    QuantProposition.bind, registeredCase, ordered, and_assoc]

/-- D2 b: liveness of code and context only; no unload order, no
loaded-envelope truth. -/
def defaultLifetime (e : Environment) (ev : Event) : Prop :=
  e.codeLive ev.function ev.time ∧ e.contextLive ev.opaqueId ev.time

theorem default_lifetime_valid_holds :
    Holds g defaultLifetimeValidSymbol ↔ defaultLifetime g.env g.ev := by
  simp [Holds, defaultLifetimeValidSymbol, holds_and_iff, holds_predicate_iff, evaluate, relation,
    function, valuation, eventFunction, defaultLifetime]

theorem registered_lifetime_valid_holds :
    Holds g registeredLifetimeValidSymbol ↔ lifetimePostcondition g.env g.ev := by
  simp [Holds, registeredLifetimeValidSymbol, holds_and_iff, holds_or_iff, holds_implies_iff,
    holds_predicate_iff, holds_equal_iff, holds_compare_iff, evaluate, relation, function,
    valuation, eventFunction, QuantProposition.bind, lifetimePostcondition, unloadOrder, ordered,
    and_assoc]

end Readings

/-! ## Faithfulness, collected -/

/-- **The 14 plugin guarded symbols are their definitions.** -/
theorem plugin_guarded_symbols_are_the_definitions (g : GuardedInvocation) :
    (Holds g artifactSymbolAbiSymbol ↔ loadedArtifacts g.env) ∧
    (Holds g callbackCandidatesSymbol ↔ callbackCandidates g.env) ∧
    (Holds g codeContextLifetimeSymbol ↔ loadedLifetime g.env) ∧
    (Holds g interceptorCustodySymbol ↔ loadedCustody g.env) ∧
    (Holds g interceptorRegistrationSymbol ↔ loadedRegistration g.env) ∧
    (Holds g lifecycleUnloadSymbol ↔ loadedUnload g.env) ∧
    (Holds g outerBridgeSymbol ↔ outerBridge g.env) ∧
    (Holds g environmentSymbol ↔ g.traceCorresponds) ∧
    (Holds g invokeSymbol ↔ g.admitsCallbackClosure) ∧
    (Holds g registeredCaseSymbol ↔ registeredCase g.env g.ev) ∧
    (Holds g defaultCaseSymbol ↔ defaultCase g.env g.ev) ∧
    (Holds g registeredLifetimeValidSymbol ↔ lifetimePostcondition g.env g.ev) ∧
    (Holds g defaultLifetimeValidSymbol ↔ defaultLifetime g.env g.ev) ∧
    (Holds g unregisteredSymbol ↔ g.refutesRegistration g.ev) :=
  ⟨artifact_symbol_abi_holds g, callback_candidates_holds g, code_context_lifetime_holds g,
    interceptor_custody_holds g, interceptor_registration_holds g, lifecycle_unload_holds g,
    outer_bridge_holds g, environment_holds g, invoke_holds g, registered_case_holds g,
    default_case_holds g, registered_lifetime_valid_holds g, default_lifetime_valid_holds g,
    unregistered_holds g⟩

/-- The seven prerequisites together are the existing `CallbackRefinement`. -/
theorem prerequisites_are_the_callback_refinement (g : GuardedInvocation) :
    (Holds g artifactSymbolAbiSymbol ∧ Holds g callbackCandidatesSymbol ∧
      Holds g interceptorRegistrationSymbol ∧ Holds g interceptorCustodySymbol ∧
      Holds g codeContextLifetimeSymbol ∧ Holds g lifecycleUnloadSymbol ∧
      Holds g outerBridgeSymbol) ↔ CallbackRefinement g.env := by
  rw [artifact_symbol_abi_holds, callback_candidates_holds, interceptor_registration_holds,
    interceptor_custody_holds, code_context_lifetime_holds, lifecycle_unload_holds,
    outer_bridge_holds]
  constructor
  · rintro ⟨a, c, r, cu, l, u, b⟩
    exact ⟨a, c, r, cu, l, u, b⟩
  · rintro ⟨a, c, r, cu, l, u, b⟩
    exact ⟨a, c, r, cu, l, u, b⟩

/-! ## The alternatives, the two lifetimes and the independence of the cases -/

/-- Under the candidate prerequisite, a reached invocation is in one of the
two cases (`candidate_premise_decomposes_both_alternatives`). -/
theorem candidates_give_a_case (g : GuardedInvocation)
    (h : Holds g callbackCandidatesSymbol) (reached : g.ev ∈ g.env.trace)
    (invokes : g.ev.action = .invoke) :
    Holds g registeredCaseSymbol ∨ Holds g defaultCaseSymbol := by
  rw [registered_case_holds, default_case_holds]
  rw [callback_candidates_holds] at h
  exact candidate_premise_decomposes_both_alternatives g.env g.ev h reached invokes

/-- D2 b: the registered postcondition entails the default one. -/
theorem lifetime_entails_default_lifetime (e : Environment) (ev : Event)
    (h : lifetimePostcondition e ev) : defaultLifetime e ev :=
  ⟨h.1, h.2.1⟩

/-! ## Concrete models -/

/-- An event of every field zero, invoking. -/
def anEvent : Event where
  action := .invoke
  plugin := 0
  function := 0
  member := 0
  opaqueId := 0
  handle := 0
  resource := 0
  time := 1
  result := 0
  publicSetter := false
  exactCandidate := false
  pointerBorrowed := false
  scalarBorrowed := false
  localResource := false
  confLive := false
  fromOnNew := false
  initialized := false
  shareConsumer := false
  handedObject := false
  exportedApi := false
  isRkCall := false
  publicRkSymbol := false
  forged := false
  apiPrecondition := false
  properList := false
  configRegistration := false
  inInterceptor := false
  activation := 0

/-- An environment whose trace is `trace`, with every candidate a static
default when `defaults` holds, and every code and context live. -/
def anEnvironment (trace : List Event) (candidates defaults : Prop) : Environment where
  trace := trace
  loaded := fun _ _ => False
  loaderEnabled := fun _ => false
  actualArtifact := fun _ => ""
  expectedArtifact := fun _ => ""
  actualBuild := fun _ => ""
  expectedBuild := fun _ => ""
  requiredSymbol := fun _ _ => False
  resolvedAddress := fun _ _ => 0
  expectedAddress := fun _ _ => 0
  actualAbi := fun _ => ""
  expectedAbi := fun _ => ""
  memberCandidate := fun _ _ => candidates
  staticDefault := fun _ _ => defaults
  codeLive := fun _ _ => True
  contextLive := fun _ _ => True
  handleFor := fun _ => 0
  refCount := fun _ _ => 0
  returnBoundary := fun _ => 0
  expectedInventory := ""
  actualInventory := ""
  preexistingHooksValid := fun _ => True

/-- An invocation in an environment, with neither assumption and no admission. -/
def bare (e : Environment) (ev : Event) : GuardedInvocation where
  env := e
  ev := ev
  admitsCallbackClosure := False
  traceCorresponds := False
  refutesRegistration := fun _ => False
  refutes_sound := fun _ h => h.elim

/-- D2 b is strictly weaker: an invocation whose code and context are live
but which a final unload precedes satisfies the default postcondition and not
the registered one. -/
theorem default_lifetime_does_not_give_the_unload_order :
    ∃ e ev, defaultLifetime e ev ∧ ¬ lifetimePostcondition e ev := by
  let close : Event := { anEvent with action := .close, time := 0 }
  refine ⟨anEnvironment [close, anEvent] True True, anEvent, ⟨trivial, trivial⟩, ?_⟩
  rintro ⟨_, _, order, _⟩
  have := order close (by simp [anEnvironment]) rfl rfl rfl rfl anEvent
    (by simp [anEnvironment]) rfl (Or.inl rfl)
  simp [close, anEvent] at this

/-- `default_case` does not assert "not registered": both cases hold of an
invocation with a stated default and a prior successful public registration. -/
theorem default_case_is_independent_of_registration :
    ∃ g : GuardedInvocation, Holds g registeredCaseSymbol ∧ Holds g defaultCaseSymbol := by
  let reg : Event := { anEvent with action := .register, time := 0, publicSetter := true }
  refine ⟨bare (anEnvironment [reg, anEvent] True True) anEvent, ?_, ?_⟩
  · rw [registered_case_holds]
    exact ⟨trivial, reg, by simp [bare, anEnvironment], rfl, rfl, rfl, by decide, rfl, rfl⟩
  · rw [default_case_holds]
    exact ⟨trivial, trivial⟩

/-! ## The two assumptions no model derives (user decisions 2026-11-23) -/

/-- D3 I2: the refutation entails the existing `unregistered`. -/
theorem unregistered_symbol_entails_unregistered (g : GuardedInvocation)
    (h : Holds g unregisteredSymbol) : unregistered g.env g.ev :=
  g.refutes_sound _ ((unregistered_holds g).1 h)

/-- A registered or defaulted invocation is never refuted. -/
theorem a_case_excludes_refutation (g : GuardedInvocation)
    (h : Holds g registeredCaseSymbol ∨ Holds g defaultCaseSymbol) :
    ¬ Holds g unregisteredSymbol := by
  intro hu
  have u := unregistered_symbol_entails_unregistered g hu
  rcases h with r | d
  · exact u.1 ((registered_case_holds g).1 r)
  · exact u.2 ((default_case_holds g).1 d)

/-- `#plg_trace_corresponds()` is never derived: every environment and
invocation has a model in which it fails. -/
theorem trace_corresponds_is_never_derived (e : Environment) (ev : Event) :
    ∃ g : GuardedInvocation, g.env = e ∧ g.ev = ev ∧ ¬ Holds g environmentSymbol :=
  ⟨bare e ev, rfl, rfl, by rw [environment_holds]; exact id⟩

/-- Nor is the refutation of registration. -/
theorem registration_refutation_is_never_derived (e : Environment) (ev : Event) :
    ∃ g : GuardedInvocation, g.env = e ∧ g.ev = ev ∧ ¬ Holds g unregisteredSymbol :=
  ⟨bare e ev, rfl, rfl, by rw [unregistered_holds]; exact id⟩

/-- Nor the admission. -/
theorem admission_is_never_derived (e : Environment) (ev : Event) :
    ∃ g : GuardedInvocation, g.env = e ∧ g.ev = ev ∧ ¬ Holds g invokeSymbol :=
  ⟨bare e ev, rfl, rfl, by rw [invoke_holds]; exact id⟩

/-- A failed registration proof is not refutation: an invocation with no
candidate, so neither case (`unregistered` holds), does not satisfy the I2
symbol without the evidence. -/
theorem absence_of_registration_is_not_refutation :
    unregistered (anEnvironment [anEvent] False False) anEvent ∧
      ¬ Holds (bare (anEnvironment [anEvent] False False) anEvent) unregisteredSymbol := by
  refine ⟨⟨fun h => h.1, fun h => h.1⟩, ?_⟩
  rw [unregistered_holds]
  exact id

end QCore.PluginGuardedSymbols

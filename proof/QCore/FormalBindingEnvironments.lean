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
import QCore.PluginPremiseDecomposition
import QCore.PremiseFormula.CjsonGuardedSymbols
import QCore.PremiseFormula.PluginGuardedSymbols

/-!
# Typed environment references (card SP10-FORMAL-BINDING-DESCRIPTIONS)

Contract specification C04.18.2 (binding manifests), C04.18.3 (flow models)
and C04.18.9 (the SP10 block). A binding's environment names its components
by typed terms, never by English descriptions:

* each component is a typed environment reference: a list of terms, each a
  parameter of a bound typed atom (`pluginParameters`, `detachParameters`) or
  a registered term (the SP7 block's `#source_corresponds`, the SP10 block
  `sp10Terms`);
* a component that binds exact bytes held elsewhere is a content digest.

`denote` gives each SP10 term its Lean meaning over
`QCore.PluginPremiseDecomposition` and `HypothesisGraph.Context`
(`sp10_terms_denote`). `references` are the 22 recorded descriptions' typed
references, bound to the Rust record; `every_reference_resolves` shows each
term resolves. `#bnd_source_corresponds` is E2's form for every other
environment: a free proposition no theorem derives
(`binding_source_correspondence_is_never_derived`).

Anything else is refused with `QF005`: `environmentVectors` is bound to the
Rust `ENVIRONMENT_DECISION_VECTORS` (`environment_vectors_agree`), and an
English formal-binding field is never admitted (`text_is_not_a_binding_field`).

Nothing here establishes that a model describes the source. The references
are evidence-only metadata.
-/

namespace QCore.FormalBindingEnvironments

open QCore.PluginPremiseDecomposition

/-- The SP10 block, in the order of the Rust table `RENDERED_TERMS`. Since the
user decision of 2026-11-24 (A′, "Prefix, then land") every name is prefixed:
`plg_` for a plugin-model component (environment implicit, SP9's sorts), `bnd_`
for a term of the binding itself. -/
def sp10Terms : List String := ["bnd_source_corresponds", "bnd_held_context", "bnd_resource_view", "bnd_null_view", "bnd_model_view", "plg_hook_install", "plg_hook_replace", "plg_cjson_link_write", "plg_cjson_call", "plg_proper_list", "plg_preexisting_hooks_valid", "plg_core_write", "plg_internal_write", "plg_api_call", "plg_is_rk_call", "plg_public_rk_symbol", "plg_api_precondition", "plg_access", "plg_handed_object", "plg_exported_api", "plg_forge", "plg_forged"]

/-- The SP10 value sorts, in the order of the Rust table `RENDERED_VALUE_SORTS`. -/
def sp10Sorts : List String := ["bnd_verdict_context", "bnd_property_view"]

/-- The A′ scheme (user decision 2026-11-24): every SP10 name is prefixed, so
the block reserves no generic name. -/
def prefixed (p t : String) : Bool := t.toList.take p.length == p.toList

theorem sp10_names_are_prefixed :
    (sp10Terms.all (fun t => prefixed "plg_" t || prefixed "bnd_" t)) = true ∧
      (sp10Sorts.all (prefixed "bnd_")) = true := by
  decide

/-- No SP10 term or sort is one of SP9's: the plugin components SP9 already
registers are reused, never registered twice. -/
theorem sp10_is_disjoint_from_sp9 :
    sp10Terms.all (fun t => !QCore.PluginGuardedSymbols.sp9Terms.contains t) = true ∧
      sp10Sorts.all (fun t => !QCore.PluginGuardedSymbols.sp9Sorts.contains t) = true := by
  decide

/-! ## What each SP10 term denotes -/

/-- The property view a bound graph is replayed from. -/
inductive PropertyView where
  | resource | null | model
  deriving DecidableEq

/-- The meaning of an SP10 term. -/
inductive Denotation where
  /-- `#bnd_source_corresponds`: the free source correspondence. -/
  | sourceCorrespondence
  /-- `#bnd_held_context`: a nine-coordinate `HypothesisGraph.Context`. -/
  | heldContext
  /-- `#bnd_resource_view`, `#bnd_null_view`, `#bnd_model_view`. -/
  | view (v : PropertyView)
  /-- An event action of the plugin model. -/
  | action (a : Action)
  /-- An integer-valued event flag of the plugin model (`flagValue`). -/
  | flag (f : Event → Bool)
  /-- `#plg_preexisting_hooks_valid`: `Environment.preexistingHooksValid`. -/
  | hooksValid

/-- Each SP10 term's meaning. -/
def denote : String → Option Denotation
  | "bnd_source_corresponds" => some .sourceCorrespondence
  | "bnd_held_context" => some .heldContext
  | "bnd_resource_view" => some (.view .resource)
  | "bnd_null_view" => some (.view .null)
  | "bnd_model_view" => some (.view .model)
  | "plg_hook_install" => some (.action .hookInstall)
  | "plg_hook_replace" => some (.action .hookReplace)
  | "plg_cjson_link_write" => some (.action .cjsonLinkWrite)
  | "plg_cjson_call" => some (.action .cjsonCall)
  | "plg_proper_list" => some (.flag Event.properList)
  | "plg_preexisting_hooks_valid" => some .hooksValid
  | "plg_core_write" => some (.action .coreWrite)
  | "plg_internal_write" => some (.action .internalWrite)
  | "plg_api_call" => some (.action .apiCall)
  | "plg_is_rk_call" => some (.flag Event.isRkCall)
  | "plg_public_rk_symbol" => some (.flag Event.publicRkSymbol)
  | "plg_api_precondition" => some (.flag Event.apiPrecondition)
  | "plg_access" => some (.action .access)
  | "plg_handed_object" => some (.flag Event.handedObject)
  | "plg_exported_api" => some (.flag Event.exportedApi)
  | "plg_forge" => some (.action .forge)
  | "plg_forged" => some (.flag Event.forged)
  | _ => none

/-- An event flag read as the integer function the term's sort states
(`function<plg_event,integer>`). -/
def flagValue (f : Event → Bool) (_ : Environment) (ev : Event) : Int :=
  if f ev then 1 else 0

/-- The meaning of `#plg_preexisting_hooks_valid` (`predicate<plg_plugin>`). -/
def hooksValid (e : Environment) (p : Nat) : Prop := e.preexistingHooksValid p

/-- Every SP10 term has a Lean meaning. -/
theorem sp10_terms_denote : sp10Terms.all (fun t => (denote t).isSome) = true := by
  decide

/-- A term outside the block denotes nothing here. -/
theorem unregistered_term_denotes_nothing : denote "environment_holds" = none := by
  decide

theorem proper_list_flag (e : Environment) (ev : Event) :
    flagValue Event.properList e ev = 1 ↔ ev.properList = true := by
  unfold flagValue
  cases ev.properList <;> simp

theorem forged_flag (e : Environment) (ev : Event) :
    flagValue Event.forged e ev = 1 ↔ ev.forged = true := by
  unfold flagValue
  cases ev.forged <;> simp

/-- The frame and pointer components name the model's `handedAccess` and
`internalFrameApi` actions and flags (no bound plugin atom quantifies them). -/
theorem frame_terms_are_model_actions :
    denote "plg_core_write" = some (.action .coreWrite) ∧
      denote "plg_internal_write" = some (.action .internalWrite) ∧
      denote "plg_forge" = some (.action .forge) ∧
      denote "plg_access" = some (.action .access) :=
  ⟨rfl, rfl, rfl, rfl⟩

/-! ## The never-derived source correspondence (E2's form) -/

/-- A binding environment's interpretation: the terms' denotations and the
free source correspondence. -/
structure Interpretation where
  sourceCorresponds : Prop

/-- No theorem establishes the correspondence: for every environment there is
an interpretation in which it fails. -/
theorem binding_source_correspondence_is_never_derived (_ : Environment) :
    ∃ i : Interpretation, ¬ i.sourceCorresponds :=
  ⟨⟨False⟩, id⟩

/-! ## The recorded references -/

def pluginParameters : List String := ["acquire", "action", "activation", "actual_abi", "actual_artifact", "actual_build", "actual_inventory", "close", "code_live", "conf_live", "config_registration", "context_live", "copy_reference", "destroy_callback", "env", "exact_candidate", "expected_abi", "expected_address", "expected_artifact", "expected_build", "expected_inventory", "fluent_c", "fluent_go", "fluent_proposed", "from_on_new", "function", "global_custody", "handle", "handle_for", "in_interceptor", "in_trace", "initialized", "intercept", "invoke", "loaded", "loader_enabled", "local_resource", "member", "member_candidate", "opaque", "plugin", "pointer_borrowed", "public_setter", "ref_count", "register", "release", "required_symbol", "resolved_address", "resource", "result", "retain", "return_boundary", "return_local", "rk", "rk_proposed", "scalar_borrowed", "share_consumer", "static_default", "time", "transfer"]
def detachParameters : List String := ["child", "children", "concurrent", "first", "keeps_links", "last", "live", "next", "next_reach", "nodup", "owned", "parent", "pred", "prev", "runs"]
def references : List (String × String × List String) := [("plugin_selected_loader_trace_v1", "artifact_build_symbols_abi", ["actual_abi", "actual_artifact", "actual_build", "actual_inventory", "expected_abi", "expected_address", "expected_artifact", "expected_build", "expected_inventory", "loader_enabled", "required_symbol", "resolved_address"]), ("plugin_selected_loader_trace_v1", "cjson", ["plg_cjson_call", "plg_cjson_link_write", "plg_hook_install", "plg_hook_replace", "plg_preexisting_hooks_valid", "plg_proper_list"]), ("plugin_selected_loader_trace_v1", "context", ["bnd_held_context"]), ("plugin_selected_loader_trace_v1", "custody", ["acquire", "activation", "global_custody", "in_interceptor", "local_resource", "pointer_borrowed", "release", "resource", "retain", "return_boundary", "return_local", "scalar_borrowed", "transfer"]), ("plugin_selected_loader_trace_v1", "frames", ["plg_access", "plg_api_call", "plg_api_precondition", "plg_core_write", "plg_exported_api", "plg_forge", "plg_forged", "plg_handed_object", "plg_internal_write", "plg_is_rk_call", "plg_public_rk_symbol"]), ("plugin_selected_loader_trace_v1", "lifetime", ["close", "code_live", "context_live", "copy_reference", "destroy_callback", "handle", "handle_for", "invoke", "opaque", "ref_count"]), ("plugin_selected_loader_trace_v1", "plugins", ["fluent_c", "fluent_go", "fluent_proposed", "loaded", "plugin", "rk", "rk_proposed"]), ("plugin_selected_loader_trace_v1", "registration", ["conf_live", "config_registration", "exact_candidate", "from_on_new", "function", "initialized", "intercept", "member", "member_candidate", "public_setter", "register", "result", "share_consumer", "static_default"]), ("plugin_selected_loader_trace_v1", "source_binding", ["bnd_source_corresponds"]), ("plugin_selected_loader_trace_v1", "trace", ["action", "activation", "env", "handle", "in_trace", "member", "opaque", "plugin", "resource", "time"]), ("plugin_selected_loader_trace_v1", "property_view", ["bnd_resource_view"]), ("plugin_selected_loader_trace_v1", "property_view", ["bnd_null_view"]), ("plugin_selected_loader_trace_v1", "property_view", ["bnd_model_view"]), ("cjson_detach_entry_and_interference_v1", "callActor", ["concurrent", "runs"]), ("cjson_detach_entry_and_interference_v1", "child_next_prev", ["child", "next", "next_reach", "prev"]), ("cjson_detach_entry_and_interference_v1", "children", ["children", "first", "last", "nodup", "pred"]), ("cjson_detach_entry_and_interference_v1", "context", ["bnd_held_context"]), ("cjson_detach_entry_and_interference_v1", "live", ["live"]), ("cjson_detach_entry_and_interference_v1", "ownedChild", ["owned"]), ("cjson_detach_entry_and_interference_v1", "parent", ["live", "parent"]), ("cjson_detach_entry_and_interference_v1", "source_mapping", ["source_corresponds"]), ("cjson_detach_entry_and_interference_v1", "trace", ["concurrent", "keeps_links", "runs"])]

/-- A term resolves in an environment: a bound atom's parameter (its own
declared, unprefixed name), an SP10 term, or an SP7 term. -/
def resolves (environment term : String) : Bool :=
  (if environment = "cjson_detach_entry_and_interference_v1" then
      detachParameters.contains term
    else pluginParameters.contains term) ||
    sp10Terms.contains term || QCore.CjsonGuardedSymbols.sp7Terms.contains term

/-- Every recorded reference is nonempty and every term in it resolves. -/
theorem every_reference_resolves :
    references.all (fun (environment, _, terms) =>
      !terms.isEmpty && terms.all (resolves environment)) = true := by
  decide

/-- Every plugin atom parameter belongs to some recorded plugin component. -/
theorem plugin_parameters_are_covered :
    pluginParameters.all (fun p => references.any (fun (environment, _, terms) =>
      environment == "plugin_selected_loader_trace_v1" && terms.contains p)) = true := by
  decide

/-- Every detach atom parameter belongs to some recorded detach component. -/
theorem detach_parameters_are_covered :
    detachParameters.all (fun p => references.any (fun (environment, _, terms) =>
      environment == "cjson_detach_entry_and_interference_v1" && terms.contains p)) = true := by
  decide

/-! ## An English description is not a reference (the `QF005` ban) -/

/-- An environment value: English or any other text, a content digest, or a
JSON list of typed terms (`canonical`: nonempty, strictly increasing by name,
canonical bytes; `resolved`: every term resolves). -/
inductive EnvValue where
  | text
  | digest
  | terms (canonical resolved : Bool)

/-- Admission of an environment value (`formal_binding::admitted`). -/
def admitEnv : EnvValue → Bool
  | .text => false
  | .digest => true
  | .terms canonical resolved => canonical && resolved

/-- Card SP10-FORMAL-BINDING-DESCRIPTIONS: no text is an environment value. -/
theorem text_is_not_an_environment_reference : admitEnv .text = false := rfl

/-- An environment value is admitted exactly when it is a digest or a
canonical, resolved typed reference. -/
theorem environment_admitted_iff (v : EnvValue) :
    admitEnv v = true ↔ v = .digest ∨ v = .terms true true := by
  cases v with
  | text => simp [admitEnv]
  | digest => simp [admitEnv]
  | terms c r => cases c <;> cases r <;> simp [admitEnv]

/-- The decision vectors shared with the Rust table
`qcore_contract::formal_binding::ENVIRONMENT_DECISION_VECTORS`. -/
def environmentVectors : List (EnvValue × Bool) := [(.terms true true, true), (.terms true true, true), (.digest, true), (.text, false), (.text, false), (.text, false), (.text, false), (.text, false), (.terms false true, false), (.terms false true, false), (.terms false true, false), (.terms false true, false), (.terms true false, false), (.terms true false, false)]

theorem environment_vectors_agree :
    environmentVectors.all (fun (v, expected) => admitEnv v == expected) = true := by
  rfl

/-- A formal binding's quantifier or conclusion: text, or the bound typed
atom's own parameters or formula. -/
inductive BindingField where
  | text
  | derived

def admitBinding : BindingField → Bool
  | .text => false
  | .derived => true

/-- An English quantifier or conclusion is never admitted. -/
theorem text_is_not_a_binding_field : admitBinding .text = false := rfl

theorem binding_admitted_iff (f : BindingField) :
    admitBinding f = true ↔ f = .derived := by
  cases f <;> simp [admitBinding]

end QCore.FormalBindingEnvironments

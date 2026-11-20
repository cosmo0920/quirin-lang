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

import QCore.PremiseFormula

/-!
# The Fluent Bit premises as formulas (card SP3-SYMBOLIC-FLUENT-BIT-LIBC-PREMISES)

Contract specification C04.18.9. `quirin-provider-fluent-bit` states three
named premises, each once; each is now a formula over the base vocabulary
and the Fluent Bit block `QCore.PremiseFormula` appends (`FluentBitTerms`).
The definitions below are the trees the Rust reader reads from the modules,
bound by `crates/qcore-frontend/tests/premise_formula.rs`
(`the_fluent_bit_formulas_are_the_lean_definitions`).

Theorems:

* the three formulas (and the ingestion premise without its last conjunct)
  are well-formed (`fluent_bit_formulas_wf`);
* each prose "so" or "means" the premises state is entailed by the formulas:
  the gateway's admission yields faithfulness (`untracked_admitted_faithful`);
  the ingestion premise's last conjunct follows from its first
  (`ingestion_core_entails_premise`); and the in-tree conjunct with the
  production conjunct puts the call's output plugin outside the tree
  (`synchronous_output_not_in_tree`).

A premise stays an undischarged assumption (C04.18): nothing here
establishes that a model describes Fluent Bit.
-/

namespace QCore.PremiseFormula.FluentBit

open QCore.PremiseFormula

/-! ## The three formulas (the modules' statements, as read) -/

/-- `untracked_handback` on `flb_input_chunk_get_name` (`input_chunk_name.qv`). -/
def untrackedHandbackFormula : Raw := (.quant "r" (.app0 "result") (.bin .conj (.bin .conj (.bin .conj (.bin .conj (.bin .conj (.bin .mem (.app0 "r") (.app1 "interior" (.app0 "args"))) (.bin .mem (.app0 "r") (.app1 "chunk_name" (.app0 "args")))) (.bin .mem (.app0 "r") (.bin .diff (.app0 "result") (.bin .union (.app0 "tracked") (.app0 "claimed"))))) (.bin .imp (.app0 "faithful") (.bin .mem (.app0 "r") (.bin .diff (.app0 "result") (.bin .union (.app0 "bound") (.app0 "stored")))))) (.bin .imp (.bin .mem (.app0 "r") (.bin .diff (.app0 "result") (.bin .union (.app0 "bound") (.app0 "stored")))) (.app0 "faithful"))) (.bin .imp (.app0 "admitted") (.bin .mem (.app0 "r") (.bin .diff (.app0 "result") (.bin .union (.app0 "bound") (.app0 "stored")))))))

/-- `ingestion_active` on `flb_input_collector_resume` (`loop_owner.qv`). -/
def ingestionActiveFormula : Raw := (.quant "g" (.app1 "config_of" (.app0 "args")) (.bin .conj (.bin .conj (.bin .imp (.bin .mem (.app0 "g") (.bin .diff (.app1 "config_of" (.app0 "args")) (.app0 "ingesting"))) (.bin .conj (.app0 "returns_zero") (.quant "c" (.app0 "collector") (.bin .mem (.app0 "c") (.bin .diff (.app0 "collector") (.bin .union (.app0 "event_registered") (.app0 "set_running"))))))) (.bin .imp (.bin .mem (.app0 "g") (.app0 "ingesting")) (.quant "c" (.app0 "collector") (.bin .conj (.bin .mem (.app0 "c") (.app0 "event_registered")) (.bin .mem (.app0 "c") (.app0 "set_running")))))) (.bin .imp (.app0 "returns_zero") (.quant "c" (.app0 "collector") (.bin .imp (.bin .mem (.app0 "c") (.app0 "set_running")) (.bin .mem (.app0 "g") (.app0 "ingesting")))))))

/-- `synchronous_output_reachability` on `flb_output_task_singleplex_enqueue`
(`output_singleplex.qv`). -/
def synchronousOutputFormula : Raw := (.bin .conj (.quant "p" (.app0 "in_tree") (.bin .mem (.app0 "p") (.bin .diff (.app0 "in_tree") (.app0 "synchronous")))) (.bin .imp (.app0 "production") (.quant "p" (.app1 "out_plugin" (.app0 "args")) (.bin .conj (.bin .mem (.app0 "p") (.app0 "synchronous")) (.bin .mem (.app0 "p") (.app0 "proxy"))))))

/-- The ingestion premise without its last conjunct ("a return of 0 means
the collector runs only while ingestion is active"). -/
def ingestionCoreFormula : Raw := (.quant "g" (.app1 "config_of" (.app0 "args")) (.bin .conj (.bin .imp (.bin .mem (.app0 "g") (.bin .diff (.app1 "config_of" (.app0 "args")) (.app0 "ingesting"))) (.bin .conj (.app0 "returns_zero") (.quant "c" (.app0 "collector") (.bin .mem (.app0 "c") (.bin .diff (.app0 "collector") (.bin .union (.app0 "event_registered") (.app0 "set_running"))))))) (.bin .imp (.bin .mem (.app0 "g") (.app0 "ingesting")) (.quant "c" (.app0 "collector") (.bin .conj (.bin .mem (.app0 "c") (.app0 "event_registered")) (.bin .mem (.app0 "c") (.app0 "set_running")))))))

theorem fluent_bit_formulas_wf :
    wf untrackedHandbackFormula = true ∧ wf ingestionActiveFormula = true ∧
      wf synchronousOutputFormula = true ∧ wf ingestionCoreFormula = true := by decide

/-! ## What each states -/

/-- What `untracked_handback` states of the result's object. -/
theorem eval_untracked (M : Model) :
    (eval M [] untrackedHandbackFormula).holds ↔
      ∀ r, M.fb.result r →
        ((((M.fb.interior M.args r ∧ M.fb.chunkName M.args r) ∧
            (M.fb.result r ∧ ¬ (M.fb.tracked r ∨ M.fb.claimed r))) ∧
          (M.fb.faithful → M.fb.result r ∧ ¬ (M.fb.bound r ∨ M.fb.stored r))) ∧
          ((M.fb.result r ∧ ¬ (M.fb.bound r ∨ M.fb.stored r)) → M.fb.faithful)) ∧
        (M.fb.admitted → M.fb.result r ∧ ¬ (M.fb.bound r ∨ M.fb.stored r)) := by
  simp [eval, untrackedHandbackFormula, lk, prim0, prim1, binVal, Val.holds]

/-- "which the gateway's admission ... checks per site": where the gateway
admits the call, the declared result clause is faithful for the result's
object. -/
theorem untracked_admitted_faithful (M : Model)
    (h : (eval M [] untrackedHandbackFormula).holds) (ha : M.fb.admitted) :
    ∀ r, M.fb.result r → M.fb.faithful := by
  intro r hr
  have hx := (eval_untracked M).1 h r hr
  exact hx.1.2 (hx.2 ha)

/-- What `ingestion_active` states, per context of the input instance. -/
theorem eval_ingestion (M : Model) :
    (eval M [] ingestionActiveFormula).holds ↔
      ∀ g, M.fb.configOf M.args g →
        (((M.fb.configOf M.args g ∧ ¬ M.fb.ingesting g) →
            M.fb.returnsZero ∧
              ∀ c, M.fb.collector c →
                M.fb.collector c ∧ ¬ (M.fb.eventRegistered c ∨ M.fb.setRunning c)) ∧
          (M.fb.ingesting g → ∀ c, M.fb.collector c →
            M.fb.eventRegistered c ∧ M.fb.setRunning c)) ∧
        (M.fb.returnsZero → ∀ c, M.fb.collector c → M.fb.setRunning c → M.fb.ingesting g) := by
  simp [eval, ingestionActiveFormula, lk, prim0, prim1, binVal, Val.holds]

theorem eval_ingestion_core (M : Model) :
    (eval M [] ingestionCoreFormula).holds ↔
      ∀ g, M.fb.configOf M.args g →
        ((M.fb.configOf M.args g ∧ ¬ M.fb.ingesting g) →
            M.fb.returnsZero ∧
              ∀ c, M.fb.collector c →
                M.fb.collector c ∧ ¬ (M.fb.eventRegistered c ∨ M.fb.setRunning c)) ∧
          (M.fb.ingesting g → ∀ c, M.fb.collector c →
            M.fb.eventRegistered c ∧ M.fb.setRunning c) := by
  simp [eval, ingestionCoreFormula, lk, prim0, prim1, binVal, Val.holds]

/-- "a return of 0 means the collector runs only while ingestion is active"
follows from "the call returns 0 without resuming the collector while
config->is_ingestion_active is false": the premise's last conjunct is
entailed by its first. -/
theorem ingestion_core_entails_premise (M : Model)
    (h : (eval M [] ingestionCoreFormula).holds) :
    (eval M [] ingestionActiveFormula).holds := by
  rw [eval_ingestion]
  intro g hg
  have hc := (eval_ingestion_core M).1 h g hg
  refine ⟨hc, fun _ c hcol hrun => ?_⟩
  apply Classical.byContradiction
  intro hni
  exact ((hc.1 ⟨hg, hni⟩).2 c hcol).2 (Or.inr hrun)

/-- What `synchronous_output_reachability` states. -/
theorem eval_synchronous (M : Model) :
    (eval M [] synchronousOutputFormula).holds ↔
      (∀ p, M.fb.inTree p → M.fb.inTree p ∧ ¬ M.fb.synchronous p) ∧
        (M.fb.production → ∀ p, M.fb.outPlugin M.args p →
          M.fb.synchronous p ∧ M.fb.proxy p) := by
  simp [eval, synchronousOutputFormula, lk, prim0, prim1, binVal, Val.holds]

/-- "in-tree output plugins never set FLB_OUTPUT_SYNCHRONOUS, so this
function runs in production only for an output (a proxy plugin) that does":
in production, the plugin of the call's output instance is not an in-tree
one. -/
theorem synchronous_output_not_in_tree (M : Model)
    (h : (eval M [] synchronousOutputFormula).holds) (hp : M.fb.production) :
    ∀ p, M.fb.outPlugin M.args p → ¬ M.fb.inTree p := by
  intro p hout hin
  have hs := (eval_synchronous M).1 h
  exact (hs.1 p hin).2 (hs.2 hp p hout).1

end QCore.PremiseFormula.FluentBit

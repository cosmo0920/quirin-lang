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
# Outer contract statements as formulas (card SP8-OUTER-CONTRACT-SYMBOLIC-STATEMENTS)

Contract specification C04.20 and C04.18.9's SP8 block. USER DECISION
2026-11-20: conditions are written as symbols, not sentences; user 2026-11-23:
"Launch the outer contract prose card too". The statement of an outer contract
definition, `quant #outer [[contract]] NAME, FORMULA;`, is a formula of the
premise language over the SP8 terms; the string statement is refused
(`QF004`). USER DECISIONS 2026-11-23: D1 A (`#meets_published(#rk_public)`,
librdkafka's published documentation, librdkafka's public functions only),
D2 B (`#provenance(#handed) && #typed_access`), D3 B (`#valid_through_last`,
an interval ending with the destruction of every holder and copy).

* `rkFormula` is `rk_plugin_library`'s statement (`librdkafka/outer.qv`) and
  `fbFormula` is `flb_external_plugin`'s (`fluent_bit/outer.qv`): the Fluent
  Bit statement is its own conjunct (`fbOwnFormula`: no store into a
  core-owned member, only handed objects and exported functions, no forged
  pointer) `&&` the librdkafka statement. Both are well-formed
  (`outer_formulas_wf`) and so denote propositions in every model.
* `fb_entails_rk`: in every model, the Fluent Bit statement entails the
  librdkafka statement, the relation the Fluent Bit sentence stated ("when it
  uses librdkafka", the librdkafka obligations), proved from the formulas.
* The header acceptance is the premise clause acceptance `accept`: a string
  statement is not an outer statement (`string_is_not_an_outer_statement`),
  and `outerVectors` is bound to the Rust `OUTER_DECISION_VECTORS`
  (`outer_vectors_agree`).

The Rust boundary is pinned by `crates/qcore-frontend/tests/premise_formula.rs`
(`the_outer_vectors_are_the_lean_table`,
`the_outer_statements_are_the_lean_definitions`,
`the_inventory_constants_are_the_lean_table`).

## What this is not

An outer statement is a reviewable condition on code outside the audited
program (C04.20): nothing here establishes that any loaded code meets it, and
no static verdict reads it.
-/

namespace QCore.PremiseFormula.Outer

open QCore.PremiseFormula

/-- The Fluent Bit statement's own conjunct. -/
def fbOwnFormula : Raw := (.bin .mapsTo (.app0 "outer_code") (.bin .conj (.bin .conj (.bin .conj (.bin .conj (.app1 "stores_none" (.app0 "flb_core_owned")) (.app1 "accesses_only" (.app0 "handed"))) (.app1 "calls_only" (.app0 "exported"))) (.app1 "provenance" (.app0 "handed"))) (.app0 "typed_access")))

/-- `rk_plugin_library`'s statement. -/
def rkFormula : Raw := (.bin .conj (.bin .conj (.bin .conj (.bin .conj (.bin .conj (.bin .conj (.bin .conj (.quant "f" (.app0 "conf_registrations") (.bin .conj (.bin .mem (.app0 "f") (.app0 "public_setter_values")) (.bin .mem (.app0 "f") (.app0 "meets_candidate")))) (.quant "f" (.app0 "interceptors") (.bin .mem (.app0 "f") (.app1 "zero_appends" (.const "CANDIDATES_5C3F8C4723832C0E9E6EBB784BE6A57EBC0264D519421D5B95A18A8F68C18147"))))) (.quant "f" (.app0 "interceptors") (.bin .mapsTo (.app0 "f") (.bin .conj (.bin .conj (.bin .conj (.bin .conj (.app0 "inert") (.app0 "borrows_args")) (.app0 "borrows_handles")) (.app0 "no_global_custody")) (.app0 "balanced"))))) (.quant "f" (.app1 "copies" (.bin .union (.app0 "conf_registrations") (.app0 "interceptors"))) (.bin .mem (.app0 "f") (.app0 "valid_through_last")))) (.quant "f" (.app0 "interceptors") (.bin .conj (.bin .imp (.bin .mem (.app0 "f") (.app1 "conf_lifecycle" (.const "CANDIDATES_5C3F8C4723832C0E9E6EBB784BE6A57EBC0264D519421D5B95A18A8F68C18147"))) (.bin .mem (.app0 "f") (.app0 "on_live_conf"))) (.bin .imp (.bin .mem (.app0 "f") (.app1 "client_lifecycle" (.const "CANDIDATES_5C3F8C4723832C0E9E6EBB784BE6A57EBC0264D519421D5B95A18A8F68C18147"))) (.bin .conj (.bin .conj (.bin .mem (.app0 "f") (.app0 "from_on_new")) (.bin .mem (.app0 "f") (.app0 "before_init"))) (.bin .mem (.app0 "f") (.bin .diff (.app1 "client_lifecycle" (.const "CANDIDATES_5C3F8C4723832C0E9E6EBB784BE6A57EBC0264D519421D5B95A18A8F68C18147")) (.app0 "on_share_consumer")))))))) (.bin .mapsTo (.app0 "outer_code") (.bin .conj (.bin .conj (.bin .conj (.bin .conj (.bin .conj (.app1 "calls_none" (.app0 "cjson_init_hooks")) (.app1 "stores_none" (.app0 "cjson_global_hooks"))) (.app1 "stores_none" (.app0 "cjson_link_cells"))) (.app1 "stores_none" (.app0 "rk_internal"))) (.app1 "calls_none" (.app0 "rk_nonpublic"))) (.app1 "meets_published" (.app0 "rk_public"))))) (.quant "a" (.app0 "cjson_handed") (.bin .conj (.bin .conj (.bin .conj (.bin .conj (.bin .conj (.bin .conj (.bin .conj (.bin .conj (.bin .conj (.bin .conj (.bin .conj (.quant "c" (.app1 "child" (.app0 "a")) (.bin .mem (.app0 "c") (.app1 "first" (.app0 "a")))) (.quant "c" (.app1 "first" (.app0 "a")) (.bin .mem (.app0 "c") (.app1 "child" (.app0 "a"))))) (.quant "b" (.app1 "children" (.app0 "a")) (.quant "p" (.app2 "pred" (.app0 "a") (.app0 "b")) (.bin .mem (.app0 "b") (.app1 "next" (.app0 "p")))))) (.quant "c" (.app1 "children" (.app0 "a")) (.bin .imp (.bin .mem (.app0 "c") (.app1 "next_reach" (.app0 "c"))) (.app0 "false")))) (.bin .imp (.quant "c" (.app1 "child" (.app0 "a")) (.app0 "false")) (.quant "c" (.app1 "children" (.app0 "a")) (.app0 "false")))) (.bin .imp (.quant "c" (.app1 "children" (.app0 "a")) (.app0 "false")) (.quant "c" (.app1 "child" (.app0 "a")) (.app0 "false")))) (.quant "b" (.app1 "children" (.app0 "a")) (.quant "p" (.app2 "pred" (.app0 "a") (.app0 "b")) (.bin .mem (.app0 "p") (.app1 "prev" (.app0 "b")))))) (.quant "f" (.app1 "first" (.app0 "a")) (.quant "l" (.app1 "last" (.app0 "a")) (.bin .mem (.app0 "l") (.app1 "prev" (.app0 "f")))))) (.app1 "nodup" (.app0 "a"))) (.quant "c" (.app1 "children" (.app0 "a")) (.quant "y" (.bin .union (.app1 "next" (.app0 "c")) (.app1 "prev" (.app0 "c"))) (.bin .mem (.app0 "y") (.app1 "children" (.app0 "a")))))) (.quant "l" (.app1 "last" (.app0 "a")) (.quant "y" (.app1 "next" (.app0 "l")) (.app0 "false")))) (.bin .imp (.bin .mem (.app0 "a") (.app1 "children" (.app0 "a"))) (.app0 "false"))))) (.app0 "allocator_hooks_premise"))

/-- `flb_external_plugin`'s statement: its own conjunct and the librdkafka
statement. -/
def fbFormula : Raw := .bin .conj fbOwnFormula rkFormula

set_option maxRecDepth 16384 in
theorem outer_formulas_wf :
    wf fbOwnFormula = true ∧ wf rkFormula = true ∧ wf fbFormula = true := by
  decide

/-- The Fluent Bit statement entails the librdkafka statement in every model. -/
theorem fb_entails_rk (M : Model) :
    (eval M [] fbFormula).holds → (eval M [] rkFormula).holds := by
  obtain ⟨p, hp⟩ := wf_denotes M outer_formulas_wf.1
  obtain ⟨q, hq⟩ := wf_denotes M outer_formulas_wf.2.1
  have h : eval M [] fbFormula =
      binVal M .conj (eval M [] fbOwnFormula) (eval M [] rkFormula) := rfl
  rw [h, hp, hq]
  simp only [binVal, Val.holds]
  exact fun hpq => hpq.2

/-- And it states its own conjunct besides. -/
theorem fb_states_own (M : Model) :
    (eval M [] fbFormula).holds → (eval M [] fbOwnFormula).holds := by
  obtain ⟨p, hp⟩ := wf_denotes M outer_formulas_wf.1
  obtain ⟨q, hq⟩ := wf_denotes M outer_formulas_wf.2.1
  have h : eval M [] fbFormula =
      binVal M .conj (eval M [] fbOwnFormula) (eval M [] rkFormula) := rfl
  rw [h, hp, hq]
  simp only [binVal, Val.holds]
  exact fun hpq => hpq.1

/-! ## The header acceptance (`QF004`) -/

/-- A string statement is not an outer statement, whatever the string. -/
theorem string_is_not_an_outer_statement (s : String) : accept (.str s) = none := rfl

/-- The outer header decision vectors (the Rust `OUTER_DECISION_VECTORS`). -/
def outerVectors : List (Stmt × Bool) := [
  ((.formula (.bin .mapsTo (.app0 "outer_code") (.app1 "stores_none" (.app0 "flb_core_owned")))), true),
  ((.formula (.quant "f" (.app0 "interceptors") (.bin .mem (.app0 "f") (.app1 "zero_appends" (.const "CANDIDATES_5C3F8C4723832C0E9E6EBB784BE6A57EBC0264D519421D5B95A18A8F68C18147"))))), true),
  ((.formula (.app0 "allocator_hooks_premise")), true),
  ((.str "a loaded plugin stores into no member the core owns"), false),
  ((.str "#outer_code |-> #balanced"), false),
  ((.str "and more"), false),
  ((.formula (.bin .mapsTo (.app0 "outer_code") (.app1 "stores_none" (.app0 "outer_code")))), false),
  ((.formula (.quant "f" (.app0 "interceptors") (.bin .mem (.app0 "f") (.app1 "zero_appends" (.const "CANDIDATES_0000000000000000000000000000000000000000000000000000000000000000"))))), false)
]

set_option maxRecDepth 8192 in
theorem outer_vectors_agree :
    outerVectors.all (fun v => (accept v.1).isSome == v.2) = true := by decide

end QCore.PremiseFormula.Outer

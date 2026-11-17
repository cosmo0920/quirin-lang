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

import QCore.RefusedCallHandOn

/-!
# A field store refused on its owner hands on its value (card CURRENT-CONTEXT-NEW-TRIAGE)

`crates/qcore-c-gateway/src/legacy_transfer.rs` (`refused_store_hand_ons`,
then `legacy_refused_operands::withdraw_refused_call`), at O00 blocker B08's
flow-sensitive owner check: a typed field store `OWNER->F = V` whose owner
does not resolve to one live identity this body acquired is refused **before
its effect** and reports the unresolved owner (`UnknownAlias`).

Like the call refusal `QCore.RefusedCallHandOn` models, that refusal is not
`Unsupported`, so W44-LEAK-PIN-AUDIT's refused hand-on rule never fired there:
the stored value stayed live with no unresolved effect and was accused at
every later exit. Measured on Monkey's `context_new`
(`lib/monkey/tls/tls_openssl.c` at Fluent Bit `046accf6`): the node linked
into the thread context's list (`thctx->contexts = ctx_head`, `thctx` from
`pthread_getspecific`) was accused of two leaks.

The model: a node's refusal is now `unsupported || refusedCall ||
refusedStore`, and the rule is `RefusedHandOnCustody.apply` at that signal
with `handed` the store's value (or, for a call whose result the store would
take, what `RefusedCallHandOn` hands on).

## What is proved

* It is the W44 rule at a wider refusal signal (`is_the_hand_on_rule`), and
  the call rule's signal is a special case (`extends_the_call_rule`), so every
  property proved there holds: it only adds unresolved effects, changes no
  status, and only withdraws accusations (`it_only_withdraws`,
  `it_changes_no_status`).
* An admitted store (the owner resolved) is untouched
  (`an_admitted_store_is_untouched`): the rule says nothing about a store the
  B08 field link models.
* A site the refused store does not name keeps its report exactly
  (`an_unnamed_site_keeps_its_leak`): an object never stored there still
  leaks.
* Non-vacuity: the stored value is withdrawn (`the_stored_value_is_withdrawn`).
* A function whose accusation the rule withdraws is never `Safe`
  (`a_refused_store_is_never_safe`): the store's own refusal is an unresolved
  identity, which classifies the function `Unknown` whenever it carries no
  violation. The withdrawal is not a discharge.

## What this is not

Not an acceptance of the store as a custody transfer: the stored value is
neither discharged nor escaped, and accepting a list root as an owner needs a
clause no rule states (a declared retained-result relation and a checked
owner released on a known path). The extraction premise is unchanged: which
name the store hands on is read off the importer's instruction (trust boundary
28), pinned by `crates/qcore-c-gateway/tests/refused_store_hand_on.rs`.
-/

namespace QCore.RefusedStoreHandOn

open QCore.RefusedHandOnCustody

/-- The rule after a node: refused when the transfer reported `Unsupported`,
refused a call before its effects, or refused a field store on its owner. -/
def applyStore (unsupported refusedCall refusedStore : Bool) (handed : Site → Bool)
    (s : State) : State :=
  apply (unsupported || refusedCall || refusedStore) handed s

theorem is_the_hand_on_rule (u c st : Bool) (h : Site → Bool) (s : State) :
    applyStore u c st h s = apply (u || c || st) h s := rfl

theorem extends_the_call_rule (u c : Bool) (h : Site → Bool) (s : State) :
    applyStore u c false h s = QCore.RefusedCallHandOn.applyCall u c h s := by
  simp [applyStore, QCore.RefusedCallHandOn.applyCall]

theorem it_only_withdraws (u c st : Bool) (h : Site → Bool) (s : State) (x : Site)
    (hx : reported (applyStore u c st h s) x = true) : reported s x = true :=
  the_rule_only_withdraws (u || c || st) h s x hx

theorem it_changes_no_status (u c st : Bool) (h : Site → Bool) (s : State) :
    (applyStore u c st h s).live = s.live ∧ (applyStore u c st h s).exempt = s.exempt :=
  the_rule_changes_no_status (u || c || st) h s

theorem an_admitted_store_is_untouched (h : Site → Bool) (s : State) :
    applyStore false false false h s = s :=
  an_admitted_node_is_untouched h s

theorem an_unnamed_site_keeps_its_leak (u c st : Bool) (h : Site → Bool) (s : State)
    (x : Site) (hx : h x = false) : reported (applyStore u c st h s) x = reported s x :=
  an_unhanded_site_keeps_its_report (u || c || st) h s x hx

theorem the_stored_value_is_withdrawn (h : Site → Bool) (s : State) (x : Site)
    (hh : h x = true) : reported (applyStore false false true h s) x = false := by
  have := a_refused_hand_on_is_withdrawn h s x hh
  simpa [applyStore] using this

/-- The refused store reports an unresolved identity: a refusal, so a function
it fully withdraws is `Unknown`, never `Safe`. -/
theorem a_refused_store_is_never_safe (violation : Bool) :
    classify violation true ≠ .safe :=
  a_withdrawal_by_the_rule_is_never_safe violation

end QCore.RefusedStoreHandOn

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

import QCore.RefusedHandOnCustody

/-!
# A call refused before its effects hands on its actuals (card W45-FB-BACKLOG-CUSTODY)

`crates/qcore-c-gateway/src/legacy_refused_operands.rs`
(`withdraw_refused_call`), called from `legacy_c::transfer` where a contracted
call is refused **before its effects** because a member-address or bound-read
actual's aggregate does not resolve (cards O04-B09B10, O04-READ-BOUND).

That refusal reports the unresolved identity (`UnknownAlias`), not
`Unsupported`, so W44-LEAK-PIN-AUDIT's refused hand-on rule
(`QCore.RefusedHandOnCustody.apply`) never fired there, and an object the
refused call may have published was still accused at the exit: Monkey's
`mk_rconf_section_entry_add` and `mk_vhost_fdt_worker_init`, and the four
standing `MK-LIST-ADD-BORROWED-PUBLICATION` leaves.

The model: a node's refusal is now `unsupported || refusedCall`, and the rule
is `RefusedHandOnCustody.apply` at that signal with `handed` what the call's
contract lets it move (custody positions, container entries, taken bound
reads).

## What is proved

* It is the W44 rule, at a wider refusal signal (`is_the_hand_on_rule`), so
  every property proved there holds: it only adds unresolved effects, changes
  no status, and only withdraws accusations.
* A site the refused call does not name keeps its report exactly
  (`an_unnamed_site_keeps_its_leak`): an object never handed to the refused call
  still leaks.
* A node the call's own refusal marks is never `Safe`
  (`a_refused_call_is_never_safe`): the refusal is reported as an unresolved
  identity, which classifies the function `Unknown` whenever it carries no
  violation.
* Non-vacuity: the object the refused call names is withdrawn (`the_named_object_is_withdrawn`).

## What this is not

The extraction premise of the W44 module is unchanged: that the call's
actuals and aggregates are what the call hands on is read off the classifier's
tokens (trust boundary 28), pinned by
`crates/qcore-c-gateway/tests/refused_call_hand_on.rs`.
-/

namespace QCore.RefusedCallHandOn

open QCore.RefusedHandOnCustody

/-- The rule after a node: refused when the transfer reported `Unsupported` or
refused a call before its effects. -/
def applyCall (unsupported refusedCall : Bool) (handed : Site → Bool) (s : State) : State :=
  apply (unsupported || refusedCall) handed s

theorem is_the_hand_on_rule (u c : Bool) (h : Site → Bool) (s : State) :
    applyCall u c h s = apply (u || c) h s := rfl

theorem it_only_withdraws (u c : Bool) (h : Site → Bool) (s : State) (x : Site)
    (hx : reported (applyCall u c h s) x = true) : reported s x = true :=
  the_rule_only_withdraws (u || c) h s x hx

theorem an_unnamed_site_keeps_its_leak (u c : Bool) (h : Site → Bool) (s : State) (x : Site)
    (hx : h x = false) : reported (applyCall u c h s) x = reported s x :=
  an_unhanded_site_keeps_its_report (u || c) h s x hx

theorem the_named_object_is_withdrawn (h : Site → Bool) (s : State) (x : Site)
    (hh : h x = true) :
    reported (applyCall false true h s) x = false := by
  have := a_refused_hand_on_is_withdrawn h s x hh
  simpa [applyCall] using this

/-- The refused call reports an unresolved identity: a refusal, so a function
it fully withdraws is `Unknown`, never `Safe`. -/
theorem a_refused_call_is_never_safe (violation : Bool) :
    classify violation true ≠ .safe :=
  a_withdrawal_by_the_rule_is_never_safe violation

end QCore.RefusedCallHandOn

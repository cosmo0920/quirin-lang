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

import QCore.OpaqueCallbackSlots

/-!
# Stating the OpenSSL slots the narrowed premise covered (card OPENSSL-UNSTATED-CALLBACK-SLOTS)

Gateway rule 12n and contract specification C04.19.3 (the opaque slot,
`QCore.OpaqueCallbackSlots`), C04.18 (named premises). Abstract rule
theorems over the checker `providers/quirin-provider-openssl/src/check.rs`
(`slot_writers_declared`, `expected_clauses`) and the crate's declarations,
not an extraction of them.

* **Every writer a page names is classified.** The checker accepts a slot
  only when its page's NAME list is exactly the union, over the slots citing
  the page, of their writers and not-writers
  (`a_named_function_is_a_writer_or_a_not_writer`); a function the page names
  that no slot lists refuses every slot citing the page
  (`an_unclassified_name_refuses`), and so does a listed writer the page does
  not name (`an_unnamed_writer_refuses`). So no setter the page documents is
  left out of the run's candidates unremarked (premise
  `P-OPAQUE-SLOT-WRITERS`'s documented half).
* **What a premise covers.** A callback a call may run is a slot or an
  ex_data callback. The slot premise `openssl_unstated_callback_slots` covers
  every callback the declaration does not state; the ex_data premise
  `openssl_ex_data_callbacks` covers ex_data callbacks only
  (`the_ex_data_premise_never_covers_a_slot`). So a declaration that drops
  the slot premise for the ex_data one is covered only where every slot it
  may run is stated (`dropping_the_slot_premise_needs_every_slot_stated`).
* **A premise moves no verdict.** A row's verdict is its own joined with
  each stated invocation's (`rowVerdict`), whatever premise it names
  (`the_verdict_ignores_the_premise`); a row is `Safe` only where every
  stated invocation decides the identity
  (`a_row_is_safe_only_if_every_invocation_decides`), and stating one more
  invocation never improves a verdict
  (`stating_an_invocation_never_improves_a_verdict`). So dropping the
  premise never makes a row `Safe` by itself.

## What this is not

That the pages' NAME sections are the library's complete writer lists, that
a declaration's stated invocations are every slot the call may run (read
from the manual pages), and that the Rust checker computes this set equality
are premises, pinned by the crate's checker tests and
`crates/qcore-c-gateway/tests/openssl_unstated_callback_slots.rs`.
-/

namespace QCore.OpensslUnstatedSlots

open QCore.CallbackContracts (Decision)
open QCore.OpaqueCallbackSlots (invocationVerdict)
open QCore.VerdictSemigroup
open QCore.VerdictSemigroup.V

/-! ## Every writer a page names is classified -/

/-- One stated slot: its page, its writers and the page's other names. -/
structure Slot where
  page : String
  writers : List String
  notWriters : List String
  deriving DecidableEq, Repr

/-- The names the slots citing `page` account for. -/
def accounted (slots : List Slot) (page : String) : List String :=
  (slots.filter fun s => s.page == page).flatMap fun s => s.writers ++ s.notWriters

/-- `slot_writers_declared`'s page check: the NAME list and the accounted
names are the same set. -/
def pageAccepted (slots : List Slot) (page : String) (named : List String) : Bool :=
  named.all (fun n => (accounted slots page).contains n) &&
    (accounted slots page).all (fun n => named.contains n)

theorem a_named_function_is_a_writer_or_a_not_writer {slots : List Slot} {page : String}
    {named : List String} (h : pageAccepted slots page named = true) {n : String}
    (hn : n ∈ named) :
    ∃ s ∈ slots, s.page = page ∧ (n ∈ s.writers ∨ n ∈ s.notWriters) := by
  simp only [pageAccepted, Bool.and_eq_true, List.all_eq_true] at h
  have := h.1 n hn
  simp only [accounted, List.contains_iff_mem, List.mem_flatMap, List.mem_filter,
    beq_iff_eq, List.mem_append] at this
  obtain ⟨s, ⟨hs, hp⟩, hm⟩ := this
  exact ⟨s, hs, hp, hm⟩

theorem an_unclassified_name_refuses {slots : List Slot} {page : String}
    {named : List String} {n : String} (hn : n ∈ named)
    (hu : n ∉ accounted slots page) : pageAccepted slots page named = false := by
  simp only [pageAccepted, Bool.and_eq_false_iff, List.all_eq_false]
  exact Or.inl ⟨n, hn, by simpa [List.contains_iff_mem] using hu⟩

theorem an_unnamed_writer_refuses {slots : List Slot} {page : String}
    {named : List String} {s : Slot} (hs : s ∈ slots) (hp : s.page = page) {w : String}
    (hw : w ∈ s.writers) (hu : w ∉ named) : pageAccepted slots page named = false := by
  simp only [pageAccepted, Bool.and_eq_false_iff, List.all_eq_false]
  refine Or.inr ⟨w, ?_, by simpa [List.contains_iff_mem] using hu⟩
  simp only [accounted, List.mem_flatMap, List.mem_filter, beq_iff_eq, List.mem_append]
  exact ⟨s, ⟨hs, hp⟩, Or.inl hw⟩

/-! ## What a premise covers -/

/-- A callback a call may run: a slot (by key) or an ex_data callback. -/
inductive Runnable where
  | slot (key : String)
  | exData
  deriving DecidableEq, Repr

/-- The premise a declaration names, if any. -/
inductive Premise where
  | none
  | unstatedSlots
  | exData
  deriving DecidableEq, Repr

/-- Whether the declaration states or premises a callback it may run. -/
def covered (stated : List String) (premise : Premise) : Runnable → Bool
  | .slot key => stated.contains key || premise == .unstatedSlots
  | .exData => premise == .unstatedSlots || premise == .exData

/-- A declaration is accepted only where every callback it may run is
covered. -/
def wellCovered (runs : List Runnable) (stated : List String) (premise : Premise) : Bool :=
  runs.all (covered stated premise)

theorem the_ex_data_premise_never_covers_a_slot (stated : List String) (key : String)
    (hk : key ∉ stated) : covered stated .exData (.slot key) = false := by
  simp [covered, hk]

theorem dropping_the_slot_premise_needs_every_slot_stated {runs : List Runnable}
    {stated : List String} {premise : Premise} (hp : premise ≠ .unstatedSlots)
    (h : wellCovered runs stated premise = true) {key : String}
    (hk : Runnable.slot key ∈ runs) : key ∈ stated := by
  simp only [wellCovered, List.all_eq_true] at h
  have := h _ hk
  cases premise <;> simp_all [covered]

theorem no_premise_needs_every_callback_stated {runs : List Runnable}
    {stated : List String} (h : wellCovered runs stated .none = true) :
    Runnable.exData ∉ runs := by
  intro hx
  simp only [wellCovered, List.all_eq_true] at h
  simpa [covered] using h _ hx

/-! ## A premise moves no verdict -/

/-- A row's verdict: its own, joined with each stated invocation's. A named
premise is published beside it, never folded in. -/
def rowVerdict (own : V) (invocations : List Decision) (_premise : Premise) : V :=
  invocations.foldl (fun acc d => join acc (invocationVerdict d)) own

theorem the_verdict_ignores_the_premise (own : V) (ds : List Decision) (p q : Premise) :
    rowVerdict own ds p = rowVerdict own ds q := rfl

theorem fold_safe_start {ds : List Decision} {own : V} {p : Premise}
    (h : rowVerdict own ds p = .safe) : own = .safe := by
  induction ds generalizing own with
  | nil => simpa [rowVerdict] using h
  | cons d ds ih =>
      simp only [rowVerdict, List.foldl_cons] at h ih
      have := ih h
      cases own <;> cases d <;> simp_all [join, rank, invocationVerdict]

theorem a_row_is_safe_only_if_every_invocation_decides {own : V} {ds : List Decision}
    {p : Premise} (h : rowVerdict own ds p = .safe) : ∀ d ∈ ds, d = .identity := by
  induction ds generalizing own with
  | nil => simp
  | cons d ds ih =>
      intro e he
      simp only [rowVerdict, List.foldl_cons] at h ih
      have hstart := fold_safe_start (ds := ds) (p := p) h
      rcases List.mem_cons.mp he with rfl | he
      · exact QCore.OpaqueCallbackSlots.a_row_resting_on_a_slot_is_safe_only_if_it_decides
          own e hstart
      · exact ih h e he

theorem stating_an_invocation_never_improves_a_verdict (own : V) (ds : List Decision)
    (d : Decision) (p : Premise) (h : rowVerdict own (ds ++ [d]) p = .safe) :
    rowVerdict own ds p = .safe := by
  simp only [rowVerdict, List.foldl_append, List.foldl_cons, List.foldl_nil] at h ⊢
  generalize List.foldl (fun acc d => join acc (invocationVerdict d)) own ds = acc at h ⊢
  cases acc <;> cases d <;> simp_all [join, rank, invocationVerdict]

end QCore.OpensslUnstatedSlots

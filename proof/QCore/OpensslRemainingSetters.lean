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

import QCore.OpensslHandshakeSlots

/-!
# The remaining OpenSSL handshake setters (card OPENSSL-REMAINING-HANDSHAKE-SETTERS)

Gateway rule 12n and contract specification C04.19.3 (the opaque slot,
`QCore.OpaqueCallbackSlots`), C04.18 (named premises). Abstract rule
theorems over the checker `providers/quirin-provider-openssl/src/check.rs`
(`slot_writers_declared`) and the crate's declarations, not an extraction
of them.

* **A setter no page documents leaves its slot unstated.** The checker
  accepts a slot only over a fetched page's NAME list (`pageNamed`); a slot
  whose writer has no page (`SSL_CTX_set_not_resumable_session_callback`,
  `SSL_set_session_ticket_ext_cb`) is never accepted
  (`an_undocumented_slot_is_never_accepted`), so a declaration that may run
  it is covered only by the unstated-slots premise
  (`an_undocumented_slot_needs_the_premise`).
* **A macro alias adds no candidate.** A run's candidates for a slot are
  what its writers are handed, read after preprocessing, where a call
  through an alias (`X509_STORE_set_verify_cb_func`) is a call to the
  writer it expands to (`resolve`). So the set over the calls as written
  with aliases resolved is the set over the expanded calls
  (`an_alias_call_is_its_writers_call`), and listing the alias as a
  not-writer loses no value (`an_alias_call_and_a_writer_call_agree`).
* **A premise's statement drops a kind only when its slots are stated.** The
  statement names the kinds it still covers (`honest`); a declaration's
  statement stays honest after dropping a kind only where every runnable of
  that kind is stated (`dropping_a_kind_needs_its_slots_stated`), and stating
  slots never makes an honest statement dishonest
  (`stating_slots_keeps_a_statement_honest`).
* **Stating more slots never improves a verdict**
  (`stating_the_remaining_slots_never_improves_a_verdict`, by
  `QCore.OpensslHandshakeSlots`).

## What this is not

That the pages' NAME sections are the library's complete writer lists, that
the pinned headers' aliases are the macros the pages say, that the handshake
and I/O calls run every slot their declarations state, and that the Rust
checker computes these sets are premises, pinned by the crate's checker tests
(`a_drifted_remaining_setter_reading_is_refused`) and
`crates/qcore-c-gateway/tests/openssl_undeclared_handshake_callbacks.rs`.
-/

namespace QCore.OpensslRemainingSetters

open QCore.CallbackContracts (Decision)
open QCore.OpensslUnstatedSlots (Runnable Premise covered wellCovered rowVerdict)
open QCore.VerdictSemigroup

/-! ## A setter no page documents leaves its slot unstated -/

/-- A slot's page as the record holds it: the NAME list of a fetched
snapshot, or nothing when no page was found. -/
abbrev PageRecord := Option (List String)

/-- `slot_writers_declared`'s page half: a fetched page names every listed
writer. -/
def pageNamed (page : PageRecord) (writers : List String) : Bool :=
  match page with
  | none => false
  | some named => writers.all named.contains

theorem an_undocumented_slot_is_never_accepted (writers : List String) :
    pageNamed none writers = false := rfl

theorem a_page_omitting_a_writer_refuses {named writers : List String} {w : String}
    (hw : w ∈ writers) (hn : w ∉ named) : pageNamed (some named) writers = false := by
  simp only [pageNamed, List.all_eq_false]
  exact ⟨w, hw, by simpa [List.contains_iff_mem] using hn⟩

/-- A slot is stated only when its page is accepted; a declaration that may
run an unaccepted slot therefore names the unstated-slots premise. -/
theorem an_undocumented_slot_needs_the_premise {runs : List Runnable} {stated : List String}
    {premise : Premise} (h : wellCovered runs stated premise = true) {key : String}
    (hk : Runnable.slot key ∈ runs) {writers : List String}
    (hstated : key ∈ stated → pageNamed none writers = true) :
    premise = .unstatedSlots := by
  have hns : key ∉ stated := fun hin => by
    have := hstated hin
    simp [pageNamed] at this
  exact QCore.OpensslHandshakeSlots.an_unstated_slot_needs_the_premise h hk hns

/-! ## A macro alias adds no candidate -/

/-- A setter call as written: its function name and the function pointer it
hands. -/
structure Call where
  function : String
  handed : String
  deriving DecidableEq, Repr

/-- Preprocessing: an alias resolves to the writer it expands to, any other
name to itself. -/
def resolve (aliases : List (String × String)) (f : String) : String :=
  match aliases.find? (fun a => a.1 == f) with
  | some (_, w) => w
  | none => f

/-- The run's candidates for a slot: what the calls to its writers hand,
read after preprocessing. -/
def candidates (aliases : List (String × String)) (writers : List String) (calls : List Call) :
    List String :=
  (calls.filter fun c => writers.contains (resolve aliases c.function)).map Call.handed

theorem an_alias_call_is_its_writers_call {aliases : List (String × String)} {a w : String}
    (h : aliases.find? (fun p => p.1 == a) = some (a, w)) : resolve aliases a = w := by
  simp [resolve, h]

/-- Where the writer itself is no alias (the pinned header declares it as a
function), an alias call and a writer call give the same candidates. -/
theorem an_alias_call_and_a_writer_call_agree (aliases : List (String × String))
    (writers : List String) (calls : List Call) {a w h : String}
    (ha : aliases.find? (fun p => p.1 == a) = some (a, w))
    (hw : aliases.find? (fun p => p.1 == w) = none) :
    candidates aliases writers (calls ++ [⟨a, h⟩]) =
      candidates aliases writers (calls ++ [⟨w, h⟩]) := by
  have hr : resolve aliases a = w := an_alias_call_is_its_writers_call ha
  have hw' : resolve aliases w = w := by simp [resolve, hw]
  simp only [candidates, List.filter_append, List.map_append, List.filter_cons,
    List.filter_nil, hr, hw']
  split <;> rfl

/-! ## A premise's statement drops a kind only when its slots are stated -/

/-- A kind a premise statement may name (the ctrl arms, CT, the undocumented
setters' slots, the lookup methods, ex_data, ...). -/
abbrev Kind := String

/-- A statement is honest for a declaration when every runnable the
declaration does not state belongs to a kind the statement names. -/
def honest (kind : Runnable → Kind) (runs : List Runnable) (stated : List String)
    (named : List Kind) : Bool :=
  runs.all fun r => covered stated .none r || named.contains (kind r)

theorem dropping_a_kind_needs_its_slots_stated {kind : Runnable → Kind}
    {runs : List Runnable} {stated : List String} {named : List Kind} {k : Kind}
    (h : honest kind runs stated (named.filter (· ≠ k)) = true) {key : String}
    (hk : Runnable.slot key ∈ runs) (hkind : kind (.slot key) = k) : key ∈ stated := by
  simp only [honest, List.all_eq_true] at h
  have := h _ hk
  simp [covered, hkind] at this
  simpa [List.contains_iff_mem] using this

theorem stating_slots_keeps_a_statement_honest {kind : Runnable → Kind}
    {runs : List Runnable} {stated more : List String} {named : List Kind}
    (h : honest kind runs stated named = true) :
    honest kind runs (stated ++ more) named = true := by
  simp only [honest, List.all_eq_true] at h ⊢
  intro r hr
  have := h r hr
  cases r with
  | slot key =>
      simp only [covered, Bool.or_eq_true, List.contains_iff_mem, List.mem_append]
        at this ⊢
      rcases this with (hs | hp) | hn
      · exact Or.inl (Or.inl (Or.inl hs))
      · exact Or.inl (Or.inr hp)
      · exact Or.inr hn
  | exData => simpa [covered] using this

/-! ## Stating more slots never improves a verdict -/

theorem stating_the_remaining_slots_never_improves_a_verdict (own : V) (ds more : List Decision)
    (p : Premise) (h : rowVerdict own (ds ++ more) p = .safe) : rowVerdict own ds p = .safe :=
  QCore.OpensslHandshakeSlots.stating_more_invocations_never_improves_a_verdict own ds more p h

theorem a_row_is_safe_only_if_every_remaining_invocation_decides {own : V}
    {ds more : List Decision} {p : Premise} (h : rowVerdict own (ds ++ more) p = .safe) :
    ∀ d ∈ more, d = .identity :=
  QCore.OpensslHandshakeSlots.a_row_is_safe_only_if_every_new_invocation_decides h

end QCore.OpensslRemainingSetters

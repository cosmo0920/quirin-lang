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

import QCore.OpensslFreeInvocations
import QCore.OpensslRemainingSetters

/-!
# Callbacks the OpenSSL frees run on objects they free in turn (card OPENSSL-FREE-INDIRECT-CALLBACKS)

Gateway rule 12n and contract specification C04.19.3 (the opaque slot beside
a reference drop), C04.18 (named premises). Abstract rule theorems over the
checker `providers/quirin-provider-openssl/src/check.rs` and the crate's
declarations of `X509_STORE_free`, `SSL_CTX_free` and `SSL_free`, not an
extraction of them.

* **The frees keep the unstated-slots premise while a remainder is
  unstated.** A free that may run a callback no clause states (a key's
  method finish function, a provider's free function) is covered only by
  the premise (`a_free_with_a_remainder_keeps_the_premise`); the ex_data
  premise alone would not cover it
  (`the_ex_data_premise_alone_leaves_the_remainder_uncovered`). The concrete
  free vector is covered with the premise and not without it
  (`store_free_vector_needs_the_premise`).
* **A statement narrows only by kinds the call never runs, or whose slots
  are stated.** Dropping a kind no runnable of the call belongs to keeps the
  statement honest (`dropping_a_kind_the_call_never_runs_keeps_it_honest`):
  the ctrl arms, CT and the undocumented handshake setters run during a
  handshake, not at a free.
* **A slot the call does not run needs no statement.** A store's twelve
  slots are run by a verification context's cleanup, not by the store's
  free; leaving them unstated keeps an honest statement honest
  (`a_slot_the_call_does_not_run_needs_no_statement`).
* **Stating the lookup slots never improves the free's outcome**
  (`stating_the_lookup_slots_never_improves`, by
  `QCore.OpensslFreeInvocations`), and the frees' vectors: closed empty sets
  apply, an uncontracted or effectful lookup candidate refuses, a key no
  declaration registers refuses.

## What this is not

That OpenSSL 3.5.5's `X509_STORE_free` runs exactly the lookup methods'
shutdown and free functions, that the store's cleanup function runs only at
`X509_STORE_CTX_cleanup`, that the survey's remainder is complete (a QUIC
object's free was not traced, which the premise names), and that the Rust
checker computes these sets are premises, pinned by the crate's checker test
`a_drifted_free_indirect_reading_is_refused` and
`crates/qcore-c-gateway/tests/openssl_undeclared_handshake_callbacks.rs`.
-/

namespace QCore.OpensslFreeIndirectCallbacks

open QCore.OpensslUnstatedSlots (Runnable Premise covered wellCovered)
open QCore.OpensslRemainingSetters (Kind honest)
open QCore.OpensslFreeInvocations (freeOutcome held Effect)
open QCore.OpensslBioReaders (SlotFacts)

/-! ## The frees keep the premise while a remainder is unstated -/

theorem a_free_with_a_remainder_keeps_the_premise {runs : List Runnable}
    {stated : List String} {premise : Premise} (h : wellCovered runs stated premise = true)
    {key : String} (hk : Runnable.slot key ∈ runs) (hs : key ∉ stated) :
    premise = .unstatedSlots :=
  QCore.OpensslHandshakeSlots.an_unstated_slot_needs_the_premise h hk hs

theorem the_ex_data_premise_alone_leaves_the_remainder_uncovered {runs : List Runnable}
    {stated : List String} {key : String} (hk : Runnable.slot key ∈ runs) (hs : key ∉ stated) :
    wellCovered runs stated .exData = false := by
  apply Bool.eq_false_iff.mpr
  intro h
  have := a_free_with_a_remainder_keeps_the_premise h hk hs
  cases this

/-! ## A statement narrows only by kinds the call never runs -/

theorem dropping_a_kind_the_call_never_runs_keeps_it_honest {kind : Runnable → Kind}
    {runs : List Runnable} {stated : List String} {named : List Kind} {k : Kind}
    (h : honest kind runs stated named = true) (hk : ∀ r ∈ runs, kind r ≠ k) :
    honest kind runs stated (named.filter (· ≠ k)) = true := by
  simp only [honest, List.all_eq_true] at h ⊢
  intro r hr
  have := h r hr
  simp only [Bool.or_eq_true, List.contains_iff_mem] at this ⊢
  rcases this with hc | hn
  · exact Or.inl hc
  · exact Or.inr (List.mem_filter.mpr ⟨hn, by simpa using hk r hr⟩)

/-! ## A slot the call does not run needs no statement -/

theorem a_slot_the_call_does_not_run_needs_no_statement {kind : Runnable → Kind}
    {runs : List Runnable} {stated : List String} {named : List Kind} {key : String}
    (h : honest kind runs stated named = true) (hk : Runnable.slot key ∉ runs) :
    honest kind runs (stated.filter (· ≠ key)) named = true := by
  simp only [honest, List.all_eq_true] at h ⊢
  intro r hr
  have := h r hr
  cases r with
  | slot k =>
      have hne : k ≠ key := fun heq => hk (heq ▸ hr)
      simp only [covered, Bool.or_eq_true, List.contains_iff_mem] at this ⊢
      rcases this with (hs | hp) | hn
      · exact Or.inl (Or.inl (List.mem_filter.mpr ⟨hs, by simpa using hne⟩))
      · exact Or.inl (Or.inr hp)
      · exact Or.inr hn
  | exData => simpa [covered] using this

/-! ## Stating the lookup slots never improves the outcome -/

theorem stating_the_lookup_slots_never_improves {last : Bool} {slots more : List SlotFacts}
    {effects : List Effect} (h : freeOutcome held last (slots ++ more) effects = .applied) :
    freeOutcome held last slots effects = .applied :=
  QCore.OpensslFreeInvocations.stating_more_slots_never_improves h

/-! ## The frees' vectors -/

/-- What `X509_STORE_free` may run: the lookup method's shutdown and free
functions (stated), a key's method finish function (the remainder) and
ex_data. -/
def storeFreeRuns : List Runnable :=
  [.slot "x509_lookup_method_st.lookup_meth_shutdown",
   .slot "x509_lookup_method_st.lookup_meth_free",
   .slot "rsa_meth_st.finish", .exData]

def storeFreeStated : List String :=
  ["x509_lookup_method_st.lookup_meth_shutdown", "x509_lookup_method_st.lookup_meth_free"]

theorem store_free_vector_needs_the_premise :
    wellCovered storeFreeRuns storeFreeStated .unstatedSlots = true ∧
      wellCovered storeFreeRuns storeFreeStated .exData = false ∧
      wellCovered storeFreeRuns storeFreeStated .none = false := by
  decide

/-- `SSL_free`: the session-remove slot, three BIO slots and the two lookup
slots, each closed and empty (the measured run), every actual `_`. -/
def sslFreeSlots : List SlotFacts := List.replicate 6 (.closed [])

theorem ssl_free_empty_slots_apply (last : Bool) :
    freeOutcome held last sslFreeSlots [] = .applied := by
  cases last <;> decide

theorem an_identity_lookup_candidate_applies :
    freeOutcome held true (.closed [.identity] :: sslFreeSlots) [] = .applied := by decide

theorem an_uncontracted_lookup_candidate_refuses :
    freeOutcome held true (.closed [.uncontracted] :: sslFreeSlots) [] = .refused := by decide

theorem an_effectful_lookup_candidate_refuses :
    freeOutcome held false (.closed [.effectful] :: sslFreeSlots) [] = .refused := by decide

theorem an_unregistered_lookup_key_refuses :
    freeOutcome held true (.unregistered :: sslFreeSlots) [] = .refused := by decide

end QCore.OpensslFreeIndirectCallbacks

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

import QCore.OpensslUnstatedSlots
import QCore.PremiseFormula

/-!
# The free audit (card OPENSSL-OBJECT-FREES-SILENT-METHOD-CALLBACKS)

Contract specification C04.18 (named premises, the formula of C04.18.9),
gateway rule 12n. A free of a key, certificate, PKCS#12 object, engine or
digest context may run a function of a method, provider or engine of an
object it frees (a key's key management free function, its `RSA_METHOD`
finish function, an ENGINE's destroy function) that no clause states. Such a
free names the unstated-slots premise in the statement of a free, the
formula `unstatedFreeFormula` (`QCore.PremiseFormula`), which quantifies over
the call's own `#freed` and so names exactly what each free runs. Abstract
rule theorems over the checker `providers/quirin-provider-openssl/src/check.rs`
(`free_audited`, `releases`) and its audit table `FREE_AUDIT`, not an
extraction of them.

* **The audit is total over releases.** An accepted entry whose shape
  releases an object is listed (`a_release_is_audited`); one left out
  refuses (`an_unaudited_release_refuses`).
* **A running free names the free statement.** An accepted entry the audit
  reads as running names it, and not the handshake's statement
  (`a_running_free_names_the_statement`), so dropping it
  (`dropping_the_statement_refuses`) or adding the handshake's statement
  (`the_handshake_statement_beside_it_refuses`) refuses.
* **The free statement stands only where the audit reads a run**
  (`the_statement_is_named_only_where_audited`); a free whose run another
  premise names carries that premise (`a_named_by_free_carries_the_premise`).
* **The statement is well formed** (`the_free_statement_is_well_formed`,
  from `QCore.PremiseFormula.openssl_formulas_wf`), and **the ex_data
  premise alone leaves a key's method uncovered**
  (`key_free_vector_needs_the_premise`, over
  `QCore.OpensslUnstatedSlots.wellCovered`).
* **The premise moves no verdict** (`the_free_premise_moves_no_verdict`,
  from `QCore.OpensslUnstatedSlots.the_verdict_ignores_the_premise`).

## What this is not

That each free runs what its audit reading says (read from the OpenSSL
3.5.5 sources), that those functions are members of `#teardown` of an object
in the call's `#freed`, and that the Rust checker computes `audited`, are
premises, pinned by the crate's checker tests
(`every_free_names_what_it_runs_on_the_objects_it_frees`,
`a_drifted_free_reading_is_refused`). Stating these functions as slots is
follow-up OPENSSL-FREE-REMAINING-INDIRECT-CALLBACKS.
-/

namespace QCore.OpensslObjectFreesAudit

open QCore.CallbackContracts (Decision)
open QCore.VerdictSemigroup
open QCore.OpensslUnstatedSlots (Runnable wellCovered)

/-- A free audit reading (`Frees`). -/
inductive Reading where
  | runs
  | namedBy (premise : String)
  | nothing
  deriving DecidableEq, Repr

/-- A documented entry, as far as the free audit reads it: whether its shape
releases an object, whether it names the free statement, its other named
premises, and whether it names the handshake's statement. -/
structure Entry where
  function : String
  releases : Bool
  named : Bool
  premises : List String
  handshake : Bool
  deriving DecidableEq, Repr

/-- The audit's readings of `f`. -/
def listed (f : String) (audit : List (String × Reading)) : List Reading :=
  (audit.filter fun p => p.1 == f).map Prod.snd

/-- `free_audited` over the readings found. -/
def decideReadings (e : Entry) : List Reading → Bool
  | [] => !e.named && !e.releases
  | [.runs] => e.named && !e.handshake
  | [.namedBy p] => !e.named && e.premises.contains p
  | [.nothing] => !e.named
  | _ :: _ :: _ => false

/-- The checker's acceptance of one entry against the audit. -/
def audited (e : Entry) (audit : List (String × Reading)) : Bool :=
  decideReadings e (listed e.function audit)

theorem mem_listed {f : String} {r : Reading} {audit : List (String × Reading)}
    (h : (f, r) ∈ audit) : r ∈ listed f audit := by
  simp only [listed, List.mem_map, List.mem_filter]
  exact ⟨(f, r), ⟨h, by simp⟩, rfl⟩

theorem listed_mem {f : String} {r : Reading} {audit : List (String × Reading)}
    (h : r ∈ listed f audit) : (f, r) ∈ audit := by
  simp only [listed, List.mem_map, List.mem_filter, beq_iff_eq] at h
  obtain ⟨⟨g, s⟩, ⟨hm, hg⟩, hs⟩ := h
  simp only at hg hs
  subst hg hs
  exact hm

/-- The single reading an accepted entry is decided over, when one is
listed. -/
theorem single_reading {e : Entry} {audit : List (String × Reading)} {r : Reading}
    (h : audited e audit = true) (hr : (e.function, r) ∈ audit) :
    listed e.function audit = [r] := by
  have hm := mem_listed hr
  unfold audited at h
  generalize listed e.function audit = l at h hm
  match l, hm, h with
  | [x], hm, _ =>
      simp only [List.mem_singleton] at hm
      rw [hm]
  | _ :: _ :: _, _, h => simp [decideReadings] at h

/-! ## The audit is total over releases -/

theorem a_release_is_audited {e : Entry} {audit : List (String × Reading)}
    (h : audited e audit = true) (hs : e.releases = true) :
    ∃ r, (e.function, r) ∈ audit := by
  unfold audited at h
  cases hl : listed e.function audit with
  | nil => rw [hl] at h; simp [decideReadings, hs] at h
  | cons r rest => exact ⟨r, listed_mem (by rw [hl]; exact List.mem_cons_self ..)⟩

theorem an_unaudited_release_refuses {e : Entry} {audit : List (String × Reading)}
    (hs : e.releases = true) (hn : ∀ r, (e.function, r) ∉ audit) :
    audited e audit = false := by
  cases h : audited e audit with
  | false => rfl
  | true =>
      obtain ⟨r, hr⟩ := a_release_is_audited h hs
      exact absurd hr (hn r)

/-! ## A running free names the free statement -/

theorem a_running_free_names_the_statement {e : Entry} {audit : List (String × Reading)}
    (h : audited e audit = true) (hr : (e.function, Reading.runs) ∈ audit) :
    e.named = true ∧ e.handshake = false := by
  have hl := single_reading h hr
  unfold audited at h
  rw [hl] at h
  simp only [decideReadings, Bool.and_eq_true, Bool.not_eq_true'] at h
  exact h

theorem dropping_the_statement_refuses {e : Entry} {audit : List (String × Reading)}
    (hr : (e.function, Reading.runs) ∈ audit) (hn : e.named = false) :
    audited e audit = false := by
  cases h : audited e audit with
  | false => rfl
  | true =>
      have := (a_running_free_names_the_statement h hr).1
      rw [hn] at this
      cases this

theorem the_handshake_statement_beside_it_refuses {e : Entry}
    {audit : List (String × Reading)} (hr : (e.function, Reading.runs) ∈ audit)
    (hh : e.handshake = true) : audited e audit = false := by
  cases h : audited e audit with
  | false => rfl
  | true =>
      have := (a_running_free_names_the_statement h hr).2
      rw [hh] at this
      cases this

/-! ## The free statement stands only where the audit reads a run -/

theorem the_statement_is_named_only_where_audited {e : Entry}
    {audit : List (String × Reading)} (h : audited e audit = true) (hp : e.named = true) :
    (e.function, Reading.runs) ∈ audit := by
  unfold audited at h
  cases hl : listed e.function audit with
  | nil => rw [hl] at h; simp [decideReadings, hp] at h
  | cons r rest =>
      rw [hl] at h
      cases rest with
      | cons _ _ => simp [decideReadings] at h
      | nil =>
          have hmem : (e.function, r) ∈ audit :=
            listed_mem (by rw [hl]; exact List.mem_singleton_self _)
          cases r with
          | runs => exact hmem
          | namedBy p => simp [decideReadings, hp] at h
          | nothing => simp [decideReadings, hp] at h

theorem a_named_by_free_carries_the_premise {e : Entry} {audit : List (String × Reading)}
    {p : String} (h : audited e audit = true) (hr : (e.function, Reading.namedBy p) ∈ audit) :
    p ∈ e.premises ∧ e.named = false := by
  have hl := single_reading h hr
  unfold audited at h
  rw [hl] at h
  simp only [decideReadings, Bool.and_eq_true, Bool.not_eq_true',
    List.contains_iff_mem] at h
  exact ⟨h.2, h.1⟩

/-! ## The statement, and why the ex_data premise alone does not do -/

theorem the_free_statement_is_well_formed :
    QCore.PremiseFormula.wf QCore.PremiseFormula.unstatedFreeFormula = true :=
  QCore.PremiseFormula.openssl_formulas_wf.2.2.1

/-- What `EVP_PKEY_free` may run: a legacy RSA key's method finish function
(no slot stated) and the ex_data free callbacks. -/
def keyFreeRuns : List Runnable := [.slot "rsa_meth_st.finish", .exData]

theorem key_free_vector_needs_the_premise :
    wellCovered keyFreeRuns [] .unstatedSlots = true ∧
      wellCovered keyFreeRuns [] .exData = false ∧
      wellCovered keyFreeRuns [] .none = false := by
  decide

/-! ## The premise moves no verdict -/

theorem the_free_premise_moves_no_verdict (own : V) (ds : List Decision) :
    QCore.OpensslUnstatedSlots.rowVerdict own ds .unstatedSlots =
      QCore.OpensslUnstatedSlots.rowVerdict own ds .exData :=
  QCore.OpensslUnstatedSlots.the_verdict_ignores_the_premise own ds _ _

/-! ## Vectors from the crate's audit -/

/-- A slice of `FREE_AUDIT`. -/
def crateAudit : List (String × Reading) :=
  [("EVP_PKEY_free", .runs), ("X509_free", .runs), ("PKCS12_free", .runs),
    ("ENGINE_free", .runs), ("EVP_MD_CTX_free", .runs), ("SSL_free", .runs),
    ("BIO_free", .namedBy "openssl_builtin_bio_methods"), ("OPENSSL_sk_free", .nothing)]

theorem vector_key_free_as_landed :
    audited ⟨"EVP_PKEY_free", true, true, ["openssl_ex_data_callbacks"], false⟩ crateAudit =
      true := by
  decide

theorem vector_key_free_as_before_refuses :
    audited ⟨"EVP_PKEY_free", true, false, ["openssl_ex_data_callbacks"], false⟩ crateAudit =
      false := by
  decide

theorem vector_engine_free_with_the_handshake_statement_refuses :
    audited ⟨"ENGINE_free", true, true, [], true⟩ crateAudit = false := by
  decide

theorem vector_bio_free_as_landed :
    audited ⟨"BIO_free", true, false, ["openssl_builtin_bio_methods"], false⟩ crateAudit =
      true := by
  decide

theorem vector_stack_free_with_the_statement_refuses :
    audited ⟨"OPENSSL_sk_free", true, true, [], false⟩ crateAudit = false := by
  decide

theorem vector_unaudited_free_refuses :
    audited ⟨"X509_CRL_free", true, false, [], false⟩ crateAudit = false := by
  decide

theorem vector_unaudited_plain_call :
    audited ⟨"SSL_get_error", false, false, [], false⟩ crateAudit = true := by
  decide

end QCore.OpensslObjectFreesAudit

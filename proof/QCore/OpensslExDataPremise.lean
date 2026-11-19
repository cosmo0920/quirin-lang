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

/-!
# The ex_data premise on every constructor and free (card OPENSSL-EX-DATA-REGISTRY)

Contract specification C04.18 (named premises). `CRYPTO_get_ex_new_index`
(3.5) runs a class's new callbacks "when a structure is initially
allocated" and its free callbacks "when a structure is free'd", for every
index registered for the class: a global registry per class, which no
clause of the vocabulary states. Abstract rule theorems over the checker
`providers/quirin-provider-openssl/src/check.rs` (`ex_data_audited`,
`creates_or_frees`) and its audit table `EX_DATA_AUDIT`, not an extraction
of them.

* **The audit is total over constructors and frees.** An accepted entry
  whose shape creates or frees an object is listed in the audit
  (`a_constructor_or_free_is_audited`); one left out refuses
  (`an_unaudited_constructor_or_free_refuses`).
* **The premise is named exactly where the audit reads a run.** An accepted
  entry the audit reads as running ex_data callbacks names the premise
  (`a_running_entry_names_the_premise`), so dropping it refuses
  (`dropping_the_premise_refuses`); an accepted entry naming it is read as
  running (`the_premise_is_named_only_where_audited`), over classes that are
  all ex_data classes (`a_run_names_only_ex_data_classes`).
* **The premise moves no verdict.** It is metadata (C04.18): a row's verdict
  is the same whichever premise it names
  (`the_ex_data_premise_moves_no_verdict`, from
  `QCore.OpensslUnstatedSlots.the_verdict_ignores_the_premise`). So the
  card changes what a row is published as resting on, never its verdict.

## What this is not

That the audit's reading of each function (which classes it creates or
frees, read from the 3.5 pages and sources) is the library's behaviour, and
that the Rust checker computes `audited`, are premises, pinned by the
crate's checker tests (`every_ex_data_class_object_names_the_premise`,
`a_drifted_ex_data_reading_is_refused`). The registry itself (a run-wide set
of callbacks per class) is not read: follow-up
OPENSSL-EX-DATA-CLASS-REGISTRY.
-/

namespace QCore.OpensslExDataPremise

open QCore.CallbackContracts (Decision)
open QCore.VerdictSemigroup

/-- The ex_data classes `CRYPTO_get_ex_new_index` (3.5) lists. -/
def exDataClasses : List String :=
  ["BIO", "DH", "DSA", "EC_KEY", "ENGINE", "EVP_PKEY", "RSA", "SSL", "SSL_CTX",
    "SSL_SESSION", "UI", "UI_METHOD", "X509", "X509_STORE", "X509_STORE_CTX"]

/-- A documented shape, as far as the audit reads it. -/
inductive Shape where
  | plain
  | constructor
  | release
  | other
  deriving DecidableEq, Repr

/-- `creates_or_frees`: a constructor or a release (a free or a reference
drop). -/
def Shape.createsOrFrees : Shape → Bool
  | .constructor => true
  | .release => true
  | _ => false

/-- An audit reading: the classes whose callbacks the call may run, or no
ex_data class. -/
inductive Reading where
  | runs (classes : List String)
  | noClass
  deriving DecidableEq, Repr

/-- A documented entry: its function, shape and whether it names the
premise. -/
structure Entry where
  function : String
  shape : Shape
  premised : Bool
  deriving DecidableEq, Repr

/-- The audit's readings of `f`. -/
def listed (f : String) (audit : List (String × Reading)) : List Reading :=
  (audit.filter fun p => p.1 == f).map Prod.snd

/-- `ex_data_audited` over the readings found. -/
def decideReadings (premised createsOrFrees : Bool) : List Reading → Bool
  | [] => !premised && !createsOrFrees
  | [.runs cs] => premised && !cs.isEmpty && cs.all fun c => exDataClasses.contains c
  | [.noClass] => !premised
  | _ :: _ :: _ => false

/-- The checker's acceptance of one entry against the audit. -/
def audited (e : Entry) (audit : List (String × Reading)) : Bool :=
  decideReadings e.premised e.shape.createsOrFrees (listed e.function audit)

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

/-! ## The audit is total over constructors and frees -/

theorem a_constructor_or_free_is_audited {e : Entry} {audit : List (String × Reading)}
    (h : audited e audit = true) (hs : e.shape.createsOrFrees = true) :
    ∃ r, (e.function, r) ∈ audit := by
  unfold audited at h
  cases hl : listed e.function audit with
  | nil => rw [hl] at h; simp [decideReadings, hs] at h
  | cons r rest => exact ⟨r, listed_mem (by rw [hl]; exact List.mem_cons_self ..)⟩

theorem an_unaudited_constructor_or_free_refuses {e : Entry}
    {audit : List (String × Reading)} (hs : e.shape.createsOrFrees = true)
    (hn : ∀ r, (e.function, r) ∉ audit) : audited e audit = false := by
  cases h : audited e audit with
  | false => rfl
  | true =>
      obtain ⟨r, hr⟩ := a_constructor_or_free_is_audited h hs
      exact absurd hr (hn r)

/-! ## The premise is named exactly where the audit reads a run -/

theorem a_running_entry_names_the_premise {e : Entry} {audit : List (String × Reading)}
    {cs : List String} (h : audited e audit = true)
    (hr : (e.function, Reading.runs cs) ∈ audit) : e.premised = true := by
  have hm := mem_listed hr
  unfold audited at h
  generalize listed e.function audit = l at h hm
  match l, hm, h with
  | [x], hm, h =>
      simp only [List.mem_singleton] at hm
      subst hm
      simp only [decideReadings, Bool.and_eq_true] at h
      exact h.1.1
  | _ :: _ :: _, _, h => simp [decideReadings] at h

theorem dropping_the_premise_refuses {f : String} {s : Shape} {cs : List String}
    {audit : List (String × Reading)} (hr : (f, Reading.runs cs) ∈ audit) :
    audited ⟨f, s, false⟩ audit = false := by
  cases h : audited ⟨f, s, false⟩ audit with
  | false => rfl
  | true => simpa using a_running_entry_names_the_premise (e := ⟨f, s, false⟩) h hr

theorem the_premise_is_named_only_where_audited {e : Entry}
    {audit : List (String × Reading)} (h : audited e audit = true)
    (hp : e.premised = true) : ∃ cs, (e.function, Reading.runs cs) ∈ audit := by
  unfold audited at h
  cases hl : listed e.function audit with
  | nil => rw [hl] at h; simp [decideReadings, hp] at h
  | cons r rest =>
      rw [hl] at h
      cases rest with
      | cons _ _ => simp [decideReadings] at h
      | nil =>
          cases r with
          | runs cs => exact ⟨cs, listed_mem (by rw [hl]; exact List.mem_singleton_self _)⟩
          | noClass => simp [decideReadings, hp] at h

theorem a_run_names_only_ex_data_classes {e : Entry} {audit : List (String × Reading)}
    {cs : List String} (h : audited e audit = true)
    (hr : (e.function, Reading.runs cs) ∈ audit) :
    cs ≠ [] ∧ ∀ c ∈ cs, c ∈ exDataClasses := by
  have hm := mem_listed hr
  unfold audited at h
  generalize listed e.function audit = l at h hm
  match l, hm, h with
  | [x], hm, h =>
      simp only [List.mem_singleton] at hm
      subst hm
      simp only [decideReadings, Bool.and_eq_true, Bool.not_eq_true',
        List.isEmpty_eq_false_iff, List.all_eq_true, List.contains_iff_mem] at h
      exact ⟨h.1.2, h.2⟩
  | _ :: _ :: _, _, h => simp [decideReadings] at h

theorem a_second_listing_refuses {e : Entry} {audit : List (String × Reading)}
    {r s : Reading} {rest : List Reading} (hl : listed e.function audit = r :: s :: rest) :
    audited e audit = false := by
  simp [audited, hl, decideReadings]

/-! ## The premise moves no verdict -/

theorem the_ex_data_premise_moves_no_verdict (own : V) (ds : List Decision) :
    QCore.OpensslUnstatedSlots.rowVerdict own ds .exData =
      QCore.OpensslUnstatedSlots.rowVerdict own ds .none :=
  QCore.OpensslUnstatedSlots.the_verdict_ignores_the_premise own ds _ _

/-! ## Vectors from the crate's audit -/

/-- A slice of `EX_DATA_AUDIT`. -/
def crateAudit : List (String × Reading) :=
  [("X509_new", .runs ["X509"]), ("SSL_new", .runs ["SSL", "BIO"]),
    ("SSL_set_accept_state", .runs ["BIO"]), ("EVP_MD_CTX_free", .runs ["EVP_PKEY"]),
    ("SSL_get1_peer_certificate", .noClass), ("OPENSSL_sk_free", .noClass)]

theorem vector_x509_new_premised : audited ⟨"X509_new", .constructor, true⟩ crateAudit = true := by
  decide

theorem vector_x509_new_unpremised :
    audited ⟨"X509_new", .constructor, false⟩ crateAudit = false := by
  decide

theorem vector_accept_state_unpremised :
    audited ⟨"SSL_set_accept_state", .plain, false⟩ crateAudit = false := by
  decide

theorem vector_up_reference_unpremised :
    audited ⟨"SSL_get1_peer_certificate", .constructor, false⟩ crateAudit = true := by
  decide

theorem vector_up_reference_premised :
    audited ⟨"SSL_get1_peer_certificate", .constructor, true⟩ crateAudit = false := by
  decide

theorem vector_unaudited_free_refuses :
    audited ⟨"PKCS12_free", .release, false⟩ crateAudit = false := by
  decide

theorem vector_unaudited_plain_call :
    audited ⟨"SSL_get_error", .plain, false⟩ crateAudit = true := by
  decide

theorem vector_unknown_class_refuses :
    audited ⟨"d2i_PKCS12_bio", .constructor, true⟩ [("d2i_PKCS12_bio", .runs ["PKCS12"])] =
      false := by
  decide

end QCore.OpensslExDataPremise

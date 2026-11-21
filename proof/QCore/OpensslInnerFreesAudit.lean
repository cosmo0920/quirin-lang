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

import QCore.OpensslObjectFreesAudit

/-!
# The free audit over inner frees (card OPENSSL-INNER-FREES-SILENT-METHOD-CALLBACKS)

Contract specification C04.18 (named premises, C04.18.9 formulas), gateway
rule 12n. Calls that are not frees free keys and certificates inside: the
`SSL_CTX_use_*` loaders (a replaced or temporary key or certificate), the PEM
and PKCS#12 readers on a failed read, the two verify-path calls (what the
file lookup read and the store did not keep), and the handshake and I/O
calls with `SSL_shutdown` (peer and ephemeral keys, contexts). The first
three groups name the free statement (`unstatedFreeFormula`); the handshake's
statement (`unstatedFormula`) now conjoins it, since a declaration names
each premise once. Abstract rule theorems over the checker
`providers/quirin-provider-openssl/src/check.rs` (`free_audited`, as revised,
and `inner_frees_audited`) and its tables `FREE_AUDIT`, `EX_DATA_AUDIT` and
`INNER_FREE_CLASSES`, not an extraction of them.

* **The handshake's statement entails the free statement** in every model
  (`the_handshake_statement_entails_the_free_statement`), and is well formed
  (`the_handshake_statement_is_well_formed`).
* **The revised reading.** A run is met by the free statement or the
  handshake's statement, never both (`a_run_names_exactly_one_statement`);
  an entry naming either is listed (`a_named_entry_is_listed`). On an entry
  that does not name the handshake's statement the revision is r45's reading
  (`agrees_with_the_release_audit`), so `QCore.OpensslObjectFreesAudit`'s
  theorems hold for it.
* **The extension.** An entry whose ex_data reading names a key or
  certificate is listed (`a_keyed_entry_is_listed`); one left out refuses,
  whatever its shape (`an_unlisted_keyed_entry_refuses`).
* **The premise moves no verdict** (`the_inner_free_premise_moves_no_verdict`).

## What this is not

That each call frees what its audit reading says (read from the OpenSSL
3.5.5 sources), that the functions it runs are members of `#teardown` of an
object in its `#freed`, and that the Rust checker computes `accepted`, are
premises, pinned by the crate's checker tests
(`every_inner_free_names_what_it_runs`,
`a_drifted_inner_free_reading_is_refused`).
-/

namespace QCore.OpensslInnerFreesAudit

open QCore.CallbackContracts (Decision)
open QCore.VerdictSemigroup
open QCore.PremiseFormula (Model Val eval binVal unstatedFormula unstatedFreeFormula wf)
open QCore.OpensslObjectFreesAudit (Reading listed decideReadings)

/-! ## The handshake's statement conjoins the free statement -/

theorem conj_holds_right {M : Model} (a b : Val M)
    (h : (binVal M .conj a b).holds) : b.holds := by
  cases a <;> cases b <;> simp_all [binVal, Val.holds]

theorem the_handshake_statement_entails_the_free_statement (M : Model)
    (h : (eval M [] unstatedFormula).holds) : (eval M [] unstatedFreeFormula).holds :=
  conj_holds_right _ _ h

theorem the_handshake_statement_is_well_formed : wf unstatedFormula = true :=
  QCore.PremiseFormula.openssl_formulas_wf.2.1

/-! ## The revised reading -/

/-- A documented entry, as the revised audit reads it: whether its shape
releases an object, whether it names the free statement, whether it names the
handshake's statement, its other premises, and whether its ex_data reading
names a key or certificate (`INNER_FREE_CLASSES`). -/
structure Entry where
  function : String
  releases : Bool
  free : Bool
  handshake : Bool
  premises : List String
  keyed : Bool
  deriving DecidableEq, Repr

/-- `free_audited` as revised: `named` is either statement. -/
def decideReadings' (e : Entry) : List Reading → Bool
  | [] => !e.free && !e.handshake && !e.releases
  | [.runs] => (e.free || e.handshake) && !(e.free && e.handshake)
  | [.namedBy p] => !(e.free || e.handshake) && e.premises.contains p
  | [.nothing] => !(e.free || e.handshake)
  | _ :: _ :: _ => false

/-- `inner_frees_audited`: a keyed entry is listed. -/
def innerListed (e : Entry) (audit : List (String × Reading)) : Bool :=
  !e.keyed || !(listed e.function audit).isEmpty

/-- The checker's acceptance of one entry: both audits. -/
def accepted (e : Entry) (audit : List (String × Reading)) : Bool :=
  decideReadings' e (listed e.function audit) && innerListed e audit

/-- r45's entry for one that names no handshake statement. -/
def Entry.release (e : Entry) : QCore.OpensslObjectFreesAudit.Entry :=
  ⟨e.function, e.releases, e.free, e.premises, e.handshake⟩

theorem agrees_with_the_release_audit (e : Entry) (hh : e.handshake = false)
    (l : List Reading) : decideReadings' e l = decideReadings e.release l := by
  match l with
  | [] => simp [decideReadings', decideReadings, Entry.release, hh]
  | [.runs] => simp [decideReadings', decideReadings, Entry.release, hh]
  | [.namedBy p] => simp [decideReadings', decideReadings, Entry.release, hh]
  | [.nothing] => simp [decideReadings', decideReadings, Entry.release, hh]
  | _ :: _ :: _ => simp [decideReadings', decideReadings]

theorem a_run_names_exactly_one_statement {e : Entry} {audit : List (String × Reading)}
    (h : accepted e audit = true) (hl : listed e.function audit = [.runs]) :
    (e.free = true ∧ e.handshake = false) ∨ (e.free = false ∧ e.handshake = true) := by
  unfold accepted at h
  rw [hl] at h
  cases hf : e.free <;> cases hh : e.handshake <;> simp_all [decideReadings']

theorem a_named_entry_is_listed {e : Entry} {audit : List (String × Reading)}
    (h : accepted e audit = true) (hn : e.free = true ∨ e.handshake = true) :
    listed e.function audit ≠ [] := by
  intro hl
  unfold accepted at h
  rw [hl] at h
  rcases hn with hn | hn <;> simp_all [decideReadings']

/-! ## The extension -/

theorem a_keyed_entry_is_listed {e : Entry} {audit : List (String × Reading)}
    (h : accepted e audit = true) (hk : e.keyed = true) :
    listed e.function audit ≠ [] := by
  intro hl
  unfold accepted innerListed at h
  rw [hl] at h
  simp [hk] at h

theorem an_unlisted_keyed_entry_refuses {e : Entry} {audit : List (String × Reading)}
    (hk : e.keyed = true) (hl : listed e.function audit = []) :
    accepted e audit = false := by
  cases h : accepted e audit with
  | false => rfl
  | true => exact absurd hl (a_keyed_entry_is_listed h hk)

/-! ## The premise moves no verdict -/

theorem the_inner_free_premise_moves_no_verdict (own : V) (ds : List Decision) :
    QCore.OpensslUnstatedSlots.rowVerdict own ds .unstatedSlots =
      QCore.OpensslUnstatedSlots.rowVerdict own ds .exData :=
  QCore.OpensslUnstatedSlots.the_verdict_ignores_the_premise own ds _ _

/-! ## Vectors from the crate's audit -/

/-- A slice of `FREE_AUDIT` with this card's entries. -/
def crateAudit : List (String × Reading) :=
  [("EVP_PKEY_free", .runs), ("SSL_CTX_use_PrivateKey", .runs),
    ("PEM_read_bio_X509", .runs), ("d2i_PKCS12_bio", .runs), ("SSL_read", .runs),
    ("SSL_shutdown", .runs), ("X509_new", .nothing),
    ("BIO_free", .namedBy "openssl_builtin_bio_methods")]

theorem vector_loader_as_landed :
    accepted ⟨"SSL_CTX_use_PrivateKey", false, true, false, [], true⟩ crateAudit = true := by
  decide

theorem vector_loader_as_before_refuses :
    accepted ⟨"SSL_CTX_use_PrivateKey", false, false, false, [], true⟩ crateAudit = false := by
  decide

theorem vector_loader_unlisted_refuses :
    accepted ⟨"SSL_CTX_use_PrivateKey", false, true, false, [], true⟩ [] = false := by
  decide

theorem vector_handshake_as_landed :
    accepted ⟨"SSL_read", false, false, true, [], true⟩ crateAudit = true := by
  decide

theorem vector_handshake_with_both_statements_refuses :
    accepted ⟨"SSL_read", false, true, true, [], true⟩ crateAudit = false := by
  decide

theorem vector_handshake_unlisted_refuses :
    accepted ⟨"SSL_read", false, false, true, [], true⟩ [] = false := by
  decide

theorem vector_constructor_frees_nothing :
    accepted ⟨"X509_new", false, false, false, [], true⟩ crateAudit = true := by
  decide

theorem vector_constructor_with_the_statement_refuses :
    accepted ⟨"X509_new", false, true, false, [], true⟩ crateAudit = false := by
  decide

theorem vector_unkeyed_plain_call :
    accepted ⟨"SSL_get_error", false, false, false, [], false⟩ crateAudit = true := by
  decide

theorem vector_key_free_unchanged :
    accepted ⟨"EVP_PKEY_free", true, true, false, [], true⟩ crateAudit = true := by
  decide

end QCore.OpensslInnerFreesAudit

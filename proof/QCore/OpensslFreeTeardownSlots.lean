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
# The teardown slots of the frees (card OPENSSL-FREE-REMAINING-INDIRECT-CALLBACKS)

Contract specification C04.18 (named premises; the formula of C04.18.9 gains
`#library` and `#loaded`), C04.19.3 (opaque slots), gateway rule 12n. The
OpenSSL frees run, on the objects they free in turn, the functions of method
tables, engines and providers: a key's ASN.1 method free and RSA, DSA, DH or
EC_KEY method finish, an engine's finish, destroy and method enumerators, the
legacy digest, cipher and public key method cleanups, an async wait
context's fd cleanups. This card declares their setters, states the slots at
the frees, and names the rest with two premises: the library's own teardown
functions (`builtinTeardownFormula`) and those of the modules it loads
(`loadedTeardownFormula`, user decision 2026-11-21). A free drops the
unstated-slots premise only when its audit leaves nothing. Abstract rule
theorems over the checker `providers/quirin-provider-openssl/src/check.rs`
(`free_audited`, `source_cited`) and its tables `FREE_AUDIT`,
`TEARDOWN_SLOTS`, `SOURCE_CITATIONS`, not an extraction of them.

* **Coverage.** When every teardown function of every freed object is
  stated, the library's or a loaded module's, the two teardown premises
  entail the free statement (`teardown_premises_cover_the_free_statement`):
  dropping the unstated-slots premise from such a free assumes nothing more.
* **The released statement is the stronger one.** The statement of a free
  through a released argument entails the free statement
  (`released_statement_entails_free_statement`), so moving `PKCS12_free` and
  `EVP_MD_CTX_free` to it narrows no premise.
* **The audit.** A free keeps the premise exactly when its audit leaves items
  (`a_left_item_keeps_the_premise`, `nothing_left_drops_the_premise`); a
  running free names both teardown premises
  (`a_running_free_names_the_teardown_premises`) and states exactly its
  audited teardown slots (`the_stated_slots_are_the_audited`); a free through
  a released argument names the released statement alone and states no slot
  (`a_released_free_states_no_slot`).
* **Source-derived entries** cite their own file and admit only 3.5.5
  (`a_source_entry_admits_only_the_release_read`).
* **Vectors** from the crate's audit.

## What this is not

That the frees run what their audit says (read from the OpenSSL 3.5.5
sources and, for the provider dispatch functions and the QUIC free path, the
census of the card's report), that the stated functions are members of
`#teardown` of an object in the call's `#freed`, and that the Rust checker
computes `audited`, are premises pinned by the crate's checker tests
(`every_free_names_what_it_runs_on_the_objects_it_frees`,
`a_drifted_free_reading_is_refused`,
`the_teardown_setters_register_their_slots`) and the gateway tests
(`openssl_undeclared_handshake_callbacks`). The premises are metadata: no
verdict-deciding consumer reads them (card SP1).
-/

namespace QCore.OpensslFreeTeardownSlots

open QCore.PremiseFormula

/-! ## Coverage: the teardown premises entail the free statement -/

/-- Every teardown function of every freed object is stated, the library's
or a loaded module's: the condition under which a free drops the
unstated-slots premise. -/
def AllStated (M : Model) : Prop :=
  ∀ o f, M.freed o → M.teardown o f → M.stated f ∨ M.library f ∨ M.loaded f

/-- What `openssl_builtin_teardown_functions` states. -/
theorem eval_builtin (M : Model) :
    (eval M [] builtinTeardownFormula).holds ↔
      ∀ o, M.freed o → ∀ f, M.teardown o f → M.library f → ∀ r, M.runs f r → inertEff M r := by
  simp [eval, builtinTeardownFormula, lk, prim0, prim1, binVal, Val.holds]

/-- The call's objects a loaded function's in-turn callbacks are read on. -/
def callObjects (M : Model) : M.U → Prop := fun u => (M.freed u ∨ M.args u) ∨ M.created u

/-- What `openssl_loaded_module_functions` states (USER DECISION 2026-11-21,
"Both conjuncts"): a loaded module's teardown function is inert, and every
callback it runs in turn on an object of the call is the library's own or a
loaded module's, never an application callback. -/
theorem eval_loaded (M : Model) :
    (eval M [] loadedTeardownFormula).holds ↔
      ∀ o, M.freed o → ∀ f, M.teardown o f → M.loaded f →
        (∀ r, M.runs f r → inertEff M r) ∧
          ∀ g, M.inTurn f (callObjects M) g → M.library g ∨ M.loaded g := by
  unfold callObjects
  simp [eval, loadedTeardownFormula, lk, prim0, prim1, prim2, binVal, Val.holds]

/-- The first conjunct: a loaded teardown function is inert. -/
theorem loaded_functions_are_inert (M : Model) (h : (eval M [] loadedTeardownFormula).holds)
    {o f : M.U} (ho : M.freed o) (ht : M.teardown o f) (hl : M.loaded f) :
    ∀ r, M.runs f r → inertEff M r :=
  ((eval_loaded M).1 h o ho f ht hl).1

/-- The second conjunct: a loaded teardown function runs no application
callback in turn on an object of the call. -/
theorem loaded_functions_run_no_application_callback (M : Model)
    (h : (eval M [] loadedTeardownFormula).holds) {o f g : M.U} (ho : M.freed o)
    (ht : M.teardown o f) (hl : M.loaded f) (hg : M.inTurn f (callObjects M) g) :
    M.library g ∨ M.loaded g :=
  ((eval_loaded M).1 h o ho f ht hl).2 g hg

/-- What the free statement states. -/
theorem eval_free (M : Model) :
    (eval M [] unstatedFreeFormula).holds ↔
      ∀ o, M.freed o → ∀ f, (M.teardown o f ∧ ¬ (M.stated f ∨ M.settable f)) →
        ∀ r, M.runs f r → inertEff M r := by
  simp [eval, unstatedFreeFormula, lk, prim0, prim1, binVal, Val.holds]

/-- What the statement of a free through a released argument states. -/
theorem eval_released (M : Model) :
    (eval M [] unstatedReleasedFreeFormula).holds ↔
      ∀ o, M.freed o → ∀ f, (M.teardown o f ∧ ¬ M.stated f) →
        ∀ r, M.runs f r → inertEff M r := by
  simp [eval, unstatedReleasedFreeFormula, lk, prim0, prim1, binVal, Val.holds]

theorem teardown_premises_cover_the_free_statement (M : Model) (hA : AllStated M)
    (hb : (eval M [] builtinTeardownFormula).holds)
    (hl : (eval M [] loadedTeardownFormula).holds) :
    (eval M [] unstatedFreeFormula).holds := by
  rw [eval_builtin] at hb
  rw [eval_loaded] at hl
  rw [eval_free]
  intro o ho f ⟨ht, hn⟩ r hr
  rcases hA o f ho ht with hs | hlib | hld
  · exact absurd (Or.inl hs) hn
  · exact hb o ho f ht hlib r hr
  · exact (hl o ho f ht hld).1 r hr

/-- A stated function needs no premise at all; only an unstated one that is
neither the library's nor a loaded module's needs the unstated-slots premise.
-/
theorem an_uncovered_function_needs_the_free_statement (M : Model) {o f : M.U}
    (ho : M.freed o) (ht : M.teardown o f) (hs : ¬ M.stated f) (hset : ¬ M.settable f)
    (h : (eval M [] unstatedFreeFormula).holds) : ∀ r, M.runs f r → inertEff M r :=
  (eval_free M).1 h o ho f ⟨ht, fun h' => h'.elim hs hset⟩

theorem released_statement_entails_free_statement (M : Model)
    (h : (eval M [] unstatedReleasedFreeFormula).holds) :
    (eval M [] unstatedFreeFormula).holds := by
  rw [eval_released] at h
  rw [eval_free]
  intro o ho f ⟨ht, hn⟩ r hr
  exact h o ho f ⟨ht, fun hs => hn (Or.inl hs)⟩ r hr

/-- The converse fails: a settable, unstated teardown function is covered by
the released statement but not by the free statement. -/
theorem free_statement_does_not_entail_released :
    ¬ ∀ M : Model, (eval M [] unstatedFreeFormula).holds →
      (eval M [] unstatedReleasedFreeFormula).holds := by
  intro hall
  let M : Model := {
    U := Bool, Run := Unit,
    args := fun _ => False, created := fun _ => False, copied := fun _ => False,
    freed := fun _ => True, cls := id, ctx := fun _ _ => False,
    bios := fun _ _ => False, chain := fun _ _ => False, sslOf := fun _ _ => False,
    slots := fun _ _ => False, stated := fun _ => False, exData := fun _ _ => False,
    exDataFree := fun _ _ => False, method := id, builtin := fun _ => False,
    fns := fun _ _ => False, store := fun _ _ => False, lookups := fun _ _ => False,
    teardown := fun _ _ => True, settable := fun _ => True,
    library := fun _ => False, loaded := fun _ => False,
    inTurn := fun _ _ _ => False, callerHeld := fun _ _ => True,
    takes := fun _ s => s, releases := fun _ _ _ => False,
    retains := fun _ _ _ => False, runs := fun _ _ => True, const := fun _ => true }
  have hF : (eval M [] unstatedFreeFormula).holds := by
    rw [eval_free]; intro o _ f hf; exact absurd (Or.inr trivial) hf.2
  have hR := (eval_released M).1 (hall M hF) true trivial true ⟨trivial, fun h => h⟩ () trivial
  have h1 := congrFun hR.1.1 true
  simp [M, noRes] at h1

theorem the_teardown_formulas_are_well_formed :
    wf builtinTeardownFormula = true ∧ wf loadedTeardownFormula = true ∧
      wf unstatedReleasedFreeFormula = true :=
  teardown_formulas_wf

/-! ## The free audit (`free_audited`) -/

/-- A free audit reading (`Frees`). -/
inductive Reading where
  | runs (slots : List String) (left : Nat)
  | throughRelease (left : Nat)
  | namedBy (premise : String)
  | nothing
  deriving DecidableEq, Repr

/-- A documented entry, as far as the free audit reads it. -/
structure Entry where
  function : String
  releases : Bool
  /-- names the free statement (`#teardown \ (#stated | #settable)`) -/
  freeStmt : Bool
  /-- names the released free's statement (`#teardown \ #stated`) -/
  releasedStmt : Bool
  handshake : Bool
  builtin : Bool
  loaded : Bool
  premises : List String
  /-- the teardown slots it states -/
  stated : List String
  deriving DecidableEq, Repr

/-- `TEARDOWN_SLOTS`. -/
def teardownSlots : List String :=
  ["evp_pkey_asn1_method_st.asn1_meth_pkey_free", "rsa_meth_st.rsa_meth_finish",
    "dsa_method.dsa_meth_finish", "dh_method.dh_meth_finish",
    "ec_key_method_st.ec_key_meth_finish", "engine_st.engine_finish",
    "engine_st.engine_destroy", "engine_st.engine_pkey_meths",
    "engine_st.engine_pkey_asn1_meths", "evp_md_st.md_meth_cleanup",
    "evp_cipher_st.cipher_meth_cleanup", "evp_pkey_method_st.pkey_meth_cleanup",
    "async_wait_ctx_st.wait_fd_cleanup"]

/-- Two lists hold the same elements. -/
def sameSet (a b : List String) : Bool := a.all b.contains && b.all a.contains

/-- `free_audited` over one reading. -/
def decideReading (e : Entry) : Reading → Bool
  | .runs slots left =>
      !e.releasedStmt && !(e.freeStmt && e.handshake) &&
        (e.handshake || (e.freeStmt == (left != 0))) && e.builtin && e.loaded &&
        slots.all teardownSlots.contains && sameSet slots e.stated
  | .throughRelease left =>
      e.releasedStmt && !e.freeStmt && !e.handshake && e.builtin && e.loaded && left != 0 &&
        e.stated.isEmpty
  | .namedBy p => !e.freeStmt && !e.releasedStmt && e.premises.contains p
  | .nothing => !e.freeStmt && !e.releasedStmt && e.stated.isEmpty

/-- The audit's readings of `f`. -/
def listed (f : String) (audit : List (String × Reading)) : List Reading :=
  (audit.filter fun p => p.1 == f).map Prod.snd

def decideReadings (e : Entry) : List Reading → Bool
  | [] => !e.freeStmt && !e.releasedStmt && !e.handshake && !e.releases
  | [r] => decideReading e r
  | _ :: _ :: _ => false

def audited (e : Entry) (audit : List (String × Reading)) : Bool :=
  decideReadings e (listed e.function audit)

theorem a_left_item_keeps_the_premise {e : Entry} {slots : List String} {left : Nat}
    (h : decideReading e (.runs slots left) = true) (hl : left ≠ 0)
    (hh : e.handshake = false) : e.freeStmt = true := by
  cases hf : e.freeStmt <;> simp_all [decideReading]

theorem nothing_left_drops_the_premise {e : Entry} {slots : List String}
    (h : decideReading e (.runs slots 0) = true) (hh : e.handshake = false) :
    e.freeStmt = false ∧ e.releasedStmt = false := by
  cases hf : e.freeStmt <;> cases hr : e.releasedStmt <;> simp_all [decideReading]

/-- Card OPENSSL-INNER-FREES-SILENT-METHOD-CALLBACKS: the handshake's
statement, whose last conjunct is the free statement, stands for it, and
never beside it. -/
theorem the_handshake_statement_never_beside_the_free_statement {e : Entry}
    {slots : List String} {left : Nat} (h : decideReading e (.runs slots left) = true)
    (hh : e.handshake = true) : e.freeStmt = false := by
  cases hf : e.freeStmt <;> simp_all [decideReading]

theorem a_running_free_names_the_teardown_premises {e : Entry} {slots : List String}
    {left : Nat} (h : decideReading e (.runs slots left) = true) :
    e.builtin = true ∧ e.loaded = true := by
  cases hb : e.builtin <;> cases hl : e.loaded <;> simp_all [decideReading]

theorem the_stated_slots_are_the_audited {e : Entry} {slots : List String} {left : Nat}
    (h : decideReading e (.runs slots left) = true) :
    (∀ s ∈ slots, s ∈ e.stated) ∧ (∀ s ∈ e.stated, s ∈ slots) ∧ (∀ s ∈ slots, s ∈ teardownSlots) := by
  simp only [decideReading, Bool.and_eq_true] at h
  have hset := h.2
  have hall := h.1.2
  simp only [sameSet, Bool.and_eq_true, List.all_eq_true, List.contains_iff_mem] at hset hall
  exact ⟨hset.1, hset.2, hall⟩

theorem a_released_free_states_no_slot {e : Entry} {left : Nat}
    (h : decideReading e (.throughRelease left) = true) :
    e.releasedStmt = true ∧ e.freeStmt = false ∧ e.stated = [] ∧ e.builtin = true ∧
      e.loaded = true := by
  cases hr : e.releasedStmt <;> cases hf : e.freeStmt <;> cases hb : e.builtin <;>
    cases hl : e.loaded <;> cases hs : e.stated <;> simp_all [decideReading]

theorem an_unaudited_release_refuses {e : Entry} {audit : List (String × Reading)}
    (hs : e.releases = true) (hn : listed e.function audit = []) : audited e audit = false := by
  simp [audited, hn, decideReadings, hs]

/-! ## Source-derived entries (`source_cited`) -/

/-- An entry's citation: its series, page (the defining file, for a source
entry), range, and the files the source citations give it. -/
structure Cited where
  series : String
  page : String
  range : String
  citedFiles : List String
  deriving DecidableEq, Repr

def sourceSeries : String := "source 3.5.5"
def sinceRead : String := "resource_library_version(openssl, since \"3.5.5\")"

/-- `source_cited`'s acceptance. -/
def sourceCited (c : Cited) : Bool :=
  if c.series == sourceSeries then c.citedFiles == [c.page] && c.range == sinceRead
  else c.citedFiles.isEmpty

theorem a_source_entry_admits_only_the_release_read {c : Cited}
    (h : sourceCited c = true) (hs : c.series = sourceSeries) :
    c.range = sinceRead ∧ c.citedFiles = [c.page] := by
  simp only [sourceCited, hs, beq_self_eq_true, ↓reduceIte, Bool.and_eq_true, beq_iff_eq] at h
  exact ⟨h.2, h.1⟩

theorem a_documented_entry_is_cited_nowhere {c : Cited}
    (h : sourceCited c = true) (hs : c.series ≠ sourceSeries) : c.citedFiles = [] := by
  have hne : (c.series == sourceSeries) = false := by simp [hs]
  simp only [sourceCited, hne] at h
  simpa using h

/-! ## Vectors from the crate's audit -/

def keySlots : List String :=
  ["evp_pkey_asn1_method_st.asn1_meth_pkey_free", "rsa_meth_st.rsa_meth_finish",
    "dsa_method.dsa_meth_finish", "dh_method.dh_meth_finish",
    "ec_key_method_st.ec_key_meth_finish", "engine_st.engine_finish",
    "engine_st.engine_destroy", "engine_st.engine_pkey_meths",
    "engine_st.engine_pkey_asn1_meths"]

def engineSlots : List String :=
  ["engine_st.engine_destroy", "engine_st.engine_pkey_meths",
    "engine_st.engine_pkey_asn1_meths"]

/-- `HANDSHAKE_TEARDOWN`. -/
def handshakeSlots : List String :=
  keySlots ++ ["evp_md_st.md_meth_cleanup", "evp_cipher_st.cipher_meth_cleanup",
    "evp_pkey_method_st.pkey_meth_cleanup"]

/-- A slice of `FREE_AUDIT`, with the number of items each reading leaves. -/
def crateAudit : List (String × Reading) :=
  [("BIO_free", .namedBy "openssl_builtin_bio_methods"),
    ("ENGINE_free", .runs engineSlots 0),
    ("EVP_MD_CTX_free", .throughRelease 1),
    ("EVP_PKEY_free", .runs keySlots 0),
    ("OPENSSL_sk_free", .nothing),
    ("PKCS12_free", .throughRelease 2),
    ("SSL_CTX_free", .runs keySlots 1),
    ("SSL_free", .runs teardownSlots 2),
    ("X509_STORE_free", .runs keySlots 1),
    ("X509_free", .runs keySlots 0),
    -- Card OPENSSL-INNER-FREES-SILENT-METHOD-CALLBACKS's calls that free inside.
    ("SSL_CTX_use_PrivateKey", .runs keySlots 1),
    ("SSL_CTX_load_verify_locations", .runs keySlots 2),
    ("SSL_connect", .runs handshakeSlots 0)]

/-- A free as landed by this card: its premises and stated teardown slots. -/
def landed (f : String) (free released : Bool) (stated : List String) : Entry :=
  ⟨f, true, free, released, false, true, true, ["openssl_ex_data_callbacks"], stated⟩

theorem vectors_as_landed :
    audited (landed "EVP_PKEY_free" false false keySlots) crateAudit = true ∧
      audited (landed "X509_free" false false keySlots) crateAudit = true ∧
      audited (landed "ENGINE_free" false false engineSlots) crateAudit = true ∧
      audited (landed "EVP_MD_CTX_free" false true []) crateAudit = true ∧
      audited (landed "PKCS12_free" false true []) crateAudit = true ∧
      audited (landed "SSL_CTX_free" true false keySlots) crateAudit = true ∧
      audited (landed "X509_STORE_free" true false keySlots) crateAudit = true ∧
      audited (landed "SSL_free" true false teardownSlots) crateAudit = true := by
  decide

/-- A call that frees inside: plain (no release), and its statement. -/
def innerFree (f : String) (free handshake : Bool) (stated : List String) : Entry :=
  ⟨f, false, free, false, handshake, true, true, ["openssl_ex_data_callbacks"], stated⟩

theorem vectors_inner_frees :
    audited (innerFree "SSL_CTX_use_PrivateKey" true false keySlots) crateAudit = true ∧
      audited (innerFree "SSL_CTX_load_verify_locations" true false keySlots) crateAudit = true ∧
      audited (innerFree "SSL_connect" false true handshakeSlots) crateAudit = true ∧
      -- relying on the free statement alone, once the setters are declared
      audited (innerFree "SSL_CTX_use_PrivateKey" true false []) crateAudit = false ∧
      audited (innerFree "SSL_connect" false true []) crateAudit = false ∧
      -- both statements
      audited (innerFree "SSL_connect" true true handshakeSlots) crateAudit = false := by
  decide

theorem vectors_drifted_refuse :
    -- a stated free keeping the premise
    audited (landed "EVP_PKEY_free" true false keySlots) crateAudit = false ∧
      -- a free leaving items dropping it
      audited (landed "SSL_CTX_free" false false keySlots) crateAudit = false ∧
      -- one teardown slot not stated
      audited (landed "X509_free" false false keySlots.tail) crateAudit = false ∧
      -- a released free stating a slot, or naming the free statement
      audited (landed "PKCS12_free" false true keySlots) crateAudit = false ∧
      audited (landed "PKCS12_free" true false []) crateAudit = false ∧
      -- a teardown premise dropped
      audited ⟨"ENGINE_free", true, false, false, false, true, false, [], engineSlots⟩ crateAudit =
        false ∧
      -- an unaudited free
      audited (landed "X509_CRL_free" false false []) crateAudit = false := by
  decide

theorem vector_source_entries :
    sourceCited ⟨"source 3.5.5", "crypto/engine/tb_pkmeth.c", sinceRead,
        ["crypto/engine/tb_pkmeth.c"]⟩ = true ∧
      sourceCited ⟨"source 3.5.5", "crypto/engine/tb_pkmeth.c",
        "resource_library_version(openssl, since \"1.0.0\")", ["crypto/engine/tb_pkmeth.c"]⟩ =
        false ∧
      sourceCited ⟨"1.1.1", "ENGINE_add", "resource_library_version(openssl, documented \"1.1.1\")",
        []⟩ = true ∧
      sourceCited ⟨"1.1.1", "ENGINE_add", "", ["crypto/engine/eng_lib.c"]⟩ = false := by
  decide

end QCore.OpensslFreeTeardownSlots

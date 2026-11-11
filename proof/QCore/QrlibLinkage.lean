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

import QCore.FfiStructImport

/-!
# `.qrlib` as the C linkage manifest of the FFI route

Card **W48-QRLIB-LINKAGE** (card 5 of the `.qr` language track; implemented
grammars §G02d.2/§G02d.3/§G02u, language specification §L14.8, contract
specification §C04.12). The model of `crates/qcore-c-gateway/src/linkage`
(`resolve` and `generate::generate`/`check_stale`) and of the FFI route's
publication (`crates/qcore-core/src/ffi.rs`'s `certificate_carries_linkage`,
`crates/qcore-verification/src/driver.rs`'s `resolve_ffi_links` and
`ffi_verdict`).

## What is modelled

A symbol table is a list of records — a name, what the record says about it
(defined global or weak, undefined, local), whether its version is a hidden
(`@`) one, whether the archive index lists it, and whether an import is by
name. The readers that produce these records from ELF, Mach-O and COFF bytes
are not modelled: they are pinned by the Rust unit tests over the checked-in
fixtures. A manifest is the library's identity, the linkage kind, the artifact
hash and one entry per bound contract, carrying the resolved record.

## What is proved

* **Admission.** A binding the resolver admits names a record of the table
  with that name, **defined**, with its default version, listed by the index
  and imported by name (`an_admitted_binding_names_an_exported_defined_symbol`),
  and it is the only such record (`an_admitted_binding_is_the_only_linkable_one`).
  A missing, undefined, local-only or doubly defined name refuses
  (`missing_undefined_local_and_duplicate_refuse`), and a weak definition is
  admitted with its weakness (`a_weak_definition_is_admitted_as_weak`).
* **Generation is a function of the artifact and the declared inputs.** Two
  artifacts with the same hash whose records agree on every bound name
  generate the same manifest (`generation_reads_only_the_hash_and_the_bound_names`);
  every entry of a generated manifest names an exported defined symbol of the
  artifact (`every_generated_entry_names_an_exported_defined_symbol`); the
  manifest pins the artifact's hash (`the_generated_manifest_pins_the_hash`), so
  a stored manifest is stale against any artifact with another hash
  (`another_artifact_makes_the_stored_manifest_stale`) and never against its
  own (`a_freshly_generated_manifest_is_not_stale`); one refusing bound name
  refuses the whole manifest (`one_refused_binding_refuses_the_manifest`).
* **The FFI verdict carries the artifact premise and never plain Safe.** A
  published FFI result names a linkage premise for every function it calls
  (`a_publication_names_every_called_functions_artifact`), is never plain
  `Safe` (`an_ffi_publication_is_never_plain_safe`), is not published when a
  called function has no link (`an_unlinked_call_is_not_published`), and every
  composed verdict keeps the premise (`every_composed_verdict_keeps_the_linkage_premise`).
-/

namespace QCore.QrlibLinkage

open QCore.VerdictSemigroup
open QCore.FfiStructImport

/-- How a defined symbol binds. -/
inductive Binding where
  | global
  | weak
  deriving DecidableEq, Repr

/-- What one record says about its name. -/
inductive Def where
  | defined (b : Binding)
  | undefined
  | local
  deriving DecidableEq, Repr

/-- One record of an artifact's symbol table. -/
structure Record where
  name : String
  defn : Def
  /-- A hidden (`@`, non-default) ELF version. -/
  hidden : Bool
  /-- The archive's symbol index lists it (true outside archives). -/
  indexed : Bool
  /-- A COFF import by name (true for everything but an ordinal-only import). -/
  byName : Bool
  deriving DecidableEq, Repr

def Record.isDefined (r : Record) : Bool :=
  match r.defn with
  | .defined _ => true
  | _ => false

/-- Linkable by name: defined, under its default version. -/
def Record.linkable (r : Record) : Bool := r.isDefined && !r.hidden

/-- Why a name does not link. -/
inductive Refusal where
  | missing
  | undefined
  | localOnly
  | ambiguous
  | hiddenOnly
  | notIndexed
  | ordinalOnly
  deriving DecidableEq, Repr

/-- The resolver (`linkage::resolve`). -/
def resolve (t : List Record) (s : String) : Except Refusal Record :=
  let named := t.filter (fun r => r.name == s)
  if (named.filter Record.isDefined).isEmpty then
    if named.any (fun r => r.defn == .local) then .error .localOnly
    else if named.any (fun r => r.defn == .undefined) then .error .undefined
    else .error .missing
  else
    match named.filter Record.linkable with
    | [] => .error .hiddenOnly
    | [r] =>
      if !r.indexed then .error .notIndexed
      else if !r.byName then .error .ordinalOnly
      else .ok r
    | _ :: _ :: _ => .error .ambiguous

/-- The refusal a resolution carries, if any. -/
def refusalOf : Except Refusal Record → Option Refusal
  | .error e => some e
  | .ok _ => none

/-- The record a resolution admits, if any. -/
def admittedOf : Except Refusal Record → Option Record
  | .error _ => none
  | .ok r => some r

theorem an_admitted_binding_names_an_exported_defined_symbol (t : List Record) (s : String)
    (r : Record) (h : resolve t s = .ok r) :
    r ∈ t ∧ r.name = s ∧ r.isDefined = true ∧ r.hidden = false ∧ r.indexed = true ∧
      r.byName = true := by
  unfold resolve at h
  simp only at h
  split at h
  · split at h <;> (try split at h) <;> simp at h
  · split at h
    · simp at h
    · rename_i r' hl
      have hmem : r' ∈ (t.filter (fun r => r.name == s)).filter Record.linkable := by
        rw [hl]; simp
      split at h
      · simp at h
      · split at h
        · simp at h
        · simp only [Except.ok.injEq] at h
          subst h
          simp only [List.mem_filter] at hmem
          obtain ⟨⟨hin, hname⟩, hlink⟩ := hmem
          simp only [Record.linkable, Bool.and_eq_true, Bool.not_eq_true'] at hlink
          refine ⟨hin, by simpa using hname, hlink.1, hlink.2, ?_, ?_⟩
          · rename_i hi _
            simpa using hi
          · rename_i hb
            simpa using hb
    · simp at h

theorem an_admitted_binding_is_the_only_linkable_one (t : List Record) (s : String)
    (r : Record) (h : resolve t s = .ok r) (r' : Record) (hin : r' ∈ t) (hname : r'.name = s)
    (hlink : r'.linkable = true) : r' = r := by
  unfold resolve at h
  simp only at h
  split at h
  · split at h <;> (try split at h) <;> simp at h
  · have hmem : r' ∈ (t.filter (fun r => r.name == s)).filter Record.linkable := by
      simp only [List.mem_filter]
      exact ⟨⟨hin, by simp [hname]⟩, hlink⟩
    split at h
    · rename_i hl
      rw [hl] at hmem
      simp at hmem
    · rename_i r'' hl
      rw [hl] at hmem
      simp only [List.mem_singleton] at hmem
      split at h
      · simp at h
      · split at h
        · simp at h
        · simp only [Except.ok.injEq] at h
          rw [hmem, h]
    · simp at h

/-! ### Non-vacuity: the checked-in ELF fixture's facts -/

def fixture : List Record :=
  [⟨"qr_add", .defined .global, false, true, true⟩,
   ⟨"qr_weak", .defined .weak, false, true, true⟩,
   ⟨"qr_localized", .local, false, true, true⟩,
   ⟨"qr_undefined_elsewhere", .undefined, false, true, true⟩,
   ⟨"qr_dup", .defined .global, false, true, true⟩,
   ⟨"qr_dup", .defined .global, false, true, true⟩,
   ⟨"qr_old", .defined .global, true, true, true⟩,
   ⟨"qr_old", .defined .global, false, true, true⟩,
   ⟨"qr_legacy", .defined .global, true, true, true⟩,
   ⟨"qr_unindexed", .defined .global, false, false, true⟩,
   ⟨"qr_by_ordinal", .defined .global, false, true, false⟩]

theorem a_weak_definition_is_admitted_as_weak :
    admittedOf (resolve fixture "qr_weak") =
      some ⟨"qr_weak", .defined .weak, false, true, true⟩ := by
  decide

theorem the_default_version_is_the_one_bound :
    admittedOf (resolve fixture "qr_old") =
      some ⟨"qr_old", .defined .global, false, true, true⟩ := by
  decide

theorem missing_undefined_local_and_duplicate_refuse :
    refusalOf (resolve fixture "qr_stripped") = some .missing ∧
      refusalOf (resolve fixture "qr_undefined_elsewhere") = some .undefined ∧
      refusalOf (resolve fixture "qr_localized") = some .localOnly ∧
      refusalOf (resolve fixture "qr_dup") = some .ambiguous := by
  decide

theorem hidden_unindexed_and_ordinal_only_refuse :
    refusalOf (resolve fixture "qr_legacy") = some .hiddenOnly ∧
      refusalOf (resolve fixture "qr_unindexed") = some .notIndexed ∧
      refusalOf (resolve fixture "qr_by_ordinal") = some .ordinalOnly := by
  decide

/-! ## The generator -/

/-- An artifact as the generator reads it: its hash and its records. -/
structure Artifact where
  hash : String
  table : List Record

/-- The build's declared inputs: the identity, the linkage kind, and the
`(symbol, module)` of every declaration of the contract modules. -/
structure Inputs where
  library : String
  version : String
  dynamic : Bool
  bindings : List (String × String)

structure Entry where
  symbol : String
  module : String
  record : Record
  deriving DecidableEq

structure Manifest where
  library : String
  version : String
  dynamic : Bool
  hash : String
  entries : List Entry
  deriving DecidableEq

/-- One entry per binding, each resolved in the table. -/
def entries (t : List Record) : List (String × String) → Except Refusal (List Entry)
  | [] => .ok []
  | (s, m) :: rest =>
    match resolve t s with
    | .error e => .error e
    | .ok r =>
      match entries t rest with
      | .error e => .error e
      | .ok es => .ok (⟨s, m, r⟩ :: es)

/-- The generator (`generate::generate`). -/
def generate (a : Artifact) (i : Inputs) : Except Refusal Manifest :=
  match entries a.table i.bindings with
  | .error e => .error e
  | .ok es => .ok ⟨i.library, i.version, i.dynamic, a.hash, es⟩

/-- The stale check (`generate::check_stale`): a stored manifest is stale
unless regeneration yields exactly it. -/
def stale (stored : Manifest) (a : Artifact) (i : Inputs) : Bool :=
  match generate a i with
  | .ok m => m != stored
  | .error _ => true

theorem entries_resolve (t : List Record) :
    ∀ (bs : List (String × String)) (es : List Entry), entries t bs = .ok es →
      ∀ e ∈ es, resolve t e.symbol = .ok e.record := by
  intro bs
  induction bs with
  | nil =>
    intro es h e he
    simp only [entries, Except.ok.injEq] at h
    subst h
    simp at he
  | cons b rest ih =>
    intro es h e he
    obtain ⟨s, m⟩ := b
    simp only [entries] at h
    split at h
    · simp at h
    · rename_i r hr
      split at h
      · simp at h
      · rename_i es' hes
        simp only [Except.ok.injEq] at h
        subst h
        simp only [List.mem_cons] at he
        rcases he with he | he
        · subst he
          exact hr
        · exact ih es' hes e he

theorem every_generated_entry_names_an_exported_defined_symbol (a : Artifact) (i : Inputs)
    (m : Manifest) (h : generate a i = .ok m) :
    ∀ e ∈ m.entries, e.record ∈ a.table ∧ e.record.name = e.symbol ∧
      e.record.isDefined = true ∧ e.record.hidden = false := by
  intro e he
  simp only [generate] at h
  split at h
  · simp at h
  · rename_i es hes
    simp only [Except.ok.injEq] at h
    subst h
    have hr := entries_resolve a.table i.bindings es hes e he
    have := an_admitted_binding_names_an_exported_defined_symbol a.table e.symbol e.record hr
    exact ⟨this.1, this.2.1, this.2.2.1, this.2.2.2.1⟩

theorem the_generated_manifest_pins_the_hash (a : Artifact) (i : Inputs) (m : Manifest)
    (h : generate a i = .ok m) : m.hash = a.hash ∧ m.library = i.library ∧ m.dynamic = i.dynamic := by
  simp only [generate] at h
  split at h
  · simp at h
  · simp only [Except.ok.injEq] at h
    subst h
    exact ⟨rfl, rfl, rfl⟩

theorem a_freshly_generated_manifest_is_not_stale (a : Artifact) (i : Inputs) (m : Manifest)
    (h : generate a i = .ok m) : stale m a i = false := by
  simp [stale, h]

theorem another_artifact_makes_the_stored_manifest_stale (a b : Artifact) (i : Inputs)
    (m : Manifest) (h : generate a i = .ok m) (hne : b.hash ≠ a.hash) : stale m b i = true := by
  simp only [stale]
  split
  · rename_i m' hm'
    have hb := (the_generated_manifest_pins_the_hash b i m' hm').1
    have ha := (the_generated_manifest_pins_the_hash a i m h).1
    have : m' ≠ m := by
      intro heq
      apply hne
      rw [← hb, ← ha, heq]
    simpa using this
  · rfl

theorem one_refused_binding_refuses_the_manifest (a : Artifact) (i : Inputs) (s m : String)
    (e : Refusal) (hin : (s, m) ∈ i.bindings) (hr : resolve a.table s = .error e) :
    ∃ e', generate a i = .error e' := by
  have key : ∀ bs : List (String × String), (s, m) ∈ bs → ∃ e', entries a.table bs = .error e' := by
    intro bs
    induction bs with
    | nil => intro h; simp at h
    | cons b rest ih =>
      intro h
      obtain ⟨s', m'⟩ := b
      simp only [entries]
      split
      · rename_i e'' _
        exact ⟨e'', rfl⟩
      · rename_i r hr'
        simp only [List.mem_cons, Prod.mk.injEq] at h
        rcases h with ⟨hs, _⟩ | h
        · subst hs
          rw [hr] at hr'
          simp at hr'
        · obtain ⟨e'', he''⟩ := ih h
          rw [he'']
          exact ⟨e'', rfl⟩
  obtain ⟨e', he'⟩ := key i.bindings hin
  exact ⟨e', by simp [generate, he']⟩

theorem entries_read_only_bound_names (t u : List Record) :
    ∀ bs : List (String × String),
      (∀ p ∈ bs, t.filter (fun r => r.name == p.1) = u.filter (fun r => r.name == p.1)) →
      entries t bs = entries u bs := by
  intro bs
  induction bs with
  | nil => intro _; rfl
  | cons b rest ih =>
    intro h
    obtain ⟨s, m⟩ := b
    have hs : t.filter (fun r => r.name == s) = u.filter (fun r => r.name == s) :=
      h (s, m) (by simp)
    have hres : resolve t s = resolve u s := by
      simp only [resolve, hs]
    have hrest := ih (fun p hp => h p (List.mem_cons_of_mem _ hp))
    simp only [entries, hres, hrest]

/-- The manifest is a function of the artifact's hash, the records of the
bound names, and the declared inputs: a record of any other name, however it
changes, changes nothing. -/
theorem generation_reads_only_the_hash_and_the_bound_names (a b : Artifact) (i : Inputs)
    (hhash : a.hash = b.hash)
    (hnames : ∀ p ∈ i.bindings,
      a.table.filter (fun r => r.name == p.1) = b.table.filter (fun r => r.name == p.1)) :
    generate a i = generate b i := by
  simp only [generate, entries_read_only_bound_names a.table b.table i.bindings hnames, hhash]

/-! ## The FFI publication -/

/-- A called function's link: the callee and the artifact hash it resolved to. -/
structure Link where
  callee : String
  hash : String

/-- The premise name a link publishes. -/
def linkedName (c : String) : String := "linked:" ++ c

/-- The FFI route's publication (`certificate_carries_linkage` and
`ffi_verdict`): published only when every called function has a link, under
the unsafe-FFI mark and one linkage premise per link. -/
def ffiPublish (v : V) (links : List Link) (called : List String) : Option Assumed :=
  if called.all (fun c => links.any (fun l => l.callee == c)) then
    some ⟨v, unsafeFfi :: links.map (fun l => linkedName l.callee)⟩
  else none

theorem a_publication_names_every_called_functions_artifact (v : V) (links : List Link)
    (called : List String) (a : Assumed) (h : ffiPublish v links called = some a) :
    ∀ c ∈ called, linkedName c ∈ a.assumptions := by
  intro c hc
  simp only [ffiPublish] at h
  split at h
  · rename_i hall
    simp only [Option.some.injEq] at h
    subst h
    simp only [List.all_eq_true, List.any_eq_true, beq_iff_eq] at hall
    obtain ⟨l, hl, hlc⟩ := hall c hc
    simp only [List.mem_cons, List.mem_map]
    exact Or.inr ⟨l, hl, by rw [hlc]⟩
  · simp at h

theorem an_ffi_publication_is_never_plain_safe (v : V) (links : List Link)
    (called : List String) (a : Assumed) (h : ffiPublish v links called = some a) :
    ¬ plainSafe a := by
  intro hp
  simp only [ffiPublish] at h
  split at h
  · simp only [Option.some.injEq] at h
    subst h
    simp [plainSafe] at hp
  · simp at h

theorem an_unlinked_call_is_not_published (v : V) (links : List Link) (called : List String)
    (c : String) (hc : c ∈ called) (hnone : ∀ l ∈ links, l.callee ≠ c) :
    ffiPublish v links called = none := by
  simp only [ffiPublish]
  split
  · rename_i hall
    simp only [List.all_eq_true, List.any_eq_true, beq_iff_eq] at hall
    obtain ⟨l, hl, hlc⟩ := hall c hc
    exact absurd hlc (hnone l hl)
  · rfl

theorem every_composed_verdict_keeps_the_linkage_premise (parts : List Assumed) (c : String)
    (h : ∃ a ∈ parts, linkedName c ∈ a.assumptions) :
    linkedName c ∈ (FfiStructImport.fold parts).assumptions :=
  foldl_keeps (linkedName c) parts _ (Or.inr h)

end QCore.QrlibLinkage

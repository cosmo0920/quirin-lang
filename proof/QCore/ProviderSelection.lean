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

import Std

namespace QCore.ProviderSelection

/-! Rule model for contract selection by provider crate and the contracts-only
evidence mode (card W44-QUIRIN-AUDIT-PROVIDERS).

`qcore_audit::providers::resolve_providers` resolves a selection of provider
crates (`NAME`, every owned module, or `NAME:MODULE`, one) on the published
contract load path; `qcore_audit::current_cli` and `qcore_audit::run_config`
choose the audit's contract source; `qcore_audit::evidence` fixes what a
contracts-only run admits. This module models:

* expansion: every selected module is owned by a crate on the load path; an
  unknown crate, an unowned module and an empty selection refuse, and a
  module selected twice refuses;
* the resource view (card W44-PROVIDER-RESOURCE-VIEW): a whole-crate entry
  selects exactly the crate's declared view, a subset of its modules; a crate
  that declares none refuses as a whole-crate entry, and a selection that
  resolves to no module refuses;
* all loaded: with no selection, a contracts-only run selects every crate on
  the load path whole, which resolves to the concatenation of every view in
  load-path order; this default exists only in contracts-only mode, needs the
  load path, and an explicit selection or manifest narrows it;
* the dependency closure: a worklist walk over declared dependency edges,
  total-or-refuse (fuel exhaustion is itself a refusal), whose result contains
  every selected crate and is closed under dependencies, and names only crates
  on the load path;
* the opt-in guard: an admitted guarded selection's closure holds only
  `quirin-provider-*` crates;
* lowering: the lowered manifest keeps the selection (crate, module, order)
  and spells a module as a load-path reference exactly when it uses inclusion;
* the both-forms rule: a command-line and a configuration selection are
  admitted only when they select the same modules, and nothing is merged;
* the contract source: a provider selection with an explicit manifest refuses,
  a selection supersedes only a profile default, and with no selection, no
  manifest and no profile default there is no contract source at all (no
  default provider set);
* contracts-only: such a run admits contract evidence only, and a request that
  also names package, verifier-host, plugin, native or pinned evidence
  refuses rather than being narrowed;
* the job count (`qcore_audit::parallel::jobs_with_precedence`): the flag,
  then the configuration, then the environment, then the host default capped
  at 16; a zero from any consulted source refuses, and an admitted count is
  positive. That the count cannot change an output is the ordered fold's
  property (card W44-AUDIT-PARALLEL-UNITS), not modelled here.

Names and modules are modelled as natural numbers (interned identifiers).
Correspondence with the Rust implementation is maintained by review; it is not
an extraction theorem (constraint 28). -/

abbrev Name := Nat
abbrev Module := Nat

structure Crate where
  name : Name
  /-- `quirin-provider-*` (true) or `qcore-provider-*` (false). -/
  quirin : Bool
  deps : List Name
  modules : List Module
  /-- The declared resource view (`resource_view` in the load path), if any.
  `providers/contracts.py` generates it from the loader and checks it; the
  load-path reader admits only a view of owned modules (`ViewsOwned`). -/
  view : Option (List Module)
  deriving DecidableEq, Repr

inductive Entry where
  | whole (crate : Name)
  | one (crate : Name) (module : Module)
  deriving DecidableEq, Repr

inductive Refusal where
  | empty
  | unknownProvider
  | missingModule
  | duplicateModule
  | qcoreInClosure
  | conflict
  | noContracts
  | twoDeclarations
  | mixedEvidence
  | exhausted
  | zeroJobs
  | noView
  | noModules
  | noLoadPath
  deriving DecidableEq, Repr

/-- The crate with this name on the load path (the load path itself has
already refused duplicate names, cycles and missing dependencies). -/
def findCrate (lp : List Crate) (n : Name) : Option Crate :=
  lp.find? (fun k => k.name == n)

/-- Every declared view names only modules its crate owns: the load-path
reader's check (`QI014`). -/
def ViewsOwned (lp : List Crate) : Prop :=
  ∀ k ∈ lp, ∀ v, k.view = some v → ∀ m ∈ v, m ∈ k.modules

/-! ## 1. Expansion -/

def expand (lp : List Crate) : Entry → Except Refusal (List (Name × Module))
  | .whole c =>
    match findCrate lp c with
    | none => .error .unknownProvider
    | some k =>
      match k.view with
      | none => .error .noView
      | some v => .ok (v.map (fun m => (c, m)))
  | .one c m =>
    match findCrate lp c with
    | none => .error .unknownProvider
    | some k => if m ∈ k.modules then .ok [(c, m)] else .error .missingModule

def expandAll (lp : List Crate) : List Entry → Except Refusal (List (Name × Module))
  | [] => .ok []
  | e :: es =>
    match expand lp e with
    | .error r => .error r
    | .ok ms =>
      match expandAll lp es with
      | .error r => .error r
      | .ok rest => .ok (ms ++ rest)

/-- Every expanded module is owned by a crate on the load path. -/
def Owned (lp : List Crate) (p : Name × Module) : Prop :=
  ∃ k, findCrate lp p.1 = some k ∧ p.2 ∈ k.modules

theorem findCrate_mem {lp : List Crate} {n : Name} {k : Crate}
    (h : findCrate lp n = some k) : k ∈ lp :=
  List.mem_of_find?_eq_some h

theorem expand_sound {lp : List Crate} (hv : ViewsOwned lp) {e : Entry}
    {ms : List (Name × Module)}
    (h : expand lp e = .ok ms) : ∀ p ∈ ms, Owned lp p := by
  intro p hp
  cases e with
  | whole c =>
    simp only [expand] at h
    split at h
    · cases h
    · rename_i k hk
      split at h
      · cases h
      · rename_i v hkv
        cases h
        rcases List.mem_map.mp hp with ⟨m, hm, rfl⟩
        exact ⟨k, hk, hv k (findCrate_mem hk) v hkv m hm⟩
  | one c m =>
    simp only [expand] at h
    split at h
    · cases h
    · rename_i k hk
      split at h
      · rename_i hm
        cases h
        simp at hp
        subst hp
        exact ⟨k, hk, hm⟩
      · cases h

theorem expandAll_sound {lp : List Crate} (hv : ViewsOwned lp) :
    ∀ {es : List Entry} {ms : List (Name × Module)},
      expandAll lp es = .ok ms → ∀ p ∈ ms, Owned lp p := by
  intro es
  induction es with
  | nil =>
    intro ms h p hp
    simp only [expandAll] at h
    cases h
    simp at hp
  | cons e es ih =>
    intro ms h p hp
    simp only [expandAll] at h
    split at h
    · cases h
    · rename_i first hfirst
      split at h
      · cases h
      · rename_i rest hrest
        cases h
        rcases List.mem_append.mp hp with hp | hp
        · exact expand_sound hv hfirst p hp
        · exact ih hrest p hp

/-- A whole-crate entry selects exactly the crate's declared view. -/
theorem expand_whole_is_view {lp : List Crate} {c : Name} {k : Crate} {v : List Module}
    (hc : findCrate lp c = some k) (hv : k.view = some v) :
    expand lp (.whole c) = .ok (v.map (fun m => (c, m))) := by
  simp [expand, hc, hv]

/-- A crate that declares no view cannot be selected whole. -/
theorem expand_whole_without_view_refused {lp : List Crate} {c : Name} {k : Crate}
    (hc : findCrate lp c = some k) (hv : k.view = none) :
    expand lp (.whole c) = .error .noView := by
  simp [expand, hc, hv]

/-! ## 2. The dependency closure -/

/-- A worklist walk over declared dependency edges. Fuel exhaustion refuses. -/
def closure (lp : List Crate) : Nat → List Name → List Name → Except Refusal (List Name)
  | 0, _, _ => .error .exhausted
  | _ + 1, [], acc => .ok acc
  | fuel + 1, n :: rest, acc =>
    if n ∈ acc then closure lp fuel rest acc
    else
      match findCrate lp n with
      | none => .error .unknownProvider
      | some k => closure lp fuel (k.deps ++ rest) (n :: acc)

theorem closure_contains {lp : List Crate} :
    ∀ (fuel : Nat) (pending acc r : List Name), closure lp fuel pending acc = .ok r →
      ∀ n, (n ∈ pending ∨ n ∈ acc) → n ∈ r := by
  intro fuel
  induction fuel with
  | zero => intro pending acc r h; simp [closure] at h
  | succ f ih =>
    intro pending acc r h n hn
    cases pending with
    | nil =>
      simp only [closure] at h
      cases h
      simpa using hn
    | cons m rest =>
      simp only [closure] at h
      split at h
      · rename_i hm
        apply ih rest acc r h n
        rcases hn with hn | hn
        · rcases List.mem_cons.mp hn with rfl | hn
          · exact Or.inr hm
          · exact Or.inl hn
        · exact Or.inr hn
      · split at h
        · cases h
        · rename_i k _
          apply ih (k.deps ++ rest) (m :: acc) r h n
          rcases hn with hn | hn
          · rcases List.mem_cons.mp hn with rfl | hn
            · exact Or.inr (by simp)
            · exact Or.inl (List.mem_append_right _ hn)
          · exact Or.inr (List.mem_cons_of_mem _ hn)

/-- Every crate in `acc` has each dependency in `acc` or still pending. -/
def DepsCovered (lp : List Crate) (acc pending : List Name) : Prop :=
  ∀ n ∈ acc, ∀ k, findCrate lp n = some k → ∀ d ∈ k.deps, d ∈ acc ∨ d ∈ pending

theorem closure_closed {lp : List Crate} :
    ∀ (fuel : Nat) (pending acc r : List Name), closure lp fuel pending acc = .ok r →
      DepsCovered lp acc pending →
      ∀ n ∈ r, ∀ k, findCrate lp n = some k → ∀ d ∈ k.deps, d ∈ r := by
  intro fuel
  induction fuel with
  | zero => intro pending acc r h; simp [closure] at h
  | succ f ih =>
    intro pending acc r h inv
    cases pending with
    | nil =>
      simp only [closure] at h
      cases h
      intro n hn k hk d hd
      rcases inv n hn k hk d hd with hd | hd
      · exact hd
      · simp at hd
    | cons m rest =>
      simp only [closure] at h
      split at h
      · rename_i hm
        apply ih rest acc r h
        intro n hn k hk d hd
        rcases inv n hn k hk d hd with hd | hd
        · exact Or.inl hd
        · rcases List.mem_cons.mp hd with rfl | hd
          · exact Or.inl hm
          · exact Or.inr hd
      · split at h
        · cases h
        · rename_i km hkm
          apply ih (km.deps ++ rest) (m :: acc) r h
          intro n hn k hk d hd
          rcases List.mem_cons.mp hn with rfl | hn
          · rw [hkm] at hk
            cases hk
            exact Or.inr (List.mem_append_left _ hd)
          · rcases inv n hn k hk d hd with hd | hd
            · exact Or.inl (List.mem_cons_of_mem _ hd)
            · rcases List.mem_cons.mp hd with rfl | hd
              · exact Or.inl (by simp)
              · exact Or.inr (List.mem_append_right _ hd)

/-- Every crate the walk admits is on the load path. -/
def Known (lp : List Crate) (acc : List Name) : Prop :=
  ∀ n ∈ acc, ∃ k, findCrate lp n = some k

theorem closure_known {lp : List Crate} :
    ∀ (fuel : Nat) (pending acc r : List Name), closure lp fuel pending acc = .ok r →
      Known lp acc → Known lp r := by
  intro fuel
  induction fuel with
  | zero => intro pending acc r h; simp [closure] at h
  | succ f ih =>
    intro pending acc r h known
    cases pending with
    | nil =>
      simp only [closure] at h
      cases h
      exact known
    | cons m rest =>
      simp only [closure] at h
      split at h
      · exact ih rest acc r h known
      · split at h
        · cases h
        · rename_i km hkm
          apply ih (km.deps ++ rest) (m :: acc) r h
          intro n hn
          rcases List.mem_cons.mp hn with rfl | hn
          · exact ⟨km, hkm⟩
          · exact known n hn

/-- A dependency the load path does not hold refuses the walk. -/
theorem closure_unknown_refused {lp : List Crate} {fuel : Nat} {n : Name}
    {rest acc : List Name} (hn : n ∉ acc) (hk : findCrate lp n = none) :
    closure lp (fuel + 1) (n :: rest) acc = .error .unknownProvider := by
  simp [closure, hn, hk]

/-! ## 3. The guard -/

def quirinOnly (lp : List Crate) (cl : List Name) : Bool :=
  cl.all (fun n => match findCrate lp n with
    | some k => k.quirin
    | none => false)

theorem quirinOnly_sound {lp : List Crate} {cl : List Name} (h : quirinOnly lp cl = true) :
    ∀ n ∈ cl, ∃ k, findCrate lp n = some k ∧ k.quirin = true := by
  intro n hn
  unfold quirinOnly at h
  rw [List.all_eq_true] at h
  have hn' := h n hn
  cases hk : findCrate lp n with
  | none => rw [hk] at hn'; simp at hn'
  | some k => rw [hk] at hn'; exact ⟨k, rfl, by simpa using hn'⟩

/-! ## 4. Resolution -/

structure Resolved where
  modules : List (Name × Module)
  closure : List Name
  deriving DecidableEq, Repr

def resolve (lp : List Crate) (fuel : Nat) (entries : List Entry) (guard : Bool) :
    Except Refusal Resolved :=
  if entries = [] then .error .empty
  else
    match expandAll lp entries with
    | .error r => .error r
    | .ok mods =>
      if mods = [] then .error .noModules
      else if mods.Nodup then
        match closure lp fuel (mods.map Prod.fst) [] with
        | .error r => .error r
        | .ok cl =>
          if guard && !quirinOnly lp cl then .error .qcoreInClosure
          else .ok ⟨mods, cl⟩
      else .error .duplicateModule

/-- The facts every admitted resolution carries. -/
theorem resolve_sound {lp : List Crate} (hv : ViewsOwned lp) {fuel : Nat}
    {entries : List Entry} {guard : Bool}
    {r : Resolved} (h : resolve lp fuel entries guard = .ok r) :
    entries ≠ [] ∧
    r.modules ≠ [] ∧
    (∀ p ∈ r.modules, Owned lp p) ∧
    r.modules.Nodup ∧
    (∀ p ∈ r.modules, p.1 ∈ r.closure) ∧
    (∀ n ∈ r.closure, ∀ k, findCrate lp n = some k → ∀ d ∈ k.deps, d ∈ r.closure) ∧
    (∀ n ∈ r.closure, ∃ k, findCrate lp n = some k) ∧
    (guard = true → ∀ n ∈ r.closure, ∃ k, findCrate lp n = some k ∧ k.quirin = true) := by
  unfold resolve at h
  split at h
  · cases h
  · rename_i hne
    split at h
    · cases h
    · rename_i mods hmods
      split at h
      · cases h
      · rename_i hnil
        split at h
        · rename_i hnodup
          split at h
          · cases h
          · rename_i cl hcl
            split at h
            · cases h
            · rename_i hguard
              cases h
              refine ⟨hne, hnil, expandAll_sound hv hmods, hnodup, ?_, ?_, ?_, ?_⟩
              · intro p hp
                exact closure_contains fuel _ [] cl hcl p.1
                  (Or.inl (List.mem_map_of_mem hp))
              · exact closure_closed fuel _ [] cl hcl (by intro n hn; simp at hn)
              · exact closure_known fuel _ [] cl hcl (by intro n hn; simp at hn)
              · intro hg
                subst hg
                apply quirinOnly_sound
                simpa using hguard
        · cases h

theorem resolve_empty_refused (lp : List Crate) (fuel : Nat) (guard : Bool) :
    resolve lp fuel [] guard = .error .empty := by
  simp [resolve]

theorem resolve_unknown_provider_refused {lp : List Crate} {fuel : Nat} {c : Name}
    {rest : List Entry} {guard : Bool} (hc : findCrate lp c = none) :
    resolve lp fuel (.whole c :: rest) guard = .error .unknownProvider ∧
    ∀ m, resolve lp fuel (.one c m :: rest) guard = .error .unknownProvider := by
  constructor
  · simp [resolve, expandAll, expand, hc]
  · intro m
    simp [resolve, expandAll, expand, hc]

theorem resolve_missing_module_refused {lp : List Crate} {fuel : Nat} {c : Name}
    {k : Crate} {m : Module} {rest : List Entry} {guard : Bool}
    (hc : findCrate lp c = some k) (hm : m ∉ k.modules) :
    resolve lp fuel (.one c m :: rest) guard = .error .missingModule := by
  simp [resolve, expandAll, expand, hc, hm]

theorem resolve_duplicate_refused {lp : List Crate} {fuel : Nat} {c : Name}
    {k : Crate} {m : Module} {guard : Bool}
    (hc : findCrate lp c = some k) (hm : m ∈ k.modules) :
    resolve lp fuel [.one c m, .one c m] guard = .error .duplicateModule := by
  simp [resolve, expandAll, expand, hc, hm]

/-- A crate whose view is empty contributes no module; selected alone, it
resolves to nothing and refuses. -/
theorem resolve_empty_view_refused {lp : List Crate} {fuel : Nat} {c : Name}
    {k : Crate} {guard : Bool} (hc : findCrate lp c = some k) (hv : k.view = some []) :
    resolve lp fuel [.whole c] guard = .error .noModules := by
  simp [resolve, expandAll, expand, hc, hv]

/-- An admitted resolution's modules are the expansion of its entries. -/
theorem resolve_modules {lp : List Crate} {fuel : Nat} {entries : List Entry}
    {guard : Bool} {r : Resolved} (h : resolve lp fuel entries guard = .ok r) :
    expandAll lp entries = .ok r.modules := by
  unfold resolve at h
  split at h
  · cases h
  · split at h
    · cases h
    · rename_i mods hmods
      split at h
      · cases h
      · split at h
        · split at h
          · cases h
          · split at h
            · cases h
            · cases h
              exact hmods
        · cases h

/-- A guarded selection whose closure reaches a `qcore-provider-*` crate
refuses. -/
theorem resolve_guard_refuses {lp : List Crate} {fuel : Nat} {entries : List Entry}
    {r : Resolved} (h : resolve lp fuel entries false = .ok r)
    (hq : quirinOnly lp r.closure = false) :
    resolve lp fuel entries true = .error .qcoreInClosure := by
  unfold resolve at h ⊢
  by_cases hne : entries = []
  · simp [hne] at h
  · simp only [hne, ite_false] at h ⊢
    cases hmods : expandAll lp entries with
    | error e => simp [hmods] at h
    | ok mods =>
      simp only [hmods] at h ⊢
      by_cases hnil : mods = []
      · simp [hnil] at h
      simp only [hnil, ite_false] at h ⊢
      by_cases hnodup : mods.Nodup
      · simp only [hnodup, ite_true] at h ⊢
        cases hcl : closure lp fuel (mods.map Prod.fst) [] with
        | error e => simp [hcl] at h
        | ok cl =>
          simp only [hcl] at h ⊢
          simp at h
          subst h
          simp at hq
          simp [hq]
      · simp [hnodup] at h

/-! ## 4b. All loaded (card W44-PROVIDER-RESOURCE-VIEW) -/

/-- The contracts-only default: every crate on the load path, whole, in
load-path order. -/
def allLoaded (lp : List Crate) : List Entry :=
  lp.map (fun k => .whole k.name)

/-- Every view, concatenated in load-path order. -/
def allViews (lp : List Crate) : List (Name × Module) :=
  lp.flatMap (fun k => (k.view.getD []).map (fun m => (k.name, m)))

/-- With distinct crate names, a crate's own name finds it. -/
theorem findCrate_self : ∀ {lp : List Crate} {k : Crate},
    (lp.map Crate.name).Nodup → k ∈ lp → findCrate lp k.name = some k := by
  intro lp
  induction lp with
  | nil => intro k _ hk; simp at hk
  | cons a rest ih =>
    intro k hnd hk
    rw [List.map_cons, List.nodup_cons] at hnd
    unfold findCrate
    rcases List.mem_cons.mp hk with rfl | hk
    · simp
    · have hne : a.name ≠ k.name := by
        intro heq
        exact hnd.1 (heq ▸ List.mem_map_of_mem hk)
      rw [List.find?_cons_of_neg (by simpa using hne)]
      exact ih hnd.2 hk

theorem expandAll_whole_crates {lp : List Crate} (hnd : (lp.map Crate.name).Nodup) :
    ∀ {ks : List Crate}, (∀ k ∈ ks, k ∈ lp ∧ k.view.isSome) →
      expandAll lp (ks.map (fun k => .whole k.name)) =
        .ok (ks.flatMap (fun k => (k.view.getD []).map (fun m => (k.name, m)))) := by
  intro ks
  induction ks with
  | nil => intro _; rfl
  | cons k rest ih =>
    intro hks
    have ⟨hk, hsome⟩ := hks k (by simp)
    obtain ⟨v, hv⟩ := Option.isSome_iff_exists.mp hsome
    have hrest := ih (fun k' hk' => hks k' (List.mem_cons_of_mem _ hk'))
    simp only [List.map_cons, expandAll, List.flatMap_cons]
    rw [expand_whole_is_view (findCrate_self hnd hk) hv, hrest]
    simp [hv]

/-- The all-loaded default resolves to every crate's view, in load-path
order: nothing more, nothing less. -/
theorem resolve_allLoaded_is_allViews {lp : List Crate} {fuel : Nat} {guard : Bool}
    {r : Resolved} (hnd : (lp.map Crate.name).Nodup) (hviews : ∀ k ∈ lp, k.view.isSome)
    (h : resolve lp fuel (allLoaded lp) guard = .ok r) : r.modules = allViews lp := by
  have hexp := resolve_modules h
  unfold allLoaded at hexp
  rw [expandAll_whole_crates hnd (fun k hk => ⟨hk, hviews k hk⟩)] at hexp
  simp only [Except.ok.injEq] at hexp
  rw [← hexp]
  rfl

/-- All-loaded is exactly naming every crate whole, in load-path order. -/
theorem allLoaded_is_every_crate_whole (lp : List Crate) :
    (allLoaded lp).map (fun e => match e with | .whole c => some c | .one _ _ => none) =
      lp.map (fun k => some k.name) := by
  simp [allLoaded, List.map_map, Function.comp_def]

/-! ## 5. Lowering -/

inductive Spelling where
  | path
  | reference
  deriving DecidableEq, Repr

/-- A module that uses inclusion is spelled as a load-path reference; any
other as a file path, which loads exactly as a schema-1 entry. -/
def lower (usesInclusion : Name × Module → Bool) (mods : List (Name × Module)) :
    List ((Name × Module) × Spelling) :=
  mods.map (fun p => (p, if usesInclusion p then .reference else .path))

theorem lower_keeps_selection (u : Name × Module → Bool) (mods : List (Name × Module)) :
    (lower u mods).map Prod.fst = mods := by
  simp [lower, List.map_map, Function.comp_def]

theorem lower_reference_iff {u : Name × Module → Bool} {mods : List (Name × Module)}
    {t : (Name × Module) × Spelling} (ht : t ∈ lower u mods) :
    t.2 = .reference ↔ u t.1 = true := by
  simp only [lower, List.mem_map] at ht
  rcases ht with ⟨p, _, rfl⟩
  cases u p <;> simp

/-! ## 6. Both forms -/

/-- A selection declared on the command line and in a configuration file. -/
def agree (commandLine configuration : Resolved) : Except Refusal Resolved :=
  if commandLine.modules = configuration.modules then .ok commandLine else .error .conflict

theorem agree_ok_iff {a b r : Resolved} :
    agree a b = .ok r ↔ a.modules = b.modules ∧ r = a := by
  unfold agree
  split
  · rename_i heq
    constructor
    · intro h; cases h; exact ⟨heq, rfl⟩
    · intro h; rw [h.2]
  · rename_i hne
    constructor
    · intro h; cases h
    · intro h; exact absurd h.1 hne

theorem agree_conflict {a b : Resolved} (h : a.modules ≠ b.modules) :
    agree a b = .error .conflict := by
  simp [agree, h]

/-! ## 7. The contract source -/

inductive Source where
  | providers
  | manifest
  | profileDefault
  | allLoaded
  deriving DecidableEq, Repr

def chooseSource (providers explicitManifest profileManifest : Bool) : Except Refusal Source :=
  if providers then
    if explicitManifest then .error .twoDeclarations else .ok .providers
  else if explicitManifest then .ok .manifest
  else if profileManifest then .ok .profileDefault
  else .error .noContracts

/-- There is no default provider set. -/
theorem no_default_provider_set :
    chooseSource false false false = .error .noContracts := by
  simp [chooseSource]

theorem providers_and_explicit_manifest_refused (profile : Bool) :
    chooseSource true true profile = .error .twoDeclarations := by
  simp [chooseSource]

/-- A provider selection supersedes only the profile's default. -/
theorem providers_supersede_profile_default (profile : Bool) :
    chooseSource true false profile = .ok .providers := by
  simp [chooseSource]

theorem chooseSource_providers_only_when_selected {p e d : Bool}
    (h : chooseSource p e d = .ok .providers) : p = true ∧ e = false := by
  unfold chooseSource at h
  cases p <;> cases e <;> cases d <;> simp at h ⊢

/-! ## 8. Contracts-only -/

inductive Mode where
  | configured
  | contractsOnly
  deriving DecidableEq, Repr

/-- The contract source per mode (card W44-PROVIDER-RESOURCE-VIEW): the full
mode is `chooseSource`, unchanged; a contracts-only run with no selection and
no explicit manifest loads all (`allLoaded`), which needs the load path, and
supersedes the profile default. -/
def chooseSourceIn (mode : Mode) (providers explicitManifest profileManifest loadPath : Bool) :
    Except Refusal Source :=
  match mode with
  | .configured => chooseSource providers explicitManifest profileManifest
  | .contractsOnly =>
    if providers || explicitManifest then chooseSource providers explicitManifest profileManifest
    else if loadPath then .ok .allLoaded
    else .error .noLoadPath

theorem full_mode_unchanged (p e d l : Bool) :
    chooseSourceIn .configured p e d l = chooseSource p e d := rfl

theorem contracts_only_default_is_all_loaded (d : Bool) :
    chooseSourceIn .contractsOnly false false d true = .ok .allLoaded := by
  simp [chooseSourceIn]

theorem contracts_only_all_loaded_needs_load_path (d : Bool) :
    chooseSourceIn .contractsOnly false false d false = .error .noLoadPath := by
  simp [chooseSourceIn]

/-- An explicit selection or manifest narrows the all-loaded default. -/
theorem contracts_only_explicit_narrows (d l : Bool) :
    chooseSourceIn .contractsOnly true false d l = .ok .providers ∧
    chooseSourceIn .contractsOnly false true d l = .ok .manifest := by
  simp [chooseSourceIn, chooseSource]

/-- All-loaded is chosen only in contracts-only mode, only with no selection
and no explicit manifest, and only with a load path. -/
theorem allLoaded_only_when_contracts_only {m : Mode} {p e d l : Bool}
    (h : chooseSourceIn m p e d l = .ok .allLoaded) :
    m = .contractsOnly ∧ p = false ∧ e = false ∧ l = true := by
  cases m <;> cases p <;> cases e <;> cases d <;> cases l <;>
    simp [chooseSourceIn, chooseSource] at h ⊢

inductive Evidence where
  | contract
  | package
  | verifierHost
  | plugin
  | native
  | pinned
  deriving DecidableEq, Repr

def admitRun (mode : Mode) (requested : List Evidence) : Except Refusal (List Evidence) :=
  match mode with
  | .configured => .ok requested
  | .contractsOnly =>
    if requested.all (· == .contract) then .ok requested else .error .mixedEvidence

theorem contractsOnly_admits_only_contracts {requested admitted : List Evidence}
    (h : admitRun .contractsOnly requested = .ok admitted) : ∀ e ∈ admitted, e = .contract := by
  simp only [admitRun] at h
  split at h
  · rename_i hall
    cases h
    intro e he
    rw [List.all_eq_true] at hall
    simpa using hall e he
  · cases h

theorem contractsOnly_mixed_refused {requested : List Evidence} {e : Evidence}
    (he : e ∈ requested) (hne : e ≠ .contract) :
    admitRun .contractsOnly requested = .error .mixedEvidence := by
  simp only [admitRun]
  split
  · rename_i hall
    rw [List.all_eq_true] at hall
    exact absurd (by simpa using hall e he) hne
  · rfl

/-- Nothing is narrowed: an admitted request is returned whole. -/
theorem admitRun_never_narrows {mode : Mode} {requested admitted : List Evidence}
    (h : admitRun mode requested = .ok admitted) : admitted = requested := by
  cases mode with
  | configured => simp only [admitRun] at h; cases h; rfl
  | contractsOnly =>
    simp only [admitRun] at h
    split at h
    · cases h; rfl
    · cases h

/-! ## 9. The job count -/

def cap : Nat := 16

def jobsWithPrecedence (flag config env : Option Nat) (host : Nat) : Except Refusal Nat :=
  match flag, config with
  | some 0, _ => .error .zeroJobs
  | some n, _ => .ok n
  | none, some 0 => .error .zeroJobs
  | none, some n => .ok n
  | none, none =>
    match env with
    | some 0 => .error .zeroJobs
    | some n => .ok n
    | none => .ok (max 1 (min host cap))

theorem jobs_flag_first {n : Nat} (config env : Option Nat) (host : Nat) (h : n ≠ 0) :
    jobsWithPrecedence (some n) config env host = .ok n := by
  cases n with
  | zero => exact absurd rfl h
  | succ k => rfl

theorem jobs_config_second {n : Nat} (env : Option Nat) (host : Nat) (h : n ≠ 0) :
    jobsWithPrecedence none (some n) env host = .ok n := by
  cases n with
  | zero => exact absurd rfl h
  | succ k => rfl

theorem jobs_environment_third {n : Nat} (host : Nat) (h : n ≠ 0) :
    jobsWithPrecedence none none (some n) host = .ok n := by
  cases n with
  | zero => exact absurd rfl h
  | succ k => rfl

theorem jobs_default_capped (host : Nat) :
    jobsWithPrecedence none none none host = .ok (max 1 (min host cap)) := rfl

theorem jobs_positive {flag config env : Option Nat} {host n : Nat}
    (h : jobsWithPrecedence flag config env host = .ok n) : 0 < n := by
  rcases flag with _ | ⟨_ | f⟩ <;> rcases config with _ | ⟨_ | c⟩ <;>
    rcases env with _ | ⟨_ | e⟩ <;>
    simp [jobsWithPrecedence, cap] at h <;> omega

theorem jobs_zero_flag_refused (config env : Option Nat) (host : Nat) :
    jobsWithPrecedence (some 0) config env host = .error .zeroJobs := rfl

theorem jobs_zero_config_refused (env : Option Nat) (host : Nat) :
    jobsWithPrecedence none (some 0) env host = .error .zeroJobs := rfl

/-! ## 10. Per-library provider scoping and dependent-library closure -/

abbrev LibraryName := Nat

structure LibraryScope where
  name : LibraryName
  providers : List Entry
  dependentLibraries : List LibraryName
  deriving DecidableEq, Repr

def findLibraryScope (scopes : List LibraryScope) (l : LibraryName) : Option LibraryScope :=
  scopes.find? (fun s => s.name == l)

/-- Worklist walk over `dependentLibraries` edges across declared library
scopes. Fuel exhaustion (cycles) and unknown dependent libraries refuse. -/
def libraryClosure (scopes : List LibraryScope) :
    Nat → List LibraryName → List LibraryName → Except Refusal (List LibraryName)
  | 0, _, _ => .error .exhausted
  | _ + 1, [], acc => .ok acc
  | fuel + 1, l :: rest, acc =>
    if l ∈ acc then libraryClosure scopes fuel rest acc
    else
      match findLibraryScope scopes l with
      | none => .error .unknownProvider
      | some sc => libraryClosure scopes fuel (sc.dependentLibraries ++ rest) (l :: acc)

theorem libraryClosure_contains {scopes : List LibraryScope} :
    ∀ (fuel : Nat) (pending acc r : List LibraryName),
      libraryClosure scopes fuel pending acc = .ok r →
      ∀ l, (l ∈ pending ∨ l ∈ acc) → l ∈ r := by
  intro fuel
  induction fuel with
  | zero => intro pending acc r h; simp [libraryClosure] at h
  | succ f ih =>
    intro pending acc r h l hl
    cases pending with
    | nil =>
      simp only [libraryClosure] at h
      cases h
      simpa using hl
    | cons m rest =>
      simp only [libraryClosure] at h
      split at h
      · rename_i hm
        apply ih rest acc r h l
        rcases hl with hl | hl
        · rcases List.mem_cons.mp hl with rfl | hl
          · exact Or.inr hm
          · exact Or.inl hl
        · exact Or.inr hl
      · split at h
        · cases h
        · rename_i sc _
          apply ih (sc.dependentLibraries ++ rest) (m :: acc) r h l
          rcases hl with hl | hl
          · rcases List.mem_cons.mp hl with rfl | hl
            · exact Or.inr (by simp)
            · exact Or.inl (List.mem_append_right _ hl)
          · exact Or.inr (List.mem_cons_of_mem _ hl)

def LibraryDepsCovered (scopes : List LibraryScope) (acc pending : List LibraryName) : Prop :=
  ∀ l ∈ acc, ∀ sc, findLibraryScope scopes l = some sc →
    ∀ d ∈ sc.dependentLibraries, d ∈ acc ∨ d ∈ pending

theorem libraryClosure_closed {scopes : List LibraryScope} :
    ∀ (fuel : Nat) (pending acc r : List LibraryName),
      libraryClosure scopes fuel pending acc = .ok r →
      LibraryDepsCovered scopes acc pending →
      ∀ l ∈ r, ∀ sc, findLibraryScope scopes l = some sc →
        ∀ d ∈ sc.dependentLibraries, d ∈ r := by
  intro fuel
  induction fuel with
  | zero => intro pending acc r h; simp [libraryClosure] at h
  | succ f ih =>
    intro pending acc r h inv
    cases pending with
    | nil =>
      simp only [libraryClosure] at h
      cases h
      intro l hl sc hsc d hd
      rcases inv l hl sc hsc d hd with hd | hd
      · exact hd
      · simp at hd
    | cons m rest =>
      simp only [libraryClosure] at h
      split at h
      · rename_i hm
        apply ih rest acc r h
        intro l hl sc hsc d hd
        rcases inv l hl sc hsc d hd with hd | hd
        · exact Or.inl hd
        · rcases List.mem_cons.mp hd with rfl | hd
          · exact Or.inl hm
          · exact Or.inr hd
      · split at h
        · cases h
        · rename_i scm hscm
          apply ih (scm.dependentLibraries ++ rest) (m :: acc) r h
          intro l hl sc hsc d hd
          rcases List.mem_cons.mp hl with rfl | hl
          · rw [hscm] at hsc
            cases hsc
            exact Or.inr (List.mem_append_left _ hd)
          · rcases inv l hl sc hsc d hd with hd | hd
            · exact Or.inl (List.mem_cons_of_mem _ hd)
            · rcases List.mem_cons.mp hd with rfl | hd
              · exact Or.inl (by simp)
              · exact Or.inr (List.mem_append_right _ hd)

def KnownLibraries (scopes : List LibraryScope) (acc : List LibraryName) : Prop :=
  ∀ l ∈ acc, ∃ sc, findLibraryScope scopes l = some sc

theorem libraryClosure_known {scopes : List LibraryScope} :
    ∀ (fuel : Nat) (pending acc r : List LibraryName),
      libraryClosure scopes fuel pending acc = .ok r →
      KnownLibraries scopes acc → KnownLibraries scopes r := by
  intro fuel
  induction fuel with
  | zero => intro pending acc r h; simp [libraryClosure] at h
  | succ f ih =>
    intro pending acc r h known
    cases pending with
    | nil =>
      simp only [libraryClosure] at h
      cases h
      exact known
    | cons m rest =>
      simp only [libraryClosure] at h
      split at h
      · exact ih rest acc r h known
      · split at h
        · cases h
        · rename_i scm hscm
          apply ih (scm.dependentLibraries ++ rest) (m :: acc) r h
          intro l hl
          rcases List.mem_cons.mp hl with rfl | hl
          · exact ⟨scm, hscm⟩
          · exact known l hl

theorem libraryClosure_unknown_refused {scopes : List LibraryScope} {fuel : Nat}
    {l : LibraryName} {rest acc : List LibraryName}
    (hl : l ∉ acc) (hsc : findLibraryScope scopes l = none) :
    libraryClosure scopes (fuel + 1) (l :: rest) acc = .error .unknownProvider := by
  simp [libraryClosure, hl, hsc]

/-- Resolve each library in a slice's `libraryClosure` and collect its
validated provider selections. -/
def resolveSliceMembers (lp : List Crate) (scopes : List LibraryScope) (fuel : Nat)
    (guard : Bool) : List LibraryName → Except Refusal (List (Name × Module))
  | [] => .ok []
  | l :: ls =>
    match findLibraryScope scopes l with
    | none => .error .unknownProvider
    | some sc =>
      match resolve lp fuel sc.providers guard with
      | .error r => .error r
      | .ok res =>
        match resolveSliceMembers lp scopes fuel guard ls with
        | .error r => .error r
        | .ok rest => .ok (res.modules ++ rest)

def resolveLibrarySlice (lp : List Crate) (scopes : List LibraryScope) (fuel : Nat)
    (lib : LibraryName) (guard : Bool) :
    Except Refusal (List LibraryName × List (Name × Module)) :=
  match libraryClosure scopes fuel [lib] [] with
  | .error r => .error r
  | .ok libs =>
    match resolveSliceMembers lp scopes fuel guard libs with
    | .error r => .error r
    | .ok mods => .ok (libs, mods)

theorem resolveLibrarySlice_unknown_library_refused {lp : List Crate}
    {scopes : List LibraryScope} {fuel : Nat} {lib : LibraryName} {guard : Bool}
    (h : findLibraryScope scopes lib = none) :
    resolveLibrarySlice lp scopes (fuel + 1) lib guard = .error .unknownProvider := by
  simp [resolveLibrarySlice, libraryClosure, h]

theorem resolveLibrarySlice_contains_root {lp : List Crate}
    {scopes : List LibraryScope} {fuel : Nat} {lib : LibraryName} {guard : Bool}
    {libs : List LibraryName} {mods : List (Name × Module)}
    (h : resolveLibrarySlice lp scopes fuel lib guard = .ok (libs, mods)) :
    lib ∈ libs := by
  unfold resolveLibrarySlice at h
  split at h
  · cases h
  · rename_i cl hcl
    split at h
    · cases h
    · cases h
      exact libraryClosure_contains fuel [lib] [] libs hcl lib (Or.inl (by simp))

theorem resolveLibrarySlice_closed_under_dependents {lp : List Crate}
    {scopes : List LibraryScope} {fuel : Nat} {lib : LibraryName} {guard : Bool}
    {libs : List LibraryName} {mods : List (Name × Module)}
    (h : resolveLibrarySlice lp scopes fuel lib guard = .ok (libs, mods)) :
    ∀ l ∈ libs, ∀ sc, findLibraryScope scopes l = some sc →
      ∀ d ∈ sc.dependentLibraries, d ∈ libs := by
  unfold resolveLibrarySlice at h
  split at h
  · cases h
  · rename_i cl hcl
    split at h
    · cases h
    · cases h
      exact libraryClosure_closed fuel [lib] [] libs hcl (by intro l hl; simp at hl)

theorem resolveSliceMembers_sound {lp : List Crate} (hv : ViewsOwned lp)
    {scopes : List LibraryScope} {fuel : Nat} {guard : Bool} :
    ∀ {libs : List LibraryName} {mods : List (Name × Module)},
      resolveSliceMembers lp scopes fuel guard libs = .ok mods →
      ∀ p ∈ mods, Owned lp p := by
  intro libs
  induction libs with
  | nil =>
    intro mods h p hp
    simp [resolveSliceMembers] at h
    subst h
    simp at hp
  | cons l ls ih =>
    intro mods h p hp
    simp only [resolveSliceMembers] at h
    split at h
    · cases h
    · split at h
      · cases h
      · rename_i res hres
        split at h
        · cases h
        · rename_i rest hrest
          cases h
          rcases List.mem_append.mp hp with hp | hp
          · exact (resolve_sound hv hres).2.2.1 p hp
          · exact ih hrest p hp

/-- Four-valued verdict lattice (`Safe < Pending < Unknown < Unsafe`, §L21.2)
for sequential per-library slice composition. -/
inductive SliceVerdict where
  | safe
  | pending
  | unknown
  | unsafe_
  deriving DecidableEq, Repr

def joinSliceVerdict : SliceVerdict → SliceVerdict → SliceVerdict
  | .unsafe_, _ => .unsafe_
  | _, .unsafe_ => .unsafe_
  | .unknown, _ => .unknown
  | _, .unknown => .unknown
  | .pending, _ => .pending
  | _, .pending => .pending
  | .safe, .safe => .safe

def foldSlices (vs : List SliceVerdict) : SliceVerdict :=
  vs.foldl joinSliceVerdict .safe

theorem joinSliceVerdict_assoc (a b c : SliceVerdict) :
    joinSliceVerdict (joinSliceVerdict a b) c =
      joinSliceVerdict a (joinSliceVerdict b c) := by
  cases a <;> cases b <;> cases c <;> rfl

theorem joinSliceVerdict_comm (a b : SliceVerdict) :
    joinSliceVerdict a b = joinSliceVerdict b a := by
  cases a <;> cases b <;> rfl

/-- Fail-closed invariant: composing an `Unknown` slice with any other slice
never produces `Safe`. -/
theorem joinSliceVerdict_unknown_never_safe (v : SliceVerdict) :
    joinSliceVerdict .unknown v ≠ .safe ∧ joinSliceVerdict v .unknown ≠ .safe := by
  cases v <;> decide

/-- Sequential slice composition equals whole-list composition over the
concatenated slice verdicts. -/
theorem foldSlices_append (xs ys : List SliceVerdict) :
    xs.foldl joinSliceVerdict (ys.foldl joinSliceVerdict .safe) =
      (ys ++ xs).foldl joinSliceVerdict .safe := by
  rw [List.foldl_append]

end QCore.ProviderSelection

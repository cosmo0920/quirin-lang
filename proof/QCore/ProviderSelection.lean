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
  unknown crate, an unowned module, a crate with no module and an empty
  selection refuse, and a module selected twice refuses;
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
  refuses rather than being narrowed.

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
  deriving DecidableEq, Repr

/-- The crate with this name on the load path (the load path itself has
already refused duplicate names, cycles and missing dependencies). -/
def findCrate (lp : List Crate) (n : Name) : Option Crate :=
  lp.find? (fun k => k.name == n)

/-! ## 1. Expansion -/

def expand (lp : List Crate) : Entry → Except Refusal (List (Name × Module))
  | .whole c =>
    match findCrate lp c with
    | none => .error .unknownProvider
    | some k => if k.modules = [] then .error .missingModule
      else .ok (k.modules.map (fun m => (c, m)))
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

theorem expand_sound {lp : List Crate} {e : Entry} {ms : List (Name × Module)}
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
      · cases h
        rcases List.mem_map.mp hp with ⟨m, hm, rfl⟩
        exact ⟨k, hk, hm⟩
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

theorem expandAll_sound {lp : List Crate} :
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
        · exact expand_sound hfirst p hp
        · exact ih hrest p hp

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
      if mods.Nodup then
        match closure lp fuel (mods.map Prod.fst) [] with
        | .error r => .error r
        | .ok cl =>
          if guard && !quirinOnly lp cl then .error .qcoreInClosure
          else .ok ⟨mods, cl⟩
      else .error .duplicateModule

/-- The facts every admitted resolution carries. -/
theorem resolve_sound {lp : List Crate} {fuel : Nat} {entries : List Entry} {guard : Bool}
    {r : Resolved} (h : resolve lp fuel entries guard = .ok r) :
    entries ≠ [] ∧
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
      · rename_i hnodup
        split at h
        · cases h
        · rename_i cl hcl
          split at h
          · cases h
          · rename_i hguard
            cases h
            refine ⟨hne, expandAll_sound hmods, hnodup, ?_, ?_, ?_, ?_⟩
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

inductive Evidence where
  | contract
  | package
  | verifierHost
  | plugin
  | native
  | pinned
  deriving DecidableEq, Repr

inductive Mode where
  | configured
  | contractsOnly
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

end QCore.ProviderSelection

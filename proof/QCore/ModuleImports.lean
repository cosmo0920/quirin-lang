/-! Explicit bundled-module selection. This models exact name resolution and
capability admission; source token extraction and per-file driver wiring are
tested Rust boundaries, not consequences of these abstract theorems.

Card `STD-MODULES` generalises the registry from two modules to five. The
existing theorem *names* and *meanings* are preserved; their structure literals
grew three fields, which is a deliberate widening recorded in that card's
report. Nothing about the concurrency or extern capabilities changed. -/
namespace QCore.ModuleImports

structure Imports where
  concurrency : Bool := false
  /-- Card `source.extern-call.v1`: calling a declared extern function is a
  second, independent file-local opt-in. -/
  externCalls : Bool := false
  /-- Card `UNSAFE-LINKAGE`: declaring an extern function with **no** contract,
  so an external library can be linked without one. A third, independent
  file-local opt-in: `std.extern` never grants it and it never grants
  `std.extern`. -/
  uncheckedExternCalls : Bool := false
  /-- Card `STD-MODULES`: naming a rendering. Opens no stream on its own. -/
  fmt : Bool := false
  /-- Card `STD-MODULES`: writing a rendered line to standard output. -/
  io : Bool := false
  /-- Card `STD-MODULES`: writing a level-tagged line to standard error, and
  the `flush` sink-handoff barrier. -/
  logger : Bool := false
  deriving DecidableEq, Repr

/-- The empty selection: every capability withheld. -/
def none : Imports := ⟨false, false, false, false, false, false⟩

/-- The field one module name sets. Total-or-reject: a name outside the
registry has no setter, which is what makes `resolve` refuse it. -/
def setterFor (name : String) : Option (Imports → Imports) :=
  if name = "std.concurrency" then some (fun i => { i with concurrency := true })
  else if name = "std.extern" then some (fun i => { i with externCalls := true })
  else if name = "std.extern.unsafe" then
    some (fun i => { i with uncheckedExternCalls := true })
  else if name = "std.fmt" then some (fun i => { i with fmt := true })
  else if name = "std.io" then some (fun i => { i with io := true })
  else if name = "std.logger" then some (fun i => { i with logger := true })
  else Option.none

def resolve : List String → Option Imports
  | [] => some none
  | name :: rest =>
    match setterFor name with
    | some set => (resolve rest).map set
    | Option.none => Option.none

def admits (imports : Imports) (needsConcurrency : Bool) : Bool :=
  !needsConcurrency || imports.concurrency

/-- The extern-call capability is gated by its own module, never by another. -/
def admitsExternCall (imports : Imports) (callsExtern : Bool) : Bool :=
  !callsExtern || imports.externCalls

/-- Card `UNSAFE-LINKAGE`: declaring an extern function with no contract is
gated by its own module, never by the call module. The gate is on the
*declaration*, because declaring an uncontracted symbol is itself the trust
act. -/
def admitsUncheckedExtern (imports : Imports) (declaresUnchecked : Bool) : Bool :=
  !declaresUnchecked || imports.uncheckedExternCalls

/-- Writing to standard output is gated by `std.io`, never by another module. -/
def admitsPrint (imports : Imports) (prints : Bool) : Bool :=
  !prints || imports.io

/-- Logging and the barrier are gated by `std.logger`, never by another. -/
def admitsLog (imports : Imports) (logs : Bool) : Bool :=
  !logs || imports.logger

/-- Naming a rendering is gated by `std.fmt`, never by another. -/
def admitsRendering (imports : Imports) (renders : Bool) : Bool :=
  !renders || imports.fmt

theorem empty_imports_grant_no_concurrency : resolve [] = some none := rfl

theorem empty_imports_grant_no_extern_calls :
    (resolve []).map Imports.externCalls = some false := rfl

theorem empty_imports_grant_no_output :
    (resolve []).map Imports.io = some false := rfl

theorem empty_imports_grant_no_logger :
    (resolve []).map Imports.logger = some false := rfl

theorem empty_imports_grant_no_rendering :
    (resolve []).map Imports.fmt = some false := rfl

theorem unknown_module_rejects (name : String) (rest : List String)
    (unknownConcurrency : name ≠ "std.concurrency")
    (unknownExtern : name ≠ "std.extern")
    (unknownUncheckedExtern : name ≠ "std.extern.unsafe")
    (unknownFmt : name ≠ "std.fmt")
    (unknownIo : name ≠ "std.io")
    (unknownLogger : name ≠ "std.logger") : resolve (name :: rest) = Option.none := by
  simp [resolve, setterFor, unknownConcurrency, unknownExtern,
    unknownUncheckedExtern, unknownFmt, unknownIo, unknownLogger]

/-- **The capability-gating theorem.** If resolution enabled a capability, the
module that owns it was named. Stated once over an arbitrary field and
instantiated five times below, so every module is gated by exactly the same
argument and none can drift. -/
theorem admitted_capability_requires_its_module
    (field : Imports → Bool) (module : String)
    (base : field none = false)
    (only : ∀ name set, setterFor name = some set → name ≠ module →
      ∀ i, field (set i) = field i)
    {names : List String} {imports : Imports}
    (resolved : resolve names = some imports) (enabled : field imports = true) :
    module ∈ names := by
  induction names generalizing imports with
  | nil =>
    simp only [resolve, Option.some.injEq] at resolved
    subst resolved
    simp [base] at enabled
  | cons name rest ih =>
    by_cases named : name = module
    · simp [named]
    · refine List.mem_cons_of_mem _ ?_
      cases setter : setterFor name with
      | none => simp [resolve, setter] at resolved
      | some set =>
        cases inner : resolve rest with
        | none => simp [resolve, setter, inner] at resolved
        | some i =>
          simp only [resolve, setter, inner, Option.map_some, Option.some.injEq] at resolved
          subst resolved
          exact ih inner (by rw [← only name set setter named i]; exact enabled)

private theorem setter_preserves_other_fields
    (field : Imports → Bool) (module : String)
    (h : ∀ i, (field { i with concurrency := true } = field i ∨ module = "std.concurrency")
      ∧ (field { i with externCalls := true } = field i ∨ module = "std.extern")
      ∧ (field { i with uncheckedExternCalls := true } = field i ∨ module = "std.extern.unsafe")
      ∧ (field { i with fmt := true } = field i ∨ module = "std.fmt")
      ∧ (field { i with io := true } = field i ∨ module = "std.io")
      ∧ (field { i with logger := true } = field i ∨ module = "std.logger")) :
    ∀ name set, setterFor name = some set → name ≠ module →
      ∀ i, field (set i) = field i := by
  intro name set setter named i
  simp only [setterFor] at setter
  split at setter
  · next eq => subst eq
               injection setter with setter; subst setter
               rcases (h i).1 with p | p
               · exact p
               · exact absurd p.symm named
  · split at setter
    · next eq => subst eq
                 injection setter with setter; subst setter
                 rcases (h i).2.1 with p | p
                 · exact p
                 · exact absurd p.symm named
    · split at setter
      · next eq => subst eq
                   injection setter with setter; subst setter
                   rcases (h i).2.2.1 with p | p
                   · exact p
                   · exact absurd p.symm named
      · split at setter
        · next eq => subst eq
                     injection setter with setter; subst setter
                     rcases (h i).2.2.2.1 with p | p
                     · exact p
                     · exact absurd p.symm named
        · split at setter
          · next eq => subst eq
                       injection setter with setter; subst setter
                       rcases (h i).2.2.2.2.1 with p | p
                       · exact p
                       · exact absurd p.symm named
          · split at setter
            · next eq => subst eq
                         injection setter with setter; subst setter
                         rcases (h i).2.2.2.2.2 with p | p
                         · exact p
                         · exact absurd p.symm named
            · simp at setter

theorem admitted_concurrency_requires_import {names : List String} {imports : Imports}
    (resolved : resolve names = some imports) (enabled : imports.concurrency = true) :
    "std.concurrency" ∈ names :=
  admitted_capability_requires_its_module Imports.concurrency "std.concurrency" rfl
    (setter_preserves_other_fields _ _ (by intro i; refine ⟨Or.inr rfl, ?_⟩; simp))
    resolved enabled

theorem admitted_extern_calls_require_the_extern_import {names : List String}
    {imports : Imports} (resolved : resolve names = some imports)
    (enabled : imports.externCalls = true) : "std.extern" ∈ names :=
  admitted_capability_requires_its_module Imports.externCalls "std.extern" rfl
    (setter_preserves_other_fields _ _
      (by intro i; refine ⟨Or.inl rfl, Or.inr rfl, ?_⟩; simp))
    resolved enabled

theorem admitted_rendering_requires_the_fmt_import {names : List String}
    {imports : Imports} (resolved : resolve names = some imports)
    (enabled : imports.fmt = true) : "std.fmt" ∈ names :=
  admitted_capability_requires_its_module Imports.fmt "std.fmt" rfl
    (setter_preserves_other_fields _ _
      (by intro i; refine ⟨Or.inl rfl, Or.inl rfl, Or.inl rfl, Or.inr rfl, ?_⟩; simp))
    resolved enabled

theorem admitted_output_requires_the_io_import {names : List String}
    {imports : Imports} (resolved : resolve names = some imports)
    (enabled : imports.io = true) : "std.io" ∈ names :=
  admitted_capability_requires_its_module Imports.io "std.io" rfl
    (setter_preserves_other_fields _ _
      (by intro i; refine ⟨Or.inl rfl, Or.inl rfl, Or.inl rfl, Or.inl rfl, Or.inr rfl, ?_⟩; simp))
    resolved enabled

theorem admitted_logging_requires_the_logger_import {names : List String}
    {imports : Imports} (resolved : resolve names = some imports)
    (enabled : imports.logger = true) : "std.logger" ∈ names :=
  admitted_capability_requires_its_module Imports.logger "std.logger" rfl
    (setter_preserves_other_fields _ _
      (by intro i
          exact ⟨Or.inl rfl, Or.inl rfl, Or.inl rfl, Or.inl rfl, Or.inl rfl, Or.inr rfl⟩))
    resolved enabled

theorem the_concurrency_import_alone_grants_no_extern_call :
    admitsExternCall ⟨true, false, false, false, false, false⟩ true = false := rfl

/-- Card `STD-MODULES`: the three new capabilities are mutually independent.
Importing standard output does not grant the logger, and vice versa. -/
theorem the_io_import_alone_grants_no_logging :
    admitsLog ⟨false, false, false, true, true, false⟩ true = false := rfl

theorem the_logger_import_alone_grants_no_output :
    admitsPrint ⟨false, false, false, true, false, true⟩ true = false := rfl

/-- `std.fmt` opens no stream: holding it alone admits neither writer. -/
theorem the_fmt_import_alone_grants_no_stream :
    admitsPrint ⟨false, false, false, true, false, false⟩ true = false
      ∧ admitsLog ⟨false, false, false, true, false, false⟩ true = false := ⟨rfl, rfl⟩

/-- Neither writer's module grants the rendering vocabulary. -/
theorem neither_writer_grants_the_rendering :
    admitsRendering ⟨false, false, false, false, true, true⟩ true = false := rfl

/-- Concurrency grants none of the three, so a threaded module gets no free
output channel. -/
theorem concurrency_grants_no_output_capability :
    admitsPrint ⟨true, false, false, false, false, false⟩ true = false
      ∧ admitsLog ⟨true, false, false, false, false, false⟩ true = false
      ∧ admitsRendering ⟨true, false, false, false, false, false⟩ true = false :=
  ⟨rfl, rfl, rfl⟩

theorem sequential_code_needs_no_extern_import (imports : Imports) :
    admitsExternCall imports false = true := by simp [admitsExternCall]

theorem code_that_prints_nothing_needs_no_io_import (imports : Imports) :
    admitsPrint imports false = true := by simp [admitsPrint]

theorem code_that_logs_nothing_needs_no_logger_import (imports : Imports) :
    admitsLog imports false = true := by simp [admitsLog]

theorem code_that_renders_nothing_needs_no_fmt_import (imports : Imports) :
    admitsRendering imports false = true := by simp [admitsRendering]

theorem duplicate_import_is_idempotent (names : List String) :
    resolve ("std.concurrency" :: "std.concurrency" :: names) =
      resolve ("std.concurrency" :: names) := by
  simp only [resolve]
  cases resolve names <;> rfl

theorem duplicate_io_import_is_idempotent (names : List String) :
    resolve ("std.io" :: "std.io" :: names) = resolve ("std.io" :: names) := by
  simp only [resolve]
  cases resolve names <;> rfl

theorem both_imports_are_independent :
    resolve ["std.concurrency", "std.extern"] =
      some ⟨true, true, false, false, false, false⟩ := by
  simp [resolve, setterFor, none]

theorem the_three_std_modules_are_independent :
    resolve ["std.fmt", "std.io", "std.logger"] =
      some ⟨false, false, false, true, true, true⟩ := by
  simp [resolve, setterFor, none]

/-- Card `UNSAFE-LINKAGE`. The waiver is granted by its own module and by no
other, and it grants no other capability in return: importing it does not let a
file call an extern function, and importing `std.extern` does not let a file
waive one. -/
theorem admitted_unchecked_linkage_requires_its_module {names : List String}
    {imports : Imports} (resolved : resolve names = some imports)
    (enabled : imports.uncheckedExternCalls = true) : "std.extern.unsafe" ∈ names :=
  admitted_capability_requires_its_module Imports.uncheckedExternCalls
    "std.extern.unsafe" rfl
    (setter_preserves_other_fields _ _
      (by intro i; refine ⟨Or.inl rfl, Or.inl rfl, Or.inr rfl, ?_⟩; simp))
    resolved enabled

theorem empty_imports_grant_no_unchecked_linkage :
    (resolve []).map Imports.uncheckedExternCalls = some false := rfl

theorem the_extern_import_alone_grants_no_waiver :
    admitsUncheckedExtern ⟨false, true, false, false, false, false⟩ true = false := rfl

theorem the_waiver_alone_grants_no_extern_call :
    admitsExternCall ⟨false, false, true, false, false, false⟩ true = false := rfl

/-- An unchecked-linkage file names both modules, and naming both grants
exactly both. -/
theorem the_two_extern_modules_are_independent :
    resolve ["std.extern", "std.extern.unsafe"] =
      some ⟨false, true, true, false, false, false⟩ := by
  simp [resolve, setterFor, none]

theorem code_that_waives_nothing_needs_no_waiver_import (imports : Imports) :
    admitsUncheckedExtern imports false = true := by simp [admitsUncheckedExtern]

/-- The waiver is not reachable through a near-miss spelling: `std.extern.unsafe`
is one registry entry, not a prefix rule over `std.extern`. -/
theorem a_near_miss_waiver_spelling_rejects :
    resolve ["std.extern.unsafe.please"] = Option.none := by
  simp [resolve, setterFor]

theorem missing_required_capability_rejects :
    admits ⟨false, false, false, false, false, false⟩ true = false := rfl

theorem sequential_code_needs_no_concurrency (imports : Imports) :
    admits imports false = true := by simp [admits]

end QCore.ModuleImports

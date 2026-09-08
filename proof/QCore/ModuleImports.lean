/-! Explicit bundled-module selection. This models exact name resolution and
capability admission; source token extraction and per-file driver wiring are
tested Rust boundaries, not consequences of these abstract theorems. -/
namespace QCore.ModuleImports

structure Imports where
  concurrency : Bool := false
  /-- Card `source.extern-call.v1`: calling a declared extern function is a
  second, independent file-local opt-in. -/
  externCalls : Bool := false
  deriving DecidableEq, Repr

def resolve : List String → Option Imports
  | [] => some ⟨false, false⟩
  | name :: rest =>
    if name = "std.concurrency" then
      (resolve rest).map (fun i => ⟨true, i.externCalls⟩)
    else if name = "std.extern" then
      (resolve rest).map (fun i => ⟨i.concurrency, true⟩)
    else none

def admits (imports : Imports) (needsConcurrency : Bool) : Bool :=
  !needsConcurrency || imports.concurrency

/-- The extern-call capability is gated by its own module, never by another. -/
def admitsExternCall (imports : Imports) (callsExtern : Bool) : Bool :=
  !callsExtern || imports.externCalls

theorem empty_imports_grant_no_concurrency : resolve [] = some ⟨false, false⟩ := rfl

theorem empty_imports_grant_no_extern_calls :
    (resolve []).map Imports.externCalls = some false := rfl

theorem unknown_module_rejects (name : String) (rest : List String)
    (unknownConcurrency : name ≠ "std.concurrency")
    (unknownExtern : name ≠ "std.extern") : resolve (name :: rest) = none := by
  simp [resolve, unknownConcurrency, unknownExtern]

theorem admitted_extern_calls_require_the_extern_import {names : List String}
    {imports : Imports} (resolved : resolve names = some imports)
    (enabled : imports.externCalls = true) : "std.extern" ∈ names := by
  induction names generalizing imports with
  | nil => simp [resolve] at resolved; subst imports; simp at enabled
  | cons name rest ih =>
    by_cases isExtern : name = "std.extern"
    · simp [isExtern]
    · by_cases isConcurrency : name = "std.concurrency"
      · simp only [resolve, isConcurrency] at resolved
        cases inner : resolve rest with
        | none => simp [inner] at resolved
        | some i =>
          simp [inner] at resolved
          subst resolved
          simp at enabled
          exact List.mem_cons_of_mem _ (ih inner enabled)
      · simp [resolve, isConcurrency, isExtern] at resolved

theorem the_concurrency_import_alone_grants_no_extern_call :
    admitsExternCall ⟨true, false⟩ true = false := rfl

theorem sequential_code_needs_no_extern_import (imports : Imports) :
    admitsExternCall imports false = true := by simp [admitsExternCall]

theorem admitted_concurrency_requires_import {names : List String} {imports : Imports}
    (resolved : resolve names = some imports) (enabled : imports.concurrency = true) :
    "std.concurrency" ∈ names := by
  induction names generalizing imports with
  | nil => simp [resolve] at resolved; subst imports; simp at enabled
  | cons name rest ih =>
    by_cases known : name = "std.concurrency"
    · simp [known]
    · by_cases isExtern : name = "std.extern"
      · simp only [resolve, isExtern] at resolved
        cases inner : resolve rest with
        | none => simp [inner] at resolved
        | some i =>
          simp [inner] at resolved
          subst resolved
          simp at enabled
          exact List.mem_cons_of_mem _ (ih inner enabled)
      · simp [resolve, known, isExtern] at resolved

theorem duplicate_import_is_idempotent (names : List String) :
    resolve ("std.concurrency" :: "std.concurrency" :: names) =
      resolve ("std.concurrency" :: names) := by
  simp only [resolve]
  cases resolve names <;> rfl

theorem both_imports_are_independent :
    resolve ["std.concurrency", "std.extern"] = some ⟨true, true⟩ := by
  simp [resolve]

theorem missing_required_capability_rejects : admits ⟨false, false⟩ true = false := rfl

theorem sequential_code_needs_no_concurrency (imports : Imports) :
    admits imports false = true := by simp [admits]

end QCore.ModuleImports

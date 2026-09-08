/-! Explicit bundled-module selection. This models exact name resolution and
capability admission; source token extraction and per-file driver wiring are
tested Rust boundaries, not consequences of these abstract theorems. -/
namespace QCore.ModuleImports

structure Imports where
  concurrency : Bool := false
  deriving DecidableEq, Repr

def resolve : List String → Option Imports
  | [] => some ⟨false⟩
  | name :: rest =>
    if name = "std.concurrency" then
      (resolve rest).map (fun _ => ⟨true⟩)
    else none

def admits (imports : Imports) (needsConcurrency : Bool) : Bool :=
  !needsConcurrency || imports.concurrency

theorem empty_imports_grant_no_concurrency : resolve [] = some ⟨false⟩ := rfl

theorem unknown_module_rejects (name : String) (rest : List String)
    (unknown : name ≠ "std.concurrency") : resolve (name :: rest) = none := by
  simp [resolve, unknown]

theorem admitted_concurrency_requires_import {names : List String} {imports : Imports}
    (resolved : resolve names = some imports) (enabled : imports.concurrency = true) :
    "std.concurrency" ∈ names := by
  cases names with
  | nil => simp [resolve] at resolved; subst imports; contradiction
  | cons name rest =>
    by_cases known : name = "std.concurrency"
    · simp [known]
    · simp [resolve, known] at resolved

theorem duplicate_import_is_idempotent (names : List String) :
    resolve ("std.concurrency" :: "std.concurrency" :: names) =
      resolve ("std.concurrency" :: names) := by
  simp only [resolve]
  cases resolve names <;> rfl

theorem missing_required_capability_rejects : admits ⟨false⟩ true = false := rfl

theorem sequential_code_needs_no_concurrency (imports : Imports) :
    admits imports false = true := by simp [admits]

end QCore.ModuleImports

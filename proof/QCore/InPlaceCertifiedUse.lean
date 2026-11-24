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

import QCore.ThreadKeyStatic

/-!
# Every in-place use certified, every in-place object one declaration

Card **CG25-IN-PLACE-REGEX** (gateway rule 74i), discharging
`CG25-IN-PLACE-REGEX-USE-CERTIFIED` and `CG25-IN-PLACE-REGEX-SHADOWED-NAME`.

CG25's in-place use (`QCore.ResourceKinds.transferI (.use …)`) checks only the
released bit, so a `regexec(&re, …)` before the status test, on the failure
edge or before any `regcomp` was admitted. Rule 11v (7) closed the same shape
for thread keys only (`QCore.ThreadKeyStatic.useK`). This module states the
generalisation the Rust makes (`report_uncertified_key_uses` over every
in-place kind; `Parser::automatic_object` for every in-place actual):

* `transferC` is CG25's `transferI` with every `.use` read as rule 11v (7)'s
  certified use, whatever the kind (`the_key_use_is_rule_11v_7`,
  `every_other_step_is_cg25`).
* Each step agrees with CG25 or adds one refusal to it
  (`each_step_agrees_or_refuses`). So a run with no refusal **is** CG25's
  run (`a_run_without_refusal_is_cg25`), and **no verdict that was not
  `Safe` becomes `Safe`** (`no_verdict_becomes_safe`).
* A `Safe` verdict used only certified objects
  (`a_safe_verdict_used_only_certified_objects`).
* An initialiser's actual is tracked only when its name has one declaration
  in the function and every reference designates it (`oneDeclaration`,
  `one_declaration_iff`, `initChecked`); a name that also designates a
  file-scope object refuses (`a_shadowed_name_is_refused`).
* Witness programs: probes R2, R3, R4 and a use before the initialiser were
  `Safe` under CG25's reading and are refused; the `rd_regex_match` shape
  without its failure-edge `regerror` stays `Safe`.

## What this is not

* Not a proof that `regcomp` returns zero exactly when it compiled the
  object: CG25's libc premise.
* Not a proof of what Clang says (trust boundary 28): which `VarDecl` and
  `DeclRefExpr` ids a name has. Those are `decls` and `refs` here, pinned by
  `crates/qcore-c-gateway/tests/in_place_regex_certified.rs`.
* Not a reading of `regerror`: no contract clause tells it from `regexec`,
  so its use on the failure edge is refused with `regexec`'s (an
  over-refusal, `CG25-IN-PLACE-REGEX-REGERROR-FAILURE-EDGE`).
-/

namespace QCore.InPlaceCertifiedUse

open QCore.KindedCustody
open QCore.ResourceKinds
open QCore.ThreadKeyStatic (certified refuse useK transferK transferI_diagnostics_grow)

/-- Rule 74i: CG25's in-place step, with every use certified. -/
def transferC (s : IState) : IInstr → IState
  | .use obj _ => useK s obj
  | i => transferI s i

def runC (s : IState) (program : List IInstr) : IState :=
  program.foldl transferC s

/-- A key's use is rule 11v (7)'s use, unchanged. -/
theorem the_key_use_is_rule_11v_7 (s : IState) (k : Nat) :
    transferC s (.use k .threadKey) = transferK s (.keyUse k) := rfl

/-- Every step but a use is CG25's. -/
theorem every_other_step_is_cg25 (s : IState) (i : IInstr)
    (h : ∀ obj k, i ≠ .use obj k) : transferC s i = transferI s i := by
  cases i with
  | use obj k => exact absurd rfl (h obj k)
  | _ => rfl

/-! ### The certified use, for every in-place kind -/

theorem a_certified_use_changes_nothing (s : IState) (obj : Nat) (k : Kind)
    (h : certified s obj = true) : transferC s (.use obj k) = s :=
  QCore.ThreadKeyStatic.a_certified_use_changes_nothing s obj h

theorem an_uncertified_use_is_refused (s : IState) (obj : Nat) (k : Kind)
    (h : certified s obj = false) : .unknown ∈ (transferC s (.use obj k)).diagnostics :=
  QCore.ThreadKeyStatic.an_uncertified_use_is_refused s obj h

theorem an_uncertified_use_is_never_safe (s : IState) (obj : Nat) (k : Kind)
    (h : certified s obj = false) : classify (transferC s (.use obj k)).diagnostics ≠ .safe :=
  a_refusal_is_never_safe _ (an_uncertified_use_is_refused s obj k h)

theorem a_use_after_finalise_is_still_a_candidate (s : IState) (obj site : Nat) (k : Kind)
    (hb : s.storage obj = some site) (hr : s.status site = some .released) :
    .useAfterClose site ∈ (transferC s (.use obj k)).diagnostics :=
  QCore.ThreadKeyStatic.a_use_after_delete_is_a_candidate s obj site hb hr

/-! ### Only refusals are added -/

/-- Each step is CG25's, or CG25's with one refusal added. -/
theorem each_step_agrees_or_refuses (s : IState) (i : IInstr) :
    transferC s i = transferI s i ∨ transferC s i = refuse (transferI s i) := by
  cases i with
  | use obj k =>
      cases hb : s.storage obj with
      | none =>
          right
          simp [transferC, useK, transferI, hb]
      | some site =>
          by_cases hr : s.status site = some .released
          · left
            simp [transferC, useK, transferI, hb, hr]
          · by_cases hc : certified s obj = true
            · left
              simp [transferC, useK, transferI, hb, hr, hc]
            · right
              simp [transferC, useK, transferI, hb, hr, hc]
  | _ => left; rfl

theorem transferC_diagnostics_grow (s : IState) (i : IInstr) :
    ∃ l, (transferC s i).diagnostics = l ++ s.diagnostics := by
  obtain ⟨l, hl⟩ := transferI_diagnostics_grow s i
  rcases each_step_agrees_or_refuses s i with h | h
  · exact ⟨l, by rw [h, hl]⟩
  · exact ⟨.unknown :: l, by rw [h]; simp [refuse, hl]⟩

theorem run_keeps_diagnostics (s : IState) (program : List IInstr) (d : Diagnostic)
    (h : d ∈ s.diagnostics) : d ∈ (runC s program).diagnostics := by
  induction program generalizing s with
  | nil => exact h
  | cons i rest ih =>
      show d ∈ (runC (transferC s i) rest).diagnostics
      apply ih
      obtain ⟨l, hl⟩ := transferC_diagnostics_grow s i
      rw [hl]
      exact List.mem_append_right _ h

/-- A run with no refusal is CG25's run. -/
theorem a_run_without_refusal_is_cg25 (s : IState) (program : List IInstr)
    (h : .unknown ∉ (runC s program).diagnostics) : runC s program = runI s program := by
  induction program generalizing s with
  | nil => rfl
  | cons i rest ih =>
      have hrest : .unknown ∉ (runC (transferC s i) rest).diagnostics := h
      rcases each_step_agrees_or_refuses s i with hs | hs
      · show runC (transferC s i) rest = runI (transferI s i) rest
        rw [hs] at hrest ⊢
        exact ih _ hrest
      · exfalso
        apply hrest
        apply run_keeps_diagnostics
        rw [hs]
        simp [refuse]

/-- **No verdict that was not `Safe` becomes `Safe`.** -/
theorem no_verdict_becomes_safe (s : IState) (program : List IInstr) (sites : List Nat)
    (h : classify (exitI (runI s program) sites) ≠ .safe) :
    classify (exitI (runC s program) sites) ≠ .safe := by
  intro hsafe
  have hu : .unknown ∉ (runC s program).diagnostics := fun hu =>
    a_refusal_is_never_safe _ (List.mem_append_right _ hu) hsafe
  rw [a_run_without_refusal_is_cg25 s program hu] at hsafe
  exact h hsafe

/-- Every use of `program`, run from `s`, met a certified object. -/
def usesCertified : IState → List IInstr → Prop
  | _, [] => True
  | s, .use obj k :: rest => certified s obj = true ∧ usesCertified (transferC s (.use obj k)) rest
  | s, i :: rest => usesCertified (transferC s i) rest

theorem a_run_without_refusal_used_only_certified_objects (s : IState) (program : List IInstr)
    (h : .unknown ∉ (runC s program).diagnostics) : usesCertified s program := by
  induction program generalizing s with
  | nil => trivial
  | cons i rest ih =>
      have hrest : .unknown ∉ (runC (transferC s i) rest).diagnostics := h
      cases i with
      | use obj k =>
          refine ⟨?_, ih _ hrest⟩
          cases hc : certified s obj
          · exact absurd (run_keeps_diagnostics _ rest _
              (an_uncertified_use_is_refused s obj k hc)) hrest
          · rfl
      | init => exact ih _ hrest
      | initUnheld => exact ih _ hrest
      | statusTest => exact ih _ hrest
      | writeStatus => exact ih _ hrest
      | finalise => exact ih _ hrest
      | dataRoute => exact ih _ hrest

/-- **A `Safe` verdict used only certified objects.** -/
theorem a_safe_verdict_used_only_certified_objects (s : IState) (program : List IInstr)
    (sites : List Nat) (h : classify (exitI (runC s program) sites) = .safe) :
    usesCertified s program := by
  apply a_run_without_refusal_used_only_certified_objects
  intro hu
  exact a_refusal_is_never_safe _ (List.mem_append_right _ hu) h

/-! ### One declaration per tracked name -/

/-- Whether a name's declarations in the function (`decls`, Clang `VarDecl`
ids) are exactly one, and every `DeclRefExpr` of the name (`refs`) designates
it: `one_automatic_object` and `every_reference_is`. -/
def oneDeclaration (decls refs : List Nat) : Bool :=
  match decls with
  | [d] => refs.all (· == d)
  | _ => false

theorem one_declaration_iff (decls refs : List Nat) :
    oneDeclaration decls refs = true ↔ ∃ d, decls = [d] ∧ ∀ r ∈ refs, r = d := by
  match decls with
  | [] => simp [oneDeclaration]
  | [d] => simp [oneDeclaration]
  | _ :: _ :: _ => simp [oneDeclaration]

/-- Rule 74i: `st = init(&obj, …)`, tracked only for one declaration. -/
def initChecked (s : IState) (decls refs : List Nat) (obj st site : Nat) : IState :=
  if oneDeclaration decls refs then transferI s (.init obj st site) else refuse s

theorem one_declaration_is_cg25 (s : IState) (decls refs : List Nat) (obj st site : Nat)
    (h : oneDeclaration decls refs = true) :
    initChecked s decls refs obj st site = transferI s (.init obj st site) := by
  simp [initChecked, h]

/-- A reference that designates another declaration refuses the initialiser,
and nothing is tracked. -/
theorem a_shadowed_name_is_refused (s : IState) (d r : Nat) (refs : List Nat) (obj st site : Nat)
    (hr : r ∈ refs) (hne : r ≠ d) :
    initChecked s [d] refs obj st site = refuse s
      ∧ .unknown ∈ (initChecked s [d] refs obj st site).diagnostics := by
  have h : oneDeclaration [d] refs = false := by
    cases hb : oneDeclaration [d] refs
    · rfl
    · obtain ⟨d', hd, hall⟩ := (one_declaration_iff [d] refs).mp hb
      cases hd
      exact absurd (hall r hr) hne
  refine ⟨by simp [initChecked, h], ?_⟩
  simp [initChecked, h, refuse]

/-! ### Witness programs (storage 0 = `re`, status 1 = `r`, site 0 a regex) -/

/-- The `rd_regex_match` control, success path: `Safe` under rule 74i. -/
theorem the_control_success_path_is_safe :
    classify (exitI (runC startI [.init 0 1 0, .statusTest 1 true, .use 0 .regex,
      .writeStatus 1, .finalise 0 .regex]) [0]) = .safe := by
  decide

/-- The control's failure path owes nothing. -/
theorem the_control_failure_path_is_safe :
    exitI (runC startI [.init 0 1 0, .statusTest 1 false]) [0] = [] := by
  decide

/-- Probe R4, a use before the status test: `Safe` under CG25, refused now. -/
theorem probe_r4_was_safe_and_is_refused :
    classify (exitI (runI startI [.init 0 1 0, .use 0 .regex, .statusTest 1 true,
      .finalise 0 .regex]) [0]) = .safe
      ∧ classify (exitI (runC startI [.init 0 1 0, .use 0 .regex, .statusTest 1 true,
      .finalise 0 .regex]) [0]) ≠ .safe := by
  decide

/-- Probe R3, a use on the failure edge: `Safe` under CG25, refused now. -/
theorem probe_r3_was_safe_and_is_refused :
    classify (exitI (runI startI [.init 0 1 0, .statusTest 1 false, .use 0 .regex]) [0]) = .safe
      ∧ classify (exitI (runC startI [.init 0 1 0, .statusTest 1 false,
      .use 0 .regex]) [0]) ≠ .safe := by
  decide

/-- A use before any initialiser: `Safe` under CG25, refused now. -/
theorem a_use_before_the_initialiser_was_safe_and_is_refused :
    classify (exitI (runI startI [.use 0 .regex, .init 0 1 0, .statusTest 1 true,
      .finalise 0 .regex]) [0]) = .safe
      ∧ classify (exitI (runC startI [.use 0 .regex, .init 0 1 0, .statusTest 1 true,
      .finalise 0 .regex]) [0]) ≠ .safe := by
  decide

/-- A use after `regfree` stays a candidate. -/
theorem a_use_after_regfree_is_a_candidate :
    .useAfterClose 0 ∈ (runC startI [.init 0 1 0, .statusTest 1 true, .finalise 0 .regex,
      .use 0 .regex]).diagnostics := by
  decide

/-- Probe R2: `{ regex_t g; r = regcomp(&g, …); if (r) return 0; } regfree(&g);`
with `g` also a file-scope object. One name, so CG25 read the `regfree` as the
local's finaliser and the program as `Safe` on both edges. The local `g` is
declaration 10; the `regfree` after the block designates the file-scope
declaration 20; rule 74i refuses the initialiser. -/
theorem probe_r2_was_safe_and_is_refused :
    classify (exitI (runI startI [.init 0 1 0, .statusTest 1 true, .finalise 0 .regex]) [0])
        = .safe
      ∧ exitI (runI startI [.init 0 1 0, .statusTest 1 false]) [0] = []
      ∧ classify (initChecked startI [10] [10, 20] 0 1 0).diagnostics ≠ .safe := by
  decide

end QCore.InPlaceCertifiedUse

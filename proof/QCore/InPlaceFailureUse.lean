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

import QCore.InPlaceCertifiedUse

/-!
# A use defined on a possibly failed in-place object

Card **CG25-IN-PLACE-REGEX-REGERROR-FAILURE-EDGE** (gateway rule 74n;
contract spec §C04.6.1, `resource_kind_failure_use(p, KIND)`).

Rule 74i (`QCore.InPlaceCertifiedUse.transferC`) refuses every use of a
tracked in-place object its initialiser status has not certified. POSIX
defines `regerror` on the `regex_t` of the `regcomp` that returned `errcode`,
and leaves `regexec` on it undefined; the contract language could not tell
them apart. The failure-use clause does: a use that carries it
(`FInstr.failureUse`) is also admitted where `admitted` holds, and every
other step is rule 74i's.

* `admitted s obj` holds exactly when the object is bound to a site of its
  latest initialiser that is live or gone by the nonzero edge, never released
  (`admitted_iff`). Read on positions:
  * before the status test (`admitted_before_the_status_test`) and on the
    failure edge (`admitted_on_the_failure_edge`): admitted;
  * before any initialiser (`refused_before_the_initialiser`): refused;
  * after the finaliser (`not_admitted_after_the_finaliser`): rule 74i's
    `use_after_close` candidate, unchanged;
  * a use without the clause (`regexec`), wherever it stands, is rule 74i's
    (`a_use_without_the_clause_is_rule_74i`), so on the failure edge it is
    refused (`a_plain_use_on_the_failure_edge_is_refused`).
* Each step is rule 74i's step, or a failure use at an admitted state that
  changes nothing (`each_step_is_74i_or_an_admitted_failure_use`). A program
  without the clause runs as under rule 74i
  (`a_program_without_the_clause_is_rule_74i`), so **only the clause can turn
  a rule 74i verdict `Safe`**, and only by its admitted uses
  (`a_new_safe_verdict_rests_on_an_admitted_failure_use`).
* A `Safe` verdict used every clause-less use on a certified object and every
  failure use on an admitted one (`a_safe_verdict_used_only_admitted_objects`).
* Witness programs: `rd_regex_match`'s failure path (`regerror` on the
  nonzero edge) is refused under rule 74i and `Safe` now; `regexec` there,
  `regerror` before `regcomp`, after `regfree`, and after a `regfree` on the
  failure edge, are not `Safe`.

## What this is not

* Not a proof of POSIX: that `regerror` is defined on the object of a failed
  `regcomp` and before the status test is the declared assumption the clause
  states (trust boundary 28), cited in `regex.qv` and the card report.
* Not a model of the Rust state: there the failure edge is a definitely-null
  binding (`targets` empty, `may_be_null`) and "live" is the `LIVE` bit;
  here they are `status site = none` with `storage obj = some site`, and
  `status site = some .live`. `crates/qcore-c-gateway/tests/
  in_place_regex_failure_use.rs` pins the correspondence.
-/

namespace QCore.InPlaceFailureUse

open QCore.KindedCustody
open QCore.ResourceKinds
open QCore.ThreadKeyStatic (certified refuse useK)
open QCore.InPlaceCertifiedUse (transferC runC)

/-- One step: a rule 74i instruction, or a use whose position carries
`resource_kind_failure_use`. -/
inductive FInstr where
  | step (i : IInstr)
  | failureUse (obj : Nat) (k : Kind)

/-- Rule 74n: the object is bound to its latest initialiser's site, which is
not released (live, possibly failed, or gone by the nonzero edge). -/
def admitted (s : IState) (obj : Nat) : Bool :=
  match s.storage obj with
  | some site => !(decide (s.status site = some .released))
  | none => false

def transferF (s : IState) : FInstr → IState
  | .step i => transferC s i
  | .failureUse obj k => if admitted s obj then s else transferC s (.use obj k)

def runF (s : IState) (program : List FInstr) : IState :=
  program.foldl transferF s

/-- The rule 74i reading of a program: the clause erased. -/
def erase : FInstr → IInstr
  | .step i => i
  | .failureUse obj k => .use obj k

/-! ### Exactly the stated positions -/

theorem admitted_iff (s : IState) (obj : Nat) :
    admitted s obj = true ↔
      ∃ site, s.storage obj = some site ∧ (s.status site = some .live ∨ s.status site = none) := by
  unfold admitted
  cases hb : s.storage obj with
  | none => simp
  | some site =>
      cases hs : s.status site with
      | none => simp [hs]
      | some st => cases st <;> simp [hs]

/-- A certified object is admitted: the clause only widens rule 74i. -/
theorem certified_is_admitted (s : IState) (obj : Nat) (h : certified s obj = true) :
    admitted s obj = true := by
  unfold certified at h
  unfold admitted
  cases hb : s.storage obj with
  | none => simp [hb] at h
  | some site =>
      simp [hb] at h
      simp [h.1]

/-- Before the status test: the initialiser's site is live and may have
failed. Admitted. -/
theorem admitted_before_the_status_test (s : IState) (obj st site : Nat) :
    admitted (transferI s (.init obj st site)) obj = true := by
  simp [admitted, transferI, update]

/-- On the failure edge: the nonzero edge removed the site. Admitted. -/
theorem admitted_on_the_failure_edge (s : IState) (obj st site : Nat) :
    admitted (transferI (transferI s (.init obj st site)) (.statusTest st false)) obj = true := by
  simp [admitted, transferI, update]

/-- On the zero edge the object is certified, as under rule 74i. -/
theorem certified_on_the_zero_edge (s : IState) (obj st site : Nat) :
    certified (transferI (transferI s (.init obj st site)) (.statusTest st true)) obj = true := by
  simp [certified, transferI, update]

/-- Before any initialiser the object is bound to nothing: refused. -/
theorem refused_before_the_initialiser (s : IState) (obj : Nat) (k : Kind)
    (h : s.storage obj = none) :
    transferF s (.failureUse obj k) = refuse s
      ∧ .unknown ∈ (transferF s (.failureUse obj k)).diagnostics := by
  have ha : admitted s obj = false := by simp [admitted, h]
  refine ⟨?_, ?_⟩
  · simp [transferF, ha, transferC, useK, h]
  · simp [transferF, ha, transferC, useK, h, refuse]

/-- After the finaliser the clause admits nothing: the use is rule 74i's,
the `use_after_close` candidate. -/
theorem not_admitted_after_the_finaliser (s : IState) (obj site : Nat) (k : Kind)
    (hb : s.storage obj = some site) (hr : s.status site = some .released) :
    admitted s obj = false
      ∧ transferF s (.failureUse obj k) = transferC s (.use obj k)
      ∧ .useAfterClose site ∈ (transferF s (.failureUse obj k)).diagnostics := by
  have ha : admitted s obj = false := by simp [admitted, hb, hr]
  have he : transferF s (.failureUse obj k) = transferC s (.use obj k) := by
    simp [transferF, ha]
  refine ⟨ha, he, ?_⟩
  rw [he]
  exact QCore.InPlaceCertifiedUse.a_use_after_finalise_is_still_a_candidate s obj site k hb hr

/-- A use without the clause is rule 74i's use, wherever it stands. -/
theorem a_use_without_the_clause_is_rule_74i (s : IState) (obj : Nat) (k : Kind) :
    transferF s (.step (.use obj k)) = transferC s (.use obj k) := rfl

/-- So `regexec` on the failure edge is refused. -/
theorem a_plain_use_on_the_failure_edge_is_refused (s : IState) (obj st site : Nat) (k : Kind) :
    .unknown ∈ (transferF (transferI (transferI s (.init obj st site)) (.statusTest st false))
      (.step (.use obj k))).diagnostics := by
  have hc : certified (transferI (transferI s (.init obj st site)) (.statusTest st false)) obj
      = false := by
    simp [certified, transferI, update]
  exact QCore.InPlaceCertifiedUse.an_uncertified_use_is_refused _ obj k hc

/-! ### Only admitted failure uses differ from rule 74i -/

theorem each_step_is_74i_or_an_admitted_failure_use (s : IState) (i : FInstr) :
    transferF s i = transferC s (erase i)
      ∨ ∃ obj k, i = .failureUse obj k ∧ admitted s obj = true ∧ transferF s i = s := by
  cases i with
  | step i => left; rfl
  | failureUse obj k =>
      cases ha : admitted s obj
      · left; simp [transferF, ha, erase]
      · right; exact ⟨obj, k, rfl, ha, by simp [transferF, ha]⟩

/-- Whether a program carries the clause at all. -/
def hasFailureUse : List FInstr → Bool
  | [] => false
  | .failureUse _ _ :: _ => true
  | .step _ :: rest => hasFailureUse rest

theorem a_program_without_the_clause_is_rule_74i (s : IState) (program : List FInstr)
    (h : hasFailureUse program = false) : runF s program = runC s (program.map erase) := by
  induction program generalizing s with
  | nil => rfl
  | cons i rest ih =>
      cases i with
      | failureUse obj k => simp [hasFailureUse] at h
      | step i =>
          show runF (transferF s (.step i)) rest = runC (transferC s i) (rest.map erase)
          exact ih _ h

/-- **Only the clause turns a rule 74i verdict `Safe`.** A program whose
reading under rule 74i is not `Safe` but is `Safe` now carries a failure
use. -/
theorem a_new_safe_verdict_rests_on_an_admitted_failure_use (s : IState) (program : List FInstr)
    (sites : List Nat)
    (hc : classify (exitI (runC s (program.map erase)) sites) ≠ .safe)
    (hf : classify (exitI (runF s program) sites) = .safe) :
    hasFailureUse program = true := by
  cases h : hasFailureUse program
  · rw [a_program_without_the_clause_is_rule_74i s program h] at hf
    exact absurd hf hc
  · rfl

theorem transferF_diagnostics_grow (s : IState) (i : FInstr) :
    ∃ l, (transferF s i).diagnostics = l ++ s.diagnostics := by
  rcases each_step_is_74i_or_an_admitted_failure_use s i with h | ⟨_, _, _, _, h⟩
  · rw [h]; exact QCore.InPlaceCertifiedUse.transferC_diagnostics_grow s (erase i)
  · exact ⟨[], by rw [h]; rfl⟩

theorem runF_keeps_diagnostics (s : IState) (program : List FInstr) (d : Diagnostic)
    (h : d ∈ s.diagnostics) : d ∈ (runF s program).diagnostics := by
  induction program generalizing s with
  | nil => exact h
  | cons i rest ih =>
      show d ∈ (runF (transferF s i) rest).diagnostics
      apply ih
      obtain ⟨l, hl⟩ := transferF_diagnostics_grow s i
      rw [hl]
      exact List.mem_append_right _ h

/-- Every clause-less use met a certified object and every failure use an
admitted one. -/
def usesAdmitted : IState → List FInstr → Prop
  | _, [] => True
  | s, .step (.use obj k) :: rest =>
      certified s obj = true ∧ usesAdmitted (transferF s (.step (.use obj k))) rest
  | s, .failureUse obj k :: rest =>
      admitted s obj = true ∧ usesAdmitted (transferF s (.failureUse obj k)) rest
  | s, i :: rest => usesAdmitted (transferF s i) rest

theorem a_run_without_refusal_used_only_admitted_objects (s : IState) (program : List FInstr)
    (h : .unknown ∉ (runF s program).diagnostics) : usesAdmitted s program := by
  induction program generalizing s with
  | nil => trivial
  | cons i rest ih =>
      have hrest : .unknown ∉ (runF (transferF s i) rest).diagnostics := h
      cases i with
      | failureUse obj k =>
          refine ⟨?_, ih _ hrest⟩
          cases ha : admitted s obj
          · exfalso
            apply hrest
            apply runF_keeps_diagnostics
            have hc : certified s obj = false := by
              cases hc : certified s obj
              · rfl
              · rw [certified_is_admitted s obj hc] at ha; cases ha
            have he : transferF s (.failureUse obj k) = transferC s (.use obj k) := by
              simp [transferF, ha]
            rw [he]
            exact QCore.InPlaceCertifiedUse.an_uncertified_use_is_refused s obj k hc
          · rfl
      | step i =>
          cases i with
          | use obj k =>
              refine ⟨?_, ih _ hrest⟩
              cases hc : certified s obj
              · exact absurd (runF_keeps_diagnostics _ rest _
                  (QCore.InPlaceCertifiedUse.an_uncertified_use_is_refused s obj k hc)) hrest
              · rfl
          | init => exact ih _ hrest
          | initUnheld => exact ih _ hrest
          | statusTest => exact ih _ hrest
          | writeStatus => exact ih _ hrest
          | finalise => exact ih _ hrest
          | dataRoute => exact ih _ hrest

/-- **A `Safe` verdict used only admitted objects.** -/
theorem a_safe_verdict_used_only_admitted_objects (s : IState) (program : List FInstr)
    (sites : List Nat) (h : classify (exitI (runF s program) sites) = .safe) :
    usesAdmitted s program := by
  apply a_run_without_refusal_used_only_admitted_objects
  intro hu
  exact a_refusal_is_never_safe _ (List.mem_append_right _ hu) h

/-! ### Witness programs (storage 0 = `re`, status 1 = `r`, site 0 a regex) -/

/-- `rd_regex_match`'s failure path, `if (r) { regerror(r, &re, …); return 0; }`:
refused under rule 74i, `Safe` with the clause. -/
theorem the_regex_match_failure_path_was_refused_and_is_safe :
    classify (exitI (runC startI [.init 0 1 0, .statusTest 1 false, .use 0 .regex]) [0])
        ≠ .safe
      ∧ classify (exitI (runF startI [.step (.init 0 1 0), .step (.statusTest 1 false),
        .failureUse 0 .regex]) [0]) = .safe := by
  decide

/-- Its success path is unchanged and `Safe`. -/
theorem the_regex_match_success_path_is_safe :
    classify (exitI (runF startI [.step (.init 0 1 0), .step (.statusTest 1 true),
      .step (.use 0 .regex), .step (.writeStatus 1), .step (.finalise 0 .regex)]) [0]) = .safe := by
  decide

/-- `regerror` before the status test, then the success path: `Safe`. -/
theorem a_regerror_before_the_test_is_safe :
    classify (exitI (runF startI [.step (.init 0 1 0), .failureUse 0 .regex,
      .step (.statusTest 1 true), .step (.finalise 0 .regex)]) [0]) = .safe := by
  decide

/-- `regexec` on the failure edge: not `Safe`. -/
theorem a_regexec_on_the_failure_edge_is_not_safe :
    classify (exitI (runF startI [.step (.init 0 1 0), .step (.statusTest 1 false),
      .step (.use 0 .regex)]) [0]) ≠ .safe := by
  decide

/-- `regerror` before any `regcomp`: not `Safe`. -/
theorem a_regerror_before_the_initialiser_is_not_safe :
    classify (exitI (runF startI [.failureUse 0 .regex, .step (.init 0 1 0),
      .step (.statusTest 1 true), .step (.finalise 0 .regex)]) [0]) ≠ .safe := by
  decide

/-- `regerror` after `regfree`: the `use_after_close` candidate. -/
theorem a_regerror_after_regfree_is_a_candidate :
    .useAfterClose 0 ∈ (runF startI [.step (.init 0 1 0), .step (.statusTest 1 true),
      .step (.finalise 0 .regex), .failureUse 0 .regex]).diagnostics := by
  decide

/-- A `regfree` on the failure edge refuses, so a `regerror` after it is not
`Safe` either. -/
theorem a_regerror_after_a_failure_edge_regfree_is_not_safe :
    classify (exitI (runF startI [.step (.init 0 1 0), .step (.statusTest 1 false),
      .step (.finalise 0 .regex), .failureUse 0 .regex]) [0]) ≠ .safe := by
  decide

end QCore.InPlaceFailureUse

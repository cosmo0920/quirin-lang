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

namespace QCore.NoBuildScope

/-! Scope of contracts-only verdicts published without build settings (card
W44-CONTRACTS-ONLY-NO-BUILD).

`quirin-audit contracts` (`qcore_audit::no_build`) preprocesses each selected
unit with an *assumed* configuration — the user's `-I`/`-D`/`--std` plus the
stated defaults — instead of a compilation database's command. This module
models the publication rule that makes those verdicts sound to publish:

* every published verdict carries the assumption set it was computed under as
  its scope, never a build configuration (`publish_scope`,
  `publish_never_configured`);
* a unit that does not prepare under the assumptions is `Unknown` with its
  reason, and no unit is dropped (`unprepared_unknown`, `run_length`,
  `run_reasons`);
* `Safe` is `Safe` relative to the assumptions only: it is exactly the
  analysis of the text those assumptions produce (`safe_relative`);
* admission binds scopes: a certificate published under assumptions is never
  admitted for a build-configured compile task, and scoped verdicts are
  comparable only within one scope (`noBuild_never_admitted_as_configured`,
  `noBuild_not_comparable_with_configured`);
* the rule is necessary: there is a world where the same source is `Safe`
  under the assumptions and `Unsafe` under its build configuration, because
  the two select different text (`favourable_direction_exists`); and it is
  exact where the text agrees: equal preprocessed text gives equal verdicts
  (`same_text_same_verdict`), so any verdict difference between the two
  routes is attributable to a text difference or a preparation refusal
  (`verdict_difference_needs_text_difference`).

Sources, texts, settings and build configurations are interned as natural
numbers. The Rust route realises the admission binding by making the recorded
assumption set each compile task's `source_profile` dependency, which every
certificate trust context digests. Correspondence with the Rust
implementation is maintained by review; it is not an extraction theorem
(constraint 28). -/

abbrev Source := Nat
abbrev Text := Nat
abbrev BuildConfig := Nat

/-- An assumed configuration: dialect, include directories, definitions. -/
structure Assumptions where
  std : Nat
  includes : List Nat
  defines : List Nat
  deriving DecidableEq, Repr

inductive Verdict where
  | safe
  | provisionalUnsafe
  | unknown
  deriving DecidableEq, Repr

/-- The reason a unit did not prepare (the route's reason codes). -/
inductive Reason where
  | missingInclude
  | unresolvedConfiguration
  | preprocessingError
  | syntaxError
  | other
  deriving DecidableEq, Repr

/-- The scope a verdict is published under. -/
inductive Scope where
  | configured (build : BuildConfig)
  | assumed (assumptions : Assumptions)
  deriving DecidableEq, Repr

structure Published where
  verdict : Verdict
  scope : Scope
  reason : Option Reason
  deriving DecidableEq, Repr

/-- Preparation of a source under settings, and the (deterministic) analysis
of a prepared text. The build route prepares under a compilation database's
configuration; the no-build route under assumptions. -/
structure World where
  prepareAssumed : Assumptions → Source → Except Reason Text
  prepareConfigured : BuildConfig → Source → Except Reason Text
  analyse : Text → Verdict

/-- The no-build route's publication of one unit. -/
def publish (w : World) (a : Assumptions) (s : Source) : Published :=
  match w.prepareAssumed a s with
  | .error r => ⟨.unknown, .assumed a, some r⟩
  | .ok t => ⟨w.analyse t, .assumed a, none⟩

/-- The build-configured route's publication of one unit. -/
def publishConfigured (w : World) (b : BuildConfig) (s : Source) : Published :=
  match w.prepareConfigured b s with
  | .error r => ⟨.unknown, .configured b, some r⟩
  | .ok t => ⟨w.analyse t, .configured b, none⟩

/-- A run publishes every selected unit, in selection order. -/
def run (w : World) (a : Assumptions) (units : List Source) : List Published :=
  units.map (publish w a)

theorem publish_scope (w : World) (a : Assumptions) (s : Source) :
    (publish w a s).scope = .assumed a := by
  unfold publish
  split <;> rfl

theorem publish_never_configured (w : World) (a : Assumptions) (s : Source) (b : BuildConfig) :
    (publish w a s).scope ≠ .configured b := by
  rw [publish_scope]
  intro h
  cases h

theorem unprepared_unknown {w : World} {a : Assumptions} {s : Source} {r : Reason}
    (h : w.prepareAssumed a s = .error r) :
    publish w a s = ⟨.unknown, .assumed a, some r⟩ := by
  unfold publish
  rw [h]

/-- A reason is published exactly for a unit that did not prepare. -/
theorem reason_iff_unprepared (w : World) (a : Assumptions) (s : Source) :
    (publish w a s).reason.isSome = true ↔ ∃ r, w.prepareAssumed a s = .error r := by
  unfold publish
  split
  · rename_i r h
    exact ⟨fun _ => ⟨r, h⟩, fun _ => rfl⟩
  · rename_i t h
    constructor
    · intro hs
      cases hs
    · rintro ⟨r, hr⟩
      rw [h] at hr
      cases hr

/-- `Safe` relative to the assumptions: the verdict is the analysis of the
text these assumptions prepare, and of no other. -/
theorem safe_relative {w : World} {a : Assumptions} {s : Source}
    (h : (publish w a s).verdict = .safe) :
    ∃ t, w.prepareAssumed a s = .ok t ∧ w.analyse t = .safe := by
  unfold publish at h
  split at h
  · cases h
  · rename_i t ht
    exact ⟨t, ht, h⟩

theorem run_length (w : World) (a : Assumptions) (units : List Source) :
    (run w a units).length = units.length := by
  simp [run]

/-- Every unit of a run is published under the run's assumptions, and one
that did not prepare is `Unknown` with its reason: nothing is dropped. -/
theorem run_reasons {w : World} {a : Assumptions} {units : List Source} {i : Nat}
    (hi : i < units.length) {r : Reason}
    (h : w.prepareAssumed a (units.get ⟨i, hi⟩) = .error r) :
    (run w a units).get ⟨i, by simpa [run_length] using hi⟩ = ⟨.unknown, .assumed a, some r⟩ := by
  simp only [run, List.get_eq_getElem, List.getElem_map]
  exact unprepared_unknown (by simpa using h)

/-- Admission of a published verdict's certificate for a compile task whose
trust context is `context`: the scopes must be identical. -/
def admit (context : Scope) (p : Published) : Bool :=
  p.verdict == .safe && p.scope == context

theorem noBuild_never_admitted_as_configured (w : World) (a : Assumptions) (s : Source)
    (b : BuildConfig) : admit (.configured b) (publish w a s) = false := by
  unfold admit
  rw [publish_scope]
  simp

/-- A no-build certificate is admitted only for its own assumption set. -/
theorem noBuild_admitted_only_under_own_assumptions {w : World} {a a' : Assumptions}
    {s : Source} (h : admit (.assumed a') (publish w a s) = true) : a' = a := by
  unfold admit at h
  rw [publish_scope] at h
  simp at h
  exact h.2.symm

/-- Two verdicts are comparable only within one scope. -/
def Comparable (p q : Published) : Prop := p.scope = q.scope

theorem noBuild_not_comparable_with_configured (w : World) (a : Assumptions) (s : Source)
    (b : BuildConfig) : ¬ Comparable (publish w a s) (publishConfigured w b s) := by
  unfold Comparable publishConfigured
  rw [publish_scope]
  split <;> (intro h; cases h)

/-- The analysis is a function of the prepared text: where the two routes
prepare the same text, they publish the same verdict. -/
theorem same_text_same_verdict {w : World} {a : Assumptions} {b : BuildConfig} {s : Source}
    {t : Text} (ha : w.prepareAssumed a s = .ok t) (hb : w.prepareConfigured b s = .ok t) :
    (publish w a s).verdict = (publishConfigured w b s).verdict := by
  unfold publish publishConfigured
  rw [ha, hb]

/-- Attribution: a verdict difference between the routes needs a preparation
refusal on one side or different prepared text. -/
theorem verdict_difference_needs_text_difference {w : World} {a : Assumptions}
    {b : BuildConfig} {s : Source}
    (h : (publish w a s).verdict ≠ (publishConfigured w b s).verdict) :
    (∃ r, w.prepareAssumed a s = .error r) ∨ (∃ r, w.prepareConfigured b s = .error r) ∨
      ∃ t t', w.prepareAssumed a s = .ok t ∧ w.prepareConfigured b s = .ok t' ∧ t ≠ t' := by
  cases ha : w.prepareAssumed a s with
  | error r => exact Or.inl ⟨r, rfl⟩
  | ok t =>
    cases hb : w.prepareConfigured b s with
    | error r => exact Or.inr (Or.inl ⟨r, rfl⟩)
    | ok t' =>
      refine Or.inr (Or.inr ⟨t, t', rfl, rfl, ?_⟩)
      intro htt
      subst htt
      exact h (same_text_same_verdict ha hb)

/-- A witness world: definition `1` selects the release (text `1`, `Safe`);
the build configuration `0` does not (text `0`, `Unsafe`). -/
def hazard : World where
  prepareAssumed := fun a _ => .ok (if 1 ∈ a.defines then 1 else 0)
  prepareConfigured := fun _ _ => .ok 0
  analyse := fun t => if t = 1 then .safe else .provisionalUnsafe

/-- The favourable direction exists: the same source is `Safe` under assumed
settings and provisional `Unsafe` under its build configuration. So a
no-build `Safe` does not transfer to the build, and the scope is necessary. -/
theorem favourable_direction_exists :
    ∃ (w : World) (a : Assumptions) (b : BuildConfig) (s : Source),
      (publish w a s).verdict = .safe ∧ (publishConfigured w b s).verdict = .provisionalUnsafe := by
  refine ⟨hazard, ⟨0, [], [1]⟩, 0, 0, ?_, ?_⟩ <;> decide

/-- Nor does it transfer between assumption sets. -/
theorem no_transfer_between_assumptions :
    ∃ (w : World) (a a' : Assumptions) (s : Source),
      (publish w a s).verdict = .safe ∧ (publish w a' s).verdict = .provisionalUnsafe := by
  refine ⟨hazard, ⟨0, [], [1]⟩, ⟨0, [], []⟩, 0, ?_, ?_⟩ <;> decide

end QCore.NoBuildScope

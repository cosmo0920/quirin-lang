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

/-! Unit `E01-TAIL`, residual `E01-F1-DENOM`: a whole-unit `Safe` claim needs
the whole selected scope to have been read.

`docs/reports/remaining-e01-f1/README.md` recorded the residual: a `Safe` unit
could still carry a non-empty `not_projected`. Measured, the cause is that
`analyze_legacy_c_translation_unit_selected_partial` intersects the selected
definition set with the definitions its own body finder reports in the
preprocessed text and **silently drops the rest** — no diagnostic, no
projection failure, no row — so a unit's report can read "N source definitions,
M analysed, `Safe`" with `N > M` and nothing explaining the gap.

Hard invariant 2 decides it: projection coverage is not safety coverage, and a
whole-unit `Safe` asserts the property over the declared scope. So the `Safe`
arm of `analyze_unrestricted_c11_with_inventory` now refuses when the selected
scope was not fully read.

What is proved here:

* **the gate is exactly full coverage** — `Safe` is published if and only if
  the analysis is clean *and* every selected definition carries a row, in both
  directions;
* **it only ever refuses** — the repaired publication is `Safe` nowhere the
  original was not, so it cannot flip an `Unknown` to `Safe` (invariant 1);
* **`Unsafe` and `Unknown` are untouched** — an `Unsafe` unit still names its
  validated reachable violation and is never downgraded by an unread
  definition, and an analysis-derived `Unknown` keeps its own reason rather
  than having it overwritten by a coverage one;
* **nothing is lost when the claim is withheld** — the analysed rows survive
  verbatim in the retained partial summary, so the withheld claim costs the
  unit-level verdict and no per-function evidence;
* **the gate subsumes E01-F1's own rule** on the shape E01-F1 could see (an
  unanalyzable definition), and is *strictly stronger*: a witness exists that
  E01-F1's rule admits and this one refuses, which is the residual itself;
* **non-vacuity** — a fully covered clean unit is still `Safe`.

What is **not** proved here, and stays a Rust boundary: that the analyzer's own
body finder reports exactly the definitions it does, and that the enumeration
selects exactly the definitions it does. Those are pinned behaviourally by
`crates/qcore-c-gateway/tests/e01_denominator_coverage.rs`, whose two measured
causes — a definition the preprocessor removed, and a definition the body
finder does not read — are transcribed below as `compiledOut` and `knr`. -/
namespace QCore.DenominatorCoverage

/-- A definition name. -/
abbrev Name := String

/-- The analyzer's own status for a translation unit, as `AnalysisStatus`. -/
inductive Status where
  | safe
  /-- `AnalysisStatus::Unsafe`. Spelled `violation` because `unsafe` is a Lean
  keyword; it is the same status. -/
  | violation
  | unknown
  deriving DecidableEq, Repr

/-- What a unit publishes, as `UnrestrictedLegacyResult`. The `Unknown`
constructor carries whether its reason came from the coverage gate or from the
analysis itself, which is what lets the "reason is not overwritten" property be
stated. -/
inductive Verdict where
  | safe (checked : List Name)
  /-- `UnrestrictedLegacyResult::Unsafe`, spelled to avoid the Lean keyword. -/
  | violation (analysed : List Name)
  | unknownFromAnalysis (analysed : List Name)
  | unknownFromCoverage (analysed unread : List Name)
  deriving DecidableEq, Repr

/-- One unit as the publication site sees it: the definitions the enumeration
selected, the definitions the analyzer actually produced a row for, and the
analyzer's status over those rows. -/
structure UnitReport where
  selected : List Name
  analysed : List Name
  status : Status
  deriving DecidableEq, Repr

/-- Selected definitions carrying no analysed row. Mirrors the `unread`
computation in the `Safe` arm. -/
def unread (u : UnitReport) : List Name :=
  u.selected.filter (fun name => !u.analysed.contains name)

/-- Whether the selected scope was fully read. -/
def fullyCovered (u : UnitReport) : Bool := unread u == []

/-! ## The two publication rules -/

/-- E01-F1's publication: the analyzer's status, unmodified. -/
def publishBefore (u : UnitReport) : Verdict :=
  match u.status with
  | .safe => .safe u.analysed
  | .violation => .violation u.analysed
  | .unknown => .unknownFromAnalysis u.analysed

/-- The repair. Only the `Safe` arm is gated. -/
def publish (u : UnitReport) : Verdict :=
  match u.status with
  | .safe => if fullyCovered u then .safe u.analysed
             else .unknownFromCoverage u.analysed (unread u)
  | .violation => .violation u.analysed
  | .unknown => .unknownFromAnalysis u.analysed

/-- Whether a verdict is a published `Safe`. -/
def isSafe : Verdict → Bool
  | .safe _ => true
  | _ => false

/-- The rows a verdict retains. -/
def rows : Verdict → List Name
  | .safe checked => checked
  | .violation analysed => analysed
  | .unknownFromAnalysis analysed => analysed
  | .unknownFromCoverage analysed _ => analysed

/-! ## The gate is exactly full coverage -/

/-- `Safe` is published exactly when the analysis is clean **and** the selected
scope was fully read. Both directions, so the gate adds no refusal beyond the
one it names and withholds nothing it should publish. -/
theorem safe_is_published_exactly_when_clean_and_covered (u : UnitReport) :
    isSafe (publish u) = true ↔ (u.status = .safe ∧ fullyCovered u = true) := by
  cases h : u.status <;> simp [publish, isSafe, h] <;>
    by_cases hc : fullyCovered u <;> simp [hc]

/-- An unread definition withholds the claim, and the withheld verdict names
it. -/
theorem an_unread_definition_withholds_safe (u : UnitReport)
    (hs : u.status = .safe) (hu : fullyCovered u = false) :
    publish u = .unknownFromCoverage u.analysed (unread u) := by
  simp [publish, hs, hu]

/-! ## The repair only ever refuses -/

/-- The publication is `Safe` nowhere the original was not: the change is
strictly in the fail-closed direction and cannot flip an `Unknown` to `Safe`
(hard invariant 1). -/
theorem the_gate_only_refuses (u : UnitReport) :
    isSafe (publish u) = true → isSafe (publishBefore u) = true := by
  intro h
  cases hs : u.status <;> simp [publishBefore, isSafe, hs] <;>
    simp [publish, isSafe, hs] at h

/-- …and it is not the identity: the residual itself is the witness. A unit
whose analysis is clean but whose selected scope carries one unread definition
was published `Safe` and is now refused. -/
theorem the_gate_is_not_vacuous :
    isSafe (publishBefore (⟨["hidden", "subject"], ["subject"], .safe⟩ : UnitReport)) = true
      ∧ isSafe (publish (⟨["hidden", "subject"], ["subject"], .safe⟩ : UnitReport)) = false := by
  decide

/-! ## The other two arms are untouched -/

/-- An unread definition never retracts an `Unsafe` finding. An `Unsafe`
verdict is a statement about a function that *was* analysed; downgrading it
would discard a real finding without gaining any safety. -/
theorem an_unread_definition_does_not_retract_unsafe (u : UnitReport) (h : u.status = .violation) :
    publish u = publishBefore u := by
  simp [publish, publishBefore, h]

/-- An analysis-derived `Unknown` keeps its own reason: the coverage gate is
reached only from the `Safe` arm, so it can never overwrite an explanation the
analysis produced. -/
theorem an_analysis_unknown_keeps_its_reason (u : UnitReport) (h : u.status = .unknown) :
    publish u = .unknownFromAnalysis u.analysed := by
  simp [publish, h]

/-- Stated once over every arm: the repair changes nothing except a `Safe` over
an incompletely read scope. -/
theorem only_the_uncovered_safe_arm_moves (u : UnitReport)
    (h : ¬(u.status = .safe ∧ fullyCovered u = false)) :
    publish u = publishBefore u := by
  cases hs : u.status
  · have : fullyCovered u = true := by
      by_cases hc : fullyCovered u
      · exact hc
      · exact absurd ⟨hs, by simpa using hc⟩ h
    simp [publish, publishBefore, hs, this]
  · simp [publish, publishBefore, hs]
  · simp [publish, publishBefore, hs]

/-! ## Nothing is lost -/

/-- A withheld claim costs the unit-level verdict and no per-function evidence:
the analysed rows survive verbatim. -/
theorem a_withheld_claim_retains_every_row (u : UnitReport) :
    rows (publish u) = u.analysed := by
  cases hs : u.status <;> simp [publish, rows, hs] <;>
    by_cases hc : fullyCovered u <;> simp [hc]

/-! ## The measured causes

Transcriptions of `crates/qcore-c-gateway/tests/e01_denominator_coverage.rs`. -/

/-- A definition the enumeration's raw half kept but the preprocessor removed:
a `#if 0` body, which is not in this translation unit at all. -/
def compiledOut : UnitReport := (⟨["hidden", "subject"], ["subject"], .safe⟩ : UnitReport)

/-- A definition present in the preprocessed text that the analyzer's body
finder does not read: a K&R body. -/
def knr : UnitReport := ⟨["knr", "subject"], ["subject"], .safe⟩

/-- The control: every selected definition carries a row. -/
def covered : UnitReport := ⟨["subject"], ["subject"], .safe⟩

/-- Both measured causes are refused, and each names the definition it refused
on. -/
theorem both_measured_causes_are_refused :
    publish compiledOut = .unknownFromCoverage ["subject"] ["hidden"]
      ∧ publish knr = .unknownFromCoverage ["subject"] ["knr"] := by
  decide

/-- The control is still `Safe`, so the refusals above are attributable to the
unread definition and to nothing else. -/
theorem the_covered_control_is_still_safe :
    publish covered = .safe ["subject"] := by
  decide

/-! ## Relation to E01-F1's own rule

E01-F1 withheld `Safe` when the *inventory* was refused or carried an
`unanalyzable` row. That rule sees the K&R cause and is blind to the
compiled-out one, which is precisely why the residual survived it. -/

/-- E01-F1's `SourceDefinitionInventory::supports_unit_safe`, over the same
unit: `established` says the completeness claim stands, `unanalyzable` names
the definitions the enumeration itself knows nothing reads. -/
def supportsUnitSafe (established : Bool) (unanalyzable : List Name) : Bool :=
  established && unanalyzable == []

/-- On the K&R cause the two rules agree: E01-F1 already withheld it. -/
theorem the_rules_agree_on_the_knr_cause :
    supportsUnitSafe true ["knr"] = false ∧ isSafe (publish knr) = false := by
  decide

/-- On the compiled-out cause they do not: E01-F1's rule admits the unit — its
claim is established and nothing is unanalyzable — and the coverage gate
refuses it. This is the residual, stated as a separation. -/
theorem the_coverage_gate_is_strictly_stronger :
    supportsUnitSafe true [] = true ∧ isSafe (publish compiledOut) = false := by
  decide

/-- Neither rule subsumes the other's *admissions*: the coverage gate refuses
everything E01-F1's rule refuses on the shapes both can see, so composing them
adds no refusal beyond the union. -/
theorem the_gate_refuses_every_unanalyzable_definition (u : UnitReport) (name : Name)
    (hs : u.status = .safe) (hsel : u.selected.contains name = true)
    (hana : u.analysed.contains name = false) :
    isSafe (publish u) = false := by
  have hmem : name ∈ unread u := by
    simp only [unread, List.mem_filter]
    refine ⟨List.mem_of_elem_eq_true hsel, ?_⟩
    simp only [Bool.not_eq_true']
    exact hana
  have hne : unread u ≠ [] := by
    intro hempty
    rw [hempty] at hmem
    exact absurd hmem (List.not_mem_nil)
  have hcov : fullyCovered u = false := by
    simp only [fullyCovered, beq_eq_false_iff_ne]
    exact hne
  simp [publish, hs, hcov, isSafe]

end QCore.DenominatorCoverage

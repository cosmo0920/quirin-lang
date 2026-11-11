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

import QCore.BucketTally

/-! Card `GEN-CTREASONS` (follow-on `D02-GEN-CTREASONS`): the CT ledger's
*reasons* map, derived rather than carried.

`docs/reviews/r0_3/acceptance_matrix.json`'s
`ct_conformance.newly_decidable_not_mapped_reasons` was hand-maintained prose
that `docs/reports/remaining-d02/build_matrix.py` dropped on every regeneration
and a later step re-added by hand. Nothing derived it, so nothing could refuse a
row that had gone stale: card HYGIENE-16 found a row explaining why CT39 was not
mapped four waves after wave 8 mapped it, and this card found a second, distinct
staleness class — a row for CT11, a case the map's own companion list
(`newly_decidable_not_mapped`) does not name.

The repair is that the map **is** derivable, and this module states why:

* its keys are the ids of the companion list, which is card T00's
  newly-decidable judgment filtered by the manifest's own unmapped set;
* each value is that case's resolved label and reason — the newest CT lane
  record that measured it, else card T00's frozen archive — which is exactly the
  resolution `QCore.BucketTally` already models, so it is imported rather than
  restated;
* and the drift check between the committed map and the derivation refuses in
  **both** directions.

Keying the map to the list is what makes both staleness classes impossible: a
mapped case leaves the list (`select_drops_a_mapped_case`), and a row belonging
to no published claim is refused (`a_row_outside_the_derivation_is_reported`).

What is **not** proved here. That `scripts/ct_stage_summary.py`
(`newly_decidable_ids`, `newly_decidable_reasons`, `check_matrix_ct_reasons`) and
`docs/reports/remaining-d02/build_matrix.py` compute these functions is a
transcription premise, checked by the generator and the checker refusing each
other's output against the committed file rather than by extraction. Whether a
case is *decidable now* is card T00's analysis, which no relation derives; this
module is about publishing exactly the cases that judgment still applies to. -/
namespace QCore.CtReasons

open QCore.BucketTally (find? resolve1 resolve)

/-- A conformance-case identifier, e.g. `CT24`. -/
abbrev CaseId := String

/-- The prose a record carries for one case. -/
abbrev Reason := String

/-- Membership in a case list. -/
def mem : List CaseId → CaseId → Bool
  | [], _ => false
  | head :: rest, case => if head = case then true else mem rest case

/-- `newly_decidable_not_mapped`: card T00's newly-decidable judgment, filtered
by the manifest's own unmapped set. The filter is the structural half — a case
that gets mapped leaves this list in the same commit that maps it, with nobody
editing the list. -/
def select (newly unmapped : List CaseId) : List CaseId :=
  newly.filter (fun case => mem unmapped case)

/-- `newly_decidable_not_mapped_reasons`: one row per id of the list above,
carrying that case's resolved reason. Total-or-refuse, inherited from
`QCore.BucketTally.resolve`: a case neither a lane nor the archive classifies is
a hole, not an empty paragraph. -/
def reasons (lanes archive : List (CaseId × Reason)) (newly unmapped : List CaseId) :
    Option (List (CaseId × Reason)) :=
  resolve lanes archive (select newly unmapped)

/-- The published keys of a resolved map. -/
def keys (rows : List (CaseId × Reason)) : List CaseId := rows.map Prod.fst

theorem mem_select {newly unmapped : List CaseId} {case : CaseId}
    (h : case ∈ select newly unmapped) : mem unmapped case = true := by
  simp [select, List.mem_filter] at h
  exact h.2

theorem select_subset {newly unmapped : List CaseId} {case : CaseId}
    (h : case ∈ select newly unmapped) : case ∈ newly := by
  simp [select, List.mem_filter] at h
  exact h.1

/-- **Every published row is a case the manifest currently leaves unmapped.**
Refusal direction one, structurally: the staleness HYGIENE-16 found by hand
(a paragraph explaining why a mapped case is not mapped) cannot be derived. -/
theorem every_selected_case_is_unmapped (newly unmapped : List CaseId) (case : CaseId)
    (h : case ∈ select newly unmapped) : mem unmapped case = true :=
  mem_select h

/-- The same sentence contrapositively: a mapped case has no row. -/
theorem select_drops_a_mapped_case (newly unmapped : List CaseId) (case : CaseId)
    (h : mem unmapped case = false) : case ∉ select newly unmapped := by
  intro contra
  rw [mem_select contra] at h
  exact Bool.noConfusion h

/-- **The keys of the derived map are the published list, in order.** This is
what makes a row belonging to no published claim — the CT11 row this card found —
impossible rather than merely unlikely. -/
theorem keys_are_the_published_list {lanes archive : List (CaseId × Reason)}
    {newly unmapped : List CaseId} {rows : List (CaseId × Reason)}
    (h : reasons lanes archive newly unmapped = some rows) :
    keys rows = select newly unmapped := by
  have general : ∀ (cases : List CaseId) (out : List (CaseId × Reason)),
      resolve lanes archive cases = some out → keys out = cases := by
    intro cases
    induction cases with
    | nil => intro out h; simp [resolve] at h; simp [keys, ← h]
    | cons case rest ih =>
      intro out h
      simp only [resolve] at h
      cases hc : resolve1 lanes archive case with
      | none => rw [hc] at h; simp at h
      | some reason =>
        cases hr : resolve lanes archive rest with
        | none => rw [hc, hr] at h; simp at h
        | some tail =>
          rw [hc, hr] at h
          simp at h
          subst h
          simp only [keys, List.map_cons, List.cons.injEq, true_and]
          exact ih tail hr
  exact general (select newly unmapped) rows h

/-- **A case with no record refuses**, rather than publishing an empty reason:
inherited from the resolution the buckets already use. -/
theorem an_unclassified_case_refuses (lanes archive : List (CaseId × Reason))
    (newly unmapped : List CaseId) (case : CaseId)
    (hs : select newly unmapped = case :: (select newly unmapped).tail)
    (hl : find? lanes case = none) (ha : find? archive case = none) :
    reasons lanes archive newly unmapped = none := by
  unfold reasons
  rw [hs]
  simp [resolve, resolve1, hl, ha]

/-- A row's text is the record's, not the editor's: a lane row wins for the case
it names, and the archive decides a case no lane measured. Both halves are
`QCore.BucketTally`'s resolution rule, restated here at the reasons map. -/
theorem a_lane_reason_wins (lanes archive : List (CaseId × Reason)) (case : CaseId)
    (reason : Reason) (h : find? lanes case = some reason) :
    resolve1 lanes archive case = some reason :=
  QCore.BucketTally.a_lane_row_wins lanes archive case reason h

theorem the_archive_reason_decides_an_unmeasured_case
    (lanes archive : List (CaseId × Reason)) (case : CaseId) (reason : Reason)
    (hl : find? lanes case = none) (ha : find? archive case = some reason) :
    resolve1 lanes archive case = some reason :=
  QCore.BucketTally.the_archive_decides_an_unmeasured_case lanes archive case reason hl ha

/-! ### The drift check, both directions

`scripts/ct_stage_summary.py::check_matrix_ct_reasons`. The generator writes the
key, so a regeneration reconciles the file; this is the check a hand edit reaches
first, and the property that matters is that neither direction is silent. -/

/-- Committed rows the derivation does not produce. -/
def unproducible (committed derived : List CaseId) : List CaseId :=
  committed.filter (fun case => !mem derived case)

/-- Derived rows the committed map does not carry. -/
def missing (committed derived : List CaseId) : List CaseId :=
  derived.filter (fun case => !mem committed case)

/-- Everything the check reports. -/
def problems (committed derived : List CaseId) : List CaseId :=
  unproducible committed derived ++ missing committed derived

theorem mem_iff (cases : List CaseId) (case : CaseId) :
    mem cases case = true ↔ case ∈ cases := by
  induction cases with
  | nil => simp [mem]
  | cons head rest ih =>
    by_cases h : head = case
    · simp [mem, h]
    · simp [mem, h, ih]
      exact fun contra => absurd contra.symm h

/-- **A committed row outside the derivation is reported** — whether it names a
case the manifest maps (HYGIENE-16's CT39) or a case the companion list does not
name (this card's CT11). One rule catches both. -/
theorem a_row_outside_the_derivation_is_reported (committed derived : List CaseId)
    (case : CaseId) (hc : case ∈ committed) (hd : case ∉ derived) :
    case ∈ problems committed derived := by
  have : mem derived case = false := by
    cases h : mem derived case with
    | false => rfl
    | true => exact absurd ((mem_iff derived case).mp h) hd
  simp [problems, unproducible, List.mem_filter, hc, this]

/-- **A derived row the committed map drops is reported too** — the direction
that let the key be lost on every regeneration for four waves. -/
theorem a_missing_derived_row_is_reported (committed derived : List CaseId)
    (case : CaseId) (hd : case ∈ derived) (hc : case ∉ committed) :
    case ∈ problems committed derived := by
  have : mem committed case = false := by
    cases h : mem committed case with
    | false => rfl
    | true => exact absurd ((mem_iff committed case).mp h) hc
  simp [problems, missing, List.mem_filter, hd, this]

/-- And the check is not vacuously loud: identical key lists report nothing. -/
theorem equal_key_lists_report_nothing (cases : List CaseId) :
    problems cases cases = [] := by
  have empty : ∀ (left right : List CaseId),
      (∀ case ∈ left, case ∈ right) → left.filter (fun case => !mem right case) = [] := by
    intro left right covered
    rw [List.filter_eq_nil_iff]
    intro case h
    simp [(mem_iff right case).mpr (covered case h)]
  simp [problems, unproducible, missing, empty cases cases (fun _ h => h)]

/-! ### The transcribed instance

The committed map's three rows against the two ids its companion list published,
at this card's base commit. Transcribing them is a premise; what is decided here
is that the derivation refuses exactly the row that belonged to no published
claim, and publishes the other two. -/

/-- The keys the committed map carried before this card (`_derivation`/`_note`
commentary keys excluded — both implementations skip a leading underscore). -/
def committedBefore : List CaseId := ["CT11", "CT24", "CT47"]

/-- Card T00's newly-decidable judgment, frozen at revision `ea2af0f`. -/
def t00NewlyDecidable : List CaseId := ["CT24", "CT39", "CT47"]

/-- The manifest's unmapped set at this card's base commit. -/
def unmappedAtBase : List CaseId :=
  ["CT06", "CT07", "CT08", "CT09", "CT10", "CT11", "CT21", "CT22", "CT23", "CT24",
   "CT25", "CT40", "CT41", "CT42", "CT44", "CT46", "CT47", "CT49"]

/-- The derived list: CT39 was mapped by wave 8, so it is gone; CT11 was never in
card T00's judgment, so it never enters. -/
theorem the_derived_list_is_ct24_and_ct47 :
    select t00NewlyDecidable unmappedAtBase = ["CT24", "CT47"] := by
  decide

/-- **The CT11 row is refused, by name** — the measured before-state of this
card. -/
theorem the_ct11_row_is_refused :
    problems committedBefore (select t00NewlyDecidable unmappedAtBase) = ["CT11"] := by
  decide

/-- **The CT39 row would be refused too** — HYGIENE-16's staleness, decided on
the same instance rather than argued: a row for a case wave 8 mapped is outside
the derivation because the case left the unmapped set. -/
theorem the_ct39_row_would_be_refused :
    problems ("CT39" :: committedBefore) (select t00NewlyDecidable unmappedAtBase)
      = ["CT39", "CT11"] := by
  decide

/-- Non-vacuity: the derivation on this instance is exactly what the committed
matrix now carries, so the check is comparing two real lists rather than an empty
one against itself. -/
theorem the_committed_map_after_this_card_reports_nothing :
    problems ["CT24", "CT47"] (select t00NewlyDecidable unmappedAtBase) = [] := by
  decide

/-- And mapping one of the two remaining cases retires its row without anyone
editing the map. -/
theorem mapping_ct24_retires_its_row :
    select t00NewlyDecidable (unmappedAtBase.filter (fun case => case ≠ "CT24"))
      = ["CT47"] := by
  decide

end QCore.CtReasons

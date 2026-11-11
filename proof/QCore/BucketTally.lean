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

/-! Card `HYGIENE-16` (findings `D02-BUCKETS-STALE`, `D02-BUCKETS-LABELS`,
`CT-LABELS-T00-REVERT`): the label-resolution and bucket-tally relation the CT
ledger publishes.

Two derivations compute the same thing and refuse each other's drift —
`scripts/ct_stage_summary.py` (`unmapped_labels`, `unmapped_bucket_counts`) and
`compiler/tests/ct_corpus.rs` (`unmapped_labels`, `unmapped_bucket_counts`). This
module models that relation and proves the three sentences the ledger rests on:

* **the tally accounts for every unmapped case.** Finding `D02-BUCKETS-STALE`
  was a hand-kept tally summing 22 against 18 unmapped cases, because a lane
  that maps a case cannot be expected to audit four rows belonging to another
  lane's record. Card HYGIENE-14 made the counted *set* the manifest's own
  unmapped set; `tally_total_is_the_case_count` is why that is sufficient, and
  `dropping_a_case_drops_the_total` is why mapping a case moves the tally in the
  same commit that maps it.
* **a case with no label is a hole, not a zero.** `resolve` is total-or-refuse.
* **an archive edit under lane coverage is invisible.**
  `archive_edits_under_lane_coverage_are_invisible` is the soundness argument
  for restoring card T00's frozen `coverage.json` to its pre-CTWIN bytes: for a
  case some CT lane measured, the archived label is not read at all, so the
  restoration moves no count and no source. Its non-vacuity companion shows the
  archive *is* read — and published — for a case no lane measured, so the
  invisibility is a property of the resolution rule rather than of the archive
  being ignored everywhere.

What is **not** proved here. That the Rust and Python implementations compute
these functions is a transcription premise, checked by the two derivations
refusing each other's output against the committed record rather than by
extraction. Which bucket a case *belongs* in is analysis no relation derives;
this module is about accounting for it exactly once. -/
namespace QCore.BucketTally

/-- A conformance-case identifier, e.g. `CT40`. -/
abbrev CaseId := String

/-- A bucket label, e.g. `blocked-on-gateway-expressiveness (C-twin measured)`. -/
abbrev Label := String

/-- First row for a case, or none. Both implementations read their records in
landing order with the newest row overwriting, which is this lookup over the
reversed list; the order is irrelevant to everything proved here. -/
def find? : List (CaseId × Label) → CaseId → Option Label
  | [], _ => none
  | (key, value) :: rest, case => if key = case then some value else find? rest case

/-- The resolution rule, one case: **the CT lane record that measured this case,
else card T00's frozen archive.** -/
def resolve1 (lanes archive : List (CaseId × Label)) (case : CaseId) : Option Label :=
  match find? lanes case with
  | some label => some label
  | none => find? archive case

/-- The resolution rule over the manifest's own unmapped set. Total-or-refuse: a
case neither a lane nor the archive classifies makes the whole resolution fail,
rather than contributing a zero to the tally. -/
def resolve (lanes archive : List (CaseId × Label)) : List CaseId →
    Option (List (CaseId × Label))
  | [] => some []
  | case :: rest =>
    match resolve1 lanes archive case, resolve lanes archive rest with
    | some label, some tail => some ((case, label) :: tail)
    | _, _ => none

/-- Add one occurrence of `label` to a count map. -/
def bump : List (Label × Nat) → Label → List (Label × Nat)
  | [], label => [(label, 1)]
  | (key, count) :: rest, label =>
    if key = label then (key, count + 1) :: rest else (key, count) :: bump rest label

/-- `unmapped_bucket_counts`: the bucket tally of a list of resolved labels. -/
def tally : List Label → List (Label × Nat)
  | [] => []
  | label :: rest => bump (tally rest) label

/-- The published sum of a count map — the number the finding's 22 was. -/
def total : List (Label × Nat) → Nat
  | [] => 0
  | (_, count) :: rest => count + total rest

/-- The labels a resolution published. -/
def labels (rows : List (CaseId × Label)) : List Label := rows.map Prod.snd

theorem total_bump (counts : List (Label × Nat)) (label : Label) :
    total (bump counts label) = total counts + 1 := by
  induction counts with
  | nil => rfl
  | cons head rest ih =>
    obtain ⟨key, count⟩ := head
    by_cases h : key = label
    · simp [bump, total, h]
      omega
    · simp [bump, total, h, ih]
      omega

/-- **The tally accounts for every label it was given.** The exact property whose
silent violation was finding `D02-BUCKETS-STALE`; both implementations refuse
rather than publish a tally violating it. -/
theorem total_tally (values : List Label) : total (tally values) = values.length := by
  induction values with
  | nil => rfl
  | cons _ rest ih => simp [tally, total_bump, ih]

theorem resolve_length {lanes archive : List (CaseId × Label)} :
    ∀ {cases : List CaseId} {rows : List (CaseId × Label)},
      resolve lanes archive cases = some rows → rows.length = cases.length := by
  intro cases
  induction cases with
  | nil => intro rows h; simp [resolve] at h; simp [← h]
  | cons case rest ih =>
    intro rows h
    simp only [resolve] at h
    cases hc : resolve1 lanes archive case with
    | none => rw [hc] at h; simp at h
    | some label =>
      cases hr : resolve lanes archive rest with
      | none => rw [hc, hr] at h; simp at h
      | some tail =>
        rw [hc, hr] at h
        simp at h
        subst h
        simp [ih hr]

/-- **One row per unmapped case, and the tally sums to that count.** This is what
makes "the counted set comes from the manifest" sufficient: a case that gets
mapped leaves the case list, and the published sum follows it without anyone
editing the tally. -/
theorem tally_total_is_the_case_count {lanes archive : List (CaseId × Label)}
    {cases : List CaseId} {rows : List (CaseId × Label)}
    (h : resolve lanes archive cases = some rows) :
    total (tally (labels rows)) = cases.length := by
  simp [total_tally, labels, resolve_length h]

/-- Mapping a case moves the tally in the same commit that maps it: a case list
one shorter publishes a total one smaller, whatever the labels are. -/
theorem dropping_a_case_drops_the_total {lanes archive : List (CaseId × Label)}
    {cases smaller : List CaseId} {rows shorter : List (CaseId × Label)}
    (h : resolve lanes archive cases = some rows)
    (h' : resolve lanes archive smaller = some shorter)
    (hlen : smaller.length + 1 = cases.length) :
    total (tally (labels shorter)) + 1 = total (tally (labels rows)) := by
  rw [tally_total_is_the_case_count h, tally_total_is_the_case_count h']
  exact hlen

/-- **A hole is refused, not counted as a zero.** -/
theorem an_unclassified_case_refuses (lanes archive : List (CaseId × Label))
    (case : CaseId) (rest : List CaseId)
    (hl : find? lanes case = none) (ha : find? archive case = none) :
    resolve lanes archive (case :: rest) = none := by
  simp [resolve, resolve1, hl, ha]

/-- A lane row wins over the archive, for the case it names. -/
theorem a_lane_row_wins (lanes archive : List (CaseId × Label)) (case : CaseId)
    (label : Label) (h : find? lanes case = some label) :
    resolve1 lanes archive case = some label := by
  simp [resolve1, h]

/-- The archive decides a case no lane measured — the non-vacuity companion to
the invisibility theorem below. -/
theorem the_archive_decides_an_unmeasured_case (lanes archive : List (CaseId × Label))
    (case : CaseId) (label : Label)
    (hl : find? lanes case = none) (ha : find? archive case = some label) :
    resolve1 lanes archive case = some label := by
  simp [resolve1, hl, ha]

/-- **Card `CT-LABELS-T00-REVERT`'s soundness argument.** If every case where two
archives disagree is a case some lane measured, the two archives resolve
identically — so restoring card T00's frozen `coverage.json` to its pre-CTWIN
bytes moves no label, no source and no count. -/
theorem archive_edits_under_lane_coverage_are_invisible
    (lanes archive archive' : List (CaseId × Label)) (cases : List CaseId)
    (covered : ∀ case, find? archive case ≠ find? archive' case →
      (find? lanes case).isSome) :
    resolve lanes archive cases = resolve lanes archive' cases := by
  have pointwise : ∀ case, resolve1 lanes archive case = resolve1 lanes archive' case := by
    intro case
    cases hl : find? lanes case with
    | some label => simp [resolve1, hl]
    | none =>
      by_cases hne : find? archive case = find? archive' case
      · simp [resolve1, hl, hne]
      · exact absurd (covered case hne) (by simp [hl])
  induction cases with
  | nil => rfl
  | cons case rest ih => simp only [resolve, pointwise case, ih]

/-- The tally follows: an invisible archive edit publishes the same counts. -/
theorem archive_edits_under_lane_coverage_move_no_count
    (lanes archive archive' : List (CaseId × Label)) (cases : List CaseId)
    (covered : ∀ case, find? archive case ≠ find? archive' case →
      (find? lanes case).isSome)
    (rows rows' : List (CaseId × Label))
    (h : resolve lanes archive cases = some rows)
    (h' : resolve lanes archive' cases = some rows') :
    tally (labels rows) = tally (labels rows') := by
  have := archive_edits_under_lane_coverage_are_invisible lanes archive archive' cases covered
  rw [h] at this
  rw [h'] at this
  simp at this
  simp [this]

/-! ### The transcribed instance

The three cases card CT-CTWIN rewrote inside card T00's archive, with the two
archive versions this card moved between. The transcription of these rows from
`docs/reports/wave9-ctwin/ct_stage_evidence.json` and
`docs/reports/remaining-t00/coverage.json` is a premise; what is decided here is
that the move is invisible to the published label, source and tally, and that
the same is *not* true of an unmeasured case. -/

/-- The lane record CT-CTWIN wrote for the three cases it left unmapped. -/
def ctwinLanes : List (CaseId × Label) :=
  [("CT40", "blocked-on-gateway-expressiveness (C-twin measured)"),
   ("CT41", "blocked-on-gateway-expressiveness (C-twin measured)"),
   ("CT42", "blocked-on-gateway-expressiveness (C-twin measured)")]

/-- Card T00's archive as this card restored it, plus one case no lane measured. -/
def archiveRestored : List (CaseId × Label) :=
  [("CT40", "blocked-on-harness-schema (C-twin stage)"),
   ("CT41", "blocked-on-harness-schema (C-twin stage)"),
   ("CT42", "blocked-on-harness-schema (C-twin stage)"),
   ("CT44", "blocked-on-unimplemented-syntax")]

/-- The same archive as card CT-CTWIN had edited it in place. -/
def archiveEdited : List (CaseId × Label) :=
  [("CT40", "blocked-on-gateway-expressiveness (C-twin measured)"),
   ("CT41", "blocked-on-gateway-expressiveness (C-twin measured)"),
   ("CT42", "blocked-on-gateway-expressiveness (C-twin measured)"),
   ("CT44", "blocked-on-unimplemented-syntax")]

def ctwinCases : List CaseId := ["CT40", "CT41", "CT42", "CT44"]

/-- The restoration is decided, not argued: both archives publish the same rows. -/
theorem the_ctwin_restoration_publishes_the_same_rows :
    resolve ctwinLanes archiveRestored ctwinCases
      = resolve ctwinLanes archiveEdited ctwinCases := by
  decide

/-- And the same tally. -/
theorem the_ctwin_restoration_publishes_the_same_tally :
    (resolve ctwinLanes archiveRestored ctwinCases).map (fun rows => tally (labels rows))
      = (resolve ctwinLanes archiveEdited ctwinCases).map (fun rows => tally (labels rows)) := by
  decide

/-- Non-vacuity: the archive text is what CT44 publishes, so an edit *there*
would move the label. The invisibility above is the resolution rule at work, not
the archive being dead. -/
theorem the_archive_still_decides_ct44 :
    resolve1 ctwinLanes archiveRestored "CT44" = some "blocked-on-unimplemented-syntax" := by
  decide

/-- Non-vacuity of the tally sum on the transcribed instance: four cases, four
counted. -/
theorem the_transcribed_instance_accounts_for_every_case :
    (resolve ctwinLanes archiveRestored ctwinCases).map (fun rows => total (tally (labels rows)))
      = some ctwinCases.length := by
  decide

end QCore.BucketTally

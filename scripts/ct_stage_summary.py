#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

"""Stage-derived CT summary (card T00-CT-SCHEMA, task T00.9).

`docs/reevaluation/ct_manifest.json` schema 2 carries, for every CT case and
every pipeline stage, a status plus the evidence behind it: the executor that
ran the stage, the artifact it ran on, how many times it ran, the exact
`target_os` gate of that runner and — since card CT-CTWIN's `legacy_c` route —
the external tools it cannot start without. `compiler/tests/ct_corpus.rs`
rederives all of that from its own dispatch and refuses drift, so the manifest
is a checked record rather than a self-certifying one.

This module turns that record into the consumable numbers:

  * `summarize(root)`   -- the generated block, mirroring
                           `ct_corpus.rs::ct_stage_summary` exactly.
  * `unmapped_labels(root)` -- each unmapped case's current bucket label beside
                           the record that measured it: the newest CT lane's
                           `unmapped_reasons` row, else card T00's frozen
                           archive (card CT-LABELS).
  * `newly_decidable_reasons(root)` -- the acceptance *matrix*'s companion
                           reasons map, derived from the same records rather
                           than hand-kept (card GEN-CTREASONS, follow-on
                           `D02-GEN-CTREASONS`).
  * `merge_evidence(root)` -- the archived per-card `ct_stage_evidence.json`
                           files (and T00's `coverage.json`) merged into one
                           matrix, including the `aux:` rows the manifest has no
                           field for.
  * `check_provenance(root)` -- every disagreement between the two, as strings.

`docs/reports/remaining-d02/build_matrix.py` consumes all three, which is what
ended its thirteen-case divergence from the manifest.

Usage:
    python3 scripts/ct_stage_summary.py [ROOT] [--write] [--check]

`--write` rewrites the generated fields of `docs/reviews/r0_3/acceptance.json`;
`--check` exits nonzero if the committed record or the archived evidence
disagrees with the manifest.
"""

from __future__ import annotations

import json
import pathlib
import sys

#: Depth rank per stage. The deepest executed stage of a case is the one with
#: the greatest rank; ranks are shared across routes so a task-route ledger
#: check and a mutex-route invariant check sit at the same depth. Mirrors
#: `STAGE_DEPTH` in `compiler/tests/ct_corpus.rs`.
STAGE_DEPTH = {
    "parse": 0,
    "elaborate": 1,
    "footprint_lowering": 2,
    "task_lowering": 2,
    "mutex_lowering": 2,
    "epoch_lowering": 2,
    "sequential_whole_program": 2,
    "task_ledger_check": 3,
    "footprint_check": 3,
    "mutex_invariant_check": 3,
    "epoch_bound_check": 3,
    "task_reference_execution": 4,
    "mutex_reference_execution": 4,
    "sequential_reference_execution": 4,
    "legacy_c_twin": 4,
    "atomic_publication_check": 4,
    "c_emission": 5,
    "native_execution": 6,
    "native_sanitized": 7,
    "proof": 8,
}

#: The fields this generator owns in `docs/reviews/r0_3/acceptance.json`.
#: Everything else in that block is recorded analysis, not stage evidence.
#:
#: `unmapped_bucket_counts` joined this list in card HYGIENE-14 (finding
#: `D02-BUCKETS-STALE`). It is derived from a second input — the per-case bucket
#: labels — rather than from stage evidence, because the manifest carries no
#: per-case blocker record for an unmapped case at all; what makes it *derived*
#: rather than hand-kept is that the set of cases it counts comes from the
#: manifest's own `mapping_status`, so a case that gets mapped leaves the tally
#: in the same commit that maps it.
#:
#: `unmapped_label_sources` joined it in card CT-LABELS (finding
#: `D02-BUCKETS-LABELS`), which is what made the *label* derived too: a CT lane
#: that examines a case and leaves it unmapped may now record the bucket it
#: measured in its own `ct_stage_evidence.json`, and the newest such record wins
#: over card T00's frozen archive. Publishing the source beside the label is the
#: whole point — a reader can see which labels are current measurements and
#: which are still the 2026-09-07 archive.
GENERATED_FIELDS = [
    "ct_mapped",
    "ct_unmapped",
    "stage_histogram",
    "observation_counts",
    "host_scope",
    "tool_scope",
    "depth_profile",
    "case_depth",
    "unmapped_bucket_counts",
    "unmapped_label_sources",
]

#: Per-card stage-evidence records, in landing order. T00's coverage.json is an
#: archived audit at revision ea2af0f and covers the original eleven cases.
EVIDENCE_FILES = [
    "docs/reports/remaining-t00/coverage.json",
    "docs/reports/remaining-t03-ct-integration/ct_stage_evidence.json",
    "docs/reports/remaining-wave3-ct-integration/ct_stage_evidence.json",
    "docs/reports/wave5-ct-integration/ct_stage_evidence.json",
    "docs/reports/wave8-epoch-ct/ct_stage_evidence.json",
    "docs/reports/wave9-ctwin/ct_stage_evidence.json",
    "docs/reports/wave11-ct33/ct_stage_evidence.json",
    "docs/reports/wave13-ct34/ct_stage_evidence.json",
    "docs/reports/remaining-ct34-sanitized/ct_stage_evidence.json",
    "docs/reports/wave17-ct40/ct_stage_evidence.json",
    "docs/reports/ct-wire-lane/ct_stage_evidence.json",
    "docs/reports/ct-wire-lane-2/ct_stage_evidence.json",
    "docs/reports/ct22-wire/ct_stage_evidence.json",
    "docs/reports/ct01-proof-leg/ct_stage_evidence.json",
    "docs/reports/ct-proof-leg-2/ct_stage_evidence.json",
    "docs/reports/ct07-write-remap/ct_stage_evidence.json",
    "docs/reports/ct-wire-lane-3/ct_stage_evidence.json",
    "docs/reports/ct03-proof-alphabet/ct_stage_evidence.json",
    "docs/reports/ct-composed-native/ct_stage_evidence.json",
    "docs/reports/ct-wire-lane-4/ct_stage_evidence.json",
    # Card `CT-CTWIN-LANE` (wave 35): CT46 mapped, and the four cases this lane
    # re-measured and deliberately left unmapped. It is also the record that
    # retires wave 13's CT46 bucket row, by declaration rather than by editing
    # that record (constraint 21).
    "docs/reports/ct-ctwin-lane/ct_stage_evidence.json",
    # Card `MUTEX-SHARE-CROSS-EMISSION` (wave 36): the checked mutex backend
    # learned to render a non-nested fork/join trace, so CT46's four unassessed
    # stages move to executed on the case's own artifact. A superseding record
    # per constraint 21 — the ct-ctwin-lane record keeps its bytes.
    "docs/reports/mutex-share-cross-emission/ct_stage_evidence.json",
    # Card `CT-LANE-5` (wave 37): CT07 re-mapped from the sequential write lend
    # onto the two-thread range write lease, so the scenario's word
    # "independently" rests on two threads for the first time. A superseding
    # record per constraint 21 — the ct07-write-remap record keeps its bytes.
    # It maps no new case, so no count moves.
    "docs/reports/ct-lane-5/ct_stage_evidence.json",
    # Card `CT07-TWO-THREAD-PROOF-LEG` (wave 39): CT07's proof stage moves to
    # executed on the named ledger-bound leg. A superseding record per
    # constraint 21 — the ct-lane-5 record keeps its bytes. Every other stage
    # row is carried from that record unchanged; proof is the only move.
    "docs/reports/ct07-two-thread-proof-leg/ct_stage_evidence.json",
    # Card `CT42-CELL-ROUTE` (wave 39): CT42 mapped on the sequential route --
    # the socket arm the wave-35 lane measured unreachable from source, now
    # routed through `disjoint NAME NAME;` and refused by the unedited
    # DisjointFootprintsV1 checker at Q664. It retires the ct-ctwin-lane
    # record's CT42 bucket row by declaration; that record keeps its bytes.
    "docs/reports/ct42-cell-route/ct_stage_evidence.json",
    # Card `CT-PROOF-LEG-3` (wave 41): CT50's proof stage moves to executed on
    # the F02-bound leg. It supersedes card T00's frozen archive for CT50 --
    # the record that still held that case's stages -- by declaration; that
    # archive keeps its bytes (constraint 21). Every other stage row is carried
    # from it unchanged; proof is the only move.
    "docs/reports/ct-proof-leg-3/ct_stage_evidence.json",
    # Card `CT24-CLEANUP-TASK-COMPOSITION`: CT24 maps on the composed-epoch
    # route at the task-ledger refusal. Its own artifact reaches no epoch,
    # emission, native or proof stage; accepted controls do not lend it those
    # stages. Card T00's bucket was archive-held, so there is no lane row to
    # retire.
    "docs/reports/ct24-cleanup-task-composition/ct_stage_evidence.json",
    # Card `CT10-SOURCE-LAYOUT`: CT10 maps on a dedicated property-only route.
    # Its stages name footprint lowering/check rather than the scoped-task
    # ledger, which this route deliberately does not execute.
    "docs/reports/ct10-source-layout/ct_stage_evidence.json",
    "docs/reports/ct41-publication-mapping/ct_stage_evidence.json",
]

ACCEPTANCE = "docs/reviews/r0_3/acceptance.json"
MANIFEST = "docs/reevaluation/ct_manifest.json"

#: Card T00's archived triage of the then-unmapped cases, at revision ea2af0f.
#: Until card CT-LABELS it was the only record in the repository carrying a
#: per-case bucket label; it remains the fallback for every case no later lane
#: measured. The manifest itself has no blocker field for an unmapped case.
TRIAGE = "docs/reports/remaining-t00/coverage.json"

#: The `label_source` a case gets when no CT lane recorded a bucket for it and
#: the label therefore comes from `TRIAGE`. Deliberately not a path: the point of
#: publishing the source is that "this label is card T00's 2026-09-07 analysis,
#: not a current measurement" is legible at a glance.
ARCHIVE_SOURCE = "t00-archive"


def _read(root: pathlib.Path, relative: str):
    return json.loads((root / relative).read_text())


def load_manifest(root: pathlib.Path):
    manifest = _read(root, MANIFEST)
    if manifest["schema_version"] != 2:
        raise SystemExit(
            "ct_manifest.json is schema %s; this summarizer reads schema 2 only"
            % manifest["schema_version"]
        )
    return manifest


def _sorted(mapping):
    return {key: mapping[key] for key in sorted(mapping)}


def unmapped_ids(manifest):
    """The ids the manifest currently records as unmapped, in case order."""
    return [case["id"] for case in manifest["cases"] if case["mapping_status"] == "unmapped"]


def _triage_rows(root: pathlib.Path):
    """Card T00's archived per-case labels, one row per case or refuse."""
    triage = _read(root, TRIAGE)["unmapped_triage"]
    rows: dict[str, dict] = {}
    for case in triage["cases"]:
        # A second row for one case would silently decide the label — measured
        # while writing this: the obvious dict comprehension lets the *last* row
        # win with no complaint. One case, one classification.
        if case["id"] in rows:
            raise SystemExit(
                "%s: two triage rows classify the same case in %s" % (case["id"], TRIAGE)
            )
        rows[case["id"]] = case
    return rows


def _lane_row_retirements(records, unmapped: set):
    """The declared retirements of earlier lanes' `unmapped_reasons` rows.

    Wave-17 CT lane. Mapping a case whose bucket an *earlier* lane recorded used
    to leave two bad options: refuse, or edit that lane's archived record — the
    in-place rewrite of a frozen archive that card CT-LABELS built this mechanism
    to replace and card HYGIENE-16 reverted. The third option is the one
    `supersedes` already uses for labels: the mapping lane declares the
    retirement in its *own* record, under `retired_unmapped_reasons`, as
    `{CASE: {retires, mapped_by}}`, and the earlier record stays byte-identical.

    Total-or-reject, mirroring `ct_corpus.rs::lane_row_retirements` exactly:

    * a retirement naming a case the manifest still leaves unmapped — that would
      silently drop a live label;
    * a `retires` that is not a stage-evidence record, that carries no row for
      the case, or that is not the *newest* record carrying one (so a retirement
      cannot go stale past a supersession);
    * a retirement of a record that is not strictly earlier than the retiring
      one;
    * two records retiring the same case.

    A row for a mapped case with no retirement at all still fails closed in
    `_lane_labels`: the mechanism admits a declaration, never silence.
    """
    newest_row: dict[str, str] = {}
    for relative, data in records:
        for case_id in (data.get("unmapped_reasons") or {}):
            newest_row[case_id] = relative

    order = {relative: index for index, (relative, _) in enumerate(records)}
    retirements: dict[str, str] = {}
    for relative, data in records:
        for case_id, row in (data.get("retired_unmapped_reasons") or {}).items():
            if case_id in unmapped:
                raise SystemExit(
                    "%s: %s retires this case's bucket, but the manifest still leaves the "
                    "case unmapped — its label is live" % (case_id, relative)
                )
            for field in ("retires", "mapped_by"):
                value = row.get(field)
                if not isinstance(value, str) or not value.strip():
                    raise SystemExit(
                        "%s: %s carries no %s for this retirement" % (case_id, relative, field)
                    )
            retires = row["retires"]
            if retires not in order:
                raise SystemExit(
                    "%s: %s retires %s, which is not a CT stage-evidence record"
                    % (case_id, relative, retires)
                )
            held = newest_row.get(case_id)
            if held is None:
                raise SystemExit(
                    "%s: %s retires a bucket, but no evidence record publishes one for this "
                    "case" % (case_id, relative)
                )
            if retires != held:
                raise SystemExit(
                    "%s: %s retires the bucket in %s, but the record holding it is %s"
                    % (case_id, relative, retires, held)
                )
            if order[retires] >= order[relative]:
                raise SystemExit(
                    "%s: %s retires %s, which is not an earlier record"
                    % (case_id, relative, retires)
                )
            if case_id in retirements:
                raise SystemExit(
                    "%s: two evidence records retire this case's bucket" % case_id
                )
            retirements[case_id] = relative
    return retirements


def _lane_labels(root: pathlib.Path, unmapped: set):
    """The per-case buckets the CT lanes measured, newest record winning.

    Card CT-LABELS (finding `D02-BUCKETS-LABELS`). A lane that examines a case
    and leaves it unmapped may record, in its own `ct_stage_evidence.json`, an
    `unmapped_reasons` block of `{CASE: {bucket, reason, measured_by}}` rows.
    The files are read in landing order, so the newest row for a case is the one
    that survives.

    Three total-or-reject conditions, each a real way this could go wrong:

    * a row naming a case the manifest currently maps — the record would publish
      a blocker for a case that has none, which is exactly the staleness
      HYGIENE-14's structural repair removed from the *counts* — unless a later
      record declares the retirement, see [`_lane_row_retirements`];
    * two records carrying different buckets for one case with no supersession
      declared — the later row would silently decide the label, the same defect
      the duplicate-triage-row refusal above catches on the other input;
    * a supersession that is stale (names a record that does not hold the label)
      or vacuous (names anything at all when no earlier row exists). A lane label
      always beats card T00's archive by the resolution rule, so declaring *that*
      is vacuous too.
    """
    records = [(relative, _read(root, relative)) for relative in EVIDENCE_FILES]
    retirements = _lane_row_retirements(records, unmapped)
    held: dict[str, dict] = {}
    for relative, data in records:
        rows = data.get("unmapped_reasons") or {}
        for case_id, row in rows.items():
            if case_id not in unmapped:
                # The case is mapped now. A row for it is admissible only where a
                # later record declares the retirement; the retirement's own
                # conditions were checked above.
                if case_id not in retirements:
                    raise SystemExit(
                        "%s: %s records an unmapped-case bucket, but the manifest maps this "
                        "case" % (case_id, relative)
                    )
                continue
            for field in ("bucket", "reason", "measured_by"):
                value = row.get(field)
                if not isinstance(value, str) or not value.strip():
                    raise SystemExit(
                        "%s: %s carries no %s for this case" % (case_id, relative, field)
                    )
            superseded = row.get("supersedes")
            previous = held.get(case_id)
            if previous is None:
                if superseded is not None:
                    raise SystemExit(
                        "%s: %s declares its bucket supersedes %s, but no earlier record "
                        "labels this case" % (case_id, relative, superseded)
                    )
            elif previous["bucket"] == row["bucket"]:
                # Re-verification that reaches the same label is not a second
                # claim on the case; it is the record saying the label still
                # holds, and it needs no supersession declaration.
                pass
            elif superseded is None:
                raise SystemExit(
                    "%s: two evidence records label this case differently (%s says %r, "
                    "%s says %r) and neither declares a supersession"
                    % (
                        case_id,
                        previous["label_source"],
                        previous["bucket"],
                        relative,
                        row["bucket"],
                    )
                )
            elif superseded != previous["label_source"]:
                raise SystemExit(
                    "%s: %s declares its bucket supersedes %s, but the record holding the "
                    "label is %s" % (case_id, relative, superseded, previous["label_source"])
                )
            held[case_id] = dict(row, label_source=relative)
    return held


def unmapped_labels(root: pathlib.Path, manifest=None):
    """The current bucket label of every unmapped case, beside its source.

    Card CT-LABELS. The resolution rule is one sentence: **the newest CT lane
    record that measured this case, else card T00's frozen archive.** Both halves
    are published — `label` and `label_source` — because a label whose source is
    invisible is exactly what let CT11's and CT46's archived text stay wrong for
    several waves while the acceptance matrix published a corrected
    `blocker_current` beside it for one case only.

    Total-or-reject: a case with neither a lane row nor an archived row is a
    hole, not a zero.
    """
    manifest = manifest or load_manifest(root)
    unmapped = unmapped_ids(manifest)
    archive = _triage_rows(root)
    lanes = _lane_labels(root, set(unmapped))
    resolved: dict[str, dict] = {}
    unclassified = [
        case_id for case_id in unmapped if case_id not in lanes and case_id not in archive
    ]
    if unclassified:
        raise SystemExit(
            "unmapped cases with no measured bucket and no archived triage row in %s: %s"
            % (TRIAGE, unclassified)
        )
    for case_id in unmapped:
        if case_id in lanes:
            resolved[case_id] = {
                "label": lanes[case_id]["bucket"],
                "label_source": lanes[case_id]["label_source"],
                "reason": lanes[case_id]["reason"],
                "measured_by": lanes[case_id]["measured_by"],
                "archived_label": archive[case_id]["current_bucket"]
                if case_id in archive
                else None,
            }
        else:
            resolved[case_id] = {
                "label": archive[case_id]["current_bucket"],
                "label_source": ARCHIVE_SOURCE,
                "reason": archive[case_id]["blocker"],
                "measured_by": ARCHIVE_SOURCE,
                "archived_label": archive[case_id]["current_bucket"],
            }
    return resolved


def unmapped_label_sources(root: pathlib.Path, manifest=None):
    """`{case: label_source}` — the published half that makes a label auditable."""
    return {
        case_id: entry["label_source"]
        for case_id, entry in unmapped_labels(root, manifest).items()
    }


def check_label_sources(root: pathlib.Path):
    """A committed `label_source` the file set cannot support, as strings.

    The generated-field drift check already catches a source that disagrees with
    the derivation. This is the other direction and the one a hand edit reaches
    first: a source naming a record that is not in `EVIDENCE_FILES` at all, or a
    record that carries no row for that case. Either way the record would be
    citing evidence that is not there.
    """
    block = _read(root, ACCEPTANCE).get("ct_evidence", {})
    committed = block.get("unmapped_label_sources") or {}
    carries: dict[str, set] = {}
    for relative in EVIDENCE_FILES:
        for case_id in (_read(root, relative).get("unmapped_reasons") or {}):
            carries.setdefault(relative, set()).add(case_id)
    problems = []
    for case_id, source in sorted(committed.items()):
        if source == ARCHIVE_SOURCE:
            continue
        if source not in EVIDENCE_FILES:
            problems.append(
                "acceptance.json labels %s from %s, which is not a CT stage-evidence record"
                % (case_id, source)
            )
        elif case_id not in carries.get(source, set()):
            problems.append(
                "acceptance.json labels %s from %s, which records no bucket for that case"
                % (case_id, source)
            )
    return problems


def unmapped_bucket_counts(root: pathlib.Path, manifest=None):
    """Bucket tally over the cases the manifest *currently* leaves unmapped.

    Card HYGIENE-14, finding `D02-BUCKETS-STALE`. The hand-kept mirror in
    `acceptance.json` had drifted to a sum of 22 against `ct_unmapped: 18`,
    because a lane that maps a case edits the row its own case left and cannot
    be expected to audit four rows belonging to another lane's record. The
    repair is structural rather than arithmetic: the *set* of counted cases is
    read from the manifest, so mapping a case removes it from the tally in the
    same commit, and this function refuses rather than publishing a tally that
    does not account for every unmapped case.

    Two total-or-reject conditions, both load-bearing:

    * every currently-unmapped id must have a label — a case neither a lane nor
      the archive classifies is a hole, not a zero;
    * the counts must sum to the number of unmapped cases — the exact property
      whose silent violation is the finding this repair closes.

    Card CT-LABELS then made the *label* derived as well, and this function now
    counts resolved labels rather than archived ones. HYGIENE-14's boundary — "a
    count is current by construction; a label is historical by construction" —
    holds only for the cases no lane measured, and `unmapped_label_sources` says
    which those are.
    """
    labels = unmapped_labels(root, manifest)
    counts: dict[str, int] = {}
    for entry in labels.values():
        counts[entry["label"]] = counts.get(entry["label"], 0) + 1
    if sum(counts.values()) != len(labels):
        raise SystemExit(
            "unmapped_bucket_counts sums to %s against %s unmapped cases"
            % (sum(counts.values()), len(labels))
        )
    return counts


#: Where the derived reasons map is published. `docs/reviews/r0_3/acceptance.json`
#: keeps its own hand-kept copy of the same claim (checked, not derived, by
#: [`check_newly_decidable`]); this one is the acceptance *matrix*'s copy, which
#: `docs/reports/remaining-d02/build_matrix.py` owns and now derives.
MATRIX = "docs/reviews/r0_3/acceptance_matrix.json"

#: The key `build_matrix.py` emits and this module derives.
REASONS_KEY = "newly_decidable_not_mapped_reasons"


def newly_decidable_ids(root: pathlib.Path, manifest=None):
    """Card T00's newly-decidable ids, filtered to the cases still unmapped.

    "Decidable now" is card T00's judgment and no record derives it; "still not
    mapped" is a manifest fact, so the filter is the manifest's own unmapped set
    and a case that gets mapped leaves this list in the same commit that maps it
    — the structural repair card HYGIENE-14 made for the bucket tally, applied
    to the field beside it.
    """
    manifest = manifest or load_manifest(root)
    unmapped = set(unmapped_ids(manifest))
    triage = _read(root, TRIAGE)["unmapped_triage"]
    return [case_id for case_id in triage["newly_decidable"] if case_id in unmapped]


def newly_decidable_reasons(root: pathlib.Path, manifest=None):
    """`{case: {label, label_source, reason, measured_by}}` — the map, derived.

    Follow-on `D02-GEN-CTREASONS`. Until card GEN-CTREASONS this map was
    hand-maintained prose that `build_matrix.py` dropped on every regeneration
    and re-added by hand afterwards, which is why one of its rows explained why
    CT39 was unmapped for four waves after wave 8 mapped it.

    It is derivable because it is the companion prose of a list this generator
    already derives: **its keys are the ids of `newly_decidable_not_mapped`, and
    each value is that case's resolved label and reason** — the newest CT lane
    record that measured the case, else card T00's frozen archive, exactly as
    [`unmapped_labels`] resolves them. Publishing `label_source`/`measured_by`
    beside the text is what makes the difference between a current measurement
    and 2026-09-07 archive prose legible, the same rule card CT-LABELS applied to
    the buckets.

    Two total-or-reject conditions:

    * every produced key is a case the manifest currently leaves unmapped — a row
      explaining why a mapped case is not mapped is the staleness this closes;
    * every produced key has a resolved label, which [`unmapped_labels`] already
      refuses to invent.
    """
    manifest = manifest or load_manifest(root)
    unmapped = set(unmapped_ids(manifest))
    labels = unmapped_labels(root, manifest)
    rows = {}
    for case_id in newly_decidable_ids(root, manifest):
        if case_id not in unmapped:
            raise SystemExit(
                "%s: a derived %s row would explain why a mapped case is not mapped"
                % (case_id, REASONS_KEY)
            )
        entry = labels[case_id]
        rows[case_id] = {
            "label": entry["label"],
            "label_source": entry["label_source"],
            "reason": entry["reason"],
            "measured_by": entry["measured_by"],
        }
    return rows


def check_matrix_ct_reasons(root: pathlib.Path):
    """Both directions between the committed matrix's map and the derivation.

    The generator writes this key, so a run of it reconciles the file. This is
    the check a hand edit reaches first, and it refuses in both directions:

    * a committed row the derivation cannot produce — a case the manifest maps,
      or one no record makes newly decidable — is reported by name rather than
      carried forward;
    * a derived row the committed map does not carry is reported too, so the
      key cannot be quietly emptied or dropped again.
    """
    matrix_path = root / MATRIX
    if not matrix_path.is_file():
        return ["%s does not exist, so its %s map cannot be checked" % (MATRIX, REASONS_KEY)]
    section = json.loads(matrix_path.read_text()).get("ct_conformance", {})
    if REASONS_KEY not in section:
        return [
            "acceptance_matrix.json carries no %s; the generator derives it and a "
            "regeneration must publish it" % REASONS_KEY
        ]
    committed = section[REASONS_KEY]
    manifest = load_manifest(root)
    derived = newly_decidable_reasons(root, manifest)
    mapped = {case["id"] for case in manifest["cases"]} - set(unmapped_ids(manifest))
    problems = []
    # A leading underscore marks the map's own commentary rather than a case.
    committed_cases = {key: value for key, value in committed.items() if not key.startswith("_")}
    for case_id in sorted(set(committed_cases) - set(derived)):
        if case_id in mapped:
            problems.append(
                "acceptance_matrix.json %s explains why %s is not mapped, but the manifest "
                "maps it" % (REASONS_KEY, case_id)
            )
        else:
            problems.append(
                "acceptance_matrix.json %s carries a %s row the derivation does not produce"
                % (REASONS_KEY, case_id)
            )
    for case_id in sorted(set(derived) - set(committed_cases)):
        problems.append(
            "acceptance_matrix.json %s is missing the derived %s row" % (REASONS_KEY, case_id)
        )
    for case_id in sorted(set(derived) & set(committed_cases)):
        if committed_cases[case_id] != derived[case_id]:
            problems.append(
                "acceptance_matrix.json %s row for %s is not what the evidence records derive"
                % (REASONS_KEY, case_id)
            )
    return problems


def check_newly_decidable(root: pathlib.Path):
    """`newly_decidable_not_mapped` is hand-kept; its ids must still be unmapped.

    This field stays analysis — whether a case is *decidable now* is a judgment
    no record derives — but "not mapped" is a manifest fact, so the half that can
    go stale the way the buckets did is checked here.

    Card HYGIENE-16 extends the same check to the companion
    `newly_decidable_not_mapped_reasons` map, which HYGIENE-14 left unchecked:
    its keys are the same claim in prose, and a mapped case keeping a "why this
    is not mapped" paragraph is the identical staleness one field over. Measured
    while writing this: the acceptance *matrix*'s copy of that map still carried
    a CT39 row that wave 8 mapped, so the defect was live, not hypothetical.
    """
    manifest = load_manifest(root)
    block = _read(root, ACCEPTANCE).get("ct_evidence", {})
    unmapped = set(unmapped_ids(manifest))
    problems = [
        "acceptance.json newly_decidable_not_mapped names %s, which the manifest maps" % case_id
        for case_id in block.get("newly_decidable_not_mapped", [])
        if case_id not in unmapped
    ]
    problems.extend(
        "acceptance.json newly_decidable_not_mapped_reasons explains why %s is not mapped, "
        "but the manifest maps it" % case_id
        for case_id in sorted(block.get("newly_decidable_not_mapped_reasons") or {})
        # A leading underscore marks the map's own commentary rather than a case.
        if not case_id.startswith("_") and case_id not in unmapped
    )
    return problems


def summarize(root: pathlib.Path):
    """The generated block.

    Every field but two is computed from the manifest's stage evidence alone;
    `unmapped_bucket_counts` and `unmapped_label_sources` additionally read the
    CT lanes' own `unmapped_reasons` rows and card T00's archived triage for the
    labels, over the case set the manifest itself fixes (see
    [`unmapped_labels`]).
    """
    manifest = load_manifest(root)
    stages = manifest["stage_vocabulary"]["stages"]
    unknown = set(stages) - set(STAGE_DEPTH)
    if unknown:
        raise SystemExit("stages without a depth rank: %s" % sorted(unknown))

    histogram = {stage: {} for stage in stages}
    observations: dict[str, int] = {}
    host_scope: dict[str, set] = {}
    tool_scope: dict[str, set] = {}
    depth_profile: dict[str, int] = {}
    case_depth: dict[str, str] = {}
    mapped = unmapped = 0

    for case in manifest["cases"]:
        if case["mapping_status"] != "mapped":
            unmapped += 1
            continue
        mapped += 1
        deepest = None
        for stage in stages:
            record = case["stages"][stage]
            status = record["status"]
            histogram[stage][status] = histogram[stage].get(status, 0) + 1
            if status != "executed":
                continue
            observations[stage] = observations.get(stage, 0) + record["observations"]
            # A stage with no gate runs on every host; one with a gate reports
            # exactly the targets its runner covers, so a report produced on
            # another host cannot inherit `executed`.
            host_scope.setdefault(stage, set()).update(record.get("host_gate") or ["any"])
            # The tool axis is separate from the host axis: the legacy_c route's
            # stages run on any target and only where Clang exists, so reading
            # "any" on the host axis alone would say the opposite (card
            # CT-CTWIN).
            tool_scope.setdefault(stage, set()).update(record.get("tool_gate") or ["none"])
            if deepest is None or STAGE_DEPTH[stage] > STAGE_DEPTH[deepest]:
                deepest = stage
        if deepest is None:
            raise SystemExit("%s: a mapped case executes at least the parser" % case["id"])
        depth_profile[deepest] = depth_profile.get(deepest, 0) + 1
        case_depth[case["id"]] = deepest

    return {
        "ct_mapped": mapped,
        "ct_unmapped": unmapped,
        "stage_histogram": {stage: _sorted(histogram[stage]) for stage in stages},
        "observation_counts": _sorted(observations),
        "host_scope": {stage: sorted(host_scope[stage]) for stage in sorted(host_scope)},
        "tool_scope": {stage: sorted(tool_scope[stage]) for stage in sorted(tool_scope)},
        "depth_profile": _sorted(depth_profile),
        "case_depth": _sorted(case_depth),
        "unmapped_bucket_counts": unmapped_bucket_counts(root, manifest),
        "unmapped_label_sources": _sorted(unmapped_label_sources(root, manifest)),
    }


def merge_evidence(root: pathlib.Path):
    """Merge the archived per-card stage-evidence matrices into one mapping.

    Each card wrote its own file in the same shape; nothing merged them, which
    is why `build_matrix.py` still read T00's eleven-case audit alone.
    """
    merged = {}
    for relative in EVIDENCE_FILES:
        data = _read(root, relative)
        for case_id, entry in data["mapped_case_stage_matrix"].items():
            superseded = entry.get("supersedes")
            held = merged.get(case_id)
            # A later card may re-measure a case an earlier card mapped — new
            # evidence for the same case, not a second claim on it. It must say
            # so, and must name the record it replaces, so a silent second claim
            # stays the error it has always been and a supersession declaration
            # cannot go stale or vacuous.
            if held is None:
                if superseded is not None:
                    raise SystemExit(
                        "%s: %s declares it supersedes %s, but no earlier record claims "
                        "this case" % (case_id, relative, superseded)
                    )
            elif superseded is None:
                raise SystemExit(
                    "%s: two evidence records claim the same case (%s and %s)"
                    % (case_id, held["source"], relative)
                )
            elif superseded != held["source"]:
                raise SystemExit(
                    "%s: %s declares it supersedes %s, but the record it would replace "
                    "is %s" % (case_id, relative, superseded, held["source"])
                )
            merged[case_id] = dict(entry, source=relative)
    return merged


def check_provenance(root: pathlib.Path):
    """Disagreements between the manifest and the archived evidence records.

    Statuses and observation counts must agree. Executor *names* deliberately do
    not: the archived files credit a case's parse/elaborate/lowering/check to
    whichever function the card's author named, while schema 2 credits them
    uniformly to `run_mapped_case`, the function that actually calls
    `compile_files` on the fixture. Stages the older records omit entirely (T00
    predates the mutex stage names) count as `not_applicable`, which is how the
    hand-kept histogram already read them.
    """
    manifest = load_manifest(root)
    stages = manifest["stage_vocabulary"]["stages"]
    evidence = merge_evidence(root)
    problems = []

    mapped = {
        case["id"]: case for case in manifest["cases"] if case["mapping_status"] == "mapped"
    }
    for case_id in sorted(set(mapped) | set(evidence), key=lambda name: int(name[2:])):
        if case_id not in mapped:
            problems.append("%s: archived stage evidence exists but the case is unmapped" % case_id)
            continue
        if case_id not in evidence:
            problems.append("%s: mapped with no archived stage-evidence record" % case_id)
            continue
        verified = evidence[case_id]["verified_stages"]
        for stage in stages:
            recorded = verified.get(stage, {"status": "not_applicable"})
            claimed = mapped[case_id]["stages"][stage]
            if recorded["status"] != claimed["status"]:
                problems.append(
                    "%s/%s: manifest says %s, %s says %s"
                    % (
                        case_id,
                        stage,
                        claimed["status"],
                        evidence[case_id]["source"],
                        recorded["status"],
                    )
                )
            elif claimed["status"] == "executed" and recorded.get("observations") != claimed.get(
                "observations"
            ):
                problems.append(
                    "%s/%s: manifest counts %s runs, %s counts %s"
                    % (
                        case_id,
                        stage,
                        claimed.get("observations"),
                        evidence[case_id]["source"],
                        recorded.get("observations"),
                    )
                )
    return problems


def check_acceptance(root: pathlib.Path):
    """Disagreements between the committed acceptance record and the manifest."""
    derived = summarize(root)
    acceptance = _read(root, ACCEPTANCE)
    block = acceptance.get("ct_evidence", {})
    problems = []
    if block.get("generated_fields") != GENERATED_FIELDS:
        problems.append("acceptance.json generated_fields drifted from the generator")
    for field in GENERATED_FIELDS:
        source = acceptance if field.startswith("ct_") else block
        if source.get(field) != derived[field]:
            problems.append("acceptance.json `%s` is not what the manifest derives" % field)
    # `derived_from` is the third record that pins the evidence-file set, and
    # `compiler/tests/ct_corpus.rs` asserts its own Rust copy of that list
    # against the same field. Checking it here is what keeps the two derivations
    # reading the same files (card CT-LABELS).
    if block.get("derived_from") != [MANIFEST] + EVIDENCE_FILES:
        problems.append(
            "acceptance.json `derived_from` is not the manifest plus this generator's "
            "evidence-file list"
        )
    return problems


def write_acceptance(root: pathlib.Path):
    derived = summarize(root)
    path = root / ACCEPTANCE
    acceptance = json.loads(path.read_text())
    acceptance["ct_mapped"] = derived["ct_mapped"]
    acceptance["ct_unmapped"] = derived["ct_unmapped"]
    block = acceptance["ct_evidence"]
    block["generated_fields"] = list(GENERATED_FIELDS)
    for field in GENERATED_FIELDS:
        if not field.startswith("ct_"):
            block[field] = derived[field]
    path.write_text(json.dumps(acceptance, indent=2) + "\n")


def main(argv):
    arguments = [value for value in argv[1:] if not value.startswith("--")]
    flags = {value for value in argv[1:] if value.startswith("--")}
    root = pathlib.Path(arguments[0] if arguments else ".").resolve()
    if "--write" in flags:
        write_acceptance(root)
        print("wrote", root / ACCEPTANCE)
    problems = (
        check_acceptance(root)
        + check_provenance(root)
        + check_newly_decidable(root)
        + check_label_sources(root)
        + check_matrix_ct_reasons(root)
    )
    for problem in problems:
        print("DRIFT:", problem)
    summary = summarize(root)
    print(
        "mapped %s / unmapped %s; depth %s"
        % (summary["ct_mapped"], summary["ct_unmapped"], summary["depth_profile"])
    )
    if problems and "--check" in flags:
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))

#!/usr/bin/env python3
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
    "task_lowering": 2,
    "mutex_lowering": 2,
    "epoch_lowering": 2,
    "sequential_whole_program": 2,
    "task_ledger_check": 3,
    "mutex_invariant_check": 3,
    "epoch_bound_check": 3,
    "task_reference_execution": 4,
    "mutex_reference_execution": 4,
    "sequential_reference_execution": 4,
    "legacy_c_twin": 4,
    "c_emission": 5,
    "native_execution": 6,
    "native_sanitized": 7,
    "proof": 8,
}

#: The fields this generator owns in `docs/reviews/r0_3/acceptance.json`.
#: Everything else in that block is recorded analysis, not stage evidence.
#:
#: `unmapped_bucket_counts` joined this list in card HYGIENE-14 (finding
#: `D02-BUCKETS-STALE`). It is derived from a second input — card T00's archived
#: triage — rather than from stage evidence, because the manifest carries no
#: per-case blocker record for an unmapped case at all; what makes it *derived*
#: rather than hand-kept is that the set of cases it counts comes from the
#: manifest's own `mapping_status`, so a case that gets mapped leaves the tally
#: in the same commit that maps it.
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
]

ACCEPTANCE = "docs/reviews/r0_3/acceptance.json"
MANIFEST = "docs/reevaluation/ct_manifest.json"

#: Card T00's archived triage of the then-unmapped cases. It is the only record
#: in the repository that carries a per-case bucket label, and it is frozen at
#: revision ea2af0f: the manifest has no blocker field for an unmapped case.
TRIAGE = "docs/reports/remaining-t00/coverage.json"


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

    * every currently-unmapped id must have a triage row — a case the archive
      does not classify is a hole, not a zero;
    * the counts must sum to the number of unmapped cases — the exact property
      whose silent violation is the finding this repair closes.

    What is *not* derived, and is recorded as the card's boundary: the bucket
    label itself is card T00's frozen analysis at revision ea2af0f. A later card
    that re-verifies a still-unmapped case's reason by measurement publishes it
    beside the archived one (`blocker_current` in the acceptance matrix,
    read from the per-card `ct_stage_evidence.json` records), and does not
    rewrite the frozen label. So a count here is current by construction while a
    label is historical by construction.
    """
    manifest = manifest or load_manifest(root)
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
    unmapped = unmapped_ids(manifest)
    unclassified = [case_id for case_id in unmapped if case_id not in rows]
    if unclassified:
        raise SystemExit(
            "unmapped cases with no archived triage row in %s: %s" % (TRIAGE, unclassified)
        )
    counts: dict[str, int] = {}
    for case_id in unmapped:
        bucket = rows[case_id]["current_bucket"]
        counts[bucket] = counts.get(bucket, 0) + 1
    if sum(counts.values()) != len(unmapped):
        raise SystemExit(
            "unmapped_bucket_counts sums to %s against %s unmapped cases"
            % (sum(counts.values()), len(unmapped))
        )
    return counts


def check_newly_decidable(root: pathlib.Path):
    """`newly_decidable_not_mapped` is hand-kept; its ids must still be unmapped.

    This field stays analysis — whether a case is *decidable now* is a judgment
    no record derives — but "not mapped" is a manifest fact, so the half that can
    go stale the way the buckets did is checked here.
    """
    manifest = load_manifest(root)
    listed = _read(root, ACCEPTANCE).get("ct_evidence", {}).get("newly_decidable_not_mapped", [])
    unmapped = set(unmapped_ids(manifest))
    return [
        "acceptance.json newly_decidable_not_mapped names %s, which the manifest maps" % case_id
        for case_id in listed
        if case_id not in unmapped
    ]


def summarize(root: pathlib.Path):
    """The generated block.

    Every field but one is computed from the manifest's stage evidence alone;
    `unmapped_bucket_counts` additionally reads card T00's archived triage for
    the labels, over the case set the manifest itself fixes (see
    [`unmapped_bucket_counts`]).
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
            if case_id in merged:
                raise SystemExit(
                    "%s: two evidence records claim the same case (%s and %s)"
                    % (case_id, merged[case_id]["source"], relative)
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
    problems = check_acceptance(root) + check_provenance(root) + check_newly_decidable(root)
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

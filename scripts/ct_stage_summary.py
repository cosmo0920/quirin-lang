#!/usr/bin/env python3
"""Stage-derived CT summary (card T00-CT-SCHEMA, task T00.9).

`docs/reevaluation/ct_manifest.json` schema 2 carries, for every CT case and
every pipeline stage, a status plus the evidence behind it: the executor that
ran the stage, the artifact it ran on, how many times it ran, and the exact
`target_os` gate of that runner. `compiler/tests/ct_corpus.rs` rederives all of
that from its own dispatch and refuses drift, so the manifest is a checked
record rather than a self-certifying one.

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
GENERATED_FIELDS = [
    "ct_mapped",
    "ct_unmapped",
    "stage_histogram",
    "observation_counts",
    "host_scope",
    "depth_profile",
    "case_depth",
]

#: Per-card stage-evidence records, in landing order. T00's coverage.json is an
#: archived audit at revision ea2af0f and covers the original eleven cases.
EVIDENCE_FILES = [
    "docs/reports/remaining-t00/coverage.json",
    "docs/reports/remaining-t03-ct-integration/ct_stage_evidence.json",
    "docs/reports/remaining-wave3-ct-integration/ct_stage_evidence.json",
    "docs/reports/wave5-ct-integration/ct_stage_evidence.json",
    "docs/reports/wave8-epoch-ct/ct_stage_evidence.json",
]

ACCEPTANCE = "docs/reviews/r0_3/acceptance.json"
MANIFEST = "docs/reevaluation/ct_manifest.json"


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


def summarize(root: pathlib.Path):
    """The generated block, computed from the manifest's stage evidence alone."""
    manifest = load_manifest(root)
    stages = manifest["stage_vocabulary"]["stages"]
    unknown = set(stages) - set(STAGE_DEPTH)
    if unknown:
        raise SystemExit("stages without a depth rank: %s" % sorted(unknown))

    histogram = {stage: {} for stage in stages}
    observations: dict[str, int] = {}
    host_scope: dict[str, set] = {}
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
        "depth_profile": _sorted(depth_profile),
        "case_depth": _sorted(case_depth),
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
    problems = check_acceptance(root) + check_provenance(root)
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

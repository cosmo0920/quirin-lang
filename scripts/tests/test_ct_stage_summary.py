"""Tests for scripts/ct_stage_summary.py (card T00-CT-SCHEMA, task T00.9).

The generator is only worth having if its output is checked against the
manifest rather than trusted, and if it fails closed on a manifest that claims
more than its evidence carries. Both directions are exercised here on the real
repository and on mutated copies; the copies are temporary and never written
back.
"""

import copy
import json
import pathlib
import shutil
import subprocess
import sys
import tempfile
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[1]))

import ct_stage_summary as summary  # noqa: E402

ROOT = pathlib.Path(__file__).resolve().parents[2]


def load_manifest():
    return json.loads((ROOT / summary.MANIFEST).read_text())


class MutableRoot:
    """A scratch tree carrying just the files the summarizer reads."""

    def __init__(self):
        self.directory = pathlib.Path(tempfile.mkdtemp(prefix="qcore-ct-summary-"))

    def __enter__(self):
        for relative in [summary.MANIFEST, summary.ACCEPTANCE] + summary.EVIDENCE_FILES:
            target = self.directory / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(ROOT / relative, target)
        return self

    def __exit__(self, *_):
        shutil.rmtree(self.directory, ignore_errors=True)

    def write_manifest(self, manifest):
        (self.directory / summary.MANIFEST).write_text(json.dumps(manifest, indent=2) + "\n")

    def read_evidence(self, relative):
        return json.loads((self.directory / relative).read_text())

    def write_evidence(self, relative, data):
        (self.directory / relative).write_text(json.dumps(data, indent=2) + "\n")


class SummaryTest(unittest.TestCase):
    def test_committed_acceptance_record_is_what_the_manifest_derives(self):
        self.assertEqual(summary.check_acceptance(ROOT), [])

    def test_archived_stage_evidence_agrees_with_the_manifest(self):
        self.assertEqual(summary.check_provenance(ROOT), [])

    def test_every_mapped_case_has_an_archived_evidence_record(self):
        manifest = load_manifest()
        mapped = {
            case["id"] for case in manifest["cases"] if case["mapping_status"] == "mapped"
        }
        self.assertEqual(set(summary.merge_evidence(ROOT)), mapped)

    def test_generated_fields_are_exactly_the_generator_s_own(self):
        block = json.loads((ROOT / summary.ACCEPTANCE).read_text())["ct_evidence"]
        self.assertEqual(block["generated_fields"], summary.GENERATED_FIELDS)
        # Nothing may be both generated and hand-maintained.
        self.assertEqual(
            set(block["generated_fields"]) & set(block["hand_maintained_fields"]),
            set(),
        )
        # Every key of the block is accounted for by one of the two lists.
        self.assertEqual(
            set(block) | {"ct_mapped", "ct_unmapped"},
            set(block["generated_fields"]) | set(block["hand_maintained_fields"]),
        )

    def test_totals_reconcile(self):
        derived = summary.summarize(ROOT)
        self.assertEqual(derived["ct_mapped"] + derived["ct_unmapped"], 50)
        self.assertEqual(sum(derived["depth_profile"].values()), derived["ct_mapped"])
        self.assertEqual(len(derived["case_depth"]), derived["ct_mapped"])
        for stage, counts in derived["stage_histogram"].items():
            self.assertEqual(
                sum(counts.values()), derived["ct_mapped"], "%s does not cover every case" % stage
            )

    def test_depth_never_exceeds_executed_evidence(self):
        manifest = load_manifest()
        derived = summary.summarize(ROOT)
        for case in manifest["cases"]:
            if case["mapping_status"] != "mapped":
                continue
            deepest = derived["case_depth"][case["id"]]
            self.assertEqual(case["stages"][deepest]["status"], "executed")
            for stage, record in case["stages"].items():
                if record["status"] == "executed":
                    self.assertLessEqual(
                        summary.STAGE_DEPTH[stage], summary.STAGE_DEPTH[deepest]
                    )

    def test_recorded_transcripts_are_never_counted_as_executed(self):
        derived = summary.summarize(ROOT)
        sanitized = derived["stage_histogram"]["native_sanitized"]
        self.assertIn("recorded_not_gated", sanitized)
        self.assertEqual(
            derived["observation_counts"].get("native_sanitized"),
            sum(
                stage["observations"]
                for case in load_manifest()["cases"]
                if case["mapping_status"] == "mapped"
                for name, stage in case["stages"].items()
                if name == "native_sanitized" and stage["status"] == "executed"
            ),
        )

    def test_a_promoted_status_is_reported_as_drift(self):
        manifest = load_manifest()
        for case in manifest["cases"]:
            if case["id"] == "CT50":
                case["stages"]["native_sanitized"] = {
                    "status": "executed",
                    "executor": "compiler/tests/ct_corpus.rs::run_native_task_scenarios",
                    "artifact": "compiler/tests/fixtures/ct/CT50/case.qcore",
                    "observations": 1,
                    "host_gate": ["linux"],
                }
        with MutableRoot() as scratch:
            scratch.write_manifest(manifest)
            problems = summary.check_acceptance(scratch.directory)
            self.assertTrue(problems)
            self.assertTrue(summary.check_provenance(scratch.directory))

    def test_an_inflated_observation_count_is_reported_as_drift(self):
        manifest = load_manifest()
        for case in manifest["cases"]:
            if case["id"] == "CT16":
                case["stages"]["native_execution"]["observations"] = 40
        with MutableRoot() as scratch:
            scratch.write_manifest(manifest)
            self.assertTrue(summary.check_acceptance(scratch.directory))
            self.assertTrue(summary.check_provenance(scratch.directory))

    def test_a_mapping_change_is_reported_as_drift(self):
        manifest = load_manifest()
        for case in manifest["cases"]:
            if case["id"] == "CT24":
                case["mapping_status"] = "mapped"
                case["route"] = "task"
                case["stages"] = copy.deepcopy(
                    next(c for c in manifest["cases"] if c["id"] == "CT02")["stages"]
                )
        with MutableRoot() as scratch:
            scratch.write_manifest(manifest)
            self.assertTrue(summary.check_acceptance(scratch.directory))
            self.assertIn(
                "CT24: mapped with no archived stage-evidence record",
                summary.check_provenance(scratch.directory),
            )

    # Card CT34-SANITIZED. A later card may re-measure a case an earlier card
    # mapped. That is new evidence for the same case, not a second claim on it,
    # and it must say so by naming the record it replaces — so a silent second
    # claim stays the error it has always been, and a supersession declaration
    # can be neither stale nor vacuous.
    SUPERSEDING = "docs/reports/remaining-ct34-sanitized/ct_stage_evidence.json"
    SUPERSEDED = "docs/reports/wave13-ct34/ct_stage_evidence.json"

    def test_a_declared_supersession_replaces_exactly_the_record_it_names(self):
        merged = summary.merge_evidence(ROOT)
        self.assertEqual(merged["CT34"]["source"], self.SUPERSEDING)
        self.assertEqual(merged["CT34"]["supersedes"], self.SUPERSEDED)
        # The superseded file is still read for every case it alone claims.
        self.assertEqual(merged["CT33"]["source"], "docs/reports/wave11-ct33/ct_stage_evidence.json")

    def test_an_undeclared_second_claim_on_a_case_fails_closed(self):
        with MutableRoot() as scratch:
            data = scratch.read_evidence(self.SUPERSEDING)
            del data["mapped_case_stage_matrix"]["CT34"]["supersedes"]
            scratch.write_evidence(self.SUPERSEDING, data)
            with self.assertRaises(SystemExit):
                summary.merge_evidence(scratch.directory)

    def test_a_supersession_naming_the_wrong_record_fails_closed(self):
        with MutableRoot() as scratch:
            data = scratch.read_evidence(self.SUPERSEDING)
            data["mapped_case_stage_matrix"]["CT34"]["supersedes"] = (
                "docs/reports/remaining-t00/coverage.json"
            )
            scratch.write_evidence(self.SUPERSEDING, data)
            with self.assertRaises(SystemExit):
                summary.merge_evidence(scratch.directory)

    def test_a_supersession_of_nothing_fails_closed(self):
        with MutableRoot() as scratch:
            data = scratch.read_evidence(self.SUPERSEDING)
            entry = copy.deepcopy(data["mapped_case_stage_matrix"]["CT34"])
            # CT26 is claimed by no earlier record this file could replace.
            data["mapped_case_stage_matrix"]["CT26"] = entry
            scratch.write_evidence(self.SUPERSEDING, data)
            with self.assertRaises(SystemExit):
                summary.merge_evidence(scratch.directory)

    def test_an_unknown_schema_version_fails_closed(self):
        manifest = load_manifest()
        manifest["schema_version"] = 3
        with MutableRoot() as scratch:
            scratch.write_manifest(manifest)
            with self.assertRaises(SystemExit):
                summary.summarize(scratch.directory)

    def test_build_matrix_regenerates_the_ct_section_from_the_manifest(self):
        """The D02 generator is back in truth (card T00-CT-SCHEMA, item 4).

        It read T00's eleven-case audit alone and asserted its mapped set
        equalled the manifest's, which stopped holding at T03-CT; the section
        then diverged by thirteen cases and was hand-maintained. This runs the
        generator into a scratch file — never over the committed matrix — and
        checks its CT rows against the manifest case for case.
        """
        generator = ROOT / "docs/reports/remaining-d02/build_matrix.py"
        manifest = load_manifest()
        with tempfile.TemporaryDirectory(prefix="qcore-ct-matrix-") as scratch:
            out = pathlib.Path(scratch) / "acceptance_matrix.json"
            finished = subprocess.run(
                [sys.executable, str(generator), str(ROOT), "0", "--out", str(out)],
                capture_output=True,
                text=True,
            )
            self.assertEqual(finished.returncode, 0, finished.stderr)
            section = json.loads(out.read_text())["ct_conformance"]
        self.assertTrue(section["generator_status"].startswith("regenerating:"))
        rows = {row["id"]: row for row in section["ct_rows"]}
        self.assertEqual(len(rows), 50)
        derived = summary.summarize(ROOT)
        self.assertEqual(section["totals"]["mapped"], derived["ct_mapped"])
        self.assertEqual(section["totals"]["unmapped"], derived["ct_unmapped"])
        self.assertEqual(section["stage_histogram"], derived["stage_histogram"])
        self.assertEqual(section["depth_profile"], derived["depth_profile"])
        self.assertEqual(section["observation_counts"], derived["observation_counts"])
        self.assertEqual(section["case_depth"], derived["case_depth"])
        for case in manifest["cases"]:
            row = rows[case["id"]]
            self.assertEqual(row["mapping_status"], case["mapping_status"])
            if case["mapping_status"] != "mapped":
                continue
            self.assertEqual(row["route"], case["route"])
            self.assertEqual(row["stage_derived_status"], case["stages"])
            self.assertEqual(row["deepest_executed_stage"], derived["case_depth"][case["id"]])

    def test_a_stage_without_a_depth_rank_fails_closed(self):
        manifest = load_manifest()
        manifest["stage_vocabulary"]["stages"].append("invented_stage")
        with MutableRoot() as scratch:
            scratch.write_manifest(manifest)
            with self.assertRaises(SystemExit):
                summary.summarize(scratch.directory)


if __name__ == "__main__":
    unittest.main()

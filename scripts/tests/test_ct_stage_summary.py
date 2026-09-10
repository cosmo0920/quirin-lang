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
        for relative in [
            summary.MANIFEST,
            summary.ACCEPTANCE,
            summary.TRIAGE,
            # Card GEN-CTREASONS: the acceptance *matrix*'s copy of the reasons
            # map is derived and checked now, so the scratch tree carries it too.
            summary.MATRIX,
        ] + summary.EVIDENCE_FILES:
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

    def read_acceptance(self):
        return json.loads((self.directory / summary.ACCEPTANCE).read_text())

    def write_acceptance(self, acceptance):
        (self.directory / summary.ACCEPTANCE).write_text(json.dumps(acceptance, indent=2) + "\n")

    def write_triage(self, coverage):
        (self.directory / summary.TRIAGE).write_text(json.dumps(coverage, indent=2) + "\n")

    def read_matrix(self):
        return json.loads((self.directory / summary.MATRIX).read_text())

    def write_matrix(self, matrix):
        (self.directory / summary.MATRIX).write_text(
            json.dumps(matrix, indent=1, ensure_ascii=False) + "\n"
        )

    def write_matrix_reasons(self, reasons):
        """Replace only the reasons map, leaving the rest of the matrix alone."""
        matrix = self.read_matrix()
        matrix["ct_conformance"][summary.REASONS_KEY] = reasons
        self.write_matrix(matrix)


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

    # ------------------------------------------------------------------
    # Card HYGIENE-14, finding D02-BUCKETS-STALE: the triage tally.
    # ------------------------------------------------------------------

    def test_the_bucket_tally_sums_to_the_unmapped_count(self):
        """The exact property whose silent violation was the finding.

        The hand-kept mirror summed to 22 against `ct_unmapped: 18` because the
        four cases wave 8 mapped were never decremented out of a row belonging
        to another lane's record.
        """
        derived = summary.summarize(ROOT)
        self.assertEqual(
            sum(derived["unmapped_bucket_counts"].values()), derived["ct_unmapped"]
        )

    def test_every_unmapped_case_is_classified_exactly_once(self):
        manifest = load_manifest()
        unmapped = summary.unmapped_ids(manifest)
        self.assertEqual(len(unmapped), summary.summarize(ROOT)["ct_unmapped"])
        rows = json.loads((ROOT / summary.TRIAGE).read_text())["unmapped_triage"]["cases"]
        classified = [case["id"] for case in rows if case["id"] in set(unmapped)]
        self.assertEqual(sorted(classified), sorted(unmapped))
        self.assertEqual(len(classified), len(set(classified)))

    def test_mapping_a_case_moves_the_tally_without_anyone_editing_it(self):
        """The structural half of the repair.

        A lane that maps a case cannot leave the tally behind: the counted set
        is the manifest's own unmapped set, so the committed record drifts the
        moment the mapping lands and the drift names this field.
        """
        manifest = load_manifest()
        before = summary.unmapped_bucket_counts(ROOT, manifest)
        for case in manifest["cases"]:
            if case["id"] == "CT24":
                case["mapping_status"] = "mapped"
                case["route"] = "task"
                case["stages"] = copy.deepcopy(
                    next(c for c in manifest["cases"] if c["id"] == "CT02")["stages"]
                )
        with MutableRoot() as scratch:
            scratch.write_manifest(manifest)
            after = summary.unmapped_bucket_counts(scratch.directory, manifest)
            self.assertEqual(sum(after.values()), sum(before.values()) - 1)
            self.assertNotIn("DECIDABLE-NOW (with a distinguishing fixture)", after)
            self.assertIn(
                "acceptance.json `unmapped_bucket_counts` is not what the manifest derives",
                summary.check_acceptance(scratch.directory),
            )

    def test_an_unclassified_unmapped_case_fails_closed(self):
        """A hole is refused rather than counted as a zero."""
        coverage = json.loads((ROOT / summary.TRIAGE).read_text())
        coverage["unmapped_triage"]["cases"] = [
            case for case in coverage["unmapped_triage"]["cases"] if case["id"] != "CT44"
        ]
        with MutableRoot() as scratch:
            scratch.write_triage(coverage)
            with self.assertRaises(SystemExit):
                summary.unmapped_bucket_counts(scratch.directory)

    def test_a_second_triage_row_for_one_case_fails_closed(self):
        """Measured while writing this test, and repaired rather than pinned.

        A duplicate row does not disturb the *sum*, so the sum assertion cannot
        catch it; the obvious lookup let the last row silently decide the label.
        One case, one classification, or refuse.
        """
        coverage = json.loads((ROOT / summary.TRIAGE).read_text())
        extra = dict(
            next(
                case
                for case in coverage["unmapped_triage"]["cases"]
                if case["id"] == "CT44"
            )
        )
        extra["current_bucket"] = "invented"
        coverage["unmapped_triage"]["cases"].append(extra)
        with MutableRoot() as scratch:
            scratch.write_triage(coverage)
            with self.assertRaises(SystemExit):
                summary.unmapped_bucket_counts(scratch.directory)

    def test_newly_decidable_ids_must_still_be_unmapped(self):
        self.assertEqual(summary.check_newly_decidable(ROOT), [])
        manifest = load_manifest()
        for case in manifest["cases"]:
            if case["id"] == "CT11":
                case["mapping_status"] = "mapped"
        with MutableRoot() as scratch:
            scratch.write_manifest(manifest)
            # Card HYGIENE-16 deliberately widened this pin rather than
            # loosening it: CT11 is named in BOTH hand-kept halves, so mapping it
            # must now be reported twice — once for the id list, once for the
            # companion reasons map. The load-bearing half (the list) is
            # unchanged and still asserted verbatim.
            self.assertEqual(
                summary.check_newly_decidable(scratch.directory),
                [
                    "acceptance.json newly_decidable_not_mapped names CT11, "
                    "which the manifest maps",
                    "acceptance.json newly_decidable_not_mapped_reasons explains why CT11 "
                    "is not mapped, but the manifest maps it",
                ],
            )

    def test_a_reason_paragraph_for_a_mapped_case_fails_closed(self):
        """Card HYGIENE-16: the prose half of the same claim goes stale too.

        Measured on the committed tree while writing this: the acceptance
        *matrix*'s copy of this map still carried a CT39 row explaining why the
        case had no epoch source syntax, four waves after wave 8 mapped it. The
        acceptance record's copy is checked here, in both derivations; the
        matrix's copy was hand-carried commentary no derivation read until card
        GEN-CTREASONS derived it (`check_matrix_ct_reasons`, below).
        """
        self.assertEqual(summary.check_newly_decidable(ROOT), [])
        acceptance = json.loads((ROOT / summary.ACCEPTANCE).read_text())
        reasons = dict(acceptance["ct_evidence"]["newly_decidable_not_mapped_reasons"])
        # CT01 is mapped, and giving it a reason paragraph is exactly the shape
        # the matrix's CT39 row had.
        reasons["CT01"] = "invented: this case has no source form"
        acceptance["ct_evidence"]["newly_decidable_not_mapped_reasons"] = reasons
        with MutableRoot() as scratch:
            scratch.write_acceptance(acceptance)
            self.assertEqual(
                summary.check_newly_decidable(scratch.directory),
                [
                    "acceptance.json newly_decidable_not_mapped_reasons explains why CT01 "
                    "is not mapped, but the manifest maps it"
                ],
            )

    def test_the_maps_own_commentary_key_is_not_read_as_a_case(self):
        """`_note` is the map's own prose, not a case id; it must not refuse."""
        acceptance = json.loads((ROOT / summary.ACCEPTANCE).read_text())
        reasons = dict(acceptance["ct_evidence"]["newly_decidable_not_mapped_reasons"])
        reasons["_note"] = "hand-maintained commentary"
        acceptance["ct_evidence"]["newly_decidable_not_mapped_reasons"] = reasons
        with MutableRoot() as scratch:
            scratch.write_acceptance(acceptance)
            self.assertEqual(summary.check_newly_decidable(scratch.directory), [])

    # ------------------------------------------------------------------
    # Card GEN-CTREASONS, follow-on D02-GEN-CTREASONS: the matrix's copy of
    # the same map, derived rather than carried.
    #
    # Nothing below names a case id or a count that is not read out of the
    # committed manifest at run time: the CT lanes move those, and a test that
    # pinned them would fail on the lane's commit rather than on a defect.
    # ------------------------------------------------------------------

    def test_the_committed_matrix_reasons_map_is_what_the_records_derive(self):
        self.assertEqual(summary.check_matrix_ct_reasons(ROOT), [])

    def test_the_derived_rows_are_the_resolved_labels_of_the_published_list(self):
        """Keys are the companion list; values are that case's resolved label.

        The defect this closes was structural rather than arithmetic: the map
        was prose nothing derived, so it could explain a case the manifest maps
        (the CT39 row HYGIENE-16 found) or a case its own companion list does
        not name (the CT11 row this card found). Keying it to the list makes
        both impossible.
        """
        derived = summary.newly_decidable_reasons(ROOT)
        published = summary.newly_decidable_ids(ROOT)
        self.assertEqual(list(derived), published)
        labels = summary.unmapped_labels(ROOT)
        for case_id, row in derived.items():
            self.assertEqual(
                row,
                {
                    "label": labels[case_id]["label"],
                    "label_source": labels[case_id]["label_source"],
                    "reason": labels[case_id]["reason"],
                    "measured_by": labels[case_id]["measured_by"],
                },
            )
        # Non-vacuity: the derivation is not the empty map, and every id it
        # publishes is a case the manifest currently leaves unmapped.
        self.assertTrue(derived)
        self.assertTrue(set(derived) <= set(summary.unmapped_ids(load_manifest())))

    def test_a_matrix_reason_row_for_a_mapped_case_is_refused(self):
        """Refusal direction one, the shape of the CT39 staleness."""
        mapped = [
            case["id"] for case in load_manifest()["cases"] if case["mapping_status"] == "mapped"
        ]
        self.assertTrue(mapped)
        case_id = mapped[0]
        with MutableRoot() as scratch:
            reasons = dict(summary.newly_decidable_reasons(ROOT))
            reasons[case_id] = {
                "label": "invented",
                "label_source": "invented",
                "reason": "invented: this case has no source form",
                "measured_by": "invented",
            }
            scratch.write_matrix_reasons(reasons)
            self.assertEqual(
                summary.check_matrix_ct_reasons(scratch.directory),
                [
                    "acceptance_matrix.json %s explains why %s is not mapped, but the "
                    "manifest maps it" % (summary.REASONS_KEY, case_id)
                ],
            )

    def test_a_committed_row_the_derivation_cannot_produce_is_refused(self):
        """Refusal direction two: an unmapped case the list does not name."""
        newly = set(summary.newly_decidable_ids(ROOT))
        others = [case for case in summary.unmapped_ids(load_manifest()) if case not in newly]
        self.assertTrue(others, "every unmapped case is newly decidable; this test is vacuous")
        case_id = others[0]
        with MutableRoot() as scratch:
            reasons = dict(summary.newly_decidable_reasons(ROOT))
            reasons[case_id] = {"label": "x", "label_source": "x", "reason": "x", "measured_by": "x"}
            scratch.write_matrix_reasons(reasons)
            self.assertEqual(
                summary.check_matrix_ct_reasons(scratch.directory),
                [
                    "acceptance_matrix.json %s carries a %s row the derivation does not "
                    "produce" % (summary.REASONS_KEY, case_id)
                ],
            )

    def test_the_historical_ct11_row_is_refused(self):
        """The measured before-state of this card, as a regression.

        The committed map carried a CT11 row while its own companion list named
        only CT24 and CT47 — a second staleness class beside HYGIENE-16's CT39
        row: not "a mapped case keeps its paragraph" but "a paragraph belongs to
        no published claim". CT11's blocker is not lost by dropping it here; the
        same section publishes it, with more provenance, as that case's
        `blocker_current`.
        """
        manifest = load_manifest()
        mapped = {
            case["id"] for case in manifest["cases"] if case["mapping_status"] == "mapped"
        }
        expected = (
            "acceptance_matrix.json %s explains why CT11 is not mapped, but the manifest "
            "maps it" % summary.REASONS_KEY
            if "CT11" in mapped
            else "acceptance_matrix.json %s carries a CT11 row the derivation does not "
            "produce" % summary.REASONS_KEY
        )
        with MutableRoot() as scratch:
            reasons = dict(summary.newly_decidable_reasons(ROOT))
            reasons["CT11"] = (
                "Measured: ScopedReadSharesV1 refuses reconstruction while a share is "
                "retained (Q600 LoanOutstanding { resource: 1, outstanding: 512, "
                "borrowers: [3] })."
            )
            scratch.write_matrix_reasons(reasons)
            self.assertEqual(summary.check_matrix_ct_reasons(scratch.directory), [expected])

    def test_dropping_the_map_or_a_row_of_it_is_refused(self):
        """The direction that let the key be lost on every regeneration."""
        derived = summary.newly_decidable_reasons(ROOT)
        case_id = next(iter(derived))
        with MutableRoot() as scratch:
            matrix = scratch.read_matrix()
            del matrix["ct_conformance"][summary.REASONS_KEY]
            scratch.write_matrix(matrix)
            problems = summary.check_matrix_ct_reasons(scratch.directory)
            self.assertEqual(len(problems), 1)
            self.assertIn("carries no %s" % summary.REASONS_KEY, problems[0])

            reasons = {key: value for key, value in derived.items() if key != case_id}
            scratch.write_matrix_reasons(reasons)
            self.assertEqual(
                summary.check_matrix_ct_reasons(scratch.directory),
                [
                    "acceptance_matrix.json %s is missing the derived %s row"
                    % (summary.REASONS_KEY, case_id)
                ],
            )

    def test_a_rewritten_reason_paragraph_is_refused(self):
        """The map is the records' text, not the editor's."""
        derived = summary.newly_decidable_reasons(ROOT)
        case_id = next(iter(derived))
        with MutableRoot() as scratch:
            reasons = copy.deepcopy(derived)
            reasons[case_id]["reason"] = "rewritten by hand"
            scratch.write_matrix_reasons(reasons)
            self.assertEqual(
                summary.check_matrix_ct_reasons(scratch.directory),
                [
                    "acceptance_matrix.json %s row for %s is not what the evidence records "
                    "derive" % (summary.REASONS_KEY, case_id)
                ],
            )

    def test_mapping_a_newly_decidable_case_retires_its_reason_row(self):
        """A lane that maps a case takes the row with it, editing nothing."""
        derived = summary.newly_decidable_reasons(ROOT)
        case_id = next(iter(derived))
        manifest = load_manifest()
        for case in manifest["cases"]:
            if case["id"] == case_id:
                case["mapping_status"] = "mapped"
        with MutableRoot() as scratch:
            scratch.write_manifest(manifest)
            after = summary.newly_decidable_reasons(scratch.directory)
            self.assertNotIn(case_id, after)
            self.assertEqual(set(after), set(derived) - {case_id})
            # And the committed copy is now reported as stale by name, rather
            # than surviving as a paragraph about a mapped case.
            self.assertEqual(
                summary.check_matrix_ct_reasons(scratch.directory),
                [
                    "acceptance_matrix.json %s explains why %s is not mapped, but the "
                    "manifest maps it" % (summary.REASONS_KEY, case_id)
                ],
            )

    def test_the_maps_own_derivation_key_is_not_read_as_a_case(self):
        """`_derivation` is the map's own prose; the committed file carries it."""
        committed = json.loads((ROOT / summary.MATRIX).read_text())
        block = committed["ct_conformance"][summary.REASONS_KEY]
        self.assertIn("_derivation", block)
        self.assertEqual(summary.check_matrix_ct_reasons(ROOT), [])

    def test_the_historical_stale_bucket_block_is_refused(self):
        """Card HYGIENE-16, finding `D02-BUCKETS-STALE`, as a regression.

        The committed record and the derivation agree today, so a test that only
        reads the tree cannot show that a *re-inflated* hand edit would be
        caught. This restores the exact historical shape — the four cases wave 8
        mapped still counted, summing 22 against `ct_unmapped: 18` — into a
        scratch acceptance record and requires the drift to be reported by
        field name.
        """
        acceptance = json.loads((ROOT / summary.ACCEPTANCE).read_text())
        derived = summary.summarize(ROOT)
        self.assertEqual(
            acceptance["ct_evidence"]["unmapped_bucket_counts"],
            derived["unmapped_bucket_counts"],
            "precondition: the committed record is currently reconciled",
        )
        stale = dict(derived["unmapped_bucket_counts"])
        # The wave-8 epoch bucket, never decremented when CT35/36/38/39 mapped.
        stale["blocked-on-epoch-source-syntax"] = 4
        acceptance["ct_evidence"]["unmapped_bucket_counts"] = stale
        self.assertEqual(sum(stale.values()), 22)
        with MutableRoot() as scratch:
            scratch.write_acceptance(acceptance)
            self.assertIn(
                "acceptance.json `unmapped_bucket_counts` is not what the manifest derives",
                summary.check_acceptance(scratch.directory),
            )

    def test_the_archive_label_of_a_lane_measured_case_does_not_move_the_tally(self):
        """Card HYGIENE-16 item 2: why restoring T00's frozen bytes is neutral.

        Card CT-CTWIN rewrote CT40/CT41/CT42's rows inside card T00's archived
        `coverage.json`; this card restored that file to its pre-CTWIN bytes and
        left the current labels to the dual-publication mechanism. The property
        that makes the restoration tally-neutral is the resolution rule itself:
        for a case a lane measured, the archived label is not read at all. This
        drives it directly — rewriting the archive rows of all three to an
        invented bucket must change neither the counts nor the sources.
        """
        before_counts = summary.unmapped_bucket_counts(ROOT)
        before_sources = summary.unmapped_label_sources(ROOT)
        coverage = json.loads((ROOT / summary.TRIAGE).read_text())
        measured = {"CT40", "CT41", "CT42"}
        touched = 0
        for case in coverage["unmapped_triage"]["cases"]:
            if case["id"] in measured:
                case["current_bucket"] = "invented-archive-label"
                case["blocker"] = "invented"
                touched += 1
        self.assertEqual(touched, len(measured))
        with MutableRoot() as scratch:
            scratch.write_triage(coverage)
            self.assertEqual(summary.unmapped_bucket_counts(scratch.directory), before_counts)
            self.assertEqual(summary.unmapped_label_sources(scratch.directory), before_sources)
            self.assertEqual(summary.check_acceptance(scratch.directory), [])
            # Non-vacuity: the archive text IS read, and published, for a case no
            # lane measured, so the neutrality above is a property of the
            # resolution rule and not of the archive being ignored everywhere.
            labels = summary.unmapped_labels(scratch.directory)
            self.assertEqual(labels["CT40"]["archived_label"], "invented-archive-label")
            self.assertNotEqual(labels["CT40"]["label"], "invented-archive-label")
            self.assertEqual(labels["CT44"]["label_source"], summary.ARCHIVE_SOURCE)

    # ------------------------------------------------------------------
    # Card CT-LABELS, finding D02-BUCKETS-LABELS: the label, not just the count.
    # ------------------------------------------------------------------

    #: The three lanes that measured a still-unmapped case's bucket, and the
    #: newest evidence record overall (which measures none, so it is the natural
    #: place to stage a conflicting second claim).
    WAVE3 = "docs/reports/remaining-wave3-ct-integration/ct_stage_evidence.json"
    WAVE9 = "docs/reports/wave9-ctwin/ct_stage_evidence.json"
    WAVE13 = "docs/reports/wave13-ct34/ct_stage_evidence.json"
    NEWEST = "docs/reports/remaining-ct34-sanitized/ct_stage_evidence.json"

    def test_a_measured_label_wins_over_the_archive_and_says_so(self):
        """The resolution rule, on the real tree.

        Five of the eighteen unmapped cases now carry a lane's measured bucket;
        the other thirteen still read card T00's 2026-09-07 analysis, and the
        published source is what makes the difference legible.
        """
        labels = summary.unmapped_labels(ROOT)
        measured = {
            case: entry["label_source"]
            for case, entry in labels.items()
            if entry["label_source"] != summary.ARCHIVE_SOURCE
        }
        self.assertEqual(
            measured,
            {
                "CT11": self.WAVE3,
                "CT40": self.WAVE9,
                "CT41": self.WAVE9,
                "CT42": self.WAVE9,
                "CT46": self.WAVE13,
            },
        )
        # The two labels that actually moved, with the archived text retained
        # beside each rather than overwritten.
        self.assertEqual(
            labels["CT11"]["label"], "blocked-on-scenario-mismatch (no local alias form)"
        )
        self.assertEqual(labels["CT11"]["archived_label"], "blocked-on-task-lowering (share captures)")
        self.assertEqual(labels["CT46"]["label"], "blocked-on-mutex-capture (CT46-MUTEX-CAPTURE)")
        self.assertEqual(
            labels["CT46"]["archived_label"], "blocked-on-unimplemented-syntax (no mutex value)"
        )
        # CT40/CT41/CT42 used to read byte-identically to the archive, because
        # card CT-CTWIN wrote its own measurement into T00's archived audit in
        # place. Card HYGIENE-16 restored that file to its pre-CTWIN bytes
        # (`CT-LABELS-T00-REVERT`), so the two halves now differ for these three
        # and the assertion is inverted deliberately: the current label comes
        # from the lane's own record, T00's original text is retained beside it,
        # and the tally did not move in either direction.
        for case in ["CT40", "CT41", "CT42"]:
            self.assertEqual(
                labels[case]["label"], "blocked-on-gateway-expressiveness (C-twin measured)"
            )
            self.assertEqual(
                labels[case]["archived_label"], "blocked-on-harness-schema (C-twin stage)"
            )
            self.assertNotEqual(labels[case]["label"], labels[case]["archived_label"])
        self.assertEqual(
            sum(summary.unmapped_bucket_counts(ROOT).values()),
            summary.summarize(ROOT)["ct_unmapped"],
        )

    def test_every_committed_label_source_is_supported_by_the_file_set(self):
        self.assertEqual(summary.check_label_sources(ROOT), [])

    def test_a_bucket_naming_a_mapped_case_fails_closed(self):
        """A mapped case has no blocker to publish.

        This is the label-side of HYGIENE-14's structural repair: a lane row
        cannot outlive the mapping of its own case. CT34 is the worked example —
        the wave-13 lane both mapped it and left CT46 unmapped, so a row for it
        in that same file is the exact mistake to refuse.
        """
        with MutableRoot() as scratch:
            data = scratch.read_evidence(self.WAVE13)
            data["unmapped_reasons"]["CT34"] = {
                "bucket": "blocked-on-unimplemented-syntax (no mutex value)",
                "reason": "stale text for a case this very lane mapped",
                "measured_by": "nothing",
            }
            scratch.write_evidence(self.WAVE13, data)
            with self.assertRaises(SystemExit):
                summary.unmapped_labels(scratch.directory)

    def test_mapping_a_labelled_case_retires_its_label(self):
        """The same refusal reached from the manifest side.

        A lane maps CT46 without deleting its own `unmapped_reasons` row: the
        derivation refuses rather than publishing a blocker for a mapped case.
        """
        manifest = load_manifest()
        for case in manifest["cases"]:
            if case["id"] == "CT46":
                case["mapping_status"] = "mapped"
        with MutableRoot() as scratch:
            scratch.write_manifest(manifest)
            with self.assertRaises(SystemExit):
                summary.unmapped_labels(scratch.directory, manifest)

    def test_two_records_labelling_one_case_differently_fail_closed(self):
        """Without a supersession the later row would silently decide the label.

        Exactly the defect the duplicate-triage-row refusal catches on the other
        input, on the input a future CT lane will actually write.
        """
        with MutableRoot() as scratch:
            data = scratch.read_evidence(self.NEWEST)
            data["unmapped_reasons"] = {
                "CT46": {
                    "bucket": "a different label",
                    "reason": "a second, undeclared claim on the same case",
                    "measured_by": "nothing",
                }
            }
            scratch.write_evidence(self.NEWEST, data)
            with self.assertRaises(SystemExit):
                summary.unmapped_labels(scratch.directory)

    def test_a_declared_bucket_supersession_replaces_exactly_the_record_it_names(self):
        """The admitted direction, and both ways of getting it wrong."""
        with MutableRoot() as scratch:
            data = scratch.read_evidence(self.NEWEST)
            data["unmapped_reasons"] = {
                "CT46": {
                    "bucket": "a later measurement",
                    "reason": "measured again on a later tree",
                    "measured_by": "a hypothetical wave-15 lane",
                    "supersedes": self.WAVE13,
                }
            }
            scratch.write_evidence(self.NEWEST, data)
            labels = summary.unmapped_labels(scratch.directory)
            self.assertEqual(labels["CT46"]["label"], "a later measurement")
            self.assertEqual(labels["CT46"]["label_source"], self.NEWEST)

            # Stale: it names a record that does not hold the label.
            data["unmapped_reasons"]["CT46"]["supersedes"] = self.WAVE9
            scratch.write_evidence(self.NEWEST, data)
            with self.assertRaises(SystemExit):
                summary.unmapped_labels(scratch.directory)

            # Vacuous: no earlier record labels CT44 at all. A lane label always
            # beats T00's archive by the resolution rule, so declaring it is not
            # a supersession of anything.
            data["unmapped_reasons"] = {
                "CT44": {
                    "bucket": "ambiguous",
                    "reason": "re-measured",
                    "measured_by": "a hypothetical wave-15 lane",
                    "supersedes": summary.TRIAGE,
                }
            }
            scratch.write_evidence(self.NEWEST, data)
            with self.assertRaises(SystemExit):
                summary.unmapped_labels(scratch.directory)

    def test_a_re_verification_reaching_the_same_label_needs_no_supersession(self):
        """Agreement is not a second claim.

        A later lane that re-measures a case and lands on the same bucket is the
        record saying the label still holds — refusing that would make honest
        re-verification harder than silence.
        """
        with MutableRoot() as scratch:
            data = scratch.read_evidence(self.NEWEST)
            data["unmapped_reasons"] = {
                "CT46": {
                    "bucket": "blocked-on-mutex-capture (CT46-MUTEX-CAPTURE)",
                    "reason": "re-verified on a later tree; unchanged",
                    "measured_by": "a hypothetical wave-15 lane",
                }
            }
            scratch.write_evidence(self.NEWEST, data)
            labels = summary.unmapped_labels(scratch.directory)
            self.assertEqual(
                labels["CT46"]["label"], "blocked-on-mutex-capture (CT46-MUTEX-CAPTURE)"
            )
            self.assertEqual(labels["CT46"]["label_source"], self.NEWEST)

    def test_a_row_missing_a_required_field_fails_closed(self):
        for field in ["bucket", "reason", "measured_by"]:
            with self.subTest(field=field), MutableRoot() as scratch:
                data = scratch.read_evidence(self.WAVE13)
                data["unmapped_reasons"]["CT46"][field] = "   "
                scratch.write_evidence(self.WAVE13, data)
                with self.assertRaises(SystemExit):
                    summary.unmapped_labels(scratch.directory)

    def test_an_unsupported_label_source_is_reported(self):
        """A `label_source` the file set cannot support.

        Two shapes, both reachable by a hand edit of the committed record: a
        source that is not a CT stage-evidence record at all, and one that is but
        carries no row for that case.
        """
        with MutableRoot() as scratch:
            acceptance = scratch.read_acceptance()
            acceptance["ct_evidence"]["unmapped_label_sources"]["CT44"] = (
                "docs/reports/remaining-t00/README.md"
            )
            acceptance["ct_evidence"]["unmapped_label_sources"]["CT47"] = self.WAVE9
            scratch.write_acceptance(acceptance)
            problems = summary.check_label_sources(scratch.directory)
            self.assertEqual(
                problems,
                [
                    "acceptance.json labels CT44 from docs/reports/remaining-t00/README.md, "
                    "which is not a CT stage-evidence record",
                    "acceptance.json labels CT47 from %s, which records no bucket for that case"
                    % self.WAVE9,
                ],
            )

    def test_a_changed_lane_label_moves_the_tally_and_drifts_the_record(self):
        with MutableRoot() as scratch:
            data = scratch.read_evidence(self.WAVE13)
            data["unmapped_reasons"]["CT46"]["bucket"] = "a relabelled bucket"
            scratch.write_evidence(self.WAVE13, data)
            counts = summary.unmapped_bucket_counts(scratch.directory)
            self.assertIn("a relabelled bucket", counts)
            self.assertNotIn("blocked-on-mutex-capture (CT46-MUTEX-CAPTURE)", counts)
            self.assertEqual(sum(counts.values()), 18)
            self.assertIn(
                "acceptance.json `unmapped_bucket_counts` is not what the manifest derives",
                summary.check_acceptance(scratch.directory),
            )

    def test_derived_from_pins_the_evidence_file_set(self):
        """The third record that ties the Python and Rust derivations together.

        Neither generator can read the other's source, so both assert their own
        file list against `acceptance.json`'s `derived_from`.
        """
        block = json.loads((ROOT / summary.ACCEPTANCE).read_text())["ct_evidence"]
        self.assertEqual(block["derived_from"], [summary.MANIFEST] + summary.EVIDENCE_FILES)
        with MutableRoot() as scratch:
            acceptance = scratch.read_acceptance()
            acceptance["ct_evidence"]["derived_from"] = acceptance["ct_evidence"]["derived_from"][
                :-1
            ]
            scratch.write_acceptance(acceptance)
            self.assertIn(
                "acceptance.json `derived_from` is not the manifest plus this generator's "
                "evidence-file list",
                summary.check_acceptance(scratch.directory),
            )

    def test_a_stage_without_a_depth_rank_fails_closed(self):
        manifest = load_manifest()
        manifest["stage_vocabulary"]["stages"].append("invented_stage")
        with MutableRoot() as scratch:
            scratch.write_manifest(manifest)
            with self.assertRaises(SystemExit):
                summary.summarize(scratch.directory)


if __name__ == "__main__":
    unittest.main()

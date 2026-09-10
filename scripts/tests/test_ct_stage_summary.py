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
        matrix copy is hand-carried commentary no derivation reads; the
        acceptance record's copy is now checked, in both derivations.
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

    def test_the_historical_stale_bucket_block_is_refused(self):
        """Card HYGIENE-16, finding `D02-BUCKETS-STALE`, as a regression.

        The committed record and the derivation agree today, so a test that only
        reads the tree cannot show that a *re-inflated* hand edit would be
        caught. This restores the exact historical shape — the four cases wave 8
        mapped still counted — into a scratch acceptance record and requires the
        drift to be reported by field name. The sum moves with `ct_unmapped`
        (22 against 18 when card HYGIENE-16 wrote it, 21 against 17 since the
        wave-17 CT lane mapped CT40); what is pinned is the four undecremented
        rows, not the total.
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
        self.assertEqual(sum(stale.values()), derived["ct_unmapped"] + 4)
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
        drives it directly — rewriting the archive rows of the lane-measured
        cases to an invented bucket must change neither the counts nor the
        sources. The wave-17 CT lane mapped CT40, so the set is CT41/CT42 and
        CT40 is asserted out of the label map entirely.
        """
        before_counts = summary.unmapped_bucket_counts(ROOT)
        before_sources = summary.unmapped_label_sources(ROOT)
        coverage = json.loads((ROOT / summary.TRIAGE).read_text())
        measured = {"CT41", "CT42"}
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
            self.assertEqual(labels["CT41"]["archived_label"], "invented-archive-label")
            self.assertNotEqual(labels["CT41"]["label"], "invented-archive-label")
            self.assertEqual(labels["CT44"]["label_source"], summary.ARCHIVE_SOURCE)
            # A mapped case has no label at all, whatever the archive says about
            # it: CT40 keeps its archived row and its retired lane row, and
            # neither reaches the published map.
            self.assertNotIn("CT40", labels)

    # ------------------------------------------------------------------
    # Card CT-LABELS, finding D02-BUCKETS-LABELS: the label, not just the count.
    # ------------------------------------------------------------------

    #: The lanes that measured a still-unmapped case's bucket, the wave-17 lane
    #: that mapped CT40 and retired its wave-9 row, and an evidence record that
    #: measures no case (the natural place to stage a conflicting second claim —
    #: it is no longer the newest file, only the newest that labels nothing).
    WAVE3 = "docs/reports/remaining-wave3-ct-integration/ct_stage_evidence.json"
    WAVE9 = "docs/reports/wave9-ctwin/ct_stage_evidence.json"
    WAVE13 = "docs/reports/wave13-ct34/ct_stage_evidence.json"
    NEWEST = "docs/reports/remaining-ct34-sanitized/ct_stage_evidence.json"
    WAVE17 = "docs/reports/wave17-ct40/ct_stage_evidence.json"

    def test_a_measured_label_wins_over_the_archive_and_says_so(self):
        """The resolution rule, on the real tree.

        Four of the seventeen unmapped cases carry a lane's measured bucket; the
        other thirteen still read card T00's 2026-09-07 analysis, and the
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
                "CT41": self.WAVE17,
                "CT42": self.WAVE17,
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
        # (`CT-LABELS-T00-REVERT`), so the two halves now differ for these
        # cases and the assertion is inverted deliberately: the current label
        # comes from the lane's own record, T00's original text is retained
        # beside it, and the tally did not move in either direction. CT40 left
        # the set entirely when the wave-17 CT lane mapped it — the label the
        # wave-9 record holds for it is retired, not deleted, and the case has
        # no published blocker any more.
        self.assertNotIn("CT40", labels)
        for case in ["CT41", "CT42"]:
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
            self.assertEqual(sum(counts.values()), 17)
            self.assertIn(
                "acceptance.json `unmapped_bucket_counts` is not what the manifest derives",
                summary.check_acceptance(scratch.directory),
            )

    # ------------------------------------------------------------------
    # Wave-17 CT lane: retiring an earlier lane's row instead of editing it.
    # ------------------------------------------------------------------

    def test_a_retired_row_is_admitted_and_publishes_no_label(self):
        """The admitted direction, on the real tree.

        The wave-9 record still carries CT40's `unmapped_reasons` row, byte for
        byte; the wave-17 record declares the retirement. The row is therefore
        admitted without publishing a blocker for a case that has none, and the
        earlier record needed no edit — the whole point of the mechanism.
        """
        wave9 = json.loads((ROOT / self.WAVE9).read_text())
        self.assertIn("CT40", wave9["unmapped_reasons"])
        wave17 = json.loads((ROOT / self.WAVE17).read_text())
        self.assertEqual(wave17["retired_unmapped_reasons"]["CT40"]["retires"], self.WAVE9)
        self.assertNotIn("CT40", summary.unmapped_labels(ROOT))
        self.assertNotIn("CT40", summary.unmapped_label_sources(ROOT))

    def test_a_row_for_a_mapped_case_still_fails_closed_without_a_retirement(self):
        """Silence is not a retirement.

        Dropping the declaration leaves exactly the pre-wave-17 state: the
        wave-9 row names a case the manifest maps, and the derivation refuses.
        """
        with MutableRoot() as scratch:
            data = scratch.read_evidence(self.WAVE17)
            del data["retired_unmapped_reasons"]
            scratch.write_evidence(self.WAVE17, data)
            with self.assertRaises(SystemExit):
                summary.unmapped_labels(scratch.directory)

    def test_retiring_a_still_unmapped_case_fails_closed(self):
        """The direction that would silently drop a live label."""
        with MutableRoot() as scratch:
            data = scratch.read_evidence(self.WAVE17)
            data["retired_unmapped_reasons"]["CT46"] = {
                "retires": self.WAVE13,
                "mapped_by": "nothing — CT46 is still unmapped",
            }
            scratch.write_evidence(self.WAVE17, data)
            with self.assertRaises(SystemExit):
                summary.unmapped_labels(scratch.directory)

    def test_a_stale_or_vacuous_retirement_fails_closed(self):
        """Three ways to name the wrong record, each refused."""
        with MutableRoot() as scratch:
            # Stale: names a record that carries no row for this case.
            data = scratch.read_evidence(self.WAVE17)
            data["retired_unmapped_reasons"]["CT40"]["retires"] = self.WAVE13
            scratch.write_evidence(self.WAVE17, data)
            with self.assertRaises(SystemExit):
                summary.unmapped_labels(scratch.directory)

            # Not a stage-evidence record at all.
            data["retired_unmapped_reasons"]["CT40"]["retires"] = "docs/reports/wave9-ctwin/README.md"
            scratch.write_evidence(self.WAVE17, data)
            with self.assertRaises(SystemExit):
                summary.unmapped_labels(scratch.directory)

            # Vacuous: no record publishes a bucket for this case at all.
            data["retired_unmapped_reasons"] = {
                "CT34": {"retires": self.WAVE9, "mapped_by": "the wave-13 CT lane"}
            }
            scratch.write_evidence(self.WAVE17, data)
            with self.assertRaises(SystemExit):
                summary.unmapped_labels(scratch.directory)

    def test_a_retirement_of_a_later_record_fails_closed(self):
        """A record cannot retire a row that lands after it.

        Staged the only way it is reachable: the wave-9 record is made to retire
        the wave-17 record's own row for a case, which is not an earlier file.
        """
        with MutableRoot() as scratch:
            wave17 = scratch.read_evidence(self.WAVE17)
            wave17["unmapped_reasons"]["CT40"] = {
                "bucket": "a later row for the mapped case",
                "reason": "staged",
                "measured_by": "nothing",
            }
            del wave17["retired_unmapped_reasons"]
            scratch.write_evidence(self.WAVE17, wave17)
            wave9 = scratch.read_evidence(self.WAVE9)
            wave9["retired_unmapped_reasons"] = {
                "CT40": {"retires": self.WAVE17, "mapped_by": "the wave-17 CT lane"}
            }
            scratch.write_evidence(self.WAVE9, wave9)
            with self.assertRaises(SystemExit):
                summary.unmapped_labels(scratch.directory)

    def test_two_records_retiring_one_case_fail_closed(self):
        with MutableRoot() as scratch:
            newest = scratch.read_evidence(self.NEWEST)
            newest["retired_unmapped_reasons"] = {
                "CT40": {"retires": self.WAVE9, "mapped_by": "a second claim on the retirement"}
            }
            scratch.write_evidence(self.NEWEST, newest)
            with self.assertRaises(SystemExit):
                summary.unmapped_labels(scratch.directory)

    def test_a_retirement_missing_a_required_field_fails_closed(self):
        for field in ["retires", "mapped_by"]:
            with self.subTest(field=field), MutableRoot() as scratch:
                data = scratch.read_evidence(self.WAVE17)
                data["retired_unmapped_reasons"]["CT40"][field] = "   "
                scratch.write_evidence(self.WAVE17, data)
                with self.assertRaises(SystemExit):
                    summary.unmapped_labels(scratch.directory)

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

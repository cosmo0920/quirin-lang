"""Tests for the non-CT sections of docs/reports/remaining-d02/build_matrix.py.

Card D02-GEN-REST. Before this card only `ct_conformance` was derived: the
requirement rows, the unverified-spec dispositions, the header prose and the
named follow-ons were Python literals from the original D02 wave, and the gate
block hard-coded its counts. A whole-file regeneration therefore reverted every
later reconciliation - the literal held eleven follow-ons against the committed
nineteen, and `provider_tests: 47` against the real 67.

The properties checked here are the two the repair has to have:

  * a regeneration PRESERVES the sections the generator does not own, so an
    integrator's later reconciliation survives it, and
  * a regeneration is a FIXED POINT for the sections it does own, so running it
    twice cannot drift.

Everything runs into a scratch file; the committed matrix is never written.
"""

import json
import pathlib
import shutil
import subprocess
import sys
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[2]
GENERATOR = ROOT / "docs/reports/remaining-d02/build_matrix.py"
COMMITTED = ROOT / "docs/reviews/r0_3/acceptance_matrix.json"

# Sections the generator carries verbatim, and the ones it recomputes. These
# lists are the test's own statement of the contract; the generator publishes
# the same split in `section_ownership`, and one test below compares them so
# neither can be edited alone.
HAND_MAINTAINED = ["header", "unverified_specs", "requirements", "named_followups"]
INTEGRATOR_SUPPLIED = ["updated"]
DERIVED = [
    "schema_version",
    "card",
    "generated",
    "report",
    "acceptance_record",
    "section_ownership",
    "dimensions",
    "derivation",
    "ct_conformance",
    "workspace",
]
MIXED = ["versions", "corpus", "validation"]

DERIVED_FIELDS = {
    "versions": ["qcore_schemas_rust_api", "provider_packages"],
    "corpus": [
        "contract_modules",
        "contract_modules_pinned_by",  # generator constant

        "contract_clause_pin",
        "contract_modules_per_directory",
        "libevent_submodule_pin",
    ],
    "validation": [
        "lean_audits",
        "lean_audit_files",
        "full_language_conformance",
        "measurement_provenance",
    ],
}


def generate(out, previous=None, extra=()):
    command = [sys.executable, str(GENERATOR), str(ROOT), "--out", str(out), *extra]
    if previous is not None:
        command += ["--previous", str(previous)]
    finished = subprocess.run(command, capture_output=True, text=True)
    return finished


def generate_ok(test, out, previous=None, extra=()):
    finished = generate(out, previous, extra)
    test.assertEqual(finished.returncode, 0, finished.stderr)
    return json.loads(pathlib.Path(out).read_text())


class BuildMatrixSections(unittest.TestCase):
    def test_regeneration_preserves_the_sections_the_generator_does_not_own(self):
        committed = json.loads(COMMITTED.read_text())
        with tempfile.TemporaryDirectory(prefix="qcore-matrix-") as scratch:
            out = pathlib.Path(scratch) / "acceptance_matrix.json"
            produced = generate_ok(self, out)
        for section in HAND_MAINTAINED + INTEGRATOR_SUPPLIED:
            self.assertEqual(
                produced[section],
                committed[section],
                f"regenerating clobbered the hand-maintained section {section!r}",
            )
        for section in MIXED:
            carried = {
                key: value
                for key, value in produced[section].items()
                if key not in DERIVED_FIELDS[section]
            }
            expected = {
                key: value
                for key, value in committed[section].items()
                if key not in DERIVED_FIELDS[section]
            }
            self.assertEqual(carried, expected, f"regenerating clobbered {section} prose")

    def test_regeneration_is_a_fixed_point_for_the_sections_it_owns(self):
        with tempfile.TemporaryDirectory(prefix="qcore-matrix-") as scratch:
            first = pathlib.Path(scratch) / "first.json"
            second = pathlib.Path(scratch) / "second.json"
            one = generate_ok(self, first)
            two = generate_ok(self, second, previous=first)
        # `carried_from` records WHERE this run read from, which is an
        # environment fact and differs by construction between the two runs
        # (the committed matrix, then the scratch file); everything else,
        # including the gate provenance sentence, must be identical.
        for produced in (one, two):
            produced["section_ownership"].pop("carried_from")
        self.assertEqual(one, two, "a second regeneration over the generator's own output drifted")

    def test_every_emitted_section_is_labelled_exactly_once(self):
        with tempfile.TemporaryDirectory(prefix="qcore-matrix-") as scratch:
            out = pathlib.Path(scratch) / "acceptance_matrix.json"
            produced = generate_ok(self, out)
        ownership = produced["section_ownership"]["sections"]
        self.assertEqual(set(ownership), set(produced))
        for section in DERIVED:
            self.assertEqual(ownership[section]["owner"], "derived", section)
        for section in HAND_MAINTAINED:
            self.assertEqual(ownership[section]["owner"], "hand_maintained", section)
        for section in INTEGRATOR_SUPPLIED:
            self.assertEqual(ownership[section]["owner"], "integrator_supplied", section)
        for section in MIXED:
            self.assertEqual(ownership[section]["owner"], "mixed", section)
            fields = produced["section_ownership"]["fields"]
            self.assertIn(f"{section}.*", fields, f"{section} has no catch-all field owner")
            for field in DERIVED_FIELDS[section]:
                self.assertEqual(fields[f"{section}.{field}"]["owner"], "derived", field)

    def test_the_derived_facts_are_recomputed_rather_than_carried(self):
        """The committed matrix carries stale derived values; they must move."""
        committed = json.loads(COMMITTED.read_text())
        with tempfile.TemporaryDirectory(prefix="qcore-matrix-") as scratch:
            out = pathlib.Path(scratch) / "acceptance_matrix.json"
            produced = generate_ok(self, out)
        schemas_version = next(
            line.split('"')[1]
            for line in (ROOT / "crates/qcore-schemas/Cargo.toml").read_text().splitlines()
            if line.startswith("version")
        )
        self.assertEqual(produced["versions"]["qcore_schemas_rust_api"], schemas_version)
        packages = {}
        for template in sorted((ROOT / "providers").glob("qcore-provider-*/provider-template.json")):
            declared = json.loads(template.read_text())
            packages[declared["plugin_id"]] = declared["package_version"]
        self.assertEqual(produced["versions"]["provider_packages"], packages)
        self.assertEqual(
            produced["corpus"]["contract_modules"],
            len(list((ROOT / "contract/qcore").glob("*/*.qcore"))),
        )
        self.assertEqual(
            produced["validation"]["lean_audit_files"],
            sorted(path.name for path in (ROOT / "proof").glob("Audit*.lean")),
        )
        self.assertEqual(
            produced["validation"]["lean_audits"],
            len(produced["validation"]["lean_audit_files"]),
        )
        # Non-vacuity: the assertions above compare the produced values to the
        # live sources, never to the committed file, so they cannot be
        # comparing a value to itself. The original form of this check
        # asserted the committed values were stale, which was true only until
        # the wave-9 consolidation adopted the regenerated output; the steady
        # state after adoption is agreement, pinned here.
        self.assertEqual(
            [committed["versions"]["qcore_schemas_rust_api"], committed["validation"]["lean_audits"]],
            [produced["versions"]["qcore_schemas_rust_api"], produced["validation"]["lean_audits"]],
        )

    def test_gate_numbers_are_supplied_or_labelled_as_carried(self):
        with tempfile.TemporaryDirectory(prefix="qcore-matrix-") as scratch:
            scratch = pathlib.Path(scratch)
            out = scratch / "carried.json"
            carried = generate_ok(self, out)
            self.assertTrue(
                carried["validation"]["measurement_provenance"].startswith(
                    "carried_from_previous_matrix"
                ),
                carried["validation"]["measurement_provenance"],
            )
            gate = scratch / "gate.json"
            gate.write_text(
                json.dumps(
                    {
                        "note": "measured for this test",
                        "workspace_tests": 1234,
                        "provider_tests": 67,
                        "python_evidence_validator_tests": 108,
                        "reevaluation_probe_exit": 0,
                        "reevaluation_followup_exit": 0,
                        "cargo_fmt": "clean",
                        "cargo_clippy": "clean",
                        "standing_gate": "passed",
                    }
                )
            )
            supplied = generate_ok(self, scratch / "supplied.json", extra=["--gate", str(gate)])
        self.assertEqual(supplied["validation"]["workspace_tests"], 1234)
        self.assertEqual(supplied["validation"]["provider_tests"], 67)
        self.assertTrue(
            supplied["validation"]["measurement_provenance"].startswith("integrator_supplied")
        )

    def test_a_partial_gate_is_refused_rather_than_merged(self):
        with tempfile.TemporaryDirectory(prefix="qcore-matrix-") as scratch:
            scratch = pathlib.Path(scratch)
            gate = scratch / "gate.json"
            gate.write_text(json.dumps({"workspace_tests": 1234}))
            finished = generate(scratch / "out.json", extra=["--gate", str(gate)])
        self.assertNotEqual(finished.returncode, 0)
        self.assertIn("missing", finished.stderr + finished.stdout)

    def test_a_gate_key_the_generator_does_not_publish_is_refused(self):
        with tempfile.TemporaryDirectory(prefix="qcore-matrix-") as scratch:
            scratch = pathlib.Path(scratch)
            gate = scratch / "gate.json"
            gate.write_text(
                json.dumps(
                    {
                        "note": "n",
                        "workspace_tests": 1,
                        "provider_tests": 1,
                        "python_evidence_validator_tests": 1,
                        "reevaluation_probe_exit": 0,
                        "reevaluation_followup_exit": 0,
                        "cargo_fmt": "clean",
                        "cargo_clippy": "clean",
                        "standing_gate": "passed",
                        "invented_gate": 1,
                    }
                )
            )
            finished = generate(scratch / "out.json", extra=["--gate", str(gate)])
        self.assertNotEqual(finished.returncode, 0)
        self.assertIn("invented_gate", finished.stderr + finished.stdout)

    def test_a_missing_previous_matrix_refuses_rather_than_writing_a_stripped_file(self):
        with tempfile.TemporaryDirectory(prefix="qcore-matrix-") as scratch:
            scratch = pathlib.Path(scratch)
            out = scratch / "out.json"
            finished = generate(out, previous=scratch / "absent.json")
        self.assertNotEqual(finished.returncode, 0)
        self.assertIn("does not exist", finished.stderr + finished.stdout)
        self.assertFalse(out.exists())

    def test_a_previous_matrix_missing_a_carried_section_refuses(self):
        committed = json.loads(COMMITTED.read_text())
        del committed["named_followups"]
        with tempfile.TemporaryDirectory(prefix="qcore-matrix-") as scratch:
            scratch = pathlib.Path(scratch)
            previous = scratch / "previous.json"
            previous.write_text(json.dumps(committed))
            finished = generate(scratch / "out.json", previous=previous)
        self.assertNotEqual(finished.returncode, 0)
        self.assertIn("named_followups", finished.stderr + finished.stdout)

    def test_a_widened_support_vocabulary_refuses(self):
        """The header publishes the vocabulary the rows are validated against."""
        committed = json.loads(COMMITTED.read_text())
        committed["header"]["support_status_vocabulary"]["probably_fine"] = "invented"
        with tempfile.TemporaryDirectory(prefix="qcore-matrix-") as scratch:
            scratch = pathlib.Path(scratch)
            previous = scratch / "previous.json"
            previous.write_text(json.dumps(committed))
            finished = generate(scratch / "out.json", previous=previous)
        self.assertNotEqual(finished.returncode, 0)
        self.assertIn("support_status_vocabulary", finished.stderr + finished.stdout)

    def test_a_carried_requirement_row_with_an_invented_status_refuses(self):
        committed = json.loads(COMMITTED.read_text())
        committed["requirements"][0]["support"]["model"]["status"] = "mostly_executed"
        with tempfile.TemporaryDirectory(prefix="qcore-matrix-") as scratch:
            scratch = pathlib.Path(scratch)
            previous = scratch / "previous.json"
            previous.write_text(json.dumps(committed))
            finished = generate(scratch / "out.json", previous=previous)
        self.assertNotEqual(finished.returncode, 0)
        self.assertIn("mostly_executed", finished.stderr + finished.stdout)

    def test_a_carried_requirement_row_missing_an_evidence_class_refuses(self):
        committed = json.loads(COMMITTED.read_text())
        del committed["requirements"][0]["support"]["observation"]
        with tempfile.TemporaryDirectory(prefix="qcore-matrix-") as scratch:
            scratch = pathlib.Path(scratch)
            previous = scratch / "previous.json"
            previous.write_text(json.dumps(committed))
            finished = generate(scratch / "out.json", previous=previous)
        self.assertNotEqual(finished.returncode, 0)

    # ------------------------------------------------------------------
    # Card GEN-CTREASONS, follow-on D02-GEN-CTREASONS.
    # ------------------------------------------------------------------

    def test_the_committed_reasons_map_is_a_fixed_point_of_the_generator(self):
        """The key the generator used to drop is now derived and committed.

        Deliberately scoped to this one derived key rather than to the whole
        `ct_conformance` section: a CT lane that maps a case moves that
        section's rows, and a test pinning them would fail on the lane's commit
        instead of on a defect. Nothing here names a case id or a count — both
        sides are read at run time.
        """
        committed = json.loads(COMMITTED.read_text())
        with tempfile.TemporaryDirectory(prefix="qcore-matrix-") as scratch:
            out = pathlib.Path(scratch) / "acceptance_matrix.json"
            produced = generate_ok(self, out)
        key = "newly_decidable_not_mapped_reasons"
        self.assertIn(key, produced["ct_conformance"], "the generator dropped the map again")
        self.assertEqual(
            produced["ct_conformance"][key],
            committed["ct_conformance"].get(key),
            "regenerating moved the derived reasons map; the committed matrix is not a "
            "fixed point of its own generator",
        )
        # Non-vacuity in the direction that matters: the map is not empty, its
        # keys are exactly the companion list the same section publishes, and
        # its rows are objects carrying the record that decided them.
        rows = {
            case: row for case, row in produced["ct_conformance"][key].items()
            if not case.startswith("_")
        }
        self.assertTrue(rows)
        self.assertEqual(
            list(rows), produced["ct_conformance"]["newly_decidable_not_mapped"]
        )
        for row in rows.values():
            self.assertEqual(
                sorted(row), ["label", "label_source", "measured_by", "reason"]
            )

    @unittest.skipUnless(shutil.which("git"), "git is required to drive the carry step")
    def test_the_wave16_carry_step_retires_itself_once_the_map_is_derived(self):
        """`docs/reports/wave16-corpus/readd_ct_reasons.py` is superseded, loudly.

        The wave-16 consolidation carried this map forward by hand after every
        regeneration, guarding only that a carried row does not describe a
        mapped case — a guard at the carry, not a derivation. Its author left
        the retirement tripwire in it (`assert KEY not in ct`), so the derivation
        landing here retires it by name rather than silently: the archived
        script's bytes are untouched, and running it now refuses.

        Driven in a scratch repository so the committed matrix is never written.
        """
        carry = ROOT / "docs/reports/wave16-corpus/readd_ct_reasons.py"
        self.assertTrue(carry.is_file(), "the carry step this test supersedes is missing")
        with tempfile.TemporaryDirectory(prefix="qcore-carry-") as scratch:
            scratch = pathlib.Path(scratch)
            (scratch / "docs/reports/wave16-corpus").mkdir(parents=True)
            (scratch / "docs/reviews/r0_3").mkdir(parents=True)
            shutil.copyfile(carry, scratch / "docs/reports/wave16-corpus/readd_ct_reasons.py")
            matrix = scratch / "docs/reviews/r0_3/acceptance_matrix.json"
            carried = {
                "ct_conformance": {
                    "ct_rows": [],
                    "newly_decidable_not_mapped": [],
                    "newly_decidable_not_mapped_reasons": {"_note": "hand-maintained"},
                }
            }
            matrix.write_text(json.dumps(carried, indent=1) + "\n")
            for command in (
                ["git", "init", "-q"],
                ["git", "add", "-A"],
                [
                    "git",
                    "-c", "user.email=card@example.invalid",
                    "-c", "user.name=card",
                    "commit", "-qm", "scratch",
                ],
            ):
                subprocess.run(command, cwd=scratch, check=True, capture_output=True)
            # The state after this card: the generator emits the key itself.
            matrix.write_text(json.dumps(carried, indent=1) + "\n")
            finished = subprocess.run(
                [sys.executable, "-B", str(scratch / "docs/reports/wave16-corpus/readd_ct_reasons.py")],
                capture_output=True,
                text=True,
            )
        self.assertNotEqual(finished.returncode, 0, "the retired carry step still ran")
        self.assertIn("retire D02-GEN-CTREASONS", finished.stderr)

    def test_the_positional_workspace_count_still_works(self):
        """T00-CT-SCHEMA documents `build_matrix.py . <workspace_test_count>`."""
        with tempfile.TemporaryDirectory(prefix="qcore-matrix-") as scratch:
            out = pathlib.Path(scratch) / "out.json"
            finished = subprocess.run(
                [sys.executable, str(GENERATOR), str(ROOT), "4242", "--out", str(out)],
                capture_output=True,
                text=True,
            )
            self.assertEqual(finished.returncode, 0, finished.stderr)
            produced = json.loads(out.read_text())
        self.assertEqual(produced["validation"]["workspace_tests"], 4242)
        self.assertIn("positionally", produced["validation"]["measurement_provenance"])


if __name__ == "__main__":
    unittest.main()

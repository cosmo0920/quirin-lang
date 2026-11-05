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
import os
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
    return json.loads(pathlib.Path(out).read_text(encoding="utf-8"))


# The nine keys the generator cannot measure and carries from --gate.
MEASURED_GATE_KEYS = {
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
CLANG_BANNER = "Ubuntu clang version 21.1.8 (6ubuntu1)"


def lean_audits_on_disk():
    return len(list((ROOT / "proof").glob("Audit*.lean")))


# CARD-GATE-NEW-AUDIT-MATRIX-COUNT (card GATE-SCOPE-PRECISE-SELECTION):
# scripts/gate_scope.py gives a card's Python suite the card's base in this
# variable; the standing gate and `gate_scope.py --landing` never set it.
CARD_GATE_BASE_ENV = "QUIRIN_CARD_GATE_BASE"


def git_at(*args):
    finished = subprocess.run(["git", "-C", str(ROOT), *args], capture_output=True, text=True)
    if finished.returncode != 0:
        raise AssertionError(f"git {' '.join(args)}: {finished.stderr.strip()}")
    return finished.stdout


def audit_drift(base_tree, base_committed, committed_files, produced_files):
    """(drift, allowed): the audit names a card's gate finds between the
    committed matrix and the tree, and the ones it may excuse -- the drift its
    base already had, plus the audits the card itself adds or removes. Nothing
    else: a card that drops an audit from the matrix, or leaves a stale matrix
    of its own making, still fails."""
    drift = set(produced_files) ^ set(committed_files)
    allowed = (set(base_tree) ^ set(base_committed)) | (set(produced_files) ^ set(base_tree))
    return drift, allowed


def lean_audit_drift_allowed_on_a_card(base, committed_files, produced_files):
    base_tree = sorted(
        name for name in git_at("ls-tree", "--name-only", f"{base}:proof").splitlines()
        if name.startswith("Audit") and name.endswith(".lean"))
    base_matrix = json.loads(git_at("show", f"{base}:docs/reviews/r0_3/acceptance_matrix.json"))
    return audit_drift(base_tree, base_matrix["validation"]["lean_audit_files"],
                       committed_files, produced_files)


class CardGateAuditDrift(unittest.TestCase):
    """CARD-GATE-NEW-AUDIT-MATRIX-COUNT: what a card gate excuses, and what not."""

    def excused(self, base_tree, base_committed, committed, produced):
        drift, allowed = audit_drift(base_tree, base_committed, committed, produced)
        return drift <= allowed

    def test_a_card_adding_an_audit_passes_its_own_gate(self):
        self.assertTrue(self.excused(["A"], ["A"], ["A"], ["A", "X"]))

    def test_a_card_removing_an_audit_passes_its_own_gate(self):
        self.assertTrue(self.excused(["A", "B"], ["A", "B"], ["A", "B"], ["A"]))

    def test_the_base_s_own_drift_is_not_the_card_s(self):
        self.assertTrue(self.excused(["A", "B"], ["A"], ["A"], ["A", "B"]))

    def test_a_matrix_edit_that_drops_an_unchanged_audit_fails(self):
        self.assertFalse(self.excused(["A", "B"], ["A", "B"], ["A"], ["A", "B"]))

    def test_a_matrix_naming_an_audit_nobody_has_fails(self):
        self.assertFalse(self.excused(["A"], ["A"], ["A", "Z"], ["A"]))

    def test_the_base_is_read_from_git_and_a_bad_base_fails_closed(self):
        if shutil.which("git") is None:
            self.skipTest("git is not installed")
        with self.assertRaises(AssertionError):
            lean_audit_drift_allowed_on_a_card("no-such-revision-r19a", [], [])


def write_gate(scratch, lean_audits=None, clang=None, recording=CLANG_BANNER, exit_code="0",
               omit=(), extra=None):
    """Write the gate.json a close publishes since the wave-38 close, beside its
    Clang recording: the nine measured keys plus the two CHECKED keys -
    `lean_audits` (checked against proof/Audit*.lean) and `clang` (a presence
    claim checked against clang-version.txt / clang-version.exit beside the
    gate). `recording=None` writes no recording; `omit` drops keys."""
    gate = dict(MEASURED_GATE_KEYS)
    gate["lean_audits"] = lean_audits_on_disk() if lean_audits is None else lean_audits
    gate["clang"] = f"present: {CLANG_BANNER}" if clang is None else clang
    for key in omit:
        gate.pop(key, None)
    if extra:
        gate.update(extra)
    if recording is not None:
        (scratch / "clang-version.txt").write_text(
            recording + "\nQCORE_CLANG=<unset>\n", encoding="utf-8"
        )
        (scratch / "clang-version.exit").write_text(exit_code + "\n", encoding="utf-8")
    path = scratch / "gate.json"
    path.write_text(json.dumps(gate), encoding="utf-8")
    return path


class BuildMatrixSections(unittest.TestCase):
    def test_regeneration_preserves_the_sections_the_generator_does_not_own(self):
        committed = json.loads(COMMITTED.read_text(encoding="utf-8"))
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
        committed = json.loads(COMMITTED.read_text(encoding="utf-8"))
        with tempfile.TemporaryDirectory(prefix="qcore-matrix-") as scratch:
            out = pathlib.Path(scratch) / "acceptance_matrix.json"
            produced = generate_ok(self, out)
        schemas_version = next(
            line.split('"')[1]
            for line in (ROOT / "crates/qcore-schemas/Cargo.toml")
            .read_text(encoding="utf-8")
            .splitlines()
            if line.startswith("version")
        )
        self.assertEqual(produced["versions"]["qcore_schemas_rust_api"], schemas_version)
        packages = {}
        for template in sorted((ROOT / "providers").glob("qcore-provider-*/provider-template.json")):
            declared = json.loads(template.read_text(encoding="utf-8"))
            packages[declared["plugin_id"]] = declared["package_version"]
        self.assertEqual(produced["versions"]["provider_packages"], packages)
        self.assertEqual(
            produced["corpus"]["contract_modules"],
            sum(sum(path.suffix in {".qcore", ".qv"}
                    for path in (ROOT / "contract" / directory).rglob("*"))
                for directory in json.loads((ROOT / "contract/corpus.json").read_text())["roots"]),
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
        base = os.environ.get(CARD_GATE_BASE_ENV)
        if not base:
            self.assertEqual(
                [committed["versions"]["qcore_schemas_rust_api"],
                 committed["validation"]["lean_audits"]],
                [produced["versions"]["qcore_schemas_rust_api"],
                 produced["validation"]["lean_audits"]],
            )
            return
        # A card's gate: the matrix is the integrator's record, regenerated at
        # landing, so it may lag the tree by the card's own audits (and by the
        # drift its base had); the landing gate keeps the exact check above.
        self.assertEqual(committed["versions"]["qcore_schemas_rust_api"],
                         produced["versions"]["qcore_schemas_rust_api"])
        committed_files = committed["validation"]["lean_audit_files"]
        self.assertEqual(committed["validation"]["lean_audits"], len(committed_files))
        drift, allowed = lean_audit_drift_allowed_on_a_card(
            base, committed_files, produced["validation"]["lean_audit_files"])
        self.assertLessEqual(drift, allowed,
                             f"audit names the card gate cannot excuse: {sorted(drift - allowed)}")

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
            gate = write_gate(scratch)
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
            gate.write_text(json.dumps({"workspace_tests": 1234}), encoding="utf-8")
            finished = generate(scratch / "out.json", extra=["--gate", str(gate)])
        self.assertNotEqual(finished.returncode, 0)
        self.assertIn("missing", finished.stderr + finished.stdout)

    def test_a_gate_key_the_generator_does_not_publish_is_refused(self):
        with tempfile.TemporaryDirectory(prefix="qcore-matrix-") as scratch:
            scratch = pathlib.Path(scratch)
            gate = write_gate(scratch, extra={"invented_gate": 1})
            finished = generate(scratch / "out.json", extra=["--gate", str(gate)])
        self.assertNotEqual(finished.returncode, 0)
        self.assertIn("invented_gate", finished.stderr + finished.stdout)

    # A close's gate.json MAY record the Lean audit count it measured, so the
    # figure has a machine-readable source in the record that publishes it
    # (GATE-ROW-FIGURE-ATTRIBUTION-DRIFT, card DOCS-COMPACT-7). The generator
    # does not carry that number: it CHECKS it against proof/Audit*.lean and
    # refuses a disagreement, so a drifted figure is refused where it would
    # have been written. The published field stays derived either way.

    def test_a_recorded_lean_count_that_matches_the_files_on_disk_is_accepted(self):
        on_disk = lean_audits_on_disk()
        with tempfile.TemporaryDirectory(prefix="qcore-matrix-") as scratch:
            scratch = pathlib.Path(scratch)
            gate = write_gate(scratch, lean_audits=on_disk)
            supplied = generate_ok(self, scratch / "out.json", extra=["--gate", str(gate)])
        self.assertEqual(supplied["validation"]["lean_audits"], on_disk)
        self.assertEqual(supplied["validation"]["workspace_tests"], 1234)
        self.assertTrue(
            supplied["validation"]["measurement_provenance"].startswith("integrator_supplied")
        )
        # Neither checked key is copied into the published block.
        self.assertNotIn("clang", supplied["validation"])

    def test_a_recorded_lean_count_that_disagrees_with_the_files_on_disk_is_refused(self):
        on_disk = lean_audits_on_disk()
        with tempfile.TemporaryDirectory(prefix="qcore-matrix-") as scratch:
            scratch = pathlib.Path(scratch)
            gate = write_gate(scratch, lean_audits=on_disk + 2)
            finished = generate(scratch / "out.json", extra=["--gate", str(gate)])
            written = (scratch / "out.json").exists()
        self.assertNotEqual(finished.returncode, 0)
        self.assertIn("lean_audits", finished.stderr + finished.stdout)
        self.assertIn(str(on_disk), finished.stderr + finished.stdout)
        self.assertFalse(written, "a refused gate must not write a matrix")

    # Since the wave-38 close the two checked keys are REQUIRED in a --gate
    # file. Card STABILITY-REEVAL-3 measured that a gate.json omitting
    # `lean_audits` passed without comment - the unchecked figure the key was
    # added to prevent (LEAN-AUDIT-GATE-KEY-OPTIONAL) - and that no close's
    # gate.json recorded whether Clang was present when its workspace count
    # was taken (CLANG-GUARDED-GREEN-FAMILY-750). The refusal is about the one
    # gate file a REGENERATION names; a frozen close's gate.json is never read.

    def _refused(self, scratch, gate, *needles):
        finished = generate(scratch / "out.json", extra=["--gate", str(gate)])
        written = (scratch / "out.json").exists()
        self.assertNotEqual(finished.returncode, 0, finished.stdout)
        for needle in needles:
            self.assertIn(needle, finished.stderr + finished.stdout)
        self.assertFalse(written, "a refused gate must not write a matrix")

    def test_a_gate_file_lacking_lean_audits_is_refused(self):
        with tempfile.TemporaryDirectory(prefix="qcore-matrix-") as scratch:
            scratch = pathlib.Path(scratch)
            gate = write_gate(scratch, omit=("lean_audits",))
            self._refused(scratch, gate, "lean_audits", "LEAN-AUDIT-GATE-KEY-OPTIONAL")

    def test_a_gate_file_lacking_clang_is_refused(self):
        with tempfile.TemporaryDirectory(prefix="qcore-matrix-") as scratch:
            scratch = pathlib.Path(scratch)
            gate = write_gate(scratch, omit=("clang",))
            self._refused(scratch, gate, "clang", "CLANG-GUARDED-GREEN-FAMILY-750")

    def test_a_clang_claim_agreeing_with_its_recording_is_accepted(self):
        with tempfile.TemporaryDirectory(prefix="qcore-matrix-") as scratch:
            scratch = pathlib.Path(scratch)
            gate = write_gate(scratch, clang=f"present: {CLANG_BANNER}")
            supplied = generate_ok(self, scratch / "out.json", extra=["--gate", str(gate)])
            # The recording may also live in a gate/ subdirectory beside the file
            # (card STABILITY-REEVAL-3's layout).
            nested = scratch / "nested"
            (nested / "gate").mkdir(parents=True)
            for name in ("clang-version.txt", "clang-version.exit"):
                (nested / "gate" / name).write_text(
                    (scratch / name).read_text(encoding="utf-8"), encoding="utf-8"
                )
            gate = write_gate(nested, recording=None)
            generate_ok(self, nested / "out.json", extra=["--gate", str(gate)])
        self.assertTrue(
            supplied["validation"]["measurement_provenance"].startswith("integrator_supplied")
        )

    def test_an_absent_clang_claim_agreeing_with_its_recording_is_accepted(self):
        with tempfile.TemporaryDirectory(prefix="qcore-matrix-") as scratch:
            scratch = pathlib.Path(scratch)
            failure = "gate.sh: line 15: clang: command not found"
            gate = write_gate(scratch, clang=f"absent: {failure}", recording=failure, exit_code="127")
            generate_ok(self, scratch / "out.json", extra=["--gate", str(gate)])

    def test_a_clang_claim_disagreeing_with_its_recording_is_refused(self):
        with tempfile.TemporaryDirectory(prefix="qcore-matrix-") as scratch:
            scratch = pathlib.Path(scratch)
            gate = write_gate(scratch, clang="present: Ubuntu clang version 20.1.0")
            self._refused(scratch, gate, "clang", "disagrees with the recording")

    def test_a_clang_claim_with_no_recording_beside_it_is_refused(self):
        with tempfile.TemporaryDirectory(prefix="qcore-matrix-") as scratch:
            scratch = pathlib.Path(scratch)
            gate = write_gate(scratch, recording=None)
            self._refused(scratch, gate, "no clang-version.txt is recorded")

    def test_a_clang_presence_claim_contradicted_by_the_recorded_exit_is_refused(self):
        with tempfile.TemporaryDirectory(prefix="qcore-matrix-") as scratch:
            scratch = pathlib.Path(scratch)
            gate = write_gate(scratch, exit_code="127")
            self._refused(scratch, gate, "claims Clang present", "127")

    def test_a_clang_value_that_is_not_a_presence_claim_is_refused(self):
        with tempfile.TemporaryDirectory(prefix="qcore-matrix-") as scratch:
            scratch = pathlib.Path(scratch)
            for value in (True, "21.1.8", "installed: clang", "present:", "present: "):
                gate = write_gate(scratch, clang=value)
                self._refused(scratch, gate, "presence claim")

    def test_a_missing_previous_matrix_refuses_rather_than_writing_a_stripped_file(self):
        with tempfile.TemporaryDirectory(prefix="qcore-matrix-") as scratch:
            scratch = pathlib.Path(scratch)
            out = scratch / "out.json"
            finished = generate(out, previous=scratch / "absent.json")
        self.assertNotEqual(finished.returncode, 0)
        self.assertIn("does not exist", finished.stderr + finished.stdout)
        self.assertFalse(out.exists())

    def test_a_previous_matrix_missing_a_carried_section_refuses(self):
        committed = json.loads(COMMITTED.read_text(encoding="utf-8"))
        del committed["named_followups"]
        with tempfile.TemporaryDirectory(prefix="qcore-matrix-") as scratch:
            scratch = pathlib.Path(scratch)
            previous = scratch / "previous.json"
            previous.write_text(json.dumps(committed), encoding="utf-8")
            finished = generate(scratch / "out.json", previous=previous)
        self.assertNotEqual(finished.returncode, 0)
        self.assertIn("named_followups", finished.stderr + finished.stdout)

    def test_a_widened_support_vocabulary_refuses(self):
        """The header publishes the vocabulary the rows are validated against."""
        committed = json.loads(COMMITTED.read_text(encoding="utf-8"))
        committed["header"]["support_status_vocabulary"]["probably_fine"] = "invented"
        with tempfile.TemporaryDirectory(prefix="qcore-matrix-") as scratch:
            scratch = pathlib.Path(scratch)
            previous = scratch / "previous.json"
            previous.write_text(json.dumps(committed), encoding="utf-8")
            finished = generate(scratch / "out.json", previous=previous)
        self.assertNotEqual(finished.returncode, 0)
        self.assertIn("support_status_vocabulary", finished.stderr + finished.stdout)

    def test_a_carried_requirement_row_with_an_invented_status_refuses(self):
        committed = json.loads(COMMITTED.read_text(encoding="utf-8"))
        committed["requirements"][0]["support"]["model"]["status"] = "mostly_executed"
        with tempfile.TemporaryDirectory(prefix="qcore-matrix-") as scratch:
            scratch = pathlib.Path(scratch)
            previous = scratch / "previous.json"
            previous.write_text(json.dumps(committed), encoding="utf-8")
            finished = generate(scratch / "out.json", previous=previous)
        self.assertNotEqual(finished.returncode, 0)
        self.assertIn("mostly_executed", finished.stderr + finished.stdout)

    def test_a_carried_requirement_row_missing_an_evidence_class_refuses(self):
        committed = json.loads(COMMITTED.read_text(encoding="utf-8"))
        del committed["requirements"][0]["support"]["observation"]
        with tempfile.TemporaryDirectory(prefix="qcore-matrix-") as scratch:
            scratch = pathlib.Path(scratch)
            previous = scratch / "previous.json"
            previous.write_text(json.dumps(committed), encoding="utf-8")
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
        committed = json.loads(COMMITTED.read_text(encoding="utf-8"))
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
        # Its keys are exactly the companion list the same section publishes,
        # and its rows are objects carrying the record that decided them.
        # CT24 was the last archived-list member still unmapped. Its mapping
        # leaves the committed fixed point empty; CT41 is the sole live
        # unmapped case and is not in this published list.
        rows = {
            case: row for case, row in produced["ct_conformance"][key].items()
            if not case.startswith("_")
        }
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
            matrix.write_text(json.dumps(carried, indent=1) + "\n", encoding="utf-8")
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
            matrix.write_text(json.dumps(carried, indent=1) + "\n", encoding="utf-8")
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
            produced = json.loads(out.read_text(encoding="utf-8"))
        self.assertEqual(produced["validation"]["workspace_tests"], 4242)
        self.assertIn("positionally", produced["validation"]["measurement_provenance"])


class CommittedMatrixIsSelfConsistent(unittest.TestCase):
    """Card STABILITY-REEVAL-1 (wave 34), discharging the first half of
    `D02-DIMENSIONS-FOLLOWUP-COUNT`.

    The two tests above compare GENERATOR OUTPUT against generator output, so
    both are self-consistent by construction and neither can see drift in the
    COMMITTED file. `dimensions.named_followups` therefore sat at 45 against a
    `named_followups` of 47 and no test went red - the residual fired twice, at
    the wave-32 and wave-33 closes, each time on a hand edit and each time
    caught by a person reading the file rather than by the suite.

    These two check the committed matrix itself, which is the artifact every
    other reader trusts. They are deliberately about the file on disk: a
    regeneration repairs both, so a red row here names a hand edit that did not
    go back through the generator (constraint 25).
    """

    def test_each_derived_dimension_matches_the_section_it_counts(self):
        committed = json.loads(COMMITTED.read_text(encoding="utf-8"))
        # Only the dimensions whose name IS a section name; the derived
        # cell/product counts are the other tests' territory.
        for section in ("requirements", "named_followups"):
            self.assertEqual(
                committed["dimensions"][section],
                len(committed[section]),
                f"dimensions.{section} is {committed['dimensions'][section]} against a "
                f"{section} of {len(committed[section])}; regenerate through "
                f"{GENERATOR.relative_to(ROOT)} rather than repairing the count by hand",
            )

    def test_the_committed_matrix_keeps_the_generators_byte_format(self):
        """The second half of the same defect: a hand edit in another format
        re-serialises all ~6,500 lines, so the next real diff is unreadable.
        The generator writes `indent=1, ensure_ascii=False` plus a trailing
        newline (card HYGIENE-16); the committed file must round-trip through
        exactly that.
        """
        raw = COMMITTED.read_text(encoding="utf-8")
        reserialised = json.dumps(json.loads(raw), indent=1, ensure_ascii=False) + "\n"
        self.assertEqual(
            reserialised,
            raw,
            "the committed matrix is not in the generator's byte format; it was edited "
            "by hand with different serialisation settings",
        )


if __name__ == "__main__":
    unittest.main()

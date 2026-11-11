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

"""Card AMENDMENTS-SCHEMA: `scripts/apply_amendments.py` validates
`quirin-amendments/1` fail-closed, applies it as the integrator did by hand,
reads every committed pre-schema file through its normaliser, and replays
c3d8ad3d4 byte for byte. Committed inputs are found from tracked files only
(constraint 29); the replay reads git objects, never the working tree."""
import contextlib
import copy
import io
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(Path(__file__).resolve().parent))
sys.path.insert(0, str(ROOT / "scripts"))
from tracked_files import tracked_files  # noqa: E402
import apply_amendments as tool  # noqa: E402
from test_evaluation_profiles import Fixture as ProfileFixture, commit  # noqa: E402

NOTICE = "> Counts and observations describe their recorded checkpoints."
RECORD = "# Wave 99, first record\n\n[History index](README.md)\n\n" + NOTICE + "\n\n**OLD.** Older paragraph.\n"
STATUS = "# Current status and evidence boundaries\n\nOld status paragraph.\n"
MATRIX = {"schema_version": 1, "named_followups": [
    {"id": "KEEP-ME", "title": "Kept", "why": "Original why.", "source": "docs/reports/a/README.md"},
    {"id": "GONE", "title": "Gone", "why": "To retire.", "source": "docs/reports/b/README.md"},
], "updated": "x"}
DOCUMENT = {
    "schema": "quirin-amendments/1",
    "card": "TEST-CARD",
    "base": "710021bd7",
    "measured_tip": "0123456789abcdef0123456789abcdef01234567",
    "increment_paragraph": "**TEST-CARD.** New paragraph.",
    "status_paragraph": "New status fact.",
    "ledger": [
        {"id": "NEW-ONE", "operation": "register", "title": "New", "text": "Why new.",
         "source": "docs/reports/test-card/README.md"},
        {"id": "KEEP-ME", "operation": "append", "text": "Appended.",
         "source": "docs/reports/test-card/README.md"},
        {"id": "GONE", "operation": "retire", "why": "DISCHARGE (TEST-CARD): done.",
         "source": "docs/reports/test-card/README.md"},
    ],
}
# The only committed file the normaliser refuses, and why (it proposes nothing
# applicable: no increment paragraph, conditional tracking instructions).
REFUSED_LEGACY = {"w48-bu-rk-result-constructors": "proposal-only legacy file"}
PARITY_COMMIT = "c3d8ad3d4"
PARITY_AMENDMENTS = "docs/reports/audit-canonical-paths/integration-amendments.json"


def make_root(directory, record=RECORD):
    root = Path(directory)
    increments = root / "docs/development/increments"
    increments.mkdir(parents=True)
    (increments / "README.md").write_text(
        "# Increment history\n\nCurrent record file: [wave-99.md](wave-99.md). Prepend.\n")
    (increments / "wave-99.md").write_text(record)
    (root / "docs/STATUS.md").write_text(STATUS)
    (root / "docs/reviews/r0_3").mkdir(parents=True)
    (root / "docs/reviews/r0_3/acceptance_matrix.json").write_text(
        json.dumps(MATRIX, indent=1, ensure_ascii=False) + "\n")
    return root


FIXTURE_FILES = ("docs/development/increments/README.md", "docs/development/increments/wave-99.md",
                 "docs/STATUS.md", "docs/reviews/r0_3/acceptance_matrix.json", "amendments.json")


def snapshot(root):
    """The fixture's files by name (no directory walk; constraint 29)."""
    root = Path(root)
    return {root / name: (root / name).read_bytes()
            for name in FIXTURE_FILES if (root / name).is_file()}


def run(root, document, *flags):
    path = Path(root) / "amendments.json"
    path.write_text(json.dumps(document))
    out, err = io.StringIO(), io.StringIO()
    with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
        code = tool.main([str(path), "--root", str(root), *flags])
    return code, out.getvalue(), err.getvalue()


class ApplyTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.root = make_root(self.directory.name)

    def tearDown(self):
        self.directory.cleanup()

    def test_apply_writes_increment_status_and_ledger(self):
        code, out, err = run(self.root, DOCUMENT)
        self.assertEqual(code, 0, err)
        record = (self.root / "docs/development/increments/wave-99.md").read_text()
        self.assertIn(NOTICE + "\n\n**TEST-CARD.** New paragraph.\n\n**OLD.**", record)
        status = (self.root / "docs/STATUS.md").read_text()
        self.assertEqual(status, "# Current status and evidence boundaries\n\nNew status fact.\n\n"
                                 "Old status paragraph.\n")
        matrix = json.loads((self.root / "docs/reviews/r0_3/acceptance_matrix.json").read_text())
        followups = matrix["named_followups"]
        self.assertEqual([entry["id"] for entry in followups], ["KEEP-ME", "NEW-ONE"])
        self.assertEqual(followups[0]["why"], "Original why. Appended.")
        self.assertEqual(followups[0]["source"],
                         "docs/reports/a/README.md, docs/reports/test-card/README.md")
        self.assertEqual(followups[1], {"id": "NEW-ONE", "title": "New", "why": "Why new.",
                                        "source": "docs/reports/test-card/README.md"})
        self.assertEqual(matrix["updated"], "x")
        self.assertIn("build_matrix.py . --gate", out)
        self.assertIn("build_docs_index.py", out)

    def test_null_status_leaves_status_untouched(self):
        document = dict(DOCUMENT, status_paragraph=None, ledger=[])
        before = snapshot(self.root)
        self.assertEqual(run(self.root, document)[0], 0)
        after = snapshot(self.root)
        self.assertEqual(after[self.root / "docs/STATUS.md"], before[self.root / "docs/STATUS.md"])
        matrix = self.root / "docs/reviews/r0_3/acceptance_matrix.json"
        self.assertEqual(after[matrix], before[matrix])

    def test_record_with_no_paragraph_after_the_notice(self):
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        root = make_root(directory.name, record="# Wave 99\n\n" + NOTICE + "\n")
        self.assertEqual(run(root, dict(DOCUMENT, ledger=[]))[0], 0)
        self.assertEqual((root / "docs/development/increments/wave-99.md").read_text(),
                         "# Wave 99\n\n" + NOTICE + "\n\n**TEST-CARD.** New paragraph.\n")

    def test_paragraph_goes_above_a_migrated_section(self):
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        record = "# W\n\n" + NOTICE + "\n\n<!-- migrated-section:start -->\nold\n<!-- migrated-section:end -->\n"
        root = make_root(directory.name, record=record)
        self.assertEqual(run(root, dict(DOCUMENT, ledger=[]))[0], 0)
        text = (root / "docs/development/increments/wave-99.md").read_text()
        self.assertLess(text.index("**TEST-CARD.**"), text.index("<!-- migrated-section:start -->"))

    def test_amend_and_append_write_the_same_bytes_and_keep_a_listed_source(self):
        for operation in ("amend", "append"):
            directory = tempfile.TemporaryDirectory()
            self.addCleanup(directory.cleanup)
            root = make_root(directory.name)
            ledger = [{"id": "KEEP-ME", "operation": operation, "text": "More.",
                       "source": "docs/reports/a/README.md"}]
            self.assertEqual(run(root, dict(DOCUMENT, ledger=ledger))[0], 0)
            entry = json.loads((root / "docs/reviews/r0_3/acceptance_matrix.json").read_text())[
                "named_followups"][0]
            self.assertEqual(entry["why"], "Original why. More.")
            self.assertEqual(entry["source"], "docs/reports/a/README.md")

    def test_register_carries_recorded_by_when_given(self):
        ledger = [dict(DOCUMENT["ledger"][0], recorded_by="TEST-CARD")]
        self.assertEqual(run(self.root, dict(DOCUMENT, ledger=ledger))[0], 0)
        entry = json.loads((self.root / "docs/reviews/r0_3/acceptance_matrix.json").read_text())[
            "named_followups"][-1]
        self.assertEqual(list(entry), ["id", "title", "why", "source", "recorded_by"])

    def test_check_and_dry_run_write_nothing(self):
        before = snapshot(self.root)
        for flag in ("--check", "--dry-run"):
            code, out, _ = run(self.root, DOCUMENT, flag)
            self.assertEqual(code, 0)
            self.assertIn("valid; would write", out)
            current = snapshot(self.root)
            current.pop(self.root / "amendments.json", None)
            expected = dict(before)
            expected.pop(self.root / "amendments.json", None)
            self.assertEqual(current, expected)

    def test_a_null_contribution_applies_nothing_more(self):
        document = dict(DOCUMENT, ledger=[], status_paragraph=None, profiles=[],
                        contribution=None)
        code, out, err = run(self.root, document)
        self.assertEqual(code, 0, err)
        self.assertNotIn("reserved", out)
        self.assertNotIn("contribution:", out)

    def test_a_contribution_naming_an_absent_ledger_is_refused(self):
        before = snapshot(self.root)
        document = dict(DOCUMENT, ledger=[], status_paragraph=None,
                        contribution={"ledgers": ["contract/contributions/33/eight-home.json"]})
        code, _, err = run(self.root, document)
        self.assertEqual(code, 1)
        self.assertIn("no such ledger", err)
        self.assertIn("nothing was written", err)
        current = snapshot(self.root)
        current.pop(self.root / "amendments.json", None)
        before.pop(self.root / "amendments.json", None)
        self.assertEqual(current, before)

    def test_profiles_in_a_tree_without_evaluation_profiles_are_refused(self):
        before = snapshot(self.root)
        document = dict(DOCUMENT, ledger=[], status_paragraph=None, profiles=["eight-home"])
        code, _, err = run(self.root, document, "--check")
        self.assertEqual(code, 1)
        self.assertIn("contract/evaluations does not exist", err)
        code, _, err = run(self.root, document)
        self.assertEqual(code, 1)
        self.assertIn("nothing was written", err)
        current = snapshot(self.root)
        current.pop(self.root / "amendments.json", None)
        self.assertEqual(current, before)

    def test_the_post_gate_sequence_is_printed_after_a_write(self):
        code, out, _ = run(self.root, DOCUMENT)
        self.assertEqual(code, 0)
        self.assertIn("build_matrix.py . --gate", out)
        self.assertIn("python3 scripts/land.py check", out)

    def test_second_application_is_refused(self):
        self.assertEqual(run(self.root, DOCUMENT)[0], 0)
        before = snapshot(self.root)
        code, _, err = run(self.root, DOCUMENT)
        self.assertEqual(code, 1)
        self.assertIn("nothing was written", err)
        self.assertEqual(snapshot(self.root), before)


def mutated(change):
    document = copy.deepcopy(DOCUMENT)
    change(document)
    return document


REFUSALS = {
    "unknown top-level key": (mutated(lambda d: d.update(rule="35")), "unknown top-level key"),
    "missing key": (mutated(lambda d: d.pop("ledger")), "missing top-level key"),
    "wrong schema": (mutated(lambda d: d.update(schema="quirin-amendments/2")), "schema is"),
    "bad revision": (mutated(lambda d: d.update(base="main")), "git revision"),
    "two paragraphs": (mutated(lambda d: d.update(increment_paragraph="a\n\nb")), "one paragraph"),
    "empty status": (mutated(lambda d: d.update(status_paragraph="")), "status_paragraph"),
    "unknown operation": (mutated(lambda d: d["ledger"][0].update(operation="add")), "operation 'add'"),
    "missing field": (mutated(lambda d: d["ledger"][0].pop("title")), "lacks ['title']"),
    "extra field": (mutated(lambda d: d["ledger"][1].update(title="x")), "unexpected ['title']"),
    "null source": (mutated(lambda d: d["ledger"][1].update(source=None)), "source"),
    "register existing": (mutated(lambda d: d["ledger"][0].update(id="KEEP-ME")), "already registered"),
    "append absent": (mutated(lambda d: d["ledger"][1].update(id="ABSENT")), "no registered follow-up"),
    "retire absent": (mutated(lambda d: d["ledger"][2].update(id="ABSENT")), "no registered follow-up"),
    "retire twice": (mutated(lambda d: d["ledger"].append(dict(d["ledger"][2]))), "no registered follow-up"),
    "bad profile": (mutated(lambda d: d.update(profiles=["Eight Home"])), "profile name"),
    "duplicate profile": (mutated(lambda d: d.update(profiles=["a", "a"])), "twice"),
    "bad contribution": (mutated(lambda d: d.update(contribution=[1])), "contribution"),
    "reserved-shape contribution": (mutated(lambda d: d.update(contribution={"rows": []})),
                                    "contribution must be null or"),
    "empty contribution": (mutated(lambda d: d.update(contribution={"ledgers": []})),
                           "non-empty list"),
    "contribution outside the ledgers": (
        mutated(lambda d: d.update(contribution={"ledgers": ["docs/x.json"]})),
        "under contract/contributions/"),
}


class ContributionTests(unittest.TestCase):
    """Card CONTRIBUTION-LEDGERS: the contribution field is checked through
    contribution_ledgers.check_contribution and regenerates the page."""

    def setUp(self):
        from test_contribution_ledgers import Tree
        self.directory = tempfile.TemporaryDirectory()
        self.tree = Tree(self.directory.name)
        self.root = self.tree.root
        self.tree.write(self.tree.record())
        self.paths = ["contract/contributions/33/demo.json",
                      "contract/contributions/35/demo.json",
                      "contract/contributions/interactions/33+35/demo.json"]

    def tearDown(self):
        self.directory.cleanup()

    def test_listed_ledgers_are_checked_and_the_page_is_regenerated(self):
        log, page = tool.check_contribution({"contribution": {"ledgers": self.paths}}, self.root)
        self.assertEqual(len(log), 3)
        self.assertTrue(log[0].startswith("contribution: contract/contributions/33/demo.json"))
        path, old, new = page
        self.assertEqual(str(path), "docs/development/contributions.md")
        self.assertEqual(old, "")
        self.assertIn("rule 33: one row", new)
        (self.root / path).parent.mkdir(parents=True, exist_ok=True)
        (self.root / path).write_text(new)
        self.assertIsNone(tool.check_contribution(
            {"contribution": {"ledgers": self.paths}}, self.root)[1])

    def test_an_omitted_or_tampered_ledger_is_refused(self):
        with self.assertRaises(tool.AmendmentError) as caught:
            tool.check_contribution({"contribution": {"ledgers": self.paths[1:]}}, self.root)
        self.assertIn("omits", str(caught.exception))
        (self.root / "docs/reports/card/native-comparison.json").write_text("{}")
        with self.assertRaises(tool.AmendmentError) as caught:
            tool.check_contribution({"contribution": {"ledgers": self.paths}}, self.root)
        self.assertIn("record digest", str(caught.exception))

    def test_an_inconsistent_tree_is_refused(self):
        self.tree.set_baseline({"Safe": 2, "Unknown": 2})
        digest = json.loads((self.root / "contract/evaluations/demo/evaluation.json")
                            .read_text())["baseline"]["sha256"]
        for relative in self.paths:
            path = self.root / relative
            stored = json.loads(path.read_text())
            stored["profile_baseline"]["sha256"] = digest
            path.write_text(json.dumps(stored))
        with self.assertRaises(tool.AmendmentError) as caught:
            tool.check_contribution({"contribution": {"ledgers": self.paths}}, self.root)
        self.assertIn("never repaired", str(caught.exception))


class RefusalTests(unittest.TestCase):
    def test_every_refusal_exits_one_and_writes_nothing(self):
        for name, (document, message) in REFUSALS.items():
            with self.subTest(name), tempfile.TemporaryDirectory() as directory:
                root = make_root(directory)
                (root / "amendments.json").write_text(json.dumps(document))
                before = snapshot(root)
                code, _, err = run(root, document)
                self.assertEqual(code, 1, name)
                self.assertIn(message, err)
                self.assertIn("nothing was written", err)
                self.assertEqual(snapshot(root), before)

    def test_legacy_null_source_is_refused_without_legacy(self):
        with self.assertRaises(tool.AmendmentError):
            tool.validate(mutated(lambda d: d["ledger"][1].update(source=None)))
        tool.validate(mutated(lambda d: d["ledger"][1].update(source=None)), legacy=True)


class CommittedFileTests(unittest.TestCase):
    def committed(self):
        files = [path for path in tracked_files(ROOT / "docs/reports")
                 if path.name == "integration-amendments.json"]
        self.assertGreaterEqual(len(files), 18)
        return files

    def test_every_committed_file_validates_or_normalises(self):
        refused, current = {}, []
        for path in self.committed():
            raw = json.loads(path.read_text(encoding="utf-8"))
            if "schema" in raw:
                tool.validate(raw)
                current.append(path.parent.name)
                continue
            try:
                document, _ = tool.normalise_legacy(raw, path)
                tool.validate(document, legacy=True)
            except tool.AmendmentError as error:
                refused[path.parent.name] = str(error)
        self.assertIn("amendments-schema", current)
        self.assertEqual(sorted(refused), sorted(REFUSED_LEGACY))
        for name, reason in REFUSED_LEGACY.items():
            self.assertIn(reason, refused[name])

    def test_own_amendments_are_in_the_new_schema(self):
        raw = json.loads((ROOT / "docs/reports/amendments-schema/integration-amendments.json")
                         .read_text(encoding="utf-8"))
        self.assertEqual(raw["schema"], tool.SCHEMA)
        tool.validate(raw)


def git_show(revision, path):
    result = subprocess.run(["git", "-C", str(ROOT), "show", f"{revision}:{path}"],
                            capture_output=True)
    if result.returncode != 0:
        raise AssertionError(
            f"the parity replay reads {revision}:{path} from git objects, which are unavailable "
            f"(a shallow clone or an export): {result.stderr.decode(errors='replace')}")
    return result.stdout


class ProfilesCheck(unittest.TestCase):
    """Card RECORDS-TOOLING (EVALUATION-PROFILES-AMENDMENTS-CHECK): every run
    checks `profiles[]` through evaluation_profiles.check_amendments against
    the document's base, fail-closed, in the evaluation-profiles fixture
    repository (one live profile `demo`, one frozen profile `old`)."""

    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.fixture = ProfileFixture(self.directory.name)
        self.root = self.fixture.root
        self.base = self.fixture.head
        increments = self.root / "docs/development/increments"
        increments.mkdir(parents=True)
        (increments / "README.md").write_text("Current record file: [wave-99.md](wave-99.md)\n")
        (increments / "wave-99.md").write_text(RECORD)
        (self.root / "docs/STATUS.md").write_text(STATUS)

    def tearDown(self):
        self.directory.cleanup()

    def document(self, profiles=None, base=None):
        document = dict(DOCUMENT, base=base or self.base, ledger=[], status_paragraph=None)
        if profiles is not None:
            document["profiles"] = profiles
        return document

    def move_demo(self):
        self.fixture.write_live({"units": 2, "rows": 2}, "moved",
                                history=self.fixture.record("demo")["baseline_history"])

    def assert_refused(self, document, needle, *flags):
        for mode in (("--check",), ("--dry-run",), ()):
            before = snapshot(self.root)
            before.pop(self.root / "amendments.json", None)
            code, _, err = run(self.root, document, *flags, *mode)
            self.assertEqual(code, 1, (mode, err))
            self.assertIn(needle, err)
            self.assertIn("nothing was written", err)
            current = snapshot(self.root)
            current.pop(self.root / "amendments.json", None)
            self.assertEqual(current, before, mode)

    def test_unmoved_tree_passes_with_an_absent_or_empty_list(self):
        for document in (self.document(), self.document([])):
            code, out, err = run(self.root, document, "--check")
            self.assertEqual(code, 0, err)
            self.assertIn(f"profiles: [] checked against {self.base}", out)

    def test_a_moved_live_profile_that_is_listed_passes(self):
        self.move_demo()
        code, out, err = run(self.root, self.document(["demo"]), "--check")
        self.assertEqual(code, 0, err)
        self.assertIn("profiles: ['demo'] checked", out)

    def test_an_unknown_profile_is_refused(self):
        self.assert_refused(self.document(["absent"]), "unknown evaluation profile 'absent'")

    def test_a_frozen_profile_is_refused(self):
        self.assert_refused(self.document(["old"]), "frozen profile 'old'")

    def test_an_unlisted_moved_profile_is_refused(self):
        self.move_demo()
        self.assert_refused(self.document(), "baseline changed since")
        self.assert_refused(self.document([]), "baseline changed since")

    def test_a_listed_unmoved_profile_is_refused(self):
        self.assert_refused(self.document(["demo"]), "baseline changed since")

    def test_a_base_outside_the_object_store_is_refused(self):
        self.assert_refused(self.document(base="f" * 40), "not in this checkout's object store")

    def test_profiles_base_overrides_the_document_base(self):
        self.move_demo()
        moved = commit(self.root, "demo moved by another card")
        document = self.document(["demo"])
        self.assertEqual(run(self.root, document, "--check")[0], 0)
        self.assert_refused(document, "baseline changed since", "--profiles-base", moved)
        code, _, err = run(self.root, self.document([]), "--check", "--profiles-base", moved)
        self.assertEqual(code, 0, err)

    def test_a_legacy_document_without_base_needs_profiles_base(self):
        document = self.document()
        document["base"] = None
        with self.assertRaisesRegex(tool.AmendmentError, "--profiles-base"):
            tool.check_profiles(document, self.root)
        self.assertEqual(tool.check_profiles(document, self.root, self.base), [])

    def test_python_isolated_mode_finds_the_sibling_module(self):
        path = self.root / "amendments.json"
        path.write_text(json.dumps(self.document()))
        result = subprocess.run([sys.executable, "-I", str(ROOT / "scripts/apply_amendments.py"),
                                 str(path), "--root", str(self.root), "--check"],
                                capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stderr)


class ParityTests(unittest.TestCase):
    """Replaying AUDIT-CANONICAL-PATHS's amendments onto c3d8ad3d4's parent
    reproduces the increment record, STATUS.md and the matrix it committed."""

    def replay(self, legacy):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            parent = f"{PARITY_COMMIT}^1"
            for path in (tool.INCREMENT_INDEX, tool.STATUS, tool.MATRIX):
                (root / path).parent.mkdir(parents=True, exist_ok=True)
                (root / path).write_bytes(git_show(parent, path))
            record = tool.current_increment_record(root, must_exist=False)
            (root / record).write_bytes(git_show(parent, record))
            raw = json.loads(git_show(PARITY_COMMIT, PARITY_AMENDMENTS))
            if not legacy:
                raw = dict(raw, schema=tool.SCHEMA)
            code, _, err = run(root, raw, *(["--legacy"] if legacy else []))
            self.assertEqual(code, 0, err)
            for path in (record, tool.STATUS, tool.MATRIX):
                self.assertEqual((root / path).read_bytes(), git_show(PARITY_COMMIT, path), str(path))

    def test_legacy_replay_is_byte_identical(self):
        self.replay(legacy=True)

    def test_schema_replay_is_byte_identical(self):
        self.replay(legacy=False)


if __name__ == "__main__":
    unittest.main()

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


def snapshot(root):
    return {path: path.read_bytes() for path in sorted(Path(root).rglob("*")) if path.is_file()}


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
        self.assertIn("build_matrix.py --gate", out)
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

    def test_reserved_fields_are_validated_and_not_applied(self):
        document = dict(DOCUMENT, ledger=[], status_paragraph=None,
                        profiles=["eight-home", "system"], contribution={"rows": []})
        code, out, err = run(self.root, document)
        self.assertEqual(code, 0, err)
        self.assertIn("reserved: profiles validated, not applied", out)
        self.assertIn("reserved: contribution validated, not applied", out)

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
}


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

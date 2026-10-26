"""Card RECORDS-TOOLING: `scripts/land.py` prints the post-gate landing
sequence and refuses a stale matrix count or report index before the Python
suite reruns."""
import contextlib
import io
import json
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
import land  # noqa: E402


def run(*argv):
    out, err = io.StringIO(), io.StringIO()
    with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
        code = land.main(list(argv))
    return code, out.getvalue(), err.getvalue()


class Land(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.root = Path(self.directory.name)
        (self.root / "scripts").mkdir()
        shutil.copy(ROOT / "scripts/build_docs_index.py", self.root / "scripts")
        (self.root / "docs/reports/card").mkdir(parents=True)
        (self.root / "docs/reports/card/README.md").write_text("# Card\n")
        subprocess.run([sys.executable, "-I", str(self.root / "scripts/build_docs_index.py")],
                       check=True, capture_output=True)
        self.matrix = {"dimensions": {"requirements": 1, "named_followups": 2},
                       "requirements": [{}], "named_followups": [{"id": "A"}, {"id": "B"}]}
        self.write_matrix()

    def tearDown(self):
        self.directory.cleanup()

    def write_matrix(self):
        path = self.root / land.MATRIX
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(self.matrix))

    def test_the_sequence_regenerates_before_the_python_suite(self):
        code, out, _ = run()
        self.assertEqual(code, 0)
        lines = out.splitlines()
        self.assertEqual(len(lines), len(land.SEQUENCE))
        position = {step: index for index, step in enumerate(land.SEQUENCE)}
        python = position["python3 -m unittest discover -s scripts/tests"]
        self.assertLess(position[land.MATRIX_COMMAND], python)
        self.assertLess(position[land.INDEX_COMMAND], python)
        self.assertLess(position["python3 scripts/land.py check"], python)

    def test_fresh_records_pass(self):
        code, out, err = run("check", "--root", str(self.root))
        self.assertEqual(code, 0, err)
        self.assertIn("fresh", out)

    def test_a_stale_named_followups_count_is_refused_with_the_command(self):
        self.matrix["named_followups"].append({"id": "C"})
        self.write_matrix()
        code, _, err = run("check", "--root", str(self.root))
        self.assertEqual(code, 1)
        self.assertIn("dimensions.named_followups is 2 against 3 entries", err)
        self.assertIn(land.MATRIX_COMMAND, err)

    def test_a_stale_report_index_is_refused(self):
        (self.root / "docs/reports/new-card").mkdir()
        (self.root / "docs/reports/new-card/README.md").write_text("# New\n")
        code, _, err = run("check", "--root", str(self.root))
        self.assertEqual(code, 1)
        self.assertIn(land.INDEX_COMMAND, err)

    def test_an_unreadable_matrix_is_refused(self):
        (self.root / land.MATRIX).write_text("{")
        self.assertEqual(run("check", "--root", str(self.root))[0], 1)
        self.matrix.pop("dimensions")
        self.write_matrix()
        self.assertEqual(run("check", "--root", str(self.root))[0], 1)


if __name__ == "__main__":
    unittest.main()

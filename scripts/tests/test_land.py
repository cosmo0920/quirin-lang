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
        self.assertLess(python, position[land.SEQUENCE[-1]])

    def test_publication_requires_a_previous_baseline_unless_bootstrapping(self):
        subprocess.run(["git", "init", "-q", str(self.root)], check=True)
        subprocess.run(["git", "-C", str(self.root), "-c", "user.name=Test",
                        "-c", "user.email=test@example.invalid", "commit", "-qm",
                        "initial", "--allow-empty"], check=True)
        release = self.root / "target/release"
        release.mkdir(parents=True)
        (release / "quirin-audit").touch()
        (release / "quirin-audit-measure").touch()
        inputs = self.root / "inputs"
        inputs.mkdir()
        subprocess.run(["git", "-C", str(self.root), "add", "-A"], check=True)
        subprocess.run(["git", "-C", str(self.root), "-c", "user.name=Test",
                        "-c", "user.email=test@example.invalid", "commit", "-qm",
                        "landing"], check=True)
        with self.assertRaisesRegex(ValueError, "previous baseline is absent"):
            land.publish(self.root, inputs_base=inputs)

    def test_first_publication_then_verified_baseline_side(self):
        subprocess.run(["git", "init", "-q", str(self.root)], check=True)
        (self.root / ".gitignore").write_text("private/\n")
        release = self.root / "target/release"
        release.mkdir(parents=True)
        host = release / "quirin-audit"
        host.write_text("""#!/usr/bin/env python3
import json, pathlib, sys
args = sys.argv[1:]
root = pathlib.Path.cwd()
log = root / 'private/calls.jsonl'
log.parent.mkdir(parents=True, exist_ok=True)
with log.open('a') as stream:
    stream.write(json.dumps(args) + '\\n')
out = pathlib.Path(args[args.index('--output') + 1])
if 'publish' in args:
    for profile in ('eight-home', 'fluent-bit-system', 'fluent-bit-tail-pause'):
        (out / profile).mkdir(parents=True)
else:
    (out / 'sides/tip').mkdir(parents=True)
    (out / 'native-comparison.json').write_text('{}')
""")
        host.chmod(0o755)
        (release / "quirin-audit-measure").touch()
        inputs = self.root / "inputs"
        inputs.mkdir()
        def commit(message):
            subprocess.run(["git", "-C", str(self.root), "add", "-A"], check=True)
            subprocess.run(["git", "-C", str(self.root), "-c", "user.name=Test",
                            "-c", "user.email=test@example.invalid", "commit", "-qm",
                            message, "--allow-empty"], check=True)
            return land.git_revision(self.root, "HEAD")
        first = commit("first")
        second = commit("landing one")
        land.publish(self.root, inputs_base=inputs, bootstrap=True)
        third = commit("landing two")
        land.publish(self.root, inputs_base=inputs)
        calls = [json.loads(line) for line in (self.root / "private/calls.jsonl").read_text().splitlines()]
        self.assertIn(f"prev=rev:{first}", calls[0])
        self.assertIn(f"tip=rev:{second}", calls[0])
        self.assertIn(f"prev=baseline:{second}", calls[2])
        self.assertIn(f"tip=rev:{third}", calls[2])

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

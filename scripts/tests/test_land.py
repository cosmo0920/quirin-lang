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
import types
import unittest
from pathlib import Path
from unittest import mock

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
import land  # noqa: E402


FIGURES = {"rows": 2, "row_statuses": {"Safe": 1, "Unknown": 1}, "row_verdicts_sha256": "a" * 64}
PROFILE_ROUTES = (("corpus", "eight-home"), ("system", "fluent-bit-system"),
                  ("slice", "fluent-bit-tail-pause"))


def comparison(before=FIGURES, after=FIGURES, routes=PROFILE_ROUTES):
    """A `quirin-native-comparison/1` record's prev -> tip slots that land.py reads."""
    return {"routes": [{"name": name, "profile": profile} for name, profile in routes],
            "comparisons": [{"before": "prev", "after": "tip", "routes": {
                name: {"figures": {"before": before, "after": after}}
                for name, _ in routes}}]}


class FakeProfiles:
    """The evaluation_profiles API land.py applies, with scripted answers."""
    ProfileError = type("ProfileError", (Exception,), {})

    def __init__(self, live=("eight-home", "fluent-bit-system", "fluent-bit-tail-pause"),
                 frozen=(), moved=(), history_error=None, baseline_moved_since_prior=()):
        self.live, self.frozen, self.moved = live, frozen, moved
        self.history_error = history_error
        self.baseline_moved = baseline_moved_since_prior
        self.compared = []

    def load_profiles(self, root):
        records = {name: {"policy": {"kind": "live"}} for name in self.live}
        records.update({name: {"policy": {"kind": "frozen"}} for name in self.frozen})
        return records

    def compare(self, root, name, figures):
        self.compared.append(name)
        if name in self.frozen:
            return 2, [f"{name} is frozen"]
        if name in self.moved:
            return 1, [f"rows: baseline 2, run {figures['rows']}"]
        return 0, []

    def check_history(self, root):
        if self.history_error:
            raise self.ProfileError(self.history_error)
        return 0

    def _record_at(self, root, revision, name):
        return {"baseline": {"sha256": "old" if name in self.baseline_moved else "same"}}

    def _load_profile(self, root, name):
        return {"baseline": {"sha256": "same"}}


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
        self.assertLess(position[land.CONTRIBUTIONS_COMMAND], position["python3 scripts/land.py check"])
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
    (out / 'native-comparison.json').write_text(
        (root / 'comparison.json').read_text())
""")
        (self.root / ".gitignore").write_text("private/\ncomparison.json\n")
        (self.root / "comparison.json").write_text(json.dumps(comparison()))
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
        fake = FakeProfiles()
        with mock.patch.object(land, "evaluation_profiles", fake):
            land.publish(self.root, inputs_base=inputs, bootstrap=True)
            third = commit("landing two")
            land.publish(self.root, inputs_base=inputs)
        self.assertEqual(sorted(fake.compared), sorted(2 * [p for _, p in PROFILE_ROUTES]))
        calls = [json.loads(line) for line in (self.root / "private/calls.jsonl").read_text().splitlines()]
        self.assertIn(f"prev=rev:{first}", calls[0])
        self.assertIn(f"tip=rev:{second}", calls[0])
        self.assertIn(f"prev=baseline:{second}", calls[2])
        self.assertIn(f"tip=rev:{third}", calls[2])

    def stub_landing(self, record):
        """A two-commit repository whose stub host writes `record`; returns the inputs."""
        subprocess.run(["git", "init", "-q", str(self.root)], check=True)
        (self.root / ".gitignore").write_text("private/\n")
        release = self.root / "target/release"
        release.mkdir(parents=True)
        host = release / "quirin-audit"
        host.write_text("#!" + sys.executable + "\n" + f"""import json, pathlib, sys
args = sys.argv[1:]
log = pathlib.Path.cwd() / 'private/calls.jsonl'
log.parent.mkdir(parents=True, exist_ok=True)
with log.open('a') as stream:
    stream.write(json.dumps(args) + '\\n')
out = pathlib.Path(args[args.index('--output') + 1])
if 'publish' in args:
    (out / 'eight-home').mkdir(parents=True)
else:
    (out / 'sides/tip').mkdir(parents=True)
    (out / 'native-comparison.json').write_text({json.dumps(json.dumps(record))})
""")
        host.chmod(0o755)
        (release / "quirin-audit-measure").touch()
        inputs = self.root / "inputs"
        inputs.mkdir()
        for message in ("first", "landing"):
            subprocess.run(["git", "-C", str(self.root), "add", "-A"], check=True)
            subprocess.run(["git", "-C", str(self.root), "-c", "user.name=Test",
                            "-c", "user.email=test@example.invalid", "commit", "-qm",
                            message, "--allow-empty"], check=True)
        return inputs

    def test_moved_live_figures_refuse_publication_and_keep_the_measurement(self):
        inputs = self.stub_landing(comparison(after=dict(FIGURES, rows=3)))
        # Every route's figures moved; the other two landed their new baselines.
        fake = FakeProfiles(moved=("eight-home",), baseline_moved_since_prior=(
            "fluent-bit-system", "fluent-bit-tail-pause"))
        with mock.patch.object(land, "evaluation_profiles", fake):
            with self.assertRaisesRegex(ValueError, "the landing does not complete") as caught:
                land.publish(self.root, inputs_base=inputs, bootstrap=True)
        message = str(caught.exception)
        self.assertIn("eight-home: the landed figures moved", message)
        self.assertIn("baseline_history", message)
        self.assertIn("rows: baseline 2, run 3", message)
        self.assertNotIn("fluent-bit-system", message)
        self.assertFalse((self.root / "private/baselines").exists(), "nothing published")
        calls = (self.root / "private/calls.jsonl").read_text().splitlines()
        self.assertEqual(len(calls), 1, "measured once, never published")
        self.assertEqual(len(list((self.root / "private/measure").iterdir())), 1)

    def test_the_policy_refuses_each_unproven_case(self):
        refuse = land.profile_policy
        self.assertEqual(refuse(self.root, comparison(), "p", FakeProfiles()), [])
        # A live profile no route measured.
        found = refuse(self.root, comparison(routes=PROFILE_ROUTES[:2]), "p", FakeProfiles())
        self.assertEqual(found, ["fluent-bit-tail-pause: live profile not measured"])
        # A side without figures.
        found = refuse(self.root, comparison(after=None), "p", FakeProfiles())
        self.assertEqual(len(found), 3)
        self.assertTrue(all("no figures on both sides" in line for line in found))
        # A frozen profile measured as live: its results never move.
        found = refuse(self.root, comparison(), "p",
                       FakeProfiles(live=("eight-home", "fluent-bit-system"),
                                    frozen=("fluent-bit-tail-pause",)))
        self.assertEqual(found, ["fluent-bit-tail-pause: fluent-bit-tail-pause is frozen"])
        # The history policy (append-only history, frozen never moves) fails.
        found = refuse(self.root, comparison(), "p", FakeProfiles(history_error="rewritten"))
        self.assertEqual(found, ["profile history policy: rewritten"])
        # No prev -> tip comparison at all.
        self.assertEqual(refuse(self.root, {"comparisons": []}, "p", FakeProfiles()),
                         ["the measurement has no single prev -> tip comparison"])
        # The figures moved since the previous landing: the committed baseline must
        # have moved in that range too.
        moved = comparison(before=dict(FIGURES, rows=1))
        found = refuse(self.root, moved, "p", FakeProfiles())
        self.assertEqual(len(found), 3)
        self.assertIn("the figures moved since p but the committed baseline did not", found[0])
        self.assertEqual(refuse(self.root, moved, "p", FakeProfiles(
            baseline_moved_since_prior=[name for _, name in PROFILE_ROUTES])), [])

    def test_the_policy_reads_the_committed_baselines(self):
        """The real evaluation_profiles API on this tree: the committed live
        baselines' own figures pass; one moved figure refuses."""
        profiles = land.evaluation_profiles.load_profiles(ROOT)
        live = {name: record for name, record in profiles.items()
                if record["policy"]["kind"] == "live"}
        figures = {name: json.loads((ROOT / record["baseline"]["path"]).read_text())["figures"]
                   for name, record in live.items()}
        routes = tuple((f"route-{name}", name) for name in sorted(live))

        def record(changed=None):
            document = comparison(routes=routes)
            for route, name in routes:
                after = dict(figures[name], **(changed or {}).get(name, {}))
                document["comparisons"][0]["routes"][route]["figures"] = {
                    "before": after, "after": after}
            return document

        self.assertEqual(land.profile_policy(ROOT, record(), "HEAD"), [])
        name = sorted(live)[0]
        found = land.profile_policy(ROOT, record({name: {"rows": -1}}), "HEAD")
        self.assertEqual(len(found), 1)
        self.assertTrue(found[0].startswith(f"{name}: the landed figures moved"), found)
        self.assertIn("rows: baseline", found[0])

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

    def test_a_stale_contributions_page_is_refused(self):
        """Card CONTRIBUTION-LEDGERS: the page is checked when the tree has it."""
        shutil.copy(ROOT / "scripts/contribution_ledgers.py", self.root / "scripts")
        code, _, err = run("check", "--root", str(self.root))
        self.assertEqual(code, 1)
        self.assertIn(land.CONTRIBUTIONS_COMMAND, err)
        subprocess.run([sys.executable, "-I", str(self.root / "scripts/contribution_ledgers.py"),
                        "--root", str(self.root), "render"], check=True, capture_output=True)
        code, _, err = run("check", "--root", str(self.root))
        self.assertEqual(code, 0, err)

    def test_an_unreadable_matrix_is_refused(self):
        (self.root / land.MATRIX).write_text("{")
        self.assertEqual(run("check", "--root", str(self.root))[0], 1)
        self.matrix.pop("dimensions")
        self.write_matrix()
        self.assertEqual(run("check", "--root", str(self.root))[0], 1)


if __name__ == "__main__":
    unittest.main()

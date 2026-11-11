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

"""Evaluation profiles, cycle bindings and the frozen/live policy
(card EVALUATION-PROFILES; `scripts/evaluation_profiles.py` is the authority)."""
import copy
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import evaluation_profiles as ep  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
LIVE = {"eight-home", "fluent-bit-system", "fluent-bit-tail-pause"}
SHA_A = "a" * 64
SHA_B = "b" * 64
REV = "1" * 40
REV2 = "2" * 40


def sha(data):
    return hashlib.sha256(data).hexdigest()


def live_record(name="demo", baseline_sha=SHA_A, history=None, inputs=None):
    history = history if history is not None else [
        {"sha256": baseline_sha, "measured_revision": REV, "reason": "first", "source": "r.md"}]
    return {
        "schema": ep.PROFILE_SCHEMA, "name": name, "description": "d",
        "policy": {"kind": "live"},
        "route": {"kind": "pinned-corpus", "command": "c"},
        "source": {"repository": "r", "revision": "abcdef1", "submodules": {}, "note": "n"},
        "inputs": {"tracked": inputs if inputs is not None else [], "untracked": []},
        "contract_set": {"lock_set": {"state": "latest"}, "references": ["Cargo.toml"],
                         "note": "n"},
        "baseline": {"path": f"contract/evaluations/{name}/baseline.json", "sha256": baseline_sha},
        "baseline_history": history,
        "evidence_version": {"analyzer_revision": history[-1]["measured_revision"],
                             "semantic_evidence": "v2"},
    }


def frozen_record(name="old", revision=REV, baseline_sha=SHA_A, evidence=SHA_B):
    record = live_record(name, baseline_sha)
    del record["baseline_history"]
    record["policy"] = {"kind": "frozen", "revision": revision}
    record["evidence_version"]["analyzer_revision"] = revision
    record["contract_set"] = {"lock_set": {"state": "retired", "evidence_sha256": evidence},
                              "references": [], "note": "n"}
    return record


def git(root, *args):
    environment = dict(os.environ, GIT_AUTHOR_NAME="t", GIT_AUTHOR_EMAIL="t@example.invalid",
                       GIT_COMMITTER_NAME="t", GIT_COMMITTER_EMAIL="t@example.invalid")
    for key in ("GIT_DIR", "GIT_WORK_TREE", "GIT_INDEX_FILE"):
        environment.pop(key, None)
    return subprocess.run(["git", "-C", str(root), *args], check=True, capture_output=True,
                          env=environment).stdout.decode().strip()


def write(root, path, value):
    target = Path(root) / path
    target.parent.mkdir(parents=True, exist_ok=True)
    data = value if isinstance(value, bytes) else (
        value.encode() if isinstance(value, str) else (json.dumps(value, indent=2) + "\n").encode())
    target.write_bytes(data)
    return sha(data)


def commit(root, message="c"):
    git(root, "add", "-A")
    git(root, "commit", "-q", "--allow-empty", "-m", message)
    return git(root, "rev-parse", "HEAD")


class Fixture:
    """A small git repository holding one live profile, one frozen profile,
    a cycle document, a matrix corpus section and the bindings."""

    def __init__(self, directory):
        self.root = Path(directory)
        git(self.root, "init", "-q")
        write(self.root, "Cargo.toml", "[workspace]\n")
        write(self.root, "providers/p/contracts/a.qv", "extern fn a();\n")
        write(self.root, "contract/evaluations/demo/input.json", "{}\n")
        self.frozen_rev = commit(self.root, "base")
        self.evidence = ep.contract_evidence_digest(self.root, self.frozen_rev)
        self.figures = {"units": 1, "rows": 2}
        self.write_live(self.figures, "first")
        baseline = {"schema": ep.BASELINE_SCHEMA, "profile": "old",
                    "measured_revision": self.frozen_rev, "source": "r.md",
                    "figures": {"verdict": "Unknown"}, "artifacts": []}
        digest = write(self.root, "contract/evaluations/old/baseline.json", baseline)
        record = frozen_record("old", self.frozen_rev, digest, self.evidence)
        record["inputs"]["tracked"] = [{
            "path": "Cargo.toml", "revision": self.frozen_rev,
            "sha256": sha(b"[workspace]\n"), "role": "r"}]
        write(self.root, "contract/evaluations/old/evaluation.json", record)
        write(self.root, "docs/cycles/CYCLE_1.md",
              "# Cycle 1\n\nEvaluation profile: `old` (frozen at "
              f"`{self.frozen_rev[:9]}`; [record](x)).\n")
        write(self.root, "docs/cycles/CYCLE_2.md",
              "# Cycle 2\n\nEvaluation profile: none (a model cycle).\n")
        write(self.root, ep.MATRIX, {"corpus": {
            "pin": 3, "row": {"routes": {"corpus": {"rows": 2}}}}})
        self.bindings = {"schema": ep.BINDINGS_SCHEMA,
                         "matrix": {"pin": {"none": "an inventory pin"},
                                    "row": {"split": {"routes/corpus": "demo"}}},
                         "pinned_cycle_documents": {}, "frozen_amendments": []}
        write(self.root, ep.BINDINGS, self.bindings)
        self.head = commit(self.root, "profiles")

    def write_live(self, figures, reason, history=None, revision=REV):
        baseline = {"schema": ep.BASELINE_SCHEMA, "profile": "demo", "measured_revision": revision,
                    "source": "r.md", "figures": figures, "artifacts": []}
        digest = write(self.root, "contract/evaluations/demo/baseline.json", baseline)
        entry = {"sha256": digest, "measured_revision": revision, "reason": reason,
                 "source": "r.md"}
        record = live_record("demo", digest, (history or []) + [entry], inputs=[{
            "path": "contract/evaluations/demo/input.json",
            "sha256": sha((self.root / "contract/evaluations/demo/input.json").read_bytes()),
            "role": "r"}])
        write(self.root, "contract/evaluations/demo/evaluation.json", record)
        return record

    def record(self, name):
        return json.loads((self.root / f"contract/evaluations/{name}/evaluation.json").read_text())

    def check(self):
        profiles = ep.check_tree(self.root)
        ep.check_bindings(self.root, profiles)
        return ep.check_history(self.root)


class CommittedProfiles(unittest.TestCase):
    """The committed records, bindings and history pass the policy."""

    def test_tree_bindings_and_history(self):
        profiles = ep.check_tree(ROOT)
        ep.check_bindings(ROOT, profiles)
        self.assertGreaterEqual(ep.check_history(ROOT), len(profiles))
        live = {name for name, record in profiles.items() if record["policy"]["kind"] == "live"}
        self.assertEqual(live, LIVE)

    def test_records_stay_out_of_first_party_discovery(self):
        """compiler/src/first_party.rs loads every directory under
        contract/profiles as a source-set profile; one without profile.json
        empties the provider registry. Records live elsewhere, and every
        tracked contract/profiles directory is a source set."""
        self.assertNotEqual(ep.PROFILES.parts[:2], ("contract", "profiles"))
        directories = {Path(path).parts[2] for path in ep.tracked(ROOT, "contract/profiles")
                       if len(Path(path).parts) > 3}
        for name in sorted(directories):
            self.assertTrue((ROOT / "contract/profiles" / name / "profile.json").is_file(), name)

    def test_frozen_revisions_are_ancestors_of_head(self):
        for name, record in ep.load_profiles(ROOT).items():
            if record["policy"]["kind"] != "frozen":
                continue
            result = subprocess.run(["git", "-C", str(ROOT), "merge-base", "--is-ancestor",
                                     record["policy"]["revision"], "HEAD"])
            self.assertEqual(result.returncode, 0, name)

    def test_live_baselines_are_route_summaries(self):
        for name in LIVE:
            record = ep.load_profiles(ROOT)[name]
            baseline = json.loads((ROOT / record["baseline"]["path"]).read_text())
            self.assertEqual(set(baseline["figures"]), {
                "units", "unit_statuses", "unit_verdicts_sha256", "rows", "row_statuses",
                "row_verdicts_sha256", "null_rows", "null_verdicts", "null_verdicts_sha256"})
            self.assertEqual(baseline["figures"]["rows"],
                             sum(baseline["figures"]["row_statuses"].values()))

    def test_cli_check(self):
        result = subprocess.run([sys.executable, str(ROOT / "scripts/evaluation_profiles.py"),
                                 "check", "--history"], capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("3 live", result.stdout)


class Records(unittest.TestCase):
    def test_valid_records(self):
        ep.validate_record(live_record(), "demo")
        ep.validate_record(frozen_record(), "old")

    def test_refusals(self):
        cases = []
        record = live_record()
        record["name"] = "other"
        cases.append(("name", record, "demo"))
        record = live_record()
        record["extra"] = 1
        cases.append(("unknown key", record, "demo"))
        record = live_record()
        record["baseline_history"][-1]["sha256"] = SHA_B
        cases.append(("history head", record, "demo"))
        record = live_record()
        record["baseline_history"][-1]["reason"] = ""
        cases.append(("empty reason", record, "demo"))
        record = frozen_record()
        record["inputs"]["tracked"] = [{"path": "a", "revision": REV2, "sha256": SHA_A, "role": "r"}]
        cases.append(("frozen input revision", record, "old"))
        record = frozen_record()
        record["baseline_history"] = []
        cases.append(("frozen history", record, "old"))
        record = frozen_record()
        record["policy"]["revision"] = "abc"
        cases.append(("short revision", record, "old"))
        record = live_record()
        record["inputs"]["untracked"] = [{"path": "/abs", "sha256": None, "role": "r"}]
        cases.append(("absolute path", record, "demo"))
        record = live_record()
        record["route"]["kind"] = "other"
        cases.append(("route kind", record, "demo"))
        record = live_record()
        record["contract_set"]["lock_set"] = {"state": "retired", "evidence_sha256": SHA_A}
        cases.append(("live lock set not latest", record, "demo"))
        record = live_record()
        record["contract_set"]["references"] = []
        cases.append(("live without references", record, "demo"))
        record = frozen_record()
        record["contract_set"]["references"] = ["providers/contract-load-path.json"]
        cases.append(("frozen depends on a live file", record, "old"))
        record = frozen_record()
        record["contract_set"]["lock_set"] = {"state": "latest"}
        cases.append(("frozen bound to latest", record, "old"))
        record = frozen_record()
        record["contract_set"]["lock_set"] = {"state": "retained", "evidence_sha256": SHA_A}
        cases.append(("retained without revision", record, "old"))
        for label, record, name in cases:
            with self.assertRaises(ep.ProfileError, msg=label):
                ep.validate_record(record, name)


class Transitions(unittest.TestCase):
    """The frozen/live policy across one commit, on synthetic records."""

    def run_transition(self, old, new, changed, blobs=None, amendments=()):
        blobs = blobs or (lambda side, path: None)
        ep.check_transition("p", old, new, changed, blobs, list(amendments), "commit x")

    def test_frozen_never_moves(self):
        old = frozen_record("p")
        new = copy.deepcopy(old)
        new["description"] = "edited"
        with self.assertRaisesRegex(ep.ProfileError, "frozen profile p changed"):
            self.run_transition(old, new, ["contract/evaluations/p/evaluation.json"],
                                lambda side, path: side.encode())
        with self.assertRaisesRegex(ep.ProfileError, "frozen profile p changed"):
            self.run_transition(old, None, ["contract/evaluations/p/evaluation.json"])

    def test_frozen_amendment_is_content_addressed(self):
        old, new = frozen_record("p"), frozen_record("p")
        path = "contract/evaluations/p/evaluation.json"
        amendment = {"profile": "p", "path": path, "from_sha256": sha(b"old"),
                     "to_sha256": sha(b"new"), "reason": "typo"}
        self.run_transition(old, new, [path], lambda side, p: side.encode(), [amendment])
        with self.assertRaises(ep.ProfileError):
            self.run_transition(old, new, [path], lambda side, p: (side + "x").encode(),
                                [amendment])

    def test_live_baseline_moves_only_with_a_reasoned_history_entry(self):
        old = live_record("p", SHA_A)
        moved = copy.deepcopy(old)
        moved["baseline"]["sha256"] = SHA_B
        moved["baseline_history"][-1] = dict(moved["baseline_history"][-1], sha256=SHA_B)
        with self.assertRaises(ep.ProfileError):  # rewrites the only entry
            self.run_transition(old, moved, ["contract/evaluations/p/baseline.json"])
        appended = live_record("p", SHA_B, old["baseline_history"] + [
            {"sha256": SHA_B, "measured_revision": REV2, "reason": "rule 36", "source": "r.md"}])
        self.run_transition(old, appended, ["contract/evaluations/p/baseline.json",
                                            "contract/evaluations/p/evaluation.json"])
        silent = copy.deepcopy(old)
        silent["baseline"]["sha256"] = SHA_B
        with self.assertRaises(ep.ProfileError):  # record invalid: history head differs
            self.run_transition(old, silent, ["contract/evaluations/p/baseline.json"])

    def test_history_without_baseline_change_and_rewrites_are_refused(self):
        old = live_record("p", SHA_A)
        extra = live_record("p", SHA_A, old["baseline_history"] + [
            {"sha256": SHA_A, "measured_revision": REV, "reason": "again", "source": "r.md"}])
        with self.assertRaisesRegex(ep.ProfileError, "without a baseline change"):
            self.run_transition(old, extra, ["contract/evaluations/p/evaluation.json"])
        rewritten = copy.deepcopy(old)
        rewritten["baseline_history"][0]["reason"] = "softened"
        with self.assertRaisesRegex(ep.ProfileError, "append-only"):
            self.run_transition(old, rewritten, ["contract/evaluations/p/evaluation.json"])

    def test_live_inputs_change_is_a_rebase(self):
        old = live_record("p", SHA_A, inputs=[{"path": "a", "sha256": SHA_A, "role": "r"}])
        new = copy.deepcopy(old)
        new["inputs"]["tracked"][0]["sha256"] = SHA_B
        with self.assertRaisesRegex(ep.ProfileError, "re-base"):
            self.run_transition(old, new, ["contract/evaluations/p/evaluation.json"])

    def test_lock_set_retirement_is_the_one_frozen_change(self):
        """Decision 9a: a retained lock set may retire (same evidence digest);
        nothing else changes with it."""
        path = "contract/evaluations/p/evaluation.json"
        old = frozen_record("p")
        old["contract_set"]["lock_set"] = {"state": "retained", "fluent_bit_revision": REV2,
                                           "evidence_sha256": SHA_B}
        ep.validate_record(old, "p")
        retired = frozen_record("p")
        self.run_transition(old, retired, [path])
        other_digest = frozen_record("p", evidence=SHA_A)
        with self.assertRaisesRegex(ep.ProfileError, "frozen profile p changed"):
            self.run_transition(old, other_digest, [path], lambda side, p: side.encode())
        also_edited = frozen_record("p")
        also_edited["description"] = "re-measured"
        with self.assertRaisesRegex(ep.ProfileError, "frozen profile p changed"):
            self.run_transition(old, also_edited, [path], lambda side, p: side.encode())
        with self.assertRaisesRegex(ep.ProfileError, "frozen profile p changed"):
            self.run_transition(retired, old, [path], lambda side, p: side.encode())

    def test_live_is_frozen_never_deleted(self):
        old = live_record("p", SHA_A)
        with self.assertRaisesRegex(ep.ProfileError, "freeze it instead"):
            self.run_transition(old, None, ["contract/evaluations/p/evaluation.json"])
        self.run_transition(old, frozen_record("p", REV, SHA_A),
                            ["contract/evaluations/p/evaluation.json"])


class History(unittest.TestCase):
    """The policy checked commit by commit in a real repository."""

    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.fixture = Fixture(self.temp.name)

    def tearDown(self):
        self.temp.cleanup()

    def test_fixture_passes(self):
        self.assertEqual(self.fixture.check(), 2)

    def test_live_baseline_move_with_reason_passes(self):
        fixture = self.fixture
        history = fixture.record("demo")["baseline_history"]
        fixture.write_live({"units": 1, "rows": 3}, "rule 99 promotes one row", history, REV2)
        commit(fixture.root)
        self.assertEqual(fixture.check(), 3)

    def test_live_baseline_move_without_history_fails(self):
        fixture = self.fixture
        record = fixture.record("demo")
        baseline = json.loads((fixture.root / record["baseline"]["path"]).read_text())
        baseline["figures"]["rows"] = 3
        digest = write(fixture.root, record["baseline"]["path"], baseline)
        commit(fixture.root, "baseline only")
        with self.assertRaisesRegex(ep.ProfileError, "does not match"):
            ep.check_tree(fixture.root)
        with self.assertRaisesRegex(ep.ProfileError, "baseline file changed but its record"):
            ep.check_history(fixture.root)

    def test_live_history_rewrite_fails(self):
        fixture = self.fixture
        record = fixture.record("demo")
        baseline = json.loads((fixture.root / record["baseline"]["path"]).read_text())
        baseline["figures"]["rows"] = 3
        digest = write(fixture.root, record["baseline"]["path"], baseline)
        record["baseline"]["sha256"] = digest
        record["baseline_history"][-1]["sha256"] = digest
        write(fixture.root, "contract/evaluations/demo/evaluation.json", record)
        commit(fixture.root, "rewrite history")
        ep.check_tree(fixture.root)
        with self.assertRaisesRegex(ep.ProfileError, "append-only"):
            ep.check_history(fixture.root)

    def test_frozen_edit_fails_history(self):
        fixture = self.fixture
        record = fixture.record("old")
        record["description"] = "re-measured"
        write(fixture.root, "contract/evaluations/old/evaluation.json", record)
        commit(fixture.root)
        with self.assertRaisesRegex(ep.ProfileError, "frozen profile old changed"):
            ep.check_history(fixture.root)

    def test_frozen_contract_evidence_needs_no_live_lock(self):
        """Removing every contract file from the tree (a retired lock set)
        leaves a frozen profile valid: its evidence is read at its revision."""
        fixture = self.fixture
        (fixture.root / "providers/p/contracts/a.qv").unlink()
        commit(fixture.root, "retire the lock set")
        fixture.check()
        record = fixture.record("old")
        self.assertEqual(record["contract_set"]["lock_set"]["evidence_sha256"],
                         ep.contract_evidence_digest(fixture.root, fixture.frozen_rev))

    def test_wrong_contract_evidence_fails_tree(self):
        fixture = self.fixture
        record = fixture.record("old")
        record["contract_set"]["lock_set"]["evidence_sha256"] = SHA_A
        write(fixture.root, "contract/evaluations/old/evaluation.json", record)
        with self.assertRaisesRegex(ep.ProfileError, "contract evidence digest"):
            ep.check_tree(fixture.root)

    def test_live_input_edit_without_rebase_fails_tree(self):
        write(self.fixture.root, "contract/evaluations/demo/input.json", "{\"x\": 1}\n")
        commit(self.fixture.root)
        with self.assertRaisesRegex(ep.ProfileError, "re-base"):
            ep.check_tree(self.fixture.root)

    def test_frozen_artifact_bytes_are_stable(self):
        fixture = self.fixture
        write(fixture.root, "docs/reports/x/after.json", "{}\n")
        commit(fixture.root)
        root = Path(fixture.root)
        # a new frozen profile citing the artifact, created in one commit
        baseline = {"schema": ep.BASELINE_SCHEMA, "profile": "art",
                    "measured_revision": fixture.frozen_rev, "source": "r.md",
                    "figures": {"a": 1},
                    "artifacts": [{"path": "docs/reports/x/after.json", "sha256": sha(b"{}\n")}]}
        digest = write(root, "contract/evaluations/art/baseline.json", baseline)
        write(root, "contract/evaluations/art/evaluation.json",
              frozen_record("art", fixture.frozen_rev, digest, fixture.evidence))
        bindings = dict(fixture.bindings, matrix=dict(fixture.bindings["matrix"], extra="art"))
        matrix = json.loads((root / ep.MATRIX).read_text())
        matrix["corpus"]["extra"] = {}
        write(root, ep.MATRIX, matrix)
        write(root, ep.BINDINGS, bindings)
        commit(root)
        fixture.check()
        write(root, "docs/reports/x/after.json", "{\"moved\": 1}\n")
        with self.assertRaisesRegex(ep.ProfileError, "bytes changed"):
            ep.check_tree(root)

    def test_record_command_writes_a_valid_move(self):
        fixture = self.fixture
        report, null = self.run_files(fixture.root)
        figures = ep.summarize(report, null)
        self.assertEqual(ep.compare(fixture.root, "demo", figures)[0], 1)
        ep.record_baseline(fixture.root, "demo", figures, REV2, "first real run", "r.md")
        commit(fixture.root)
        fixture.check()
        self.assertEqual(ep.compare(fixture.root, "demo", figures), (0, []))
        self.assertEqual(ep.compare(fixture.root, "old", figures)[0], 2)
        with self.assertRaises(ep.ProfileError):
            ep.record_baseline(fixture.root, "old", figures, REV2, "no", "r.md")

    def test_amendments_profiles(self):
        fixture = self.fixture
        self.assertEqual(ep.check_amendments(fixture.root, {"profiles": ["demo"]}), ["demo"])
        for bad in (["nope"], ["old"], ["demo", "demo"]):
            with self.assertRaises(ep.ProfileError, msg=bad):
                ep.check_amendments(fixture.root, {"profiles": bad})
        base = fixture.head
        self.assertEqual(ep.check_amendments(fixture.root, {"profiles": []}, base), [])
        history = fixture.record("demo")["baseline_history"]
        fixture.write_live({"units": 1, "rows": 4}, "moved", history, REV2)
        commit(fixture.root)
        with self.assertRaisesRegex(ep.ProfileError, "changed since"):
            ep.check_amendments(fixture.root, {"profiles": []}, base)
        self.assertEqual(ep.check_amendments(fixture.root, {"profiles": ["demo"]}, base),
                         ["demo"])

    @staticmethod
    def run_files(root):
        report = {"units": [
            {"library": "l", "file": "a.c", "status": "Unknown", "reason_code": "x",
             "functions": [{"function": "f", "status": "Safe"},
                           {"function": "f", "status": "Unknown"}]},
            {"library": "l", "file": "a.c", "status": "Safe", "functions": []}]}
        write(root, "run/report.json", report)
        write(root, "run/null.jsonl",
              json.dumps({"library": "l", "file": "a.c", "function": "f", "custody": "Safe",
                          "combined": "Safe", "null": {"verdict": "Safe", "checks": []}}) + "\n")
        return Path(root) / "run/report.json", Path(root) / "run/null.jsonl"


class Bindings(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.fixture = Fixture(self.temp.name)
        self.root = self.fixture.root

    def tearDown(self):
        self.temp.cleanup()

    def refused(self, pattern):
        with self.assertRaisesRegex(ep.ProfileError, pattern):
            ep.check_bindings(self.root, ep.check_tree(self.root))

    def test_cycle_without_a_line(self):
        write(self.root, "docs/cycles/CYCLE_3.md", "# Cycle 3\n")
        commit(self.root)
        self.refused("exactly one evaluation profile")

    def test_cycle_with_two_lines_or_unknown_profile(self):
        write(self.root, "docs/cycles/CYCLE_3.md", "Evaluation profile: none (a).\n"
              "Evaluation profile: none (b).\n")
        commit(self.root)
        self.refused("found 2")
        write(self.root, "docs/cycles/CYCLE_3.md", "Evaluation profile: `ghost` (live).\n")
        commit(self.root)
        self.refused("unknown profile")

    def test_cycle_must_state_the_policy(self):
        write(self.root, "docs/cycles/CYCLE_1.md", "Evaluation profile: `old` (live).\n")
        commit(self.root)
        self.refused("must state the policy")

    def test_unbound_matrix_row(self):
        matrix = json.loads((self.root / ep.MATRIX).read_text())
        matrix["corpus"]["new_row"] = {"safe": 1}
        write(self.root, ep.MATRIX, matrix)
        commit(self.root)
        self.refused("bind to no evaluation profile")

    def test_split_route_coverage(self):
        matrix = json.loads((self.root / ep.MATRIX).read_text())
        matrix["corpus"]["row"]["routes"]["slice"] = {}
        write(self.root, ep.MATRIX, matrix)
        commit(self.root)
        self.refused("bind to no profile")

    def test_orphan_profile(self):
        bindings = copy.deepcopy(self.fixture.bindings)
        bindings["matrix"]["row"] = {"none": "not a result"}
        write(self.root, ep.BINDINGS, bindings)
        commit(self.root)
        self.refused("bound by no cycle")

    def test_pinned_cycle_document(self):
        bindings = copy.deepcopy(self.fixture.bindings)
        bindings["pinned_cycle_documents"] = {
            "docs/cycles/CYCLE_2.md": {"profile": None, "reason": "digest pinned"}}
        write(self.root, ep.BINDINGS, bindings)
        commit(self.root)
        self.refused("must not also carry a binding line")
        write(self.root, "docs/cycles/CYCLE_2.md", "# Cycle 2\n")
        commit(self.root)
        ep.check_bindings(self.root, ep.check_tree(self.root))


if __name__ == "__main__":
    unittest.main()

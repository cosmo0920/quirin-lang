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

"""Scope mapping of the scoped card gate (card GATE-ELASTIC)."""
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import gate_scope as scope_mod  # noqa: E402
from gate_scope import Crate, Model  # noqa: E402

REPO_ROOT = Path(__file__).resolve().parents[2]


def crate(name, directory, deps=(), workspace="root"):
    return Crate(name, workspace, directory, set(deps))


# A small synthetic repository: crates, sources and fixtures, all in memory.
CRATES = [
    crate("qcore-core", "crates/qcore-core"),
    crate("alpha", "crates/alpha", deps=["qcore-core"]),
    crate("beta", "crates/beta", deps=["alpha"]),
    crate("gamma", "crates/gamma"),
    crate("delta", "crates/delta", deps=["gamma"]),
    crate("qcore-compiler", "compiler", deps=["alpha"]),
    crate("prov", "providers/prov", deps=["gamma"], workspace="providers"),
]
FILES = {
    "crates/qcore-core/src/lib.rs": "pub fn core() {}\n",
    "crates/alpha/src/lib.rs": "pub fn alpha() {}\n",
    "crates/alpha/tests/fixtures/ct07/case.qr": "fn main() {}\n",
    "crates/alpha/tests/fixtures/ct07/expected.json": "{}\n",
    "crates/beta/src/lib.rs": "pub fn beta() {}\n",
    # gamma's test reads alpha's fixture by include_str!: the fixture reader.
    "crates/gamma/src/lib.rs": "pub fn gamma() {}\n",
    "crates/gamma/tests/reads_fixture.rs": (
        'const CASE: &str = include_str!("../../alpha/tests/fixtures/ct07/case.qr");\n'
        '#[path = "../../../tests/support/helper.rs"]\nmod helper;\n'
    ),
    "crates/delta/src/lib.rs": "pub fn delta() {}\n",
    "compiler/src/lib.rs": "pub fn compile() {}\n",
    "compiler/tests/examples.rs": (
        'fn root() -> std::path::PathBuf { std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))'
        '.join("..") }\n'
        'fn examples() -> std::path::PathBuf { root().join("examples") }\n'
    ),
    "compiler/examples/inventory.rs": 'const A: &str = "docs/reports/x";\n',
    # The helper reached through #[path] reads its own sibling by include_str!.
    "tests/support/helper.rs": 'pub const DATA: &str = include_str!("helper.json");\n',
    "tests/support/helper.json": "{}\n",
    "examples/hello.qr": "fn main() {}\n",
    "docs/reports/x/README.md": "# x\n",
    "docs/agents/validation.md": "# gate\n",
    "contract/orphan.qv": "fn orphan();\n",
    "providers/prov/src/main.rs": "fn main() {}\n",
    "providers/package.py": "\n",
    "scripts/tool.py": "\n",
    "proof/QCore.lean": "import QCore.A\nimport QCore.B\n",
    "proof/QCore/A.lean": "import Init\n",
    "proof/QCore/B.lean": "import QCore.A\n",
    "proof/QCore/C.lean": "import Init\n",
    "proof/AuditA.lean": "import QCore.A\n",
    "proof/AuditB.lean": "import QCore.B\n",
    "proof/AuditC.lean": "import QCore.C\n",
    "proof/AuditAll.lean": "import QCore\n",
    "proof/lakefile.toml": "name = \"qcore\"\n",
}


def model(granularity="directory", files=FILES):
    return Model(CRATES, sorted(files), read_text=lambda path: files[path],
                 granularity=granularity)


def plan(paths, granularity="directory", files=FILES, **kwargs):
    return scope_mod.plan(paths, model(granularity, files), exists=lambda p: p in files,
                          read_text=lambda p: files[p], **kwargs)


class OwnershipAndReverseDependencies(unittest.TestCase):
    def test_reverse_dependencies_are_closed_over(self):
        scope = plan(["crates/gamma/src/lib.rs"])
        self.assertEqual(scope["mode"], "scoped")
        self.assertEqual(sorted(scope["affected"]), ["delta", "gamma", "prov"])
        self.assertEqual(scope["affected"]["delta"], ["depends on gamma"])
        self.assertEqual(scope["affected"]["gamma"], ["owns crates/gamma/src/lib.rs"])
        # Provider members are gated by the provider gate, not by the root suite.
        self.assertEqual(scope["rust_packages"], ["delta", "gamma"])
        self.assertFalse(scope["provider_gate"])
        self.assertEqual(scope["provider_members_reached_without_providers_change"], ["prov"])

    def test_transitive_reverse_dependency(self):
        scope = plan(["crates/alpha/src/lib.rs"])
        self.assertEqual(sorted(scope["affected"]), ["alpha", "beta", "qcore-compiler"])
        self.assertTrue(scope["probes"], "qcore-compiler is affected, so the probes run")
        self.assertNotIn("gamma", scope["affected"])

    def test_an_unrelated_crate_is_not_selected(self):
        scope = plan(["crates/beta/src/lib.rs"])
        self.assertEqual(sorted(scope["affected"]), ["beta"])
        self.assertFalse(scope["probes"])


READS_FIXTURE = "gamma::test::reads_fixture"


class FixtureReaders(unittest.TestCase):
    def test_a_fixture_read_by_include_str_belongs_to_the_reading_crate(self):
        scope = plan(["crates/alpha/tests/fixtures/ct07/case.qr"])
        # The reader is one of gamma's integration tests (card
        # GATE-SCOPE-PRECISE-SELECTION): that target runs, not all of gamma.
        self.assertIn(READS_FIXTURE, scope["rust_targets"])
        self.assertNotIn("gamma", scope["affected"])
        reasons = " ".join(scope["targets"]["gamma"][READS_FIXTURE])
        self.assertIn("include_str!", reasons)
        self.assertIn("crates/gamma/tests/reads_fixture.rs:1", reasons)
        # Nothing depends on a test target, so gamma's reverse dependencies stay out.
        self.assertNotIn("delta", scope["affected"])
        # The fixture's owner reads it nowhere itself: alpha runs whole (fail-closed).
        self.assertIn("alpha", scope["affected"])

    def test_the_read_directory_belongs_to_the_reader_by_default(self):
        sibling = "crates/alpha/tests/fixtures/ct07/expected.json"
        self.assertIn(READS_FIXTURE, plan([sibling])["rust_targets"])
        self.assertNotIn(READS_FIXTURE, plan([sibling], granularity="file")["rust_targets"])

    def test_a_path_module_and_its_own_includes_belong_to_the_compiling_crate(self):
        scope = plan(["tests/support/helper.rs"])
        self.assertIn("compiles it via #[path]",
                      " ".join(scope["targets"]["gamma"][READS_FIXTURE]))
        # helper.rs is compiled by gamma's test, so its include_str! is that test's read.
        scope = plan(["tests/support/helper.json"], granularity="file")
        self.assertEqual(scope["rust_targets"], [READS_FIXTURE])
        self.assertEqual(scope["mode"], "scoped")

    def test_a_runtime_join_names_a_directory(self):
        scope = plan(["examples/hello.qr"])
        self.assertIn("qcore-compiler::test::examples", scope["rust_targets"])
        self.assertIn('names examples in a string literal',
                      " ".join(scope["targets"]["qcore-compiler"]["qcore-compiler::test::examples"]))
        # No walker declares it, so the whole directory is attributed (fail-closed).
        self.assertIn("examples", [site["literal"]
                                   for site in scope["undeclared_directory_literals"]])

    def test_an_example_s_runtime_literal_is_not_a_test_read(self):
        scope = plan(["docs/reports/x/README.md"])
        self.assertEqual(scope["affected"], {})


class FullSuiteTriggers(unittest.TestCase):
    def test_a_core_crate_forces_the_full_suite(self):
        scope = plan(["crates/qcore-core/src/lib.rs"])
        self.assertEqual(scope["mode"], "full")
        self.assertIn("the diff reaches qcore-core", scope["full_reasons"])
        root_members = sorted(c.name for c in CRATES if c.workspace == "root")
        self.assertEqual(scope["rust_packages"], root_members)

    def test_each_named_core_crate_is_a_trigger(self):
        self.assertEqual(scope_mod.FULL_CRATES,
                         ("qcore-c-gateway", "qcore-core", "qcore-contract", "qcore-schemas"))

    def test_workspace_files_and_the_runner_force_the_full_suite(self):
        for path in ("Cargo.lock", "Cargo.toml", "scripts/run_test_binaries.py",
                     "scripts/gate_scope.py"):
            scope = plan([path], files={**FILES, path: ""})
            self.assertEqual(scope["mode"], "full", path)

    def test_an_unowned_unread_input_fails_closed_to_full(self):
        scope = plan(["contract/orphan.qv"])
        self.assertEqual(scope["mode"], "full")
        self.assertIn("fail-closed", scope["full_reasons"][0])

    def test_prose_and_python_alone_select_no_crate(self):
        scope = plan(["docs/agents/validation.md", "scripts/tool.py"])
        self.assertEqual(scope["mode"], "scoped")
        self.assertEqual(scope["rust_packages"], [])
        self.assertEqual(scope_mod.steps_for(scope, Path("/o"))[-1].name, "python")

    def test_forced_full(self):
        self.assertEqual(plan(["docs/agents/validation.md"], force_full=True)["mode"], "full")


class PythonCheckedRecords(unittest.TestCase):
    """Card RECORDS-TOOLING: contract/evaluations/** is Python-suite data."""

    RECORD = "contract/evaluations/demo/baseline.json"

    def test_a_record_only_diff_selects_no_crate_and_runs_python(self):
        scope = plan([self.RECORD], files={**FILES, self.RECORD: "{}\n"})
        self.assertEqual(scope["mode"], "scoped")
        self.assertEqual(scope["full_reasons"], [])
        self.assertEqual(scope["rust_packages"], [])
        self.assertIn(f"{self.RECORD}: a record the Python suite checks", scope["python_reasons"])
        self.assertIn("class records", scope["attribution"][self.RECORD])
        steps = scope_mod.steps_for(scope, Path("/o"))
        self.assertEqual([step.name for step in steps if step.slot == "rust-suite"], [])
        self.assertEqual(steps[-1].name, "python")

    def test_a_crate_reading_the_records_still_claims_them(self):
        files = {**FILES, self.RECORD: "{}\n",
                 "crates/delta/tests/reads_records.rs": (
                     'fn records() -> std::path::PathBuf { std::path::PathBuf::from('
                     'env!("CARGO_MANIFEST_DIR")).join("../../contract/evaluations") }\n')}
        scope = plan([self.RECORD], files=files)
        self.assertEqual(scope["mode"], "scoped")
        self.assertEqual(scope["rust_packages"], [])
        self.assertEqual(scope["rust_targets"], ["delta::test::reads_records"])
        self.assertIn(f"{self.RECORD}: a record the Python suite checks", scope["python_reasons"])

    def test_contribution_ledgers_are_python_checked_records(self):
        """Card CONTRIBUTION-LEDGERS: a ledger-only diff selects no crate."""
        for ledger in ("contract/contributions/33/eight-home.json",
                       "contract/contributions/interactions/33+35/eight-home.json"):
            scope = plan([ledger], files={**FILES, ledger: "{}\n"})
            self.assertEqual(scope["mode"], "scoped", ledger)
            self.assertEqual(scope["rust_packages"], [], ledger)
            self.assertIn(f"{ledger}: a record the Python suite checks", scope["python_reasons"])
            self.assertIn("class records", scope["attribution"][ledger])

    def test_other_contract_paths_still_fail_closed(self):
        for path in ("contract/orphan.qv", "contract/evaluations.json",
                     "contract/evaluationsX/a.json", "contract/contributions.json",
                     "contract/contributionsX/a.json"):
            scope = plan([path], files={**FILES, path: ""})
            self.assertEqual(scope["mode"], "full", path)
            self.assertIn("fail-closed", scope["full_reasons"][0], path)


class LeanPlanning(unittest.TestCase):
    def test_a_module_change_selects_its_importers_and_their_audits(self):
        lean = plan(["proof/QCore/A.lean"])["lean"]
        self.assertEqual(lean["mode"], "modules")
        self.assertEqual(lean["modules"], ["QCore.A", "QCore.B"])
        self.assertEqual(lean["audits"], ["AuditA.lean", "AuditB.lean"])

    def test_an_audit_change_selects_that_audit(self):
        lean = plan(["proof/AuditC.lean"])["lean"]
        self.assertEqual((lean["modules"], lean["audits"]), ([], ["AuditC.lean"]))

    def test_roots_configuration_deletion_and_the_runner_select_everything(self):
        for path in ("proof/QCore.lean", "proof/AuditAll.lean", "proof/lakefile.toml",
                     "proof/QCore/Gone.lean", "scripts/check_lean_audits.py",
                     "scripts/gate_lean.py"):
            self.assertEqual(plan([path])["lean"]["mode"], "all", path)

    def test_no_proof_change_no_lean_step(self):
        self.assertEqual(plan(["crates/beta/src/lib.rs"])["lean"]["mode"], "none")


class Steps(unittest.TestCase):
    def test_scoped_steps_restrict_the_suite_and_clippy(self):
        scope = plan(["crates/gamma/src/lib.rs"])
        steps = {s.name: s for s in scope_mod.steps_for(scope, Path("/o"))}
        suite = steps["rust-suite"]
        self.assertEqual(suite.slot, "rust-suite")
        self.assertIn("--packages", suite.argv)
        self.assertEqual(suite.argv[suite.argv.index("--packages") + 1], "delta,gamma")
        self.assertEqual(steps["clippy"].argv[:6], ["cargo", "clippy", "-p", "delta", "-p", "gamma"])
        self.assertNotIn("lean", steps)
        self.assertNotIn("provider-test", steps)

    def test_full_steps_are_the_standing_gate_commands(self):
        scope = plan(["crates/qcore-core/src/lib.rs", "proof/QCore.lean"])
        steps = {s.name: s for s in scope_mod.steps_for(scope, Path("/o"))}
        self.assertNotIn("--packages", steps["rust-suite"].argv)
        self.assertEqual(steps["clippy"].argv,
                         ["cargo", "clippy", "--all-targets", "--", "-D", "warnings"])
        self.assertEqual(steps["fmt"].argv, ["cargo", "fmt", "--all", "--", "--check"])
        self.assertTrue(steps["lean"].argv[-1].endswith("check_lean_audits.py"))
        self.assertEqual(steps["lean"].slot, "lean")
        self.assertIn("probe-reevaluation_probe", steps)
        self.assertIn("probe-reevaluation_followup", steps)

    def test_a_providers_change_runs_the_provider_gate_under_the_suite_slot(self):
        scope = plan(["providers/package.py"])
        self.assertTrue(scope["provider_gate"])
        steps = {s.name: s for s in scope_mod.steps_for(scope, Path("/o"))}
        self.assertEqual(steps["provider-test"].slot, "rust-suite")
        self.assertIn("/o/provider-packages", steps["provider-package-build"].argv)

    def test_provider_commands_equal_the_provider_readme_block(self):
        text = (REPO_ROOT / "providers" / "README.md").read_text(encoding="utf-8")
        section = text.split("## Build, verify and reproduce", 1)[1]
        block = re.findall(r"```sh\r?\n(.*?)```", section, re.S)[0]
        self.assertEqual([line for line in block.splitlines() if line.strip()],
                         list(scope_mod.PROVIDER_COMMANDS))


class GateLeanRefusals(unittest.TestCase):
    """scripts/gate_lean.py refuses before invoking Lake."""

    def setUp(self):
        import gate_lean
        self.gate_lean = gate_lean

    def quiet(self, argv):
        with open(os.devnull, "w") as sink:
            stderr, sys.stderr = sys.stderr, sink
            try:
                return self.gate_lean.main(argv)
            finally:
                sys.stderr = stderr

    def test_audit_all_belongs_to_the_full_run(self):
        self.assertEqual(self.quiet(["--audit", "AuditAll.lean"]), 1)

    def test_a_missing_or_non_audit_file_refuses(self):
        self.assertEqual(self.quiet(["--audit", "AuditNoSuchGateElastic.lean"]), 1)
        self.assertEqual(self.quiet(["--audit", "QCore.lean"]), 1)

    def test_nothing_to_do_is_a_usage_error(self):
        with self.assertRaises(SystemExit) as raised:
            self.quiet([])
        self.assertEqual(raised.exception.code, 2)


class GitDiff(unittest.TestCase):
    def git(self, root, *args):
        subprocess.run(["git", "-C", str(root), *args], check=True, capture_output=True,
                       env={**os.environ, "GIT_AUTHOR_NAME": "t", "GIT_AUTHOR_EMAIL": "t@t",
                            "GIT_COMMITTER_NAME": "t", "GIT_COMMITTER_EMAIL": "t@t"})

    def test_both_sides_of_a_rename_and_uncommitted_paths_are_in_scope(self):
        if shutil.which("git") is None:
            self.skipTest("git is not installed")
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.git(root, "init", "-q")
            (root / "old.txt").write_text("same content\n" * 20)
            (root / "keep.txt").write_text("k\n")
            self.git(root, "add", ".")
            self.git(root, "commit", "-qm", "base")
            base = subprocess.run(["git", "-C", str(root), "rev-parse", "HEAD"],
                                  capture_output=True, text=True, check=True).stdout.strip()
            self.git(root, "mv", "old.txt", "new.txt")
            self.git(root, "commit", "-qm", "rename")
            (root / "keep.txt").write_text("changed\n")
            (root / "untracked.txt").write_text("u\n")
            base_sha, _, committed, uncommitted = scope_mod.diff_paths(base, root)
            self.assertEqual(base_sha, base)
            self.assertEqual(committed, ["new.txt", "old.txt"])
            self.assertEqual(uncommitted, ["keep.txt", "untracked.txt"])

    def test_an_unknown_base_refuses(self):
        with self.assertRaises(scope_mod.ScopeError):
            scope_mod.diff_paths("no-such-revision-gate-elastic", REPO_ROOT)


@unittest.skipIf(shutil.which("cargo") is None, "cargo is not installed")
class RealRepository(unittest.TestCase):
    """The scanner over this repository finds the readers named in the records."""

    @classmethod
    def setUpClass(cls):
        cls.model = Model(scope_mod.load_crates(), scope_mod.tracked_files())

    def test_both_workspaces_are_mapped(self):
        names = self.model.crates
        self.assertEqual(names["qcore-compiler"].directory, "compiler")
        self.assertEqual(names["qcore-provider-conformance"].workspace, "providers")
        self.assertIn("qcore-audit", self.model.reverse["qcore-c-gateway"])
        self.assertIn("qcore-provider-conformance", self.model.reverse["qcore-audit"])

    def test_known_cross_crate_readers_are_found(self):
        readers = {(r.crate, r.kind) for r in self.model.readers_of(
            "crates/qcore-c-gateway/tests/fixtures/contracts/q_base.qv")}
        self.assertIn(("qcore-compiler", "include_str"), readers)
        readers = {r.crate for r in self.model.readers_of("tests/support/tracked_files.rs")}
        self.assertLessEqual({"qcore-c-gateway", "qcore-compiler", "qcore-provider-conformance"},
                             readers)
        # CT-FIXTURE-EXTERNAL-READERS' own example: write_lend.rs reads CT07.
        # File granularity (the default since 2026-10-25) claims exactly the
        # file the macro compiles in; directory granularity claims its siblings.
        def write_lend_reads(model, path):
            return "compiler/tests/write_lend.rs" in {
                r.source.split(":")[0] for r in model.readers_of(path)
                if r.kind == "include_str"}
        read = "compiler/tests/fixtures/ct/CT07/sequential_write_halves.qr"
        sibling = "compiler/tests/fixtures/ct/CT07/case.qr"
        self.assertTrue(write_lend_reads(self.model, read))
        self.assertFalse(write_lend_reads(self.model, sibling))
        by_directory = Model(scope_mod.load_crates(), scope_mod.tracked_files(),
                             granularity="directory")
        self.assertTrue(write_lend_reads(by_directory, sibling))

    def test_evaluation_records_are_claimed_by_their_readers(self):
        """GATE-SCOPE-EVALUATION-RECORDS: no crate owns a tracked
        contract/evaluations path. A crate that reads one (MEASURE-TOOL's
        quirin-audit-plugin-measure reads the profile records at run time) still
        claims it, so a records-only diff runs that crate's tests as well as
        the Python suite, and never the full suite on records alone."""
        records = [path for path in scope_mod.tracked_files()
                   if path.startswith(scope_mod.PYTHON_RECORD_PREFIXES)]
        self.assertTrue(records)
        for path in records:
            self.assertIsNone(self.model.owner(path), path)
            readers = sorted({reader.crate for reader in self.model.readers_of(path)})
            result = scope_mod.plan([path], self.model)
            self.assertEqual(result["mode"], "scoped", (path, result["full_reasons"]))
            self.assertTrue(set(readers) <= set(result["rust_packages"]), (path, readers))
            self.assertTrue(result["python_reasons"], path)

    def test_the_census_is_json(self):
        census = scope_mod.cross_crate_readers(self.model)
        self.assertTrue(census["reads"])
        json.dumps(census)


class DefaultGranularityTests(unittest.TestCase):
    """User decision 2026-10-25: a compile-time read claims only the file read."""

    def test_the_model_default_is_file(self):
        model = scope_mod.Model([], [], read_text=lambda path: "")
        self.assertEqual(model.granularity, "file")

    def test_the_cli_default_is_file(self):
        import inspect
        self.assertIn('default="file"', inspect.getsource(scope_mod.main))


if __name__ == "__main__":
    unittest.main()

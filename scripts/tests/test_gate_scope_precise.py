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

"""Precise selection of the scoped card gate (card GATE-SCOPE-PRECISE-SELECTION).

R1 (GATE-SCOPE-RUNTIME-READS-HEURISTIC): a directory literal is narrowed only by
a declared walker; an undeclared one keeps its whole directory.  R2
(GATE-SCOPE-FULL-CRATES-TEST-ONLY): a change that reaches a crate only through
its integration tests selects those test targets, also for a FULL_CRATES
member.  R6: GATE-SCOPE-PROVIDER-LOCK and CARD-GATE-NEW-AUDIT-MATRIX-COUNT.
"""
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import gate_scope as scope_mod  # noqa: E402
import run_test_binaries as runner_mod  # noqa: E402
from gate_scope import Crate, Model  # noqa: E402

REPO_ROOT = Path(__file__).resolve().parents[2]


def crate(name, directory, deps=(), workspace="root"):
    return Crate(name, workspace, directory, set(deps))


CRATES = [
    crate("qcore-c-gateway", "crates/qcore-c-gateway"),
    crate("user", "crates/user", deps=["qcore-c-gateway"]),
    crate("walker", "crates/walker"),
    crate("qcore-compiler", "compiler"),
    crate("prov", "providers/prov", deps=["qcore-c-gateway"], workspace="providers"),
]
FILES = {
    "crates/qcore-c-gateway/Cargo.toml": "[package]\n",
    "crates/qcore-c-gateway/build.rs": "fn main() {}\n",
    "crates/qcore-c-gateway/src/lib.rs": "pub mod inner;\n",
    "crates/qcore-c-gateway/src/inner.rs": "pub fn f() {}\n",
    "crates/qcore-c-gateway/tests/one.rs": (
        "mod common;\n"
        'const A: &str = include_str!("fixtures/one.json");\n'),
    "crates/qcore-c-gateway/tests/two.rs": (
        "mod common;\n"
        'fn data() -> &\'static str { "tests/fixtures/two.json" }\n'),
    "crates/qcore-c-gateway/tests/alone.rs": "#[test] fn t() {}\n",
    "crates/qcore-c-gateway/tests/common/mod.rs": "pub fn helper() {}\n",
    "crates/qcore-c-gateway/tests/fixtures/one.json": "{}\n",
    "crates/qcore-c-gateway/tests/fixtures/two.json": "{}\n",
    "crates/qcore-c-gateway/tests/fixtures/unread.json": "{}\n",
    "crates/user/src/lib.rs": "pub fn u() {}\n",
    "crates/walker/src/lib.rs": "pub fn w() {}\n",
    # A walker of contracts/ filtered to .qv, and a bare directory join no
    # walker declares.
    "crates/walker/tests/walks.rs": (
        'fn walk() { tracked_files(&root().join("contracts")).filter(|p| p.ends_with(".qv")); }\n'),
    "crates/walker/tests/joins.rs": 'fn base() { root().join("data"); }\n',
    "crates/walker/tests/names_base.rs": 'fn base() { root().join("docs"); }\n',
    "compiler/src/lib.rs": 'pub const DEFAULT: &str = "contracts/load-path.json";\n',
    "contracts/a.qv": "fn a();\n",
    "contracts/sub/b.qv": "fn b();\n",
    "contracts/README.md": "# contracts\n",
    "contracts/load-path.json": "{}\n",
    "data/x.txt": "x\n",
    "docs/guide.md": "# guide\n",
    "providers/prov/src/main.rs": "fn main() {}\n",
}
WALKERS = [
    {"source": "crates/walker/tests/walks.rs", "literal": "contracts",
     "reads": ["contracts/**/*.qv"], "why": "tracked_files(contracts) filtered to .qv"},
    {"source": "crates/walker/tests/names_base.rs", "literal": "docs", "reads": [],
     "why": "a base path; nothing under it is read"},
]
WALKS = "walker::test::walks"


def model(files=FILES, walkers=WALKERS, crates=CRATES):
    return Model(crates, sorted(files), read_text=lambda path: files[path], walkers=walkers)


def plan(paths, files=FILES, walkers=WALKERS, crates=CRATES, **kwargs):
    return scope_mod.plan(paths, model(files, walkers, crates), exists=lambda p: p in files,
                          read_text=lambda p: files[p], **kwargs)


class TestOnlyChangesStayScoped(unittest.TestCase):
    """R2: GATE-SCOPE-FULL-CRATES-TEST-ONLY."""

    def test_an_integration_test_of_a_full_crate_selects_that_test_alone(self):
        scope = plan(["crates/qcore-c-gateway/tests/alone.rs"])
        self.assertEqual(scope["mode"], "scoped", scope["full_reasons"])
        self.assertEqual(scope["rust_targets"], ["qcore-c-gateway::test::alone"])
        self.assertEqual(scope["rust_packages"], [])
        # Nothing depends on a test target: no reverse dependency joins.
        self.assertNotIn("user", scope["affected"])

    def test_a_shared_test_module_selects_every_test_that_declares_it(self):
        scope = plan(["crates/qcore-c-gateway/tests/common/mod.rs"])
        self.assertEqual(scope["mode"], "scoped")
        self.assertEqual(scope["rust_targets"], ["qcore-c-gateway::test::one",
                                                 "qcore-c-gateway::test::two"])

    def test_a_fixture_selects_its_readers_only(self):
        scope = plan(["crates/qcore-c-gateway/tests/fixtures/one.json"])
        self.assertEqual(scope["rust_targets"], ["qcore-c-gateway::test::one"])
        scope = plan(["crates/qcore-c-gateway/tests/fixtures/two.json"])
        self.assertEqual(scope["rust_targets"], ["qcore-c-gateway::test::two"])

    def test_a_fixture_nothing_is_seen_reading_fails_closed_to_the_whole_crate(self):
        scope = plan(["crates/qcore-c-gateway/tests/fixtures/unread.json"])
        self.assertEqual(scope["mode"], "full")
        self.assertIn("the diff reaches qcore-c-gateway", scope["full_reasons"])

    def test_src_the_manifest_and_the_build_script_stay_full(self):
        for path in ("crates/qcore-c-gateway/src/inner.rs", "crates/qcore-c-gateway/Cargo.toml",
                     "crates/qcore-c-gateway/build.rs"):
            scope = plan([path])
            self.assertEqual(scope["mode"], "full", path)
            self.assertIn("the diff reaches qcore-c-gateway", scope["full_reasons"], path)

    def test_a_test_file_with_a_library_change_is_full(self):
        scope = plan(["crates/qcore-c-gateway/tests/alone.rs",
                      "crates/qcore-c-gateway/src/lib.rs"])
        self.assertEqual(scope["mode"], "full")

    def test_an_rs_file_no_target_compiles_fails_closed(self):
        files = {**FILES, "crates/qcore-c-gateway/tests/orphan/stray.rs": "fn x() {}\n"}
        scope = plan(["crates/qcore-c-gateway/tests/orphan/stray.rs"], files=files)
        self.assertEqual(scope["mode"], "full")

    def test_target_steps_restrict_clippy_and_the_suite(self):
        scope = plan(["crates/qcore-c-gateway/tests/alone.rs"])
        steps = {s.name: s for s in scope_mod.steps_for(scope, Path("/o"))}
        self.assertNotIn("clippy", steps)
        self.assertEqual(steps["clippy-qcore-c-gateway"].argv[:6],
                         ["cargo", "clippy", "-p", "qcore-c-gateway", "--test", "alone"])
        suite = steps["rust-suite"].argv
        self.assertNotIn("--packages", suite)
        self.assertEqual(suite[suite.index("--targets") + 1], "qcore-c-gateway::test::alone")
        self.assertEqual(steps["rust-suite"].slot, "rust-suite")

    def test_metadata_targets_override_the_inferred_layout(self):
        target = scope_mod.Target("renamed", ("test",), "crates/qcore-c-gateway/tests/alone.rs")
        lib = scope_mod.Target("qcore_c_gateway", ("lib",), "crates/qcore-c-gateway/src/lib.rs")
        crates = [Crate("qcore-c-gateway", "root", "crates/qcore-c-gateway", set(),
                        targets=[target, lib])]
        scope = plan(["crates/qcore-c-gateway/tests/alone.rs"], crates=crates)
        self.assertEqual(scope["rust_targets"], ["qcore-c-gateway::test::renamed"])
        # one.rs is not a target here: nothing compiles it, so it is crate-wide.
        self.assertEqual(plan(["crates/qcore-c-gateway/tests/one.rs"], crates=crates)["mode"],
                         "full")


class DeclaredWalkers(unittest.TestCase):
    """R1: GATE-SCOPE-RUNTIME-READS-HEURISTIC."""

    def test_a_declared_walker_claims_only_the_files_it_filters(self):
        self.assertEqual(plan(["contracts/a.qv"])["rust_targets"], [WALKS])
        self.assertEqual(plan(["contracts/sub/b.qv"])["rust_targets"], [WALKS])
        # A new file the filter admits is claimed too (the walk would see it).
        files = {**FILES, "contracts/new.qv": "fn n();\n"}
        self.assertEqual(plan(["contracts/new.qv"], files=files)["rust_targets"], [WALKS])
        # A file the filter rejects is not.
        scope = plan(["contracts/README.md"])
        self.assertNotIn(WALKS, scope["rust_targets"])

    def test_an_undeclared_directory_literal_keeps_its_whole_directory(self):
        scope = plan(["data/x.txt"])
        self.assertEqual(scope["rust_targets"], ["walker::test::joins"])
        self.assertEqual([(s["literal"], s["source"].split(":")[0])
                          for s in scope["undeclared_directory_literals"]],
                         [("data", "crates/walker/tests/joins.rs")])

    def test_without_the_walker_the_literal_claims_the_whole_directory(self):
        scope = plan(["contracts/README.md"], walkers=[])
        self.assertIn(WALKS, scope["rust_targets"])

    def test_a_declared_base_path_claims_nothing(self):
        scope = plan(["docs/guide.md"])
        self.assertEqual(scope["rust_targets"], [])
        self.assertEqual(scope["mode"], "scoped")

    def test_a_file_literal_claims_exactly_that_file(self):
        scope = plan(["contracts/load-path.json"])
        # compiler/src names it: the library reads it, so the crate runs whole.
        self.assertIn("qcore-compiler", scope["affected"])
        self.assertNotIn("qcore-compiler", plan(["contracts/a.qv"])["affected"])

    def test_a_walker_keyed_on_a_file_literal_adds_its_data_derived_reads(self):
        walkers = WALKERS + [{"source": "compiler/src/lib.rs",
                              "literal": "contracts/load-path.json",
                              "reads": ["contracts/*.qv"], "why": "the load path lists them"}]
        scope = plan(["contracts/a.qv"], walkers=walkers)
        self.assertIn("qcore-compiler", scope["affected"])
        self.assertNotIn("qcore-compiler", plan(["contracts/sub/b.qv"], walkers=walkers)["affected"])

    def test_a_manifest_expansion_gives_its_listed_files_the_manifest_s_readers(self):
        expansions = [{"file": "contracts/load-path.json", "reads": ["contracts/sub/*.qv"],
                       "why": "the manifest lists contracts/sub"}]
        m = Model(CRATES, sorted(FILES), read_text=lambda path: FILES[path], walkers=WALKERS,
                  expansions=expansions)
        scope = scope_mod.plan(["contracts/sub/b.qv"], m, exists=lambda p: p in FILES,
                               read_text=lambda p: FILES[p])
        # compiler/src names the manifest, so the library reads what it lists.
        self.assertIn("qcore-compiler", scope["affected"])
        self.assertIn("which lists contracts/sub/b.qv", " ".join(scope["affected"]["qcore-compiler"]))
        scope = scope_mod.plan(["contracts/a.qv"], m, exists=lambda p: p in FILES,
                               read_text=lambda p: FILES[p])
        self.assertNotIn("qcore-compiler", scope["affected"])

    def test_a_walker_whose_site_is_gone_is_reported_stale(self):
        walkers = WALKERS + [{"source": "crates/walker/tests/gone.rs", "literal": "x",
                              "reads": [], "why": "removed"}]
        self.assertEqual([entry["source"] for entry in plan(["docs/guide.md"],
                                                            walkers=walkers)["stale_walkers"]],
                         ["crates/walker/tests/gone.rs"])

    def test_globs(self):
        regex = scope_mod.glob_regex("providers/*/contracts/**/*.qv")
        self.assertTrue(regex.match("providers/a/contracts/x.qv"))
        self.assertTrue(regex.match("providers/a/contracts/d/e/x.qv"))
        self.assertFalse(regex.match("providers/a/b/contracts/x.qv"))
        self.assertFalse(regex.match("providers/a/contracts/x.qvx"))
        self.assertTrue(scope_mod.glob_regex("a/**").match("a/b/c"))
        self.assertFalse(scope_mod.glob_regex("a/?.rs").match("a/bc.rs"))

    def test_a_malformed_table_refuses(self):
        cases = [
            {"schema": "other", "walkers": []},
            {"schema": scope_mod.WALKERS_SCHEMA, "walkers": [{"source": "a", "literal": "b",
                                                               "reads": [], "why": ""}]},
            {"schema": scope_mod.WALKERS_SCHEMA, "walkers": [{"source": "a", "literal": "b",
                                                               "reads": "x", "why": "w"}]},
            {"schema": scope_mod.WALKERS_SCHEMA, "walkers": [
                {"source": "a", "literal": "b", "reads": [], "why": "w"},
                {"source": "a", "literal": "b", "reads": ["c"], "why": "w"}]},
        ]
        with tempfile.TemporaryDirectory() as directory:
            for index, document in enumerate(cases):
                path = Path(directory) / f"{index}.json"
                path.write_text(json.dumps(document))
                with self.assertRaises(scope_mod.ScopeError, msg=document):
                    scope_mod.load_walkers(path)

    def test_the_committed_table_loads(self):
        walkers = scope_mod.load_walkers()
        self.assertTrue(all(entry["why"] for entry in walkers))


class ProviderLock(unittest.TestCase):
    """R6: GATE-SCOPE-PROVIDER-LOCK."""

    def test_a_manifest_the_provider_workspace_builds_selects_the_lock_check(self):
        scope = plan(["crates/qcore-c-gateway/Cargo.toml"])
        self.assertTrue(scope["provider_lock_check"])
        self.assertFalse(scope["provider_gate"])
        steps = {s.name: s for s in scope_mod.steps_for(scope, Path("/o"))}
        self.assertEqual(steps["provider-lock"].argv[:3], ["cargo", "metadata", "--locked"])
        self.assertIn(str(REPO_ROOT / "providers" / "Cargo.toml"), steps["provider-lock"].argv)

    def test_the_root_lock_and_manifest_select_it(self):
        for path in ("Cargo.lock", "Cargo.toml"):
            files = {**FILES, path: ""}
            self.assertTrue(plan([path], files=files)["provider_lock_check"], path)

    def test_a_crate_outside_the_provider_closure_does_not(self):
        files = {**FILES, "crates/walker/Cargo.toml": "[package]\n"}
        self.assertFalse(plan(["crates/walker/Cargo.toml"], files=files)["provider_lock_check"])

    def test_the_provider_gate_already_resolves_the_lock(self):
        scope = plan(["crates/qcore-c-gateway/Cargo.toml", "providers/prov/src/main.rs"])
        self.assertTrue(scope["provider_gate"])
        self.assertFalse(scope["provider_lock_check"])

    def test_the_closure_is_transitive(self):
        crates = CRATES + [crate("deep", "crates/deep"),
                           crate("mid", "crates/mid", deps=["deep"]),
                           crate("prov2", "providers/prov2", deps=["mid"], workspace="providers")]
        self.assertIn("deep", scope_mod.provider_path_closure(crates))
        self.assertNotIn("walker", scope_mod.provider_path_closure(crates))


class CardGateBase(unittest.TestCase):
    """R6: CARD-GATE-NEW-AUDIT-MATRIX-COUNT, the gate side."""

    def test_a_card_gate_passes_its_base_to_the_python_suite(self):
        scope = plan(["docs/guide.md"])
        steps = {s.name: s for s in scope_mod.steps_for(scope, Path("/o"), card_base="abc")}
        self.assertEqual(steps["python"].env, {scope_mod.CARD_GATE_BASE_ENV: "abc"})
        steps = {s.name: s for s in scope_mod.steps_for(scope, Path("/o"))}
        self.assertIsNone(steps["python"].env)

    def test_the_variable_names_agree(self):
        text = (Path(__file__).resolve().parent / "test_build_matrix_sections.py").read_text(
            encoding="utf-8")
        self.assertIn(f'CARD_GATE_BASE_ENV = "{scope_mod.CARD_GATE_BASE_ENV}"', text)

    def test_the_landing_flag_is_offered(self):
        import inspect
        self.assertIn('"--landing"', inspect.getsource(scope_mod.main))
        self.assertIn("card_base = None if args.landing else base_sha",
                      inspect.getsource(scope_mod.main))


class RunnerTargets(unittest.TestCase):
    """run_test_binaries.py --targets."""

    DOCUMENT = {"packages": [
        {"name": "a", "targets": [{"kind": ["lib"], "name": "a"}, {"kind": ["test"], "name": "x"},
                                  {"kind": ["test"], "name": "y"}]},
        {"name": "b", "targets": [{"kind": ["bin"], "name": "b"}, {"kind": ["test"], "name": "z"}]},
    ]}

    def test_targets_are_grouped_by_member(self):
        self.assertIsNone(runner_mod.parse_targets(None, self.DOCUMENT, None))
        self.assertEqual(runner_mod.parse_targets(["a::test::y,b::test::z", "a::test::x"],
                                                  self.DOCUMENT, None),
                         {"a": [("test", "x"), ("test", "y")], "b": [("test", "z")]})

    def test_a_bad_target_refuses(self):
        # A library's unit-test executable (a::lib::a) is accepted since card
        # GATE-SCOPE-LIB-UNIT-TEST-MODULES; a binary's, a wrong library name or
        # kind, and a test named as a library still refuse.
        for values, packages in ((["a::test::nope"], None), (["c::test::x"], None),
                                 (["a::lib::wrong"], None), (["b::bin::b"], None),
                                 (["a::rlib::a"], None), (["a::lib::x"], None),
                                 (["a::x"], None), ([" , "], None),
                                 (["a::test::x"], ["a"]), (["a::lib::a"], ["a"])):
            with self.assertRaises(ValueError, msg=(values, packages)):
                runner_mod.parse_targets(values, self.DOCUMENT, packages)

    def quiet_main(self, argv):
        with open(os.devnull, "w") as sink:
            stderr, sys.stderr = sys.stderr, sink
            try:
                return runner_mod.main(argv)
            finally:
                sys.stderr = stderr

    @unittest.skipIf(shutil.which("cargo") is None, "cargo is not installed")
    def test_refusals_happen_before_building(self):
        for extra in (["--targets", "qcorec::test::no-such-target-r19a"],
                      ["--targets", "qcorec::test::cli", "--packages", "qcorec"],
                      ["--targets", "qcorec::test::cli", "--batch-size", "4"]):
            with tempfile.TemporaryDirectory() as directory:
                log_dir = Path(directory) / "logs"
                self.assertEqual(self.quiet_main(["--log-dir", str(log_dir), *extra]), 2, extra)
                self.assertFalse(log_dir.exists(), "nothing was built or run")

    def test_the_summary_names_the_targets(self):
        with tempfile.TemporaryDirectory() as directory:
            document = runner_mod.write_summary(Path(directory), [], [], {}, 1, None, [],
                                                targets=["a::test::x"])
            self.assertEqual(document["targets"], ["a::test::x"])
            self.assertEqual(document["packages"], [])


@unittest.skipIf(shutil.which("cargo") is None or shutil.which("git") is None,
                 "cargo or git is not installed")
class RealRepositoryPrecision(unittest.TestCase):
    """The plan page's probes and the GATE-SCOPE-PROVIDER-LOCK replay on this tree."""

    @classmethod
    def setUpClass(cls):
        cls.model = scope_mod.real_model()

    def test_a_gateway_test_alone_is_scoped(self):
        scope = scope_mod.plan(["crates/qcore-c-gateway/tests/libc_gap_declarations.rs"],
                               self.model)
        self.assertEqual(scope["mode"], "scoped", scope["full_reasons"])
        self.assertIn("qcore-c-gateway::test::libc_gap_declarations", scope["rust_targets"])
        self.assertNotIn("qcore-c-gateway", scope["affected"])

    def test_the_rule32a_landing_diff_selects_the_provider_lock_check(self):
        # The RULE32A-PILOT landing (c82895503..62da8997d) added
        # rule-32a-caller-frame-hold to qcore-c-gateway's dependencies;
        # providers/Cargo.lock went stale because its gate chose no provider
        # step (docs/reports/landing-provider-lock/).  These are that range's
        # paths outside docs/ (`git diff --name-only c82895503 62da8997d`).
        paths = ["Cargo.lock", "Cargo.toml", "compiler/tests/workspace_layering.rs",
                 "crates/gateway-rules/registry.toml",
                 "crates/gateway-rules/rule-32a-caller-frame-hold/Cargo.toml",
                 "crates/gateway-rules/rule-32a-caller-frame-hold/src/hold.rs",
                 "crates/gateway-rules/rule-32a-caller-frame-hold/src/lib.rs",
                 "crates/gateway-rules/rule-32a-caller-frame-hold/tests/unit/hold.rs",
                 "crates/qcore-c-gateway/Cargo.toml",
                 "crates/qcore-c-gateway/src/gateway_rule_registry.rs",
                 "crates/qcore-c-gateway/src/gateway_rules.rs",
                 "crates/qcore-c-gateway/src/legacy_caller_frame_hold.rs",
                 "crates/qcore-c-gateway/tests/rule32a_hold.rs",
                 "crates/qcore-gateway-rule-api/src/lib.rs",
                 "proof/AuditCallerFrameHoldSwitch.lean", "proof/QCore.lean",
                 "proof/QCore/CallerFrameHoldSwitch.lean", "scripts/tests/test_gateway_registry.py"]
        scope = scope_mod.plan(paths, self.model)
        self.assertFalse(scope["provider_gate"])
        self.assertTrue(scope["provider_lock_check"])
        self.assertIn("provider-lock", [s.name for s in scope_mod.steps_for(scope, Path("/o"))])

    def test_the_provider_closure_holds_the_gateway(self):
        self.assertIn("qcore-c-gateway", self.model.provider_closure)
        self.assertIn("qcore-schemas", self.model.provider_closure)


if __name__ == "__main__":
    unittest.main()

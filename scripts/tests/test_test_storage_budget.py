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

"""Card TEST-STORAGE-BUDGET: the test-build configuration, its rustc wrapper
and the runner's batches."""
import json
import os
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import run_test_binaries as runner_mod  # noqa: E402
import rustc_test_dylib as wrapper  # noqa: E402
import test_link_config  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
MEMBER = str(ROOT / "crates" / "qcore-core")


class ConfigTests(unittest.TestCase):
    def test_linux_links_dynamically_through_the_wrapper(self):
        args = test_link_config.cargo_config_args(platform="linux")
        self.assertEqual(args[:2], ["--config", "build.incremental=false"])
        self.assertEqual(args[2], "--config")
        key, value = args[3].split("=", 1)
        self.assertEqual(key, "build.rustc-wrapper")
        self.assertEqual(Path(json.loads(value)), test_link_config.WRAPPER)
        self.assertTrue(test_link_config.WRAPPER.exists())

    def test_other_platforms_link_statically(self):
        for platform in ("win32", "darwin"):
            self.assertEqual(test_link_config.default_link(platform), "static")
            self.assertEqual(test_link_config.cargo_config_args(platform=platform),
                             ["--config", "build.incremental=false"])

    def test_an_explicit_mode_wins_and_an_unknown_one_is_refused(self):
        self.assertEqual(test_link_config.cargo_config_args("static", platform="linux"),
                         ["--config", "build.incremental=false"])
        with self.assertRaises(ValueError):
            test_link_config.cargo_config_args("shared")


class WrapperTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.deps = Path(self.directory.name) / "deps"
        self.deps.mkdir()
        for name in ("libqcore_core-1.rlib", "libqcore_core-1.so", "libserde-2.rlib",
                     "libserde-2.rmeta", "libqcore_schemas-3.rmeta"):
            (self.deps / name).write_bytes(b"")

    def tearDown(self):
        self.directory.cleanup()

    def args(self, *extra):
        return ["--crate-name", "qcore_frontend", "--emit=dep-info,metadata,link",
                "-L", f"dependency={self.deps}",
                "--extern", f"qcore_core={self.deps}/libqcore_core-1.rlib",
                "--extern", f"serde={self.deps}/libserde-2.rmeta", *extra]

    def test_a_library_becomes_rlib_and_dylib_linked_dynamically(self):
        out = wrapper.rewrite(self.args("--crate-type", "lib"), MEMBER, "/sysroot/lib")
        self.assertEqual(out[-8:-6], ["--crate-type", "dylib"])
        self.assertIn("prefer-dynamic", out)
        self.assertIn(f"link-arg=-Wl,-rpath,{self.deps.resolve()}", out)
        self.assertIn(f"link-arg=-Wl,-rpath,{Path('/sysroot/lib').resolve()}", out)
        externs = [out[i + 1] for i, arg in enumerate(out) if arg == "--extern"]
        # The rlib and its .so; an rmeta becomes the rlib it stands for.
        self.assertEqual(externs, [f"qcore_core={self.deps}/libqcore_core-1.rlib",
                                   f"qcore_core={self.deps}/libqcore_core-1.so",
                                   f"serde={self.deps}/libserde-2.rlib"])

    def test_every_per_unit_output_directory_is_on_the_runpath(self):
        # CI-CARGO-BUILD-DIR-LAYOUT: Cargo 1.100 builds each unit into its own
        # build/<pkg>/<hash>/out and passes one `-L dependency=` per unit; a
        # runpath holding only the first left the others' dylibs unloadable.
        units = [Path(self.directory.name) / "build" / name / "h" / "out"
                 for name in ("rule-35", "itoa")]
        for unit in units:
            unit.mkdir(parents=True)
        (units[0] / "librule_35-9.rlib").write_bytes(b"")
        (units[0] / "librule_35-9.so").write_bytes(b"")
        args = ["--crate-name", "qcore_c_gateway", "--emit=dep-info,metadata,link",
                "-L", f"dependency={units[1]}", "-L", f"dependency={units[0]}",
                "--extern", f"rule_35={units[0]}/librule_35-9.rlib", "--test"]
        out = wrapper.rewrite(args, MEMBER, "/sysroot/lib")
        runpath = [arg.split("-rpath,", 1)[1] for arg in out if "-rpath," in arg]
        self.assertEqual(runpath, [str(units[1].resolve()), str(units[0].resolve()),
                                   str(Path("/sysroot/lib").resolve())])

    def test_an_rmeta_without_its_rlib_is_kept(self):
        self.assertEqual(wrapper.linkable_extern(f"s={self.deps}/libqcore_schemas-3.rmeta"),
                         [f"s={self.deps}/libqcore_schemas-3.rmeta"])

    def test_a_test_harness_links_dynamically_without_becoming_a_dylib(self):
        out = wrapper.rewrite(self.args("--test"), MEMBER, "/sysroot/lib")
        self.assertNotIn("dylib", out)
        self.assertIn("prefer-dynamic", out)

    def test_what_passes_through_unchanged(self):
        unchanged = [
            (self.args("--crate-type", "lib"), str(ROOT / "providers" / "conformance")),
            (self.args("--crate-type", "lib"), "/home/user/.cargo/registry/src/serde"),
            (self.args("--crate-type", "proc-macro"), MEMBER),
            (["--crate-name", "build_script_build", "--crate-type", "bin"], MEMBER),
            (["--crate-name", "x", "--crate-type", "lib", "--emit=dep-info,metadata"], MEMBER),
            (["-vV"], MEMBER),
            (["-", "--crate-name", "___", "--print=file-names", "--crate-type", "bin"], MEMBER),
        ]
        for args, manifest_dir in unchanged:
            self.assertEqual(wrapper.rewrite(args, manifest_dir, "/sysroot/lib"), args)

    def test_only_the_metadata_announcement_is_withheld(self):
        self.assertTrue(wrapper.withheld(
            b'{"$message_type":"artifact","artifact":"/t/libx.rmeta","emit":"metadata"}\n'))
        self.assertFalse(wrapper.withheld(
            b'{"$message_type":"artifact","artifact":"/t/x.d","emit":"dep-info"}\n'))
        self.assertFalse(wrapper.withheld(b'{"$message_type":"diagnostic","message":"x"}\n'))
        self.assertFalse(wrapper.withheld(b"warning: plain text\n"))


class BatchTests(unittest.TestCase):
    DOCUMENT = {"packages": [
        {"name": "a", "targets": [
            {"kind": ["lib"], "name": "a", "test": True},
            {"kind": ["bin"], "name": "tool", "test": True},
            {"kind": ["bin"], "name": "example-bin", "test": False},
            {"kind": ["test"], "name": "shared", "test": True},
            {"kind": ["test"], "name": "only_a", "test": True},
            {"kind": ["example"], "name": "demo", "test": False},
            {"kind": ["custom-build"], "name": "build-script-build", "test": False},
        ]},
        {"name": "b", "targets": [
            {"kind": ["lib"], "name": "b", "test": True},
            {"kind": ["test"], "name": "shared", "test": True},
            {"kind": ["example"], "name": "checked", "test": True},
        ]},
        {"name": "c", "targets": [{"kind": ["test"], "name": "only_c", "test": True}]},
    ]}

    def test_targets_mirror_what_cargo_test_builds(self):
        filters, expected, examples = runner_mod.test_targets(self.DOCUMENT, {"a", "b"})
        self.assertEqual(filters, [["--lib"], ["--bin", "tool"], ["--example", "checked"],
                                   ["--test", "only_a"], ["--test", "shared"]])
        self.assertEqual(expected, {"a::lib::a", "a::bin::tool", "a::test::shared",
                                    "a::test::only_a", "b::lib::b", "b::test::shared",
                                    "b::example::checked"})
        self.assertEqual(examples, ["demo"])

    def test_chunks(self):
        self.assertEqual(runner_mod.chunks([1, 2, 3, 4, 5], 2), [[1, 2], [3, 4], [5]])
        self.assertEqual(runner_mod.chunks([], 3), [])

    def test_remove_outputs_deletes_every_name_of_the_file(self):
        with tempfile.TemporaryDirectory() as directory:
            examples = Path(directory)
            hashed = examples / "demo-0123"
            hashed.write_bytes(b"x" * 10)
            os.link(hashed, examples / "demo")
            (examples / "other").write_bytes(b"y")
            freed = runner_mod.remove_outputs([str(examples / "demo")])
            self.assertEqual(freed, 10)
            self.assertFalse((examples / "demo").exists())
            self.assertFalse(hashed.exists(), "the other hard-link name is deleted too")
            self.assertTrue((examples / "other").exists(), "an unrelated file stays")
            self.assertEqual(runner_mod.remove_outputs([str(examples / "gone")]), 0)

    def test_stale_exclusive_is_checked_over_the_whole_run(self):
        entry = next(iter(runner_mod.EXCLUSIVE))
        package = entry.split("::", 1)[0]
        siblings = {uid for uid in runner_mod.EXCLUSIVE if uid.startswith(package + "::")}
        self.assertEqual(runner_mod.stale_exclusive([package], siblings), [])
        self.assertIn(f"EXCLUSIVE entry matches no built binary (stale?): {entry}",
                      runner_mod.stale_exclusive([package], siblings - {entry}))
        self.assertEqual(runner_mod.stale_exclusive(["unrelated"], set()), [])


if __name__ == "__main__":
    unittest.main()

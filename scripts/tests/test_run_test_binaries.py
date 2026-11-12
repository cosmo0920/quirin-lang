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

"""Regression checks for the fail-closed parallel test-binary runner."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import textwrap
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import run_test_binaries as runner_mod

OK_OUTPUT = textwrap.dedent("""\

    running 3 tests
    test b::second ... ok
    test a::first ... ok
    test c::third ... ignored, needs a network
    test result: ok. 2 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.01s

""")

FAILED_OUTPUT = textwrap.dedent("""\

    running 2 tests
    test a::first ... ok
    test a::broken ... FAILED

    failures:

    ---- a::broken stdout ----
    thread 'a::broken' panicked at src/lib.rs:1:1

    failures:
        a::broken

    test result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

""")


def fake(script: str) -> list[str]:
    """argv for a fake test binary written in Python."""
    return [sys.executable, "-c", textwrap.dedent(script)]


def printing(output: str, exit_code: int = 0) -> list[str]:
    return fake(f"import sys\nsys.stdout.write({output!r})\nsys.exit({exit_code})\n")


class Quiet:
    def __init__(self):
        self.lines = []

    def __call__(self, line):
        self.lines.append(line)


class ParseTests(unittest.TestCase):
    def test_summary_and_test_lines_are_parsed(self):
        summaries, tests = runner_mod.parse_output(OK_OUTPUT)
        self.assertEqual(summaries, [{"status": "ok", "passed": 2, "failed": 0,
                                      "ignored": 1, "measured": 0, "filtered": 0}])
        self.assertEqual(tests, sorted([
            "test b::second ... ok",
            "test a::first ... ok",
            "test c::third ... ignored, needs a network",
        ]))

    def test_totals_aggregate_every_summary_of_every_unit(self):
        doc_output = OK_OUTPUT + OK_OUTPUT.replace("2 passed", "5 passed")
        results = []
        for text in (OK_OUTPUT, FAILED_OUTPUT, doc_output):
            summaries, tests = runner_mod.parse_output(text)
            results.append(runner_mod.Result("u", False, 0, None, None, 0.0, 1.0,
                                             summaries, tests, "", ""))
        self.assertEqual(runner_mod.totals(results), {
            "passed": 2 + 1 + 7, "failed": 1, "ignored": 1 + 0 + 2,
            "measured": 0, "filtered": 0, "summaries": 4,
        })

    def test_verdict_is_fail_closed(self):
        clean = [{"status": "ok", "passed": 1, "failed": 0, "ignored": 0,
                  "measured": 0, "filtered": 0}]
        self.assertTrue(runner_mod.unit_ok(0, None, clean))
        self.assertFalse(runner_mod.unit_ok(0, None, []), "no summary is not success")
        self.assertFalse(runner_mod.unit_ok(101, None, clean), "nonzero exit wins")
        self.assertFalse(runner_mod.unit_ok(-11, None, clean), "a signal wins")
        self.assertFalse(runner_mod.unit_ok(None, "OSError", clean), "a spawn error wins")
        self.assertFalse(runner_mod.unit_ok(0, None, clean + [dict(clean[0], status="FAILED")]))
        self.assertFalse(runner_mod.unit_ok(0, None, [dict(clean[0], failed=1)]))


class RunnerTests(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.log_dir = Path(self._tmp.name)

    def tearDown(self):
        self._tmp.cleanup()

    def run_units(self, units, jobs=4):
        runner = runner_mod.Runner(self.log_dir, jobs, echo=Quiet())
        return runner, {r.uid: r for r in runner.run(units)}

    def unit(self, uid, argv, **kwargs):
        return runner_mod.Unit(uid=uid, argv=argv, cwd=os.getcwd(),
                               env=dict(os.environ), **kwargs)

    def test_passing_failing_crashing_and_missing_binaries(self):
        units = [
            self.unit("pass", printing(OK_OUTPUT)),
            self.unit("fail", printing(FAILED_OUTPUT, 101)),
            self.unit("crash", fake(f"""
                import os, signal, sys
                sys.stdout.write({OK_OUTPUT!r}); sys.stdout.flush()
                os.kill(os.getpid(), signal.SIGSEGV)
            """)),
            self.unit("abort", fake("import os; os.abort()")),
            self.unit("missing", [str(self.log_dir / "no-such-test-binary")]),
            self.unit("silent", fake("pass")),
            self.unit("lying", printing(OK_OUTPUT, 1)),
        ]
        _, results = self.run_units(units)
        self.assertTrue(results["pass"].ok)
        self.assertEqual(runner_mod.totals([results["pass"]])["passed"], 2)
        self.assertFalse(results["fail"].ok)
        self.assertEqual(results["fail"].returncode, 101)
        self.assertIn("test a::broken ... FAILED", results["fail"].tests)
        self.assertFalse(results["crash"].ok, "a clean summary before a crash is not success")
        if os.name == "nt":
            self.assertIsNone(results["crash"].signal)
            self.assertNotEqual(results["crash"].returncode, 0)
        else:
            self.assertEqual(results["crash"].signal, "SIGSEGV")
        self.assertFalse(results["abort"].ok)
        if os.name == "nt":
            self.assertIsNone(results["abort"].signal)
            self.assertNotEqual(results["abort"].returncode, 0)
        else:
            self.assertEqual(results["abort"].signal, "SIGABRT")
        self.assertFalse(results["missing"].ok)
        self.assertIsNone(results["missing"].returncode)
        self.assertIn("FileNotFoundError", results["missing"].error)
        self.assertFalse(results["silent"].ok, "exit 0 without a summary is not success")
        self.assertFalse(results["lying"].ok, "an ok summary with a nonzero exit is not success")
        for result in results.values():
            self.assertTrue(Path(result.stdout_log).exists())
            self.assertTrue(Path(result.stderr_log).exists())

    def test_a_failing_run_writes_a_failing_summary(self):
        _, results = self.run_units([self.unit("pass", printing(OK_OUTPUT)),
                                     self.unit("fail", printing(FAILED_OUTPUT, 101))])
        document = runner_mod.write_summary(self.log_dir, list(results.values()), [], {}, 4, None)
        self.assertFalse(document["ok"])
        self.assertEqual(document["units_failed"], ["fail"])
        self.assertEqual(document["totals"]["failed"], 1)
        self.assertEqual(json.loads((self.log_dir / "summary.json").read_text())["ok"], False)

    def test_problems_alone_fail_the_run(self):
        _, results = self.run_units([self.unit("pass", printing(OK_OUTPUT))])
        document = runner_mod.write_summary(self.log_dir, list(results.values()),
                                            ["built test executable was never launched"],
                                            {}, 4, None)
        self.assertFalse(document["ok"])

    def test_exclusive_units_never_share_the_machine(self):
        sleeper = fake(f"""
            import sys, time
            time.sleep(0.4)
            sys.stdout.write({OK_OUTPUT!r})
        """)
        units = [self.unit(f"parallel-{i}", sleeper, weight=float(i)) for i in range(6)]
        units += [self.unit(f"exclusive-{i}", sleeper, exclusive=True) for i in range(3)]
        runner, results = self.run_units(units, jobs=3)
        self.assertTrue(all(result.ok for result in results.values()))
        intervals = runner.intervals
        exclusive = [i for i in intervals if i[1]]
        self.assertEqual(len(exclusive), 3)
        for uid, _, start, end in exclusive:
            for other_uid, _, other_start, other_end in intervals:
                if other_uid == uid:
                    continue
                self.assertTrue(end <= other_start or other_end <= start,
                                f"{uid} overlapped {other_uid}")
        parallel = [i for i in intervals if not i[1]]
        overlapping = any(
            a[2] < b[3] and b[2] < a[3] for a in parallel for b in parallel if a is not b
        )
        self.assertTrue(overlapping, "the parallel phase really ran units concurrently")
        concurrent_peak = max(
            sum(1 for b in parallel if b[2] <= a[2] < b[3]) for a in parallel
        )
        self.assertLessEqual(concurrent_peak, 3, "the job limit holds")

    def test_longest_recorded_units_start_first(self):
        stamp = fake("""
            import sys, time
            print(time.monotonic_ns(), file=sys.stderr)
            sys.stdout.write("test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out\\n")
        """)
        units = [self.unit(name, stamp, weight=weight)
                 for name, weight in (("short", 1.0), ("long", 50.0), ("middle", 5.0))]
        _, results = self.run_units(units, jobs=1)
        started = sorted(results.values(), key=lambda r: r.started)
        self.assertEqual([r.uid for r in started], ["long", "middle", "short"])

    def test_captured_cwd_and_environment_are_replayed_exactly(self):
        probe = fake("""
            import json, os, sys
            sys.stderr.write(json.dumps({"cwd": os.getcwd(), "env": dict(os.environ)}))
            sys.stdout.write("test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out\\n")
        """)
        package_root = self.log_dir / "package-root"
        package_root.mkdir()
        env = {"PATH": os.environ.get("PATH", ""), "CARGO_MANIFEST_DIR": str(package_root),
               "CARGO_TARGET_TMPDIR": "/somewhere/tmp", "LD_LIBRARY_PATH": "/deps"}
        record = {"sequence": 1, "argv": probe, "cwd": str(package_root), "env": env}
        units, problems = runner_mod.plan_units(
            {probe[0]: "pkg::test::probe"}, [record], ["pkg"], {}, None)
        self.assertEqual(problems, [])
        _, results = self.run_units(units)
        seen = json.loads(Path(results["pkg::test::probe"].stderr_log).read_text())
        self.assertEqual(Path(seen["cwd"]).resolve(), package_root.resolve())
        # Python may add nothing of its own; the child sees exactly the record.
        extra = set(seen["env"]) - set(env)
        self.assertLessEqual(extra, {"LC_CTYPE"}, extra)
        for key, value in env.items():
            self.assertEqual(seen["env"][key], value)
        self.assertNotIn("HOME", seen["env"], "the runner's own environment does not leak in")


class PlanTests(unittest.TestCase):
    def record(self, executable, sequence=0):
        return {"sequence": sequence, "argv": [executable], "cwd": "/pkg", "env": {"A": "1"}}

    def test_built_but_never_launched_fails(self):
        _, problems = runner_mod.plan_units(
            {"/t/a": "p::test::a", "/t/b": "p::test::b"}, [self.record("/t/a")], ["p"], {}, None)
        self.assertEqual(problems, ["built test executable was never launched by cargo: p::test::b"])

    def test_launched_but_never_built_fails(self):
        _, problems = runner_mod.plan_units(
            {"/t/a": "p::test::a"}, [self.record("/t/a"), self.record("/t/x")], ["p"], {}, None)
        self.assertTrue(any("did not name" in p for p in problems), problems)

    def test_stale_exclusive_entry_fails_only_inside_its_workspace(self):
        entry = next(iter(runner_mod.EXCLUSIVE))
        package = entry.split("::", 1)[0]
        _, problems = runner_mod.plan_units({"/t/a": "p::test::a"}, [self.record("/t/a")],
                                            ["p", package], {}, None)
        self.assertTrue(any(entry in p for p in problems), problems)
        _, problems = runner_mod.plan_units({"/t/a": "p::test::a"}, [self.record("/t/a")],
                                            ["p"], {}, None)
        self.assertEqual(problems, [])

    def test_exclusive_entries_are_marked_and_weights_applied(self):
        executables = {f"/t/x{i}": uid for i, uid in enumerate(runner_mod.EXCLUSIVE)}
        executables["/t/a"] = "p::test::a"
        records = [self.record(exe, i) for i, exe in enumerate(executables)]
        packages = {uid.split("::", 1)[0] for uid in runner_mod.EXCLUSIVE} | {"p"}
        units, problems = runner_mod.plan_units(
            executables, records, packages, {"p::test::a": 42.0}, None)
        self.assertEqual(problems, [])
        by_id = {unit.uid: unit for unit in units}
        for uid in runner_mod.EXCLUSIVE:
            self.assertTrue(by_id[uid].exclusive)
        self.assertFalse(by_id["p::test::a"].exclusive)
        self.assertEqual(by_id["p::test::a"].weight, 42.0)
        self.assertEqual(by_id["p::test::a"].env, {"A": "1"})
        self.assertEqual(by_id["p::test::a"].cwd, "/pkg")


class WorkspaceTests(unittest.TestCase):
    def test_a_workspace_other_than_the_root_is_refused_before_building(self):
        with tempfile.TemporaryDirectory() as directory:
            log_dir = Path(directory) / "logs"
            with open(os.devnull, "w") as sink:
                stderr = sys.stderr
                sys.stderr = sink
                try:
                    code = runner_mod.main(["--log-dir", str(log_dir), "--",
                                            "--manifest-path",
                                            str(runner_mod.REPO_ROOT / "providers/Cargo.toml")])
                finally:
                    sys.stderr = stderr
            self.assertEqual(code, 2)
            self.assertFalse(log_dir.exists(), "nothing was built or run")


class PackageSelectionTests(unittest.TestCase):
    """`--packages` (card GATE-ELASTIC): the scoped card gate's restricted suite."""

    def test_selection_arguments(self):
        self.assertEqual(runner_mod.selection_args(None), ["--workspace"])
        self.assertEqual(runner_mod.selection_args(["a", "b"]), ["-p", "a", "-p", "b"])
        self.assertIsNone(runner_mod.parse_packages(None))
        self.assertEqual(runner_mod.parse_packages(["b,a", "a", " c "]), ["a", "b", "c"])
        with self.assertRaises(ValueError):
            runner_mod.parse_packages([" , "])

    def test_only_members_with_a_library_get_doc_tests(self):
        document = {"packages": [
            {"name": "lib", "targets": [{"kind": ["lib"]}, {"kind": ["test"]}]},
            {"name": "bin", "targets": [{"kind": ["bin"]}, {"kind": ["test"]}]},
            {"name": "macro", "targets": [{"kind": ["proc-macro"]}]},
        ]}
        self.assertEqual(runner_mod.library_packages(document), {"lib", "macro"})

    def test_a_restricted_run_keeps_the_exclusive_phase_of_selected_members(self):
        entry = "qcorec::test::plugin_packages"
        self.assertIn(entry, runner_mod.EXCLUSIVE)
        record = {"sequence": 0, "argv": ["/t/p"], "cwd": "/pkg", "env": {}}
        units, problems = runner_mod.plan_units({"/t/p": entry}, [record], {"qcorec"}, {}, None)
        self.assertEqual(problems, [])
        self.assertTrue(units[0].exclusive)
        # A selected member whose EXCLUSIVE binary was not built still fails closed.
        _, problems = runner_mod.plan_units({}, [], {"qcorec"}, {}, None)
        self.assertTrue(any(entry in problem for problem in problems), problems)

    def test_the_summary_names_the_selection(self):
        with tempfile.TemporaryDirectory() as directory:
            document = runner_mod.write_summary(Path(directory), [], [], {}, 1, None, ["a", "b"])
            self.assertEqual(document["packages"], ["a", "b"])
            document = runner_mod.write_summary(Path(directory), [], [], {}, 1, None)
            self.assertIsNone(document["packages"], "None is the whole workspace")

    def test_a_non_member_is_refused_before_building(self):
        with tempfile.TemporaryDirectory() as directory:
            log_dir = Path(directory) / "logs"
            with open(os.devnull, "w") as sink:
                stderr = sys.stderr
                sys.stderr = sink
                try:
                    code = runner_mod.main(["--log-dir", str(log_dir),
                                            "--packages", "qcorec,no-such-member"])
                finally:
                    sys.stderr = stderr
            self.assertEqual(code, 2)
            self.assertFalse(log_dir.exists(), "nothing was built or run")


class CaptureTests(unittest.TestCase):
    def test_capture_runner_records_argv_cwd_and_environment_without_running(self):
        with tempfile.TemporaryDirectory() as directory:
            capture_dir = Path(directory) / "capture"
            capture_dir.mkdir()
            package_root = Path(directory) / "package"
            package_root.mkdir()
            marker = Path(directory) / "ran"
            env = {"CARGO_MANIFEST_DIR": str(package_root), "ONLY": "this",
                   "LD_LIBRARY_PATH": "/deps"}
            # cargo appends the executable and its arguments to the runner argv.
            argv = runner_mod.capture_command(capture_dir) + [
                "/usr/bin/touch", str(marker)]
            completed = subprocess.run(argv, cwd=package_root, env=env)
            self.assertEqual(completed.returncode, 0)
            self.assertFalse(marker.exists(), "the captured executable is not run")
            [path] = list(capture_dir.glob("*.json"))
            record = json.loads(path.read_text())
            self.assertEqual(record["argv"], ["/usr/bin/touch", str(marker)])
            self.assertEqual(Path(record["cwd"]), package_root.resolve())
            extra = set(record["env"]) - set(env)
            self.assertLessEqual(extra, {"LC_CTYPE"}, extra)
            for key, value in env.items():
                self.assertEqual(record["env"][key], value)

    def test_capture_runner_fails_when_it_cannot_record(self):
        argv = runner_mod.capture_command(Path("/nonexistent-capture-dir")) + ["/bin/true"]
        completed = subprocess.run(argv, capture_output=True)
        self.assertNotEqual(completed.returncode, 0, "an unrecorded launch is not a pass")


class FailureAnnotations(unittest.TestCase):
    """CI-FAILURE-ANNOTATIONS: a check run's annotations are public where its
    log is not, so the panics must survive in at most 10 `::error` commands."""

    def result(self, root: Path, name: str, stdout: str) -> "runner_mod.Result":
        log = root / f"{name}.stdout.log"
        log.write_text(stdout, encoding="utf-8")
        return runner_mod.Result(name, False, 101, None, None, 0.0, 1.0, [], [],
                                 str(log), str(root / "missing.stderr.log"))

    def test_digest_keeps_each_panic_and_its_message(self):
        with tempfile.TemporaryDirectory() as temporary:
            result = self.result(Path(temporary), "u", textwrap.dedent("""\
                running 2 tests
                ---- a stdout ----

                thread 'a' (1) panicked at crates\\x\\tests\\a.rs:4:5:
                assertion `left == right` failed
                  left: 1

                ---- b stdout ----
                thread 'b' (2) panicked at crates\\x\\tests\\b.rs:9:1:
                boom
                """))
            digest = runner_mod.panic_digest(result)
        self.assertIn("a.rs:4:5:\nassertion `left == right` failed\n  left: 1", digest)
        self.assertIn("b.rs:9:1:\nboom", digest)
        self.assertNotIn("running 2 tests", digest)

    def test_annotations_escape_and_stay_within_ten(self):
        digests = [(f"unit{n}", "50% failed\r\nline " + "x" * 9000) for n in range(40)]
        commands = runner_mod.failure_annotations(digests)
        self.assertEqual(len(commands), 10)
        self.assertTrue(all(c.startswith("::error title=failed test units (") for c in commands))
        self.assertTrue(all("\n" not in c and "\r" not in c for c in commands))
        self.assertIn("50%25 failed%0D%0Aline", commands[0])
        self.assertIn("units not shown in full: ", commands[-1])
        self.assertIn("unit39", commands[-1])
        shown = "".join(commands[:-1])
        for n in range(27):
            self.assertIn(f"== unit{n}%0A", shown)

    def test_few_failures_fit_one_annotation(self):
        commands = runner_mod.failure_annotations([("a", "p1"), ("b", "p2")])
        self.assertEqual(commands, ["::error title=failed test units (1/1)::== a%0Ap1%0A== b%0Ap2"])


if __name__ == "__main__":
    unittest.main()

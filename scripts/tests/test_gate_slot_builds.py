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

"""The gate's build class (card GATE-SLOT-BUILDS-UNSLOTTED).

fmt, Clippy and cargo builds take the `build` class: unlimited, no memory
admission, no queue, no legacy lock, and kept out only while a rust-suite
job's EXCLUSIVE phase runs (run_test_binaries.py under a suite slot, or
`rust-suite --close-builds`).  Every test runs in a scratch gate directory
with a scratch legacy lock and a fake meminfo; the live /tmp/quirin-gate is
never touched, and every process a test starts is stopped by the pid it
captured."""
import fcntl
import json
import os
from pathlib import Path
import sys
import time
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import gate_scope as scope_mod  # noqa: E402
import gate_slot as slot_mod  # noqa: E402
import run_test_binaries as runner_mod  # noqa: E402

sys.path.insert(0, str(Path(__file__).resolve().parent))
from test_gate_slot_starvation import LINUX_PROC, Scratch, wait_for  # noqa: E402

OK_OUTPUT = "running 1 test\ntest t ... ok\n\ntest result: ok. 1 passed; 0 failed; " \
            "0 ignored; 0 measured; 0 filtered out; finished in 0.00s\n"


def builds_open(gate_dir: Path) -> bool:
    """What a build job would see now: build admission can be taken shared."""
    path = gate_dir / "build" / "admission"
    if not path.exists():
        return True
    fd = os.open(path, os.O_RDWR)
    try:
        fcntl.flock(fd, fcntl.LOCK_SH | fcntl.LOCK_NB)
        fcntl.flock(fd, fcntl.LOCK_UN)
        return True
    except BlockingIOError:
        return False
    finally:
        os.close(fd)


# A command that records whether the builds were open while it ran.
PROBE = ("import fcntl, os, sys\n"
         "path = os.path.join(sys.argv[1], 'build', 'admission')\n"
         "state = 'open'\n"
         "if os.path.exists(path):\n"
         "    fd = os.open(path, os.O_RDWR)\n"
         "    try:\n"
         "        fcntl.flock(fd, fcntl.LOCK_SH | fcntl.LOCK_NB)\n"
         "    except BlockingIOError:\n"
         "        state = 'closed'\n"
         "open(sys.argv[2], 'w').write(state)\n")


@unittest.skipUnless(LINUX_PROC, "needs /proc/locks")
class BuildClass(Scratch):
    def closure(self, timeout=5.0):
        closure = slot_mod.BuildClosure(self.gate_dir, timeout, poll=0.02, legacy=self.legacy,
                                        echo=lambda line: None)
        self.addCleanup(closure.reopen)
        return closure

    def test_a_build_runs_at_once_with_no_slot_count_queue_or_legacy_lock(self):
        receipts = [self.root / f"b{i}.json" for i in range(3)]
        builds = [self.start_slot("build", "--poll", "0.02", "--receipt", str(receipt), "--",
                                  sys.executable, "-c", "import time; time.sleep(1.5)")
                  for receipt in receipts]
        # Three builds hold build/running together (no slot count), none queued.
        self.assertTrue(wait_for(lambda: len({entry["pid"] for entry in
                                              slot_mod.gate_locks(self.gate_dir, self.legacy)
                                              if entry["label"] == "build/running"}) == 3))
        for build in builds:
            self.assertEqual(build.wait(timeout=30), 0)
        document = json.loads(receipts[0].read_text())
        record = document["slot_record"]
        self.assertEqual(record["class"], "build")
        self.assertEqual(record["held"], ["build/running"])
        self.assertNotIn("legacy_lock", record, "a build takes no legacy lock")
        self.assertNotIn("queue", record)
        self.assertFalse(record["waited_for_exclusive_phase"])
        self.assertFalse((self.gate_dir / "build" / "queue").exists())

    def test_a_legacy_job_does_not_hold_builds_back(self):
        self.legacy.touch()
        fd = os.open(self.legacy, os.O_RDWR)
        try:
            fcntl.flock(fd, fcntl.LOCK_EX)  # what `flock <legacy lock> <cmd>` does
            completed = self.run_slot("build", "--receipt", str(self.root / "b.json"), "--",
                                      "true", timeout=30)
        finally:
            os.close(fd)
        self.assertEqual(completed.returncode, 0, completed.stderr)

    def test_an_exclusive_phase_keeps_new_builds_out_until_it_reopens(self):
        closure = self.closure()
        record = closure.close()
        self.assertTrue(record["drained"])
        self.assertEqual(record["builds_in_flight_at_close"], [])
        marker = self.root / "built"
        receipt = self.root / "b.json"
        build = self.start_slot("build", "--poll", "0.02", "--receipt", str(receipt), "--",
                                sys.executable, "-c", f"open({str(marker)!r}, 'w')")
        status = wait_for(lambda: (lambda s: s if s["builds"]["waiting"] == [build.pid] else None)(
            slot_mod.gate_status(self.gate_dir, self.legacy, self.meminfo, 0)))
        self.assertTrue(status, "the waiting build is listed")
        roles = {job["pid"]: job["role"] for job in status["jobs"]}
        self.assertEqual(roles[build.pid],
                         "waiting for the builds to reopen (an EXCLUSIVE phase runs)")
        self.assertEqual(roles[os.getpid()], "EXCLUSIVE phase (builds closed)")
        self.assertEqual(status["builds"]["closed_by"], [os.getpid()])
        self.assertIn("builds: closed by pid", slot_mod.render_status(status))
        time.sleep(0.3)
        self.assertFalse(marker.exists(), "a build ran during the EXCLUSIVE phase")
        closure.reopen()
        self.assertEqual(build.wait(timeout=30), 0)
        self.assertTrue(marker.exists())
        self.assertTrue(json.loads(receipt.read_text())["slot_record"]
                        ["waited_for_exclusive_phase"])

    def test_an_exclusive_phase_waits_for_the_builds_in_flight(self):
        done = self.root / "done"
        build = self.start_slot("build", "--poll", "0.02", "--receipt",
                                str(self.root / "b.json"), "--", sys.executable, "-c",
                                f"import time; time.sleep(1.5); open({str(done)!r}, 'w')")
        self.assertTrue(wait_for(lambda: not self.closure_free()))
        closure = self.closure()
        record = closure.close()
        self.assertTrue(record["drained"])
        self.assertEqual([entry["pid"] for entry in record["builds_in_flight_at_close"]],
                         [build.pid])
        self.assertTrue(done.exists(), "the phase began while a build still ran")
        self.assertGreater(record["waited_seconds"], 0)
        self.assertEqual(build.wait(timeout=30), 0)

    def closure_free(self) -> bool:
        path = self.gate_dir / "build" / "running"
        if not path.exists():
            return True
        fd = os.open(path, os.O_RDWR)
        try:
            fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
            fcntl.flock(fd, fcntl.LOCK_UN)
            return True
        except BlockingIOError:
            return False
        finally:
            os.close(fd)

    def test_past_its_bound_the_phase_runs_beside_the_build_and_says_so(self):
        build = self.start_slot("build", "--poll", "0.02", "--receipt",
                                str(self.root / "b.json"), "--", sys.executable, "-c",
                                "import time; time.sleep(600)")
        self.assertTrue(wait_for(lambda: not self.closure_free()))
        closure = self.closure(timeout=0.3)
        record = closure.close()
        self.assertFalse(record["drained"], "an overlap is never reported as drained")
        self.assertEqual([entry["pid"] for entry in record["overlapping_builds"]], [build.pid])
        self.assertGreaterEqual(record["waited_seconds"], 0.3)
        self.assertIsNone(build.poll(), "no job stops another")
        self.assertFalse(builds_open(self.gate_dir), "new builds stay out")
        status = slot_mod.gate_status(self.gate_dir, self.legacy, self.meminfo, 0)
        closer = next(job for job in status["jobs"] if job["pid"] == os.getpid())
        self.assertTrue(closer["role"].startswith("draining builds"), closer["role"])
        self.assertEqual(closer["blocked_by"], [{"lock": "build/running", "pids": [build.pid]}])
        closure.reopen()
        self.assertTrue(builds_open(self.gate_dir))

    def test_a_suite_outside_its_exclusive_phase_leaves_builds_open(self):
        """The rust-suite class excludes builds during its EXCLUSIVE phase only:
        holding the suite slot (the drain, the build and parallel phases) does
        not close them."""
        suite = self.gate()
        suite.acquire("rust-suite", timeout=10)
        completed = self.run_slot("build", "--receipt", str(self.root / "b.json"), "--", "true",
                                  timeout=30)
        self.assertEqual(completed.returncode, 0, completed.stderr)

    def test_close_builds_makes_the_whole_suite_command_an_exclusive_phase(self):
        seen = self.root / "seen"
        receipt = self.root / "suite.json"
        completed = self.run_slot("rust-suite", "--close-builds", "--poll", "0.02",
                                  "--receipt", str(receipt), "--", sys.executable, "-c", PROBE,
                                  str(self.gate_dir), str(seen))
        self.assertEqual(completed.returncode, 0, completed.stderr)
        self.assertEqual(seen.read_text(), "closed")
        document = json.loads(receipt.read_text())
        self.assertTrue(document["build_exclusion"]["drained"])
        self.assertIn("reopened_at", document["build_exclusion"])
        self.assertTrue(document["limits"]["close_builds"])
        self.assertTrue(builds_open(self.gate_dir), "reopened when the command ended")
        # Without the flag the suite's command sees the builds open.
        completed = self.run_slot("rust-suite", "--poll", "0.02", "--receipt",
                                  str(self.root / "plain.json"), "--", sys.executable, "-c",
                                  PROBE, str(self.gate_dir), str(seen))
        self.assertEqual(completed.returncode, 0, completed.stderr)
        self.assertEqual(seen.read_text(), "open")

    def test_refusals_and_nesting(self):
        completed = self.run_slot("audit", "--close-builds", "--receipt",
                                  str(self.root / "a.json"), "--", "true")
        self.assertEqual(completed.returncode, 2)
        self.assertIn("--close-builds applies to the rust-suite class only", completed.stderr)
        completed = self.run_slot("build", "--memory-budget", "1G", "--receipt",
                                  str(self.root / "b.json"), "--", "true")
        self.assertEqual(completed.returncode, 2)
        # Nested under a class that never runs beside an EXCLUSIVE phase: covered.
        closure = self.closure()
        closure.close()
        for parent in ("rust-suite/0,audit/admission", "audit/0", "short/0", "build/running"):
            completed = self.run_slot("build", "--receipt", str(self.root / "n.json"), "--",
                                      "true", timeout=30, QUIRIN_GATE_HELD=parent)
            self.assertEqual(completed.returncode, 0, (parent, completed.stderr))
            self.assertTrue(json.loads((self.root / "n.json").read_text())
                            ["slot_record"]["covered_by_parent"])
        # A lean job runs beside a suite, so a build nested in one is refused.
        completed = self.run_slot("build", "--receipt", str(self.root / "l.json"), "--", "true",
                                  QUIRIN_GATE_HELD="lean/0")
        self.assertEqual(completed.returncode, 2)
        self.assertIn("nested build slot under a held lean slot", completed.stderr)

    def test_a_job_nested_in_a_build_holds_the_class_in_its_environment(self):
        out = self.root / "held"
        completed = self.run_slot("build", "--receipt", str(self.root / "b.json"), "--",
                                  sys.executable, "-c",
                                  f"import os; open({str(out)!r}, 'w').write("
                                  "os.environ['QUIRIN_GATE_HELD'])")
        self.assertEqual(completed.returncode, 0, completed.stderr)
        self.assertEqual(out.read_text(), "build/running")


@unittest.skipUnless(LINUX_PROC, "needs /proc/locks")
class RunnerClosure(Scratch):
    """run_test_binaries.py closes the build class around its EXCLUSIVE phase
    only, when it runs under a rust-suite slot."""

    def env_for_runner(self, **extra):
        return {"QUIRIN_GATE_DIR": str(self.gate_dir),
                "QUIRIN_GATE_LEGACY_LOCK": str(self.legacy), **extra}

    def test_the_closure_is_taken_only_under_a_suite_slot(self):
        self.assertIsNone(slot_mod.exclusive_phase_closure(self.env_for_runner()))
        self.assertIsNone(slot_mod.exclusive_phase_closure(
            self.env_for_runner(QUIRIN_GATE_HELD="audit/0")))
        closure = slot_mod.exclusive_phase_closure(self.env_for_runner(
            QUIRIN_GATE_HELD="rust-suite/0,audit/admission,audit/0",
            QUIRIN_GATE_BUILD_DRAIN="90s"))
        self.assertEqual((closure.gate_dir, closure.timeout), (self.gate_dir, 90.0))
        with self.assertRaises(slot_mod.Refusal):
            slot_mod.exclusive_phase_closure(self.env_for_runner(
                QUIRIN_GATE_HELD="rust-suite/0", QUIRIN_GATE_BUILD_DRAIN="ninety"))

    def test_only_the_exclusive_phase_runs_with_builds_closed(self):
        logs = self.root / "logs"
        logs.mkdir()
        env = dict(os.environ)

        def unit(uid, exclusive):
            seen = self.root / f"{uid}.seen"
            script = PROBE + f"print({OK_OUTPUT!r})\n"
            return runner_mod.Unit(uid=uid, argv=[sys.executable, "-c", script,
                                                  str(self.gate_dir), str(seen)],
                                   cwd=os.getcwd(), env=env, exclusive=exclusive)

        factory = lambda: slot_mod.BuildClosure(self.gate_dir, 5.0, poll=0.02,  # noqa: E731
                                                legacy=self.legacy, echo=lambda line: None)
        runner = runner_mod.Runner(logs, 2, echo=lambda line: None, build_closure=factory)
        results = runner.run([unit("p1", False), unit("p2", False), unit("x1", True),
                              unit("x2", True)])
        self.assertTrue(all(result.ok for result in results))
        seen = {uid: (self.root / f"{uid}.seen").read_text() for uid in ("p1", "p2", "x1", "x2")}
        self.assertEqual(seen, {"p1": "open", "p2": "open", "x1": "closed", "x2": "closed"})
        self.assertTrue(builds_open(self.gate_dir), "reopened after the phase")
        [phase] = runner.build_exclusion
        self.assertTrue(phase["drained"])
        self.assertEqual(phase["units"], ["x1", "x2"])
        document = runner_mod.write_summary(logs, results, [], {}, 2, runner)
        self.assertEqual(document["build_exclusion"], [phase])
        # No exclusive unit: nothing is closed; no factory: null in the summary.
        runner = runner_mod.Runner(logs, 2, echo=lambda line: None, build_closure=factory)
        runner.run([unit("p3", False)])
        self.assertEqual(runner.build_exclusion, [])
        plain = runner_mod.Runner(logs, 2, echo=lambda line: None)
        self.assertIsNone(runner_mod.write_summary(logs, [], [], {}, 2, plain)["build_exclusion"])


class ScopedGateSteps(unittest.TestCase):
    """gate_scope.py sends fmt, Clippy and its cargo builds through the build
    class and the provider suite through rust-suite --close-builds."""

    def steps(self, scope):
        return {step.name: step for step in scope_mod.steps_for(scope, Path("/o"))}

    def scope(self, **overrides):
        scope = {"mode": "full", "rust_packages": [], "rust_targets": [], "probes": True,
                 "lean": {"mode": "none"}, "provider_gate": True}
        scope.update(overrides)
        return scope

    def test_build_steps_take_the_build_class(self):
        steps = self.steps(self.scope())
        for name in ("fmt", "clippy", "probe-reevaluation_probe", "probe-reevaluation_followup",
                     "provider-package-build", "provider-fmt", "provider-clippy"):
            self.assertEqual(steps[name].slot, "build", name)
        self.assertEqual(steps["rust-suite"].slot, "rust-suite")
        self.assertEqual(steps["rust-suite"].slot_args, ())
        self.assertEqual(steps["provider-test"].slot, "rust-suite")
        self.assertEqual(steps["provider-test"].slot_args, ("--close-builds",))
        for name in ("submodules", "python", "provider-package-verify",
                     "provider-package-verify-all"):
            self.assertIsNone(steps[name].slot, name)
        scoped = self.steps(self.scope(mode="scoped", rust_packages=["a"],
                                       rust_targets=["b::test::t", "c::lib::c"],
                                       probes=False, provider_gate=False))
        for name in ("clippy", "clippy-b", "clippy-c-lib"):
            self.assertEqual(scoped[name].slot, "build", name)

    def test_run_steps_passes_the_slot_options_before_the_command(self):
        out = Path(self._tmp())
        step = scope_mod.Step("provider-test", ["true"], slot="rust-suite",
                              slot_args=("--close-builds",))
        calls = []

        class Done:
            returncode = 0

        def fake_run(argv, **kwargs):
            calls.append(argv)
            return Done()

        original = scope_mod.subprocess.run
        scope_mod.subprocess.run = fake_run
        try:
            [record] = scope_mod.run_steps([step], out, use_slots=True, echo=lambda line: None)
        finally:
            scope_mod.subprocess.run = original
        argv = calls[0]
        self.assertEqual(argv[2:4], ["rust-suite", "--close-builds"])
        self.assertEqual(argv[argv.index("--"):], ["--", "true"])
        self.assertEqual(record["argv"], argv)

    def _tmp(self):
        import tempfile
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        return directory.name


if __name__ == "__main__":
    unittest.main()

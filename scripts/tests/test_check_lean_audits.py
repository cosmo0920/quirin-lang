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

"""Regression checks for bounded Lean verification orchestration."""
import argparse
import contextlib
import io
import os
from pathlib import Path
import stat
import subprocess
import sys
import tempfile
import threading
import time
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import check_lean_audits as checks


class RunnerTests(unittest.TestCase):
    def test_invalid_limits_rejected(self):
        for value in ("0", "-1", "nan", "inf"):
            with self.subTest(value=value), self.assertRaises(argparse.ArgumentTypeError):
                checks.positive_seconds(value)

    def test_invalid_worker_counts_rejected(self):
        for value in ("0", "-1"):
            with self.subTest(value=value), self.assertRaises(argparse.ArgumentTypeError):
                checks.positive_jobs(value)
        self.assertEqual(checks.positive_jobs("4"), 4)

    def test_captured_output_is_returned_not_streamed(self):
        with tempfile.TemporaryFile(mode="w+") as captured:
            terminal = io.StringIO()
            with contextlib.redirect_stdout(terminal):
                checks.run_step(
                    [sys.executable, "-c", "print('audit said this')"],
                    cwd=Path.cwd(), label="captured fixture", timeout=5, heartbeat=1,
                    output=captured, quiet=True,
                )
            captured.seek(0)
            self.assertEqual(captured.read().strip(), "audit said this")
        self.assertEqual(terminal.getvalue(), "")

    def test_silent_step_reports_progress_and_finishes(self):
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            checks.run_step(
                [sys.executable, "-c", "import time; time.sleep(.15)"],
                cwd=Path.cwd(), label="silent fixture", timeout=5, heartbeat=.05,
            )
        self.assertIn("Still running silent fixture", output.getvalue())
        self.assertIn("Passed silent fixture", output.getvalue())

    def test_nonzero_exit_is_not_success(self):
        with self.assertRaises(subprocess.CalledProcessError) as caught:
            checks.run_step(
                [sys.executable, "-c", "raise SystemExit(7)"], cwd=Path.cwd(),
                label="failure", timeout=5, heartbeat=1,
            )
        self.assertEqual(caught.exception.returncode, 7)

    def test_stopped_supervisor_refuses_before_popen(self):
        supervisor = checks.ChildSupervisor()
        supervisor.stop()
        with patch.object(checks.subprocess, "Popen") as popen:
            with self.assertRaises(checks.RunStopped):
                checks.run_step(
                    [sys.executable, "-c", "raise SystemExit(99)"], cwd=Path.cwd(),
                    label="stopped fixture", timeout=5, heartbeat=1,
                    supervisor=supervisor,
                )
        popen.assert_not_called()

    @unittest.skipUnless(os.name == "posix", "POSIX process-group cleanup")
    def test_timeout_kills_descendant_even_when_it_ignores_term(self):
        with tempfile.TemporaryDirectory() as directory:
            marker = Path(directory) / "escaped"
            worker = (
                "import signal,time; from pathlib import Path; "
                "signal.signal(signal.SIGTERM, signal.SIG_IGN); "
                f"time.sleep(1.5); Path({str(marker)!r}).touch(); time.sleep(10)"
            )
            parent = (
                "import subprocess,sys,time; "
                f"subprocess.Popen([sys.executable, '-c', {worker!r}]); time.sleep(10)"
            )
            with self.assertRaises(TimeoutError):
                checks.run_step(
                    [sys.executable, "-c", parent], cwd=directory,
                    label="stalled fixture", timeout=.5, heartbeat=.2,
                )
            time.sleep(1.2)
            self.assertFalse(marker.exists(), "worker survived the timeout")


class AuditTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.proof = Path(self.directory.name)
        self.args = argparse.Namespace(skip_build=False, verbose=False,
                                       build_timeout=600, audit_timeout=120, heartbeat=15)
        for name in ("AuditAll.lean", "AuditFixture.lean"):
            (self.proof / name).touch()

    def test_build_precedes_every_audit_in_pinned_directory(self):
        with patch.object(checks, "run_step") as run:
            checks.check_audits(self.proof, self.args)
        self.assertEqual(len(run.call_args_list), 3)
        self.assertEqual(run.call_args_list[0].args[0], ["lake", "build"])
        for call, name in zip(run.call_args_list[1:], ("AuditAll.lean", "AuditFixture.lean")):
            self.assertEqual(call.args[0], ["lake", "env", "lean", "-DwarningAsError=true", name])
        self.assertTrue(all(call.kwargs["cwd"] == self.proof for call in run.call_args_list))

    def test_build_failure_prevents_stale_audits(self):
        with patch.object(checks, "run_step", side_effect=TimeoutError("build")) as run:
            with self.assertRaises(TimeoutError):
                checks.check_audits(self.proof, self.args)
        self.assertEqual(run.call_count, 1)

    def test_missing_whole_project_audit_refuses(self):
        (self.proof / "AuditAll.lean").unlink()
        with patch.object(checks, "run_step") as run:
            with self.assertRaises(ValueError):
                checks.check_audits(self.proof, self.args)
        run.assert_not_called()

    def test_skip_build_is_explicit(self):
        self.args.skip_build = True
        with patch.object(checks, "run_step") as run:
            checks.check_audits(self.proof, self.args)
        self.assertEqual(run.call_count, 2)

    def test_absent_worker_count_keeps_the_sequential_transcript(self):
        """A caller that predates `--jobs` still gets one audit at a time."""
        self.assertFalse(hasattr(self.args, "jobs"))
        with patch.object(checks, "run_step") as run:
            checks.check_audits(self.proof, self.args)
        self.assertTrue(all(call.kwargs.get("output") is None for call in run.call_args_list))


class ParallelAuditTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.proof = Path(self.directory.name)
        self.names = ["AuditAll.lean"] + [f"Audit{index}.lean" for index in range(1, 6)]
        for name in self.names:
            (self.proof / name).touch()
        self.args = argparse.Namespace(skip_build=True, verbose=False, jobs=4,
                                       build_timeout=600, audit_timeout=120, heartbeat=15)

    def test_every_audit_runs_once_with_the_sequential_command(self):
        with patch.object(checks, "run_step", return_value=0.0) as run:
            checks.check_audits(self.proof, self.args)
        commands = sorted(call.args[0] for call in run.call_args_list)
        self.assertEqual(
            commands,
            sorted(checks.audit_command(name) for name in sorted(self.names)),
        )
        self.assertTrue(all(call.kwargs["quiet"] for call in run.call_args_list))
        self.assertTrue(all(call.kwargs["cwd"] == self.proof for call in run.call_args_list))

    def test_results_are_reported_in_audit_order_not_completion_order(self):
        order = sorted(self.names)

        def slowest_first(command, **kwargs):
            # The first audit finishes last; the transcript must not follow that.
            time.sleep(.05 if command[-1] == order[0] else 0)
            return 0.0

        output = io.StringIO()
        with patch.object(checks, "run_step", side_effect=slowest_first):
            with contextlib.redirect_stdout(output):
                checks.check_audits(self.proof, self.args)
        reported = [line.split()[1] for line in output.getvalue().splitlines()
                    if line.startswith("Passed Audit")]
        self.assertEqual(reported, order)

    def test_a_failing_audit_fails_the_run_and_shows_its_own_output(self):
        def fail_on_one(command, *, output=None, **kwargs):
            if command[-1] == "Audit3.lean":
                if output is not None:
                    output.write("Audit3 said why\n")
                raise subprocess.CalledProcessError(1, command)
            return 0.0

        printed = io.StringIO()
        with patch.object(checks, "run_step", side_effect=fail_on_one):
            with contextlib.redirect_stdout(printed):
                with self.assertRaises(subprocess.CalledProcessError):
                    checks.check_audits(self.proof, self.args)
        self.assertIn("Audit3 said why", printed.getvalue())
        self.assertNotIn("Passed 6 Lean audits", printed.getvalue())

    @unittest.skipUnless(os.name == "posix", "POSIX process-group cleanup")
    def test_later_failure_interrupts_an_earlier_slow_audit_promptly(self):
        scripts = {
            "Audit1.lean": "import time; time.sleep(5)",
            "Audit2.lean": "import time; time.sleep(.05); raise SystemExit(7)",
        }

        def command(name):
            return [sys.executable, "-c", scripts.get(name, "import time; time.sleep(5)")]

        started = time.monotonic()
        with patch.object(checks, "audit_command", side_effect=command):
            with self.assertRaises(subprocess.CalledProcessError) as caught:
                checks.check_audits(self.proof, self.args)
        elapsed = time.monotonic() - started
        self.assertEqual(caught.exception.returncode, 7)
        self.assertLess(elapsed, 1.5, "failure waited for the first submitted audit")

    @unittest.skipUnless(os.name == "posix", "POSIX process-group cleanup")
    def test_cancellation_collects_a_spawn_already_in_progress_and_run_is_reusable(self):
        marker = self.proof / "late-child-escaped"
        late_spawn_gate = self.proof / "late-spawn-entered"
        late_spawn_entered = threading.Event()
        real_popen = subprocess.Popen

        scripts = {
            "Audit1.lean": (
                "import time; from pathlib import Path; "
                f"gate=Path({str(late_spawn_gate)!r}); "
                "deadline=time.monotonic()+2; "
                "\nwhile not gate.exists() and time.monotonic() < deadline: time.sleep(.01)"
                "\nraise SystemExit(9)"
            ),
            "Audit2.lean": (
                "import time; from pathlib import Path; time.sleep(.3); "
                f"Path({str(marker)!r}).touch()"
            ),
        }

        def command(name):
            body = scripts.get(name, "import time; time.sleep(5)")
            return [sys.executable, "-c", body, name]

        def delayed_popen(command, **kwargs):
            if command[-1] == "Audit2.lean":
                late_spawn_entered.set()
                late_spawn_gate.touch()
                # Force the failure handler to meet this worker at the exact
                # spawn boundary. It must wait, register, terminate and reap.
                time.sleep(.2)
            return real_popen(command, **kwargs)

        with patch.object(checks, "audit_command", side_effect=command):
            with patch.object(checks.subprocess, "Popen", side_effect=delayed_popen):
                with self.assertRaises(subprocess.CalledProcessError) as caught:
                    checks.check_audits(self.proof, self.args)
        self.assertEqual(caught.exception.returncode, 9)
        self.assertTrue(late_spawn_entered.is_set(), "fixture did not drive late spawn")
        time.sleep(.5)
        self.assertFalse(marker.exists(), "a child started or survived after failure return")

        # Cancellation state belongs to one run; drive six real children
        # through the following run's fresh supervisor and observe each one.
        def reuse_command(name):
            reuse_marker = self.proof / f"reuse-{name}"
            return [sys.executable, "-c", f"from pathlib import Path; Path({str(reuse_marker)!r}).touch()"]

        with patch.object(checks, "audit_command", side_effect=reuse_command):
            checks.check_audits(self.proof, self.args)
        self.assertEqual(
            sorted(path.name.removeprefix("reuse-") for path in self.proof.glob("reuse-*")),
            sorted(self.names),
        )

    @unittest.skipUnless(os.name == "posix", "POSIX process-group cleanup")
    def test_submission_failure_collects_started_workers(self):
        original_submit = checks.ThreadPoolExecutor.submit
        submissions = 0

        def fail_third_submit(executor, *args, **kwargs):
            nonlocal submissions
            submissions += 1
            if submissions == 3:
                raise RuntimeError("fixture submission failure")
            return original_submit(executor, *args, **kwargs)

        def command(name):
            return [sys.executable, "-c", "import time; time.sleep(5)", name]

        started = time.monotonic()
        with patch.object(checks, "audit_command", side_effect=command):
            with patch.object(checks.ThreadPoolExecutor, "submit", new=fail_third_submit):
                with self.assertRaisesRegex(RuntimeError, "fixture submission failure"):
                    checks.check_audits(self.proof, self.args)
        self.assertLess(time.monotonic() - started, 1.5)

    def test_worker_count_never_exceeds_the_audits_present(self):
        self.args.jobs = 999
        with patch.object(checks, "run_audits_in_parallel") as parallel:
            checks.check_audits(self.proof, self.args)
        self.assertEqual(parallel.call_args.args[-1], len(self.names))


class ExitStatusTests(unittest.TestCase):
    """`main` reports failure through its return value, which `__main__` raises
    as the process exit code. A run that printed "Lean checks failed" while
    exiting 0 masked a real Lean failure (wave-24 integration, 2026-09-12), so
    every failure path is pinned non-zero here."""

    def run_main(self, **patches):
        stderr = io.StringIO()
        with patch.object(checks.shutil, "which", return_value="/usr/bin/lake"):
            with patch.object(checks, "check_pinned_toolchain", return_value="4.35.0"):
                with patch.object(checks, "check_audits", **patches):
                    with contextlib.redirect_stderr(stderr):
                        status = checks.main([])
        return status, stderr.getvalue()

    def test_success_returns_zero(self):
        status, stderr = self.run_main(return_value=None)
        self.assertEqual(status, 0)
        self.assertEqual(stderr, "")

    def test_every_caught_failure_returns_nonzero(self):
        failures = (
            OSError(2, "No such file or directory", "lake"),
            ValueError("missing whole-project Lean axiom audit"),
            TimeoutError("Lean build exceeded 600s; stopped its process tree"),
            subprocess.CalledProcessError(1, ["lake", "build"]),
        )
        for failure in failures:
            with self.subTest(failure=type(failure).__name__):
                status, stderr = self.run_main(side_effect=failure)
                self.assertEqual(status, 1)
                self.assertTrue(stderr.startswith("Lean checks failed: "))

    def test_interrupt_returns_130(self):
        status, stderr = self.run_main(side_effect=KeyboardInterrupt())
        self.assertEqual(status, 130)
        self.assertIn("Lean checks interrupted", stderr)

    def test_missing_lake_is_a_hard_error_naming_the_elan_path(self):
        stderr = io.StringIO()
        with patch.object(checks.shutil, "which", return_value=None):
            with patch.object(checks, "check_audits") as audits:
                with contextlib.redirect_stderr(stderr):
                    status = checks.main([])
        audits.assert_not_called()
        self.assertEqual(status, 1)
        message = stderr.getvalue()
        self.assertTrue(message.startswith("Lean checks failed: "))
        self.assertIn(str(Path.home() / ".elan" / "bin"), message)
        self.assertIn("proof/lean-toolchain", message)


@unittest.skipUnless(os.name == "posix", "stub lake is a POSIX shell script")
class ProcessExitCodeTests(unittest.TestCase):
    """The gate reads only the child's exit code, so pin it at the process
    boundary rather than trusting the in-process return value."""

    script = Path(checks.__file__).resolve()

    def run_script(self, path_dir, *argv):
        return subprocess.run(
            [sys.executable, str(self.script), *argv],
            env={**os.environ, "PATH": str(path_dir)},
            capture_output=True, text=True, timeout=120,
        )

    def stub_lake(self, directory, body, version=None):
        # The runner first asks `lake env lean --version` and refuses a Lean
        # other than the pinned one; the stub answers with the real pin unless
        # a test supplies another version.
        version = version or checks.pinned_lean_version(self.script.parents[1] / "proof")
        lake = Path(directory) / "lake"
        lake.write_text(
            "#!/bin/sh\n"
            'if [ "$1 $2 $3" = "env lean --version" ]; then\n'
            f'  echo "Lean (version {version}, x86_64-unknown-linux-gnu, commit 0, Release)"\n'
            "  exit 0\n"
            "fi\n"
            f"{body}\n"
        )
        lake.chmod(lake.stat().st_mode | stat.S_IXUSR)
        return Path(directory)

    def test_missing_lake_exits_nonzero(self):
        with tempfile.TemporaryDirectory() as empty:
            result = self.run_script(Path(empty))
        self.assertEqual(result.returncode, 1)
        self.assertIn("Lean checks failed: ", result.stderr)
        self.assertIn(".elan", result.stderr)

    def test_failing_lake_build_exits_nonzero(self):
        with tempfile.TemporaryDirectory() as directory:
            path_dir = self.stub_lake(directory, 'echo "error: build failed" >&2\nexit 1')
            result = self.run_script(path_dir)
        self.assertEqual(result.returncode, 1)
        self.assertIn(
            "Lean checks failed: Command '['lake', 'build']' returned "
            "non-zero exit status 1.", result.stderr,
        )

    def test_a_lean_other_than_the_pin_exits_nonzero_before_any_build(self):
        with tempfile.TemporaryDirectory() as directory:
            marker = Path(directory) / "built"
            path_dir = self.stub_lake(directory, f"touch {marker}\nexit 0", version="0.0.1")
            result = self.run_script(path_dir)
            built = marker.exists()
        self.assertEqual(result.returncode, 1)
        pinned = checks.pinned_lean_version(self.script.parents[1] / "proof")
        self.assertIn(
            f"Lean checks failed: active Lean 0.0.1 is not the pinned Lean {pinned} "
            "(proof/lean-toolchain)", result.stderr,
        )
        self.assertFalse(built, "the build ran under a Lean other than the pin")

    def test_successful_run_keeps_the_passed_line_and_exit_zero(self):
        # CI and the archived reports reference the "Passed N Lean audits"
        # line; a no-op lake exercises the real transcript against the real
        # proof directory without a toolchain.
        with tempfile.TemporaryDirectory() as directory:
            path_dir = self.stub_lake(directory, "exit 0")
            result = self.run_script(path_dir)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertRegex(result.stdout, r"Passed \d+ Lean audits\n$")


class ToolchainGuardTests(unittest.TestCase):
    """LEAN-4-35-TOOLCHAIN: the runner refuses to build or audit under a Lean
    other than the one `proof/lean-toolchain` pins, so an override (elan's
    `ELAN_TOOLCHAIN`, a `+toolchain` shim, a non-elan `lake`) cannot check the
    proofs under a different parser (Lean 4.35 made `given` a contract
    keyword in `def` signatures)."""

    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.proof = Path(self.directory.name)

    def pin(self, text):
        (self.proof / "lean-toolchain").write_text(text)

    def query(self, stdout="", returncode=0, stderr=""):
        return subprocess.CompletedProcess(
            ["lake", "env", "lean", "--version"], returncode, stdout, stderr,
        )

    def guard(self, **query):
        with patch.object(checks.subprocess, "run", return_value=self.query(**query)) as run:
            with contextlib.redirect_stdout(io.StringIO()):
                active = checks.check_pinned_toolchain(self.proof, 5)
        return active, run

    def test_pin_is_read_from_lean_toolchain(self):
        for text, version in (
            ("leanprover/lean4:v4.35.0\n", "4.35.0"),
            ("leanprover/lean4:v4.36.0-rc1", "4.36.0-rc1"),
        ):
            with self.subTest(text=text):
                self.pin(text)
                self.assertEqual(checks.pinned_lean_version(self.proof), version)

    def test_unreadable_or_unversioned_pin_refuses(self):
        with self.assertRaises(ValueError):
            checks.pinned_lean_version(self.proof)
        for text in ("stable", "leanprover/lean4:nightly", "leanprover/lean4:v4.35"):
            with self.subTest(text=text):
                self.pin(text)
                with self.assertRaises(ValueError):
                    checks.pinned_lean_version(self.proof)

    def test_repository_pin_is_readable(self):
        proof = Path(checks.__file__).resolve().parents[1] / "proof"
        self.assertRegex(checks.pinned_lean_version(proof), r"^\d+\.\d+\.\d+")

    def test_matching_lean_passes_and_queries_lake_inside_proof(self):
        self.pin("leanprover/lean4:v4.35.0\n")
        active, run = self.guard(
            stdout="Lean (version 4.35.0, x86_64-unknown-linux-gnu, commit eb6cafb, Release)\n")
        self.assertEqual(active, "4.35.0")
        self.assertEqual(run.call_args.args[0], ["lake", "env", "lean", "--version"])
        self.assertEqual(run.call_args.kwargs["cwd"], self.proof)

    def test_other_lean_fails_naming_both_versions(self):
        self.pin("leanprover/lean4:v4.35.0\n")
        for other in ("4.34.0", "4.35.1", "4.35.0-rc1"):
            with self.subTest(active=other):
                with self.assertRaises(ValueError) as caught:
                    self.guard(stdout=f"Lean (version {other}, x86_64-unknown-linux-gnu, Release)\n")
                message = str(caught.exception)
                self.assertIn(f"active Lean {other} is not the pinned Lean 4.35.0", message)
                self.assertIn("proof/lean-toolchain", message)

    def test_failed_or_unreadable_version_query_fails(self):
        self.pin("leanprover/lean4:v4.35.0\n")
        with self.assertRaises(ValueError):
            self.guard(returncode=1, stderr="error: unknown toolchain")
        with self.assertRaises(ValueError):
            self.guard(stdout="Lake version 5.0.0\n")

    def test_version_query_timeout_is_a_timeout(self):
        self.pin("leanprover/lean4:v4.35.0\n")
        timeout = subprocess.TimeoutExpired(["lake"], 5)
        with patch.object(checks.subprocess, "run", side_effect=timeout):
            with self.assertRaises(TimeoutError):
                checks.check_pinned_toolchain(self.proof, 5)

    def test_main_refuses_before_any_build_or_audit(self):
        stderr = io.StringIO()
        refusal = ValueError("active Lean 4.34.0 is not the pinned Lean 4.35.0 (proof/lean-toolchain)")
        with patch.object(checks.shutil, "which", return_value="/usr/bin/lake"):
            with patch.object(checks, "check_pinned_toolchain", side_effect=refusal):
                with patch.object(checks, "check_audits") as audits:
                    with contextlib.redirect_stderr(stderr):
                        status = checks.main([])
        audits.assert_not_called()
        self.assertEqual(status, 1)
        self.assertIn("is not the pinned Lean", stderr.getvalue())


if __name__ == "__main__":
    unittest.main()

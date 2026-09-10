"""Regression checks for bounded Lean verification orchestration."""
import argparse
import contextlib
import io
import os
from pathlib import Path
import subprocess
import sys
import tempfile
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

    def test_worker_count_never_exceeds_the_audits_present(self):
        self.args.jobs = 999
        with patch.object(checks, "run_audits_in_parallel") as parallel:
            checks.check_audits(self.proof, self.args)
        self.assertEqual(parallel.call_args.args[-1], len(self.names))


if __name__ == "__main__":
    unittest.main()

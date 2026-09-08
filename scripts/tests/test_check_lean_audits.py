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


if __name__ == "__main__":
    unittest.main()

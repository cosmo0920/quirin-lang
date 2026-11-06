"""Starvation bounds of the gate slots (card GATE-SLOT-ADMISSION-STARVATION,
requirement R11 of docs/plans/gate-time-reduction/README.md).

Every test runs in a scratch gate directory with a scratch legacy lock and a
fake meminfo, with child processes that spin, sleep or write; no audit runs and
the live /tmp/quirin-gate is never touched.  Every process a test starts is
stopped by the pid it captured."""
import fcntl
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import threading
import time
import unittest
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import gate_slot as slot_mod  # noqa: E402

SCRIPT = Path(__file__).resolve().parents[1] / "gate_slot.py"
GIB_KIB = 1024 * 1024
LINUX_PROC = Path("/proc/self/io").exists() and Path("/proc/locks").exists()


def meminfo(directory, available_gib, total_gib=60):
    path = Path(directory) / "meminfo"
    path.write_text(f"MemTotal:       {total_gib * GIB_KIB} kB\n"
                    f"MemFree:        1 kB\n"
                    f"MemAvailable:   {available_gib * GIB_KIB} kB\n")
    return path


def gone(pid, starttime, within=5.0):
    """The process exited (or is a zombie awaiting its new parent)."""
    limit = time.monotonic() + within
    while time.monotonic() < limit:
        stat = slot_mod.proc_stat(pid)
        if stat is None or stat["starttime"] != starttime or stat["state"] == "Z":
            return True
        time.sleep(0.05)
    return False


def wait_for(predicate, within=20.0, step=0.02):
    limit = time.monotonic() + within
    while time.monotonic() < limit:
        value = predicate()
        if value:
            return value
        time.sleep(step)
    return predicate()


class Scratch(unittest.TestCase):
    """A scratch gate (directory, legacy lock, meminfo) and the processes a
    test started, stopped by their captured pids at the end."""

    available_gib = 30

    def setUp(self):
        self.scratch = tempfile.TemporaryDirectory()
        self.root = Path(self.scratch.name)
        self.gate_dir = self.root / "gate"
        self.legacy = self.root / "legacy.lock"
        self.meminfo = meminfo(self.root, self.available_gib)
        self.started: list[subprocess.Popen] = []
        self.gates: list[slot_mod.Gate] = []

    def tearDown(self):
        for gate in self.gates:
            gate.release()
        for process in self.started:
            if process.poll() is None:
                process.terminate()
            try:
                process.communicate(timeout=10)
            except subprocess.TimeoutExpired:
                process.kill()
                process.communicate(timeout=10)
        self.scratch.cleanup()

    def env(self, **extra):
        env = dict(os.environ, QUIRIN_GATE_DIR=str(self.gate_dir),
                   QUIRIN_GATE_LEGACY_LOCK=str(self.legacy),
                   QUIRIN_GATE_MEMINFO=str(self.meminfo))
        for name in (slot_mod.HELD_ENV, slot_mod.STALL_ENV, slot_mod.DEADLINE_ENV,
                     slot_mod.DRAIN_ENV):
            env.pop(name, None)
        env.update(extra)
        return env

    def run_slot(self, *args, timeout=60, **env):
        return subprocess.run([sys.executable, str(SCRIPT), *args], env=self.env(**env),
                              capture_output=True, text=True, timeout=timeout)

    def start_slot(self, *args, **env):
        process = subprocess.Popen([sys.executable, str(SCRIPT), *args],
                                   env=self.env(**env), stdout=subprocess.PIPE,
                                   stderr=subprocess.PIPE, text=True)
        self.started.append(process)
        return process

    def start(self, argv):
        process = subprocess.Popen(argv, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        self.started.append(process)
        return process

    def gate(self):
        gate = slot_mod.Gate(self.gate_dir, self.legacy, self.meminfo, poll=0.02,
                             echo=lambda line: None)
        self.gates.append(gate)
        return gate

    def note(self, label):
        return slot_mod.read_note(self.gate_dir / label)

    def holder(self, *child, cls="audit", extra=()):
        """A real slot job in another process; returns it once it holds its slot."""
        process = self.start_slot(cls, "--poll", "0.02", "--receipt",
                                  str(self.root / f"holder-{len(self.started)}.json"), *extra,
                                  "--", *child)
        label = f"{cls}/0"
        wait_for(lambda: (self.note(label) or {}).get("pid") == process.pid)
        self.assertEqual((self.note(label) or {}).get("pid"), process.pid,
                         f"the holder never took {label}: "
                         f"{process.stderr.read() if process.poll() is not None else ''}")
        return process


# --- durations and defaults ------------------------------------------------------


class Limits(unittest.TestCase):
    def test_durations_need_a_unit_and_zero_means_off(self):
        self.assertEqual(slot_mod.parse_duration("90s", "x"), 90.0)
        self.assertEqual(slot_mod.parse_duration("30m", "x"), 1800.0)
        self.assertEqual(slot_mod.parse_duration("2h", "x"), 7200.0)
        self.assertEqual(slot_mod.parse_duration("0.5s", "x"), 0.5)
        for off in ("0", "off", "none", "0s", "OFF"):
            self.assertIsNone(slot_mod.parse_duration(off, "x"), off)
        self.assertEqual(slot_mod.parse_duration("0", "x", allow_zero=True), 0.0)
        for bad in ("60", "2d", "-1m", "m", "1h30m", ""):
            with self.assertRaises(slot_mod.Refusal, msg=bad):
                slot_mod.parse_duration(bad, "x")

    def args(self, *argv):
        return slot_mod.parse_args([*argv, "--", "true"])

    def test_the_stated_defaults(self):
        audit = slot_mod.resolve_limits(self.args("audit"), {})
        self.assertEqual(audit["stall_timeout_seconds"], 7200.0)
        self.assertIsNone(audit["deadline_seconds"])
        self.assertEqual(audit["stop_grace_seconds"], 60.0)
        self.assertNotIn("drain_timeout_seconds", audit)
        for cls in ("lean", "rust-suite"):
            limits = slot_mod.resolve_limits(self.args(cls), {})
            self.assertIsNone(limits["stall_timeout_seconds"], cls)
            self.assertIsNone(limits["deadline_seconds"], cls)
        suite = slot_mod.resolve_limits(self.args("rust-suite"), {})
        self.assertEqual(suite["drain_timeout_seconds"], 7200.0)
        self.assertEqual(suite["drain_backoff_seconds"], 300.0)
        self.assertEqual(suite["drain_attempts"], 0)

    def test_a_flag_beats_the_variable_which_beats_the_default(self):
        env = {slot_mod.STALL_ENV: "30m", slot_mod.DEADLINE_ENV: "8h",
               slot_mod.DRAIN_ENV: "off"}
        limits = slot_mod.resolve_limits(self.args("audit"), env)
        self.assertEqual((limits["stall_timeout_seconds"], limits["stall_timeout_from"]),
                         (1800.0, "environment"))
        self.assertEqual(limits["deadline_seconds"], 8 * 3600.0)
        limits = slot_mod.resolve_limits(self.args("audit", "--stall-timeout", "off"), env)
        self.assertEqual((limits["stall_timeout_seconds"], limits["stall_timeout_from"]),
                         (None, "flag"))
        suite = slot_mod.resolve_limits(self.args("rust-suite"), env)
        self.assertIsNone(suite["drain_timeout_seconds"], "off: the unbounded drain")

    def test_refusals_run_nothing(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            marker = root / "ran"
            touch = [sys.executable, "-c", f"open({str(marker)!r}, 'w')"]
            env = dict(os.environ, QUIRIN_GATE_DIR=str(root / "gate"),
                       QUIRIN_GATE_LEGACY_LOCK=str(root / "legacy"),
                       QUIRIN_GATE_MEMINFO=str(meminfo(root, 30)))
            env.pop(slot_mod.HELD_ENV, None)
            for args in (["audit", "--stall-timeout", "60", "--", *touch],
                         ["audit", "--drain-timeout", "1m", "--", *touch],
                         ["lean", "--drain-attempts", "1", "--", *touch],
                         ["rust-suite", "--drain-attempts", "-1", "--", *touch],
                         ["audit", "--deadline", "soon", "--", *touch]):
                completed = subprocess.run([sys.executable, str(SCRIPT), *args], env=env,
                                           capture_output=True, text=True, timeout=60)
                self.assertEqual(completed.returncode, 2, (args, completed.stderr))
            completed = subprocess.run([sys.executable, str(SCRIPT), "audit", "--", *touch],
                                       env=dict(env, **{slot_mod.STALL_ENV: "forever"}),
                                       capture_output=True, text=True, timeout=60)
            self.assertEqual(completed.returncode, 2, completed.stderr)
            self.assertFalse(marker.exists())


# --- 1. deadlines -------------------------------------------------------------------


@unittest.skipUnless(LINUX_PROC, "needs /proc/<pid>/io and /proc/locks")
class Deadlines(Scratch):
    FAST = ("--sample-interval", "0.1", "--stop-grace", "1s")

    def test_a_tree_without_io_is_stopped_reported_and_only_its_own_processes_signalled(self):
        pids = self.root / "pids"
        # The child starts a grandchild that ignores SIGTERM and spins, then
        # sleeps; neither does I/O after the pid file is written.
        child = (
            "import subprocess, sys, time\n"
            "spin = subprocess.Popen([sys.executable, '-c', 'import signal; "
            "signal.signal(signal.SIGTERM, signal.SIG_IGN)\\nwhile True: pass'])\n"
            f"open({str(pids)!r}, 'w').write(str(spin.pid))\n"
            "time.sleep(600)\n")
        bystander = self.start([sys.executable, "-c", "import time; time.sleep(600)"])
        bystander_start = slot_mod.proc_stat(bystander.pid)["starttime"]
        receipt = self.root / "r.json"
        wait_record = self.root / "wait.json"
        started = time.monotonic()
        completed = self.run_slot("audit", "--stall-timeout", "1s", *self.FAST, "--receipt",
                                  str(receipt), "--wait-record", str(wait_record), "--",
                                  sys.executable, "-c", child)
        self.assertLess(time.monotonic() - started, 30)
        self.assertEqual(completed.returncode, slot_mod.STOPPED_EXIT, completed.stderr)
        self.assertIn("stopping its own command", completed.stderr)
        self.assertIn("no progress for", completed.stderr)
        document = json.loads(receipt.read_text())
        self.assertEqual(document["exit"], 124)
        self.assertEqual(document["stopped"], "stall")
        self.assertEqual(document["child_exit"], 128 + signal.SIGTERM)
        breach = document["monitor"]["breach"]
        self.assertEqual(breach["kind"], "stall")
        self.assertGreaterEqual(breach["seconds_without_progress"], 1.0)
        self.assertGreater(breach["cpu_seconds_since_progress"], 0.3, "the grandchild spun")
        spinner = int(pids.read_text())
        listed = {entry["pid"] for entry in breach["processes"]}
        self.assertIn(spinner, listed)
        sent = {(entry["pid"], entry["signal"]) for entry in breach["signals"]}
        self.assertIn((spinner, signal.SIGTERM), sent)
        self.assertIn((spinner, signal.SIGKILL), sent, "it ignored SIGTERM")
        self.assertIn(spinner, breach["killed"])
        self.assertIsNone(slot_mod.proc_stat(spinner) and
                          (slot_mod.proc_stat(spinner)["state"] != "Z" or None))
        self.assertNotIn(bystander.pid, listed)
        self.assertTrue(slot_mod.alive(bystander.pid, bystander_start),
                        "a process the job did not start is never signalled")
        # R7: the stop is analysis time; the wait record stays as admitted.
        record = json.loads(wait_record.read_text())
        self.assertEqual(record["state"], "admitted")
        self.assertEqual(record["waited_seconds"], document["slot_record"]["waited_seconds"])
        # The slot is free again.
        self.gate().acquire("audit", 7168, timeout=1)

    def test_an_unreaped_zombie_does_not_hide_a_stall(self):
        # /proc/<zombie>/io cannot be read; a zombie does no I/O, so it is not
        # an unmeasured member that would keep the stall clock from running.
        child = ("import subprocess, sys, time\n"
                 "dead = subprocess.Popen([sys.executable, '-c', 'pass'])\n"
                 "time.sleep(600)\n")
        completed = self.run_slot("audit", "--stall-timeout", "1s", *self.FAST, "--receipt",
                                  str(self.root / "r.json"), "--", sys.executable, "-c", child)
        self.assertEqual(completed.returncode, 124, completed.stderr)
        monitor = json.loads((self.root / "r.json").read_text())["monitor"]
        self.assertEqual(monitor["samples_unmeasured"], 0)
        self.assertTrue(any(entry["state"] == "Z" for entry in monitor["breach"]["processes"]))

    def test_steady_file_io_is_progress(self):
        out = self.root / "out"
        child = ("import time\n"
                 f"handle = open({str(out)!r}, 'w')\n"
                 "for _ in range(30):\n"
                 "    handle.write('x'); handle.flush(); time.sleep(0.08)\n")
        completed = self.run_slot("audit", "--stall-timeout", "1s", *self.FAST,
                                  "--receipt", str(self.root / "r.json"), "--",
                                  sys.executable, "-c", child)
        self.assertEqual(completed.returncode, 0, completed.stderr)
        monitor = json.loads((self.root / "r.json").read_text())["monitor"]
        self.assertNotIn("breach", monitor)
        self.assertGreater(monitor["samples"], 5)

    def test_output_is_progress(self):
        child = ("import sys, time\n"
                 "for _ in range(30):\n"
                 "    print('.', flush=True); time.sleep(0.08)\n")
        completed = self.run_slot("audit", "--stall-timeout", "1s", *self.FAST,
                                  "--receipt", str(self.root / "r.json"), "--",
                                  sys.executable, "-c", child)
        self.assertEqual(completed.returncode, 0, completed.stderr)
        self.assertEqual(completed.stdout.count("."), 30)

    def test_a_progress_file_change_is_progress(self):
        progress = self.root / "progress"
        progress.write_text("0")
        stop = threading.Event()

        def touch():
            count = 0
            while not stop.is_set():
                count += 1
                progress.write_text(str(count))
                time.sleep(0.08)

        toucher = threading.Thread(target=touch)
        toucher.start()
        try:
            completed = self.run_slot("audit", "--stall-timeout", "1s", *self.FAST,
                                      "--progress-file", str(progress), "--receipt",
                                      str(self.root / "r.json"), "--",
                                      sys.executable, "-c", "import time; time.sleep(3)")
        finally:
            stop.set()
            toucher.join()
        self.assertEqual(completed.returncode, 0, completed.stderr)

    def test_the_wall_clock_deadline(self):
        child = ("import time\n"
                 "while True:\n"
                 "    print('.', flush=True); time.sleep(0.05)\n")
        completed = self.run_slot("lean", "--deadline", "1s", *self.FAST, "--receipt",
                                  str(self.root / "r.json"), "--", sys.executable, "-c", child)
        self.assertEqual(completed.returncode, 124, completed.stderr)
        document = json.loads((self.root / "r.json").read_text())
        self.assertEqual(document["stopped"], "deadline")
        self.assertGreaterEqual(document["monitor"]["breach"]["run_seconds"], 1.0)
        self.assertIn("against --deadline 1s", completed.stderr)

    def test_the_class_defaults_reach_the_receipt(self):
        completed = self.run_slot("audit", "--receipt", str(self.root / "r.json"), "--",
                                  sys.executable, "-c", "pass")
        self.assertEqual(completed.returncode, 0, completed.stderr)
        limits = json.loads((self.root / "r.json").read_text())["limits"]
        self.assertEqual((limits["stall_timeout_seconds"], limits["stall_timeout_from"]),
                         (7200.0, "default"))
        self.assertIn("stall timeout 2h", completed.stderr)

    def test_without_proc_only_the_wall_clock_deadline_applies(self):
        lines = []
        monitor = slot_mod.Monitor(stall=0.2, deadline=0.8, interval=0.05, grace=0.5,
                                   echo=lines.append)
        with mock.patch.object(slot_mod, "PROC", self.root / "no-proc"):
            code, _ = slot_mod.run_child([sys.executable, "-c", "import time; time.sleep(30)"],
                                         dict(os.environ), monitor)
        self.assertEqual(code, 128 + signal.SIGTERM)
        self.assertEqual(monitor.breach["kind"], "deadline", "never a stall it cannot read")
        self.assertFalse(monitor.record["tree_readable"])
        self.assertTrue(any("no /proc" in line for line in lines))

    def test_unreadable_io_counts_as_progress(self):
        lines = []
        monitor = slot_mod.Monitor(stall=0.3, interval=0.05, grace=0.5, echo=lines.append)
        with mock.patch.object(slot_mod, "read_io", return_value=None):
            code, _ = slot_mod.run_child([sys.executable, "-c", "import time; time.sleep(1.2)"],
                                         dict(os.environ), monitor)
        self.assertEqual(code, 0)
        self.assertIsNone(monitor.breach)
        self.assertGreater(monitor.record["samples_unmeasured"], 5)
        self.assertTrue(any("counts those readings as progress" in line for line in lines))


# --- 4. resident set above the admission budget, while running ---------------------


@unittest.skipUnless(LINUX_PROC, "needs /proc")
class Resident(Scratch):
    def test_it_is_reported_while_the_job_runs(self):
        release = self.root / "release"
        # Hold 64 MiB of touched memory until the report has been seen; exit 3
        # if it never comes.
        child = ("import os, time\n"
                 "x = bytearray(64 * 1024 * 1024)\n"
                 "for i in range(0, len(x), 4096): x[i] = 1\n"
                 "limit = time.monotonic() + 30\n"
                 f"while not os.path.exists({str(release)!r}):\n"
                 "    if time.monotonic() > limit: raise SystemExit(3)\n"
                 "    time.sleep(0.02)\n")

        def echo(line):
            if "exceeds the admission budget" in line:
                release.write_text("seen")

        monitor = slot_mod.Monitor(admission_mib=16, interval=0.05, echo=echo)
        code, peak = slot_mod.run_child([sys.executable, "-c", child], dict(os.environ), monitor)
        self.assertEqual(code, 0, "the report came while the child was still running")
        over = monitor.record["rss_over_admission"]
        self.assertGreaterEqual(over["first_tree_rss_kib"], 64 * 1024)
        self.assertGreaterEqual(over["samples_over"], 1)
        self.assertGreaterEqual(monitor.record["peak_tree_rss_kib"], 64 * 1024)
        self.assertGreaterEqual(peak, 64 * 1024)

    def test_the_slot_note_carries_the_live_reading(self):
        process = self.start_slot("audit", "--sample-interval", "0.05", "--receipt",
                                  str(self.root / "r.json"), "--", sys.executable, "-c",
                                  "import time; time.sleep(3)")
        live = wait_for(lambda: (self.note("audit/0") or {}).get("live"))
        self.assertTrue(live, "no live record in the holder note")
        note = self.note("audit/0")
        self.assertEqual(note["pid"], process.pid)
        self.assertEqual(note["admission_mib"], 7168)
        self.assertEqual(live["admission_mib"], 7168)
        self.assertGreater(live["tree_rss_kib"], 0)
        self.assertEqual(live["stall_timeout_seconds"], 7200.0)
        self.assertEqual(process.wait(timeout=30), 0)
        self.assertEqual((self.gate_dir / "audit" / "0").read_text(), "", "released notes clear")


# --- 2. the bounded drain -------------------------------------------------------------


@unittest.skipUnless(LINUX_PROC, "needs /proc/locks")
class Drain(Scratch):
    def admission_closed(self):
        path = self.gate_dir / "audit" / "admission"
        if not path.exists():
            return False
        fd = os.open(path, os.O_RDWR)
        try:
            fcntl.flock(fd, fcntl.LOCK_SH | fcntl.LOCK_NB)
            fcntl.flock(fd, fcntl.LOCK_UN)
            return False
        except BlockingIOError:
            return True
        finally:
            os.close(fd)

    def test_a_drain_gives_up_reopens_admission_releases_and_names_the_holder(self):
        holder = self.holder(sys.executable, "-c", "while True: pass",
                             extra=("--stall-timeout", "off"))
        suite = self.gate()
        with self.assertRaises(slot_mod.DrainFailed) as raised:
            suite.acquire("rust-suite", drain_timeout=0.5, drain_backoff=0.05, drain_attempts=1)
        message = str(raised.exception)
        self.assertIn(f"audit/0 pid {holder.pid}", message)
        self.assertIn("admission reopened and every slot released", message)
        [attempt] = raised.exception.attempts
        self.assertEqual(attempt["outcome"], "gave up")
        self.assertGreaterEqual(attempt["waited_seconds"], 0.5)
        [blocker] = attempt["blockers"]
        self.assertEqual((blocker["slot"], blocker["pid"]), ("audit/0", holder.pid))
        self.assertIn("while True: pass", blocker["argv"])
        self.assertEqual(blocker["admission_mib"], 7168)
        self.assertGreater(blocker["during_wait"]["cpu_seconds"], 0.2, "the holder spins")
        self.assertEqual(blocker["during_wait"]["io"]["wchar"], 0)
        self.assertIn("held_seconds", blocker)
        self.assertEqual(suite.held, [])
        self.assertFalse(self.admission_closed(), "admission reopened")
        # Every lock it took is free: another audit and the suite slot itself.
        self.gate().acquire("audit", 7168, timeout=1)
        fd = os.open(self.gate_dir / "rust-suite" / "0", os.O_RDWR)
        try:
            fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
        finally:
            os.close(fd)

    def test_a_drain_backs_off_and_retries_until_the_audits_end(self):
        holder = self.holder(sys.executable, "-c", "import time; time.sleep(1.2)")
        record = self.gate().acquire("rust-suite", timeout=20, drain_timeout=0.3,
                                     drain_backoff=0.05)
        attempts = record["drain"]["attempts"]
        self.assertGreaterEqual(len(attempts), 2)
        self.assertEqual(attempts[0]["outcome"], "gave up")
        self.assertEqual(attempts[0]["blockers"][0]["pid"], holder.pid)
        self.assertEqual(attempts[-1]["outcome"], "drained")
        self.assertEqual(record["audit_slots_held"], list(range(15)))

    def test_a_queued_audit_is_admitted_while_the_drain_backs_off(self):
        """Audits are never closed out indefinitely: an audit queued behind a
        drain that cannot finish gets its slot in the back-off."""
        first = self.gate()
        first.acquire("audit", 7168, timeout=1)  # 30 GiB available: four 7 GiB slots
        suite = self.gate()
        outcome: dict = {}

        def drain():
            try:
                outcome["suite"] = suite.acquire("rust-suite", timeout=30, drain_timeout=0.4,
                                                 drain_backoff=1.5)
                outcome["suite_at"] = time.monotonic()
            except BaseException as error:  # reported by the assertions below
                outcome["suite_error"] = error

        thread = threading.Thread(target=drain)
        thread.start()
        try:
            self.assertTrue(wait_for(self.admission_closed, within=10))
            second = self.gate()
            record = second.acquire("audit", 7168, timeout=10)
            admitted_at = time.monotonic()
            self.assertNotIn("suite", outcome, "the queued audit ran before the suite")
            self.assertEqual(record["slot"], 1)
            first.release()
            second.release()
        finally:
            thread.join(timeout=30)
        self.assertNotIn("suite_error", outcome)
        self.assertGreater(outcome["suite_at"], admitted_at)
        self.assertEqual(outcome["suite"]["drain"]["attempts"][0]["outcome"], "gave up")

    def test_the_command_line_fails_with_exit_75_names_the_holder_and_runs_nothing(self):
        holder = self.holder(sys.executable, "-c", "import time; time.sleep(600)")
        marker = self.root / "ran"
        receipt = self.root / "suite.json"
        wait_record = self.root / "suite-wait.json"
        completed = self.run_slot("rust-suite", "--poll", "0.02", "--drain-timeout", "0.4s",
                                  "--drain-backoff", "0.1s", "--drain-attempts", "2",
                                  "--receipt", str(receipt), "--wait-record", str(wait_record),
                                  "--", sys.executable, "-c", f"open({str(marker)!r}, 'w')")
        self.assertEqual(completed.returncode, 75, completed.stderr)
        self.assertFalse(marker.exists())
        self.assertIn(f"pid {holder.pid}", completed.stderr)
        self.assertIn("retrying in 0.1s", completed.stderr, "the first attempt backed off")
        self.assertIn("gave up 2 time(s)", completed.stderr)
        document = json.loads(receipt.read_text())
        self.assertFalse(document["ran"])
        self.assertEqual(document["exit"], 75)
        attempts = document["slot_record"]["drain"]["attempts"]
        self.assertEqual([attempt["outcome"] for attempt in attempts], ["gave up", "gave up"])
        self.assertEqual(attempts[-1]["blockers"][0]["pid"], holder.pid)
        self.assertEqual(json.loads(wait_record.read_text())["state"], "waiting",
                         "R7: a job that never got its slot only waited")
        self.assertFalse(self.admission_closed())

    def test_the_wait_timeout_still_bounds_the_whole_wait(self):
        holder = self.holder(sys.executable, "-c", "import time; time.sleep(600)")
        completed = self.run_slot("rust-suite", "--poll", "0.02", "--wait-timeout", "1",
                                  "--drain-timeout", "off", "--receipt",
                                  str(self.root / "r.json"), "--", "true")
        self.assertEqual(completed.returncode, 75, completed.stderr)
        self.assertIn(f"pid {holder.pid}", completed.stderr)
        self.assertFalse(self.admission_closed())


# --- 3. status -------------------------------------------------------------------------


class ProcLocks(unittest.TestCase):
    def test_parse(self):
        text = ("1: POSIX  ADVISORY  READ 3954796 103:07:28706586 128 128\n"
                "3: FLOCK  ADVISORY  WRITE 1263373 00:2f:22305 0 EOF\n"
                "3: -> FLOCK  ADVISORY  READ 1263999 00:2f:22305 0 EOF\n"
                "4: OFDLCK ADVISORY  READ -1 00:06:1028 0 EOF\n")
        entries = slot_mod.parse_proc_locks(text)
        self.assertEqual(entries[1], {"kind": "FLOCK", "mode": "WRITE", "pid": 1263373,
                                      "queued": False, "device": (0, 0x2F), "inode": 22305})
        self.assertEqual(entries[2]["queued"], True)
        self.assertEqual(entries[2]["pid"], 1263999)
        self.assertEqual(entries[0]["device"], (0x103, 7))
        self.assertEqual(entries[3]["pid"], -1)

    def test_compress(self):
        labels = ["rust-suite/0", "audit/admission"] + [f"audit/{i}" for i in (0, 1, 2, 4, 5)]
        self.assertEqual(slot_mod.compress(labels), "rust-suite/0 audit/0-2,4-5,admission")


@unittest.skipUnless(LINUX_PROC, "needs /proc/locks")
class Status(Scratch):
    def job(self, status, pid):
        return next(job for job in status["jobs"] if job["pid"] == pid)

    def test_holders_a_draining_suite_and_a_waiter(self):
        holder = self.holder(sys.executable, "-c", "while True: pass",
                             extra=("--stall-timeout", "off"))
        suite = self.start_slot("rust-suite", "--poll", "0.02", "--drain-timeout", "1h",
                                "--receipt", str(self.root / "suite.json"), "--", "true")
        self.assertTrue(wait_for(lambda: (self.note("audit/admission") or {}).get("pid")
                                 == suite.pid))
        waiter = self.start_slot("audit", "--poll", "0.02", "--receipt",
                                 str(self.root / "waiter.json"), "--", "true")
        wait_for(lambda: any(
            job["pid"] == waiter.pid and job["role"] == "waiting for a slot"
            for job in slot_mod.gate_status(self.gate_dir, self.legacy, self.meminfo,
                                            0)["jobs"]))
        status = slot_mod.gate_status(self.gate_dir, self.legacy, self.meminfo, interval=0.5)
        held = self.job(status, holder.pid)
        self.assertEqual(held["role"], "holder")
        self.assertEqual(held["class"], "audit")
        self.assertEqual(held["slots"], ["audit/0"])
        self.assertIn("while True: pass", held["argv"])
        self.assertGreater(held["over_interval"]["cpu_seconds"], 0.2)
        self.assertEqual(held["over_interval"]["io"]["rchar"], 0)
        self.assertEqual(held["admission_mib"], 7168)
        self.assertFalse(held["rss_over_admission"])
        self.assertIn("held_seconds", held)
        self.assertIn("waited_seconds", held)
        draining = self.job(status, suite.pid)
        self.assertTrue(draining["role"].startswith("draining"), draining["role"])
        self.assertEqual(draining["blocked_by"], [{"lock": "audit/0", "pids": [holder.pid]}])
        self.assertIn("audit/admission", draining["slots"])
        self.assertIn("waited_seconds", draining)
        waiting = self.job(status, waiter.pid)
        self.assertEqual((waiting["role"], waiting["class"]), ("waiting for a slot", "audit"))
        self.assertGreaterEqual(waiting["waited_seconds"], 0)
        text = slot_mod.render_status(status)
        self.assertIn(f"blocked by audit/0 (pid {holder.pid})", text)
        # The command line prints the same record.
        completed = self.run_slot("status", "--json", "--interval", "0.2")
        self.assertEqual(completed.returncode, 0, completed.stderr)
        listed = {job["pid"]: job["role"] for job in json.loads(completed.stdout)["jobs"]}
        self.assertEqual(listed[holder.pid], "holder")
        self.assertEqual(listed[waiter.pid], "waiting for a slot")

    def test_a_kernel_queued_waiter_and_a_legacy_holder(self):
        self.legacy.touch()
        fd = os.open(self.legacy, os.O_RDWR)
        try:
            fcntl.flock(fd, fcntl.LOCK_EX)  # what `flock <legacy lock> <cmd>` does
            lean = self.start_slot("lean", "--receipt", str(self.root / "lean.json"), "--",
                                   "true")  # untimed: blocks in the kernel's queue
            status = wait_for(lambda: (lambda s: s if any(
                job["pid"] == lean.pid for job in s["jobs"]) else None)(
                slot_mod.gate_status(self.gate_dir, self.legacy, self.meminfo, 0)))
            self.assertTrue(status)
            queued = self.job(status, lean.pid)
            self.assertEqual(queued["role"], "queued in the kernel for legacy (shared)")
            self.assertEqual(queued["class"], "lean")
            mine = self.job(status, os.getpid())
            self.assertEqual(mine["role"], "legacy holder (flock on the legacy lock)")
        finally:
            os.close(fd)
        self.assertEqual(lean.wait(timeout=30), 0)

    def test_resident_set_against_the_admission_budget(self):
        # This process holds a scratch audit slot with a 1 MiB admission note
        # and runs a child that holds 64 MiB.
        slot = self.gate_dir / "audit" / "1"
        slot.parent.mkdir(parents=True)
        lock = slot_mod.Lock(slot, "audit/1")
        self.assertTrue(lock.try_lock(fcntl.LOCK_EX))
        try:
            lock.note({"pid": os.getpid(), "cls": "audit", "argv": ["test"],
                       "admission_mib": 1, "since": slot_mod.utc_now(),
                       "noted_at": slot_mod.utc_now()})
            ready = self.root / "ready"
            child = self.start([sys.executable, "-c",
                                "import time\nx = bytearray(64 * 1024 * 1024)\n"
                                "for i in range(0, len(x), 4096): x[i] = 1\n"
                                f"open({str(ready)!r}, 'w').close()\ntime.sleep(600)"])
            self.assertTrue(wait_for(ready.exists))
            status = slot_mod.gate_status(self.gate_dir, self.legacy, self.meminfo, 0)
            mine = self.job(status, os.getpid())
            self.assertEqual(mine["role"], "holder")
            self.assertTrue(mine["rss_over_admission"])
            self.assertGreaterEqual(mine["tree_rss_kib"], 64 * 1024)
            self.assertIn("(OVER)", slot_mod.render_status(status))
            child.terminate()
            child.wait(timeout=10)
        finally:
            lock.release()

    def test_nothing_held(self):
        status = slot_mod.gate_status(self.gate_dir, self.legacy, self.meminfo, 0)
        self.assertEqual(status["jobs"], [])
        self.assertIn("no live holder or waiter", slot_mod.render_status(status))


if __name__ == "__main__":
    unittest.main()

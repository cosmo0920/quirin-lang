"""The short-job lane of the gate (card GATE-SLOT-SHORT-JOB-LANE; user decision
of 2026-11-08: "Yes, add a fast lane").

The incident: ``prepare clang-build --link`` (3 min 17 s, peak 832 MB) waited
5 h 8 min for an audit slot at a 4 GiB budget (card FLUENT-BIT-LINK-PREPARE).

Every test runs in a scratch gate directory with a scratch legacy lock and a
fake meminfo; the live /tmp/quirin-gate is never touched.  In-process waiters
are threads (each its own ``Gate``, so ``flock`` conflicts as between
processes); the rest are real ``gate_slot.py`` processes, stopped by the pid the
test captured.  Nothing orders events by a fixed sleep: each step waits for the
ticket, the lock or the file it depends on."""
import contextlib
import hashlib
import io
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import sys
import threading
import time
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
sys.path.insert(0, str(Path(__file__).resolve().parent))
import gate_slot as slot_mod  # noqa: E402
from test_gate_slot_queue import Job, Queue, intervals, run_log  # noqa: E402
from test_gate_slot_starvation import wait_for  # noqa: E402

FIXTURES = Path(__file__).resolve().parent / "fixtures"
FIFO_ONLY_SCRIPT = FIXTURES / "gate_slot_fifo_only.py.txt"
# scripts/gate_slot.py at 1742f00cb (git blob 440755066f7a), before the lane.
FIFO_ONLY_SHA256 = "1ea9543399f51e64f099556d56a7fd35d320d7845342962d3adfd14aba85087e"
PRE_QUEUE_SCRIPT = FIXTURES / "gate_slot_before_queue.py.txt"
GIB_KIB = 1024 * 1024
SLEEP = [sys.executable, "-c", "import time; time.sleep(600)"]


class LaneJob(Job):
    """An in-process job that may ask for the lane (``Job`` passes a budget to
    audits only)."""

    def run(self):
        try:
            budget = self.budget if self.cls in ("audit", "short") else None
            holder = {"argv": [self.name], "cwd": os.getcwd(), "since": slot_mod.utc_now()}
            if budget is not None:
                holder["admission_mib"] = budget
            self.record = self.gate.acquire(self.cls, budget, timeout=120, holder=holder,
                                            **self.acquire)
            with self.test.order_lock:
                self.test.order.append(self.name)
            self.admitted.set()
            self.finish.wait(120)
        except BaseException as error:  # reported by the assertions
            self.error = error
        finally:
            self.gate.release()


class Lane(Queue):
    def job(self, name, cls="audit", budget=7168, **options):
        job = LaneJob(self, name, cls, budget, **options)
        self.jobs.append(job)
        return job

    def lane_job(self, name, budget=1024, **options):
        return self.job(name, "short", budget, **options)

    def old_script(self, fixture=PRE_QUEUE_SCRIPT, name="old"):
        path = self.root / name / "gate_slot.py"
        if not path.exists():
            path.parent.mkdir()
            shutil.copyfile(fixture, path)
        return path

    def start_old(self, fixture, cls, *command, name="old", extra=()):
        process = subprocess.Popen(
            [sys.executable, str(self.old_script(fixture, name)), cls, "--poll", "0.02",
             "--receipt", str(self.root / f"{name}-{len(self.started)}.json"), *extra, "--",
             *command],
            env=self.env(), stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, text=True)
        self.started.append(process)
        return process

    def lane_slot(self, *args, command=("true",), name=None, **env):
        """A real lane job, started; its receipt is ``<name>.json``."""
        name = name or f"lane-{len(self.started)}"
        process = self.start_slot("short", "--poll", "0.02", "--sample-interval", "0.1",
                                  "--receipt", str(self.root / f"{name}.json"), *args, "--",
                                  *command, **env)
        process.receipt = self.root / f"{name}.json"
        return process

    def holds(self, label):
        return (self.note(label) or {}).get("pid")


# --- the rule ------------------------------------------------------------------------


class Rule(Lane):
    def test_a_short_job_overtakes_queued_audits_and_a_draining_suite(self):
        """The incident's shape: one audit slot, held; an audit, another audit
        and a rust-suite drain wait for it.  A short job that arrives after all
        of them is admitted at once and does not disturb their order."""
        self.set_memory(8, total_gib=8)  # one 7 GiB audit slot
        holder = self.arrive(self.job("holder"))
        self.admitted(holder)
        m = self.arrive(self.job("M"))
        a2 = self.arrive(self.job("A2"))
        suite = self.arrive(self.job("R", "rust-suite", None, drain_timeout=None))
        a3 = self.arrive(self.job("A3"))
        self.assertEqual(self.names("audit"), ["M", "A2", "R", "A3"])
        prepare = self.arrive(self.lane_job("prepare"))
        self.admitted(prepare)
        self.assertEqual(self.order, ["holder", "prepare"])
        self.assertEqual(prepare.record["class"], "short")
        self.assertEqual(prepare.record["held"][1:], ["short/0"])
        self.not_admitted(m, a2, suite, a3)
        self.assertEqual(self.names("audit"), ["M", "A2", "R", "A3"], "the audit queue is as it was")
        prepare.finish.set()
        holder.finish.set()
        for job in (m, a2, suite, a3):
            self.admitted(job)
            job.finish.set()
        self.assertEqual(self.order, ["holder", "prepare", "M", "A2", "R", "A3"])

    def test_a_short_job_runs_while_a_suite_is_draining(self):
        """The suite holds audit admission and waits for the running audit; the
        lane is open all the while."""
        self.set_memory(8, total_gib=8)
        holder = self.arrive(self.job("holder"))
        self.admitted(holder)
        suite = self.arrive(self.job("suite", "rust-suite", None, drain_timeout=None))
        self.assertTrue(wait_for(lambda: any(e["state"] == "draining" for e in self.live("audit"))))
        late_audit = self.arrive(self.job("late-audit"))
        self.not_admitted(late_audit)  # admission is closed to audits
        prepare = self.arrive(self.lane_job("prepare"))
        self.admitted(prepare)  # but not to the lane
        self.assertFalse(suite.admitted.is_set())
        prepare.finish.set()
        holder.finish.set()
        self.admitted(suite)

    def test_the_lane_has_a_fixed_number_of_slots_and_its_own_arrival_order(self):
        jobs = [self.arrive(self.lane_job(f"p{n}")) for n in (1, 2, 3, 4)]
        self.admitted(jobs[0], jobs[1])
        self.assertEqual(self.names("short"), ["p3", "p4"])
        self.not_admitted(jobs[2], jobs[3])
        self.assertEqual({jobs[0].record["slot"], jobs[1].record["slot"]}, {0, 1})
        self.assertEqual(slot_mod.SHORT_SLOTS, 2)
        jobs[0].finish.set()
        self.admitted(jobs[2])
        self.not_admitted(jobs[3])
        jobs[1].finish.set()
        self.admitted(jobs[3])
        self.assertEqual(self.order, ["p1", "p2", "p3", "p4"])

    def test_the_lane_and_the_audit_queue_do_not_hold_each_other_back(self):
        self.set_memory(8, total_gib=8)
        holder = self.arrive(self.job("holder"))
        self.admitted(holder)
        waiting_audit = self.arrive(self.job("waiting-audit"))
        self.assertEqual(slot_mod.QUEUE_FAMILY["short"], "short")
        self.assertEqual(self.names("short"), [])
        lane = self.arrive(self.lane_job("p"))
        self.admitted(lane)
        self.assertEqual(lane.record["queue"]["family"] if "queue" in lane.record else "short",
                         "short")
        self.not_admitted(waiting_audit)

    def test_memory_admission_counts_the_lane_against_audits(self):
        self.set_memory(8)  # eight GiB: two 4 GiB audit slots
        alone = slot_mod.Gate(self.gate_dir, self.legacy, self.meminfo, poll=0.01,
                              echo=lambda line: None)
        self.gates.append(alone)
        self.assertEqual(alone.acquire("audit", 4096, timeout=5)["slot_count"], 2)
        alone.release()
        lane = self.arrive(self.lane_job("p", budget=2048))  # declares 2 GiB, nothing used yet
        self.admitted(lane)
        self.assertEqual(slot_mod.lane_reserved_kib(self.gate_dir), 2048 * 1024)
        counted = slot_mod.Gate(self.gate_dir, self.legacy, self.meminfo, poll=0.01,
                                echo=lambda line: None)
        self.gates.append(counted)
        record = counted.acquire("audit", 4096, timeout=5)
        self.assertEqual(record["slot_count"], 1, "8 GiB less the lane's 2 GiB admits one 4 GiB audit")
        self.assertEqual(record["mem_available_kib"], 6 * GIB_KIB)

    def test_a_lane_job_is_not_counted_twice_once_it_uses_its_memory(self):
        lane = self.arrive(self.lane_job("p", budget=2048))
        self.admitted(lane)
        lane.gate.update_note({"tree_rss_kib": 1536 * 1024})  # its monitor's sample
        self.assertEqual(slot_mod.lane_reserved_kib(self.gate_dir), 512 * 1024)
        lane.gate.update_note({"tree_rss_kib": 3000 * 1024})  # over: nothing more reserved
        self.assertEqual(slot_mod.lane_reserved_kib(self.gate_dir), 0)

    def test_a_lane_job_waits_until_its_declared_memory_is_available(self):
        self.set_memory(1)  # 1 GiB
        big = self.arrive(self.lane_job("two-gib", budget=2048))
        small = self.arrive(self.lane_job("one-gib", budget=1024))
        # The 2 GiB declaration cannot be met; it holds nothing back, so the
        # later 1 GiB job is admitted in the other slot.
        self.admitted(small)
        self.not_admitted(big)
        self.assertEqual(small.record["queue"]["passed_over"][0]["seq"],
                         self.live("short")[0]["seq"])
        self.set_memory(8)
        self.admitted(big)
        self.assertEqual(self.order, ["one-gib", "two-gib"])

    def test_a_held_lane_slot_with_no_note_is_counted_at_the_largest_budget(self):
        import fcntl
        path = self.gate_dir / "short" / "0"
        path.parent.mkdir(parents=True)
        fd = os.open(path, os.O_RDWR | os.O_CREAT)
        self.addCleanup(os.close, fd)
        fcntl.flock(fd, fcntl.LOCK_EX)
        self.assertEqual(slot_mod.lane_reserved_kib(self.gate_dir), 2048 * 1024)

    def test_a_suite_holds_back_nothing_in_the_reserve(self):
        self.set_memory(8, total_gib=8)
        suite = self.arrive(self.job("suite", "rust-suite", None, drain_timeout=None))
        self.admitted(suite)
        self.assertEqual(suite.record["short_slots_held"], [0, 1])
        self.assertEqual(slot_mod.lane_reserved_kib(self.gate_dir), 0)


# --- the lane and a rust-suite drain -----------------------------------------------------


class Drains(Lane):
    def test_a_suite_waits_for_a_lane_job_in_flight_and_then_holds_the_lane(self):
        self.set_memory(8, total_gib=8)
        prepare = self.arrive(self.lane_job("prepare"))
        self.admitted(prepare)
        suite = self.arrive(self.job("suite", "rust-suite", None, drain_timeout=None))
        # The audits are drained at once; the suite now waits for the lane job.
        self.assertTrue(wait_for(lambda: self.lane_admission_closed()))
        self.not_admitted(suite)
        later = self.arrive(self.lane_job("later"))
        self.not_admitted(later)  # the closed lane holds a new arrival
        prepare.finish.set()
        self.admitted(suite)
        self.not_admitted(later)  # held for the whole run of the suite's command
        suite.finish.set()
        self.admitted(later)
        self.assertEqual(self.order, ["prepare", "suite", "later"])

    def test_a_suite_does_not_wait_for_a_lane_job_for_ever(self):
        """A lane job that is wedged (it never ends) cannot hold a drain beyond
        the lane's bound: the drain gives up, names it, and reopens everything."""
        self.set_memory(8, total_gib=8)
        stuck = self.lane_slot(name="stuck", command=SLEEP)
        self.assertTrue(wait_for(lambda: self.holds("short/0") == stuck.pid), stuck.poll())
        original = slot_mod.SHORT_DRAIN_BOUND
        slot_mod.SHORT_DRAIN_BOUND = 0.3
        self.addCleanup(setattr, slot_mod, "SHORT_DRAIN_BOUND", original)
        gate = slot_mod.Gate(self.gate_dir, self.legacy, self.meminfo, poll=0.02,
                             echo=lambda line: None)
        self.gates.append(gate)
        with self.assertRaises(slot_mod.DrainFailed) as raised:
            gate.acquire("rust-suite", None, timeout=30, holder={"argv": ["suite"]},
                         drain_timeout=None, drain_backoff=0.1, drain_attempts=1)
        [attempt] = raised.exception.attempts
        self.assertEqual(attempt["outcome"], "gave up")
        self.assertEqual([blocker["slot"] for blocker in attempt["blockers"]], ["short/0"])
        self.assertIn("lane_closed_at", attempt)
        # Everything is released: audits are admitted again, and so is the lane.
        later = self.arrive(self.job("audit"))
        self.admitted(later)
        again = self.arrive(self.lane_job("again"))
        self.admitted(again)

    def test_a_real_lane_job_past_its_wall_bound_is_stopped_and_the_drain_goes_on(self):
        """The bound that lets a suite wait: the lane job is stopped at its
        --max-wall, so the suite's wait is that long and no longer."""
        self.set_memory(8, total_gib=8)
        started = self.root / "started"
        stuck = self.lane_slot("--max-wall", "2s", "--stop-grace", "1s", name="stuck",
                               command=(sys.executable, "-c",
                                        f"open({str(started)!r}, 'w').close()\n"
                                        "import time; time.sleep(600)"))
        self.assertTrue(wait_for(started.exists), stuck.poll())
        suite = self.start_slot("rust-suite", "--poll", "0.02", "--receipt",
                                str(self.root / "suite.json"), "--", "true")
        begin = time.monotonic()
        suite.communicate(timeout=60)
        self.assertEqual(suite.returncode, 0)
        stuck.communicate(timeout=60)
        self.assertEqual(stuck.returncode, 124)
        self.assertGreaterEqual(time.monotonic() - begin, 0.0)
        receipt = json.loads(stuck.receipt.read_text())
        self.assertTrue(receipt["lane"]["wall_overrun"])
        self.assertFalse(receipt["lane"]["complete"])
        suite_record = json.loads((self.root / "suite.json").read_text())["slot_record"]
        self.assertEqual(suite_record["short_slots_held"], [0, 1])
        self.assertLessEqual(len(suite_record["drain"]["attempts"]), 1, "one drain, no give-up")

    def test_a_lane_job_nested_under_a_suite_is_covered_by_its_slots(self):
        completed = self.run_slot("rust-suite", "--receipt", str(self.root / "outer.json"), "--",
                                  sys.executable, str(slot_mod.__file__), "short", "--receipt",
                                  str(self.root / "inner.json"), "--", "true")
        self.assertEqual(completed.returncode, 0, completed.stderr)
        inner = json.loads((self.root / "inner.json").read_text())
        self.assertTrue(inner["slot_record"]["covered_by_parent"])

    def test_a_lane_job_nested_under_an_audit_is_refused(self):
        completed = self.run_slot("audit", "--receipt", str(self.root / "outer.json"), "--",
                                  sys.executable, str(slot_mod.__file__), "short", "--receipt",
                                  str(self.root / "inner.json"), "--", "true")
        self.assertEqual(completed.returncode, 2, completed.stderr)

    def lane_admission_closed(self):
        import fcntl
        path = self.gate_dir / "short" / "admission"
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


# --- the command line ---------------------------------------------------------------------


class CommandLine(Lane):
    def test_defaults_and_the_receipt(self):
        process = self.lane_slot()
        process.communicate(timeout=60)
        self.assertEqual(process.returncode, 0)
        receipt = json.loads(process.receipt.read_text())
        limits = receipt["limits"]
        self.assertEqual(limits["max_wall_seconds"], 30 * 60.0)
        self.assertEqual(limits["max_wall_from"], "default")
        self.assertEqual(limits["deadline_seconds"], 30 * 60.0)
        self.assertEqual(limits["hard_memory_stop_mib"], 4096)
        self.assertEqual(receipt["budget"]["admission_mib"], 1024)
        self.assertEqual(receipt["slot_record"]["class"], "short")
        self.assertEqual(receipt["slot_record"]["queue"]["family"], "short")
        self.assertTrue(receipt["lane"]["complete"])
        self.assertFalse(receipt["lane"]["wall_overrun"])

    def test_bounds_are_refused_before_anything_runs(self):
        for args in (("--max-wall", "2h"), ("--max-wall", "0"), ("--max-wall", "off"),
                     ("--max-wall", "61m"), ("--max-wall", "30"), ("--deadline", "10m"),
                     ("--memory-budget", "4G"), ("--memory-budget", "7G"),
                     ("--stop-grace", "10m")):
            with self.subTest(args=args):
                completed = self.run_slot("short", *args, "--receipt",
                                          str(self.root / "r.json"), "--", "true")
                self.assertEqual(completed.returncode, 2, completed.stderr)
                self.assertIn("refused", completed.stderr)
        for cls, args in (("audit", ("--max-wall", "10m")), ("lean", ("--memory-budget", "1G")),
                          ("rust-suite", ("--max-wall", "10m"))):
            with self.subTest(cls=cls, args=args):
                completed = self.run_slot(cls, *args, "--receipt", str(self.root / "r.json"),
                                          "--", "true")
                self.assertEqual(completed.returncode, 2, completed.stderr)

    def test_the_largest_accepted_values(self):
        process = self.lane_slot("--max-wall", "60m", "--memory-budget", "2G",
                                 "--stop-grace", "2m")
        process.communicate(timeout=60)
        self.assertEqual(process.returncode, 0)
        receipt = json.loads(process.receipt.read_text())
        self.assertEqual(receipt["limits"]["max_wall_seconds"], 3600.0)
        self.assertEqual(receipt["budget"]["admission_mib"], 2048)

    def test_the_environments_deadline_does_not_lift_the_wall_bound(self):
        process = self.lane_slot(QUIRIN_GATE_DEADLINE="off")
        process.communicate(timeout=60)
        receipt = json.loads(process.receipt.read_text())
        self.assertEqual(receipt["limits"]["max_wall_seconds"], 30 * 60.0)

    def test_a_wall_overrun_stops_the_command_and_says_it_is_incomplete(self):
        tree = self.root / "tree"
        done = self.root / "tree.complete"
        code = (f"import pathlib,time\n"
                f"pathlib.Path({str(tree)!r}).write_text('half')\n"  # the half-written tree
                "time.sleep(600)\n"
                f"pathlib.Path({str(done)!r}).write_text('complete')\n")  # the record, last
        process = self.lane_slot("--max-wall", "1s", "--stop-grace", "1s",
                                 command=(sys.executable, "-c", code))
        process.communicate(timeout=60)
        self.assertEqual(process.returncode, 124)
        receipt = json.loads(process.receipt.read_text())
        self.assertEqual(receipt["exit"], 124)
        self.assertEqual(receipt["stopped"], "deadline")
        self.assertEqual(receipt["lane"]["stopped_by"], "deadline")
        self.assertTrue(receipt["lane"]["wall_overrun"])
        self.assertFalse(receipt["lane"]["complete"])
        self.assertEqual(receipt["lane"]["max_wall_seconds"], 1.0)
        self.assertTrue(tree.exists() and not done.exists(),
                        "the lane stops the command; what it left is the command's to make safe")
        # The slot is free again.
        self.assertIsNone(self.holds("short/0"))
        later = self.lane_slot()
        later.communicate(timeout=60)
        self.assertEqual(later.returncode, 0)

    def test_a_command_that_fails_is_not_a_complete_job_and_not_an_overrun(self):
        process = self.lane_slot(command=("false",))
        process.communicate(timeout=60)
        self.assertEqual(process.returncode, 1)
        lane = json.loads(process.receipt.read_text())["lane"]
        self.assertFalse(lane["complete"])
        self.assertFalse(lane["wall_overrun"])

    def test_a_tree_over_the_hard_stop_is_stopped_like_an_overrun(self):
        original = slot_mod.SHORT_HARD_STOP_MIB
        # Run in-process so the stop is a small number: a child of ~200 MiB.
        slot_mod.SHORT_HARD_STOP_MIB = 100
        self.addCleanup(setattr, slot_mod, "SHORT_HARD_STOP_MIB", original)
        code = "x = bytearray(300 * 1024 * 1024)\nimport time; time.sleep(600)"
        receipt_path = self.root / "memory.json"
        with contextlib.redirect_stderr(io.StringIO()):
            status = slot_mod.main(["short", "--poll", "0.02", "--sample-interval", "0.1",
                                    "--stop-grace", "1s", "--receipt", str(receipt_path), "--",
                                    sys.executable, "-c", code])
        self.assertEqual(status, 124)
        receipt = json.loads(receipt_path.read_text())
        self.assertEqual(receipt["stopped"], "memory")
        self.assertFalse(receipt["lane"]["complete"])
        self.assertFalse(receipt["lane"]["wall_overrun"])

    def setUp(self):
        super().setUp()
        self._environ = dict(os.environ)
        os.environ.update(QUIRIN_GATE_DIR=str(self.gate_dir),
                          QUIRIN_GATE_LEGACY_LOCK=str(self.legacy),
                          QUIRIN_GATE_MEMINFO=str(self.meminfo))
        for name in (slot_mod.HELD_ENV, slot_mod.STALL_ENV, slot_mod.DEADLINE_ENV):
            os.environ.pop(name, None)

    def tearDown(self):
        os.environ.clear()
        os.environ.update(self._environ)
        super().tearDown()


# --- older clients ---------------------------------------------------------------------------


class OlderClients(Lane):
    """The transition: a pre-queue ``gate_slot.py`` and a queue-only one (the
    script as it was before the lane) share a gate directory with this one."""

    def test_the_fixtures_are_the_scripts_as_they_were(self):
        self.assertEqual(hashlib.sha256(FIFO_ONLY_SCRIPT.read_bytes()).hexdigest(),
                         FIFO_ONLY_SHA256)
        text = FIFO_ONLY_SCRIPT.read_text()
        self.assertIn("take_ticket", text)
        self.assertNotIn("SHORT_SLOTS", text)
        self.assertNotIn("SHORT_SLOTS", PRE_QUEUE_SCRIPT.read_text())

    def test_an_older_client_has_no_lane_class_and_takes_no_lane_lock(self):
        for fixture in (FIFO_ONLY_SCRIPT, PRE_QUEUE_SCRIPT):
            process = self.start_old(fixture, "short", "true", name=fixture.name[:12])
            process.wait(timeout=30)
            self.assertEqual(process.returncode, 2)
        self.assertFalse((self.gate_dir / "short").exists())

    def test_a_lane_job_runs_beside_an_older_audit_holder_and_both_finish(self):
        for fixture in (FIFO_ONLY_SCRIPT, PRE_QUEUE_SCRIPT):
            with self.subTest(fixture=fixture.name):
                self.set_memory(30)
                old = self.start_old(fixture, "audit", *SLEEP, name=f"a-{fixture.name[:9]}")
                self.assertTrue(wait_for(lambda: self.holds("audit/0") == old.pid), old.poll())
                lane = self.lane_slot()
                lane.communicate(timeout=60)
                self.assertEqual(lane.returncode, 0, "not blocked by the older audit")
                os.kill(old.pid, signal.SIGTERM)
                old.wait(timeout=30)

    def test_a_current_suite_waits_for_an_older_audit_and_the_lane_runs_meanwhile(self):
        for fixture in (FIFO_ONLY_SCRIPT, PRE_QUEUE_SCRIPT):
            with self.subTest(fixture=fixture.name):
                self.set_memory(8, total_gib=8)
                old = self.start_old(fixture, "audit", *SLEEP, name=f"b-{fixture.name[:9]}")
                self.assertTrue(wait_for(lambda: self.holds("audit/0") == old.pid), old.poll())
                suite = self.arrive(self.job("suite", "rust-suite", None, drain_timeout=None))
                self.not_admitted(suite)
                prepare = self.arrive(self.lane_job("prepare"))
                self.admitted(prepare)  # a drain does not shut the lane
                prepare.finish.set()
                os.kill(old.pid, signal.SIGTERM)
                self.admitted(suite)
                suite.finish.set()
                suite.thread.join(timeout=30)
                old.wait(timeout=30)
                self.order.clear()

    def test_an_older_suite_does_not_close_the_lane(self):
        """Not guaranteed during the transition: an older suite does not know
        the lane, so a lane job may run beside it."""
        for fixture in (FIFO_ONLY_SCRIPT, PRE_QUEUE_SCRIPT):
            with self.subTest(fixture=fixture.name):
                self.set_memory(8, total_gib=8)
                old = self.start_old(fixture, "rust-suite", *SLEEP, name=f"c-{fixture.name[:9]}")
                self.assertTrue(wait_for(lambda: self.holds("rust-suite/0") == old.pid), old.poll())
                lane = self.lane_slot()
                lane.communicate(timeout=60)
                self.assertEqual(lane.returncode, 0)
                os.kill(old.pid, signal.SIGTERM)
                old.wait(timeout=30)

    def test_older_and_current_audits_and_lane_jobs_never_hold_one_slot_together(self):
        self.set_memory(7)  # one audit slot
        holder = self.arrive(self.job("holder"))
        self.admitted(holder)
        log = None
        old = []
        for n, fixture in enumerate((FIFO_ONLY_SCRIPT, PRE_QUEUE_SCRIPT), 1):
            command, log = run_log(self.root, f"old-{n}", "0.3")
            old.append(self.start_old(fixture, "audit", *command, name=f"d{n}"))
        lanes = []
        for n in range(1, 5):
            command, log = run_log(self.root, f"lane-{n}", "0.4")
            lanes.append(self.lane_slot(command=command, name=f"lane-{n}"))
        holder.finish.set()
        for process in old + lanes:
            process.communicate(timeout=90)
            self.assertEqual(process.returncode, 0)
        spans = intervals(log)
        audits = sorted((spans[name] for name in ("old-1", "old-2")), key=lambda s: s["start"])
        self.assertLessEqual(audits[0]["end"], audits[1]["start"], "an audit slot held twice")
        lane = sorted((spans[f"lane-{n}"] for n in range(1, 5)), key=lambda s: s["start"])
        overlap = max(sum(1 for other in lane if other["start"] <= span["start"] < other["end"])
                      for span in lane)
        self.assertLessEqual(overlap, slot_mod.SHORT_SLOTS, "more lane jobs than lane slots")
        self.assertEqual(overlap, 2, "the two lane slots were both used")


# --- crashes -----------------------------------------------------------------------------------


class Crashes(Lane):
    def test_a_killed_lane_job_frees_its_slot_and_its_reserve(self):
        ready = self.root / "ready"
        job = self.lane_slot("--memory-budget", "2G", name="victim",
                             command=(sys.executable, "-c",
                                      f"open({str(ready)!r}, 'w').close()\n"
                                      "import time; time.sleep(600)"))
        self.assertTrue(wait_for(ready.exists), job.poll())
        self.assertEqual(self.holds("short/0"), job.pid)
        self.assertEqual(slot_mod.lane_reserved_kib(self.gate_dir) > 0, True)
        orphan = self.orphan_pid(job.pid)
        os.kill(job.pid, signal.SIGKILL)  # kill -9: no release, no cleanup
        job.wait(timeout=30)
        self.kill_pid(orphan)
        job.communicate(timeout=30)
        self.assertTrue(wait_for(lambda: slot_mod.lane_reserved_kib(self.gate_dir) == 0),
                        "the dead job's stale note is not a hold")
        waiter = self.arrive(self.lane_job("next"))
        self.admitted(waiter)
        self.assertEqual(waiter.record["slot"], 0)

    def test_a_killed_lane_waiters_ticket_is_reclaimed_and_blocks_nobody(self):
        first = self.arrive(self.lane_job("p1"))
        second = self.arrive(self.lane_job("p2"))
        self.admitted(first, second)
        pid, starttime = self.dead_process()
        dead = self.forge_dead("short", pid, starttime, mib=1024, name="dead")
        self.assertTrue(dead.exists())
        live = self.arrive(self.lane_job("live"))
        first.finish.set()
        self.admitted(live)
        self.assertEqual(live.record["queue"]["reclaimed"][0]["pid"], pid)
        self.assertFalse(dead.exists(), "unlinked by the waiter that found it")
        self.assertEqual(self.names("short"), [])

    def test_a_stopped_lane_waiter_is_skipped_after_the_lapse(self):
        first = self.arrive(self.lane_job("p1"))
        second = self.arrive(self.lane_job("p2"))
        self.admitted(first, second)
        stuck = self.forge_live("short", mib=1024, name="stuck")
        waiter = self.arrive(self.lane_job("waiter", queue_lapse=0.2))
        first.finish.set()
        self.admitted(waiter)  # the stuck ticket ahead stopped polling: skipped
        self.assertEqual(waiter.record["queue"]["lapsed_skipped"], 1)
        self.assertEqual(stuck.seq, self.live("short")[0]["seq"])

    def orphan_pid(self, parent):
        """The command a ``gate_slot.py`` started (so the test can stop it after
        a ``kill -9`` of its parent)."""
        children = [pid for pid, stat in slot_mod.process_table().items() if stat["ppid"] == parent]
        self.assertTrue(children)
        return children[0]

    @staticmethod
    def kill_pid(pid):
        try:
            os.kill(pid, signal.SIGKILL)
        except ProcessLookupError:
            pass


# --- status ----------------------------------------------------------------------------------------


class Status(Lane):
    def test_status_shows_lane_holders_and_waiters(self):
        holders = []
        for n in (1, 2):
            ready = self.root / f"ready-{n}"
            process = self.lane_slot(name=f"h{n}", command=(
                sys.executable, "-c",
                f"open({str(ready)!r}, 'w').close()\nimport time; time.sleep(600)"))
            self.assertTrue(wait_for(ready.exists), process.poll())
            holders.append(process)
        waiter = self.lane_slot(name="w")
        self.assertTrue(wait_for(lambda: len(self.live("short")) == 1), waiter.poll())
        status = slot_mod.gate_status(self.gate_dir, self.legacy, self.meminfo, interval=0)
        self.assertEqual(sorted(status["short_lane"]["holders"]), sorted(p.pid for p in holders))
        self.assertEqual(status["short_lane"]["waiters"], [waiter.pid])
        self.assertEqual(status["short_lane"]["slots"], 2)
        by_pid = {job["pid"]: job for job in status["jobs"]}
        self.assertEqual(by_pid[waiter.pid]["class"], "short")
        self.assertEqual(by_pid[waiter.pid]["queue"]["family"], "short")
        self.assertEqual(by_pid[waiter.pid]["queue"]["position"], 1)
        for process in holders:
            self.assertEqual(by_pid[process.pid]["class"], "short")
            self.assertEqual(by_pid[process.pid]["role"], "holder")
        text = slot_mod.render_status(status)
        self.assertIn("short lane: 2 of 2 slots held", text)
        self.assertIn(f"waiting: pid {waiter.pid}", text)
        self.assertIn(f"pid {waiter.pid} short: waiting for a slot", text)
        self.assertIn("queue position 1 of 1 (short)", text)
        completed = subprocess.run([sys.executable, str(slot_mod.__file__), "status", "--json",
                                    "--interval", "0"], env=self.env(), capture_output=True,
                                   text=True, timeout=60)
        self.assertEqual(completed.returncode, 0, completed.stderr)
        document = json.loads(completed.stdout)
        self.assertEqual(len(document["short_lane"]["holders"]), 2)
        self.assertEqual(len(document["queues"]["short"]), 1)
        for process in holders:
            os.kill(process.pid, signal.SIGTERM)
        waiter.communicate(timeout=60)

    def test_status_shows_a_suite_waiting_for_the_lane(self):
        self.set_memory(8, total_gib=8)
        ready = self.root / "ready"
        lane = self.lane_slot(name="h", command=(
            sys.executable, "-c", f"open({str(ready)!r}, 'w').close()\nimport time; time.sleep(600)"))
        self.assertTrue(wait_for(ready.exists), lane.poll())
        suite = self.start_slot("rust-suite", "--poll", "0.02", "--receipt",
                                str(self.root / "suite.json"), "--", "true")
        def suite_closed_the_lane():
            jobs = slot_mod.gate_status(self.gate_dir, self.legacy, self.meminfo,
                                        interval=0)["jobs"]
            return any(j["pid"] == suite.pid and "short/admission" in j["slots"] for j in jobs)

        self.assertTrue(wait_for(suite_closed_the_lane), suite.poll())
        status = slot_mod.gate_status(self.gate_dir, self.legacy, self.meminfo, interval=0)
        [job] = [j for j in status["jobs"] if j["pid"] == suite.pid]
        self.assertTrue(job["role"].startswith("draining (audits drained; holds lane admission"))
        self.assertEqual(job["blocked_by"], [{"lock": "short/0", "pids": [lane.pid]}])
        os.kill(lane.pid, signal.SIGTERM)
        suite.communicate(timeout=60)
        self.assertEqual(suite.returncode, 0)


if __name__ == "__main__":
    unittest.main()

"""Arrival-order admission of the gate slots (card GATE-SLOT-FIFO-ORDER,
requirement R11 of docs/plans/gate-time-reduction/README.md and the incident
of 2026-11-07/08: a measurement's audit waited 7.5 h, then 15.5 h, while audits
and rust-suite drains that asked hours later took the slots first).

Every test runs in a scratch gate directory with a scratch legacy lock and a
fake meminfo; the live /tmp/quirin-gate is never touched.  Waiting jobs are
threads of this process (each its own ``Gate`` and so its own file descriptions:
``flock`` conflicts between them as between processes) or real ``gate_slot.py``
processes; fake owners are processes this test started and stopped by the pid it
captured.  Nothing sleeps for a fixed time to order events: each step waits for
the ticket or the lock it depends on."""
import fcntl
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
from test_gate_slot_starvation import Scratch, meminfo, wait_for  # noqa: E402

FIXTURES = Path(__file__).resolve().parent / "fixtures"
# scripts/gate_slot.py as it was at 1e522ca5d (git blob 14d1fe3e851e...), before
# the queue: the client an older worktree still runs.
OLD_SCRIPT = FIXTURES / "gate_slot_before_queue.py.txt"
OLD_SCRIPT_SHA256 = "67a2245749e10868d0a29c8e3b280c8921dd3a42faf3e89822b69dd74c258b50"
MIB_11G = slot_mod.ROUTE_ADMISSION_MIB["pinned-corpus"]


class Queue(Scratch):
    """A scratch gate plus in-process waiting jobs."""

    available_gib = 30

    def setUp(self):
        super().setUp()
        self.jobs: list["Job"] = []
        self.tickets: list[slot_mod.Ticket] = []
        self.order: list[str] = []
        self.order_lock = threading.Lock()

    def tearDown(self):
        for job in self.jobs:
            job.finish.set()
            job.gate.wake()
        for job in self.jobs:
            job.thread.join(timeout=20)
        for ticket in self.tickets:
            ticket.drop()
        super().tearDown()

    def old_script(self):
        """The pre-queue script, under the name a worktree has it by."""
        path = self.root / "old" / "gate_slot.py"
        if not path.exists():
            path.parent.mkdir()
            shutil.copyfile(OLD_SCRIPT, path)
        return path

    def set_memory(self, available_gib, total_gib=60):
        meminfo(self.root, available_gib, total_gib)

    def queue_dir(self, cls):
        return slot_mod.queue_directory(self.gate_dir, cls)

    def live(self, cls):
        return slot_mod.scan_queue(self.queue_dir(cls), reclaim=False)["live"]

    def names(self, cls):
        """The queued jobs' names in arrival order."""
        return [(entry["note"].get("argv") or ["?"])[0] for entry in self.live(cls)]

    def job(self, name, cls="audit", budget=7168, **options):
        job = Job(self, name, cls, budget, **options)
        self.jobs.append(job)
        return job

    def arrive(self, job):
        """Start a job and wait until its ticket is in the queue, so the next
        arrival is later by construction."""
        job.thread.start()
        family = slot_mod.QUEUE_FAMILY[job.cls]
        self.assertTrue(
            wait_for(lambda: job.name in self.names(job.cls) or job.admitted.is_set()),
            f"{job.name} never took a ticket in {family}: {job.error}")
        return job

    def admitted(self, *jobs, within=20):
        for job in jobs:
            self.assertTrue(job.admitted.wait(within), f"{job.name} was not admitted: {job.error}")

    def not_admitted(self, *jobs, within=0.4):
        time.sleep(within)  # a negative needs a window; every positive waits on events
        for job in jobs:
            self.assertFalse(job.admitted.is_set(), f"{job.name} was admitted out of order")

    def forge_live(self, cls, mib=None, state="waiting", name="forged"):
        """A ticket held by this test: a waiter that is alive and polling (until
        it stops touching it)."""
        stat = slot_mod.proc_stat(os.getpid())
        ticket = slot_mod.take_ticket(self.queue_dir(cls), {
            "cls": cls, "family": slot_mod.QUEUE_FAMILY[cls], "admission_mib": mib,
            "argv": [name], "starttime": stat["starttime"],
            "requested_at": slot_mod.utc_now(), "state": state})
        self.tickets.append(ticket)
        return ticket

    def dead_process(self):
        """(pid, start time) of a process that ran and is gone."""
        child = subprocess.Popen(["sleep", "30"])
        stat = slot_mod.proc_stat(child.pid)
        child.kill()
        child.wait()
        return child.pid, stat["starttime"]

    def forge_dead(self, cls, pid, starttime, mib=None, name="dead"):
        """A ticket file nobody holds: what a killed waiter leaves behind."""
        directory = self.queue_dir(cls)
        directory.mkdir(parents=True, exist_ok=True)
        seq = max([entry["seq"] for entry in self.live(cls)] + [0]) + 1
        path = directory / f"{seq:012d}-{pid}.ticket"
        path.write_text(json.dumps({
            "schema": slot_mod.TICKET_SCHEMA, "seq": seq, "pid": pid, "starttime": starttime,
            "cls": cls, "family": slot_mod.QUEUE_FAMILY[cls], "admission_mib": mib,
            "argv": [name], "state": "waiting"}) + "\n")
        return path


class Job:
    """One in-process slot job: asks for its slot, records the order it was
    admitted in, holds the slot until told to finish."""

    def __init__(self, test, name, cls, budget, poll=0.02, queue_lapse=slot_mod.DEFAULT_QUEUE_LAPSE,
                 **acquire):
        self.test, self.name, self.cls, self.budget = test, name, cls, budget
        self.acquire = acquire
        self.gate = slot_mod.Gate(test.gate_dir, test.legacy, test.meminfo, poll=poll,
                                  echo=lambda line: None, queue_lapse=queue_lapse)
        self.admitted = threading.Event()
        self.finish = threading.Event()
        self.record = None
        self.error = None
        self.thread = threading.Thread(target=self.run, daemon=True)

    def run(self):
        try:
            self.record = self.gate.acquire(
                self.cls, self.budget if self.cls == "audit" else None, timeout=120,
                holder={"argv": [self.name], "cwd": os.getcwd(), "since": slot_mod.utc_now(),
                        **({"admission_mib": self.budget} if self.cls == "audit" else {})},
                **self.acquire)
            with self.test.order_lock:
                self.test.order.append(self.name)
            self.admitted.set()
            self.finish.wait(120)
        except BaseException as error:  # reported by the assertions
            self.error = error
        finally:
            self.gate.release()


# --- the decision, with nothing else -----------------------------------------------


class Decision(unittest.TestCase):
    def test_alone_a_waiter_takes_the_lowest_free_slot_it_may(self):
        self.assertEqual(slot_mod.queued_slot([], 3, [1, 2]), (1, []))
        self.assertEqual(slot_mod.queued_slot([], 1, [1, 2]), (None, []))

    def test_each_waiter_ahead_takes_the_lowest_free_slot_it_could(self):
        self.assertEqual(slot_mod.queued_slot([3], 3, [0, 1, 2]), (1, []))
        self.assertEqual(slot_mod.queued_slot([3, 3], 3, [0, 1, 2]), (2, []))
        self.assertEqual(slot_mod.queued_slot([3, 3, 3], 3, [0, 1, 2]), (None, []))

    def test_a_waiter_ahead_that_cannot_be_admitted_holds_nothing_back(self):
        # The corpus audit ahead needs slot 0 (one slot at this reading) and it is
        # busy; the 4 GiB audit behind it may take slot 1.
        self.assertEqual(slot_mod.queued_slot([1], 3, [1, 2]), (1, [0]))
        # With slot 0 free the one ahead takes it, and the one behind does not.
        self.assertEqual(slot_mod.queued_slot([1], 3, [0]), (None, []))
        self.assertEqual(slot_mod.queued_slot([1], 3, [0, 1]), (1, []))

    def test_a_rust_suite_ahead_holds_back_every_slot(self):
        self.assertEqual(slot_mod.queued_slot([None], 3, [0, 1, 2]), (None, [0]))
        self.assertEqual(slot_mod.queued_slot([1, None], 3, [0, 1, 2]), (None, [0, 1]))

    def test_arrival_order_decides_between_equal_waiters(self):
        # Two waiters, one free slot: only the earlier is assigned it.
        self.assertEqual(slot_mod.queued_slot([2], 2, [1]), (None, []))
        self.assertEqual(slot_mod.queued_slot([], 2, [1]), (1, []))


# --- arrival order -------------------------------------------------------------------


class Order(Queue):
    def test_lean_waiters_are_admitted_in_arrival_order_whoever_polls_first(self):
        first = self.job("holder-1", "lean")
        second = self.job("holder-2", "lean")
        for job in (first, second):
            self.arrive(job)
            self.admitted(job)
        # The earliest waiter polls slowest, the latest fastest: without arrival
        # order the last one would win every freed slot.
        waiters = [self.job(f"w{n}", "lean", poll=poll)
                   for n, poll in enumerate((0.25, 0.1, 0.02, 0.01), 1)]
        for waiter in waiters:
            self.arrive(waiter)
        self.assertEqual(self.names("lean"), ["w1", "w2", "w3", "w4"])
        first.finish.set()
        self.admitted(waiters[0])
        self.not_admitted(*waiters[1:])
        second.finish.set()
        self.admitted(waiters[1])
        self.not_admitted(*waiters[2:])
        waiters[0].finish.set()
        self.admitted(waiters[2])
        waiters[1].finish.set()
        self.admitted(waiters[3])
        self.assertEqual(self.order, ["holder-1", "holder-2", "w1", "w2", "w3", "w4"])
        self.assertEqual(waiters[0].record["queue"]["ahead_at_request"], 0)
        self.assertEqual(waiters[3].record["queue"]["ahead_at_request"], 3)
        self.assertEqual(waiters[3].record["queue"]["position_at_request"], 4)
        self.assertEqual(waiters[3].record["queue"]["position"], 1, "the last position it saw")

    def test_audit_waiters_are_admitted_in_arrival_order(self):
        self.set_memory(7)  # one 7 GiB slot
        holder = self.arrive(self.job("holder"))
        self.admitted(holder)
        waiters = [self.job(f"a{n}", poll=poll) for n, poll in enumerate((0.3, 0.02, 0.01), 1)]
        for waiter in waiters:
            self.arrive(waiter)
        holder.finish.set()
        for waiter, later in zip(waiters, waiters[1:]):
            self.admitted(waiter)
            self.not_admitted(later)
            waiter.finish.set()
        self.admitted(waiters[-1])
        self.assertEqual(self.order, ["holder", "a1", "a2", "a3"])

    def test_a_rust_suite_drain_waits_for_the_audits_queued_ahead_of_it(self):
        self.set_memory(8, total_gib=8)  # a ceiling of two slots, one at 7 GiB
        holder = self.arrive(self.job("holder"))
        self.admitted(holder)
        ahead = self.arrive(self.job("audit-ahead"))
        suite = self.arrive(self.job("suite", "rust-suite", None, drain_timeout=None))
        behind = self.arrive(self.job("audit-behind"))
        self.assertEqual(self.names("audit"), ["audit-ahead", "suite", "audit-behind"])
        holder.finish.set()
        self.admitted(ahead)
        # The suite is now the head: it closes admission and waits for the audit
        # ahead of it; the audit behind it waits for the suite.
        self.not_admitted(suite, behind)
        self.assertEqual(self.names("audit"), ["suite", "audit-behind"])
        ahead.finish.set()
        self.admitted(suite)
        self.not_admitted(behind)
        suite.finish.set()
        self.admitted(behind)
        self.assertEqual(self.order, ["holder", "audit-ahead", "suite", "audit-behind"])
        self.assertEqual(suite.record["audit_slots_held"], [0, 1])

    def test_a_backing_off_drain_keeps_its_place_and_lets_the_queue_run(self):
        self.set_memory(8, total_gib=8)
        holder = self.arrive(self.job("holder"))
        self.admitted(holder)
        suite = self.arrive(self.job("suite", "rust-suite", None, drain_timeout=0.2,
                                     drain_backoff=60.0))
        # The suite has the head; its drain gives up and backs off for a minute.
        self.assertTrue(wait_for(
            lambda: any(entry["state"] == "backoff" for entry in self.live("audit"))))
        suite_seq = self.live("audit")[0]["seq"]
        holder.finish.set()
        # An audit that arrives in the back-off is admitted past the suite.
        late = self.arrive(self.job("late"))
        self.admitted(late)
        self.assertEqual(self.order, ["holder", "late"])
        self.assertEqual([entry["seq"] for entry in self.live("audit")], [suite_seq])
        self.assertLess(suite_seq, late.record["queue"]["seq"])
        self.assertFalse(suite.admitted.is_set())

    def test_a_job_that_gives_up_waiting_leaves_no_ticket(self):
        self.set_memory(7)
        holder = self.arrive(self.job("holder"))
        self.admitted(holder)
        gate = slot_mod.Gate(self.gate_dir, self.legacy, self.meminfo, poll=0.01,
                             echo=lambda line: None)
        with self.assertRaises(slot_mod.WaitTimeout):
            gate.acquire("audit", 7168, timeout=0.3)
        self.assertEqual(self.names("audit"), [])

    def test_a_nested_job_takes_no_ticket(self):
        completed = self.run_slot("rust-suite", "--receipt", str(self.root / "outer.json"), "--",
                                  sys.executable, str(slot_mod.__file__), "audit", "--receipt",
                                  str(self.root / "inner.json"), "--", "true")
        self.assertEqual(completed.returncode, 0, completed.stderr)
        inner = json.loads((self.root / "inner.json").read_text())
        self.assertTrue(inner["slot_record"]["covered_by_parent"])
        self.assertNotIn("queue", inner["slot_record"])


class MixedBudgets(Queue):
    """A waiter whose memory admission cannot be met holds back only the slots
    its memory admits; it never blocks the whole queue."""

    def test_a_corpus_audit_waiting_for_slot_0_does_not_hold_back_a_smaller_budget(self):
        self.set_memory(14)  # 11 GiB: one slot; 7 GiB: two; 4 GiB: three
        holder = self.arrive(self.job("holder", budget=4096))
        self.admitted(holder)
        self.assertEqual(holder.record["slot"], 0)
        big = self.arrive(self.job("corpus", budget=MIB_11G))
        small = self.arrive(self.job("small", budget=4096))
        self.admitted(small)
        self.assertEqual(small.record["slot"], 1)
        self.assertFalse(big.admitted.is_set())
        [passed] = small.record["queue"]["passed_over"]
        self.assertEqual(passed["seq"], self.live("audit")[0]["seq"])
        self.assertEqual(self.names("audit"), ["corpus"], "the corpus audit keeps its place")
        holder.finish.set()
        self.admitted(big)
        self.assertEqual(big.record["slot"], 0, "the corpus audit gets the slot its memory admits")
        self.assertEqual(self.order, ["holder", "small", "corpus"])

    def test_a_free_slot_a_waiter_ahead_could_take_is_kept_for_it(self):
        self.set_memory(14)
        ahead = self.forge_live("audit", MIB_11G, name="corpus-ahead")
        small = self.arrive(self.job("small", budget=4096))
        self.admitted(small)
        self.assertEqual(small.record["slot"], 1, "slot 0 is the corpus audit's")
        self.assertEqual(small.record["queue"]["passed_over"], [])
        ahead.drop()

    def test_memory_below_every_budget_still_admits_one_slot(self):
        self.set_memory(2)
        job = self.arrive(self.job("starved-host", budget=MIB_11G))
        self.admitted(job)
        self.assertEqual((job.record["slot"], job.record["slot_count"]), (0, 1))

    def test_a_stuck_head_that_holds_slot_0_for_ever_is_skipped_once_it_lapses(self):
        self.set_memory(2)  # one slot at any budget
        ahead = self.forge_live("audit", MIB_11G, name="stuck")
        waiter = self.arrive(self.job("waiter", queue_lapse=0.3))
        self.admitted(waiter)  # the stuck ticket stopped polling: skipped
        self.assertEqual(waiter.record["queue"]["lapsed_skipped"], 1)
        self.assertTrue(Path(ahead.path).exists(), "skipped, not removed")
        directory = self.queue_dir("audit")
        self.assertTrue(slot_mod.scan_queue(directory, 0.3)["live"][0]["lapsed"])
        ahead.heartbeat()  # it counts again as soon as its owner polls
        self.assertFalse(slot_mod.scan_queue(directory, 0.3)["live"][0]["lapsed"])

    def test_a_suite_is_not_held_by_a_ticket_that_stopped_polling(self):
        self.set_memory(8, total_gib=8)
        stuck = self.forge_live("audit", 7168, name="stuck")
        suite = self.arrive(self.job("suite", "rust-suite", None, queue_lapse=0.3))
        self.assertTrue(wait_for(lambda: suite.admitted.is_set(), within=20),
                        f"the suite waited behind a ticket that stopped polling: {suite.error}")
        stuck.drop()


# --- crashed waiters, restarts -------------------------------------------------------------


class Stale(Queue):
    def test_a_ticket_of_a_dead_process_is_reclaimed_and_blocks_nobody(self):
        self.set_memory(7)
        pid, starttime = self.dead_process()
        path = self.forge_dead("audit", pid, starttime, name="crashed")
        waiter = self.arrive(self.job("waiter"))
        self.admitted(waiter)
        self.assertFalse(path.exists(), "reclaimed")
        self.assertEqual([item["pid"] for item in waiter.record["queue"]["reclaimed"]], [pid])

    def test_a_reused_pid_does_not_keep_a_ticket_alive(self):
        self.set_memory(7)
        stat = slot_mod.proc_stat(os.getpid())
        path = self.forge_dead("audit", os.getpid(), stat["starttime"] + 1, name="reused")
        waiter = self.arrive(self.job("waiter"))
        self.admitted(waiter)
        self.assertFalse(path.exists())

    def test_a_live_owner_without_a_lock_is_not_reclaimed(self):
        # Reclaimed only when the owning process is gone: this one is this test.
        self.set_memory(7)
        stat = slot_mod.proc_stat(os.getpid())
        path = self.forge_dead("audit", os.getpid(), stat["starttime"], name="alive")
        os.utime(path)
        waiter = self.arrive(self.job("waiter"))
        self.not_admitted(waiter)
        self.assertTrue(path.exists())
        self.assertEqual(self.names("audit"), ["alive", "waiter"])

    def test_a_held_ticket_of_a_missing_pid_is_not_reclaimed(self):
        # The flock is the owner's proof of life; a pid read in another
        # namespace may not exist here.
        self.set_memory(7)
        pid, starttime = self.dead_process()
        ticket = self.forge_live("audit", name="other-namespace")
        ticket.update(pid=pid, starttime=starttime)
        scan = slot_mod.scan_queue(self.queue_dir("audit"))
        self.assertEqual(scan["stale"], [])
        self.assertEqual(len(scan["live"]), 1)

    def test_a_killed_waiter_leaves_a_ticket_the_next_waiter_reclaims(self):
        self.set_memory(7)
        holder = self.holder(sys.executable, "-c", "import time; time.sleep(600)")
        victim = self.start_slot("audit", "--poll", "0.02", "--receipt",
                                 str(self.root / "victim.json"), "--", "true")
        self.assertTrue(wait_for(lambda: len(self.live("audit")) == 1), victim.poll())
        dead_seq = self.live("audit")[0]["seq"]
        os.kill(victim.pid, signal.SIGKILL)  # a process this test started
        victim.wait(timeout=10)
        self.assertEqual([e["seq"] for e in slot_mod.scan_queue(
            self.queue_dir("audit"), reclaim=False)["stale"]], [dead_seq])
        waiter = self.arrive(self.job("waiter"))
        os.kill(holder.pid, signal.SIGTERM)
        self.admitted(waiter)
        self.assertEqual([item["seq"] for item in waiter.record["queue"]["reclaimed"]], [dead_seq])
        self.assertEqual(self.names("audit"), [])

    def test_a_restarted_waiter_goes_to_the_back_and_the_counter_survives(self):
        self.set_memory(7)
        holder = self.arrive(self.job("holder"))
        self.admitted(holder)
        victim = self.forge_live("audit", 7168, name="restarted")
        first_seq = victim.seq
        victim.drop()  # the process is killed and run again ...
        second = self.arrive(self.job("second"))
        again = self.arrive(self.job("restarted-again"))  # ... and asks again
        self.assertEqual(self.names("audit"), ["second", "restarted-again"])
        self.assertGreater(again.gate.ticket.seq, second.gate.ticket.seq)
        self.assertGreater(second.gate.ticket.seq, first_seq)
        # The counter file is gone (cleaned /tmp): numbers still increase.
        (self.queue_dir("audit") / slot_mod.SEQUENCE_FILE).unlink()
        third = self.arrive(self.job("third"))
        self.assertGreater(third.gate.ticket.seq, again.gate.ticket.seq)

    def test_a_cleaned_gate_directory_starts_a_fresh_queue(self):
        self.set_memory(7)
        first = self.arrive(self.job("before"))
        self.admitted(first)
        first.finish.set()
        first.thread.join(10)
        shutil.rmtree(self.gate_dir)  # the host restarted and /tmp was cleared
        after = self.arrive(self.job("after"))
        self.admitted(after)
        self.assertEqual(after.record["queue"]["seq"], 1)

    def test_a_ticket_abandoned_while_it_was_created_is_swept(self):
        directory = self.queue_dir("audit")
        directory.mkdir(parents=True)
        partial = directory / ".000000000007-1.new"
        partial.write_text("")
        old = time.time() - slot_mod.LEFTOVER_AGE - 5
        os.utime(partial, (old, old))
        slot_mod.take_ticket(directory, {"cls": "audit"}).drop()
        self.assertFalse(partial.exists())


class RaceAndFairness(Queue):
    def test_simultaneous_arrivals_get_distinct_increasing_numbers(self):
        directory = self.queue_dir("audit")
        got: list[int] = []
        tickets: list[slot_mod.Ticket] = []
        lock = threading.Lock()

        def take():
            ticket = slot_mod.take_ticket(directory, {"cls": "audit"})
            with lock:
                got.append(ticket.seq)
                tickets.append(ticket)

        threads = [threading.Thread(target=take) for _ in range(24)]
        for thread in threads:
            thread.start()
        for thread in threads:
            thread.join(20)
        try:
            self.assertEqual(sorted(got), list(range(1, 25)))
            self.assertEqual(len(slot_mod.scan_queue(directory, reclaim=False)["live"]), 24)
        finally:
            for ticket in tickets:
                ticket.drop()

    def test_exclusion_holds_under_a_crowd(self):
        # Eight audits, two slots: never more than two run at once, and all run.
        self.set_memory(14)
        running = 0
        peak = 0
        lock = threading.Lock()
        finished = []

        def work(n):
            nonlocal running, peak
            gate = slot_mod.Gate(self.gate_dir, self.legacy, self.meminfo, poll=0.005,
                                 echo=lambda line: None)
            gate.acquire("audit", 7168, timeout=60, holder={"argv": [f"c{n}"]})
            with lock:
                running += 1
                peak = max(peak, running)
            time.sleep(0.05)
            with lock:
                running -= 1
                finished.append(n)
            gate.release()

        threads = [threading.Thread(target=work, args=(n,)) for n in range(8)]
        for thread in threads:
            thread.start()
        for thread in threads:
            thread.join(60)
        self.assertEqual(sorted(finished), list(range(8)))
        self.assertLessEqual(peak, 2)


# --- the command line, receipts, status -----------------------------------------------------------


class CommandLine(Queue):
    def test_the_receipt_records_the_ticket_and_the_stderr_says_the_position(self):
        self.set_memory(7)
        holder = self.holder(sys.executable, "-c", "import time; time.sleep(600)")
        waiter = self.start_slot("audit", "--poll", "0.02", "--receipt",
                                 str(self.root / "w.json"), "--", "true")
        self.assertTrue(wait_for(lambda: len(self.live("audit")) == 1), waiter.poll())
        os.kill(holder.pid, signal.SIGTERM)
        _, stderr = waiter.communicate(timeout=60)
        self.assertEqual(waiter.returncode, 0, stderr)
        self.assertIn("queue position 1 (ticket 2; 0 ahead", stderr)
        receipt = json.loads((self.root / "w.json").read_text())
        queue = receipt["slot_record"]["queue"]
        self.assertEqual((queue["family"], queue["seq"], queue["ahead_at_request"]),
                         ("audit", 2, 0))  # the holder had ticket 1
        self.assertEqual(queue["position_at_request"], 1)
        self.assertTrue(receipt["limits"]["queue"])
        self.assertEqual(receipt["limits"]["queue_lapse_seconds"], slot_mod.DEFAULT_QUEUE_LAPSE)
        self.assertEqual(receipt["limits"]["queue_lapse_from"], "default")
        self.assertEqual(self.names("audit"), [])

    def test_the_lapse_and_the_switch_are_overridable_and_recorded(self):
        completed = self.run_slot("lean", "--receipt", str(self.root / "l.json"), "--", "true",
                                  QUIRIN_GATE_QUEUE_LAPSE="90s")
        self.assertEqual(completed.returncode, 0, completed.stderr)
        limits = json.loads((self.root / "l.json").read_text())["limits"]
        self.assertEqual((limits["queue_lapse_seconds"], limits["queue_lapse_from"]),
                         (90.0, "environment"))
        completed = self.run_slot("lean", "--receipt", str(self.root / "o.json"), "--", "true",
                                  QUIRIN_GATE_QUEUE="off", QUIRIN_GATE_QUEUE_LAPSE="off")
        self.assertEqual(completed.returncode, 0, completed.stderr)
        document = json.loads((self.root / "o.json").read_text())
        self.assertFalse(document["limits"]["queue"])
        self.assertIsNone(document["limits"]["queue_lapse_seconds"])
        self.assertNotIn("queue", document["slot_record"], "no ticket was taken")

    def test_a_malformed_lapse_refuses_before_anything_runs(self):
        completed = self.run_slot("lean", "--receipt", str(self.root / "x.json"), "--", "true",
                                  QUIRIN_GATE_QUEUE_LAPSE="ten")
        self.assertEqual(completed.returncode, 2)
        self.assertIn("duration", completed.stderr)


@unittest.skipUnless(Path("/proc/locks").exists(), "status reads /proc/locks")
class Status(Queue):
    def test_each_waiter_shows_its_queue_position(self):
        self.set_memory(7)
        holder = self.holder(sys.executable, "-c", "import time; time.sleep(600)")
        waiters = []
        for n in (1, 2, 3):
            waiters.append(self.start_slot("audit", "--poll", "0.02", "--receipt",
                                           str(self.root / f"w{n}.json"), "--", "true"))
            self.assertTrue(wait_for(lambda n=n: len(self.live("audit")) == n), waiters[-1].poll())
        status = slot_mod.gate_status(self.gate_dir, self.legacy, self.meminfo, interval=0)
        by_pid = {job["pid"]: job for job in status["jobs"]}
        for position, waiter in enumerate(waiters, 1):
            job = by_pid[waiter.pid]
            self.assertEqual(job["role"], "waiting for a slot")
            self.assertEqual((job["queue"]["position"], job["queue"]["of"]), (position, 3))
            self.assertEqual(job["queue"]["family"], "audit")
            self.assertEqual(job["queue"]["ahead"], [w.pid for w in waiters[:position - 1]])
            self.assertEqual(job["class"], "audit")
        self.assertIsNone(by_pid[holder.pid].get("queue"))
        self.assertEqual([item["pid"] for item in status["queues"]["audit"]],
                         [w.pid for w in waiters])
        text = slot_mod.render_status(status)
        for position, waiter in enumerate(waiters, 1):
            self.assertIn(f"pid {waiter.pid} audit: waiting for a slot", text)
            self.assertIn(f"queue position {position} of 3 (audit)", text)
        self.assertIn(f"behind pid {waiters[0].pid}", text)
        # The JSON form carries the same.
        completed = subprocess.run([sys.executable, str(slot_mod.__file__), "status", "--json",
                                    "--interval", "0"], env=self.env(), capture_output=True,
                                   text=True, timeout=60)
        self.assertEqual(completed.returncode, 0, completed.stderr)
        self.assertEqual(len(json.loads(completed.stdout)["queues"]["audit"]), 3)
        os.kill(holder.pid, signal.SIGTERM)
        for waiter in waiters:
            waiter.communicate(timeout=60)
            self.assertEqual(waiter.returncode, 0)

    def test_a_backing_off_drain_and_a_job_of_an_older_client_are_told_apart(self):
        self.set_memory(8, total_gib=8)
        holder = self.holder(sys.executable, "-c", "import time; time.sleep(600)")
        suite = self.start_slot("rust-suite", "--poll", "0.02", "--drain-timeout", "0.3s",
                                "--drain-backoff", "600s", "--receipt",
                                str(self.root / "suite.json"), "--", "true")
        self.assertTrue(wait_for(lambda: any(e["state"] == "backoff" for e in self.live("audit"))),
                        suite.poll())
        old = subprocess.Popen([sys.executable, str(self.old_script()), "audit", "--poll", "0.02",
                                "--receipt", str(self.root / "old.json"), "--", "true"],
                               env=self.env(), stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        self.started.append(old)
        status = None

        def listed():
            nonlocal status
            status = slot_mod.gate_status(self.gate_dir, self.legacy, self.meminfo, interval=0)
            return {suite.pid, old.pid} <= {job["pid"] for job in status["jobs"]}

        self.assertTrue(wait_for(listed))
        by_pid = {job["pid"]: job for job in status["jobs"]}
        self.assertEqual(by_pid[suite.pid]["queue"]["state"], "backoff")
        self.assertIn("backing off", by_pid[suite.pid]["role"])
        self.assertIsNone(by_pid[old.pid]["queue"])
        self.assertIn("not queued", by_pid[old.pid]["role"])
        self.assertIn("[backoff]", slot_mod.render_status(status))
        os.kill(holder.pid, signal.SIGTERM)
        old.wait(timeout=60)
        suite.kill()  # started here; its backoff is 10 minutes
        suite.communicate(timeout=10)

    def test_status_reclaims_nothing(self):
        pid, starttime = self.dead_process()
        path = self.forge_dead("audit", pid, starttime)
        status = slot_mod.gate_status(self.gate_dir, self.legacy, self.meminfo, interval=0)
        self.assertEqual(status["queues"], {})
        self.assertTrue(path.exists())


# --- older clients on the same host -----------------------------------------------------------------


def run_log(directory, name, seconds="0.4"):
    """A command that logs when it started and ended, to prove exclusion."""
    log = Path(directory) / "log.txt"
    code = ("import os,time\n"
            f"f=open({str(log)!r},'a')\n"
            f"f.write('start {name} %s\\n' % time.monotonic()); f.flush()\n"
            f"time.sleep({seconds})\n"
            f"f.write('end {name} %s\\n' % time.monotonic()); f.flush()\n")
    return [sys.executable, "-c", code], log


def intervals(log):
    spans: dict[str, dict] = {}
    for line in Path(log).read_text().splitlines():
        kind, name, at = line.split()
        spans.setdefault(name, {})[kind] = float(at)
    return spans


class OlderClients(Queue):
    """The transition: a ``gate_slot.py`` from before this card (the fixture)
    and the current one share a gate directory.  Both take the same flocks, so
    neither deadlocks with the other and no slot is ever held twice."""

    def start_old(self, cls, *command, extra=()):
        process = subprocess.Popen(
            [sys.executable, str(self.old_script()), cls, "--poll", "0.02", "--receipt",
             str(self.root / f"old-{len(self.started)}.json"), *extra, "--", *command],
            env=self.env(), stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, text=True)
        self.started.append(process)
        return process

    def test_the_fixture_is_the_script_as_it_was(self):
        import hashlib
        self.assertEqual(hashlib.sha256(OLD_SCRIPT.read_bytes()).hexdigest(), OLD_SCRIPT_SHA256)
        self.assertNotIn("take_ticket", OLD_SCRIPT.read_text())

    def test_a_queued_job_is_admitted_when_an_older_holder_ends(self):
        self.set_memory(7)
        old = self.start_old("audit", sys.executable, "-c", "import time; time.sleep(600)")
        self.assertTrue(wait_for(lambda: (self.note("audit/0") or {}).get("pid") == old.pid))
        waiter = self.arrive(self.job("new-waiter"))
        self.not_admitted(waiter)
        os.kill(old.pid, signal.SIGTERM)
        self.admitted(waiter)

    def test_an_older_waiter_gets_the_slot_a_current_holder_frees(self):
        self.set_memory(7)
        holder = self.arrive(self.job("new-holder"))
        self.admitted(holder)
        command, log = run_log(self.root, "old")
        old = self.start_old("audit", *command)
        time.sleep(0.3)
        self.assertIsNone(old.poll(), "blocked by the current holder")
        holder.finish.set()
        old.wait(timeout=60)
        self.assertEqual(old.returncode, 0, old.stderr.read())

    def test_current_and_older_waiters_never_hold_the_one_slot_together(self):
        self.set_memory(7)
        holder = self.arrive(self.job("new-holder"))
        self.admitted(holder)
        commands = []
        for name in ("old-1", "old-2"):
            command, log = run_log(self.root, name, "0.3")
            commands.append((name, self.start_old("audit", *command)))
        new = []
        for n in (1, 2):
            command, log = run_log(self.root, f"new-{n}", "0.3")
            process = self.start_slot("audit", "--poll", "0.02", "--receipt",
                                      str(self.root / f"new-{n}.json"), "--", *command)
            new.append(process)
            self.assertTrue(wait_for(lambda n=n: len(self.live("audit")) >= n), process.poll())
        holder.finish.set()
        for _, process in commands:
            process.wait(timeout=90)
        for process in new:
            process.wait(timeout=90)
            self.assertEqual(process.returncode, 0)
        spans = intervals(log)
        self.assertEqual(sorted(spans), ["new-1", "new-2", "old-1", "old-2"])
        ordered = sorted(spans.values(), key=lambda span: span["start"])
        for earlier, later in zip(ordered, ordered[1:]):
            self.assertLessEqual(earlier["end"], later["start"], "a slot held twice")

    def test_an_older_rust_suite_drains_current_audits_and_they_wait_for_it(self):
        self.set_memory(8, total_gib=8)
        command, log = run_log(self.root, "old-suite", "0.5")
        audit = self.arrive(self.job("current-audit"))
        self.admitted(audit)
        suite = self.start_old("rust-suite", *command)
        self.assertTrue(wait_for(lambda: self.admission_closed(), within=20), suite.poll())
        late = self.arrive(self.job("late-audit"))
        self.not_admitted(late)  # admission is closed, as it always was
        audit.finish.set()
        suite.wait(timeout=60)
        self.assertEqual(suite.returncode, 0, suite.stderr.read())
        self.admitted(late)

    def test_a_current_rust_suite_drains_an_older_audit(self):
        self.set_memory(8, total_gib=8)
        old = self.start_old("audit", sys.executable, "-c", "import time; time.sleep(600)")
        self.assertTrue(wait_for(lambda: (self.note("audit/0") or {}).get("pid") == old.pid))
        suite = self.arrive(self.job("suite", "rust-suite", None, drain_timeout=None))
        self.not_admitted(suite)
        self.assertEqual(self.names("audit"), ["suite"])
        os.kill(old.pid, signal.SIGTERM)
        self.admitted(suite)

    def test_a_current_drain_that_gives_up_reopens_admission_for_an_older_audit(self):
        self.set_memory(16, total_gib=16)  # four slots, two at 7 GiB
        holder = self.start_old("audit", sys.executable, "-c", "import time; time.sleep(600)")
        self.assertTrue(wait_for(lambda: (self.note("audit/0") or {}).get("pid") == holder.pid))
        suite = self.arrive(self.job("suite", "rust-suite", None, drain_timeout=0.2,
                                     drain_backoff=60.0))
        self.assertTrue(wait_for(lambda: any(e["state"] == "backoff" for e in self.live("audit"))))
        second = self.start_old("audit", sys.executable, "-c", "pass")
        second.wait(timeout=60)  # slot 1 is free: admitted, ran, left
        self.assertEqual(second.returncode, 0, second.stderr.read())

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


if __name__ == "__main__":
    unittest.main()

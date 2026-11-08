#!/usr/bin/env python3
"""Run a heavy gate job in a named, numbered slot instead of one global lock.

Card GATE-ELASTIC (seam E of docs/plans/pluggable-cards/README.md; user
decision 5 of 2026-10-24).  Usage::

    gate_slot.py <class> [--memory-budget 7G|5G|4G] [--receipt PATH] [--wait-record PATH]
                 [--stall-timeout D] [--deadline D] [--progress-file PATH]
                 [--drain-timeout D] [--drain-backoff D] [--drain-attempts N] -- <cmd> [args...]
    gate_slot.py status [--json] [--interval SECONDS]

Slots are ``flock(2)`` locks on files ``<gate dir>/<class>/<n>`` (default gate
dir ``/tmp/quirin-gate``).  Classes:

``rust-suite``  one slot.  While it runs it also holds **every** ``audit``
                slot, so constraint 16's load-sensitive rows never share the
                host with an audit.  Use it for the root Rust suite and the
                provider workspace's ``cargo test``.
``audit``       ``max(1, floor(MemAvailable / budget))`` slots, computed when the
                job asks for a slot.  The budget is 7 GiB (7168 MiB) by default,
                or 5G / 4G; it is passed to the audit's own MiB planning budget
                (``--memory-budget-mib``, ``memory_budget_mib``; card
                AUDIT-IR-WORKING-SET), so a smaller budget lowers that run's own
                worker count instead of overrunning its slot.  A route whose
                measured peak exceeds that budget is admitted at its own
                measured budget instead (``ROUTE_ADMISSION_MIB``: the pinned
                corpus at 11 GiB) while its workers stay planned at the slot
                budget (card LANDING-PUBLISH-COMPLETION).
``lean``        two slots.

Transition interlock: every slot job holds the legacy lock
``/tmp/qcore-c-card-validation-20261022.lock`` **shared** for its whole run.
The legacy users run ``flock <legacy lock> <cmd>``, which takes it exclusive,
so a legacy job excludes every slot job and every slot job excludes a legacy
job, while slot jobs share the legacy lock with one another.

The command's exit code is this script's exit code (128 + N for a signal).
A receipt (class, slots held, budget, admission budget, memory reading, waits,
argv as run, exit, the child's peak resident set) is written to ``--receipt`` or ``private/gate-slot/<stamp>-<pid>-<class>.json``.
The receipt lands when the command ends; ``--wait-record PATH`` also keeps a
live record of the wait (``quirin-gate-wait/1``): ``{"state": "waiting"}`` from
the moment the script asks for its slot, then ``{"state": "admitted",
"waited_seconds": ...}`` once the slot is held and before the command starts,
each replaced whole by a rename, so a reader never sees a partial record.
``quirin-audit measure`` reads it to tell a route that waits for a slot from
one that runs (card MEASURE-QUOTA-AND-INCREMENTAL, requirement R7 of
docs/plans/gate-time-reduction/README.md).  A nested job covered by its
parent's slot is admitted at once, ``waited_seconds`` 0.
Exit 2 is a refusal before anything ran (bad class or budget, a budget the
command would override, a nested slot the parent does not cover, a malformed
duration).

Starvation bounds (card GATE-SLOT-ADMISSION-STARVATION, requirement R11 of
docs/plans/gate-time-reduction/README.md):

* **Deadlines.** While the command runs, this process samples the command's
  process tree (the child and every descendant, never itself) every 10 s.
  ``--stall-timeout D`` stops the command when, for ``D``, no process of that
  tree changed its ``/proc/<pid>/io`` counters (``rchar``, ``wchar``,
  ``syscr``, ``syscw``, ``read_bytes``, ``write_bytes``: output to a terminal,
  pipe or log is a ``write`` and counts), the tree gained or lost no process and
  the ``--progress-file`` (if given) changed neither size nor mtime.
  ``--deadline D`` stops it ``D`` after it started (slot wait excluded).
  Defaults: ``audit`` 2 h stall, no wall-clock deadline; ``rust-suite`` and
  ``lean`` neither.  ``0`` or ``off`` disables one; the variables
  ``QUIRIN_GATE_STALL_TIMEOUT`` and ``QUIRIN_GATE_DEADLINE`` apply when the
  flag is absent.  A sample whose I/O cannot be read counts as progress, so an
  unmeasurable tree is never stopped for stalling (the receipt says so;
  without ``/proc`` only the wall-clock deadline applies).  On a
  breach the receipt and stderr say why (stall or deadline, the last progress,
  the CPU spent since, the processes); SIGTERM goes to the child and to every
  descendant it started (each re-identified by its start time through a pidfd
  before it is signalled), SIGKILL to those left after ``--stop-grace`` (60 s),
  and the script exits 124 with the child's own status as ``child_exit``.
  Nothing outside the command's own tree is signalled.
* **Resident set.** Each sample sums the tree's resident set; an ``audit`` job
  whose sum exceeds its admission budget says so on stderr at once (again for
  every further GiB), and the receipt keeps the first crossing and the peak.
  The slot file's holder note carries a ``live`` record of the last sample.
* **Bounded drain.** A ``rust-suite`` job closes audit admission and waits at
  most ``--drain-timeout`` (default 2 h, variable
  ``QUIRIN_GATE_DRAIN_TIMEOUT``) for the running audits.  Then it reopens
  admission, releases every lock it took, names each blocking holder (pid,
  argv, how long it has held the slot, its CPU and I/O over the wait, its
  resident set against its admission budget), backs off for
  ``--drain-backoff`` (5 min) so queued audits are admitted, and tries again;
  ``--drain-attempts N`` (default 0, unlimited) makes the Nth such attempt a
  failure, exit 75, before the command ran.  The command never runs without
  the whole drain.
* **Status.** ``gate_slot.py status [--json] [--interval S]`` lists the live
  holders and waiters: the ``/proc/locks`` entries whose inode is a slot file
  under the gate directory or the legacy lock, with the holder notes, wait and
  hold times, the CPU and I/O of each job's tree over the interval, and its
  resident set against its admission budget.  A slot job waiting for an
  ``audit`` slot holds only the legacy lock, shared, so it shows as a waiter;
  a job waiting for the legacy lock shows in the kernel's queue.  Each queued
  waiter shows its queue position (below).

Arrival order (card GATE-SLOT-FIFO-ORDER):

* **Tickets.** A slot job that must wait takes a ticket in its class's queue
  directory, ``<gate dir>/<family>/queue/<seq>-<pid>.ticket`` (family ``audit``
  for ``audit`` and ``rust-suite``, because a ``rust-suite`` job takes every
  audit slot; family ``lean``), after it holds the legacy lock.  The sequence
  number comes from a counter file under a flock held for microseconds; the
  ticket is created locked (a flock the job keeps for its whole wait, closed
  on exit) and renamed into place, so a ticket is never visible unlocked.
  No daemon: the tickets are plain files beside the slot flocks.
* **Order.** A waiter may take a slot only if no waiter ahead of it (a smaller
  sequence number) can take that slot.  A waiter ahead holds back exactly the
  free slots it could itself take at the current memory reading (slot index
  below its own ``floor(MemAvailable / admission budget)``, at least one; ``lean``:
  both).  A waiter ahead that cannot be admitted now (every slot its memory
  admits is busy) therefore holds back nothing else: a 4 GiB audit behind an
  11 GiB corpus audit that waits for slot 0 runs in slot 1.  A ``rust-suite``
  job ahead holds back every audit behind it, and it starts its drain only when
  no waiter is ahead of it, so it respects the audits queued before it.
* **No queue blocked for ever.** A ticket stops holding the others back when
  its owner process is gone (the ticket is then unlinked by whoever finds it:
  its flock is free *and* its pid is gone or has another start time), when its
  owner has not polled for ``QUIRIN_GATE_QUEUE_LAPSE`` (default 10 min; 0/off
  disables; the ticket is skipped, not removed, and counts again when its owner
  polls), or while a ``rust-suite`` drain backs off after giving up (it keeps its
  place and resumes there; the audits queued behind it run meanwhile).
  ``QUIRIN_GATE_QUEUE=off`` takes no ticket: the job competes as before.
* **Older clients.** A ``gate_slot.py`` without this card ignores the queue; it
  takes the same slot flocks, so it can neither deadlock with a queued job nor
  hold a slot another job holds, and it may be admitted ahead of a queued
  waiter (order is guaranteed among queue-aware jobs only).

The short-job lane (card GATE-SLOT-SHORT-JOB-LANE; user decision of 2026-11-08,
"Yes, add a fast lane"): ``gate_slot.py short [--memory-budget 1G|2G]
[--max-wall D] -- <cmd>`` is for a CPU-bound, memory-light step that takes
minutes (``prepare``, a build, a link, a regeneration), not an analysis.

* **Where it sits.** ``short`` is a class of its own: ``SHORT_SLOTS`` (2) slots
  ``<gate dir>/short/<n>``, its own arrival-order queue (family ``short``), and
  it takes **no audit lock**: it is not queued behind audits, is not shut out by
  a ``rust-suite`` job that is draining, and does not wait for the audits a
  drain waits for.  It holds the legacy lock shared like every slot job.
* **Eligibility, enforced.** ``--max-wall`` (default 30 min, at most 1 h; it
  cannot be switched off; ``--deadline`` is refused for the class) is a wall
  bound the monitor enforces: past it the command is stopped (SIGTERM, then
  SIGKILL after ``--stop-grace``, at most 2 min) and the script exits 124 with
  ``lane.wall_overrun`` and ``lane.complete: false`` in the receipt.  It is
  stopped rather than only recorded because the lane's promise to a draining
  suite is that a lane job ends within its bound; a recorded overrun would be an
  unbounded drain.  The exit is non-zero, so no caller reads a stopped job as
  finished, but the lane does not clean up after the command: what it was
  writing is the command's to make safe (write the completion record last).
  ``--memory-budget`` is the memory the job declares (1G default, 2G), not
  applied to the command: an excess is recorded (``rss_over_admission``) and
  warned, and a process tree over ``SHORT_HARD_STOP_MIB`` (4 GiB, the smallest
  audit budget) is stopped like an overrun (``stopped: memory``): such a job is
  an audit.
* **Memory admission.** A lane job is admitted only when ``MemAvailable``, less
  the unspent declared memory of the lane jobs already running, covers its
  declaration.  An audit (and a lane waiter) takes the same reserve from its
  reading: for each lane slot held, its declared budget less the resident set of
  its last sample (``lane_reserved_kib``), so a lane job counts against audits
  before it has allocated, and not twice once it has.
* **Drains.** A ``rust-suite`` job drains the audits as before, with the lane
  open.  Once the audits are drained it closes the lane (``short/admission``,
  exclusive) and waits for the lane jobs in flight, which end within their wall
  bound plus one sample and the stop grace (``SHORT_DRAIN_BOUND``, a little
  over an hour at the extreme, normally minutes); it then holds the lane slots
  while its command runs, so its load-sensitive rows run alone, and a lane job
  that arrives meanwhile waits for the suite to end.  A lane job nested under
  the suite is covered by its slots.  A lane wait past the bound gives up as an
  audit drain does.
* **Older clients.** A script without the lane has no ``short`` class and takes
  no ``short`` lock: it cannot deadlock with a lane job or share a slot with
  one.  An older ``rust-suite`` job does not close the lane, so a lane job may
  run beside it (not guaranteed alone for load-sensitive rows until every
  worktree has this script), and an older audit does not subtract the lane's
  reserve (it sees ``MemAvailable`` as it is).
"""

from __future__ import annotations

import argparse
import datetime
import errno
import fcntl
import json
import os
from pathlib import Path
import re
import select
import signal
import subprocess
import sys
import threading
import time
from typing import Callable, Optional, Sequence

REPO_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_GATE_DIR = Path("/tmp/quirin-gate")
LEGACY_LOCK = Path("/tmp/qcore-c-card-validation-20261022.lock")
MEMINFO = Path("/proc/meminfo")

# Budgets in MiB (binary units, as the audit's planning budget counts them:
# its report's example "7168 MiB is a binary-unit planning budget").
BUDGETS_MIB = {"7G": 7168, "5G": 5120, "4G": 4096}
DEFAULT_BUDGET = "7G"
SMALLEST_BUDGET_MIB = min(BUDGETS_MIB.values())

# Mirror of crates/qcore-audit/src/parallel.rs `budgeted_jobs`: a 1024 MiB
# coordinator reserve plus 2048 MiB per worker, refused below 3072 MiB.
AUDIT_COORDINATOR_MIB = 1024
AUDIT_WORKER_MIB = 2048

# Routes admitted at a measured budget above the planning budget, in MiB.  The
# pinned corpus route (`run_pinned_audit.py`, `quirin-audit pinned`) peaked at
# 10,129,997,824 B process RSS (9,661 MiB) at --jobs 3 (CFG-RELEASE, BASE
# 710021bd7; ledger CORPUS-PEAK-ABOVE-AUDIT-SLOT-BUDGET), above the 7 GiB slot
# that plans its three workers.  It is admitted at 11 GiB, a documented measured
# exception to user decision 5's 7 GiB default; its workers stay planned at the
# slot budget.  The admission budget is never below the slot budget.
ROUTE_ADMISSION_MIB = {"pinned-corpus": 11264}

FIXED_SLOTS = {"rust-suite": 1, "lean": 2}
CLASSES = ("rust-suite", "audit", "lean", "short")
# Which classes a held slot already covers, for a nested invocation.  A
# rust-suite job holds the short lane too (below), so it covers it.
SHORT = "short"
COVERS = {"rust-suite": {"rust-suite", "audit", "short"}, "audit": {"audit"},
          "lean": {"lean"}, "short": {"short"}}
HELD_ENV = "QUIRIN_GATE_HELD"

# --- the short-job lane (card GATE-SLOT-SHORT-JOB-LANE; user decision of
# 2026-11-08: "Yes, add a fast lane") ---------------------------------------------
# A CPU-bound, memory-light step (prepare, build, link, regeneration) that takes
# minutes must not wait hours behind audits and rust-suite drains (the incident:
# `prepare clang-build --link`, 3 min 17 s and 832 MB, waited 5 h 8 min).  The
# lane is a class of its own: SHORT_SLOTS slots, its own arrival-order queue, no
# audit lock.  What keeps it honest: a wall bound that is enforced (the command
# is stopped, exit 124), a memory budget the job declares (1G or 2G), and a hard
# stop at the smallest audit budget, because past that the job is an audit.
SHORT_SLOTS = 2
SHORT_BUDGETS_MIB = {"1G": 1024, "2G": 2048}
DEFAULT_SHORT_BUDGET = "1G"
DEFAULT_SHORT_WALL = 30 * 60.0   # a few tens of minutes
SHORT_WALL_CAP = 60 * 60.0       # longer is not a short job: use the audit class
SHORT_GRACE_CAP = 120.0          # SIGTERM-to-SIGKILL for a lane job, at most
SHORT_HARD_STOP_MIB = 4096       # tree resident set that stops a lane job
# How long a rust-suite job waits, once the audits are drained, for the lane
# jobs in flight: their wall bound, one monitor sample (10 s), the stop grace,
# plus slack.  A longer wait means a lane job is wedged, and the drain gives up
# as it does for audits.
SHORT_DRAIN_BOUND = SHORT_WALL_CAP + 10.0 + SHORT_GRACE_CAP + 300.0

# --- starvation bounds (card GATE-SLOT-ADMISSION-STARVATION, R11) ---------------
# Defaults in seconds; None means no deadline.  The audit stall default is long
# on purpose: a measured pinned-corpus route runs about two hours with steady
# I/O (clang per unit, logs, checkpoints), and the observed runaway spun
# 10 h 18 min with zero I/O, so two hours without any I/O by any process of
# the tree is far outside a working run and still cuts that loss by eight
# hours.  The drain bound covers one such route started just before the drain.
DEFAULT_STALL_TIMEOUT = {"audit": 2 * 3600.0, "rust-suite": None, "lean": None, "short": None}
DEFAULT_DEADLINE = {"audit": None, "rust-suite": None, "lean": None,
                    "short": DEFAULT_SHORT_WALL}
DEFAULT_DRAIN_TIMEOUT = 2 * 3600.0
DEFAULT_DRAIN_BACKOFF = 5 * 60.0
DEFAULT_STOP_GRACE = 60.0
SAMPLE_INTERVAL = 10.0
STALL_ENV = "QUIRIN_GATE_STALL_TIMEOUT"
DEADLINE_ENV = "QUIRIN_GATE_DEADLINE"
DRAIN_ENV = "QUIRIN_GATE_DRAIN_TIMEOUT"
STOPPED_EXIT = 124  # as timeout(1): the command was stopped by its deadline
WAIT_FAILED_EXIT = 75  # EX_TEMPFAIL: no slot (wait timeout or drain attempts spent)

# --- arrival order (card GATE-SLOT-FIFO-ORDER) -----------------------------------
# A waiter's ticket lives in <gate dir>/<family>/queue/.  Both classes that use
# audit slots share one queue: a rust-suite job takes every audit slot, so its
# place among the audits queued before it is the whole point.
QUEUE_DIR = "queue"
QUEUE_FAMILY = {"audit": "audit", "rust-suite": "audit", "lean": "lean", "short": "short"}
QUEUE_ENV = "QUIRIN_GATE_QUEUE"
LAPSE_ENV = "QUIRIN_GATE_QUEUE_LAPSE"
# A queued job polls every 2 s.  Ten minutes without a poll is a stopped or
# wedged process, not a slow one; its ticket is skipped (not removed) so it
# cannot hold the queue for ever.
DEFAULT_QUEUE_LAPSE = 10 * 60.0
TICKET_SCHEMA = "quirin-gate-ticket/1"
TICKET_NAME = re.compile(r"(\d{12})-(\d+)\.ticket")
SEQUENCE_FILE = ".sequence"
LEFTOVER_AGE = 600.0  # a ticket still being created for this long was abandoned
PROC = Path("/proc")
PROC_LOCKS = PROC / "locks"
IO_FIELDS = ("rchar", "wchar", "syscr", "syscw", "read_bytes", "write_bytes")
GIB_KIB = 1024 * 1024
try:
    CLOCK_TICKS = os.sysconf("SC_CLK_TCK")
    PAGE_KIB = os.sysconf("SC_PAGE_SIZE") // 1024
except (AttributeError, ValueError, OSError):  # pragma: no cover - non-POSIX
    CLOCK_TICKS, PAGE_KIB = 100, 4


class Refusal(Exception):
    """A request refused before any lock was taken or anything ran."""


class DrainFailed(Exception):
    """A rust-suite drain gave up as often as ``--drain-attempts`` allows; every
    lock it took is released.  ``attempts`` holds each attempt's record."""

    def __init__(self, message: str, attempts: list):
        super().__init__(message)
        self.attempts = attempts


class WaitTimeout(TimeoutError):
    """``--wait-timeout`` passed; for a rust-suite job, ``attempts`` holds its
    drain attempts so far (blockers named)."""

    def __init__(self, message: str, attempts: Optional[list] = None):
        super().__init__(message)
        self.attempts = attempts or []


# --- durations ------------------------------------------------------------------


_DURATION = re.compile(r"(\d+(?:\.\d+)?)(s|m|h)")
_UNIT_SECONDS = {"s": 1.0, "m": 60.0, "h": 3600.0}


def parse_duration(text: str, what: str, allow_zero: bool = False) -> Optional[float]:
    """Seconds of ``90s``/``30m``/``2h`` (a unit is required, so ``60`` is not
    read as either); ``0``/``off``/``none`` is None (no deadline), or 0.0 when
    ``allow_zero`` (a grace of nothing)."""
    value = text.strip().lower()
    if value in ("0", "off", "none"):
        return 0.0 if allow_zero else None
    match = _DURATION.fullmatch(value)
    if not match:
        raise Refusal(f"{what} must be a duration such as 90s, 30m or 2h, or 0/off "
                      f"(got {text!r})")
    seconds = float(match[1]) * _UNIT_SECONDS[match[2]]
    if seconds == 0 and not allow_zero:
        return None
    return seconds


def format_duration(seconds: Optional[float]) -> str:
    if seconds is None:
        return "none"
    seconds = max(0.0, seconds)
    if seconds < 60:
        return f"{seconds:.1f}s" if seconds != int(seconds) else f"{int(seconds)}s"
    whole = int(round(seconds))
    hours, rest = divmod(whole, 3600)
    minutes, secs = divmod(rest, 60)
    parts = [f"{hours}h" if hours else "", f"{minutes}m" if minutes else "",
             f"{secs}s" if secs else ""]
    return "".join(parts) or "0s"


# --- process trees (Linux /proc; elsewhere every reading is absent) -------------


def proc_stat(pid: int, proc: Path = PROC) -> Optional[dict]:
    """state, ppid, CPU ticks (own and reaped children's), start time (ticks
    since boot), resident pages, command name; None when unreadable."""
    try:
        text = (proc / str(pid) / "stat").read_text(encoding="utf-8", errors="replace")
    except OSError:
        return None
    head, _, rest = text.rpartition(")")
    fields = rest.split()
    try:
        return {"state": fields[0], "ppid": int(fields[1]),
                "cpu_ticks": sum(int(field) for field in fields[11:15]),
                "starttime": int(fields[19]), "rss_pages": int(fields[21]),
                "comm": head.partition("(")[2]}
    except (IndexError, ValueError):
        return None


def process_table(proc: Path = PROC) -> dict[int, dict]:
    table: dict[int, dict] = {}
    try:
        entries = list(os.scandir(proc))
    except OSError:
        return table
    for entry in entries:
        if entry.name.isdigit():
            stat = proc_stat(int(entry.name), proc)
            if stat is not None:
                table[int(entry.name)] = stat
    return table


def tree_pids(root: int, table: dict[int, dict], include_root: bool = True) -> list[int]:
    """``root`` and every live descendant (by parent pid), root first."""
    if root not in table:
        return []
    children: dict[int, list[int]] = {}
    for pid, stat in table.items():
        children.setdefault(stat["ppid"], []).append(pid)
    found = [root]
    stack = [root]
    while stack:
        for child in sorted(children.get(stack.pop(), ())):
            if child not in found:
                found.append(child)
                stack.append(child)
    return found if include_root else found[1:]


def read_io(pid: int, proc: Path = PROC) -> Optional[dict[str, int]]:
    """``/proc/<pid>/io`` (reaped children's counts included), or None."""
    try:
        text = (proc / str(pid) / "io").read_text(encoding="ascii")
    except OSError:
        return None
    values: dict[str, int] = {}
    for line in text.splitlines():
        name, _, value = line.partition(":")
        if name.strip() in IO_FIELDS:
            try:
                values[name.strip()] = int(value)
            except ValueError:
                return None
    return values if len(values) == len(IO_FIELDS) else None


def read_cmdline(pid: int, proc: Path = PROC) -> Optional[list[str]]:
    try:
        raw = (proc / str(pid) / "cmdline").read_bytes()
    except OSError:
        return None
    return [part.decode(errors="replace") for part in raw.split(b"\0") if part] or None


def sample_tree(root: int, include_root: bool = True, table: Optional[dict] = None) -> dict:
    """One reading of a process tree: its members (pid -> start time), summed
    CPU seconds, I/O counters and resident set, the largest member, the pids
    whose I/O could not be read, and a fingerprint that changes whenever any
    member's I/O counters, state or membership change."""
    table = process_table() if table is None else table
    pids = tree_pids(root, table, include_root)
    io_total = {field: 0 for field in IO_FIELDS}
    unreadable: list[int] = []
    fingerprint = []
    rss_kib = 0
    largest = (0, None)
    cpu_ticks = 0
    for pid in pids:
        stat = table[pid]
        io = read_io(pid)
        if io is None:
            # A zombie's counters are unreadable and it does no I/O; anything
            # else unreadable is unmeasured.
            if stat["state"] != "Z":
                unreadable.append(pid)
        else:
            for field in IO_FIELDS:
                io_total[field] += io[field]
        fingerprint.append((pid, stat["starttime"], stat["state"] == "Z",
                            tuple(io[field] for field in IO_FIELDS) if io else None))
        member_kib = stat["rss_pages"] * PAGE_KIB
        rss_kib += member_kib
        largest = max(largest, (member_kib, pid), key=lambda item: item[0])
        cpu_ticks += stat["cpu_ticks"]
    return {"monotonic": time.monotonic(), "pids": pids,
            "members": {pid: table[pid]["starttime"] for pid in pids},
            "cpu_seconds": cpu_ticks / CLOCK_TICKS, "io": io_total,
            "io_unreadable": unreadable, "fingerprint": tuple(sorted(fingerprint)),
            "tree_rss_kib": rss_kib, "largest_rss_kib": largest[0], "largest_pid": largest[1]}


def sample_delta(before: dict, after: dict) -> dict:
    """CPU and I/O spent between two readings of one tree, over its window."""
    return {"window_seconds": round(after["monotonic"] - before["monotonic"], 3),
            "cpu_seconds": round(after["cpu_seconds"] - before["cpu_seconds"], 2),
            "io": {field: after["io"][field] - before["io"][field] for field in IO_FIELDS},
            "io_unreadable": sorted(set(before["io_unreadable"]) | set(after["io_unreadable"])),
            "processes": len(after["pids"]), "tree_rss_kib": after["tree_rss_kib"]}


def alive(pid: int, starttime: int) -> bool:
    """The process with that pid is the one seen before and not yet a zombie."""
    stat = proc_stat(pid)
    return stat is not None and stat["starttime"] == starttime and stat["state"] != "Z"


def signal_verified(pid: int, starttime: int, signum: int) -> bool:
    """Signal ``pid`` only if it is still the process with that start time:
    a pidfd pins the process, then its start time is checked, so a recycled
    pid is never signalled."""
    pidfd = None
    try:
        pidfd = os.pidfd_open(pid)
    except (AttributeError, OSError):
        pidfd = None
    try:
        if not alive(pid, starttime):
            return False
        if pidfd is not None:
            signal.pidfd_send_signal(pidfd, signum)
        else:  # pragma: no cover - no pidfd: the start-time check alone
            os.kill(pid, signum)
        return True
    except (ProcessLookupError, PermissionError, OSError):
        return False
    finally:
        if pidfd is not None:
            os.close(pidfd)


# --- kernel locks ------------------------------------------------------------------


def parse_proc_locks(text: str) -> list[dict]:
    """``/proc/locks`` lines: kind, mode, pid, device (major, minor), inode and
    whether the line is a waiter (``->``)."""
    entries = []
    for line in text.splitlines():
        tokens = line.split()
        where = next((index for index, token in enumerate(tokens)
                      if re.fullmatch(r"[0-9a-fA-F]+:[0-9a-fA-F]+:\d+", token)), None)
        if where is None or where < 3:
            continue
        major, minor, inode = tokens[where].split(":")
        try:
            pid = int(tokens[where - 1])
        except ValueError:
            continue
        queued = len(tokens) > 1 and tokens[1] == "->"
        entries.append({"kind": tokens[2 if queued else 1], "mode": tokens[where - 2],
                        "pid": pid, "queued": queued,
                        "device": (int(major, 16), int(minor, 16)), "inode": int(inode)})
    return entries


def gate_files(gate_dir: Path, legacy: Path) -> dict[tuple, str]:
    """(major, minor, inode) -> label of every slot file under the gate
    directory (``<class>/<n>``, ``audit/admission``) and the legacy lock."""
    files: dict[tuple, str] = {}
    candidates = [(legacy, "legacy")]
    try:
        for directory in sorted(gate_dir.iterdir()):
            if directory.is_dir():
                candidates.extend((path, f"{directory.name}/{path.name}")
                                  for path in sorted(directory.iterdir()))
    except OSError:
        pass
    for path, label in candidates:
        try:
            stat = path.stat()
        except OSError:
            continue
        if path.is_file():
            files[(os.major(stat.st_dev), os.minor(stat.st_dev), stat.st_ino)] = label
    return files


def gate_locks(gate_dir: Path, legacy: Path, locks: Path = PROC_LOCKS) -> Optional[list[dict]]:
    """The kernel's locks on gate files, labelled; None when ``/proc/locks``
    cannot be read (not Linux)."""
    try:
        text = locks.read_text(encoding="ascii", errors="replace")
    except OSError:
        return None
    files = gate_files(gate_dir, legacy)
    found = []
    for entry in parse_proc_locks(text):
        label = files.get((*entry["device"], entry["inode"]))
        if label is not None:
            found.append(dict(entry, label=label))
    return found


def read_note(path: Path) -> Optional[dict]:
    """A slot file's holder note (rewritten in place, so retried once)."""
    for _ in range(3):
        try:
            text = path.read_text(encoding="utf-8")
        except OSError:
            return None
        if not text.strip():
            return None
        try:
            value = json.loads(text)
            return value if isinstance(value, dict) else None
        except ValueError:
            time.sleep(0.01)
    return None


# --- the arrival-order queue ---------------------------------------------------


def queue_directory(gate_dir: Path, cls: str) -> Path:
    return gate_dir / QUEUE_FAMILY[cls] / QUEUE_DIR


def owner_gone(pid: Optional[int], starttime: Optional[int]) -> bool:
    """The process that took a ticket no longer exists: no such pid, a zombie,
    or a pid now started at another time (a reused pid).  Without a readable
    ``/proc`` a pid that answers ``kill(pid, 0)`` is taken to be the owner."""
    if not isinstance(pid, int) or pid <= 0:
        return True
    stat = proc_stat(pid)
    if stat is not None:
        return stat["state"] == "Z" or (starttime is not None and stat["starttime"] != starttime)
    if (PROC / "self" / "stat").exists():
        return True  # /proc reads, and has no such pid
    try:
        os.kill(pid, 0)
    except ProcessLookupError:
        return True
    except PermissionError:
        return False
    return False


def queued_slot(ahead: Sequence[Optional[int]], my_count: int,
                free: Sequence[int]) -> tuple[Optional[int], list[int]]:
    """The slot a waiter may take, given the waiters ahead of it, in order.

    ``ahead`` holds, for each waiter ahead, how many slots it could take (slot
    indices ``0..n-1``) or None if it needs them all (a ``rust-suite`` job).  Each
    waiter ahead takes the lowest free slot it could; this waiter may take the
    lowest of those left below ``my_count``.  Returns (slot or None, positions in
    ``ahead`` that found no slot): a waiter that finds none holds back nothing."""
    remaining = sorted(free)
    blocked: list[int] = []
    for position, count in enumerate(ahead):
        if count is None:
            return None, list(range(len(ahead)))
        taken = next((slot for slot in remaining if slot < count), None)
        if taken is None:
            blocked.append(position)
        else:
            remaining.remove(taken)
    return next((slot for slot in remaining if slot < my_count), None), blocked


def scan_queue(directory: Path, lapse: Optional[float] = DEFAULT_QUEUE_LAPSE,
               reclaim: bool = True, now: Optional[float] = None) -> dict:
    """Read a queue directory: its live tickets in arrival order and the stale
    ones.  A ticket is *held* while its owner keeps the flock it took at
    creation.  It is stale (and, with ``reclaim``, unlinked) only when nobody
    holds that flock **and** the owner pid is gone or has another start time; a
    free flock under a live owner is read as live.  A live ticket is *lapsed*
    when its owner has not touched it for ``lapse`` seconds (not while the
    drain backs off).  Each live entry: seq, path, pid, cls, state, admission_mib,
    held, lapsed, mtime, note."""
    now = time.time() if now is None else now
    live: list[dict] = []
    stale: list[dict] = []
    try:
        names = sorted(os.listdir(directory))
    except OSError:
        return {"live": live, "stale": stale}
    for name in names:
        match = TICKET_NAME.fullmatch(name)
        if match is None:
            continue
        path = directory / name
        try:
            fd = os.open(path, os.O_RDWR | os.O_CLOEXEC)
        except OSError:
            continue
        try:
            held = False
            try:
                fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
            except OSError as error:
                if error.errno not in (errno.EWOULDBLOCK, errno.EAGAIN, errno.EACCES):
                    continue
                held = True
            try:
                mtime = os.fstat(fd).st_mtime
            except OSError:
                continue
            note = read_note(path) or {}
            if not held and (owner_gone(note.get("pid"), note.get("starttime"))
                             if note else now - mtime > LEFTOVER_AGE):
                stale.append({"seq": int(match[1]), "pid": note.get("pid"), "path": str(path)})
                if reclaim:
                    try:
                        os.unlink(path)  # while the probe lock is held
                    except OSError:
                        pass
                continue
            state = note.get("state", "waiting")
            live.append({
                "seq": int(match[1]), "path": path, "pid": note.get("pid", int(match[2])),
                "cls": note.get("cls"), "state": state, "held": held,
                "admission_mib": note.get("admission_mib"), "mtime": mtime, "note": note,
                "lapsed": state != "backoff" and lapse is not None and now - mtime > lapse})
        finally:
            os.close(fd)
    return {"live": live, "stale": stale}


class Ticket:
    """One waiter's place in a queue: a file the waiter keeps flocked."""

    def __init__(self, path: Path, fd: int, seq: int, body: dict):
        self.path = path
        self.fd: Optional[int] = fd
        self.seq = seq
        self.body = body

    def _write(self) -> None:
        if self.fd is None:
            return
        data = (json.dumps(self.body, sort_keys=True) + "\n").encode()
        os.pwrite(self.fd, data, 0)
        os.ftruncate(self.fd, len(data))

    def update(self, **fields) -> None:
        self.body.update(fields)
        self._write()

    def heartbeat(self) -> None:
        """Touch the ticket: the owner is polling."""
        if self.fd is not None:
            try:
                os.utime(self.fd)
            except OSError:
                pass

    def drop(self) -> None:
        if self.fd is not None:
            try:
                os.unlink(self.path)
            except OSError:
                pass
            os.close(self.fd)
            self.fd = None


def take_ticket(directory: Path, body: dict) -> Ticket:
    """A new ticket after every one in ``directory``.  The next number is read,
    and the ticket created already flocked and renamed into place, under a
    flock on ``.sequence`` that is held for those few system calls."""
    directory.mkdir(parents=True, exist_ok=True)
    sequence = os.open(directory / SEQUENCE_FILE, os.O_RDWR | os.O_CREAT | os.O_CLOEXEC, 0o666)
    try:
        fcntl.flock(sequence, fcntl.LOCK_EX)
        try:
            counter = int(os.pread(sequence, 32, 0).decode("ascii").strip() or 0)
        except (ValueError, OSError):
            counter = 0
        names = os.listdir(directory)
        seen = max([int(m[1]) for m in map(TICKET_NAME.fullmatch, names) if m] or [0])
        seq = max(counter, seen) + 1
        data = f"{seq}\n".encode()
        os.pwrite(sequence, data, 0)
        os.ftruncate(sequence, len(data))
        for name in names:  # tickets abandoned while being created
            if name.startswith(".") and name.endswith(".new"):
                try:
                    if time.time() - (directory / name).stat().st_mtime > LEFTOVER_AGE:
                        os.unlink(directory / name)
                except OSError:
                    pass
        pid = os.getpid()
        partial = directory / f".{seq:012d}-{pid}.new"
        fd = os.open(partial, os.O_RDWR | os.O_CREAT | os.O_EXCL | os.O_CLOEXEC, 0o666)
        fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
        body = dict(body, schema=TICKET_SCHEMA, seq=seq, pid=pid)
        final = directory / f"{seq:012d}-{pid}.ticket"
        ticket = Ticket(final, fd, seq, body)
        ticket._write()
        os.rename(partial, final)
        return ticket
    finally:
        os.close(sequence)


# --- arithmetic ---------------------------------------------------------------


def parse_budget(text: str) -> int:
    try:
        return BUDGETS_MIB[text.upper()]
    except KeyError:
        raise Refusal(
            f"--memory-budget must be one of {', '.join(BUDGETS_MIB)} (got {text!r})"
        ) from None


def parse_short_budget(text: str) -> int:
    try:
        return SHORT_BUDGETS_MIB[text.upper()]
    except KeyError:
        raise Refusal(
            f"--memory-budget for the short class must be one of "
            f"{', '.join(SHORT_BUDGETS_MIB)} (got {text!r}); a job that needs more "
            "is not memory-light: use the audit class") from None


def lane_reserved_kib(gate_dir: Path) -> int:
    """Memory the lane jobs running now have been admitted for and have not
    yet used, in KiB: for each lane slot held, its declared budget less the
    resident set of its last sample (all of it before the first sample), and
    nothing for the part already in use, which ``MemAvailable`` already
    excludes.  An audit or lane request takes this from its reading, so a lane
    job that has just been admitted counts against audits before it has
    allocated.  A slot is held when a probe cannot lock it; the probe is let go
    at once.  A held slot with no readable note counts at the largest lane
    budget (fail closed); a ``rust-suite`` job's own hold counts for nothing
    (the audits are shut out by its admission lock)."""
    reserved = 0
    for index in range(SHORT_SLOTS):
        probe = Lock(gate_dir / SHORT / str(index), f"{SHORT}/{index}")
        try:
            if probe.try_lock(fcntl.LOCK_EX):
                continue
        except OSError:
            continue
        finally:
            probe.release()
        note = read_note(gate_dir / SHORT / str(index)) or {}
        if note.get("cls") == "rust-suite":
            continue
        budget = note.get("admission_mib")
        budget_kib = (budget if isinstance(budget, int) and budget > 0
                      else max(SHORT_BUDGETS_MIB.values())) * 1024
        live = note.get("live") or {}
        used = live.get("tree_rss_kib") if isinstance(live.get("tree_rss_kib"), int) else 0
        reserved += max(0, budget_kib - used)
    return reserved


def read_meminfo(path: Path = MEMINFO) -> dict[str, int]:
    """`/proc/meminfo` fields in KiB."""
    values: dict[str, int] = {}
    for line in path.read_text(encoding="ascii").splitlines():
        name, _, rest = line.partition(":")
        parts = rest.split()
        if parts and parts[0].isdigit():
            values[name.strip()] = int(parts[0])
    for field in ("MemAvailable", "MemTotal"):
        if field not in values:
            raise Refusal(f"{path} has no {field} field")
    return values


def audit_slot_count(mem_available_kib: int, budget_mib: int) -> int:
    """floor(MemAvailable / budget), at least 1."""
    return max(1, mem_available_kib // (budget_mib * 1024))


def audit_slot_ceiling(mem_total_kib: int) -> int:
    """The most audit slots any budget can open on this host (MemTotal at the
    smallest budget).  A ``rust-suite`` job takes all of them."""
    return max(1, mem_total_kib // (SMALLEST_BUDGET_MIB * 1024))


def audit_jobs_cap(budget_mib: int) -> int:
    """Workers the audit plans for under ``budget_mib`` (parallel.rs)."""
    if budget_mib < AUDIT_COORDINATOR_MIB + AUDIT_WORKER_MIB:
        raise Refusal(f"budget {budget_mib} MiB is below the audit's 3072 MiB minimum")
    return (budget_mib - AUDIT_COORDINATOR_MIB) // AUDIT_WORKER_MIB


def route_class(argv: Sequence[str]) -> Optional[str]:
    """The measured route class a command runs, or None: ``pinned-corpus`` for
    ``scripts/run_pinned_audit.py`` or ``quirin-audit pinned`` anywhere in the
    argv (a namespace or timing wrapper in front of it does not hide it)."""
    for index, arg in enumerate(argv):
        if Path(arg).name == "run_pinned_audit.py":
            return "pinned-corpus"
        if _is_quirin_audit(arg) and index + 1 < len(argv) and argv[index + 1] == "pinned":
            return "pinned-corpus"
    return None


def admission_budget(argv: Sequence[str], budget_mib: int) -> tuple[int, Optional[str]]:
    """(MiB the slot admission counts, route class): the larger of the slot
    budget and the route's measured budget."""
    route = route_class(argv)
    return max(budget_mib, ROUTE_ADMISSION_MIB.get(route or "", 0)), route


# --- passing the budget to the audit --------------------------------------------


def _flag_value(argv: Sequence[str], flag: str) -> Optional[str]:
    for index, arg in enumerate(argv):
        if arg == flag and index + 1 < len(argv):
            return argv[index + 1]
        if arg.startswith(flag + "="):
            return arg.split("=", 1)[1]
    return None


def _is_quirin_audit(arg: str) -> bool:
    return Path(arg).name in ("quirin-audit", "quirin-audit.exe")


def apply_budget(argv: Sequence[str], env: dict[str, str], budget_mib: int,
                 cwd: Path) -> tuple[list[str], dict[str, str], dict]:
    """Return (argv, env, record) carrying the budget into the audit.

    * ``quirin-audit current``: ``--memory-budget-mib N`` is added in the
      two-word form ``current_cli.rs`` reads (an existing flag, in either
      spelling, must already equal the budget); the audit caps its own workers.
    * ``quirin-audit CONFIG.json``: a configured ``memory_budget_mib`` must equal
      the budget; otherwise ``QCORE_AUDIT_JOBS`` carries the cap, refused when
      the configuration's ``jobs`` would override it with more workers.
    * anything else (``quirin-audit pinned``, ``run_pinned_audit.py``, a
      wrapper script): ``QCORE_AUDIT_JOBS`` is lowered to the budget's worker
      cap, and an explicit ``--jobs`` above the cap is refused, because the
      flag would override the variable.
    """
    argv = list(argv)
    env = dict(env)
    cap = audit_jobs_cap(budget_mib)
    env["QUIRIN_GATE_MEMORY_BUDGET_MIB"] = str(budget_mib)
    record: dict = {"memory_budget_mib": budget_mib, "jobs_cap": cap}
    position = next((i for i, arg in enumerate(argv) if _is_quirin_audit(arg)), None)
    route = argv[position + 1] if position is not None and position + 1 < len(argv) else None
    if route == "current":
        given = _flag_value(argv, "--memory-budget-mib")
        if given is not None:
            if given != str(budget_mib):
                raise Refusal(
                    f"the command's --memory-budget-mib {given} differs from the slot "
                    f"budget {budget_mib} MiB"
                )
        else:
            argv[position + 2:position + 2] = ["--memory-budget-mib", str(budget_mib)]
        record["mechanism"] = "quirin-audit current --memory-budget-mib"
        return argv, env, record
    if route is not None and route.endswith(".json"):
        config_path = Path(route) if Path(route).is_absolute() else cwd / route
        try:
            config = json.loads(config_path.read_text(encoding="utf-8"))
        except (OSError, ValueError) as error:
            raise Refusal(f"cannot read the audit configuration {config_path}: {error}") from None
        configured = config.get("memory_budget_mib") if isinstance(config, dict) else None
        if configured is not None:
            if configured != budget_mib:
                raise Refusal(
                    f"{config_path} sets memory_budget_mib {configured}, not the slot "
                    f"budget {budget_mib} MiB"
                )
            record["mechanism"] = "configuration memory_budget_mib"
            return argv, env, record
        jobs = config.get("jobs") if isinstance(config, dict) else None
        if isinstance(jobs, int) and jobs > cap:
            raise Refusal(
                f"{config_path} sets jobs {jobs}, above the {budget_mib} MiB budget's "
                f"{cap} workers, and has no memory_budget_mib"
            )
    jobs_flag = _flag_value(argv, "--jobs")
    if jobs_flag is not None:
        try:
            requested = int(jobs_flag)
        except ValueError:
            raise Refusal(f"--jobs {jobs_flag!r} is not an integer") from None
        if requested > cap:
            raise Refusal(
                f"--jobs {requested} overrides the {budget_mib} MiB budget's {cap} workers; "
                f"pass --jobs {cap} or less"
            )
    previous = env.get("QCORE_AUDIT_JOBS")
    effective = cap
    if previous is not None:
        try:
            effective = min(cap, int(previous))
        except ValueError:
            raise Refusal(f"QCORE_AUDIT_JOBS={previous!r} is not an integer") from None
        if effective < 1:
            raise Refusal(f"QCORE_AUDIT_JOBS={previous!r} is not positive")
    env["QCORE_AUDIT_JOBS"] = str(effective)
    record["mechanism"] = "QCORE_AUDIT_JOBS"
    record["QCORE_AUDIT_JOBS"] = {"before": previous, "after": str(effective)}
    return argv, env, record


# --- locks ----------------------------------------------------------------------


class Lock:
    """One flock on one file (its own open file description)."""

    def __init__(self, path: Path, label: str):
        self.path = path
        self.label = label
        self.fd: Optional[int] = None
        self.noted = False

    def try_lock(self, mode: int) -> bool:
        self.path.parent.mkdir(parents=True, exist_ok=True)
        if self.fd is None:
            self.fd = os.open(self.path, os.O_RDWR | os.O_CREAT | os.O_CLOEXEC, 0o666)
        try:
            fcntl.flock(self.fd, mode | fcntl.LOCK_NB)
            return True
        except OSError as error:
            if error.errno in (errno.EWOULDBLOCK, errno.EAGAIN, errno.EACCES):
                return False
            raise

    def lock_blocking(self, mode: int) -> None:
        """Wait in the kernel's queue, beside `flock(1)` waiters, not by polling."""
        self.path.parent.mkdir(parents=True, exist_ok=True)
        if self.fd is None:
            self.fd = os.open(self.path, os.O_RDWR | os.O_CREAT | os.O_CLOEXEC, 0o666)
        fcntl.flock(self.fd, mode)

    def note(self, holder: dict) -> None:
        """Write who holds an exclusive slot into the slot file (diagnostics)."""
        assert self.fd is not None
        data = (json.dumps(holder, sort_keys=True) + "\n").encode()
        os.pwrite(self.fd, data, 0)  # then cut the tail: a reader never sees it empty
        os.ftruncate(self.fd, len(data))
        self.noted = True

    def release(self) -> None:
        if self.fd is not None:
            try:
                if self.noted:
                    os.ftruncate(self.fd, 0)  # no stale holder after release
                    self.noted = False
                fcntl.flock(self.fd, fcntl.LOCK_UN)
            finally:
                os.close(self.fd)
                self.fd = None


class Gate:
    """Slot accounting for one request.  ``acquire`` blocks (polling) until the
    request is admitted or ``timeout`` seconds pass (WaitTimeout).  A
    ``rust-suite`` request drains the audit slots for at most ``drain_timeout``
    seconds per attempt (None: unbounded), then reopens admission, releases
    everything, backs off ``drain_backoff`` seconds and tries again, or raises
    DrainFailed once ``drain_attempts`` (0: unlimited) attempts gave up.

    With ``queue`` (the default) a request that waits takes a ticket and is
    admitted in arrival order (module docstring, "Arrival order"); ``queue_lapse``
    is how long a queued job may go without polling before the others skip its
    ticket (None: never)."""

    def __init__(self, gate_dir: Path = DEFAULT_GATE_DIR, legacy: Path = LEGACY_LOCK,
                 meminfo: Path = MEMINFO, poll: float = 2.0,
                 echo: Callable[[str], None] = lambda line: print(line, file=sys.stderr,
                                                                  flush=True),
                 queue: bool = True, queue_lapse: Optional[float] = DEFAULT_QUEUE_LAPSE):
        self.gate_dir = gate_dir
        self.legacy = legacy
        self.meminfo = meminfo
        self.poll = poll
        self.echo = echo
        self.held: list[Lock] = []
        self.record: dict = {}
        self.primary: Optional[Lock] = None
        self.holder: dict = {}
        self.queue = queue
        self.queue_lapse = queue_lapse
        self.ticket: Optional[Ticket] = None
        self.position: Optional[int] = None
        self._wake = threading.Event()

    def _wait(self, attempt: Callable[[], bool], what: str, deadline: Optional[float]) -> None:
        announced = False
        while True:
            if self.ticket is not None:
                self.ticket.heartbeat()
            if attempt():
                return
            if deadline is not None and time.monotonic() >= deadline:
                raise WaitTimeout(f"timed out waiting for {what}")
            if not announced:
                self.echo(f"gate_slot: waiting for {what}")
                announced = True
            time.sleep(self.poll)

    def wake(self) -> None:
        """End a drain's back-off now (the next attempt starts at once)."""
        self._wake.set()

    def _note(self, lock: Lock, holder: dict) -> None:
        lock.note(dict(holder, noted_at=utc_now()))
        if self.primary is None and lock.label not in ("audit/admission", "short/admission"):
            self.primary = lock

    def _exclusive(self, cls: str, index: "int | str", deadline: Optional[float], holder: dict) -> None:
        lock = Lock(self.gate_dir / cls / str(index), f"{cls}/{index}")
        try:
            self._wait(lambda: lock.try_lock(fcntl.LOCK_EX), f"slot {cls}/{index}", deadline)
        except BaseException:
            lock.release()
            raise
        self._note(lock, holder)
        self.held.append(lock)

    def _take_slot(self, cls: str, index: int, holder: dict) -> bool:
        lock = Lock(self.gate_dir / cls / str(index), f"{cls}/{index}")
        if lock.try_lock(fcntl.LOCK_EX):
            self._note(lock, holder)
            self.held.append(lock)
            return True
        lock.release()
        return False

    def _first_free(self, cls: str, count: int, holder: dict) -> Optional[int]:
        for index in range(count):
            if self._take_slot(cls, index, holder):
                return index
        return None

    # -- arrival order -------------------------------------------------------------

    def _join_queue(self, cls: str, admission_mib: Optional[int], holder: dict) -> None:
        """Take a ticket after every job already queued for this class's slots
        (not for a nested job, which holds its parent's slot)."""
        if not self.queue or self.ticket is not None:
            return
        directory = queue_directory(self.gate_dir, cls)
        stat = proc_stat(os.getpid())
        self.ticket = take_ticket(directory, {
            "cls": cls, "family": QUEUE_FAMILY[cls], "admission_mib": admission_mib,
            "argv": holder.get("argv"), "cwd": holder.get("cwd"),
            "starttime": stat["starttime"] if stat else None,
            "requested_at": utc_now(), "state": "waiting"})
        view = self._ahead()
        self.record["queue"] = {
            "enabled": True, "family": QUEUE_FAMILY[cls], "seq": self.ticket.seq,
            "ahead_at_request": len(view["ahead"]),
            "position_at_request": len(view["ahead"]) + 1, "lapse_seconds": self.queue_lapse,
            "reclaimed": [], "lapsed_skipped": 0, "passed_over": []}
        self._note_scan(view)
        self._announce(view)

    def _ahead(self) -> dict:
        """The queue as this ticket sees it: the live tickets ahead that hold
        others back (not lapsed, not backing off), the stale ones just reclaimed."""
        assert self.ticket is not None
        scan = scan_queue(self.ticket.path.parent, self.queue_lapse)
        before = [entry for entry in scan["live"] if entry["seq"] < self.ticket.seq]
        ahead = [entry for entry in before if not entry["lapsed"] and entry["state"] != "backoff"]
        return {"ahead": ahead, "skipped": [entry for entry in before if entry["lapsed"]],
                "behind": len(scan["live"]) - len(before) - 1, "stale": scan["stale"]}

    def _note_scan(self, view: dict) -> None:
        queue = self.record.get("queue")
        if queue is None:
            return
        for entry in view["stale"]:
            if entry["seq"] not in {item["seq"] for item in queue["reclaimed"]}:
                queue["reclaimed"].append({"seq": entry["seq"], "pid": entry["pid"]})
                self.echo(f"gate_slot: reclaimed ticket {entry['seq']} of pid {entry['pid']} "
                          "(its owner is gone)")
        queue["lapsed_skipped"] = max(queue["lapsed_skipped"], len(view["skipped"]))

    def _announce(self, view: dict) -> None:
        position = len(view["ahead"]) + 1
        if position != self.position:
            self.position = position
            assert self.ticket is not None
            self.echo(f"gate_slot: queue position {position} (ticket {self.ticket.seq}; "
                      f"{len(view['ahead'])} ahead, {max(0, view['behind'])} behind)")
            if "queue" in self.record:
                self.record["queue"]["position"] = position

    def _at_head(self) -> bool:
        """No waiter ahead holds this one back (a rust-suite drain starts only
        then, so it respects the audits queued before it)."""
        view = self._ahead()
        self._note_scan(view)
        self._announce(view)
        return not view["ahead"]

    def _free_slots(self, cls: str, upto: int) -> list[int]:
        """The slots below ``upto`` nobody holds (each probed and let go at once)."""
        free = []
        for index in range(upto):
            probe = Lock(self.gate_dir / cls / str(index), f"{cls}/{index}")
            try:
                if probe.try_lock(fcntl.LOCK_EX):
                    free.append(index)
            finally:
                probe.release()
        return free

    def _queued_slot(self, cls: str, my_count: int, count_of: Callable[[dict], Optional[int]],
                     holder: dict) -> Optional[int]:
        """Take the slot this waiter's turn allows, if any (see ``queued_slot``)."""
        view = self._ahead()
        self._note_scan(view)
        self._announce(view)
        counts = [count_of(entry) for entry in view["ahead"]]
        if any(count is None for count in counts):
            return None
        free = self._free_slots(cls, max([my_count, *counts]))
        index, blocked = queued_slot(counts, my_count, free)
        if index is None or not self._take_slot(cls, index, holder):
            return None
        passed = [{"seq": view["ahead"][position]["seq"], "pid": view["ahead"][position]["pid"]}
                  for position in blocked]
        if passed and "queue" in self.record:
            self.record["queue"]["passed_over"] = passed
        return index

    def _leave_queue(self) -> None:
        if self.ticket is not None:
            if "queue" in self.record:
                self.record["queue"]["admitted_at"] = utc_now()
            self.ticket.drop()
            self.ticket = None

    def _legacy_shared(self, deadline: Optional[float], record: dict) -> None:
        """The legacy interlock, shared: excludes `flock <legacy>` jobs."""
        legacy = Lock(self.legacy, "legacy")
        what = f"the legacy lock {self.legacy} (held exclusive by a legacy job)"
        try:
            if deadline is None and not legacy.try_lock(fcntl.LOCK_SH):
                # A poller loses every race against `flock <legacy>` jobs
                # blocked in the kernel (measured on this card's own
                # demonstration), so an untimed wait blocks like them.
                self.echo(f"gate_slot: waiting for {what}")
                legacy.lock_blocking(fcntl.LOCK_SH)
            else:
                self._wait(lambda: legacy.try_lock(fcntl.LOCK_SH), what, deadline)
        except BaseException:
            legacy.release()
            raise
        self.held.append(legacy)
        record["legacy_lock"] = {"path": str(self.legacy), "mode": "shared"}

    def update_note(self, live: dict) -> None:
        """Rewrite the primary slot's holder note with a ``live`` reading."""
        if self.primary is not None and self.primary.fd is not None:
            self.primary.note(dict(self.holder, noted_at=self.record.get("admitted_at"),
                                   live=live))

    # -- the rust-suite drain ------------------------------------------------------

    def audit_holders(self, ceiling: int, cls: str = "audit") -> list[dict]:
        """The slots of ``cls`` (``audit``, or the ``short`` lane) another
        process holds: label, pid (kernel locks, else the holder note) and the
        note."""
        mine = os.getpid()
        locks = gate_locks(self.gate_dir, self.legacy)
        holders = []
        for index in range(ceiling):
            label = f"{cls}/{index}"
            if any(lock.label == label for lock in self.held):
                continue
            note = read_note(self.gate_dir / cls / str(index))
            if locks is not None:
                pids = sorted({entry["pid"] for entry in locks
                               if entry["label"] == label and not entry["queued"]
                               and entry["pid"] != mine})
            else:
                pids = [note["pid"]] if note and isinstance(note.get("pid"), int) else []
            for pid in pids:
                holders.append({"slot": label, "pid": pid,
                                "note": note if note and note.get("pid") == pid else None})
        return holders

    def describe_blockers(self, holders: list[dict], before: dict) -> list[dict]:
        """Each blocking holder with its argv, hold time, and its job tree's CPU
        and I/O since the drain closed admission (``before``)."""
        table = process_table()
        described = []
        for holder in holders:
            pid = holder["pid"]
            note = holder.get("note") or {}
            entry = {"slot": holder["slot"], "pid": pid,
                     "argv": note.get("argv") or read_cmdline(pid),
                     "cwd": note.get("cwd"), "requested_at": note.get("since"),
                     "held_since": note.get("noted_at"),
                     "admission_mib": note.get("admission_mib")}
            held = seconds_since(note.get("noted_at"))
            if held is not None:
                entry["held_seconds"] = round(held, 1)
            after = sample_tree(pid, include_root=False, table=table)
            if pid in before and after["pids"]:
                entry["during_wait"] = sample_delta(before[pid], after)
            entry["tree_rss_kib"] = after["tree_rss_kib"]
            if entry["admission_mib"] is not None:
                entry["rss_over_admission"] = after["tree_rss_kib"] > entry["admission_mib"] * 1024
            described.append(entry)
        return described

    def _drain(self, ceiling: int, deadline: Optional[float], holder: dict,
               drain_timeout: Optional[float], drain_backoff: float,
               drain_attempts: int, record: dict) -> None:
        attempts: list[dict] = []
        record["drain"] = {"timeout_seconds": drain_timeout, "backoff_seconds": drain_backoff,
                           "attempts_allowed": drain_attempts or None, "attempts": attempts}
        while True:
            self._legacy_shared(deadline, record)
            self._join_queue("rust-suite", None, holder)
            if self.ticket is not None:
                self.ticket.update(state="waiting")
                self._wait(self._at_head, "its turn in the audit queue", deadline)
            self._exclusive("rust-suite", 0, deadline, holder)
            if self.ticket is not None:
                self.ticket.update(state="draining")
            # Close audit admission first so new audits stop starting, then
            # wait, at most the drain bound, for each running audit's slot.
            self._exclusive("audit", "admission", deadline, holder)
            closed = time.monotonic()
            attempt: dict = {"admission_closed_at": utc_now()}
            attempts.append(attempt)
            bound = None if drain_timeout is None else closed + drain_timeout
            drain_deadline = bound if deadline is None else (
                deadline if bound is None else min(bound, deadline))
            table = process_table()
            before = {entry["pid"]: sample_tree(entry["pid"], include_root=False, table=table)
                      for entry in self.audit_holders(ceiling)}
            lane_before: dict = {}
            try:
                for index in range(ceiling):
                    self._exclusive("audit", index, drain_deadline, holder)
                # The audits are drained.  Now (not while they drained: the lane
                # is open then) shut the short lane and wait for the lane jobs in
                # flight, each stopped at its own wall bound, so the suite's
                # load-sensitive rows run alone on the host.  This wait is
                # bounded by the lane's wall bound, not by the audit drain's.
                lane_bound = time.monotonic() + SHORT_DRAIN_BOUND
                lane_deadline = lane_bound if deadline is None else min(lane_bound, deadline)
                attempt["lane_closed_at"] = utc_now()
                self._exclusive(SHORT, "admission", lane_deadline, holder)
                lane_before = {entry["pid"]: sample_tree(entry["pid"], include_root=False)
                               for entry in self.audit_holders(SHORT_SLOTS, SHORT)}
                for index in range(SHORT_SLOTS):
                    self._exclusive(SHORT, index, lane_deadline, holder)
            except WaitTimeout:
                blockers = self.describe_blockers(
                    self.audit_holders(ceiling) + self.audit_holders(SHORT_SLOTS, SHORT),
                    {**before, **lane_before})
                waited = time.monotonic() - closed
                self._release_locks()  # reopen admission; give back every lock taken
                if self.ticket is not None:  # keep the place; let the queue run
                    self.ticket.update(state="backoff")
                attempt.update(outcome="gave up", waited_seconds=round(waited, 3),
                               reopened_at=utc_now(), blockers=blockers)
                summary = describe_blockers_line(blockers)
                if deadline is not None and time.monotonic() >= deadline:
                    raise WaitTimeout(
                        f"timed out draining the audit slots after {format_duration(waited)}; "
                        f"admission reopened and every slot released; blocked by {summary}",
                        attempts) from None
                if drain_attempts and len(attempts) >= drain_attempts:
                    raise DrainFailed(
                        f"audit drain gave up {len(attempts)} time(s), the last after "
                        f"{format_duration(waited)}; admission reopened and every slot "
                        f"released; blocked by {summary}", attempts) from None
                pause = drain_backoff
                if deadline is not None:
                    pause = max(0.0, min(pause, deadline - time.monotonic()))
                self.echo(f"gate_slot: audit drain gave up after {format_duration(waited)}; "
                          f"admission reopened and every slot released; blocked by {summary}; "
                          f"retrying in {format_duration(pause)}")
                self._wake.wait(pause)
                self._wake.clear()
                continue
            attempt.update(outcome="drained",
                           waited_seconds=round(time.monotonic() - closed, 3))
            record.update(slot=0, slot_count=1, audit_slots_held=list(range(ceiling)),
                          short_slots_held=list(range(SHORT_SLOTS)))
            self._leave_queue()
            return

    def acquire(self, cls: str, budget_mib: Optional[int] = None,
                timeout: Optional[float] = None, holder: Optional[dict] = None,
                drain_timeout: Optional[float] = None,
                drain_backoff: float = DEFAULT_DRAIN_BACKOFF,
                drain_attempts: int = 0) -> dict:
        if cls not in CLASSES:
            raise Refusal(f"unknown slot class {cls!r}; classes: {', '.join(CLASSES)}")
        holder = dict(holder or {}, pid=os.getpid(), cls=cls)
        self.holder = holder
        deadline = None if timeout is None else time.monotonic() + timeout
        started = time.monotonic()
        record: dict = {"class": cls, "gate_dir": str(self.gate_dir)}
        self.record = record
        try:
            memory = read_meminfo(self.meminfo)
            ceiling = audit_slot_ceiling(memory["MemTotal"])
            record["audit_slot_ceiling"] = ceiling
            if cls == "rust-suite":
                self._drain(ceiling, deadline, holder, drain_timeout, drain_backoff,
                            drain_attempts, record)
            elif cls == "audit":
                assert budget_mib is not None
                self._legacy_shared(deadline, record)
                self._join_queue("audit", budget_mib, holder)
                admitted: dict = {}
                available_kib = [0]  # the reading of this poll, for the tickets ahead

                def audit_count(entry: dict) -> Optional[int]:
                    """Slots a waiter ahead could take at this reading."""
                    if entry["cls"] == "rust-suite":
                        return None  # a drain takes every audit slot
                    mib = entry["admission_mib"] or SMALLEST_BUDGET_MIB
                    return min(ceiling, audit_slot_count(available_kib[0], mib))

                def attempt() -> bool:
                    admission = Lock(self.gate_dir / "audit" / "admission", "audit/admission")
                    try:
                        if not admission.try_lock(fcntl.LOCK_SH):
                            return False  # a rust-suite job holds or awaits every slot
                        # Lane jobs admitted ahead of this audit count against it
                        # (their declared memory not yet in MemAvailable).
                        reserved = lane_reserved_kib(self.gate_dir)
                        available = max(0, read_meminfo(self.meminfo)["MemAvailable"] - reserved)
                        count = min(ceiling, audit_slot_count(available, budget_mib))
                        available_kib[0] = available
                        if self.ticket is not None:
                            index = self._queued_slot("audit", count, audit_count, holder)
                        else:
                            index = self._first_free("audit", count, holder)
                        if index is None:
                            return False
                        admitted.update(slot=index, slot_count=count,
                                        mem_available_kib=available)
                        return True
                    finally:
                        admission.release()

                self._wait(attempt, "an audit slot", deadline)
                record.update(admitted)
            elif cls == SHORT:
                assert budget_mib is not None
                self._legacy_shared(deadline, record)
                self._join_queue(SHORT, budget_mib, holder)
                taken: dict = {}
                lane_available = [0]

                def lane_count(entry: dict) -> Optional[int]:
                    """Slots a lane waiter ahead could take at this reading: all
                    of them if its declared memory fits, none if it does not (it
                    then holds nothing back)."""
                    mib = entry["admission_mib"] or max(SHORT_BUDGETS_MIB.values())
                    return SHORT_SLOTS if lane_available[0] >= mib * 1024 else 0

                def attempt_short() -> bool:
                    # The lane's own admission lock: a rust-suite job that has
                    # drained the audits closes it (and only then).  Audit
                    # locks are never taken, so no audit or drain holds this up.
                    admission = Lock(self.gate_dir / SHORT / "admission", "short/admission")
                    try:
                        if not admission.try_lock(fcntl.LOCK_SH):
                            return False
                        available = max(0, read_meminfo(self.meminfo)["MemAvailable"]
                                        - lane_reserved_kib(self.gate_dir))
                        lane_available[0] = available
                        count = SHORT_SLOTS if available >= budget_mib * 1024 else 0
                        if self.ticket is not None:
                            index = self._queued_slot(SHORT, count, lane_count, holder)
                        else:
                            index = self._first_free(SHORT, count, holder)
                        if index is None:
                            return False
                        taken.update(slot=index, slot_count=SHORT_SLOTS,
                                     mem_available_kib=available)
                        return True
                    finally:
                        admission.release()

                self._wait(attempt_short, "a short-lane slot", deadline)
                record.update(taken)
            else:
                self._legacy_shared(deadline, record)
                count = FIXED_SLOTS[cls]
                self._join_queue(cls, None, holder)

                def attempt_fixed() -> bool:
                    if self.ticket is not None:
                        index = self._queued_slot(cls, count, lambda entry: count, holder)
                    else:
                        index = self._first_free(cls, count, holder)
                    if index is None:
                        return False
                    record.update(slot=index, slot_count=count)
                    return True

                self._wait(attempt_fixed, f"a {cls} slot", deadline)
            self._leave_queue()
            final = read_meminfo(self.meminfo)
            record.setdefault("mem_available_kib", final["MemAvailable"])
            record["mem_total_kib"] = final["MemTotal"]
        except BaseException:
            self.release()
            raise
        record["held"] = [lock.label for lock in self.held]
        record["waited_seconds"] = round(time.monotonic() - started, 3)
        record["admitted_at"] = utc_now()
        return record

    def _release_locks(self) -> None:
        while self.held:
            self.held.pop().release()
        self.primary = None

    def release(self) -> None:
        """Give back every lock and the ticket."""
        self._release_locks()
        if self.ticket is not None:
            self.ticket.drop()
            self.ticket = None


def seconds_since(stamp: Optional[str]) -> Optional[float]:
    if not stamp:
        return None
    try:
        then = datetime.datetime.fromisoformat(stamp)
    except (TypeError, ValueError):
        return None
    return (datetime.datetime.now(datetime.timezone.utc) - then).total_seconds()


def describe_blockers_line(blockers: list[dict]) -> str:
    if not blockers:
        return "no live holder (the slots were freed as the bound passed)"
    parts = []
    for blocker in blockers:
        argv = " ".join((blocker.get("argv") or ["?"])[:4])
        text = f"{blocker['slot']} pid {blocker['pid']} ({argv}"
        if "held_seconds" in blocker:
            text += f"; held {format_duration(blocker['held_seconds'])}"
        during = blocker.get("during_wait")
        if during:
            io = during["io"]
            text += (f"; over the {format_duration(during['window_seconds'])} wait: "
                     f"CPU {during['cpu_seconds']}s, rchar +{io['rchar']}, "
                     f"wchar +{io['wchar']}, read_bytes +{io['read_bytes']}, "
                     f"write_bytes +{io['write_bytes']}")
        text += f"; RSS {blocker['tree_rss_kib']} KiB"
        if blocker.get("admission_mib") is not None:
            text += f" against {blocker['admission_mib']} MiB admitted"
        parts.append(text + ")")
    return "; ".join(parts)


# --- running ----------------------------------------------------------------------


def parse_held(text: Optional[str]) -> list[str]:
    return [item for item in (text or "").split(",") if item]


class Monitor:
    """Watches the command's process tree while it runs (card
    GATE-SLOT-ADMISSION-STARVATION): the stall and wall-clock deadlines, the
    resident set against the admission budget, and the slot note's live
    record.  A reading that fails never stops the command; ``record`` is the
    receipt's ``monitor`` slot."""

    def __init__(self, stall: Optional[float] = None, deadline: Optional[float] = None,
                 interval: float = SAMPLE_INTERVAL, grace: float = DEFAULT_STOP_GRACE,
                 progress_file: Optional[Path] = None, admission_mib: Optional[int] = None,
                 echo: Callable[[str], None] = lambda line: print(line, file=sys.stderr,
                                                                  flush=True),
                 on_sample: Optional[Callable[[dict], None]] = None,
                 receipt: Optional[Path] = None, rss_stop_mib: Optional[int] = None):
        self.rss_stop_mib = rss_stop_mib
        self.stall = stall
        self.deadline = deadline
        self.interval = interval
        self.grace = grace
        self.progress_file = progress_file
        self.admission_mib = admission_mib
        self.echo = echo
        self.on_sample = on_sample
        self.receipt = receipt
        self.root: Optional[int] = None
        self.started = 0.0
        self.next_due = 0.0
        self.last_key: object = None
        self.last_progress = 0.0
        self.last_progress_at = ""
        self.cpu_at_progress = 0.0
        self.reported_rss_kib = 0
        self.warned_unmeasured = False
        self.breach: Optional[dict] = None
        self.record: dict = {
            "stall_timeout_seconds": stall, "deadline_seconds": deadline,
            "stop_grace_seconds": grace, "sample_interval_seconds": interval,
            "progress_file": str(progress_file) if progress_file else None,
            "admission_mib": admission_mib, "samples": 0, "samples_unmeasured": 0,
            "sample_errors": 0, "peak_tree_rss_kib": 0,
        }

    def start(self, root: int) -> None:
        self.root = root
        self.started = time.monotonic()
        self.next_due = self.started + self.interval
        self.last_progress = self.started
        self.last_progress_at = utc_now()
        if not (PROC / str(root) / "stat").exists():
            # No /proc (not Linux): the tree cannot be read, so there is no
            # stall or resident-set reading; the wall-clock deadline still holds.
            self.record["tree_readable"] = False
            self.echo("gate_slot: cannot read the process tree here (no /proc); the stall "
                      "timeout and the resident-set report are inactive, the wall-clock "
                      "deadline still applies")

    def wait_budget(self) -> float:
        return max(0.0, self.next_due - time.monotonic())

    def _progress_file_state(self) -> object:
        if self.progress_file is None:
            return None
        try:
            stat = self.progress_file.stat()
        except OSError:
            return "absent"
        return (stat.st_mtime_ns, stat.st_size)

    def sample(self) -> Optional[str]:
        """Read the tree once; return ``"stall"`` or ``"deadline"`` on a breach."""
        assert self.root is not None
        self.next_due = time.monotonic() + self.interval
        if self.record.get("tree_readable") is False:
            now = time.monotonic()
            self.last_progress = now  # unmeasurable: never a stall
            if self.deadline is not None and now - self.started >= self.deadline:
                return self._breached("deadline", now, None)
            return None
        try:
            reading = sample_tree(self.root)
        except Exception as error:  # a reading that fails never stops the command
            self.record["sample_errors"] += 1
            self.record["last_sample_error"] = repr(error)
            now = time.monotonic()
            self.last_progress = now  # unmeasured counts as progress
            if self.deadline is not None and now - self.started >= self.deadline:
                return self._breached("deadline", now, None)
            return None
        if not reading["pids"]:
            return None  # the child is exiting; the wait reaps it
        now = time.monotonic()
        self.record["samples"] += 1
        key = (reading["fingerprint"], self._progress_file_state())
        if reading["io_unreadable"]:
            # Unmeasurable I/O counts as progress: never stop what cannot be read.
            self.record["samples_unmeasured"] += 1
            self.record["io_unreadable_pids"] = reading["io_unreadable"][:16]
            if not self.warned_unmeasured:
                self.warned_unmeasured = True
                self.echo(f"gate_slot: cannot read the I/O of pid(s) "
                          f"{reading['io_unreadable'][:8]}; the stall deadline counts "
                          "those readings as progress")
        if key != self.last_key or reading["io_unreadable"]:
            self.last_key = key
            self.last_progress = now
            self.last_progress_at = utc_now()
            self.cpu_at_progress = reading["cpu_seconds"]
        self._resident(reading, now)
        live = {"at": utc_now(), "processes": len(reading["pids"]),
                "cpu_seconds": round(reading["cpu_seconds"], 2), "io": reading["io"],
                "tree_rss_kib": reading["tree_rss_kib"],
                "peak_tree_rss_kib": self.record["peak_tree_rss_kib"],
                "admission_mib": self.admission_mib,
                "last_progress_at": self.last_progress_at,
                "stall_timeout_seconds": self.stall, "deadline_seconds": self.deadline}
        self.record["last_sample"] = live
        if self.on_sample is not None:
            try:
                self.on_sample(live)
            except Exception as error:  # diagnostics only
                self.record["note_errors"] = repr(error)
        if self.stall is not None and now - self.last_progress >= self.stall:
            return self._breached("stall", now, reading)
        if self.deadline is not None and now - self.started >= self.deadline:
            return self._breached("deadline", now, reading)
        if self.rss_stop_mib is not None and reading["tree_rss_kib"] > self.rss_stop_mib * 1024:
            return self._breached("memory", now, reading)
        return None

    def _resident(self, reading: dict, now: float) -> None:
        rss = reading["tree_rss_kib"]
        if rss > self.record["peak_tree_rss_kib"]:
            self.record["peak_tree_rss_kib"] = rss
            self.record["peak_at"] = utc_now()
        if self.admission_mib is None or rss <= self.admission_mib * 1024:
            return
        over = self.record.setdefault("rss_over_admission", {
            "first_at": utc_now(), "first_after_seconds": round(now - self.started, 1),
            "first_tree_rss_kib": rss, "largest_pid": reading["largest_pid"],
            "largest_rss_kib": reading["largest_rss_kib"], "samples_over": 0})
        over["samples_over"] += 1
        over["peak_tree_rss_kib"] = self.record["peak_tree_rss_kib"]
        if rss >= self.reported_rss_kib + (GIB_KIB if self.reported_rss_kib else 0):
            self.reported_rss_kib = rss
            self.echo(f"gate_slot: resident set {rss} KiB across {len(reading['pids'])} "
                      f"process(es) (largest pid {reading['largest_pid']}, "
                      f"{reading['largest_rss_kib']} KiB) exceeds the admission budget "
                      f"{self.admission_mib} MiB, {format_duration(now - self.started)} "
                      "into the run")

    def _breached(self, kind: str, now: float, reading: Optional[dict]) -> str:
        quiet = now - self.last_progress
        table = process_table()
        processes = [{"pid": pid, "comm": table[pid]["comm"], "state": table[pid]["state"],
                      "argv": (read_cmdline(pid) or [])[:6]}
                     for pid in (reading or {}).get("pids", [])[:32] if pid in table]
        self.breach = {
            "kind": kind, "at": utc_now(),
            "run_seconds": round(now - self.started, 1),
            "last_progress_at": self.last_progress_at,
            "seconds_without_progress": round(quiet, 1),
            "cpu_seconds_since_progress": (round(reading["cpu_seconds"] - self.cpu_at_progress, 2)
                                           if reading else None),
            "tree_rss_kib": reading["tree_rss_kib"] if reading else None,
            "processes": processes, "signals": [],
        }
        self.record["breach"] = self.breach
        if kind == "stall":
            why = (f"no progress for {format_duration(quiet)} (no output, no progress-file "
                   f"change and no I/O by its {len(reading['pids'])} process(es) since "
                   f"{self.last_progress_at}; CPU {self.breach['cpu_seconds_since_progress']}s "
                   f"in that window) against --stall-timeout {format_duration(self.stall)}")
        elif kind == "memory":
            why = (f"its process tree's resident set is {reading['tree_rss_kib']} KiB, over the "
                   f"short lane's hard stop of {self.rss_stop_mib} MiB (a job that needs more "
                   "is an audit: use the audit class)")
        else:
            why = (f"ran {format_duration(now - self.started)} against --deadline "
                   f"{format_duration(self.deadline)}")
        self.echo(f"gate_slot: stopping its own command, pid {self.root}: {why}; SIGTERM to "
                  f"its process tree now, SIGKILL after {format_duration(self.grace)}"
                  + (f"; receipt {self.receipt}" if self.receipt else ""))
        return kind

    def stop(self, wait: Callable[[Optional[float]], Optional[tuple]]) -> Optional[tuple]:
        """SIGTERM the child and its descendants, wait ``grace``, SIGKILL the
        rest.  Only the child (not yet reaped, so its pid is its own) and
        processes found under it, re-identified by start time, are signalled.
        Returns the child's wait result when it was reaped here."""
        assert self.root is not None and self.breach is not None
        table = process_table()
        members = {pid: table[pid]["starttime"] for pid in tree_pids(self.root, table)}
        signals = self.breach["signals"]

        def send(signum: int, targets: dict, reaped: bool) -> None:
            for pid, starttime in targets.items():
                if pid == self.root and not reaped:
                    try:
                        os.kill(pid, signum)
                        signals.append({"pid": pid, "signal": signum, "at": utc_now()})
                    except ProcessLookupError:
                        pass
                elif pid != self.root and signal_verified(pid, starttime, signum):
                    signals.append({"pid": pid, "signal": signum, "at": utc_now()})

        send(signal.SIGTERM, members, False)
        limit = time.monotonic() + self.grace
        result = wait(self.grace)
        while time.monotonic() < limit and any(
                alive(pid, start) for pid, start in members.items() if pid != self.root):
            time.sleep(min(0.1, max(0.0, limit - time.monotonic())))
        if result is None:
            table = process_table()
            for pid in tree_pids(self.root, table):
                members.setdefault(pid, table[pid]["starttime"])
        survivors = {pid: start for pid, start in members.items()
                     if (pid == self.root and result is None)
                     or (pid != self.root and alive(pid, start))}
        send(signal.SIGKILL, survivors, result is not None)
        self.breach["killed"] = sorted(survivors)
        return result


def _open_pidfd(pid: int) -> Optional[int]:
    try:
        return os.pidfd_open(pid)
    except (AttributeError, OSError):
        return None


def _wait_child(pid: int, pidfd: Optional[int], timeout: Optional[float]) -> Optional[tuple]:
    """``wait4`` the child within ``timeout`` seconds (None: until it exits);
    None when it is still running."""
    if timeout is None:
        _, status, usage = os.wait4(pid, 0)
        return status, usage
    if pidfd is not None:
        poller = select.poll()
        poller.register(pidfd, select.POLLIN)
        if not poller.poll(int(timeout * 1000)):
            return None
        _, status, usage = os.wait4(pid, 0)
        return status, usage
    limit = time.monotonic() + timeout  # pragma: no cover - no pidfd (not Linux)
    while True:
        found, status, usage = os.wait4(pid, os.WNOHANG)
        if found:
            return status, usage
        if time.monotonic() >= limit:
            return None
        time.sleep(min(0.2, max(0.0, limit - time.monotonic())))


def run_child(argv: Sequence[str], env: dict[str, str],
              monitor: Optional[Monitor] = None) -> tuple[int, int]:
    """(exit code, peak resident set in KiB).  The peak is ``wait4``'s
    ``ru_maxrss``: the largest single process among the child and every
    descendant it waited for (for an audit route, the ``quirin-audit``
    process), not their sum.  With a ``monitor``, the tree is sampled while it
    runs and stopped on a breach (``monitor.breach``)."""
    process = subprocess.Popen(list(argv), env=env)
    forwarded: dict[int, object] = {}
    reaped = False

    def forward(signum, _frame):
        if reaped:
            return
        try:
            os.kill(process.pid, signum)  # not send_signal: its poll() could reap
        except ProcessLookupError:
            pass

    for signum in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP):
        forwarded[signum] = signal.signal(signum, forward)
    pidfd = _open_pidfd(process.pid) if monitor is not None else None
    try:
        result = None
        if monitor is None:
            result = _wait_child(process.pid, None, None)
        else:
            monitor.start(process.pid)
            while result is None:
                result = _wait_child(process.pid, pidfd, monitor.wait_budget())
                if result is None and monitor.sample() is not None:
                    result = monitor.stop(
                        lambda timeout: _wait_child(process.pid, pidfd, timeout))
                    if result is None:
                        result = _wait_child(process.pid, pidfd, None)
        status, usage = result
        reaped = True
        process.returncode = os.waitstatus_to_exitcode(status)
    finally:
        if pidfd is not None:
            os.close(pidfd)
        for signum, previous in forwarded.items():
            signal.signal(signum, previous)  # type: ignore[arg-type]
    code = process.returncode
    return (128 - code if code < 0 else code), usage.ru_maxrss


WAIT_SCHEMA = "quirin-gate-wait/1"


def write_wait_record(path: Optional[Path], state: str, cls: str, **fields) -> None:
    """Replace the live wait record whole: write a sibling, then rename it."""
    if path is None:
        return
    document = dict(fields, schema=WAIT_SCHEMA, state=state, pid=os.getpid(),
                    at=utc_now())
    document["class"] = cls
    path.parent.mkdir(parents=True, exist_ok=True)
    partial = path.with_name(f".{path.name}.{os.getpid()}.partial")
    partial.write_text(json.dumps(document, sort_keys=True) + "\n", encoding="utf-8")
    os.replace(partial, path)


def default_receipt(cls: str) -> Path:
    stamp = datetime.datetime.now(datetime.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    return REPO_ROOT / "private" / "gate-slot" / f"{stamp}-{os.getpid()}-{cls}.json"


def utc_now() -> str:
    return datetime.datetime.now(datetime.timezone.utc).isoformat(timespec="seconds")


def parse_args(argv: Sequence[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Run a heavy gate job in a named slot (card GATE-ELASTIC). "
                    "`gate_slot.py status` lists live holders and waiters.")
    parser.add_argument("cls", metavar="class", choices=CLASSES,
                        help="rust-suite (1 slot, holds every audit slot), audit "
                             "(memory-budget slots), lean (2 slots) or short (the lane for "
                             f"CPU-bound, memory-light jobs of at most "
                             f"{format_duration(SHORT_WALL_CAP)}: prepare, build, link; "
                             f"{SHORT_SLOTS} slots, admitted ahead of queued audits)")
    parser.add_argument("--memory-budget", metavar="7G|5G|4G|1G|2G",
                        help="audit class: per-audit budget 7G, 5G or 4G (default 7G); "
                             "short class: the memory the job declares, 1G or 2G (default 1G)")
    parser.add_argument("--max-wall", metavar="DURATION",
                        help="short class only: the job's wall bound; the command is stopped "
                             f"(exit 124) after it (default {format_duration(DEFAULT_SHORT_WALL)}, "
                             f"at most {format_duration(SHORT_WALL_CAP)}; it cannot be switched off)")
    parser.add_argument("--receipt", type=Path,
                        help="where the JSON receipt goes "
                             "(default private/gate-slot/<stamp>-<pid>-<class>.json)")
    parser.add_argument("--wait-record", type=Path,
                        help="keep a live record of the slot wait here (waiting, then "
                             "admitted with waited_seconds)")
    parser.add_argument("--wait-timeout", type=float,
                        help="give up (exit 75) after this many seconds without a slot")
    parser.add_argument("--stall-timeout", metavar="DURATION",
                        help="stop the command (exit 124) after this long without output, "
                             "progress-file change or I/O by any process of its tree "
                             f"(default: audit {format_duration(DEFAULT_STALL_TIMEOUT['audit'])}, "
                             f"other classes none; 0/off disables; ${STALL_ENV} when absent)")
    parser.add_argument("--deadline", metavar="DURATION",
                        help="stop the command (exit 124) this long after it started "
                             f"(default none; ${DEADLINE_ENV} when absent)")
    parser.add_argument("--progress-file", type=Path,
                        help="a file the command updates as it works; a change of its size "
                             "or mtime counts as progress")
    parser.add_argument("--stop-grace", metavar="DURATION",
                        help="between SIGTERM and SIGKILL when stopping "
                             f"(default {format_duration(DEFAULT_STOP_GRACE)})")
    parser.add_argument("--drain-timeout", metavar="DURATION",
                        help="rust-suite only: the most one drain waits for running audits "
                             f"before it reopens admission (default "
                             f"{format_duration(DEFAULT_DRAIN_TIMEOUT)}; 0/off: unbounded; "
                             f"${DRAIN_ENV} when absent)")
    parser.add_argument("--drain-backoff", metavar="DURATION",
                        help="rust-suite only: the pause between drain attempts "
                             f"(default {format_duration(DEFAULT_DRAIN_BACKOFF)})")
    parser.add_argument("--drain-attempts", type=int, default=None,
                        help="rust-suite only: fail (exit 75) when this many drains gave up "
                             "(default 0: retry until --wait-timeout)")
    parser.add_argument("--poll", type=float, default=2.0, help=argparse.SUPPRESS)
    parser.add_argument("--sample-interval", type=float, default=SAMPLE_INTERVAL,
                        help=argparse.SUPPRESS)
    parser.usage = "%(prog)s class [options] -- command [args...]\n       %(prog)s status [--json] [--interval SECONDS]"
    argv = list(argv)
    if "--" not in argv:
        parser.error("no command given after --")
    split = argv.index("--")
    args = parser.parse_args(argv[:split])
    args.command = argv[split + 1:]
    if not args.command:
        parser.error("no command given after --")
    return args


def resolve_limits(args: argparse.Namespace, env: dict[str, str]) -> dict:
    """The deadlines this job runs under: the flag, else the variable, else the
    class default; each with where it came from."""
    def pick(flag: Optional[str], variable: Optional[str], default: Optional[float],
             what: str, allow_zero: bool = False) -> tuple[Optional[float], str]:
        if flag is not None:
            return parse_duration(flag, what, allow_zero), "flag"
        if variable is not None and env.get(variable, "").strip():
            return parse_duration(env[variable], f"${variable}", allow_zero), "environment"
        return default, "default"

    rust = args.cls == "rust-suite"
    for name in ("drain_timeout", "drain_backoff", "drain_attempts"):
        if getattr(args, name) is not None and not rust:
            raise Refusal(f"--{name.replace('_', '-')} applies to the rust-suite class only")
    if args.drain_attempts is not None and args.drain_attempts < 0:
        raise Refusal("--drain-attempts must be 0 (unlimited) or more")
    if args.sample_interval <= 0:
        raise Refusal("--sample-interval must be positive")
    short = args.cls == SHORT
    if args.max_wall is not None and not short:
        raise Refusal("--max-wall applies to the short class only")
    stall, stall_from = pick(args.stall_timeout, STALL_ENV, DEFAULT_STALL_TIMEOUT[args.cls],
                             "--stall-timeout")
    if short:
        # The lane's wall bound is not optional: it is what lets a rust-suite
        # drain wait for lane jobs.  --deadline and the environment's deadline
        # do not apply; the bound is --max-wall.
        if args.deadline is not None:
            raise Refusal("--deadline does not apply to the short class: use --max-wall "
                          f"(at most {format_duration(SHORT_WALL_CAP)})")
        if args.max_wall is None:
            deadline, deadline_from = DEFAULT_SHORT_WALL, "default"
        else:
            deadline, deadline_from = parse_duration(args.max_wall, "--max-wall"), "flag"
        if deadline is None or deadline > SHORT_WALL_CAP:
            raise Refusal(f"--max-wall must be a duration of at most "
                          f"{format_duration(SHORT_WALL_CAP)} and cannot be 0/off; a longer "
                          "job is not a short job: use the audit class")
    else:
        deadline, deadline_from = pick(args.deadline, DEADLINE_ENV, DEFAULT_DEADLINE[args.cls],
                                       "--deadline")
    grace, _ = pick(args.stop_grace, None, DEFAULT_STOP_GRACE, "--stop-grace", True)
    if short and grace is not None and grace > SHORT_GRACE_CAP:
        raise Refusal(f"--stop-grace for the short class is at most "
                      f"{format_duration(SHORT_GRACE_CAP)}")
    lapse, lapse_from = pick(None, LAPSE_ENV, DEFAULT_QUEUE_LAPSE, "queue lapse")
    queue = env.get(QUEUE_ENV, "").strip().lower() not in ("off", "0", "no", "none")
    limits = {"stall_timeout_seconds": stall, "stall_timeout_from": stall_from,
              "deadline_seconds": deadline, "deadline_from": deadline_from,
              "stop_grace_seconds": grace, "queue": queue,
              "queue_lapse_seconds": lapse, "queue_lapse_from": lapse_from}
    if short:
        limits.update(max_wall_seconds=deadline, max_wall_from=deadline_from,
                      max_wall_cap_seconds=SHORT_WALL_CAP,
                      hard_memory_stop_mib=SHORT_HARD_STOP_MIB)
    if rust:
        drain, drain_from = pick(args.drain_timeout, DRAIN_ENV, DEFAULT_DRAIN_TIMEOUT,
                                 "--drain-timeout")
        backoff, _ = pick(args.drain_backoff, None, DEFAULT_DRAIN_BACKOFF, "--drain-backoff",
                          True)
        limits.update(drain_timeout_seconds=drain, drain_timeout_from=drain_from,
                      drain_backoff_seconds=backoff, drain_attempts=args.drain_attempts or 0)
    return limits


def write_receipt(path: Path, receipt: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(receipt, indent=1, sort_keys=True) + "\n", encoding="utf-8")


def main(argv: Optional[Sequence[str]] = None) -> int:
    argv = list(sys.argv[1:] if argv is None else argv)
    if argv[:1] == ["status"]:
        return status_main(argv[1:])
    args = parse_args(argv)
    gate_dir = Path(os.environ.get("QUIRIN_GATE_DIR", DEFAULT_GATE_DIR))
    legacy = Path(os.environ.get("QUIRIN_GATE_LEGACY_LOCK", LEGACY_LOCK))
    meminfo = Path(os.environ.get("QUIRIN_GATE_MEMINFO", MEMINFO))
    receipt_path = (args.receipt or default_receipt(args.cls)).resolve()
    receipt: dict = {"schema": "quirin-gate-slot/1", "class": args.cls,
                     "argv_requested": args.command, "cwd": os.getcwd(), "pid": os.getpid()}
    env = dict(os.environ)
    argv_run = list(args.command)
    try:
        if args.memory_budget is not None and args.cls not in ("audit", SHORT):
            raise Refusal("--memory-budget applies to the audit and short classes only")
        limits = resolve_limits(args, env)
        receipt["limits"] = limits
        budget_mib = None
        admission_mib = None
        if args.cls == SHORT:
            label = (args.memory_budget or DEFAULT_SHORT_BUDGET).upper()
            budget_mib = admission_mib = parse_short_budget(label)
            receipt["budget"] = {"label": label, "memory_budget_mib": budget_mib,
                                 "admission_mib": admission_mib, "route_class": None,
                                 "mechanism": "declared (not applied to the command)"}
        if args.cls == "audit":
            label = (args.memory_budget or DEFAULT_BUDGET).upper()
            budget_mib = parse_budget(label)
            argv_run, env, budget = apply_budget(argv_run, env, budget_mib, Path.cwd())
            admission_mib, route = admission_budget(argv_run, budget_mib)
            receipt["budget"] = dict(budget, label=label, admission_mib=admission_mib,
                                     route_class=route)
        held = parse_held(env.get(HELD_ENV))
        if held:
            parent = held[0].split("/", 1)[0]
            if args.cls not in COVERS.get(parent, set()):
                raise Refusal(
                    f"nested {args.cls} slot under a held {parent} slot ({','.join(held)}) "
                    "would deadlock or bypass the accounting"
                )
    except Refusal as error:
        print(f"gate_slot: refused: {error}", file=sys.stderr)
        return 2

    gate = Gate(gate_dir, legacy, meminfo, poll=args.poll, queue=limits["queue"],
                queue_lapse=limits["queue_lapse_seconds"])
    wait_record = args.wait_record.resolve() if args.wait_record else None
    if held:
        receipt["nested_under"] = held
        receipt["slot_record"] = {"class": args.cls, "covered_by_parent": True}
        write_wait_record(wait_record, "admitted", args.cls, waited_seconds=0.0)
    else:
        try:
            write_wait_record(wait_record, "waiting", args.cls)
            holder = {"argv": args.command[:6], "cwd": os.getcwd(), "since": utc_now()}
            if admission_mib is not None:
                holder["admission_mib"] = admission_mib
            receipt["slot_record"] = gate.acquire(
                args.cls, admission_mib, args.wait_timeout, holder=holder,
                drain_timeout=limits.get("drain_timeout_seconds"),
                drain_backoff=limits.get("drain_backoff_seconds", DEFAULT_DRAIN_BACKOFF),
                drain_attempts=limits.get("drain_attempts", 0))
        except (WaitTimeout, DrainFailed) as error:
            # Nothing ran; the receipt names what blocked the slot.
            print(f"gate_slot: {error}", file=sys.stderr)
            receipt["slot_record"] = gate.record
            receipt["ran"] = False
            receipt["exit"] = WAIT_FAILED_EXIT
            receipt["failure"] = str(error)
            receipt["finished"] = utc_now()
            write_receipt(receipt_path, receipt)
            print(f"gate_slot: exit {WAIT_FAILED_EXIT}, nothing ran; receipt {receipt_path}",
                  file=sys.stderr, flush=True)
            return WAIT_FAILED_EXIT
        except KeyboardInterrupt:
            print("gate_slot: interrupted while waiting; nothing ran", file=sys.stderr)
            return 130
        except Refusal as error:
            print(f"gate_slot: refused: {error}", file=sys.stderr)
            return 2
        env[HELD_ENV] = ",".join(gate.record["held"][1:])  # slots, not the legacy lock
        write_wait_record(wait_record, "admitted", args.cls,
                          waited_seconds=gate.record["waited_seconds"])
    env["QUIRIN_GATE_CLASS"] = args.cls
    receipt["argv_run"] = argv_run
    receipt["started"] = utc_now()
    slot = receipt["slot_record"].get("slot")
    print(f"gate_slot: {args.cls} slot {slot if slot is not None else '(parent)'}"
          + (f", budget {receipt['budget']['memory_budget_mib']} MiB, admitted at "
             f"{receipt['budget']['admission_mib']} MiB"
             if "budget" in receipt else "")
          + f"; stall timeout {format_duration(limits['stall_timeout_seconds'])}, "
            f"deadline {format_duration(limits['deadline_seconds'])}",
          file=sys.stderr, flush=True)
    monitor = Monitor(stall=limits["stall_timeout_seconds"], deadline=limits["deadline_seconds"],
                      interval=args.sample_interval, grace=limits["stop_grace_seconds"],
                      progress_file=args.progress_file.resolve() if args.progress_file else None,
                      admission_mib=admission_mib, on_sample=gate.update_note,
                      receipt=receipt_path,
                      rss_stop_mib=SHORT_HARD_STOP_MIB if args.cls == SHORT else None)
    peak_kib = None
    try:
        code, peak_kib = run_child(argv_run, env, monitor)
    except OSError as error:
        print(f"gate_slot: cannot run {argv_run[0]}: {error}", file=sys.stderr)
        code = 127
    finally:
        gate.release()
    receipt["finished"] = utc_now()
    receipt["monitor"] = monitor.record
    if monitor.breach is not None:
        receipt["child_exit"] = code
        receipt["stopped"] = monitor.breach["kind"]
        code = STOPPED_EXIT
    receipt["exit"] = code
    if args.cls == SHORT:
        stopped_by = monitor.breach["kind"] if monitor.breach is not None else None
        receipt["lane"] = {
            "max_wall_seconds": limits["max_wall_seconds"],
            "ran_seconds": monitor.breach["run_seconds"] if monitor.breach is not None else None,
            "wall_overrun": stopped_by == "deadline", "stopped_by": stopped_by,
            # Only an exit 0 that was not stopped is a finished job.  A stopped
            # command may have left a half-written tree: the exit is 124 so no
            # caller reads success, and the lane does not clean up after it.
            "complete": code == 0 and stopped_by is None,
            "memory_over_declared": monitor.record.get("rss_over_admission") is not None}
    if peak_kib is not None:
        receipt["max_rss_kib"] = peak_kib
        if admission_mib is not None:
            within = peak_kib <= admission_mib * 1024
            receipt["max_rss_within_admission"] = within
            if not within:
                print(f"gate_slot: peak resident set {peak_kib} KiB exceeds the admission "
                      f"budget {admission_mib} MiB", file=sys.stderr, flush=True)
    over = monitor.record.get("rss_over_admission")
    if over is not None:
        print(f"gate_slot: the process tree's resident set exceeded the admission budget "
              f"{admission_mib} MiB in {over['samples_over']} sample(s) from "
              f"{over['first_at']}, peak {over['peak_tree_rss_kib']} KiB",
              file=sys.stderr, flush=True)
    write_receipt(receipt_path, receipt)
    stopped = (f" (stopped: {receipt['stopped']}; the command's own exit {receipt['child_exit']})"
               if monitor.breach is not None else "")
    print(f"gate_slot: exit {code}{stopped}; receipt {receipt_path}", file=sys.stderr, flush=True)
    return code


# --- status ----------------------------------------------------------------------


def boot_time() -> Optional[float]:
    try:
        for line in (PROC / "stat").read_text(encoding="ascii").splitlines():
            if line.startswith("btime "):
                return float(line.split()[1])
    except (OSError, ValueError, IndexError):
        pass
    return None


def slot_class_of(argv: Optional[list[str]]) -> Optional[str]:
    """The class a ``gate_slot.py`` command line asks for, if it is one."""
    if not argv:
        return None
    for index, arg in enumerate(argv):
        if Path(arg).name == "gate_slot.py":
            return next((item for item in argv[index + 1:] if item in CLASSES), None)
    return None


def compress(labels: list[str]) -> str:
    """``audit/0, audit/1, audit/2`` -> ``audit/0-2``."""
    groups: dict[str, list] = {}
    for label in labels:
        cls, _, index = label.partition("/")
        groups.setdefault(cls, []).append(index)
    parts = []
    for cls, indices in groups.items():
        numbers = sorted(int(i) for i in indices if i.isdigit())
        named = [i for i in indices if not i.isdigit()]
        runs = []
        for number in numbers:
            if runs and number == runs[-1][1] + 1:
                runs[-1][1] = number
            else:
                runs.append([number, number])
        spans = [f"{a}" if a == b else f"{a}-{b}" for a, b in runs] + named
        parts.append(f"{cls}/{','.join(spans)}" if spans else cls)
    return " ".join(parts)


def gate_status(gate_dir: Path, legacy: Path, meminfo: Path, interval: float = 5.0,
                locks_path: Path = PROC_LOCKS) -> dict:
    """Live holders and waiters of the gate's slots (``/proc/locks`` entries
    on the gate's files), with each job tree's CPU and I/O over ``interval``
    seconds and its resident set against its admission budget."""
    locks = gate_locks(gate_dir, legacy, locks_path)
    if locks is None:
        return {"schema": "quirin-gate-status/1", "error": f"{locks_path} is not readable"}
    try:
        memory = read_meminfo(meminfo)
    except (OSError, Refusal):
        memory = {}
    ceiling = audit_slot_ceiling(memory["MemTotal"]) if "MemTotal" in memory else None
    jobs: dict[int, dict] = {}
    for entry in locks:
        job = jobs.setdefault(entry["pid"], {"pid": entry["pid"], "holds": [], "queued_for": []})
        mode = "exclusive" if entry["mode"] == "WRITE" else "shared"
        if entry["queued"]:
            job["queued_for"].append({"lock": entry["label"], "mode": mode})
        else:
            job["holds"].append({"lock": entry["label"], "mode": mode})
    queues = queue_status(gate_dir)
    for pid in queues["by_pid"]:
        jobs.setdefault(pid, {"pid": pid, "holds": [], "queued_for": []})
    table = process_table()
    before = {pid: sample_tree(pid, include_root=False, table=table) for pid in jobs}
    if interval > 0:
        time.sleep(interval)
    table = process_table()
    boot = boot_time()
    now = time.time()
    holders_of: dict[str, list[int]] = {}
    for pid, job in jobs.items():
        for held in job["holds"]:
            holders_of.setdefault(held["lock"], []).append(pid)
    for pid, job in jobs.items():
        slots = [held["lock"] for held in job["holds"] if held["lock"] != "legacy"]
        legacy_mode = next((held["mode"] for held in job["holds"] if held["lock"] == "legacy"),
                           None)
        argv = read_cmdline(pid)
        note = None
        for label in slots:
            candidate = read_note(gate_dir / label)
            if candidate is not None and candidate.get("pid") == pid:
                note = candidate
                break
        ticket = queues["by_pid"].get(pid)
        job["slots"] = slots
        job["class"] = (note or {}).get("cls") or (ticket or {}).get("class") \
            or slot_class_of(argv)
        job["argv"] = (note or {}).get("argv") or (ticket or {}).get("argv") or argv
        job["cwd"] = (note or {}).get("cwd")
        job["queue"] = ticket
        job["alive"] = pid in table
        if pid in table and boot is not None:
            job["process_started_at"] = datetime.datetime.fromtimestamp(
                boot + table[pid]["starttime"] / CLOCK_TICKS, datetime.timezone.utc
            ).isoformat(timespec="seconds")
        requested = (note or {}).get("since") or (ticket or {}).get("requested_at") \
            or job.get("process_started_at")
        job["requested_at"] = requested
        if note is not None:
            job["admitted_at"] = note.get("noted_at")
            job["admission_mib"] = note.get("admission_mib")
            if note.get("live"):
                job["live"] = note["live"]
        if job["queued_for"]:
            job["role"] = "queued in the kernel for " + ", ".join(
                f"{q['lock']} ({q['mode']})" for q in job["queued_for"])
        elif slots and job["class"] == "rust-suite" and ceiling is not None and \
                "audit/admission" in slots and \
                len([s for s in slots if s.startswith("audit/") and s != "audit/admission"]) < ceiling:
            missing = [f"audit/{i}" for i in range(ceiling) if f"audit/{i}" not in slots]
            blockers = [{"lock": label, "pids": holders_of.get(label, [])}
                        for label in missing if holders_of.get(label)]
            job["role"] = "draining (holds audit admission, waits for running audits)"
            job["blocked_by"] = blockers
        elif job["class"] == "rust-suite" and "short/admission" in slots and \
                len([s for s in slots if s.startswith("short/") and s != "short/admission"]) \
                < SHORT_SLOTS:
            blockers = [{"lock": f"short/{i}", "pids": holders_of.get(f"short/{i}", [])}
                        for i in range(SHORT_SLOTS) if f"short/{i}" not in slots
                        and holders_of.get(f"short/{i}")]
            job["role"] = "draining (audits drained; holds lane admission, waits for lane jobs)"
            job["blocked_by"] = blockers
        elif slots:
            job["role"] = "holder"
        elif legacy_mode == "exclusive":
            job["role"] = "legacy holder (flock on the legacy lock)"
        elif ticket is not None and ticket["state"] == "backoff":
            job["role"] = "waiting for a slot (drain gave up; backing off, place kept)"
        elif (legacy_mode == "shared" and job["class"] is not None) or ticket is not None:
            job["role"] = "waiting for a slot" + (
                "" if ticket is not None or job["class"] == "rust-suite" else
                " (not queued: a gate_slot.py without arrival order)")
        else:
            job["role"] = "holds the legacy lock shared"
        waited_from = seconds_since(requested)
        if job["role"].startswith(("holder", "draining")) and job.get("admitted_at"):
            held_for = seconds_since(job["admitted_at"])
            if held_for is not None:
                job["held_seconds"] = round(held_for, 1)
            admitted = seconds_since(job["admitted_at"])
            if waited_from is not None and admitted is not None and \
                    job["role"].startswith("holder"):
                job["waited_seconds"] = round(max(0.0, waited_from - admitted), 1)
        if not job["role"].startswith("holder") and waited_from is not None:
            job["waited_seconds"] = round(waited_from, 1)
        after = sample_tree(pid, include_root=False, table=table)
        job["processes"] = len(after["pids"])
        job["tree_rss_kib"] = after["tree_rss_kib"]
        job["largest_rss_kib"] = after["largest_rss_kib"]
        job["cpu_seconds"] = round(after["cpu_seconds"], 2)
        if pid in before and interval > 0:
            job["over_interval"] = sample_delta(before[pid], after)
        if job.get("admission_mib") is not None:
            job["rss_over_admission"] = after["tree_rss_kib"] > job["admission_mib"] * 1024
    order = {"holder": 0, "draining": 1, "legacy": 2, "waiting": 3, "queued": 4, "holds": 5}
    listed = sorted(jobs.values(), key=lambda job: (
        next((rank for prefix, rank in order.items() if job["role"].startswith(prefix)), 9),
        job["pid"]))
    lane_holders = [job["pid"] for job in listed
                    if job["class"] == SHORT and any(s.startswith("short/") for s in job["slots"])]
    lane_waiters = [job["pid"] for job in listed
                    if job["class"] == SHORT and job["pid"] not in lane_holders
                    and job["role"].startswith(("waiting", "queued"))]
    return {"schema": "quirin-gate-status/1", "at": utc_now(), "gate_dir": str(gate_dir),
            "legacy_lock": str(legacy), "interval_seconds": interval,
            "mem_available_kib": memory.get("MemAvailable"),
            "mem_total_kib": memory.get("MemTotal"), "audit_slot_ceiling": ceiling,
            "short_lane": {"slots": SHORT_SLOTS, "holders": lane_holders,
                           "waiters": lane_waiters},
            "queues": queues["families"], "jobs": listed}


def queue_status(gate_dir: Path, env: Optional[dict] = None) -> dict:
    """The live tickets of every queue (read only: nothing is reclaimed), in
    arrival order with their positions, and the same by owner pid."""
    env = os.environ if env is None else env
    try:
        lapse = parse_duration(env[LAPSE_ENV], f"${LAPSE_ENV}") if env.get(LAPSE_ENV, "").strip() \
            else DEFAULT_QUEUE_LAPSE
    except Refusal:
        lapse = DEFAULT_QUEUE_LAPSE
    families: dict[str, list] = {}
    by_pid: dict[int, dict] = {}
    for family in sorted(set(QUEUE_FAMILY.values())):
        live = scan_queue(gate_dir / family / QUEUE_DIR, lapse, reclaim=False)["live"]
        entries = []
        for position, entry in enumerate(live, 1):
            note = entry["note"]
            info = {"family": family, "position": position, "of": len(live), "seq": entry["seq"],
                    "pid": entry["pid"], "class": entry["cls"], "state": entry["state"],
                    "lapsed": entry["lapsed"], "admission_mib": entry["admission_mib"],
                    "requested_at": note.get("requested_at"), "argv": note.get("argv"),
                    "ahead": [earlier["pid"] for earlier in live[:position - 1]
                              if not earlier["lapsed"] and earlier["state"] != "backoff"]}
            entries.append(info)
            by_pid.setdefault(entry["pid"], info)
        if entries:
            families[family] = entries
    return {"families": families, "by_pid": by_pid}


def render_status(status: dict) -> str:
    if "error" in status:
        return f"gate_slot status: {status['error']}"
    lines = [f"gate {status['gate_dir']} (legacy lock {status['legacy_lock']}) at {status['at']}"]
    if status.get("mem_total_kib"):
        lines[0] += (f"; MemAvailable {status['mem_available_kib'] // 1024} MiB of "
                     f"{status['mem_total_kib'] // 1024} MiB")
    if not status["jobs"]:
        lines.append("no live holder or waiter")
    lane = status.get("short_lane")
    if lane and (lane["holders"] or lane["waiters"]):
        lines.append(f"short lane: {len(lane['holders'])} of {lane['slots']} slots held"
                     + (" by pid " + ", ".join(str(p) for p in lane["holders"])
                        if lane["holders"] else "")
                     + (f"; waiting: pid " + ", ".join(str(p) for p in lane["waiters"])
                        if lane["waiters"] else ""))
    for job in status["jobs"]:
        head = f"pid {job['pid']} {job['class'] or '?'}: {job['role']}"
        if job["slots"]:
            head += f" [{compress(job['slots'])}]"
        if "held_seconds" in job:
            head += f", held {format_duration(job['held_seconds'])}"
        if "waited_seconds" in job:
            head += f", waited {format_duration(job['waited_seconds'])}"
        queued = job.get("queue")
        if queued:
            head += f", queue position {queued['position']} of {queued['of']} ({queued['family']})"
            if queued["state"] != "waiting":
                head += f" [{queued['state']}]"
            if queued["lapsed"]:
                head += " [lapsed: skipped by the others until it polls]"
            if queued["ahead"]:
                head += "; behind pid " + ", ".join(str(pid) for pid in queued["ahead"])
        lines.append(head)
        if job.get("argv"):
            lines.append("    argv: " + " ".join(job["argv"][:8]))
        for blocker in job.get("blocked_by", []):
            lines.append(f"    blocked by {blocker['lock']} (pid "
                         + ", ".join(str(pid) for pid in blocker["pids"]) + ")")
        body = (f"    tree: {job['processes']} process(es), CPU {job['cpu_seconds']}s, "
                f"RSS {job['tree_rss_kib']} KiB")
        if job.get("admission_mib") is not None:
            body += (f" against {job['admission_mib']} MiB admitted"
                     + (" (OVER)" if job.get("rss_over_admission") else ""))
        lines.append(body)
        delta = job.get("over_interval")
        if delta:
            io = delta["io"]
            lines.append(f"    last {format_duration(delta['window_seconds'])}: CPU "
                         f"{delta['cpu_seconds']}s, rchar +{io['rchar']}, wchar +{io['wchar']}, "
                         f"read_bytes +{io['read_bytes']}, write_bytes +{io['write_bytes']}"
                         + (f"; I/O unreadable for {delta['io_unreadable']}"
                            if delta["io_unreadable"] else ""))
        live = job.get("live")
        if live:
            lines.append(f"    job's own last sample {live.get('at')}: last progress "
                         f"{live.get('last_progress_at')}, stall timeout "
                         f"{format_duration(live.get('stall_timeout_seconds'))}, peak RSS "
                         f"{live.get('peak_tree_rss_kib')} KiB")
    return "\n".join(lines)


def status_main(argv: Sequence[str]) -> int:
    parser = argparse.ArgumentParser(
        prog="gate_slot.py status",
        description="List the gate's live slot holders and waiters (kernel locks on the "
                    "gate files), with CPU and I/O over an interval and resident set "
                    "against the admission budget.")
    parser.add_argument("--json", action="store_true", help="print the record as JSON")
    parser.add_argument("--interval", type=float, default=5.0,
                        help="seconds between the two readings the deltas compare (0: none)")
    args = parser.parse_args(list(argv))
    status = gate_status(Path(os.environ.get("QUIRIN_GATE_DIR", DEFAULT_GATE_DIR)),
                         Path(os.environ.get("QUIRIN_GATE_LEGACY_LOCK", LEGACY_LOCK)),
                         Path(os.environ.get("QUIRIN_GATE_MEMINFO", MEMINFO)),
                         max(0.0, args.interval))
    print(json.dumps(status, indent=1, sort_keys=True) if args.json else render_status(status))
    return 1 if "error" in status else 0


if __name__ == "__main__":
    sys.exit(main())

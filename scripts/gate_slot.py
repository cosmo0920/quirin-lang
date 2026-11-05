#!/usr/bin/env python3
"""Run a heavy gate job in a named, numbered slot instead of one global lock.

Card GATE-ELASTIC (seam E of docs/plans/pluggable-cards/README.md; user
decision 5 of 2026-10-24).  Usage::

    gate_slot.py <class> [--memory-budget 7G|5G|4G] [--receipt PATH] [--wait-record PATH] -- <cmd> [args...]

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
command would override, a nested slot the parent does not cover).
"""

from __future__ import annotations

import argparse
import datetime
import errno
import fcntl
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
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
CLASSES = ("rust-suite", "audit", "lean")
# Which classes a held slot already covers, for a nested invocation.
COVERS = {"rust-suite": {"rust-suite", "audit"}, "audit": {"audit"}, "lean": {"lean"}}
HELD_ENV = "QUIRIN_GATE_HELD"


class Refusal(Exception):
    """A request refused before any lock was taken or anything ran."""


# --- arithmetic ---------------------------------------------------------------


def parse_budget(text: str) -> int:
    try:
        return BUDGETS_MIB[text.upper()]
    except KeyError:
        raise Refusal(
            f"--memory-budget must be one of {', '.join(BUDGETS_MIB)} (got {text!r})"
        ) from None


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
        os.ftruncate(self.fd, 0)
        os.pwrite(self.fd, data, 0)
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
    request is admitted or ``timeout`` seconds pass (TimeoutError)."""

    def __init__(self, gate_dir: Path = DEFAULT_GATE_DIR, legacy: Path = LEGACY_LOCK,
                 meminfo: Path = MEMINFO, poll: float = 2.0,
                 echo: Callable[[str], None] = lambda line: print(line, file=sys.stderr,
                                                                  flush=True)):
        self.gate_dir = gate_dir
        self.legacy = legacy
        self.meminfo = meminfo
        self.poll = poll
        self.echo = echo
        self.held: list[Lock] = []
        self.record: dict = {}

    def _wait(self, attempt: Callable[[], bool], what: str, deadline: Optional[float]) -> None:
        announced = False
        while not attempt():
            if deadline is not None and time.monotonic() >= deadline:
                raise TimeoutError(f"timed out waiting for {what}")
            if not announced:
                self.echo(f"gate_slot: waiting for {what}")
                announced = True
            time.sleep(self.poll)

    def _exclusive(self, cls: str, index: "int | str", deadline: Optional[float], holder: dict) -> None:
        lock = Lock(self.gate_dir / cls / str(index), f"{cls}/{index}")
        try:
            self._wait(lambda: lock.try_lock(fcntl.LOCK_EX), f"slot {cls}/{index}", deadline)
        except BaseException:
            lock.release()
            raise
        lock.note(holder)
        self.held.append(lock)

    def _first_free(self, cls: str, count: int, holder: dict) -> Optional[int]:
        for index in range(count):
            lock = Lock(self.gate_dir / cls / str(index), f"{cls}/{index}")
            if lock.try_lock(fcntl.LOCK_EX):
                lock.note(holder)
                self.held.append(lock)
                return index
            lock.release()
        return None

    def acquire(self, cls: str, budget_mib: Optional[int] = None,
                timeout: Optional[float] = None, holder: Optional[dict] = None) -> dict:
        if cls not in CLASSES:
            raise Refusal(f"unknown slot class {cls!r}; classes: {', '.join(CLASSES)}")
        holder = dict(holder or {}, pid=os.getpid(), cls=cls)
        deadline = None if timeout is None else time.monotonic() + timeout
        started = time.monotonic()
        record: dict = {"class": cls, "gate_dir": str(self.gate_dir)}
        try:
            # 1. The legacy interlock, shared: excludes `flock <legacy>` jobs.
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
            memory = read_meminfo(self.meminfo)
            ceiling = audit_slot_ceiling(memory["MemTotal"])
            record["audit_slot_ceiling"] = ceiling
            if cls == "rust-suite":
                self._exclusive("rust-suite", 0, deadline, holder)
                # Close audit admission first so new audits stop starting, then
                # wait for each running audit's slot.
                self._exclusive("audit", "admission", deadline, holder)
                for index in range(ceiling):
                    self._exclusive("audit", index, deadline, holder)
                record.update(slot=0, slot_count=1,
                              audit_slots_held=list(range(ceiling)))
            elif cls == "audit":
                assert budget_mib is not None
                admitted: dict = {}

                def attempt() -> bool:
                    admission = Lock(self.gate_dir / "audit" / "admission", "audit/admission")
                    try:
                        if not admission.try_lock(fcntl.LOCK_SH):
                            return False  # a rust-suite job holds or awaits every slot
                        available = read_meminfo(self.meminfo)["MemAvailable"]
                        count = min(ceiling, audit_slot_count(available, budget_mib))
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
            else:
                count = FIXED_SLOTS[cls]

                def attempt_fixed() -> bool:
                    index = self._first_free(cls, count, holder)
                    if index is None:
                        return False
                    record.update(slot=index, slot_count=count)
                    return True

                self._wait(attempt_fixed, f"a {cls} slot", deadline)
            final = read_meminfo(self.meminfo)
            record.setdefault("mem_available_kib", final["MemAvailable"])
            record["mem_total_kib"] = final["MemTotal"]
        except BaseException:
            self.release()
            raise
        record["held"] = [lock.label for lock in self.held]
        record["waited_seconds"] = round(time.monotonic() - started, 3)
        self.record = record
        return record

    def release(self) -> None:
        while self.held:
            self.held.pop().release()


# --- running ----------------------------------------------------------------------


def parse_held(text: Optional[str]) -> list[str]:
    return [item for item in (text or "").split(",") if item]


def run_child(argv: Sequence[str], env: dict[str, str]) -> tuple[int, int]:
    """(exit code, peak resident set in KiB).  The peak is ``wait4``'s
    ``ru_maxrss``: the largest single process among the child and every
    descendant it waited for (for an audit route, the ``quirin-audit``
    process), not their sum."""
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
    try:
        _, status, usage = os.wait4(process.pid, 0)
        reaped = True
        process.returncode = os.waitstatus_to_exitcode(status)
    finally:
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
        description="Run a heavy gate job in a named slot (card GATE-ELASTIC).")
    parser.add_argument("cls", metavar="class", choices=CLASSES,
                        help="rust-suite (1 slot, holds every audit slot), audit "
                             "(memory-budget slots) or lean (2 slots)")
    parser.add_argument("--memory-budget", metavar="7G|5G|4G",
                        help="audit class only: per-audit budget (default 7G)")
    parser.add_argument("--receipt", type=Path,
                        help="where the JSON receipt goes "
                             "(default private/gate-slot/<stamp>-<pid>-<class>.json)")
    parser.add_argument("--wait-record", type=Path,
                        help="keep a live record of the slot wait here (waiting, then "
                             "admitted with waited_seconds)")
    parser.add_argument("--wait-timeout", type=float,
                        help="give up (exit 75) after this many seconds without a slot")
    parser.add_argument("--poll", type=float, default=2.0, help=argparse.SUPPRESS)
    parser.usage = "%(prog)s class [options] -- command [args...]"
    argv = list(argv)
    if "--" not in argv:
        parser.error("no command given after --")
    split = argv.index("--")
    args = parser.parse_args(argv[:split])
    args.command = argv[split + 1:]
    if not args.command:
        parser.error("no command given after --")
    return args


def main(argv: Optional[Sequence[str]] = None) -> int:
    args = parse_args(list(sys.argv[1:] if argv is None else argv))
    gate_dir = Path(os.environ.get("QUIRIN_GATE_DIR", DEFAULT_GATE_DIR))
    legacy = Path(os.environ.get("QUIRIN_GATE_LEGACY_LOCK", LEGACY_LOCK))
    meminfo = Path(os.environ.get("QUIRIN_GATE_MEMINFO", MEMINFO))
    receipt_path = (args.receipt or default_receipt(args.cls)).resolve()
    receipt: dict = {"schema": "quirin-gate-slot/1", "class": args.cls,
                     "argv_requested": args.command, "cwd": os.getcwd(), "pid": os.getpid()}
    env = dict(os.environ)
    argv_run = list(args.command)
    try:
        if args.memory_budget is not None and args.cls != "audit":
            raise Refusal("--memory-budget applies to the audit class only")
        budget_mib = None
        admission_mib = None
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

    gate = Gate(gate_dir, legacy, meminfo, poll=args.poll)
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
                args.cls, admission_mib, args.wait_timeout, holder=holder)
        except TimeoutError as error:
            print(f"gate_slot: {error}", file=sys.stderr)
            return 75
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
             if "budget" in receipt else ""), file=sys.stderr, flush=True)
    peak_kib = None
    try:
        code, peak_kib = run_child(argv_run, env)
    except OSError as error:
        print(f"gate_slot: cannot run {argv_run[0]}: {error}", file=sys.stderr)
        code = 127
    finally:
        gate.release()
    receipt["finished"] = utc_now()
    receipt["exit"] = code
    if peak_kib is not None:
        receipt["max_rss_kib"] = peak_kib
        if admission_mib is not None:
            within = peak_kib <= admission_mib * 1024
            receipt["max_rss_within_admission"] = within
            if not within:
                print(f"gate_slot: peak resident set {peak_kib} KiB exceeds the admission "
                      f"budget {admission_mib} MiB", file=sys.stderr, flush=True)
    receipt_path.parent.mkdir(parents=True, exist_ok=True)
    receipt_path.write_text(json.dumps(receipt, indent=1, sort_keys=True) + "\n",
                            encoding="utf-8")
    print(f"gate_slot: exit {code}; receipt {receipt_path}", file=sys.stderr, flush=True)
    return code


if __name__ == "__main__":
    sys.exit(main())

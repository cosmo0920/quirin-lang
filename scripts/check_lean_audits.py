#!/usr/bin/env python3
"""Build the pinned Lean project and run every audit with bounded, visible steps."""
import argparse
from concurrent.futures import ThreadPoolExecutor
import math
import os
from pathlib import Path
import shutil
import signal
import subprocess
import sys
import tempfile
import threading
import time


def positive_seconds(value):
    seconds = float(value)
    if not math.isfinite(seconds) or seconds <= 0:
        raise argparse.ArgumentTypeError("must be a finite positive number of seconds")
    return seconds


def positive_jobs(value):
    jobs = int(value)
    if jobs <= 0:
        raise argparse.ArgumentTypeError("must be a positive number of audit workers")
    return jobs


def default_jobs():
    """Detected CPUs, capped: the phase is bounded by the single longest audit
    (`AuditAll`, which imports the whole project), so beyond a handful of workers
    more concurrency buys nothing and only raises peak memory. Measured here:
    3.57 s at 16 workers against 3.74 s at 8, from 25.50 s at one."""
    return min(os.cpu_count() or 1, 8)


# Every child this run has started and not yet reaped. A worker thread cannot
# observe the main thread's Ctrl-C, so an interrupt stops the trees from here.
_active = set()
_active_lock = threading.Lock()


def stop_process_tree(process):
    """Do not leave Lake's Lean workers running after a timeout or Ctrl-C."""
    if os.name == "posix":
        try:
            os.killpg(process.pid, signal.SIGTERM)
        except ProcessLookupError:
            pass
        try:
            process.wait(timeout=2)
        except subprocess.TimeoutExpired:
            pass
        # The leader can exit before a worker; always clean up its group too.
        try:
            os.killpg(process.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
    else:
        subprocess.run(
            ["taskkill", "/PID", str(process.pid), "/T", "/F"],
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=False,
        )
        if process.poll() is None:
            process.kill()
    process.wait()


def stop_active_process_trees():
    with _active_lock:
        running = list(_active)
    for process in running:
        stop_process_tree(process)


def run_step(command, *, cwd, label, timeout, heartbeat, output=None, quiet=False):
    """Run one bounded child.

    `output` captures the child's streams into a file instead of inheriting this
    process's, which is what lets independent audits share one terminal without
    interleaving; `quiet` withholds the progress lines a worker would otherwise
    print out of turn. Neither changes what is run or how failure is judged.
    """
    if not quiet:
        print(f"Starting {label} (limit {timeout:g}s)", flush=True)
    started = time.monotonic()
    process = subprocess.Popen(
        command, cwd=cwd, start_new_session=os.name == "posix",
        stdout=output, stderr=subprocess.STDOUT if output is not None else None,
    )
    with _active_lock:
        _active.add(process)
    try:
        while True:
            remaining = timeout - (time.monotonic() - started)
            if remaining <= 0:
                raise TimeoutError(f"{label} exceeded {timeout:g}s; stopped its process tree")
            try:
                code = process.wait(timeout=min(heartbeat, remaining))
                break
            except subprocess.TimeoutExpired:
                elapsed = time.monotonic() - started
                if not quiet:
                    print(f"Still running {label}: {elapsed:.1f}s / {timeout:g}s", flush=True)
        if code:
            raise subprocess.CalledProcessError(code, command)
    except BaseException:
        stop_process_tree(process)
        raise
    finally:
        with _active_lock:
            _active.discard(process)
    elapsed = time.monotonic() - started
    if not quiet:
        print(f"Passed {label} in {elapsed:.1f}s", flush=True)
    return elapsed


def audit_command(name):
    return ["lake", "env", "lean", "-DwarningAsError=true", name]


class AuditFailure(Exception):
    """Carry a failed audit's captured output back to the ordered reporter."""

    def __init__(self, cause, output):
        super().__init__(str(cause))
        self.cause = cause
        self.output = output


def run_audits_in_parallel(proof, audits, args, jobs):
    """Check the audits concurrently.

    An audit reads the artifacts a successful build already produced and writes
    none of its own, so checking several at once checks exactly what checking
    them one after another checked. Each child's output is captured and replayed
    in audit order, so the transcript does not depend on completion order, and a
    failure still stops the whole run with that audit's own output shown.
    """
    def one(index, audit):
        label = f"{audit.name} ({index}/{len(audits)})"
        with tempfile.TemporaryFile(mode="w+") as captured:
            try:
                elapsed = run_step(
                    audit_command(audit.name), cwd=proof, label=label,
                    timeout=args.audit_timeout, heartbeat=args.heartbeat,
                    output=captured, quiet=True,
                )
            except (OSError, TimeoutError, subprocess.CalledProcessError) as error:
                captured.seek(0)
                raise AuditFailure(error, captured.read()) from None
            captured.seek(0)
            return label, elapsed, captured.read()

    print(f"Starting {len(audits)} audits, {jobs} at a time "
          f"(limit {args.audit_timeout:g}s each)", flush=True)
    pool = ThreadPoolExecutor(max_workers=jobs)
    futures = [pool.submit(one, index, audit) for index, audit in enumerate(audits, 1)]
    try:
        for future in futures:
            label, elapsed, text = future.result()
            if text:
                print(text, end="", flush=True)
            print(f"Passed {label} in {elapsed:.1f}s", flush=True)
    except BaseException as error:
        for pending in futures:
            pending.cancel()
        stop_active_process_trees()
        if isinstance(error, AuditFailure):
            if error.output:
                print(error.output, end="", flush=True)
            raise error.cause
        raise
    finally:
        pool.shutdown(wait=False)


def check_audits(proof, args):
    audits = sorted(proof.glob("Audit*.lean"))
    if not audits or proof / "AuditAll.lean" not in audits:
        raise ValueError("missing whole-project Lean axiom audit")
    # Set cwd before launching elan: `lake -d proof` alone selects the caller's
    # default toolchain, potentially querying the network for `stable` first.
    if not args.skip_build:
        run_step(
            ["lake", *(["-v"] if args.verbose else []), "build"],
            cwd=proof, label="Lean build", timeout=args.build_timeout,
            heartbeat=args.heartbeat,
        )
    jobs = min(getattr(args, "jobs", None) or 1, len(audits))
    if jobs > 1:
        run_audits_in_parallel(proof, audits, args, jobs)
    else:
        for index, audit in enumerate(audits, 1):
            run_step(
                audit_command(audit.name), cwd=proof,
                label=f"{audit.name} ({index}/{len(audits)})",
                timeout=args.audit_timeout, heartbeat=args.heartbeat,
            )
    print(f"Passed {len(audits)} Lean audits", flush=True)


def missing_lake_error():
    """A run without `lake` must be a hard error, not a printed shrug: both
    `--skip-build` audits and the build itself invoke it."""
    elan_bin = Path.home() / ".elan" / "bin"
    return OSError(
        "`lake` was not found on PATH; the pinned Lean toolchain "
        f"(proof/lean-toolchain) is installed by elan into {elan_bin} — "
        "install elan and put that directory on PATH"
    )


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--build-timeout", type=positive_seconds, default=600,
                        help="build time limit in seconds (default: 600)")
    parser.add_argument("--audit-timeout", type=positive_seconds, default=120,
                        help="time limit per audit in seconds (default: 120)")
    parser.add_argument("--heartbeat", type=positive_seconds, default=15,
                        help="progress interval in seconds (default: 15)")
    parser.add_argument("--jobs", type=positive_jobs, default=default_jobs(),
                        help=f"audits to check at once (default: {default_jobs()}, the "
                             "detected CPU count capped at 8); --jobs 1 keeps the "
                             "one-at-a-time transcript")
    parser.add_argument("--verbose", action="store_true", help="show Lake build commands")
    parser.add_argument("--skip-build", action="store_true",
                        help="audit existing artifacts; only use after a successful lake build")
    args = parser.parse_args(argv)
    proof = Path(__file__).resolve().parents[1] / "proof"
    # The status is returned and raised as this process's own exit code below,
    # never delegated: a run that printed "Lean checks failed" while exiting 0
    # masked a real Lean failure from every gate that reads only the code.
    try:
        if shutil.which("lake") is None:
            raise missing_lake_error()
        check_audits(proof, args)
    except (OSError, ValueError, TimeoutError, subprocess.CalledProcessError) as error:
        print(f"Lean checks failed: {error}", file=sys.stderr, flush=True)
        return 1
    except KeyboardInterrupt:
        print("Lean checks interrupted; stopped active process tree",
              file=sys.stderr, flush=True)
        return 130
    return 0


if __name__ == "__main__":
    sys.exit(main())

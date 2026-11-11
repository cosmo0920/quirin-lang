#!/usr/bin/env python3
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

"""Build the pinned Lean project and run every audit with bounded, visible steps."""
import argparse
from concurrent.futures import ThreadPoolExecutor, as_completed
import math
import os
from pathlib import Path
import re
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


class RunStopped(Exception):
    """A worker reached the spawn boundary after its audit run was stopped."""


class ChildSupervisor:
    """Make child creation and cancellation one synchronized lifecycle.

    Holding the lock through Popen and registration is deliberate. Once
    ``stop`` has set the stopped flag, no worker can pass the flag check and
    create an unregistered child. If Popen is already in progress, ``stop``
    waits for registration and includes that child in its termination set.
    """

    def __init__(self):
        self._lock = threading.Lock()
        self._active = set()
        self._stopped = False

    def spawn(self, command, **kwargs):
        with self._lock:
            if self._stopped:
                raise RunStopped("audit run stopped before child creation")
            process = subprocess.Popen(command, **kwargs)
            self._active.add(process)
            return process

    def discard(self, process):
        with self._lock:
            self._active.discard(process)

    def stop(self):
        with self._lock:
            self._stopped = True
            running = list(self._active)
        for process in running:
            stop_process_tree(process)


def run_step(command, *, cwd, label, timeout, heartbeat, output=None, quiet=False,
             supervisor=None):
    """Run one bounded child.

    `output` captures the child's streams into a file instead of inheriting this
    process's, which is what lets independent audits share one terminal without
    interleaving; `quiet` withholds the progress lines a worker would otherwise
    print out of turn. Neither changes what is run or how failure is judged.
    """
    if not quiet:
        print(f"Starting {label} (limit {timeout:g}s)", flush=True)
    started = time.monotonic()
    owner = supervisor or ChildSupervisor()
    process = owner.spawn(
        command, cwd=cwd, start_new_session=os.name == "posix",
        stdout=output, stderr=subprocess.STDOUT if output is not None else None,
    )
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
        owner.discard(process)
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
    supervisor = ChildSupervisor()

    def one(index, audit):
        label = f"{audit.name} ({index}/{len(audits)})"
        with tempfile.TemporaryFile(mode="w+") as captured:
            try:
                elapsed = run_step(
                    audit_command(audit.name), cwd=proof, label=label,
                    timeout=args.audit_timeout, heartbeat=args.heartbeat,
                    output=captured, quiet=True, supervisor=supervisor,
                )
            except (OSError, TimeoutError, subprocess.CalledProcessError) as error:
                captured.seek(0)
                raise AuditFailure(error, captured.read()) from None
            captured.seek(0)
            return index, label, elapsed, captured.read()

    print(f"Starting {len(audits)} audits, {jobs} at a time "
          f"(limit {args.audit_timeout:g}s each)", flush=True)
    pool = ThreadPoolExecutor(max_workers=jobs)
    futures = []
    results = {}
    failure = None
    try:
        for index, audit in enumerate(audits, 1):
            futures.append(pool.submit(one, index, audit))
        for future in as_completed(futures):
            try:
                index, label, elapsed, text = future.result()
                results[index] = (label, elapsed, text)
            except BaseException as error:
                failure = error
                supervisor.stop()
                for pending in futures:
                    pending.cancel()
                break
    except BaseException as error:
        failure = error
        supervisor.stop()
        for pending in futures:
            pending.cancel()
    finally:
        # Collection is part of the cancellation contract: after this returns,
        # no worker remains able to create or own a child for this audit run.
        pool.shutdown(wait=True, cancel_futures=True)

    if failure is not None:
        # Workers stopped by the supervisor are consequences of the first
        # observed failure. Preserve the causal failure and its captured log.
        if isinstance(failure, AuditFailure):
            if failure.output:
                print(failure.output, end="", flush=True)
            raise failure.cause
        raise failure

    # Detection follows completion order above; successful transcript replay
    # remains stable for CI logs and archived evidence.
    for index in range(1, len(audits) + 1):
        label, elapsed, text = results[index]
        if text:
            print(text, end="", flush=True)
        print(f"Passed {label} in {elapsed:.1f}s", flush=True)


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


PIN_PATTERN = re.compile(r"leanprover/lean4:v?(?P<version>\d+\.\d+\.\d+(?:-[0-9A-Za-z.]+)?)")
ACTIVE_PATTERN = re.compile(r"\bLean \(version (?P<version>\d+\.\d+\.\d+(?:-[0-9A-Za-z.]+)?)[,)]")


def pinned_lean_version(proof):
    """The version `proof/lean-toolchain` pins, e.g. `4.35.0`. A pin this runner
    cannot read is refused, never guessed: the version guard below compares
    against it."""
    path = proof / "lean-toolchain"
    try:
        text = path.read_text(encoding="utf-8").strip()
    except OSError as error:
        raise ValueError(f"cannot read the Lean pin {path}: {error}") from None
    match = PIN_PATTERN.fullmatch(text)
    if match is None:
        raise ValueError(
            f"{path} does not name a leanprover/lean4 release (read {text!r})"
        )
    return match.group("version")


def check_pinned_toolchain(proof, timeout):
    """Fail clearly when the Lean that Lake launches inside `proof` is not the
    pinned one (LEAN-4-35-TOOLCHAIN). The audits and the build both run through
    `lake` with `cwd=proof`, so elan normally selects `proof/lean-toolchain`;
    an `ELAN_TOOLCHAIN` override, a `+toolchain` shim, a non-elan `lake` on
    PATH or a stale default would otherwise check the proofs under a different
    parser. `lake env lean --version` reports exactly the binary the audit
    command `lake env lean` runs."""
    pinned = pinned_lean_version(proof)
    command = ["lake", "env", "lean", "--version"]
    try:
        result = subprocess.run(
            command, cwd=proof, capture_output=True, text=True, timeout=timeout,
            check=False,
        )
    except subprocess.TimeoutExpired:
        raise TimeoutError(
            f"Lean version query exceeded {timeout:g}s: {' '.join(command)}"
        ) from None
    if result.returncode != 0:
        raise ValueError(
            f"Lean version query `{' '.join(command)}` in {proof} exited "
            f"{result.returncode}: {(result.stderr or result.stdout).strip()}"
        )
    match = ACTIVE_PATTERN.search(result.stdout)
    if match is None:
        raise ValueError(
            f"cannot read the active Lean version from `{' '.join(command)}`: "
            f"{result.stdout.strip()!r}"
        )
    active = match.group("version")
    if active != pinned:
        raise ValueError(
            f"active Lean {active} is not the pinned Lean {pinned} "
            f"(proof/lean-toolchain); run through elan from proof/ without a "
            "toolchain override (ELAN_TOOLCHAIN, `+toolchain`), or install the "
            "pinned toolchain"
        )
    print(f"Lean {active} matches proof/lean-toolchain", flush=True)
    return active


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
        check_pinned_toolchain(proof, args.build_timeout)
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

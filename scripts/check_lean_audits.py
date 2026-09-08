#!/usr/bin/env python3
"""Build the pinned Lean project and run every audit with bounded, visible steps."""
import argparse
import math
import os
from pathlib import Path
import signal
import subprocess
import time


def positive_seconds(value):
    seconds = float(value)
    if not math.isfinite(seconds) or seconds <= 0:
        raise argparse.ArgumentTypeError("must be a finite positive number of seconds")
    return seconds


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


def run_step(command, *, cwd, label, timeout, heartbeat):
    print(f"Starting {label} (limit {timeout:g}s)", flush=True)
    started = time.monotonic()
    process = subprocess.Popen(command, cwd=cwd, start_new_session=os.name == "posix")
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
                print(f"Still running {label}: {elapsed:.1f}s / {timeout:g}s", flush=True)
        if code:
            raise subprocess.CalledProcessError(code, command)
    except BaseException:
        stop_process_tree(process)
        raise
    print(f"Passed {label} in {time.monotonic() - started:.1f}s", flush=True)


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
    for index, audit in enumerate(audits, 1):
        run_step(
            ["lake", "env", "lean", "-DwarningAsError=true", audit.name],
            cwd=proof, label=f"{audit.name} ({index}/{len(audits)})",
            timeout=args.audit_timeout, heartbeat=args.heartbeat,
        )
    print(f"Passed {len(audits)} Lean audits", flush=True)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--build-timeout", type=positive_seconds, default=600,
                        help="build time limit in seconds (default: 600)")
    parser.add_argument("--audit-timeout", type=positive_seconds, default=120,
                        help="time limit per audit in seconds (default: 120)")
    parser.add_argument("--heartbeat", type=positive_seconds, default=15,
                        help="progress interval in seconds (default: 15)")
    parser.add_argument("--verbose", action="store_true", help="show Lake build commands")
    parser.add_argument("--skip-build", action="store_true",
                        help="audit existing artifacts; only use after a successful lake build")
    args = parser.parse_args(argv)
    proof = Path(__file__).resolve().parents[1] / "proof"
    try:
        check_audits(proof, args)
    except (OSError, ValueError, TimeoutError, subprocess.CalledProcessError) as error:
        parser.exit(1, f"Lean checks failed: {error}\n")
    except KeyboardInterrupt:
        parser.exit(130, "Lean checks interrupted; stopped active process tree\n")


if __name__ == "__main__":
    main()

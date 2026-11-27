#!/usr/bin/env python3
"""Profile one pinned corpus audit run (card W44-AUDIT-PARALLEL-UNITS).

Usage: profile_run.py BINARY OUT_DIR

Runs BINARY with exactly `run_audit.py`'s command shape and environment, plus

* `QCORE_PROFILE_LOG=OUT_DIR/profile.tsv`, read only by the temporary
  instrumentation of `profile-instrumentation.patch` (a binary without it
  ignores the variable);
* `strace -f --seccomp-bpf -e trace=execve -ttt`, so every Clang and provider
  subprocess has an exec and an exit timestamp on the same clock;
* a 0.25 s sampler of the resident set of the audit process and of its whole
  process tree (`rss.tsv`).

OUT_DIR must not exist. The audit's stdout/stderr go to `audit.log`.
"""

import json
import os
import pathlib
import subprocess
import sys
import threading
import time

PIN = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/build/cycle19")


def descendants(root: int) -> list[int]:
    children: dict[int, list[int]] = {}
    for entry in os.listdir("/proc"):
        if not entry.isdigit():
            continue
        try:
            stat = pathlib.Path(f"/proc/{entry}/stat").read_text()
        except OSError:
            continue
        ppid = int(stat.rsplit(")", 1)[1].split()[1])
        children.setdefault(ppid, []).append(int(entry))
    found, stack = [], [root]
    while stack:
        pid = stack.pop()
        for child in children.get(pid, []):
            found.append(child)
            stack.append(child)
    return found


def rss_kib(pid: int) -> tuple[str, int]:
    try:
        status = pathlib.Path(f"/proc/{pid}/status").read_text()
    except OSError:
        return "", 0
    name, rss = "", 0
    for line in status.splitlines():
        if line.startswith("Name:"):
            name = line.split()[1]
        elif line.startswith("VmRSS:"):
            rss = int(line.split()[1])
    return name, rss


def main() -> int:
    binary = pathlib.Path(sys.argv[1]).resolve()
    out = pathlib.Path(sys.argv[2]).resolve()
    out.mkdir(parents=True, exist_ok=False)
    command = [
        str(binary),
        "--pinned-evidence",
        str(PIN / "c-gateway-wave-pin/fluent-bit-clang-source"),
        str(PIN / "c-gateway-wave-pin/monkey/monkey-build/compile_commands.json"),
        str(out / "monkey-source.json"),
        str(out / "resource-diagnostics.json"),
    ]
    environment = dict(os.environ)
    environment["QCORE_CLANG"] = "/usr/bin/clang"
    environment["QCORE_CHUNKIO_PACKAGE"] = str(PIN / "provider-packages-wave20/qcore.chunkio")
    environment.pop("QCORE_SOURCE_CONTRACTS", None)
    environment["QCORE_PROFILE_LOG"] = str(out / "profile.tsv")
    traced = [
        "strace", "-f", "--seccomp-bpf", "-q", "-e", "trace=execve", "-e", "signal=none",
        "-ttt", "-o", str(out / "strace.txt"), *command,
    ]
    start = time.time()
    with open(out / "audit.log", "wb") as log:
        process = subprocess.Popen(traced, env=environment, stdout=log, stderr=log)
        samples = []
        done = threading.Event()

        def sample() -> None:
            while not done.is_set():
                now = time.time()
                audit, tree = 0, 0
                for pid in descendants(process.pid):
                    name, rss = rss_kib(pid)
                    tree += rss
                    if name.startswith("cycle19_fluent"):
                        audit = max(audit, rss)
                samples.append((now, audit, tree))
                time.sleep(0.25)

        sampler = threading.Thread(target=sample)
        sampler.start()
        exit_code = process.wait()
        done.set()
        sampler.join()
    end = time.time()
    with open(out / "rss.tsv", "w") as handle:
        for now, audit, tree in samples:
            handle.write(f"{now:.3f}\t{audit}\t{tree}\n")
    record = {
        "command": command,
        "binary_sha256": subprocess.run(
            ["sha256sum", str(binary)], capture_output=True, text=True, check=True
        ).stdout.split()[0],
        "start": start,
        "end": end,
        "wall_s": end - start,
        "exit": exit_code,
        "peak_audit_rss_kib": max((s[1] for s in samples), default=0),
        "peak_tree_rss_kib": max((s[2] for s in samples), default=0),
    }
    (out / "run.json").write_text(json.dumps(record, indent=2) + "\n")
    print(json.dumps(record, indent=2))
    return exit_code


if __name__ == "__main__":
    sys.exit(main())

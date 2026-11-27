#!/usr/bin/env python3
"""cpu_poll.py OUT.jsonl INTERVAL COMMAND... -- DEEP-AST-TRANSPORTS-AND-WALL-TIME,
measurement only: run COMMAND and, every INTERVAL seconds until it exits,
record every thread's CPU time (utime + stime from /proc/PID/task/TID/stat,
in clock ticks) with the Unix time, one JSON line per poll. Joined to the
`unit-timing` intervals (cpu_units.py), it gives each unit analysis's own CPU
seconds on its thread, which the host's other load and the run's scheduling
do not move as they move wall time. Clang's processes and helper threads are
not the unit's thread and are not counted. Exits with COMMAND's exit."""
import json
import os
import subprocess
import sys
import time


def threads(pid):
    found = {}
    try:
        tasks = os.listdir(f"/proc/{pid}/task")
    except OSError:
        return found
    for tid in tasks:
        try:
            with open(f"/proc/{pid}/task/{tid}/stat") as stat:
                fields = stat.read().rsplit(")", 1)[1].split()
            found[tid] = int(fields[11]) + int(fields[12])
        except (OSError, IndexError, ValueError):
            continue
    return found


def main():
    out, interval, command = sys.argv[1], float(sys.argv[2]), sys.argv[3:]
    child = subprocess.Popen(command)
    with open(out, "w") as log:
        while child.poll() is None:
            log.write(json.dumps({"unix": time.time(), "cpu": threads(child.pid)}) + "\n")
            time.sleep(interval)
    return child.returncode


if __name__ == "__main__":
    sys.exit(main())

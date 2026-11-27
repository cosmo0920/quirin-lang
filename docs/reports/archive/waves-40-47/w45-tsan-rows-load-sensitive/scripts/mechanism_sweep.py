#!/usr/bin/env python3
"""The mechanism table of the report: every cell of mechanism.py, in order,
appended as JSON lines to OUT.

usage: mechanism_sweep.py KEEP_DIR OUT_JSONL   (run from a private directory)
KEEP_DIR holds the programs captured with capture.patch (suffix = the pid).
"""
import glob, os, subprocess, sys

keep, out = sys.argv[1], sys.argv[2]
here = os.path.dirname(os.path.abspath(__file__))

def subject(prefix):
    (match,) = glob.glob(os.path.join(keep, f"qcore-task-route-tsan-{prefix}-no-failure-*"))
    return match

T = subject("overlap-ct07-case-forced-together")
R = subject("overlap-ct07-case-forced-reverse")
U = subject("unjoined-shared_readers-forced-together")
UR = subject("unjoined-shared_readers-forced-reverse")
UC = subject("unjoined-ct07-case-forced-together")

cells = []
cells += [(T, 10, d, "shipped", 2, None) for d in (0, 1, 2, 3, 5, 20, 100)]
cells += [(T, 10, d, "shipped", 2, "halt_on_error=1:history_size=7") for d in (5, 10, 20, 100)]
cells += [(T, 10, d, "shipped", 2, "halt_on_error=1:history_size=0") for d in (1, 2, 3)]
cells += [(T, 10, d, v, 2, None) for v in ("slow-barrier", "relaxed-barrier") for d in (5, 20, 100)]
cells += [(T, 10, 20, v, 2, None) for v in ("quiet-barrier", "quiet-finish", "no-start-print")]
cells += [(T, 10, 20, "shipped", 1, None)]
cells += [(R, 10, 20, "shipped", o, None) for o in (1, 2)]
cells += [(T, 4, d, "count", 2, None) for d in (2, 3, 5)]
cells += [(s, 5, d, v, 2, None) for s in (U, UC) for d in (0, 20) for v in ("shipped", "relaxed-barrier")]
cells += [(UR, 4, d, "shipped", o, None) for o in (1, 2) for d in (20, 60, 120)]

with open(out, "w") as sink:
    for s, runs, delay, variant, ordinal, options in cells:
        argv = [sys.executable, os.path.join(here, "mechanism.py"), s, str(runs), str(delay), variant, str(ordinal)]
        if options:
            argv.append(options)
        result = subprocess.run(argv, capture_output=True, text=True)
        if result.returncode != 0:
            raise SystemExit(result.stderr)
        sink.write(result.stdout)
        sink.flush()
        print(result.stdout.strip().splitlines()[-1])

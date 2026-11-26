#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Card PRIORITY0-ATTRIBUTION: every PREDICTION.md and native-comparison
record added between 556460e31 and 9aed1913a, with the step its adding
commit falls in (`P-C`: an ancestor of 2c07390b6; `C-T`: otherwise). Output
is the cards-interval.txt run_chain.py reads."""
import subprocess, sys, json
W = "/media/Data/GitHub/quirin-lang/.claude/worktrees/agent-a14d1f75495df89fd"
PREV, COMMON, TIP = "556460e31", "2c07390b6", "9aed1913a"

def git(*a):
    return subprocess.run(["git", *a], cwd=W, capture_output=True, text=True).stdout

def anc(a, b):
    return subprocess.run(["git", "merge-base", "--is-ancestor", a, b], cwd=W).returncode == 0

files = git("diff", "--name-only", "--diff-filter=AM", PREV, TIP, "--",
            "docs/reports/*/PREDICTION.md", "docs/reports/*native-comparison*").split()
out = []
for f in files:
    added = git("log", "--diff-filter=A", "--format=%h", TIP, "--", f).split()
    a = added[-1] if added else "?"
    if anc(a, PREV):
        iv = "pre"
    elif anc(a, COMMON):
        iv = "P-C"
    else:
        iv = "C-T"
    out.append((iv, f.replace("docs/reports/", ""), a))
out.sort()
for o in out:
    print(*o)

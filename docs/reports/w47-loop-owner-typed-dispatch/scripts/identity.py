#!/usr/bin/env python3
"""Each arbiter measurement of a run against the W47-LOOP-OWNER-CONTRACTS archive.

Usage: identity.py RUN_ROOT OUT_DIR

RUN_ROOT is an arbiters.py output (corpus/, system/, slice.path). Each
measurement is compared with the baseline archive (filename-keyed manifests,
built in the worktree agent-a04a30eb19f1c410e at 9b3f9f88) with this
checkout's root aliased to the baseline's
(W47-LOOP-OWNER-COROUTINE-HOP's manifest_identity.py), and composed with it
(compose3: verdicts, leaves, counts). Every exit goes to a file.
"""

import pathlib
import subprocess
import sys

REPO = pathlib.Path(__file__).resolve().parents[4]
BASELINE_ROOT = "/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-a04a30eb19f1c410e"
ARCHIVE = REPO / "docs/reports/w47-loop-owner-contracts/artifacts/after"
IDENTITY = REPO / "docs/reports/w47-loop-owner-coroutine-hop/scripts/manifest_identity.py"
COMPOSE = pathlib.Path(
    "/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-implementation-"
    "wave-47-681db5/c8ef992e-a75f-4250-b4e1-07e6e5477601/scratchpad/compose3.py")


def main():
    run, out = pathlib.Path(sys.argv[1]).resolve(), pathlib.Path(sys.argv[2])
    out.mkdir(parents=True, exist_ok=False)
    for measure in ("corpus", "system", "slice"):
        directory = run / measure
        if measure == "slice":
            directory = pathlib.Path((run / "slice.path").read_text().strip())
        archive = ARCHIVE / measure
        for name, command in (
                ("identity", ["python3", str(IDENTITY), str(directory), str(archive),
                              "--root-alias", f"{REPO}={BASELINE_ROOT}"]),
                ("compose", ["python3", str(COMPOSE), str(archive), str(archive), str(archive),
                             str(directory)])):
            with open(out / f"{measure}.{name}", "wb") as log:
                code = subprocess.run(command, stdout=log, stderr=subprocess.STDOUT,
                                      check=False).returncode
            (out / f"{measure}.{name}.exit").write_text(f"{code}\n")
            last = (out / f"{measure}.{name}").read_text().strip().splitlines()[-1:]
            print(measure, name, code, last)


if __name__ == "__main__":
    main()

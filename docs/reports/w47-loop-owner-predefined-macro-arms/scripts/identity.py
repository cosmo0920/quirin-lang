#!/usr/bin/env python3
"""Each arbiter measurement of a run against the round-7 baseline archive.

Usage: identity.py RUN_ROOT OUT_DIR

RUN_ROOT is an `arbiters.py` output (corpus/, system/, slice.path). Each file
of docs/reports/w47-flb-lib-lifecycle/integration/artifacts/<measure>'s
filename-keyed manifest is compared raw (SHA-256) first; a file that differs
is compared again with this checkout's root rewritten to the integration
worktree's root (a disclosure, never a substitute), and the lines that still
differ are counted. Each measurement is then composed with the archive
(compose3: verdicts, leaves, counts). `measure.identity.exit` is 0 only when
`resource-diagnostics.json` is byte-identical raw. Every exit goes to a file.
Read-only.
"""

import gzip
import hashlib
import json
import pathlib
import subprocess
import sys

REPO = pathlib.Path(__file__).resolve().parents[4]
BASELINE = REPO / "docs/reports/w47-flb-lib-lifecycle/integration/artifacts"
BASELINE_ROOT = "/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/implementation-wave-47-681db5"
COMPOSE = pathlib.Path(
    "/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-implementation-"
    "wave-47-681db5/c8ef992e-a75f-4250-b4e1-07e6e5477601/scratchpad/compose3.py")


def compare(directory, archive, log):
    manifest = json.loads((archive / "manifest.json").read_text())
    verdicts_identical = False
    for name, entry in sorted(manifest.items()):
        path = directory / name
        if not path.exists():
            log.write(f"{name}: missing from the run\n")
            continue
        data = path.read_bytes()
        digest = hashlib.sha256(data).hexdigest()
        line = f"{name}: run {digest[:16]} archive {entry['sha256'][:16]} "
        if digest == entry["sha256"]:
            line += "identical"
            verdicts_identical |= name == "resource-diagnostics.json"
        else:
            archived = gzip.decompress((archive / entry["file"]).read_bytes())
            aliased = data.replace(str(REPO).encode(), BASELINE_ROOT.encode())
            ours = aliased.decode("utf-8", "replace").splitlines()
            theirs = archived.decode("utf-8", "replace").splitlines()
            changed = sum(1 for a, b in zip(ours, theirs) if a != b)
            same = hashlib.sha256(aliased).hexdigest() == entry["sha256"]
            line += (f"DIFFERENT raw; with the root alias "
                     f"{'identical' if same else f'{changed} line(s) differ'}"
                     f" (line count {len(ours)} vs {len(theirs)})")
        log.write(line + "\n")
    return verdicts_identical


def main():
    root, out = pathlib.Path(sys.argv[1]).resolve(), pathlib.Path(sys.argv[2])
    out.mkdir(parents=True, exist_ok=False)
    for measure in ("corpus", "system", "slice"):
        directory = root / measure
        if measure == "slice":
            directory = pathlib.Path((root / "slice.path").read_text().strip())
        archive = BASELINE / measure
        with open(out / f"{measure}.identity", "w") as log:
            identical = compare(directory, archive, log)
        (out / f"{measure}.identity.exit").write_text(f"{0 if identical else 1}\n")
        with open(out / f"{measure}.compose", "wb") as log:
            code = subprocess.run(["python3", str(COMPOSE), str(archive), str(archive),
                                   str(archive), str(directory)],
                                  stdout=log, stderr=subprocess.STDOUT, check=False).returncode
        (out / f"{measure}.compose.exit").write_text(f"{code}\n")
        last = (out / f"{measure}.compose").read_text().strip().splitlines()[-1:]
        print(measure, "resource-diagnostics identical" if identical else "DIFFERS raw",
              "compose", code, last)


if __name__ == "__main__":
    main()

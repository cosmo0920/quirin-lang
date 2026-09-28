#!/usr/bin/env python3
"""Each arbiter measurement of a run against the wave-47 baseline archive.

Usage: identity.py RUN_ROOT OUT_DIR [BASELINE_DIR]

RUN_ROOT is an `arbiters.py` output (corpus/, system/, slice.path). Each
measurement is compared file by file with the filename-keyed manifest of the
baseline (default: docs/reports/w47-condvar-join-publication/integration/
artifacts/<measure>, the W47-LOOP-OWNER-INSTANCE-FLOW in_tail resolution integration's
baseline: the in_tail resolution integration), raw SHA-256 first; a file that differs is rewritten with this
checkout's root read as the baseline's unknown root only when its difference
is confined to lines naming a path (disclosed, never a substitute). Each is
then composed with the baseline (compose3: verdicts, leaves, counts). Every
exit goes to a file. Read-only.
"""

import gzip
import hashlib
import json
import pathlib
import subprocess
import sys

REPO = pathlib.Path(__file__).resolve().parents[4]
BASELINE = REPO / "docs/reports/w47-loop-owner-instance-flow/integration/artifacts"
COMPOSE = pathlib.Path(
    "/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-implementation-"
    "wave-47-681db5/c8ef992e-a75f-4250-b4e1-07e6e5477601/scratchpad/compose3.py")


def compare(directory, archive, log):
    manifest = json.loads((archive / "manifest.json").read_text())
    verdict_identical = False
    for name, entry in sorted(manifest.items()):
        path = directory / name
        if not path.exists():
            log.write(f"{name}: missing from the run\n")
            continue
        data = path.read_bytes()
        digest = hashlib.sha256(data).hexdigest()
        same = digest == entry["sha256"]
        line = f"{name}: run {digest[:16]} archive {entry['sha256'][:16]} "
        if same:
            line += "identical"
            if name == "resource-diagnostics.json":
                verdict_identical = True
        else:
            archived = gzip.decompress((archive / entry["file"]).read_bytes())
            ours = data.decode("utf-8", "replace").splitlines()
            theirs = archived.decode("utf-8", "replace").splitlines()
            changed = [a for a, b in zip(ours, theirs) if a != b]
            line += f"DIFFERENT ({len(changed)} of {len(ours)} lines differ"
            line += f", line count {len(ours)} vs {len(theirs)})"
        log.write(line + "\n")
    return verdict_identical


def main():
    run, out = pathlib.Path(sys.argv[1]).resolve(), pathlib.Path(sys.argv[2])
    baseline = pathlib.Path(sys.argv[3]) if len(sys.argv) > 3 else BASELINE
    out.mkdir(parents=True, exist_ok=False)
    for measure in ("corpus", "system", "slice"):
        directory = run / measure
        if measure == "slice":
            directory = pathlib.Path((run / "slice.path").read_text().strip())
        archive = baseline / measure
        with open(out / f"{measure}.identity", "w") as log:
            identical = compare(directory, archive, log)
        (out / f"{measure}.identity.exit").write_text(f"{0 if identical else 1}\n")
        with open(out / f"{measure}.compose", "wb") as log:
            code = subprocess.run(["python3", str(COMPOSE), str(archive), str(archive),
                                   str(archive), str(directory)],
                                  stdout=log, stderr=subprocess.STDOUT, check=False).returncode
        (out / f"{measure}.compose.exit").write_text(f"{code}\n")
        last = (out / f"{measure}.compose").read_text().strip().splitlines()[-1:]
        print(measure, "resource-diagnostics identical" if identical else "DIFFERS", code, last)


if __name__ == "__main__":
    main()

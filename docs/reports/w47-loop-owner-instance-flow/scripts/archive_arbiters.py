#!/usr/bin/env python3
"""Archive an arbiters.py run as the new baseline, file set as the old one's.

Usage: archive_arbiters.py RUN_ROOT DEST_DIR [BASELINE_DIR]

For each of corpus, system and slice (slice read from RUN_ROOT/slice.path),
archives exactly the files the baseline's manifest lists (default baseline:
docs/reports/w47-condvar-join-publication/integration/artifacts) that the
run has, gzipped with a manifest (archive.py's format). Prints what the run
lacks. DEST_DIR must not exist.
"""

import json
import pathlib
import subprocess
import sys

REPO = pathlib.Path(__file__).resolve().parents[4]
BASELINE = REPO / "docs/reports/w47-condvar-join-publication/integration/artifacts"
ARCHIVE = pathlib.Path(__file__).with_name("archive.py")


def main():
    run, dest = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2])
    baseline = pathlib.Path(sys.argv[3]) if len(sys.argv) > 3 else BASELINE
    for measure in ("corpus", "system", "slice"):
        source = run / measure
        if measure == "slice":
            source = pathlib.Path((run / "slice.path").read_text().strip())
        names = sorted(json.loads((baseline / measure / "manifest.json").read_text()))
        present = [name for name in names if (source / name).exists()]
        for name in sorted(set(names) - set(present)):
            print(f"{measure}: {name} not in the run")
        subprocess.run(["python3", str(ARCHIVE), str(source), str(dest / measure), *present],
                       check=True)


if __name__ == "__main__":
    main()

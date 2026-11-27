#!/usr/bin/env python3
"""Archive one slice run of a private attempt into the report's artifacts.

usage: archive_slice.py ATTEMPT RUN DEST_DIR

Copies the gzipped report and contracts sidecar, the invocation record, the
console log, the run's exit, the attempt's prepare.json and prepare exit.
DEST_DIR must not exist.
"""
import gzip
import pathlib
import shutil
import sys

ROOT = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w45-fb-retry-lifetime")


def main():
    attempt, run, dest = sys.argv[1], sys.argv[2], pathlib.Path(sys.argv[3])
    source = ROOT / attempt / run
    dest.mkdir(parents=True, exist_ok=False)
    for name in ("resource-diagnostics.json", "resource-diagnostics.contracts.json"):
        with open(source / name, "rb") as raw, gzip.open(dest / (name + ".gz"), "wb", 9) as packed:
            shutil.copyfileobj(raw, packed)
    for name in ("resource-diagnostics.invocation.json", "console.log", "stderr.log", "exit"):
        if (source / name).exists():
            (dest / name).write_bytes((source / name).read_bytes())
    (dest / "prepare.json").write_bytes((ROOT / attempt / "prepare.json").read_bytes())
    (dest / "prepare.exit").write_bytes((ROOT / f"prepare-{attempt}.exit").read_bytes())


if __name__ == "__main__":
    main()

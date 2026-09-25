#!/usr/bin/env python3
"""Copy this card's private run records and attribution into the report.

Usage: publish_evidence.py

Reads the worktree-private `measure/` tree written by the route (b)
reproduction steps in the README (`measure/v2/`, `measure/attr2/`) and writes
`runs/`, `attribution/` and the census documents beside this script's report.
Large JSON is gzip-compressed with mtime 0. The route (a) evidence, published
by an earlier revision of this script, is kept under `route-a/`.
"""

import gzip
import pathlib
import shutil

ROOT = pathlib.Path(__file__).resolve().parents[4]
REPORT = ROOT / "docs/reports/w44-libc-residuals"
MEASURE = ROOT / "measure"
STAGES = ("before", "stage0", "stage1", "stage2", "stage3", "stage4")


def gz(source, target):
    target.write_bytes(gzip.compress(source.read_bytes(), mtime=0))


def main() -> int:
    runs = REPORT / "runs"
    attribution = REPORT / "attribution"
    runs.mkdir(exist_ok=True)
    attribution.mkdir(exist_ok=True)
    for stage in STAGES:
        shutil.copyfile(MEASURE / "v2" / stage / "run.json", runs / f"{stage}-run.json")
        shutil.copyfile(MEASURE / "v2" / stage / "stage-files.json", runs / f"{stage}-files.json")
    shutil.copyfile(MEASURE / "v2/before-identity.txt", runs / "before-identity.txt")
    shutil.copyfile(MEASURE / "v2/stage0-vs-before.json", runs / "stage0-vs-before.json")
    shutil.copyfile(MEASURE / "attr2/after-vs-before.json", runs / "after-vs-before.json")
    shutil.copyfile(MEASURE / "attr2/contracts-diff.txt", runs / "contracts-diff.txt")
    for stage in ("stage1", "stage2", "stage3", "stage4"):
        gz(MEASURE / f"attr2/{stage}.json", attribution / f"{stage}.json.gz")
    shutil.copyfile(MEASURE / "attr2/compose.json", attribution / "compose.json")
    shutil.copyfile(MEASURE / "attr2/valist-callers.json", attribution / "valist-callers.json")
    gz(MEASURE / "attr2/tail-residual-after.json", attribution / "tail-residual-after.json.gz")
    gz(MEASURE / "attr2/tail-nested-after.json", REPORT / "census-tail-nested-after.json.gz")
    gz(MEASURE / "attr2/tail-nested-before.json", REPORT / "census-tail-nested-before.json.gz")
    gz(MEASURE / "attr2/census-leaves-after.json", REPORT / "census-leaves-after.json.gz")
    gz(MEASURE / "attr2/census-leaves-before.json", REPORT / "census-leaves-before.json.gz")
    shutil.copyfile(MEASURE / "attr2/sole-moves.json", REPORT / "sole-moves.json")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

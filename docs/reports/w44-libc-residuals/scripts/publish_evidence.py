#!/usr/bin/env python3
"""Copy this card's private run records and attribution into the report.

Usage: publish_evidence.py

Reads the worktree-private `measure/` tree written by the reproduction steps
in the README and writes `runs/`, `attribution/` and the census documents
beside this script's report. Large JSON is gzip-compressed with mtime 0.
"""

import gzip
import pathlib
import shutil

ROOT = pathlib.Path(__file__).resolve().parents[4]
REPORT = ROOT / "docs/reports/w44-libc-residuals"
MEASURE = ROOT / "measure"


def gz(source, target):
    target.write_bytes(gzip.compress(source.read_bytes(), mtime=0))


def main() -> int:
    runs = REPORT / "runs"
    attribution = REPORT / "attribution"
    runs.mkdir(exist_ok=True)
    attribution.mkdir(exist_ok=True)
    shutil.copyfile(MEASURE / "before/run.json", runs / "before-run.json")
    shutil.copyfile(MEASURE / "stage0-reader/run.json", runs / "stage0-reader-run.json")
    shutil.copyfile(MEASURE / "stage1-tail/run.json", runs / "stage1-narrow-run.json")
    shutil.copyfile(MEASURE / "stage1-tail/stage-files.json", runs / "stage1-narrow-files.json")
    for stage in ("stage1b-tail", "stage2-inline", "stage3-valist", "stage4-after"):
        shutil.copyfile(MEASURE / stage / "run.json", runs / f"{stage}-run.json")
        shutil.copyfile(MEASURE / stage / "stage-files.json", runs / f"{stage}-files.json")
    shutil.copyfile(MEASURE / "before-identity.txt", runs / "before-identity.txt")
    shutil.copyfile(MEASURE / "stage0-vs-before.json", runs / "stage0-vs-before.json")
    shutil.copyfile(MEASURE / "attr/after-vs-before.json", runs / "after-vs-before.json")
    shutil.copyfile(MEASURE / "attr/contracts-diff.txt", runs / "contracts-diff.txt")
    for stage in ("stage1", "stage1b", "stage2", "stage3", "stage4"):
        gz(MEASURE / f"attr/{stage}.json", attribution / f"{stage}.json.gz")
    shutil.copyfile(MEASURE / "attr/compose.json", attribution / "compose.json")
    shutil.copyfile(MEASURE / "attr/valist-callers.json", attribution / "valist-callers.json")
    gz(MEASURE / "attr/tail-residual-1b.json", attribution / "tail-residual-after.json.gz")
    gz(MEASURE / "attr/tail-nested-after.json", REPORT / "census-tail-nested-after.json.gz")
    gz(MEASURE / "attr/census-leaves-after.json", REPORT / "census-leaves-after.json.gz")
    shutil.copyfile(MEASURE / "attr/sole-moves.json", REPORT / "sole-moves.json")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

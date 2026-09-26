#!/usr/bin/env python3
"""Where a run's source-effects sidecar differs from the archived baseline's.

Usage: sidecar_difference.py RUN_DIR BASELINE_ARTIFACT_DIR

Loads `resource-diagnostics.source-effects.json` from the run and the gzipped
archive from the baseline, and prints every differing JSON leaf path with both
values (strings truncated to 160 characters), then the count. A disclosure of
what differs, never a substitute for the raw comparison.
"""

import gzip
import json
import pathlib
import sys


def leaves(value, path=""):
    if isinstance(value, dict):
        for key, child in value.items():
            yield from leaves(child, f"{path}/{key}")
    elif isinstance(value, list):
        for index, child in enumerate(value):
            yield from leaves(child, f"{path}[{index}]")
    else:
        yield path, value


def short(value):
    text = json.dumps(value)
    return text if len(text) <= 160 else text[:157] + "..."


def main() -> int:
    run = pathlib.Path(sys.argv[1]) / "resource-diagnostics.source-effects.json"
    baseline = pathlib.Path(sys.argv[2]) / "source-effects.json.gz"
    ours = dict(leaves(json.loads(run.read_text())))
    theirs = dict(leaves(json.loads(gzip.decompress(baseline.read_bytes()))))
    differing = sorted(
        path for path in ours.keys() | theirs.keys() if ours.get(path, "<absent>") != theirs.get(path, "<absent>")
    )
    for path in differing:
        print(f"{path}\n  run:      {short(ours.get(path, '<absent>'))}\n  baseline: {short(theirs.get(path, '<absent>'))}")
    print(f"differing leaves: {len(differing)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())

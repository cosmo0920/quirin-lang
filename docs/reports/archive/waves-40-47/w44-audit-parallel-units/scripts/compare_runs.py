#!/usr/bin/env python3
"""Compare two audit run directories byte for byte (card W44-AUDIT-PARALLEL-UNITS).

Usage: compare_runs.py REFERENCE_RUN OTHER_RUN [--mask-preparation] [--file NAME ...]

For each artifact (the five `archive.py` archives, `audit.log`, and any
census `*.jsonl` present in either directory, plus every `--file`), prints
its SHA-256 in both runs and whether the bytes are identical.

`audit.log` names the run's own output path once ("Resource diagnostics:
<dir>/resource-diagnostics.json"), so it is compared twice: raw, and with the
directory each run wrote to (from its `run.json`, since a run directory may be
renamed afterwards) rewritten to `<RUN>`. The masked comparison is a
disclosure beside the raw one, never a substitute: the raw verdict is always
printed.

`--mask-preparation` additionally rewrites the three microsecond figures of
the configured route's `preparation:` line to `<us>`. They are summed
per-unit wall-clock measurements, which differ between any two runs at any
job count; the unit and Clang-invocation counts on the same line are kept.

Exits 0 only when every artifact is identical, `audit.log` after the path
(and, if requested, preparation-figure) masking.
"""

import hashlib
import json
import pathlib
import re
import sys

ARTIFACTS = [
    "resource-diagnostics.json",
    "resource-diagnostics.chunk-contracts.json",
    "resource-diagnostics.contracts.json",
    "resource-diagnostics.source-effects.json",
    "monkey-source.json",
]
PREPARATION = re.compile(rb"(-E -P |syntax |line markers )\d+ us")


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def written_to(run: pathlib.Path) -> pathlib.Path:
    """The directory the run wrote to: `run.json`'s diagnostics path's parent
    (a run may have been renamed afterwards), else the directory itself."""
    record = run / "run.json"
    if record.exists():
        return pathlib.Path(json.loads(record.read_text())["command"][-1]).parent
    return run


def main() -> int:
    reference = pathlib.Path(sys.argv[1]).resolve()
    other = pathlib.Path(sys.argv[2]).resolve()
    arguments = sys.argv[3:]
    mask_preparation = "--mask-preparation" in arguments
    extra = [arguments[i + 1] for i, a in enumerate(arguments) if a == "--file"]
    names = list(ARTIFACTS) + extra
    census = sorted({p.name for d in (reference, other) for p in d.glob("*.jsonl")})
    names += census
    result = {"reference": str(reference), "other": str(other), "artifacts": {}}
    identical = True
    for name in names:
        left, right = reference / name, other / name
        if not left.exists() and not right.exists():
            continue
        if not left.exists() or not right.exists():
            result["artifacts"][name] = {"present_in_both": False}
            identical = False
            continue
        a, b = left.read_bytes(), right.read_bytes()
        same = a == b
        result["artifacts"][name] = {"reference_sha256": digest(a), "other_sha256": digest(b), "byte_identical": same}
        identical &= same
    a = (reference / "audit.log").read_bytes()
    b = (other / "audit.log").read_bytes()
    masked_a = a.replace(str(written_to(reference)).encode(), b"<RUN>")
    masked_b = b.replace(str(written_to(other)).encode(), b"<RUN>")
    if mask_preparation:
        masked_a = PREPARATION.sub(rb"\1<us>", masked_a)
        masked_b = PREPARATION.sub(rb"\1<us>", masked_b)
    differing = [
        index + 1
        for index, (x, y) in enumerate(zip(a.split(b"\n"), b.split(b"\n")))
        if x != y
    ]
    entry = {
        "reference_sha256": digest(a),
        "other_sha256": digest(b),
        "raw_identical": a == b,
        "raw_differing_lines": differing[:10],
        "raw_line_counts": [a.count(b"\n"), b.count(b"\n")],
        "identical_after_run_path_mask": masked_a == masked_b,
    }
    if mask_preparation:
        entry["preparation_figures_masked"] = True
    result["artifacts"]["audit.log"] = entry
    identical &= masked_a == masked_b
    result["identical"] = identical
    print(json.dumps(result, indent=2))
    return 0 if identical else 1


if __name__ == "__main__":
    sys.exit(main())

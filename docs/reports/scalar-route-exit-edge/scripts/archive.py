#!/usr/bin/env python3
"""Card SCALAR-ROUTE-EXIT-EDGE: archive the re-pin artifacts.

Gzips each BEFORE and AFTER artifact into the card's report directory and
writes `artifact-manifest.json` beside them, in the same shape and with the
same key set as the preceding arbiters' manifests. Digests are of the **raw**
JSON, computed here rather than copied.

Usage: python3 measure/joined-provenance-call-frames/archive.py <before> <after>
"""
import gzip
import hashlib
import json
import pathlib
import sys

WT = pathlib.Path(__file__).resolve().parents[2]
MEASURE = WT / "measure" / "scalar-route-exit-edge"
OUT = WT / "docs" / "reports" / "scalar-route-exit-edge" / "artifacts"

ARTIFACTS = {
    "diagnostics": "resource-diagnostics.json",
    "source": "monkey-source.json",
    "chunk-contracts": "resource-diagnostics.chunk-contracts.json",
    "source-effects": "resource-diagnostics.source-effects.json",
    "contracts": "resource-diagnostics.contracts.json",
}


def main():
    before, after = sys.argv[1], sys.argv[2]
    OUT.mkdir(parents=True, exist_ok=True)
    manifest = {}
    for stage, run in (("after", after), ("before", before)):
        for stem, produced in ARTIFACTS.items():
            raw = (MEASURE / "runs" / run / produced).read_bytes()
            key = stem if stage == "after" else f"{stem}-before"
            # mtime=0 so the archive bytes depend only on the content.
            with gzip.GzipFile(OUT / f"{key}.json.gz", "wb", mtime=0) as handle:
                handle.write(raw)
            manifest[key] = {
                "bytes": len(raw),
                "produced_as": produced,
                "sha256": hashlib.sha256(raw).hexdigest(),
                "stage": stage,
            }
    document = {
        "artifacts": dict(sorted(manifest.items())),
        "base": "477b733d",
        "baseline": "docs/reports/joined-provenance-call-frames/artifacts",
        "card": "SCALAR-ROUTE-EXIT-EDGE",
        "note": (
            "AFTER is this card's re-pin and is byte-identical to the baseline "
            "in both verdict-bearing artifacts, as is BEFORE: the card moved no "
            "leaf, no function and no unit. The restriction it lands (a loop "
            "header's exit edge is never removed by the value route) is "
            "measured to reach neither deciding arm at any of the corpus's 201 "
            "loop-header BranchOnNull nodes. source-effects and contracts "
            "differ identically in BEFORE and AFTER (checkout root and "
            "identity digests) and are not attributable to this card."
        ),
    }
    (OUT / "artifact-manifest.json").write_text(
        json.dumps(document, indent=2, sort_keys=True) + "\n"
    )
    for name, entry in document["artifacts"].items():
        print(f"{entry['bytes']:>10}  {name}  {entry['sha256'][:16]}")
    print(f"\nwrote {OUT}")
    return 0


if __name__ == "__main__":
    sys.exit(main())

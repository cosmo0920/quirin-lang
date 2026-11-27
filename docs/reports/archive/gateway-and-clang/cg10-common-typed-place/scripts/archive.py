#!/usr/bin/env python3
"""archive.py BEFORE AFTER -- card CG10-COMMON-TYPED-PLACE: copy the
comparison, attribution and identity records of two private measurement
phases (private/cg10/BEFORE, private/cg10/AFTER) into artifacts/, gzipped
deterministically (mtime 0) where large, with the diagnostics' SHA-256.
"""
import gzip
import hashlib
import pathlib
import shutil
import sys

HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parents[3]
ARTIFACTS = HERE.parent / "artifacts"


def main() -> int:
    before, after = sys.argv[1], sys.argv[2]
    private = ROOT / "private/cg10"
    ARTIFACTS.mkdir(parents=True, exist_ok=True)
    copies = {
        f"{before}/corpus.identity.json": "before-corpus.identity.json",
        f"{before}/system.identity.json": "before-system.identity.json",
        f"{before}/slice.identity.json": "before-slice.identity.json",
        f"{before}/binary.sha256": "before-binary.sha256",
        f"{after}/binary.sha256": "after-binary.sha256",
        f"{after}/corpus.entries.json": "after-corpus.entries.json",
        f"{after}/system.compare.json": "after-system.compare.json",
        f"{after}/slice.compare.json": "after-slice.compare.json",
    }
    for source, name in copies.items():
        shutil.copyfile(private / source, ARTIFACTS / name)
    for source, name in {
        f"{after}/corpus.compare.json": "after-corpus.compare.json.gz",
        f"{after}/corpus.attribution.json": "after-corpus.attribution.json.gz",
    }.items():
        (ARTIFACTS / name).write_bytes(gzip.compress((private / source).read_bytes(), 9, mtime=0))
    with open(private / after / "corpus/parity.jsonl", "rb") as stream:
        rows = b"".join(line for line in stream if b'"typed-place"' in line)
    (ARTIFACTS / "after-corpus.typed-place.jsonl.gz").write_bytes(gzip.compress(rows, 9, mtime=0))
    lines = []
    for phase in (before, after):
        path = private / phase / "corpus/resource-diagnostics.json"
        lines.append(f"{hashlib.sha256(path.read_bytes()).hexdigest()}  {phase}/corpus/resource-diagnostics.json")
    (ARTIFACTS / "corpus-diagnostics.sha256").write_text("\n".join(lines) + "\n")
    for path in sorted(ARTIFACTS.iterdir()):
        print(path.stat().st_size, path.name)
    return 0


if __name__ == "__main__":
    sys.exit(main())

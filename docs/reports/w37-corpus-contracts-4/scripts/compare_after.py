#!/usr/bin/env python3
"""Card W37-CORPUS-CONTRACTS-4: does the rebased AFTER reproduce the archived one?

If every artifact matches, the two zero-movement code arbiters beneath this card
do not interact with its ten declarations, and the whole attribution in the
report stands unchanged. If any differs, that is a finding to name and
re-attribute leaf by leaf.
"""
import gzip
import hashlib
import pathlib

STEMS = {
    "resource-diagnostics.json": "diagnostics",
    "resource-diagnostics.chunk-contracts.json": "chunk-contracts",
    "monkey-source.json": "source",
    "resource-diagnostics.source-effects.json": "source-effects",
    "resource-diagnostics.contracts.json": "contracts",
}
ARCH = pathlib.Path("docs/reports/w37-corpus-contracts-4/artifacts")
RUN = pathlib.Path("measure/w37/runs/after2")

all_ok = True
for name, stem in STEMS.items():
    new = hashlib.sha256((RUN / name).read_bytes()).hexdigest()
    old = hashlib.sha256(gzip.decompress((ARCH / f"{stem}.json.gz").read_bytes())).hexdigest()
    ok = new == old
    all_ok &= ok
    print(f"{stem:16s} archived={old[:16]} rebased={new[:16]} identical={ok}")
print()
print("ALL FIVE AFTER ARTIFACTS BYTE-IDENTICAL:", all_ok)
raise SystemExit(0 if all_ok else 1)

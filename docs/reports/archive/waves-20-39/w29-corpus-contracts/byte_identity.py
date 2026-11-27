"""Uncompressed SHA-256 of each archived baseline artifact against the produced run.

Card W29-CORPUS-CONTRACTS. The authoritative baseline is card
GATEWAY-C-LOCATION's archive, which is the run the tip's leak pin of 266 and
its 73-module contract corpus describe; `docs/reports/aom-binding-poison/` and
`docs/reports/wave23-c-operations/` remain the frozen archives underneath it
and are not touched (standing constraint 21).
"""
import gzip
import hashlib
import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
ARCHIVE = ROOT / "docs/reports/gateway-c-location/artifacts"
RUN = pathlib.Path(sys.argv[1]) if len(sys.argv) > 1 else ROOT / "measure/runs/before"

PAIRS = {
    "diagnostics": "resource-diagnostics.json",
    "source": "monkey-source.json",
    "chunk-contracts": "resource-diagnostics.chunk-contracts.json",
    "source-effects": "resource-diagnostics.source-effects.json",
    "contracts": "resource-diagnostics.contracts.json",
}


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def canonical(data: bytes) -> str:
    return digest(json.dumps(json.loads(data), sort_keys=True, separators=(",", ":")).encode())


for stem, produced in PAIRS.items():
    archived = ARCHIVE / f"{stem}.json.gz"
    made = RUN / produced
    if not archived.exists() or not made.exists():
        print(f"{stem:16s} MISSING archived={archived.exists()} produced={made.exists()}")
        continue
    a = gzip.decompress(archived.read_bytes())
    b = made.read_bytes()
    print(
        f"{stem:16s} raw {'IDENTICAL' if digest(a) == digest(b) else 'differs'}"
        f"   canonical {'IDENTICAL' if canonical(a) == canonical(b) else 'differs'}"
        f"   {digest(a)[:16]} / {digest(b)[:16]}"
    )

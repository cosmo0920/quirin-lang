"""Uncompressed SHA-256 of each archived baseline artifact against the produced run.

Card W31-CORPUS-CONTRACTS-2 (wave 31). The authoritative baseline is card
C-GATEWAY-WAVE's archive — the run the tip's leak pin of 272, `unknown_alias`
4,208, `unsupported` 50,155, `double_release` 1 and 74-module contract corpus
describe. It supersedes the wave-30 `w4-recursive-summaries` archive this
card's interrupted first attempt targeted; the gateway change landed while
this card was down. The chain beneath it —
`docs/reports/w4-recursive-summaries/`, `docs/reports/w29-corpus-contracts/`,
`docs/reports/gateway-c-location/`, `docs/reports/aom-binding-poison/`,
`docs/reports/aom-destroy-siblings/`, `docs/reports/wave23-c-operations/` —
stays frozen and is not touched (standing constraint 21).
"""
import gzip
import hashlib
import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
ARCHIVE = ROOT / "docs/reports/c-gateway-wave/artifacts"
RUN = pathlib.Path(sys.argv[1]) if len(sys.argv) > 1 else ROOT / "measure/runs/before"
if len(sys.argv) > 2:
    ARCHIVE = pathlib.Path(sys.argv[2])

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

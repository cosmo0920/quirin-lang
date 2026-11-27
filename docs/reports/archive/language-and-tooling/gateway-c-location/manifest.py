"""SHA-256 receipts for this card's runs, binaries and archived artifacts.

Card GATEWAY-C-LOCATION. Read-only.
"""
import gzip
import hashlib
import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[1]
OUT = ROOT / "docs/reports/gateway-c-location"

PRODUCED = {
    "diagnostics": "resource-diagnostics.json",
    "source": "monkey-source.json",
    "chunk-contracts": "resource-diagnostics.chunk-contracts.json",
    "source-effects": "resource-diagnostics.source-effects.json",
    "contracts": "resource-diagnostics.contracts.json",
}


def sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


document = {
    "card": "GATEWAY-C-LOCATION",
    "note": "uncompressed digests of both runs' artifacts, the archived AFTER set "
    "(compressed and uncompressed), the two measured binaries and the pinned "
    "compile database; the BEFORE set is not archived because its diagnostics "
    "and chunk-contracts are byte-identical to docs/reports/aom-binding-poison/"
    "artifacts, which is not touched",
    "binaries": {},
    "compile_database": {},
    "runs": {},
    "archived": {},
}

for name in ("before", "after"):
    path = ROOT / "measure/bin" / name
    if path.exists():
        document["binaries"][name] = sha(path.read_bytes())
    document["runs"][name] = {
        stem: sha((ROOT / "measure/runs" / name / produced).read_bytes())
        for stem, produced in PRODUCED.items()
        if (ROOT / "measure/runs" / name / produced).exists()
    }

database = (
    ROOT
    / "build/cycle19/clang-pin/fluent-bit-clang-source/build/compile_commands.json"
)
if database.exists():
    entries = json.loads(database.read_text())
    document["compile_database"] = {
        "sha256": sha(database.read_bytes()),
        "entries": len(entries),
    }

for path in sorted((OUT / "artifacts").glob("*.json.gz")):
    raw = path.read_bytes()
    document["archived"][path.name] = {
        "compressed_sha256": sha(raw),
        "uncompressed_sha256": sha(gzip.decompress(raw)),
        "compressed_bytes": len(raw),
    }

text = json.dumps(document, indent=2) + "\n"
(OUT / "artifact-manifest.json").write_text(text)
print(text)

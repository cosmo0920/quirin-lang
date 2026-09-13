"""SHA-256 receipts for this card's runs, binary and archived artifacts.

Card W31-CORPUS-CONTRACTS-2 (wave 31). Read-only.
"""
import gzip
import hashlib
import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[1]
OUT = ROOT / "docs/reports/w31-corpus-contracts-2"

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
    "card": "W31-CORPUS-CONTRACTS-2",
    "note": "uncompressed digests of both counted runs' artifacts, the archived "
    "AFTER set (compressed and uncompressed), the single measured binary and the "
    "pinned compile database. This card changes no Rust, so the BEFORE and AFTER "
    "runs executed the same binary; the only input difference is the contract "
    "manifest and the two modules it selects. The BEFORE set is not archived "
    "because its diagnostics and chunk-contracts are byte-identical to "
    "docs/reports/w4-recursive-summaries/artifacts, which is not touched, and "
    "the run-to-run identity of source, source-effects and chunk-contracts is "
    "measured in logs/byte-identity-before-to-after.txt.",
    "binaries": {},
    "compile_database": {},
    "runs": {},
    "archived": {},
}

binary = ROOT / "measure/bin/audit"
document["binaries"]["measure/bin/audit"] = sha(binary.read_bytes())

database = (
    ROOT / "build/cycle19/clang-pin/fluent-bit-clang-source/build/compile_commands.json"
)
document["compile_database"] = {
    "path": str(database.relative_to(ROOT)),
    "sha256": sha(database.read_bytes()),
    "entries": len(json.loads(database.read_text())),
}

for run in ("before", "after"):
    directory = ROOT / "measure/runs" / run
    document["runs"][run] = {
        stem: sha((directory / name).read_bytes()) for stem, name in PRODUCED.items()
    }

for stem in PRODUCED:
    archived = OUT / "artifacts" / f"{stem}.json.gz"
    raw = archived.read_bytes()
    document["archived"][stem] = {
        "compressed_sha256": sha(raw),
        "uncompressed_sha256": sha(gzip.decompress(raw)),
    }

(OUT / "artifact-manifest.json").write_text(json.dumps(document, indent=1) + "\n")
print("wrote", OUT / "artifact-manifest.json")

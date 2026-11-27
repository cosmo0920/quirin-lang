"""SHA-256 receipts for this card's runs, binary and archived artifacts.

Card W4-RECURSIVE-SUMMARIES (wave 30). Read-only.
"""
import gzip
import hashlib
import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[1]
OUT = ROOT / "docs/reports/w4-recursive-summaries"

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
    "card": "W4-RECURSIVE-SUMMARIES",
    "note": "uncompressed digests of both counted runs' artifacts, the archived "
    "AFTER set (compressed and uncompressed), the single measured binary and the "
    "pinned compile database. This card changes no Rust, no contract and no "
    "manifest, so the BEFORE and AFTER runs executed the same binary over the "
    "same inputs; their byte identity is the run-to-run determinism measurement "
    "the report's section 5 states. The BEFORE set is not archived because its "
    "five artifacts are byte-identical to the archived AFTER set, and its "
    "diagnostics and chunk-contracts are byte-identical to "
    "docs/reports/w29-corpus-contracts/artifacts, which is not touched.",
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

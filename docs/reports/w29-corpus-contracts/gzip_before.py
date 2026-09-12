"""Gzip the BEFORE run's five artifacts under the archive stem names, so
`diff_artifacts.py` can itemise BEFORE -> AFTER. Card W29-CORPUS-CONTRACTS."""
import gzip
import pathlib

MAP = {
    "diagnostics": "resource-diagnostics.json",
    "source": "monkey-source.json",
    "chunk-contracts": "resource-diagnostics.chunk-contracts.json",
    "source-effects": "resource-diagnostics.source-effects.json",
    "contracts": "resource-diagnostics.contracts.json",
}
ROOT = pathlib.Path(__file__).resolve().parent
SRC = ROOT / "runs/before"
DST = ROOT / "before-as-archive"
DST.mkdir(exist_ok=True)
for stem, name in MAP.items():
    DST.joinpath(f"{stem}.json.gz").write_bytes(gzip.compress((SRC / name).read_bytes()))
print("ok")

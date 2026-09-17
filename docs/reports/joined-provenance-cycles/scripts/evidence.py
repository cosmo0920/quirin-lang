#!/usr/bin/env python3
"""Card JOINED-PROVENANCE-CYCLES: archive the evidence beside the report.

Deterministic gzip (mtime 0, level 9) so a re-archive of the same run is
byte-identical. Refuses to write anything if the BEFORE run has stopped
reproducing the baseline, because then the chain is not attributable and the
archive would be a claim this card cannot make.
"""
import gzip
import hashlib
import json
import pathlib
import shutil
import sys

M = pathlib.Path(__file__).resolve().parent
WT = M.parents[1]
R = WT / "docs" / "reports" / "joined-provenance-cycles"
BASELINE = WT / "docs" / "reports" / "c-gateway-constant-do-while" / "artifacts"

ARTIFACTS = {
    "diagnostics": "resource-diagnostics.json",
    "source": "monkey-source.json",
    "chunk-contracts": "resource-diagnostics.chunk-contracts.json",
    "source-effects": "resource-diagnostics.source-effects.json",
    "contracts": "resource-diagnostics.contracts.json",
}
VERDICT_BEARING = ("diagnostics", "chunk-contracts")


def main():
    # Gate: BEFORE must still reproduce the baseline on both verdict-bearing
    # artifacts, or nothing is written.
    for stem in VERDICT_BEARING:
        measured = (M / "runs" / "before" / ARTIFACTS[stem]).read_bytes()
        archived = gzip.decompress((BASELINE / f"{stem}.json.gz").read_bytes())
        if measured != archived:
            print(f"REFUSING: BEFORE no longer reproduces the baseline `{stem}`", file=sys.stderr)
            return 1

    (R / "artifacts").mkdir(parents=True, exist_ok=True)
    (R / "logs").mkdir(exist_ok=True)
    (R / "scripts").mkdir(exist_ok=True)

    manifest = {
        "card": "JOINED-PROVENANCE-CYCLES",
        "base": "ebc0fd49",
        "baseline": "docs/reports/c-gateway-constant-do-while/artifacts",
        "note": (
            "AFTER is this card's re-pin. BEFORE reproduces the baseline byte for byte in "
            "both verdict-bearing artifacts; source-effects and contracts differ identically "
            "in BEFORE and AFTER (checkout root and identity digests) and are not attributable "
            "to this card."
        ),
        "artifacts": {},
    }
    for stem, produced in ARTIFACTS.items():
        for stage, suffix in (("before", "-before"), ("after", "")):
            raw = (M / "runs" / stage / produced).read_bytes()
            out = R / "artifacts" / f"{stem}{suffix}.json.gz"
            with out.open("wb") as fh:
                with gzip.GzipFile(filename="", mode="wb", fileobj=fh, compresslevel=9, mtime=0) as gz:
                    gz.write(raw)
            manifest["artifacts"][f"{stem}{suffix}"] = {
                "sha256": hashlib.sha256(raw).hexdigest(),
                "bytes": len(raw),
                "produced_as": produced,
                "stage": stage,
            }
    (R / "artifacts" / "artifact-manifest.json").write_text(
        json.dumps(manifest, indent=2, sort_keys=True) + "\n"
    )

    for name in ("mutations.txt", "census.txt", "leaves.txt", "attribution.txt",
                 "byte-identity-before.txt"):
        src = M / name
        if src.exists():
            shutil.copy(src, R / "logs" / name)
    for name in ("run_audit.py", "byte_identity.py", "census.py", "leaves.py",
                 "attribution.py", "mutations.py", "evidence.py",
                 "legacy_joined_provenance.instrumented.rs"):
        shutil.copy(M / name, R / "scripts" / name)

    print("archived:")
    for p in sorted((R / "artifacts").iterdir()):
        print(f"  {p.stat().st_size:>9}  artifacts/{p.name}")
    for p in sorted((R / "logs").iterdir()):
        print(f"  {p.stat().st_size:>9}  logs/{p.name}")
    return 0


if __name__ == "__main__":
    sys.exit(main())

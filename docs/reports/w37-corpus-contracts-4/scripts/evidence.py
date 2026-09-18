#!/usr/bin/env python3
"""Card W37-CORPUS-CONTRACTS-4: archive the evidence beside the report.

Deterministic gzip (mtime 0, level 9) so a re-archive of the same run is
byte-identical. REFUSES to write anything if the BEFORE run has stopped
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
R = WT / "docs" / "reports" / "w37-corpus-contracts-4"
BASELINE = WT / "docs" / "reports" / "joined-provenance-call-frames" / "artifacts"

ARTIFACTS = {
    "diagnostics": "resource-diagnostics.json",
    "source": "monkey-source.json",
    "chunk-contracts": "resource-diagnostics.chunk-contracts.json",
    "source-effects": "resource-diagnostics.source-effects.json",
    "contracts": "resource-diagnostics.contracts.json",
}
VERDICT_BEARING = ("diagnostics", "chunk-contracts")


def main():
    for stem in VERDICT_BEARING:
        measured = (M / "runs" / "before2" / ARTIFACTS[stem]).read_bytes()
        archived = gzip.decompress((BASELINE / f"{stem}.json.gz").read_bytes())
        if measured != archived:
            print(f"REFUSING: BEFORE no longer reproduces the baseline `{stem}`", file=sys.stderr)
            return 1

    (R / "artifacts").mkdir(parents=True, exist_ok=True)
    (R / "logs").mkdir(exist_ok=True)
    (R / "scripts").mkdir(exist_ok=True)

    manifest = {
        "card": "W37-CORPUS-CONTRACTS-4",
        "base": "797661f3",
        "baseline": "docs/reports/joined-provenance-call-frames/artifacts",
        "note": (
            "Declarative card: no checker, loader, backend or any other crate source changed, "
            "so ONE audit binary served both stages and the only difference between them is the "
            "contract manifest (27 -> 33 modules, 49 -> 59 resource-v1 declarations). The card was REBASED from 477b733d onto 797661f3 and BOTH STAGES RE-RUN there against a binary rebuilt on that tree (sha256 05011d2fa5fc789255156dabb18b787f9fb6f5d7918a279e4c7a3e4224f74cab); these archives are the REBASED runs, and they are byte-identical to the pre-rebase runs in all five artifacts, so the two gateway code arbiters beneath this card (SCALAR-ROUTE-EXIT-EDGE, JOINED-PROVENANCE-ROUND-BOUND) move nothing this corpus can see. AFTER is "
            "this card's re-pin. BEFORE reproduces the baseline byte for byte in both "
            "verdict-bearing artifacts AND in `source`; `source-effects` and `contracts` differ "
            "identically in BEFORE and AFTER (checkout root and identity digests) and are not "
            "attributable to this card. `chunk-contracts` is byte-identical between BEFORE and "
            "AFTER: the ten declarations move no chunk contract at all."
        ),
        "artifacts": {},
    }
    for stem, produced in ARTIFACTS.items():
        for stage, suffix in (("before2", "-before"), ("after2", "")):
            raw = (M / "runs" / stage / produced).read_bytes()
            out = R / "artifacts" / f"{stem}{suffix}.json.gz"
            with out.open("wb") as fh:
                with gzip.GzipFile(filename="", mode="wb", fileobj=fh,
                                   compresslevel=9, mtime=0) as gz:
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

    for name in ("byte-identity-before.txt", "byte-identity-after.txt", "census.txt",
                 "attribution.txt", "cover-baseline.txt", "greedy-baseline.txt",
                 "gate-exits.txt", "gate-exits-run1.txt", "gate-exits-run2.txt",
                 "reverify.txt", "before-run.txt", "after-run.txt",
                 "before2-run.txt", "after2-run.txt",
                 "byte-identity-before2.txt", "rebase-after-identity.txt"):
        src = M / "logs" / name
        if src.exists():
            shutil.copy(src, R / "logs" / name)
    for name in ("run_audit.py", "stage.sh", "byte_identity.py", "cover.py", "greedy.py",
                 "census.py", "attribution.py", "evidence.py", "gate.sh", "reverify.sh",
                 "register.py", "stage_out.sh", "compare_after.py", "resolve_matrix.py"):
        src = M / name
        if src.exists():
            shutil.copy(src, R / "scripts" / name)

    print("archived:")
    for p in sorted((R / "artifacts").iterdir()):
        print(f"  {p.stat().st_size:>9}  artifacts/{p.name}")
    for p in sorted((R / "logs").iterdir()):
        print(f"  {p.stat().st_size:>9}  logs/{p.name}")
    return 0


if __name__ == "__main__":
    sys.exit(main())

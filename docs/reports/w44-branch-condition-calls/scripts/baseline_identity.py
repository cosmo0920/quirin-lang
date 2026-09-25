#!/usr/bin/env python3
"""Compare one audit run's five raw artifacts with an archived baseline manifest.

Usage: baseline_identity.py RUN_DIR BASELINE_ARTIFACT_DIR [--root-alias ROOT=CANONICAL]

For each artifact, prints the run's raw SHA-256, the manifest's raw SHA-256 and
whether they are byte-identical. For a non-identical artifact it additionally
reports whether the difference disappears once every occurrence of each ROOT
is rewritten to its CANONICAL spelling (a checkout-path-only difference); that
normalised answer is a disclosure, never a substitute for the raw comparison.
Exits 0 only when every verdict-bearing artifact (the diagnostics) is
byte-identical.
"""

import gzip
import hashlib
import json
import pathlib
import sys

NAMES = {
    "diagnostics": "resource-diagnostics.json",
    "chunk-contracts": "resource-diagnostics.chunk-contracts.json",
    "contracts": "resource-diagnostics.contracts.json",
    "source-effects": "resource-diagnostics.source-effects.json",
    "source": "monkey-source.json",
}


def main() -> int:
    run = pathlib.Path(sys.argv[1])
    baseline = pathlib.Path(sys.argv[2])
    aliases = []
    references = []
    arguments = sys.argv[3:]
    while arguments:
        if arguments[0] == "--root-alias":
            root, canonical = arguments[1].split("=", 1)
            aliases.append((root.encode(), canonical.encode()))
            arguments = arguments[2:]
        elif arguments[0] == "--reference":
            # A later same-source raw checkpoint (for example the canonical
            # contract-location parity run) compared file by file.
            references.append(pathlib.Path(arguments[1]))
            arguments = arguments[2:]
        elif arguments[0] == "--worktree-alias":
            # This checkout's root, rewritten to the main checkout that owns
            # `.claude/worktrees/<name>`, where the W42 baseline was measured.
            root = pathlib.Path(__file__).resolve().parents[4]
            if root.parent.name == "worktrees" and root.parent.parent.name == ".claude":
                aliases.append((str(root).encode(), str(root.parents[2]).encode()))
            arguments = arguments[1:]
        else:
            raise SystemExit(f"unknown argument {arguments[0]}")
    manifest = json.loads((baseline / "manifest.json").read_text())
    result = {"run": str(run), "baseline": str(baseline), "artifacts": {}}
    for label, name in NAMES.items():
        data = (run / name).read_bytes()
        archived = gzip.decompress((baseline / manifest[label]["file"]).read_bytes())
        entry = {
            "file": name,
            "run_sha256": hashlib.sha256(data).hexdigest(),
            "baseline_raw_sha256": manifest[label]["raw_sha256"],
            "archive_matches_manifest": hashlib.sha256(archived).hexdigest()
            == manifest[label]["raw_sha256"],
            "byte_identical": data == archived,
        }
        if not entry["byte_identical"] and aliases:
            normalised = data
            for root, canonical in aliases:
                normalised = normalised.replace(root, canonical)
            entry["identical_after_root_alias"] = normalised == archived
        for reference in references:
            other = (reference / name).read_bytes()
            normalised = data
            for root, canonical in aliases:
                normalised = normalised.replace(root, canonical)
            entry.setdefault("references", []).append(
                {
                    "reference": str(reference / name),
                    "reference_sha256": hashlib.sha256(other).hexdigest(),
                    "byte_identical": data == other,
                    "identical_after_root_alias": normalised == other,
                }
            )
        result["artifacts"][label] = entry
    result["verdict_bearing_identical"] = result["artifacts"]["diagnostics"]["byte_identical"]
    print(json.dumps(result, indent=2))
    return 0 if result["verdict_bearing_identical"] else 1


if __name__ == "__main__":
    sys.exit(main())

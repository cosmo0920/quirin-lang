#!/usr/bin/env python3
"""Extract the selected chunk-function inventory from the pinned sidecar.

Card `W42-CHUNK-LIFECYCLE-CERT`. This script transports identities; it decides
nothing. Every lifecycle verdict is decided in Rust by
`qcore_verification::chunk_lifecycle_coverage`, which
`compiler/tests/chunk_lifecycle_coverage.rs` runs over this file's output and
compares against the committed result.

The extract is refused unless the source artifact's uncompressed content digest
equals the recorded pin, so a re-measured corpus cannot silently re-bind an
already published lifecycle inventory.

    python3 docs/reports/w42-chunk-lifecycle-cert/scripts/lifecycle_inventory_extract.py \
        docs/reports/w42-priority-integration/artifacts/chunk-contracts.json.gz \
        [--check docs/reports/w42-chunk-lifecycle-cert/inventory.json]
"""

import argparse
import gzip
import hashlib
import json
import pathlib
import sys

# Pinned by the wave-42 priority integration and byte-identical to the B-series
# copy under docs/reports/b-series-integration/artifacts/.
EXPECTED_SHA256 = "c4f438e4ed3dab3c3b0e776af5c7c5181385353a72c67b6874ab4df5ba917980"
EXPECTED_SOURCE_REVISION = "66910c10a4d7eafa810b84229cf2dcf7b0f26f97"


def extract(path: pathlib.Path) -> dict:
    stored = path.read_bytes()
    raw = gzip.decompress(stored) if path.suffix == ".gz" else stored
    digest = hashlib.sha256(raw).hexdigest()
    if digest != EXPECTED_SHA256:
        raise SystemExit(
            f"{path}: uncompressed digest {digest} does not match the pinned "
            f"{EXPECTED_SHA256}; re-pin the inventory deliberately"
        )
    artifact = json.loads(raw)
    revision = artifact["source_revision"]
    if revision != EXPECTED_SOURCE_REVISION:
        raise SystemExit(f"{path}: source revision {revision} is not the pinned revision")

    functions = []
    seen = set()
    for row in artifact["results"]:
        identity = (row["library"], row["file"], row["function"])
        if identity in seen:
            raise SystemExit(f"duplicate source-function identity {identity}")
        seen.add(identity)
        status = row["status"]
        if status == "checked":
            field_contract = {"status": "checked", "states": row["checked_states"]}
        elif status == "unknown":
            reason = row.get("reason", "")
            reason_code = row.get("reason_code", "")
            if not reason or not reason_code:
                raise SystemExit(f"{identity}: an Unknown row without a reason is not a residual")
            field_contract = {
                "status": "refused",
                "reason": reason,
                "reason_code": reason_code,
            }
        else:
            raise SystemExit(f"{identity}: unrecognised field-contract status {status!r}")
        functions.append(
            {
                "library": row["library"],
                "file": row["file"],
                "function": row["function"],
                "field_contract": field_contract,
            }
        )
    functions.sort(key=lambda row: (row["library"], row["file"], row["function"]))
    if not functions:
        raise SystemExit(f"{path}: the pinned sidecar selects no function")

    teardown_kinds = sorted(
        {row["kind"] for row in artifact["backend_teardown"] if row.get("kind")}
    )
    return {
        "card": "W42-CHUNK-LIFECYCLE-CERT",
        "artifact": str(path).replace("\\", "/"),
        "artifact_sha256": digest,
        "source_revision": revision,
        "source_profile": "+".join(sorted({row["library"] for row in artifact["results"]})),
        "artifact_scope": artifact["scope"],
        "backend_teardown_records": len(artifact["backend_teardown"]),
        "backend_teardown_kinds": teardown_kinds,
        "selected_functions": len(functions),
        "functions": functions,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("artifact", type=pathlib.Path)
    parser.add_argument("--check", type=pathlib.Path, default=None)
    arguments = parser.parse_args()
    extracted = extract(arguments.artifact)
    rendered = json.dumps(extracted, indent=2) + "\n"
    if arguments.check is None:
        sys.stdout.write(rendered)
        return 0
    committed = arguments.check.read_text()
    if committed != rendered:
        raise SystemExit(f"{arguments.check} differs from the extract of {arguments.artifact}")
    print(f"{arguments.check}: {extracted['selected_functions']} selected functions, extract matches")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

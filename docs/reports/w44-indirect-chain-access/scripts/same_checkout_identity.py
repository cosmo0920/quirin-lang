#!/usr/bin/env python3
"""Card W44-INDIRECT-CHAIN-ACCESS: compare two runs taken in the same checkout.

Usage: same_checkout_identity.py RUN_A RUN_B OUT_JSON

For each of the five raw artifacts, records both SHA-256 digests and whether
the files are byte-identical. Exits 0 only when every artifact is identical.
"""

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


def main():
    first, second = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2])
    result = {"a": str(first), "b": str(second), "artifacts": {}}
    identical = True
    for label, name in NAMES.items():
        left = (first / name).read_bytes()
        right = (second / name).read_bytes()
        same = left == right
        identical &= same
        result["artifacts"][label] = {
            "a_sha256": hashlib.sha256(left).hexdigest(),
            "b_sha256": hashlib.sha256(right).hexdigest(),
            "byte_identical": same,
        }
    result["all_identical"] = identical
    pathlib.Path(sys.argv[3]).write_text(json.dumps(result, indent=1) + "\n")
    print(json.dumps({k: v["byte_identical"] for k, v in result["artifacts"].items()}))
    return 0 if identical else 1


if __name__ == "__main__":
    sys.exit(main())

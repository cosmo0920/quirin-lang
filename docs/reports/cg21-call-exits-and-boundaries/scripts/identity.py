#!/usr/bin/env python3
"""CG21-CALL-EXITS-AND-BOUNDARIES: raw and tree-normalised digests of route
reports.

usage: identity.py REPORT_JSON [REPORT_JSON ...]

A `quirin-audit current` report depends on the checkout path of the tree it
ran in: review holds name their contract module by absolute path, and the
per-unit `resource_contract_sources` and `inventory_sha256` digests cover the
contract sources' paths. A BEFORE over an extraction of the base tree
(private/cg21/basetree) and a run in another worktree therefore differ in
those bytes alone. The normalised digest is the SHA-256 of the report re-dumped
canonically (sorted keys) with every `/…/contract/profiles/` prefix replaced
by `<tree>/contract/profiles/` and those two digests replaced by
`<path-dependent>`; the raw digest is printed beside it.
"""

import hashlib
import json
import pathlib
import re
import sys

PREFIX = re.compile(r"/[^\"\s]*?/contract/profiles/")
PATH_DEPENDENT = {"resource_contract_sources", "inventory_sha256"}


def normalise(value):
    if isinstance(value, dict):
        return {
            key: ("<path-dependent>" if key in PATH_DEPENDENT else normalise(item))
            for key, item in value.items()
        }
    if isinstance(value, list):
        return [normalise(item) for item in value]
    if isinstance(value, str):
        return PREFIX.sub("<tree>/contract/profiles/", value)
    return value


def main():
    for name in sys.argv[1:]:
        raw = pathlib.Path(name).read_bytes()
        canonical = json.dumps(
            normalise(json.loads(raw)), sort_keys=True, ensure_ascii=False
        ).encode()
        print(hashlib.sha256(raw).hexdigest(), hashlib.sha256(canonical).hexdigest(), name)


if __name__ == "__main__":
    main()

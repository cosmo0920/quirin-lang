#!/usr/bin/env python3
"""Whether an arbiter run's contract sidecar differs from the archived
baseline in exactly this card's changes.

usage: contracts_difference.py RUN_DIR BASELINE_ARTIFACT_DIR ROOT=CANONICAL

Card W45-CONTRACT-REVISION-MODE changes the contract sidecar in three ways,
and in no other: it adds the `contract_revision` key; it marks every libc
contract `revision flexible`, so each libc module's retained text gains one
comment block and one `requires resource_revision(flexible);` line per
declaration; and, since `contract_inclusion` retains the text and SHA-256 of
every included libc parent, those change with the parents' text. This script
undoes exactly those edits on the run's sidecar (after the root alias), drops
the additive key, recomputes the included parents' digests from the restored
text, and reports whether the result equals the baseline as JSON. It is a
disclosure beside `baseline_identity.py`, never a substitute for it.
"""

import gzip
import hashlib
import json
import pathlib
import sys

MARKER = "    requires resource_revision(flexible);\n"
NOTE = """// Card W45-CONTRACT-REVISION-MODE: every contract in this module is
// `revision flexible` (contract spec C06.2): it binds by name and declared
// signature at any revision of the audited project, and reports list the libc
// contracts as one standing assumption, the libc/C-standard ABI premise. The
// C library's version stays in the build's provenance.

"""


def restore(text):
    return text.replace(NOTE, "").replace(MARKER, "")


def main() -> int:
    run = pathlib.Path(sys.argv[1]) / "resource-diagnostics.contracts.json"
    baseline_dir = pathlib.Path(sys.argv[2])
    root, canonical = sys.argv[3].split("=", 1)
    manifest = json.loads((baseline_dir / "manifest.json").read_text())
    baseline = json.loads(gzip.decompress(
        (baseline_dir / manifest["contracts"]["file"]).read_bytes()))
    data = json.loads(run.read_text().replace(root, canonical))
    present = "contract_revision" in data and "contract_revision" not in baseline
    data.pop("contract_revision", None)
    libc_modules = []
    for key, text in list(data["sources"].items()):
        if "/quirin-provider-libc/" in key and restore(text) != text:
            libc_modules.append(key)
            data["sources"][key] = restore(text)
    parents = []
    digests = {}
    modules = data.get("contract_inclusion", {}).get("modules", {}).values()
    for module in modules:
        for key, included in module.get("included", {}).items():
            if key.startswith("quirin-provider-libc:"):
                old = included["identity"]["sha256"]
                included["source"] = restore(included["source"])
                included["identity"]["sha256"] = hashlib.sha256(
                    included["source"].encode()).hexdigest()
                digests[old] = included["identity"]["sha256"]
                parents.append(key)
    # Each inheritance chain names its parent module by the same digest.
    for module in modules:
        for chain in module.get("inherited", {}).values():
            for link in chain:
                identity = link["module"]
                identity["sha256"] = digests.get(identity["sha256"], identity["sha256"])
    result = {
        "additive_key_present": present,
        "libc_modules_restored": len(libc_modules),
        "included_libc_parents_restored": sorted(set(parents)),
        "identical_after_restoring_the_card_edits": data == baseline,
        "top_level_keys_differing": sorted(
            key for key in set(data) | set(baseline) if data.get(key) != baseline.get(key)),
    }
    print(json.dumps(result, indent=2))
    return 0 if result["identical_after_restoring_the_card_edits"] and present else 1


if __name__ == "__main__":
    sys.exit(main())

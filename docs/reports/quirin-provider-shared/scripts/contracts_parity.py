#!/usr/bin/env python3
"""Show that a corpus run's contracts artifact differs from its predecessor by path only.

Usage: contracts_parity.py BEFORE_RUN AFTER_RUN

Reads `resource-diagnostics.contracts.json` from both run directories (the same
checkout, before and after this card). Rewrites every new shared-crate path
spelling in AFTER back to the former `providers/shared/contracts/` spelling:

    providers/quirin-provider-<library>/contracts/  ->  providers/shared/contracts/
    providers/quirin-provider-shared/contracts/     ->  providers/shared/contracts/

and requires the rewritten document to equal BEFORE exactly: the same source
keys with byte-identical module texts, the same embedded manifest, and every
other top-level key unchanged. It also runs the constraint-22 itemiser
(`docs/reports/corpus-repin/diff_artifacts.py`'s `diff_contracts`) on the raw
pair and reports its module-row counts, which must show no added, removed or
changed logical module. Prints one JSON document; exit 0 only if both hold.
"""

import importlib.util
import json
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[4]
NAME = "resource-diagnostics.contracts.json"
NEW = re.compile(r"providers/quirin-provider-[a-z-]+/contracts/")
OLD = "providers/shared/contracts/"


def main() -> int:
    before_text = (pathlib.Path(sys.argv[1]) / NAME).read_text()
    after_text = (pathlib.Path(sys.argv[2]) / NAME).read_text()
    before, after = json.loads(before_text), json.loads(after_text)
    rewritten = json.loads(NEW.sub(OLD, after_text))
    moved = sorted(key for key in after["sources"] if NEW.search(key))
    unchanged = sorted(key for key in after["sources"] if key in before["sources"])
    keys = sorted(set(before) | set(after))
    result = {
        "before_sources": len(before["sources"]),
        "after_sources": len(after["sources"]),
        "moved_source_keys": len(moved),
        "unchanged_source_keys": len(unchanged),
        "moved_texts_identical": all(
            after["sources"][key] == before["sources"][NEW.sub(OLD, key)] for key in moved
        ),
        "top_level_keys_differing_raw": [key for key in keys if before.get(key) != after.get(key)],
        "identical_after_path_rewrite": rewritten == before,
    }
    spec = importlib.util.spec_from_file_location(
        "diff_artifacts", ROOT / "docs/reports/corpus-repin/diff_artifacts.py"
    )
    tool = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(tool)
    section = tool.diff_contracts(before, after, False)
    result["diff_artifacts_source_counts"] = section["sources"]["counts"]
    result["diff_artifacts_key_collisions"] = section["sources"].get("key_collisions", [])
    counts = section["sources"]["counts"]
    ok = (
        result["identical_after_path_rewrite"]
        and result["moved_texts_identical"]
        and counts["added"] == counts["removed"] == counts["changed"] == 0
        and not result["diff_artifacts_key_collisions"]
    )
    result["path_only"] = ok
    print(json.dumps(result, indent=1))
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())

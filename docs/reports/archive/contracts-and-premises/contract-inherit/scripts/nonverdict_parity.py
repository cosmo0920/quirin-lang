#!/usr/bin/env python3
"""Attribute this card's two non-verdict artifact differences from the W43 baseline.

Usage: nonverdict_parity.py RUN_DIR OUT_JSON

* contracts: rewrite this checkout's root to the W43 checkout's root, and the
  post-QUIRIN-PROVIDER-SHARED crate paths back to `providers/shared/contracts/`
  (the rewrite that card's `contracts_parity.py` performs), then require the
  document to equal the W43 archive exactly.
* source-effects: list every differing JSON leaf against the W43 archive with
  the root alias applied, and the set of differing key names, so a reader can
  check they are the checkout-path digests the W43 integration confirmation
  attributed (41 leaves in QUIRIN-PROVIDER-SHARED's measurement).

Writes one JSON document and exits 0 only if the contracts document is equal.
"""
import gzip
import json
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[4]
W43 = ROOT / "docs/reports/w43-return-expression/artifacts"
W43_ROOT = "/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/" + "agent-ae2a69ef909727ba3"
CRATE_PATH = re.compile(r"providers/quirin-provider-[a-z-]+/contracts/")


def leaves(value, path=()):
    if isinstance(value, dict):
        for key, item in value.items():
            yield from leaves(item, path + (key,))
    elif isinstance(value, list):
        for index, item in enumerate(value):
            yield from leaves(item, path + (index,))
    else:
        yield path, value


def main() -> int:
    run = pathlib.Path(sys.argv[1])
    out = pathlib.Path(sys.argv[2])
    result = {}
    text = (run / "resource-diagnostics.contracts.json").read_text()
    text = text.replace(str(ROOT), W43_ROOT)
    rewritten = json.loads(CRATE_PATH.sub("providers/shared/contracts/", text))
    baseline = json.loads(gzip.decompress((W43 / "contracts.json.gz").read_bytes()))
    result["contracts_equal_after_root_alias_and_crate_path_rewrite"] = rewritten == baseline
    result["contracts_source_keys"] = len(baseline.get("sources", {}))
    result["contracts_texts_identical_by_key"] = rewritten.get("sources") == baseline.get("sources")
    effects = json.loads(
        (run / "resource-diagnostics.source-effects.json").read_text().replace(str(ROOT), W43_ROOT))
    effects_base = json.loads(gzip.decompress((W43 / "source-effects.json.gz").read_bytes()))
    left = dict(leaves(effects_base))
    right = dict(leaves(effects))
    differing = sorted(set(left) ^ set(right) | {k for k in left if k in right and left[k] != right[k]},
                       key=str)
    result["source_effects_differing_leaves"] = len(differing)
    result["source_effects_differing_key_names"] = sorted(
        {str(next((p for p in reversed(k) if isinstance(p, str)), "")) for k in differing})
    out.write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(result, indent=2))
    return 0 if result["contracts_equal_after_root_alias_and_crate_path_rewrite"] else 1


if __name__ == "__main__":
    sys.exit(main())

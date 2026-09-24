#!/usr/bin/env python3
"""Attribute the non-verdict-bearing arbiter artifacts that differ from a baseline.

Usage: attribute_arbiter.py RUN_DIR BASELINE_ARTIFACT_DIR THIS_ROOT BASELINE_ROOT OUT.json

For `contracts` and `source-effects`: rewrite THIS_ROOT to BASELINE_ROOT in the
run's text, then list every differing JSON leaf path, the contract-module files
whose entries differ, and whether any differing leaf mentions this card's new
module (`descriptor_native_observation`). Read-only on both inputs.
"""
import gzip
import json
import pathlib
import sys

ARTIFACTS = {"contracts": ("resource-diagnostics.contracts.json", "contracts.json.gz"),
             "source-effects": ("resource-diagnostics.source-effects.json", "source-effects.json.gz")}


def leaves(value, path=()):
    if isinstance(value, dict):
        for key, item in value.items():
            yield from leaves(item, path + (str(key),))
    elif isinstance(value, list):
        for index, item in enumerate(value):
            yield from leaves(item, path + (str(index),))
    else:
        yield path, value


def main() -> int:
    run, baseline, this_root, baseline_root, out = sys.argv[1:6]
    result = {}
    for label, (run_name, baseline_name) in ARTIFACTS.items():
        mine = (pathlib.Path(run) / run_name).read_text().replace(this_root, baseline_root)
        theirs = gzip.decompress((pathlib.Path(baseline) / baseline_name).read_bytes()).decode()
        left = dict(leaves(json.loads(theirs)))
        right = dict(leaves(json.loads(mine)))
        differing = sorted(set(left) ^ set(right) | {p for p in set(left) & set(right) if left[p] != right[p]})
        modules = sorted({part for path in differing for part in path if part.endswith(".qcore")})
        keys = sorted({path[-1] for path in differing if not path[-1].isdigit()})
        result[label] = {
            "identical_after_root_rewrite": mine == theirs,
            "differing_leaves": len(differing),
            "differing_modules": modules,
            "differing_key_names": keys[:60],
            "mentions_descriptor_native_observation": "descriptor_native_observation" in mine,
            "examples": [{"path": "/".join(p), "baseline": left.get(p), "run": right.get(p)} for p in differing[:12]],
        }
    pathlib.Path(out).write_text(json.dumps(result, indent=1) + "\n")
    print(json.dumps({k: {x: v[x] for x in ("identical_after_root_rewrite", "differing_leaves",
                                             "mentions_descriptor_native_observation")} | {"modules": len(v["differing_modules"])}
                      for k, v in result.items()}, indent=1))
    return 0


if __name__ == "__main__":
    sys.exit(main())

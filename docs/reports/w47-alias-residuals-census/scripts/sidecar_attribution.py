#!/usr/bin/env python3
"""Attribute the two non-identical sidecars of a post-merge run to named commits.

usage: sidecar_attribution.py BASELINE_DIR RUN_DIR OUT_JSON --alias FROM=TO [--alias ...]
                              [--names FILE,FILE,...]

`--names` replaces the two sidecars by other artifacts of the run (the system
slice's diagnostics carry the contract manifest and source digests in each
unit's compile configuration, so they are checked the same way).

The contracts sidecar and the source-effects sidecar carry provenance (module
paths, digests over them); the diagnostics do not. On `9b2ac682..3ba72320`
EXT-02-CONTRACT-EXTENSIONS (`.qcore` -> `.qv`, archived in
docs/reports/extension-contract-physical/) renamed every contract module.
This script checks, per artifact:

* `equal_after_root_alias_and_rename`: the run equals the baseline once the
  checkout-root alias is applied and every `.qcore` on both sides is spelt
  `.qv` (EXT-02 renamed the files and the prose that names them; comments
  that kept the old spelling are normalised the same way on both sides);
* `structural_equal_with_digests_masked`: equal as JSON after the rename and
  with every run of 16 or more hex digits masked (module, lock, task,
  obligation and configuration digests hash the renamed paths and texts), so
  every structural field (verdict, scope, bounds, assumptions, profiles,
  contract text) is equal.
Contract source texts (`sources`) are compared after the rename and after
collapsing runs of spaces (an included module's `include` line is recorded
blanked to its own length, three spaces shorter per `.qv` include); any other
text difference is reported and fails the check.
"""

import json
import re
import sys

HEX = re.compile(r"[0-9a-f]{16,}")


def mask(value):
    if isinstance(value, dict):
        return {mask_key(k): mask(v) for k, v in value.items()}
    if isinstance(value, list):
        return [mask(v) for v in value]
    if isinstance(value, str):
        return HEX.sub("<digest>", value)
    return value


def mask_key(key):
    return HEX.sub("<digest>", key)


def differing(a, b, path, out):
    if type(a) != type(b):
        out.append(path)
    elif isinstance(a, dict):
        for k in set(a) | set(b):
            if k not in a or k not in b:
                out.append(path + "/" + k)
            else:
                differing(a[k], b[k], path + "/" + k, out)
    elif isinstance(a, list):
        if len(a) != len(b):
            out.append(path)
        for i, (x, y) in enumerate(zip(a, b)):
            differing(x, y, f"{path}[{i}]", out)
    elif a != b:
        out.append(path)


def main():
    baseline, run, out = sys.argv[1:4]
    aliases = []
    names = ["resource-diagnostics.contracts.json", "resource-diagnostics.source-effects.json"]
    rest = sys.argv[4:]
    while rest:
        if rest[0] == "--alias":
            aliases.append(rest[1].split("=", 1))
        elif rest[0] == "--names":
            names = rest[1].split(",")
        else:
            raise SystemExit(f"unknown argument {rest[0]}")
        rest = rest[2:]
    result = {}
    for name in names:
        old_text = open(f"{baseline}/{name}").read().replace(".qcore", ".qv")
        new_text = open(f"{run}/{name}").read().replace(".qcore", ".qv")
        for source, target in aliases:
            new_text = new_text.replace(source, target)
        # A module's resolved `include` line is blanked to its own length in
        # the recorded text, so the rename shortens that blank by three
        # spaces per include: space runs are collapsed on both sides.
        old_text, new_text = re.sub(r" {2,}", " ", old_text), re.sub(r" {2,}", " ", new_text)
        old, new = json.loads(old_text), json.loads(new_text)
        paths = []
        differing(old, new, "", paths)
        entry = {
            "equal_after_root_alias_and_rename": old == new,
            "structural_equal_with_digests_masked": mask(old) == mask(new),
            "differing_paths": len(paths),
            "differing_path_sample": sorted(paths)[:20],
        }
        if isinstance(old, dict) and "sources" in old:
            entry["contract_texts_equal_after_rename"] = old["sources"] == new.get("sources")
            entry["contract_modules"] = len(old["sources"])
        result[name] = entry
    open(out, "w").write(json.dumps(result, indent=1, sort_keys=True) + "\n")
    print(json.dumps(result, indent=1, sort_keys=True))
    ok = all(v["structural_equal_with_digests_masked"] and v.get("contract_texts_equal_after_rename", True)
             for v in result.values())
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())

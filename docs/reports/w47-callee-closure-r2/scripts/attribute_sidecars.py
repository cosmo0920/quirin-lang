#!/usr/bin/env python3
"""Attribute a BEFORE sidecar's difference from the P3 archive to named commits.

Usage: attribute_sidecars.py BASELINE_DIR RUN_DIR OUT_JSON --root-alias RUN_ROOT=BASELINE_ROOT

For the contracts and source-effects sidecars, the baseline text is rewritten
by each named commit's mechanical change on `9b2ac682..3ba72320`, in order,
and the result compared with the run after the checkout-root alias:

* `f4018a92` ("Migrate contract sources and lock paths to .qv"): every
  contract module path `*.qcore` became `*.qv`, and an `#include` directive's
  text names the parent with its new extension;
* digests over those texts (module SHA-256s and every digest that binds them)
  are then recomputed by the run, so the remaining differing leaves must all
  be 64-hex digests.

Exit 0 only when every remaining difference is a hex digest leaf whose path
also differs in a digest-bearing key.
"""

import gzip
import json
import pathlib
import re
import sys

HEX = re.compile(r"[0-9a-f]{64}")
# A digest-derived identifier: `resource_contracts:<first 24 hex of a digest>`.
IDENT = re.compile(r"resource_contracts:[0-9a-f]{24}")
NAMES = ("resource-diagnostics.contracts.json", "resource-diagnostics.source-effects.json")


def read(path):
    if path.exists():
        return path.read_bytes()
    return gzip.decompress(path.with_name(path.name + ".gz").read_bytes())


def walk(old, new, path, out):
    if isinstance(old, dict) and isinstance(new, dict):
        only_old = sorted(set(old) - set(new))
        only_new = sorted(set(new) - set(old))
        if (len(only_old) == len(only_new) == 1 and IDENT.fullmatch(only_old[0])
                and IDENT.fullmatch(only_new[0])):
            # One digest-derived key renamed: compare its values in place.
            out.append({"path": f"{path}/<{only_old[0]} -> {only_new[0]}>", "kind": "digest"})
            walk(old[only_old[0]], new[only_new[0]], f"{path}/<renamed key>", out)
        else:
            for key in only_old + only_new:
                out.append({"path": f"{path}/{key}", "kind": "key-set"})
        for key in sorted(set(old) & set(new)):
            walk(old[key], new[key], f"{path}/{key}", out)
        return
    if isinstance(old, list) and isinstance(new, list) and len(old) == len(new):
        for index, (a, b) in enumerate(zip(old, new)):
            walk(a, b, f"{path}[{index}]", out)
        return
    if old != new:
        digest = isinstance(old, str) and isinstance(new, str) and (
            (HEX.fullmatch(old) and HEX.fullmatch(new))
            or (IDENT.fullmatch(old) and IDENT.fullmatch(new)))
        out.append({"path": path, "kind": "digest" if digest else "other",
                    "old": old if not isinstance(old, str) or len(old) < 200 else old[:200],
                    "new": new if not isinstance(new, str) or len(new) < 200 else new[:200]})


BLANK = re.compile(r"(?m)^ +$")


def rename(value):
    """f4018a92's rename: a module path or key `*.qcore` is `*.qv`. A module
    text (a long string) keeps its own words; only an `#include` directive's
    parent and the blank padding the loader leaves where it expanded one change,
    so a text is normalised (directive extension, blank-line padding) rather
    than renamed."""
    if isinstance(value, dict):
        return {rename(key): rename(item) for key, item in value.items()}
    if isinstance(value, list):
        return [rename(item) for item in value]
    if isinstance(value, str):
        if len(value) < 400 or value.startswith("{"):
            # A path, or an embedded JSON manifest listing module paths.
            return value.replace(".qcore", ".qv")
        return BLANK.sub("", value.replace('.qcore";', '.qv";'))
    return value


def normalise_texts(value):
    if isinstance(value, dict):
        return {key: normalise_texts(item) for key, item in value.items()}
    if isinstance(value, list):
        return [normalise_texts(item) for item in value]
    if isinstance(value, str) and len(value) >= 400:
        return BLANK.sub("", value)
    return value


def main():
    baseline, run, out = map(pathlib.Path, sys.argv[1:4])
    alias = sys.argv[sys.argv.index("--root-alias") + 1].split("=", 1)
    result = {}
    clean = True
    for name in NAMES:
        old_raw = read(baseline / name)
        new_raw = read(run / name).replace(alias[0].encode(), alias[1].encode())
        renamed = rename(json.loads(old_raw))
        new = normalise_texts(json.loads(new_raw))
        entry = {
            "raw_identical": old_raw == new_raw,
            "identical_after_f4018a92_rename": renamed == new,
        }
        if renamed != new:
            differences = []
            walk(renamed, new, "", differences)
            kinds = {}
            for item in differences:
                kinds[item["kind"]] = kinds.get(item["kind"], 0) + 1
            entry["remaining_after_rename"] = kinds
            entry["non_digest"] = [item for item in differences if item["kind"] != "digest"]
            entry["digest_paths"] = sorted({re.sub(r"\[\d+\]", "[]", item["path"])
                                            for item in differences if item["kind"] == "digest"})
            if entry["non_digest"]:
                clean = False
        result[name] = entry
    result["attributed"] = clean
    pathlib.Path(out).write_text(json.dumps(result, indent=1) + "\n")
    print(json.dumps({k: (v if not isinstance(v, dict) else {kk: vv for kk, vv in v.items() if kk != "non_digest"})
                      for k, v in result.items()}, indent=1)[:4000])
    return 0 if clean else 1


if __name__ == "__main__":
    sys.exit(main())

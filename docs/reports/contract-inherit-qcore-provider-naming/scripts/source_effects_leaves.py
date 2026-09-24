#!/usr/bin/env python3
"""List every differing leaf of two `resource-diagnostics.source-effects.json`
files (card CONTRACT-INHERIT-QCORE-PROVIDER-NAMING), after rewriting each
checkout ROOT to a common spelling.

Usage: source_effects_leaves.py LEFT_DIR RIGHT_DIR OUT.json [--root-alias ROOT=CANONICAL ...]

Each differing leaf is recorded with its JSON path and both values, and
classified: `checkout-path digest` when the leaf's key is a digest or
identifier derived from the configuration (`build_digest`,
`configuration_digest`, `task_digest`, `obligation_id`) or a string that names
`resource_contracts:<digest>`; `contract_sources digest`; `resource_contracts:<digest>
key` when a bound is keyed by the environment digest (the script also checks
that every such removed key has an added twin with an equal value); otherwise
`other`, which needs its own attribution. A path-only rewrite is a disclosure; nothing is normalised in the
raw comparison the caller makes separately.
"""
import json
import sys
from pathlib import Path

NAME = "resource-diagnostics.source-effects.json"
DIGEST_KEYS = {"build_digest", "configuration_digest", "task_digest", "obligation_id"}


def load(directory, aliases):
    text = (Path(directory) / NAME).read_text()
    for root, canonical in aliases:
        text = text.replace(root, canonical)
    return json.loads(text)


def walk(left, right, path, out):
    if isinstance(left, dict) and isinstance(right, dict):
        for key in sorted(set(left) | set(right)):
            walk(left.get(key), right.get(key), path + [key], out)
    elif isinstance(left, list) and isinstance(right, list) and len(left) == len(right):
        for index, (a, b) in enumerate(zip(left, right)):
            walk(a, b, path + [index], out)
    elif left != right:
        out.append((path, left, right))


def classify(path, left, right):
    keys = {part for part in path if isinstance(part, str)}
    if keys & DIGEST_KEYS:
        return "checkout-path digest"
    if "contract_sources" in keys:
        return "contract_sources digest"
    if any(key.startswith("resource_contracts:") for key in keys):
        return "resource_contracts:<digest> key"
    text = json.dumps([left, right])
    if "resource_contracts:" in text:
        return "checkout-path digest"
    return "other"


def renamed_keys_keep_values(rows):
    """Every `resource_contracts:<digest>` key removed from a result must be
    matched by one added to the same parent with an equal value: the key's
    digest moved, the bound it names did not."""
    removed, added = {}, {}
    for row in rows:
        if row["class"] != "resource_contracts:<digest> key":
            continue
        parent = json.dumps(row["path"][:-1])
        if row["right"] is None:
            removed.setdefault(parent, []).append(json.dumps(row["left"], sort_keys=True))
        elif row["left"] is None:
            added.setdefault(parent, []).append(json.dumps(row["right"], sort_keys=True))
        else:
            return False
    return {k: sorted(v) for k, v in removed.items()} == {k: sorted(v) for k, v in added.items()}


def main():
    left_dir, right_dir, out = sys.argv[1:4]
    aliases = []
    rest = sys.argv[4:]
    while rest:
        if rest[0] != "--root-alias":
            raise SystemExit(f"unknown argument {rest[0]}")
        root, canonical = rest[1].split("=", 1)
        aliases.append((root, canonical))
        rest = rest[2:]
    differences = []
    walk(load(left_dir, aliases), load(right_dir, aliases), [], differences)
    rows = [{"path": path, "left": left, "right": right, "class": classify(path, left, right)}
            for path, left, right in differences]
    summary = {}
    for row in rows:
        summary[row["class"]] = summary.get(row["class"], 0) + 1
    keep = renamed_keys_keep_values(rows)
    Path(out).write_text(json.dumps({"leaves": len(rows), "classes": summary,
                                     "renamed_bound_keys_keep_values": keep, "rows": rows},
                                    indent=1) + "\n")
    print(json.dumps({"leaves": len(rows), "classes": summary,
                      "renamed_bound_keys_keep_values": keep}))


if __name__ == "__main__":
    main()

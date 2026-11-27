#!/usr/bin/env python3
"""Compare two arbiter runs artifact by artifact (card CONTRACT-INHERIT-ADOPTION).

Usage: arbiter_compare.py LEFT_DIR RIGHT_DIR OUT.json [--root-alias ROOT=CANONICAL ...]

For each of the five audit artifacts: raw SHA-256 on both sides and byte
identity. For the verdict-bearing diagnostics that differ, every changed leaf
is listed (unit, function, description, line) with its side, so a delta can be
attributed leaf by leaf. For the `contracts` sidecar, the manifest text, every
source key added, removed or changed, and the `contract_inclusion` block are
itemised, after rewriting each ROOT to CANONICAL (checkout-path identity).
For `source-effects`, the differing leaf paths are listed after the same
rewrite. Nothing is normalised in the raw comparison.
"""
import hashlib
import json
import sys
from collections import Counter
from pathlib import Path

NAMES = {
    "diagnostics": "resource-diagnostics.json",
    "chunk-contracts": "resource-diagnostics.chunk-contracts.json",
    "contracts": "resource-diagnostics.contracts.json",
    "source-effects": "resource-diagnostics.source-effects.json",
    "source": "monkey-source.json",
}


def leaves(value, path=""):
    if isinstance(value, dict):
        for key, child in value.items():
            yield from leaves(child, f"{path}/{key}")
    elif isinstance(value, list):
        for index, child in enumerate(value):
            yield from leaves(child, f"{path}[{index}]")
    else:
        yield path, value


def unit_name(unit):
    return f"{unit.get('library', '')}:{unit.get('file', '')}"


def diagnostic_leaves(document):
    rows = Counter()
    for unit in document.get("units", []):
        for entry in unit.get("functions", []):
            for diagnostic in entry.get("diagnostics") or []:
                rows[(unit_name(unit), entry.get("function", ""), diagnostic.get("category", ""),
                      diagnostic.get("description", ""), str(diagnostic.get("line", "")),
                      diagnostic.get("origin", ""))] += 1
    return rows


def statuses(document):
    functions, units = {}, {}
    for unit in document.get("units", []):
        units[unit_name(unit)] = unit.get("status")
        for entry in unit.get("functions", []):
            functions[(unit_name(unit), entry.get("function", ""))] = entry.get("status")
    return functions, units


def rewrite(text, aliases):
    for root, canonical in aliases:
        text = text.replace(root, canonical)
    return text


def main():
    left, right, out = Path(sys.argv[1]), Path(sys.argv[2]), Path(sys.argv[3])
    aliases = []
    arguments = sys.argv[4:]
    while arguments:
        if arguments[0] == "--root-alias":
            root, canonical = arguments[1].split("=", 1)
            aliases.append((root, canonical))
            arguments = arguments[2:]
        else:
            raise SystemExit(f"unknown argument {arguments[0]}")
    record = {"left": str(left), "right": str(right), "artifacts": {}}
    for name, file in NAMES.items():
        a, b = (left / file).read_bytes(), (right / file).read_bytes()
        entry = {
            "left_sha256": hashlib.sha256(a).hexdigest(),
            "right_sha256": hashlib.sha256(b).hexdigest(),
            "byte_identical": a == b,
        }
        if a != b:
            ja = json.loads(rewrite(a.decode(), aliases))
            jb = json.loads(rewrite(b.decode(), aliases))
            entry["identical_after_root_alias"] = ja == jb
            if name == "diagnostics":
                la, lb = diagnostic_leaves(ja), diagnostic_leaves(jb)
                removed = la - lb
                added = lb - la
                entry["removed_leaves"] = sum(removed.values())
                entry["added_leaves"] = sum(added.values())
                entry["removed"] = [list(k) + [n] for k, n in sorted(removed.items())]
                entry["added"] = [list(k) + [n] for k, n in sorted(added.items())]
                fa, ua = statuses(ja)
                fb, ub = statuses(jb)
                entry["function_status_moves"] = sorted(
                    [list(k), fa.get(k), fb.get(k)] for k in set(fa) | set(fb) if fa.get(k) != fb.get(k))
                entry["unit_status_moves"] = sorted(
                    [k, ua.get(k), ub.get(k)] for k in set(ua) | set(ub) if ua.get(k) != ub.get(k))
                other = {p: v for p, v in leaves(ja) if "/units[" not in p}
                other_b = {p: v for p, v in leaves(jb) if "/units[" not in p}
                entry["top_level_changed"] = sorted(p for p in set(other) | set(other_b)
                                                    if other.get(p) != other_b.get(p))
            elif name == "contracts":
                entry["manifest_changed"] = ja.get("manifest") != jb.get("manifest")
                sa, sb = ja.get("sources", {}), jb.get("sources", {})
                entry["sources_removed"] = sorted(set(sa) - set(sb))
                entry["sources_added"] = sorted(set(sb) - set(sa))
                entry["sources_changed"] = sorted(k for k in set(sa) & set(sb) if sa[k] != sb[k])
                entry["contract_inclusion_left"] = "contract_inclusion" in ja
                entry["contract_inclusion_right"] = "contract_inclusion" in jb
                entry["other_keys_changed"] = sorted(
                    k for k in (set(ja) | set(jb)) - {"manifest", "sources", "contract_inclusion"}
                    if ja.get(k) != jb.get(k))
                if "contract_inclusion" in jb:
                    chains = {}
                    for module, evidence in jb["contract_inclusion"]["modules"].items():
                        for child, chain in evidence["inherited"].items():
                            chains[f"{module}#{child}"] = [
                                f"{link['module']['crate_name']}@{link['module']['crate_version']}:"
                                f"{link['module']['module']}#{link['declaration']} "
                                f"sha256={link['module']['sha256'][:16]}" for link in chain]
                    entry["right_chains"] = chains
            else:
                la, lb = dict(leaves(ja)), dict(leaves(jb))
                changed = sorted(p for p in set(la) | set(lb) if la.get(p) != lb.get(p))
                entry["changed_leaf_count"] = len(changed)
                entry["changed_leaf_keys"] = sorted(Counter(p.rsplit("/", 1)[-1].split("[")[0]
                                                            for p in changed).items())
        record["artifacts"][name] = entry
    record["verdict_bearing_identical"] = record["artifacts"]["diagnostics"]["byte_identical"]
    out.write_text(json.dumps(record, indent=2) + "\n")
    summary = {name: {k: v for k, v in entry.items() if k in (
        "byte_identical", "identical_after_root_alias", "removed_leaves", "added_leaves",
        "sources_removed", "sources_added", "sources_changed", "changed_leaf_count",
        "changed_leaf_keys", "manifest_changed", "contract_inclusion_right", "other_keys_changed")}
        for name, entry in record["artifacts"].items()}
    print(json.dumps(summary, indent=1))
    return 0


if __name__ == "__main__":
    sys.exit(main())

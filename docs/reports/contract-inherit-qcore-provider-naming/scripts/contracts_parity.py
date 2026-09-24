#!/usr/bin/env python3
"""Show that the `contracts` sidecar moved only where this card says it moved
(card CONTRACT-INHERIT-QCORE-PROVIDER-NAMING).

Usage: contracts_parity.py BEFORE_DIR AFTER_DIR OUT.json [--root-alias ROOT=CANONICAL ...]

After rewriting each checkout ROOT to a common spelling it checks, and
records:

* every top-level key other than `manifest`, `sources` and
  `contract_inclusion` is equal;
* `manifest` differs only by the two selection entries that moved from a path
  to a `crate:module` reference (`monkey/memory.qcore`,
  `librdkafka/memory.qcore`), in the same positions;
* `sources` differs only by those two keys: each removed path key's text is
  the hand-written module as it stood at the base (compared with the fixture
  archived for the parity tests), and each added reference key's text is
  that module's elaboration; every other source is byte-equal;
* `contract_inclusion` keeps every BEFORE module record unchanged and adds
  exactly the two converted modules, with 5 and 3 chains of one link each.

Exits 0 only when all of that holds.
"""
import json
import sys
from pathlib import Path

NAME = "resource-diagnostics.contracts.json"
MOVED = {
    "providers/qcore-provider-libevent/contracts/monkey/memory.qcore":
        ("qcore-provider-libevent:monkey/memory.qcore", "monkey_memory_hand_written.qcore", 3),
    "providers/qcore-provider-librdkafka/contracts/librdkafka/memory.qcore":
        ("qcore-provider-librdkafka:librdkafka/memory.qcore", "librdkafka_memory_hand_written.qcore", 5),
}
FIXTURES = Path(__file__).resolve().parents[4] / "compiler/tests/fixtures/contract_inherit_qcore_provider"


def load(directory, aliases):
    text = (Path(directory) / NAME).read_text()
    for root, canonical in aliases:
        text = text.replace(root, canonical)
    return json.loads(text)


def main():
    before_dir, after_dir, out = sys.argv[1:4]
    aliases = []
    rest = sys.argv[4:]
    while rest:
        root, canonical = rest[1].split("=", 1)
        aliases.append((root, canonical))
        rest = rest[2:]
    before, after = load(before_dir, aliases), load(after_dir, aliases)
    checks = {}
    checks["other_top_level_keys_equal"] = all(
        before.get(key) == after.get(key)
        for key in set(before) | set(after) if key not in {"manifest", "sources", "contract_inclusion"})
    manifest_before = json.loads(before["manifest"])["modules"]
    manifest_after = json.loads(after["manifest"])["modules"]
    expected = [MOVED[entry][0] if entry in MOVED else entry for entry in manifest_before]
    checks["manifest_moves_only_two_entries_in_place"] = manifest_after == expected
    removed = sorted(set(before["sources"]) - set(after["sources"]))
    added = sorted(set(after["sources"]) - set(before["sources"]))
    kept = set(before["sources"]) & set(after["sources"])
    checks["other_sources_byte_equal"] = all(before["sources"][k] == after["sources"][k] for k in kept)
    removed_ok = []
    for key in removed:
        suffix = next((path for path in MOVED if key.endswith(path)), None)
        hand = (FIXTURES / MOVED[suffix][1]).read_text() if suffix else None
        removed_ok.append(suffix is not None and before["sources"][key] == hand)
    checks["removed_sources_are_the_hand_written_modules"] = (
        len(removed) == 2 and all(removed_ok))
    checks["added_sources_are_the_two_references"] = added == sorted(v[0] for v in MOVED.values())
    modules_before = before["contract_inclusion"]["modules"]
    modules_after = after["contract_inclusion"]["modules"]
    checks["inclusion_keeps_before_records"] = all(
        modules_after.get(k) == v for k, v in modules_before.items())
    new = sorted(set(modules_after) - set(modules_before))
    chain_counts = {k: len(modules_after[k]["inherited"]) for k in new}
    one_link = all(len(chain) == 1 for k in new for chain in modules_after[k]["inherited"].values())
    checks["inclusion_adds_the_two_modules"] = (
        new == sorted(v[0] for v in MOVED.values())
        and chain_counts == {v[0]: v[2] for v in MOVED.values()} and one_link)
    checks["contract_inclusion_trust_equal"] = (
        before["contract_inclusion"].get("trust") == after["contract_inclusion"].get("trust"))
    result = {"checks": checks, "sources_removed": removed, "sources_added": added,
              "sources_kept": len(kept), "inclusion_modules_before": sorted(modules_before),
              "inclusion_modules_added": new, "chains_added": chain_counts}
    Path(out).write_text(json.dumps(result, indent=1) + "\n")
    print(json.dumps(checks, indent=1))
    return 0 if all(checks.values()) else 1


if __name__ == "__main__":
    sys.exit(main())

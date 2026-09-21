#!/usr/bin/env python3
"""Maintain the inventories for canonical provider-owned contracts.

Edit contracts in their provider crate, then run `sync`. `check` never writes.
The retired ``contract/qcore`` compatibility tree is refused so it cannot
silently become a second source of provider contracts again.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parent
# Ownership is intentionally narrow: shared language/support policies stay in
# contract/shared. The ChunkIO provider also handles Fluent Bit's busy-state API.
OWNERS = {
    "qcore-provider-chunkio": ("chunkio", "fluent_bit/busy_state.qcore"),
    "qcore-provider-cmetrics": ("cmetrics",),
    "qcore-provider-ctraces": ("ctraces",),
    "qcore-provider-libevent": ("monkey",),
    "qcore-provider-librdkafka": ("librdkafka",),
}


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def owned(name, selectors):
    return any(name == selector or name.startswith(selector + "/") for selector in selectors)


def files(directory):
    if not directory.is_dir() or directory.is_symlink():
        raise ValueError(f"missing or symbolic-link contract directory: {directory}")
    result = {}
    for path in sorted(directory.rglob("*")):
        if path.is_symlink():
            raise ValueError(f"symbolic link in contract inventory: {path}")
        if path.is_file():
            name = path.relative_to(directory).as_posix()
            if path.suffix != ".qcore" or not re.fullmatch(r"[a-z0-9_/-]+\.qcore", name):
                raise ValueError(f"unexpected contract asset: {path}")
            result[name] = path
    if not result:
        raise ValueError(f"empty provider contract inventory: {directory}")
    return result


def maintain(root=ROOT, sync=False):
    root = Path(root)
    legacy = root.parent / "contract/qcore"
    if legacy.exists() or legacy.is_symlink():
        raise ValueError(f"legacy contract corpus must be absent: {legacy}")
    rows = []
    claimed = set()
    for crate, selectors in OWNERS.items():
        provider = root / crate
        if provider.is_symlink():
            raise ValueError(f"symbolic-link provider directory: {provider}")
        canonical = files(provider / "contracts")
        template_path = provider / "provider-template.json"
        if template_path.is_symlink() or not template_path.is_file():
            raise ValueError(f"missing or symbolic-link provider template: {template_path}")
        template = json.loads(template_path.read_text())
        for name in canonical:
            if not owned(name, selectors) or name in claimed:
                raise ValueError(f"unowned or multiply owned contract: {crate}/{name}")
            claimed.add(name)
        expected = {
            "schema_version": 1,
            "plugin_id": template["plugin_id"],
            "contracts": {"contracts/" + name: sha256(path) for name, path in canonical.items()},
        }
        inventory = provider / "contracts.json"
        if inventory.is_symlink():
            raise ValueError(f"symbolic-link contract inventory: {inventory}")
        if sync:
            inventory.write_text(json.dumps(expected, indent=2) + "\n")
        else:
            if inventory.is_symlink() or not inventory.is_file():
                raise ValueError(f"missing or symbolic-link contract inventory: {inventory}")
            if json.loads(inventory.read_text()) != expected:
                raise ValueError(f"stale provider contract inventory: {inventory}")
        rows.append({"provider": template["plugin_id"], "crate": crate,
                     "contracts": len(canonical), "inventory_sha256": sha256(inventory)})
    return rows


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=["check", "sync"])
    args = parser.parse_args()
    try:
        rows = maintain(sync=args.action == "sync")
    except (OSError, ValueError, KeyError) as error:
        parser.exit(1, f"provider contracts: {error}\n")
    print(json.dumps({"providers": rows, "total_contracts": sum(r["contracts"] for r in rows)}, indent=2))


if __name__ == "__main__":
    main()

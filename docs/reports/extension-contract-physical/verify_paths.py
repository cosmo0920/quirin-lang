#!/usr/bin/env python3
"""EXT-02 source-byte and lock-identity comparison; never repins contracts.

Run `baseline` before physical changes and `verify` afterwards. Both modes are
read-only. The input map is the parser-derived EXT-01 inventory.
"""

import argparse
import csv
import hashlib
import json
import pathlib
import re
import subprocess
import sys


ROOT = pathlib.Path(__file__).resolve().parents[3]
MAPPING = ROOT / "docs/reports/extension-compatibility/mapping.tsv"
BASELINE = ROOT / "docs/reports/extension-contract-physical/baseline.json"


def mappings():
    with MAPPING.open(newline="") as source:
        rows = list(csv.DictReader(source, delimiter="\t"))
    selected = {
        row["old_path"]: row["proposed_new_path"]
        for row in rows
        if row["archive"] == "false" and row["proposed_new_path"].endswith(".qv")
    }
    if len(selected) != 177 or len(set(selected.values())) != 177:
        raise ValueError("expected 177 unique live contract rename pairs")
    if any(not old.endswith(".qcore") or new != old[:-6] + ".qv" for old, new in selected.items()):
        raise ValueError("unexpected contract rename spelling")
    return selected, {row["old_path"]: row["index_blob"] for row in rows if row["old_path"] in selected}


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def expected_source_bytes(path):
    """Only inclusion target suffixes may change inside a renamed source."""
    before = path.read_bytes()
    after, count = re.subn(rb'(?m)^(#include "[^"\n]+)\.qcore(";)$', rb'\1.qv\2', before)
    return after, count


def index_blobs():
    result = subprocess.run(
        ["git", "ls-files", "--stage", "-z"], cwd=ROOT, capture_output=True, check=True
    )
    blobs = {}
    for record in result.stdout.split(b"\0"):
        if record:
            metadata, name = record.split(b"\t", 1)
            blobs[name.decode()] = metadata.split()[1].decode()
    return blobs


def lock_modules(mapping):
    result = {}
    for path in sorted((ROOT / "providers").glob("*/contracts.lock")):
        relative = path.relative_to(ROOT).as_posix()
        owner = path.parent.relative_to(ROOT).as_posix()
        prefix = owner + "/contracts/"
        names = {
            old.removeprefix(prefix): new.removeprefix(prefix)
            for old, new in mapping.items()
            if old.startswith(prefix)
        }
        lock = json.loads(path.read_text())
        entries = lock["pinned"] + lock["flexible"]
        result[relative] = {
            "sha256": sha256(path),
            "metadata": {k: v for k, v in lock.items() if k not in {"pinned", "flexible"}},
            "pinned": len(lock["pinned"]),
            "flexible": len(lock["flexible"]),
            "entries": [
                {
                    "function": entry["function"],
                    "old_module": entry["module"],
                    "new_module": names.get(entry["module"], entry["module"]),
                    "contract_sha256": entry["contract_sha256"],
                    "identities": entry.get("identities"),
                    "other": {k: v for k, v in entry.items() if k not in {"function", "module", "contract_sha256", "identities"}},
                }
                for entry in entries
            ],
        }
    return result


def baseline(mapping, expected_blobs):
    blobs = index_blobs()
    rows = []
    for old, new in sorted(mapping.items()):
        old_file, new_file = ROOT / old, ROOT / new
        if not old_file.is_file() or new_file.exists() or blobs.get(old) != expected_blobs[old]:
            raise ValueError(f"unexpected source state: {old} -> {new}")
        expected, includes = expected_source_bytes(old_file)
        rows.append({"old_path": old, "new_path": new, "index_blob": blobs[old],
                     "sha256": sha256(old_file), "expected_sha256": hashlib.sha256(expected).hexdigest(),
                     "include_targets_rewritten": includes})
    data = {"base": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
            "source_count": len(rows), "sources": rows, "locks": lock_modules(mapping)}
    BASELINE.parent.mkdir(parents=True, exist_ok=True)
    BASELINE.write_text(json.dumps(data, indent=2, sort_keys=True) + "\n")
    print(f"baseline: {len(rows)} sources, {len(data['locks'])} locks; {BASELINE.relative_to(ROOT)}")


def verify(mapping):
    data = json.loads(BASELINE.read_text())
    if data["source_count"] != 177:
        raise ValueError("baseline source count changed")
    for row in data["sources"]:
        old, new = row["old_path"], row["new_path"]
        if mapping.get(old) != new or (ROOT / old).exists() or not (ROOT / new).is_file():
            raise ValueError(f"rename set mismatch: {old} -> {new}")
        if sha256(ROOT / new) != row["expected_sha256"]:
            raise ValueError(f"source bytes differ beyond include targets: {new}")
    for relative, before in data["locks"].items():
        path = ROOT / relative
        lock = json.loads(path.read_text())
        if {k: v for k, v in lock.items() if k not in {"pinned", "flexible"}} != before["metadata"]:
            raise ValueError(f"lock metadata changed: {relative}")
        if len(lock["pinned"]) != before["pinned"] or len(lock["flexible"]) != before["flexible"]:
            raise ValueError(f"lock counts changed: {relative}")
        after_entries = lock["pinned"] + lock["flexible"]
        if len(after_entries) != len(before["entries"]):
            raise ValueError(f"lock entries changed: {relative}")
        for old, after in zip(before["entries"], after_entries, strict=True):
            if after["module"] != old["new_module"]:
                raise ValueError(f"lock module path mismatch: {relative}: {old['function']}")
            if after["function"] != old["function"] or after["contract_sha256"] != old["contract_sha256"]:
                raise ValueError(f"lock declaration identity changed: {relative}: {old['function']}")
            if after.get("identities") != old["identities"]:
                raise ValueError(f"reviewed source identities changed: {relative}: {old['function']}")
            other = {k: v for k, v in after.items() if k not in {"function", "module", "contract_sha256", "identities"}}
            if other != old["other"]:
                raise ValueError(f"lock entry metadata changed: {relative}: {old['function']}")
    print(f"verified: {len(data['sources'])} byte-identical renamed sources, {len(data['locks'])} locks with unchanged declaration digests and reviewed identities")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=("baseline", "verify"))
    args = parser.parse_args()
    try:
        mapping, expected_blobs = mappings()
        baseline(mapping, expected_blobs) if args.mode == "baseline" else verify(mapping)
    except (KeyError, OSError, ValueError, subprocess.CalledProcessError) as error:
        print(f"EXT-02 evidence refused: {error}", file=sys.stderr)
        sys.exit(1)

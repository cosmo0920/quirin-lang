#!/usr/bin/env python3
"""GATEWAY-RULE-API: the corpus route's identity check beyond byte equality.

The pinned corpus route under `quirin-audit measure` runs each side in its own
working directory (`sides/<name>/work-corpus`), and every unit's build
identity is the digest of the unit, the working directory and the Clang
identity (`qcore-audit` `projection.rs` `hypothesis_context`), so every digest
derived from it (reading-site evidence ids and environment digests,
`resource_contracts:<24 hex>` obligation ids) moves with the directory, and
reading-site evidence, kept sorted by id, can change order.

Usage: identity_check.py BEFORE_DIR AFTER_DIR FILE [FILE ...]

For each FILE (relative to both directories) it reports, as one JSON line:
`byte-identical`; or `equal with digests blanked` when the two parsed
documents (JSON, or JSONL line by line) are equal after every 64-hex string
and every `resource_contracts:` 24-hex id is blanked and every list is read
as a multiset; otherwise `differs`. For `resource-diagnostics.json` it also
checks that the per-function build identities map one to one between the
sides with exactly one value per unit on each side. Exit 1 on any `differs`
or a failed mapping check.
"""
import hashlib
import json
import re
import sys
from pathlib import Path

HEX64 = re.compile(r"^[0-9a-f]{64}$")
CONTRACT_ID = re.compile(r"resource_contracts:[0-9a-f]{24}")


def blank(value):
    if isinstance(value, dict):
        return {CONTRACT_ID.sub("resource_contracts:H", key): blank(item)
                for key, item in value.items()}
    if isinstance(value, list):
        return sorted((blank(item) for item in value),
                      key=lambda item: json.dumps(item, sort_keys=True))
    if isinstance(value, str):
        return "H" if HEX64.match(value) else CONTRACT_ID.sub("resource_contracts:H", value)
    return value


def parse(path):
    text = path.read_text(encoding="utf-8")
    if path.suffix == ".jsonl":
        return [json.loads(line) for line in text.splitlines() if line.strip()]
    return json.loads(text)


def build_identities(document):
    pairs = []
    for unit_index, unit in enumerate(document.get("units", [])):
        for function in unit.get("functions", []):
            context = function.get("hypothesis_context") or {}
            if "build_identity" in context:
                pairs.append((unit_index, context["build_identity"]))
    return pairs


def main(arguments):
    if len(arguments) < 3:
        raise SystemExit(__doc__)
    before_dir, after_dir = Path(arguments[0]), Path(arguments[1])
    ok = True
    for name in arguments[2:]:
        before, after = before_dir / name, after_dir / name
        raw_before, raw_after = before.read_bytes(), after.read_bytes()
        record = {"file": name,
                  "before_sha256": hashlib.sha256(raw_before).hexdigest(),
                  "after_sha256": hashlib.sha256(raw_after).hexdigest()}
        if raw_before == raw_after:
            record["result"] = "byte-identical"
        else:
            left, right = parse(before), parse(after)
            record["result"] = ("equal with digests blanked" if blank(left) == blank(right)
                                else "differs")
            if Path(name).name == "resource-diagnostics.json":
                lb, rb = build_identities(left), build_identities(right)
                forward, backward, units = {}, {}, {}
                mapped = len(lb) == len(rb) and [u for u, _ in lb] == [u for u, _ in rb]
                for (unit, b), (_, a) in zip(lb, rb):
                    mapped &= forward.setdefault(b, a) == a and backward.setdefault(a, b) == b
                    mapped &= units.setdefault(unit, b) == b
                record["build_identity"] = {
                    "rows": len(lb), "units": len(units), "distinct": len(forward),
                    "moved": sum(1 for b, a in forward.items() if a != b),
                    "one_to_one_one_per_unit": bool(mapped and len(forward) == len(units))}
                ok &= record["build_identity"]["one_to_one_one_per_unit"]
        ok &= record["result"] != "differs"
        print(json.dumps(record))
    return 0 if ok else 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))

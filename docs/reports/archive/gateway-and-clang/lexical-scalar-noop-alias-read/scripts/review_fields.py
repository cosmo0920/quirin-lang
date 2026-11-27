#!/usr/bin/env python3
"""Inventory every artifact field change after explicit checkout normalization."""
from collections import Counter
import gzip
import json
from pathlib import Path
import sys

REPORT = Path(__file__).resolve().parents[1]
ROOTS = ("/tmp/qcore-lexical-scalar-noop-alias-read", "/tmp/qcore-detach-same-item", "/tmp/qcore-fb-lifetime-s3",
         "/home/cosmo/Documents/GitHub/qcore-c")


def normalize(value):
    if isinstance(value, str):
        for root in ROOTS:
            value = value.replace(root, "$CHECKOUT")
        return value
    if isinstance(value, list):
        return [normalize(item) for item in value]
    if isinstance(value, dict):
        result = {normalize(key): normalize(item) for key, item in value.items()}
        assert len(result) == len(value), "checkout normalization collision"
        return result
    return value


def changes(a, b, path=()):
    if a == b:
        return []
    if isinstance(a, dict) and isinstance(b, dict):
        result = []
        for key in sorted(a.keys() | b.keys()):
            if key not in a or key not in b:
                result.append({"path": path + (key,), "before": a.get(key), "after": b.get(key),
                               "before_present": key in a, "after_present": key in b})
            else:
                result.extend(changes(a[key], b[key], path + (key,)))
        return result
    # Lists are retained whole: ordering and duplicate configurations are evidence.
    return [{"path": path, "before": a, "after": b}]


route = sys.argv[1]
out = REPORT / "artifacts" / route
result = {"meaning": "Comparison-only checkout normalization; raw artifacts remain retained. No field is discarded. Whole changed lists preserve order and duplicate configurations.",
          "route": route, "artifacts": {}}
for stem in ("diagnostics", "source", "chunk-contracts", "source-effects", "contracts"):
    before, after = [out / side / f"{stem}.json.gz" for side in ("before", "after")]
    assert before.exists() == after.exists(), f"{stem}: presence changed"
    if not before.exists():
        result["artifacts"][stem] = {"present": False}
        continue
    a, b = [json.loads(gzip.decompress(path.read_bytes())) for path in (before, after)]
    delta = changes(normalize(a), normalize(b))
    result["artifacts"][stem] = {"present": True, "changes": delta,
        "top_fields": dict(Counter(item["path"][0] if item["path"] else "$ROOT" for item in delta))}
(out / "normalized-field-differences.json.gz").write_bytes(
    gzip.compress((json.dumps(result, indent=2) + "\n").encode(), mtime=0))
print(json.dumps({key: value.get("top_fields", {"present": value["present"]})
                  for key, value in result["artifacts"].items()}, indent=2))

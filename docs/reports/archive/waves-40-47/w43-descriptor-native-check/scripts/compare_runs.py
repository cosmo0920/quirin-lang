#!/usr/bin/env python3
"""Compare two audit runs' five raw artifacts, raw and after a root rewrite.

Usage: compare_runs.py BEFORE_DIR AFTER_DIR BEFORE_ROOT AFTER_ROOT OUT.json

BEFORE is main's binary built in its own checkout (BEFORE_ROOT); AFTER is this
card's tip (AFTER_ROOT). Reports each artifact's raw SHA-256 pair, byte
identity, and identity after rewriting BEFORE_ROOT to AFTER_ROOT in BEFORE's
text (a checkout-path-only difference). Exit 0 only if every artifact is
identical after the rewrite.
"""
import hashlib
import json
import pathlib
import sys

NAMES = ["resource-diagnostics.json", "resource-diagnostics.chunk-contracts.json",
         "resource-diagnostics.contracts.json", "resource-diagnostics.source-effects.json", "monkey-source.json"]


def main() -> int:
    before, after, before_root, after_root, out = sys.argv[1:6]
    result = {"before": before, "after": after, "before_root": before_root, "after_root": after_root,
              "artifacts": {}}
    ok = True
    for name in NAMES:
        left = (pathlib.Path(before) / name).read_bytes()
        right = (pathlib.Path(after) / name).read_bytes()
        rewritten = left.replace(before_root.encode(), after_root.encode())
        entry = {"before_sha256": hashlib.sha256(left).hexdigest(),
                 "after_sha256": hashlib.sha256(right).hexdigest(),
                 "byte_identical": left == right,
                 "identical_after_root_rewrite": rewritten == right}
        ok &= entry["identical_after_root_rewrite"]
        result["artifacts"][name] = entry
    result["all_identical_after_root_rewrite"] = ok
    pathlib.Path(out).write_text(json.dumps(result, indent=1) + "\n")
    print(json.dumps(result, indent=1))
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())

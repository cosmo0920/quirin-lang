#!/usr/bin/env python3
"""CG00-CLANG-GAP-BASELINE: compare one arbiter run with the archived baseline.

usage: identity.py RUN_DIR BASELINE_DIR

BASELINE_DIR holds a file-keyed `manifest.json` (`{name: {file, sha256}}`, the
form `docs/reports/w47-flb-lib-lifecycle/integration/artifacts/corpus/` uses)
and the gzipped artifacts. For each artifact the run's SHA-256 is compared with
the manifest's; the two sidecars that record the checkout path
(`contracts`, `source-effects`) are compared again after rewriting this
checkout's root to the main checkout's, as a disclosure only. Prints JSON and
exits 0 only when the verdict-bearing `resource-diagnostics.json` is
byte-identical. The function tallies are printed beside it.
"""

import collections
import gzip
import hashlib
import json
import pathlib
import re
import sys

NAMES = [
    "resource-diagnostics.json",
    "resource-diagnostics.chunk-contracts.json",
    "resource-diagnostics.contracts.json",
    "resource-diagnostics.source-effects.json",
    "monkey-source.json",
]


def main() -> int:
    run, baseline = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2])
    manifest = json.loads((baseline / "manifest.json").read_text())
    result = {"run": str(run), "baseline": str(baseline), "artifacts": {}}
    for name in NAMES:
        data = (run / name).read_bytes()
        entry = {"run_sha256": hashlib.sha256(data).hexdigest(),
                 "baseline_sha256": manifest[name]["sha256"]}
        entry["byte_identical"] = entry["run_sha256"] == entry["baseline_sha256"]
        if not entry["byte_identical"]:
            archived = gzip.decompress((baseline / manifest[name]["file"]).read_bytes())
            # The baseline records the checkout it ran in; rewrite both roots
            # to one spelling. A disclosure beside the raw comparison only.
            pattern = re.compile(rb"/home/cosmo/Documents/GitHub/qcore-c(/\.claude/worktrees/[^/\"]+)?")
            canonical = b"<checkout>"
            entry["identical_after_checkout_alias"] = (
                pattern.sub(canonical, data) == pattern.sub(canonical, archived))
        result["artifacts"][name] = entry
    report = json.loads((run / "resource-diagnostics.json").read_text())
    status = collections.Counter()
    categories = collections.Counter()
    for unit in report["units"]:
        for function in unit["functions"]:
            status[function["status"]] += 1
            for leaf in function["diagnostics"]:
                categories[leaf["category"]] += 1
    result["functions"] = dict(sorted(status.items()))
    result["diagnostic_counts"] = dict(sorted(categories.items()))
    result["verdict_bearing_identical"] = result["artifacts"]["resource-diagnostics.json"]["byte_identical"]
    print(json.dumps(result, indent=1))
    return 0 if result["verdict_bearing_identical"] else 1


if __name__ == "__main__":
    sys.exit(main())

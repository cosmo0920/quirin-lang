#!/usr/bin/env python3
"""Whether a run's source-effects sidecar differs from the archived baseline
only in digest-valued tokens.

Usage: source_effects_digests.py RUN_DIR BASELINE_ARTIFACT_DIR ROOT=CANONICAL

The source-effects sidecar carries build and configuration digests that bind
the checkout path, so a run from another worktree cannot reproduce them byte
for byte (the same disclosure every W45 card's identity record makes). This
rewrites the run's checkout root to the baseline's, masks every 64-hex digest
and every `resource_contracts:` obligation id in both texts, and reports
whether the masked texts are equal and how many distinct digests differ. It is
a disclosure beside `baseline_identity.py`, never a substitute for it.
"""

import gzip
import json
import pathlib
import re
import sys

DIGEST = re.compile(r"\b[0-9a-f]{64}\b|resource_contracts:[0-9a-f]{24}")


def main() -> int:
    run = pathlib.Path(sys.argv[1]) / "resource-diagnostics.source-effects.json"
    baseline_dir = pathlib.Path(sys.argv[2])
    root, canonical = sys.argv[3].split("=", 1)
    manifest = json.loads((baseline_dir / "manifest.json").read_text())
    baseline = gzip.decompress(
        (baseline_dir / manifest["source-effects"]["file"]).read_bytes()
    ).decode()
    text = run.read_text().replace(root, canonical)
    result = {
        "byte_identical_after_root_alias": text == baseline,
        "identical_after_masking_digests": DIGEST.sub("H", text) == DIGEST.sub("H", baseline),
        "digest_tokens": [len(DIGEST.findall(text)), len(DIGEST.findall(baseline))],
        "distinct_digests_only_in_run": len(set(DIGEST.findall(text)) - set(DIGEST.findall(baseline))),
        "distinct_digests_only_in_baseline": len(
            set(DIGEST.findall(baseline)) - set(DIGEST.findall(text))
        ),
    }
    print(json.dumps(result, indent=2))
    return 0 if result["identical_after_masking_digests"] else 1


if __name__ == "__main__":
    sys.exit(main())

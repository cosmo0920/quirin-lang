#!/usr/bin/env python3
"""Card C-GATEWAY-INDIRECT-CENSUS-2's inertness assertion.

The card changes no analyzer rule, so a corpus audit run by the **instrumented**
binary, with the census actively streaming, must reproduce the current baseline
`docs/reports/c-gateway-constant-do-while/artifacts/` byte for byte in the two
verdict-bearing artifacts. That is a stronger statement than "the feature is off
in production": it says the instrumentation perturbs no decision even when on.

The other three artifacts carry the checkout root and identity digests and have
differed at every re-pin in this chain for that reason, which this tool states
rather than hides.

Usage: python3 byte_identity.py <run-dir> [archive-dir]
"""
import gzip
import hashlib
import json
import pathlib
import sys

ARTIFACTS = {
    "diagnostics": "resource-diagnostics.json",
    "source": "monkey-source.json",
    "chunk-contracts": "resource-diagnostics.chunk-contracts.json",
    "source-effects": "resource-diagnostics.source-effects.json",
    "contracts": "resource-diagnostics.contracts.json",
}
VERDICT_BEARING = ("diagnostics", "chunk-contracts")


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def canonical(raw):
    return hashlib.sha256(
        json.dumps(json.loads(raw), sort_keys=True, separators=(",", ":")).encode()
    ).hexdigest()[:16]


def main():
    run = pathlib.Path(sys.argv[1])
    archive = pathlib.Path(
        sys.argv[2] if len(sys.argv) > 2
        else "docs/reports/c-gateway-constant-do-while/artifacts"
    )
    print("archive: %s" % archive)
    print("run:     %s" % run)
    print()
    identical = True
    for stem, produced in ARTIFACTS.items():
        a = gzip.decompress((archive / ("%s.json.gz" % stem)).read_bytes())
        m = (run / produced).read_bytes()
        raw = "IDENTICAL" if a == m else "differs  "
        can = "IDENTICAL" if canonical(a) == canonical(m) else "differs  "
        mark = "*" if stem in VERDICT_BEARING else " "
        print("%s%-16s raw %s  canonical %s" % (mark, stem, raw, can))
        print("   archive sha256 %s" % digest(a))
        print("   run     sha256 %s" % digest(m))
        if stem in VERDICT_BEARING and a != m:
            identical = False
    print()
    print("* = verdict-bearing artifact")
    if identical:
        print("RESULT: with the census feature ON and the census actively")
        print("streaming, both verdict-bearing artifacts reproduce the wave-35")
        print("baseline BYTE FOR BYTE. The instrumentation moves no verdict.")
        return 0
    print("RESULT: a verdict-bearing artifact DIFFERS. The instrumentation is")
    print("not inert; stop and report this rather than publishing the census.")
    return 1


if __name__ == "__main__":
    sys.exit(main())

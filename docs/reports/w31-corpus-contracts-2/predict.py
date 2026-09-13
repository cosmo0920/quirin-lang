#!/usr/bin/env python3
"""Re-derive card W31-CORPUS-CONTRACTS-2's prediction against the NEW baseline.

The card was interrupted; while it was down the user landed `C-GATEWAY-WAVE`,
which moved the corpus baseline to `docs/reports/c-gateway-wave/artifacts/`.
The predecessor's prediction was computed against
`docs/reports/w4-recursive-summaries/artifacts/`. This script recomputes the
same three inventories over whichever archive is named, so the two predictions
can be compared as a finding.

Read-only. Usage:
    python3 -B measure/predict.py docs/reports/c-gateway-wave/artifacts
"""
from __future__ import annotations

import collections
import gzip
import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
ARCHIVE = pathlib.Path(sys.argv[1]) if len(sys.argv) > 1 else ROOT / "docs/reports/c-gateway-wave/artifacts"

CALLEES = ("cio_chunk_get_content_size", "flb_input_chunk_total_size")
SIBLINGS = (
    "flb_input_chunk_is_up",
    "flb_input_chunk_set_up",
    "input_chunk_metadata_view",
    "cio_chunk_get_real_size",
    "cio_meta_size",
    "cio_stream_size_chunks_up",
)

path = ARCHIVE / "diagnostics.json.gz"
data = json.loads(gzip.decompress(path.read_bytes()))
print(f"archive: {path}")

units = data["units"]
print(f"units: {len(units)}")
histogram = collections.Counter(u.get("status") for u in units)
print(f"unit status histogram: {dict(sorted(histogram.items()))}")
functions = [(u, f) for u in units for f in u.get("functions", [])]
fhist = collections.Counter(f.get("status") for _, f in functions)
print(f"projected functions: {len(functions)}  {dict(sorted(fhist.items()))}")
counts: collections.Counter = collections.Counter()
for u in units:
    counts.update(u.get("diagnostic_counts", {}))
print("corpus diagnostic counts:")
for category in sorted(counts):
    print(f"  {category}: {counts[category]}")

print()
print("=== leaves whose description names a declared callee ===")
total = 0
for u in units:
    for f in u.get("functions", []):
        for d in f.get("diagnostics", []):
            text = d.get("description") or ""
            if any(name in text for name in CALLEES):
                total += 1
                print(
                    f"  {u.get('library')}/{u.get('file')} :: {f.get('function')} "
                    f"L{d.get('line')} origin={d.get('origin')} [{d.get('category')}] {text}"
                )
print(f"  TOTAL: {total}")

print()
print("=== every leaf inside the functions that hold those leaves ===")
holders = set()
for u in units:
    for f in u.get("functions", []):
        if any(
            any(name in (d.get("description") or "") for name in CALLEES)
            for d in f.get("diagnostics", [])
        ):
            holders.add((u.get("library"), u.get("file"), f.get("function")))
for lib, file, fn in sorted(holders):
    for u in units:
        if u.get("library") != lib or u.get("file") != file:
            continue
        for f in u.get("functions", []):
            if f.get("function") != fn:
                continue
            print(f"  --- {lib}/{file} :: {fn}  status={f.get('status')}")
            for d in f.get("diagnostics", []):
                print(
                    f"      L{d.get('line')} origin={d.get('origin')} "
                    f"[{d.get('category')}] {d.get('description')}"
                )

print()
print("=== sibling-callee leaves (for the upheld non-declarations) ===")
sib: collections.Counter = collections.Counter()
for u in units:
    for f in u.get("functions", []):
        for d in f.get("diagnostics", []):
            text = d.get("description") or ""
            for name in SIBLINGS:
                if name in text:
                    sib[name] += 1
                    print(
                        f"  {name}: {u.get('library')}/{u.get('file')} :: {f.get('function')} "
                        f"L{d.get('line')} origin={d.get('origin')} [{d.get('category')}] {text}"
                    )
print(f"  sibling totals: {dict(sorted(sib.items()))}")

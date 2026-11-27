#!/usr/bin/env python3
"""Split a timestamped plain `cargo test` log (see ts.py) into per-binary records.

Usage:
  cargo_log.py <output.log>                 # per-binary durations, longest first
  cargo_log.py <output.log> --json OUT      # records for compare.py

A record is keyed by the executable's stem (the target name, hash stripped) or
`doc-tests` for every `Doc-tests <crate>` section together. Duration is the gap
from a binary's `Running` line to the next `Running`/`Doc-tests` line (or the
last line), which is how the per-binary profile of the brief was measured.
"""

import argparse
import json
from pathlib import Path
import re
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[4] / "scripts"))
import run_test_binaries as runner  # noqa: E402  (shared libtest parsers)

LINE_RE = re.compile(r"^\s*(?P<t>\d+\.\d+) (?P<text>.*)$")
RUNNING_RE = re.compile(r"^\s+Running (?P<src>.+) \((?P<exe>[^)]+)\)$")
DOC_RE = re.compile(r"^\s+Doc-tests (?P<crate>\S+)$")


def stem(exe: str) -> str:
    name = Path(exe).name
    return re.sub(r"-[0-9a-f]{16}$", "", name)


def split(path: Path):
    records = []
    current = None
    first_running = None
    last_t = 0.0
    for raw in path.read_text(encoding="utf-8", errors="replace").splitlines():
        match = LINE_RE.match(raw)
        if not match:
            continue
        t = float(match["t"])
        text = match["text"]
        last_t = t
        running = RUNNING_RE.match(text)
        doc = DOC_RE.match(text)
        if running or doc:
            if current is not None:
                current["end"] = t
            if first_running is None:
                first_running = t
            if running:
                current = {"key": stem(running["exe"]), "src": running["src"],
                           "start": t, "lines": []}
            else:
                current = {"key": "doc-tests", "src": doc["crate"], "start": t, "lines": []}
            records.append(current)
            continue
        if current is not None:
            current["lines"].append(text)
    if current is not None:
        current["end"] = last_t
    return records, first_running, last_t


def summarize(records):
    merged = {}
    for record in records:
        summaries, tests = runner.parse_output("\n".join(record["lines"]))
        entry = merged.setdefault(record["key"], {"seconds": 0.0, "summaries": [], "tests": [],
                                                  "sources": []})
        entry["seconds"] += record["end"] - record["start"]
        entry["summaries"] += summaries
        entry["tests"] += tests
        entry["sources"].append(record["src"])
    for entry in merged.values():
        entry["tests"].sort()
        entry["seconds"] = round(entry["seconds"], 3)
    return merged


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("log", type=Path)
    parser.add_argument("--json", type=Path)
    parser.add_argument("--top", type=int, default=25)
    args = parser.parse_args()
    records, first_running, last_t = split(args.log)
    merged = summarize(records)
    total = sum(entry["seconds"] for entry in merged.values())
    print(f"build (to first Running): {first_running:.1f}s; run: {last_t - first_running:.1f}s; "
          f"binaries+doc sections: {len(records)}; summed: {total:.1f}s")
    for key, entry in sorted(merged.items(), key=lambda kv: -kv[1]["seconds"])[: args.top]:
        count = sum(int(s["passed"]) + int(s["failed"]) + int(s["ignored"])
                    for s in entry["summaries"])
        print(f"{entry['seconds']:9.1f}s {count:5d} tests  {key}")
    if args.json:
        args.json.write_text(json.dumps({"build_seconds": first_running,
                                         "run_seconds": last_t - first_running,
                                         "binaries": merged}, indent=1) + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())

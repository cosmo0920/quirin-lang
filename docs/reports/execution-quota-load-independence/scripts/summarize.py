#!/usr/bin/env python3
"""Summarize loadbisect.py JSON files as a Markdown table.

Usage: summarize.py LABEL=FILE.json [LABEL=FILE.json ...]

One row per load level: the level's configuration, runs, failing runs, the
1-minute load-average range and the UID task-count range seen at the end of the
runs, and every failure classified by its signature.
"""

from __future__ import annotations

import collections
import json
import sys


def signature(panic: str) -> str:
    if "Text file busy" in panic:
        return "ETXTBSY"
    if "Broken pipe" in panic:
        return "EPIPE"
    if "File exists" in panic:
        return "EEXIST (quota None)"
    if "peak=Some(1)" in panic:
        return "descendant never started (peak 1; base test ignores the run's error)"
    if "None" in panic or "no observation" in panic:
        return "quota None (other)"
    return "other: " + panic[-120:]


def main() -> int:
    print("| set | sleepers | spinners | forkers | peers | netns loops | host stall ms | child stall ms | amplifier | runs | failed runs | load1 | UID tasks | failures |")
    print("|---|---:|---:|---:|---:|---:|---:|---:|---|---:|---:|---|---|---|")
    for arg in sys.argv[1:]:
        label, path = arg.split("=", 1)
        data = json.load(open(path))
        amp = ", ".join(
            item.removeprefix("QCORE_AMPLIFY_")
            for item in data.get("env", [])
            if item.startswith("QCORE_AMPLIFY_")
        ) or "—"
        for level in data["levels"]:
            runs = level["runs"]
            loads = [run["load1"] for run in runs]
            tasks = [run["uid_tasks"] for run in runs]
            counts = collections.Counter(
                f"{failure['test']}: {signature(failure['panic'])}"
                for run in runs
                for failure in run["failures"]
            )
            failures = "; ".join(f"{key} ×{value}" for key, value in sorted(counts.items())) or "—"
            print(
                f"| {label} | {level['sleepers']} | {level['spinners']} | {level['forkers']} "
                f"| {level.get('peers', 0)} | {level.get('netns', 0)} | {level.get('stall_ms', 0)} | {level.get('child_stall_ms', 0)} | {amp} "
                f"| {len(runs)} | {level.get('failed_runs', sum(1 for r in runs if r['exit']))} "
                f"| {min(loads):.1f}–{max(loads):.1f} | {min(tasks)}–{max(tasks)} | {failures} |"
            )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

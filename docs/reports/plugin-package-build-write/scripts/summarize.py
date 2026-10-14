#!/usr/bin/env python3
"""Summarize private/ppb/stress/*.json (card
PLUGIN-PACKAGE-BUILD-WRITES-EXECUTABLE-IN-PROCESS) as a Markdown table:
per campaign, the binary digest, runs, deviating runs, runs whose log names
ETXTBSY, child stalls and wall time."""

import json
import sys
from pathlib import Path

root = Path(sys.argv[1] if len(sys.argv) > 1 else "private/ppb/stress")
print("| Campaign | Binary sha256 | Runs | Deviating | With ETXTBSY | Child stalls (total) | Wall s (max) |")
print("|---|---|---|---|---|---|---|")
for path in sorted(root.glob("*-rel-close*.json")):
    data = json.loads(path.read_text())
    runs = [run for level in data["levels"] for run in level["runs"]]
    deviating = [run for run in runs if run["exit"] != 0]
    busy = [
        run for run in runs
        if "Text file busy (os error 26)" in Path(run["log"]).read_text(errors="replace")
    ]
    print(
        f"| `{path.stem}` | `{data['binary_sha256'][:12]}…` | {len(runs)} | **{len(deviating)}** | "
        f"{len(busy)} | {sum(run['child_stalls'] for run in runs)} | "
        f"{max(run['wall_s'] for run in runs)} |"
    )

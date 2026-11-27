#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Card PRIORITY0-ATTRIBUTION: drive `quirin-audit measure chain` (built from
QUIRIN-AUDIT-CONNECT abc7ce7ad into W/private/chain-target) over the priority0
measurement's side directories, as run for the committed chain record.

    python3 run_chain.py OUT [--dry]

Steps: `--runs sides/prev sides/common` and `--runs sides/common sides/tip`.
`--declared N=` every PREDICTION.md added after 556460e31 (N = 1 when the
card's prediction is an ancestor of 2c07390b6, else 2; card_intervals.py
writes that list to W/private/cards-interval.txt). `--attribute N=` every
native-comparison record added after 556460e31, except the 556460e31 landing
record (before prev) and two tool demonstrations. Paths are this card's
worktree's; the measurement directory is only read.
"""
import json, os, re, subprocess, sys
from pathlib import Path

W = Path("/media/Data/GitHub/quirin-lang/.claude/worktrees/agent-a14d1f75495df89fd")
M = Path("/media/Data/GitHub/quirin-lang/.claude/worktrees/priority0/private/measure/priority0-20261125T173228Z")
TOOL = W / "private/chain-target/release"
REPORTS = W / "docs/reports"

out = Path(sys.argv[1]).resolve()
dry = "--dry" in sys.argv

cards = [l.split() for l in open(W / "private/cards-interval.txt")]
step_of = {"P-C": 1, "C-T": 2}

# Records placed by their own measured revisions (records.py): SP1's record
# measures b7f86a0de->2c07390b6, inside prev->common although committed later.
record_step = {
    "sp1-symbolic-openssl-premises/integration/native-comparison.json.gz": 1,
}
skip_records = {
    # the landing record of 7b726587b->556460e31: before the prev side.
    "landing-556460e31/native-comparison.json.gz",
    # tool demonstrations of side wiring (three comparisons each, no card move)
    "measure-contract-only-side-wiring/evidence/native-comparison-tip-a.json",
    "measure-contract-only-side-wiring/evidence/native-comparison-tip-b.json",
}

declared, attribute, noblock = [], [], []
for iv, path, _ in cards:
    step = step_of[iv]
    full = REPORTS / path
    if path.endswith("PREDICTION.md"):
        text = full.read_text()
        if "```measure-prediction" not in text:
            noblock.append(path)
            continue
        blocks = re.findall(r"```measure-prediction\n.*?```", text, re.S)
        if len(blocks) > 1:
            # The tool reads exactly one block per file. Split into one file
            # per block; MEMBER-EFFECT-STORES' second block was written after
            # its landing measurement ("for the record only"), so only its
            # first block is a prediction.
            card = path.split("/")[0]
            use = blocks[:1] if card == "member-effect-stores" else blocks
            split = W / "private/chain-predictions"
            split.mkdir(parents=True, exist_ok=True)
            for i, block in enumerate(use, 1):
                part = split / f"{card}.block{i}.md"
                part.write_text(f"Block {i} of docs/reports/{path}, verbatim.\n\n{block}\n")
                declared += ["--declared", f"{step}={part}"]
            continue
        declared += ["--declared", f"{step}={full}"]
    else:
        if path in skip_records:
            continue
        step = record_step.get(path, step)
        attribute += ["--attribute", f"{step}={full}"]

args = ["--runs", str(M / "sides/prev"), str(M / "sides/common"),
        "--runs", str(M / "sides/common"), str(M / "sides/tip")]
args += declared + attribute
print("declared", len(declared) // 2, "attribute", len(attribute) // 2)
print("no measure-prediction block:", noblock)
if dry:
    sys.exit(0)
out.mkdir(parents=True, exist_ok=True)
cmd = [str(TOOL / "quirin-audit"), "--plugin-path", str(TOOL), "--output", str(out),
       "measure", "chain", *args]
(out / "argv.json").write_text(json.dumps(cmd, indent=1) + "\n")
with open(out / "stdout.log", "wb") as so, open(out / "stderr.log", "wb") as se:
    code = subprocess.run(cmd, cwd=W, stdout=so, stderr=se).returncode
(out / "exit").write_text(f"{code}\n")
print("exit", code)

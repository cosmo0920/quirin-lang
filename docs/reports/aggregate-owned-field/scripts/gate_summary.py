#!/usr/bin/env python3
"""Summarize a detached gate run's directory: every step's exit file, the
workspace test totals summed over every `test result` line, the target counts,
the Lean and Python tails, and the Clang banner. Usage: gate_summary.py DIR."""
import pathlib
import re
import sys

d = pathlib.Path(sys.argv[1])
for step in ["submodule", "cargo-test", "fmt", "clippy", "probe", "followup", "lean", "python", "clang-version"]:
    f = d / f"{step}.exit"
    print(f"{step}: {f.read_text().strip() if f.exists() else 'MISSING'}")
for name in ["start.txt", "cargo-test-end.txt", "end.txt"]:
    print(name, (d / name).read_text().strip())
log = (d / "cargo-test.log").read_text(errors="replace")
passed = failed = ignored = summaries = 0
for m in re.finditer(r"^test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored", log, re.M):
    summaries += 1
    passed += int(m.group(2))
    failed += int(m.group(3))
    ignored += int(m.group(4))
print(f"workspace: {passed} passed / {failed} failed / {ignored} ignored over {summaries} result summaries")
print("Running targets:", len(re.findall(r"^\s+Running ", log, re.M)))
print("Doc-test sections:", len(re.findall(r"^\s+Doc-tests ", log, re.M)))
for line in re.findall(r"^test result: FAILED.*$", log, re.M):
    print("FAILED SUMMARY:", line)
for line in re.findall(r"^test .* FAILED$", log, re.M)[:20]:
    print("FAILED TEST:", line)
print("lean tail:", (d / "lean.log").read_text(errors="replace").strip().splitlines()[-2:])
print("python tail:", (d / "python.log").read_text(errors="replace").strip().splitlines()[-4:])
print("clang:", (d / "clang-version.txt").read_text(errors="replace").splitlines()[0])
print("df end:", (d / "df-end.txt").read_text().strip().splitlines()[-1])

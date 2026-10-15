#!/usr/bin/env python3
"""targeted_summary.py NAME -- LIBRDKAFKA-PLUGIN-PREMISE-REMAINDER: the exits
and test counts of targeted.sh NAME (private/lppr/targeted-NAME), printed for
the report."""
import pathlib
import re
import sys

HERE = pathlib.Path(__file__).resolve().parent
out = HERE.parents[3] / "private/lppr" / f"targeted-{sys.argv[1]}"
print("head", (out / "head").read_text().strip())
for path in sorted(out.glob("*.exit")):
    line = f"{path.stem} {path.read_text().strip()}"
    log = path.with_suffix(".log")
    text = log.read_text(errors="replace") if log.exists() else ""
    runs = re.findall(r"test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored", text)
    if runs:
        line += " passed {} failed {} ignored {}".format(*(sum(int(r[i]) for r in runs) for i in (1, 2, 3)))
    lean = re.search(r"Passed (\d+) Lean audits", text)
    if lean:
        line += f" lean {lean.group(1)}"
    py = re.search(r"Ran (\d+) tests", text)
    if py:
        line += f" python {py.group(1)} {'OK' if re.search(r'^OK', text, re.M) else 'NOT OK'}"
    if "skipping Clang-dependent" in text:
        line += " (Clang-dependent controls skipped!)"
    print(line)

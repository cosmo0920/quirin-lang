#!/usr/bin/env python3
"""TSAN-ROWS-2: derive docs/reports/tsan-rows-2/gate.json from the detached gate's files.

Every figure below is read from a file gate.sh wrote — an exit file per step,
the cargo log's `test result:` summaries, the Python suite's `Ran N tests`, the
Lean script's `Passed N Lean audits` (checked against proof/Audit*.lean on
disk, which is the generator's own check), and clang-version.txt's first line.
Only the generator's keys are emitted, in the wave-38 close's shape; the
provider figure is CARRIED, and the script refuses to write if providers/ or
qcore-schemas changed since the base (then it must be re-measured).

usage: gatejson.py <gate-dir> <base sha> <note text file>
"""
import json
import pathlib
import re
import subprocess
import sys

gate_dir = pathlib.Path(sys.argv[1])
base = sys.argv[2]
note = pathlib.Path(sys.argv[3]).read_text(encoding="utf-8").strip()


def exit_of(step):
    return int((gate_dir / f"{step}.exit").read_text().strip())


exits = {step: exit_of(step) for step in ["cargo-test", "fmt", "clippy", "probe", "followup", "lean", "python", "clang-version"]}
# A step re-run after a recorded repair publishes its post-fix exit beside the
# detached one (clippy-post: the type-alias fix; python-post: the re-run after
# the --gate regeneration the tripwire demands). Both exits stay in the note.
for step in ["clippy", "python", "fmt"]:
    post = gate_dir / f"{step}-post.exit"
    if post.is_file():
        exits[f"{step}-post"] = int(post.read_text().strip())
python_step = "python-post" if "python-post" in exits else "python"
clippy_step = "clippy-post" if "clippy-post" in exits else "clippy"
fmt_step = "fmt-post" if "fmt-post" in exits else "fmt"

log = (gate_dir / "cargo-test.log").read_text(encoding="utf-8", errors="replace")
passed = failed = ignored = summaries = 0
for m in re.finditer(r"^test result: \w+\. (\d+) passed; (\d+) failed; (\d+) ignored", log, re.M):
    summaries += 1
    passed += int(m.group(1))
    failed += int(m.group(2))
    ignored += int(m.group(3))
failed_lines = re.findall(r"^test result: FAILED", log, re.M)

python_log = (gate_dir / f"{python_step}.log").read_text(encoding="utf-8", errors="replace")
ran = re.findall(r"^Ran (\d+) tests?", python_log, re.M)
python_tests = int(ran[-1]) if ran else None

lean_log = (gate_dir / "lean.log").read_text(encoding="utf-8", errors="replace")
lean_match = re.findall(r"^Passed (\d+) Lean audits", lean_log, re.M)
lean_audits = int(lean_match[-1]) if lean_match else None
on_disk = len(list(pathlib.Path("proof").glob("Audit*.lean")))
if exits["lean"] != 0 or lean_audits != on_disk:
    sys.exit(f"lean: exit {exits['lean']}, log says {lean_audits}, disk has {on_disk}; refusing to write")

clang_first = (gate_dir / "clang-version.txt").read_text(encoding="utf-8", errors="replace").splitlines()[0].strip()
clang = f"present: {clang_first}" if exits["clang-version"] == 0 else f"absent: {clang_first}"

changed = subprocess.run(
    ["git", "diff", "--name-only", base, "HEAD", "--", "providers/", "crates/qcore-schemas/"],
    capture_output=True, text=True, check=True,
).stdout.strip()
if changed:
    sys.exit(f"providers/ or qcore-schemas changed since {base}; the carried provider figure must be re-measured:\n{changed}")

standing = "pass" if all(
    exits[s] == 0 for s in ["cargo-test", fmt_step, clippy_step, "probe", "followup", "lean", python_step]
) and failed == 0 else "FAIL"

gate = {
    "note": note.replace("{PASSED}", str(passed)).replace("{FAILED}", str(failed)).replace("{IGNORED}", str(ignored))
    .replace("{SUMMARIES}", str(summaries)).replace("{FAILED_SUMMARIES}", str(len(failed_lines)))
    .replace("{PYTHON}", str(python_tests)).replace("{LEAN}", str(lean_audits))
    .replace("{START}", (gate_dir / "start.txt").read_text().strip())
    .replace("{END}", (gate_dir / "end.txt").read_text().strip())
    .replace("{HEAD}", (gate_dir / "head.txt").read_text().strip()[:8])
    .replace("{EXITS}", json.dumps(exits, separators=(", ", "=")).replace('"', "")),
    "workspace_tests": passed,
    "provider_tests": 122,
    "python_evidence_validator_tests": python_tests,
    "reevaluation_probe_exit": exits["probe"],
    "reevaluation_followup_exit": exits["followup"],
    "cargo_fmt": "clean" if exits[fmt_step] == 0 else "FAIL",
    "cargo_clippy": "clean" if exits[clippy_step] == 0 else "FAIL",
    "standing_gate": standing,
    "lean_audits": lean_audits,
    "clang": clang,
}
out = pathlib.Path("docs/reports/tsan-rows-2/gate.json")
out.write_text(json.dumps(gate, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
print(json.dumps({k: v for k, v in gate.items() if k != "note"}, indent=1))
print("exits", exits, "summaries", summaries, "failed summaries", len(failed_lines), "ignored", ignored)

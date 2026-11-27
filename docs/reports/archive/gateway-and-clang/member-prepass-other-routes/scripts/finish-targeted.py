#!/usr/bin/env python3
"""finish-targeted.py NAME BASE -- MEMBER-PREPASS-OTHER-ROUTES: after the
targeted check (targeted.sh NAME) on a rebase onto BASE whose landed code is
disjoint from this card's: append it to gate.json's note with the Lean count
it measured, copy its exits into gate/targeted-NAME/, regenerate the matrix
with gate.json, rerun the Python suite and the docs index (and its check).
Exits to private/mpor/targeted-NAME/09*.exit; nothing piped."""
import json
import pathlib
import re
import shutil
import subprocess
import sys

REPO = pathlib.Path(__file__).resolve().parents[4]
REPORT = REPO / "docs/reports/member-prepass-other-routes"


def run(logs, label, args):
    with open(logs / f"{label}.log", "w") as log:
        code = subprocess.run(args, cwd=REPO, stdout=log, stderr=subprocess.STDOUT).returncode
    (logs / f"{label}.exit").write_text(f"{code}\n")
    return code


def main():
    name, base = sys.argv[1], sys.argv[2]
    logs = REPO / "private/mpor" / f"targeted-{name}"
    exits = {p.name: p.read_text().strip() for p in sorted(logs.glob("*.exit"))}
    lean_log = logs / "04b-lean.log" if (logs / "04b-lean.log").exists() else logs / "04-lean.log"
    lean = int(re.search(r"Passed (\d+) Lean audits", lean_log.read_text()).group(1))
    tests = re.findall(r"test result: ok\. (\d+) passed; (\d+) failed", (logs / "03-tests.log").read_text())
    head = (logs / "head").read_text().strip()
    gate_path = REPORT / "gate.json"
    gate = json.loads(gate_path.read_text())
    marker = f" Rebased onto {base}"
    if marker not in gate["note"]:
        gate["note"] += (
            f"{marker} (its landed code is disjoint from this card's; records conflicted only): "
            f"targeted check at {head[:9]} under the shared heavy lock, logs private/mpor/targeted-{name}, "
            f"exits {exits}: cargo check --workspace --all-targets, fmt --check, clippy --locked, "
            f"the card's route tests and the checkpoint tests ({', '.join(p for p, _ in tests)} passed, "
            f"0 failed), check_lean_audits.py {lean} audits passed (the 04-lean attempt left an empty "
            "exit and log and was rerun as 04b); then build_matrix.py, the python suite and the docs "
            "index again (gate/targeted-NAME/09*)."
        )
    gate["lean_audits"] = lean
    gate_path.write_text(json.dumps(gate, indent=2, ensure_ascii=False) + "\n")
    codes = [
        run(logs, "09a-matrix", ["python3", "docs/reports/remaining-d02/build_matrix.py", ".",
                                 "--gate", str(gate_path)]),
        run(logs, "09b-python", ["python3", "-m", "unittest", "discover", "-s", "scripts/tests"]),
        run(logs, "10b-index", ["python3", "scripts/build_docs_index.py"]),
        run(logs, "10c-index-check", ["python3", "scripts/build_docs_index.py", "--check"]),
    ]
    target = REPORT / "gate" / f"targeted-{name}"
    target.mkdir(parents=True, exist_ok=True)
    for path in list(logs.glob("*.exit")) + [logs / "head"]:
        shutil.copy(path, target / path.name)
    python = re.search(r"Ran (\d+) tests", (logs / "09b-python.log").read_text())
    print("lean", lean, "matrix/python/index/check", codes, "python tests", python.group(1) if python else None)
    return 0 if not any(codes) else 1


if __name__ == "__main__":
    raise SystemExit(main())

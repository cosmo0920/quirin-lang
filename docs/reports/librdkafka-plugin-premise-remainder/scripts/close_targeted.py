#!/usr/bin/env python3
"""close_targeted.py NAME -- LIBRDKAFKA-PLUGIN-PREMISE-REMAINDER: after
targeted.sh NAME, record its Lean audit count in gate.json (build_matrix.py
refuses a gate figure that disagrees with the tree) with a note naming the
targeted check, copy its exits into gate/targeted-NAME/, then rerun the
matrix regeneration, the Python suite and the docs index (09a2, 09b2, 10b2,
10c2). Exits to files; nothing piped."""
import json
import pathlib
import re
import shutil
import subprocess
import sys

HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parents[3]
REPORT = HERE.parent
name = sys.argv[1]
logs = REPO / "private/lppr" / f"targeted-{name}"
head = (logs / "head").read_text().strip()


def tests(label):
    text = (logs / f"{label}.log").read_text(errors="replace")
    runs = re.findall(r"test result: \w+\. (\d+) passed; (\d+) failed", text)
    return sum(int(p) for p, _ in runs), sum(int(f) for _, f in runs)


lean = int(re.search(r"Passed (\d+) Lean audits", (logs / "07-lean.log").read_text()).group(1))
counts = {k: tests(k) for k in ("04a-contract", "04b-gateway", "04c-audit", "04d-audit-routes", "04e-compiler") if (logs / f"{k}.log").exists()}
exits = {p.stem: p.read_text().strip() for p in sorted(logs.glob("*.exit"))}
gate_path = REPORT / "gate.json"
gate = json.loads(gate_path.read_text())
gate["lean_audits"] = lean
marker = f"targeted check at {head[:9]}"
LABELS = {
    "04a-contract": "cargo test -p qcore-contract",
    "04b-gateway": "the gateway tests (r2: its null tests; r3: cargo test -p qcore-c-gateway)",
    "04c-audit": "the audit tests (r2: --lib; r3: cargo test -p qcore-audit)",
    "04d-audit-routes": "member_prepass_routes",
    "04e-compiler": "the compiler contract corpus, loader, view and inheritance tests",
}
REBASES = {
    "r2": "a7e0c9a3d (MEMBER-PREPASS-OTHER-ROUTES; its landed code is disjoint from this card's; records conflicted only)",
    "r3": ("b2fbde2c5 (HANDOFF-CANDIDATE-OWN-CALLBACKS, which also records dependencies; the spec C04.18 "
           "paragraph and the records conflicted)"),
}
if marker not in gate["note"]:
    tests_text = ", ".join(f"{LABELS[k]} {p} passed" for k, (p, _) in counts.items())
    gate["note"] += (
        f" Rebased onto {REBASES[name]}: " + marker + " under the shared heavy lock (scripts "
        f"targeted.sh, close_targeted.py; logs private/lppr/targeted-{name}; exits in gate/targeted-{name}/): "
        f"cargo check --workspace --all-targets {exits['00-check']}, fmt --check {exits['02-fmt']}, "
        f"clippy --locked {exits['03-clippy']}; {tests_text}, "
        f"{sum(f for _, f in counts.values())} failed; check_lean_audits.py {lean} audits passed; "
        "lean_audits records the targeted check's count, and the matrix, the python suite and the docs "
        "index were run on it (09a2, 09b2, 10b2, 10c2)."
    )
gate_path.write_text(json.dumps(gate, indent=2, ensure_ascii=False) + "\n")
dest = REPORT / "gate" / f"targeted-{name}"
dest.mkdir(parents=True, exist_ok=True)
for path in list(logs.glob("*.exit")) + [logs / "head"]:
    shutil.copyfile(path, dest / path.name)
for label, command in (
    ("09a2-matrix", ["python3", "docs/reports/remaining-d02/build_matrix.py", ".", "--gate", str(gate_path)]),
    ("09b2-python", ["python3", "-m", "unittest", "discover", "-s", "scripts/tests"]),
    ("10b2-index", ["python3", "scripts/build_docs_index.py"]),
    ("10c2-index-check", ["python3", "scripts/build_docs_index.py", "--check"]),
):
    with open(logs / f"{label}.log", "wb") as log:
        code = subprocess.run(command, cwd=REPO, stdout=log, stderr=subprocess.STDOUT).returncode
    (logs / f"{label}.exit").write_text(f"{code}\n")
    (dest / f"{label}.exit").write_text(f"{code}\n")
    print(label, code)

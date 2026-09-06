#!/usr/bin/env python3
"""Run every checked-in Lean audit with warnings treated as failures.

AuditAll checks the compiled QCore axiom closure; the other audits retain
per-cycle theorem evidence and detect stale names. Run lake build first.
"""
from pathlib import Path
import subprocess


def main():
    proof = Path(__file__).resolve().parents[1] / "proof"
    audits = sorted(proof.glob("Audit*.lean"))
    if not audits or proof / "AuditAll.lean" not in audits:
        raise SystemExit("missing whole-project Lean axiom audit")
    for audit in audits:
        print(f"Checking {audit.name}", flush=True)
        subprocess.run(
            ["lake", "env", "lean", "-DwarningAsError=true", audit.name],
            cwd=proof,
            check=True,
        )
    print(f"Passed {len(audits)} Lean audits", flush=True)


if __name__ == "__main__":
    main()

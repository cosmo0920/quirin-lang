#!/usr/bin/env python3
"""Run the staged arbiter: one audit per cumulative declaration stage.

Usage: run_stages.py FULL_STDIO.qcore OUT_ROOT COMMIT

Stages, in the module's own order: `printf`, `+fprintf`, `+snprintf`; the
fourth (`+sprintf`) is the committed module itself and is run separately as
AFTER. Before each audit `apply_stage.py` writes the staged module; after the
last stage the full text is restored byte for byte and the round trip is
asserted (operations: restore by rewriting bytes). Each audit is
`w43-return-expression/scripts/run_audit.py OUT_ROOT/<stage> COMMIT+<stage>`.
"""

import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[4]
STAGES = [("printf", ["printf"]), ("fprintf", ["printf", "fprintf"]),
          ("snprintf", ["printf", "fprintf", "snprintf"])]


def main() -> int:
    full = pathlib.Path(sys.argv[1]).resolve()
    out_root = pathlib.Path(sys.argv[2]).resolve()
    commit = sys.argv[3]
    target = ROOT / "providers/quirin-provider-libc/contracts/libc/stdio.qcore"
    assert target.read_bytes() == full.read_bytes(), "start from the full module"
    apply = ROOT / "docs/reports/w43-libc-printf-family/scripts/apply_stage.py"
    audit = ROOT / "docs/reports/w43-return-expression/scripts/run_audit.py"
    status = 0
    try:
        for label, names in STAGES:
            subprocess.run([sys.executable, str(apply), str(full), *names], check=True)
            code = subprocess.run([sys.executable, str(audit), str(out_root / label), f"{commit}+{label}"]).returncode
            print(f"stage {label} exit {code}", flush=True)
            status = status or code
    finally:
        target.write_bytes(full.read_bytes())
        assert target.read_bytes() == full.read_bytes()
        print("restored", flush=True)
    return status


if __name__ == "__main__":
    sys.exit(main())

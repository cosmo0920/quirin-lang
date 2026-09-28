#!/usr/bin/env python3
"""identity.py COMPOSE3 OUT_DIR BASE:RUN...  -- run the integrator's
compose3.py as `compose3.py BASE BASE BASE RUN` (a run reproduces BASE when it
COMPOSES with 0 mismatches) for each pair and write each output to
OUT_DIR/<name>.txt, where <name> is the pair's position label."""
import pathlib
import subprocess
import sys


def main():
    compose3, out = sys.argv[1], pathlib.Path(sys.argv[2])
    out.mkdir(parents=True, exist_ok=True)
    status = 0
    for item in sys.argv[3:]:
        name, base, run = item.split(":", 2)
        result = subprocess.run([sys.executable, compose3, base, base, base, run],
                                capture_output=True, text=True)
        (out / f"{name}.txt").write_text(result.stdout + result.stderr)
        verdict = result.stdout.strip().splitlines()[-1] if result.stdout.strip() else "?"
        print(name, result.returncode, verdict)
        status |= result.returncode
    return status


if __name__ == "__main__":
    raise SystemExit(main())

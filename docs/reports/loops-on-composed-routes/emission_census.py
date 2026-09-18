#!/usr/bin/env python3
"""Emission census for card LOOPS-ON-COMPOSED-ROUTES (wave 39).

Runs two `quirin` binaries — the base tree's and this card's — over every
`.qcore` file in the repository (outside `target/`) with `quirin emit`, and
compares, program by program: whether each admitted the program, and whether
the emitted C is byte-identical. The claim the census supports is that every
program the base admitted is admitted after the card with byte-identical C —
unrolling is unchanged — and that every new admission is a program the base
refused for the loop-beside-a-loan shape this card lifts.

    python3 docs/reports/loops-on-composed-routes/emission_census.py \
        --base measure/census/quirin-base --card target/debug/quirin \
        --out docs/reports/loops-on-composed-routes/emission-census.json
"""
import argparse
import hashlib
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path


def emit(binary: str, source: Path) -> tuple[int, bytes, str]:
    with tempfile.TemporaryDirectory() as directory:
        out = Path(directory) / "out.c"
        run = subprocess.run(
            [binary, "emit", "-o", str(out), str(source)],
            capture_output=True,
            text=False,
            timeout=120,
        )
        c = out.read_bytes() if out.exists() else b""
        stderr = run.stderr.decode("utf-8", "replace")
        return run.returncode, c, stderr


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--base", required=True)
    parser.add_argument("--card", required=True)
    parser.add_argument("--out", required=True)
    parser.add_argument("--root", default=".")
    args = parser.parse_args()
    root = Path(args.root)
    programs = sorted(
        p
        for p in root.rglob("*.qcore")
        if "target" not in p.parts and ".lake" not in p.parts and ".git" not in p.parts
    )
    rows = []
    counts = {
        "programs": 0,
        "admitted_by_both_byte_identical": 0,
        "admitted_by_both_differing": 0,
        "admitted_by_base_only": 0,
        "admitted_by_card_only": 0,
        "refused_by_both": 0,
    }
    for program in programs:
        base_exit, base_c, base_err = emit(args.base, program)
        card_exit, card_c, card_err = emit(args.card, program)
        base_ok = base_exit == 0 and bool(base_c)
        card_ok = card_exit == 0 and bool(card_c)
        row = {
            "program": str(program),
            "base_exit": base_exit,
            "card_exit": card_exit,
            "base_sha256": hashlib.sha256(base_c).hexdigest() if base_ok else None,
            "card_sha256": hashlib.sha256(card_c).hexdigest() if card_ok else None,
        }
        counts["programs"] += 1
        if base_ok and card_ok:
            if base_c == card_c:
                row["verdict"] = "admitted_by_both_byte_identical"
            else:
                row["verdict"] = "admitted_by_both_differing"
        elif base_ok:
            row["verdict"] = "admitted_by_base_only"
            row["card_stderr_first_line"] = card_err.strip().splitlines()[:1]
        elif card_ok:
            row["verdict"] = "admitted_by_card_only"
            row["base_stderr_first_line"] = base_err.strip().splitlines()[:1]
        else:
            row["verdict"] = "refused_by_both"
        counts[row["verdict"]] += 1
        rows.append(row)
    result = {
        "card": "LOOPS-ON-COMPOSED-ROUTES",
        "base_binary": args.base,
        "card_binary": args.card,
        "counts": counts,
        "moved": [r for r in rows if r["verdict"] not in ("admitted_by_both_byte_identical", "refused_by_both")],
        "rows": rows,
    }
    Path(args.out).write_text(json.dumps(result, indent=1) + "\n")
    print(json.dumps(counts, indent=1))
    for row in result["moved"]:
        print(row["verdict"], row["program"])
    return 0 if counts["admitted_by_both_differing"] == 0 and counts["admitted_by_base_only"] == 0 else 1


if __name__ == "__main__":
    sys.exit(main())

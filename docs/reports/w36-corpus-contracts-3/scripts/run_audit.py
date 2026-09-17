#!/usr/bin/env python3
"""Card C-GATEWAY-RETURN-CALL-W4: one pinned Cycle 19 corpus audit run.

Usage: python3 measure/c-gateway-constant-do-while/run_audit.py <stage> <binary>

The pin, the compile database, the ChunkIO package and the two environment
variables are exactly the ones wave 33's chain used, so a BEFORE run of the
base binary must reproduce the current baseline archives
(`docs/reports/c-gateway-return-position/artifacts/`) byte for byte. The
transport stays `HistoricalStdin` (constraint 11) and is not selected here: it
is the audit example's own pinned-evidence route.
"""
import os
import pathlib
import subprocess
import sys
import time

WT = pathlib.Path(__file__).resolve().parents[2]
MEASURE = WT / "measure" / "w36"
PIN = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin")
PKG = pathlib.Path(
    "/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/provider-packages-wave20/qcore.chunkio"
)


def main():
    if len(sys.argv) != 3:
        print("usage: run_audit.py <stage> <binary>", file=sys.stderr)
        return 2
    stage, binary = sys.argv[1], pathlib.Path(sys.argv[2]).resolve()
    out = MEASURE / "runs" / stage
    out.mkdir(parents=True, exist_ok=True)
    env = dict(os.environ)
    env["QCORE_CLANG"] = "/usr/bin/clang"
    env["QCORE_CHUNKIO_PACKAGE"] = str(PKG)
    env.pop("QCORE_SOURCE_CONTRACTS", None)
    argv = [
        str(binary),
        "--pinned-evidence",
        str(PIN / "fluent-bit-clang-source"),
        str(PIN / "monkey" / "monkey-build" / "compile_commands.json"),
        str(out / "monkey-source.json"),
        str(out / "resource-diagnostics.json"),
    ]
    started = time.time()
    with (out / "audit.log").open("wb") as log:
        code = subprocess.call(argv, stdout=log, stderr=subprocess.STDOUT, env=env)
    print(f"stage={stage} exit={code} seconds={time.time() - started:.1f}")
    for produced in sorted(out.iterdir()):
        print(f"  {produced.stat().st_size:>10}  {produced.name}")
    return code


if __name__ == "__main__":
    sys.exit(main())

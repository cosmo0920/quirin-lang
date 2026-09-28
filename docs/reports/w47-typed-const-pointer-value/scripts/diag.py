#!/usr/bin/env python3
"""Run the temporary census binary (census_patch.py, never committed) over the
pinned corpus into a fresh private directory. A diagnostic, not a measurement.

Usage: diag.py WORKTREE BINARY OUT_DIR
"""
import os
import pathlib
import subprocess
import sys

W, BINARY, OUT = (pathlib.Path(arg) for arg in sys.argv[1:4])
PIN = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/build/cycle19")
OUT.mkdir(parents=True, exist_ok=False)
env = dict(os.environ)
env["QCORE_CLANG"] = "/usr/bin/clang"
env["QCORE_CHUNKIO_PACKAGE"] = str(PIN / "provider-packages-wave20/qcore.chunkio")
env.pop("QCORE_SOURCE_CONTRACTS", None)
cmd = [str(BINARY), "pinned", "--jobs", "5",
       str(PIN / "c-gateway-wave-pin/fluent-bit-clang-source"),
       str(PIN / "c-gateway-wave-pin/monkey/monkey-build/compile_commands.json"),
       str(OUT / "monkey-source.json"), str(OUT / "resource-diagnostics.json")]
with open(OUT / "stdout.log", "wb") as out, open(OUT / "stderr.log", "wb") as err:
    code = subprocess.run(cmd, env=env, stdout=out, stderr=err).returncode
(OUT / "exit").write_text(f"{code}\n")
print(code)

#!/usr/bin/env python3
"""Run the temporary census binary over the Fluent Bit system slice at
`046accf6`, exactly as `w46-contract-gap-expansion/scripts/run_system.py`
runs the release binary. A diagnostic, not a measurement.

Usage: diag_system.py WORKTREE BINARY OUT_DIR
"""
import os
import pathlib
import subprocess
import sys

root, binary, out = (pathlib.Path(arg).resolve() for arg in sys.argv[1:4])
prepared = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w46-external-callee-contracts/attempt-1")
source = prepared / "fluent-bit-clang-source"
out.mkdir(parents=True, exist_ok=False)
command = [str(binary), "current", "--checkout", str(source),
           "--profile-directory", str(root / "contract/profiles/fluent-bit"),
           "--compile-database", str(prepared / "fluent-bit-clang-production/compile_commands.json"),
           "--selection", str(root / "contract/profiles/fluent-bit/system-selection.json"),
           "--jobs", "5", "--report", str(out / "resource-diagnostics.json")]
environment = dict(os.environ, QCORE_CLANG="/usr/lib/llvm-21/bin/clang")
with (out / "console.log").open("wb") as stdout, (out / "stderr.log").open("wb") as stderr:
    code = subprocess.run(command, cwd=root, env=environment, stdout=stdout, stderr=stderr).returncode
(out / "exit").write_text(f"{code}\n")
print(code)

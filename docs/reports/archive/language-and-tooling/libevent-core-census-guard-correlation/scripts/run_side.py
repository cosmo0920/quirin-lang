#!/usr/bin/env python3
"""run_side.py SIDE RUN: the libevent-core configured audit with SIDE's binary
and SIDE's own profile directory, over the one shared prepared input set
(compile database and checkout), in an `audit` gate slot.

Writes private/runs/<RUN>/{config.json,console.log,receipt.json} and the
report beside it. Refuses an existing run directory (fresh paths per run).
"""
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

PRIVATE = Path("/home/cosmo/Documents/GitHub/quirin-lang/.claude/worktrees/agent-a4e7bae9ed2b2649b/private")
SIDES = {"base": PRIVATE / "base-src", "tip": PRIVATE / "tip-src", "merge": PRIVATE / "merge-src"}

side, run = sys.argv[1], sys.argv[2]
tree = SIDES[side]
out = PRIVATE / "runs" / run
out.mkdir(parents=True, exist_ok=False)
# The committed contracts.json still names libevent_memory.qcore, which
# f4018a925 renamed to .qv; the route cannot load it as committed at either
# revision. Each side reads a copy of its own profile directory whose
# contracts.json names the .qv file; nothing else is changed.
profile = out / "libevent-core"
shutil.copytree(tree / "contract/profiles/libevent-core", profile, copy_function=shutil.copyfile)
contracts = json.loads((profile / "contracts.json").read_text())
assert contracts["modules"] == ["libevent_memory.qcore"], contracts
contracts["modules"] = ["libevent_memory.qv"]
(profile / "contracts.json").write_text(json.dumps(contracts, indent=2) + "\n")
prep = PRIVATE / "libevent-prep" / "libevent-clang-build"
config = {
    "schema_version": 1,
    "profile_directory": str(profile),
    "checkout": str(PRIVATE / "tip-src/third_party/libevent"),
    "compile_database": str(prep / "compile_commands.json"),
    "libraries": ["libevent-core"],
    "compile_target": {"object_prefix": "CMakeFiles/event_core_static.dir/"},
    "report": str(out / "libevent-core-audit.json"),
}
(out / "config.json").write_text(json.dumps(config, indent=2) + "\n")
env = dict(os.environ, QCORE_CLANG="/usr/lib/llvm-21/bin/clang")
argv = [
    "python3", str(PRIVATE.parent / "scripts/gate_slot.py"), "audit", "--memory-budget", "7G",
    "--receipt", str(out / "receipt.json"), "--",
    str(tree / "target/release/quirin-audit"), str(out / "config.json"),
]
with open(out / "console.log", "w") as console:
    code = subprocess.call(argv, stdout=console, stderr=subprocess.STDOUT, env=env, cwd=tree)
(out / "exit").write_text(f"{code}\n")
print(f"{side} {run} exit {code}")

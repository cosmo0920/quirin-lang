"""Card-private spot check: the card's release quirin-audit on the four
librdkafka units of the six rows, with the null property (read-only inputs)."""
import os
import subprocess
import sys

WT = "/media/Data/GitHub/quirin-lang/.claude/worktrees/agent-ae1e0c71488222ce7"
FB = "/media/Data/GitHub/quirin-lang/build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source"
env = dict(os.environ)
env.update({
    "QCORE_CLANG": "/usr/lib/llvm-21/bin/clang",
    "QCORE_NULL_PROPERTY": f"{WT}/private/spot/spot-null.jsonl",
    "QCORE_PROFILES": f"{WT}/contract/profiles",
    "QCORE_CROSS_UNIT_LINK_MAP": f"{WT}/docs/reports/w48-bu-single-leaf-bodies/link-map.corpus.json",
    "SOURCE_DATE_EPOCH": "1767225600",
})
command = [
    f"{WT}/target/release/quirin-audit", "current", "--memory-budget-mib", "7168",
    "--checkout", FB,
    "--profile-directory", f"{WT}/contract/profiles/fluent-bit",
    "--compile-database", f"{FB}/build/compile_commands.json",
    "--selection", f"{WT}/private/spot/selection.json",
    "--jobs", "3",
    "--report", f"{WT}/private/spot/resource-diagnostics.json",
]
sys.exit(subprocess.call(command, env=env, cwd=WT))
